use flow_like_storage::files::store::FlowLikeStore;
use flow_like_storage::lancedb::connection::ConnectBuilder;
use flow_like_storage::object_store::path::Path;
use flow_like_types::Ok;
use flow_like_types::sync::{DashMap, Mutex, RwLock};
#[cfg(feature = "flow-runtime")]
use flow_like_types::tokio_util::sync::CancellationToken;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::{Arc, Weak};
#[cfg(feature = "flow-runtime")]
use std::time::Instant;

use crate::flow::event::Event;
#[cfg(feature = "flow-runtime")]
use crate::flow::execution::{LogMeta, log::LogMessage};

#[cfg(feature = "flow-runtime")]
use crate::flow::board::Board;
use crate::flow::node::Node;
#[cfg(feature = "flow-runtime")]
use crate::flow::node::NodeLogic;

use crate::models::embedding_factory::EmbeddingFactory;
#[cfg(feature = "model")]
use crate::models::llm::ModelFactory;
#[cfg(feature = "bit")]
use crate::utils::download_manager::DownloadManager;
use crate::utils::http::HTTPClient;
#[cfg(feature = "model")]
use flow_like_model_provider::provider::ModelProviderConfiguration;

#[derive(Clone, Default)]
pub struct FlowLikeStores {
    pub bits_store: Option<FlowLikeStore>,
    pub user_store: Option<FlowLikeStore>,
    pub app_storage_store: Option<FlowLikeStore>,
    pub app_meta_store: Option<FlowLikeStore>,
    pub temporary_store: Option<FlowLikeStore>,
    pub log_store: Option<FlowLikeStore>,
}

#[derive(Clone, Default)]
pub struct FlowLikeCallbacks {
    pub build_project_database: Option<Arc<dyn (Fn(Path) -> ConnectBuilder) + Send + Sync>>,
    pub build_logs_database: Option<Arc<dyn (Fn(Path) -> ConnectBuilder) + Send + Sync>>,
}

#[derive(Clone, Default)]
pub struct FlowLikeConfig {
    pub stores: FlowLikeStores,
    pub callbacks: FlowLikeCallbacks,
}

impl FlowLikeConfig {
    pub fn new() -> Self {
        FlowLikeConfig {
            callbacks: FlowLikeCallbacks::default(),
            stores: FlowLikeStores::default(),
        }
    }

    pub fn with_default_store(store: FlowLikeStore) -> Self {
        FlowLikeConfig {
            callbacks: FlowLikeCallbacks::default(),
            stores: FlowLikeStores {
                app_storage_store: Some(store.clone()),
                app_meta_store: Some(store.clone()),
                bits_store: Some(store.clone()),
                user_store: Some(store.clone()),
                temporary_store: Some(store.clone()),
                log_store: Some(store),
            },
        }
    }

    pub fn register_app_storage_store(&mut self, store: FlowLikeStore) {
        self.stores.app_storage_store = Some(store);
    }

    pub fn register_app_meta_store(&mut self, store: FlowLikeStore) {
        self.stores.app_meta_store = Some(store);
    }

    pub fn register_user_store(&mut self, store: FlowLikeStore) {
        self.stores.user_store = Some(store);
    }

    pub fn register_bits_store(&mut self, store: FlowLikeStore) {
        self.stores.bits_store = Some(store);
    }

    pub fn register_temporary_store(&mut self, store: FlowLikeStore) {
        self.stores.temporary_store = Some(store);
    }

    pub fn register_log_store(&mut self, store: FlowLikeStore) {
        self.stores.log_store = Some(store);
    }

    pub fn register_build_project_database(
        &mut self,
        callback: Arc<dyn (Fn(Path) -> ConnectBuilder) + Send + Sync>,
    ) {
        self.callbacks.build_project_database = Some(callback);
    }

    pub fn register_build_logs_database(
        &mut self,
        callback: Arc<dyn (Fn(Path) -> ConnectBuilder) + Send + Sync>,
    ) {
        self.callbacks.build_logs_database = Some(callback);
    }
}

#[cfg(feature = "flow-runtime")]
#[derive(Default, Clone)]
pub struct FlowNodeRegistryInner {
    pub registry: HashMap<String, (Node, Arc<dyn NodeLogic>)>,
}

