use std::sync::Arc;

use flow_like_types::{async_trait, sync::Mutex};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::{
    flow::node::{Node, NodeLogic},
    state::FlowLikeState,
};

use super::Board;

pub mod comments;
pub mod nodes;
pub mod pins;
pub mod variables;

#[async_trait]
pub trait Command: Send + Sync {
    async fn execute(
        &mut self,
        board: &mut Board,
        state: Arc<Mutex<FlowLikeState>>,
    ) -> flow_like_types::Result<()>;
    async fn undo(
        &mut self,
        board: &mut Board,
        state: Arc<Mutex<FlowLikeState>>,
    ) -> flow_like_types::Result<()>;

    async fn node_to_logic(
        &self,
        node: &Node,
        state: Arc<Mutex<FlowLikeState>>,
    ) -> flow_like_types::Result<Arc<dyn NodeLogic>> {
        let node_registry = {
            let state_guard = state.lock().await;
            state_guard.node_registry().clone()
        };

        let registry_guard = node_registry.read().await;

        registry_guard.instantiate(node)
    }
}

#[derive(Clone, Serialize, Deserialize, JsonSchema)]
#[serde(tag = "command_type")]
pub enum GenericCommand {
    RemoveComment(comments::remove_comment::RemoveCommentCommand),
    UpsertComment(comments::upsert_comment::UpsertCommentCommand),
    AddNode(nodes::add_node::AddNodeCommand),
    CopyPaste(nodes::copy_paste::CopyPasteCommand),
    MoveNode(nodes::move_node::MoveNodeCommand),
    RemoveNode(nodes::remove_node::RemoveNodeCommand),
    UpdateNode(nodes::update_node::UpdateNodeCommand),
    DisconnectPin(pins::disconnect_pins::DisconnectPinsCommand),
    ConnectPin(pins::connect_pins::ConnectPinsCommand),
    UpsertPin(pins::upsert_pin::UpsertPinCommand),
    RemoveVariable(variables::remove_variable::RemoveVariableCommand),
    UpsertVariable(variables::upsert_variable::UpsertVariableCommand),
}

impl GenericCommand {
    pub fn to_dyn(&self) -> Arc<Mutex<dyn Command>> {
        match self {
            GenericCommand::RemoveComment(cmd) => Arc::new(Mutex::new(cmd.clone())),
            GenericCommand::UpsertComment(cmd) => Arc::new(Mutex::new(cmd.clone())),
            GenericCommand::AddNode(cmd) => Arc::new(Mutex::new(cmd.clone())),
            GenericCommand::CopyPaste(cmd) => Arc::new(Mutex::new(cmd.clone())),
            GenericCommand::MoveNode(cmd) => Arc::new(Mutex::new(cmd.clone())),
            GenericCommand::RemoveNode(cmd) => Arc::new(Mutex::new(cmd.clone())),
            GenericCommand::UpdateNode(cmd) => Arc::new(Mutex::new(cmd.clone())),
            GenericCommand::DisconnectPin(cmd) => Arc::new(Mutex::new(cmd.clone())),
            GenericCommand::ConnectPin(cmd) => Arc::new(Mutex::new(cmd.clone())),
            GenericCommand::UpsertPin(cmd) => Arc::new(Mutex::new(cmd.clone())),
            GenericCommand::RemoveVariable(cmd) => Arc::new(Mutex::new(cmd.clone())),
            GenericCommand::UpsertVariable(cmd) => Arc::new(Mutex::new(cmd.clone())),
        }
    }

    pub async fn execute(
        &mut self,
        board: &mut Board,
        state: Arc<Mutex<FlowLikeState>>,
    ) -> flow_like_types::Result<()> {
        let command = match self {
            GenericCommand::RemoveComment(cmd) => cmd.execute(board, state),
            GenericCommand::UpsertComment(cmd) => cmd.execute(board, state),
            GenericCommand::AddNode(cmd) => cmd.execute(board, state),
            GenericCommand::CopyPaste(cmd) => cmd.execute(board, state),
            GenericCommand::MoveNode(cmd) => cmd.execute(board, state),
            GenericCommand::RemoveNode(cmd) => cmd.execute(board, state),
            GenericCommand::UpdateNode(cmd) => cmd.execute(board, state),
            GenericCommand::DisconnectPin(cmd) => cmd.execute(board, state),
            GenericCommand::ConnectPin(cmd) => cmd.execute(board, state),
            GenericCommand::UpsertPin(cmd) => cmd.execute(board, state),
            GenericCommand::RemoveVariable(cmd) => cmd.execute(board, state),
            GenericCommand::UpsertVariable(cmd) => cmd.execute(board, state),
        };
        command.await
    }

    pub async fn undo(
        &mut self,
        board: &mut Board,
        state: Arc<Mutex<FlowLikeState>>,
    ) -> flow_like_types::Result<()> {
        let command = match self {
            GenericCommand::RemoveComment(cmd) => cmd.undo(board, state),
            GenericCommand::UpsertComment(cmd) => cmd.undo(board, state),
            GenericCommand::AddNode(cmd) => cmd.undo(board, state),
            GenericCommand::CopyPaste(cmd) => cmd.undo(board, state),
            GenericCommand::MoveNode(cmd) => cmd.undo(board, state),
            GenericCommand::RemoveNode(cmd) => cmd.undo(board, state),
            GenericCommand::UpdateNode(cmd) => cmd.undo(board, state),
            GenericCommand::DisconnectPin(cmd) => cmd.undo(board, state),
            GenericCommand::ConnectPin(cmd) => cmd.undo(board, state),
            GenericCommand::UpsertPin(cmd) => cmd.undo(board, state),
            GenericCommand::RemoveVariable(cmd) => cmd.undo(board, state),
            GenericCommand::UpsertVariable(cmd) => cmd.undo(board, state),
        };
        command.await
    }
}
