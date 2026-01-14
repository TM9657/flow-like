use crate::{error::ApiError, middleware::jwt::AppUser, state::AppState};
use axum::{
    extract::State,
    routing::post,
    Extension, Json, Router,
};
use flow_like::a2ui::SurfaceComponent;
use flow_like::copilot::{
    CopilotScope, RunContext, UIActionContext, UnifiedChatMessage, UnifiedCopilotResponse,
};
use flow_like::flow::board::Board;
use serde::Deserialize;

pub fn routes() -> Router<AppState> {
    Router::new().route("/chat", post(copilot_chat))
}

/// Request payload for the unified copilot endpoint
#[derive(Deserialize)]
pub struct CopilotChatRequest {
    /// The scope of operation: "Board", "Frontend", or "Both"
    pub scope: CopilotScope,

    /// Board context (optional for Frontend scope)
    #[serde(default)]
    pub board: Option<Board>,
    #[serde(default)]
    pub selected_node_ids: Vec<String>,

    /// UI context (optional for Board scope)
    #[serde(default)]
    pub current_surface: Option<Vec<SurfaceComponent>>,
    #[serde(default)]
    pub selected_component_ids: Vec<String>,

    /// The user's prompt
    pub user_prompt: String,

    /// Chat history
    #[serde(default)]
    pub history: Vec<UnifiedChatMessage>,

    /// Optional model ID to use
    #[serde(default)]
    pub model_id: Option<String>,

    /// Run context for log queries (board mode)
    #[serde(default)]
    pub run_context: Option<RunContext>,

    /// Action context for UI (frontend mode)
    #[serde(default)]
    pub action_context: Option<UIActionContext>,

    /// Whether to stream the response
    #[serde(default)]
    pub stream: bool,
}

// NOTE: ServerCatalogProvider and full implementation are commented out
// due to Rig agent lifetime issues in async context. The implementation
// is preserved in comments for future reference when the issue is resolved.

/// Unified copilot chat endpoint
///
/// This endpoint handles AI-powered assistance for both workflow graph editing
/// and UI generation. The scope parameter determines which capabilities are available.
///
/// NOTE: Currently returns a placeholder response. Full implementation requires
/// resolving Rig agent lifetime issues in async context.
pub async fn copilot_chat(
    State(_state): State<AppState>,
    Extension(user): Extension<AppUser>,
    Json(payload): Json<CopilotChatRequest>,
) -> Result<Json<UnifiedCopilotResponse>, ApiError> {
    let sub = user.sub()?;

    tracing::info!(
        "[copilot_chat] User {} requested scope {:?}",
        sub,
        payload.scope
    );

    // TODO: Full implementation requires resolving Rig agent lifetime issues
    // For now, return a placeholder response
    let response = UnifiedCopilotResponse {
        message: "Copilot API endpoint is under construction. Please use the desktop application for full copilot functionality.".to_string(),
        commands: vec![],
        components: vec![],
        suggestions: vec![],
        active_scope: payload.scope,
    };

    Ok(Json(response))
}
