use crate::{
    state::FlowLikeState,
    utils::{
        compression::{compress_to_file, from_compressed},
        hash::hash_string_non_cryptographic,
    },
    vault::Vault,
};
use async_trait::async_trait;
use object_store::{path::Path, ObjectStore};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::{
    collections::{HashMap, HashSet},
    sync::{Arc, Weak},
    time::SystemTime,
};
use tokio::sync::Mutex;

use super::{
    execution::LogLevel,
    node::{Node, NodeLogic},
    pin::Pin,
    variable::Variable,
};

pub mod commands;

#[derive(Debug, Clone)]
pub enum BoardParent {
    Vault(Weak<Mutex<Vault>>),
}

#[derive(Serialize, Deserialize, JsonSchema, Clone)]
pub enum ExecutionStage {
    Dev,
    Int,
    QA,
    PreProd,
    Prod,
}

#[derive(Serialize, Deserialize, JsonSchema, Clone)]
pub struct Board {
    pub id: String,
    pub name: String,
    pub description: String,
    pub nodes: HashMap<String, Node>,
    pub variables: HashMap<String, Variable>,
    pub comments: HashMap<String, Comment>,
    pub viewport: (f32, f32, f32),
    pub version: (u8, u8, u8),
    pub stage: ExecutionStage,
    pub log_level: LogLevel,
    pub refs: HashMap<String, String>,

    pub created_at: SystemTime,
    pub updated_at: SystemTime,

    #[serde(skip)]
    pub parent: Option<BoardParent>,

    #[serde(skip)]
    pub undo_stack: Vec<Vec<Arc<Mutex<dyn Command>>>>,
    #[serde(skip)]
    pub redo_stack: Vec<Vec<Arc<Mutex<dyn Command>>>>,

    #[serde(skip)]
    pub board_dir: Path,

    #[serde(skip)]
    pub logic_nodes: HashMap<String, Arc<Mutex<dyn NodeLogic>>>,

    #[serde(skip)]
    pub app_state: Option<Arc<Mutex<FlowLikeState>>>,
}

#[derive(Serialize, Deserialize, JsonSchema, Clone)]
pub struct BoardUndoRedoStack {
    pub undo_stack: Vec<String>,
    pub redo_stack: Vec<String>,
}

impl Board {
    /// Create a new board with a unique ID
    /// The board is created in the base directory appended with the ID
    pub fn new(base_dir: Path, app_state: Arc<Mutex<FlowLikeState>>) -> Self {
        let id = cuid2::create_id();
        let board_dir = base_dir.child(id.clone());

        Board {
            id,
            name: "New Board".to_string(),
            description: "".to_string(),
            nodes: HashMap::new(),
            variables: HashMap::new(),
            comments: HashMap::new(),
            log_level: LogLevel::Debug,
            stage: ExecutionStage::Dev,
            viewport: (0.0, 0.0, 0.0),
            version: (0, 0, 1),
            created_at: SystemTime::now(),
            updated_at: SystemTime::now(),
            refs: HashMap::new(),
            parent: None,
            undo_stack: Vec::new(),
            redo_stack: Vec::new(),
            board_dir,
            logic_nodes: HashMap::new(),
            app_state: Some(app_state.clone()),
        }
    }

    async fn fixate_node_update(&mut self, state: Arc<Mutex<FlowLikeState>>) {
        let reference = Arc::new(self.clone());
        for node in self.nodes.values_mut() {
            let node_logic = match self.logic_nodes.get(&node.name) {
                Some(logic) => Arc::clone(logic),
                None => {
                    match state
                        .lock()
                        .await
                        .node_registry()
                        .read()
                        .await
                        .instantiate(node)
                        .await
                    {
                        Ok(new_logic) => {
                            self.logic_nodes
                                .insert(node.name.clone(), Arc::clone(&new_logic));
                            Arc::clone(&new_logic)
                        }
                        Err(_) => continue,
                    }
                }
            };
            node_logic
                .lock()
                .await
                .on_update(node, reference.clone())
                .await;
        }
    }

    pub async fn execute_command(
        &mut self,
        command: Arc<Mutex<dyn Command>>,
        state: Arc<Mutex<FlowLikeState>>,
        append: bool,
    ) -> anyhow::Result<()> {
        command.lock().await.execute(self, state.clone()).await?;
        if append {
            self.undo_stack.last_mut().unwrap().push(command);
        } else {
            self.undo_stack.push(vec![command]);
        }
        self.redo_stack.clear();
        self.fixate_node_update(state).await;
        self.updated_at = SystemTime::now();
        Ok(())
    }

    pub async fn undo(&mut self, state: Arc<Mutex<FlowLikeState>>) -> anyhow::Result<()> {
        if let Some(commands) = self.undo_stack.pop() {
            let mut redo_commands = vec![];
            for command in commands.iter().rev() {
                command.lock().await.undo(self, state.clone()).await?;
                redo_commands.push(command.clone());
            }
            self.redo_stack.push(redo_commands);
            Ok(())
        } else {
            Err(anyhow::anyhow!("No actions to undo"))
        }
    }

