use axum::{
    Router,
    routing::{delete, get, put},
};
use flow_like_storage::Path as FlowPath;
use sea_orm::{ColumnTrait, EntityTrait, QueryFilter};
use serde::Deserialize;
use utoipa::ToSchema;

use crate::{
    auth::AppUser,
    ensure_in_project, ensure_permission,
    entity::{
        app_group, app_group_member, meta,
        sea_orm_active_enums::{AppGroupMemberStatus, Visibility},
    },
    error::ApiError,
    middleware::jwt::AppPermissionResponse,
    permission::role_permission::RolePermissions,
    routes::app::connection::deny_connected_app,
    state::AppState,
};

pub mod push_media;
pub mod remove_media;

pub mod get_meta;
pub mod upsert_meta;

pub fn routes() -> Router<AppState> {
    Router::new()
        .route("/", get(get_meta::get_meta).put(upsert_meta::upsert_meta))
        .route("/media", put(push_media::push_media))
        .route("/media/{media_id}", delete(remove_media::remove_media))
}

#[derive(Deserialize, Debug, ToSchema)]
pub struct MetaQuery {
    pub language: Option<String>,
    pub template_id: Option<String>,
    pub course_id: Option<String>,
    pub widget_id: Option<String>,
    pub group_id: Option<String>,
}

#[derive(Deserialize, Debug, ToSchema)]
#[serde(rename_all = "lowercase")]
pub enum MediaItem {
    Icon,
    Thumbnail,
    Preview,
}
#[derive(Deserialize, Debug, ToSchema)]
pub struct MediaQuery {
    pub language: Option<String>,
    pub template_id: Option<String>,
    pub course_id: Option<String>,
    pub widget_id: Option<String>,
    pub group_id: Option<String>,
    pub item: MediaItem,
    pub extension: String,
}

/// Extension charset for every key the API mints from client input: ASCII
/// alphanumerics only, lower-cased, at most 16 bytes.
pub(crate) fn sanitize_ext(input: Option<&str>) -> Option<String> {
    let ext = input?.trim().trim_start_matches('.').to_ascii_lowercase();
    if ext.is_empty() || ext.len() > 16 || !ext.chars().all(|c| c.is_ascii_alphanumeric()) {
        return None;
    }
    Some(ext)
}

/// `{item_id}.{ext}` for a media upload; an extension that could smuggle path
/// or percent characters into the object key is rejected.
pub(crate) fn media_item_name(item_id: &str, extension: &str) -> Result<String, ApiError> {
    let ext = sanitize_ext(Some(extension)).ok_or_else(|| {
        ApiError::bad_request(format!("unsupported media extension '{extension}'"))
    })?;
    Ok(format!("{item_id}.{ext}"))
}

pub enum MetaMode {
    Template(String),
    App(String),
    Course(String),
    Widget(String),
    Group(String),
}

impl MetaMode {
    pub fn new(query: &MetaQuery, app_id: &str) -> Self {
        if let Some(template_id) = &query.template_id {
            MetaMode::Template(template_id.clone())
        } else if let Some(course_id) = &query.course_id {
            MetaMode::Course(course_id.clone())
        } else if let Some(widget_id) = &query.widget_id {
            MetaMode::Widget(widget_id.clone())
        } else if let Some(group_id) = &query.group_id {
            MetaMode::Group(group_id.clone())
        } else {
            MetaMode::App(app_id.to_string())
        }
    }

    pub fn from_media_query(query: &MediaQuery, app_id: &str) -> Self {
        if let Some(template_id) = &query.template_id {
            MetaMode::Template(template_id.clone())
        } else if let Some(course_id) = &query.course_id {
            MetaMode::Course(course_id.clone())
        } else if let Some(widget_id) = &query.widget_id {
            MetaMode::Widget(widget_id.clone())
        } else if let Some(group_id) = &query.group_id {
            MetaMode::Group(group_id.clone())
        } else {
            MetaMode::App(app_id.to_string())
        }
    }

    /// Object-store prefix holding this entity's media. Groups keep their own
    /// folder so forking an app (which copies `media/apps/{app_id}` wholesale)
    /// never drags suite artwork along.
    pub fn media_prefix(&self, app_id: &str) -> FlowPath {
        match self {
            MetaMode::Group(group_id) => FlowPath::from("media")
                .join("groups")
                .join(group_id.clone()),
            _ => FlowPath::from("media")
                .join("apps")
                .join(app_id.to_string()),
        }
    }

    pub async fn ensure_write_permission(
        &self,
        user: &AppUser,
        app_id: &str,
        state: &AppState,
    ) -> Result<AppPermissionResponse, ApiError> {
        match self {
            MetaMode::Template(_) => Ok(ensure_permission!(
                user,
                app_id,
                state,
                RolePermissions::WriteTemplates
            )),
            MetaMode::Course(_) => Ok(ensure_permission!(
                user,
                app_id,
                state,
                RolePermissions::WriteCourses
            )),
            MetaMode::Widget(_) => Ok(ensure_permission!(
                user,
                app_id,
                state,
                RolePermissions::WriteWidgets
            )),
            MetaMode::App(_) => Ok(ensure_permission!(
                user,
                app_id,
                state,
                RolePermissions::WriteMeta
            )),
            MetaMode::Group(group_id) => {
                deny_connected_app(user)?;
                let permission = ensure_permission!(user, app_id, state, RolePermissions::Admin);
                Self::ensure_group_anchor(group_id, app_id, state).await?;
                Ok(permission)
            }
        }
    }

