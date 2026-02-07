use super::element_utils::extract_element_id;
use flow_like::a2ui::components::ImageHotspotProps;
use flow_like::flow::{
    execution::context::ExecutionContext,
    node::{Node, NodeLogic},
    pin::PinOptions,
    variable::VariableType,
};
use flow_like_types::{Value, async_trait, json::json};

/// Clears all hotspots from an ImageHotspot element.
#[crate::register_node]
#[derive(Default)]
pub struct ClearHotspots;

impl ClearHotspots {
    pub fn new() -> Self {
        Self
    }
}

#[async_trait]
impl NodeLogic for ClearHotspots {
    fn get_node(&self) -> Node {
        let mut node = Node::new(
            "a2ui_clear_hotspots",
            "Clear Hotspots",
            "Removes all hotspots from an ImageHotspot element",
            "UI/Elements/Hotspot",
        );
        node.add_icon("/flow/icons/a2ui.svg");

        node.add_input_pin("exec_in", "▶", "Execution input", VariableType::Execution);

        node.add_input_pin(
            "element_ref",
            "Hotspot Image",
            "Reference to the ImageHotspot element",
            VariableType::Struct,
        )
        .set_schema::<ImageHotspotProps>()
        .set_options(PinOptions::new().set_enforce_schema(false).build());

        node.add_output_pin("exec_out", "▶", "Execution output", VariableType::Execution);

        node.set_long_running(true);

        node
    }

    async fn run(&self, context: &mut ExecutionContext) -> flow_like_types::Result<()> {
        context.deactivate_exec_pin("exec_out").await?;

        let element_value: Value = context.evaluate_pin("element_ref").await?;
        let element_id = extract_element_id(&element_value)
            .ok_or_else(|| flow_like_types::anyhow!("Invalid element reference"))?;

        context
            .upsert_element(&element_id, json!({"hotspots": []}))
            .await?;
        context.activate_exec_pin("exec_out").await?;

        Ok(())
    }
}
