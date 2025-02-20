use std::sync::Arc;

use crate::{
    flow::{
        board::Board,
        execution::{context::ExecutionContext, Cacheable},
        node::{Node, NodeLogic},
        pin::PinOptions,
        variable::VariableType,
    },
    state::FlowLikeState,
    vault::vector::lancedb::LanceDBVectorStore,
};
use async_trait::async_trait;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

pub mod count;
pub mod delete;
pub mod filter;
pub mod fts_search;
pub mod hybrid_search;
pub mod index;
pub mod insert;
pub mod list;
pub mod optimize;
pub mod purge;
pub mod upsert;
pub mod vector_search;

#[derive(Default, Serialize, Deserialize, JsonSchema, Clone)]
pub struct NodeDBConnection {
    pub cache_key: String,
}

impl NodeDBConnection {
    pub async fn load(
        &self,
        context: &mut ExecutionContext,
        cache_key: &str,
    ) -> anyhow::Result<LanceDBVectorStore> {
        let cached = context
            .cache
            .read()
            .await
            .get(cache_key)
            .cloned()
            .ok_or(anyhow::anyhow!("No cache found"))?;
        let db = cached
            .as_any()
            .downcast_ref::<LanceDBVectorStore>()
            .ok_or(anyhow::anyhow!("Could not downcast"))?;
        Ok(db.clone())
    }
}

#[derive(Default)]
pub struct CreateLocalDatabaseNode {}

impl CreateLocalDatabaseNode {
    pub fn new() -> Self {
        CreateLocalDatabaseNode {}
    }
}

#[async_trait]
impl NodeLogic for CreateLocalDatabaseNode {
    async fn get_node(&self, _app_state: &FlowLikeState) -> Node {
        let mut node = Node::new(
            "open_local_db",
            "Open Local Database",
            "Open a local database",
            "Database/Local",
        );
        node.add_icon("/flow/icons/database.svg");

        node.add_input_pin("exec_in", "Input", "", VariableType::Execution);
        node.add_input_pin(
            "name",
            "Table Name",
            "Name of the Table",
            VariableType::String,
        );

        node.add_output_pin(
            "exec_out",
            "Created Database",
            "Done Creating Database",
            VariableType::Execution,
        );

        node.add_output_pin(
            "database",
            "Database",
            "Database Connection Reference",
            VariableType::Struct,
        )
        .set_schema::<NodeDBConnection>();

        return node;
    }

    async fn run(&mut self, context: &mut ExecutionContext) -> anyhow::Result<()> {
        let table: String = context.evaluate_pin("name").await?;
        let cache_key = format!("db_{}", table);
        let cache_set = context.cache.read().await.contains_key(&cache_key);
        if !cache_set {
            let board_dir = context
                .execution_cache
                .clone()
                .ok_or(anyhow::anyhow!("No execution cache found"))?
                .get_cache(false)?;
            let board_dir = board_dir.child("db");
            let db =
                context
                    .app_state
                    .lock()
                    .await
                    .config
                    .read()
                    .await
                    .callbacks
                    .build_project_database
                    .clone()
                    .ok_or(anyhow::anyhow!("No database builder found"))?(board_dir);
            let db = db.execute().await?;
            let intermediate = LanceDBVectorStore::from_connection(db, table).await;
            let cacheable: Arc<dyn Cacheable> = Arc::new(intermediate.clone());
            context
                .cache
                .write()
                .await
                .insert(cache_key.clone(), cacheable);
        }

        let db = NodeDBConnection {
            cache_key: cache_key,
        };

        let db: serde_json::Value = serde_json::to_value(&db)?;

        context.set_pin_value("database", db).await?;
        context.activate_exec_pin("exec_out").await?;
        Ok(())
    }
}
