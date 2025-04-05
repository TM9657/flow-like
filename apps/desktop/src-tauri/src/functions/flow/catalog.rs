use flow_like::flow::node::Node;
use tauri::AppHandle;

use crate::{functions::TauriFunctionError, state::TauriFlowLikeState};

#[tauri::command(async)]
pub async fn get_catalog(handler: AppHandle) -> Result<Vec<Node>, TauriFunctionError> {
    let nodes = TauriFlowLikeState::construct(&handler)
        .await?
        .lock()
        .await
        .node_registry
        .read()
        .await
        .get_nodes()?;
    Ok(nodes)
}
