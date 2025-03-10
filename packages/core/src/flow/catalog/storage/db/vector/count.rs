use super::NodeDBConnection;
use crate::{
    db::vector::VectorStore,
    flow::{
        execution::context::ExecutionContext,
        node::{Node, NodeLogic},
        pin::PinOptions,
        variable::VariableType,
    },
    state::FlowLikeState,
};
use async_trait::async_trait;
use serde_json::json;

#[derive(Default)]
pub struct CountLocalDatabaseNode {}

impl CountLocalDatabaseNode {
    pub fn new() -> Self {
        CountLocalDatabaseNode {}
    }
}

#[async_trait]
impl NodeLogic for CountLocalDatabaseNode {
    async fn get_node(&self, _app_state: &FlowLikeState) -> Node {
        let mut node = Node::new(
            "count_local_db",
            "Count",
            "Count Items",
            "Database/Local/Meta",
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

        node.add_output_pin("count", "Count", "Found Items Count", VariableType::Integer);

        return node;
    }

    async fn run(&mut self, context: &mut ExecutionContext) -> anyhow::Result<()> {
        let database: NodeDBConnection = context.evaluate_pin("database").await?;
        let database = database.load(context, &database.cache_key).await?;
        let filter: String = context.evaluate_pin("filter").await?;
        let filter: Option<String> = if filter.is_empty() {
            None
        } else {
            Some(filter)
        };
        let result = database.count(filter).await?;
        context.set_pin_value("count", json!(result)).await?;
        context.activate_exec_pin("exec_out").await?;
        Ok(())
    }
}
