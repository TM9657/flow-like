use serde::{Deserialize, Serialize};

use crate::flow::board::{Board, LayerType};
use crate::flow::node::Node;
use crate::flow::pin::{Pin, PinType};
use flow_like_types::Result;

/// Compact node representation for context
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct NodeContext {
    pub id: String,
    #[serde(rename = "t")] // "type" abbreviated
    pub node_type: String,
    #[serde(rename = "n")] // "name" abbreviated
    pub friendly_name: String,
    #[serde(rename = "i")] // "inputs" abbreviated
    pub inputs: Vec<PinContext>,
    #[serde(rename = "o")] // "outputs" abbreviated
    pub outputs: Vec<PinContext>,
    #[serde(rename = "p")] // "position" abbreviated
    pub position: (i32, i32),
    #[serde(rename = "s")] // "size" abbreviated
    pub estimated_size: (u16, u16),
}

/// Compact pin representation
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct PinContext {
    #[serde(rename = "n")] // "name" abbreviated
    pub name: String,
    #[serde(rename = "t")] // "type" abbreviated
    pub type_name: String,
    /// Only included if pin has a non-empty default value
    #[serde(rename = "v", skip_serializing_if = "Option::is_none")] // "value" abbreviated
    pub default_value: Option<String>,
}

/// Compact edge representation
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct EdgeContext {
    #[serde(rename = "f")] // "from" abbreviated
    pub from_node_id: String,
    #[serde(rename = "fp")] // "from_pin" abbreviated
    pub from_pin_name: String,
    #[serde(rename = "t")] // "to" abbreviated
    pub to_node_id: String,
    #[serde(rename = "tp")] // "to_pin" abbreviated
    pub to_pin_name: String,
}

/// Result-cache settings attached to a Function layer.
///
/// The persisted board calls the grouping value a `prefix`; FlowScript and FlowPilot expose the
/// behavior-oriented name `namespace`, matching the cache invalidation surface users interact
/// with. Keeping this as a small, owned context type avoids leaking board serialization details
/// into the provider-neutral manifest.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct LayerCacheContext {
    pub enabled: bool,
    pub namespace: String,
    /// `None` means permanent on persisted boards. When re-authoring it, FlowPilot must use
    /// explicit zero because omission on a new cache object now defaults to 300 seconds.
    pub ttl_seconds: Option<u64>,
    /// `app` for a shared result or `user` for a result private to the triggering user.
    pub scope: String,
}

/// Compact layer representation for context
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct LayerContext {
    pub id: String,
    #[serde(rename = "n")] // "name" abbreviated
    pub name: String,
    /// Layer kind (`Function`, `Macro`, or `Collapsed`). Kept compact in provider context.
    #[serde(rename = "t", default)]
    pub layer_type: String,
    /// Parent layer ID if nested, None if at root
    #[serde(rename = "p", skip_serializing_if = "Option::is_none")]
    pub parent_id: Option<String>,
    /// Node IDs contained in this layer
    #[serde(rename = "nodes")]
    pub node_ids: Vec<String>,
    #[serde(rename = "pos")] // "position" abbreviated
    pub position: (i32, i32),
    /// Input pins (for connecting TO this layer)
    #[serde(rename = "i", skip_serializing_if = "Vec::is_empty")]
    pub inputs: Vec<PinContext>,
    /// Output pins (for connecting FROM this layer)
    #[serde(rename = "o", skip_serializing_if = "Vec::is_empty")]
    pub outputs: Vec<PinContext>,
    /// Function result-cache settings. `None` means the function is not cache-configured.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub cache: Option<LayerCacheContext>,
}

/// Compact variable representation for context
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct VariableContext {
    pub id: String,
    #[serde(rename = "n")] // "name" abbreviated
    pub name: String,
    #[serde(rename = "t")] // "type" abbreviated
    pub data_type: String,
    #[serde(rename = "vt")] // "value_type" abbreviated
    pub value_type: String,
    #[serde(rename = "c", skip_serializing_if = "Option::is_none")] // "category" abbreviated
    pub category: Option<String>,
    #[serde(rename = "v", skip_serializing_if = "Option::is_none")] // "value" abbreviated
    pub default_value: Option<String>,
}