impl FlowNodeRegistryInner {
    pub fn new(size: usize) -> Self {
        FlowNodeRegistryInner {
            registry: HashMap::with_capacity(size),
        }
    }

    pub fn insert(&mut self, node: Node, logic: Arc<dyn NodeLogic>) {
        self.registry.insert(node.name.clone(), (node, logic));
    }

    pub fn get_nodes(&self) -> Vec<Node> {
        self.registry.values().map(|node| node.0.clone()).collect()
    }

    pub async fn prepare(state: &FlowLikeState, nodes: &Arc<Vec<Arc<dyn NodeLogic>>>) -> Self {
        let mut registry = FlowNodeRegistryInner {
            registry: HashMap::with_capacity(nodes.len()),
        };

        for logic in nodes.iter() {
            let node = logic.get_node(state).await;
            registry.insert(node, logic.clone());
        }

        registry
    }

    #[inline]
    pub fn get_node(&self, node_id: &str) -> flow_like_types::Result<Node> {
        let node = self.registry.get(node_id);
        match node {
            Some(node) => Ok(node.0.clone()),
            None => Err(flow_like_types::anyhow!("Node not found - Get Node")),
        }
    }

    #[inline]
    pub fn instantiate(&self, node: &Node) -> flow_like_types::Result<Arc<dyn NodeLogic>> {
        let node = self.registry.get(&node.name);
        match node {
            Some(node) => Ok(node.1.clone()),
            None => Err(flow_like_types::anyhow!("Node not found - Instancing")),
        }
    }
}

#[cfg(feature = "flow-runtime")]
pub struct FlowNodeRegistry {
    pub node_registry: Arc<FlowNodeRegistryInner>,
    pub parent: Option<Weak<Mutex<FlowLikeState>>>,
}

#[cfg(feature = "flow-runtime")]
impl Default for FlowNodeRegistry {
    fn default() -> Self {
        Self::new()
    }
}

impl FlowNodeRegistry {
    pub fn new() -> Self {
        FlowNodeRegistry {
            node_registry: Arc::new(FlowNodeRegistryInner::new(0)),
            parent: None,
        }
    }

    pub fn initialize(&mut self, parent: Weak<Mutex<FlowLikeState>>) {
        self.parent = Some(parent);
    }

    pub fn get_nodes(&self) -> flow_like_types::Result<Vec<Node>> {
        let nodes = self.node_registry.get_nodes();
        Ok(nodes)
    }

    pub async fn push_node(&mut self, logic: Arc<dyn NodeLogic>) -> flow_like_types::Result<()> {
        let state = self
            .parent
            .as_ref()
            .and_then(|weak| weak.upgrade())
            .ok_or(flow_like_types::anyhow!("Parent not found"))?;
        let guard = state.lock().await;
        let mut registry = FlowNodeRegistryInner {
            registry: self.node_registry.registry.clone(),
        };
        let node = logic.get_node(&guard).await;
        registry.insert(node, logic);
        self.node_registry = Arc::new(registry);
        Ok(())
    }

    pub async fn push_nodes(
        &mut self,
        nodes: Vec<Arc<dyn NodeLogic>>,
    ) -> flow_like_types::Result<()> {
        let state = self
            .parent
            .as_ref()
            .and_then(|weak| weak.upgrade())
            .ok_or(flow_like_types::anyhow!("Parent not found"))?;
        let guard = state.lock().await;

        let mut registry = FlowNodeRegistryInner {
            registry: self.node_registry.registry.clone(),
        };

        for logic in nodes {
            let node = logic.get_node(&guard).await;
            registry.insert(node, logic);
        }
        self.node_registry = Arc::new(registry);
        Ok(())
    }

    pub fn get_node(&self, node_id: &str) -> flow_like_types::Result<Node> {
        let node = self.node_registry.get_node(node_id)?;
        Ok(node)
    }

    pub fn instantiate(&self, node: &Node) -> flow_like_types::Result<Arc<dyn NodeLogic>> {
        let node = self.node_registry.instantiate(node)?;
        Ok(node)
    }
}

#[derive(Clone)]
pub struct RunData {
    pub start_time: Instant,
    pub board_id: Arc<str>,
    pub node_id: Arc<str>,
    pub event_id: Option<Arc<str>>,
    pub cancellation_token: CancellationToken,
}

