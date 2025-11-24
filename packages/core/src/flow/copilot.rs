use std::sync::Arc;

use async_trait::async_trait;
use flow_like_types::Result;
use rig::{
    agent::MultiTurnStreamItem,
    completion::{Prompt, ToolDefinition},
    streaming::{StreamedAssistantContent, StreamingPrompt},
    tool::Tool,
};
use serde::{Deserialize, Serialize};
use serde_json::json;

use crate::bit::{Bit, BitModelPreference, BitTypes, LLMParameters};
use crate::flow::board::Board;
use crate::flow::node::Node;
use crate::flow::pin::PinType;
use crate::profile::Profile;
use crate::state::FlowLikeState;
use flow_like_model_provider::llm::ModelLogic;
use flow_like_model_provider::provider::ModelProvider;
use flow_like_types::sync::Mutex;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NodeMetadata {
    pub name: String,
    pub description: String,
    pub inputs: Vec<String>,
    pub outputs: Vec<String>,
}

#[async_trait]
pub trait CatalogProvider: Send + Sync {
    async fn search(&self, query: &str) -> Vec<NodeMetadata>;
    async fn get_all_nodes(&self) -> Vec<String>;
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Edge {
    pub id: String,
    pub from: String,
    pub from_pin: String,
    pub to: String,
    pub to_pin: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Suggestion {
    pub node_type: String,
    pub reason: String,
    pub connection_description: String,
}

#[derive(Debug, Serialize, Deserialize)]
struct GraphContext {
    nodes: Vec<NodeContext>,
    edges: Vec<EdgeContext>,
    selected_nodes: Vec<String>,
}

#[derive(Debug, Serialize, Deserialize)]
struct NodeContext {
    id: String,
    name: String,
    friendly_name: String,
    description: String,
    node_type: String,
    inputs: Vec<PinContext>,
    outputs: Vec<PinContext>,
}

#[derive(Debug, Serialize, Deserialize)]
struct PinContext {
    name: String,
    type_name: String,
    default_value: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
struct EdgeContext {
    from_node: String,
    from_pin: String,
    to_node: String,
    to_pin: String,
}

#[derive(Deserialize)]
struct SearchArgs {
    query: String,
}

#[derive(Debug, thiserror::Error)]
#[error("Catalog tool error")]
struct CatalogToolError;

struct CatalogTool {
    provider: Arc<dyn CatalogProvider>,
}

impl Tool for CatalogTool {
    const NAME: &'static str = "catalog_search";

    type Error = CatalogToolError;
    type Args = SearchArgs;
    type Output = String;

    async fn definition(&self, _prompt: String) -> ToolDefinition {
        ToolDefinition {
            name: "catalog_search".to_string(),
            description: "Search for nodes in the catalog by functionality or name".to_string(),
            parameters: json!({
                "type": "object",
                "properties": {
                    "query": {
                        "type": "string",
                        "description": "Search query"
                    }
                },
                "required": ["query"]
            }),
        }
    }

    async fn call(&self, args: Self::Args) -> Result<Self::Output, Self::Error> {
        let matches = self.provider.search(&args.query).await;

        let matches_json: Vec<_> = matches
            .iter()
            .take(5)
            .map(|n| {
                json!({
                    "name": n.name,
                    "description": n.description,
                    "inputs": n.inputs,
                    "outputs": n.outputs,
                })
            })
            .collect();

        Ok(serde_json::to_string(&matches_json).unwrap_or_default())
    }
}

use futures::StreamExt;

pub struct Copilot {
    state: FlowLikeState,
    catalog_provider: Arc<dyn CatalogProvider>,
    profile: Option<Arc<Profile>>,
}

impl Copilot {
    pub fn new(
        state: FlowLikeState,
        catalog_provider: Arc<dyn CatalogProvider>,
        profile: Option<Arc<Profile>>,
    ) -> Self {
        Self {
            state,
            catalog_provider,
            profile,
        }
    }

    pub async fn autocomplete<F>(
        &self,
        board: &Board,
        selected_node_ids: &[String],
        user_prompt: Option<String>,
        model_id: Option<String>,
        token: Option<String>,
        on_token: Option<F>,
    ) -> Result<Vec<Suggestion>>
    where
        F: Fn(String) + Send + Sync + 'static,
    {
        // 1. Prepare Context
        let context = self.prepare_context(board, selected_node_ids).await?;
        let context_json = flow_like_types::json::to_string_pretty(&context)?;

        // 2. Get Model using ModelFactory
        let bit = if let Some(profile) = &self.profile {
            if let Some(id) = model_id {
                profile
                    .find_bit(&id, self.state.http_client.clone())
                    .await?
            } else {
                let preference = BitModelPreference {
                    reasoning_weight: Some(1.0),
                    ..Default::default()
                };
                profile
                    .get_best_model(&preference, false, true, self.state.http_client.clone())
                    .await?
            }
        } else {
            Bit {
                id: "gpt-4o".to_string(),
                bit_type: BitTypes::Llm,
                parameters: serde_json::to_value(LLMParameters {
                    context_length: 128000,
                    provider: ModelProvider {
                        provider_name: "openai".to_string(),
                        model_id: None,
                        version: None,
                        params: None,
                    },
                    model_classification: Default::default(),
                })
                .unwrap(),
                ..Default::default()
            }
        };

        let (model_name, completion_client) = {
            let model_factory = self.state.model_factory.clone();
            let model = model_factory
                .lock()
                .await
                .build(&bit, Arc::new(Mutex::new(self.state.clone())), token)
                .await?;
            let provider = model.provider().await?;
            let client = provider.client();
            let completion = client
                .as_completion()
                .ok_or_else(|| flow_like_types::anyhow!("Model does not support completion"))?;
            (
                model.default_model().await.unwrap_or("gpt-4o".to_string()),
                completion,
            )
        };

        let available_nodes = self.catalog_provider.get_all_nodes().await;
        let available_nodes_str = available_nodes.join(", ");

        let agent = completion_client.agent(&model_name)
            .preamble(&format!("You are an expert graph editor assistant.
Based on the current graph state and selected nodes, suggest the next logical nodes to add.
You have access to a catalog of nodes. Use the 'catalog_search' tool to find relevant nodes if you are unsure what is available.

Available Node Types: {}

If the user asks for suggestions or 'autocomplete', output a JSON list of suggestions with 'node_type', 'reason', and 'connection_description'.
Do NOT output markdown code blocks, just the raw JSON array.
If the user asks a general question or wants to chat, answer normally in text.
", available_nodes_str))
            .tool(CatalogTool { provider: self.catalog_provider.clone() })
            .build();

        let prompt = format!(
            "User Intent: {:?}

Graph Context:
{}

Find the best nodes to add next.
",
            user_prompt, context_json
        );

        let response = if let Some(callback) = on_token {
            let mut stream = agent.stream_prompt(prompt.clone()).await;
            let mut full_response = String::new();
            while let Some(chunk) = stream.next().await {
                match chunk {
                    Ok(item) => {
                        // Try to match Text variant, compiler will correct me if wrong
                        if let MultiTurnStreamItem::StreamAssistantItem(
                            StreamedAssistantContent::Text(token),
                        ) = item
                        {
                            full_response.push_str(&token.to_string());
                            callback(token.to_string());
                        }
                    }
                    Err(e) => {
                        println!("Error in stream: {}", e);
                    }
                }
            }
            full_response
        } else {
            agent
                .prompt(&prompt)
                .await
                .map_err(|e| flow_like_types::anyhow!("Rig error: {}", e))?
        };

        // Parse response - expecting JSON
        // We might need to clean the response if it contains markdown code blocks
        let clean_response = response
            .trim()
            .trim_start_matches("```json")
            .trim_start_matches("```")
            .trim_end_matches("```");

        let suggestions: Vec<Suggestion> =
            flow_like_types::json::from_str(clean_response).unwrap_or_default();
        Ok(suggestions)
    }

    async fn prepare_context(
        &self,
        board: &Board,
        selected_node_ids: &[String],
    ) -> Result<GraphContext> {
        let mut node_contexts = Vec::new();
        let mut pin_to_node_map = std::collections::HashMap::new();

        // Helper to process nodes
        let mut process_nodes = |nodes: &std::collections::HashMap<String, Node>| {
            for node in nodes.values() {
                for pin_id in node.pins.keys() {
                    pin_to_node_map.insert(pin_id.clone(), node.id.clone());
                }
            }
        };

        // Build pin to node map for root nodes
        process_nodes(&board.nodes);
        // Build pin to node map for layer nodes
        for layer in board.layers.values() {
            process_nodes(&layer.nodes);
        }

        // Helper to create context
        let mut create_node_contexts = |nodes: &std::collections::HashMap<String, Node>| {
            for node in nodes.values() {
                let inputs = node
                    .pins
                    .values()
                    .filter(|p| p.pin_type == PinType::Input)
                    .map(|p| PinContext {
                        name: p.name.clone(),
                        type_name: format!("{:?}", p.data_type),
                        default_value: p
                            .default_value
                            .as_ref()
                            .map(|v| String::from_utf8_lossy(v).to_string()),
                    })
                    .collect();

                let outputs = node
                    .pins
                    .values()
                    .filter(|p| p.pin_type == PinType::Output)
                    .map(|p| PinContext {
                        name: p.name.clone(),
                        type_name: format!("{:?}", p.data_type),
                        default_value: p
                            .default_value
                            .as_ref()
                            .map(|v| String::from_utf8_lossy(v).to_string()),
                    })
                    .collect();

                node_contexts.push(NodeContext {
                    id: node.id.clone(),
                    name: node.name.clone(),
                    friendly_name: node.friendly_name.clone(),
                    description: node.description.clone(),
                    node_type: node.name.clone(),
                    inputs,
                    outputs,
                });
            }
        };

        create_node_contexts(&board.nodes);
        for layer in board.layers.values() {
            create_node_contexts(&layer.nodes);
        }

        let mut edge_contexts = Vec::new();

        let mut process_edges = |nodes: &std::collections::HashMap<String, Node>| {
            for node in nodes.values() {
                for pin in node.pins.values() {
                    // We only care about outgoing connections to avoid duplicates
                    if pin.pin_type == PinType::Output {
                        for connected_pin_id in &pin.connected_to {
                            if let Some(target_node_id) = pin_to_node_map.get(connected_pin_id) {
                                let target_pin = board.get_pin_by_id(connected_pin_id);
                                edge_contexts.push(EdgeContext {
                                    from_node: node.id.clone(),
                                    from_pin: pin.name.clone(),
                                    to_node: target_node_id.clone(),
                                    to_pin: target_pin.map(|p| p.name.clone()).unwrap_or_default(),
                                });
                            }
                        }
                    }
                }
            }
        };

        process_edges(&board.nodes);
        for layer in board.layers.values() {
            process_edges(&layer.nodes);
        }

        Ok(GraphContext {
            nodes: node_contexts,
            edges: edge_contexts,
            selected_nodes: selected_node_ids.to_vec(),
        })
    }
}
