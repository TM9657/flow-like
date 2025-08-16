use std::collections::{BTreeSet, HashMap, HashSet};

use flow_like_types::create_id;

use crate::flow::{
    board::{
        Board, Layer,
        cleanup::{BoardCleanupLogic, NodeOrLayer, NodeOrLayerRef, PinLookup},
    },
    pin::Pin,
};

#[derive(Default)]
struct BridgePlan {
    outside_connected_to: BTreeSet<String>,
    outside_depends_on: BTreeSet<String>,
}

#[derive(Default)]
pub struct BridgeLayersCleanup {
    empty_layers: HashSet<String>,
    pin_layer: HashMap<String, Option<String>>,
    bridge_plans: HashMap<(String, String), BridgePlan>,
}

impl BoardCleanupLogic for BridgeLayersCleanup {
    fn init(board: &mut Board) -> Self
    where
        Self: Sized,
    {
        Self {
            empty_layers: HashSet::with_capacity(10),
            pin_layer: HashMap::with_capacity((board.nodes.len() + board.layers.len()) * 4),
            bridge_plans: HashMap::with_capacity(10),
        }
    }

    fn initial_pin_iteration(&mut self, pin: &Pin, parent: NodeOrLayerRef) {
        match parent {
            NodeOrLayerRef::Node(node) => {
                self.pin_layer.insert(pin.id.clone(), node.layer.clone());
            }
            NodeOrLayerRef::Layer(layer) => {
                self.pin_layer
                    .insert(pin.id.clone(), Some(layer.id.clone()));
            }
        }
    }

    fn main_pin_iteration(&mut self, pin: &mut Pin, _pin_lookup: &PinLookup) {
        let layer = self.pin_layer.get(&pin.id).cloned().flatten();
        let layer_id = if let Some(layer_id) = &layer {
            layer_id.clone()
        } else {
            return;
        };

        if !self.empty_layers.contains(&layer_id) {
            return;
        }

        // This Pin is inside an empty Layer. Now let´s collect all connects to the outside layers and set them in a bridge plan!
        pin.connected_to.iter().for_each(|connected_to| {
            let connected_layer = self.pin_layer.get(connected_to).cloned().flatten();
            if connected_layer != Some(layer_id.clone()) {
                let key = (layer_id.clone(), pin.id.clone());
                let plan = self.bridge_plans.entry(key).or_default();
                plan.outside_connected_to.insert(connected_to.clone());
            }
        });

        pin.depends_on.iter().for_each(|depends_on| {
            let depends_on_layer = self.pin_layer.get(depends_on).cloned().flatten();
            if depends_on_layer != Some(layer_id.clone()) {
                let key = (layer_id.clone(), pin.id.clone());
                let plan = self.bridge_plans.entry(key).or_default();
                plan.outside_depends_on.insert(depends_on.clone());
            }
        });
    }

    fn initial_layer_iteration(&mut self, layer: &Layer) {
        if layer.pins.is_empty() {
            self.empty_layers.insert(layer.id.clone());
        }
    }

    fn post_process(&mut self, board: &mut Board, pin_lookup: &PinLookup) {
        for ((layer_id, layer_pin_id), plan) in self.bridge_plans.drain() {
            if !board.layers.contains_key(&layer_id) {
                tracing::warn!(
                    "Layer {} not found in board during bridge cleanup",
                    layer_id
                );
                continue;
            }

            if plan.outside_connected_to.is_empty() && plan.outside_depends_on.is_empty() {
                continue;
            }

            let Some(original_pin) = get_pin_mut(board, pin_lookup, &layer_pin_id) else {
                tracing::warn!(
                    "Pin {} not found in layer {} during bridge cleanup",
                    layer_pin_id,
                    layer_id
                );
                continue;
            };

            let original_pin_id = original_pin.id.clone();

            original_pin.connected_to.retain(|connected_to| {
                !plan.outside_connected_to.contains(connected_to)
            });

            original_pin.depends_on.retain(|depends_on| {
                !plan.outside_depends_on.contains(depends_on)
            });

            let bridge_pin_id = create_id();
            let mut bridge_pin = original_pin.clone();
            bridge_pin.id = bridge_pin_id.clone();
            bridge_pin.connected_to = plan.outside_connected_to.clone();
            bridge_pin.depends_on = plan.outside_depends_on.clone();

            if !bridge_pin.connected_to.is_empty() {
                original_pin.connected_to.insert(bridge_pin_id.clone());
                bridge_pin.depends_on.insert(original_pin.id.clone());
            }

            if !bridge_pin.depends_on.is_empty() {
                original_pin.depends_on.insert(bridge_pin.id.clone());
                bridge_pin.connected_to.insert(original_pin.id.clone());
            }

            let layer = if let Some(layer) = board.layers.get_mut(&layer_id) {
                layer
            } else {
                tracing::warn!(
                    "Layer {} not found in board during bridge cleanup",
                    layer_id
                );
                continue;
            };

            layer.pins.insert(bridge_pin_id.clone(), bridge_pin);

            for connected_pin in plan.outside_connected_to {
                let Some(pin) = get_pin_mut(board, pin_lookup, &connected_pin) else {
                    tracing::warn!(
                        "Connected Pin {} not found in pin lookup or board during bridge cleanup",
                        connected_pin
                    );
                    continue;
                };

                pin.depends_on.insert(bridge_pin_id.clone());
                pin.depends_on.remove(&original_pin_id);
            }

            for dep_pin in plan.outside_depends_on {
                let Some(pin) = get_pin_mut(board, pin_lookup, &dep_pin) else {
                    tracing::warn!(
                        "Dependent Pin {} not found in pin lookup or board during bridge cleanup",
                        dep_pin
                    );
                    continue;
                };

                pin.connected_to.insert(bridge_pin_id.clone());
                pin.connected_to.remove(&original_pin_id);
            }
        }
    }
}

fn get_pin_mut<'a>(
    board: &'a mut Board,
    pin_lookup: &PinLookup,
    pin_id: &str,
) -> Option<&'a mut Pin> {
    match pin_lookup.get(pin_id) {
        Some((pin_meta, parent)) => match parent {
            NodeOrLayer::Node(_) => board
                .nodes
                .get_mut(parent.id())
                .and_then(|n| n.pins.get_mut(&pin_meta.id)),
            NodeOrLayer::Layer(_) => board
                .layers
                .get_mut(parent.id())
                .and_then(|l| l.pins.get_mut(&pin_meta.id)),
        },
        None => None,
    }
}
