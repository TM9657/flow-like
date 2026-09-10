use flow_like::flow::{
    execution::context::ExecutionContext,
    node::{Node, NodeLogic},
    variable::VariableType,
};
use flow_like_types::{async_trait, json::json};

#[crate::register_node]
#[derive(Default)]
pub struct BoolEqual {}

impl BoolEqual {
    pub fn new() -> Self {
        BoolEqual {}
    }
}

#[async_trait]
impl NodeLogic for BoolEqual {
    fn get_node(&self) -> Node {
        let mut node = Node::new("bool_equal", "== (Bool)", "Boolean Equal", "Utils/Bool");
        node.set_flowscript_name("bool", "equal");
        node.set_receiver("boolean");
        node.add_icon("/flow/icons/bool.svg");

        node.add_input_pin(
            "boolean",
            "Boolean",
            "Boolean value to compare",
            VariableType::Boolean,
        )
        .set_default_value(Some(json!(false)));

        node.add_input_pin(
            "boolean",
            "Boolean",
            "Boolean value to compare",
            VariableType::Boolean,
        )
        .set_default_value(Some(json!(false)));

        node.add_output_pin(
            "result",
            "Result",
            "== operation between all boolean inputs",
            VariableType::Boolean,
        );

        node
    }

    async fn run(&self, context: &mut ExecutionContext) -> flow_like_types::Result<()> {
        let mut first_value = None;
        let mut all_equal = true;

        let boolean_pins = context.get_pins_by_name("boolean").await?;

        for pin in boolean_pins {
            let pin: bool = context.evaluate_pin_ref(pin).await?;

            if first_value.is_none() {
                first_value = Some(pin);
                continue;
            }

            if first_value != Some(pin) {
                all_equal = false;
                break;
            }
        }

        let output_value = first_value.is_some() && all_equal;

        context.set_pin_value("result", json!(output_value)).await?;

        return Ok(());
    }
}
