use flow_like::{
    bit::BitModelPreference,
    flow::{
        execution::context::ExecutionContext,
        node::{Node, NodeLogic},
        pin::PinOptions,
        variable::VariableType,
    },
    state::FlowLikeState,
};
use flow_like_types::{async_trait, json::json};

#[derive(Default)]
pub struct SetModelHintNode {}

impl SetModelHintNode {
    pub fn new() -> Self {
        SetModelHintNode {}
    }
}

#[async_trait]
impl NodeLogic for SetModelHintNode {
    async fn get_node(&self, _app_state: &FlowLikeState) -> Node {
        let mut node = Node::new(
            "ai_generative_set_model_hint",
            "Set Model Hint",
            "Sets the model hint in BitModelPreference",
            "AI/Generative/Preferences",
        );
        node.add_icon("/flow/icons/struct.svg");

        node.add_input_pin(
            "exec_in",
            "Input",
            "Initiate Execution",
            VariableType::Execution,
        );

        node.add_input_pin(
            "preferences_in",
            "Preferences",
            "Current Preferences",
            VariableType::Struct,
        )
        .set_schema::<BitModelPreference>()
        .set_options(PinOptions::new().set_enforce_schema(true).build());

        node.add_input_pin(
            "model_hint",
            "Model Hint",
            "Model Hint to set",
            VariableType::String,
        );

        node.add_output_pin(
            "exec_out",
            "Output",
            "Done with the Execution",
            VariableType::Execution,
        );

        node.add_output_pin(
            "preferences_out",
            "Preferences",
            "Updated Preferences",
            VariableType::Struct,
        )
        .set_schema::<BitModelPreference>();

        return node;
    }

    async fn run(&self, context: &mut ExecutionContext) -> flow_like_types::Result<()> {
        let mut preferences: BitModelPreference = context.evaluate_pin("preferences_in").await?;
        let model_hint: String = context.evaluate_pin("model_hint").await?;

        preferences.model_hint = Some(model_hint);

        context
            .set_pin_value("preferences_out", json!(preferences))
            .await?;
        context.activate_exec_pin("exec_out").await?;

        Ok(())
    }
}
