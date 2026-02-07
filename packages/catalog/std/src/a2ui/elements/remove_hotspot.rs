use super::element_utils::extract_element_id;
use flow_like::a2ui::components::ImageHotspotProps;
use flow_like::flow::{
    execution::context::ExecutionContext,
    node::{Node, NodeLogic},
    pin::PinOptions,
    variable::VariableType,
};
use flow_like_types::{Value, async_trait, json::json};

/// Removes a hotspot from an ImageHotspot element by ID.
#[crate::register_node]
#[derive(Default)]
pub struct RemoveHotspot;

impl RemoveHotspot {
    pub fn new() -> Self {
        Self
    }
}

#[async_trait]
impl NodeLogic for RemoveHotspot {
    fn get_node(&self) -> Node {
        let mut node = Node::new(
            "a2ui_remove_hotspot",
            "Remove Hotspot",
            "Removes a hotspot from an ImageHotspot by ID",
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

        node.add_input_pin(
            "hotspot_id",
            "Hotspot ID",
            "ID of the hotspot to remove",
            VariableType::String,
        );

        node.add_output_pin("exec_out", "▶", "Execution output", VariableType::Execution);

        node.set_long_running(true);

        node
    }

    async fn run(&self, context: &mut ExecutionContext) -> flow_like_types::Result<()> {
        context.deactivate_exec_pin("exec_out").await?;

        let element_value: Value = context.evaluate_pin("element_ref").await?;
        let element_id = extract_element_id(&element_value)
            .ok_or_else(|| flow_like_types::anyhow!("Invalid element reference"))?;

        let hotspot_id: String = context.evaluate_pin("hotspot_id").await?;

        let update_value = json!({
            "type": "removeHotspot",
            "hotspotId": hotspot_id
        });

        context.upsert_element(&element_id, update_value).await?;
        context.activate_exec_pin("exec_out").await?;

        Ok(())
    }
}
