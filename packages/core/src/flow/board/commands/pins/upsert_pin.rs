use async_trait::async_trait;
use std::sync::Arc;
use tokio::sync::Mutex;

use crate::{
    flow::{
        board::{Board, Command},
        pin::Pin,
    },
    state::FlowLikeState,
};
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize)]
pub struct UpsertPinCommand {
    pub node_id: String,
    pub pin: Pin,
    pub old_pin: Option<Pin>,
}

impl UpsertPinCommand {
    pub fn new(node_id: String, pin: Pin) -> Self {
        UpsertPinCommand {
            node_id,
            pin,
            old_pin: None,
        }
    }
}

#[async_trait]
impl Command for UpsertPinCommand {
    async fn execute(
        &mut self,
        board: &mut Board,
        _: Arc<Mutex<FlowLikeState>>,
    ) -> anyhow::Result<()> {
        let node = match board.nodes.get_mut(&self.node_id) {
            Some(node) => node,
            None => return Err(anyhow::anyhow!("Node not found".to_string())),
        };

        self.old_pin = node.pins.insert(self.pin.id.clone(), self.pin.clone());

        board.fix_pins();

        Ok(())
    }

    async fn undo(
        &mut self,
        board: &mut Board,
        _: Arc<Mutex<FlowLikeState>>,
    ) -> anyhow::Result<()> {
        let node = match board.nodes.get_mut(&self.node_id) {
            Some(node) => node,
            None => return Err(anyhow::anyhow!("Node not found".to_string())),
        };

        if let Some(old_pin) = self.old_pin.take() {
            node.pins.insert(old_pin.id.clone(), old_pin);
        } else {
            node.pins.remove(&self.pin.id);
        }

        board.fix_pins();

        Ok(())
    }
}
