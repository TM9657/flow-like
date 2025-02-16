use anyhow::anyhow;
use axum::body::Body;
use axum::extract::State;
use axum::middleware::Next;
use axum::response::{IntoResponse, Redirect};
use axum::routing::post;
use axum::{http::Request, routing::get, Router};
use hyper::Uri;

use crate::error::AppError;
use crate::state::AppState;

pub fn routes() -> Router<AppState> {
    Router::new()
        .route("/discovery", get(discovery))
        .route("/jwks", get(jwks))
        .route("/authorize", get(proxy_authorize).post(proxy_authorize))
        .route("/token", post(proxy_token))
        .route("/userinfo", get(proxy_userinfo).post(proxy_userinfo))
        .route("/revoke", get(proxy_revoke).post(proxy_revoke))
}

#[tracing::instrument]
async fn discovery(State(state): State<AppState>) -> Redirect {
    Redirect::temporary(
        &state
            .platform_config
            .authentication
            .openid
            .as_ref()
            .unwrap()
            .discovery_url,
    )
}

#[tracing::instrument]
async fn jwks(State(state): State<AppState>) -> Redirect {
    Redirect::temporary(
        &state
            .platform_config
            .authentication
            .openid
            .as_ref()
            .unwrap()
            .jwks_url,
    )
}

#[tracing::instrument]
async fn proxy_authorize(
    State(state): State<AppState>,
    req: Request<Body>,
) -> Result<impl IntoResponse, AppError> {
    proxy_request(state, req, "authorize").await
}

#[tracing::instrument]
async fn proxy_token(
    State(state): State<AppState>,
    req: Request<Body>,
) -> Result<impl IntoResponse, AppError> {
    proxy_request(state, req, "token").await
}

#[tracing::instrument]
async fn proxy_userinfo(
    State(state): State<AppState>,
    req: Request<Body>,
) -> Result<impl IntoResponse, AppError> {
    proxy_request(state, req, "userinfo").await
}

#[tracing::instrument]
async fn proxy_revoke(
    State(state): State<AppState>,
    req: Request<Body>,
) -> Result<impl IntoResponse, AppError> {
    proxy_request(state, req, "revoke").await
}

#[tracing::instrument]
async fn proxy_request(
    state: AppState,
    mut req: Request<Body>,
    endpoint: &str,
) -> Result<impl IntoResponse, AppError> {
    let client = state.client.clone();

    let openid_config = state
        .platform_config
        .authentication
        .openid
        .as_ref()
        .ok_or(anyhow!("OpenID Configuration Error"))?;

    let url = match endpoint {
        "authorize" => openid_config.proxy.authorize.clone(),
        "token" => openid_config.proxy.token.clone(),
        "userinfo" => openid_config.proxy.userinfo.clone(),
        "revoke" => openid_config.proxy.revoke.clone(),
        _ => return Err(AppError::from(anyhow!("Invalid endpoint"))),
    }
    .ok_or(anyhow!("Invalid endpoint"))?;

    *req.uri_mut() = Uri::try_from(&url).unwrap();

    Ok(client
        .request(req)
        .await
        .map_err(|_| anyhow!("Bad Request"))?
        .into_response())
}

pub async fn auth_middleware(req: Request<Body>, next: Next) -> impl IntoResponse {
    // Implement your authorization logic here
    // For example, check for a valid token in the headers
    if let Some(auth_header) = req.headers().get("Authorization") {
        if auth_header == "Bearer valid_token" {
            return next.run(req).await;
        }
    }

    // If authorization fails, return a 401 Unauthorized response
    axum::response::Response::builder()
        .status(axum::http::StatusCode::UNAUTHORIZED)
        .body(axum::body::Body::from("Unauthorized"))
        .unwrap()
}
