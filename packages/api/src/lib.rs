use std::sync::Arc;

use axum::{Json, Router, middleware::from_fn_with_state, routing::get};
use error::InternalError;
use flow_like::hub::Hub;
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
        .nest("/llm", routes::llm::routes())
        .nest("/admin", routes::admin::routes())
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
) -> Result<Json<Hub>, InternalError> {
    Ok(Json(state.platform_config.clone()))
}
