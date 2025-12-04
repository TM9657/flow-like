use std::sync::Arc;

use axum::{Json, Router, middleware::from_fn_with_state, routing::get};
use error::InternalError;
use flow_like_types::Value;
use middleware::jwt::jwt_middleware;
use state::{AppState, State};
use tower::ServiceBuilder;
use tower_http::{
    compression::{CompressionLayer, DefaultPredicate, Predicate, predicate::NotForContentType},
    cors::CorsLayer,
    decompression::RequestDecompressionLayer,
};

pub mod entity;
mod middleware;
mod routes;

pub mod credentials;
pub mod error;
pub mod permission;
pub mod state;
pub mod user_management;

pub use axum;
pub mod auth {
    use crate::middleware;
    pub use middleware::jwt::AppUser;
}

pub use sea_orm;

pub fn construct_router(state: Arc<State>) -> Router {
    let router = Router::new()
        .route("/", get(hub_info))
        .nest("/health", routes::health::routes())
        .nest("/info", routes::info::routes())
        .nest("/user", routes::user::routes())
        .nest("/profile", routes::profile::routes())
        .nest("/apps", routes::app::routes())
        .nest("/bit", routes::bit::routes())
        .nest("/store", routes::store::routes())
        .nest("/auth", routes::auth::routes())
        .nest("/oauth", routes::oauth::routes())
        .nest("/chat", routes::chat::routes())
        .nest("/admin", routes::admin::routes())
        .nest("/tmp", routes::tmp::routes())
        .with_state(state.clone())
        .route("/version", get(|| async { "0.0.0" }))
        .layer(from_fn_with_state(state.clone(), jwt_middleware))
        .layer(CorsLayer::permissive())
        .layer(
            ServiceBuilder::new()
                // .layer(TimeoutLayer::new(Duration::from_secs(15 * 60)))
                .layer(RequestDecompressionLayer::new())
                .layer(CompressionLayer::new().compress_when(
                    DefaultPredicate::new().and(NotForContentType::new("text/event-stream")),
                )),
        );

    Router::new().nest("/api/v1", router)
}

#[tracing::instrument(name = "GET /", skip(state))]
async fn hub_info(
    axum::extract::State(state): axum::extract::State<AppState>,
) -> Result<Json<Value>, InternalError> {
    // Serialize hub to JSON value so we can modify it
    let mut hub_value: Value = serde_json::to_value(&state.platform_config)?;

    // Strip sensitive OAuth fields (client_secret_env and client_secret)
    if let Some(oauth_providers) = hub_value.get_mut("oauth_providers") {
        if let Some(providers_obj) = oauth_providers.as_object_mut() {
            for (_provider_id, provider_config) in providers_obj.iter_mut() {
                if let Some(config_obj) = provider_config.as_object_mut() {
                    config_obj.remove("client_secret_env");
                    config_obj.remove("client_secret");
                }
            }
        }
    }

    Ok(Json(hub_value))
}
