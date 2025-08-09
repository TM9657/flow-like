use std::sync::Arc;

use crate::storage::path::FlowPath;
use flow_like::{
    flow::{
        execution::context::ExecutionContext,
        node::{Node, NodeLogic},
        variable::VariableType,
    },
    state::FlowLikeState,
};
use flow_like_storage::files::store::FlowLikeStore;
use flow_like_types::{Cacheable, async_trait, json::json};

#[derive(Default)]
pub struct VirtualDirNode {}

impl VirtualDirNode {
    pub fn new() -> Self {
        VirtualDirNode {}
    }
}

#[async_trait]
impl NodeLogic for VirtualDirNode {
    async fn get_node(&self, _app_state: &FlowLikeState) -> Node {
        let mut node = Node::new(
            "path_virtual_dir",
            "Virtual Dir",
            "Creates an in-memory virtual directory path",
            "Storage/Paths/Directories",
        );
        node.add_icon("/flow/icons/memory.svg");

        node.add_input_pin(
            "exec_in",
            "Input",
            "Initiate Execution",
            VariableType::Execution,
        );

        node.add_input_pin(
            "name",
            "Name",
            "Virtual directory name",
            VariableType::String,
        )
        .set_default_value(Some(json!("/virtual")));

        node.add_output_pin(
            "exec_out",
            "Output",
            "Done with the Execution",
            VariableType::Execution,
        );

        node.add_output_pin(
            "path",
            "Path",
            "Virtual directory path",
            VariableType::Struct,
        )
        .set_schema::<FlowPath>();

        return node;
    }

    async fn run(&self, context: &mut ExecutionContext) -> flow_like_types::Result<()> {
        context.deactivate_exec_pin("exec_out").await?;

        let name: String = context.evaluate_pin("name").await?;

        let cache_path = format!("virtual_dir_{}", name);

        if !context.has_cache(&cache_path).await {
            let store = FlowLikeStore::Memory(Arc::new(
                flow_like_storage::object_store::memory::InMemory::new(),
            ));
            let store: Arc<dyn Cacheable> = Arc::new(store);
            context.set_cache(&cache_path, store).await;
        }

        let virtual_path = FlowPath {
            path: "".to_string(),
            store_ref: cache_path,
            cache_store_ref: None,
        };
        context.set_pin_value("path", json!(virtual_path)).await?;

        context.activate_exec_pin("exec_out").await?;
        Ok(())
    }
}
