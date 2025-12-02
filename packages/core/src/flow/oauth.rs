use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Represents an OAuth/OIDC provider configuration that a node requires.
/// Nodes that need third-party service access declare their OAuth requirements
/// using this structure. Compatible with both OAuth 2.0 and OpenID Connect.
#[derive(Serialize, Deserialize, JsonSchema, Debug, Clone)]
pub struct OAuthProvider {
    /// Unique identifier for this provider (e.g., "google_drive", "github")
    pub id: String,
    /// Display name shown to users (e.g., "Google Drive")
    pub name: String,
    /// OAuth authorization endpoint URL
    pub auth_url: String,
    /// OAuth token endpoint URL
    pub token_url: String,
    /// OAuth client ID (can be empty if provided by environment or user input)
    pub client_id: String,
    /// OAuth client secret (optional, for providers that require it like Notion)
    /// Note: For public clients, leave this empty and use PKCE instead
    pub client_secret: Option<String>,
    /// Required OAuth scopes
    pub scopes: Vec<String>,
    /// Whether PKCE (Proof Key for Code Exchange) is required
    pub pkce_required: bool,
    /// Optional: URL for token revocation
    pub revoke_url: Option<String>,
    /// Optional: URL for user info endpoint (OAuth2/OIDC)
    pub userinfo_url: Option<String>,
    /// Optional: OpenID Connect discovery URL (/.well-known/openid-configuration)
    /// If provided, auth_url, token_url, userinfo_url can be auto-discovered
    pub oidc_discovery_url: Option<String>,
    /// Optional: JWKS URL for validating ID tokens (OIDC)
    pub jwks_url: Option<String>,
    /// Optional: ID token audience claim for validation
    pub audience: Option<String>,
    /// Optional: Device authorization endpoint URL for device flow (RFC 8628)
    /// Used for CLI tools and devices without browsers
    pub device_auth_url: Option<String>,
    /// Whether to use device flow instead of standard authorization code flow
    pub use_device_flow: bool,
}

impl OAuthProvider {
    pub fn new(id: &str, name: &str) -> Self {
        OAuthProvider {
            id: id.to_string(),
            name: name.to_string(),
            auth_url: String::new(),
            token_url: String::new(),
            client_id: String::new(),
            client_secret: None,
            scopes: Vec::new(),
            pkce_required: true,
            revoke_url: None,
            userinfo_url: None,
            oidc_discovery_url: None,
            jwks_url: None,
            audience: None,
            device_auth_url: None,
            use_device_flow: false,
        }
    }

    pub fn set_auth_url(mut self, url: &str) -> Self {
        self.auth_url = url.to_string();
        self
    }

    pub fn set_token_url(mut self, url: &str) -> Self {
        self.token_url = url.to_string();
        self
    }

    pub fn set_client_id(mut self, client_id: &str) -> Self {
        self.client_id = client_id.to_string();
        self
    }

    pub fn set_client_secret(mut self, client_secret: &str) -> Self {
        self.client_secret = Some(client_secret.to_string());
        self
    }

    pub fn add_scope(mut self, scope: &str) -> Self {
        self.scopes.push(scope.to_string());
        self
    }

    pub fn set_scopes(mut self, scopes: Vec<String>) -> Self {
        self.scopes = scopes;
        self
    }

    pub fn set_pkce_required(mut self, required: bool) -> Self {
        self.pkce_required = required;
        self
    }

    pub fn set_revoke_url(mut self, url: &str) -> Self {
        self.revoke_url = Some(url.to_string());
        self
    }

    pub fn set_userinfo_url(mut self, url: &str) -> Self {
        self.userinfo_url = Some(url.to_string());
        self
    }

    pub fn set_oidc_discovery_url(mut self, url: &str) -> Self {
        self.oidc_discovery_url = Some(url.to_string());
        self
    }

    pub fn set_jwks_url(mut self, url: &str) -> Self {
        self.jwks_url = Some(url.to_string());
        self
    }

    pub fn set_audience(mut self, audience: &str) -> Self {
        self.audience = Some(audience.to_string());
        self
    }

    pub fn set_device_auth_url(mut self, url: &str) -> Self {
        self.device_auth_url = Some(url.to_string());
        self
    }

    pub fn set_use_device_flow(mut self, use_device_flow: bool) -> Self {
        self.use_device_flow = use_device_flow;
        self
    }

    pub fn build(self) -> Self {
        self
    }
}

/// Token passed from the frontend after OAuth authentication
#[derive(Serialize, Deserialize, JsonSchema, Debug, Clone)]
pub struct OAuthToken {
    /// The access token for API calls
    pub access_token: String,
    /// Optional refresh token for obtaining new access tokens
    pub refresh_token: Option<String>,
    /// Unix timestamp when the access token expires
    pub expires_at: Option<u64>,
    /// The token type (usually "Bearer")
    pub token_type: Option<String>,
}

impl OAuthToken {
    pub fn new(access_token: String) -> Self {
        OAuthToken {
            access_token,
            refresh_token: None,
            expires_at: None,
            token_type: Some("Bearer".to_string()),
        }
    }

    pub fn is_expired(&self) -> bool {
        if let Some(expires_at) = self.expires_at {
            let now = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.as_secs())
                .unwrap_or(0);
            // Consider expired if less than 60 seconds remaining
            expires_at <= now + 60
        } else {
            false
        }
    }

    pub fn bearer_header(&self) -> String {
        format!(
            "{} {}",
            self.token_type.as_deref().unwrap_or("Bearer"),
            self.access_token
        )
    }
}

/// Collection of OAuth tokens keyed by provider ID
pub type OAuthTokens = HashMap<String, OAuthToken>;
