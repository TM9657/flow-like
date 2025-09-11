use crate::{
    entity::{app, join_queue, membership, sea_orm_active_enums::Visibility},
    error::ApiError,
    middleware::jwt::AppUser,
    state::AppState,
};
use axum::{
    Extension, Json,
    extract::{Path, State},
};
use flow_like_types::create_id;
use sea_orm::{
    ActiveModelTrait, ActiveValue::Set, ColumnTrait, EntityTrait, QueryFilter, TransactionTrait,
};

#[derive(Debug, Clone, serde::Deserialize)]
pub struct RequestJoinParams {
    pub comment: Option<String>,
}

#[tracing::instrument(name = "PUT /apps/{app_id}/team/queue", skip(state, user))]
pub async fn request_join(
    State(state): State<AppState>,
    Extension(user): Extension<AppUser>,
    Path(app_id): Path<String>,
    Json(params): Json<RequestJoinParams>,
) -> Result<Json<()>, ApiError> {
    let sub = user.sub()?;
    let txn = state.db.begin().await?;

    let membership = membership::Entity::find()
        .filter(membership::Column::AppId.eq(app_id.clone()))
        .filter(membership::Column::UserId.eq(sub.clone()))
        .one(&txn)
        .await?;

    if membership.is_some() {
        tracing::warn!(
            "User {} is trying to join app {} but is already a member",
            sub,
            app_id
        );
        return Err(ApiError::Forbidden);
    }

    let app = app::Entity::find_by_id(app_id.clone())
        .one(&txn)
        .await?
        .ok_or_else(|| ApiError::NotFound)?;

    let default_role_id = app
        .default_role_id
        .clone()
        .ok_or_else(|| ApiError::NotFound)?;

    if app.visibility == Visibility::Public && app.price <= 0 {
        let membership = membership::ActiveModel {
            id: Set(create_id()),
            user_id: Set(sub.clone()),
            app_id: Set(app_id.clone()),
            role_id: Set(default_role_id),
            created_at: Set(chrono::Utc::now().naive_utc()),
            updated_at: Set(chrono::Utc::now().naive_utc()),
            joined_via: Set(Some("request_join".to_string())),
        };

        membership.insert(&txn).await?;
        txn.commit().await?;
        return Ok(Json(()));
    }

    if !matches!(app.visibility, Visibility::PublicRequestAccess | Visibility::Public) {
        tracing::warn!(
            "User {} is trying to join app {} but the app is not public",
            sub,
            app_id
        );
        return Err(ApiError::Forbidden);
    }

    let existing_request = join_queue::Entity::find()
        .filter(join_queue::Column::AppId.eq(app_id.clone()))
        .filter(join_queue::Column::UserId.eq(sub.clone()))
        .one(&txn)
        .await?;

    if let Some(existing_request) = existing_request {
        tracing::warn!(
            "User {} is trying to join app {} but already has a pending request",
            sub,
            app_id
        );

        let mut existing_request: join_queue::ActiveModel = existing_request.into();
        existing_request.comment = Set(params.comment);
        existing_request.updated_at = Set(chrono::Utc::now().naive_utc());
        existing_request.update(&txn).await?;
        txn.commit().await?;
        return Ok(Json(()));
    }

    let new_request = join_queue::ActiveModel {
        id: Set(create_id()),
        user_id: Set(sub),
        app_id: Set(app_id.clone()),
        comment: Set(params.comment),
        created_at: Set(chrono::Utc::now().naive_utc()),
        updated_at: Set(chrono::Utc::now().naive_utc()),
    };

    new_request.insert(&txn).await?;
    txn.commit().await?;
    Ok(Json(()))
}
