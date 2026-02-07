use super::element_utils::extract_element_id;
use flow_like::a2ui::components::GeoMapProps;
use flow_like::flow::{
    execution::context::ExecutionContext,
    node::{Node, NodeLogic},
    pin::PinOptions,
    variable::VariableType,
};
use flow_like_types::{Value, async_trait, json::json};

/// Sets all markers on a GeoMap element, replacing any existing markers.
#[crate::register_node]
#[derive(Default)]
pub struct SetGeoMapMarkers;

impl SetGeoMapMarkers {
    pub fn new() -> Self {
        Self
    }
}

#[async_trait]
impl NodeLogic for SetGeoMapMarkers {
    fn get_node(&self) -> Node {
        let mut node = Node::new(
            "a2ui_set_geomap_markers",
            "Set GeoMap Markers",
            "Sets all markers on a GeoMap element, replacing existing ones",
            "UI/Elements/GeoMap",
        );
        node.add_icon("/flow/icons/a2ui.svg");

        node.add_input_pin("exec_in", "▶", "Execution input", VariableType::Execution);

        node.add_input_pin(
            "element_ref",
            "GeoMap",
            "Reference to the GeoMap element",
            VariableType::Struct,
        )
        .set_schema::<GeoMapProps>()
        .set_options(PinOptions::new().set_enforce_schema(false).build());

        node.add_input_pin(
            "markers",
            "Markers",
            "Array of marker definitions [{id, coordinate: {latitude, longitude}, color, label, popup, draggable}]",
            VariableType::Generic,
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

        let markers: Value = context.evaluate_pin("markers").await?;

        let update_value = json!({
            "type": "setGeoMapMarkers",
            "markers": { "literalJson": flow_like_types::json::to_string(&markers)? }
        });

        context.upsert_element(&element_id, update_value).await?;
        context.activate_exec_pin("exec_out").await?;

        Ok(())
    }
}
