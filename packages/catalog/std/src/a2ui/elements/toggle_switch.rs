use flow_like::flow::{
    execution::context::ExecutionContext,
    node::{Node, NodeLogic},
    pin::PinOptions,
    variable::VariableType,
};
use flow_like::a2ui::components::SwitchProps;
use flow_like_types::{Value, async_trait, json::json};
use super::element_utils::{extract_element_id, find_element};

/// Toggles the state of a switch element.
#[crate::register_node]
#[derive(Default)]
pub struct ToggleSwitch;

impl ToggleSwitch {
    pub fn new() -> Self {
        Self
    }
}

#[async_trait]
impl NodeLogic for ToggleSwitch {
    fn get_node(&self) -> Node {
        let mut node = Node::new(
            "a2ui_toggle_switch",
            "Toggle Switch",
            "Toggles the on/off state of a switch element",
            "A2UI/Elements/Switch",
        );
        node.add_icon("/flow/icons/a2ui.svg");

        node.add_input_pin("exec_in", "▶", "Execution input", VariableType::Execution);

        node.add_input_pin(
            "element_ref",
            "Switch",
            "Reference to the switch element",
            VariableType::Struct,
        )
        .set_schema::<SwitchProps>()
        .set_options(PinOptions::new().set_enforce_schema(false).build());

        node.add_output_pin("exec_out", "▶", "Execution output", VariableType::Execution);
        node.add_output_pin("new_state", "New State", "The new checked state after toggle", VariableType::Boolean);

        node.set_long_running(true);

        node
    }

    async fn run(&self, context: &mut ExecutionContext) -> flow_like_types::Result<()> {
        context.deactivate_exec_pin("exec_out").await?;

        let element_value: Value = context.evaluate_pin("element_ref").await?;
        let element_id = extract_element_id(&element_value)
            .ok_or_else(|| flow_like_types::anyhow!("Invalid element reference"))?;

        let elements = context.get_frontend_elements().await?;
        let element = elements.as_ref().and_then(|e| find_element(e, &element_id));

        let current_checked = element
            .map(|(_, el)| el)
            .and_then(|el| el.get("component"))
            .and_then(|c| c.get("checked").or_else(|| c.get("value")))
            .and_then(|v| v.as_bool())
            .unwrap_or(false);

        let new_checked = !current_checked;

        context.upsert_element(&element_id, json!({
            "type": "setChecked",
            "checked": new_checked
        })).await?;

        context.get_pin_by_name("new_state").await?.set_value(Value::Bool(new_checked)).await;
        context.activate_exec_pin("exec_out").await?;

        Ok(())
    }
}
