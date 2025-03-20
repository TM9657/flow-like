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
pub struct BoolEqual {}

impl BoolEqual {
    pub fn new() -> Self {
        BoolEqual {}
    }
}

#[async_trait]
impl NodeLogic for BoolEqual {
    async fn get_node(&self, _app_state: &FlowLikeState) -> Node {
        let mut node = Node::new("bool_equal", "==", "Boolean Equal", "Utils/Bool");
        node.add_icon("/flow/icons/bool.svg");

        node.add_input_pin(
            "boolean",
            "Boolean",
            "Input Pin for OR Operation",
            VariableType::Boolean,
        )
        .set_default_value(Some(json!(false)));

        node.add_input_pin(
            "boolean",
            "Boolean",
            "Input Pin for OR Operation",
            VariableType::Boolean,
        )
        .set_default_value(Some(json!(false)));

        node.add_output_pin(
            "result",
            "Result",
            "== operation between all boolean inputs",
            VariableType::Boolean,
        );

        return node;
    }

    async fn run(&self, context: &mut ExecutionContext) -> anyhow::Result<()> {
        let mut output_value = None;

        let boolean_pins = context.get_pins_by_name("boolean").await?;

        for pin in boolean_pins {
            let pin: bool = context.evaluate_pin_ref(pin).await?;

            if output_value.is_none() {
                output_value = Some(pin);
                continue;
            }

            let out = output_value.unwrap();
            if out != pin {
                output_value = Some(false);
                break;
            }
        }

        let output_value = output_value.unwrap_or(false);

        context.set_pin_value("result", json!(output_value)).await?;

        return Ok(());
    }
}
