use super::element_utils::extract_element_id;
use flow_like::a2ui::components::Model3dProps;
use flow_like::flow::{
    execution::context::ExecutionContext,
    node::{Node, NodeLogic},
    pin::PinOptions,
    variable::VariableType,
};
use flow_like_types::{Value, async_trait, json::json};

#[crate::register_node]
#[derive(Default)]
pub struct SetModel3dTransform;

impl SetModel3dTransform {
    pub fn new() -> Self {
        Self
    }
}

#[async_trait]
impl NodeLogic for SetModel3dTransform {
    fn get_node(&self) -> Node {
        let mut node = Node::new(
            "a2ui_set_model3d_transform",
            "Set Model3D Transform",
            "Sets the position, rotation, and scale of a 3D model element",
            "UI/Elements/Game",
        );
        node.add_icon("/flow/icons/a2ui.svg");

        node.add_input_pin("exec_in", "▶", "Execution input", VariableType::Execution);

        node.add_input_pin(
            "element_ref",
            "Model3D",
            "Reference to the 3D model element",
            VariableType::Struct,
        )
        .set_schema::<Model3dProps>()
        .set_options(PinOptions::new().set_enforce_schema(false).build());

        node.add_input_pin(
            "position",
            "Position",
            "Position as {x, y, z} object",
            VariableType::Struct,
        );

        node.add_input_pin(
            "rotation",
            "Rotation",
            "Rotation as {x, y, z} degrees",
            VariableType::Struct,
        );

        node.add_input_pin(
            "scale",
            "Scale",
            "Scale factor (number or {x, y, z})",
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

        let position: Value = context.evaluate_pin("position").await?;
        let rotation: Value = context.evaluate_pin("rotation").await?;
        let scale: Value = context.evaluate_pin("scale").await?;

        let update_value = json!({
            "type": "setProps",
            "props": {
                "position": position,
                "rotation": rotation,
                "scale": scale
            }
        });

        context.upsert_element(&element_id, update_value).await?;
        context.activate_exec_pin("exec_out").await?;
        Ok(())
    }
}
