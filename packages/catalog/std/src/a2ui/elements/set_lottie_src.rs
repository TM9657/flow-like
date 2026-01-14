use flow_like::flow::{
    execution::context::ExecutionContext,
    node::{Node, NodeLogic},
    pin::PinOptions,
    variable::VariableType,
};
use flow_like::a2ui::components::LottieProps;
use flow_like_types::{Value, async_trait, json::json};
use super::element_utils::extract_element_id;

/// Sets the source URL of a Lottie animation element.
#[crate::register_node]
#[derive(Default)]
pub struct SetLottieSrc;

impl SetLottieSrc {
    pub fn new() -> Self {
        Self
    }
}

#[async_trait]
impl NodeLogic for SetLottieSrc {
    fn get_node(&self) -> Node {
        let mut node = Node::new(
            "a2ui_set_lottie_src",
            "Set Lottie Source",
            "Sets the source URL of a Lottie animation element",
            "A2UI/Elements/Media",
        );
        node.add_icon("/flow/icons/a2ui.svg");

        node.add_input_pin("exec_in", "▶", "Execution input", VariableType::Execution);

        node.add_input_pin(
            "element_ref",
            "Lottie",
            "Reference to the Lottie animation element",
            VariableType::Struct,
        )
        .set_schema::<LottieProps>()
        .set_options(PinOptions::new().set_enforce_schema(false).build());

        node.add_input_pin(
            "src",
            "URL",
            "Lottie JSON animation URL",
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

        context.upsert_element(&element_id, json!({
            "type": "setLottieSrc",
            "src": src
        })).await?;
        context.activate_exec_pin("exec_out").await?;

        Ok(())
    }
}
