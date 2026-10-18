//! Bounded HTTP requests for approved, one-off FlowPilot API calls.
//!
//! The host must obtain the tool's approval before calling this engine. This module enforces
//! public destinations and transport limits; it does not decide whether an action is authorized.

use std::{
    future::Future,
    sync::{
        Arc, LazyLock,
        atomic::{AtomicBool, Ordering},
    },
    time::Duration,
};

use flow_like_types::{reqwest, tokio};
use reqwest::{
    Method,
    header::{HeaderMap, HeaderName, HeaderValue},
};
use serde_json::{Value, json};
use url::Url;

use super::public_web::{
    parse_public_url, public_client_builder, read_bounded_body_with_limit, resolve_public_addresses,
};

const TOOL_NAME: &str = "http_request";
const MAX_REQUEST_BODY_BYTES: usize = 1024 * 1024;
const MAX_RESPONSE_BODY_BYTES: usize = 2 * 1024 * 1024;
const MAX_HEADER_BYTES: usize = 16 * 1024;
const MAX_HEADERS: usize = 64;
const DEFAULT_MAX_CHARS: usize = 20_000;
const MAX_OUTPUT_CHARS: usize = 40_000;
const OPERATION_TIMEOUT: Duration = Duration::from_secs(30);
static REQUEST_CONCURRENCY: LazyLock<tokio::sync::Semaphore> =
    LazyLock::new(|| tokio::sync::Semaphore::new(4));

#[derive(Debug)]
struct HttpRequest {
    url: Url,
    method: Method,
    headers: HeaderMap,
    body: Option<Vec<u8>>,
    max_chars: usize,
}

#[derive(Debug)]
struct RequestFailure {
    code: &'static str,
    message: &'static str,
    http_status: Option<u16>,
}

impl RequestFailure {
    fn new(code: &'static str, message: &'static str) -> Self {
        Self {
            code,
            message,
            http_status: None,
        }
    }

    fn with_status(mut self, http_status: u16) -> Self {
        self.http_status = Some(http_status);
        self
    }

    fn as_json(&self, may_have_been_applied: bool) -> Value {
        // Error messages never contain URLs, request headers, body text, or reqwest errors, which
        // may include credentials. A failed response read does not prove a write failed.
        json!({
            "status": "error",
            "tool": TOOL_NAME,
            "code": self.code,
            "error": self.message,
            "http_status": self.http_status,
            "retryable": false,
            "request_may_have_been_applied": may_have_been_applied,
            "recovery_hint": if may_have_been_applied {
                "The request may have reached the API. Check its state before sending the action again."
            } else {
                "Inspect the request and error before deciding whether to try again."
            },
        })
    }
}

/// Execute an already authorized request. No redirects or automatic retries are performed.
pub async fn run_http_request(args: &Value) -> Value {
    run_http_request_cancellable(args, std::future::pending()).await
}

/// Execute with the owning run's live cancellation signal, including queue and DNS waits.
pub async fn run_http_request_cancellable(
    args: &Value,
    cancellation: impl Future<Output = ()>,
) -> Value {
    let request = match parse_request(args) {
        Ok(request) => request,
        Err(error) => return error.as_json(false),
    };
    let mutating = !matches!(request.method, Method::GET | Method::HEAD | Method::OPTIONS);
    let sent = Arc::new(AtomicBool::new(false));
    let operation = tokio::time::timeout(OPERATION_TIMEOUT, async {
        let _permit = REQUEST_CONCURRENCY.acquire().await.map_err(|_| {
            RequestFailure::new(
                "concurrency_unavailable",
                "The HTTP request service is unavailable.",
            )
        })?;
        let addresses = resolve_public_addresses(&request.url)
            .await
            .map_err(|error| {
                RequestFailure::new(
                    error.code,
                    "The HTTP destination could not be resolved to public Internet addresses.",
                )
            })?;
        let client = public_client_builder(&request.url, &addresses)
            .retry(reqwest::retry::never())
            .build()
            .map_err(|_| {
                RequestFailure::new(
                    "client_unavailable",
                    "The HTTP client could not be created.",
                )
            })?;
        send_request(request, client, &sent).await
    });
    let outcome = tokio::select! {
        biased;
        _ = cancellation => {
            let mut error = RequestFailure::new("cancelled", "The owning FlowPilot run was cancelled.")
                .as_json(mutating && sent.load(Ordering::Acquire));
            error["status"] = json!("cancelled");
            return error;
        }
        outcome = operation => outcome,
    };
    let may_have_been_applied = mutating && sent.load(Ordering::Acquire);
    match outcome {
        Ok(Ok(result)) => result,
        Ok(Err(error)) => error.as_json(may_have_been_applied),
        Err(_) => RequestFailure::new(
            "timeout",
            "The HTTP request exceeded its 30 second deadline.",
        )
        .as_json(may_have_been_applied),
    }
}

