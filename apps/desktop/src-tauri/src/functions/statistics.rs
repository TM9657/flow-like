use crate::{
    functions::TauriFunctionError,
    state::{TauriFlowLikeState, TauriSettingsState},
};
use flow_like::{app::App, flow::board::Board};
use flow_like_types::tokio::task::JoinSet;
use serde::{Deserialize, Serialize};
use std::collections::{BTreeSet, HashMap, HashSet};
use tauri::AppHandle;

#[derive(Serialize, Deserialize, Clone, Debug, Default)]
pub struct NodeUsage {
    pub name: String,
    pub friendly_name: String,
    pub category: String,
    pub count: u32,
    pub boards: Vec<String>,
}

struct BoardLoadResult {
    summary: BoardSummary,
    graph: BoardGraph,
    node_usages: HashMap<String, (String, String, String)>,
}

/// Reference to a board for linking
#[derive(Serialize, Deserialize, Clone, Debug, Default)]
pub struct BoardRef {
    pub id: String,
    pub name: String,
    pub app_id: String,
}

/// A reusable pattern of connected nodes found across boards.
#[derive(Serialize, Deserialize, Clone, Debug, Default)]
pub struct NodePattern {
    pub nodes: Vec<String>,
    pub edge_count: u32,
    pub occurrences: u32,
    pub boards: Vec<BoardRef>,
    /// Rarity score: size^2 * ln(board_count) / sqrt(frequency) - prefers rare large patterns
    pub rarity_score: f32,
    /// Frequency score: size * frequency - prefers common patterns
    pub frequency_score: f32,
}

#[derive(Serialize, Deserialize, Clone, Debug, Default)]
pub struct CategoryStats {
    pub name: String,
    pub node_count: u32,
    pub unique_nodes: u32,
}

#[derive(Serialize, Deserialize, Clone, Debug, Default)]
pub struct BoardSummary {
    pub id: String,
    pub app_id: String,
    pub name: String,
    pub node_count: u32,
    pub connection_count: u32,
    pub variable_count: u32,
    pub layer_count: u32,
    pub comment_count: u32,
}

#[derive(Serialize, Deserialize, Clone, Debug, Default)]
pub struct BoardStatistics {
    pub total_boards: u32,
    pub total_nodes: u32,
    pub total_connections: u32,
    pub total_variables: u32,
    pub total_layers: u32,
    pub total_comments: u32,
    pub avg_nodes_per_board: f32,
    pub avg_connections_per_board: f32,
    pub most_used_nodes: Vec<NodeUsage>,
    /// Rare but interesting patterns (large structures appearing few times)
    pub rare_patterns: Vec<NodePattern>,
    /// Common patterns (frequently used combinations)
    pub common_patterns: Vec<NodePattern>,
    pub category_stats: Vec<CategoryStats>,
    pub board_summaries: Vec<BoardSummary>,
}

struct BoardGraph {
    board_id: String,
    board_name: String,
    app_id: String,
    node_types: HashMap<String, String>,
    adjacency: HashMap<String, HashSet<String>>,
}

impl BoardGraph {
    fn from_board(board: &Board, app_id: &str) -> Self {
        let mut node_types = HashMap::new();
        let mut adjacency: HashMap<String, HashSet<String>> = HashMap::new();
        let mut pin_to_node: HashMap<String, String> = HashMap::new();

        for (node_id, node) in &board.nodes {
            node_types.insert(node_id.clone(), node.name.clone());
            adjacency.entry(node_id.clone()).or_default();
            for pin_id in node.pins.keys() {
                pin_to_node.insert(pin_id.clone(), node_id.clone());
            }
        }

        for layer in board.layers.values() {
            for (node_id, node) in &layer.nodes {
                node_types.insert(node_id.clone(), node.name.clone());
                adjacency.entry(node_id.clone()).or_default();
                for pin_id in node.pins.keys() {
                    pin_to_node.insert(pin_id.clone(), node_id.clone());
                }
            }
        }

        for node in board.nodes.values() {
            for pin in node.pins.values() {
                for connected_pin_id in &pin.connected_to {
                    if let Some(other_node_id) = pin_to_node.get(connected_pin_id) {
                        if let Some(this_node_id) = pin_to_node.get(&pin.id) {
                            if this_node_id != other_node_id {
                                adjacency
                                    .entry(this_node_id.clone())
                                    .or_default()
                                    .insert(other_node_id.clone());
                                adjacency
                                    .entry(other_node_id.clone())
                                    .or_default()
                                    .insert(this_node_id.clone());
                            }
                        }
                    }
                }
            }
        }

        for layer in board.layers.values() {
            for node in layer.nodes.values() {
                for pin in node.pins.values() {
                    for connected_pin_id in &pin.connected_to {
                        if let Some(other_node_id) = pin_to_node.get(connected_pin_id) {
                            if let Some(this_node_id) = pin_to_node.get(&pin.id) {
                                if this_node_id != other_node_id {
                                    adjacency
                                        .entry(this_node_id.clone())
                                        .or_default()
                                        .insert(other_node_id.clone());
                                    adjacency
                                        .entry(other_node_id.clone())
                                        .or_default()
                                        .insert(this_node_id.clone());
                                }
                            }
                        }
                    }
                }
            }
        }

        Self {
            board_id: board.id.clone(),
            board_name: board.name.clone(),
            app_id: app_id.to_string(),
            node_types,
            adjacency,
        }
    }