impl RunData {
    pub fn new(
        board_id: &str,
        node_id: &str,
        event_id: Option<String>,
        cancellation_token: CancellationToken,
    ) -> Self {
        RunData {
            start_time: Instant::now(),
            board_id: Arc::from(board_id),
            node_id: Arc::from(node_id),
            event_id: event_id.map(|s| Arc::from(s.as_str())),
            cancellation_token,
        }
    }

    pub fn is_cancelled(&self) -> bool {
        self.cancellation_token.is_cancelled()
    }

    pub fn cancel(&self) {
        self.cancellation_token.cancel();
    }

    pub fn elapsed(&self) -> std::time::Duration {
        self.start_time.elapsed()
    }

    pub fn from_event(event: &Event, cancellation_token: CancellationToken) -> Self {
        RunData {
            start_time: Instant::now(),
            board_id: Arc::from(event.board_id.as_str()),
            node_id: Arc::from(event.node_id.as_str()),
            event_id: Some(Arc::from(event.id.as_str())),
            cancellation_token,
        }
    }
}

// TODO: implement dashmap
#[derive(Clone)]
pub struct FlowLikeState {
    pub config: Arc<RwLock<FlowLikeConfig>>,
    pub http_client: Arc<HTTPClient>,

    #[cfg(feature = "bit")]
    pub download_manager: Arc<Mutex<DownloadManager>>,

    #[cfg(feature = "model")]
    pub model_provider_config: Arc<ModelProviderConfiguration>,

    #[cfg(feature = "model")]
    pub model_factory: Arc<Mutex<ModelFactory>>,
    #[cfg(feature = "model")]
    pub embedding_factory: Arc<Mutex<EmbeddingFactory>>,

    #[cfg(feature = "flow-runtime")]
    pub node_registry: Arc<RwLock<FlowNodeRegistry>>,
    #[cfg(feature = "flow-runtime")]
    pub board_registry: Arc<DashMap<String, Arc<Mutex<Board>>>>, // TODO: should board be wrapped in RWLock or Mutex?
    #[cfg(feature = "flow-runtime")]
    pub board_run_registry: Arc<DashMap<String, Arc<RunData>>>,
}

impl FlowLikeState {
    pub fn new(config: FlowLikeConfig, client: HTTPClient) -> Self {
        FlowLikeState {
            config: Arc::new(RwLock::new(config)),
            http_client: Arc::new(client),

            #[cfg(feature = "bit")]
            download_manager: Arc::new(Mutex::new(DownloadManager::new())),

            #[cfg(feature = "model")]
            model_provider_config: Arc::new(ModelProviderConfiguration::default()),
            #[cfg(feature = "model")]
            model_factory: Arc::new(Mutex::new(ModelFactory::new())),

            #[cfg(feature = "model")]
            embedding_factory: Arc::new(Mutex::new(EmbeddingFactory::new())),

            #[cfg(feature = "flow-runtime")]
            node_registry: Arc::new(RwLock::new(FlowNodeRegistry::new())),
            #[cfg(feature = "flow-runtime")]
            board_registry: Arc::new(DashMap::new()),
            #[cfg(feature = "flow-runtime")]
            board_run_registry: Arc::new(DashMap::new()),
        }
    }

    #[cfg(feature = "bit")]
    pub fn download_manager(&self) -> Arc<Mutex<DownloadManager>> {
        self.download_manager.clone()
    }

    #[cfg(feature = "model")]
    pub fn model_factory(&self) -> Arc<Mutex<ModelFactory>> {
        self.model_factory.clone()
    }

    #[cfg(feature = "flow-runtime")]
    pub fn node_registry(&self) -> Arc<RwLock<FlowNodeRegistry>> {
        self.node_registry.clone()
    }

    #[cfg(feature = "flow-runtime")]
    pub fn board_registry(&self) -> Arc<DashMap<String, Arc<Mutex<Board>>>> {
        self.board_registry.clone()
    }

    #[cfg(feature = "flow-runtime")]
    pub fn get_board(
        &self,
        board_id: &str,
        version: Option<(u32, u32, u32)>,
    ) -> flow_like_types::Result<Arc<Mutex<Board>>> {
        let key = if let Some(version) = version {
            format!("{}-{}-{}-{}", board_id, version.0, version.1, version.2)
        } else {
            board_id.to_string()
        };

        let board = self.board_registry.try_get(&key);

        match board.try_unwrap() {
            Some(board) => Ok(board.clone()),
            None => Err(flow_like_types::anyhow!(
                "Board not found or could not be locked"
            )),
        }
    }

