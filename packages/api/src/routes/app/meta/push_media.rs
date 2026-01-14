use std::time::Duration;

use crate::{
    entity::meta,
    error::ApiError,
    middleware::jwt::AppUser,
    routes::app::meta::{MediaItem, MediaQuery, MetaMode},
    state::AppState,
};
use axum::{
    Extension, Json,
    extract::{Path, Query, State},
};
use flow_like_storage::Path as FlowPath;
use flow_like_types::{anyhow, create_id};
use sea_orm::{ActiveModelTrait, ActiveValue::Set, TransactionTrait};

#[derive(Debug, serde::Serialize)]
pub struct PushMediaResponse {
    pub signed_url: String,
}

#[tracing::instrument(name = "PUT /apps/{app_id}/meta/media", skip(state, user))]
pub async fn push_media(
    State(state): State<AppState>,
    Extension(user): Extension<AppUser>,
    Path(app_id): Path<String>,
    Query(query): Query<MediaQuery>,
) -> Result<Json<PushMediaResponse>, ApiError> {
    let mode = MetaMode::from_media_query(&query, &app_id);
    mode.ensure_write_permission(&user, &app_id, &state).await?;
    let language = query.language.as_deref().unwrap_or("en");

    let txn = state.db.begin().await?;

    let existing_meta = mode
        .find_existing_meta(language, &txn)
        .await?
        .ok_or_else(|| ApiError::NotFound)?;

    let mut existing_preview = existing_meta.preview_media.clone().unwrap_or_default();

    let mut model: meta::ActiveModel = existing_meta.clone().into();
    model.updated_at = Set(chrono::Utc::now().naive_utc());
    let item_id = create_id();
    let item_name = format!("{}.{}", item_id, query.extension);

    let master_store = state.master_credentials().await?;
    let master_store = master_store.to_store(false).await?;

    match &query.item {
        MediaItem::Icon => {
            if let Some(icon) = &existing_meta.icon {
                let file_name = format!("{}.webp", icon);
                let path = FlowPath::from("media")
                    .child("apps")
                    .child(app_id.clone())
                    .child(file_name);
                if let Err(err) = master_store.as_generic().delete(&path).await {
                    tracing::error!("Failed to delete existing icon at {}: {:?}", path, err);
                }
            }
            model.icon = Set(Some(item_id));
        }
        MediaItem::Thumbnail => {
            if let Some(thumbnail) = &existing_meta.thumbnail {
                let file_name = format!("{}.webp", thumbnail);
                let path = FlowPath::from("media")
                    .child("apps")
                    .child(app_id.clone())
                    .child(file_name);
                if let Err(err) = master_store.as_generic().delete(&path).await {
                    tracing::error!("Failed to delete existing thumbnail at {}: {:?}", path, err);
                }
            }
            model.thumbnail = Set(Some(item_id));
        }
        MediaItem::Preview => {
            existing_preview.push(item_id.clone());
            model.preview_media = Set(Some(existing_preview));
        }
    }

    model.update(&txn).await?;
    let path = FlowPath::from("media")
        .child("apps")
        .child(app_id)
        .child(item_name.clone());
    let signed_url = master_store
        .sign("PUT", &path, Duration::from_secs(60 * 60 * 24))
        .await
        .map_err(|e| {
            let id = create_id();
            tracing::error!(
                "[{}] Failed to sign URL for media item '{}' - {:?}",
                id,
                item_name,
                e
            );
            ApiError::InternalError(
                anyhow!("Failed to create signed URL, reference ID: {}", id),
            )
        })?;

    txn.commit().await?;
    Ok(Json(PushMediaResponse {
        signed_url: signed_url.to_string(),
    }))
}
