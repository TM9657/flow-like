use super::node::{Node, NodeLogic};

use futures::future::join_all;
use std::sync::Arc;
use tokio::sync::Mutex;

pub mod ai;
pub mod control;
pub mod events;
pub mod http;
pub mod logging;
pub mod math;
pub mod storage;
pub mod structs;
pub mod utils;
pub mod variables;
pub mod bit;

use crate::state::FlowLikeState;

pub async fn node_to_dyn(
    app_state: &FlowLikeState,
    node: &Node,
) -> anyhow::Result<Arc<Mutex<dyn NodeLogic>>> {
    let registry_state = app_state.node_registry();
    let registry = registry_state.read().await;

    let node = registry.instantiate(node).await?;
    Ok(node)
}

pub async fn load_catalog(app_state: &FlowLikeState) -> Vec<Node> {
    let catalog = app_state.node_registry();
    let catalog = catalog.read().await;
    let items = catalog.get_nodes();

    if let Ok(items) = items {
        return items;
    }

    drop(catalog);

    let intermediate_registry = [
        ai::register_functions().await,
        control::register_functions().await,
        variables::register_functions().await,
        logging::register_functions().await,
        events::register_functions().await,
        utils::register_functions().await,
        structs::register_functions().await,
        storage::register_functions().await,
    ];

    let futures: Vec<_> = intermediate_registry
        .iter()
        .flatten()
        .map(|node| async {
            let node_ref = node.clone();
            let node = node.lock().await.get_node(app_state).await;
            (node, node_ref)
        })
        .collect();

    let nodes = join_all(futures).await;
    let registry_guard = app_state.node_registry();
    let mut registry = registry_guard.write().await;
    registry.initialize(nodes);
    registry.get_nodes().unwrap_or(vec![])
}
