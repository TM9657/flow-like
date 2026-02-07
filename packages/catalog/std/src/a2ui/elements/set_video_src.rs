use super::element_utils::extract_element_id;
use flow_like::a2ui::components::VideoProps;
use flow_like::flow::{
    execution::context::ExecutionContext,
    node::{Node, NodeLogic},
    pin::PinOptions,
    variable::VariableType,
};
use flow_like_types::{Value, async_trait, json::json};

/// Sets the source URL of a video element.
#[crate::register_node]
#[derive(Default)]
pub struct SetVideoSrc;

impl SetVideoSrc {
    pub fn new() -> Self {
        Self
    }
}

#[async_trait]
impl NodeLogic for SetVideoSrc {
    fn get_node(&self) -> Node {
        let mut node = Node::new(
            "a2ui_set_video_src",
            "Set Video Source",
            "Sets the source URL of a video element",
            "UI/Elements/Media",
        );
        node.add_icon("/flow/icons/a2ui.svg");

        node.add_input_pin("exec_in", "▶", "Execution input", VariableType::Execution);

        node.add_input_pin(
            "element_ref",
            "Video",
            "Reference to the video element",
            VariableType::Struct,
        )
        .set_schema::<VideoProps>()
        .set_options(PinOptions::new().set_enforce_schema(false).build());

        node.add_input_pin("src", "URL", "Video source URL", VariableType::String);

        node.add_input_pin(
            "poster",
            "Poster",
            "Poster image URL (optional)",
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

        let src: String = context.evaluate_pin("src").await?;
        let poster: String = context.evaluate_pin("poster").await.unwrap_or_default();

        let mut update = json!({
            "type": "setVideoSrc",
            "src": src
        });

        if !poster.is_empty() {
            update["poster"] = Value::String(poster);
        }

        context.upsert_element(&element_id, update).await?;
        context.activate_exec_pin("exec_out").await?;

        Ok(())
    }
}
