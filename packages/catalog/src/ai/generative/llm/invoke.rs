use ahash::AHashSet;
use flow_like::{
    bit::Bit,
    flow::{
        execution::{
            LogLevel,
            context::ExecutionContext,
            internal_node::InternalNode,
            log::{LogMessage, LogStat},
        },
        node::{Node, NodeLogic},
        pin::PinOptions,
        variable::VariableType,
    },
    state::FlowLikeState,
};
use flow_like_model_provider::{
    history::History, llm::LLMCallback, response::Response, response_chunk::ResponseChunk,
};
use flow_like_types::{
    async_trait,
    json::json,
    sync::{DashMap, Mutex},
};
use std::sync::{
    Arc,
    atomic::{AtomicUsize, Ordering},
};

#[derive(Default)]
pub struct InvokeLLM {}

impl InvokeLLM {
    pub fn new() -> Self {
        InvokeLLM {}
    }
}

#[async_trait]
impl NodeLogic for InvokeLLM {
    async fn get_node(&self, _app_state: &FlowLikeState) -> Node {
        let mut node = Node::new(
            "ai_generative_invoke",
            "Invoke Model",
            "Invokes the Model",
            "AI/Generative",
        );
        node.add_icon("/flow/icons/bot-invoke.svg");

        node.add_input_pin("exec_in", "Input", "Trigger Pin", VariableType::Execution);

        node.add_input_pin("model", "Model", "Model", VariableType::Struct)
            .set_schema::<Bit>()
            .set_options(PinOptions::new().set_enforce_schema(true).build());

        node.add_input_pin("history", "History", "Chat History", VariableType::Struct)
            .set_schema::<History>()
            .set_options(PinOptions::new().set_enforce_schema(true).build());

        node.add_output_pin(
            "on_stream",
            "On Stream",
            "Triggers on Streaming Output",
            VariableType::Execution,
        );

        node.add_output_pin("chunk", "Chunk", "", VariableType::Struct)
            .set_schema::<ResponseChunk>()
            .set_options(PinOptions::new().set_enforce_schema(true).build());

        node.add_output_pin("done", "Done", "Done", VariableType::Execution);

        node.add_output_pin(
            "result",
            "Result",
            "Resulting Model Output",
            VariableType::Struct,
        )
        .set_schema::<Response>()
        .set_options(PinOptions::new().set_enforce_schema(true).build());

        node.set_long_running(true);

        return node;
    }

    async fn run(&self, context: &mut ExecutionContext) -> flow_like_types::Result<()> {
        context.deactivate_exec_pin("done").await?;
        let model = context.evaluate_pin::<Bit>("model").await?;
        let mut model_name = model.id.clone();
        if let Some(meta) = model.meta.get("en") {
            model_name = meta.name.clone();
        }
        let history = context.evaluate_pin::<History>("history").await?;
        let model_factory = context.app_state.lock().await.model_factory.clone();
        let model = model_factory
            .lock()
            .await
            .build(&model, context.app_state.clone(), context.token.clone())
            .await?;

        let on_stream = context.get_pin_by_name("on_stream").await?;
        context.activate_exec_pin_ref(&on_stream).await?;

        let connected_nodes = Arc::new(DashMap::new());
        let connected = on_stream.lock().await.get_connected_nodes().await;
        for node in connected {
            let context = Arc::new(Mutex::new(context.create_sub_context(&node).await));
            connected_nodes.insert(node.node.lock().await.id.clone(), context);
        }

        let parent_node_id = context.node.node.lock().await.id.clone();
        let ctx = context.clone();
        let collection_nodes = connected_nodes.clone();
        let callback_count = Arc::new(AtomicUsize::new(0));
        let collection_callback_count = Arc::clone(&callback_count);
        let callback: LLMCallback = Arc::new(move |input: ResponseChunk| {
            let ctx = ctx.clone();
            let parent_node_id = parent_node_id.clone();
            let connected_nodes = connected_nodes.clone();
            let callback_count = Arc::clone(&callback_count); // Clone the Arc for use in the callback
            Box::pin(async move {
                let mut recursion_guard = AHashSet::new();
                recursion_guard.insert(parent_node_id.clone());
                let string_token = input.get_streamed_token().unwrap_or("".to_string());
                ctx.set_pin_value("chunk", json!(input)).await?;
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

        message.end();
        message.put_stats(LogStat::new(
            None,
            Some(collection_callback_count.load(Ordering::SeqCst) as u64),
            None,
        ));
        context.log(message);

        for entry in collection_nodes.iter() {
            let (_, sub_context) = entry.pair();
            let mut sub_context = sub_context.lock().await;
            context.push_sub_context(&mut sub_context);
        }

        context.set_pin_value("result", json!(res)).await?;
        context.deactivate_exec_pin("on_stream").await?;
        context.activate_exec_pin("done").await?;

        return Ok(());
    }
}
