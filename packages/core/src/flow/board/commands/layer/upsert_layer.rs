use flow_like_types::async_trait;
use flow_like_types::sync::Mutex;
use schemars::JsonSchema;
use std::collections::HashSet;
use std::sync::Arc;

use crate::flow::board::Layer;
use crate::{
    flow:: board::{Board, commands::Command},
    state::FlowLikeState,
};
use serde::{Deserialize, Serialize};

#[derive(Clone, Serialize, Deserialize, JsonSchema)]
pub struct UpsertLayerCommand {
    pub old_layer: Option<Layer>,
    pub layer: Layer,
    pub node_ids: Vec<String>,
}

impl UpsertLayerCommand {
    pub fn new(layer: Layer) -> Self {
        UpsertLayerCommand { layer, old_layer: None, node_ids: vec![] }
    }
}

#[async_trait]
impl Command for UpsertLayerCommand {
    async fn execute(
        &mut self,
        board: &mut Board,
        _state: Arc<Mutex<FlowLikeState>>,
    ) -> flow_like_types::Result<()> {
        let nodes_set: HashSet<String> = HashSet::from_iter(self.node_ids.iter().cloned());
        self.old_layer = board.layers.insert(self.layer.id.clone(), self.layer.clone());
        for node in board.nodes.values_mut() {
            if nodes_set.contains(&node.id) {
                node.layer = Some(self.layer.id.clone());
            }
        }

        for layer in board.layers.values_mut() {
            if nodes_set.contains(&layer.id) {
                layer.parent_id = Some(self.layer.id.clone());
            }
        }

        board.fix_pins_set_layer();

        Ok(())
    }

    async fn undo(
        &mut self,
        board: &mut Board,
        _: Arc<Mutex<FlowLikeState>>,
    ) -> flow_like_types::Result<()> {
        let mut old_layer_id = None;
        if let Some(old_layer) = self.old_layer.take() {
            old_layer_id = Some(old_layer.id.clone());
            board.layers.insert(old_layer.id.clone(), old_layer.clone());
        } else {
            board.layers.remove(&self.layer.id);
        }

        for node in board.nodes.values_mut() {
            if node.layer == Some(self.layer.id.clone()) {
                node.layer = old_layer_id.clone();
            }
        }

        board.fix_pins_set_layer();

        Ok(())
    }
}
