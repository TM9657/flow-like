use crate::state::{TauriFlowLikeState, TauriSettingsState};
use async_trait::async_trait;
use flow_like::flow::board::Board;
use flow_like::flow::copilot::{CatalogProvider, Copilot, Edge, NodeMetadata, Suggestion};
use flow_like::flow::node::Node;
use flow_like::flow::pin::PinType;
use flow_like_catalog::get_catalog;
use std::sync::Arc;
use tauri::{AppHandle, State, ipc::Channel};

struct DesktopCatalogProvider {
    state: TauriFlowLikeState,
}

#[async_trait]
impl CatalogProvider for DesktopCatalogProvider {
    async fn search(&self, query: &str) -> Vec<NodeMetadata> {
        let catalog = get_catalog();
        let query = query.to_lowercase();

        let mut matches = Vec::new();

        let state_guard = self.state.0.lock().await;

        for logic in catalog {
            let node = logic.get_node(&state_guard).await;
            if node.name.to_lowercase().contains(&query)
                || node.description.to_lowercase().contains(&query)
            {
                matches.push(NodeMetadata {
                    name: node.name,
                    description: node.description,
                    inputs: node
                        .pins
                        .values()
                        .filter(|p| p.pin_type == PinType::Input)
                        .map(|p| p.name.clone())
                        .collect(),
                    outputs: node
                        .pins
                        .values()
                        .filter(|p| p.pin_type == PinType::Output)
                        .map(|p| p.name.clone())
                        .collect(),
                });
            }
            if matches.len() >= 5 {
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
pub async fn autocomplete(
    app_handle: AppHandle,
    state: State<'_, TauriFlowLikeState>,
    board: Option<Board>,
    selected_node_ids: Option<Vec<String>>,
    user_prompt: Option<String>,
    model_id: Option<String>,
    token: Option<String>,
    channel: Channel<String>,
) -> Result<Vec<Suggestion>, String> {
    let board = board.ok_or("Board is required")?;
    let selected_node_ids = selected_node_ids.unwrap_or_default();

    let state_guard = state.0.lock().await;
    let state_clone = state_guard.clone();
    drop(state_guard); // Release lock

    let profile = TauriSettingsState::current_profile(&app_handle)
        .await
        .ok()
        .map(|p| Arc::new(p.hub_profile));

    let provider = Arc::new(DesktopCatalogProvider {
        state: state.inner().clone(),
    });
    let copilot = Copilot::new(state_clone, provider, profile);

    let on_token = Some(move |token: String| {
        let _ = channel.send(token);
    });

    copilot
        .autocomplete(
            &board,
            &selected_node_ids,
            user_prompt,
            model_id,
            token,
            on_token,
        )
        .await
        .map_err(|e| e.to_string())
}
