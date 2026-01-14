use flow_like::flow::{
    execution::context::ExecutionContext,
    node::{Node, NodeLogic},
    pin::PinOptions,
    variable::VariableType,
};
use flow_like::a2ui::components::ImageLabelerProps;
use flow_like_types::{Value, async_trait, json::json};
use super::element_utils::extract_element_id;

/// Sets the source image for an ImageLabeler element.
#[crate::register_node]
#[derive(Default)]
pub struct SetLabelerImage;

impl SetLabelerImage {
    pub fn new() -> Self {
        Self
    }
}

#[async_trait]
impl NodeLogic for SetLabelerImage {
    fn get_node(&self) -> Node {
        let mut node = Node::new(
            "a2ui_set_labeler_image",
            "Set Labeler Image",
            "Sets the source image for an ImageLabeler element",
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
            "src",
            "URL",
            "Image source URL",
            VariableType::String,
        );

        node.add_input_pin(
            "clear_boxes",
            "Clear Boxes",
            "Clear existing boxes when changing image",
            VariableType::Boolean,
        )
        .set_default_value(Some(json!(true)));

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
        let clear_boxes: bool = context.evaluate_pin("clear_boxes").await.unwrap_or(true);

        let update_value = json!({
            "type": "setLabelerImage",
            "src": src,
            "clearBoxes": clear_boxes
        });

        context.upsert_element(&element_id, update_value).await?;
        context.activate_exec_pin("exec_out").await?;

        Ok(())
    }
}
