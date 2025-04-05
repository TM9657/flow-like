use flow_like::{
    flow::{
        execution::context::ExecutionContext,
        node::{Node, NodeLogic},
        variable::VariableType,
    },
    state::FlowLikeState,
};
use flow_like_types::async_trait;
use std::collections::HashMap;

#[derive(Default)]
pub struct MakeStructNode {}

impl MakeStructNode {
    pub fn new() -> Self {
        MakeStructNode {}
    }
}

#[async_trait]
impl NodeLogic for MakeStructNode {
    async fn get_node(&self, _app_state: &FlowLikeState) -> Node {
        let mut node = Node::new(
            "struct_make",
            "Make Struct",
            "Creates a new struct",
            "Structs",
        );
        node.add_icon("/flow/icons/struct.svg");

        node.add_output_pin("struct", "Struct", "Struct Output", VariableType::Struct);

        return node;
    }

    async fn run(&self, context: &mut ExecutionContext) -> flow_like_types::Result<()> {
        let empty_struct: HashMap<String, flow_like_types::Value> = HashMap::new();
        context
            .set_pin_value("struct", flow_like_types::json::json!(empty_struct))
            .await?;

        return Ok(());
    }
}
