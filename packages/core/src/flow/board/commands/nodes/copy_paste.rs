use std::{collections::HashMap, sync::Arc};

use crate::{
    flow::{
        board::{Board, Comment, Layer, commands::Command},
        node::Node,
        pin::PinType,
        variable::Variable,
    },
    state::FlowLikeState,
};
use flow_like_types::{async_trait, sync::Mutex};
use flow_like_types::{create_id, json::from_slice};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

#[derive(Clone, Serialize, Deserialize, JsonSchema)]
pub struct CopyPasteCommand {
    pub original_nodes: Vec<Node>,
    pub original_comments: Vec<Comment>,
    pub original_layers: Vec<Layer>,
    pub new_comments: Vec<Comment>,
    pub new_nodes: Vec<Node>,
    pub new_layers: Vec<Layer>,
    pub current_layer: Option<String>,
    pub old_mouse: Option<(f32, f32, f32)>,
    pub offset: (f32, f32, f32),
}

impl CopyPasteCommand {
    pub fn new(
        original_nodes: Vec<Node>,
        comments: Vec<Comment>,
        layers: Vec<Layer>,
        offset: (f32, f32, f32),
    ) -> Self {
        CopyPasteCommand {
            original_nodes,
            original_comments: comments,
            original_layers: layers,
            old_mouse: None,
            current_layer: None,
            offset,
            new_nodes: vec![],
            new_comments: vec![],
            new_layers: vec![],
        }
    }
}

