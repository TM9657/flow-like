use std::{collections::{HashMap, HashSet}, sync::Arc, time::Duration};
use flow_like::{flow::{board::Board, execution::{context::ExecutionContext, internal_node::InternalNode, log::LogMessage, LogLevel}, node::{Node, NodeLogic}, pin::{PinOptions, PinType, ValueType}, variable::{Variable, VariableType}}, state::FlowLikeState};
use flow_like_types::{async_trait, json::{json, Deserialize, Serialize}, reqwest, sync::{DashMap, Mutex}, Bytes, Error, JsonSchema, Value};
use nalgebra::DVector;
use regex::Regex;
use flow_like_storage::{databases::vector::{VectorStore, lancedb::LanceDBVectorStore}, object_store::PutPayload, Path};
use futures::StreamExt;
use crate::{storage::path::FlowPath, web::api::{HttpBody, HttpRequest, HttpResponse, Method}};

use super::NodeDBConnection;

#[derive(Default)]
pub struct DeleteLocalDatabaseNode {}

impl DeleteLocalDatabaseNode {
    pub fn new() -> Self {
        DeleteLocalDatabaseNode {}
    }
}

#[async_trait]
impl NodeLogic for DeleteLocalDatabaseNode {
    async fn get_node(&self, _app_state: &FlowLikeState) -> Node {
        let mut node = Node::new(
            "filter_delete_local_db",
            "Delete",
            "Filter Database",
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

        node.add_input_pin(
            "filter",
            "SQL Filter",
            "Optional SQL Filter",
            VariableType::String,
        )
        .set_default_value(Some(json!("")));

        node.add_output_pin(
            "exec_out",
            "Created Database",
            "Done Creating Database",
            VariableType::Execution,
        );

        return node;
    }

    async fn run(&self, context: &mut ExecutionContext) ->flow_like_types::Result<()> {
        let database: NodeDBConnection = context.evaluate_pin("database").await?;
        let database = database
            .load(context, &database.cache_key)
            .await?
            .db
            .clone();
        let database = database.read().await;
        let filter: String = context.evaluate_pin("filter").await?;
        database.delete(&filter).await?;
        context.activate_exec_pin("exec_out").await?;
        Ok(())
    }
}
