
use flow_like::{
    flow::{
        execution::context::ExecutionContext,
        node::{Node, NodeLogic},
        pin::PinOptions,
        variable::VariableType,
    },
    state::FlowLikeState,
};
use flow_like_storage::databases::vector::VectorStore;
use flow_like_types::{async_trait, json::json};

use super::NodeDBConnection;

/// # Get Database Schema
/// Retrieves schema from local database as struct
#[derive(Default)]
pub struct GetSchemaLocalDatabaseNode {}

impl GetSchemaLocalDatabaseNode {
    pub fn new() -> Self {
        GetSchemaLocalDatabaseNode {}
    }
}

#[async_trait]
impl NodeLogic for GetSchemaLocalDatabaseNode {
    async fn get_node(&self, _app_state: &FlowLikeState) -> Node {
        let mut node = Node::new(
            "schema_local_db",
            "Get Schema",
            "Get Local Database Schema",
            "Database/Local/Meta",
        );
        node.add_icon("/flow/icons/database.svg");

        // inputs
        node.add_input_pin(
            "exec_in", 
            "Input", 
            "", 
            VariableType::Execution
        );
        
        node.add_input_pin(
            "database",
            "Database",
            "Database Connection Reference",
            VariableType::Struct,
        )
        .set_schema::<NodeDBConnection>()
        .set_options(
            PinOptions::new()
            .set_enforce_schema(true)
            .build()
        );

        // outputs
        node.add_output_pin(
            "exec_out",
            "Done",
            "Done Fetching Local Database Schema",
            VariableType::Execution,
        );

        node.add_output_pin(
            "schema", 
            "Schema", 
            "Local Database Schema", 
            VariableType::Struct
        );
        node
    }

    async fn run(&self, context: &mut ExecutionContext) -> flow_like_types::Result<()> {
        // get inputs
        context.deactivate_exec_pin("exec_out").await?;
        let database: NodeDBConnection = context.evaluate_pin("database").await?;
        let database = database
            .load(context, &database.cache_key)
            .await?
            .db
            .clone();
        let database = database.read().await;
        
        // get schema
        let schema = database.schema().await?;
        
        // set outputs
        context.set_pin_value("schema", json!(schema)).await?;
        context.activate_exec_pin("exec_out").await?;
        Ok(())
    }
}
