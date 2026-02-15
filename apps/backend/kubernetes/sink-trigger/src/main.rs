use serde::Serialize;
use std::env;
use std::process::ExitCode;

#[derive(Serialize)]
struct TriggerRequest {
    event_id: String,
    sink_type: String,
}

#[tokio::main(flavor = "current_thread")]
async fn main() -> ExitCode {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::from_default_env()
                .add_directive(tracing::Level::INFO.into()),
        )
        .compact()
        .init();

    let event_id = match env::var("EVENT_ID") {
        Ok(v) => v,
        Err(_) => {
            tracing::error!("EVENT_ID environment variable is required");
            return ExitCode::FAILURE;
        }
    };

    let sink_type = env::var("SINK_TYPE").unwrap_or_else(|_| "cron".to_string());

    let api_base_url = match env::var("API_BASE_URL") {
        Ok(v) => v,
        Err(_) => {
            tracing::error!("API_BASE_URL environment variable is required");
            return ExitCode::FAILURE;
        }
    };

    let jwt = match env::var("SINK_TRIGGER_JWT") {
        Ok(v) => v,
        Err(_) => {
            tracing::error!("SINK_TRIGGER_JWT environment variable is required");
            return ExitCode::FAILURE;
        }
    };

    let url = format!(
        "{}/api/v1/sink/trigger/async",
        api_base_url.trim_end_matches('/')
    );
    let body = TriggerRequest {
        event_id: event_id.clone(),
        sink_type,
    };

    tracing::info!(event_id = %event_id, url = %url, "Triggering sink event");

    let client = reqwest::Client::new();
    let result = client
        .post(&url)
        .header("Authorization", format!("Bearer {}", jwt))
        .header("Content-Type", "application/json")
        .json(&body)
        .send()
        .await;

    match result {
        Ok(response) => {
            let status = response.status();
            if status.is_success() {
                tracing::info!(status = %status, "Sink trigger successful");
                ExitCode::SUCCESS
            } else {
                let body = response.text().await.unwrap_or_default();
                tracing::error!(status = %status, body = %body, "Sink trigger failed");
                ExitCode::FAILURE
            }
        }
        Err(e) => {
            tracing::error!(error = %e, "Request failed");
            ExitCode::FAILURE
        }
    }
}
