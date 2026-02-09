use crate::state::{TauriFlowLikeState, TauriSettingsState};
use async_trait::async_trait;
use flow_like::a2ui::SurfaceComponent;
use flow_like::copilot::{
    CopilotScope, UIActionContext, UnifiedChatMessage, UnifiedContext, UnifiedCopilot,
    UnifiedCopilotResponse,
};
use flow_like::flow::board::Board;
use flow_like::flow::copilot::{
    BoardCommand, CatalogProvider, NodeMetadata, PinMetadata, RunContext,
};
use flow_like::flow::pin::{Pin, PinType};
use flow_like::flow::variable::VariableType;
use flow_like_catalog::get_catalog;
use std::sync::Arc;
use tauri::{AppHandle, State, ipc::Channel};

/// Desktop implementation of the catalog provider for node search
struct DesktopCatalogProvider {
    _state: TauriFlowLikeState,
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
        schema: p.schema.clone(),
        is_generic,
        valid_values,
        enforce_schema,
    }
}

#[async_trait]
impl CatalogProvider for DesktopCatalogProvider {
    async fn search(&self, query: &str) -> Vec<NodeMetadata> {
        let catalog = get_catalog();
        let query_lower = query.to_lowercase();
        let query_tokens: Vec<&str> = query_lower.split_whitespace().collect();

        let mut scored_matches: Vec<(i32, NodeMetadata)> = Vec::new();

        for logic in catalog {
            let node = logic.get_node();
            let name_lower = node.name.to_lowercase();
            let friendly_lower = node.friendly_name.to_lowercase();
            let desc_lower = node.description.to_lowercase();

            let category = name_lower.split("::").nth(1).unwrap_or("");

            let mut score = 0i32;

            if name_lower.contains(&query_lower) {
                score += 100;
            }
            if friendly_lower.contains(&query_lower) {
                score += 90;
            }

            for token in &query_tokens {
                if name_lower.contains(token) {
                    score += 30;
                }
                if friendly_lower.contains(token) {
                    score += 25;
                }
                if category.contains(token) {
                    score += 20;
                }
                if desc_lower.contains(token) {
                    score += 10;
                }
            }

            let name_parts: Vec<&str> = name_lower.split([':', '_']).collect();
            for token in &query_tokens {
                if name_parts.iter().any(|part| part == token) {
                    score += 15;
                }
            }

            if score > 0 {
                scored_matches.push((
                    score,
                    NodeMetadata {
                        name: node.name,
                        friendly_name: node.friendly_name,
                        description: node.description,
                        inputs: node
                            .pins
                            .values()
                            .filter(|p| p.pin_type == PinType::Input)
                            .map(pin_to_metadata)
                            .collect(),
                        outputs: node
                            .pins
                            .values()
                            .filter(|p| p.pin_type == PinType::Output)
                            .map(pin_to_metadata)
                            .collect(),
                        category: Some(category.to_string()),
                    },
                ));
            }
        }

        scored_matches.sort_by(|a, b| b.0.cmp(&a.0));
        scored_matches
            .into_iter()
            .take(10)
            .map(|(_, meta)| meta)
            .collect()
    }

    async fn search_by_pin_type(&self, pin_type: &str, is_input: bool) -> Vec<NodeMetadata> {
        let catalog = get_catalog();
        let pin_type = pin_type.to_lowercase();
        let mut matches = Vec::new();

        for logic in catalog {
            let node = logic.get_node();
            let name_lower = node.name.to_lowercase();
            let category = name_lower.split("::").nth(1).unwrap_or("");

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
                matches.push(NodeMetadata {
                    name: node.name,
                    friendly_name: node.friendly_name,
                    description: node.description,
                    inputs: node
                        .pins
                        .values()
                        .filter(|p| p.pin_type == PinType::Input)
                        .map(pin_to_metadata)
                        .collect(),
                    outputs: node
                        .pins
                        .values()
                        .filter(|p| p.pin_type == PinType::Output)
                        .map(pin_to_metadata)
                        .collect(),
                    category: Some(category.to_string()),
                });
            }
            if matches.len() >= 10 {
                break;
            }
        }
        matches
    }

    async fn filter_by_category(&self, category_prefix: &str) -> Vec<NodeMetadata> {
        let catalog = get_catalog();
        let category_prefix = category_prefix.to_lowercase();
        let mut matches = Vec::new();

        for logic in catalog {
            let node = logic.get_node();
            let name_lower = node.name.to_lowercase();
            let category = name_lower.split("::").nth(1).unwrap_or("");

            if category.starts_with(&category_prefix) || name_lower.contains(&category_prefix) {
                matches.push(NodeMetadata {
                    name: node.name,
                    friendly_name: node.friendly_name,
                    description: node.description,
                    inputs: node
                        .pins
                        .values()
                        .filter(|p| p.pin_type == PinType::Input)
                        .map(pin_to_metadata)
                        .collect(),
                    outputs: node
                        .pins
                        .values()
                        .filter(|p| p.pin_type == PinType::Output)
                        .map(pin_to_metadata)
                        .collect(),
                    category: Some(category.to_string()),
                });
            }
            if matches.len() >= 15 {
                break;
            }
        }
        matches
    }

    async fn get_all_nodes(&self) -> Vec<String> {
        let catalog = get_catalog();
        catalog.iter().map(|logic| logic.get_node().name).collect()
    }
}

