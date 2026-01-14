use flow_like::flow::{
    board::Board,
    execution::{LogLevel, context::ExecutionContext},
    node::{Node, NodeLogic},
    pin::PinOptions,
    variable::VariableType,
};
use flow_like::a2ui::components::CheckboxProps;
use flow_like_types::{Value, async_trait, json::json};
use std::sync::Arc;

use super::element_utils::extract_element_id;

/// Sets the checked state of a checkbox or switch element.
#[crate::register_node]
#[derive(Default)]
pub struct SetCheckboxChecked;

impl SetCheckboxChecked {
    pub fn new() -> Self {
        Self
    }
}

#[async_trait]
impl NodeLogic for SetCheckboxChecked {
    fn get_node(&self) -> Node {
        let mut node = Node::new(
            "a2ui_set_checkbox_checked",
            "Set Checkbox Checked",
            "Sets the checked state of a checkbox or switch",
            "A2UI/Elements/Checkbox",
        );
        node.add_icon("/flow/icons/a2ui.svg");

        node.add_input_pin("exec_in", "▶", "Execution input", VariableType::Execution);

        node.add_input_pin(
            "element_ref",
            "Checkbox",
            "Element ID string or element object from Get Element",
            VariableType::Struct,
        )
        .set_schema::<CheckboxProps>()
        .set_options(PinOptions::new().set_enforce_schema(false).build());

        node.add_input_pin(
            "checked",
            "Checked",
            "Whether the checkbox should be checked",
            VariableType::Boolean,
        );

        node.add_output_pin("exec_out", "▶", "Execution output", VariableType::Execution);

        node
    }

    async fn run(&self, context: &mut ExecutionContext) -> flow_like_types::Result<()> {
        context.activate_exec_pin("exec_out").await?;

        let element_value: Value = context.evaluate_pin("element_ref").await?;
        let element_id = extract_element_id(&element_value)
            .ok_or_else(|| flow_like_types::anyhow!("Invalid element reference - expected string ID or element object"))?;
        let checked: bool = context.evaluate_pin("checked").await?;

        context.upsert_element(&element_id, json!({
            "type": "setChecked",
            "checked": checked
        })).await?;

        context.log_message(&format!("Set checkbox checked: {} = {}", element_id, checked), LogLevel::Debug);

        Ok(())
    }

    async fn on_update(&self, node: &mut Node, _board: Arc<Board>) {
        node.error = None;
    }
}
