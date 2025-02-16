use anyhow::Ok;
use object_store::ObjectStore;
use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashMap;
use std::sync::Arc;
use std::time::SystemTime;
use tokio::sync::{mpsc, Mutex, RwLock};

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
use crate::utils::local_object_store::LocalObjectStore;

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

#[derive(Debug, Clone)]
pub struct FlowLikeConfig {
    pub local_store: Option<Arc<LocalObjectStore>>,
    pub bits_store: Arc<dyn ObjectStore>,
    pub user_store: Arc<dyn ObjectStore>,
    pub project_store: Arc<dyn ObjectStore>,
}

impl FlowLikeConfig {
    pub fn new<T: ObjectStore>(
        local_store: Option<Arc<LocalObjectStore>>,
        bits_store: Arc<T>,
        user_store: Arc<T>,
        project_store: Arc<T>,
    ) -> Self {
        FlowLikeConfig {
            local_store,
            bits_store,
            user_store,
            project_store,
        }
    }
}

#[cfg(feature = "flow-runtime")]
pub struct FlowNodeRegistry {
    // TODO: replace with dashmap
    pub node_registry: HashMap<String, (Node, Arc<Mutex<dyn NodeLogic>>)>,
    initialized: bool,
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
            node_registry: HashMap::new(),
            initialized: false,
        }
    }

    pub fn get_nodes(&self) -> anyhow::Result<Vec<Node>> {
        if !self.initialized {
            return Err(anyhow::anyhow!("Registry not initialized"));
        }

        let nodes = self
            .node_registry
            .iter()
            .map(|(_, (node, _))| node.clone())
            .collect();
        Ok(nodes)
    }

    pub fn initialize(&mut self, nodes: Vec<(Node, Arc<Mutex<dyn NodeLogic>>)>) {
        for (node, logic) in nodes {
            self.node_registry.insert(node.name.clone(), (node, logic));
        }

        self.initialized = true;
    }

    pub fn get_node(&self, node_id: &str) -> anyhow::Result<Node> {
        if !self.initialized {
            return Err(anyhow::anyhow!("Registry not initialized"));
        }

        let node = self.node_registry.get(node_id);
        match node {
            Some(node) => Ok(node.0.clone()),
            None => Err(anyhow::anyhow!("Node not found")),
        }
    }

    pub async fn instantiate(&self, node: &Node) -> anyhow::Result<Arc<Mutex<dyn NodeLogic>>> {
        if !self.initialized {
            return Err(anyhow::anyhow!("Registry not initialized"));
        }

        let node = self.node_registry.get(&node.name);
        match node {
            Some((_, logic)) => Ok(logic.clone()),
            None => Err(anyhow::anyhow!("Node not found")),
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
    pub model_factory: Arc<Mutex<ModelFactory>>,

    #[cfg(feature = "model")]
    pub embedding_factory: Arc<Mutex<EmbeddingFactory>>,

    #[cfg(feature = "flow-runtime")]
    pub node_registry: Arc<RwLock<FlowNodeRegistry>>,
    #[cfg(feature = "flow-runtime")]
    pub board_registry: Arc<Mutex<HashMap<String, Arc<Mutex<Board>>>>>,
    #[cfg(feature = "flow-runtime")]
    pub board_run_registry: Arc<Mutex<HashMap<String, Arc<Mutex<InternalRun>>>>>,

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
                board_registry: Arc::new(Mutex::new(HashMap::new())),
                #[cfg(feature = "flow-runtime")]
                board_run_registry: Arc::new(Mutex::new(HashMap::new())),
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
    pub fn board_registry(&self) -> Arc<Mutex<HashMap<String, Arc<Mutex<Board>>>>> {
        self.board_registry.clone()
    }

    #[cfg(feature = "flow-runtime")]
    pub fn board_run_registry(&self) -> Arc<Mutex<HashMap<String, Arc<Mutex<InternalRun>>>>> {
        self.board_run_registry.clone()
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
