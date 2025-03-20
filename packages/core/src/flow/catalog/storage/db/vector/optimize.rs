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
            "failed",
            "Failed Optimization",
            "Triggered if the Ingest failed",
            VariableType::Execution,
        );

        return node;
    }

    async fn run(&self, context: &mut ExecutionContext) -> anyhow::Result<()> {
        context.activate_exec_pin("failed").await?;
        context.deactivate_exec_pin("exec_out").await?;
        let database: NodeDBConnection = context.evaluate_pin("database").await?;
        let database = database.load(context, &database.cache_key).await?;
        let keep_versions: bool = context.evaluate_pin("keep_versions").await?;
        let result = database.optimize(keep_versions).await;

        if result.is_ok() {
            context.deactivate_exec_pin("failed").await?;
            context.activate_exec_pin("exec_out").await?;
        }

        Ok(())
    }
}
