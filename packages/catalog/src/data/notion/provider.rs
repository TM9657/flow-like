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

pub const NOTION_PROVIDER_ID: &str = "notion";

const NOTION_CLIENT_ID: Option<&str> = option_env!("NOTION_CLIENT_ID");
const NOTION_CLIENT_SECRET: Option<&str> = option_env!("NOTION_CLIENT_SECRET");

/// Notion provider - works with both OAuth and Internal Integration tokens
#[derive(Serialize, Deserialize, JsonSchema, Debug, Clone)]
pub struct NotionProvider {
    pub provider_id: String,
    pub access_token: String,
    pub workspace_id: Option<String>,
    pub workspace_name: Option<String>,
}

// =============================================================================
// Internal Integration Provider (API Key)
// =============================================================================

#[crate::register_node]
#[derive(Default)]
pub struct NotionApiKeyProviderNode {}

impl NotionApiKeyProviderNode {
    pub fn new() -> Self {
        Self {}
    }
}

#[async_trait]
impl NodeLogic for NotionApiKeyProviderNode {
    async fn get_node(&self, _app_state: &FlowLikeState) -> Node {
        let mut node = Node::new(
            "data_notion_provider_api_key",
            "Notion (API Key)",
            "Connect to Notion using an Internal Integration token. Create an integration at notion.so/my-integrations and paste the token here.",
            "Data/Notion",
        );
        node.add_icon("/flow/icons/cloud.svg");

        node.add_input_pin(
            "integration_token",
            "Integration Token",
            "Your Notion Internal Integration token (starts with 'secret_'). Get it from notion.so/my-integrations",
            VariableType::String,
        )
        .set_options(PinOptions::new().set_sensitive(true).build());

        node.add_output_pin(
            "provider",
            "Provider",
            "Notion provider with authentication token",
            VariableType::Struct,
        )
        .set_schema::<NotionProvider>()
        .set_options(PinOptions::new().set_enforce_schema(true).build());

        node.set_scores(
            NodeScores::new()
                .set_privacy(6)
                .set_security(7)
                .set_performance(7)
                .set_governance(7)
                .set_reliability(9)
                .set_cost(10)
                .build(),
        );

        node
    }

    async fn run(&self, context: &mut ExecutionContext) -> flow_like_types::Result<()> {
        let token: String = context.evaluate_pin("integration_token").await?;

        if token.is_empty() {
            return Err(flow_like_types::anyhow!(
                "Integration token is required. Get one from notion.so/my-integrations"
            ));
        }

        let provider = NotionProvider {
            provider_id: NOTION_PROVIDER_ID.to_string(),
            access_token: token,
            workspace_id: None,
            workspace_name: None,
        };

        context.set_pin_value("provider", json!(provider)).await?;

        Ok(())
    }
}

// =============================================================================
// OAuth Provider (Public Integration)
// =============================================================================

#[crate::register_node]
#[derive(Default)]
pub struct NotionOAuthProviderNode {}

impl NotionOAuthProviderNode {
    pub fn new() -> Self {
        Self {}
    }
}

#[async_trait]
impl NodeLogic for NotionOAuthProviderNode {
    async fn get_node(&self, _app_state: &FlowLikeState) -> Node {
        let mut node = Node::new(
            "data_notion_provider_oauth",
            "Notion (OAuth)",
            "Connect to Notion using OAuth. Requires NOTION_CLIENT_ID and NOTION_CLIENT_SECRET environment variables to be set at build time.",
            "Data/Notion",
        );
        node.add_icon("/flow/icons/cloud.svg");

        let env_client_id = NOTION_CLIENT_ID.unwrap_or_default();
        let env_client_secret = NOTION_CLIENT_SECRET.unwrap_or_default();

        node.add_output_pin(
            "provider",
            "Provider",
            "Notion provider with authentication token",
            VariableType::Struct,
        )
        .set_schema::<NotionProvider>()
        .set_options(PinOptions::new().set_enforce_schema(true).build());

        // Only add OAuth provider if credentials are available
        if !env_client_id.is_empty() && !env_client_secret.is_empty() {
            let oauth_provider = OAuthProvider::new(NOTION_PROVIDER_ID, "Notion")
                .set_auth_url("https://api.notion.com/v1/oauth/authorize")
                .set_token_url("https://api.notion.com/v1/oauth/token")
                .set_client_id(env_client_id)
                .set_client_secret(env_client_secret)
                .set_scopes(vec![])
                .set_pkce_required(false);

            node.add_oauth_provider(oauth_provider);
        }

        node.set_scores(
            NodeScores::new()
                .set_privacy(6)
                .set_security(8)
                .set_performance(7)
                .set_governance(7)
                .set_reliability(9)
                .set_cost(7)
                .build(),
        );

        node
    }

    async fn run(&self, context: &mut ExecutionContext) -> flow_like_types::Result<()> {
        let env_client_id = NOTION_CLIENT_ID.unwrap_or_default();
        let env_client_secret = NOTION_CLIENT_SECRET.unwrap_or_default();

        if env_client_id.is_empty() || env_client_secret.is_empty() {
            return Err(flow_like_types::anyhow!(
                "Notion OAuth requires NOTION_CLIENT_ID and NOTION_CLIENT_SECRET environment variables. \
                Please set these at build time or use the 'Notion (API Key)' node instead."
            ));
        }

        let token = context
            .get_oauth_token(NOTION_PROVIDER_ID)
            .ok_or_else(|| {
                flow_like_types::anyhow!(
                    "Notion not authenticated. Please authorize access when prompted."
                )
            })?
            .clone();

        let provider = NotionProvider {
            provider_id: NOTION_PROVIDER_ID.to_string(),
            access_token: token.access_token,
            workspace_id: None,
            workspace_name: None,
        };

        context.set_pin_value("provider", json!(provider)).await?;

        Ok(())
    }
}
