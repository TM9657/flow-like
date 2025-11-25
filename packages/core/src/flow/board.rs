use super::{
    execution::LogLevel,
    node::{Node, NodeLogic},
    pin::Pin,
    variable::Variable,
};
use crate::{
    app::App,
    state::FlowLikeState,
    utils::compression::{compress_to_file, from_compressed},
};
use commands::GenericCommand;
use flow_like_storage::object_store::{ObjectStore, path::Path};
use flow_like_types::{FromProto, ToProto, create_id, sync::Mutex};
use futures::StreamExt;
use highway::{HighwayHash, HighwayHasher};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::{
    collections::HashMap,
    sync::{Arc, Weak},
    time::SystemTime,
};
use tracing::instrument;

pub mod cleanup;
pub mod commands;

#[derive(Debug, Clone)]
pub enum BoardParent {
    App(Weak<Mutex<App>>),
}

#[derive(Serialize, Deserialize, JsonSchema, Clone)]
pub enum ExecutionStage {
    Dev,
    Int,
    QA,
    PreProd,
    Prod,
}
#[derive(Serialize, Deserialize, JsonSchema, Clone, Debug)]
pub enum LayerType {
    Function,
    Macro,
    Collapsed,
}

#[derive(Serialize, Deserialize, JsonSchema, Clone)]
pub enum VersionType {
    Major,
    Minor,
    Patch,
}

#[derive(Serialize, Deserialize, JsonSchema, Clone, Debug)]
pub struct Layer {
    pub id: String,
    pub parent_id: Option<String>,
    pub name: String,
    pub r#type: LayerType,
    pub nodes: HashMap<String, Node>,
    pub variables: HashMap<String, Variable>,
    pub comments: HashMap<String, Comment>,
    pub coordinates: (f32, f32, f32),
    pub in_coordinates: Option<(f32, f32, f32)>,
    pub out_coordinates: Option<(f32, f32, f32)>,
    pub pins: HashMap<String, Pin>,
    pub comment: Option<String>,
    pub error: Option<String>,
    pub color: Option<String>,
    pub hash: Option<u64>,
}

impl Layer {
    pub fn new(id: String, name: String, r#type: LayerType) -> Self {
        Layer {
            id,
            parent_id: None,
            name,
            r#type,
            nodes: HashMap::new(),
            variables: HashMap::new(),
            comments: HashMap::new(),
            coordinates: (0.0, 0.0, 0.0),
            in_coordinates: None,
            out_coordinates: None,
            pins: HashMap::new(),
            comment: None,
            error: None,
            color: None,
            hash: None,
        }
    }

