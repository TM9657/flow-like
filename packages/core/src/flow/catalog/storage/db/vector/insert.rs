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
use serde_json::Value;

#[derive(Default)]
pub struct InsertLocalDatabaseNode {}

impl InsertLocalDatabaseNode {
    pub fn new() -> Self {
        InsertLocalDatabaseNode {}
    }
}

#[async_trait]
impl NodeLogic for InsertLocalDatabaseNode {
    async fn get_node(&self, _app_state: &FlowLikeState) -> Node {
        let mut node = Node::new(
            "insert_local_db",
            "Insert",
            "Faster than Upsert, but might write duplicate items.",
            "Database/Local/Insert",
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

        node.add_input_pin("value", "Value", "Value to Insert", VariableType::Struct);

        node.add_output_pin(
            "exec_out",
            "Created Database",
            "Done Creating Database",
            VariableType::Execution,
        );

        node.add_output_pin(
            "failed",
            "Failed Insert",
            "Triggered if the Ingest failed",
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
        let mut database = database.write().await;
        let value: Value = context.evaluate_pin("value").await?;
        let value = vec![value];
        let results = database.insert(value).await;

        if results.is_ok() {
            context.deactivate_exec_pin("failed").await?;
            context.activate_exec_pin("exec_out").await?;
        }

        Ok(())
    }
}

#[derive(Default)]
pub struct BatchInsertLocalDatabaseNode {}

impl BatchInsertLocalDatabaseNode {
    pub fn new() -> Self {
        BatchInsertLocalDatabaseNode {}
    }
}

#[async_trait]
impl NodeLogic for BatchInsertLocalDatabaseNode {
    async fn get_node(&self, _app_state: &FlowLikeState) -> Node {
        let mut node = Node::new(
            "batch_insert_local_db",
            "Batch Insert",
            "Inserts multiple items at once. Faster than Upsert but might produce duplicates.",
            "Database/Local/Insert",
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

        node.add_input_pin("value", "Value", "Value to Insert", VariableType::Struct)
            .set_value_type(crate::flow::pin::ValueType::Array);

        node.add_output_pin(
            "exec_out",
            "Created Database",
            "Done Creating Database",
            VariableType::Execution,
        );

        node.add_output_pin(
            "failed",
            "Failed Insert",
            "Triggered if the Ingest failed",
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
        let mut database = database.write().await;
        let value: Vec<Value> = context.evaluate_pin("value").await?;
        let results = database.insert(value).await;

        if results.is_ok() {
            context.deactivate_exec_pin("failed").await?;
            context.activate_exec_pin("exec_out").await?;
            return Ok(());
        }

        Ok(())
    }
}
