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
pub struct ClearSetNode {}

impl ClearSetNode {
    pub fn new() -> Self {
        ClearSetNode {}
    }
}

#[async_trait]
impl NodeLogic for ClearSetNode {
    async fn get_node(&self, _app_state: &FlowLikeState) -> Node {
        let mut node = Node::new(
            "set_clear",
            "Clear set",
            "Removes / Clears all elements from a set",
            "Utils/Set",
        );

        node.add_icon("/flow/icons/grip.svg");

        node.add_input_pin("exec_in", "In", "", VariableType::Execution);

        node.add_input_pin("set_in", "Set", "Your Set", VariableType::Generic)
            .set_value_type(ValueType::HashSet);

        node.add_output_pin("exec_out", "Out", "", VariableType::Execution);

        node.add_output_pin("set_out", "Emptied", "Empty Set", VariableType::Generic)
            .set_value_type(ValueType::HashSet);

        return node;
    }

    async fn run(&self, context: &mut ExecutionContext) -> flow_like_types::Result<()> {
        let array_out: HashSet<Value> = HashSet::new();

        context.set_pin_value("set_out", json!(array_out)).await?;
        context.activate_exec_pin("exec_out").await?;

        Ok(())
    }

    async fn on_update(&self, node: &mut Node, board: Arc<Board>) {
        let _ = node.match_type("set_in", board.clone(), Some(ValueType::HashSet), None);
        let _ = node.match_type("set_out", board.clone(), Some(ValueType::HashSet), None);
        node.harmonize_type(vec!["set_in", "set_out"], true);
    }
}
