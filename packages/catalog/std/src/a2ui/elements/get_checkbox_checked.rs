use flow_like::flow::{
    board::Board,
    execution::{LogLevel, context::ExecutionContext},
    node::{Node, NodeLogic},
    pin::PinOptions,
    variable::VariableType,
};
use flow_like::a2ui::components::CheckboxProps;
use flow_like_types::{Value, async_trait};
use std::sync::Arc;

use super::element_utils::{find_element, extract_element_id_from_pin};

/// Gets the checked state of a checkbox or switch element.
#[crate::register_node]
#[derive(Default)]
pub struct GetCheckboxChecked;

impl GetCheckboxChecked {
    pub fn new() -> Self {
        Self
    }
}

#[async_trait]
impl NodeLogic for GetCheckboxChecked {
    fn get_node(&self) -> Node {
        let mut node = Node::new(
            "a2ui_get_checkbox_checked",
            "Get Checkbox Checked",
            "Gets whether a checkbox or switch is checked",
            "A2UI/Elements/Checkbox",
        );
        node.add_icon("/flow/icons/a2ui.svg");

        node.add_input_pin(
            "element_ref",
            "Checkbox",
            "Reference to the checkbox or switch element",
            VariableType::Struct,
        )
        .set_schema::<CheckboxProps>()
        .set_options(PinOptions::new().set_enforce_schema(false).build());

        node.add_output_pin("checked", "Checked", "Whether the checkbox is checked", VariableType::Boolean);

        node
    }

    async fn run(&self, context: &mut ExecutionContext) -> flow_like_types::Result<()> {
        let element_value: Value = context.evaluate_pin("element_ref").await?;
        let element_id = extract_element_id_from_pin(element_value)
            .ok_or_else(|| flow_like_types::anyhow!("Invalid element reference"))?;

        let elements = context.get_frontend_elements().await?;
        let element = elements.as_ref().and_then(|e| find_element(e, &element_id));

        let checked = element
            .map(|(_, el)| el)
            .and_then(|el| el.get("component"))
            .and_then(|c| c.get("checked").or_else(|| c.get("value")))
            .and_then(|v| v.as_bool())
            .unwrap_or(false);

        context.get_pin_by_name("checked").await?.set_value(Value::Bool(checked)).await;

        Ok(())
    }

    async fn on_update(&self, node: &mut Node, _board: Arc<Board>) {
        node.error = None;
    }
}
