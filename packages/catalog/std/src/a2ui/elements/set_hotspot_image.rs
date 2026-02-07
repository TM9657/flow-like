use super::element_utils::extract_element_id;
use flow_like::a2ui::components::ImageHotspotProps;
use flow_like::flow::{
    execution::context::ExecutionContext,
    node::{Node, NodeLogic},
    pin::PinOptions,
    variable::VariableType,
};
use flow_like_types::{Value, async_trait, json::json};

/// Sets the source image for an ImageHotspot element.
#[crate::register_node]
#[derive(Default)]
pub struct SetHotspotImage;

impl SetHotspotImage {
    pub fn new() -> Self {
        Self
    }
}

#[async_trait]
impl NodeLogic for SetHotspotImage {
    fn get_node(&self) -> Node {
        let mut node = Node::new(
            "a2ui_set_hotspot_image",
            "Set Hotspot Image",
            "Sets the source image for an ImageHotspot element",
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
            "src",
            "Image Source",
            "URL or base64 source of the image",
            VariableType::String,
        );

        node.add_input_pin(
            "alt",
            "Alt Text",
            "Alternative text for the image",
            VariableType::String,
        )
        .set_default_value(Some(json!("")));

        node.add_input_pin(
            "clear_hotspots",
            "Clear Hotspots",
            "Clear all hotspots when setting new image",
            VariableType::Boolean,
        )
        .set_default_value(Some(json!(false)));

        node.add_output_pin("exec_out", "▶", "Execution output", VariableType::Execution);

        node.set_long_running(true);

        node
    }

    async fn run(&self, context: &mut ExecutionContext) -> flow_like_types::Result<()> {
        context.deactivate_exec_pin("exec_out").await?;

        let element_value: Value = context.evaluate_pin("element_ref").await?;
        let element_id = extract_element_id(&element_value)
            .ok_or_else(|| flow_like_types::anyhow!("Invalid element reference"))?;

        let src: String = context.evaluate_pin("src").await?;
        let alt: String = context.evaluate_pin("alt").await?;
        let clear_hotspots: bool = context.evaluate_pin("clear_hotspots").await?;

        let update_value = if clear_hotspots {
            json!({
                "type": "setHotspotImage",
                "src": src,
                "alt": alt,
                "hotspots": []
            })
        } else {
            json!({
                "type": "setHotspotImage",
                "src": src,
                "alt": alt
            })
        };

        context.upsert_element(&element_id, update_value).await?;
        context.activate_exec_pin("exec_out").await?;

        Ok(())
    }
}
