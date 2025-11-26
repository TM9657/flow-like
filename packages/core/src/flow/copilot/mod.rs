//! Flow Copilot - AI-powered graph editing assistant
//!
//! This module provides the Copilot struct which enables natural language
//! interaction with flow graphs, supporting both explanation and modification.

mod context;
mod provider;
mod tools;
mod types;

pub use context::{EdgeContext, GraphContext, NodeContext, PinContext, prepare_context};
pub use provider::CatalogProvider;
pub use tools::{
    CatalogTool, EmitCommandsTool, FilterCategoryTool, FocusNodeTool, QueryLogsTool,
    SearchByPinTool, SearchTemplatesTool, get_tool_description,
};
pub use types::{
    AgentType, BoardCommand, ChatImage, ChatMessage, ChatRole, Connection, CopilotResponse, Edge,
    NodeMetadata, NodePosition, PinMetadata, PlaceholderPinDef, PlanStep, PlanStepStatus,
    RunContext, StreamEvent, Suggestion, TemplateInfo,
};

use std::sync::Arc;

use flow_like_types::Result;
use futures::StreamExt;
use rig::{
    OneOrMany,
    client::completion::CompletionClientDyn,
    completion::Completion,
    message::{
        AssistantContent, DocumentSourceKind, Image, ImageDetail, ImageMediaType,
        ToolResult as RigToolResult, ToolResultContent, UserContent,
    },
    streaming::StreamedAssistantContent,
    tools::ThinkTool,
};
use serde_json::json;

use crate::app::App;
use crate::bit::{Bit, BitModelPreference, BitTypes, LLMParameters, Metadata};
use crate::flow::board::Board;
use crate::profile::Profile;
use crate::state::FlowLikeState;
use flow_like_model_provider::provider::ModelProvider;
use flow_like_types::sync::Mutex;

use tools::{
    EmitCommandsArgs, FilterCategoryArgs, FocusNodeArgs, QueryLogsArgs, SearchArgs,
    SearchByPinArgs, SearchTemplatesArgs, ThinkingArgs,
};

/// The main Copilot struct that provides AI-powered graph editing
pub struct Copilot {
    state: FlowLikeState,
    catalog_provider: Arc<dyn CatalogProvider>,
    profile: Option<Arc<Profile>>,
    templates: Vec<TemplateInfo>,
    /// Current template ID if editing a template (prioritized in search)
    current_template_id: Option<String>,
}

impl Copilot {
    /// Create a new Copilot - always loads templates from profile
    pub async fn new(
        state: FlowLikeState,
        catalog_provider: Arc<dyn CatalogProvider>,
        profile: Option<Arc<Profile>>,
        current_template_id: Option<String>,
    ) -> Result<Self> {
        let templates = if let Some(ref profile) = profile {
            Self::load_templates_from_profile(&state, profile)
                .await
                .unwrap_or_default()
        } else {
            Vec::new()
        };

        Ok(Self {
            state,
            catalog_provider,
            profile,
            templates,
            current_template_id,
        })
    }

    /// Load all templates from the user's profile apps
    async fn load_templates_from_profile(
        state: &FlowLikeState,
        profile: &Profile,
    ) -> Result<Vec<TemplateInfo>> {
        let mut templates = Vec::new();

        let app_ids: Vec<String> = profile
            .apps
            .as_ref()
            .map(|apps| apps.iter().map(|a| a.app_id.clone()).collect())
            .unwrap_or_default();

        let state_arc = Arc::new(Mutex::new(state.clone()));

        for app_id in app_ids {
            // Try to load the app
            let app = match App::load(app_id.clone(), state_arc.clone()).await {
                Ok(app) => app,
                Err(_) => continue,
            };

            // Load templates from this app
            for template_id in &app.templates {
                let template_info = match Self::load_template_info(&app, template_id).await {
                    Ok(info) => info,
                    Err(_) => continue,
                };
                templates.push(template_info);
            }
        }

        Ok(templates)
    }

