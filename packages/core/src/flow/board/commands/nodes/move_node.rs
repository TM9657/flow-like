use async_trait::async_trait;
use std::sync::Arc;
use tokio::sync::Mutex;

use crate::{
    flow::board::{Board, Command},
    state::FlowLikeState,
};
use serde::{Deserialize, Serialize};
#[derive(Serialize, Deserialize)]
pub struct MoveNodeCommand {
    pub node_id: String,
    pub from_coordinates: Option<(f32, f32, f32)>,
    pub to_coordinates: (f32, f32, f32),
}

impl MoveNodeCommand {
    pub fn new(node_id: String, to_coordinates: (f32, f32, f32)) -> Self {
        MoveNodeCommand {
            node_id,
            from_coordinates: None,
            to_coordinates,
        }
    }
}

#[async_trait]
impl Command for MoveNodeCommand {
    async fn execute(
        &mut self,
        board: &mut Board,
        _: Arc<Mutex<FlowLikeState>>,
    ) -> anyhow::Result<()> {
        let node = match board.nodes.get_mut(&self.node_id) {
            Some(node) => node,
            None => return Err(anyhow::anyhow!(format!("Node {} not found", self.node_id))),
        };

        self.from_coordinates = node.coordinates;
        node.coordinates = Some(self.to_coordinates);

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

        node.coordinates = self.from_coordinates;

        Ok(())
    }
}
