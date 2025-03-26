use std::{
    collections::HashSet,
    sync::{
        atomic::{AtomicUsize, Ordering},
        Arc,
    },
};

use crate::{
    bit::Bit,
    flow::{
        execution::{
            context::ExecutionContext,
            internal_node::InternalNode,
            log::{LogMessage, LogStat},
            LogLevel,
        },
        node::{Node, NodeLogic},
        pin::PinOptions,
        variable::VariableType,
    },
    models::{history::History, llm::LLMCallback},
    state::FlowLikeState,
};
use async_trait::async_trait;
use dashmap::DashMap;
use serde_json::json;
use tokio::sync::Mutex;

#[derive(Default)]
pub struct InvokeLLMSimpleNode {}

impl InvokeLLMSimpleNode {
    pub fn new() -> Self {
        InvokeLLMSimpleNode {}
    }
}

#[async_trait]
impl NodeLogic for InvokeLLMSimpleNode {
    async fn get_node(&self, _app_state: &FlowLikeState) -> Node {
        let mut node = Node::new(
            "ai_generative_invoke_simple",
            "Invoke Simple",
            "Invokes the Model",
            "AI/Generative",
        );
        node.add_icon("/flow/icons/bot-invoke.svg");

        node.add_input_pin("exec_in", "Input", "Trigger Pin", VariableType::Execution);

        node.add_input_pin("model", "Model", "Model", VariableType::Struct)
            .set_schema::<Bit>()
            .set_options(PinOptions::new().set_enforce_schema(true).build());

        node.add_input_pin("system_prompt", "System Prompt", "", VariableType::String)
            .set_default_value(Some(json!("")));
        node.add_input_pin("prompt", "Prompt", "", VariableType::String)
            .set_default_value(Some(json!("")));

        node.add_output_pin(
            "on_stream",
            "On Stream",
            "Triggers on Streaming Output",
            VariableType::Execution,
        );

        node.add_output_pin("token", "Token", "Token", VariableType::String);

        node.add_output_pin("done", "Done", "Done", VariableType::Execution);

        node.add_output_pin(
            "result",
            "Result",
            "Resulting Model Output",
            VariableType::String,
        );

        node.set_long_running(true);

        return node;
    }

    async fn run(&self, context: &mut ExecutionContext) -> anyhow::Result<()> {
        context.deactivate_exec_pin("done").await?;
        let model = context.evaluate_pin::<Bit>("model").await?;
        let mut model_name = model.id.clone();
        if let Some(meta) = model.meta.get("en") {
            model_name = meta.name.clone();
        }
        let system_prompt = context.evaluate_pin::<String>("system_prompt").await?;
        let prompt = context.evaluate_pin::<String>("prompt").await?;
        let model_factory = context.app_state.lock().await.model_factory.clone();
        let model = model_factory
            .lock()
            .await
            .build(&model, context.app_state.clone())
            .await?;

        let mut history = History::new(model_name.clone(), vec![]);
        history.set_system_prompt(system_prompt.clone());
        history.push_message(crate::models::history::HistoryMessage::from_string(
            crate::models::history::Role::User,
            &prompt,
        ));

        let on_stream = context.get_pin_by_name("on_stream").await?;
        context.activate_exec_pin_ref(&on_stream).await?;

        let connected_nodes = Arc::new(DashMap::new());
        let connected = on_stream.lock().await.connected_to.clone();
        for pin in connected {
            let node = pin.upgrade().ok_or(anyhow::anyhow!("Pin is not set"))?;
            let node = node.lock().await.node.clone();
            if let Some(node) = node.upgrade() {
                let context = Arc::new(Mutex::new(context.create_sub_context(&node).await));
                connected_nodes.insert(node.node.lock().await.id.clone(), context);
            }
        }

        let parent_node_id = context.node.node.lock().await.id.clone();
        let ctx = context.clone();
        let collection_nodes = connected_nodes.clone();
        let callback_count = Arc::new(AtomicUsize::new(0));
        let collection_callback_count = Arc::clone(&callback_count);
        let callback: LLMCallback =
            Arc::new(move |input: crate::models::response_chunk::ResponseChunk| {
                let ctx = ctx.clone();
                let parent_node_id = parent_node_id.clone();
                let connected_nodes = connected_nodes.clone();
                let callback_count = Arc::clone(&callback_count); // Clone the Arc for use in the callback
                Box::pin(async move {
                    let mut recursion_guard = HashSet::new();
                    recursion_guard.insert(parent_node_id.clone());
                    let string_token = input.get_streamed_token().unwrap_or("".to_string());
                    ctx.set_pin_value("token", json!(string_token)).await?;
                    callback_count.fetch_add(1, Ordering::SeqCst);
                    for entry in connected_nodes.iter() {
                        let (id, context) = entry.pair();
                        let mut context = context.lock().await;
                        let mut message = LogMessage::new(
                            &format!("Tracing Token, {:?}", string_token),
                            LogLevel::Debug,
                            None,
                        );
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
                            Err(_) => {
                                println!("Error running stream node {}", id);
                            }
                        }
                    }
                    Ok(())
                })
            });

        let mut message = LogMessage::new(
            &format!("Invoking Model, {}", model_name),
            LogLevel::Info,
            None,
        );

        let res = model.invoke(&history, Some(callback)).await?;
        let mut response_string = "".to_string();

        if let Some(response) = res.last_message() {
            response_string = response.content.clone().unwrap_or("".to_string());
        }

        message.end();
        message.put_stats(LogStat::new(
            None,
            Some(collection_callback_count.load(Ordering::SeqCst) as u64),
            None,
        ));
        context.log(message);

        for entry in collection_nodes.iter() {
            let (_, sub_context) = entry.pair();
            let sub_context = sub_context.lock().await;
            context.push_sub_context(sub_context.clone());
        }

        context
            .set_pin_value("result", json!(response_string))
            .await?;
        context.deactivate_exec_pin("on_stream").await?;
        context.activate_exec_pin("done").await?;

        return Ok(());
    }
}
