use async_trait::async_trait;
use schemars::JsonSchema;
use std::sync::Arc;
use tokio::sync::Mutex;

use crate::{
    flow::{
        board::{commands::Command, Board},
        node::Node,
    },
    state::FlowLikeState,
};
use serde::{Deserialize, Serialize};

#[derive(Clone, Serialize, Deserialize, JsonSchema)]
pub struct UpdateNodeCommand {
    pub old_node: Option<Node>,
    pub node: Node,
}

impl UpdateNodeCommand {
    pub fn new(node: Node) -> Self {
        UpdateNodeCommand {
            node,
            old_node: None,
        }
    }
}

#[async_trait]
impl Command for UpdateNodeCommand {
    async fn execute(
        &mut self,
        board: &mut Board,
        _state: Arc<Mutex<FlowLikeState>>,
    ) -> anyhow::Result<()> {
        self.old_node = board.nodes.insert(self.node.id.clone(), self.node.clone());
        Ok(())
    }

    async fn undo(
        &mut self,
        board: &mut Board,
        _state: Arc<Mutex<FlowLikeState>>,
    ) -> anyhow::Result<()> {
        if let Some(old_node) = self.old_node.take() {
            board.nodes.insert(old_node.id.clone(), old_node.clone());
        } else {
            board.nodes.remove(&self.node.id);
        }
        Ok(())
    }
}
