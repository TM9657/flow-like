// https://github.com/awslabs/aws-lambda-rust-runtime/tree/main/examples/http-axum

#[global_allocator]
static GLOBAL: mimalloc::MiMalloc = mimalloc::MiMalloc;

use axum::http::StatusCode;
use axum::{routing::get, Router};
use flow_like_types::tokio;
use lambda_http::{run_with_streaming_response, tracing, Error};
use std::env::set_var;

async fn health_check() -> (StatusCode, String) {
    let health = true;
    match health {
        true => (StatusCode::OK, "Healthy!".to_string()),
        false => (
            StatusCode::INTERNAL_SERVER_ERROR,
            "Not healthy!".to_string(),
        ),
    }
}

#[flow_like_types::tokio::main]
async fn main() -> Result<(), Error> {
    // If you use API Gateway stages, the Rust Runtime will include the stage name
    // as part of the path that your application receives.
    // Setting the following environment variable, you can remove the stage from the path.
    // This variable only applies to API Gateway stages,
    // you can remove it if you don't use them.
    // i.e with: `GET /test-stage/todo/id/123` without: `GET /todo/id/123`
    set_var("AWS_LAMBDA_HTTP_IGNORE_STAGE_IN_PATH", "true");

    // required to enable CloudWatch error logging by the runtime
    tracing::init_default_subscriber();

    let app = Router::new().route("/health/", get(health_check));

    run_with_streaming_response(app).await
}