    pub fn hash(&mut self) {
        let mut hasher = HighwayHasher::new(highway::Key([
            0x0123456789abcdfe,
            0xfedcba9876543210,
            0x0011223344556677,
            0x8899aabbccddeeff,
        ]));

        hasher.append(self.id.as_bytes());
        hasher.append(self.name.as_bytes());
        hasher.append(format!("{:?}", self.r#type).as_bytes());

        if let Some(parent_id) = &self.parent_id {
            hasher.append(parent_id.as_bytes());
        }

        let mut sorted_nodes: Vec<_> = self.nodes.iter().collect();
        sorted_nodes.sort_by_key(|(id, _)| *id);
        for (id, node) in sorted_nodes {
            hasher.append(id.as_bytes());
            hasher.append(node.id.as_bytes());
        }

        let mut sorted_variables: Vec<_> = self.variables.iter().collect();
        sorted_variables.sort_by_key(|(id, _)| *id);
        for (id, variable) in sorted_variables {
            hasher.append(id.as_bytes());
            hasher.append(variable.id.as_bytes());
        }

        let mut sorted_comments: Vec<_> = self.comments.iter().collect();
        sorted_comments.sort_by_key(|(id, _)| *id);
        for (id, comment) in sorted_comments {
            hasher.append(id.as_bytes());
            hasher.append(comment.id.as_bytes());
        }

        let mut sorted_pins: Vec<_> = self.pins.iter().collect();
        sorted_pins.sort_by_key(|(id, _)| *id);
        for (_id, pin) in sorted_pins {
            pin.hash(&mut hasher);
        }

        hasher.append(&self.coordinates.0.to_le_bytes());
        hasher.append(&self.coordinates.1.to_le_bytes());
        hasher.append(&self.coordinates.2.to_le_bytes());

        if let Some(comment) = &self.comment {
            hasher.append(comment.as_bytes());
        }

        if let Some(color) = &self.color {
            hasher.append(color.as_bytes());
        }

        self.hash = Some(hasher.finalize64());
    }
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
    pub version: (u32, u32, u32),
    pub stage: ExecutionStage,
    pub log_level: LogLevel,
    pub refs: HashMap<String, String>,
    pub layers: HashMap<String, Layer>,

    pub created_at: SystemTime,
    pub updated_at: SystemTime,

    #[serde(skip)]
    pub parent: Option<BoardParent>,

    #[serde(skip)]
    pub board_dir: Path,

    #[serde(skip)]
    pub logic_nodes: HashMap<String, Arc<dyn NodeLogic>>,

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
    pub fn new(id: Option<String>, base_dir: Path, app_state: Arc<Mutex<FlowLikeState>>) -> Self {
        let id = id.unwrap_or(create_id());
        let board_dir = base_dir;

        Board {
            id,
            name: "New Board".to_string(),
            description: "Your new Workflow!".to_string(),
            nodes: HashMap::new(),
            variables: HashMap::new(),
            comments: HashMap::new(),
            log_level: LogLevel::Info,
            stage: ExecutionStage::Dev,
            viewport: (0.0, 0.0, 0.0),
            version: (0, 0, 1),
            created_at: SystemTime::now(),
            updated_at: SystemTime::now(),
            layers: HashMap::new(),
            refs: HashMap::new(),
            parent: None,
            board_dir,
            logic_nodes: HashMap::new(),
            app_state: Some(app_state.clone()),
        }
    }

    async fn node_updates(&mut self, state: Arc<Mutex<FlowLikeState>>) {
        let reference = Arc::new(self.clone());
        let registry = state.lock().await.node_registry().clone();
        let registry = registry.read().await;
        for node in self.nodes.values_mut() {
            let node_logic = match self.logic_nodes.get(&node.name) {
                Some(logic) => Arc::clone(logic),
                None => match registry.instantiate(node) {
                    Ok(new_logic) => {
                        self.logic_nodes
                            .insert(node.name.clone(), Arc::clone(&new_logic));
                        Arc::clone(&new_logic)
                    }
                    Err(_) => continue,
                },
            };
            node_logic.on_update(node, reference.clone()).await;

            node.hash();
        }

        for layer in self.layers.values_mut() {
            layer.hash();
        }

        for variable in self.variables.values_mut() {
            variable.hash();
        }

        for comment in self.comments.values_mut() {
            comment.hash();
        }
    }

    pub async fn execute_command(
        &mut self,
        command: GenericCommand,
        state: Arc<Mutex<FlowLikeState>>,
    ) -> flow_like_types::Result<GenericCommand> {
        let mut command = command;
        let cmd_json = serde_json::to_string(&command).unwrap_or_default();
        println!("[Board] execute_command: {}", cmd_json);
        if let Err(e) = command.execute(self, state.clone()).await {
            println!("[Board] execute_command ❌ ERROR: {:?}", e);
            return Err(e);
        }
        println!("[Board] execute_command ✓ Success");
        self.node_updates(state).await;
        self.updated_at = SystemTime::now();
        self.cleanup();
        Ok(command)
    }

    pub async fn execute_commands(
        &mut self,
        commands: Vec<GenericCommand>,
        state: Arc<Mutex<FlowLikeState>>,
    ) -> flow_like_types::Result<Vec<GenericCommand>> {
        let mut commands = commands;
        println!("[Board] execute_commands: {} commands", commands.len());
        for (i, command) in commands.iter_mut().enumerate() {
            let cmd_json = serde_json::to_string(&command).unwrap_or_default();
            println!("[Board]   [{}] Executing: {}", i, cmd_json);
            let res = command.execute(self, state.clone()).await;
            if let Err(e) = res {
                println!("[Board]   [{}] ❌ ERROR: {:?}", i, e);
            } else {
                println!("[Board]   [{}] ✓ Success", i);
            }
        }
        self.node_updates(state).await;
        self.updated_at = SystemTime::now();
        self.cleanup();
        Ok(commands)
    }

    pub async fn undo(
        &mut self,
        commands: Vec<GenericCommand>,
        state: Arc<Mutex<FlowLikeState>>,
    ) -> flow_like_types::Result<()> {
        let mut commands = commands;
        for command in commands.iter_mut().rev() {
            command.undo(self, state.clone()).await?;
        }
        self.cleanup();
        Ok(())
    }

    pub async fn redo(
        &mut self,
        commands: Vec<GenericCommand>,
        state: Arc<Mutex<FlowLikeState>>,
    ) -> flow_like_types::Result<()> {
        let mut commands = commands;
        for command in commands.iter_mut() {
            command.execute(self, state.clone()).await?;
        }
        self.cleanup();
        Ok(())
    }

    pub fn get_pin_by_id(&self, pin_id: &str) -> Option<&Pin> {
        for node in self.nodes.values() {
            if let Some(pin) = node.pins.get(pin_id) {
                return Some(pin);
            }
        }

        for layer in self.layers.values() {
            if let Some(pin) = layer.pins.get(pin_id) {
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

    pub async fn create_version(
        &mut self,
        version_type: VersionType,
        store: Option<Arc<dyn ObjectStore>>,
    ) -> flow_like_types::Result<(u32, u32, u32)> {
        let version = self.version;

        let to = self
            .board_dir
            .child("versions")
            .child(self.id.clone())
            .child(format!(
                "{}_{}_{}.board",
                self.version.0, self.version.1, self.version.2
            ));

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
                .app_meta_store
                .clone()
                .ok_or(flow_like_types::anyhow!("Project store not found"))?
                .as_generic(),
        };

        let board = self.to_proto();
        compress_to_file(store.clone(), to, &board).await?;

        let new_version = match version_type {
            VersionType::Major => (version.0 + 1, 0, 0),
            VersionType::Minor => (version.0, version.1 + 1, 0),
            VersionType::Patch => (version.0, version.1, version.2 + 1),
        };

        self.version = new_version;
        self.updated_at = SystemTime::now();
        self.save(Some(store)).await?;
        Ok(new_version)
    }

    pub async fn get_versions(
        &self,
        store: Option<Arc<dyn ObjectStore>>,
    ) -> flow_like_types::Result<Vec<(u32, u32, u32)>> {
        let versions_dir = self
            .board_dir
            .clone()
            .child("versions")
            .child(self.id.clone());

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
                .app_meta_store
                .clone()
                .ok_or(flow_like_types::anyhow!("Project store not found"))?
                .as_generic(),
        };

        let mut versions = store.list(Some(&versions_dir));
        let mut version_list = Vec::new();

        while let Some(Ok(meta)) = versions.next().await {
            let file_name = match meta.location.filename() {
                Some(name) => name,
                None => continue,
            };
            if !file_name.ends_with(".board") {
                continue;
            }
            let version = file_name.strip_suffix(".board").unwrap_or(file_name);
            if version == "latest" {
                continue;
            }
            let version = version.strip_prefix("v").unwrap_or(version);
            let version = version.split("_").collect::<Vec<&str>>();

            if version.len() < 3 {
                continue;
            }

            let version = (
                version[0].parse::<u32>().unwrap_or(0),
                version[1].parse::<u32>().unwrap_or(0),
                version[2].parse::<u32>().unwrap_or(0),
            );

            version_list.push(version);
        }
        Ok(version_list)
    }

    #[instrument(name = "Board::load", skip(app_state), level = "debug")]
    pub async fn load(
        path: Path,
        id: &str,
        app_state: Arc<Mutex<FlowLikeState>>,
        version: Option<(u32, u32, u32)>,
    ) -> flow_like_types::Result<Self> {
        let store = app_state
            .lock()
            .await
            .config
            .read()
            .await
            .stores
            .app_meta_store
            .clone()
            .ok_or_else(|| {
                tracing::error!("Project store not found while loading board: id={}", id);
                flow_like_types::anyhow!("Project store not found")
            })?
            .as_generic();

        let board_dir = path.clone();
        let path = if let Some(version) = version {
            path.child("versions")
                .child(id)
                .child(format!("{}_{}_{}.board", version.0, version.1, version.2))
        } else {
            path.child(format!("{}.board", id))
        };

        let board: flow_like_types::proto::Board = from_compressed(store, path).await?;
        let mut board = Board::from_proto(board);
        board.board_dir = board_dir;
        board.app_state = Some(app_state.clone());
        board.logic_nodes = HashMap::new();
        Ok(board)
    }

    pub async fn save(&self, store: Option<Arc<dyn ObjectStore>>) -> flow_like_types::Result<()> {
        let to = self.board_dir.child(format!("{}.board", self.id));
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
                .app_meta_store
                .clone()
                .ok_or(flow_like_types::anyhow!("Project store not found"))?
                .as_generic(),
        };

        let board = self.to_proto();
        compress_to_file(store, to, &board).await?;
        Ok(())
    }

    /// TEMPLATE FUNCTIONS

    pub async fn save_as_template(
        &self,
        store: Option<Arc<dyn ObjectStore>>,
    ) -> flow_like_types::Result<()> {
        let to = self.board_dir.child(format!("{}.template", self.id));
        println!("Saving template to: {:?}", to);
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
                .app_meta_store
                .clone()
                .ok_or(flow_like_types::anyhow!("Project store not found"))?
                .as_generic(),
        };

        let board = self.to_proto();
        compress_to_file(store, to, &board).await?;
        Ok(())
    }

    pub async fn overwrite_template_version(
        &mut self,
        version: (u32, u32, u32),
        store: Option<Arc<dyn ObjectStore>>,
    ) -> flow_like_types::Result<()> {
        let to = self
            .board_dir
            .child("templates")
            .child("versions")
            .child(self.id.clone())
            .child(format!(
                "{}_{}_{}.template",
                version.0, version.1, version.2
            ));

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
                .app_meta_store
                .clone()
                .ok_or(flow_like_types::anyhow!("Project store not found"))?
                .as_generic(),
        };

        let board = self.to_proto();
        compress_to_file(store, to, &board).await?;
        Ok(())
    }