    /// Load template info (metadata + structure analysis)
    async fn load_template_info(app: &App, template_id: &str) -> Result<TemplateInfo> {
        // Get template metadata
        let meta = app
            .get_template_meta(template_id, None)
            .await
            .unwrap_or_else(|_| Metadata::default());

        // Load the template board to analyze its structure
        let template_board = app.open_template(template_id.to_string(), None).await?;

        // Extract unique node types used in this template
        let node_types: Vec<String> = template_board
            .nodes
            .values()
            .map(|n| n.name.clone())
            .collect::<std::collections::HashSet<_>>()
            .into_iter()
            .take(10) // Limit to avoid bloating context
            .collect();

        Ok(TemplateInfo {
            id: template_id.to_string(),
            app_id: app.id.clone(),
            name: meta.name,
            description: meta.description,
            tags: meta.tags,
            node_count: template_board.nodes.len(),
            node_types,
        })
    }

    /// Main entry point - unified agent that can both explain and modify
    pub async fn chat<F>(
        &self,
        board: &Board,
        selected_node_ids: &[String],
        user_prompt: String,
        history: Vec<ChatMessage>,
        model_id: Option<String>,
        token: Option<String>,
        run_context: Option<RunContext>,
        on_token: Option<F>,
    ) -> Result<CopilotResponse>
    where
        F: Fn(String) + Send + Sync + 'static,
    {
        println!(
            "[Copilot::chat] Starting chat with run_context: {:?}",
            run_context
        );

        let context = prepare_context(board, selected_node_ids)?;
        let context_json = flow_like_types::json::to_string_pretty(&context)?;

        // Only include node type names (not full paths) for context efficiency
        let available_nodes = self.catalog_provider.get_all_nodes().await;
        let node_count = available_nodes.len();

        let (model_name, completion_client) = self.get_model(model_id, token).await?;

        // Build a compact system prompt
        let system_prompt = Self::build_system_prompt(
            &context_json,
            node_count,
            !self.templates.is_empty(),
            run_context.is_some(),
        );

        let mut agent_builder = completion_client
            .agent(&model_name)
            .preamble(&system_prompt)
            .tool(ThinkTool)
            .tool(FocusNodeTool)
            .tool(EmitCommandsTool)
            .tool(CatalogTool {
                provider: self.catalog_provider.clone(),
            })
            .tool(SearchByPinTool {
                provider: self.catalog_provider.clone(),
            })
            .tool(FilterCategoryTool {
                provider: self.catalog_provider.clone(),
            });

        // Only add templates tool if we have templates
        if !self.templates.is_empty() {
            agent_builder = agent_builder.tool(SearchTemplatesTool {
                templates: self.templates.clone(),
                current_template_id: self.current_template_id.clone(),
            });
        }

        // Add logs query tool if run context is provided
        if run_context.is_some() {
            println!(
                "[Copilot::chat] Adding QueryLogsTool with run_context: {:?}",
                run_context
            );
            agent_builder = agent_builder.tool(QueryLogsTool {
                state: self.state.clone(),
                run_context: run_context.clone(),
            });
        } else {
            println!("[Copilot::chat] No run_context provided, QueryLogsTool NOT added");
        }

        let agent = agent_builder.build();

        let prompt = user_prompt.clone();

        // Helper to convert media type string to ImageMediaType
        let parse_media_type = |s: &str| -> Option<ImageMediaType> {
            match s.to_lowercase().as_str() {
                "image/jpeg" | "jpeg" | "jpg" => Some(ImageMediaType::JPEG),
                "image/png" | "png" => Some(ImageMediaType::PNG),
                "image/gif" | "gif" => Some(ImageMediaType::GIF),
                "image/webp" | "webp" => Some(ImageMediaType::WEBP),
                _ => None,
            }
        };

        // Convert chat history to rig message format (including images)
        let mut current_history: Vec<rig::message::Message> = history
            .iter()
            .filter_map(|msg| {
                match msg.role {
                    ChatRole::User => {
                        let mut contents: Vec<UserContent> =
                            vec![UserContent::Text(rig::message::Text {
                                text: msg.content.clone(),
                            })];

                        // Add images if present
                        if let Some(images) = &msg.images {
                            for img in images {
                                contents.push(UserContent::Image(Image {
                                    data: DocumentSourceKind::Base64(img.data.clone()),
                                    media_type: parse_media_type(&img.media_type),
                                    detail: Some(ImageDetail::Auto),
                                    additional_params: None,
                                }));
                            }
                        }

                        // Use many() which returns Result, handle the error case
                        match OneOrMany::many(contents) {
                            Ok(content) => Some(rig::message::Message::User { content }),
                            Err(_) => None, // Empty contents, skip this message
                        }
                    }
                    ChatRole::Assistant => Some(rig::message::Message::Assistant {
                        id: None,
                        content: OneOrMany::one(AssistantContent::Text(rig::message::Text {
                            text: msg.content.clone(),
                        })),
                    }),
                }
            })
            .collect();

        let mut full_response = String::new();
        let mut all_commands: Vec<BoardCommand> = Vec::new();
        let max_iterations = 10u64;
        let mut plan_step_counter = 0u32;

        for iteration in 0..max_iterations {
            // Build completion request - tools are already attached via agent builder
            let request = agent
                .completion(prompt.clone(), current_history.clone())
                .await
                .map_err(|e| flow_like_types::anyhow!("Completion error: {}", e))?;

            // Stream the response
            let mut stream = request
                .stream()
                .await
                .map_err(|e| flow_like_types::anyhow!("Stream error: {}", e))?;

            let mut response_contents: Vec<AssistantContent> = Vec::new();
            let mut iteration_text = String::new();
            let mut current_reasoning = String::new();
            let mut reasoning_step_id: Option<String> = None;

            while let Some(item) = stream.next().await {
                let content =
                    item.map_err(|e| flow_like_types::anyhow!("Stream chunk error: {}", e))?;

                match content {
                    StreamedAssistantContent::Text(text) => {
                        iteration_text.push_str(&text.text);
                        if let Some(ref callback) = on_token {
                            callback(text.text.clone());
                        }
                        response_contents.push(AssistantContent::Text(text));
                    }
                    StreamedAssistantContent::ToolCall(tool_call) => {
                        response_contents.push(AssistantContent::ToolCall(tool_call));
                    }
                    StreamedAssistantContent::ToolCallDelta { .. } => {
                        // Deltas are accumulated into the final ToolCall
                    }
                    StreamedAssistantContent::Reasoning(reasoning) => {
                        let reasoning_text = reasoning.reasoning.join("\n");
                        current_reasoning.push_str(&reasoning_text);
                        current_reasoning.push('\n');

                        // Send reasoning as a plan step (streaming update)
                        if let Some(ref callback) = on_token {
                            // Create or update the reasoning step
                            if reasoning_step_id.is_none() {
                                plan_step_counter += 1;
                                reasoning_step_id =
                                    Some(format!("reasoning_{}", plan_step_counter));
                            }

                            let step_event = StreamEvent::PlanStep(PlanStep {
                                id: reasoning_step_id.clone().unwrap(),
                                description: current_reasoning.trim().to_string(),
                                status: PlanStepStatus::InProgress,
                                tool_name: Some("think".to_string()),
                            });
                            callback(format!(
                                "<plan_step>{}</plan_step>",
                                serde_json::to_string(&step_event).unwrap_or_default()
                            ));
                        }
                    }
                    StreamedAssistantContent::Final(_) => {
                        // Mark reasoning step as completed
                        if let (Some(callback), Some(step_id)) = (&on_token, &reasoning_step_id) {
                            let step_event = StreamEvent::PlanStep(PlanStep {
                                id: step_id.clone(),
                                description: current_reasoning.trim().to_string(),
                                status: PlanStepStatus::Completed,
                                tool_name: Some("think".to_string()),
                            });
                            callback(format!(
                                "<plan_step>{}</plan_step>",
                                serde_json::to_string(&step_event).unwrap_or_default()
                            ));
                        }
                        reasoning_step_id = None;
                        current_reasoning.clear();
                    }
                }
            }

            // Mark reasoning step as completed if stream ended while reasoning
            if let (Some(callback), Some(step_id)) = (&on_token, &reasoning_step_id) {
                let step_event = StreamEvent::PlanStep(PlanStep {
                    id: step_id.clone(),
                    description: current_reasoning.trim().to_string(),
                    status: PlanStepStatus::Completed,
                    tool_name: Some("think".to_string()),
                });
                callback(format!(
                    "<plan_step>{}</plan_step>",
                    serde_json::to_string(&step_event).unwrap_or_default()
                ));
            }

            full_response.push_str(&iteration_text);

            // Collect all tool calls first for parallel execution
            let tool_calls: Vec<_> = response_contents
                .iter()
                .filter_map(|content| {
                    if let AssistantContent::ToolCall(tool_call) = content {
                        Some(tool_call.clone())
                    } else {
                        None
                    }
                })
                .collect();

            let tool_calls_found = !tool_calls.is_empty();

            if tool_calls_found {
                // Emit plan steps for all tool calls starting
                let mut step_ids: Vec<(String, String, u32)> = Vec::new();
                for tool_call in &tool_calls {
                    plan_step_counter += 1;
                    let step_id = format!("step_{}", plan_step_counter);
                    let step_description = get_tool_description(
                        &tool_call.function.name,
                        &tool_call.function.arguments,
                    );

                    if let Some(ref callback) = on_token {
                        callback(format!("tool_call:{}", tool_call.function.name));
                        let step_event = StreamEvent::PlanStep(PlanStep {
                            id: step_id.clone(),
                            description: step_description.clone(),
                            status: PlanStepStatus::InProgress,
                            tool_name: Some(tool_call.function.name.clone()),
                        });
                        callback(format!(
                            "<plan_step>{}</plan_step>",
                            serde_json::to_string(&step_event).unwrap_or_default()
                        ));
                    }

                    step_ids.push((step_id, step_description, plan_step_counter));
                }

                // Execute all tools in parallel
                let tool_futures: Vec<_> = tool_calls
                    .iter()
                    .map(|tool_call| {
                        let name = tool_call.function.name.clone();
                        let arguments = tool_call.function.arguments.clone();
                        let id = tool_call.id.clone();
                        let ctx = run_context.clone();
                        async move {
                            let output = self.execute_tool(&name, &arguments, ctx.as_ref()).await;
                            (id, name, output)
                        }
                    })
                    .collect();

                let tool_results: Vec<(String, String, String)> =
                    futures::future::join_all(tool_futures).await;

                // Process results and emit completion events
                for (i, (id, name, tool_output)) in tool_results.iter().enumerate() {
                    println!(
                        "[Copilot] Tool '{}' (id={}) output length: {} chars",
                        name,
                        id,
                        tool_output.len()
                    );

                    // Parse commands from emit_commands tool output
                    if name == "emit_commands" {
                        let parsed = Self::parse_commands(tool_output);
                        println!("[Copilot] emit_commands parsed {} commands:", parsed.len());
                        for (idx, cmd) in parsed.iter().enumerate() {
                            println!("[Copilot]   [{}] {:?}", idx, cmd);
                        }

                        // Deduplicate: only add commands that don't already exist
                        for cmd in parsed {
                            let is_duplicate = all_commands
                                .iter()
                                .any(|existing| Self::commands_are_duplicate(existing, &cmd));
                            if !is_duplicate {
                                all_commands.push(cmd);
                            } else {
                                println!("[Copilot] Skipping duplicate command");
                            }
                        }

                        println!(
                            "[Copilot] all_commands now has {} total commands (after dedup)",
                            all_commands.len()
                        );
                    }

                    // Emit plan step completion
                    if let Some(ref callback) = on_token {
                        if let Some((step_id, step_description, _)) = step_ids.get(i) {
                            let step_event = StreamEvent::PlanStep(PlanStep {
                                id: step_id.clone(),
                                description: step_description.clone(),
                                status: PlanStepStatus::Completed,
                                tool_name: Some(name.clone()),
                            });
                            callback(format!(
                                "<plan_step>{}</plan_step>",
                                serde_json::to_string(&step_event).unwrap_or_default()
                            ));
                        }
                        callback("tool_result:done".to_string());
                    }

                    // Only stream focus_node results to the user
                    if let Some(ref callback) = on_token {
                        if name == "focus_node" {
                            callback(tool_output.clone());
                        }
                    }
                }

                // Add assistant message with tool calls to history
                let assistant_msg = rig::message::Message::Assistant {
                    id: None,
                    content: OneOrMany::many(response_contents.clone()).unwrap_or_else(|_| {
                        OneOrMany::one(AssistantContent::Text(rig::message::Text {
                            text: String::new(),
                        }))
                    }),
                };
                current_history.push(assistant_msg);

                // Add tool results to history
                for (tool_id, _tool_name, tool_output) in &tool_results {
                    let tool_result_msg = rig::message::Message::User {
                        content: OneOrMany::one(UserContent::ToolResult(RigToolResult {
                            id: tool_id.clone(),
                            call_id: None,
                            content: OneOrMany::one(ToolResultContent::text(tool_output.clone())),
                        })),
                    };
                    current_history.push(tool_result_msg);
                }
            } else {
                // No tool calls, we're done
                break;
            }

            // Continue to next iteration (agent will see tool results and continue)
            if iteration == max_iterations - 1 {
                break;
            }
        }

        let has_commands = !all_commands.is_empty();
        println!(
            "[Copilot] Final response: {} total commands, agent_type={:?}",
            all_commands.len(),
            if has_commands {
                AgentType::Edit
            } else {
                AgentType::Explain
            }
        );

        // Log the serialized response for debugging
        let response = CopilotResponse {
            agent_type: if has_commands {
                AgentType::Edit
            } else {
                AgentType::Explain
            },
            message: Self::clean_message(&full_response),
            commands: all_commands,
            suggestions: vec![],
        };

        if let Ok(json) = serde_json::to_string(&response) {
            println!("[Copilot] Response JSON length: {} chars", json.len());
            if !response.commands.is_empty() {
                println!(
                    "[Copilot] First command serialized: {:?}",
                    serde_json::to_string(&response.commands[0])
                );
            }
        }

        Ok(response)
    }

