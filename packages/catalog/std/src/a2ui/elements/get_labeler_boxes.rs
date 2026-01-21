use super::element_utils::{extract_element_id_from_pin, find_element};
use flow_like::a2ui::components::ImageLabelerProps;
use flow_like::flow::{
    board::Board,
    execution::context::ExecutionContext,
    node::{Node, NodeLogic},
    pin::PinOptions,
    variable::VariableType,
};
use flow_like_types::{Value, async_trait, json::json};
use std::sync::Arc;

/// Gets all bounding boxes from an ImageLabeler element.
///
/// Returns an array of LabelBox objects with id, x, y, width, height, and label.
#[crate::register_node]
#[derive(Default)]
pub struct GetLabelerBoxes;

impl GetLabelerBoxes {
    pub fn new() -> Self {
        Self
    }
}

#[async_trait]
impl NodeLogic for GetLabelerBoxes {
    fn get_node(&self) -> Node {
        let mut node = Node::new(
            "a2ui_get_labeler_boxes",
            "Get Labeler Boxes",
            "Gets all bounding boxes from an ImageLabeler element",
            "A2UI/Elements/Labeler",
        );
        node.add_icon("/flow/icons/a2ui.svg");

        node.add_input_pin(
            "element_ref",
            "Labeler",
            "Reference to the ImageLabeler element",
            VariableType::Struct,
        )
        .set_schema::<ImageLabelerProps>()
        .set_options(PinOptions::new().set_enforce_schema(false).build());

        node.add_output_pin(
            "boxes",
            "Boxes",
            "Array of bounding boxes [{id, x, y, width, height, label}, ...]",
            VariableType::Generic,
        );

        node.add_output_pin(
            "count",
            "Count",
            "Number of bounding boxes",
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

        let boxes = element
            .map(|(_, el)| el)
            .and_then(|el| el.get("component"))
            .and_then(|c| c.get("boxes"))
            .cloned()
            .unwrap_or(json!([]));

        let count = if let Value::Array(arr) = &boxes {
            arr.len() as i64
        } else {
            0
        };

        context.set_pin_value("boxes", boxes).await?;
        context.set_pin_value("count", json!(count)).await?;

        Ok(())
    }

    async fn on_update(&self, node: &mut Node, _board: Arc<Board>) {
        node.error = None;
    }
}
