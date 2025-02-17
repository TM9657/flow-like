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
pub struct ToStringNode {}

impl ToStringNode {
    pub fn new() -> Self {
        ToStringNode {}
    }
}

#[async_trait]
impl NodeLogic for ToStringNode {
    async fn get_node(&self, _app_state: &FlowLikeState) -> Node {
        let mut node = Node::new(
            "val_to_string",
            "To String",
            "Convert any object to String",
            "Utils/Conversions",
        );
        node.add_icon("/flow/icons/convert.svg");

        node.add_input_pin("value", "Value", "Input Value", VariableType::Generic);
        node.add_input_pin("pretty", "Pretty?", "Should the struct be pretty printed?", VariableType::Boolean);
        node.add_input_pin("type", "Variable Type", "Variable Type", VariableType::String).set_default_value(Some(serde_json::json!("Normal"))).set_options(PinOptions::new().set_valid_values(vec![
            "Normal".to_string(),
            "Array".to_string(),
            "HashSet".to_string(),
            "HashMap".to_string(),
        ]).build());

        node.add_output_pin(
            "string",
            "String",
            "Output String",
            VariableType::String,
        );

        return node;
    }

    async fn run(&mut self, context: &mut ExecutionContext) -> anyhow::Result<()> {
        let string: serde_json::Value = context.evaluate_pin("value").await?;
        let pretty = context.evaluate_pin::<bool>("pretty").await?;
        let value: String = if pretty {
            serde_json::to_string_pretty(&string)?
        } else {
            serde_json::to_string(&string)?
        };
        context.set_pin_value("string", serde_json::json!(value)).await?;
        Ok(())
    }

    async fn on_update(&self, node: &mut Node, board: Arc<Board>) {
        let mut value_type = ValueType::Normal;
        let var_type = node.get_pin_by_name("type").unwrap().default_value.clone();

        if let Some(var_type) = var_type {
            let parsed : serde_json::Value = serde_json::from_slice(&var_type).unwrap();
            let parsed : String = serde_json::from_value(parsed).unwrap();
            match parsed.as_str() {
                "Normal" => value_type = ValueType::Normal,
                "Array" => value_type = ValueType::Array,
                "HashSet" => value_type = ValueType::HashSet,
                "HashMap" => value_type = ValueType::HashMap,
                _ => value_type = ValueType::Normal,
            }
        }

        let match_type = node.match_type("value", board, Some(value_type));

        if match_type.is_err() {
            eprintln!("Error: {:?}", match_type.err());
        }
    }
}
