use crate::{
    flow::execution::{context::ExecutionContext, Cacheable},
    state::FlowLikeStore,
    utils::{hash::hash_string_non_cryptographic, local_object_store::LocalObjectStore},
};
use anyhow::anyhow;
use object_store::path::Path;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::{path::PathBuf, sync::Arc};
use crate::flow::node::NodeLogic;
use tokio::sync::Mutex;

pub mod content;
pub mod dirs;
pub mod manipulation;
pub mod operations;
pub mod path_from_buf;

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct FlowPath {
    pub path: String,
    pub store_ref: String,
}

impl FlowPath {
    pub fn new(path: String, store_ref: String) -> Self {
        Self { path, store_ref }
    }

    pub async fn to_store(&self, context: &mut ExecutionContext) -> anyhow::Result<FlowLikeStore> {
        let store = context
            .get_cache(&self.store_ref)
            .await
            .ok_or(anyhow!("Failed to get Store from Cache"))?;
        let down_casted: &FlowLikeStore = store
            .downcast_ref()
            .ok_or(anyhow!("Failed to downcast Store"))?;
        let store = down_casted.clone();
        Ok(store)
    }

    pub async fn to_runtime(
        &self,
        context: &mut ExecutionContext,
    ) -> anyhow::Result<FlowPathRuntime> {
        let store = self.to_store(context).await?;
        let path = Path::from(self.path.clone());
        Ok(FlowPathRuntime {
            path,
            store: Arc::new(store),
            hash: self.store_ref.clone(),
        })
    }

    pub async fn from_pathbuf(
        path: PathBuf,
        context: &mut ExecutionContext,
    ) -> anyhow::Result<Self> {
        let mut object_path = Path::from("");
        let mut path = path;
        if path.is_file() {
            let file_name = path
                .file_name()
                .ok_or(anyhow!("Failed to get Filename"))?
                .to_str()
                .ok_or(anyhow!("Failed to convert Filename to String"))?;
            object_path = Path::from(file_name);
            path.pop();
        }

        let store = LocalObjectStore::new(path.clone())?;
        let store_hash = hash_string_non_cryptographic(&store.to_string()).to_string();
        let store = FlowLikeStore::Local(Arc::new(store));
        let cacheable_store: Arc<dyn Cacheable> = Arc::new(store);

        context.set_cache(&store_hash, cacheable_store).await;
        let string_object_path = object_path.as_ref();

        Ok(Self {
            path: string_object_path.to_string(),
            store_ref: store_hash,
        })
    }

    pub async fn from_upload_dir(context: &mut ExecutionContext) -> anyhow::Result<Self> {
        let exec_context = context
            .execution_cache
            .clone()
            .ok_or(anyhow!("Failed to get Execution Cache"))?;
        let dir = exec_context.get_upload_dir()?;
        let store_hash = String::from(format!("dirs__upload_{}", dir.as_ref()));

        if let Some(_) = context.get_cache(&store_hash).await {
            return Ok(Self {
                store_ref: store_hash,
                path: dir.as_ref().to_string(),
            });
        }

        let store = exec_context
            .stores
            .project_store
            .clone()
            .ok_or(anyhow!("Failed to get Project Store"))?;

        let cacheable_store: Arc<dyn Cacheable> = Arc::new(store.clone());
        context.set_cache(&store_hash, cacheable_store).await;

        Ok(Self {
            store_ref: store_hash,
            path: dir.as_ref().to_string(),
        })
    }

    pub async fn from_storage_dir(
        context: &mut ExecutionContext,
        node: bool,
    ) -> anyhow::Result<Self> {
        let exec_context = context
            .execution_cache
            .clone()
            .ok_or(anyhow!("Failed to get Execution Cache"))?;
        let dir = exec_context.get_storage(node)?;
        let store_hash = String::from(format!("dirs__storage_{}", dir.as_ref()));

        if let Some(_) = context.get_cache(&store_hash).await {
            return Ok(Self {
                store_ref: store_hash,
                path: dir.as_ref().to_string(),
            });
        }

        let store = exec_context
            .stores
            .project_store
            .clone()
            .ok_or(anyhow!("Failed to get Project Store"))?;

        let cacheable_store: Arc<dyn Cacheable> = Arc::new(store.clone());
        context.set_cache(&store_hash, cacheable_store).await;

        Ok(Self {
            store_ref: store_hash,
            path: dir.as_ref().to_string(),
        })
    }

