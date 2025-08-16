use super::{
    execution::LogLevel,
    node::{Node, NodeLogic},
    pin::Pin,
    variable::Variable,
};
use crate::{
    app::App,
    state::FlowLikeState,
    utils::{
        compression::{compress_to_file, from_compressed},
        hash::hash_string_non_cryptographic,
    },
};
use commands::GenericCommand;
use flow_like_storage::object_store::{ObjectStore, path::Path};
use flow_like_types::{FromProto, ToProto, create_id, sync::Mutex};
use futures::StreamExt;
use highway::{HighwayHash, HighwayHasher};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::{
    collections::{HashMap, HashSet},
    sync::{Arc, Weak},
    time::SystemTime,
};
use tracing::instrument;

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
            log_level: LogLevel::Debug,
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
        command.execute(self, state.clone()).await?;
        self.node_updates(state).await;
        self.updated_at = SystemTime::now();
        Ok(command)
    }

    pub async fn execute_commands(
        &mut self,
        commands: Vec<GenericCommand>,
        state: Arc<Mutex<FlowLikeState>>,
    ) -> flow_like_types::Result<Vec<GenericCommand>> {
        let mut commands = commands;
        for command in commands.iter_mut() {
            let res = command.execute(self, state.clone()).await;
            if let Err(e) = res {
                println!("Error executing command: {:?}", e);
            }
        }
        self.node_updates(state).await;
        self.updated_at = SystemTime::now();
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
        Ok(())
    }

    pub async fn redo(
        &mut self,
        commands: Vec<GenericCommand>,
        state: Arc<Mutex<FlowLikeState>>,
    ) -> flow_like_types::Result<()> {
        let mut commands = commands;
        for command in commands.iter_mut().rev() {
            command.execute(self, state.clone()).await?;
        }
        Ok(())
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

    pub fn fix_pins_set_layer(&mut self) {
        // Index all pins (node pins + layer pins) and where they live
        #[derive(Clone)]
        enum Owner {
            Node(String),
            Layer(String),
        }

        let default_layer = None::<String>; // None = not in any layer
        let mut pins: HashMap<String, &Pin> = HashMap::with_capacity(self.nodes.len() * 5);
        let mut pin_owner: HashMap<String, Owner> = HashMap::with_capacity(self.nodes.len() * 5);
        let mut pin_layer: HashMap<String, Option<String>> =
            HashMap::with_capacity(self.nodes.len() * 5);

        // Node pins
        for (node_id, node) in self.nodes.iter() {
            let node_layer = node.layer.clone();
            for pin in node.pins.values() {
                pins.insert(pin.id.clone(), pin);
                pin_owner.insert(pin.id.clone(), Owner::Node(node_id.clone()));
                pin_layer.insert(pin.id.clone(), node_layer.clone());
            }
        }

        // Layer pins
        for (layer_id, layer) in self.layers.iter() {
            for pin in layer.pins.values() {
                pins.insert(pin.id.clone(), pin);
                pin_owner.insert(pin.id.clone(), Owner::Layer(layer_id.clone()));
                pin_layer.insert(pin.id.clone(), Some(layer_id.clone()));
            }
        }

        // Newly created (empty) layers we should seed with bridge pins
        let mut empty_layers = HashSet::with_capacity(self.layers.len());
        for (layer_id, layer) in self.layers.iter() {
            if layer.pins.is_empty() {
                empty_layers.insert(layer_id.clone());
            }
        }

        // Collect removals for invalid/missing reciprocal links (node pins only)
        let mut node_pins_connected_to_remove: HashMap<String, HashMap<String, HashSet<String>>> =
            HashMap::new();
        let mut node_pins_depends_on_remove: HashMap<String, HashMap<String, HashSet<String>>> =
            HashMap::new();

        // Plan new layer bridge pins for empty layers:
        // (layer_id, base_pin_id) -> { outside_connected_to, outside_depends_on }
        #[derive(Default)]
        struct BridgePlan {
            outside_connected_to: HashSet<String>,
            outside_depends_on: HashSet<String>,
        }
        let mut bridge_plans: HashMap<(String, String), BridgePlan> = HashMap::new();

        // Scan node pins to validate links and to prepare bridge pins for empty layers
        for (node_id, node) in self.nodes.iter() {
            let node_layer = node.layer.clone();
            for pin in node.pins.values() {
                // Validate connected_to -> requires target.depends_on contains this pin
                for connected_to in &pin.connected_to {
                    if let Some(target_pin) = pins.get(connected_to) {
                        if !target_pin.depends_on.contains(&pin.id) {
                            node_pins_connected_to_remove
                                .entry(node_id.clone())
                                .or_default()
                                .entry(pin.id.clone())
                                .or_default()
                                .insert(connected_to.clone());
                        }

                        // If this node is inside an empty layer and targets a pin outside, plan a bridge
                        if let Some(layer_id) = node_layer.clone() {
                            if empty_layers.contains(&layer_id) {
                                let other_layer = pin_layer.get(connected_to).cloned().unwrap_or(default_layer.clone());
                                if other_layer != Some(layer_id.clone()) {
                                    let key = (layer_id.clone(), pin.id.clone());
                                    bridge_plans
                                        .entry(key)
                                        .or_default()
                                        .outside_connected_to
                                        .insert(connected_to.clone());
                                }
                            }
                        }
                    } else {
                        // Target pin does not exist -> remove
                        node_pins_connected_to_remove
                            .entry(node_id.clone())
                            .or_default()
                            .entry(pin.id.clone())
                            .or_default()
                            .insert(connected_to.clone());
                    }
                }

                // Validate depends_on -> requires target.connected_to contains this pin
                for depends_on in &pin.depends_on {
                    if let Some(target_pin) = pins.get(depends_on) {
                        if !target_pin.connected_to.contains(&pin.id) {
                            node_pins_depends_on_remove
                                .entry(node_id.clone())
                                .or_default()
                                .entry(pin.id.clone())
                                .or_default()
                                .insert(depends_on.clone());
                        }

                        // If this node is inside an empty layer and depends on a pin outside, plan a bridge
                        if let Some(layer_id) = node_layer.clone() {
                            if empty_layers.contains(&layer_id) {
                                let other_layer = pin_layer.get(depends_on).cloned().unwrap_or(default_layer.clone());
                                if other_layer != Some(layer_id.clone()) {
                                    let key = (layer_id.clone(), pin.id.clone());
                                    bridge_plans
                                        .entry(key)
                                        .or_default()
                                        .outside_depends_on
                                        .insert(depends_on.clone());
                                }
                            }
                        }
                    } else {
                        // Target pin does not exist -> remove
                        node_pins_depends_on_remove
                            .entry(node_id.clone())
                            .or_default()
                            .entry(pin.id.clone())
                            .or_default()
                            .insert(depends_on.clone());
                    }
                }
            }
        }

        // Prepare additions to ensure reciprocal links and bridge wiring
        let mut add_connected_to: HashMap<String, HashSet<String>> = HashMap::new();
        let mut add_depends_on: HashMap<String, HashSet<String>> = HashMap::new();

        // For each planned bridge, create a new layer pin (with new ID) and wire it
        // - Bridge pin depends_on the inside pin; inside pin connected_to the bridge
        // - Bridge pin mirrors outside references:
        //      * connected_to outside pins; outside pins depends_on bridge
        //      * depends_on outside pins; outside pins connected_to bridge
        // We only create for empty layers (as collected above).
        let mut new_layer_pins: HashMap<String, Vec<Pin>> = HashMap::new();
        for ((layer_id, base_pin_id), plan) in bridge_plans {
            // Ensure base pin still exists
            let Some(base_pin) = pins.get(&base_pin_id) else { continue };

            // Skip if layer is no longer empty (race within the same pass)
            if !empty_layers.contains(&layer_id) {
                continue;
            }

            let mut bridge_pin = (*base_pin).clone();
            bridge_pin.id = create_id();
            bridge_pin.connected_to.clear();
            bridge_pin.depends_on.clear();

            // Set index later upon insertion (after we can count by type)

            // Wire inside <-> bridge
            add_connected_to
                .entry(base_pin_id.clone())
                .or_default()
                .insert(bridge_pin.id.clone());
            add_depends_on
                .entry(bridge_pin.id.clone())
                .or_default()
                .insert(base_pin_id.clone());

            // Wire bridge -> outside (connected_to) and reciprocal
            for outside in plan.outside_connected_to {
                add_connected_to
                    .entry(bridge_pin.id.clone())
                    .or_default()
                    .insert(outside.clone());
                add_depends_on
                    .entry(outside.clone())
                    .or_default()
                    .insert(bridge_pin.id.clone());
            }

            // Wire bridge depends_on outside and reciprocal
            for outside in plan.outside_depends_on {
                add_depends_on
                    .entry(bridge_pin.id.clone())
                    .or_default()
                    .insert(outside.clone());
                add_connected_to
                    .entry(outside.clone())
                    .or_default()
                    .insert(bridge_pin.id.clone());
            }

            new_layer_pins.entry(layer_id.clone()).or_default().push(bridge_pin.clone());
        }

        // Insert newly created layer pins with proper indices
        for (layer_id, mut pins_to_add) in new_layer_pins {
            if let Some(layer) = self.layers.get_mut(&layer_id) {
                for mut p in pins_to_add.drain(..) {
                    let next_index = layer
                        .pins
                        .iter()
                        .filter(|(_, lp)| lp.pin_type == p.pin_type)
                        .count() as u16
                        + 1;
                    p.index = next_index;
                    // Record ownership for later additions
                    pin_owner.insert(p.id.clone(), Owner::Layer(layer_id.clone()));
                    pin_layer.insert(p.id.clone(), Some(layer_id.clone()));
                    layer.pins.insert(p.id.clone(), p);
                }
            }
        }

        // Apply additions to connected_to and depends_on (nodes and layers)
        let mut apply_additions = |map: HashMap<String, HashSet<String>>, is_connected_to: bool| {
            for (pin_id, targets) in map {
                let owner = match pin_owner.get(&pin_id) {
                    Some(o) => o.clone(),
                    None => continue,
                };
                match owner {
                    Owner::Node(node_id) => {
                        if let Some(node) = self.nodes.get_mut(&node_id) {
                            if let Some(pin) = node.pins.get_mut(&pin_id) {
                                if is_connected_to {
                                    for t in targets {
                                        pin.connected_to.insert(t);
                                    }
                                } else {
                                    for t in targets {
                                        pin.depends_on.insert(t);
                                    }
                                }
                            }
                        }
                    }
                    Owner::Layer(layer_id) => {
                        if let Some(layer) = self.layers.get_mut(&layer_id) {
                            if let Some(pin) = layer.pins.get_mut(&pin_id) {
                                if is_connected_to {
                                    for t in targets {
                                        pin.connected_to.insert(t);
                                    }
                                } else {
                                    for t in targets {
                                        pin.depends_on.insert(t);
                                    }
                                }
                            }
                        }
                    }
                }
            }
        };

        apply_additions(add_connected_to, true);
        apply_additions(add_depends_on, false);

        // Removals for invalid node pin links (layer pins are left untouched here)
        for (node_id, pins) in node_pins_connected_to_remove {
            if let Some(node) = self.nodes.get_mut(&node_id) {
                for (pin_id, to_remove) in pins {
                    if let Some(pin) = node.pins.get_mut(&pin_id) {
                        for connected_to in to_remove {
                            println!(
                                "Node Pins connected to remove: {} {} {}",
                                node_id, pin_id, connected_to
                            );
                            pin.connected_to.remove(&connected_to);
                        }
                    }
                }
            }
        }

        for (node_id, pins) in node_pins_depends_on_remove {
            if let Some(node) = self.nodes.get_mut(&node_id) {
                for (pin_id, to_remove) in pins {
                    if let Some(pin) = node.pins.get_mut(&pin_id) {
                        for depends_on in to_remove {
                            println!(
                                "Node Pins depends on remove: {} {} {}",
                                node_id, pin_id, depends_on
                            );
                            pin.depends_on.remove(&depends_on);
                        }
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
        board.fix_pins_set_layer();
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
        board.fix_pins_set_layer();
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
