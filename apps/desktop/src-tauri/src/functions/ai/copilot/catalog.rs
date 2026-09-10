//! Desktop catalog search and runtime node metadata.

use crate::state::TauriFlowLikeState;
use async_trait::async_trait;
use flow_like::{
    app::App,
    flow::{
        ast::apply_board_commands_to_board,
        board::Board,
        copilot::{
            BoardCommand, CatalogProvider, NodeMetadata, PinMetadata, enrich_node_metadata,
            score_catalog_metadata,
        },
        node::Node,
        pin::{Pin, PinType},
        variable::VariableType,
    },
};
use flow_like_catalog::get_catalog;
use std::{collections::HashSet, sync::Arc};
use tauri::{AppHandle, Manager};

/// Desktop implementation of the catalog provider for node search
pub(super) struct DesktopCatalogProvider {
    nodes: Arc<Vec<Node>>,
}

impl DesktopCatalogProvider {
    pub(super) fn new(injected_nodes: Option<Vec<Node>>) -> Self {
        let mut nodes = static_catalog_nodes();

        if let Some(injected_nodes) = injected_nodes {
            let mut wasm_node_keys: HashSet<(String, String)> = nodes
                .iter()
                .filter_map(|node| {
                    node.wasm
                        .as_ref()
                        .map(|wasm| (wasm.package_id.clone(), node.name.clone()))
                })
                .collect();

            for node in injected_nodes {
                let Some(wasm) = node.wasm.as_ref() else {
                    continue;
                };

                if wasm_node_keys.insert((wasm.package_id.clone(), node.name.clone())) {
                    nodes.push(node);
                }
            }
        }

        Self {
            nodes: Arc::new(nodes),
        }
    }

    pub(super) fn len(&self) -> usize {
        self.nodes.len()
    }

    pub(super) fn all_metadata(&self) -> Vec<NodeMetadata> {
        self.nodes.iter().map(node_to_metadata).collect()
    }
}

fn static_catalog_nodes() -> Vec<Node> {
    get_catalog()
        .into_iter()
        .map(|logic| logic.get_node())
        .collect()
}

pub(super) async fn authoritative_app_catalog_nodes(
    app_handle: &AppHandle,
    app_id: Option<&str>,
) -> Option<Vec<Node>> {
    let app_id = app_id.map(str::trim).filter(|app_id| !app_id.is_empty())?;
    let managed_state = app_handle.try_state::<TauriFlowLikeState>()?;
    let flow_like_state = managed_state.0.clone();
    let app = App::load(app_id.to_string(), flow_like_state.clone())
        .await
        .ok()?;
    let allowed_packages = app.packages.keys().cloned().collect::<HashSet<_>>();
    let nodes = flow_like_state
        .node_registry
        .read()
        .await
        .get_nodes()
        .ok()?;
    Some(
        nodes
            .into_iter()
            .filter(|node| match &node.wasm {
                None => true,
                Some(wasm) => allowed_packages.contains(&wasm.package_id),
            })
            .collect(),
    )
}

fn pin_to_metadata(p: &Pin) -> PinMetadata {
    let is_generic = p.data_type == VariableType::Generic;
    let enforce_schema = p
        .options
        .as_ref()
        .and_then(|o| o.enforce_schema)
        .unwrap_or(false);
    let valid_values = p.options.as_ref().and_then(|o| o.valid_values.clone());

    PinMetadata {
        name: p.name.clone(),
        friendly_name: p.friendly_name.clone(),
        description: p.description.clone(),
        data_type: format!("{:?}", p.data_type),
        value_type: format!("{:?}", p.value_type),
        default_value: p
            .default_value
            .as_ref()
            .map(|value| String::from_utf8_lossy(value).to_string())
            .filter(|value| !value.is_empty() && value != "null"),
        schema: p.schema.clone(),
        is_generic,
        valid_values,
        enforce_schema,
    }
}

fn node_to_metadata(node: &Node) -> NodeMetadata {
    let derived_category = node
        .name
        .to_lowercase()
        .split("::")
        .nth(1)
        .unwrap_or("")
        .to_string();
    let category = if derived_category.is_empty() {
        node.category.clone()
    } else {
        derived_category
    };

    let mut inputs: Vec<&Pin> = node
        .pins
        .values()
        .filter(|p| p.pin_type == PinType::Input)
        .collect();
    inputs.sort_by_key(|p| (p.index, p.name.clone()));

    let mut outputs: Vec<&Pin> = node
        .pins
        .values()
        .filter(|p| p.pin_type == PinType::Output)
        .collect();
    outputs.sort_by_key(|p| (p.index, p.name.clone()));

    enrich_node_metadata(NodeMetadata {
        name: node.name.clone(),
        friendly_name: node.friendly_name.clone(),
        description: node.description.clone(),
        inputs: inputs.into_iter().map(pin_to_metadata).collect(),
        outputs: outputs.into_iter().map(pin_to_metadata).collect(),
        category: Some(category),
        required_inputs: Vec::new(),
        companion_nodes: Vec::new(),
        capability_tags: Vec::new(),
        namespace: Some(node.flowscript_namespace()),
        alias: Some(node.flowscript_alias()),
        receiver: node.flowscript_receiver(),
    })
}

