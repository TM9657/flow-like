use flow_like::{
    flow::{
        board::Board,
        execution::context::ExecutionContext,
        node::{Node, NodeLogic},
        pin::PinOptions,
        variable::VariableType,
    },
    state::FlowLikeState,
};
use flow_like_model_provider::history::{History, ResponseFormat};
use flow_like_types::{Value, async_trait, json::json};
use std::sync::Arc;

#[derive(Default)]
pub struct SetHistoryResponseFormatNode {}

impl SetHistoryResponseFormatNode {
    pub fn new() -> Self {
        SetHistoryResponseFormatNode {}
    }
}

#[async_trait]
impl NodeLogic for SetHistoryResponseFormatNode {
    async fn get_node(&self, _app_state: &FlowLikeState) -> Node {
        let mut node = Node::new(
            "ai_generative_set_history_response_format",
            "Set Response Format",
            "Sets the response_format attribute in a ChatHistory",
            "AI/Generative/History",
        );
        node.add_icon("/flow/icons/history.svg");

        node.add_input_pin(
            "exec_in",
            "Input",
            "Initiate Execution",
            VariableType::Execution,
        );

        node.add_input_pin("history", "History", "ChatHistory", VariableType::Struct)
            .set_schema::<History>()
            .set_options(PinOptions::new().set_enforce_schema(true).build());

        node.add_input_pin(
            "response_format",
            "Response Format",
            "Response Format Value",
            VariableType::Generic,
        )
        .set_options(
            PinOptions::new()
                .set_enforce_generic_value_type(true)
                .build(),
        );

        node.add_output_pin(
            "exec_out",
            "Output",
            "Done with the Execution",
            VariableType::Execution,
        );

        node.add_output_pin(
            "history_out",
            "History",
            "Updated ChatHistory",
            VariableType::Struct,
        )
        .set_schema::<History>();

        return node;
    }

    async fn run(&self, context: &mut ExecutionContext) -> flow_like_types::Result<()> {
        let mut history: History = context.evaluate_pin("history").await?;
        let response_format: Value = context.evaluate_pin("response_format").await?;

        match response_format {
            Value::String(string) => {
                history.response_format = Some(ResponseFormat::String(string));
            }
            _ => {
                history.response_format = Some(ResponseFormat::Object(response_format));
            }
        }

        context.set_pin_value("history_out", json!(history)).await?;
        context.activate_exec_pin("exec_out").await?;

        Ok(())
    }

    async fn on_update(&self, node: &mut Node, board: Arc<Board>) {
        let match_type = node.match_type("response_format", board.clone(), None, None);

        if let Err(err) = match_type {
            eprintln!("Error: {:?}", err);
            return;
        }

        let match_type = match_type.unwrap();
        if match_type != VariableType::String && match_type != VariableType::Struct {
            if let Some(pin) = node.get_pin_mut_by_name("response_format") {
                pin.depends_on.clear();
            }
        }
    }
}