/// Complete graph context for the LLM
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct GraphContext {
    pub nodes: Vec<NodeContext>,
    pub edges: Vec<EdgeContext>,
    /// All layers in the board with their hierarchy
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub layers: Vec<LayerContext>,
    /// All variables defined in the board
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub variables: Vec<VariableContext>,
    pub selected_nodes: Vec<String>,
}

/// Spatial facts for one node that the FlowScript render cannot express.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct NodeLayoutContext {
    #[serde(rename = "p")]
    pub position: (i32, i32),
    #[serde(rename = "s")]
    pub estimated_size: (u16, u16),
    /// Containing layer ID, absent for root-level nodes
    #[serde(rename = "l", skip_serializing_if = "Option::is_none")]
    pub layer_id: Option<String>,
}

/// Layout-only board context for authoring prompts where the anchored FlowScript render already
/// carries every node, pin, default, and edge. Only position/size/layer membership survive here.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct BoardLayoutContext {
    pub nodes: std::collections::BTreeMap<String, NodeLayoutContext>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub layers: Vec<LayerContext>,
    pub selected_nodes: Vec<String>,
}

fn estimate_node_size(node: &Node) -> (u16, u16) {
    let input_count = node
        .pins
        .values()
        .filter(|p| p.pin_type == PinType::Input)
        .count();
    let output_count = node
        .pins
        .values()
        .filter(|p| p.pin_type == PinType::Output)
        .count();
    let max_pins = input_count.max(output_count);
    (200u16, 32u16 + (max_pins as u16 * 20))
}

fn node_position(node: &Node) -> (i32, i32) {
    node.coordinates
        .map(|(x, y, _)| (x as i32, y as i32))
        .unwrap_or((0, 0))
}

fn build_layer_contexts(board: &Board) -> Vec<LayerContext> {
    board
        .layers
        .values()
        .map(|layer| {
            let (x, y) = (layer.coordinates.0 as i32, layer.coordinates.1 as i32);

            let inputs: Vec<PinContext> = layer
                .pins
                .values()
                .filter(|p| p.pin_type == PinType::Input)
                .map(|p| PinContext {
                    name: p.name.clone(),
                    type_name: format!("{:?}", p.data_type),
                    default_value: None,
                })
                .collect();

            let outputs: Vec<PinContext> = layer
                .pins
                .values()
                .filter(|p| p.pin_type == PinType::Output)
                .map(|p| PinContext {
                    name: p.name.clone(),
                    type_name: format!("{:?}", p.data_type),
                    default_value: None,
                })
                .collect();

            LayerContext {
                id: layer.id.clone(),
                name: layer.name.clone(),
                layer_type: match &layer.r#type {
                    LayerType::Function => "Function",
                    LayerType::Macro => "Macro",
                    LayerType::Collapsed => "Collapsed",
                    LayerType::Module => "Module",
                }
                .to_string(),
                parent_id: layer.parent_id.clone(),
                node_ids: board
                    .nodes
                    .values()
                    .filter(|node| node.layer.as_deref() == Some(layer.id.as_str()))
                    .map(|node| node.id.clone())
                    .chain(layer.nodes.keys().cloned())
                    .collect::<std::collections::BTreeSet<_>>()
                    .into_iter()
                    .collect(),
                position: (x, y),
                inputs,
                outputs,
                cache: layer.cache.as_ref().map(|cache| LayerCacheContext {
                    enabled: cache.enabled,
                    namespace: cache.prefix.clone(),
                    ttl_seconds: cache.ttl_seconds,
                    scope: cache.scope.as_str().to_string(),
                }),
            }
        })
        .collect()
}

/// Prepare the layout-only context embedded in the authoring prompt next to the FlowScript render.
pub fn prepare_layout_context(board: &Board, selected_node_ids: &[String]) -> BoardLayoutContext {
    let mut nodes = std::collections::BTreeMap::new();
    let mut insert_nodes = |source: &std::collections::HashMap<String, Node>,
                            layer_id: Option<&str>| {
        for node in source.values() {
            nodes.insert(
                node.id.clone(),
                NodeLayoutContext {
                    position: node_position(node),
                    estimated_size: estimate_node_size(node),
                    layer_id: node.layer.as_deref().or(layer_id).map(str::to_string),
                },
            );
        }
    };
    insert_nodes(&board.nodes, None);
    for layer in board.layers.values() {
        insert_nodes(&layer.nodes, Some(&layer.id));
    }

    BoardLayoutContext {
        nodes,
        layers: build_layer_contexts(board),
        selected_nodes: selected_node_ids.to_vec(),
    }
}

