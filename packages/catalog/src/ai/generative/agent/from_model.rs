/// # Agent from Model Node
/// Creates an Agent object from a model Bit with basic configuration.
/// This is the starting point for building an agent in the flow.
use crate::ai::generative::agent::Agent;
use flow_like::{
    bit::Bit,
    flow::{
        execution::context::ExecutionContext,
        node::{Node, NodeLogic, NodeScores},
        pin::PinOptions,
        variable::VariableType,
    },
    state::FlowLikeState,
};
use flow_like_types::{async_trait, json};

#[crate::register_node]
#[derive(Default)]
pub struct AgentFromModelNode {}

impl AgentFromModelNode {
    pub fn new() -> Self {
        AgentFromModelNode {}
    }
}

#[async_trait]
impl NodeLogic for AgentFromModelNode {
    async fn get_node(&self, _app_state: &FlowLikeState) -> Node {
        let mut node = Node::new(
            "agent_from_model",
            "Agent from Model",
            "Creates an Agent object from a model Bit with configuration",
            "AI/Agents/Builder",
        );
        node.add_icon("/flow/icons/bot-invoke.svg");

        node.set_scores(
            NodeScores::new()
                .set_privacy(5)
                .set_security(5)
                .set_performance(8)
                .set_governance(6)
                .set_reliability(7)
                .set_cost(2)
                .build(),
        );

        node.add_input_pin(
            "model",
            "Model",
            "LLM model Bit that will power the agent",
            VariableType::Struct,
        )
        .set_schema::<Bit>()
        .set_options(PinOptions::new().set_enforce_schema(true).build());

        node.add_input_pin(
            "max_iter",
            "Max Iterations",
            "Maximum number of tool call iterations before stopping",
            VariableType::Integer,
        )
        .set_default_value(Some(json::json!(15)));

        node.add_output_pin(
            "agent_out",
            "Agent",
            "Configured Agent object ready for tool registration and execution",
            VariableType::Struct,
        )
        .set_schema::<Agent>()
        .set_options(PinOptions::new().set_enforce_schema(true).build());

        node
    }

    async fn run(&self, context: &mut ExecutionContext) -> flow_like_types::Result<()> {
        let model: Bit = context.evaluate_pin("model").await?;
        let max_iter: u64 = context.evaluate_pin("max_iter").await?;

        let mut agent = Agent::new(model.clone(), max_iter);

        // Store model display name
        if let Some(meta) = model.meta.get("en") {
            agent.model_display_name = Some(meta.name.clone());
        }

        context
            .set_pin_value("agent_out", json::json!(agent))
            .await?;

        Ok(())
    }
}
