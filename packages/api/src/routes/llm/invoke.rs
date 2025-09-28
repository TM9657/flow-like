use crate::entity::llm_usage_tracking;
use crate::{entity::bit, error::ApiError, middleware::jwt::AppUser, state::AppState};
use axum::{
    Extension, Json,
    body::Body,
    extract::State,
    http::{HeaderMap, HeaderValue},
    response::Response as AxumResponse,
};
use flow_like::bit::Bit;
use flow_like_types::Bytes;
use flow_like_types::create_id;
use flow_like_types::{anyhow, json::json};
use futures_util::StreamExt;
use sea_orm::EntityTrait;
use sea_orm::{ActiveModelTrait, Set};
use serde_json::Value as JsonValue;
use std::convert::Infallible;

// We now always return a plain AxumResponse (JSON for non-stream, raw event stream for stream=true)

// --- helpers ---
async fn fetch_provider(
    state: &AppState,
    model_field: &str,
) -> Result<flow_like::flow_like_model_provider::provider::ModelProvider, ApiError> {
    let bit_model = bit::Entity::find_by_id(model_field)
        .one(&state.db)
        .await?
        .ok_or_else(|| anyhow!("Bit not found"))?;
    let bit_model = Bit::from(bit_model);
    let provider = bit_model
        .try_to_provider()
        .ok_or_else(|| anyhow!("Bit is not a model provider"))?;
    if provider.provider_name != "Hosted" {
        return Err(ApiError::BadRequest(
            "Only 'Hosted' models are supported via this endpoint".into(),
        ));
    }
    Ok(provider)
}

async fn enforce_tier(
    user: &AppUser,
    state: &AppState,
    provider: &flow_like::flow_like_model_provider::provider::ModelProvider,
) -> Result<(), ApiError> {
    let user_tier: flow_like::hub::UserTier = user.tier(state).await?;
    let params = provider.params.clone().unwrap_or_default();
    let tier = params
        .get("tier")
        .and_then(|v| v.as_str())
        .unwrap_or("ENTERPRISE");
    if !user_tier.llm_tiers.iter().any(|t| t == tier) {
        tracing::warn!(
            "User tier {:?} does not allow access to model tier {}",
            user_tier,
            tier
        );
        return Err(ApiError::Forbidden);
    }
    Ok(())
}

fn prepare_upstream_body(
    payload: &serde_json::Value,
    upstream_model_id: &str,
    tracking_user: Option<&str>,
) -> (serde_json::Value, bool) {
    let mut body = payload.clone();
    if let Some(obj) = body.as_object_mut() {
        obj.insert("model".to_string(), json!(upstream_model_id));
        // Ensure usage.include=true for OpenRouter usage tokens & cost reporting
        let usage = obj.entry("usage").or_insert_with(|| json!({}));
        if usage.is_object() {
            usage
                .as_object_mut()
                .unwrap()
                .insert("include".to_string(), json!(true));
        }
        // OpenRouter supports a top-level `user` field (same as OpenAI) for end-user identification / abuse monitoring.
        if let Some(u) = tracking_user {
            obj.insert("user".to_string(), json!(u));
        }
    }
    let stream = body
        .get("stream")
        .and_then(|v| v.as_bool())
        .unwrap_or(false);
    (body, stream)
}

fn build_openai_url() -> Result<(String, String), ApiError> {
    let endpoint = std::env::var("OPENAI_ENDPOINT").unwrap_or_default();
    if endpoint.is_empty() {
        return Err(anyhow!("OPENAI_ENDPOINT not configured").into());
    }
    let api_key = std::env::var("OPENAI_API_KEY").unwrap_or_default();
    if api_key.is_empty() {
        return Err(anyhow!("OPENAI_API_KEY not configured").into());
    }
    let url = format!("{}/v1/chat/completions", endpoint.trim_end_matches('/'));
    Ok((url, api_key))
}

// Accumulator for streaming usage/cost extraction
#[derive(Default, Debug)]
struct StreamingAccum {
    in_tok: Option<i64>,
    out_tok: Option<i64>,
    cost_micro: Option<i64>,
}

