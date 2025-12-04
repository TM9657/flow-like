use flow_like::{
    flow::{
        execution::context::ExecutionContext,
        node::{Node, NodeLogic, NodeScores},
        pin::PinOptions,
        variable::VariableType,
    },
    state::FlowLikeState,
};
use flow_like_types::{JsonSchema, async_trait, json::json};
use serde::{Deserialize, Serialize};

pub const ATLASSIAN_PROVIDER_ID: &str = "atlassian";

/// Atlassian provider - works with both OAuth and API Token authentication
/// Supports both Jira and Confluence APIs
#[derive(Serialize, Deserialize, JsonSchema, Debug, Clone)]
pub struct AtlassianProvider {
    /// The provider ID
    pub provider_id: String,
    /// The access token (OAuth, API token, or PAT)
    pub access_token: String,
    /// The base URL for the Atlassian instance
    /// For cloud: https://your-domain.atlassian.net
    /// For server/data center: https://your-server.com
    pub base_url: String,
    /// Email for Cloud API token auth (not needed for OAuth or Server PAT)
    pub email: Option<String>,
    /// Whether this is a cloud instance
    pub is_cloud: bool,
    /// Authentication type: "oauth", "api_token" (cloud), or "pat" (server/dc)
    pub auth_type: String,
}

impl AtlassianProvider {
    pub fn jira_api_url(&self, path: &str) -> String {
        let base = self.base_url.trim_end_matches('/');
        let api_path = if self.is_cloud {
            "/rest/api/3"
        } else {
            "/rest/api/2"
        };
        if path.starts_with('/') {
            format!("{}{}{}", base, api_path, path)
        } else {
            format!("{}{}/{}", base, api_path, path)
        }
    }

    pub fn confluence_api_url(&self, path: &str) -> String {
        let base = self.base_url.trim_end_matches('/');
        let api_path = if self.is_cloud {
            "/wiki/api/v2"
        } else {
            "/wiki/rest/api"
        };
        if path.starts_with('/') {
            format!("{}{}{}", base, api_path, path)
        } else {
            format!("{}{}/{}", base, api_path, path)
        }
    }

    pub fn auth_header(&self) -> String {
        use flow_like_types::base64::Engine;
        match self.auth_type.as_str() {
            "oauth" => {
                // OAuth uses Bearer token
                format!("Bearer {}", self.access_token)
            }
            "pat" => {
                // Server/Data Center PAT uses Bearer token
                format!("Bearer {}", self.access_token)
            }
            "api_token" | _ => {
                // Cloud API Token auth uses Basic auth with email:token
                if let Some(email) = &self.email {
                    let credentials = format!("{}:{}", email, self.access_token);
                    format!(
                        "Basic {}",
                        flow_like_types::base64::engine::general_purpose::STANDARD
                            .encode(credentials.as_bytes())
                    )
                } else {
                    // Fallback to Bearer if no email (shouldn't happen for api_token)
                    format!("Bearer {}", self.access_token)
                }
            }
        }
    }
}

// =============================================================================
// API Token Provider (Personal Access Token)
// =============================================================================

#[crate::register_node]
#[derive(Default)]
pub struct AtlassianApiTokenProviderNode {}

impl AtlassianApiTokenProviderNode {
    pub fn new() -> Self {
        Self {}
    }
}

#[async_trait]
impl NodeLogic for AtlassianApiTokenProviderNode {
    async fn get_node(&self, _app_state: &FlowLikeState) -> Node {
        let mut node = Node::new(
            "data_atlassian_provider_api_token",
            "Atlassian (API Token)",
            "Connect to Jira and Confluence using an API Token. For cloud: create token at id.atlassian.com/manage-profile/security/api-tokens. For server: use personal access token.",
            "Data/Atlassian",
        );
        node.add_icon("/flow/icons/atlassian.svg");

        node.add_input_pin(
            "base_url",
            "Base URL",
            "Your Atlassian instance URL. Cloud: https://your-domain.atlassian.net, Server: https://your-server.com",
            VariableType::String,
        )
        .set_default_value(Some(json!("https://your-domain.atlassian.net")));

        node.add_input_pin(
            "email",
            "Email",
            "Your Atlassian account email (required for cloud API tokens, optional for server PAT)",
            VariableType::String,
        );

        node.add_input_pin(
            "api_token",
            "API Token",
            "Your API token or Personal Access Token",
            VariableType::String,
        )
        .set_options(PinOptions::new().set_sensitive(true).build());

        node.add_input_pin(
            "is_cloud",
            "Is Cloud",
            "Whether this is an Atlassian Cloud instance (affects API version)",
            VariableType::Boolean,
        )
        .set_default_value(Some(json!(true)));

        node.add_output_pin(
            "provider",
            "Provider",
            "Atlassian provider for Jira and Confluence APIs",
            VariableType::Struct,
        )
        .set_schema::<AtlassianProvider>()
        .set_options(PinOptions::new().set_enforce_schema(true).build());

        node.set_scores(
            NodeScores::new()
                .set_privacy(6)
                .set_security(7)
                .set_performance(8)
                .set_governance(7)
                .set_reliability(9)
                .set_cost(10)
                .build(),
        );

        node
    }

