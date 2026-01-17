use super::element_utils::extract_element_id;
use flow_like::a2ui::components::ImageLabelerProps;
use flow_like::flow::{
    execution::context::ExecutionContext,
    node::{Node, NodeLogic},
    pin::PinOptions,
    variable::VariableType,
};
use flow_like_types::{Value, async_trait, json::json};

/// Removes a bounding box from an ImageLabeler element by ID.
#[crate::register_node]
#[derive(Default)]
pub struct RemoveLabelerBox;

impl RemoveLabelerBox {
    pub fn new() -> Self {
        Self
    }
}

#[async_trait]
impl NodeLogic for RemoveLabelerBox {
    fn get_node(&self) -> Node {
        let mut node = Node::new(
            "a2ui_remove_labeler_box",
            "Remove Labeler Box",
            "Removes a bounding box from an ImageLabeler by ID",
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
            "box_id",
            "Box ID",
            "ID of the box to remove",
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

        let box_id: String = context.evaluate_pin("box_id").await?;

        let update_value = json!({
            "type": "removeLabelerBox",
            "boxId": box_id
        });

        context.upsert_element(&element_id, update_value).await?;
        context.activate_exec_pin("exec_out").await?;

        Ok(())
    }
}
