use flow_like::flow::{
    execution::context::ExecutionContext,
    node::{Node, NodeLogic},
    variable::VariableType,
};
use flow_like_storage::databases::vector::lancedb::LanceDBVectorStore;
use flow_like_types::{
    Cacheable, JsonSchema, Value, async_trait,
    json::{Deserialize, Serialize},
    sync::RwLock,
};
use std::sync::Arc;

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
pub mod schema;
pub mod upsert;
pub mod vector_search;

#[derive(Default, Serialize, Deserialize, JsonSchema, Clone)]
pub struct NodeDBConnection {
    pub cache_key: String,
}

#[derive(Clone)]
pub struct CachedDB {
    pub db: Arc<RwLock<LanceDBVectorStore>>,
}

impl Cacheable for CachedDB {
    fn as_any(&self) -> &dyn std::any::Any {
        self
    }

    fn as_any_mut(&mut self) -> &mut dyn std::any::Any {
        self
    }
}

impl NodeDBConnection {
    pub async fn load(&self, context: &mut ExecutionContext) -> flow_like_types::Result<CachedDB> {
        let cached = context
            .cache
            .read()
            .await
            .get(self.cache_key.as_str())
            .cloned()
            .ok_or(flow_like_types::anyhow!("No cache found"))?;
        let db = cached
            .as_any()
            .downcast_ref::<CachedDB>()
            .ok_or(flow_like_types::anyhow!("Could not downcast"))?;
        Ok(db.clone())
    }
}

#[crate::register_node]
#[derive(Default)]
pub struct CreateLocalDatabaseNode {}

impl CreateLocalDatabaseNode {
    pub fn new() -> Self {
        CreateLocalDatabaseNode {}
    }
}

#[async_trait]
impl NodeLogic for CreateLocalDatabaseNode {
    fn get_node(&self) -> Node {
        let mut node = Node::new(
            "open_local_db",
            "Open Database",
            "Open a local database",
            "Data/Database",
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

        node
    }

    async fn run(&self, context: &mut ExecutionContext) -> flow_like_types::Result<()> {
        context.deactivate_exec_pin("exec_out").await?;

        let table: String = context.evaluate_pin("name").await?;
        let cache_key = format!("db_{}", table);
        let cache_set = context.cache.read().await.contains_key(&cache_key);
        if !cache_set {
            let context_cache = context
                .execution_cache
                .clone()
                .ok_or(flow_like_types::anyhow!("No execution cache found"))?;
            let app_id = context_cache.app_id.clone();
            let board_dir = context_cache.get_storage(false)?;
            let board_dir = board_dir.child("db");

            let db = if let Some(credentials) = &context.credentials {
                credentials.to_db(&app_id).await?
            } else {
                context
                    .app_state
                    .config
                    .read()
                    .await
                    .callbacks
                    .build_project_database
                    .clone()
                    .ok_or(flow_like_types::anyhow!("No database builder found"))?(
                    board_dir
                )
            };

            let db = db.execute().await?;
            let intermediate = LanceDBVectorStore::from_connection(db, table).await;
            let intermediate = CachedDB {
                db: Arc::new(RwLock::new(intermediate)),
            };
            let cacheable: Arc<dyn Cacheable> = Arc::new(intermediate.clone());
            context
                .cache
                .write()
                .await
                .insert(cache_key.clone(), cacheable);
        }

        let db = NodeDBConnection { cache_key };

        let db: Value = flow_like_types::json::to_value(&db)?;

        context.set_pin_value("database", db).await?;
        context.activate_exec_pin("exec_out").await?;
        Ok(())
    }
}