    fn extract_patterns(&self, max_size: usize) -> Vec<PatternSignature> {
        let mut patterns = Vec::new();
        let node_ids: Vec<&String> = self.node_types.keys().collect();

        for start_node in &node_ids {
            self.grow_pattern_from(start_node, max_size, &mut patterns);
        }

        patterns
    }

    fn grow_pattern_from(
        &self,
        start: &str,
        max_size: usize,
        patterns: &mut Vec<PatternSignature>,
    ) {
        let mut frontier: Vec<HashSet<String>> = vec![HashSet::from([start.to_string()])];

        for _size in 1..max_size {
            let mut next_frontier = Vec::new();

            for subgraph in &frontier {
                let mut candidates: HashSet<String> = HashSet::new();
                for node_id in subgraph {
                    if let Some(neighbors) = self.adjacency.get(node_id) {
                        for neighbor in neighbors {
                            if !subgraph.contains(neighbor) {
                                candidates.insert(neighbor.clone());
                            }
                        }
                    }
                }

                for candidate in candidates {
                    if candidate.as_str() > start {
                        let mut new_subgraph = subgraph.clone();
                        new_subgraph.insert(candidate);
                        next_frontier.push(new_subgraph);
                    }
                }
            }

            for subgraph in &next_frontier {
                if let Some(sig) = self.canonicalize_subgraph(subgraph) {
                    patterns.push(sig);
                }
            }

            next_frontier.sort_by(|a, b| {
                let a_sorted: BTreeSet<_> = a.iter().collect();
                let b_sorted: BTreeSet<_> = b.iter().collect();
                a_sorted.cmp(&b_sorted)
            });
            next_frontier.dedup_by(|a, b| {
                let a_sorted: BTreeSet<_> = a.iter().collect();
                let b_sorted: BTreeSet<_> = b.iter().collect();
                a_sorted == b_sorted
            });

            frontier = next_frontier;
            if frontier.is_empty() {
                break;
            }
        }
    }

