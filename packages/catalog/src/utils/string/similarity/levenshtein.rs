use flow_like::{
    flow::{
        execution::context::ExecutionContext,
        node::{Node, NodeLogic},
        variable::VariableType,
    },
    state::FlowLikeState,
};
use flow_like_types::{async_trait, json::json};

#[derive(Default)]
pub struct LevenshteinDistanceNode {}

impl LevenshteinDistanceNode {
    pub fn new() -> Self {
        LevenshteinDistanceNode {}
    }
}

#[async_trait]
impl NodeLogic for LevenshteinDistanceNode {
    async fn get_node(&self, _app_state: &FlowLikeState) -> Node {
        let mut node = Node::new(
            "levenshtein_distance",
            "Levenshtein Distance",
            "Calculates the Levenshtein distance between two strings",
            "Utils/String/Similarity",
        );
        node.add_icon("/flow/icons/distance.svg");

        node.add_input_pin("string1", "String 1", "First String", VariableType::String);
        node.add_input_pin("string2", "String 2", "Second String", VariableType::String);
        node.add_input_pin(
            "normalize",
            "Normalize",
            "Normalize the Distance",
            VariableType::Boolean,
        );

        node.add_output_pin(
            "distance",
            "Distance",
            "Levenshtein Distance",
            VariableType::Float,
        );

        return node;
    }

    async fn run(&self, context: &mut ExecutionContext) -> flow_like_types::Result<()> {
        let string1: String = context.evaluate_pin("string1").await?;
        let string2: String = context.evaluate_pin("string2").await?;
        let normalize: bool = context.evaluate_pin("normalize").await?;

        let distance = match normalize {
            true => strsim::normalized_levenshtein(&string1, &string2) as f64,
            false => strsim::levenshtein(&string1, &string2) as f64,
        };

        context.set_pin_value("distance", json!(distance)).await?;

        Ok(())
    }
}