fn parse_request(args: &Value) -> Result<HttpRequest, RequestFailure> {
    let raw_url = args.get("url").and_then(Value::as_str).unwrap_or_default();
    let url = parse_public_url(raw_url).map_err(|error| {
        RequestFailure::new(error.code, "Use a public HTTP(S) URL on its default port, without URL credentials or secret query parameters. Put API credentials in HTTPS request headers.")
    })?;
    let raw_method = match args.get("method") {
        None => "GET",
        Some(Value::String(method)) => method.trim(),
        _ => {
            return Err(RequestFailure::new(
                "invalid_method",
                "method must be an HTTP method string.",
            ));
        }
    };
    let method = match raw_method.to_ascii_uppercase().as_str() {
        "GET" => Method::GET,
        "HEAD" => Method::HEAD,
        "POST" => Method::POST,
        "PUT" => Method::PUT,
        "PATCH" => Method::PATCH,
        "DELETE" => Method::DELETE,
        "OPTIONS" => Method::OPTIONS,
        _ => {
            return Err(RequestFailure::new(
                "invalid_method",
                "Supported methods are GET, HEAD, POST, PUT, PATCH, DELETE, and OPTIONS.",
            ));
        }
    };
    let max_chars = match args.get("max_chars") {
        None => DEFAULT_MAX_CHARS,
        Some(value) => value
            .as_u64()
            .filter(|value| (1000..=MAX_OUTPUT_CHARS as u64).contains(value))
            .map(|value| value as usize)
            .ok_or_else(|| {
                RequestFailure::new(
                    "invalid_max_chars",
                    "max_chars must be an integer from 1000 to 40000.",
                )
            })?,
    };
    let mut headers = parse_headers(args.get("headers"), &url)?;
    let body = match (args.get("body"), args.get("json")) {
        (Some(_), Some(_)) => {
            return Err(RequestFailure::new(
                "conflicting_body",
                "Supply either body or json, not both.",
            ));
        }
        (Some(Value::String(body)), None) => {
            if body.len() > MAX_REQUEST_BODY_BYTES {
                return Err(RequestFailure::new(
                    "body_too_large",
                    "The request body exceeds the 1 MiB limit.",
                ));
            }
            Some(body.as_bytes().to_vec())
        }
        (Some(_), None) => {
            return Err(RequestFailure::new(
                "invalid_body",
                "body must be a string; use json for structured content.",
            ));
        }
        (None, Some(value)) => {
            headers
                .entry(reqwest::header::CONTENT_TYPE)
                .or_insert(HeaderValue::from_static("application/json"));
            Some(serde_json::to_vec(value).map_err(|_| {
                RequestFailure::new("invalid_json", "The JSON body could not be serialized.")
            })?)
        }
        (None, None) => None,
    };
    if body
        .as_ref()
        .is_some_and(|body| body.len() > MAX_REQUEST_BODY_BYTES)
    {
        return Err(RequestFailure::new(
            "body_too_large",
            "The request body exceeds the 1 MiB limit.",
        ));
    }
    if body.is_some() && matches!(method, Method::GET | Method::HEAD) {
        return Err(RequestFailure::new(
            "body_not_allowed",
            "GET and HEAD requests cannot include a body.",
        ));
    }
    Ok(HttpRequest {
        url,
        method,
        headers,
        body,
        max_chars,
    })
}