    fn canonicalize_subgraph(&self, node_ids: &HashSet<String>) -> Option<PatternSignature> {
        if node_ids.len() < 2 {
            return None;
        }

        let mut node_types: Vec<String> = node_ids
            .iter()
            .filter_map(|id| self.node_types.get(id).cloned())
            .collect();
        node_types.sort();

        let mut edge_count = 0u32;
        let mut counted_edges: HashSet<(String, String)> = HashSet::new();

        for node_id in node_ids {
            if let Some(neighbors) = self.adjacency.get(node_id) {
                for neighbor in neighbors {
                    if node_ids.contains(neighbor) {
                        let edge = if node_id < neighbor {
                            (node_id.clone(), neighbor.clone())
                        } else {
                            (neighbor.clone(), node_id.clone())
                        };
                        if counted_edges.insert(edge) {
                            edge_count += 1;
                        }
                    }
                }
            }
        }

        if edge_count == 0 {
            return None;
        }

        let mut edge_types: Vec<(String, String)> = Vec::new();
        for node_id in node_ids {
            let node_type = self.node_types.get(node_id)?;
            if let Some(neighbors) = self.adjacency.get(node_id) {
                for neighbor in neighbors {
                    if node_ids.contains(neighbor) && node_id < neighbor {
                        let neighbor_type = self.node_types.get(neighbor)?;
                        let edge = if node_type <= neighbor_type {
                            (node_type.clone(), neighbor_type.clone())
                        } else {
                            (neighbor_type.clone(), node_type.clone())
                        };
                        edge_types.push(edge);
                    }
                }
            }
        }
        edge_types.sort();

        Some(PatternSignature {
            node_types,
            edge_types,
            edge_count,
        })
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
struct PatternSignature {
    node_types: Vec<String>,
    edge_types: Vec<(String, String)>,
    edge_count: u32,
}

impl PatternSignature {
    fn to_canonical_key(&self) -> String {
        let nodes = self.node_types.join(",");
        let edges: Vec<String> = self
            .edge_types
            .iter()
            .map(|(a, b)| format!("{a}~{b}"))
            .collect();
        format!("[{nodes}]:{}", edges.join(";"))
    }
}

fn mine_patterns(boards: &[BoardGraph], max_pattern_size: usize) -> (Vec<NodePattern>, Vec<NodePattern>) {
    let mut pattern_counts: HashMap<String, (PatternSignature, u32, HashMap<String, BoardRef>)> =
        HashMap::new();

    for board in boards {
        let board_patterns = board.extract_patterns(max_pattern_size);

        let unique_patterns: HashSet<String> =
            board_patterns.iter().map(|p| p.to_canonical_key()).collect();

        for key in unique_patterns {
            if let Some(sig) = board_patterns.iter().find(|p| p.to_canonical_key() == key) {
                let entry = pattern_counts
                    .entry(key)
                    .or_insert_with(|| (sig.clone(), 0, HashMap::new()));
                entry.1 += 1;
                entry.2.insert(board.board_id.clone(), BoardRef {
                    id: board.board_id.clone(),
                    name: board.board_name.clone(),
                    app_id: board.app_id.clone(),
                });
            }
        }
    }

    let patterns: Vec<NodePattern> = pattern_counts
        .into_iter()
        .filter(|(_, (sig, count, _))| *count >= 2 && sig.node_types.len() >= 2)
        .map(|(_, (sig, occurrences, boards))| {
            let size = sig.node_types.len() as f32;
            let board_count = boards.len() as f32;
            // Rarity score: prefers large rare patterns
            let rarity_score = (size * size) * (1.0 + board_count).ln() / (occurrences as f32).sqrt();
            // Frequency score: prefers common patterns (size matters less)
            let frequency_score = size * (occurrences as f32) * (1.0 + board_count).ln();

            NodePattern {
                nodes: sig.node_types,
                edge_count: sig.edge_count,
                occurrences,
                boards: boards.into_values().collect(),
                rarity_score,
                frequency_score,
            }
        })
        .collect();

    // Sort by rarity (rare large patterns first)
    let mut rare_patterns = patterns.clone();
    rare_patterns.sort_by(|a, b| b.rarity_score.partial_cmp(&a.rarity_score).unwrap_or(std::cmp::Ordering::Equal));
    let rare_patterns = filter_redundant_patterns(rare_patterns, 20);

    // Sort by frequency (common patterns first)
    let mut common_patterns = patterns;
    common_patterns.sort_by(|a, b| b.frequency_score.partial_cmp(&a.frequency_score).unwrap_or(std::cmp::Ordering::Equal));
    let common_patterns = filter_redundant_patterns(common_patterns, 20);

    (rare_patterns, common_patterns)
}

fn filter_redundant_patterns(patterns: Vec<NodePattern>, max_count: usize) -> Vec<NodePattern> {
    let mut result = Vec::new();

    for pattern in patterns {
        let dominated = result.iter().any(|existing: &NodePattern| {
            pattern.nodes.len() < existing.nodes.len()
                && pattern.edge_count <= existing.edge_count
                && pattern.nodes.iter().all(|n| existing.nodes.contains(n))
        });

        if !dominated {
            result.push(pattern);
            if result.len() >= max_count {
                break;
            }
        }
    }

    result
}

fn analyze_board(board: &Board, app_id: &str) -> (BoardSummary, HashMap<String, (String, String, String)>, u32) {
    let mut node_usage: HashMap<String, (String, String, String)> = HashMap::new();
    let mut connection_count = 0u32;

    for node in board.nodes.values() {
        node_usage
            .entry(node.name.clone())
            .or_insert((node.name.clone(), node.friendly_name.clone(), node.category.clone()));

        for pin in node.pins.values() {
            connection_count += pin.connected_to.len() as u32;
        }
    }

    for layer in board.layers.values() {
        for node in layer.nodes.values() {
            node_usage
                .entry(node.name.clone())
                .or_insert((node.name.clone(), node.friendly_name.clone(), node.category.clone()));

            for pin in node.pins.values() {
                connection_count += pin.connected_to.len() as u32;
            }
        }
    }

    connection_count /= 2;

    let summary = BoardSummary {
        id: board.id.clone(),
        app_id: app_id.to_string(),
        name: board.name.clone(),
        node_count: board.nodes.len() as u32
            + board.layers.values().map(|l| l.nodes.len() as u32).sum::<u32>(),
        connection_count,
        variable_count: board.variables.len() as u32,
        layer_count: board.layers.len() as u32,
        comment_count: board.comments.len() as u32,
    };

    (summary, node_usage, connection_count)
}

#[tauri::command(async)]
pub async fn get_board_statistics(
    app_handle: AppHandle,
) -> Result<BoardStatistics, TauriFunctionError> {
    let profile = TauriSettingsState::current_profile(&app_handle).await?;
    let flow_like_state = TauriFlowLikeState::construct(&app_handle).await?;

    let apps = profile.hub_profile.apps.unwrap_or_default();

    let mut join_set: JoinSet<Result<Vec<BoardLoadResult>, TauriFunctionError>> = JoinSet::new();

    for profile_app in apps {
        let app_id = profile_app.app_id.clone();
        let state = flow_like_state.clone();

        join_set.spawn(async move {
            let app = App::load(app_id.clone(), state)
                .await
                .map_err(|e| TauriFunctionError::new(&format!("Failed to load app: {e}")))?;

            let mut results = Vec::new();
            for board_id in &app.boards {
                if let Ok(board) = app.open_board(board_id.clone(), Some(true), None).await {
                    let board_lock = board.lock().await;
                    let (summary, node_usages, _) = analyze_board(&board_lock, &app_id);
                    let graph = BoardGraph::from_board(&board_lock, &app_id);
                    drop(board_lock);

                    results.push(BoardLoadResult {
                        summary,
                        graph,
                        node_usages,
                    });
                }
            }
            Ok(results)
        });
    }

    let mut all_results: Vec<BoardLoadResult> = Vec::new();
    while let Some(result) = join_set.join_next().await {
        match result {
            Ok(Ok(results)) => all_results.extend(results),
            Ok(Err(e)) => tracing::warn!("Failed to load boards: {:?}", e),
            Err(e) => tracing::warn!("Task failed: {:?}", e),
        }
    }

    let mut stats = BoardStatistics::default();
    let mut global_node_usage: HashMap<String, NodeUsage> = HashMap::new();
    let mut category_node_counts: HashMap<String, (u32, HashSet<String>)> = HashMap::new();

    for result in &all_results {
        let summary = &result.summary;
        stats.total_nodes += summary.node_count;
        stats.total_connections += summary.connection_count;
        stats.total_variables += summary.variable_count;
        stats.total_layers += summary.layer_count;
        stats.total_comments += summary.comment_count;

        for (name, (node_name, friendly_name, category)) in &result.node_usages {
            let entry = global_node_usage
                .entry(name.clone())
                .or_insert(NodeUsage {
                    name: node_name.clone(),
                    friendly_name: friendly_name.clone(),
                    category: category.clone(),
                    count: 0,
                    boards: Vec::new(),
                });
            entry.count += 1;
            if !entry.boards.contains(&summary.name) {
                entry.boards.push(summary.name.clone());
            }

            let cat_entry = category_node_counts
                .entry(category.clone())
                .or_insert((0, HashSet::new()));
            cat_entry.0 += 1;
            cat_entry.1.insert(name.clone());
        }

        stats.board_summaries.push(summary.clone());
    }

    stats.total_boards = all_results.len() as u32;
    stats.avg_nodes_per_board = if stats.total_boards > 0 {
        stats.total_nodes as f32 / stats.total_boards as f32
    } else {
        0.0
    };
    stats.avg_connections_per_board = if stats.total_boards > 0 {
        stats.total_connections as f32 / stats.total_boards as f32
    } else {
        0.0
    };

    let graphs: Vec<BoardGraph> = all_results.into_iter().map(|r| r.graph).collect();
    let (rare, common) = flow_like_types::tokio::task::spawn_blocking(move || {
        mine_patterns(&graphs, 8)
    })
    .await
    .map_err(|e| TauriFunctionError::new(&format!("Mining task failed: {e}")))?;

    stats.rare_patterns = rare;
    stats.common_patterns = common;

    let mut node_usage_vec: Vec<NodeUsage> = global_node_usage.into_values().collect();
    node_usage_vec.sort_by(|a, b| b.count.cmp(&a.count));
    stats.most_used_nodes = node_usage_vec.into_iter().take(50).collect();

    stats.category_stats = category_node_counts
        .into_iter()
        .map(|(name, (count, unique))| CategoryStats {
            name,
            node_count: count,
            unique_nodes: unique.len() as u32,
        })
        .collect();
    stats.category_stats.sort_by(|a, b| b.node_count.cmp(&a.node_count));

    stats.board_summaries.sort_by(|a, b| b.node_count.cmp(&a.node_count));

    Ok(stats)
}
