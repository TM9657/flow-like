use crate::{
    bit::{Bit, BitModelPreference},
    flow::{
        execution::context::ExecutionContext,
        node::{Node, NodeLogic},
        pin::PinOptions,
        variable::VariableType,
    },
    state::FlowLikeState,
};
use async_trait::async_trait;
use serde_json::json;

#[derive(Default)]
pub struct FindLLMNode {}

impl FindLLMNode {
    pub fn new() -> Self {
        FindLLMNode {}
    }
}

#[async_trait]
impl NodeLogic for FindLLMNode {
    async fn get_node(&self, _app_state: &FlowLikeState) -> Node {
        let mut node = Node::new(
            "ai_generative_find_model",
            "Find Model",
            "Finds the best model based on certain selection criteria",
            "AI/Generative",
        );
        node.add_icon("/flow/icons/find_model.svg");

        node.add_input_pin("exec_in", "Input", "Trigger Pin", VariableType::Execution);
        node.add_input_pin(
            "preferences",
            "Preferences",
            "Preferences for the model",
            VariableType::Struct,
        )
        .set_default_value(Some(json!(BitModelPreference::default())))
        .set_schema::<BitModelPreference>()
        .set_options(PinOptions::new().set_enforce_schema(true).build());

        node.add_output_pin("exec_out", "Output", "Done", VariableType::Execution);
        node.add_output_pin("model", "Model", "The selected model", VariableType::Struct)
            .set_schema::<Bit>();

        node.set_long_running(true);

        return node;
    }

    async fn run(&self, context: &mut ExecutionContext) -> anyhow::Result<()> {
        let mut preference: BitModelPreference = context.evaluate_pin("preferences").await?;
        preference.enforce_bounds();

        let http_client = context.app_state.lock().await.http_client.clone();
        let bit = context
            .profile
            .get_best_model(&preference, false, false, http_client)
            .await?;
        context
            .set_pin_value("model", serde_json::json!(bit))
            .await?;

        context.activate_exec_pin("exec_out").await?;

        return Ok(());
    }
}
