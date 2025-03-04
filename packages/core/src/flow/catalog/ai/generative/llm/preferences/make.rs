use crate::{
    bit::BitModelPreference,
    flow::{
        execution::context::ExecutionContext,
        node::{Node, NodeLogic},
        variable::VariableType,
    },
    state::FlowLikeState,
};
use async_trait::async_trait;
use serde_json::json;

#[derive(Default)]
pub struct MakePreferencesNode {}

impl MakePreferencesNode {
    pub fn new() -> Self {
        MakePreferencesNode {}
    }
}

#[async_trait]
impl NodeLogic for MakePreferencesNode {
    async fn get_node(&self, _app_state: &FlowLikeState) -> Node {
        let mut node = Node::new(
            "ai_generative_make_preferences",
            "Make Preferences",
            "Creates Model Preferences for model selection",
            "AI/Generative/Preferences",
        );
        node.add_icon("/flow/icons/struct.svg");

        node.add_output_pin("preferences", "Preferences", "BitModelPreference", VariableType::Struct)
            .set_schema::<BitModelPreference>();

        return node;
    }

    async fn run(&mut self, context: &mut ExecutionContext) -> anyhow::Result<()> {
        let preferences = BitModelPreference::default();

        context.set_pin_value("preferences", json!(preferences)).await?;

        Ok(())
    }
}