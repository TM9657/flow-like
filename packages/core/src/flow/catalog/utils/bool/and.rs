use crate::{
    flow::{
        execution::{context::ExecutionContext, LogLevel},
        node::{Node, NodeLogic},
        variable::VariableType,
    },
    state::FlowLikeState,
};
use async_trait::async_trait;
use serde_json::json;

#[derive(Default)]
pub struct BoolAnd {}

impl BoolAnd {
    pub fn new() -> Self {
        BoolAnd {}
    }
}

#[async_trait]
impl NodeLogic for BoolAnd {
    async fn get_node(&self, _app_state: &FlowLikeState) -> Node {
        let mut node = Node::new("bool_and", "And", "Boolean And operation", "Utils/Bool");

        node.add_icon("/flow/icons/bool.svg");

        node.add_input_pin(
            "boolean",
            "Boolean",
            "Input Pin for AND Operation",
            VariableType::Boolean,
        )
        .set_default_value(Some(json!(false)));

        node.add_input_pin(
            "boolean",
            "Boolean",
            "Input Pin for AND Operation",
            VariableType::Boolean,
        )
        .set_default_value(Some(json!(false)));

        node.add_output_pin(
            "result",
            "Result",
            "AND operation between all boolean inputs",
            VariableType::Boolean,
        );

        return node;
    }

    async fn run(&self, context: &mut ExecutionContext) -> anyhow::Result<()> {
        let mut output_value = true;

        let boolean_pins = context.get_pins_by_name("boolean").await?;

        for pin in boolean_pins {
            let pin = context.evaluate_pin_ref(pin).await?;

            output_value = output_value && pin;
            if !output_value {
                break;
            }
        }

        let result = context.get_pin_by_name("result").await?;

        context.log_message(
            &format!("AND Operation Result: {}", output_value),
            LogLevel::Debug,
        );
        context
            .set_pin_ref_value(&result, json!(output_value))
            .await?;

        return Ok(());
    }
}
