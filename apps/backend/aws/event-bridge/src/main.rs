#[global_allocator]
static GLOBAL: mimalloc::MiMalloc = mimalloc::MiMalloc;

use aws_lambda_events::cloudwatch_events::CloudWatchEvent;
use flow_like_types::tokio;
use lambda_runtime::{Error, LambdaEvent, run, service_fn, tracing};
use serde::{Deserialize, Serialize};
use std::env;
use std::sync::OnceLock;

static SINK_JWT: OnceLock<String> = OnceLock::new();

fn get_sink_jwt() -> Result<&'static str, Error> {
    if let Some(value) = SINK_JWT.get() {
        return Ok(value.as_str());
    }

    let value = env::var("SINK_JWT").map_err(|_| Error::from("SINK_JWT not set"))?;
    let _ = SINK_JWT.set(value);
    Ok(SINK_JWT
        .get()
        .expect("SINK_JWT value must be initialized")
        .as_str())
}

#[derive(Debug, Deserialize, Serialize)]
struct EventDetail {
    event_id: String,
}

#[derive(Debug, Serialize)]
struct TriggerRequest {
    event_id: String,
    sink_type: String,
}

#[tokio::main]
async fn main() -> Result<(), Error> {
    tracing::init_default_subscriber();
    run(service_fn(event_bridge_handler)).await
}

async fn event_bridge_handler(
    event: LambdaEvent<CloudWatchEvent<EventDetail>>,
) -> Result<(), Error> {
    let api_base_url = env::var("API_BASE_URL").map_err(|_| Error::from("API_BASE_URL not set"))?;

    let sink_jwt = get_sink_jwt()?;

    let detail = event
        .payload
        .detail
        .ok_or_else(|| Error::from("Missing event detail"))?;

    tracing::info!(event_id = %detail.event_id, "Processing scheduled event");

    let client = reqwest::Client::new();
    let trigger_url = format!("{}/api/v1/sink/trigger/async", api_base_url);

    let request_body = TriggerRequest {
        event_id: detail.event_id.clone(),
        sink_type: "cron".to_string(),
    };

    let response = client
        .post(&trigger_url)
        .header("Authorization", format!("Bearer {}", sink_jwt))
        .json(&request_body)
        .send()
        .await
        .map_err(|e| {
            tracing::error!(error = %e, "Failed to send trigger request");
            Error::from(format!("HTTP request failed: {}", e))
        })?;

    if !response.status().is_success() {
        let status = response.status();
        let body = response.text().await.unwrap_or_default();
        tracing::error!(status = %status, body = %body, "API returned error");
        return Err(Error::from(format!("API error: {} - {}", status, body)));
    }

    tracing::info!(event_id = %detail.event_id, "Successfully triggered event");
    Ok(())
}
