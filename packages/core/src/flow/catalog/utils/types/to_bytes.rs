use std::sync::Arc;

use crate::{
    flow::{
        board::Board, execution::context::ExecutionContext, node::{Node, NodeLogic}, pin::{PinOptions, ValueType}, variable::VariableType
    },
    state::FlowLikeState,
};
use ahash::HashMap;
use async_trait::async_trait;

#[derive(Default)]
pub struct ToBytesNode {}

impl ToBytesNode {
    pub fn new() -> Self {
        ToBytesNode {}
    }
}

#[async_trait]
impl NodeLogic for ToBytesNode {
    async fn get_node(&self, _app_state: &FlowLikeState) -> Node {
        let mut node = Node::new(
            "val_to_bytes",
            "To Bytes",
            "Convert Struct to Bytes",
            "Utils/Conversions",
        );
        node.add_icon("/flow/icons/convert.svg");

        node.add_input_pin("value", "Value", "Input Value", VariableType::Generic);
        node.add_input_pin("pretty", "Pretty?", "Should the struct be pretty printed?", VariableType::Boolean);
    
        node.add_output_pin(
            "bytes",
            "Bytes",
            "Output Bytes",
            VariableType::Byte,
        ).set_value_type(crate::flow::pin::ValueType::Array);

        return node;
    }

    async fn run(&mut self, context: &mut ExecutionContext) -> anyhow::Result<()> {
        let value: serde_json::Value = context.evaluate_pin("value").await?;
        let pretty = context.evaluate_pin::<bool>("pretty").await?;
        let bytes: Vec<u8> = if pretty {
            serde_json::to_vec_pretty(&value)?
        } else {
            serde_json::to_vec(&value)?
        };
        context.set_pin_value("bytes", serde_json::json!(bytes)).await?;
        Ok(())
    }

    async fn on_update(&self, node: &mut Node, board: Arc<Board>) {
        let match_type = node.match_type("value", board, None);

        if match_type.is_err() {
            eprintln!("Error: {:?}", match_type.err());
        }
    }
}
