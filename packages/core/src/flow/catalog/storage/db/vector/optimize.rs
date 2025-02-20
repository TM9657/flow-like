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
use serde_json::json;
use std::sync::Arc;

#[derive(Default)]
pub struct OptimizeLocalDatabaseNode {}

impl OptimizeLocalDatabaseNode {
    pub fn new() -> Self {
        OptimizeLocalDatabaseNode {}
    }
}

#[async_trait]
impl NodeLogic for OptimizeLocalDatabaseNode {
    async fn get_node(&self, _app_state: &FlowLikeState) -> Node {
        let mut node = Node::new(
            "optimize_local_db",
            "Optimize and Update",
            "Optimize and Update the Database",
            "Database/Local/Optimization",
        );
        node.set_long_running(true);
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
            "keep_versions",
            "Keep Versions?",
            "Otherwise deletes old versions",
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
            "success",
            "Success",
            "Was optimization successfull?",
            VariableType::Boolean,
        );

        return node;
    }

    async fn run(&mut self, context: &mut ExecutionContext) -> anyhow::Result<()> {
        let database: NodeDBConnection = context.evaluate_pin("database").await?;
        let database = database.load(context, &database.cache_key).await?;
        let keep_versions: bool = context.evaluate_pin("keep_versions").await?;
        let result = database.optimize(keep_versions).await;
        context
            .set_pin_value("success", json!(result.is_ok()))
            .await?;
        context.activate_exec_pin("exec_out").await?;
        Ok(())
    }
}
