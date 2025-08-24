use flow_like::{
    flow::{
        execution::context::ExecutionContext,
        node::{Node, NodeLogic},
        pin::{PinOptions, ValueType},
        variable::VariableType,
    },
    state::FlowLikeState,
};
use flow_like_storage::databases::vector::VectorStore;
use flow_like_types::{async_trait, json::json};

use super::NodeDBConnection;

#[derive(Default)]
pub struct FTSLocalDatabaseNode {}

impl FTSLocalDatabaseNode {
    pub fn new() -> Self {
        FTSLocalDatabaseNode {}
    }
}

#[async_trait]
impl NodeLogic for FTSLocalDatabaseNode {
    async fn get_node(&self, _app_state: &FlowLikeState) -> Node {
        let mut node = Node::new(
            "fts_search_local_db",
            "Full-Text Search",
            "Searches the Database based on a Vector and Text",
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
        node.add_input_pin(
            "search",
            "Search Term",
            "Full Text Search Term",
            VariableType::String,
        )
        .set_default_value(Some(json!("")));

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
            .set_value_type(ValueType::Array);

        return node;
    }

    async fn run(&self, context: &mut ExecutionContext) -> flow_like_types::Result<()> {
        context.deactivate_exec_pin("exec_out").await?;

        let database: NodeDBConnection = context.evaluate_pin("database").await?;
        let search: String = context.evaluate_pin("search").await?;
        let filter: String = context.evaluate_pin("filter").await?;
        let filter: Option<&str> = if filter.is_empty() {
            None
        } else {
            Some(&filter)
        };
        let limit: i64 = context.evaluate_pin("limit").await?;
        let offset: i64 = context.evaluate_pin("offset").await?;
        let database = database
            .load(context, &database.cache_key)
            .await?
            .db
            .clone();
        let database = database.read().await;
        let results = database
            .fts_search(&search, filter, None, limit as usize, offset as usize)
            .await?;
        context.set_pin_value("values", json!(results)).await?;
        context.activate_exec_pin("exec_out").await?;
        Ok(())
    }
}
