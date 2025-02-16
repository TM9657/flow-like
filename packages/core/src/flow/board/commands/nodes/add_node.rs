use async_trait::async_trait;
use std::sync::Arc;
use tokio::sync::Mutex;

use crate::{
    flow::{
        board::{Board, Command},
        node::Node,
    },
    state::FlowLikeState,
};
use serde::{Deserialize, Serialize};

use cuid2;

#[derive(Serialize, Deserialize)]
pub struct AddNodeCommand {
    pub node: Node,
}

impl AddNodeCommand {
    pub fn new(node: Node) -> Self {
        // we randomize the node id and pin ids to avoid conflicts
        let mut node = node;
        node.id = cuid2::cuid();

        let pin_ids: Vec<_> = node.pins.keys().cloned().collect();
        for pin_id in pin_ids {
            if let Some(mut pin) = node.pins.remove(&pin_id) {
                pin.id = cuid2::cuid();
                node.pins.insert(pin.id.clone(), pin);
            }
        }

        AddNodeCommand { node }
    }
}

#[async_trait]
impl Command for AddNodeCommand {
    async fn execute(
        &mut self,
        board: &mut Board,
        _state: Arc<Mutex<FlowLikeState>>,
    ) -> anyhow::Result<()> {
        board.nodes.insert(self.node.id.clone(), self.node.clone());
        Ok(())
    }

    async fn undo(
        &mut self,
        board: &mut Board,
        _: Arc<Mutex<FlowLikeState>>,
    ) -> anyhow::Result<()> {
        board.nodes.remove(&self.node.id);
        Ok(())
    }
}