/// Unified copilot chat command that handles both board and UI generation
#[tauri::command]
pub async fn copilot_chat(
    app_handle: AppHandle,
    state: State<'_, TauriFlowLikeState>,
    // Scope selection
    scope: CopilotScope,
    // Board context (optional for Frontend scope)
    board: Option<Board>,
    selected_node_ids: Option<Vec<String>>,
    // UI context (optional for Board scope)
    current_surface: Option<Vec<SurfaceComponent>>,
    selected_component_ids: Option<Vec<String>>,
    // Common parameters
    user_prompt: String,
    history: Option<Vec<UnifiedChatMessage>>,
    model_id: Option<String>,
    token: Option<String>,
    // Extended context
    run_context: Option<RunContext>,
    action_context: Option<UIActionContext>,
    // Streaming channel
    channel: Channel<String>,
) -> Result<UnifiedCopilotResponse, String> {
    // Check if using Copilot SDK (model_id starts with "copilot:")
    if let Some(ref id) = model_id
        && let Some(copilot_model) = id.strip_prefix("copilot:")
    {
        return copilot_sdk_chat_internal(
            copilot_model,
            scope,
            board.as_ref(),
            selected_node_ids.as_deref().unwrap_or(&[]),
            current_surface.as_ref(),
            user_prompt,
            history.unwrap_or_default(),
            channel,
        )
        .await;
    }

    println!(
        "[copilot_chat] Called with scope: {:?}, run_context: {:?}",
        scope, run_context
    );

    let selected_node_ids = selected_node_ids.unwrap_or_default();
    let selected_component_ids = selected_component_ids.unwrap_or_default();
    let history = history.unwrap_or_default();

    let state_clone = state.0.clone();

    let profile = TauriSettingsState::current_profile(&app_handle)
        .await
        .ok()
        .map(|p| Arc::new(p.hub_profile));

    // Only create catalog provider if we might need it (Board or Both scope)
    let catalog_provider: Option<Arc<dyn CatalogProvider>> = match scope {
        CopilotScope::Frontend => None,
        _ => Some(Arc::new(DesktopCatalogProvider {
            _state: state.inner().clone(),
        })),
    };

    let copilot = UnifiedCopilot::new(state_clone, catalog_provider, profile, None)
        .await
        .map_err(|e| e.to_string())?;

    let on_token = Some(move |token: String| {
        let _ = channel.send(token);
    });

    // Build unified context
    let context = if run_context.is_some() || action_context.is_some() {
        Some(UnifiedContext {
            scope,
            run_context,
            action_context,
        })
    } else {
        None
    };

    copilot
        .chat(
            scope,
            board.as_ref(),
            &selected_node_ids,
            current_surface.as_ref(),
            &selected_component_ids,
            user_prompt,
            history,
            model_id,
            token,
            context,
            on_token,
        )
        .await
        .map_err(|e| e.to_string())
}

