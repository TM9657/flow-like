use std::collections::HashMap;
use std::sync::OnceLock;

use axum::{
    Json, Router,
    extract::{Path, State},
    http::StatusCode,
    response::{IntoResponse, Response},
    routing::post,
};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

use crate::state::AppState;

/// OAuth configs loaded at build time (without secrets)
static OAUTH_CONFIG: &str = include_str!(concat!(env!("OUT_DIR"), "/oauth_config.json"));

/// Cached resolved configs with secrets from env
static RESOLVED_CONFIGS: OnceLock<HashMap<String, ResolvedOAuthConfig>> = OnceLock::new();

/// Config as stored in flow-like.config.json (without resolved secrets)
#[derive(Debug, Clone, Deserialize)]
struct OAuthProviderConfig {
    name: String,
    #[serde(default)]
    client_id: Option<String>,
    /// Environment variable name containing the client secret
    client_secret_env: Option<String>,
    auth_url: String,
    token_url: String,
    #[serde(default)]
    scopes: Vec<String>,
    #[serde(default)]
    pkce_required: bool,
    #[serde(default)]
    requires_secret_proxy: bool,
    revoke_url: Option<String>,
    userinfo_url: Option<String>,
    device_auth_url: Option<String>,
    #[serde(default)]
    use_device_flow: bool,
    #[serde(default)]
    use_implicit_flow: bool,
    audience: Option<String>,
}

/// Resolved config with secrets loaded from env at runtime
#[derive(Debug, Clone)]
struct ResolvedOAuthConfig {
    client_id: Option<String>,
    client_secret: Option<String>,
    token_url: String,
    requires_secret_proxy: bool,
}

fn get_oauth_configs() -> &'static HashMap<String, ResolvedOAuthConfig> {
    RESOLVED_CONFIGS.get_or_init(|| {
        let raw_configs: HashMap<String, OAuthProviderConfig> =
            flow_like_types::json::from_str(OAUTH_CONFIG).unwrap_or_default();

        raw_configs
            .into_iter()
            .map(|(provider_id, cfg)| {
                // Resolve client_secret from env var at runtime
                let client_secret = cfg
                    .client_secret_env
                    .as_ref()
                    .and_then(|env_name| std::env::var(env_name).ok())
                    .filter(|s| !s.is_empty());

                let resolved = ResolvedOAuthConfig {
                    client_id: cfg.client_id,
                    client_secret,
                    token_url: cfg.token_url,
                    requires_secret_proxy: cfg.requires_secret_proxy,
                };

                (provider_id, resolved)
            })
            .collect()
    })
}

pub fn routes() -> Router<AppState> {
    Router::new()
        .route("/token/{provider_id}", post(token_exchange))
        .route("/refresh/{provider_id}", post(token_refresh))
}

/// Custom error type for OAuth proxy errors
pub struct OAuthProxyError {
    status: StatusCode,
    message: String,
}

impl OAuthProxyError {
    fn new(status: StatusCode, message: impl Into<String>) -> Self {
        Self {
            status,
            message: message.into(),
        }
    }
}

impl IntoResponse for OAuthProxyError {
    fn into_response(self) -> Response {
        let body = serde_json::json!({
            "error": "oauth_proxy_error",
            "error_description": self.message,
        });
        (self.status, Json(body)).into_response()
    }
}

#[derive(Debug, Deserialize, ToSchema)]
pub struct TokenExchangeRequest {
    /// The authorization code from the OAuth flow
    pub code: String,
    /// The redirect URI used in the authorization request
    pub redirect_uri: String,
    /// The PKCE code verifier (if PKCE was used)
    pub code_verifier: Option<String>,
}

#[derive(Debug, Deserialize, ToSchema)]
pub struct TokenRefreshRequest {
    /// The refresh token
    pub refresh_token: String,
}

#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct TokenResponse {
    pub access_token: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub refresh_token: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub expires_in: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub token_type: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub scope: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub workspace_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub workspace_name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub workspace_icon: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub bot_id: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct ErrorResponse {
    pub error: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error_description: Option<String>,
}

