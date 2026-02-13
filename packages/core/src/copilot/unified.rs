//! Unified Copilot - Delegates to the appropriate existing copilot implementations
//!
//! This module provides a unified `UnifiedCopilot` struct that delegates to either
//! the flow `Copilot` or A2UI `A2UICopilot` based on the requested scope.

use std::sync::Arc;

use flow_like_types::Result;

use crate::a2ui::SurfaceComponent;
use crate::a2ui::copilot::A2UICopilot;
use crate::flow::board::Board;
use crate::flow::copilot::{CatalogProvider, Copilot, RunContext};
use crate::profile::Profile;
use crate::state::FlowLikeState;

use super::types::*;

/// The unified copilot that delegates to appropriate implementations
pub struct UnifiedCopilot {
    state: Arc<FlowLikeState>,
    catalog_provider: Option<Arc<dyn CatalogProvider>>,
    profile: Option<Arc<Profile>>,
    current_template_id: Option<String>,
}

impl UnifiedCopilot {
    /// Create a new UnifiedCopilot
    pub async fn new(
        state: Arc<FlowLikeState>,
        catalog_provider: Option<Arc<dyn CatalogProvider>>,
        profile: Option<Arc<Profile>>,
        current_template_id: Option<String>,
    ) -> Result<Self> {
        Ok(Self {
            state,
            catalog_provider,
            profile,
            current_template_id,
        })
    }

    /// Main entry point - unified chat that can handle board, UI, or both
    pub async fn chat<F>(
        &self,
        scope: CopilotScope,
        // Board context (optional for Frontend scope)
        board: Option<&Board>,
        selected_node_ids: &[String],
        // UI context (optional for Board scope)
        current_surface: Option<&Vec<SurfaceComponent>>,
        selected_component_ids: &[String],
        // Common parameters
        user_prompt: String,
        history: Vec<UnifiedChatMessage>,
        model_id: Option<String>,
        token: Option<String>,
        context: Option<UnifiedContext>,
        on_token: Option<F>,
    ) -> Result<UnifiedCopilotResponse>
    where
        F: Fn(String) + Send + Sync + 'static + Clone,
    {
        // Determine effective scope based on available data
        let effective_scope = self.determine_effective_scope(scope, board, current_surface);

        // Send scope decision event
        if let Some(ref callback) = on_token {
            let event = UnifiedStreamEvent::ScopeDecision(effective_scope);
            callback(format!(
                "<scope_decision>{}</scope_decision>",
                serde_json::to_string(&event).unwrap_or_default()
            ));
        }

        match effective_scope {
            CopilotScope::Board => {
                self.delegate_to_board(
                    board.ok_or_else(|| {
                        flow_like_types::anyhow!("Board is required for Board scope")
                    })?,
                    selected_node_ids,
                    user_prompt,
                    history,
                    model_id,
                    token,
                    context.and_then(|c| c.run_context),
                    on_token,
                )
                .await
            }
            CopilotScope::Frontend => {
                self.delegate_to_frontend(
                    current_surface,
                    selected_component_ids,
                    user_prompt,
                    history,
                    model_id,
                    token,
                    context.and_then(|c| c.action_context),
                    on_token,
                )
                .await
            }
            CopilotScope::Both => {
                // For Both scope, we run both copilots and merge results
                self.run_both(
                    board,
                    selected_node_ids,
                    current_surface,
                    selected_component_ids,
                    user_prompt,
                    history,
                    model_id,
                    token,
                    context,
                    on_token,
                )
                .await
            }
        }
    }

    /// Determine the effective scope based on available data
    fn determine_effective_scope(
        &self,
        requested_scope: CopilotScope,
        board: Option<&Board>,
        current_surface: Option<&Vec<SurfaceComponent>>,
    ) -> CopilotScope {
        match requested_scope {
            CopilotScope::Board => {
                if board.is_some() && self.catalog_provider.is_some() {
                    CopilotScope::Board
                } else {
                    CopilotScope::Frontend
                }
            }
            CopilotScope::Frontend => CopilotScope::Frontend,
            CopilotScope::Both => {
                if board.is_some() && self.catalog_provider.is_some() {
                    CopilotScope::Both
                } else if current_surface.is_some() || board.is_none() {
                    CopilotScope::Frontend
                } else {
                    CopilotScope::Board
                }
            }
        }
    }