    pub async fn create_template(
        &mut self,
        template_id: String,
        version_type: VersionType,
        old_template: Option<Board>,
        store: Option<Arc<dyn ObjectStore>>,
    ) -> flow_like_types::Result<(u32, u32, u32)> {
        // Either the old_template version or (0,0,0)
        let version = {
            if let Some(old_template) = &old_template {
                old_template.version
            } else {
                (0, 0, 0)
            }
        };

        let to = self
            .board_dir
            .child("templates")
            .child("versions")
            .child(self.id.clone())
            .child(format!(
                "{}_{}_{}.template",
                version.0, version.1, version.2
            ));

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
                .app_meta_store
                .clone()
                .ok_or(flow_like_types::anyhow!("Project store not found"))?
                .as_generic(),
        };

        let mut new_version = (0, 0, 0);

        if let Some(old_template) = &old_template {
            // If an old template is provided, we move it to the versions directory
            compress_to_file(store.clone(), to, &old_template.to_proto()).await?;
            new_version = match version_type {
                VersionType::Major => (version.0 + 1, 0, 0),
                VersionType::Minor => (version.0, version.1 + 1, 0),
                VersionType::Patch => (version.0, version.1, version.2 + 1),
            }
        }

        let mut template = self.clone();
        template.id = template_id;
        template.version = new_version;
        template.updated_at = SystemTime::now();