fn parse_headers(value: Option<&Value>, url: &Url) -> Result<HeaderMap, RequestFailure> {
    let mut headers = HeaderMap::new();
    let Some(value) = value else {
        return Ok(headers);
    };
    let object = value.as_object().ok_or_else(|| {
        RequestFailure::new(
            "invalid_headers",
            "headers must be an object of string values.",
        )
    })?;
    if object.len() > MAX_HEADERS {
        return Err(RequestFailure::new(
            "headers_too_large",
            "At most 64 request headers are allowed.",
        ));
    }
    let mut size = 0;
    for (name, value) in object {
        let value = value.as_str().ok_or_else(|| {
            RequestFailure::new(
                "invalid_headers",
                "Every request header value must be a string.",
            )
        })?;
        size += name.len() + value.len();
        if size > MAX_HEADER_BYTES {
            return Err(RequestFailure::new(
                "headers_too_large",
                "Request headers exceed the 16 KiB limit.",
            ));
        }
        let name = HeaderName::from_bytes(name.as_bytes()).map_err(|_| {
            RequestFailure::new("invalid_header_name", "A request header name is invalid.")
        })?;
        if forbidden_request_header(name.as_str()) {
            return Err(RequestFailure::new(
                "transport_header_not_allowed",
                "Transport, proxy, forwarding, and compression headers are managed by the HTTP client.",
            ));
        }
        if url.scheme() != "https" && sensitive_header(name.as_str()) {
            return Err(RequestFailure::new(
                "credentials_require_https",
                "Credential and cookie headers require an HTTPS destination.",
            ));
        }
        let value = HeaderValue::from_str(value).map_err(|_| {
            RequestFailure::new("invalid_header_value", "A request header value is invalid.")
        })?;
        if headers.insert(name, value).is_some() {
            return Err(RequestFailure::new(
                "duplicate_header",
                "Header names must be unique regardless of capitalization.",
            ));
        }
    }
    Ok(headers)
}

fn forbidden_request_header(name: &str) -> bool {
    matches!(
        name,
        "host"
            | "connection"
            | "content-length"
            | "transfer-encoding"
            | "te"
            | "trailer"
            | "upgrade"
            | "expect"
            | "accept-encoding"
            | "forwarded"
            | "via"
    ) || name.starts_with("proxy-")
        || name.starts_with("x-forwarded-")
}

fn sensitive_header(name: &str) -> bool {
    matches!(
        name,
        "authorization"
            | "cookie"
            | "cookie2"
            | "set-cookie"
            | "set-cookie2"
            | "authentication-info"
            | "proxy-authentication-info"
            | "proxy-authorization"
            | "proxy-authenticate"
            | "www-authenticate"
    ) || name.contains("api-key")
        || name.contains("apikey")
        || name.contains("token")
        || name.contains("secret")
        || name.contains("credential")
}

async fn send_request(
    request: HttpRequest,
    client: reqwest::Client,
    sent: &AtomicBool,
) -> Result<Value, RequestFailure> {
    let requested_url = request.url.to_string();
    let mut builder = client
        .request(request.method.clone(), request.url)
        .headers(request.headers)
        .header(reqwest::header::ACCEPT_ENCODING, "identity");
    if let Some(body) = request.body {
        builder = builder.body(body);
    }
    sent.store(true, Ordering::Release);
    let response = builder.send().await.map_err(|error| {
        if error.is_timeout() {
            RequestFailure::new("request_timeout", "The HTTP request timed out.")
        } else {
            RequestFailure::new("request_failed", "The HTTP request could not complete.")
        }
    })?;
    let status = response.status().as_u16();
    let (headers, headers_truncated) = response_headers(response.headers());
    let content_type = response
        .headers()
        .get(reqwest::header::CONTENT_TYPE)
        .and_then(|value| value.to_str().ok())
        .unwrap_or_default()
        .to_string();
    if response
        .headers()
        .get(reqwest::header::CONTENT_ENCODING)
        .and_then(|value| value.to_str().ok())
        .is_some_and(|encoding| !encoding.is_empty() && !encoding.eq_ignore_ascii_case("identity"))
    {
        return Err(RequestFailure::new(
            "unsupported_content_encoding",
            "The API returned compressed content despite the identity encoding request.",
        )
        .with_status(status));
    }
    let (body, body_truncated) = read_bounded_body_with_limit(response, MAX_RESPONSE_BODY_BYTES)
        .await
        .map_err(|_| {
            RequestFailure::new(
                "body_read_failed",
                "The API responded, but its response body could not be read.",
            )
            .with_status(status)
        })?;
    let text = decode_response_text(&body, &content_type, body_truncated)
        .map_err(|error| error.with_status(status))?;
    let mut chars = text.chars();
    let content: String = chars.by_ref().take(request.max_chars).collect();
    let truncated = body_truncated || chars.next().is_some();
    let parsed_json = (!truncated)
        .then(|| serde_json::from_str::<Value>(&content).ok())
        .flatten();
    Ok(json!({
        "status": "ok",
        "tool": TOOL_NAME,
        "method": request.method.as_str(),
        "requested_url": requested_url,
        "url": requested_url,
        "fetched_at": chrono::Utc::now().to_rfc3339(),
        "http_status": status,
        "http_ok": (200..300).contains(&status),
        "headers": headers,
        "headers_truncated": headers_truncated,
        "content_type": content_type,
        "body": content,
        "json": parsed_json,
        "bytes_read": body.len(),
        "truncated": truncated,
        "redirect_followed": false,
        "untrusted_content": true,
    }))
}

