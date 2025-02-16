use crate::{
    error::AppError,
    state::{AppState, Contact, Features},
};
use axum::{extract::State, routing::get, Json, Router};

pub fn routes() -> Router<AppState> {
    Router::new()
        .route("/legal", get(legal_notice))
        .route("/privacy", get(privacy_policy))
        .route("/terms", get(terms_of_service))
        .route("/contact", get(contact))
        .route("/features", get(features))
}

#[tracing::instrument]
async fn legal_notice(State(state): State<AppState>) -> Result<String, AppError> {
    let notice = state.platform_config.legal_notice.clone();
    Ok(notice)
}

#[tracing::instrument]
async fn privacy_policy(State(state): State<AppState>) -> Result<String, AppError> {
    let privacy_policy = state.platform_config.privacy_policy.clone();
    Ok(privacy_policy)
}

#[tracing::instrument]
async fn terms_of_service(State(state): State<AppState>) -> Result<String, AppError> {
    let terms_of_service = state.platform_config.terms_of_service.clone();
    Ok(terms_of_service)
}

#[tracing::instrument]
async fn contact(State(state): State<AppState>) -> Result<Json<Contact>, AppError> {
    let contact = state.platform_config.contact.clone();
    Ok(Json(contact))
}

#[tracing::instrument]
async fn features(State(state): State<AppState>) -> Result<Json<Features>, AppError> {
    let features = state.platform_config.features.clone();
    Ok(Json(features))
}
