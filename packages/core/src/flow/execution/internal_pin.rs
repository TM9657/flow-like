use flow_like_types::{sync::Mutex, Value};
use std::{
    collections::HashSet,
    sync::{Arc, Weak},
};

use crate::flow::pin::Pin;

use super::internal_node::InternalNode;

pub struct InternalPin {
    pub pin: Arc<Mutex<Pin>>,
    pub node: Weak<InternalNode>,
    pub connected_to: Vec<Weak<Mutex<InternalPin>>>,
    pub depends_on: Vec<Weak<Mutex<InternalPin>>>,
}

impl InternalPin {
    pub async fn reset(&mut self) {
        let mut pin = self.pin.lock().await;
        pin.value = None;
    }

    pub async fn get_connected_and_dependent_nodes(&self) -> Vec<Arc<InternalNode>> {
        let mut connected = self.get_connected_nodes().await;
        let dependent = self.get_dependent_nodes().await;

        connected.extend(dependent);
        connected
    }

    pub async fn get_connected_nodes(&self) -> Vec<Arc<InternalNode>> {
        let mut connected_nodes = vec![];
        let mut ids = HashSet::new();
        for connected_pin in &self.connected_to {
            let Some(connected_pin) = connected_pin.upgrade() else {
                continue;
            };
            let connected_pin_guard = connected_pin.lock().await;
            let connected_node = connected_pin_guard.node.upgrade();

            if let Some(connected_node) = connected_node {
                let connected_id = connected_node.node.lock().await.id.clone();
                if ids.contains(&connected_id) {
                    continue;
                }

                ids.insert(connected_id);
                connected_nodes.push(connected_node);
            }
        }

        connected_nodes
    }

    pub async fn get_dependent_nodes(&self) -> Vec<Arc<InternalNode>> {
        let mut connected_nodes = vec![];

        for depends_on_pin in &self.depends_on {
            let Some(depends_on_pin) = depends_on_pin.upgrade() else {
                continue;
            };
            let depends_on_pin_guard = depends_on_pin.lock().await;
            let depends_on_node = depends_on_pin_guard.node.upgrade();

            if let Some(depends_on_node) = depends_on_node {
                connected_nodes.push(depends_on_node);
            }
        }

        connected_nodes
    }

    pub async fn set_value(&self, value: Value) {
        let mut pin = self.pin.lock().await;
        pin.value = Some(Arc::new(Mutex::new(value)));
    }

    pub async fn set_pin_by_ref(&self, value: Arc<Mutex<Value>>) {
        let mut pin = self.pin.lock().await;
        pin.value = Some(value.clone());
    }

    pub async fn is_pure(&self) -> bool {
        if let Some(internal_node) = self.node.upgrade() {
            return internal_node.is_pure().await;
        }
        false
    }
}
