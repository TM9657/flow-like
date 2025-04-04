use std::{collections::HashSet, sync::Arc};

use crate::{
    flow::{
        execution::{
            LogLevel, context::ExecutionContext, internal_node::InternalNode, log::LogMessage,
        },
        node::{Node, NodeLogic},
        pin::PinOptions,
        variable::VariableType,
    },
    state::FlowLikeState,
};
use async_trait::async_trait;
use dashmap::DashMap;
use serde_json::json;
use tokio::sync::Mutex;

use super::{HttpRequest, HttpResponse, StreamingCallback};

#[derive(Default)]
pub struct StreamingHttpFetchNode {}

impl StreamingHttpFetchNode {
    pub fn new() -> Self {
        StreamingHttpFetchNode {}
    }
}

#[async_trait]
impl NodeLogic for StreamingHttpFetchNode {
    async fn get_node(&self, _app_state: &FlowLikeState) -> Node {
        let mut node = Node::new(
            "streaming_http_fetch",
            "Streaming HTTP Fetch",
            "Performs an HTTP request",
            "Web/API",
        );

        node.add_icon("/flow/icons/web.svg");

        node.add_input_pin(
            "exec_in",
            "Execute",
            "Initiate the HTTP request",
            VariableType::Execution,
        );
        node.add_input_pin(
            "request",
            "Request",
            "The HTTP request to perform",
            VariableType::Struct,
        )
        .set_schema::<HttpRequest>()
        .set_options(PinOptions::new().set_enforce_schema(true).build());

        node.add_output_pin(
            "streaming_exec",
            "On Stream",
            "Intermediate Result",
            VariableType::Execution,
        );
        node.add_output_pin(
            "streaming_response",
            "Stream Response",
            "The HTTP response",
            VariableType::Byte,
        )
        .set_value_type(crate::flow::pin::ValueType::Array);

        node.add_output_pin(
            "exec_success",
            "Success",
            "Execution if the request succeeds",
            VariableType::Execution,
        );
        node.add_output_pin(
            "response",
            "Response",
            "The HTTP response",
            VariableType::Struct,
        )
        .set_schema::<HttpResponse>();

        node.add_output_pin(
            "exec_error",
            "Error",
            "Execution if the request fails",
            VariableType::Execution,
        );
        node
    }

    async fn run(&self, context: &mut ExecutionContext) -> anyhow::Result<()> {
        context.deactivate_exec_pin("exec_success").await?;
        context.activate_exec_pin("exec_error").await?;

        let streaming_pin = context.get_pin_by_name("streaming_exec").await?;
        let streaming_response_pin = context.get_pin_by_name("streaming_response").await?;
        context.activate_exec_pin_ref(&streaming_pin).await?;

        let request: HttpRequest = context.evaluate_pin("request").await?;
        let client = reqwest::Client::new();
        let connected_nodes = Arc::new(DashMap::new());
        let connected = streaming_pin.lock().await.connected_to.clone();
        for pin in connected {
            let node = pin.upgrade().ok_or(anyhow::anyhow!("Pin is not set"))?;
            let node = node.lock().await.node.clone();
            if let Some(node) = node.upgrade() {
                let context = Arc::new(Mutex::new(context.create_sub_context(&node).await));
                connected_nodes.insert(node.node.lock().await.id.clone(), context);
            }
        }

        let parent_node_id = context.node.node.lock().await.id.clone();
        let callback: StreamingCallback = Arc::new(move |response: bytes::Bytes| {
            let response = response.to_vec();
            let streaming_response_pin = streaming_response_pin.clone();
            let connected_nodes = connected_nodes.clone();
            let parent_node_id = parent_node_id.clone();

            Box::pin(async move {
                let mut recursion_guard = HashSet::new();
                recursion_guard.insert(parent_node_id.clone());
                streaming_response_pin
                    .lock()
                    .await
                    .set_value(json!(response))
                    .await;
                for entry in connected_nodes.iter() {
                    let (_id, context) = entry.pair();
                    let mut context = context.lock().await;
                    let mut message =
                        LogMessage::new("Streaming Intermediate Response", LogLevel::Debug, None);
                    let run = InternalNode::trigger(
                        &mut context,
                        &mut Some(recursion_guard.clone()),
                        true,
                    )
                    .await;
                    message.end();
                    context.log(message);
                    context.end_trace();
                    match run {
                        Ok(_) => {}
                        Err(err) => {
                            println!("Error running stream node {:?}", err);
                        }
                    }
                }
                Ok(())
            })
        });

        let response = request.streaming_trigger(&client, Some(callback)).await?;
        context.deactivate_exec_pin_ref(&streaming_pin).await?;

        context.set_pin_value("response", json!(response)).await?;
        let success = response.is_success();

        if success {
            context.deactivate_exec_pin("exec_error").await?;
            context.activate_exec_pin("exec_success").await?;
        }

        Ok(())
    }
}
