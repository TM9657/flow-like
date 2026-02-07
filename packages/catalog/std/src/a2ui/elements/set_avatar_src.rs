use super::element_utils::extract_element_id;
use flow_like::a2ui::components::AvatarProps;
use flow_like::flow::{
    execution::context::ExecutionContext,
    node::{Node, NodeLogic},
    pin::PinOptions,
    variable::VariableType,
};
use flow_like_types::{Value, async_trait, json::json};

/// Sets the source image of an avatar element.
#[crate::register_node]
#[derive(Default)]
pub struct SetAvatarSrc;

impl SetAvatarSrc {
    pub fn new() -> Self {
        Self
    }
}

#[async_trait]
impl NodeLogic for SetAvatarSrc {
    fn get_node(&self) -> Node {
        let mut node = Node::new(
            "a2ui_set_avatar_src",
            "Set Avatar Source",
            "Sets the source image of an avatar element",
            "UI/Elements/Display",
        );
        node.add_icon("/flow/icons/a2ui.svg");

        node.add_input_pin("exec_in", "▶", "Execution input", VariableType::Execution);

        node.add_input_pin(
            "element_ref",
            "Avatar",
            "Reference to the avatar element",
            VariableType::Struct,
        )
        .set_schema::<AvatarProps>()
        .set_options(PinOptions::new().set_enforce_schema(false).build());

        node.add_input_pin("src", "URL", "Avatar image URL", VariableType::String);

        node.add_input_pin(
            "fallback",
            "Fallback",
            "Fallback text (initials) when image fails to load",
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
        let fallback: String = context.evaluate_pin("fallback").await.unwrap_or_default();

        let mut update = json!({
            "type": "setAvatarSrc",
            "src": src
        });

        if !fallback.is_empty() {
            update["fallback"] = Value::String(fallback);
        }

        context.upsert_element(&element_id, update).await?;
        context.activate_exec_pin("exec_out").await?;

        Ok(())
    }
}
