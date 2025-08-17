use dotenv::dotenv;
use flow_like_api::axum;
use flow_like_api::construct_router;
use flow_like_storage::object_store::aws::AmazonS3Builder;
use flow_like_types::tokio;
use socket2::{Domain, Socket, Type};
use std::{
    io,
    net::{SocketAddr, ToSocketAddrs},
    sync::Arc,
    time::Duration,
};
use tracing_subscriber::prelude::*;

#[flow_like_types::tokio::main]
async fn main() {
    dotenv().ok();

    let sentry_endpoint = std::env::var("SENTRY_ENDPOINT").unwrap_or_default();

    let _sentry_guard = if sentry_endpoint.is_empty() {
        tracing_subscriber::registry()
            .with(tracing_subscriber::fmt::layer())
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
            .with(tracing_subscriber::fmt::layer())
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

    let catalog = Arc::new(flow_like_catalog::get_catalog().await);
    let state = Arc::new(flow_like_api::state::State::new(catalog, Arc::new(cdn_bucket)).await);

    let app = construct_router(state);

    let port = 3210;
    let listener = match create_listener(format!("0.0.0.0:{}", port)) {
        Ok(listener) => listener,
        Err(err) => {
            eprintln!("Failed to bind to port {}: {:?}", port, err);
            return;
        }
    };
    axum::serve(listener, app).await.unwrap();
}

fn create_listener<A: ToSocketAddrs>(
    addr: A,
) -> io::Result<flow_like_types::tokio::net::TcpListener> {
    let mut addrs = addr.to_socket_addrs()?;
    let addr = addrs.next().unwrap();
    let listener = match &addr {
        SocketAddr::V4(_) => Socket::new(Domain::IPV4, Type::STREAM, None)?,
        SocketAddr::V6(_) => Socket::new(Domain::IPV6, Type::STREAM, None)?,
    };

    listener.set_nonblocking(true)?;
    listener.set_nodelay(true)?;
    listener.set_reuse_address(true)?;
    listener.set_linger(Some(Duration::from_secs(0)))?;
    listener.bind(&addr.into())?;
    listener.listen(i32::MAX)?;

    let listener = std::net::TcpListener::from(listener);
    let listener = flow_like_types::tokio::net::TcpListener::from_std(listener)?;
    Ok(listener)
}
