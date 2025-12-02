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

pub const GITHUB_PROVIDER_ID: &str = "github";

const GH_CLIENT_ID: Option<&str> = option_env!("GH_CLIENT_ID");

/// GitHub provider - works with OAuth, PAT, or GitHub App tokens
#[derive(Serialize, Deserialize, JsonSchema, Debug, Clone)]
pub struct GitHubProvider {
    pub provider_id: String,
    pub access_token: String,
    pub base_url: String,
}

impl GitHubProvider {
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
// Personal Access Token Provider
// =============================================================================

#[crate::register_node]
#[derive(Default)]
pub struct GitHubPatProviderNode {}

impl GitHubPatProviderNode {
    pub fn new() -> Self {
        Self {}
    }
}

#[async_trait]
impl NodeLogic for GitHubPatProviderNode {
    async fn get_node(&self, _app_state: &FlowLikeState) -> Node {
        let mut node = Node::new(
            "data_github_provider_pat",
            "GitHub (PAT)",
            "Connect to GitHub using a Personal Access Token. Generate one at github.com/settings/tokens",
            "Data/GitHub",
        );
        node.add_icon("/flow/icons/github.svg");

        node.add_input_pin(
            "token",
            "Personal Access Token",
            "Your GitHub Personal Access Token (classic or fine-grained)",
            VariableType::String,
        )
        .set_options(PinOptions::new().set_sensitive(true).build());

        node.add_input_pin(
            "base_url",
            "API Base URL",
            "GitHub API base URL. Use 'https://api.github.com' for github.com or 'https://your-enterprise.com/api/v3' for Enterprise",
            VariableType::String,
        )
        .set_default_value(Some(json!("https://api.github.com")));

        node.add_output_pin(
            "provider",
            "Provider",
            "GitHub provider with authentication",
            VariableType::Struct,
        )
        .set_schema::<GitHubProvider>()
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
            .unwrap_or_else(|_| "https://api.github.com".to_string());

        if token.is_empty() {
            return Err(flow_like_types::anyhow!(
                "Personal Access Token is required. Generate one at github.com/settings/tokens"
            ));
        }

        let provider = GitHubProvider {
            provider_id: GITHUB_PROVIDER_ID.to_string(),
            access_token: token,
            base_url,
        };

        context.set_pin_value("provider", json!(provider)).await?;

        Ok(())
    }
}

// =============================================================================
// OAuth Provider (Device Flow)
// =============================================================================

#[crate::register_node]
#[derive(Default)]
pub struct GitHubOAuthProviderNode {}

impl GitHubOAuthProviderNode {
    pub fn new() -> Self {
        Self {}
    }
}

#[async_trait]
impl NodeLogic for GitHubOAuthProviderNode {
    async fn get_node(&self, _app_state: &FlowLikeState) -> Node {
        let mut node = Node::new(
            "data_github_provider_oauth",
            "GitHub (OAuth)",
            "Connect to GitHub using OAuth Device Flow. Requires GH_CLIENT_ID environment variable to be set at build time.",
            "Data/GitHub",
        );
        node.add_icon("/flow/icons/github.svg");

        let env_client_id = GH_CLIENT_ID.unwrap_or_default();

        node.add_input_pin(
            "base_url",
            "API Base URL",
            "GitHub API base URL. Use 'https://api.github.com' for github.com or 'https://your-enterprise.com/api/v3' for Enterprise",
            VariableType::String,
        )
        .set_default_value(Some(json!("https://api.github.com")));

        node.add_input_pin(
            "scopes",
            "Scopes",
            "OAuth scopes to request (comma-separated). Common: repo, read:user, read:org, gist",
            VariableType::String,
        )
        .set_default_value(Some(json!("repo,read:user,read:org")));

        node.add_output_pin(
            "provider",
            "Provider",
            "GitHub provider with authentication",
            VariableType::Struct,
        )
        .set_schema::<GitHubProvider>()
        .set_options(PinOptions::new().set_enforce_schema(true).build());

        // Only add OAuth provider if client_id is available
        if !env_client_id.is_empty() {
            // GitHub Device Flow (RFC 8628) - perfect for desktop apps
            // User gets a code to enter at github.com/login/device
            // No client_secret needed, no redirect handling needed
            let oauth_provider = OAuthProvider::new(GITHUB_PROVIDER_ID, "GitHub")
                .set_auth_url("https://github.com/login/oauth/authorize")
                .set_token_url("https://github.com/login/oauth/access_token")
                .set_device_auth_url("https://github.com/login/device/code")
                .set_client_id(env_client_id)
                .set_scopes(vec![
                    "repo".to_string(),
                    "read:user".to_string(),
                    "read:org".to_string(),
                ])
                .set_pkce_required(false)
                .set_use_device_flow(true);

            node.add_oauth_provider(oauth_provider);
        }

        node.set_scores(
            NodeScores::new()
                .set_privacy(6)
                .set_security(8)
                .set_performance(8)
                .set_governance(7)
                .set_reliability(9)
                .set_cost(7)
                .build(),
        );

        node
    }

