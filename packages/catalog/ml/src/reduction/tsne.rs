//! Node for **t-SNE Dimensionality Reduction**
//!
//! This is a placeholder for t-SNE implementation.
//! t-SNE requires the `linfa-tsne` crate which is not yet included in dependencies.
//!
//! TODO: Add linfa-tsne dependency and implement full t-SNE node.

use flow_like::flow::{
    execution::context::ExecutionContext,
    node::{Node, NodeLogic, NodeScores},
    variable::VariableType,
};
use flow_like_types::{Result, async_trait, json::json};

#[crate::register_node]
#[derive(Default)]
pub struct TsneNode {}

impl TsneNode {
    pub fn new() -> Self {
        TsneNode {}
    }
}

#[async_trait]
impl NodeLogic for TsneNode {
    fn get_node(&self) -> Node {
        let mut node = Node::new(
            "fit_tsne",
            "t-SNE Reduction",
            "t-Distributed Stochastic Neighbor Embedding for dimensionality reduction (placeholder - not yet implemented)",
            "AI/ML/Reduction",
        );
        node.add_icon("/flow/icons/chart-network.svg");

        node.set_scores(
            NodeScores::new()
                .set_privacy(6)
                .set_security(6)
                .set_performance(3) // t-SNE is computationally expensive
                .set_governance(6)
                .set_reliability(5)
                .set_cost(4)
                .build(),
        );

        node.add_input_pin(
            "exec_in",
            "Input",
            "Execution trigger",
            VariableType::Execution,
        );

        node.add_input_pin(
            "n_components",
            "Components",
            "Number of dimensions to reduce to (typically 2 or 3)",
            VariableType::Integer,
        )
        .set_default_value(Some(json!(2)));

        node.add_input_pin(
            "perplexity",
            "Perplexity",
            "Related to the number of nearest neighbors (typical values: 5-50)",
            VariableType::Float,
        )
        .set_default_value(Some(json!(30.0)));

        node.add_output_pin(
            "exec_out",
            "Done",
            "Activated once t-SNE transformation completes",
            VariableType::Execution,
        );

        // Mark as not implemented
        node.error = Some("t-SNE not yet implemented - requires linfa-tsne dependency".to_string());

        node
    }

    async fn run(&self, _context: &mut ExecutionContext) -> Result<()> {
        Err(flow_like_types::anyhow!(
            "t-SNE not yet implemented. This node requires the linfa-tsne dependency."
        ))
    }
}
