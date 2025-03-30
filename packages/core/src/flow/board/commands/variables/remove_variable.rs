use async_trait::async_trait;
use schemars::JsonSchema;
use std::sync::Arc;
use tokio::sync::Mutex;

use crate::{
    flow::{
        board::{commands::Command, Board},
        variable::Variable,
    },
    state::FlowLikeState,
};
use serde::{Deserialize, Serialize};

#[derive(Clone, Serialize, Deserialize, JsonSchema)]
pub struct RemoveVariableCommand {
    pub variable: Variable,
}

impl RemoveVariableCommand {
    pub fn new(variable: Variable) -> Self {
        RemoveVariableCommand { variable }
    }
}

#[async_trait]
impl Command for RemoveVariableCommand {
    async fn execute(
        &mut self,
        board: &mut Board,
        _: Arc<Mutex<FlowLikeState>>,
    ) -> anyhow::Result<()> {
        let old_variable = board.variables.remove(&self.variable.id);

        if let Some(old_variable) = old_variable {
            if !old_variable.editable {
                board
                    .variables
                    .insert(old_variable.id.clone(), old_variable);
                return Err(anyhow::anyhow!("Variable is not editable"));
            }

            self.variable = old_variable;
        }

        Ok(())
    }

    async fn undo(
        &mut self,
        board: &mut Board,
        _: Arc<Mutex<FlowLikeState>>,
    ) -> anyhow::Result<()> {
        board
            .variables
            .insert(self.variable.id.clone(), self.variable.clone());
        Ok(())
    }
}