    /// Build a compact system prompt to reduce context size
    fn build_system_prompt(
        context_json: &str,
        node_count: usize,
        has_templates: bool,
        has_run_context: bool,
    ) -> String {
        let templates_tool = if has_templates {
            "\n- **search_templates**: Search workflow templates for implementation examples"
        } else {
            ""
        };

        let logs_tool = if has_run_context {
            "\n- **query_logs**: Query execution logs from the current run. Use focus_node to highlight problematic nodes."
        } else {
            ""
        };

        format!(
            r#"You are an expert graph editor assistant. You help users understand and modify visual workflows.

## Graph Context (abbreviated keys: t=type, n=name, i=inputs, o=outputs, p=position, s=size, f=from, fp=from_pin, tp=to_pin, v=value)
{}

## Tools
**Understanding**: think (reason step-by-step), focus_node (highlight node by ID)
**Catalog** ({} nodes): catalog_search (by name/description), search_by_pin (by pin type), filter_category (by category){}{}
**Modify**: emit_commands (execute graph changes)

## Key Rules
1. Reference nodes: <focus_node>ID</focus_node> - put ONLY the node ID between tags, nothing else
2. Node IDs are cuid2 format (lowercase alphanumeric, 24+ chars, e.g. "tz4a98xxat96ipl6cg5ebkj1")
3. WRONG: <focus_node node_id="..."> or <focus_node>Node Name</focus_node> - these will NOT work
4. Use pin `n` (name) in commands for pin connections
4. Connect compatible types only (check t=type from catalog)
5. New nodes need ref_id ("$0", "$1"...) for subsequent connections
6. Connect execution flow: exec_out → exec_in
7. Position nodes left-to-right, 250px horizontal spacing
8. Each command needs a `summary` field
9. Limit output to 20 commands per turn

## Commands
AddNode(node_type, ref_id, position, summary) | RemoveNode(node_id, summary)
AddPlaceholder(name, ref_id, position, pins[], summary) - Create a placeholder node for process modeling with custom name and pins
ConnectPins(from_node, from_pin, to_node, to_pin, summary) | DisconnectPins(same)
UpdateNodePin(node_id, pin_id, value, summary) | MoveNode(node_id, position, summary)
CreateVariable(name, data_type, value_type, summary) | CreateComment(content, position, summary)

## Process Modeling
Use these tools when the user wants to model/sketch a process before implementing with real nodes:

**Placeholders** (AddPlaceholder): Create custom process steps with named pins
- Always have exec_in and exec_out pins automatically
- Add custom data pins: pins[]: Array of {{name, friendly_name, pin_type (Input/Output), data_type (String/Integer/Float/Boolean/Struct/Generic)}}

**Branches** (node_type: "control_branch"): Decision points with condition input and True/False execution outputs
- Use for if/else logic, approvals, validations

**Parallel Execution** (node_type: "control_par_execution"): Run multiple paths simultaneously
- Use for tasks that can happen concurrently (e.g., send notifications while processing)

**Comments** (CreateComment): Add documentation/notes to explain process sections

IMPORTANT: Every process flow needs a START EVENT:
1. First add a "Simple Event" node (node_type: "events_simple") - this is the entry point
2. Then add placeholders, branches, sequences for process steps
3. Connect them: Simple Event → Step 1 → Branch → (True path / False path) etc.

Example process: Simple Event → Validate Order (placeholder) → Branch (is_valid) → True: Process Payment → Ship Order | False: Notify Customer

## Command Order
ALWAYS emit commands in this order:
1. AddNode commands first (create nodes)
2. ConnectPins commands (wire nodes together)
3. UpdateNodePin commands LAST (set default values)

## CRITICAL: Do NOT repeat commands
- After emit_commands succeeds, those commands are QUEUED - do NOT emit them again
- Check tool results to see what was already created before adding more
- Each node/placeholder should only be created ONCE

## Workflow: Start from TARGET, work backwards. Search catalog first. Connect exec pins."#,
            context_json, node_count, templates_tool, logs_tool
        )
    }