    #[cfg(feature = "flow-runtime")]
    pub fn get_template(
        &self,
        template_id: &str,
        version: Option<(u32, u32, u32)>,
    ) -> flow_like_types::Result<Arc<Mutex<Board>>> {
        let key = if let Some(version) = version {
            format!("{}-{}-{}-{}", template_id, version.0, version.1, version.2)
        } else {
            template_id.to_string()
        };

        let board = self.board_registry.try_get(&key);

        match board.try_unwrap() {
            Some(board) => Ok(board.clone()),
            None => Err(flow_like_types::anyhow!(
                "Board not found or could not be locked"
            )),
        }
    }

    #[cfg(feature = "flow-runtime")]
    pub fn remove_board(
        &self,
        board_id: &str,
    ) -> flow_like_types::Result<Option<Arc<Mutex<Board>>>> {
        let removed = self.board_registry.remove(board_id);

        match removed {
            Some((_id, board)) => Ok(Some(board)),
            None => Ok(None),
        }
    }

    #[cfg(feature = "flow-runtime")]
    pub fn register_board(
        &self,
        board_id: &str,
        board: Arc<Mutex<Board>>,
        version: Option<(u32, u32, u32)>,
    ) -> flow_like_types::Result<()> {
        let key = if let Some(version) = version {
            format!("{}-{}-{}-{}", board_id, version.0, version.1, version.2)
        } else {
            board_id.to_string()
        };
        self.board_registry.insert(key, board.clone());
        Ok(())
    }

    #[cfg(feature = "flow-runtime")]
    pub fn board_run_registry(&self) -> Arc<DashMap<String, Arc<RunData>>> {
        self.board_run_registry.clone()
    }

    #[cfg(feature = "flow-runtime")]
    pub fn list_runs(&self) -> flow_like_types::Result<Vec<(String, Arc<RunData>)>> {
        let runs = self
            .board_run_registry
            .iter()
            .map(|entry| (entry.key().clone(), entry.value().clone()))
            .collect::<Vec<_>>();
        Ok(runs)
    }

    #[cfg(feature = "flow-runtime")]
    pub fn register_run(&self, run_id: &str, run: RunData) {
        self.board_run_registry
            .insert(run_id.to_string(), Arc::new(run));
    }

    #[cfg(feature = "flow-runtime")]
    pub fn remove_and_cancel_run(&self, run_id: &str) -> flow_like_types::Result<()> {
        let removed = self.board_run_registry.remove(run_id);
        if let Some((_id, run)) = removed {
            if !run.is_cancelled() {
                run.cancel();
            }
        }

        Ok(())
    }

    #[cfg(feature = "flow-runtime")]
    pub fn get_run(&self, run_id: &str) -> flow_like_types::Result<Arc<RunData>> {
        let run = self.board_run_registry.try_get(run_id);

        match run.try_unwrap() {
            Some(run) => Ok(run.clone()),
            None => Err(flow_like_types::anyhow!(
                "Run not found or could not be locked"
            )),
        }
    }

    #[cfg(feature = "flow-runtime")]
    pub async fn query_run(
        &self,
        meta: &LogMeta,
        query: &str,
        limit: Option<usize>,
        offset: Option<usize>,
    ) -> flow_like_types::Result<Vec<LogMessage>> {
        use flow_like_storage::{
            lancedb::query::{ExecutableQuery, QueryBase},
            serde_arrow,
        };
        use flow_like_types::anyhow;
        use futures::TryStreamExt;

        use crate::flow::execution::log::StoredLogMessage;

        let limit = limit.unwrap_or(100);
        let offset = offset.unwrap_or(0);

        let db = {
            let guard = self.config.read().await;

            guard.callbacks.build_logs_database.clone()
        };

        let db_fn = db
            .as_ref()
            .ok_or_else(|| anyhow!("No log database configured"))?;
        let base_path = Path::from("runs")
            .child(meta.app_id.clone())
            .child(meta.board_id.clone());
        let db = db_fn(base_path.clone()).execute().await?;

        let db = db.open_table(meta.run_id.clone()).execute().await?;
        let mut q = db.query();

        if !query.is_empty() {
            q = q.only_if(query);
        }

        let results = q.limit(limit).offset(offset).execute().await?;
        let results = results.try_collect::<Vec<_>>().await?;

        let mut log_messages = Vec::with_capacity(results.len() * 10);
        for result in results {
            let result = serde_arrow::from_record_batch::<Vec<StoredLogMessage>>(&result)
                .unwrap_or_default();
            let result = result
                .into_iter()
                .map(|log| {
                    let log: LogMessage = log.into();
                    log
                })
                .collect::<Vec<_>>();
            log_messages.extend(result);
        }

        Ok(log_messages)
    }