fn response_headers(source: &HeaderMap) -> (serde_json::Map<String, Value>, bool) {
    let mut headers = serde_json::Map::new();
    let mut bytes = 0;
    let mut truncated = false;
    for (name, value) in source {
        if sensitive_header(name.as_str()) {
            continue;
        }
        let Ok(value) = value.to_str() else {
            continue;
        };
        // Redirects are not followed, and signed or credential-bearing destinations are not
        // copied into the model's tool output.
        if name == reqwest::header::LOCATION && parse_public_url(value).is_err() {
            continue;
        }
        bytes += name.as_str().len() + value.len();
        if headers.len() >= MAX_HEADERS || bytes > MAX_HEADER_BYTES {
            truncated = true;
            break;
        }
        headers.insert(name.as_str().to_string(), json!(value));
    }
    (headers, truncated)
}

fn decode_response_text(
    body: &[u8],
    content_type: &str,
    body_truncated: bool,
) -> Result<String, RequestFailure> {
    if body.is_empty() {
        return Ok(String::new());
    }
    let mime = content_type
        .split(';')
        .next()
        .unwrap_or_default()
        .trim()
        .to_ascii_lowercase();
    if !mime.is_empty()
        && !(mime.starts_with("text/")
            || mime == "application/json"
            || mime.ends_with("+json")
            || mime == "application/xml"
            || mime.ends_with("+xml")
            || mime == "application/javascript"
            || mime == "application/x-www-form-urlencoded")
    {
        return Err(RequestFailure::new(
            "unsupported_content_type",
            "The API response is not textual content.",
        ));
    }
    let text = match std::str::from_utf8(body) {
        Ok(text) => text,
        Err(error) if body_truncated && error.error_len().is_none() => {
            // A byte budget may cut only the final UTF-8 codepoint. Keep its valid prefix;
            // malformed bytes elsewhere still fail rather than changing the API's data.
            std::str::from_utf8(&body[..error.valid_up_to()]).expect("validated UTF-8 prefix")
        }
        Err(_) => {
            return Err(RequestFailure::new(
                "unsupported_text_encoding",
                "The API response is not valid UTF-8 text.",
            ));
        }
    };
    if text.contains('\0') {
        return Err(RequestFailure::new(
            "binary_content",
            "Binary response content cannot be returned as text.",
        ));
    }
    Ok(text
        .chars()
        .filter(|character| !character.is_control() || matches!(character, '\n' | '\r' | '\t'))
        .collect())
}

#[cfg(test)]
mod tests {
    use super::*;
    use tokio::io::{AsyncReadExt, AsyncWriteExt};

    #[test]
    fn request_policy_accepts_api_payloads_and_https_credentials() {
        let parsed = parse_request(&json!({"url":"https://api.example.com/weather?latitude=52", "method":"post", "headers":{"Authorization":"Bearer test-secret", "Accept":"application/json"}, "json":{"current":true}})).unwrap();
        assert_eq!(parsed.method, Method::POST);
        assert_eq!(
            parsed.headers[reqwest::header::CONTENT_TYPE],
            "application/json"
        );
        assert_eq!(
            parsed.headers[reqwest::header::AUTHORIZATION],
            "Bearer test-secret"
        );
        assert_eq!(
            serde_json::from_slice::<Value>(parsed.body.as_ref().unwrap()).unwrap(),
            json!({"current":true})
        );
        let parsed = parse_request(&json!({"url":"https://example.com", "method":"PATCH", "headers":{"Content-Type":"application/custom"}, "body":"payload"})).unwrap();
        assert_eq!(
            parsed.headers[reqwest::header::CONTENT_TYPE],
            "application/custom"
        );
        assert_eq!(parsed.body.unwrap(), b"payload");
    }

