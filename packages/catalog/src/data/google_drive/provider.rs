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

/// Generic Google provider ID - used for all Google services (Drive, Sheets, Docs, Gmail, YouTube, etc.)
pub const GOOGLE_PROVIDER_ID: &str = "google";

/// Legacy alias for backward compatibility
#[deprecated(note = "Use GOOGLE_PROVIDER_ID instead")]
pub const GOOGLE_DRIVE_PROVIDER_ID: &str = GOOGLE_PROVIDER_ID;

/// Google OAuth Client ID - set via environment variable at build time
const GOOGLE_CLIENT_ID: Option<&str> = option_env!("GOOGLE_CLIENT_ID");

/// Google provider output - contains authentication token for all Google services
/// (Drive, Sheets, Docs, Slides, Gmail, YouTube, Meet, Calendar, etc.)
#[derive(Serialize, Deserialize, JsonSchema, Debug, Clone)]
pub struct GoogleProvider {
    /// The provider ID
    pub provider_id: String,
    /// The OAuth access token
    pub access_token: String,
    /// Optional refresh token
    pub refresh_token: Option<String>,
    /// Token expiration timestamp (unix seconds)
    pub expires_at: Option<u64>,
}

/// Legacy alias for backward compatibility
pub type GoogleDriveProvider = GoogleProvider;

#[crate::register_node]
#[derive(Default)]
pub struct GoogleProviderNode {}

impl GoogleProviderNode {
    pub fn new() -> Self {
        Self {}
    }
}

#[async_trait]
impl NodeLogic for GoogleProviderNode {
    async fn get_node(&self, _app_state: &FlowLikeState) -> Node {
        let mut node = Node::new(
            "data_google_provider",
            "Google",
            "Authenticate with Google to access Drive, Sheets, Docs, Gmail, YouTube, Calendar and more. Requires GOOGLE_CLIENT_ID environment variable.",
            "Data/Google",
        );
        node.add_icon("/flow/icons/google.svg");

        // Output: The Google provider (contains provider ID + token)
        node.add_output_pin(
            "provider",
            "Provider",
            "Google provider with authentication token - works with all Google services",
            VariableType::Struct,
        )
        .set_schema::<GoogleProvider>()
        .set_options(PinOptions::new().set_enforce_schema(true).build());

        // Configure OAuth provider metadata (used by frontend for auth flow)
        // Client ID is read from GOOGLE_CLIENT_ID environment variable at compile time
        let client_id = GOOGLE_CLIENT_ID.unwrap_or_default();

        // Base scopes are minimal - individual nodes add their required scopes dynamically
        let oauth_provider = OAuthProvider::new(GOOGLE_PROVIDER_ID, "Google")
            .set_auth_url("https://accounts.google.com/o/oauth2/v2/auth")
            .set_token_url("https://oauth2.googleapis.com/token")
            .set_client_id(client_id)
            .set_scopes(vec![
                "openid".to_string(),
                "email".to_string(),
                "profile".to_string(),
            ])
            .set_pkce_required(true)
            .set_userinfo_url("https://www.googleapis.com/oauth2/v2/userinfo")
            .set_revoke_url("https://oauth2.googleapis.com/revoke");

        node.add_oauth_provider(oauth_provider);

        // Set scores - Google is a cloud service
        node.set_scores(
            NodeScores::new()
                .set_privacy(5)    // Cloud services with third-party access
                .set_security(8)   // OAuth 2.0 with PKCE
                .set_performance(7) // Network dependent
                .set_governance(6)  // Google's data policies apply
                .set_reliability(9) // Google's infrastructure
                .set_cost(5)       // Has free tier but limits apply
                .build(),
        );

        node
    }

    async fn run(&self, context: &mut ExecutionContext) -> flow_like_types::Result<()> {
        // Get the OAuth token from the context (injected by the execution engine after auth)
        let token = context
            .get_oauth_token(GOOGLE_PROVIDER_ID)
            .ok_or_else(|| flow_like_types::anyhow!(
                "Google not authenticated. Please authorize access when prompted."
            ))?
            .clone();

        // Build the provider output
        let provider = GoogleProvider {
            provider_id: GOOGLE_PROVIDER_ID.to_string(),
            access_token: token.access_token,
            refresh_token: token.refresh_token,
            expires_at: token.expires_at,
        };

        context.set_pin_value("provider", json!(provider)).await?;

        Ok(())
    }
}
