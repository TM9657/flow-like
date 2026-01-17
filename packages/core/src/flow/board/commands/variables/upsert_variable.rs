use canonical_json::ser::to_string as canonical_json_string;
use flow_like_types::async_trait;
use schemars::JsonSchema;
use std::sync::Arc;

use crate::{
    flow::{
        board::{Board, commands::Command},
        variable::{Variable, VariableType, infer_schema_from_json},
    },
    state::FlowLikeState,
};
use serde::{Deserialize, Serialize};

/// Normalizes a JSON string to canonical format (sorted keys, no extra whitespace)
fn normalize_schema(schema: &str) -> Option<String> {
    serde_json::from_str::<serde_json::Value>(schema)
        .ok()
        .and_then(|v| canonical_json_string(&v).ok())
}

#[derive(Clone, Serialize, Deserialize, JsonSchema)]
pub struct UpsertVariableCommand {
    pub variable: Variable,
    pub old_variable: Option<Variable>,
}

impl UpsertVariableCommand {
    pub fn new(variable: Variable) -> Self {
        UpsertVariableCommand {
            variable,
            old_variable: None,
        }
    }
}

#[async_trait]
impl Command for UpsertVariableCommand {
    async fn execute(
        &mut self,
        board: &mut Board,
        _: Arc<FlowLikeState>,
    ) -> flow_like_types::Result<()> {
        // If the variable is a Struct type and has a schema that looks like example JSON,
        // infer the proper JSON Schema from it. For other types, preserve the schema as-is.
        if self.variable.data_type == VariableType::Struct {
            if let Some(ref schema_str) = self.variable.schema {
                if !schema_str.trim().is_empty() {
                    if let Ok(inferred) = infer_schema_from_json(schema_str) {
                        self.variable.schema = Some(inferred);
                    }
                    // If inference fails, keep the original schema
                }
            }
        }
        // For non-Struct types, keep schema as-is (don't set to None)

        // Normalize schema to canonical JSON format for consistent hashing
        if let Some(ref schema_str) = self.variable.schema {
            if let Some(normalized) = normalize_schema(schema_str) {
                self.variable.schema = Some(normalized);
            }
        }

        if let Some(old_variable) = board
            .variables
            .insert(self.variable.id.clone(), self.variable.clone())
        {
            if !old_variable.editable {
                board
                    .variables
                    .insert(old_variable.id.clone(), old_variable);
                return Err(flow_like_types::anyhow!("Variable is not editable"));
            }

            self.old_variable = Some(old_variable);
        }
        Ok(())
    }

    async fn undo(
        &mut self,
        board: &mut Board,
        _: Arc<FlowLikeState>,
    ) -> flow_like_types::Result<()> {
        board.variables.remove(&self.variable.id);
        if let Some(old_variable) = self.old_variable.take() {
            board
                .variables
                .insert(old_variable.id.clone(), old_variable);
        }
        Ok(())
    }
}
