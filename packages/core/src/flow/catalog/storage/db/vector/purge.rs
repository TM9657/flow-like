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

#[derive(Default)]
pub struct PurgeLocalDatabaseNode {}

impl PurgeLocalDatabaseNode {
    pub fn new() -> Self {
        PurgeLocalDatabaseNode {}
    }
}

#[async_trait]
impl NodeLogic for PurgeLocalDatabaseNode {
    async fn get_node(&self, _app_state: &FlowLikeState) -> Node {
        let mut node = Node::new(
            "purge_local_db",
            "Purge",
            "Purge Database",
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

        node.add_output_pin(
            "exec_out",
            "Created Database",
            "Done Creating Database",
            VariableType::Execution,
        );

        return node;
    }

    async fn run(&self, context: &mut ExecutionContext) -> anyhow::Result<()> {
        let database: NodeDBConnection = context.evaluate_pin("database").await?;
        let database = database.load(context, &database.cache_key).await?;
        database.purge().await?;
        context.activate_exec_pin("exec_out").await?;
        Ok(())
    }
}
