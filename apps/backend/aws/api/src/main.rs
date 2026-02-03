#[global_allocator]
static GLOBAL: mimalloc::MiMalloc = mimalloc::MiMalloc;

use flow_like_api::construct_router;
use flow_like_catalog::get_catalog;
use flow_like_storage::object_store::aws::AmazonS3Builder;
use flow_like_types::tokio;
use lambda_http::{Error, run_with_streaming_response};
use std::sync::Arc;
use tracing_subscriber::prelude::*;

#[flow_like_types::tokio::main(flavor = "current_thread")]
async fn main() -> Result<(), Error> {
    let sentry_endpoint = std::env::var("SENTRY_ENDPOINT").unwrap_or_default();

    let env_filter = flow_like_api::warn_env_filter();

    let _sentry_guard = if sentry_endpoint.is_empty() {
        tracing_subscriber::registry()
            .with(tracing_subscriber::fmt::layer().with_filter(env_filter))
            .init();
        None
    } else {
        let guard = sentry::init((
            sentry_endpoint,
            sentry::ClientOptions {
                release: sentry::release_name!(),
                traces_sample_rate: 0.3,
                ..Default::default()
            },
        ));
        tracing_subscriber::registry()
            .with(tracing_subscriber::fmt::layer().with_filter(env_filter))
            .with(sentry_tracing::layer())
            .init();
        Some(guard)
    };

    let cdn_bucket = std::env::var("CDN_BUCKET_NAME").unwrap();
    let cdn_bucket_endpoint = std::env::var("CDN_BUCKET_ENDPOINT").ok();
    let cdn_bucket_access_key = std::env::var("CDN_BUCKET_ACCESS_KEY_ID").ok();
    let cdn_bucket_secret_key = std::env::var("CDN_BUCKET_SECRET_ACCESS_KEY").ok();

    let mut cdn_bucket = AmazonS3Builder::new().with_bucket_name(cdn_bucket);
    if let Some(endpoint) = cdn_bucket_endpoint
        && !endpoint.is_empty()
    {
        cdn_bucket = cdn_bucket.with_endpoint(endpoint);
    }

    if let (Some(access_key), Some(secret_key)) = (cdn_bucket_access_key, cdn_bucket_secret_key)
        && !access_key.is_empty()
        && !secret_key.is_empty()
    {
        cdn_bucket = cdn_bucket.with_access_key_id(access_key);
        cdn_bucket = cdn_bucket.with_secret_access_key(secret_key);
    }

    let cdn_bucket =
        flow_like_storage::files::store::FlowLikeStore::AWS(Arc::new(cdn_bucket.build().unwrap()));

    let catalog = Arc::new(get_catalog());
    let state = Arc::new(flow_like_api::state::State::new(catalog, Arc::new(cdn_bucket)).await);
    let app = construct_router(state);

    run_with_streaming_response(app).await
}
