use async_trait::async_trait;
use std::sync::Arc;
use tokio::sync::Mutex;

use crate::{
    flow::board::{Board, Command, Comment},
    state::FlowLikeState,
};
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize)]
pub struct RemoveCommentCommand {
    pub comment: Comment,
}

impl RemoveCommentCommand {
    pub fn new(comment: Comment) -> Self {
        RemoveCommentCommand { comment }
    }
}

#[async_trait]
impl Command for RemoveCommentCommand {
    async fn execute(
        &mut self,
        board: &mut Board,
        _: Arc<Mutex<FlowLikeState>>,
    ) -> anyhow::Result<()> {
        board.comments.remove(&self.comment.id);
        Ok(())
    }

    async fn undo(
        &mut self,
        board: &mut Board,
        _: Arc<Mutex<FlowLikeState>>,
    ) -> anyhow::Result<()> {
        board
            .comments
            .insert(self.comment.id.clone(), self.comment.clone());
        Ok(())
    }
}