/// Internal function to handle Copilot SDK chat
async fn copilot_sdk_chat_internal(
    model_id: &str,
    scope: CopilotScope,
    board: Option<&Board>,
    selected_node_ids: &[String],
    current_surface: Option<&Vec<SurfaceComponent>>,
    user_prompt: String,
    history: Vec<UnifiedChatMessage>,
    channel: Channel<String>,
) -> Result<UnifiedCopilotResponse, String> {
    use super::copilot_sdk_tools::{create_board_tools, create_frontend_tools};
    use copilot_sdk::SessionEventData;
    use flow_like::flow::copilot::prepare_context;

    let guard = COPILOT_CLIENT.lock().await;
    let client = guard
        .as_ref()
        .ok_or("Copilot SDK not running. Please start it first.")?;

    // Build graph context for board tools (only if in Board or Both scope)
    let graph_context = match scope {
        CopilotScope::Board | CopilotScope::Both => {
            if let Some(board) = board {
                prepare_context(board, selected_node_ids).ok().map(Arc::new)
            } else {
                None
            }
        }
        CopilotScope::Frontend => None,
    };

    // Create tools based on scope
    let tools: Vec<(copilot_sdk::Tool, copilot_sdk::ToolHandler)> = match scope {
        CopilotScope::Board => create_board_tools(graph_context),
        CopilotScope::Frontend => create_frontend_tools(),
        CopilotScope::Both => {
            let mut all_tools = create_board_tools(graph_context);
            all_tools.extend(create_frontend_tools());
            all_tools
        }
    };

    // Extract just the Tool definitions for SessionConfig
    let tool_defs: Vec<copilot_sdk::Tool> = tools.iter().map(|(t, _)| t.clone()).collect();

    // Build context from history for the system message
    let mut context_parts = vec![];
    for msg in &history {
        let role = match msg.role {
            flow_like::flow::copilot::ChatRole::User => "User",
            flow_like::flow::copilot::ChatRole::Assistant => "Assistant",
        };
        context_parts.push(format!("{}: {}", role, msg.content));
    }

    // Build system prompt - use specialized prompt based on scope
    let mut system_content = match scope {
        CopilotScope::Board => BOARD_AGENT_PROMPT.to_string(),
        CopilotScope::Frontend => A2UI_AGENT_PROMPT.to_string(),
        CopilotScope::Both => {
            let mut s = GENERAL_AGENT_PROMPT.to_string();
            s.push_str("\n\nYou are working in UNIFIED mode - you can help with both workflow automation and UI components.");
            s.push_str("\n\nFor workflows: Use emit_commands tool with AddNode, ConnectPins, UpdateNodePin");
            s.push_str("\nFor UI: Use emit_ui tool with A2UI JSON format (NOT file editing)");
            s
        }
    };

    // Add current UI surface context for Frontend/Both scopes
    if matches!(scope, CopilotScope::Frontend | CopilotScope::Both)
        && let Some(components) = current_surface
        && !components.is_empty()
    {
        let components_json =
            serde_json::to_string_pretty(components).unwrap_or_else(|_| "[]".to_string());
        system_content.push_str(&format!(
                    "\n\n## CURRENT UI COMPONENTS\nThe user has the following existing UI. You can modify or extend it:\n```json\n{}\n```",
                    components_json
                ));
    }

    // Add conversation history
    if !context_parts.is_empty() {
        system_content.push_str(&format!(
            "\n\nConversation history:\n{}",
            context_parts.join("\n\n")
        ));
    }

    // For Frontend mode, restrict to ONLY emit_ui tool and exclude file editing tools
    let (available_tools, excluded_tools) = match scope {
        CopilotScope::Frontend => (
            Some(vec!["emit_ui".to_string()]),
            Some(vec![
                "Read".to_string(),
                "Edit".to_string(),
                "Write".to_string(),
                "shell".to_string(),
                "powershell".to_string(),
                "bash".to_string(),
                "Grep".to_string(),
            ]),
        ),
        _ => (None, None),
    };

    let config = copilot_sdk::SessionConfig {
        model: Some(model_id.to_string()),
        streaming: true,
        tools: tool_defs,
        available_tools,
        excluded_tools,
        system_message: Some(copilot_sdk::SystemMessageConfig {
            content: Some(system_content),
            mode: Some(copilot_sdk::SystemMessageMode::Replace),
        }),
        infinite_sessions: Some(copilot_sdk::InfiniteSessionConfig::enabled()),
        ..Default::default()
    };

    let session = client
        .create_session(config)
        .await
        .map_err(|e| format!("Failed to create session: {}", e))?;

    // Register tool handlers
    for (tool, handler) in tools {
        session
            .register_tool_with_handler(tool, Some(handler))
            .await;
    }

    let mut events = session.subscribe();
    session
        .send(user_prompt.as_str())
        .await
        .map_err(|e| format!("Failed to send message: {}", e))?;

    let mut full_response = String::new();
    let mut extracted_commands: Vec<BoardCommand> = Vec::new();
    let mut extracted_components: Vec<SurfaceComponent> = Vec::new();
    let mut extracted_canvas_settings: Option<serde_json::Value> = None;
    let mut extracted_root_component_id: Option<String> = None;

    loop {
        match events.recv().await {
            Ok(event) => match &event.data {
                SessionEventData::AssistantMessageDelta(delta) => {
                    full_response.push_str(&delta.delta_content);
                    let _ = channel.send(delta.delta_content.clone());
                }
                SessionEventData::AssistantMessage(msg) => {
                    // Don't overwrite accumulated content unless it's truly final
                    if full_response.is_empty() {
                        full_response = msg.content.clone();
                    }
                }
                SessionEventData::ToolExecutionStart(tool_event) => {
                    // Send tool start event to frontend
                    let tool_msg = format!(
                        "<tool_start>{{\"tool\":\"{}\",\"status\":\"running\"}}</tool_start>",
                        tool_event.tool_name
                    );
                    let _ = channel.send(tool_msg);
                }
                SessionEventData::ToolExecutionComplete(tool_complete) => {
                    if let Some(ref result) = tool_complete.result
                        && let Ok(parsed) =
                            serde_json::from_str::<serde_json::Value>(&result.content)
                    {
                        // Extract commands from emit_commands tool (status: "queued")
                        if parsed.get("status").and_then(|s| s.as_str()) == Some("queued")
                            && let Some(cmds) = parsed.get("commands")
                            && let Ok(commands) =
                                serde_json::from_value::<Vec<BoardCommand>>(cmds.clone())
                        {
                            let cmd_event = format!(
                                "<commands>{}</commands>",
                                serde_json::to_string(&commands).unwrap_or_default()
                            );
                            let _ = channel.send(cmd_event);
                            extracted_commands.extend(commands);
                        }
                        // Extract components from emit_ui tool (status: "rendered")
                        if parsed.get("status").and_then(|s| s.as_str()) == Some("rendered") {
                            // Extract canvasSettings
                            if let Some(canvas) = parsed.get("canvasSettings") {
                                extracted_canvas_settings = Some(canvas.clone());
                            }
                            // Extract rootComponentId
                            if let Some(root_id) =
                                parsed.get("rootComponentId").and_then(|v| v.as_str())
                            {
                                extracted_root_component_id = Some(root_id.to_string());
                            }
                            // Extract components
                            if let Some(comps) = parsed.get("components")
                                && let Ok(components) =
                                    serde_json::from_value::<Vec<SurfaceComponent>>(comps.clone())
                            {
                                // Send components WITH canvas settings to frontend
                                let comp_event = format!(
                                    "<components>{}</components>",
                                    serde_json::to_string(&components).unwrap_or_default()
                                );
                                let _ = channel.send(comp_event);
                                // Also send canvas settings
                                if let Some(ref canvas) = extracted_canvas_settings {
                                    let canvas_event = format!(
                                        "<canvas_settings>{}</canvas_settings>",
                                        serde_json::to_string(canvas).unwrap_or_default()
                                    );
                                    let _ = channel.send(canvas_event);
                                }
                                extracted_components.extend(components);
                            }
                        }
                    }

                    // Send tool completion event to frontend
                    let status = if tool_complete.success {
                        "done"
                    } else {
                        "error"
                    };
                    let tool_msg = format!(
                        "<tool_end>{{\"tool_call_id\":\"{}\",\"status\":\"{}\"}}</tool_end>",
                        tool_complete.tool_call_id, status
                    );
                    let _ = channel.send(tool_msg);
                }
                SessionEventData::SessionIdle(_) => {
                    break;
                }
                SessionEventData::SessionError(err) => {
                    return Err(format!("Session error: {:?}", err));
                }
                _ => {}
            },
            Err(e) => {
                println!("[copilot_sdk_chat] Event receive error: {}", e);
                break;
            }
        }
    }

    Ok(UnifiedCopilotResponse {
        message: full_response,
        commands: extracted_commands,
        suggestions: vec![],
        components: extracted_components,
        canvas_settings: extracted_canvas_settings,
        root_component_id: extracted_root_component_id,
        active_scope: scope,
    })
}

