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
pub struct ListLocalDatabaseNode {}

impl ListLocalDatabaseNode {
    pub fn new() -> Self {
        ListLocalDatabaseNode {}
    }
}

#[async_trait]
impl NodeLogic for ListLocalDatabaseNode {
    async fn get_node(&self, _app_state: &FlowLikeState) -> Node {
        let mut node = Node::new(
            "list_local_db",
            "List",
            "List Content",
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

        node.add_input_pin("limit", "Limit", "Limit", VariableType::Integer)
            .set_default_value(Some(json!(10)));

        node.add_input_pin("offset", "Offset", "Offset", VariableType::Integer)
            .set_default_value(Some(json!(0)));

        node.add_output_pin(
            "exec_out",
            "Created Database",
            "Done Creating Database",
            VariableType::Execution,
        );

        node.add_output_pin("values", "Values", "Found Items", VariableType::Struct)
            .set_value_type(crate::flow::pin::ValueType::Array);

        return node;
    }

    async fn run(&self, context: &mut ExecutionContext) -> anyhow::Result<()> {
        let database: NodeDBConnection = context.evaluate_pin("database").await?;
        let limit: i64 = context.evaluate_pin("limit").await?;
        let offset: i64 = context.evaluate_pin("offset").await?;
        let database = database
            .load(context, &database.cache_key)
            .await?
            .db
            .clone();
        let database = database.read().await;
        let results = database.list(limit as usize, offset as usize).await?;
        context.set_pin_value("values", json!(results)).await?;
        context.activate_exec_pin("exec_out").await?;
        Ok(())
    }
}
