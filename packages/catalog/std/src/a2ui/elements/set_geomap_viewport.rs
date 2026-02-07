use super::element_utils::extract_element_id;
use flow_like::a2ui::components::GeoMapProps;
use flow_like::flow::{
    execution::context::ExecutionContext,
    node::{Node, NodeLogic},
    pin::PinOptions,
    variable::VariableType,
};
use flow_like_types::{Value, async_trait, json::json};

/// Sets the viewport (center, zoom, bearing, pitch) of a GeoMap element.
#[crate::register_node]
#[derive(Default)]
pub struct SetGeoMapViewport;

impl SetGeoMapViewport {
    pub fn new() -> Self {
        Self
    }
}

#[async_trait]
impl NodeLogic for SetGeoMapViewport {
    fn get_node(&self) -> Node {
        let mut node = Node::new(
            "a2ui_set_geomap_viewport",
            "Set GeoMap Viewport",
            "Navigates a GeoMap to a specific location and zoom level",
            "A2UI/Elements/GeoMap",
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
            "latitude",
            "Latitude",
            "Center latitude (-90 to 90)",
            VariableType::Float,
        )
        .set_default_value(Some(json!(48.137)));

        node.add_input_pin(
            "longitude",
            "Longitude",
            "Center longitude (-180 to 180)",
            VariableType::Float,
        )
        .set_default_value(Some(json!(11.576)));

        node.add_input_pin(
            "zoom",
            "Zoom",
            "Zoom level (0-22)",
            VariableType::Float,
        )
        .set_default_value(Some(json!(10.0)));

        node.add_input_pin(
            "bearing",
            "Bearing",
            "Map rotation in degrees (optional)",
            VariableType::Float,
        )
        .set_default_value(Some(json!(0.0)));

        node.add_input_pin(
            "pitch",
            "Pitch",
            "Map tilt in degrees (optional, 0-85)",
            VariableType::Float,
        )
        .set_default_value(Some(json!(0.0)));

        node.add_output_pin("exec_out", "▶", "Execution output", VariableType::Execution);

        node.set_long_running(true);

        node
    }

    async fn run(&self, context: &mut ExecutionContext) -> flow_like_types::Result<()> {
        context.deactivate_exec_pin("exec_out").await?;

        let element_value: Value = context.evaluate_pin("element_ref").await?;
        let element_id = extract_element_id(&element_value)
            .ok_or_else(|| flow_like_types::anyhow!("Invalid element reference"))?;

        let latitude: f64 = context.evaluate_pin("latitude").await?;
        let longitude: f64 = context.evaluate_pin("longitude").await?;
        let zoom: f64 = context.evaluate_pin("zoom").await?;
        let bearing: f64 = context.evaluate_pin("bearing").await.unwrap_or(0.0);
        let pitch: f64 = context.evaluate_pin("pitch").await.unwrap_or(0.0);

        let viewport = json!({
            "center": { "latitude": latitude, "longitude": longitude },
            "zoom": zoom,
            "bearing": bearing,
            "pitch": pitch
        });

        let update_value = json!({
            "type": "setGeoMapViewport",
            "viewport": { "literalJson": flow_like_types::json::to_string(&viewport)? }
        });

        context.upsert_element(&element_id, update_value).await?;
        context.activate_exec_pin("exec_out").await?;

        Ok(())
    }
}