    /// Execute a tool by name and return the result
    async fn execute_tool(
        &self,
        name: &str,
        arguments: &serde_json::Value,
        run_context: Option<&RunContext>,
    ) -> String {
        match name {
            "think" => {
                if let Ok(args) = serde_json::from_value::<ThinkingArgs>(arguments.clone()) {
                    format!("Thinking: {}", args.thought)
                } else {
                    "Thinking...".to_string()
                }
            }
            "focus_node" => {
                if let Ok(args) = serde_json::from_value::<FocusNodeArgs>(arguments.clone()) {
                    format!("<focus_node>{}</focus_node>", args.node_id)
                } else {
                    "".to_string()
                }
            }
            "emit_commands" => {
                match serde_json::from_value::<EmitCommandsArgs>(arguments.clone()) {
                    Ok(args) => {
                        let commands_json =
                            serde_json::to_string(&args.commands).unwrap_or_default();
                        println!(
                            "[Copilot] emit_commands: {} commands, json length: {} chars",
                            args.commands.len(),
                            commands_json.len()
                        );
                        format!(
                            "<commands>{}</commands>\n\n{}",
                            commands_json, args.explanation
                        )
                    }
                    Err(e) => {
                        println!("[Copilot] emit_commands: Failed to parse args: {:?}", e);
                        println!("[Copilot] emit_commands: Raw arguments: {:?}", arguments);
                        format!("Failed to parse commands: {}", e)
                    }
                }
            }
            "catalog_search" => {
                if let Ok(args) = serde_json::from_value::<SearchArgs>(arguments.clone()) {
                    let matches = self.catalog_provider.search(&args.query).await;
                    serde_json::to_string(&matches).unwrap_or_default()
                } else {
                    "[]".to_string()
                }
            }
            "search_by_pin" => {
                if let Ok(args) = serde_json::from_value::<SearchByPinArgs>(arguments.clone()) {
                    let matches = self
                        .catalog_provider
                        .search_by_pin_type(&args.pin_type, args.is_input)
                        .await;
                    serde_json::to_string(&matches).unwrap_or_default()
                } else {
                    "[]".to_string()
                }
            }
            "filter_category" => {
                if let Ok(args) = serde_json::from_value::<FilterCategoryArgs>(arguments.clone()) {
                    let matches = self
                        .catalog_provider
                        .filter_by_category(&args.category_prefix)
                        .await;
                    serde_json::to_string(&matches).unwrap_or_default()
                } else {
                    "[]".to_string()
                }
            }
            "search_templates" => {
                if let Ok(args) = serde_json::from_value::<SearchTemplatesArgs>(arguments.clone()) {
                    let query_lower = args.query.to_lowercase();
                    let mut matches: Vec<&TemplateInfo> = self
                        .templates
                        .iter()
                        .filter(|t| {
                            // Skip current template being edited
                            if let Some(ref current_id) = self.current_template_id {
                                if &t.id == current_id {
                                    return false;
                                }
                            }
                            t.name.to_lowercase().contains(&query_lower)
                                || t.description.to_lowercase().contains(&query_lower)
                                || t.tags
                                    .iter()
                                    .any(|tag| tag.to_lowercase().contains(&query_lower))
                                || t.node_types
                                    .iter()
                                    .any(|nt| nt.to_lowercase().contains(&query_lower))
                        })
                        .take(5)
                        .collect();
                    // Sort by relevance
                    matches.sort_by(|a, b| {
                        let a_name_match = a.name.to_lowercase().contains(&query_lower);
                        let b_name_match = b.name.to_lowercase().contains(&query_lower);
                        b_name_match.cmp(&a_name_match)
                    });
                    serde_json::to_string(&matches).unwrap_or_default()
                } else {
                    "[]".to_string()
                }
            }
            "query_logs" => {
                #[cfg(feature = "flow-runtime")]
                {
                    if let Some(ctx) = run_context {
                        let args = serde_json::from_value::<QueryLogsArgs>(arguments.clone())
                            .unwrap_or(QueryLogsArgs {
                                filter: None,
                                limit: None,
                            });

                        let limit = args.limit.unwrap_or(50).min(100);
                        let filter = args.filter.unwrap_or_default();

                        let log_meta = crate::flow::execution::LogMeta {
                            app_id: ctx.app_id.clone(),
                            run_id: ctx.run_id.clone(),
                            board_id: ctx.board_id.clone(),
                            start: 0,
                            end: 0,
                            log_level: 0,
                            version: String::new(),
                            nodes: None,
                            logs: None,
                            node_id: String::new(),
                            event_version: None,
                            event_id: String::new(),
                            payload: vec![],
                        };

                        match self
                            .state
                            .query_run(&log_meta, &filter, Some(limit), Some(0))
                            .await
                        {
                            Ok(logs) => {
                                if logs.is_empty() {
                                    if filter.is_empty() {
                                        "No logs found for this run.".to_string()
                                    } else {
                                        "No logs matching your filter criteria.".to_string()
                                    }
                                } else {
                                    let formatted: Vec<serde_json::Value> = logs.iter().map(|log| {
                                        json!({
                                            "level": match log.log_level {
                                                crate::flow::execution::LogLevel::Debug => "Debug",
                                                crate::flow::execution::LogLevel::Info => "Info",
                                                crate::flow::execution::LogLevel::Warn => "Warn",
                                                crate::flow::execution::LogLevel::Error => "Error",
                                                crate::flow::execution::LogLevel::Fatal => "Fatal",
                                            },
                                            "message": log.message,
                                            "node_id": log.node_id,
                                        })
                                    }).collect();
                                    serde_json::to_string_pretty(&formatted).unwrap_or_default()
                                }
                            }
                            Err(e) => format!("Failed to query logs: {}", e),
                        }
                    } else {
                        "No run context available. Please select a run first.".to_string()
                    }
                }
                #[cfg(not(feature = "flow-runtime"))]
                {
                    let _ = run_context; // Suppress unused variable warning
                    "Log querying is not available in this build.".to_string()
                }
            }
            _ => {
                println!("[Copilot] Unknown tool requested: {}", name);
                format!("Unknown tool: {}", name)
            }
        }
    }