// =============================================================================
// GitHub Copilot SDK Direct Integration
// =============================================================================

use copilot_sdk::{Client, LogLevel};
use once_cell::sync::Lazy;
use serde::{Deserialize, Serialize};
use tokio::sync::Mutex;

/// Global Copilot client instance (singleton) - uses tokio::sync::Mutex for async safety
static COPILOT_CLIENT: Lazy<Mutex<Option<Client>>> = Lazy::new(|| Mutex::new(None));

/// Model info returned from GitHub Copilot
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CopilotModelInfo {
    pub id: String,
    pub name: String,
}

/// Auth status returned from GitHub Copilot
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CopilotAuthStatus {
    pub authenticated: bool,
    pub login: Option<String>,
}

/// Start the GitHub Copilot SDK client
#[tauri::command]
pub async fn copilot_sdk_start(
    use_stdio: Option<bool>,
    cli_url: Option<String>,
) -> Result<(), String> {
    let use_stdio = use_stdio.unwrap_or(true);

    let mut builder = Client::builder()
        .use_stdio(use_stdio)
        .log_level(LogLevel::Error);

    if let Some(url) = cli_url {
        builder = builder.cli_url(url);
    }

    let client = builder
        .build()
        .map_err(|e| format!("Failed to build Copilot client: {}", e))?;
    client
        .start()
        .await
        .map_err(|e| format!("Failed to start Copilot client: {}", e))?;

    let mut guard = COPILOT_CLIENT.lock().await;
    *guard = Some(client);

    Ok(())
}

/// Stop the GitHub Copilot SDK client
#[tauri::command]
pub async fn copilot_sdk_stop() -> Result<(), String> {
    let client = {
        let mut guard = COPILOT_CLIENT.lock().await;
        guard.take()
    };

    if let Some(client) = client {
        client
            .stop()
            .await
            .map_err(|e| format!("Failed to stop Copilot client: {}", e))?;
    }

    Ok(())
}

/// Check if the Copilot SDK client is running
#[tauri::command]
pub async fn copilot_sdk_is_running() -> Result<bool, String> {
    let guard = COPILOT_CLIENT.lock().await;
    Ok(guard.is_some())
}

/// List available GitHub Copilot models
#[tauri::command]
pub async fn copilot_sdk_list_models() -> Result<Vec<CopilotModelInfo>, String> {
    let guard = COPILOT_CLIENT.lock().await;
    let client = guard.as_ref().ok_or("Copilot client not started")?;
    let models = client
        .list_models()
        .await
        .map_err(|e| format!("Failed to list models: {}", e))?;

    Ok(models
        .iter()
        .map(|m| CopilotModelInfo {
            id: m.id.clone(),
            name: m.name.clone(),
        })
        .collect())
}

/// Get GitHub Copilot authentication status
#[tauri::command]
pub async fn copilot_sdk_get_auth_status() -> Result<CopilotAuthStatus, String> {
    let guard = COPILOT_CLIENT.lock().await;
    let client = guard.as_ref().ok_or("Copilot client not started")?;
    let status = client
        .get_auth_status()
        .await
        .map_err(|e| format!("Failed to get auth status: {}", e))?;

    Ok(CopilotAuthStatus {
        authenticated: status.is_authenticated,
        login: status.login.clone(),
    })
}

// =============================================================================
// Specialized Agents Configuration
// =============================================================================

/// Specialized agent type
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum SpecializedAgentType {
    General,
    Frontend,
    Backend,
}

/// System prompts for specialized agents
const FRONTEND_AGENT_PROMPT: &str = r#"You are a specialized FRONTEND development agent focused on UI/UX.

