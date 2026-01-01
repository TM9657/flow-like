use flow_like::flow::execution::context::ExecutionContext;
use flow_like_storage::databases::vector::lancedb::LanceDBVectorStore;
use flow_like_types::{
    Cacheable, JsonSchema,
    json::{Deserialize, Serialize},
    sync::RwLock,
};
use std::sync::Arc;

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