        for variable in template.variables.values_mut() {
            if variable.secret {
                variable.default_value = None;
            }
        }

        template.save_as_template(Some(store)).await?;
        Ok(new_version)
    }

    pub async fn load_template(
        path: Path,
        template_id: &str,
        app_state: Arc<Mutex<FlowLikeState>>,
        version: Option<(u32, u32, u32)>,
    ) -> flow_like_types::Result<Self> {
        let store = app_state
            .lock()
            .await
            .config
            .read()
            .await
            .stores
            .app_meta_store
            .clone()
            .ok_or(flow_like_types::anyhow!("Project store not found"))?
            .as_generic();

        let board_dir = path.clone();
        let path = if let Some(version) = version {
            path.child("templates")
                .child("versions")
                .child(template_id)
                .child(format!(
                    "{}_{}_{}.template",
                    version.0, version.1, version.2
                ))
        } else {
            path.child(format!("{}.template", template_id))
        };

        let board: flow_like_types::proto::Board = from_compressed(store, path).await?;
        let mut board = Board::from_proto(board);
        board.board_dir = board_dir;
        board.app_state = Some(app_state.clone());
        board.logic_nodes = HashMap::new();
        Ok(board)
    }

    pub async fn get_template_versions(
        &self,
        store: Option<Arc<dyn ObjectStore>>,
    ) -> flow_like_types::Result<Vec<(u32, u32, u32)>> {
        let versions_dir = self
            .board_dir
            .clone()
            .child("templates")
            .child("versions")
            .child(self.id.clone());

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
                .app_meta_store
                .clone()
                .ok_or(flow_like_types::anyhow!("Project store not found"))?
                .as_generic(),
        };

        let mut versions = store.list(Some(&versions_dir));
        let mut version_list = Vec::new();

        while let Some(Ok(meta)) = versions.next().await {
            let file_name = match meta.location.filename() {
                Some(name) => name,
                None => continue,
            };
            if !file_name.ends_with(".template") {
                continue;
            }
            let version = file_name.strip_suffix(".template").unwrap_or(file_name);
            if version == "latest" {
                continue;
            }
            let version = version.strip_prefix("v").unwrap_or(version);
            let version = version.split("_").collect::<Vec<&str>>();

            if version.len() < 3 {
                continue;
            }

            let version = (
                version[0].parse::<u32>().unwrap_or(0),
                version[1].parse::<u32>().unwrap_or(0),
                version[2].parse::<u32>().unwrap_or(0),
            );

            version_list.push(version);
        }
        Ok(version_list)
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
    pub id: String,
    pub author: Option<String>,
    pub content: String,
    pub comment_type: CommentType,
    pub timestamp: SystemTime,
    pub coordinates: (f32, f32, f32),
    pub width: Option<f32>,
    pub height: Option<f32>,
    pub layer: Option<String>,
    pub color: Option<String>,
    pub z_index: Option<i32>,
    pub hash: Option<u64>,
    pub is_locked: Option<bool>,
}

