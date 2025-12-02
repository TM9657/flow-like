use flow_like::{
    flow::{
        execution::context::ExecutionContext,
        node::{Node, NodeLogic, NodeScores},
        oauth::OAuthProvider,
        pin::PinOptions,
        variable::VariableType,
    },
    state::FlowLikeState,
};
use flow_like_types::{JsonSchema, async_trait, json::json};
use serde::{Deserialize, Serialize};

pub const MICROSOFT_PROVIDER_ID: &str = "microsoft";

const MS_CLIENT_ID: Option<&str> = option_env!("MS_CLIENT_ID");

/// Microsoft Graph provider - works with OAuth or access tokens
#[derive(Serialize, Deserialize, JsonSchema, Debug, Clone)]
pub struct MicrosoftGraphProvider {
    pub provider_id: String,
    pub access_token: String,
    pub base_url: String,
}

impl MicrosoftGraphProvider {
    pub fn api_url(&self, path: &str) -> String {
        let base = self.base_url.trim_end_matches('/');
        if path.starts_with('/') {
            format!("{}{}", base, path)
        } else {
            format!("{}/{}", base, path)
        }
    }
}

// =============================================================================
// Access Token Provider (for server-to-server or manual token)
// =============================================================================

#[crate::register_node]
#[derive(Default)]
pub struct MicrosoftGraphTokenProviderNode {}

impl MicrosoftGraphTokenProviderNode {
    pub fn new() -> Self {
        Self {}
    }
}

#[async_trait]
impl NodeLogic for MicrosoftGraphTokenProviderNode {
    async fn get_node(&self, _app_state: &FlowLikeState) -> Node {
        let mut node = Node::new(
            "data_microsoft_provider_token",
            "Microsoft Graph (Token)",
            "Connect to Microsoft Graph API using an access token. Use for server-to-server auth or manual token management.",
            "Data/Microsoft",
        );
        node.add_icon("/flow/icons/microsoft.svg");

        node.add_input_pin(
            "token",
            "Access Token",
            "Microsoft Graph API access token",
            VariableType::String,
        )
        .set_options(PinOptions::new().set_sensitive(true).build());

        node.add_input_pin(
            "base_url",
            "API Base URL",
            "Microsoft Graph API base URL",
            VariableType::String,
        )
        .set_default_value(Some(json!("https://graph.microsoft.com/v1.0")));

        node.add_output_pin(
            "provider",
            "Provider",
            "Microsoft Graph provider with authentication",
            VariableType::Struct,
        )
        .set_schema::<MicrosoftGraphProvider>()
        .set_options(PinOptions::new().set_enforce_schema(true).build());

        node.set_scores(
            NodeScores::new()
                .set_privacy(6)
                .set_security(7)
                .set_performance(9)
                .set_governance(6)
                .set_reliability(9)
                .set_cost(10)
                .build(),
        );

        node
    }

    async fn run(&self, context: &mut ExecutionContext) -> flow_like_types::Result<()> {
        let token: String = context.evaluate_pin("token").await?;
        let base_url: String = context
            .evaluate_pin("base_url")
            .await
            .unwrap_or_else(|_| "https://graph.microsoft.com/v1.0".to_string());

        if token.is_empty() {
            return Err(flow_like_types::anyhow!("Access token is required"));
        }

        let provider = MicrosoftGraphProvider {
            provider_id: MICROSOFT_PROVIDER_ID.to_string(),
            access_token: token,
            base_url,
        };

        context.set_pin_value("provider", json!(provider)).await?;

        Ok(())
    }
}

// =============================================================================
// OAuth Provider (Authorization Code Flow with PKCE)
// =============================================================================

#[crate::register_node]
#[derive(Default)]
pub struct MicrosoftGraphOAuthProviderNode {}

impl MicrosoftGraphOAuthProviderNode {
    pub fn new() -> Self {
        Self {}
    }
}

#[async_trait]
impl NodeLogic for MicrosoftGraphOAuthProviderNode {
    async fn get_node(&self, _app_state: &FlowLikeState) -> Node {
        let mut node = Node::new(
            "data_microsoft_provider_oauth",
            "Microsoft Graph (OAuth)",
            "Connect to Microsoft Graph using OAuth Authorization Code Flow with PKCE. Requires MS_CLIENT_ID environment variable.",
            "Data/Microsoft",
        );
        node.add_icon("/flow/icons/microsoft.svg");

        let env_client_id = MS_CLIENT_ID.unwrap_or_default();

        node.add_input_pin(
            "base_url",
            "API Base URL",
            "Microsoft Graph API base URL",
            VariableType::String,
        )
        .set_default_value(Some(json!("https://graph.microsoft.com/v1.0")));

        node.add_output_pin(
            "provider",
            "Provider",
            "Microsoft Graph provider with authentication",
            VariableType::Struct,
        )
        .set_schema::<MicrosoftGraphProvider>()
        .set_options(PinOptions::new().set_enforce_schema(true).build());

        // Microsoft Identity Platform with Authorization Code Flow + PKCE
        // Base scopes only - individual nodes add their required scopes
        let oauth_provider = OAuthProvider::new(MICROSOFT_PROVIDER_ID, "Microsoft")
            .set_auth_url("https://login.microsoftonline.com/common/oauth2/v2.0/authorize")
            .set_token_url("https://login.microsoftonline.com/common/oauth2/v2.0/token")
            .set_client_id(env_client_id)
            .set_scopes(vec!["User.Read".to_string(), "offline_access".to_string()])
            .set_pkce_required(true)
            .set_userinfo_url("https://graph.microsoft.com/v1.0/me")
            .set_revoke_url("https://login.microsoftonline.com/common/oauth2/v2.0/logout");

        node.add_oauth_provider(oauth_provider);

        node.set_scores(
            NodeScores::new()
                .set_privacy(6)
                .set_security(9) // PKCE is more secure
                .set_performance(8)
                .set_governance(7)
                .set_reliability(9)
                .set_cost(7)
                .build(),
        );

        node
    }

    async fn run(&self, context: &mut ExecutionContext) -> flow_like_types::Result<()> {
        let env_client_id = MS_CLIENT_ID.unwrap_or_default();

        if env_client_id.is_empty() {
            return Err(flow_like_types::anyhow!(
                "Microsoft OAuth requires MS_CLIENT_ID environment variable. \
                Please set it at build time or use the 'Microsoft Graph (Token)' node instead."
            ));
        }

        let base_url: String = context
            .evaluate_pin("base_url")
            .await
            .unwrap_or_else(|_| "https://graph.microsoft.com/v1.0".to_string());

        let token = context
            .get_oauth_token(MICROSOFT_PROVIDER_ID)
            .ok_or_else(|| {
                flow_like_types::anyhow!(
                    "Microsoft not authenticated. Please authorize access when prompted."
                )
            })?
            .clone();

        let provider = MicrosoftGraphProvider {
            provider_id: MICROSOFT_PROVIDER_ID.to_string(),
            access_token: token.access_token,
            base_url,
        };

        context.set_pin_value("provider", json!(provider)).await?;

        Ok(())
    }
}
