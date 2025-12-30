use flow_like::{
    flow::{
        execution::{LogLevel, context::ExecutionContext},
        node::{Node, NodeLogic},
        pin::{PinOptions, ValueType},
        variable::VariableType,
    },
    state::FlowLikeState,
};
use flow_like_storage::{databases::vector::VectorStore, object_store::buffered::BufReader};
use flow_like_types::{Value, async_trait, json::json};
use futures::StreamExt;

use crate::data::path::FlowPath;

use super::NodeDBConnection;

#[crate::register_node]
#[derive(Default)]
pub struct InsertLocalDatabaseNode {}

impl InsertLocalDatabaseNode {
    pub fn new() -> Self {
        InsertLocalDatabaseNode {}
    }
}

#[async_trait]
impl NodeLogic for InsertLocalDatabaseNode {
    fn get_node(&self) -> Node {
        let mut node = Node::new(
            "insert_local_db",
            "Insert",
            "Faster than Upsert, but might write duplicate items.",
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
        let value: Value = context.evaluate_pin("value").await?;
        let value = vec![value];
        database.insert(value).await?;

        context.activate_exec_pin("exec_out").await?;

        Ok(())
    }
}

#[crate::register_node]
#[derive(Default)]
pub struct BatchInsertLocalDatabaseNode {}

impl BatchInsertLocalDatabaseNode {
    pub fn new() -> Self {
        BatchInsertLocalDatabaseNode {}
    }
}

#[async_trait]
impl NodeLogic for BatchInsertLocalDatabaseNode {
    fn get_node(&self) -> Node {
        let mut node = Node::new(
            "batch_insert_local_db",
            "Batch Insert",
            "Inserts multiple items at once. Faster than Upsert but might produce duplicates.",
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
        database.insert(value).await?;

        context.activate_exec_pin("exec_out").await?;

        Ok(())
    }
}

#[crate::register_node]
#[derive(Default)]
pub struct BatchInsertCSVLocalDatabaseNode {}

impl BatchInsertCSVLocalDatabaseNode {
    pub fn new() -> Self {
        BatchInsertCSVLocalDatabaseNode {}
    }
}

#[async_trait]
impl NodeLogic for BatchInsertCSVLocalDatabaseNode {
    fn get_node(&self) -> Node {
        let mut node = Node::new(
            "csv_insert_local_db",
            "Batch Insert (CSV)",
            "Inserts multiple items at once. Faster than Upsert but might produce duplicates.",
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

        node.add_input_pin("csv", "CSV", "CSV Path", VariableType::Struct)
            .set_schema::<FlowPath>()
            .set_options(PinOptions::new().set_enforce_schema(true).build());

        node.add_input_pin(
            "chunk_size",
            "Chunk Size",
            "Chunk Size for Buffered Read",
            VariableType::Integer,
        )
        .set_default_value(Some(json!(10_000)));

        node.add_input_pin(
            "delimiter",
            "Delimiter",
            "Delimiter for CSV",
            VariableType::String,
        )
        .set_default_value(Some(json!(",")));

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
        let delimiter: String = context.evaluate_pin("delimiter").await?;
        let delimiter = delimiter.as_bytes()[0];
        let csv_path: FlowPath = context.evaluate_pin("csv").await?;
        let store = csv_path.to_runtime(context).await?;
        let location = store.path.clone();
        let get_request = store.store.as_generic().get(&location).await?;
        let reader = BufReader::new(store.store.as_generic(), &get_request.meta);

        let mut rdr = csv_async::AsyncReaderBuilder::new()
            .has_headers(true)
            .buffer_capacity(32 * 1024 * 1024)
            .delimiter(delimiter)
            .create_reader(reader);

        let chunk_size: u64 = context.evaluate_pin("chunk_size").await?;
        let headers = rdr.byte_headers().await?.clone();
        let headers = headers
            .iter()
            .map(|h| {
                let lossy_header = String::from_utf8_lossy(h);
                lossy_header.to_string()
            })
            .collect::<Vec<String>>();

        let mut records = rdr.byte_records();
        let mut chunk = Vec::with_capacity(chunk_size as usize);

        while let Some(element) = records.next().await {
            let record = match element {
                Ok(record) => record,
                Err(e) => {
                    println!("Error reading record: {:?}", e);
                    continue;
                }
            };
            let json_obj =
                headers
                    .iter()
                    .zip(record.iter())
                    .fold(json!({}), |mut acc, (header, value)| {
                        let lossy_value = String::from_utf8_lossy(value);
                        acc[header] = json!(lossy_value.to_string());
                        acc
                    });
            chunk.push(json_obj);
            if chunk.len() as u64 == chunk_size {
                let insert = database.insert(chunk.to_owned()).await;
                if let Err(e) = insert {
                    context
                        .log_message(&format!("Error inserting chunk: {:?}", e), LogLevel::Error);
                }
                chunk = Vec::with_capacity(chunk_size as usize);
            }
        }

        if !chunk.is_empty() {
            let insert = database.insert(chunk.to_owned()).await;
            if let Err(e) = insert {
                context.log_message(&format!("Error inserting chunk: {:?}", e), LogLevel::Error);
            }
        }

        context.activate_exec_pin("exec_out").await?;
        Ok(())
    }
}
