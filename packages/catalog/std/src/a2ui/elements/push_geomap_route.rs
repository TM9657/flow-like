use super::element_utils::extract_element_id;
use flow_like::a2ui::components::GeoMapProps;
use flow_like::flow::{
    execution::context::ExecutionContext,
    node::{Node, NodeLogic},
    pin::PinOptions,
    variable::VariableType,
};
use flow_like_types::{Value, async_trait, json::json};

/// Pushes a route onto a GeoMap element without clearing existing routes.
#[crate::register_node]
#[derive(Default)]
pub struct PushGeoMapRoute;

impl PushGeoMapRoute {
    pub fn new() -> Self {
        Self
    }
}

#[async_trait]
impl NodeLogic for PushGeoMapRoute {
    fn get_node(&self) -> Node {
        let mut node = Node::new(
            "a2ui_push_geomap_route",
            "Push GeoMap Route",
            "Adds a route to a GeoMap without removing existing routes",
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
            "id",
            "Route ID",
            "Unique identifier for the route",
            VariableType::String,
        );

        node.add_input_pin(
            "coordinates",
            "Coordinates",
            "Array of coordinates [{latitude, longitude}] defining the route path",
            VariableType::Generic,
        );

        node.add_input_pin(
            "color",
            "Color",
            "Route line color (CSS color string, e.g. '#4285F4')",
            VariableType::String,
        )
        .set_default_value(Some(json!("#4285F4")));

        node.add_input_pin(
            "width",
            "Width",
            "Route line width in pixels",
            VariableType::Float,
        )
        .set_default_value(Some(json!(3.0)));

        node.add_input_pin(
            "opacity",
            "Opacity",
            "Route line opacity (0.0 to 1.0)",
            VariableType::Float,
        )
        .set_default_value(Some(json!(0.8)));

        node.add_output_pin("exec_out", "▶", "Execution output", VariableType::Execution);

        node.set_long_running(true);

        node
    }

    async fn run(&self, context: &mut ExecutionContext) -> flow_like_types::Result<()> {
        context.deactivate_exec_pin("exec_out").await?;

        let element_value: Value = context.evaluate_pin("element_ref").await?;
        let element_id = extract_element_id(&element_value)
            .ok_or_else(|| flow_like_types::anyhow!("Invalid element reference"))?;

        let id: String = context.evaluate_pin("id").await?;
        let coordinates: Value = context.evaluate_pin("coordinates").await?;
        let color: String = context
            .evaluate_pin("color")
            .await
            .unwrap_or_else(|_| "#4285F4".to_string());
        let width: f64 = context.evaluate_pin("width").await.unwrap_or(3.0);
        let opacity: f64 = context.evaluate_pin("opacity").await.unwrap_or(0.8);

        let route = json!({
            "id": id,
            "coordinates": coordinates,
            "color": color,
            "width": width,
            "opacity": opacity
        });

        let update_value = json!({
            "type": "pushGeoMapRoute",
            "route": route
        });

        context.upsert_element(&element_id, update_value).await?;
        context.activate_exec_pin("exec_out").await?;

        Ok(())
    }
}