    #[inline]
    pub async fn stores(state: &Arc<Mutex<FlowLikeState>>) -> FlowLikeStores {
        state.lock().await.config.read().await.stores.clone()
    }

    #[inline]
    pub async fn project_storage_store(
        state: &Arc<Mutex<FlowLikeState>>,
    ) -> flow_like_types::Result<FlowLikeStore> {
        state
            .lock()
            .await
            .config
            .read()
            .await
            .stores
            .app_storage_store
            .clone()
            .ok_or(flow_like_types::anyhow!("No project store"))
    }

    #[inline]
    pub async fn project_meta_store(
        state: &Arc<Mutex<FlowLikeState>>,
    ) -> flow_like_types::Result<FlowLikeStore> {
        state
            .lock()
            .await
            .config
            .read()
            .await
            .stores
            .app_meta_store
            .clone()
            .ok_or(flow_like_types::anyhow!("No project store"))
    }

    #[inline]
    pub async fn bit_store(
        state: &Arc<Mutex<FlowLikeState>>,
    ) -> flow_like_types::Result<FlowLikeStore> {
        state
            .lock()
            .await
            .config
            .read()
            .await
            .stores
            .bits_store
            .clone()
            .ok_or(flow_like_types::anyhow!("No bit store"))
    }

    #[inline]
    pub async fn user_store(
        state: &Arc<Mutex<FlowLikeState>>,
    ) -> flow_like_types::Result<FlowLikeStore> {
        state
            .lock()
            .await
            .config
            .read()
            .await
            .stores
            .user_store
            .clone()
            .ok_or(flow_like_types::anyhow!("No user store"))
    }
}

#[derive(Serialize, Deserialize, Debug)]
#[serde(rename_all = "lowercase")]
pub enum ToastLevel {
    Success,
    Info,
    Warning,
    Error,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct ToastEvent {
    pub message: String,
    pub level: ToastLevel,
}

impl ToastEvent {
    pub fn new(message: &str, level: ToastLevel) -> Self {
        ToastEvent {
            message: message.to_string(),
            level,
        }
    }
}

impl Default for ToastEvent {
    fn default() -> Self {
        ToastEvent {
            message: "".to_string(),
            level: ToastLevel::Info,
        }
    }
}

#[cfg(test)]
mod tests {
    use flow_like_storage::object_store::{ObjectStore, PutPayload};
    use flow_like_types::{Bytes, Cacheable, tokio};

    use super::*;
    use std::path::PathBuf;

    #[test]
    fn object_store_path_serialization() {
        let path = Path::from("test").child("path").child("one");
        let event = PathBuf::from("random").join(path.to_string());
        assert_eq!(path.to_string(), "test/path/one".to_string());
        assert_eq!(event.to_str().unwrap(), "random/test/path/one");
    }

    #[tokio::test]
    async fn test_object_store_any_cast() {
        let memory_store = flow_like_storage::object_store::memory::InMemory::new();
        let test_string = b"Hi, I am Testing";
        let test_path = Path::from("test");
        memory_store
            .put(&test_path, PutPayload::from_static(test_string))
            .await
            .unwrap();
        let store: FlowLikeStore = FlowLikeStore::Other(Arc::new(memory_store));
        let store: Arc<dyn Cacheable> = Arc::new(store.clone());
        let down_casted: &FlowLikeStore = store.downcast_ref().unwrap();
        let read_bytes: Bytes = down_casted
            .as_generic()
            .get(&test_path)
            .await
            .unwrap()
            .bytes()
            .await
            .unwrap();
        let test_bytes = Bytes::from_static(test_string);
        assert_eq!(read_bytes, test_bytes);
    }
}
