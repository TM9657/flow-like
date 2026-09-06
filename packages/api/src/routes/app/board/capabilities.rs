//! Each request declares the newest board format its client understands.
//! Core board access enforces that limit when loading or saving, including
//! mutations that raise the required version while a storage lease is held.

use crate::error::ApiError;
use axum::{Json, extract::Request, http::HeaderMap, middleware::Next, response::Response};
use flow_like::flow::board::format::{
    CURRENT_BOARD_FORMAT_VERSION, LEGACY_BOARD_FORMAT_VERSION, with_supported_version,
};
use serde_json::{Value, json};

pub const HEADER: &str = "x-flow-like-board-format";

/// Missing headers belong to legacy clients. A malformed declaration cannot
/// establish which board formats a client can safely read or write.
pub fn supported_version(headers: &HeaderMap) -> Result<u32, ApiError> {
    let mut values = headers.get_all(HEADER).iter();
    let Some(value) = values.next() else {
        return Ok(LEGACY_BOARD_FORMAT_VERSION);
    };
    let invalid = || ApiError::bad_request(format!("{HEADER} must be one positive integer"));
    if values.next().is_some() {
        return Err(invalid());
    }
    let value = value.to_str().map_err(|_| invalid())?.trim();
    if value.is_empty() || !value.bytes().all(|byte| byte.is_ascii_digit()) {
        return Err(invalid());
    }
    let version = value.parse::<u32>().map_err(|_| invalid())?;
    if version == 0 {
        return Err(invalid());
    }
    Ok(version)
}

pub async fn capabilities() -> Json<Value> {
    Json(json!({"board_format_version": CURRENT_BOARD_FORMAT_VERSION}))
}

/// The scope lasts for this request future and never changes a process-wide
/// limit. Board operations also enforce the runtime's own supported version.
pub async fn negotiate_board_format(request: Request, next: Next) -> Result<Response, ApiError> {
    let supported = supported_version(request.headers())?;
    Ok(with_supported_version(supported, next.run(request)).await)
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::{Router, body::Body, http::StatusCode, routing::get};
    use flow_like::flow::{
        board::Board,
        pin::ValueType,
        variable::{Variable, VariableType},
    };
    use tower::ServiceExt;

    #[test]
    fn missing_header_means_legacy_and_newer_clients_can_negotiate() {
        let mut headers = HeaderMap::new();
        assert_eq!(supported_version(&headers).unwrap(), 1);
        for version in [1, 2, 3, u32::MAX] {
            headers.insert(HEADER, version.to_string().parse().unwrap());
            assert_eq!(supported_version(&headers).unwrap(), version);
        }
    }

    #[test]
    fn malformed_and_duplicate_headers_are_rejected() {
        let mut headers = HeaderMap::new();
        for value in [
            "",
            "0",
            "-1",
            "+2",
            "2.0",
            "1,2",
            "geometry-v1",
            "4294967296",
        ] {
            headers.insert(HEADER, value.parse().unwrap());
            assert_eq!(
                supported_version(&headers).unwrap_err().status(),
                StatusCode::BAD_REQUEST
            );
        }
        headers.insert(HEADER, "2".parse().unwrap());
        headers.append(HEADER, "2".parse().unwrap());
        assert!(supported_version(&headers).is_err());
    }

    #[tokio::test]
    async fn server_advertises_its_format_without_a_deployment_flag() {
        let Json(response) = capabilities().await;
        assert_eq!(response, json!({"board_format_version": 2}));
    }

    fn geometry_board() -> Board {
        let mut board =
            Board::new_detached(Some("board".into()), flow_like_storage::Path::default());
        let variable = Variable::new("location", VariableType::Geometry, ValueType::Normal);
        board.variables.insert(variable.id.clone(), variable);
        board
    }

    #[tokio::test]
    async fn middleware_isolates_concurrent_clients_and_returns_generic_upgrade_errors() {
        async fn read_geometry() -> Result<Json<Board>, ApiError> {
            // Yield inside the request to exercise task-local scope across polls.
            flow_like_types::tokio::task::yield_now().await;
            let board = geometry_board();
            board.ensure_supported_format()?;
            Ok(Json(board))
        }
        let app = Router::new()
            .route("/", get(read_geometry))
            .route_layer(axum::middleware::from_fn(negotiate_board_format));
        let legacy = Request::builder().uri("/").body(Body::empty()).unwrap();
        let current = Request::builder()
            .uri("/")
            .header(HEADER, "2")
            .body(Body::empty())
            .unwrap();
        let (legacy, current) = tokio::join!(app.clone().oneshot(legacy), app.oneshot(current));
        let legacy = legacy.unwrap();
        assert_eq!(legacy.status(), StatusCode::UPGRADE_REQUIRED);
        let body = axum::body::to_bytes(legacy.into_body(), 4096)
            .await
            .unwrap();
        let body: Value = serde_json::from_slice(&body).unwrap();
        assert_eq!(body["error"]["code"], "BOARD_FORMAT_UPGRADE_REQUIRED");
        assert_eq!(current.unwrap().status(), StatusCode::OK);
        geometry_board().ensure_supported_format().unwrap();
    }

    #[test]
    fn contextual_runtime_errors_keep_the_upgrade_status() {
        let error = flow_like_types::Error::new(flow_like::flow::board::format::BoardFormatError {
            required: 3,
            supported: 2,
        })
        .context("loading a board snapshot");
        let error = ApiError::from(error);
        assert_eq!(error.status(), StatusCode::UPGRADE_REQUIRED);
        assert_eq!(error.public_code(), "BOARD_FORMAT_UPGRADE_REQUIRED");
        assert!(error.public_message().unwrap().contains("version 3"));
    }
}
