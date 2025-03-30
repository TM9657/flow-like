use async_trait::async_trait;
use schemars::JsonSchema;
use std::sync::Arc;
use tokio::sync::Mutex;

use crate::{
    flow::board::{commands::Command, Board, Comment},
    state::FlowLikeState,
};
use serde::{Deserialize, Serialize};

#[derive(Clone, Serialize, Deserialize, JsonSchema)]
pub struct UpsertCommentCommand {
    pub comment: Comment,
    pub old_comment: Option<Comment>,
}

impl UpsertCommentCommand {
    pub fn new(comment: Comment) -> Self {
        UpsertCommentCommand {
            comment,
            old_comment: None,
        }
    }
}

#[async_trait]
impl Command for UpsertCommentCommand {
    async fn execute(
        &mut self,
        board: &mut Board,
        _: Arc<Mutex<FlowLikeState>>,
    ) -> anyhow::Result<()> {
        if let Some(old_variable) = board
            .comments
            .insert(self.comment.id.clone(), self.comment.clone())
        {
            self.old_comment = Some(old_variable);
        }

        Ok(())
    }

    async fn undo(
        &mut self,
        board: &mut Board,
        _: Arc<Mutex<FlowLikeState>>,
    ) -> anyhow::Result<()> {
        board.comments.remove(&self.comment.id);
        if let Some(old_comment) = self.old_comment.take() {
            board.comments.insert(old_comment.id.clone(), old_comment);
        }
        Ok(())
    }
}