    #[test]
    fn request_policy_rejects_unsafe_destinations_and_transport_overrides() {
        for url in [
            "http://127.0.0.1/",
            "http://[::1]/",
            "http://metadata.google.internal/",
            "http://example.com:8080/",
            "https://user:secret@example.com/",
            "https://example.com/?api_key=secret",
        ] {
            assert!(
                parse_request(&json!({"url":url})).is_err(),
                "accepted {url}"
            );
        }
        for name in [
            "Host",
            "Connection",
            "Content-Length",
            "Transfer-Encoding",
            "TE",
            "Trailer",
            "Upgrade",
            "Proxy-Authorization",
            "X-Forwarded-For",
            "Accept-Encoding",
        ] {
            assert_eq!(
                parse_request(&json!({"url":"https://example.com", "headers":{name:"value"}}))
                    .unwrap_err()
                    .code,
                "transport_header_not_allowed"
            );
        }
        assert_eq!(
            parse_request(
                &json!({"url":"http://example.com", "headers":{"Authorization":"Bearer secret"}})
            )
            .unwrap_err()
            .code,
            "credentials_require_https"
        );
        assert_eq!(parse_request(&json!({"url":"https://example.com", "headers":{"x-test":"value\r\nInjected: true"}})).unwrap_err().code, "invalid_header_value");
        assert_eq!(
            parse_request(
                &json!({"url":"https://example.com", "headers":{"X-Test":"a", "x-test":"b"}})
            )
            .unwrap_err()
            .code,
            "duplicate_header"
        );
    }

    #[test]
    fn request_policy_bounds_and_validates_payloads() {
        for args in [
            json!({"method":"CONNECT"}),
            json!({"method":1}),
            json!({"headers":[]}),
            json!({"headers":{"x-test":1}}),
            json!({"method":"POST","body":{},"json":{}}),
            json!({"method":"POST","body":1}),
            json!({"method":"GET","body":"x"}),
            json!({"max_chars":0}),
            json!({"max_chars":40001}),
        ] {
            let mut args = args;
            args["url"] = json!("https://example.com");
            assert!(parse_request(&args).is_err(), "accepted {args}");
        }
        assert_eq!(parse_request(&json!({"url":"https://example.com", "method":"POST", "body":"x".repeat(MAX_REQUEST_BODY_BYTES + 1)})).unwrap_err().code, "body_too_large");
        assert_eq!(parse_request(&json!({"url":"https://example.com", "method":"POST", "json":{"payload":"x".repeat(MAX_REQUEST_BODY_BYTES + 1)}})).unwrap_err().code, "body_too_large");
        assert_eq!(parse_request(&json!({"url":"https://example.com", "headers":{"x-test":"x".repeat(MAX_HEADER_BYTES)}})).unwrap_err().code, "headers_too_large");
    }

    #[test]
    fn byte_truncation_preserves_valid_utf8_without_accepting_malformed_content() {
        assert_eq!(
            decode_response_text(b"forecast \xe2\x98", "text/plain", true).unwrap(),
            "forecast "
        );
        assert!(decode_response_text(b"forecast \xe2\x98", "text/plain", false).is_err());
        assert!(decode_response_text(b"forecast \xff broken", "text/plain", true).is_err());
        assert_eq!(
            decode_response_text(&[], "application/octet-stream", false).unwrap(),
            ""
        );
        assert!(decode_response_text(b"text", "application/octet-stream", false).is_err());
    }

    #[tokio::test]
    async fn already_cancelled_requests_never_start_network_io() {
        let result = run_http_request_cancellable(
            &json!({
                "url":"https://example.com/api", "method":"POST", "json":{"action":"apply"}
            }),
            std::future::ready(()),
        )
        .await;
        assert_eq!(result["status"], "cancelled");
        assert_eq!(result["request_may_have_been_applied"], false);
        assert_eq!(result["retryable"], false);
    }

