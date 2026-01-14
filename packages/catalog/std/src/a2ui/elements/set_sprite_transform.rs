use flow_like::flow::{
    execution::context::ExecutionContext,
    node::{Node, NodeLogic},
    pin::PinOptions,
    variable::VariableType,
};
use flow_like::a2ui::components::SpriteProps;
use flow_like_types::{Value, async_trait, json::json};
use super::element_utils::extract_element_id;

#[crate::register_node]
#[derive(Default)]
pub struct SetSpriteTransform;

impl SetSpriteTransform {
    pub fn new() -> Self {
        Self
    }
}

#[async_trait]
impl NodeLogic for SetSpriteTransform {
    fn get_node(&self) -> Node {
        let mut node = Node::new(
            "a2ui_set_sprite_transform",
            "Set Sprite Transform",
            "Sets rotation, scale, and opacity of a sprite element",
            "A2UI/Elements/Game",
        );
        node.add_icon("/flow/icons/a2ui.svg");

        node.add_input_pin("exec_in", "▶", "Execution input", VariableType::Execution);

        node.add_input_pin(
            "element_ref",
            "Sprite",
            "Reference to the sprite element",
            VariableType::Struct,
        )
        .set_schema::<SpriteProps>()
        .set_options(PinOptions::new().set_enforce_schema(false).build());

        node.add_input_pin("rotation", "Rotation", "Rotation in degrees", VariableType::Float);
        node.add_input_pin("scale", "Scale", "Scale factor (1.0 = normal)", VariableType::Float);
        node.add_input_pin("opacity", "Opacity", "Opacity (0.0 to 1.0)", VariableType::Float);

        node.add_output_pin("exec_out", "▶", "Execution output", VariableType::Execution);
        node.set_long_running(true);

        node
    }

    async fn run(&self, context: &mut ExecutionContext) -> flow_like_types::Result<()> {
        context.deactivate_exec_pin("exec_out").await?;

        let element_value: Value = context.evaluate_pin("element_ref").await?;
        let element_id = extract_element_id(&element_value)
            .ok_or_else(|| flow_like_types::anyhow!("Invalid element reference"))?;

        let rotation: f64 = context.evaluate_pin("rotation").await?;
        let scale: f64 = context.evaluate_pin("scale").await?;
        let opacity: f64 = context.evaluate_pin("opacity").await?;

        let update_value = json!({
            "type": "setProps",
            "props": {
                "rotation": rotation,
                "scale": scale,
                "opacity": opacity
            }
        });

        context.upsert_element(&element_id, update_value).await?;
        context.activate_exec_pin("exec_out").await?;
        Ok(())
    }
}
