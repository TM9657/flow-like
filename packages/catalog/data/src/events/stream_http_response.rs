use flow_like::flow::{
    board::Board,
    execution::context::ExecutionContext,
    node::{Node, NodeLogic},
    variable::VariableType,
};
use flow_like_types::{Value, async_trait, http_streaming::HTTP_RESPONSE_CHUNK};

#[crate::register_node]
#[derive(Default)]
pub struct StreamHttpResponseNode {}

#[async_trait]
impl NodeLogic for StreamHttpResponseNode {
    fn get_node(&self) -> Node {
        let mut node = Node::new(
            "events_http_stream_response",
            "Stream HTTP Response",
            "Send a partial JSON response to an HTTP caller that requested streaming. The final result is unchanged.",
            "Events/HTTP",
        );
        node.set_flowscript_name("http", "streamResponse");
        node.add_icon("/flow/icons/event.svg");
        node.set_event_callback(true);
        node.add_input_pin(
            "exec_in",
            "Input",
            "Send the partial response",
            VariableType::Execution,
        );
        node.add_input_pin(
            "response",
            "Response",
            "Partial response serialized as JSON",
            VariableType::Generic,
        );
        node.add_output_pin(
            "exec_out",
            "Output",
            "Continue execution after sending the partial response",
            VariableType::Execution,
        );
        node
    }

    async fn run(&self, context: &mut ExecutionContext) -> flow_like_types::Result<()> {
        context.deactivate_exec_pin("exec_out").await?;
        let response: Value = context.evaluate_pin("response").await?;
        context
            .stream_response(HTTP_RESPONSE_CHUNK, response)
            .await?;
        context.activate_exec_pin("exec_out").await?;
        Ok(())
    }

    async fn on_update(&self, node: &mut Node, board: &Board) {
        let _ = node.match_type("response", board, None, None);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use flow_like::{
        flow::{
            board::ExecutionStage,
            execution::{LogLevel, internal_node::InternalNode, internal_pin::InternalPin},
        },
        profile::Profile,
        state::{FlowLikeConfig, FlowLikeState},
        utils::http::HTTPClient,
    };
    use flow_like_types::{
        intercom::InterComCallback,
        json::json,
        sync::{Mutex, RwLock},
    };
    use std::sync::{Arc, Weak};

    async fn context(callback: InterComCallback) -> ExecutionContext {
        let logic = Arc::new(StreamHttpResponseNode::default());
        let node = logic.get_node();
        let pins = node
            .pins
            .iter()
            .map(|(id, pin)| (id.clone(), Arc::new(InternalPin::new(pin, false))))
            .collect();
        let node = Arc::new(InternalNode::new(node, pins, logic, Default::default()));
        let state = Arc::new(FlowLikeState::new(
            FlowLikeConfig::new(),
            HTTPClient::new_without_refetch(),
        ));
        ExecutionContext::new(
            Arc::new(
                [(node.node_id().to_string(), node.clone())]
                    .into_iter()
                    .collect(),
            ),
            &Weak::new(),
            &state,
            &node,
            &Arc::new(Mutex::new(Default::default())),
            &Arc::new(RwLock::new(Default::default())),
            LogLevel::Info,
            ExecutionStage::Dev,
            Arc::new(Profile::default()),
            callback,
            Arc::new(RwLock::new(Vec::new())),
            None,
            None,
            Arc::new(Default::default()),
            None,
        )
        .await
    }

    #[tokio::test]
    async fn partial_values_continue_execution_without_replacing_the_result() {
        let (tx, mut rx) = tokio::sync::mpsc::channel(4);
        let callback: InterComCallback = Some(Arc::new(move |event| {
            let tx = tx.clone();
            Box::pin(async move { tx.send(event).await.map_err(Into::into) })
        }));
        let mut context = context(callback).await;
        context.set_result(json!({"final": true}));
        for value in [json!("first\nline"), json!({"items": [1, false, null]})] {
            context
                .set_pin_value("response", value.clone())
                .await
                .unwrap();
            StreamHttpResponseNode::default()
                .run(&mut context)
                .await
                .unwrap();
            let event = rx.try_recv().expect("chunk sent during node execution");
            assert_eq!(event.event_type, HTTP_RESPONSE_CHUNK);
            assert_eq!(event.payload, value);
            assert_eq!(context.result, Some(json!({"final": true})));
            assert_eq!(
                context
                    .get_pin_by_name("exec_out")
                    .await
                    .unwrap()
                    .get_raw_value()
                    .await,
                Some(json!(true))
            );
        }
    }

    #[tokio::test]
    async fn a_flow_without_a_streaming_callback_keeps_its_final_result() {
        let mut context = context(None).await;
        context.set_result(json!(42));
        context
            .set_pin_value("response", json!({"progress": 0.5}))
            .await
            .unwrap();
        StreamHttpResponseNode::default()
            .run(&mut context)
            .await
            .unwrap();
        assert_eq!(context.result, Some(json!(42)));
    }
}
