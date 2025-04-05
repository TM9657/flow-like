use flow_like_types::{async_trait, sync::Mutex};
use schemars::JsonSchema;
use std::collections::HashSet;
use std::sync::Arc;


use serde::{Deserialize, Serialize};

use crate::{
    flow::{
        board::{Board, commands::Command},
        node::Node,
    },
    state::FlowLikeState,
};

#[derive(Serialize, Deserialize, Debug, Clone, JsonSchema)]
pub struct RemoveNodeCommand {
    pub node: Node,
    pub connected_nodes: Vec<Node>,
}

impl RemoveNodeCommand {
    pub fn new(node: Node) -> Self {
        RemoveNodeCommand {
            node,
            connected_nodes: vec![],
        }
    }
}

#[async_trait]
impl Command for RemoveNodeCommand {
    async fn execute(
        &mut self,
        board: &mut Board,
        _state: Arc<Mutex<FlowLikeState>>,
    ) -> flow_like_types::Result<()> {
        let node = match board.nodes.get(&self.node.id) {
            Some(node) => node,
            None => return Err(flow_like_types::anyhow!("Node not found".to_string())),
        };

        let mut connected_pins = HashSet::new();
        let mut node_pins = HashSet::new();

        node.pins.iter().for_each(|(pin_id, pin)| {
            node_pins.insert(pin_id);
            pin.connected_to.iter().for_each(|connected_pin_id| {
                connected_pins.insert(connected_pin_id.clone());
            });

            pin.depends_on.iter().for_each(|depends_on_pin_id| {
                connected_pins.insert(depends_on_pin_id.clone());
            });
        });

        let mut connected_nodes = vec![];
        let mut changed_nodes = vec![];

        for (node_id, other) in &board.nodes {
            if node_id == &self.node.id {
                continue;
            }

            for pin in other.pins.values() {
                if connected_pins.contains(&pin.id) {
                    connected_nodes.push(other.clone());

                    let mut cloned = other.clone();
                    cloned.pins.iter_mut().for_each(|(_pin_id, pin)| {
                        pin.connected_to = pin
                            .connected_to
                            .iter()
                            .filter(|connected_pin_id| !node_pins.contains(connected_pin_id))
                            .cloned()
                            .collect();
                        pin.depends_on = pin
                            .depends_on
                            .iter()
                            .filter(|depends_on_pin_id| !node_pins.contains(depends_on_pin_id))
                            .cloned()
                            .collect();
                    });
                    changed_nodes.push(cloned);

                    break;
                }
            }
        }

        self.connected_nodes = connected_nodes;
        board.nodes.remove(&self.node.id);

        for node in &changed_nodes {
            board.nodes.insert(node.id.clone(), node.clone());
        }

        board.fix_pins();

        Ok(())
    }

    async fn undo(
        &mut self,
        board: &mut Board,
        _state: Arc<Mutex<FlowLikeState>>,
    ) -> flow_like_types::Result<()> {
        board.nodes.insert(self.node.id.clone(), self.node.clone());

        for node in &self.connected_nodes {
            board.nodes.insert(node.id.clone(), node.clone());
        }

        board.fix_pins();

        Ok(())
    }
}
