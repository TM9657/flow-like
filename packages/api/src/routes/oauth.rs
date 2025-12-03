use std::collections::HashMap;
use std::sync::OnceLock;

use axum::{
    Json,
    extract::{Path, State},
    http::StatusCode,
    response::{IntoResponse, Response},
    routing::post,
    Router,
};
use serde::{Deserialize, Serialize};

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
    audience: Option<String>,
}

/// Resolved config with secrets loaded from env at runtime
#[derive(Debug, Clone)]
struct ResolvedOAuthConfig {
    name: String,
    client_id: Option<String>,
    client_secret: Option<String>,
    auth_url: String,
    token_url: String,
    #[allow(dead_code)]
    scopes: Vec<String>,
    #[allow(dead_code)]
    pkce_required: bool,
    requires_secret_proxy: bool,
    #[allow(dead_code)]
    revoke_url: Option<String>,
    #[allow(dead_code)]
    userinfo_url: Option<String>,
    #[allow(dead_code)]
    device_auth_url: Option<String>,
    #[allow(dead_code)]
    use_device_flow: bool,
    #[allow(dead_code)]
    audience: Option<String>,
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
                    name: cfg.name,
                    client_id: cfg.client_id,
                    client_secret,
                    auth_url: cfg.auth_url,
                    token_url: cfg.token_url,
                    scopes: cfg.scopes,
                    pkce_required: cfg.pkce_required,
                    requires_secret_proxy: cfg.requires_secret_proxy,
                    revoke_url: cfg.revoke_url,
                    userinfo_url: cfg.userinfo_url,
                    device_auth_url: cfg.device_auth_url,
                    use_device_flow: cfg.use_device_flow,
                    audience: cfg.audience,
                };

                (provider_id, resolved)
            })
            .collect()
    })
}

pub fn routes() -> Router<AppState> {
    Router::new()
        .route("/token/:provider_id", post(proxy_token_exchange))
        .route("/refresh/:provider_id", post(proxy_token_refresh))
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

#[derive(Debug, Deserialize)]
pub struct TokenExchangeRequest {
    /// The authorization code from the OAuth flow
    pub code: String,
    /// The redirect URI used in the authorization request
    pub redirect_uri: String,
    /// The PKCE code verifier (if PKCE was used)
    pub code_verifier: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct TokenRefreshRequest {
    /// The refresh token
    pub refresh_token: String,
}

#[derive(Debug, Serialize, Deserialize)]
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
    // Notion-specific fields
    #[serde(skip_serializing_if = "Option::is_none")]
    pub workspace_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub workspace_name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub workspace_icon: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub bot_id: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ErrorResponse {
    pub error: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error_description: Option<String>,
}

/// Proxy endpoint for OAuth token exchange
/// This adds the client_secret to the request for providers that require it
#[tracing::instrument(name = "POST /oauth/token/:provider_id", skip(state))]
async fn proxy_token_exchange(
    State(state): State<AppState>,
    Path(provider_id): Path<String>,
    Json(request): Json<TokenExchangeRequest>,
) -> Result<Json<TokenResponse>, OAuthProxyError> {
    let configs = get_oauth_configs();
    let provider_config = configs.get(&provider_id).ok_or_else(|| {
        OAuthProxyError::new(
            StatusCode::NOT_FOUND,
            format!("OAuth provider '{}' not found in config", provider_id),
        )
    })?;

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

    // Build the token exchange request
    let mut params = vec![
        ("grant_type", "authorization_code".to_string()),
        ("code", request.code),
        ("redirect_uri", request.redirect_uri),
    ];

    if let Some(code_verifier) = request.code_verifier {
        params.push(("code_verifier", code_verifier));
    }

    let client = flow_like_types::reqwest::Client::new();

    // Determine auth method based on provider
    // Notion uses HTTP Basic Auth, Atlassian uses client_secret in body
    let response = if provider_id == "notion" {
        // Notion: HTTP Basic Auth with client_id:client_secret
        let credentials =
            flow_like_types::base64::Engine::encode(
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

/// Proxy endpoint for OAuth token refresh
/// This adds the client_secret to the request for providers that require it
#[tracing::instrument(name = "POST /oauth/refresh/:provider_id", skip(state))]
async fn proxy_token_refresh(
    State(state): State<AppState>,
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
        let credentials =
            flow_like_types::base64::Engine::encode(
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
