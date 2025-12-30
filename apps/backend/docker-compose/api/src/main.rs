#[global_allocator]
static GLOBAL: mimalloc::MiMalloc = mimalloc::MiMalloc;

use axum::Router;
use flow_like_api::{construct_router, state::State};
use flow_like_catalog::get_catalog;
use std::sync::Arc;
use tower_http::cors::CorsLayer;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

mod config;
mod storage;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    tracing_subscriber::registry()
        .with(tracing_subscriber::fmt::layer())
        .with(tracing_subscriber::EnvFilter::from_default_env())
        .init();

    tracing::info!("Starting Flow-Like Docker Compose API Service");

    let config = config::Config::from_env()?;
    tracing::info!("Loaded configuration: provider={}", config.provider());

    if !flow_like_api::execution::is_jwt_configured() {
        tracing::warn!(
            "Execution JWT keys not configured. \
            Generate keys using ./scripts/gen-execution-keys.sh and set \
            EXECUTION_KEY, EXECUTION_PUB environment variables."
        );
    }

    let catalog = get_catalog();

    let cdn_bucket = storage::create_store(&config)?;

    let state = Arc::new(State::new(Arc::new(catalog), Arc::new(cdn_bucket)).await);

    let app = Router::new()
        .merge(construct_router(state.clone()))
        .layer(CorsLayer::permissive());

    let addr = format!("0.0.0.0:{}", config.port);
    tracing::info!("Listening on {}", addr);

    let listener = tokio::net::TcpListener::bind(&addr).await?;
    axum::serve(listener, app).await?;

    Ok(())
}
