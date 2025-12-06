use flow_like::{
    bit::Bit,
    flow::{
        execution::context::ExecutionContext,
        node::{Node, NodeLogic},
        variable::VariableType,
    },
    state::FlowLikeState,
};
use flow_like_types::{async_trait, bail, json::json};

#[crate::register_node]
#[derive(Default)]
pub struct BitFromStringNode {}

impl BitFromStringNode {
    pub fn new() -> Self {
        BitFromStringNode {}
    }
}

#[async_trait]
impl NodeLogic for BitFromStringNode {
    async fn get_node(&self, _app_state: &FlowLikeState) -> Node {
        let mut node = Node::new(
            "bit_from_string",
            "Load Bit",
            "Loads a Bit from a string ID",
            "Bit",
        );
        node.add_icon("/flow/icons/bit.svg");

        node.add_input_pin(
            "exec_in",
            "Input",
            "Initiate Execution",
            VariableType::Execution,
        );

        node.add_output_pin(
            "exec_out",
            "Output",
            "Done with the Execution",
            VariableType::Execution,
        );

        node.add_input_pin("bit_id", "Bit ID", "Input String", VariableType::String);

        node.add_output_pin("output_bit", "Bit", "Output Bit", VariableType::Struct)
            .set_schema::<Bit>();

        return node;
    }

    async fn run(&self, context: &mut ExecutionContext) -> flow_like_types::Result<()> {
        context.deactivate_exec_pin("exec_out").await?;
        let bit_id: String = context.evaluate_pin("bit_id").await?;
        let http_client = context.app_state.http_client.clone();
        let bit = context.profile.find_bit(&bit_id, http_client).await;

        if let Ok(bit) = bit {
            context.set_pin_value("output_bit", json!(bit)).await?;
            context.activate_exec_pin("exec_out").await?;
            return Ok(());
        }

        let err = bit.err().unwrap();
        bail!("Bit not found: {}", err);
    }
}