    async fn mock_response(args: Value, response: &str) -> (Value, String) {
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let address = listener.local_addr().unwrap();
        let response = response.to_string();
        let server = tokio::spawn(async move {
            let (mut socket, _) = listener.accept().await.unwrap();
            let mut request = Vec::new();
            loop {
                let mut buffer = [0; 4096];
                let count = socket.read(&mut buffer).await.unwrap();
                if count == 0 {
                    break;
                }
                request.extend_from_slice(&buffer[..count]);
                if let Some(header_end) = request.windows(4).position(|part| part == b"\r\n\r\n") {
                    let headers =
                        String::from_utf8_lossy(&request[..header_end]).to_ascii_lowercase();
                    let length = headers
                        .lines()
                        .find_map(|line| line.strip_prefix("content-length: "))
                        .and_then(|value| value.parse::<usize>().ok())
                        .unwrap_or(0);
                    if request.len() >= header_end + 4 + length {
                        break;
                    }
                }
            }
            socket.write_all(response.as_bytes()).await.unwrap();
            String::from_utf8(request).unwrap()
        });
        let mut request = parse_request(&args).unwrap();
        // The local fixture bypasses only destination validation; the public entry point always
        // validates and pins DNS before this transport seam.
        request.url = Url::parse(&format!("http://{address}/endpoint")).unwrap();
        let client = public_client_builder(&request.url, &[address])
            .retry(reqwest::retry::never())
            .build()
            .unwrap();
        let value = send_request(request, client, &AtomicBool::new(false))
            .await
            .unwrap();
        (value, server.await.unwrap())
    }

    #[tokio::test]
    async fn transport_preserves_method_body_errors_and_filters_response_credentials() {
        let (output, request) = mock_response(json!({"url":"https://api.example.com", "method":"POST", "headers":{"X-Custom":"test"}, "json":{"city":"Berlin"}}), "HTTP/1.1 422 Unprocessable Entity\r\nContent-Type: application/json\r\nContent-Length: 19\r\nSet-Cookie: secret=1\r\nX-Request-Id: request-1\r\nConnection: close\r\n\r\n{\"error\":\"invalid\"}").await;
        assert!(request.starts_with("POST /endpoint HTTP/1.1\r\n"));
        assert!(request.to_ascii_lowercase().contains("x-custom: test\r\n"));
        assert!(request.ends_with("{\"city\":\"Berlin\"}"));
        assert_eq!(output["status"], "ok");
        assert_eq!(output["http_status"], 422);
        assert_eq!(output["http_ok"], false);
        assert_eq!(output["json"]["error"], "invalid");
        assert!(output["headers"].get("set-cookie").is_none());
        assert_eq!(output["headers"]["x-request-id"], "request-1");
    }

    #[tokio::test]
    async fn transport_returns_empty_and_redirect_responses_without_following() {
        let (empty, _) = mock_response(
            json!({"url":"https://example.com", "method":"DELETE"}),
            "HTTP/1.1 204 No Content\r\nConnection: close\r\n\r\n",
        )
        .await;
        assert_eq!(empty["http_status"], 204);
        assert_eq!(empty["body"], "");
        let (redirect, _) = mock_response(json!({"url":"https://example.com"}), "HTTP/1.1 302 Found\r\nLocation: http://127.0.0.1/private\r\nContent-Length: 0\r\nConnection: close\r\n\r\n").await;
        assert_eq!(redirect["http_status"], 302);
        assert_eq!(redirect["redirect_followed"], false);
        assert!(redirect["headers"].get("location").is_none());
    }

    #[tokio::test]
    async fn transport_bounds_output_and_keeps_errors_free_of_request_secrets() {
        let response = format!(
            "HTTP/1.1 200 OK\r\nContent-Type: text/plain\r\nContent-Length: 1100\r\nConnection: close\r\n\r\n{}",
            "x".repeat(1100)
        );
        let (output, _) = mock_response(
            json!({"url":"https://example.com", "max_chars":1000}),
            &response,
        )
        .await;
        assert_eq!(output["body"].as_str().unwrap().len(), 1000);
        assert_eq!(output["truncated"], true);
        let error = run_http_request(&json!({"url":"https://user:secret@example.com", "method":"POST", "body":"hidden-payload"})).await;
        assert!(!error.to_string().contains("hidden-payload"));
        assert!(!error.to_string().contains("user:secret"));
        assert_eq!(error["request_may_have_been_applied"], false);
        let uncertain =
            RequestFailure::new("request_timeout", "The HTTP request timed out.").as_json(true);
        assert_eq!(uncertain["retryable"], false);
        assert_eq!(uncertain["request_may_have_been_applied"], true);
    }
}
