use flow_like::flow::{
    execution::context::ExecutionContext,
    node::{Node, NodeLogic},
    pin::PinOptions,
    variable::VariableType,
};
use flow_like::a2ui::components::Model3dProps;
use flow_like_types::{Value, async_trait, json::json};
use super::element_utils::extract_element_id;

#[crate::register_node]
#[derive(Default)]
pub struct SetModel3dSrc;

impl SetModel3dSrc {
    pub fn new() -> Self {
        Self
    }
}

#[async_trait]
impl NodeLogic for SetModel3dSrc {
    fn get_node(&self) -> Node {
        let mut node = Node::new(
            "a2ui_set_model3d_src",
            "Set Model3D Src",
            "Sets the source URL of a 3D model element",
            "A2UI/Elements/Game",
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

        node.add_input_pin("src", "Src", "3D model source URL (GLTF/GLB)", VariableType::String);

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

        let update_value = json!({
            "type": "setProps",
            "props": { "src": src }
        });

        context.upsert_element(&element_id, update_value).await?;
        context.activate_exec_pin("exec_out").await?;
        Ok(())
    }
}