Your expertise:
- Creating responsive, accessible UI components
- React patterns and hooks
- CSS/Tailwind styling
- User experience design
- Component composition and state management
- A2UI component system

When working with workflows:
- Suggest UI components that complement workflow outputs
- Design user interfaces for workflow inputs/outputs
- Create dashboards for monitoring workflow results

Always prioritize:
- User experience and accessibility
- Clean, maintainable component structure
- Responsive design patterns
- Performance optimization"#;

const BACKEND_AGENT_PROMPT: &str = r#"You are a specialized BACKEND workflow agent focused on data processing and automation.

Your expertise:
- Flow graph design and node connections
- Data transformation and processing
- API integrations and webhooks
- Error handling and retry logic
- Performance optimization for workflows
- Event-driven architecture

When working with UI:
- Suggest workflow nodes that power UI features
- Design data flows that feed into UI components
- Create automation that responds to user actions

Always prioritize:
- Data integrity and validation
- Efficient execution paths
- Proper error handling
- Scalable architecture patterns"#;

const GENERAL_AGENT_PROMPT: &str = r#"You are an expert development assistant capable of both frontend UI and backend workflow development.

You can seamlessly switch between:
- Creating visual UI components (A2UI)
- Designing workflow graphs (nodes, connections)
- Integrating UI with workflows

Analyze the user's request and determine whether it requires:
- UI work (components, layouts, styling)
- Workflow work (nodes, data processing)
- Both (integrated solutions)"#;

const BOARD_AGENT_PROMPT: &str = r#"You are an expert workflow/graph editor assistant. You help users create and modify visual workflow automations.

## CRITICAL WORKFLOW - Follow These Steps:

### Step 1: Search Catalog First
Before adding ANY node, use `catalog_search` to find the exact `node_type`.
- Query by functionality: "http request", "parse json", "loop", "condition"
- The result gives you the exact `node_type` string needed for AddNode

### Step 2: Inspect Existing Nodes
Use `get_node_details` on existing nodes to:
- Get their exact position (for placing new nodes nearby)
- Get their exact pin names (needed for connections)
- Understand what inputs/outputs they have

### Step 3: Emit Commands Together
Always batch related commands in a single `emit_commands` call:
1. AddNode commands FIRST (create all needed nodes)
2. ConnectPins commands (wire execution and data flow)
3. UpdateNodePin commands LAST (set default values)

## NODE POSITIONING RULES
- Place new nodes NEAR related nodes (within 250-300px)
- Use horizontal flow: left-to-right execution
- Standard spacing: x+250 for horizontal, y+150 for vertical
- If connecting TO an existing node, place new node to its LEFT
- If connecting FROM an existing node, place new node to its RIGHT
- Example: If existing node is at {x: 500, y: 200}, place connected node at {x: 750, y: 200}

## CONNECTION RULES
- ALWAYS connect execution flow: from_node.exec_out → to_node.exec_in
- Connect data pins by matching types
- Use EXACT pin names from `get_node_details` (case-sensitive!)
- ref_ids: Use '$0', '$1', '$2' to reference nodes created in same batch

## PIN VALUES
- Use `UpdateNodePin` to set required input values
- pin_id is the pin NAME (not ID), like "url", "method", "body"
- value must be JSON: strings as `"value"`, numbers as `123`, booleans as `true`

## EXAMPLE WORKFLOW: "Make HTTP GET request and parse JSON"

1. catalog_search("http request") → finds "http::request::send_request"
2. catalog_search("parse json") → finds "data::json::parse"
3. emit_commands:
```json
{
  "commands": [
    {"command_type": "AddNode", "node_type": "http::request::send_request", "ref_id": "$0", "position": {"x": 300, "y": 200}, "summary": "HTTP request node"},
    {"command_type": "AddNode", "node_type": "data::json::parse", "ref_id": "$1", "position": {"x": 550, "y": 200}, "summary": "JSON parser"},
    {"command_type": "ConnectPins", "from_node": "$0", "from_pin": "exec_out", "to_node": "$1", "to_pin": "exec_in", "summary": "Connect execution"},
    {"command_type": "ConnectPins", "from_node": "$0", "from_pin": "response_body", "to_node": "$1", "to_pin": "json_string", "summary": "Pass response to parser"},
    {"command_type": "UpdateNodePin", "node_id": "$0", "pin_id": "url", "value": "https://api.example.com/data", "summary": "Set URL"},
    {"command_type": "UpdateNodePin", "node_id": "$0", "pin_id": "method", "value": "GET", "summary": "Set method"}
  ],
  "explanation": "Created HTTP request → JSON parse workflow"
}
```

## KEY RULES
1. NEVER guess node_type - always use catalog_search first
2. NEVER guess pin names - use get_node_details to find exact names
3. ALWAYS include position in AddNode (near related nodes)
4. ALWAYS connect exec_out → exec_in for execution flow
5. ALWAYS set required pin values with UpdateNodePin
6. Use ref_ids ($0, $1, $2...) to reference new nodes in same batch
7. Each command needs a "summary" field

## COMMAND TYPES REFERENCE
- AddNode: {command_type, node_type, ref_id, position: {x, y}, summary}
- ConnectPins: {command_type, from_node, from_pin, to_node, to_pin, summary}
- UpdateNodePin: {command_type, node_id, pin_id, value, summary}
- RemoveNode: {command_type, node_id, summary}
- AddPlaceholder: {command_type, name, ref_id, position, pins?, summary}
- CreateVariable: {command_type, name, data_type, value_type, summary}
- CreateComment: {command_type, content, position, summary}"#;

