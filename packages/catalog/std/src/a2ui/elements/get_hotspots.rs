use super::element_utils::{extract_element_id_from_pin, find_element};
use flow_like::a2ui::components::ImageHotspotProps;
use flow_like::flow::{
    board::Board,
    execution::context::ExecutionContext,
    node::{Node, NodeLogic},
    pin::PinOptions,
    variable::VariableType,
};
use flow_like_types::{Value, async_trait, json::json};
use std::sync::Arc;

/// Gets all hotspots from an ImageHotspot element.
///
/// Returns an array of Hotspot objects with id, x, y, size, label, etc.
#[crate::register_node]
#[derive(Default)]
pub struct GetHotspots;

impl GetHotspots {
    pub fn new() -> Self {
        Self
    }
}

#[async_trait]
impl NodeLogic for GetHotspots {
    fn get_node(&self) -> Node {
        let mut node = Node::new(
            "a2ui_get_hotspots",
            "Get Hotspots",
            "Gets all hotspots from an ImageHotspot element",
            "UI/Elements/Hotspot",
        );
        node.add_icon("/flow/icons/a2ui.svg");

        node.add_input_pin(
            "element_ref",
            "Hotspot Image",
            "Reference to the ImageHotspot element",
            VariableType::Struct,
        )
        .set_schema::<ImageHotspotProps>()
        .set_options(PinOptions::new().set_enforce_schema(false).build());

        node.add_output_pin(
            "hotspots",
            "Hotspots",
            "Array of hotspots [{id, x, y, size, label, ...}, ...]",
            VariableType::Generic,
        );

        node.add_output_pin(
            "count",
            "Count",
            "Number of hotspots",
            VariableType::Integer,
        );

        node
    }

    async fn run(&self, context: &mut ExecutionContext) -> flow_like_types::Result<()> {
        let element_value: Value = context.evaluate_pin("element_ref").await?;
        let element_id = extract_element_id_from_pin(element_value)
            .ok_or_else(|| flow_like_types::anyhow!("Invalid element reference"))?;

        let elements = context.get_frontend_elements().await?;
        let element = elements.as_ref().and_then(|e| find_element(e, &element_id));

        let hotspots = element
            .map(|(_, el)| el)
            .and_then(|el| el.get("component"))
            .and_then(|c| c.get("hotspots"))
            .cloned()
            .unwrap_or(json!([]));

        let count = if let Value::Array(arr) = &hotspots {
            arr.len() as i64
        } else {
            0
        };

        context.set_pin_value("hotspots", hotspots).await?;
        context.set_pin_value("count", json!(count)).await?;

        Ok(())
    }

    async fn on_update(&self, node: &mut Node, _board: Arc<Board>) {
        node.error = None;
    }
}