    pub async fn redo(&mut self, state: Arc<Mutex<FlowLikeState>>) -> anyhow::Result<()> {
        if let Some(commands) = self.redo_stack.pop() {
            let mut undo_commands = vec![];
            for command in commands.iter().rev() {
                command.lock().await.execute(self, state.clone()).await?;
                undo_commands.push(command.clone());
            }
            self.undo_stack.push(undo_commands);
            Ok(())
        } else {
            Err(anyhow::anyhow!("No actions to redo"))
        }
    }

    pub fn cleanup(&mut self) {
        let mut refs = self.refs.clone();
        let mut abandoned_hashes = refs.keys().cloned().collect::<HashSet<String>>();
        for node in self.nodes.values_mut() {
            if !refs.contains_key(&node.description) {
                let description_hash = hash_string_non_cryptographic(&node.description).to_string();
                refs.insert(description_hash.clone(), node.description.clone());
                abandoned_hashes.remove(&description_hash);
                node.description = description_hash;
            } else {
                abandoned_hashes.remove(&node.description);
            }

            for pin in node.pins.values_mut() {
                if !refs.contains_key(&pin.description) {
                    let description_hash =
                        hash_string_non_cryptographic(&pin.description).to_string();
                    refs.insert(description_hash.clone(), pin.description.clone());
                    abandoned_hashes.remove(&description_hash);
                    pin.description = description_hash;
                } else {
                    abandoned_hashes.remove(&pin.description);
                }

                if let Some(schema) = pin.schema.clone() {
                    if !refs.contains_key(&schema) {
                        let schema_hash = hash_string_non_cryptographic(&schema).to_string();
                        refs.insert(schema_hash.clone(), schema.clone());
                        abandoned_hashes.remove(&schema_hash);
                        pin.schema = Some(schema_hash);
                    } else {
                        abandoned_hashes.remove(&schema);
                    }
                }
            }
        }

        self.refs = refs;
    }

    pub fn fix_pins(&mut self) {
        let mut pins = HashMap::with_capacity(self.nodes.len() * 2);
        for node in self.nodes.values() {
            for pin in node.pins.values() {
                pins.insert(pin.id.clone(), pin);
            }
        }

        let mut node_pins_to_remove = HashMap::new();
        let mut node_pins_connected_to_remove = HashMap::new();
        let mut node_pins_depends_on_remove = HashMap::new();
        for node in self.nodes.values() {
            for pin in node.pins.values() {
                if !pins.contains_key(&pin.id) {
                    node_pins_to_remove.insert(node.id.clone(), pin.id.clone());
                    continue;
                }

                for connected_to in &pin.connected_to {
                    if let Some(connected_pin) = pins.get(connected_to) {
                        if !connected_pin.depends_on.contains(&pin.id) {
                            node_pins_connected_to_remove
                                .entry(node.id.clone())
                                .or_insert_with(HashMap::new)
                                .insert(pin.id.clone(), connected_to.clone());
                        }

                        continue;
                    }

                    node_pins_connected_to_remove
                        .entry(node.id.clone())
                        .or_insert_with(HashMap::new)
                        .insert(pin.id.clone(), connected_to.clone());
                }

                for depends_on in &pin.depends_on {
                    if let Some(depends_on_pin) = pins.get(depends_on) {
                        if !depends_on_pin.connected_to.contains(&pin.id) {
                            node_pins_depends_on_remove
                                .entry(node.id.clone())
                                .or_insert_with(HashMap::new)
                                .insert(pin.id.clone(), depends_on.clone());
                        }

                        continue;
                    }

                    node_pins_depends_on_remove
                        .entry(node.id.clone())
                        .or_insert_with(HashMap::new)
                        .insert(pin.id.clone(), depends_on.clone());
                }
            }
        }

        for (node_id, pin_id) in node_pins_to_remove {
            if let Some(node) = self.nodes.get_mut(&node_id) {
                println!("Node Pins to remove: {} {}", node_id, pin_id);
                node.pins.remove(&pin_id);
            }
        }

        for (node_id, pins) in node_pins_connected_to_remove {
            if let Some(node) = self.nodes.get_mut(&node_id) {
                for (pin_id, connected_to) in pins {
                    if let Some(pin) = node.pins.get_mut(&pin_id) {
                        println!(
                            "Node Pins connected to remove: {} {} {}",
                            node_id, pin_id, connected_to
                        );
                        pin.connected_to.remove(&connected_to);
                    }
                }
            }
        }

        for (node_id, pins) in node_pins_depends_on_remove {
            if let Some(node) = self.nodes.get_mut(&node_id) {
                for (pin_id, depends_on) in pins {
                    if let Some(pin) = node.pins.get_mut(&pin_id) {
                        println!(
                            "Node Pins depends on remove: {} {} {}",
                            node_id, pin_id, depends_on
                        );
                        pin.depends_on.remove(&depends_on);
                    }
                }
            }
        }

        self.cleanup();
    }

