use crate::state::{TauriFlowLikeState, TauriSettingsState};
use async_trait::async_trait;
use flow_like::a2ui::SurfaceComponent;
use flow_like::copilot::{
    CopilotScope, UIActionContext, UnifiedChatMessage, UnifiedContext, UnifiedCopilot,
    UnifiedCopilotResponse,
};
use flow_like::flow::board::Board;
use flow_like::flow::copilot::{CatalogProvider, NodeMetadata, PinMetadata, RunContext};
use flow_like::flow::pin::{Pin, PinType};
use flow_like::flow::variable::VariableType;
use flow_like_catalog::get_catalog;
use std::sync::Arc;
use tauri::{AppHandle, State, ipc::Channel};

/// Desktop implementation of the catalog provider for node search
struct DesktopCatalogProvider {
    _state: TauriFlowLikeState,
}

fn pin_to_metadata(p: &Pin) -> PinMetadata {
    let is_generic = p.data_type == VariableType::Generic;
    let enforce_schema = p
        .options
        .as_ref()
        .and_then(|o| o.enforce_schema)
        .unwrap_or(false);
    let valid_values = p.options.as_ref().and_then(|o| o.valid_values.clone());

    PinMetadata {
        name: p.name.clone(),
        friendly_name: p.friendly_name.clone(),
        description: p.description.clone(),
        data_type: format!("{:?}", p.data_type),
        value_type: format!("{:?}", p.value_type),
        schema: p.schema.clone(),
        is_generic,
        valid_values,
        enforce_schema,
    }
}

#[async_trait]
impl CatalogProvider for DesktopCatalogProvider {
    async fn search(&self, query: &str) -> Vec<NodeMetadata> {
        let catalog = get_catalog();
        let query_lower = query.to_lowercase();
        let query_tokens: Vec<&str> = query_lower.split_whitespace().collect();

        let mut scored_matches: Vec<(i32, NodeMetadata)> = Vec::new();

        for logic in catalog {
            let node = logic.get_node();
            let name_lower = node.name.to_lowercase();
            let friendly_lower = node.friendly_name.to_lowercase();
            let desc_lower = node.description.to_lowercase();

            let category = name_lower.split("::").nth(1).unwrap_or("");

            let mut score = 0i32;

            if name_lower.contains(&query_lower) {
                score += 100;
            }
            if friendly_lower.contains(&query_lower) {
                score += 90;
            }

            for token in &query_tokens {
                if name_lower.contains(token) {
                    score += 30;
                }
                if friendly_lower.contains(token) {
                    score += 25;
                }
                if category.contains(token) {
                    score += 20;
                }
                if desc_lower.contains(token) {
                    score += 10;
                }
            }

            let name_parts: Vec<&str> = name_lower.split([':', '_']).collect();
            for token in &query_tokens {
                if name_parts.iter().any(|part| part == token) {
                    score += 15;
                }
            }

            if score > 0 {
                scored_matches.push((
                    score,
                    NodeMetadata {
                        name: node.name,
                        friendly_name: node.friendly_name,
                        description: node.description,
                        inputs: node
                            .pins
                            .values()
                            .filter(|p| p.pin_type == PinType::Input)
                            .map(pin_to_metadata)
                            .collect(),
                        outputs: node
                            .pins
                            .values()
                            .filter(|p| p.pin_type == PinType::Output)
                            .map(pin_to_metadata)
                            .collect(),
                        category: Some(category.to_string()),
                    },
                ));
            }
        }

        scored_matches.sort_by(|a, b| b.0.cmp(&a.0));
        scored_matches
            .into_iter()
            .take(10)
            .map(|(_, meta)| meta)
            .collect()
    }

