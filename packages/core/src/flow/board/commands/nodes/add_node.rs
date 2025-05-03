use flow_like_types::async_trait;
use flow_like_types::create_id;
use flow_like_types::sync::Mutex;
use schemars::JsonSchema;
use std::sync::Arc;

use crate::{
    flow::{
        board::{Board, commands::Command},
        node::Node,
    },
    state::FlowLikeState,
};
use serde::{Deserialize, Serialize};

#[derive(Clone, Serialize, Deserialize, JsonSchema)]
pub struct AddNodeCommand {
    pub node: Node,
    pub current_layer: Option<String>,
}

impl AddNodeCommand {
    pub fn new(node: Node) -> Self {
        // we randomize the node id and pin ids to avoid conflicts
        let mut node = node;
        node.id = create_id();

        let pin_ids: Vec<_> = node.pins.keys().cloned().collect();
        for pin_id in pin_ids {
            if let Some(mut pin) = node.pins.remove(&pin_id) {
                pin.id = create_id();
                node.pins.insert(pin.id.clone(), pin);
            }
        }

        AddNodeCommand {
            node,
            current_layer: None,
        }
    }
}

#[async_trait]
impl Command for AddNodeCommand {
    async fn execute(
        &mut self,
        board: &mut Board,
        _state: Arc<Mutex<FlowLikeState>>,
    ) -> flow_like_types::Result<()> {
        self.node.layer = self.current_layer.clone();
        board.nodes.insert(self.node.id.clone(), self.node.clone());
        Ok(())
    }

    async fn undo(
        &mut self,
        board: &mut Board,
        _: Arc<Mutex<FlowLikeState>>,
    ) -> flow_like_types::Result<()> {
        board.nodes.remove(&self.node.id);
        Ok(())
    }
}