#[utoipa::path(
    post,
    path = "/oauth/token/{provider_id}",
    tag = "oauth",
    params(
        ("provider_id" = String, Path, description = "OAuth provider identifier")
    ),
    request_body = TokenExchangeRequest,
    responses(
        (status = 200, description = "Token exchange successful", body = TokenResponse),
        (status = 400, description = "Bad request", body = ErrorResponse),
        (status = 404, description = "Provider not found", body = ErrorResponse),
        (status = 500, description = "Internal error", body = ErrorResponse)
    )
)]
#[tracing::instrument(name = "POST /oauth/token/:provider_id", skip(_state))]
pub async fn token_exchange(
    State(_state): State<AppState>,
    Path(provider_id): Path<String>,
    Json(request): Json<TokenExchangeRequest>,
) -> Result<Json<TokenResponse>, OAuthProxyError> {
    tracing::info!(
        provider_id = %provider_id,
        has_code = !request.code.is_empty(),
        has_code_verifier = request.code_verifier.is_some(),
        redirect_uri = %request.redirect_uri,
        "OAuth token exchange request received"
    );

    let configs = get_oauth_configs();
    let provider_config = configs.get(&provider_id).ok_or_else(|| {
        tracing::error!(provider_id = %provider_id, "OAuth provider not found in config");
        OAuthProxyError::new(
            StatusCode::NOT_FOUND,
            format!("OAuth provider '{}' not found in config", provider_id),
        )
    })?;

    tracing::info!(
        provider_id = %provider_id,
        requires_secret_proxy = provider_config.requires_secret_proxy,
        has_client_id = provider_config.client_id.is_some(),
        has_client_secret = provider_config.client_secret.is_some(),
        "Provider config loaded"
    );

    // Only proxy for providers that require the secret proxy
    if !provider_config.requires_secret_proxy {
        return Err(OAuthProxyError::new(
            StatusCode::BAD_REQUEST,
            format!(
                "Provider '{}' does not require secret proxy - exchange tokens directly",
                provider_id
            ),
        ));
    }

    let client_id = provider_config.client_id.as_ref().ok_or_else(|| {
        tracing::error!(provider_id = %provider_id, "Client ID not configured");
        OAuthProxyError::new(
            StatusCode::INTERNAL_SERVER_ERROR,
            format!(
                "Client ID not configured for provider '{}' in flow-like.config.json",
                provider_id,
            ),
        )
    })?;

    let client_secret = provider_config.client_secret.as_ref().ok_or_else(|| {
        tracing::error!(provider_id = %provider_id, "Client secret not configured - check GOOGLE_CLIENT_SECRET env var");
        OAuthProxyError::new(
            StatusCode::INTERNAL_SERVER_ERROR,
            format!(
                "Client secret not configured for provider '{}'. Set the environment variable specified in client_secret_env.",
                provider_id,
            ),
        )
    })?;

    // Build the token exchange request
    let mut params = vec![
        ("grant_type", "authorization_code".to_string()),
        ("code", request.code),
        ("redirect_uri", request.redirect_uri),
    ];

    if let Some(code_verifier) = request.code_verifier {
        params.push(("code_verifier", code_verifier));
    }

    tracing::info!(
        provider_id = %provider_id,
        token_url = %provider_config.token_url,
        param_count = params.len(),
        "Sending token exchange request to provider"
    );

    let client = flow_like_types::reqwest::Client::new();

    // Determine auth method based on provider
    // Notion uses HTTP Basic Auth, Atlassian uses client_secret in body
    let response = if provider_id == "notion" {
        // Notion: HTTP Basic Auth with client_id:client_secret
        let credentials = flow_like_types::base64::Engine::encode(
            &flow_like_types::base64::engine::general_purpose::STANDARD,
            format!("{}:{}", client_id, client_secret),
        );

        client
            .post(&provider_config.token_url)
            .header("Authorization", format!("Basic {}", credentials))
            .header("Content-Type", "application/json")
            .json(&serde_json::json!({
                "grant_type": "authorization_code",
                "code": params.iter().find(|(k, _)| *k == "code").map(|(_, v)| v).unwrap(),
                "redirect_uri": params.iter().find(|(k, _)| *k == "redirect_uri").map(|(_, v)| v).unwrap(),
            }))
            .send()
            .await
            .map_err(|e| OAuthProxyError::new(StatusCode::BAD_GATEWAY, e.to_string()))?
    } else {
        // Atlassian and others: client_secret in body
        params.push(("client_id", client_id.clone()));
        params.push(("client_secret", client_secret.clone()));

        tracing::debug!(
            provider_id = %provider_id,
            "Sending form-urlencoded token request with client_id and client_secret"
        );

        client
            .post(&provider_config.token_url)
            .header("Content-Type", "application/x-www-form-urlencoded")
            .header("Accept", "application/json")
            .form(&params)
            .send()
            .await
            .map_err(|e| OAuthProxyError::new(StatusCode::BAD_GATEWAY, e.to_string()))?
    };

    let status = response.status();
    let response_text = response
        .text()
        .await
        .map_err(|e| OAuthProxyError::new(StatusCode::BAD_GATEWAY, e.to_string()))?;

    tracing::info!(
        provider_id = %provider_id,
        status = %status,
        response_len = response_text.len(),
        "Token exchange response received"
    );

    if !status.is_success() {
        tracing::error!(
            provider_id = %provider_id,
            status = %status,
            response = %response_text,
            "Token exchange failed"
        );
        // Try to parse error response
        if let Ok(error_resp) = serde_json::from_str::<ErrorResponse>(&response_text) {
            return Err(OAuthProxyError::new(
                StatusCode::from_u16(status.as_u16()).unwrap_or(StatusCode::BAD_REQUEST),
                format!(
                    "Token exchange failed: {} - {}",
                    error_resp.error,
                    error_resp.error_description.unwrap_or_default()
                ),
            ));
        }
        return Err(OAuthProxyError::new(
            StatusCode::from_u16(status.as_u16()).unwrap_or(StatusCode::BAD_REQUEST),
            format!("Token exchange failed: {}", response_text),
        ));
    }

    let token_response: TokenResponse = serde_json::from_str(&response_text).map_err(|e| {
        OAuthProxyError::new(
            StatusCode::INTERNAL_SERVER_ERROR,
            format!("Failed to parse token response: {} - {}", e, response_text),
        )
    })?;

    Ok(Json(token_response))
}