/// A2UI Agent prompt - for frontend/UI generation mode
/// CRITICAL: This generates A2UI JSON, NOT file edits!
const A2UI_AGENT_PROMPT: &str = r#"# CRITICAL: YOU MUST CALL THE emit_ui TOOL

You are a UI generator. Your ONLY action is to call the emit_ui tool with JSON. Text responses do NOTHING - the UI will not render.

WORKFLOW:
1. Read what the user wants
2. IMMEDIATELY call emit_ui with complete JSON
3. DO NOT explain, describe, or ask questions

## emit_ui TOOL SCHEMA
{
  "rootComponentId": "root",
  "canvasSettings": {
    "backgroundColor": "bg-background",
    "padding": "1rem",
    "customCss": ".my-class { color: red; }"
  },
  "components": [...]
}

## COMPONENT FORMAT
{
  "id": "unique-id",
  "style": {"className": "tailwind classes AND/OR custom class names"},
  "component": {"type": "componentType", ...props}
}

## BOUNDVALUE - ALL props MUST use this format
- String: {"literalString": "text"}
- Number: {"literalNumber": 42}
- Boolean: {"literalBool": true}
- JSON data: {"literalJson": "[{\"x\": 1, \"y\": 2}]"}
- Options: {"literalOptions": [{"value": "v", "label": "L"}]}
- Children: {"explicitList": ["child-id-1", "child-id-2"]}

---
## ALL AVAILABLE COMPONENTS (60+)

### Layout
- `column` - Vertical flex (gap, align, justify, wrap, reverse, children)
- `row` - Horizontal flex (gap, align, justify, wrap, reverse, children)
- `grid` - CSS Grid (columns, rows, gap, autoFlow, children)
- `stack` - Z-axis layering (align, children) - REQUIRES min-height!
- `scrollArea` - Scrollable (direction: "vertical"|"horizontal"|"both", children)
- `absolute` - Free positioning (width, height, children)
- `aspectRatio` - Maintain ratio (ratio, children)
- `overlay` - Position over base (children)
- `box` - Semantic container (semanticRole, children)
- `center` - Center content (children)
- `spacer` - Spacing (size, direction, flexible)

### Display
- `text` - Typography (content, variant: "p"|"h1"|"h2"|"h3"|"h4"|"lead"|"large"|"small"|"muted"|"code"|"blockquote")
- `image` - Image (src, alt, width, height, fit, fallbackSrc)
- `icon` - Lucide icons (name, size, color)
- `video` - Video player (src, poster, autoPlay, controls, loop, muted)
- `lottie` - Animations (src, autoplay, loop, speed)
- `markdown` - Markdown renderer (content)
- `badge` - Label (text, variant: "default"|"secondary"|"destructive"|"outline")
- `avatar` - User avatar (src, fallback, size)
- `progress` - Progress bar (value, max, variant)
- `spinner` - Loading (size)
- `divider` - Separator (orientation: "horizontal"|"vertical")
- `skeleton` - Loading placeholder (variant: "text"|"circular"|"rectangular", width, height)

### Interactive
- `button` - Clickable (label, variant: "default"|"destructive"|"outline"|"secondary"|"ghost"|"link", size, disabled, loading)
- `textField` - Text input (value, placeholder, label, type: "text"|"email"|"password"|"number"|"tel"|"url", disabled)
- `select` - Dropdown (value, options, placeholder, label, disabled)
- `slider` - Range (value, min, max, step, label)
- `checkbox` - Boolean (checked, label, disabled)
- `switch` - Toggle (checked, label, disabled)
- `radioGroup` - Radio (value, options, orientation)
- `dateTimeInput` - Date/time picker (value, label, mode: "date"|"time"|"datetime")
- `fileInput` - File upload (accept, multiple, label)
- `imageInput` - Image upload (value, accept, showPreview)
- `link` - Navigation (href, text, openInNewTab, variant)

### Container
- `card` - Content card (children)
- `modal` - Dialog overlay (open, title, description, children)
- `tabs` - Tabbed content (defaultValue, tabs: [{value, label, content: children}])
- `accordion` - Collapsible (type: "single"|"multiple", items: [{value, trigger, content}])
- `drawer` - Slide panel (open, side: "left"|"right"|"top"|"bottom", title, children)
- `tooltip` - Hover tip (content, children)
- `popover` - Click popup (trigger, content)

### Data Display
- `table` - Data table (columns: [{key, label, sortable?}], data, pageSize, sortable, showPagination)
- `iframe` - Embedded content (src, width, height, sandbox, allow)
- `filePreview` - File viewer (url, mimeType, width, height)

### Charts (Nivo - 25+ types)
- `nivoChart` - Nivo charts (chartType, data, height, colors, showLegend, plus chart-specific style)

**Chart Types & Data Formats:**

**bar** - Bar chart
```json
{"chartType": {"literalString": "bar"}, "data": {"literalJson": "[{\"category\": \"A\", \"value\": 10}, {\"category\": \"B\", \"value\": 20}]"}, "indexBy": {"literalString": "category"}, "keys": {"literalJson": "[\"value\"]"}, "barStyle": {"literalJson": "{\"groupMode\": \"grouped\", \"layout\": \"vertical\", \"padding\": 0.3}"}}
```