#[async_trait]
impl Command for CopyPasteCommand {
    async fn execute(
        &mut self,
        board: &mut Board,
        state: Arc<Mutex<FlowLikeState>>,
    ) -> flow_like_types::Result<()> {
        let node_registry = {
            let state_guard = state.lock().await;
            state_guard.node_registry.read().await.node_registry.clone()
        };
        let mut translated_connection = HashMap::new();
        let mut intermediate_nodes = Vec::with_capacity(self.original_nodes.len());
        let offset = self.offset;
        let offset = self
            .original_comments
            .first()
            .map(|comment| {
                let old_coors = comment.coordinates;
                (
                    offset.0 - old_coors.0,
                    offset.1 - old_coors.1,
                    offset.2 - old_coors.2,
                )
            })
            .unwrap_or(offset);
        let mut offset = self
            .original_nodes
            .first()
            .map(|node| {
                let old_coors = node.coordinates.unwrap_or((0.0, 0.0, 0.0));
                (
                    offset.0 - old_coors.0,
                    offset.1 - old_coors.1,
                    offset.2 - old_coors.2,
                )
            })
            .unwrap_or(offset);

        if let Some(old_mouse) = self.old_mouse {
            offset = (
                self.offset.0 - old_mouse.0,
                self.offset.1 - old_mouse.1,
                self.offset.2 - old_mouse.2,
            );
        }

        for comment in self.original_comments.iter() {
            let mut new_comment = comment.clone();
            new_comment.id = create_id();
            new_comment.coordinates = (
                new_comment.coordinates.0 + offset.0,
                new_comment.coordinates.1 + offset.1,
                new_comment.coordinates.2 + offset.2,
            );

            if new_comment.layer.is_none() || new_comment.layer == Some("".to_string()) {
                new_comment.layer = self.current_layer.clone();
            }

            board
                .comments
                .insert(new_comment.id.clone(), new_comment.clone());
            self.new_comments.push(new_comment);
        }

        for layer in self.original_layers.iter() {
            if board.layers.contains_key(&layer.id) {
                continue;
            }
            let mut new_layer = layer.clone();
            new_layer.coordinates = (
                new_layer.coordinates.0 + offset.0,
                new_layer.coordinates.1 + offset.1,
                new_layer.coordinates.2 + offset.2,
            );

            if new_layer.parent_id.is_none() || new_layer.parent_id == Some("".to_string()) {
                new_layer.parent_id = self.current_layer.clone();
            }

            board.layers.insert(new_layer.id.clone(), new_layer.clone());
            self.new_layers.push(new_layer);
        }

        for node in self.original_nodes.iter() {
            let mut new_node = node.clone();
            let blueprint_node = node_registry
                .get_node(&node.name)
                .ok()
                .unwrap_or(node.clone());
            let old_id = new_node.id.clone();
            let new_id = create_id();
            translated_connection.insert(old_id, new_id.clone());
            new_node.id = new_id.clone();
            new_node.category = blueprint_node.category.clone();
            new_node.docs = blueprint_node.docs.clone();
            new_node.description = blueprint_node.description.clone();
            new_node.icon = blueprint_node.icon.clone();
            new_node.scores = blueprint_node.scores.clone();
            new_node.start = blueprint_node.start;
            new_node.event_callback = blueprint_node.event_callback;
            new_node.coordinates = Some((
                new_node.coordinates.unwrap_or((0.0, 0.0, 0.0)).0 + offset.0,
                new_node.coordinates.unwrap_or((0.0, 0.0, 0.0)).1 + offset.1,
                new_node.coordinates.unwrap_or((0.0, 0.0, 0.0)).2 + offset.2,
            ));

            if new_node.layer.is_none() || new_node.layer == Some("".to_string()) {
                new_node.layer = self.current_layer.clone();
            }

            new_node.pins = new_node
                .pins
                .values()
                .map(|pin| {
                    let mut pin = pin.clone();
                    let old_pin_id = pin.id.clone();
                    let (_, blueprint_pin) = blueprint_node
                        .pins
                        .iter()
                        .find(|(_, p)| p.name == pin.name && pin.pin_type == p.pin_type)
                        .unwrap_or((&String::new(), &pin));
                    let blueprint_pin = blueprint_pin.clone();
                    let new_pin_id = create_id();
                    translated_connection.insert(old_pin_id, new_pin_id.clone());
                    pin.id = new_pin_id.clone();
                    pin.description = blueprint_pin.description.clone();

                    if pin.name == "var_ref" {
                        if let Some(var_ref) = pin.default_value.as_ref() {
                            let var_ref = from_slice::<String>(var_ref);
                            if let Ok(var_ref) = var_ref {
                                let variable_ref = board.variables.get(&var_ref);
                                if variable_ref.is_none() {
                                    let var_name = new_node.friendly_name.replace("Get ", "");
                                    println!(
                                        "Creating new variable: {}, friendly name: {}",
                                        var_name, new_node.friendly_name
                                    );
                                    let mut new_var = Variable::new(
                                        &var_name,
                                        pin.data_type.clone(),
                                        pin.value_type.clone(),
                                    );
                                    new_var.id = var_ref.clone();
                                    board.variables.insert(var_ref.clone(), new_var);
                                }
                            }
                        }
                    }

                    pin.schema = blueprint_pin.schema.clone();
                    pin.options = blueprint_pin.options.clone();

                    if new_node.start.unwrap_or(false)
                        && pin.pin_type == PinType::Input
                        && pin.name != "type"
                    {
                        pin.default_value = None;
                    }

                    (new_pin_id, pin)
                })
                .collect();

            new_node.friendly_name = blueprint_node.friendly_name.clone();
            intermediate_nodes.push(new_node);
        }

        for node in intermediate_nodes.iter() {
            let mut new_node = node.clone();
            for pin in new_node.pins.values_mut() {
                pin.depends_on = pin
                    .depends_on
                    .iter()
                    .filter(|dep_id| translated_connection.contains_key(*dep_id))
                    .map(|dep_id| translated_connection.get(dep_id).unwrap_or(dep_id).clone())
                    .collect();

                pin.connected_to = pin
                    .connected_to
                    .iter()
                    .filter(|dep_id| translated_connection.contains_key(*dep_id))
                    .map(|dep_id| translated_connection.get(dep_id).unwrap_or(dep_id).clone())
                    .collect();
            }

            board.nodes.insert(new_node.id.clone(), new_node.clone());
            self.new_nodes.push(new_node);
        }

        board.fix_pins_set_layer();

        Ok(())
    }

    async fn undo(
        &mut self,
        board: &mut Board,
        _: Arc<Mutex<FlowLikeState>>,
    ) -> flow_like_types::Result<()> {
        for node in self.new_nodes.iter() {
            board.nodes.remove(&node.id);
        }

        for comment in self.new_comments.iter() {
            board.comments.remove(&comment.id);
        }

        for layer in self.new_layers.iter() {
            board.layers.remove(&layer.id);
        }

        board.fix_pins_set_layer();
        Ok(())
    }
}