    async fn search_by_pin_type(&self, pin_type: &str, is_input: bool) -> Vec<NodeMetadata> {
        let catalog = get_catalog();
        let pin_type = pin_type.to_lowercase();
        let mut matches = Vec::new();

        for logic in catalog {
            let node = logic.get_node();
            let name_lower = node.name.to_lowercase();
            let category = name_lower.split("::").nth(1).unwrap_or("");

            let has_matching_pin = node.pins.values().any(|p| {
                let is_correct_direction = if is_input {
                    p.pin_type == PinType::Input
                } else {
                    p.pin_type == PinType::Output
                };
                is_correct_direction
                    && format!("{:?}", p.data_type)
                        .to_lowercase()
                        .contains(&pin_type)
            });

            if has_matching_pin {
                matches.push(NodeMetadata {
                    name: node.name,
                    friendly_name: node.friendly_name,
                    description: node.description,
                    inputs: node
                        .pins
                        .values()
                        .filter(|p| p.pin_type == PinType::Input)
                        .map(pin_to_metadata)
                        .collect(),
                    outputs: node
                        .pins
                        .values()
                        .filter(|p| p.pin_type == PinType::Output)
                        .map(pin_to_metadata)
                        .collect(),
                    category: Some(category.to_string()),
                });
            }
            if matches.len() >= 10 {
                break;
            }
        }
        matches
    }

    async fn filter_by_category(&self, category_prefix: &str) -> Vec<NodeMetadata> {
        let catalog = get_catalog();
        let category_prefix = category_prefix.to_lowercase();
        let mut matches = Vec::new();

        for logic in catalog {
            let node = logic.get_node();
            let name_lower = node.name.to_lowercase();
            let category = name_lower.split("::").nth(1).unwrap_or("");

            if category.starts_with(&category_prefix) || name_lower.contains(&category_prefix) {
                matches.push(NodeMetadata {
                    name: node.name,
                    friendly_name: node.friendly_name,
                    description: node.description,
                    inputs: node
                        .pins
                        .values()
                        .filter(|p| p.pin_type == PinType::Input)
                        .map(pin_to_metadata)
                        .collect(),
                    outputs: node
                        .pins
                        .values()
                        .filter(|p| p.pin_type == PinType::Output)
                        .map(pin_to_metadata)
                        .collect(),
                    category: Some(category.to_string()),
                });
            }
            if matches.len() >= 15 {
                break;
            }
        }
        matches
    }

    async fn get_all_nodes(&self) -> Vec<String> {
        let catalog = get_catalog();
        catalog.iter().map(|logic| logic.get_node().name).collect()
    }
}

/// Unified copilot chat command that handles both board and UI generation
#[tauri::command]
pub async fn copilot_chat(
    app_handle: AppHandle,
    state: State<'_, TauriFlowLikeState>,
    // Scope selection
    scope: CopilotScope,
    // Board context (optional for Frontend scope)
    board: Option<Board>,
    selected_node_ids: Option<Vec<String>>,
    // UI context (optional for Board scope)
    current_surface: Option<Vec<SurfaceComponent>>,
    selected_component_ids: Option<Vec<String>>,
    // Common parameters
    user_prompt: String,
    history: Option<Vec<UnifiedChatMessage>>,
    model_id: Option<String>,
    token: Option<String>,
    // Extended context
    run_context: Option<RunContext>,
    action_context: Option<UIActionContext>,
    // Streaming channel
    channel: Channel<String>,
) -> Result<UnifiedCopilotResponse, String> {
    println!(
        "[copilot_chat] Called with scope: {:?}, run_context: {:?}",
        scope, run_context
    );

    let selected_node_ids = selected_node_ids.unwrap_or_default();
    let selected_component_ids = selected_component_ids.unwrap_or_default();
    let history = history.unwrap_or_default();

    let state_clone = state.0.clone();

    let profile = TauriSettingsState::current_profile(&app_handle)
        .await
        .ok()
        .map(|p| Arc::new(p.hub_profile));

    // Only create catalog provider if we might need it (Board or Both scope)
    let catalog_provider: Option<Arc<dyn CatalogProvider>> = match scope {
        CopilotScope::Frontend => None,
        _ => Some(Arc::new(DesktopCatalogProvider {
            _state: state.inner().clone(),
        })),
    };

    let copilot = UnifiedCopilot::new(state_clone, catalog_provider, profile, None)
        .await
        .map_err(|e| e.to_string())?;

    let on_token = Some(move |token: String| {
        let _ = channel.send(token);
    });

    // Build unified context
    let context = if run_context.is_some() || action_context.is_some() {
        Some(UnifiedContext {
            scope,
            run_context,
            action_context,
        })
    } else {
        None
    };

    copilot
        .chat(
            scope,
            board.as_ref(),
            &selected_node_ids,
            current_surface.as_ref(),
            &selected_component_ids,
            user_prompt,
            history,
            model_id,
            token,
            context,
            on_token,
        )
        .await
        .map_err(|e| e.to_string())
}
