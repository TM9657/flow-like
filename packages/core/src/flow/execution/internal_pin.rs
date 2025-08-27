use flow_like_types::{Value, json::from_value, sync::Mutex};
use serde::de::DeserializeOwned;
use std::{
    collections::HashSet,
    sync::{Arc, Weak},
};

use crate::flow::pin::Pin;

use super::internal_node::InternalNode;

pub struct InternalPin {
    pub pin: Arc<Mutex<Pin>>,
    pub node: Option<Weak<InternalNode>>,
    pub connected_to: Vec<Weak<Mutex<InternalPin>>>,
    pub depends_on: Vec<Weak<Mutex<InternalPin>>>,
    pub layer_pin: bool,
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
        let mut result = Vec::new();
        let mut node_ids = HashSet::new();
        let mut visited_pins: HashSet<usize> = HashSet::new();
        let mut stack: Vec<Arc<Mutex<InternalPin>>> = Vec::new();

        for weak in &self.connected_to {
            if let Some(p) = weak.upgrade() {
                stack.push(p);
            }
        }

        while let Some(pin_arc) = stack.pop() {
            let pin_key = Arc::as_ptr(&pin_arc) as usize;
            if !visited_pins.insert(pin_key) {
                continue;
            }

            let node_opt = {
                let guard = pin_arc.lock().await;
                if let Some(node_weak) = &guard.node {
                    node_weak.upgrade()
                } else {
                    for next in &guard.connected_to {
                        if let Some(next_arc) = next.upgrade() {
                            stack.push(next_arc);
                        }
                    }
                    None
                }
            };

            if let Some(node_arc) = node_opt {
                let id = node_arc.node.lock().await.id.clone();
                if node_ids.insert(id) {
                    result.push(node_arc);
                }
            }
        }

        result
    }

    pub async fn get_dependent_nodes(&self) -> Vec<Arc<InternalNode>> {
        let seed = self.depends_on.len();

        let mut result = Vec::with_capacity(seed);
        let mut node_ids = HashSet::with_capacity(seed.saturating_mul(2));
        let mut visited_pins: HashSet<usize> = HashSet::with_capacity(seed.saturating_mul(4));
        let mut stack: Vec<Arc<Mutex<InternalPin>>> = Vec::with_capacity(seed);

        for weak in &self.depends_on {
            if let Some(p) = weak.upgrade() {
                stack.push(p);
            }
        }

        while let Some(pin_arc) = stack.pop() {
            let pin_key = Arc::as_ptr(&pin_arc) as usize;
            if !visited_pins.insert(pin_key) {
                continue;
            }

            let node_opt = {
                let guard = pin_arc.lock().await;
                if let Some(node_weak) = &guard.node {
                    node_weak.upgrade()
                } else {
                    for next in &guard.depends_on {
                        if let Some(next_arc) = next.upgrade() {
                            stack.push(next_arc);
                        }
                    }
                    None
                }
            };

            if let Some(node_arc) = node_opt {
                let id = node_arc.node.lock().await.id.clone();
                if node_ids.insert(id) {
                    result.push(node_arc);
                }
            }
        }

        result
    }

    pub async fn set_value(&self, value: Value) {
        let mut pin = self.pin.lock().await;
        pin.value = Some(Arc::new(Mutex::new(value)));
    }

    pub async fn get_value<T: DeserializeOwned>(&self) -> Option<T> {
        let pin = self.pin.lock().await;
        let value = pin.value.as_ref()?;
        let value = value.lock().await;
        from_value::<T>(value.clone()).ok()
    }

    pub async fn set_pin_by_ref(&self, value: Arc<Mutex<Value>>) {
        let mut pin = self.pin.lock().await;
        pin.value = Some(value.clone());
    }

    // Pins without a parent report as pure!
    pub async fn is_pure(&self) -> bool {
        if let Some(node) = &self.node {
            if let Some(internal_node) = node.upgrade() {
                return internal_node.is_pure().await;
            } else {
                return false;
            }
        }
        true
    }
}
