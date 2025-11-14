use std::sync::Arc;

use crate::{data::path::FlowPath, events::chat_event::Attachment};
use flow_like::{
    flow::{
        execution::context::ExecutionContext,
        node::{Node, NodeLogic},
        pin::PinOptions,
        variable::VariableType,
    },
    state::FlowLikeState,
};
use flow_like_model_provider::history::{Content, History, MessageContent};
use flow_like_storage::files::store::FlowLikeStore;
use flow_like_types::Cacheable;
use flow_like_types::{async_trait, json::json};

fn extract_image_urls(history: &History) -> Vec<String> {
    history
        .messages
        .last()
        .and_then(|m| match &m.content {
            MessageContent::Contents(contents) => Some(contents),
            _ => None,
        })
        .map(|contents| {
            contents
                .iter()
                .filter_map(|c| match c {
                    Content::Image { image_url, .. } => Some(image_url.url.clone()),
                    _ => None,
                })
                .collect()
        })
        .unwrap_or_default()
}

#[crate::register_node]
#[derive(Default)]
pub struct ExtractAttachments {}

impl ExtractAttachments {
    pub fn new() -> Self {
        ExtractAttachments {}
    }
}

#[async_trait]
impl NodeLogic for ExtractAttachments {
    async fn get_node(&self, _app_state: &FlowLikeState) -> Node {
        let mut node = Node::new(
            "ai_gen_llm_history_extract_attachments",
            "Extract Attachments",
            "Extracts attachments from the chat",
            "Events/Chat",
        );
        node.add_icon("/flow/icons/paperclip.svg");

        node.add_input_pin(
            "exec_in",
            "Input",
            "Initiate Execution",
            VariableType::Execution,
        );

        node.add_input_pin(
            "history",
            "History",
            "History of the Chat",
            VariableType::Struct,
        )
        .set_schema::<History>()
        .set_options(PinOptions::new().set_enforce_schema(true).build());

        node.add_input_pin(
            "attachments",
            "Attachments",
            "Attachments from the Chat",
            VariableType::Struct,
        )
        .set_schema::<Attachment>()
        .set_default_value(Some(flow_like_types::json::json!([])))
        .set_value_type(flow_like::flow::pin::ValueType::Array)
        .set_options(PinOptions::new().set_enforce_schema(true).build());

        node.add_output_pin(
            "exec_out",
            "Output",
            "Done with the Execution",
            VariableType::Execution,
        );

        node.add_output_pin(
            "paths",
            "Paths",
            "Extracted Attachment Paths",
            VariableType::Struct,
        )
        .set_schema::<FlowPath>()
        .set_value_type(flow_like::flow::pin::ValueType::Array)
        .set_options(PinOptions::new().set_enforce_schema(true).build());

        return node;
    }

    async fn run(&self, context: &mut ExecutionContext) -> flow_like_types::Result<()> {
        context.deactivate_exec_pin("exec_out").await?;
        let mut attachments = context
            .evaluate_pin::<Vec<Attachment>>("attachments")
            .await?;

        if let Ok(history) = context.evaluate_pin::<History>("history").await {
            let urls = extract_image_urls(&history);
            attachments.extend(urls.into_iter().map(Attachment::Url));
        }

        let id = context.id.clone();
        let cache_path = format!("virtual_dir_{}", id);

        if !context.has_cache(&cache_path).await {
            let store = FlowLikeStore::Memory(Arc::new(
                flow_like_storage::object_store::memory::InMemory::new(),
            ));
            let store: Arc<dyn Cacheable> = Arc::new(store);
            context.set_cache(&cache_path, store).await;
        }

        let mut paths = Vec::with_capacity(attachments.len());
        let virtual_path = FlowPath {
            path: "".to_string(),
            store_ref: cache_path.clone(),
            cache_store_ref: None,
        };

        let runtime = virtual_path.to_runtime(context).await?;
        let store = runtime.store;

        for attachment in &attachments {
            match attachment {
                Attachment::Url(url) => {
                    let (path, _len) = store.put_from_url(url).await?;
                    let virtual_path = FlowPath {
                        path: path.to_string(),
                        store_ref: cache_path.clone(),
                        cache_store_ref: None,
                    };
                    paths.push(virtual_path);
                }
                Attachment::Complex(complex) => {
                    let (path, _len) = store.put_from_url(&complex.url).await?;
                    let virtual_path = FlowPath {
                        path: path.to_string(),
                        store_ref: cache_path.clone(),
                        cache_store_ref: None,
                    };
                    paths.push(virtual_path);
                }
            }
        }

        context.set_pin_value("paths", json!(&paths)).await?;
        context.activate_exec_pin("exec_out").await?;

        return Ok(());
    }
}