    async fn run(&self, context: &mut ExecutionContext) -> flow_like_types::Result<()> {
        let base_url: String = context.evaluate_pin("base_url").await?;
        let email: String = context.evaluate_pin("email").await?;
        let api_token: String = context.evaluate_pin("api_token").await?;
        let is_cloud: bool = context.evaluate_pin("is_cloud").await?;

        if api_token.is_empty() {
            return Err(flow_like_types::anyhow!(
                "API Token is required. For cloud: create at id.atlassian.com/manage-profile/security/api-tokens"
            ));
        }

        if base_url.is_empty() || base_url == "https://your-domain.atlassian.net" {
            return Err(flow_like_types::anyhow!(
                "Please provide your Atlassian instance URL (e.g., https://your-domain.atlassian.net)"
            ));
        }

        // For cloud API tokens, email is required
        let email_opt = if email.is_empty() {
            if is_cloud {
                return Err(flow_like_types::anyhow!(
                    "Email is required for Atlassian Cloud API token authentication"
                ));
            }
            None
        } else {
            Some(email)
        };

        let auth_type = if is_cloud { "api_token" } else { "pat" };

        let provider = AtlassianProvider {
            provider_id: ATLASSIAN_PROVIDER_ID.to_string(),
            access_token: api_token,
            base_url,
            email: email_opt,
            is_cloud,
            auth_type: auth_type.to_string(),
        };

        context.set_pin_value("provider", json!(provider)).await?;

        Ok(())
    }
}

// =============================================================================
// OAuth Provider
// =============================================================================

#[crate::register_node]
#[derive(Default)]
pub struct AtlassianOAuthProviderNode {}

impl AtlassianOAuthProviderNode {
    pub fn new() -> Self {
        Self {}
    }
}

#[async_trait]
impl NodeLogic for AtlassianOAuthProviderNode {
    async fn get_node(&self, _app_state: &FlowLikeState) -> Node {
        let mut node = Node::new(
            "data_atlassian_provider_oauth",
            "Atlassian (OAuth)",
            "Connect to Jira and Confluence using OAuth 2.0. Requires OAuth provider configuration in flow-like.config.json.",
            "Data/Atlassian",
        );
        node.add_icon("/flow/icons/atlassian.svg");

        node.add_input_pin(
            "base_url",
            "Base URL",
            "Your Atlassian Cloud instance URL (e.g., https://your-domain.atlassian.net)",
            VariableType::String,
        )
        .set_default_value(Some(json!("https://your-domain.atlassian.net")));

        node.add_output_pin(
            "provider",
            "Provider",
            "Atlassian provider for Jira and Confluence APIs",
            VariableType::Struct,
        )
        .set_schema::<AtlassianProvider>()
        .set_options(PinOptions::new().set_enforce_schema(true).build());

        // Add OAuth provider reference - full config comes from Hub
        node.add_oauth_provider(ATLASSIAN_PROVIDER_ID);
        node.add_required_oauth_scopes(
            ATLASSIAN_PROVIDER_ID,
            vec![
                "read:jira-work",
                "read:jira-user",
                "read:confluence-content.all",
                "read:confluence-user",
                "offline_access",
            ],
        );

        node.set_scores(
            NodeScores::new()
                .set_privacy(6)
                .set_security(8)
                .set_performance(7)
                .set_governance(7)
                .set_reliability(8)
                .set_cost(7)
                .build(),
        );

        node
    }

    async fn run(&self, context: &mut ExecutionContext) -> flow_like_types::Result<()> {
        let base_url: String = context.evaluate_pin("base_url").await?;

        if base_url.is_empty() || base_url == "https://your-domain.atlassian.net" {
            return Err(flow_like_types::anyhow!(
                "Please provide your Atlassian instance URL (e.g., https://your-domain.atlassian.net)"
            ));
        }

        let token = context
            .get_oauth_token(ATLASSIAN_PROVIDER_ID)
            .ok_or_else(|| {
                flow_like_types::anyhow!(
                    "Atlassian not authenticated. Please authorize access when prompted."
                )
            })?
            .clone();

        let provider = AtlassianProvider {
            provider_id: ATLASSIAN_PROVIDER_ID.to_string(),
            access_token: token.access_token,
            base_url,
            email: None,
            is_cloud: true, // OAuth is only for cloud
            auth_type: "oauth".to_string(),
        };

        context.set_pin_value("provider", json!(provider)).await?;

        Ok(())
    }
}
