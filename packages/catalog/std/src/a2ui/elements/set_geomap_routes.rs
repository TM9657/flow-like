use super::element_utils::extract_element_id;
use flow_like::a2ui::components::GeoMapProps;
use flow_like::flow::{
    execution::context::ExecutionContext,
    node::{Node, NodeLogic},
    pin::PinOptions,
    variable::VariableType,
};
use flow_like_types::{Value, async_trait, json::json};

/// Sets all routes on a GeoMap element, replacing any existing routes.
#[crate::register_node]
#[derive(Default)]
pub struct SetGeoMapRoutes;

impl SetGeoMapRoutes {
    pub fn new() -> Self {
        Self
    }
}

#[async_trait]
impl NodeLogic for SetGeoMapRoutes {
    fn get_node(&self) -> Node {
        let mut node = Node::new(
            "a2ui_set_geomap_routes",
            "Set GeoMap Routes",
            "Sets all routes on a GeoMap element, replacing existing ones",
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
            "routes",
            "Routes",
            "Array of route definitions [{id, coordinates: [{latitude, longitude}], color, width, opacity}]",
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

        let routes: Value = context.evaluate_pin("routes").await?;

        let update_value = json!({
            "type": "setGeoMapRoutes",
            "routes": { "literalJson": flow_like_types::json::to_string(&routes)? }
        });

        context.upsert_element(&element_id, update_value).await?;
        context.activate_exec_pin("exec_out").await?;

        Ok(())
    }
}