    /// A group's branding is editable only through its owner (anchor) app, so
    /// admin rights on an unrelated app can never rewrite someone else's suite.
    async fn ensure_group_anchor(
        group_id: &str,
        app_id: &str,
        state: &AppState,
    ) -> Result<app_group::Model, ApiError> {
        let group = app_group::Entity::find_by_id(group_id)
            .one(&state.db)
            .await?
            .ok_or(ApiError::NOT_FOUND)?;
        if group.owner_app_id != app_id {
            return Err(ApiError::FORBIDDEN);
        }
        Ok(group)
    }

    /// Suites are readable by their anchor app and by any app with an active
    /// membership; anything else falls through to the public-visibility check.
    pub async fn is_publicly_visible_group(
        group_id: &str,
        state: &AppState,
    ) -> Result<bool, ApiError> {
        let group = app_group::Entity::find_by_id(group_id)
            .one(&state.db)
            .await?
            .ok_or(ApiError::NOT_FOUND)?;
        Ok(matches!(
            group.visibility,
            Visibility::Public | Visibility::PublicRequestAccess
        ))
    }

    pub async fn ensure_read_permission(
        &self,
        user: &AppUser,
        app_id: &str,
        state: &AppState,
    ) -> Result<AppPermissionResponse, ApiError> {
        match self {
            MetaMode::Template(_) => Ok(ensure_permission!(
                user,
                app_id,
                state,
                RolePermissions::ReadTemplates
            )),
            MetaMode::Course(_) => Ok(ensure_permission!(
                user,
                app_id,
                state,
                RolePermissions::ReadCourses
            )),
            MetaMode::Widget(_) => Ok(ensure_permission!(
                user,
                app_id,
                state,
                RolePermissions::ReadWidgets
            )),
            MetaMode::App(_) => Ok(ensure_in_project!(user, &app_id, &state)),
            MetaMode::Group(group_id) => {
                let permission = ensure_permission!(user, app_id, state, RolePermissions::ReadTeam);
                let group = app_group::Entity::find_by_id(group_id)
                    .one(&state.db)
                    .await?
                    .ok_or(ApiError::NOT_FOUND)?;
                if group.owner_app_id == app_id {
                    return Ok(permission);
                }
                let is_member = app_group_member::Entity::find()
                    .filter(app_group_member::Column::GroupId.eq(group_id))
                    .filter(app_group_member::Column::AppId.eq(app_id))
                    .filter(app_group_member::Column::Status.eq(AppGroupMemberStatus::Active))
                    .one(&state.db)
                    .await?
                    .is_some();
                if !is_member {
                    return Err(ApiError::FORBIDDEN);
                }
                Ok(permission)
            }
        }
    }

    pub async fn find_existing_meta<C: sea_orm::ConnectionTrait>(
        &self,
        language: &str,
        db: &C,
    ) -> Result<Option<meta::Model>, sea_orm::DbErr> {
        match self {
            MetaMode::Template(id) => {
                meta::Entity::find()
                    .filter(meta::Column::TemplateId.eq(id))
                    .filter(meta::Column::Lang.eq(language))
                    .one(db)
                    .await
            }
            MetaMode::App(id) => {
                meta::Entity::find()
                    .filter(meta::Column::AppId.eq(id))
                    .filter(meta::Column::Lang.eq(language))
                    .one(db)
                    .await
            }
            MetaMode::Course(id) => {
                meta::Entity::find()
                    .filter(meta::Column::CourseId.eq(id))
                    .filter(meta::Column::Lang.eq(language))
                    .one(db)
                    .await
            }
            MetaMode::Widget(id) => {
                meta::Entity::find()
                    .filter(meta::Column::WidgetId.eq(id))
                    .filter(meta::Column::Lang.eq(language))
                    .one(db)
                    .await
            }
            MetaMode::Group(id) => {
                meta::Entity::find()
                    .filter(meta::Column::GroupId.eq(id))
                    .filter(meta::Column::Lang.eq(language))
                    .one(db)
                    .await
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sanitize_ext_keeps_only_ascii_alphanumerics() {
        assert_eq!(sanitize_ext(Some(" .PNG ")).as_deref(), Some("png"));
        assert_eq!(sanitize_ext(Some("webp")).as_deref(), Some("webp"));
        assert_eq!(sanitize_ext(Some("pn%g")), None);
        assert_eq!(sanitize_ext(Some("Übersicht")), None);
        assert_eq!(sanitize_ext(Some("png/../x")), None);
        assert_eq!(sanitize_ext(Some("")), None);
        assert_eq!(sanitize_ext(Some("averyverylongextension")), None);
        assert_eq!(sanitize_ext(None), None);
    }

    #[test]
    fn media_item_name_rejects_unsafe_extensions() {
        assert_eq!(media_item_name("id-1", "JPG").unwrap(), "id-1.jpg");
        assert!(media_item_name("id-1", "pn%g").is_err());
        assert!(media_item_name("id-1", "Übersicht").is_err());
        assert!(media_item_name("id-1", "").is_err());
    }
}