**line** - Line chart (series format)
```json
{"chartType": {"literalString": "line"}, "data": {"literalJson": "[{\"id\": \"Series A\", \"data\": [{\"x\": \"Jan\", \"y\": 10}, {\"x\": \"Feb\", \"y\": 20}]}]"}, "lineStyle": {"literalJson": "{\"curve\": \"monotoneX\", \"enableArea\": true, \"enablePoints\": true}"}}
```

**pie** - Pie/donut chart
```json
{"chartType": {"literalString": "pie"}, "data": {"literalJson": "[{\"id\": \"A\", \"value\": 30}, {\"id\": \"B\", \"value\": 50}]"}, "pieStyle": {"literalJson": "{\"innerRadius\": 0.5, \"padAngle\": 0.7, \"cornerRadius\": 3}"}}
```

**radar** - Radar/spider chart
```json
{"chartType": {"literalString": "radar"}, "data": {"literalJson": "[{\"skill\": \"JS\", \"person1\": 90, \"person2\": 70}]"}, "indexBy": {"literalString": "skill"}, "keys": {"literalJson": "[\"person1\", \"person2\"]"}}
```

**heatmap** - Heatmap grid
```json
{"chartType": {"literalString": "heatmap"}, "data": {"literalJson": "[{\"id\": \"Row1\", \"data\": [{\"x\": \"Col1\", \"y\": 10}]}]"}}
```

**scatter** - Scatter plot
```json
{"chartType": {"literalString": "scatter"}, "data": {"literalJson": "[{\"id\": \"Group\", \"data\": [{\"x\": 10, \"y\": 20}]}]"}}
```

**funnel** - Funnel chart
```json
{"chartType": {"literalString": "funnel"}, "data": {"literalJson": "[{\"id\": \"Visitors\", \"value\": 10000}, {\"id\": \"Leads\", \"value\": 3000}]"}}
```

**treemap** - Treemap (hierarchical)
```json
{"chartType": {"literalString": "treemap"}, "data": {"literalJson": "{\"name\": \"root\", \"children\": [{\"name\": \"A\", \"value\": 100}]}"}}
```

**sunburst** - Sunburst (hierarchical)
```json
{"chartType": {"literalString": "sunburst"}, "data": {"literalJson": "{\"name\": \"root\", \"children\": [{\"name\": \"A\", \"value\": 50}]}"}}
```

**calendar** - Calendar heatmap
```json
{"chartType": {"literalString": "calendar"}, "data": {"literalJson": "[{\"day\": \"2024-01-01\", \"value\": 10}]"}}
```

**sankey** - Sankey flow diagram
```json
{"chartType": {"literalString": "sankey"}, "data": {"literalJson": "{\"nodes\": [{\"id\": \"A\"}, {\"id\": \"B\"}], \"links\": [{\"source\": \"A\", \"target\": \"B\", \"value\": 100}]}"}}
```

**chord** - Chord diagram (matrix)
```json
{"chartType": {"literalString": "chord"}, "data": {"literalJson": "[[100, 30], [30, 80]]"}, "keys": {"literalJson": "[\"A\", \"B\"]"}}
```

**bump/areaBump** - Ranking over time
```json
{"chartType": {"literalString": "bump"}, "data": {"literalJson": "[{\"id\": \"Team A\", \"data\": [{\"x\": \"Week 1\", \"y\": 1}]}]"}}
```

**stream** - Stream chart
```json
{"chartType": {"literalString": "stream"}, "data": {"literalJson": "[{\"cat1\": 10, \"cat2\": 20}]"}, "keys": {"literalJson": "[\"cat1\", \"cat2\"]"}}
```

**radialBar** - Radial bar
```json
{"chartType": {"literalString": "radialBar"}, "data": {"literalJson": "[{\"id\": \"Metric\", \"data\": [{\"x\": \"Target\", \"y\": 80}]}]"}}
```

**waffle** - Waffle chart
```json
{"chartType": {"literalString": "waffle"}, "data": {"literalJson": "[{\"id\": \"cats\", \"label\": \"Cats\", \"value\": 35}]"}}
```

**Color Schemes:** "nivo", "category10", "paired", "pastel1", "pastel2", "set1", "set2", "set3", "spectral", "blues", "greens"

### Charts (Plotly - interactive)
- `plotlyChart` - Plotly.js (chartType: "line"|"bar"|"scatter"|"pie"|"area"|"histogram", data, title, layout, config)

```json
{"type": "plotlyChart", "chartType": {"literalString": "line"}, "data": {"literalJson": "[{\"x\": [1,2,3], \"y\": [4,5,6], \"type\": \"scatter\", \"mode\": \"lines+markers\"}]"}, "height": {"literalString": "400px"}}
```

### Computer Vision / ML
- `boundingBoxOverlay` - Display detection boxes (src, boxes: [{id, x, y, width, height, label, confidence, color}], showLabels, showConfidence, normalized)
- `imageLabeler` - Draw/annotate boxes (src, labels: ["Person", "Car"], boxes, disabled)
- `imageHotspot` - Clickable hotspots (src, hotspots: [{id, x, y, icon, label, description}], markerStyle: "pulse"|"dot"|"ring")