    pub async fn from_cache_dir(
        context: &mut ExecutionContext,
        node: bool,
    ) -> anyhow::Result<Self> {
        let exec_context = context
            .execution_cache
            .clone()
            .ok_or(anyhow!("Failed to get Execution Cache"))?;
        let dir = exec_context.get_cache(node)?;
        let store_hash = String::from(format!("dirs__cache_{}", dir.as_ref()));

        if let Some(_) = context.get_cache(&store_hash).await {
            return Ok(Self {
                store_ref: store_hash,
                path: dir.as_ref().to_string(),
            });
        }

        let store = exec_context
            .stores
            .project_store
            .clone()
            .ok_or(anyhow!("Failed to get Project Store"))?;

        let cacheable_store: Arc<dyn Cacheable> = Arc::new(store.clone());
        context.set_cache(&store_hash, cacheable_store).await;

        Ok(Self {
            store_ref: store_hash,
            path: dir.as_ref().to_string(),
        })
    }

    pub async fn from_user_dir(context: &mut ExecutionContext, node: bool) -> anyhow::Result<Self> {
        let exec_context = context
            .execution_cache
            .clone()
            .ok_or(anyhow!("Failed to get Execution Cache"))?;
        let dir = exec_context.get_user_cache(node)?;
        let store_hash = String::from(format!("dirs__user_{}", dir.as_ref()));

        if let Some(_) = context.get_cache(&store_hash).await {
            return Ok(Self {
                store_ref: store_hash,
                path: dir.as_ref().to_string(),
            });
        }

        let store = exec_context
            .stores
            .user_store
            .clone()
            .ok_or(anyhow!("Failed to get Project Store"))?;

        let cacheable_store: Arc<dyn Cacheable> = Arc::new(store.clone());
        context.set_cache(&store_hash, cacheable_store).await;

        Ok(Self {
            store_ref: store_hash,
            path: dir.as_ref().to_string(),
        })
    }
}

#[derive(Clone)]
pub struct FlowPathRuntime {
    pub path: Path,
    pub store: Arc<FlowLikeStore>,
    pub hash: String,
}

impl FlowPathRuntime {
    pub async fn serialize(&self) -> FlowPath {
        FlowPath {
            store_ref: self.hash.clone(),
            path: self.path.as_ref().to_string(),
        }
    }
}



pub async fn register_functions() -> Vec<Arc<Mutex<dyn NodeLogic>>> {
    let mut nodes = vec![];

    nodes.extend(content::register_functions().await);
    nodes.extend(dirs::register_functions().await);
    nodes.extend(manipulation::register_functions().await);
    nodes.extend(operations::register_functions().await);
    nodes.push(Arc::new(Mutex::new(path_from_buf::PathBufToPathNode::new())));

    nodes
}


#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    #[tokio::test]
    async fn test_empty_path_serialization() {
        let path = Path::from("");
        let serialized = path.as_ref();
        assert_eq!(serialized, "");
    }

    #[tokio::test]
    async fn test_complex_path_serialization() {
        let mut input_buf = PathBuf::from("test/test2/test3.txt");
        let file_name = input_buf
            .file_name()
            .ok_or(anyhow!("Failed to get Filename"))
            .unwrap()
            .to_str()
            .ok_or(anyhow!("Failed to convert Filename to String"))
            .unwrap();
        let path = Path::from(file_name);
        input_buf.pop();

        let serialized = path.as_ref();

        assert_eq!(serialized, "test3.txt");
        assert_eq!(input_buf.to_str().unwrap(), "test/test2");
    }
}
