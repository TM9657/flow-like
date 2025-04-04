use anyhow::Ok;
use dashmap::DashMap;
use flow_like_storage::files::store::FlowLikeStore;
use flow_like_storage::lancedb::connection::ConnectBuilder;
use flow_like_storage::object_store::ObjectStore;
use flow_like_storage::object_store::path::Path;
use futures::StreamExt;
use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashMap;
use std::sync::Arc;
use std::time::SystemTime;
use tokio::sync::{Mutex, RwLock, mpsc};

#[cfg(feature = "flow-runtime")]
use crate::flow::board::Board;
#[cfg(feature = "flow-runtime")]
use crate::flow::execution::InternalRun;
use crate::flow::node::Node;
#[cfg(feature = "flow-runtime")]
use crate::flow::node::NodeLogic;

use crate::models::embedding_factory::EmbeddingFactory;
#[cfg(feature = "model")]
use crate::models::llm::ModelFactory;

#[cfg(feature = "bit")]
use crate::utils::download_manager::DownloadManager;
use crate::utils::http::HTTPClient;

#[derive(Debug, Serialize, Deserialize)]
pub struct FlowLikeEvent {
    pub event_id: String,
    pub payload: Value,
    pub timestamp: SystemTime,
}

impl FlowLikeEvent {
    pub fn new<T>(event_id: &str, payload: T) -> Self
    where
        T: Serialize + DeserializeOwned,
    {
        FlowLikeEvent {
            event_id: event_id.to_string(),
            payload: serde_json::to_value(payload).unwrap(),
            timestamp: SystemTime::now(),
        }
    }
}

#[derive(Clone, Default)]
pub struct FlowLikeStores {
    pub bits_store: Option<FlowLikeStore>,
    pub user_store: Option<FlowLikeStore>,
    pub project_store: Option<FlowLikeStore>,
}

#[derive(Clone, Default)]
pub struct FlowLikeCallbacks {
    pub build_project_database: Option<Arc<dyn (Fn(Path) -> ConnectBuilder) + Send + Sync>>,
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

    pub fn register_project_store(&mut self, store: FlowLikeStore) {
        self.stores.project_store = Some(store);
    }

    pub fn register_user_store(&mut self, store: FlowLikeStore) {
        self.stores.user_store = Some(store);
    }

    pub fn register_bits_store(&mut self, store: FlowLikeStore) {
        self.stores.bits_store = Some(store);
    }

