use crate::{
    flow::{
        board::Board,
        execution::{context::ExecutionContext, Cacheable},
        node::{Node, NodeLogic},
        pin::PinOptions,
        variable::VariableType,
    },
    state::FlowLikeState,
    vault::vector::{lancedb::LanceDBVectorStore, VectorStore},
};
use async_trait::async_trait;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::sync::Arc;

#[derive(Default)]
pub struct CuidNode {}

impl CuidNode {
    pub fn new() -> Self {
        CuidNode {}
    }
}

#[async_trait]
impl NodeLogic for CuidNode {
    async fn get_node(&self, _app_state: &FlowLikeState) -> Node {
        let mut node = Node::new(
            "cuid",
            "CUID v2",
            "Generates a Collision Resistant Unique Identifier",
            "Utils",
        );
        node.add_icon("/flow/icons/random.svg");

        node.add_input_pin("exec_in", "Input", "", VariableType::Execution);

        node.add_output_pin("exec_out", "Output", "", VariableType::Execution);

        node.add_output_pin("cuid", "Cuid", "Generated CUID", VariableType::String);

        return node;
    }

    async fn run(&mut self, context: &mut ExecutionContext) -> anyhow::Result<()> {
        let cuid = cuid2::create_id();
        context.set_pin_value("cuid", json!(cuid)).await?;
        context.activate_exec_pin("exec_out").await?;
        Ok(())
    }
}
