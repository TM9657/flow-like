use super::NodeDBConnection;
use crate::{
    flow::{
        execution::context::ExecutionContext,
        node::{Node, NodeLogic},
        pin::PinOptions,
        variable::VariableType,
    },
    state::FlowLikeState,
    vault::vector::VectorStore,
};
use async_trait::async_trait;
use serde_json::json;

#[derive(Default)]
pub struct DeleteLocalDatabaseNode {}

impl DeleteLocalDatabaseNode {
    pub fn new() -> Self {
        DeleteLocalDatabaseNode {}
    }
}

#[async_trait]
impl NodeLogic for DeleteLocalDatabaseNode {
    async fn get_node(&self, _app_state: &FlowLikeState) -> Node {
        let mut node = Node::new(
            "filter_local_db",
            "(SQL) Filter Database",
            "Filter Database",
            "Database/Local/Delete",
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

        node.add_input_pin(
            "filter",
            "SQL Filter",
            "Optional SQL Filter",
            VariableType::String,
        )
        .set_default_value(Some(json!("")));

        node.add_output_pin(
            "exec_out",
            "Created Database",
            "Done Creating Database",
            VariableType::Execution,
        );

        return node;
    }

    async fn run(&mut self, context: &mut ExecutionContext) -> anyhow::Result<()> {
        let database: NodeDBConnection = context.evaluate_pin("database").await?;
        let database = database.load(context, &database.cache_key).await?;
        let filter: String = context.evaluate_pin("filter").await?;
        database.delete(&filter).await?;
        context.activate_exec_pin("exec_out").await?;
        Ok(())
    }
}