    pub fn register_build_project_database(
        &mut self,
        callback: Arc<dyn (Fn(Path) -> ConnectBuilder) + Send + Sync>,
    ) {
        self.callbacks.build_project_database = Some(callback);
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

    #[inline]
    pub fn get_node(&self, node_id: &str) -> anyhow::Result<Node> {
        let node = self.registry.get(node_id);
        match node {
            Some(node) => Ok(node.0.clone()),
            None => Err(anyhow::anyhow!("Node not found")),
        }
    }

    #[inline]
    pub fn instantiate(&self, node: &Node) -> anyhow::Result<Arc<dyn NodeLogic>> {
        let node = self.registry.get(&node.name);
        match node {
            Some(node) => Ok(node.1.clone()),
            None => Err(anyhow::anyhow!("Node not found")),
        }
    }
}

#[cfg(feature = "flow-runtime")]
pub struct FlowNodeRegistry {
    pub node_registry: Arc<FlowNodeRegistryInner>,
    pub initialized: bool,
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
            initialized: false,
        }
    }

    pub fn get_nodes(&self) -> anyhow::Result<Vec<Node>> {
        if !self.initialized {
            return Err(anyhow::anyhow!("Registry not initialized"));
        }

        let nodes = self.node_registry.get_nodes();

        Ok(nodes)
    }

    pub fn initialize(&mut self, nodes: Vec<(Node, Arc<dyn NodeLogic>)>) {
        let mut registry = FlowNodeRegistryInner::new(nodes.len());
        for (node, logic) in nodes {
            registry.insert(node, logic);
        }

        self.node_registry = Arc::new(registry);
        self.initialized = true;
    }

    pub fn push_node(&mut self, node: Node, logic: Arc<dyn NodeLogic>) -> anyhow::Result<()> {
        if !self.initialized {
            return Err(anyhow::anyhow!("Registry not initialized"));
        }
        let mut registry = FlowNodeRegistryInner {
            registry: self.node_registry.registry.clone(),
        };
        registry.insert(node, logic);
        self.node_registry = Arc::new(registry);
        Ok(())
    }

    pub fn push_nodes(&mut self, nodes: Vec<(Node, Arc<dyn NodeLogic>)>) -> anyhow::Result<()> {
        if !self.initialized {
            return Err(anyhow::anyhow!("Registry not initialized"));
        }
        let mut registry = FlowNodeRegistryInner {
            registry: self.node_registry.registry.clone(),
        };

        for (node, logic) in nodes {
            registry.insert(node, logic);
        }
        self.node_registry = Arc::new(registry);
        Ok(())
    }

    pub fn get_node(&self, node_id: &str) -> anyhow::Result<Node> {
        if !self.initialized {
            return Err(anyhow::anyhow!("Registry not initialized"));
        }

        let node = self.node_registry.get_node(node_id)?;
        Ok(node)
    }

    pub async fn instantiate(&self, node: &Node) -> anyhow::Result<Arc<dyn NodeLogic>> {
        if !self.initialized {
            return Err(anyhow::anyhow!("Registry not initialized"));
        }

        let node = self.node_registry.instantiate(node)?;
        Ok(node)
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
    pub model_factory: Arc<Mutex<ModelFactory>>,

    #[cfg(feature = "model")]
    pub embedding_factory: Arc<Mutex<EmbeddingFactory>>,

    #[cfg(feature = "flow-runtime")]
    pub node_registry: Arc<RwLock<FlowNodeRegistry>>,
    #[cfg(feature = "flow-runtime")]
    pub board_registry: Arc<DashMap<String, Arc<Mutex<Board>>>>, // TODO: should board be wrapped in RWLock or Mutex?
    #[cfg(feature = "flow-runtime")]
    pub board_run_registry: Arc<DashMap<String, Arc<Mutex<InternalRun>>>>,

    pub event_sender: Arc<Mutex<mpsc::Sender<FlowLikeEvent>>>,
}

impl FlowLikeState {
    pub fn new(
        config: FlowLikeConfig,
        client: HTTPClient,
    ) -> (Self, mpsc::Receiver<FlowLikeEvent>) {
        let (event_sender, event_receiver) = mpsc::channel(1000);

        (
            FlowLikeState {
                config: Arc::new(RwLock::new(config)),
                http_client: Arc::new(client),

                #[cfg(feature = "bit")]
                download_manager: Arc::new(Mutex::new(DownloadManager::new())),

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
                event_sender: Arc::new(Mutex::new(event_sender)),
            },
            event_receiver,
        )
    }

    pub fn instance(
        config: FlowLikeConfig,
        client: HTTPClient,
    ) -> (Arc<Mutex<Self>>, mpsc::Receiver<FlowLikeEvent>) {
        let (state, receiver) = Self::new(config, client);

        (Arc::new(Mutex::new(state)), receiver)
    }

    pub async fn emit<T>(&self, event_id: &str, payload: T) -> anyhow::Result<()>
    where
        T: Serialize + DeserializeOwned,
    {
        let event = FlowLikeEvent {
            event_id: event_id.to_string(),
            payload: serde_json::to_value(payload).unwrap(),
            timestamp: SystemTime::now(),
        };

        let event_sender = self.event_sender.lock().await;
        Ok(event_sender.send(event).await?)
    }