    async fn run(&self, context: &mut ExecutionContext) -> flow_like_types::Result<()> {
        let env_client_id = GH_CLIENT_ID.unwrap_or_default();

        if env_client_id.is_empty() {
            return Err(flow_like_types::anyhow!(
                "GitHub OAuth requires GH_CLIENT_ID environment variable. \
                Please set it at build time or use the 'GitHub (PAT)' node instead."
            ));
        }

        let base_url: String = context
            .evaluate_pin("base_url")
            .await
            .unwrap_or_else(|_| "https://api.github.com".to_string());

        let token = context
            .get_oauth_token(GITHUB_PROVIDER_ID)
            .ok_or_else(|| {
                flow_like_types::anyhow!(
                    "GitHub not authenticated. Please authorize access when prompted."
                )
            })?
            .clone();

        let provider = GitHubProvider {
            provider_id: GITHUB_PROVIDER_ID.to_string(),
            access_token: token.access_token,
            base_url,
        };

        context.set_pin_value("provider", json!(provider)).await?;

        Ok(())
    }
}

// =============================================================================
// GitHub App Installation Token Provider
// =============================================================================

#[crate::register_node]
#[derive(Default)]
pub struct GitHubAppProviderNode {}

impl GitHubAppProviderNode {
    pub fn new() -> Self {
        Self {}
    }
}

#[async_trait]
impl NodeLogic for GitHubAppProviderNode {
    async fn get_node(&self, _app_state: &FlowLikeState) -> Node {
        let mut node = Node::new(
            "data_github_provider_app",
            "GitHub (App Token)",
            "Connect to GitHub using a GitHub App installation token. Use this for server-to-server authentication.",
            "Data/GitHub",
        );
        node.add_icon("/flow/icons/github.svg");

        node.add_input_pin(
            "installation_token",
            "Installation Token",
            "GitHub App installation access token",
            VariableType::String,
        )
        .set_options(PinOptions::new().set_sensitive(true).build());

        node.add_input_pin(
            "base_url",
            "API Base URL",
            "GitHub API base URL. Use 'https://api.github.com' for github.com or 'https://your-enterprise.com/api/v3' for Enterprise",
            VariableType::String,
        )
        .set_default_value(Some(json!("https://api.github.com")));

        node.add_output_pin(
            "provider",
            "Provider",
            "GitHub provider with authentication",
            VariableType::Struct,
        )
        .set_schema::<GitHubProvider>()
        .set_options(PinOptions::new().set_enforce_schema(true).build());

        node.set_scores(
            NodeScores::new()
                .set_privacy(6)
                .set_security(9)
                .set_performance(9)
                .set_governance(8)
                .set_reliability(9)
                .set_cost(10)
                .build(),
        );

        node
    }

    async fn run(&self, context: &mut ExecutionContext) -> flow_like_types::Result<()> {
        let token: String = context.evaluate_pin("installation_token").await?;
        let base_url: String = context
            .evaluate_pin("base_url")
            .await
            .unwrap_or_else(|_| "https://api.github.com".to_string());

        if token.is_empty() {
            return Err(flow_like_types::anyhow!("Installation token is required"));
        }

        let provider = GitHubProvider {
            provider_id: GITHUB_PROVIDER_ID.to_string(),
            access_token: token,
            base_url,
        };

        context.set_pin_value("provider", json!(provider)).await?;

        Ok(())
    }
}
