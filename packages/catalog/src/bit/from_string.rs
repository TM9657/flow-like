use flow_like::{
    bit::Bit,
    flow::{
        execution::{LogLevel, context::ExecutionContext},
        node::{Node, NodeLogic},
        variable::VariableType,
    },
    state::FlowLikeState,
};
use flow_like_types::{async_trait, json::json};

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
            "Bit From String",
            "Converts a string to a boolean (bit)",
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

        node.add_output_pin(
            "failed",
            "Failed Loading",
            "Failed to load the bit",
            VariableType::Execution,
        );

        return node;
    }

    async fn run(&self, context: &mut ExecutionContext) -> flow_like_types::Result<()> {
        context.activate_exec_pin("failed").await?;
        context.deactivate_exec_pin("exec_out").await?;
        let bit_id: String = context.evaluate_pin("bit_id").await?;
        let http_client = context.app_state.lock().await.http_client.clone();
        let bit = context.profile.find_bit(&bit_id, http_client).await;

        if let Ok(bit) = bit {
            context.set_pin_value("output_bit", json!(bit)).await?;
            context.deactivate_exec_pin("failed").await?;
            context.activate_exec_pin("exec_out").await?;
            return Ok(());
        }

        let err = bit.err().unwrap();
        context.log_message(&format!("Bit not found: {}", err), LogLevel::Error);
        Ok(())
    }
}
