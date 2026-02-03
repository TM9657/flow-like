use lambda_runtime::{run, service_fn, Error};

mod event_handler;
use event_handler::function_handler;
use tracing_subscriber::{EnvFilter, layer::SubscriberExt, util::SubscriberInitExt};

#[tokio::main]
async fn main() -> Result<(), Error> {
    // Default to warn level to reduce log noise (errors, warnings, fatals only)
    // Can be overridden with RUST_LOG env var
    let env_filter = EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| EnvFilter::new("warn"));

    let _tracing = tracing_subscriber::registry()
        .with(tracing_subscriber::fmt::layer())
        .with(env_filter)
        .try_init();

    run(service_fn(function_handler)).await
}