impl Comment {
    pub fn hash(&mut self) {
        let mut hasher = HighwayHasher::new(highway::Key([
            0x0123456789abcdfe,
            0xfedcba9876543210,
            0x0011223344556677,
            0x8899aabbccddeeff,
        ]));

        hasher.append(self.id.as_bytes());
        hasher.append(self.content.as_bytes());
        hasher.append(format!("{:?}", self.comment_type).as_bytes());

        if let Some(author) = &self.author {
            hasher.append(author.as_bytes());
        }

        hasher.append(&self.coordinates.0.to_le_bytes());
        hasher.append(&self.coordinates.1.to_le_bytes());
        hasher.append(&self.coordinates.2.to_le_bytes());

        if let Some(width) = self.width {
            hasher.append(&width.to_le_bytes());
        }

        if let Some(height) = self.height {
            hasher.append(&height.to_le_bytes());
        }

        if let Some(layer) = &self.layer {
            hasher.append(layer.as_bytes());
        }

        if let Some(color) = &self.color {
            hasher.append(color.as_bytes());
        }

        if let Some(z_index) = self.z_index {
            hasher.append(&z_index.to_le_bytes());
        }

        if let Some(is_locked) = self.is_locked {
            hasher.append(&[is_locked as u8]);
        }

        self.hash = Some(hasher.finalize64());
    }
}

#[cfg(test)]
mod tests {
    use crate::{state::FlowLikeConfig, utils::http::HTTPClient};
    use flow_like_storage::{
        files::store::FlowLikeStore,
        object_store::{self, path::Path},
    };
    use flow_like_types::{FromProto, ToProto};
    use flow_like_types::{Message, sync::Mutex, tokio};
    use std::sync::Arc;

    async fn flow_state() -> Arc<Mutex<crate::state::FlowLikeState>> {
        let mut config: FlowLikeConfig = FlowLikeConfig::new();
        config.register_app_meta_store(FlowLikeStore::Other(Arc::new(
            object_store::memory::InMemory::new(),
        )));
        let (http_client, _refetch_rx) = HTTPClient::new();
        let flow_like_state = crate::state::FlowLikeState::new(config, http_client);
        Arc::new(Mutex::new(flow_like_state))
    }

    #[tokio::test]
    async fn serialize_board() {
        let state = flow_state().await;
        let base_dir = Path::from("boards");
        let board = super::Board::new(None, base_dir, state);

        let mut buf = Vec::new();
        board.to_proto().encode(&mut buf).unwrap();
        let deser_board =
            super::Board::from_proto(flow_like_types::proto::Board::decode(&buf[..]).unwrap());

        assert_eq!(board.id, deser_board.id);
    }
}