fn extract_usage_and_cost_from_json(
    v: &serde_json::Value,
) -> Option<(Option<i64>, Option<i64>, Option<i64>)> {
    let usage = v.get("usage")?;
    let in_tok = usage
        .get("prompt_tokens")
        .or_else(|| usage.get("input_tokens"))
        .and_then(|v| v.as_i64());
    let out_tok = usage
        .get("completion_tokens")
        .or_else(|| usage.get("output_tokens"))
        .and_then(|v| v.as_i64());
    let cost_micro = usage
        .get("cost")
        .or_else(|| usage.get("total_cost"))
        .and_then(|c| c.as_f64())
        .map(|f| (f * 1_000_000.0) as i64);
    if in_tok.is_some() || out_tok.is_some() || cost_micro.is_some() {
        Some((in_tok, out_tok, cost_micro))
    } else {
        None
    }
}

fn parse_sse_bytes(accum: &std::sync::Arc<std::sync::Mutex<StreamingAccum>>, bytes: &Bytes) {
    if let Ok(text) = std::str::from_utf8(bytes) {
        for line in text.split('\n') {
            let line = line.trim();
            if !line.starts_with("data: ") {
                continue;
            }
            let data = &line[6..];
            if data == "[DONE]" {
                continue;
            }
            if let Ok(json) = serde_json::from_str::<serde_json::Value>(data) {
                if let Some((in_tok, out_tok, cost_micro)) = extract_usage_and_cost_from_json(&json)
                {
                    if in_tok.is_some() || out_tok.is_some() || cost_micro.is_some() {
                        let mut a = accum.lock().unwrap();
                        a.in_tok = in_tok.or(a.in_tok);
                        a.out_tok = out_tok.or(a.out_tok);
                        a.cost_micro = cost_micro.or(a.cost_micro);
                    }
                }
            }
        }
    }
}

use futures_util::Stream;
use futures_util::task::{Context, Poll};
use pin_project_lite::pin_project;
use std::pin::Pin;

pin_project! {
    struct TrackingStream<S> {
        #[pin]
        inner: S,
        accum: std::sync::Arc<std::sync::Mutex<StreamingAccum>>,
        finalized: bool,
        state: AppState,
        user: String,
        model: String,
    }
}

impl<S> Stream for TrackingStream<S>
where
    S: Stream<Item = Result<Bytes, flow_like_types::reqwest::Error>>,
{
    type Item = Result<Bytes, Infallible>;
    fn poll_next(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        let mut this = self.project();
        match this.inner.as_mut().poll_next(cx) {
            Poll::Ready(Some(Ok(chunk_bytes))) => {
                parse_sse_bytes(this.accum, &chunk_bytes);
                Poll::Ready(Some(Ok(chunk_bytes)))
            }
            Poll::Ready(Some(Err(e))) => {
                tracing::error!(error=%e, "Error reading upstream stream");
                Poll::Ready(Some(Ok(Bytes::from_static(b""))))
            }
            Poll::Ready(None) => {
                if !*this.finalized {
                    *this.finalized = true;
                    let acc = this.accum.clone();
                    let state_c = this.state.clone();
                    let user_c = this.user.clone();
                    let model_c = this.model.clone();
                    flow_like_types::tokio::spawn(async move {
                        let (in_tok, out_tok, cost_micro) = {
                            let a = acc.lock().unwrap();
                            (a.in_tok, a.out_tok, a.cost_micro)
                        };
                        if let (Some(in_t), Some(out_t)) = (in_tok, out_tok) {
                            let price = cost_micro.unwrap_or(0);
                            if let Err(e) =
                                track_llm_usage(&state_c, &user_c, &model_c, in_t, out_t, price)
                                    .await
                            {
                                tracing::warn!(error=%e, "Failed to track streaming LLM usage");
                            }
                        }
                    });
                }
                Poll::Ready(None)
            }
            Poll::Pending => Poll::Pending,
        }
    }
}

