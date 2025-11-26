use crate::state::{TauriFlowLikeState, TauriSettingsState};
use async_trait::async_trait;
use flow_like::flow::board::Board;
use flow_like::flow::copilot::{
    CatalogProvider, ChatMessage, Copilot, CopilotResponse, NodeMetadata, PinMetadata, Suggestion,
};
use flow_like::flow::node::Node;
use flow_like::flow::pin::{Pin, PinType};
use flow_like::flow::variable::VariableType;
use flow_like_catalog::get_catalog;
use std::sync::Arc;
use tauri::{AppHandle, State, ipc::Channel};

struct DesktopCatalogProvider {
    state: TauriFlowLikeState,
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
        let state_guard = self.state.0.lock().await;

        for logic in catalog {
            let node = logic.get_node(&state_guard).await;
            let name_lower = node.name.to_lowercase();
            let friendly_lower = node.friendly_name.to_lowercase();
            let desc_lower = node.description.to_lowercase();

            // Extract category from name (e.g., "flow_like_catalog::string::concat" -> "string")
            let category = name_lower.split("::").nth(1).unwrap_or("");

            // Score based on match quality
            let mut score = 0i32;

            // Exact full query match (highest priority)
            if name_lower.contains(&query_lower) {
                score += 100;
            }
            if friendly_lower.contains(&query_lower) {
                score += 90;
            }

            // Token-based matching
            for token in &query_tokens {
                // Name matches (high value)
                if name_lower.contains(token) {
                    score += 30;
                }
                // Friendly name matches (high value)
                if friendly_lower.contains(token) {
                    score += 25;
                }
                // Category matches (medium value)
                if category.contains(token) {
                    score += 20;
                }
                // Description matches (lower value, but important for discovery)
                if desc_lower.contains(token) {
                    score += 10;
                }
            }

            // Bonus for exact word boundaries
            let name_parts: Vec<&str> = name_lower.split(|c: char| c == ':' || c == '_').collect();
            for token in &query_tokens {
                if name_parts.iter().any(|part| part == token) {
                    score += 15; // Exact word match in name
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
                            .map(|p| pin_to_metadata(p))
                            .collect(),
                        outputs: node
                            .pins
                            .values()
                            .filter(|p| p.pin_type == PinType::Output)
                            .map(|p| pin_to_metadata(p))
                            .collect(),
                        category: Some(category.to_string()),
                    },
                ));
            }
        }

        // Sort by score descending
        scored_matches.sort_by(|a, b| b.0.cmp(&a.0));

        // Return top 10 results
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
        let state_guard = self.state.0.lock().await;

        for logic in catalog {
            let node = logic.get_node(&state_guard).await;
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
                        .map(|p| pin_to_metadata(p))
                        .collect(),
                    outputs: node
                        .pins
                        .values()
                        .filter(|p| p.pin_type == PinType::Output)
                        .map(|p| pin_to_metadata(p))
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
        let state_guard = self.state.0.lock().await;

        for logic in catalog {
            let node = logic.get_node(&state_guard).await;
            let name_lower = node.name.to_lowercase();
            // Extract category from name (e.g., "flow_like_catalog::string::concat" -> "string")
            let category = name_lower.split("::").nth(1).unwrap_or("");

            // Match if category starts with or contains the prefix
            if category.starts_with(&category_prefix) || name_lower.contains(&category_prefix) {
                matches.push(NodeMetadata {
                    name: node.name,
                    friendly_name: node.friendly_name,
                    description: node.description,
                    inputs: node
                        .pins
                        .values()
                        .filter(|p| p.pin_type == PinType::Input)
                        .map(|p| pin_to_metadata(p))
                        .collect(),
                    outputs: node
                        .pins
                        .values()
                        .filter(|p| p.pin_type == PinType::Output)
                        .map(|p| pin_to_metadata(p))
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
        let state_guard = self.state.0.lock().await;
        let mut names = Vec::new();
        for logic in catalog {
            let node = logic.get_node(&state_guard).await;
            names.push(node.name);
        }
        names
    }
}

#[tauri::command]
pub async fn flowpilot_chat(
    app_handle: AppHandle,
    state: State<'_, TauriFlowLikeState>,
    board: Option<Board>,
    selected_node_ids: Option<Vec<String>>,
    user_prompt: String,
    history: Option<Vec<ChatMessage>>,
    model_id: Option<String>,
    token: Option<String>,
    run_context: Option<flow_like::flow::copilot::RunContext>,
    channel: Channel<String>,
) -> Result<CopilotResponse, String> {
    println!(
        "[flowpilot_chat] Called with run_context: {:?}",
        run_context
    );

    let board = board.ok_or("Board is required")?;
    let selected_node_ids = selected_node_ids.unwrap_or_default();
    let history = history.unwrap_or_default();

    let state_guard = state.0.lock().await;
    let state_clone = state_guard.clone();
    drop(state_guard);

    let profile = TauriSettingsState::current_profile(&app_handle)
        .await
        .ok()
        .map(|p| Arc::new(p.hub_profile));

    let provider = Arc::new(DesktopCatalogProvider {
        state: state.inner().clone(),
    });
    let copilot = Copilot::new(state_clone, provider, profile, None)
        .await
        .map_err(|e| e.to_string())?;

    let on_token = Some(move |token: String| {
        let _ = channel.send(token);
    });

    copilot
        .chat(
            &board,
            &selected_node_ids,
            user_prompt,
            history,
            model_id,
            token,
            run_context,
            on_token,
        )
        .await
        .map_err(|e| e.to_string())
}