#[utoipa::path(
    post,
    path = "/oauth/refresh/{provider_id}",
    tag = "oauth",
    params(
        ("provider_id" = String, Path, description = "OAuth provider identifier")
    ),
    request_body = TokenRefreshRequest,
    responses(
        (status = 200, description = "Token refresh successful", body = TokenResponse),
        (status = 400, description = "Bad request", body = ErrorResponse),
        (status = 404, description = "Provider not found", body = ErrorResponse),
        (status = 500, description = "Internal error", body = ErrorResponse)
    )
)]
#[tracing::instrument(name = "POST /oauth/refresh/:provider_id", skip(_state))]
pub async fn token_refresh(
    State(_state): State<AppState>,
    Path(provider_id): Path<String>,
    Json(request): Json<TokenRefreshRequest>,
) -> Result<Json<TokenResponse>, OAuthProxyError> {
    let configs = get_oauth_configs();
    let provider_config = configs.get(&provider_id).ok_or_else(|| {
        OAuthProxyError::new(
            StatusCode::NOT_FOUND,
            format!("OAuth provider '{}' not found in config", provider_id),
        )
    })?;

    if !provider_config.requires_secret_proxy {
        return Err(OAuthProxyError::new(
            StatusCode::BAD_REQUEST,
            format!(
                "Provider '{}' does not require secret proxy - refresh tokens directly",
                provider_id
            ),
        ));
    }

    let client_id = provider_config.client_id.as_ref().ok_or_else(|| {
        OAuthProxyError::new(
            StatusCode::INTERNAL_SERVER_ERROR,
            format!(
                "Client ID not configured for provider '{}' in flow-like.config.json",
                provider_id,
            ),
        )
    })?;

    let client_secret = provider_config.client_secret.as_ref().ok_or_else(|| {
        OAuthProxyError::new(
            StatusCode::INTERNAL_SERVER_ERROR,
            format!(
                "Client secret not configured for provider '{}'. Set the environment variable specified in client_secret_env.",
                provider_id,
            ),
        )
    })?;

    let client = flow_like_types::reqwest::Client::new();

    let response = if provider_id == "notion" {
        // Notion uses HTTP Basic Auth
        let credentials = flow_like_types::base64::Engine::encode(
            &flow_like_types::base64::engine::general_purpose::STANDARD,
            format!("{}:{}", client_id, client_secret),
        );

        client
            .post(&provider_config.token_url)
            .header("Authorization", format!("Basic {}", credentials))
            .header("Content-Type", "application/json")
            .json(&serde_json::json!({
                "grant_type": "refresh_token",
                "refresh_token": request.refresh_token,
            }))
            .send()
            .await
            .map_err(|e| OAuthProxyError::new(StatusCode::BAD_GATEWAY, e.to_string()))?
    } else {
        // Others use form body with client_secret
        let params = vec![
            ("grant_type", "refresh_token"),
            ("refresh_token", &request.refresh_token),
            ("client_id", client_id),
            ("client_secret", client_secret),
        ];

        client
            .post(&provider_config.token_url)
            .header("Content-Type", "application/x-www-form-urlencoded")
            .header("Accept", "application/json")
            .form(&params)
            .send()
            .await
            .map_err(|e| OAuthProxyError::new(StatusCode::BAD_GATEWAY, e.to_string()))?
    };

    let status = response.status();
    let response_text = response
        .text()
        .await
        .map_err(|e| OAuthProxyError::new(StatusCode::BAD_GATEWAY, e.to_string()))?;

    if !status.is_success() {
        if let Ok(error_resp) = serde_json::from_str::<ErrorResponse>(&response_text) {
            return Err(OAuthProxyError::new(
                StatusCode::from_u16(status.as_u16()).unwrap_or(StatusCode::BAD_REQUEST),
                format!(
                    "Token refresh failed: {} - {}",
                    error_resp.error,
                    error_resp.error_description.unwrap_or_default()
                ),
            ));
        }
        return Err(OAuthProxyError::new(
            StatusCode::from_u16(status.as_u16()).unwrap_or(StatusCode::BAD_REQUEST),
            format!("Token refresh failed: {}", response_text),
        ));
    }

    let token_response: TokenResponse = serde_json::from_str(&response_text).map_err(|e| {
        OAuthProxyError::new(
            StatusCode::INTERNAL_SERVER_ERROR,
            format!("Failed to parse token response: {} - {}", e, response_text),
        )
    })?;

    Ok(Json(token_response))
}
