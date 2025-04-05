use std::{collections::HashMap, sync::Arc};

use crate::{
    flow::{
        board::{Board, Comment, commands::Command},
        node::Node,
    },
    state::FlowLikeState,
};
use flow_like_types::create_id;
use flow_like_types::{async_trait, sync::Mutex};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

#[derive(Clone, Serialize, Deserialize, JsonSchema)]
pub struct CopyPasteCommand {
    pub original_nodes: Vec<Node>,
    pub original_comments: Vec<Comment>,
    pub new_comments: Vec<Comment>,
    pub new_nodes: Vec<Node>,
    pub offset: (f32, f32, f32),
}

impl CopyPasteCommand {
    pub fn new(original_nodes: Vec<Node>, comments: Vec<Comment>, offset: (f32, f32, f32)) -> Self {
        CopyPasteCommand {
            original_nodes: original_nodes.clone(),
            original_comments: comments.clone(),
            offset,
            new_nodes: vec![],
            new_comments: vec![],
        }
    }
}

#[async_trait]
impl Command for CopyPasteCommand {
    async fn execute(
        &mut self,
        board: &mut Board,
        _state: Arc<Mutex<FlowLikeState>>,
    ) -> flow_like_types::Result<()> {
        let mut translated_connection = HashMap::new();
        let mut intermediate_nodes = Vec::with_capacity(self.original_nodes.len());
        let offset = self.offset;
        let offset = self
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

        for comment in self.original_comments.iter() {
            let mut new_comment = comment.clone();
            new_comment.id = create_id();
            new_comment.coordinates = (
                new_comment.coordinates.0 + self.offset.0,
                new_comment.coordinates.1 + self.offset.1,
                new_comment.coordinates.2 + self.offset.2,
            );
            board
                .comments
                .insert(new_comment.id.clone(), new_comment.clone());
            self.new_comments.push(new_comment);
        }

        for node in self.original_nodes.iter() {
            let mut new_node = node.clone();
            let old_id = new_node.id.clone();
            let new_id = create_id();
            translated_connection.insert(old_id, new_id.clone());
            new_node.id = new_id.clone();

            new_node.coordinates = Some((
                new_node.coordinates.unwrap_or((0.0, 0.0, 0.0)).0 + offset.0,
                new_node.coordinates.unwrap_or((0.0, 0.0, 0.0)).1 + offset.1,
                new_node.coordinates.unwrap_or((0.0, 0.0, 0.0)).2 + offset.2,
            ));

            new_node.pins = new_node
                .pins
                .values()
                .map(|pin| {
                    let mut pin = pin.clone();
                    let old_pin_id = pin.id.clone();
                    let new_pin_id = create_id();
                    translated_connection.insert(old_pin_id, new_pin_id.clone());
                    pin.id = new_pin_id.clone();
                    (new_pin_id, pin)
                })
                .collect();

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

        board.fix_pins();

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
        board.fix_pins();
        Ok(())
    }
}