### Game / Interactive Media
- `canvas2d` - 2D canvas (width, height, backgroundColor, pixelPerfect, children: sprites/shapes)
- `sprite` - 2D sprite (src, x, y, width, height, rotation, scale, opacity, flipX, flipY, zIndex)
- `shape` - 2D shape (shapeType: "rectangle"|"circle"|"ellipse"|"polygon"|"line"|"path", x, y, width, height, fill, stroke)
- `scene3d` - 3D scene (width, height, cameraType, cameraPosition, controlMode: "orbit"|"fly"|"fixed"|"auto-rotate", ambientLight, directionalLight, showGrid, children: model3d)
- `model3d` - 3D model (src: GLB/GLTF, position, rotation, scale, animation, viewerHeight, lightingPreset: "neutral"|"warm"|"cool"|"studio"|"dramatic", environment)
- `dialogue` - Visual novel dialogue (text, speakerName, typewriter, typewriterSpeed)
- `characterPortrait` - Character portrait (image, expression, position: "left"|"right"|"center", size, dimmed)
- `choiceMenu` - Choice menu (choices: [{id, text}], title, layout: "vertical"|"horizontal"|"grid")
- `inventoryGrid` - Inventory (items: [{id, icon, name, quantity}], columns, rows, cellSize)
- `healthBar` - Resource bar (value, maxValue, label, showValue, fillColor, backgroundColor, variant: "bar"|"segmented"|"circular")
- `miniMap` - Mini-map (mapImage, width, height, markers: [{id, x, y, icon, color, label}], playerX, playerY, playerRotation)

### Widget System
- `widgetInstance` - Reusable widget (widgetId, widgetInputs, bindOutputs)

---
## THEME COLORS (Always use these for dark/light mode support)
- Background: bg-background, bg-muted, bg-muted/50, bg-card, bg-primary, bg-secondary, bg-accent, bg-destructive
- Text: text-foreground, text-muted-foreground, text-primary, text-primary-foreground, text-destructive
- Borders: border-border, border-primary, border-destructive
- Focus: ring-ring

## CUSTOM CSS - For advanced effects
Put CSS in canvasSettings.customCss, then reference classes in component className.

EXAMPLE - Animated gradient:
```json
{
  "canvasSettings": {
    "customCss": ".gradient-bg { background: linear-gradient(135deg, #667eea, #764ba2); animation: grad 3s ease infinite; background-size: 200% 200%; } @keyframes grad { 0%{background-position:0% 50%} 50%{background-position:100% 50%} 100%{background-position:0% 50%} }"
  },
  "components": [{"id": "container", "style": {"className": "gradient-bg p-8"}, ...}]
}
```

EXAMPLE - Glass morphism:
```json
{"customCss": ".glass { backdrop-filter: blur(10px); background: rgba(255,255,255,0.1); border: 1px solid rgba(255,255,255,0.2); }"}
```

EXAMPLE - Glow effect:
```json
{"customCss": ".glow { box-shadow: 0 0 20px rgba(102,126,234,0.5); }"}
```

EXAMPLE - Hover lift:
```json
{"customCss": ".hover-lift { transition: transform 0.2s; } .hover-lift:hover { transform: translateY(-4px); box-shadow: 0 10px 40px rgba(0,0,0,0.15); }"}
```

## RULES
1. CALL emit_ui IMMEDIATELY - text responses render nothing
2. Put ALL components in ONE emit_ui call
3. Use appropriate chart type and data format for the visualization
4. Use customCss for animations, gradients, advanced effects
5. Make design choices autonomously - do not ask questions
6. For 3D models, use GLB/GLTF format - model3d can be standalone or inside scene3d
7. For game UIs, combine canvas2d with sprites/shapes, or scene3d with model3d"#;

/// Get the system prompt for a specialized agent
fn get_agent_prompt(agent_type: &SpecializedAgentType) -> &'static str {
    match agent_type {
        SpecializedAgentType::General => GENERAL_AGENT_PROMPT,
        SpecializedAgentType::Frontend => FRONTEND_AGENT_PROMPT,
        SpecializedAgentType::Backend => BACKEND_AGENT_PROMPT,
    }
}

/// Create a session with a specialized agent using Copilot SDK
#[tauri::command]
pub async fn copilot_sdk_create_agent_session(
    agent_type: SpecializedAgentType,
    model_id: Option<String>,
) -> Result<String, String> {
    let guard = COPILOT_CLIENT.lock().await;
    let client = guard.as_ref().ok_or("Copilot client not started")?;

    let system_prompt = get_agent_prompt(&agent_type);

    let config = copilot_sdk::SessionConfig {
        model: model_id,
        streaming: true,
        system_message: Some(copilot_sdk::SystemMessageConfig {
            content: Some(system_prompt.to_string()),
            mode: Some(copilot_sdk::SystemMessageMode::Append),
        }),
        infinite_sessions: Some(copilot_sdk::InfiniteSessionConfig::enabled()),
        ..Default::default()
    };

    let session = client
        .create_session(config)
        .await
        .map_err(|e| format!("Failed to create session: {}", e))?;

    Ok(session.session_id().to_string())
}
