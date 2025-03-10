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
pub struct VectorSearchLocalDatabaseNode {}

impl VectorSearchLocalDatabaseNode {
    pub fn new() -> Self {
        VectorSearchLocalDatabaseNode {}
    }
}

#[async_trait]
impl NodeLogic for VectorSearchLocalDatabaseNode {
    async fn get_node(&self, _app_state: &FlowLikeState) -> Node {
        let mut node = Node::new(
            "vector_search_local_db",
            "Vector Search",
            "Searches the Database based on a Vector",
            "Database/Local/Search",
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
        node.add_input_pin("vector", "Vector", "Vector to Search", VariableType::Float)
            .set_value_type(crate::flow::pin::ValueType::Array);
        node.add_input_pin(
            "filter",
            "SQL Filter",
            "Optional SQL Filter",
            VariableType::String,
        )
        .set_default_value(Some(json!("")));

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

    async fn run(&mut self, context: &mut ExecutionContext) -> anyhow::Result<()> {
        let database: NodeDBConnection = context.evaluate_pin("database").await?;
        let vector: Vec<f64> = context.evaluate_pin("vector").await?;
        let filter: String = context.evaluate_pin("filter").await?;
        let filter: Option<&str> = if filter.is_empty() {
            None
        } else {
            Some(&filter)
        };
        let limit: i64 = context.evaluate_pin("limit").await?;
        let offset: i64 = context.evaluate_pin("offset").await?;
        let database = database.load(context, &database.cache_key).await?;
        let results = database
            .vector_search(vector, filter, limit as usize, offset as usize)
            .await?;
        context.set_pin_value("values", json!(results)).await?;
        context.activate_exec_pin("exec_out").await?;
        Ok(())
    }
}
