use super::NodeDBConnection;
use crate::{
    flow::{
        board::Board,
        execution::{context::ExecutionContext, Cacheable},
        node::{Node, NodeLogic},
        pin::PinOptions,
        variable::VariableType,
    },
    state::FlowLikeState,
    vault::vector::{lancedb::LanceDBVectorStore, VectorStore},
};
use async_trait::async_trait;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::sync::Arc;

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

        node.add_output_pin("success", "Success", "Success", VariableType::Boolean);

        return node;
    }

    async fn run(&mut self, context: &mut ExecutionContext) -> anyhow::Result<()> {
        let database: NodeDBConnection = context.evaluate_pin("database").await?;
        let mut database = database.load(context, &database.cache_key).await?;
        let value: Value = context.evaluate_pin("value").await?;
        let value = vec![value];
        let results = database.insert(value).await;
        context
            .set_pin_value("success", json!(results.is_ok()))
            .await?;
        context.activate_exec_pin("exec_out").await?;
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

        node.add_output_pin("success", "Success", "Success", VariableType::Boolean);

        return node;
    }

    async fn run(&mut self, context: &mut ExecutionContext) -> anyhow::Result<()> {
        let database: NodeDBConnection = context.evaluate_pin("database").await?;
        let mut database = database.load(context, &database.cache_key).await?;
        let value: Vec<Value> = context.evaluate_pin("value").await?;
        let results = database.insert(value).await;
        context
            .set_pin_value("success", json!(results.is_ok()))
            .await?;
        context.activate_exec_pin("exec_out").await?;
        Ok(())
    }
}
