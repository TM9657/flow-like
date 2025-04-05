use flow_like_types::{async_trait, sync::Mutex};
use schemars::JsonSchema;
use std::sync::Arc;


use crate::{
    flow::board::{Board, commands::Command},
    state::FlowLikeState,
};
use serde::{Deserialize, Serialize};
#[derive(Clone, Serialize, Deserialize, JsonSchema)]
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
    ) -> flow_like_types::Result<()> {
        let node = match board.nodes.get_mut(&self.node_id) {
            Some(node) => node,
            None => return Err(flow_like_types::anyhow!(format!("Node {} not found", self.node_id))),
        };

        self.from_coordinates = node.coordinates;
        node.coordinates = Some(self.to_coordinates);

        Ok(())
    }

    async fn undo(
        &mut self,
        board: &mut Board,
        _: Arc<Mutex<FlowLikeState>>,
    ) -> flow_like_types::Result<()> {
        let node = match board.nodes.get_mut(&self.node_id) {
            Some(node) => node,
            None => return Err(flow_like_types::anyhow!("Node not found".to_string())),
        };

        node.coordinates = self.from_coordinates;

        Ok(())
    }
}
