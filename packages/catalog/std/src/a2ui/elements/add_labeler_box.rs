use super::element_utils::extract_element_id;
use flow_like::a2ui::components::ImageLabelerProps;
use flow_like::flow::{
    execution::context::ExecutionContext,
    node::{Node, NodeLogic},
    pin::PinOptions,
    variable::VariableType,
};
use flow_like_types::{Value, async_trait, json::json};

/// Adds a single bounding box to an ImageLabeler element.
///
/// The box will be added to the existing boxes without replacing them.
#[crate::register_node]
#[derive(Default)]
pub struct AddLabelerBox;

impl AddLabelerBox {
    pub fn new() -> Self {
        Self
    }
}

#[async_trait]
impl NodeLogic for AddLabelerBox {
    fn get_node(&self) -> Node {
        let mut node = Node::new(
            "a2ui_add_labeler_box",
            "Add Labeler Box",
            "Adds a bounding box to an ImageLabeler element",
            "A2UI/Elements/Labeler",
        );
        node.add_icon("/flow/icons/a2ui.svg");

        node.add_input_pin("exec_in", "▶", "Execution input", VariableType::Execution);

        node.add_input_pin(
            "element_ref",
            "Labeler",
            "Reference to the ImageLabeler element",
            VariableType::Struct,
        )
        .set_schema::<ImageLabelerProps>()
        .set_options(PinOptions::new().set_enforce_schema(false).build());

        node.add_input_pin(
            "id",
            "ID",
            "Unique identifier for the box (auto-generated if empty)",
            VariableType::String,
        )
        .set_default_value(Some(json!("")));

        node.add_input_pin(
            "x",
            "X",
            "X coordinate (pixels or normalized 0-1)",
            VariableType::Float,
        );

        node.add_input_pin(
            "y",
            "Y",
            "Y coordinate (pixels or normalized 0-1)",
            VariableType::Float,
        );

        node.add_input_pin("width", "Width", "Box width", VariableType::Float);

        node.add_input_pin("height", "Height", "Box height", VariableType::Float);

        node.add_input_pin(
            "label",
            "Label",
            "Label/class name for the box",
            VariableType::String,
        );

        node.add_output_pin("exec_out", "▶", "Execution output", VariableType::Execution);

        node.add_output_pin(
            "box_id",
            "Box ID",
            "The ID of the created box",
            VariableType::String,
        );

        node.set_long_running(true);

        node
    }

    async fn run(&self, context: &mut ExecutionContext) -> flow_like_types::Result<()> {
        context.deactivate_exec_pin("exec_out").await?;

        let element_value: Value = context.evaluate_pin("element_ref").await?;
        let element_id = extract_element_id(&element_value)
            .ok_or_else(|| flow_like_types::anyhow!("Invalid element reference"))?;

        let mut id: String = context.evaluate_pin("id").await.unwrap_or_default();
        if id.is_empty() {
            id = format!("box_{}", chrono::Utc::now().timestamp_millis());
        }

        let x: f64 = context.evaluate_pin("x").await?;
        let y: f64 = context.evaluate_pin("y").await?;
        let width: f64 = context.evaluate_pin("width").await?;
        let height: f64 = context.evaluate_pin("height").await?;
        let label: String = context.evaluate_pin("label").await?;

        let box_data = json!({
            "id": id,
            "x": x,
            "y": y,
            "width": width,
            "height": height,
            "label": label
        });

        let update_value = json!({
            "type": "addLabelerBox",
            "box": box_data
        });

        context.upsert_element(&element_id, update_value).await?;
        context.set_pin_value("box_id", json!(id)).await?;
        context.activate_exec_pin("exec_out").await?;

        Ok(())
    }
}
