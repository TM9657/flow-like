use flow_like::flow::node::Node;
use tauri::AppHandle;

use crate::{functions::TauriFunctionError, state::TauriFlowLikeState};

#[tauri::command(async)]
pub async fn get_catalog(handler: AppHandle) -> Result<Vec<Node>, TauriFunctionError> {
    let flow_like_state = TauriFlowLikeState::construct(&handler).await?;
    let catalog = flow_like::flow::catalog::load_catalog(flow_like_state.clone()).await;
    Ok(catalog)
}