#[async_trait]
impl CatalogProvider for DesktopCatalogProvider {
    async fn test_draft_board(
        &self,
        board: Board,
        commands: Vec<BoardCommand>,
        entry: String,
        payload: Option<serde_json::Value>,
    ) -> Result<serde_json::Value, String> {
        let result = flow_like_catalog::draft_test::prepare_and_test_draft_board(
            board,
            &entry,
            payload,
            |mut board, state| async move {
                let catalog = state
                    .node_registry
                    .read()
                    .await
                    .get_nodes()
                    .map_err(|error| error.to_string())?;
                let applied =
                    apply_board_commands_to_board(&mut board, commands, &catalog, state, None)
                        .await
                        .map_err(|error| error.to_string())?;
                if !applied.diagnostics.is_empty() {
                    return Err(applied.diagnostics.join("\n"));
                }
                Ok(board)
            },
        )
        .await?;
        serde_json::to_value(result).map_err(|error| error.to_string())
    }

    async fn search(&self, query: &str) -> Vec<NodeMetadata> {
        let mut scored_matches: Vec<(i32, NodeMetadata)> = Vec::new();

        for node in self.nodes.iter() {
            let metadata = node_to_metadata(node);
            let score = score_catalog_metadata(&metadata, query);

            if score > 0 {
                scored_matches.push((score, metadata));
            }
        }

        scored_matches.sort_by(|(left_score, left), (right_score, right)| {
            right_score
                .cmp(left_score)
                .then_with(|| left.name.cmp(&right.name))
        });
        scored_matches
            .into_iter()
            .take(10)
            .map(|(_, meta)| meta)
            .collect()
    }

    async fn search_by_pin_type(&self, pin_type: &str, is_input: bool) -> Vec<NodeMetadata> {
        let pin_type = pin_type.to_lowercase();
        let mut matches = Vec::new();

        for node in self.nodes.iter() {
            let has_matching_pin = node.pins.values().any(|p| {
                let is_correct_direction = if is_input {
                    p.pin_type == PinType::Input
                } else {
                    p.pin_type == PinType::Output
                };
                is_correct_direction
                    && format!("{:?}", p.data_type)
                        .to_lowercase()
                        .contains(&pin_type)
            });

            if has_matching_pin {
                matches.push(node_to_metadata(node));
            }
            if matches.len() >= 10 {
                break;
            }
        }
        matches
    }

    async fn filter_by_category(&self, category_prefix: &str) -> Vec<NodeMetadata> {
        let category_prefix = category_prefix.to_lowercase().replace("::", "/");
        let mut matches = Vec::new();

        for node in self.nodes.iter() {
            let category = node.category.to_lowercase();
            let namespace = node.flowscript_namespace().to_lowercase().replace('.', "/");
            let name_lower = node.name.to_lowercase();

            if category.contains(&category_prefix)
                || namespace.contains(&category_prefix)
                || name_lower.contains(&category_prefix)
            {
                matches.push(node_to_metadata(node));
            }
            if matches.len() >= 15 {
                break;
            }
        }
        matches
    }

    async fn get_node_metadata(&self, node_type: &str) -> Option<NodeMetadata> {
        self.nodes
            .iter()
            .find(|node| node.name == node_type)
            .map(node_to_metadata)
    }

    async fn get_all_nodes(&self) -> Vec<String> {
        self.nodes.iter().map(|node| node.name.clone()).collect()
    }

    async fn get_all_metadata(&self) -> Vec<NodeMetadata> {
        self.all_metadata()
    }
}

#[cfg(test)]
mod retrieval_tests {
    use super::*;

    #[tokio::test]
    async fn live_catalog_retrieval_covers_contract_fields_and_service_names() {
        let provider = DesktopCatalogProvider::new(None);
        let cases = [
            ("attachment MIME", "email::attachmentToFields"),
            ("microsoft graph odata deltaLink", "microsoft::graphRequest"),
            ("ontology action idempotency key", "ontology::actionInput"),
            ("jira attachment upload", "jira::uploadAttachment"),
            (
                "confluence attachment upload",
                "confluence::uploadAttachment",
            ),
            ("datafusion create session", "df::createSession"),
            ("datafusion register Lance", "df::registerLance"),
        ];
        let queries = cases
            .iter()
            .map(|(query, _)| (*query).to_string())
            .collect::<Vec<_>>();
        let started = std::time::Instant::now();
        let results = provider.get_declarations_batch(&queries).await;
        let elapsed = started.elapsed();
        let mut failures = Vec::new();
        for ((query, expected), result) in cases.into_iter().zip(results) {
            let resolution: serde_json::Value = result
                .lines()
                .find_map(|line| {
                    line.strip_prefix("// flowpilot.declaration-resolution/v1 ")
                        .and_then(|payload| serde_json::from_str(payload).ok())
                })
                .expect("declaration response must report resolution evidence");
            let top = &resolution["candidates"][0];
            println!(
                "{}",
                serde_json::json!({
                    "query": query, "expected": expected,
                    "status": resolution["status"], "top": top,
                })
            );
            // This query names data but no service or operation. Several live contracts
            // contain MIME information, so the resolver must ask for refinement.
            if query == "attachment MIME" {
                if resolution["status"] != "ambiguous"
                    || !resolution["candidates"]
                        .as_array()
                        .unwrap()
                        .iter()
                        .any(|candidate| candidate["function_name"] == expected)
                {
                    failures.push(format!("{query}: {resolution}"));
                }
                for candidate in resolution["candidates"].as_array().unwrap() {
                    assert!(
                        !result.contains(&format!(
                            "function {}(",
                            candidate["function_name"].as_str().unwrap()
                        )),
                        "ambiguous results must not expose usable declarations: {result}"
                    );
                }
            } else if top["function_name"] != expected
                || top["accepted"] != true
                || resolution["status"] != "resolved"
            {
                failures.push(format!("{query}: {resolution}"));
            }
        }
        println!(
            "catalog_nodes={} queries={} elapsed_ms={}",
            provider.len(),
            queries.len(),
            elapsed.as_millis()
        );
        assert!(failures.is_empty(), "{}", failures.join("\n"));
    }
}
