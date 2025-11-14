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
use flow_like_types::{Value, async_trait};

use super::NodeDBConnection;

#[crate::register_node]
#[derive(Default)]
pub struct UpsertLocalDatabaseNode {}

impl UpsertLocalDatabaseNode {
    pub fn new() -> Self {
        UpsertLocalDatabaseNode {}
    }
}

#[async_trait]
impl NodeLogic for UpsertLocalDatabaseNode {
    async fn get_node(&self, _app_state: &FlowLikeState) -> Node {
        let mut node = Node::new(
            "upsert_local_db",
            "Upsert",
            "Inserts if the Item does not exist, Updates if it does",
            "Data/Database/Insert",
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
        node.add_input_pin("id_row", "ID Column", "The ID Column", VariableType::String);

        node.add_input_pin("value", "Value", "Value to Insert", VariableType::Struct);

        node.add_output_pin(
            "exec_out",
            "Created Database",
            "Done Creating Database",
            VariableType::Execution,
        );

        return node;
    }

    async fn run(&self, context: &mut ExecutionContext) -> flow_like_types::Result<()> {
        context.deactivate_exec_pin("exec_out").await?;

        let database: NodeDBConnection = context.evaluate_pin("database").await?;
        let database = database.load(context).await?.db.clone();
        let mut database = database.write().await;
        let id_row: String = context.evaluate_pin("id_row").await?;
        let value: Value = context.evaluate_pin("value").await?;
        let value = vec![value];
        database.upsert(value, id_row).await?;

        context.activate_exec_pin("exec_out").await?;

        Ok(())
    }
}

#[crate::register_node]
#[derive(Default)]
pub struct BatchUpsertLocalDatabaseNode {}

impl BatchUpsertLocalDatabaseNode {
    pub fn new() -> Self {
        BatchUpsertLocalDatabaseNode {}
    }
}

#[async_trait]
impl NodeLogic for BatchUpsertLocalDatabaseNode {
    async fn get_node(&self, _app_state: &FlowLikeState) -> Node {
        let mut node = Node::new(
            "batch_upsert_local_db",
            "Batch Upsert",
            "Inserts if the Item does not exist, Updates if it does",
            "Data/Database/Insert",
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
        node.add_input_pin("id_row", "ID Column", "The ID Column", VariableType::String);

        node.add_input_pin("value", "Value", "Value to Insert", VariableType::Struct)
            .set_value_type(ValueType::Array);

        node.add_output_pin(
            "exec_out",
            "Created Database",
            "Done Creating Database",
            VariableType::Execution,
        );

        return node;
    }

    async fn run(&self, context: &mut ExecutionContext) -> flow_like_types::Result<()> {
        context.deactivate_exec_pin("exec_out").await?;

        let database: NodeDBConnection = context.evaluate_pin("database").await?;
        let database = database.load(context).await?.db.clone();
        let mut database = database.write().await;
        let value: Vec<Value> = context.evaluate_pin("value").await?;
        let id_row: String = context.evaluate_pin("id_row").await?;
        database.upsert(value, id_row).await?;

        context.activate_exec_pin("exec_out").await?;

        Ok(())
    }
}