    /// Create a new instance of a subscriber, BE CAREFUL; THIS WILL OVERWRITE THE OLD SUBSCRIBER
    /// Use Cases: API where you want to listen to changes and send them to the client in a streaming scenario; Every API call needs separate callback handling.
    pub fn re_subscribe(&mut self) -> mpsc::Receiver<FlowLikeEvent> {
        let (event_sender, event_receiver) = mpsc::channel(1000);
        self.event_sender = Arc::new(Mutex::new(event_sender));
        event_receiver
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
    pub fn get_board(&self, board_id: &str) -> anyhow::Result<Arc<Mutex<Board>>> {
        let board = self.board_registry.try_get(board_id);

        match board.try_unwrap() {
            Some(board) => Ok(board.clone()),
            None => Err(anyhow::anyhow!("Board not found or could not be locked")),
        }
    }

    #[cfg(feature = "flow-runtime")]
    pub fn remove_board(&self, board_id: &str) -> anyhow::Result<Option<Arc<Mutex<Board>>>> {
        let removed = self.board_registry.remove(board_id);

        match removed {
            Some((_id, board)) => Ok(Some(board)),
            None => Ok(None),
        }
    }

    #[cfg(feature = "flow-runtime")]
    pub fn register_board(&self, board_id: &str, board: Arc<Mutex<Board>>) -> anyhow::Result<()> {
        self.board_registry
            .insert(board_id.to_string(), board.clone());
        Ok(())
    }

    #[cfg(feature = "flow-runtime")]
    pub fn board_run_registry(&self) -> Arc<DashMap<String, Arc<Mutex<InternalRun>>>> {
        self.board_run_registry.clone()
    }

    #[cfg(feature = "flow-runtime")]
    pub fn register_run(&self, run_id: &str, run: Arc<Mutex<InternalRun>>) {
        self.board_run_registry.insert(run_id.to_string(), run);
    }

    #[cfg(feature = "flow-runtime")]
    pub fn remove_run(&self, run_id: &str) -> Option<Arc<Mutex<InternalRun>>> {
        let removed = self.board_run_registry.remove(run_id);
        removed.map(|(_id, run)| run)
    }

    #[cfg(feature = "flow-runtime")]
    pub fn get_run(&self, run_id: &str) -> anyhow::Result<Arc<Mutex<InternalRun>>> {
        let run = self.board_run_registry.try_get(run_id);

        match run.try_unwrap() {
            Some(run) => Ok(run.clone()),
            None => Err(anyhow::anyhow!("Run not found or could not be locked")),
        }
    }

    #[inline]
    pub async fn stores(state: &Arc<Mutex<FlowLikeState>>) -> FlowLikeStores {
        state.lock().await.config.read().await.stores.clone()
    }

    #[inline]
    pub async fn project_store(state: &Arc<Mutex<FlowLikeState>>) -> anyhow::Result<FlowLikeStore> {
        state
            .lock()
            .await
            .config
            .read()
            .await
            .stores
            .project_store
            .clone()
            .ok_or(anyhow::anyhow!("No project store"))
    }

    #[inline]
    pub async fn bit_store(state: &Arc<Mutex<FlowLikeState>>) -> anyhow::Result<FlowLikeStore> {
        state
            .lock()
            .await
            .config
            .read()
            .await
            .stores
            .bits_store
            .clone()
            .ok_or(anyhow::anyhow!("No bit store"))
    }

    #[inline]
    pub async fn user_store(state: &Arc<Mutex<FlowLikeState>>) -> anyhow::Result<FlowLikeStore> {
        state
            .lock()
            .await
            .config
            .read()
            .await
            .stores
            .user_store
            .clone()
            .ok_or(anyhow::anyhow!("No user store"))
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
    use flow_like_storage::object_store::PutPayload;
    use flow_like_types::Cacheable;

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
        let read_bytes: bytes::Bytes = down_casted
            .as_generic()
            .get(&test_path)
            .await
            .unwrap()
            .bytes()
            .await
            .unwrap();
        let test_bytes = bytes::Bytes::from_static(test_string);
        assert_eq!(read_bytes, test_bytes);
    }
}
