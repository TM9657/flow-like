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
pub struct PushSetNode {}

impl PushSetNode {
    pub fn new() -> Self {
        PushSetNode {}
    }
}

#[async_trait]
impl NodeLogic for PushSetNode {
    async fn get_node(&self, _app_state: &FlowLikeState) -> Node {
        let mut node = Node::new(
            "push",
            "Push Element",
            "Pushes an element to the set",
            "Utils/Set",
        );

        node.add_icon("/flow/icons/grip.svg");

        node.add_input_pin("exec_in", "In", "", VariableType::Execution);

        node.add_input_pin("set_in", "Set", "", VariableType::Generic)
            .set_value_type(ValueType::HashSet);

        node.add_input_pin("value", "Value", "Value to push", VariableType::Generic);

        node.add_output_pin("set_out", "Set", "", VariableType::Generic)
            .set_value_type(ValueType::HashSet);

        node.add_output_pin("exec_out", "Out", "", VariableType::Execution);

        return node;
    }

    async fn run(&self, context: &mut ExecutionContext) -> flow_like_types::Result<()> {
        let mut set_in: HashSet<Value> = context.evaluate_pin("set_in").await?;
        let element: Value = context.evaluate_pin("value").await?;
        set_in.insert(element);
        context.set_pin_value("set_out", json!(set_in)).await?;
        context.activate_exec_pin("exec_out").await?;
        Ok(())
    }

    async fn on_update(&self, node: &mut Node, board: Arc<Board>) {
        let _ = node.match_type("set_out", board.clone(), Some(ValueType::HashSet), None);
        let _ = node.match_type("set_in", board.clone(), Some(ValueType::HashSet), None);
        let _ = node.match_type("value", board, Some(ValueType::Normal), None);
        node.harmonize_type(vec!["set_in", "set_out", "value"], true);
    }
}
