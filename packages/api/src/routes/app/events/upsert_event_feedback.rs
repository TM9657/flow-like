use crate::{
    ensure_permission, entity::feedback, error::ApiError, middleware::jwt::AppUser,
    permission::role_permission::RolePermissions, state::AppState,
};
use axum::{
    Extension, Json,
    extract::{Path, State},
};
use flow_like_types::{Value, create_id};
use sea_orm::{
    ActiveModelTrait, ActiveValue::Set, ColumnTrait, EntityTrait, IntoActiveModel, QueryFilter,
    TransactionTrait,
};
use serde::{Deserialize, Serialize};

#[derive(Deserialize, Debug)]
pub struct FeedbackBody {
    pub rating: i64,
    pub context: Option<Value>,
    pub comment: String,
    pub feedback_id: String,
}

#[derive(Serialize, Debug)]
pub struct FeedbackResponse {
    pub feedback_id: String,
}

#[tracing::instrument(
    name = "PUT /apps/{app_id}/events/{event_id}/feedback",
    skip(state, user)
)]
pub async fn upsert_event_feedback(
    State(state): State<AppState>,
    Extension(user): Extension<AppUser>,
    Path((app_id, event_id)): Path<(String, String)>,
    Json(body): Json<FeedbackBody>,
) -> Result<Json<FeedbackResponse>, ApiError> {
    let permission = ensure_permission!(user, &app_id, &state, RolePermissions::ExecuteEvents);
    let sub = permission.sub()?;

    let txn = state.db.begin().await?;

    let existing_feedback = feedback::Entity::find()
        .filter(feedback::Column::AppId.eq(app_id.clone()))
        .filter(feedback::Column::EventId.eq(event_id.clone()))
        .filter(feedback::Column::Id.eq(body.feedback_id.clone()))
        .one(&txn)
        .await?;

    if let Some(existing) = existing_feedback {
        if existing.user_id.as_ref() != Some(&sub) {
            return Err(ApiError::Forbidden);
        }

        // Update existing feedback
        let mut feedback = existing.into_active_model();
        feedback.context = Set(body.context);
        feedback.comment = Set(body.comment);
        feedback.rating = Set(body.rating.clamp(0, 5));
        feedback.updated_at = Set(chrono::Utc::now().naive_utc());

        feedback.update(&txn).await?;
        txn.commit().await?;
        return Ok(Json(FeedbackResponse {
            feedback_id: body.feedback_id.clone(),
        }));
    }

    let id = create_id();
    let feedback = feedback::Model {
        id: id.clone(),
        app_id: Some(app_id.clone()),
        user_id: Some(sub),
        event_id: Some(event_id.clone()),
        context: body.context,
        comment: body.comment,
        rating: body.rating.clamp(0, 5) as i64,
        template_id: None,
        created_at: chrono::Utc::now().naive_utc(),
        updated_at: chrono::Utc::now().naive_utc(),
    };

    let mut feedback = feedback::ActiveModel::from(feedback);
    feedback = feedback.reset_all();

    feedback.insert(&txn).await?;
    txn.commit().await?;

    Ok(Json(FeedbackResponse { feedback_id: id }))
}
