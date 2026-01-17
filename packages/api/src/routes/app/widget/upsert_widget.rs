use crate::{
    ensure_permission,
    entity::{meta, widget},
    error::ApiError,
    middleware::jwt::AppUser,
    permission::role_permission::RolePermissions,
    state::AppState,
};
use axum::{
    Extension, Json,
    extract::{Path, State},
};
use flow_like::a2ui::widget::Widget;
use flow_like_types::create_id;
use sea_orm::{ActiveModelTrait, ActiveValue::Set, ColumnTrait, EntityTrait, QueryFilter};
use serde::{Deserialize, Serialize};

#[derive(Clone, Deserialize, Serialize)]
pub struct WidgetUpsert {
    pub widget: Widget,
}

#[tracing::instrument(
    name = "PUT /apps/{app_id}/widgets/{widget_id}",
    skip(state, user, widget_data)
)]
pub async fn upsert_widget(
    State(state): State<AppState>,
    Extension(user): Extension<AppUser>,
    Path((app_id, widget_id)): Path<(String, String)>,
    Json(widget_data): Json<WidgetUpsert>,
) -> Result<Json<Widget>, ApiError> {
    ensure_permission!(user, &app_id, &state, RolePermissions::WriteWidgets);

    if widget_id.is_empty() || app_id.is_empty() {
        return Err(ApiError::FORBIDDEN);
    }

    let mut app = state
        .scoped_app(
            &user.sub()?,
            &app_id,
            &state,
            crate::credentials::CredentialsAccess::EditApp,
        )
        .await?;

    let mut widget = widget_data.widget;
    widget.id = widget_id.clone();

    app.save_widget(&widget).await?;

    // Check if widget exists in DB
    let existing = widget::Entity::find_by_id(&widget_id)
        .filter(widget::Column::AppId.eq(&app_id))
        .one(&state.db)
        .await?;

    if existing.is_none() {
        // Create new widget record in DB
        let new_widget = widget::ActiveModel {
            id: Set(widget_id.clone()),
            app_id: Set(app_id.to_string()),
            version: Set(widget.version.map(|v| format!("{}.{}.{}", v.0, v.1, v.2))),
            created_at: Set(chrono::Utc::now().naive_utc()),
            updated_at: Set(chrono::Utc::now().naive_utc()),
        };

        widget::Entity::insert(new_widget)
            .exec_with_returning(&state.db)
            .await?;

        // Create default meta
        let meta_model = meta::ActiveModel {
            id: Set(create_id()),
            lang: Set("en".to_string()),
            name: Set(widget.name.clone()),
            description: Set(widget.description.clone()),
            widget_id: Set(Some(widget_id.clone())),
            created_at: Set(chrono::Utc::now().naive_utc()),
            updated_at: Set(chrono::Utc::now().naive_utc()),
            ..Default::default()
        };

        meta::Entity::insert(meta_model).exec(&state.db).await?;

        if !app.widget_ids.contains(&widget_id) {
            app.widget_ids.push(widget_id);
            app.save().await?;
        }
    } else {
        // Update existing widget record
        let update_widget = widget::ActiveModel {
            id: Set(widget_id.clone()),
            app_id: Set(app_id.to_string()),
            version: Set(widget.version.map(|v| format!("{}.{}.{}", v.0, v.1, v.2))),
            updated_at: Set(chrono::Utc::now().naive_utc()),
            ..Default::default()
        };

        update_widget.update(&state.db).await?;
    }

    Ok(Json(widget))
}
