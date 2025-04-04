use super::NodeDBConnection;
use crate::{
    flow::{
        execution::context::ExecutionContext,
        node::{Node, NodeLogic},
        pin::PinOptions,
        variable::VariableType,
    },
    state::FlowLikeState,
};
use async_trait::async_trait;
use flow_like_storage::databases::vector::VectorStore;
use serde_json::json;

#[derive(Default)]
pub struct IndexLocalDatabaseNode {}

impl IndexLocalDatabaseNode {
    pub fn new() -> Self {
        IndexLocalDatabaseNode {}
    }
}

#[async_trait]
impl NodeLogic for IndexLocalDatabaseNode {
    async fn get_node(&self, _app_state: &FlowLikeState) -> Node {
        let mut node = Node::new(
            "index_local_db",
            "Build Index",
            "Build Index",
            "Database/Local/Optimization",
        );
        node.add_icon("/flow/icons/database.svg");

        node.add_input_pin("exec_in", "Input", "", VariableType::Execution);
        node.add_input_pin(
            "database",
            "Database",
            "Database Connection Reference",
            VariableType::Struct,
        )
        .set_schema::<NodeDBConnection>()
        .set_options(PinOptions::new().set_enforce_schema(true).build());

        node.add_input_pin("column", "Column", "Column to Index", VariableType::String)
            .set_default_value(Some(json!("")));
        node.add_input_pin(
            "fts",
            "Full-Text Search?",
            "Is this index meant for full text search?",
            VariableType::Boolean,
        )
        .set_default_value(Some(json!(false)));

        node.add_output_pin(
            "exec_out",
            "Created Database",
            "Done Creating Database",
            VariableType::Execution,
        );

        node.add_output_pin(
            "failed",
            "Failed Indexing",
            "Failed to index the column",
            VariableType::Execution,
        );

        return node;
    }

    async fn run(&self, context: &mut ExecutionContext) -> anyhow::Result<()> {
        context.activate_exec_pin("failed").await?;
        context.deactivate_exec_pin("exec_out").await?;
        let database: NodeDBConnection = context.evaluate_pin("database").await?;
        let database = database
            .load(context, &database.cache_key)
            .await?
            .db
            .clone();
        let database = database.read().await;
        let column: String = context.evaluate_pin("column").await?;
        let fts: bool = context.evaluate_pin("fts").await?;
        let result = database.index(&column, fts).await;
        if result.is_ok() {
            context.deactivate_exec_pin("failed").await?;
            context.activate_exec_pin("exec_out").await?;
            return Ok(());
        }
        Ok(())
    }
}
