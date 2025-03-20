use crate::{
    flow::{
        execution::context::ExecutionContext,
        node::{Node, NodeLogic},
        variable::VariableType,
    },
    state::FlowLikeState,
};
use async_trait::async_trait;
use serde_json::json;

#[derive(Default)]
pub struct BoolXor {}

impl BoolXor {
    pub fn new() -> Self {
        BoolXor {}
    }
}

#[async_trait]
impl NodeLogic for BoolXor {
    async fn get_node(&self, _app_state: &FlowLikeState) -> Node {
        let mut node = Node::new("bool_xor", "^", "Boolean XOR", "Utils/Bool");
        node.add_icon("/flow/icons/bool.svg");

        node.add_input_pin("boolean", "Boolean", "Input Boolean", VariableType::Boolean)
            .set_default_value(Some(json!(false)));

        node.add_input_pin("boolean", "Boolean", "Input Boolean", VariableType::Boolean)
            .set_default_value(Some(json!(false)));

        node.add_output_pin(
            "result",
            "Result",
            "XOR operation between all boolean inputs",
            VariableType::Boolean,
        );

        return node;
    }

    async fn run(&self, context: &mut ExecutionContext) -> anyhow::Result<()> {
        let mut output_value: Option<bool> = None;

        let boolean_pins = context.get_pins_by_name("boolean").await?;

        for pin in boolean_pins {
            let pin: bool = context.evaluate_pin_ref(pin).await?;

            output_value = match output_value {
                Some(val) => Some(val ^ pin),
                None => Some(pin),
            };
        }

        let output_value = output_value.unwrap_or(false); // Default to false if no inputs

        context.set_pin_value("result", json!(output_value)).await?;

        return Ok(());
    }
}