    pub fn get_pin_by_id(&self, pin_id: &str) -> Option<&Pin> {
        for node in self.nodes.values() {
            if let Some(pin) = node.pins.get(pin_id) {
                return Some(pin);
            }
        }

        None
    }

    pub fn get_dependent_nodes(&self, node_id: &str) -> Vec<&Node> {
        let mut dependent_nodes = HashMap::new();
        for node in self.nodes.values() {
            for pin in node.pins.values() {
                if pin.depends_on.contains(node_id) {
                    dependent_nodes.insert(&node.id, node);
                }
            }
        }

        dependent_nodes.values().cloned().collect()
    }

    pub fn get_connected_nodes(&self, node_id: &str) -> Vec<&Node> {
        let mut connected_nodes = HashMap::new();
        for node in self.nodes.values() {
            for pin in node.pins.values() {
                if pin.connected_to.contains(node_id) {
                    connected_nodes.insert(&node.id, node);
                }
            }
        }

        connected_nodes.values().cloned().collect()
    }

    pub fn get_variable(&self, variable_id: &str) -> Option<&Variable> {
        self.variables.get(variable_id)
    }

    pub async fn save(&self, store: Option<Arc<dyn ObjectStore>>) -> anyhow::Result<()> {
        let to = self.board_dir.child("manifest.board");
        let store = match store {
            Some(store) => store,
            None => self
                .app_state
                .as_ref()
                .expect("app_state should always be set")
                .lock()
                .await
                .config
                .read()
                .await
                .stores
                .project_store
                .clone()
                .ok_or(anyhow::anyhow!("Project store not found"))?
                .as_generic(),
        };

        compress_to_file(store, to, self).await?;
        Ok(())
    }

    pub async fn load(path: Path, app_state: Arc<Mutex<FlowLikeState>>) -> anyhow::Result<Self> {
        let store = app_state
            .lock()
            .await
            .config
            .read()
            .await
            .stores
            .project_store
            .clone()
            .ok_or(anyhow::anyhow!("Project store not found"))?
            .as_generic();

        let mut board: Board = from_compressed(store, path.child("manifest.board")).await?;
        board.board_dir = path;
        board.app_state = Some(app_state.clone());
        board.redo_stack = Vec::new();
        board.undo_stack = Vec::new();
        board.logic_nodes = HashMap::new();
        board.fix_pins();
        Ok(board)
    }
}

#[derive(Serialize, Deserialize, JsonSchema, Debug, Clone)]
pub enum CommentType {
    Text,
    Image,
    Video,
}

#[derive(Serialize, Deserialize, JsonSchema, Debug, Clone)]
pub struct Comment {
    id: String,
    author: Option<String>,
    content: String,
    comment_type: CommentType,
    timestamp: SystemTime,
    coordinates: (f32, f32, f32),
}

#[async_trait]
pub trait Command: Send + Sync {
    async fn execute(
        &mut self,
        board: &mut Board,
        state: Arc<Mutex<FlowLikeState>>,
    ) -> anyhow::Result<()>;
    async fn undo(
        &mut self,
        board: &mut Board,
        state: Arc<Mutex<FlowLikeState>>,
    ) -> anyhow::Result<()>;

    async fn node_to_logic(
        &self,
        node: &Node,
        state: Arc<Mutex<FlowLikeState>>,
    ) -> anyhow::Result<Arc<Mutex<dyn NodeLogic>>> {
        let node_registry = state.lock().await;
        let node_registry = node_registry.node_registry();
        let node_registry = node_registry.read().await;
        Ok(node_registry.instantiate(node).await?)
    }
}

#[cfg(test)]
mod tests {
    use crate::{state::FlowLikeConfig, utils::http::HTTPClient};
    use object_store::path::Path;
    use std::sync::Arc;
    use tokio::sync::Mutex;

    async fn flow_state() -> Arc<Mutex<crate::state::FlowLikeState>> {
        let mut config: FlowLikeConfig = FlowLikeConfig::new();
        config.register_project_store(crate::state::FlowLikeStore::Remote(Arc::new(
            object_store::memory::InMemory::new(),
        )));
        let (http_client, _refetch_rx) = HTTPClient::new();
        let (flow_like_state, _) = crate::state::FlowLikeState::new(config, http_client);
        Arc::new(Mutex::new(flow_like_state))
    }

    #[tokio::test]
    async fn serialize_board() {
        let state = flow_state().await;
        let base_dir = Path::from("boards");
        let board = super::Board::new(base_dir, state);

        let ser = bitcode::serialize(&board).unwrap();
        let deser: super::Board = bitcode::deserialize(&ser).unwrap();

        assert_eq!(board.id, deser.id);
    }
}