/// Prepare graph context from a board
pub fn prepare_context(board: &Board, selected_node_ids: &[String]) -> Result<GraphContext> {
    let mut node_contexts = Vec::new();

    // Helper to create context
    let mut create_node_contexts = |nodes: &std::collections::HashMap<String, Node>| {
        for node in nodes.values() {
            // Only include non-execution pins with meaningful info
            let inputs: Vec<PinContext> = node
                .pins
                .iter()
                .filter(|(_, p)| p.pin_type == PinType::Input)
                .map(|(_, p)| {
                    let default_val = p
                        .default_value
                        .as_ref()
                        .map(|v| String::from_utf8_lossy(v).to_string())
                        .filter(|s| !s.is_empty() && s != "null");
                    PinContext {
                        name: p.name.clone(),
                        type_name: format!("{:?}", p.data_type),
                        default_value: default_val,
                    }
                })
                .collect();

            let outputs: Vec<PinContext> = node
                .pins
                .iter()
                .filter(|(_, p)| p.pin_type == PinType::Output)
                .map(|(_, p)| PinContext {
                    name: p.name.clone(),
                    type_name: format!("{:?}", p.data_type),
                    default_value: None, // Outputs don't have default values
                })
                .collect();

            node_contexts.push(NodeContext {
                id: node.id.clone(),
                node_type: node.name.clone(),
                friendly_name: node.friendly_name.clone(),
                position: node_position(node),
                estimated_size: estimate_node_size(node),
                inputs,
                outputs,
            });
        }
    };

    create_node_contexts(&board.nodes);
    for layer in board.layers.values() {
        create_node_contexts(&layer.nodes);
    }

    let edge_contexts = prepare_edge_contexts(board);

    let layer_contexts = build_layer_contexts(board);

    // Build variable contexts
    let variable_contexts: Vec<VariableContext> = board
        .variables
        .values()
        .map(|var| {
            let default_val = var
                .default_value
                .as_ref()
                .map(|v| String::from_utf8_lossy(v).to_string())
                .filter(|s| !s.is_empty() && s != "null");
            VariableContext {
                id: var.id.clone(),
                name: var.name.clone(),
                data_type: format!("{:?}", var.data_type),
                value_type: format!("{:?}", var.value_type),
                category: var.category.clone(),
                default_value: default_val,
            }
        })
        .collect();

    Ok(GraphContext {
        nodes: node_contexts,
        edges: edge_contexts,
        layers: layer_contexts,
        variables: variable_contexts,
        selected_nodes: selected_node_ids.to_vec(),
    })
}