    /// Delegate to the flow Copilot for board operations
    async fn delegate_to_board<F>(
        &self,
        board: &Board,
        selected_node_ids: &[String],
        user_prompt: String,
        history: Vec<UnifiedChatMessage>,
        model_id: Option<String>,
        token: Option<String>,
        run_context: Option<RunContext>,
        on_token: Option<F>,
    ) -> Result<UnifiedCopilotResponse>
    where
        F: Fn(String) + Send + Sync + 'static,
    {
        let catalog_provider = self
            .catalog_provider
            .as_ref()
            .ok_or_else(|| flow_like_types::anyhow!("Catalog provider required for Board mode"))?;

        let copilot = Copilot::new(
            self.state.clone(),
            catalog_provider.clone(),
            self.profile.clone(),
            self.current_template_id.clone(),
        )
        .await?;

        // Convert history to flow ChatMessage format
        let board_history = history
            .into_iter()
            .map(|m| crate::flow::copilot::ChatMessage {
                role: m.role,
                content: m.content,
                images: m.images,
            })
            .collect();

        let response = copilot
            .chat(
                board,
                selected_node_ids,
                user_prompt,
                board_history,
                model_id,
                token,
                run_context,
                on_token,
            )
            .await?;

        // Convert response
        Ok(UnifiedCopilotResponse {
            message: response.message,
            commands: response.commands,
            components: vec![],
            suggestions: response
                .suggestions
                .into_iter()
                .map(|s| UnifiedSuggestion {
                    label: s.node_type,
                    prompt: s.reason,
                    scope: Some(CopilotScope::Board),
                })
                .collect(),
            active_scope: CopilotScope::Board,
            canvas_settings: None,
            root_component_id: None,
        })
    }

    /// Delegate to the A2UI Copilot for frontend operations
    async fn delegate_to_frontend<F>(
        &self,
        current_surface: Option<&Vec<SurfaceComponent>>,
        selected_component_ids: &[String],
        user_prompt: String,
        history: Vec<UnifiedChatMessage>,
        model_id: Option<String>,
        token: Option<String>,
        _action_context: Option<UIActionContext>,
        on_token: Option<F>,
    ) -> Result<UnifiedCopilotResponse>
    where
        F: Fn(String) + Send + Sync + 'static,
    {
        let copilot = A2UICopilot::new(self.state.clone(), self.profile.clone()).await?;

        // Convert history
        let ui_history = history
            .into_iter()
            .map(|m| crate::a2ui::copilot::A2UIChatMessage {
                role: match m.role {
                    ChatRole::User => crate::a2ui::copilot::A2UIChatRole::User,
                    ChatRole::Assistant => crate::a2ui::copilot::A2UIChatRole::Assistant,
                },
                content: m.content,
                images: m.images.map(|imgs| {
                    imgs.into_iter()
                        .map(|img| crate::a2ui::copilot::A2UIChatImage {
                            data: img.data,
                            media_type: img.media_type,
                        })
                        .collect()
                }),
            })
            .collect();

        // Note: A2UICopilot doesn't support action_context yet, so we ignore it
        let response = copilot
            .chat(
                current_surface,
                selected_component_ids,
                user_prompt,
                ui_history,
                model_id,
                token,
                on_token,
            )
            .await?;

        // Convert response
        Ok(UnifiedCopilotResponse {
            message: response.message,
            commands: vec![],
            components: response.components,
            suggestions: vec![],
            active_scope: CopilotScope::Frontend,
            canvas_settings: None,
            root_component_id: None,
        })
    }

    /// Run both copilots for unified mode
    async fn run_both<F>(
        &self,
        board: Option<&Board>,
        selected_node_ids: &[String],
        current_surface: Option<&Vec<SurfaceComponent>>,
        selected_component_ids: &[String],
        user_prompt: String,
        history: Vec<UnifiedChatMessage>,
        model_id: Option<String>,
        token: Option<String>,
        context: Option<UnifiedContext>,
        on_token: Option<F>,
    ) -> Result<UnifiedCopilotResponse>
    where
        F: Fn(String) + Send + Sync + 'static + Clone,
    {
        // Analyze the prompt to determine primary focus
        let prompt_lower = user_prompt.to_lowercase();
        let is_ui_focused = prompt_lower.contains("ui")
            || prompt_lower.contains("button")
            || prompt_lower.contains("form")
            || prompt_lower.contains("component")
            || prompt_lower.contains("layout")
            || prompt_lower.contains("style")
            || prompt_lower.contains("display");

        let is_workflow_focused = prompt_lower.contains("workflow")
            || prompt_lower.contains("node")
            || prompt_lower.contains("connect")
            || prompt_lower.contains("flow")
            || prompt_lower.contains("automat");

        // If clearly one type, delegate to that
        if is_ui_focused && !is_workflow_focused {
            return self
                .delegate_to_frontend(
                    current_surface,
                    selected_component_ids,
                    user_prompt,
                    history,
                    model_id,
                    token,
                    context.and_then(|c| c.action_context),
                    on_token,
                )
                .await;
        }

        if is_workflow_focused
            && !is_ui_focused
            && let Some(b) = board
        {
            return self
                .delegate_to_board(
                    b,
                    selected_node_ids,
                    user_prompt,
                    history,
                    model_id,
                    token,
                    context.and_then(|c| c.run_context),
                    on_token,
                )
                .await;
        }

        // Default to board if available, otherwise frontend
        if let Some(b) = board
            && self.catalog_provider.is_some()
        {
            return self
                .delegate_to_board(
                    b,
                    selected_node_ids,
                    user_prompt,
                    history,
                    model_id,
                    token,
                    context.and_then(|c| c.run_context),
                    on_token,
                )
                .await;
        }

        self.delegate_to_frontend(
            current_surface,
            selected_component_ids,
            user_prompt,
            history,
            model_id,
            token,
            context.and_then(|c| c.action_context),
            on_token,
        )
        .await
    }
}