async fn handle_streaming(
    request_builder: flow_like_types::reqwest::RequestBuilder,
    state: AppState,
    user_sub: String,
    model_id: String,
) -> Result<AxumResponse, ApiError> {
    let resp = request_builder.send().await.map_err(|e| {
        tracing::error!(error=%e, "Upstream streaming request failed");
        anyhow!("Upstream request failed: {e}")
    })?;
    if !resp.status().is_success() {
        let status = resp.status();
        let text = resp.text().await.unwrap_or_default();
        tracing::error!(status=%status, body=%text, "Upstream error");
        return Err(ApiError::BadRequest("Upstream error".into()));
    }

    let mut builder = AxumResponse::builder().status(resp.status());
    if let Some(ct) = resp.headers().get(axum::http::header::CONTENT_TYPE) {
        builder = builder.header(axum::http::header::CONTENT_TYPE, ct);
    } else {
        builder = builder.header(axum::http::header::CONTENT_TYPE, "text/event-stream");
    }

    // Wrap upstream stream so we can observe chunks while leaving bytes unchanged.
    let upstream = resp.bytes_stream();
    let tracking_stream = TrackingStream {
        inner: upstream,
        accum: std::sync::Arc::new(std::sync::Mutex::new(StreamingAccum::default())),
        finalized: false,
        state,
        user: user_sub,
        model: model_id,
    };
    let body_stream = tracking_stream.map(|res| res.map(|b| b));
    let body = passthrough_byte_stream(body_stream);
    Ok(builder.body(body).unwrap())
}

async fn handle_non_streaming(
    request_builder: flow_like_types::reqwest::RequestBuilder,
    upstream_model_id: &str,
    state: &AppState,
    user_sub: &str,
) -> Result<AxumResponse, ApiError> {
    let resp = request_builder.send().await.map_err(|e| {
        tracing::error!(error=%e, "Upstream request failed");
        anyhow!("Upstream request failed: {e}")
    })?;
    let status = resp.status();
    let headers = resp.headers().clone();
    let body_bytes = resp.bytes().await.map_err(|e| {
        tracing::error!(error=%e, "Failed to read upstream body");
        anyhow!("Failed to read upstream body: {e}")
    })?;
    if status.is_success() {
        tracing::info!(model = %upstream_model_id, bytes = body_bytes.len(), "LLM invoke success (non-stream)");
        if let Some((in_tok, out_tok)) = extract_usage_from_body(&body_bytes) {
            // Placeholder cost calculation (0). Replace with proper pricing logic later.
            if let Err(e) =
                track_llm_usage(state, user_sub, upstream_model_id, in_tok, out_tok, 0).await
            {
                tracing::warn!(error=%e, "Failed to track LLM usage");
            }
        }
    } else {
        tracing::warn!(status = %status, body = %String::from_utf8_lossy(&body_bytes), "LLM invoke upstream error");
    }
    let mut out_headers = HeaderMap::new();
    if let Some(ct) = headers.get(axum::http::header::CONTENT_TYPE) {
        out_headers.insert(axum::http::header::CONTENT_TYPE, ct.clone());
    } else {
        out_headers.insert(
            axum::http::header::CONTENT_TYPE,
            HeaderValue::from_static("application/json"),
        );
    }
    let response = AxumResponse::builder()
        .status(status)
        .body(Body::from(body_bytes))
        .unwrap();
    Ok(response)
}

