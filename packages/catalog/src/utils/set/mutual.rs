use flow_like::{
    flow::{
        board::Board,
        execution::context::ExecutionContext,
        node::{Node, NodeLogic},
        pin::ValueType,
        variable::VariableType,
    },
    state::FlowLikeState,
};
use flow_like_types::{Value, async_trait, json::json};
use std::{collections::HashSet, sync::Arc};

#[derive(Default)]
pub struct IsMutualSetNode {}

impl IsMutualSetNode {
    pub fn new() -> Self {
        IsMutualSetNode {}
    }
}

#[async_trait]
impl NodeLogic for IsMutualSetNode {
    async fn get_node(&self, _app_state: &FlowLikeState) -> Node {
        let mut node = Node::new(
            "is_mutual",
            "Is Mutual",
            "Checks if one of the hash sets has at least one mutual element",
            "Utils/Set",
        );

        node.add_icon("/flow/icons/grip.svg");

        node.add_input_pin("exec_in", "In", "", VariableType::Execution);

        node.add_input_pin("set_in_1", "Set #1", "", VariableType::Generic)
            .set_value_type(ValueType::HashSet);

        node.add_input_pin("set_in_2", "Set #2", "", VariableType::Generic)
            .set_value_type(ValueType::HashSet);

        node.add_output_pin(
            "is_mutual",
            "Contains Mutual Element?",
            "Does it include a mutual element that both sets share or not?",
            VariableType::Boolean,
        );

        node.add_output_pin("exec_out", "Out", "", VariableType::Execution);

        return node;
    }

    async fn run(&self, context: &mut ExecutionContext) -> flow_like_types::Result<()> {
        let set_in_1: HashSet<Value> = context.evaluate_pin("set_in_1").await?;
        let set_in_2: HashSet<Value> = context.evaluate_pin("set_in_1").await?;
        let is_mutual: bool = !set_in_1.is_disjoint(&set_in_2);
        context.set_pin_value("is_mutual", json!(is_mutual)).await?;
        context.activate_exec_pin("exec_out").await?;
        Ok(())
    }

    async fn on_update(&self, node: &mut Node, board: Arc<Board>) {
        let _ = node.match_type("set_in_1", board.clone(), Some(ValueType::HashSet), None);
        let _ = node.match_type("set_in_2", board.clone(), Some(ValueType::HashSet), None);
        node.harmonize_type(vec!["set_in_1", "set_in_2"], true);
    }
}
