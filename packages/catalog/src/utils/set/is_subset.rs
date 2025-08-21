use std::collections::HashSet;
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
use std::sync::Arc;

#[derive(Default)]
pub struct SetIsSubsetNode {}

impl SetIsSubsetNode {
    pub fn new() -> Self {
        SetIsSubsetNode {}
    }
}

#[async_trait]
impl NodeLogic for SetIsSubsetNode {
    async fn get_node(&self, _app_state: &FlowLikeState) -> Node {
        let mut node = Node::new(
            "set_is_subset",
            "Is Subset",
            "Checks if a hash set is a subset from a supposed bigger one",
            "Utils/Set",
        );

        node.add_icon("/flow/icons/grip.svg");

        node.add_input_pin("set_in_1", "Set", "Your Smaller Set", VariableType::Generic)
            .set_value_type(ValueType::HashSet);

        node.add_input_pin("set_in_2", "Set", "Your Bigger Set", VariableType::Generic)
            .set_value_type(ValueType::HashSet);

        node.add_output_pin(
            "is_subset",
            "Is It A Subset?",
            "Is the first set a subset of the second?",
            VariableType::Boolean,
        );

        return node;
    }

    async fn run(&self, context: &mut ExecutionContext) -> flow_like_types::Result<()> {
        let set_in_1: &HashSet<Value> = &context.evaluate_pin("set_in_1").await?;
        let set_in_2: &HashSet<Value> = &context.evaluate_pin("set_in_2").await?;
        let result = set_in_1.is_subset(&set_in_2);
        context.set_pin_value("is_subset", json!(result)).await?;
        Ok(())
    }

    async fn on_update(&self, node: &mut Node, board: Arc<Board>) {
        let _ = node.match_type("set_in_1", board.clone(), Some(ValueType::HashSet), None);
        let _ = node.match_type("set_in_2", board.clone(), Some(ValueType::HashSet), None);
        node.harmonize_type(vec!["set_in_1", "set_in_2"], true);
    }
}
