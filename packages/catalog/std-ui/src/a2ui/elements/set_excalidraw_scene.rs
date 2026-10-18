use super::element_utils::extract_element_id;
use flow_like::a2ui::components::{ExcalidrawProps, ExcalidrawScene};
use flow_like::flow::{
    execution::context::ExecutionContext,
    node::{Node, NodeLogic},
    pin::PinOptions,
    variable::VariableType,
};
use flow_like_types::{Value, async_trait, json::json};

/// Loads a complete drawing, including embedded images, into an Excalidraw element.
#[crate::register_node]
#[derive(Default)]
pub struct SetExcalidrawScene;

impl SetExcalidrawScene {
    pub fn new() -> Self {
        Self
    }
}

#[async_trait]
impl NodeLogic for SetExcalidrawScene {
    fn get_node(&self) -> Node {
        let mut node = Node::new(
            "a2ui_set_excalidraw_scene",
            "Set Excalidraw Scene",
            "Loads an Excalidraw scene with its elements, application state and image files",
            "UI/Elements/Excalidraw",
        );
        node.set_flowscript_name("ui", "setExcalidrawScene");
        node.add_icon("/flow/icons/a2ui.svg");
        node.add_input_pin("exec_in", "▶", "Execution input", VariableType::Execution);
        node.add_input_pin(
            "element_ref",
            "Excalidraw",
            "Reference to the Excalidraw element",
            VariableType::Struct,
        )
        .set_schema::<ExcalidrawProps>()
        .set_options(PinOptions::new().set_enforce_schema(false).build());
        node.add_input_pin(
            "scene",
            "Scene",
            "Complete Excalidraw representation, including embedded image files",
            VariableType::Struct,
        )
        .set_schema::<ExcalidrawScene>()
        .set_options(PinOptions::new().set_enforce_schema(true).build());
        node.add_output_pin("exec_out", "▶", "Execution output", VariableType::Execution);
        node.set_long_running(true);
        node
    }

    async fn run(&self, context: &mut ExecutionContext) -> flow_like_types::Result<()> {
        context.deactivate_exec_pin("exec_out").await?;
        let element_value: Value = context.evaluate_pin("element_ref").await?;
        let element_id = extract_element_id(&element_value)
            .ok_or_else(|| flow_like_types::anyhow!("Invalid element reference"))?;
        let scene: ExcalidrawScene = context.evaluate_pin("scene").await?;
        context
            .upsert_element(&element_id, json!({ "type": "setValue", "value": scene }))
            .await?;
        context.activate_exec_pin("exec_out").await?;
        Ok(())
    }
}