    /// Parse commands from the agent's response
    fn parse_commands(response: &str) -> Vec<BoardCommand> {
        // Look for <commands>...</commands> tags
        if let Some(start) = response.find("<commands>") {
            if let Some(end) = response.find("</commands>") {
                let json_str = &response[start + 10..end];
                if let Ok(commands) = serde_json::from_str::<Vec<BoardCommand>>(json_str) {
                    return commands;
                }
            }
        }
        vec![]
    }

    /// Check if two commands are duplicates (same type and key identifiers)
    fn commands_are_duplicate(a: &BoardCommand, b: &BoardCommand) -> bool {
        match (a, b) {
            (
                BoardCommand::AddNode {
                    node_type: t1,
                    ref_id: r1,
                    ..
                },
                BoardCommand::AddNode {
                    node_type: t2,
                    ref_id: r2,
                    ..
                },
            ) => t1 == t2 && r1 == r2,
            (
                BoardCommand::AddPlaceholder {
                    name: n1,
                    ref_id: r1,
                    ..
                },
                BoardCommand::AddPlaceholder {
                    name: n2,
                    ref_id: r2,
                    ..
                },
            ) => n1 == n2 || r1 == r2,
            (
                BoardCommand::RemoveNode { node_id: id1, .. },
                BoardCommand::RemoveNode { node_id: id2, .. },
            ) => id1 == id2,
            (
                BoardCommand::ConnectPins {
                    from_node: f1,
                    from_pin: fp1,
                    to_node: t1,
                    to_pin: tp1,
                    ..
                },
                BoardCommand::ConnectPins {
                    from_node: f2,
                    from_pin: fp2,
                    to_node: t2,
                    to_pin: tp2,
                    ..
                },
            ) => f1 == f2 && fp1 == fp2 && t1 == t2 && tp1 == tp2,
            _ => false,
        }
    }

    /// Clean the message by removing command tags
    fn clean_message(response: &str) -> String {
        // Remove <commands>...</commands> block
        let mut result = response.to_string();
        if let Some(start) = result.find("<commands>") {
            if let Some(end) = result.find("</commands>") {
                result = format!("{}{}", &result[..start], &result[end + 11..]);
            }
        }
        result.trim().to_string()
    }

    /// Get the model for the agent
    async fn get_model<'a>(
        &self,
        model_id: Option<String>,
        token: Option<String>,
    ) -> Result<(String, Box<dyn CompletionClientDyn + 'a>)> {
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

        let model_factory = self.state.model_factory.clone();
        let model = model_factory
            .lock()
            .await
            .build(&bit, Arc::new(Mutex::new(self.state.clone())), token)
            .await?;
        let default_model = model.default_model().await.unwrap_or("gpt-4o".to_string());
        let provider = model.provider().await?;
        let client = provider.client();
        let completion = client
            .as_completion()
            .ok_or_else(|| flow_like_types::anyhow!("Model does not support completion"))?;

        Ok((default_model, completion))
    }
}