fn prepare_edge_contexts(board: &Board) -> Vec<EdgeContext> {
    let mut endpoints: std::collections::HashMap<&str, (&str, &Pin)> =
        std::collections::HashMap::new();
    for node in board
        .nodes
        .values()
        .chain(board.layers.values().flat_map(|layer| layer.nodes.values()))
    {
        for pin in node.pins.values() {
            endpoints.insert(&pin.id, (&node.id, pin));
        }
    }
    for layer in board.layers.values() {
        for pin in layer.pins.values() {
            endpoints.insert(&pin.id, (&layer.id, pin));
        }
    }

    // Function inputs feed body nodes and function outputs receive their returns. Their stored
    // directions describe the boundary, so filtering sources to Output pins loses valid edges.
    // Both persisted edge halves are read, as in FlowScript lowering. Unknown endpoints are omitted.
    let mut connections = std::collections::BTreeSet::new();
    for (pin_id, (_, pin)) in &endpoints {
        for target in &pin.connected_to {
            if endpoints.contains_key(target.as_str()) {
                connections.insert((*pin_id, target.as_str()));
            }
        }
        for source in &pin.depends_on {
            if endpoints.contains_key(source.as_str()) {
                connections.insert((source.as_str(), *pin_id));
            }
        }
    }
    connections
        .into_iter()
        .map(|(from, to)| {
            let (from_owner, from_pin) = endpoints[from];
            let (to_owner, to_pin) = endpoints[to];
            EdgeContext {
                from_node_id: from_owner.to_string(),
                from_pin_name: from_pin.name.clone(),
                to_node_id: to_owner.to_string(),
                to_pin_name: to_pin.name.clone(),
            }
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use super::*;
    use crate::flow::ast::apply_flowscript_to_board;
    use crate::flow::board::{Layer, LayerCache, LayerCacheScope, LayerType};
    use crate::flow::copilot::tools::{build_node_details_output, build_unconfigured_nodes_output};
    use crate::flow::pin::ValueType;
    use crate::flow::variable::VariableType;
    use crate::state::{FlowLikeConfig, FlowLikeState};
    use crate::utils::http::HTTPClient;
    use flow_like_storage::Path;

    fn layout_fixture_board() -> (Board, String, String, String) {
        let mut board = Board::new_detached(Some("layout-context".to_string()), Path::default());

        let mut fetch = Node::new("http_fetch", "Fetch Data", "", "web");
        fetch.add_input_pin("exec_in", "Exec In", "", VariableType::Execution);
        fetch
            .add_input_pin("url", "Url", "", VariableType::String)
            .default_value = Some(b"\"https://example.com/some/long/default/path\"".to_vec());
        fetch.add_output_pin("exec_out", "Exec Out", "", VariableType::Execution);
        fetch.add_output_pin("body", "Body", "", VariableType::String);
        fetch.coordinates = Some((120.0, -40.0, 0.0));
        let fetch_id = fetch.id.clone();

        let mut layer = Layer::new(
            "pricing-layer".to_string(),
            "calculatePricing".to_string(),
            LayerType::Function,
        );
        let mut log = Node::new("log_info", "Log", "", "logging");
        log.add_input_pin("exec_in", "Exec In", "", VariableType::Execution);
        let log_id = log.id.clone();
        layer.nodes.insert(log.id.clone(), log);
        let layer_id = layer.id.clone();

        board.nodes.insert(fetch.id.clone(), fetch);
        board.layers.insert(layer.id.clone(), layer);
        (board, fetch_id, log_id, layer_id)
    }

    #[test]
    fn layout_context_maps_node_ids_to_spatial_facts_only() {
        let (board, fetch_id, log_id, layer_id) = layout_fixture_board();

        let layout = prepare_layout_context(&board, std::slice::from_ref(&fetch_id));
        let fetch_layout = &layout.nodes[&fetch_id];
        assert_eq!(fetch_layout.position, (120, -40));
        assert_eq!(fetch_layout.estimated_size, (200, 32 + 2 * 20));
        assert_eq!(fetch_layout.layer_id, None);
        assert_eq!(
            layout.nodes[&log_id].layer_id.as_deref(),
            Some(layer_id.as_str())
        );
        assert_eq!(layout.layers.len(), 1);
        assert_eq!(layout.layers[0].id, layer_id);
        assert_eq!(layout.selected_nodes, vec![fetch_id.clone()]);

        let json = serde_json::to_value(&layout).expect("serialized layout context");
        let fetch_json = &json["nodes"][fetch_id.as_str()];
        assert_eq!(fetch_json["p"][0], 120);
        assert_eq!(fetch_json["s"][0], 200);
        assert!(fetch_json.get("i").is_none(), "no pin lists in layout");
        assert!(fetch_json.get("v").is_none(), "no defaults in layout");
        assert!(json.get("edges").is_none(), "no edges in layout");
        assert!(json.get("variables").is_none(), "no variables in layout");
    }

    #[tokio::test]
    async fn compiled_helper_parameter_remains_connected_in_inspection_context() {
        let mut board = Board::new_detached(Some("helper-context".to_string()), Path::default());
        let mut contains = Node::new("string_contains_any", "Contains Any", "", "Utils/String");
        contains.set_flowscript_name("string", "containsAny");
        contains.set_receiver("string");
        contains.add_input_pin("string", "String", "", VariableType::String);
        contains
            .add_input_pin("substrings", "Substrings", "", VariableType::String)
            .set_value_type(ValueType::Array);
        contains.add_input_pin("ignore_case", "Ignore Case", "", VariableType::Boolean);
        contains.add_output_pin("contains", "Contains", "", VariableType::Boolean);
        contains.add_output_pin("matched", "Matched", "", VariableType::String);
        let state = Arc::new(FlowLikeState::new(
            FlowLikeConfig::new(),
            HTTPClient::new_without_refetch(),
        ));
        let result = apply_flowscript_to_board(
            &mut board,
            r#"function routeRecord(summary: string): (contains: bool) {
    return summary.containsAny({ substrings: ["outage", "service interruption", "cannot log in"], ignoreCase: true }).contains
}
"#,
            &[contains],
            state,
            None,
            false,
        )
        .await
        .expect("helper source applies");
        assert!(result.diagnostics.is_empty(), "{:?}", result.diagnostics);
        let layer = board.layers.values().next().expect("helper layer");
        let layer_id = layer.id.clone();
        let parameter_id = layer
            .pins
            .values()
            .find(|pin| pin.name == "summary")
            .expect("summary parameter")
            .id
            .clone();
        let body = board
            .nodes
            .values()
            .find(|node| node.name == "string_contains_any")
            .expect("compiled Contains Any");
        let body_id = body.id.clone();
        let input_id = body
            .pins
            .values()
            .find(|pin| pin.name == "string")
            .expect("receiver input")
            .id
            .clone();
        assert!(body.pins[&input_id].depends_on.contains(&parameter_id));
        assert!(layer.pins[&parameter_id].connected_to.contains(&input_id));

        let context = prepare_context(&board, &[]).expect("inspection context");
        assert_eq!(context.layers[0].node_ids, vec![body_id.clone()]);
        assert_eq!(
            prepare_layout_context(&board, &[]).nodes[&body_id].layer_id,
            Some(layer_id.clone())
        );
        let details: serde_json::Value =
            serde_json::from_str(&build_node_details_output(&body_id, &context)).unwrap();
        assert_eq!(
            details["incoming_connections"],
            serde_json::json!([{
                "from_node": layer_id,
                "from_pin": "summary",
                "to_pin": "string"
            }])
        );
        assert!(build_unconfigured_nodes_output(&context).starts_with("All nodes are configured"));
        assert!(context.edges.iter().any(|edge| {
            edge.from_node_id == body_id
                && edge.from_pin_name == "contains"
                && edge.to_node_id == layer_id
                && edge.to_pin_name == "contains"
        }));

        board
            .layers
            .get_mut(&layer_id)
            .unwrap()
            .pins
            .get_mut(&parameter_id)
            .unwrap()
            .connected_to
            .remove(&input_id);
        board
            .nodes
            .get_mut(&body_id)
            .unwrap()
            .pins
            .get_mut(&input_id)
            .unwrap()
            .depends_on
            .remove(&parameter_id);
        let disconnected = prepare_context(&board, &[]).unwrap();
        let missing: serde_json::Value =
            serde_json::from_str(&build_unconfigured_nodes_output(&disconnected)).unwrap();
        assert_eq!(missing[0]["node_id"], body_id);
        assert_eq!(
            missing[0]["missing_inputs"],
            serde_json::json!([{"pin": "string", "type": "String"}])
        );
    }

    #[test]
    fn inspection_edges_preserve_legacy_boundaries_without_duplicates_or_dangling_sources() {
        let mut board = Board::new_detached(Some("legacy-context".to_string()), Path::default());
        let mut layer = Layer::new(
            "helper".to_string(),
            "helper".to_string(),
            LayerType::Function,
        );
        let mut boundary = Node::new("boundary", "Boundary", "", "test");
        let mut parameter = boundary
            .add_input_pin("summary", "Summary", "", VariableType::String)
            .clone();
        let mut body = Node::new("string_contains_any", "Contains Any", "", "Utils/String");
        let receiver = body.add_input_pin("string", "String", "", VariableType::String);
        receiver.depends_on.insert(parameter.id.clone());
        parameter.connected_to.insert(receiver.id.clone());
        let body_id = body.id.clone();
        let parameter_id = parameter.id.clone();
        layer.nodes.insert(body.id.clone(), body);
        layer.pins.insert(parameter.id.clone(), parameter);
        board.layers.insert(layer.id.clone(), layer);
        let context = prepare_context(&board, &[]).unwrap();
        assert_eq!(
            context.edges.len(),
            1,
            "symmetric wiring must produce one edge"
        );
        assert_eq!(context.edges[0].from_node_id, "helper");
        assert_eq!(context.edges[0].to_node_id, body_id);

        board
            .layers
            .get_mut("helper")
            .unwrap()
            .pins
            .get_mut(&parameter_id)
            .unwrap()
            .connected_to
            .clear();
        assert_eq!(
            prepare_context(&board, &[]).unwrap().edges,
            context.edges,
            "reader-side wiring remains visible as it does in FlowScript lowering"
        );
        board
            .layers
            .get_mut("helper")
            .unwrap()
            .pins
            .remove(&parameter_id);
        let dangling = prepare_context(&board, &[]).unwrap();
        assert!(dangling.edges.is_empty());
        assert!(build_unconfigured_nodes_output(&dangling).contains("missing_inputs"));
    }

    #[test]
    fn layout_context_embedding_is_smaller_than_graph_context() {
        let (board, fetch_id, _, _) = layout_fixture_board();

        let rich = flow_like_types::json::to_string_pretty(
            &prepare_context(&board, std::slice::from_ref(&fetch_id)).expect("graph context"),
        )
        .expect("rich json");
        let slim = flow_like_types::json::to_string_pretty(&prepare_layout_context(
            &board,
            std::slice::from_ref(&fetch_id),
        ))
        .expect("slim json");
        eprintln!(
            "layout embedding: slim {} bytes vs rich {} bytes",
            slim.len(),
            rich.len()
        );
        assert!(
            slim.len() < rich.len(),
            "layout embedding must shrink the prompt: slim {} bytes vs rich {} bytes",
            slim.len(),
            rich.len()
        );
    }

    #[test]
    fn default_function_cache_is_exposed_in_graph_context_with_flowscript_names() {
        let mut board = Board::new_detached(Some("cache-context".to_string()), Path::default());
        let mut layer = Layer::new(
            "pricing-layer".to_string(),
            "calculatePricing".to_string(),
            LayerType::Function,
        );
        layer.cache = Some(LayerCache {
            enabled: true,
            prefix: "global".to_string(),
            ttl_seconds: Some(300),
            scope: LayerCacheScope::App,
        });
        board.layers.insert(layer.id.clone(), layer);

        let context = prepare_context(&board, &[]).expect("graph context");
        assert_eq!(context.layers[0].layer_type, "Function");
        let cache = context.layers[0]
            .cache
            .as_ref()
            .expect("function cache context");
        assert!(cache.enabled);
        assert_eq!(cache.namespace, "global");
        assert_eq!(cache.ttl_seconds, Some(300));
        assert_eq!(cache.scope, "app");

        let json = serde_json::to_value(&context).expect("serialized graph context");
        assert_eq!(json["layers"][0]["cache"]["namespace"], "global");
        assert_eq!(json["layers"][0]["cache"]["ttl_seconds"], 300);
        assert_eq!(json["layers"][0]["cache"]["scope"], "app");
    }

    #[test]
    fn disabled_function_cache_settings_remain_visible_to_flowpilot() {
        let mut board = Board::new_detached(Some("cache-context".to_string()), Path::default());
        let mut layer = Layer::new(
            "pricing-layer".to_string(),
            "calculatePricing".to_string(),
            LayerType::Function,
        );
        layer.cache = Some(LayerCache {
            enabled: false,
            prefix: "remembered-pricing".to_string(),
            ttl_seconds: Some(0),
            scope: LayerCacheScope::User,
        });
        board.layers.insert(layer.id.clone(), layer);

        let context = prepare_context(&board, &[]).expect("graph context");
        let cache = context.layers[0]
            .cache
            .as_ref()
            .expect("disabled settings should remain inspectable");
        assert!(!cache.enabled);
        assert_eq!(cache.namespace, "remembered-pricing");
        assert_eq!(cache.ttl_seconds, Some(0));
        assert_eq!(cache.scope, "user");
    }

    #[test]
    fn permanent_cache_exposes_null_ttl_to_flowpilot() {
        let mut board = Board::new_detached(Some("cache-context".to_string()), Path::default());
        let mut layer = Layer::new(
            "legacy-cache".to_string(),
            "legacyCached".to_string(),
            LayerType::Function,
        );
        layer.cache = Some(LayerCache {
            enabled: true,
            prefix: "global".to_string(),
            ttl_seconds: None,
            scope: LayerCacheScope::App,
        });
        board.layers.insert(layer.id.clone(), layer);

        let context = prepare_context(&board, &[]).expect("graph context");
        let json = serde_json::to_value(&context).expect("serialized graph context");
        assert!(json["layers"][0]["cache"]["ttl_seconds"].is_null());
    }
}