#[tracing::instrument(name = "POST /llm", skip(state, user, payload))]
pub async fn invoke_llm(
    State(state): State<AppState>,
    Extension(user): Extension<AppUser>,
    Json(payload): Json<serde_json::Value>,
) -> Result<AxumResponse, ApiError> {
    user.sub()?;
    let model_field = payload
        .get("model")
        .and_then(|v| v.as_str())
        .ok_or_else(|| ApiError::BadRequest("Missing 'model' field".into()))?;
    let provider = fetch_provider(&state, model_field).await?;
    enforce_tier(&user, &state, &provider).await?;
    let upstream_model_id = provider
        .model_id
        .clone()
        .unwrap_or_else(|| model_field.to_string());
    let tracking_id_opt = user.tracking_id(&state).await.ok().flatten();
    let (upstream_body, stream) =
        prepare_upstream_body(&payload, &upstream_model_id, tracking_id_opt.as_deref());
    let (url, api_key) = build_openai_url()?;
    let client = flow_like_types::reqwest::Client::new();
    let mut request_builder = client.post(&url).bearer_auth(api_key).json(&upstream_body);
    // Inject OpenRouter recommended headers for attribution & user tracking
    request_builder = request_builder
        .header("HTTP-Referer", "https://flow-like.com")
        .header("X-Title", "Flow-Like");
    if let Some(tracking_id) = &tracking_id_opt {
        // Pass header variant too (OpenRouter extra_headers style)
        request_builder = request_builder.header("X-User-Id", tracking_id);
    }
    let user_sub = user.sub()?;
    if stream {
        handle_streaming(request_builder, state, user_sub, upstream_model_id).await
    } else {
        handle_non_streaming(request_builder, &upstream_model_id, &state, &user_sub).await
    }
}

// -------- Cost Tracking --------
fn extract_usage_from_body(body: &[u8]) -> Option<(i64, i64)> {
    if let Ok(v) = serde_json::from_slice::<JsonValue>(body) {
        // Try common OpenAI style usage fields
        if let Some(usage) = v.get("usage") {
            let in_tok = usage
                .get("prompt_tokens")
                .or_else(|| usage.get("input_tokens"))?
                .as_i64()?;
            let out_tok = usage
                .get("completion_tokens")
                .or_else(|| usage.get("output_tokens"))?
                .as_i64()?;
            return Some((in_tok, out_tok));
        }
    }
    None
}

async fn track_llm_usage(
    state: &AppState,
    user_sub: &str,
    model: &str,
    token_in: i64,
    token_out: i64,
    price: i64,
) -> Result<(), flow_like_types::Error> {
    use chrono::Utc;
    use llm_usage_tracking::ActiveModel;
    let now = Utc::now().naive_utc();
    let record = ActiveModel {
        id: Set(create_id()),
        model_id: Set(model.to_string()),
        token_in: Set(token_in),
        token_out: Set(token_out),
        latency: Set(None),
        user_id: Set(Some(user_sub.to_string())),
        app_id: Set(None),
        price: Set(price),
        created_at: Set(now),
        updated_at: Set(now),
    };
    // Best-effort insert
    match record.insert(&state.db).await {
        Ok(_) => Ok(()),
        Err(e) => Err(e.into()),
    }
}

// Turn a stream of Bytes into a Body verbatim.
fn passthrough_byte_stream<S>(s: S) -> Body
where
    S: futures_util::Stream<Item = Result<Bytes, Infallible>> + Send + 'static,
{
    Body::from_stream(s)
}

// -------- Tests --------
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_prepare_upstream_body_rewrites_model() {
        let payload = serde_json::json!({"model": "bit_123", "messages": [], "stream": false});
        let (rewritten, stream) = prepare_upstream_body(&payload, "gpt-4o-mini", Some("user_123"));
        assert!(!stream);
        assert_eq!(
            rewritten.get("model").unwrap().as_str().unwrap(),
            "gpt-4o-mini"
        );
        assert_eq!(rewritten.get("user").unwrap().as_str().unwrap(), "user_123");
        assert_eq!(
            rewritten
                .get("usage")
                .unwrap()
                .get("include")
                .unwrap()
                .as_bool(),
            Some(true)
        );
    }

    #[test]
    fn test_extract_usage_from_body() {
        let body = serde_json::json!({
            "id": "chatcmpl-test",
            "usage": {"prompt_tokens": 12, "completion_tokens": 34, "total_tokens": 46}
        });
        let bytes = serde_json::to_vec(&body).unwrap();
        let usage = extract_usage_from_body(&bytes).unwrap();
        assert_eq!(usage, (12, 34));
    }
}
