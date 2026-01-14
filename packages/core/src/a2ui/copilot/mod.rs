//! A2UI Copilot - AI-powered UI generation assistant
//!
//! This module provides the A2UICopilot struct which enables natural language
//! UI generation using the A2UI component system.

mod tools;
mod types;

pub use tools::*;
pub use types::*;

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

use crate::a2ui::SurfaceComponent;
use crate::bit::{Bit, BitModelPreference, BitTypes, LLMParameters};
use crate::profile::Profile;
use crate::state::FlowLikeState;
use flow_like_model_provider::provider::ModelProvider;

// Note: Tool types are re-exported publicly from `pub use tools::*;` above

/// The main A2UI Copilot struct that provides AI-powered UI generation
pub struct A2UICopilot {
    state: Arc<FlowLikeState>,
    profile: Option<Arc<Profile>>,
}

impl A2UICopilot {
    /// Create a new A2UICopilot
    pub async fn new(
        state: Arc<FlowLikeState>,
        profile: Option<Arc<Profile>>,
    ) -> Result<Self> {
        Ok(Self { state, profile })
    }

    /// Main entry point - generate or modify A2UI surfaces
    pub async fn chat<F>(
        &self,
        current_surface: Option<&Vec<SurfaceComponent>>,
        selected_component_ids: &[String],
        user_prompt: String,
        history: Vec<A2UIChatMessage>,
        model_id: Option<String>,
        token: Option<String>,
        on_token: Option<F>,
    ) -> Result<A2UICopilotResponse>
    where
        F: Fn(String) + Send + Sync + 'static,
    {
        let context = self.prepare_context(current_surface, selected_component_ids)?;
        let context_json = flow_like_types::json::to_string_pretty(&context)?;

        let (model_name, completion_client) = self.get_model(model_id, token).await?;

        let system_prompt = Self::build_system_prompt(&context_json);

        let agent = completion_client
            .agent(&model_name)
            .preamble(&system_prompt)
            .tool(ThinkTool)
            .tool(GetComponentSchemaTool)
            .tool(GetStyleExamplesTool)
            .tool(ModifyComponentTool {
                current_components: current_surface.cloned(),
            })
            .tool(EmitSurfaceTool)
            .build();

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

        // Convert chat history to rig message format
        let mut current_history: Vec<rig::message::Message> = history
            .iter()
            .filter_map(|msg| {
                match msg.role {
                    A2UIChatRole::User => {
                        let mut contents: Vec<UserContent> =
                            vec![UserContent::Text(rig::message::Text {
                                text: msg.content.clone(),
                            })];

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

                        match OneOrMany::many(contents) {
                            Ok(content) => Some(rig::message::Message::User { content }),
                            Err(_) => None,
                        }
                    }
                    A2UIChatRole::Assistant => Some(rig::message::Message::Assistant {
                        id: None,
                        content: OneOrMany::one(AssistantContent::Text(rig::message::Text {
                            text: msg.content.clone(),
                        })),
                    }),
                }
            })
            .collect();

        let mut full_response = String::new();
        let mut generated_components: Vec<SurfaceComponent> = Vec::new();
        let max_iterations = 5u64;
        let mut plan_step_counter = 0u32;

        for iteration in 0..max_iterations {
            // Send iteration start event
            if let Some(ref callback) = on_token {
                plan_step_counter += 1;
                let step_event = A2UIStreamEvent::PlanStep(A2UIPlanStep {
                    id: format!("iteration_{}", iteration),
                    description: if iteration == 0 {
                        "Analyzing request...".to_string()
                    } else {
                        "Processing tool results...".to_string()
                    },
                    status: A2UIPlanStepStatus::InProgress,
                    tool_name: Some("analyze".to_string()),
                });
                callback(format!(
                    "<plan_step>{}</plan_step>",
                    serde_json::to_string(&step_event).unwrap_or_default()
                ));
            }

            let request = agent
                .completion(prompt.clone(), current_history.clone())
                .await
                .map_err(|e| flow_like_types::anyhow!("Completion error: {}", e))?;

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
                    StreamedAssistantContent::ToolCallDelta { .. } => {}
                    StreamedAssistantContent::Reasoning(reasoning) => {
                        let reasoning_text = reasoning.reasoning.join("\n");
                        current_reasoning.push_str(&reasoning_text);
                        current_reasoning.push('\n');

                        if let Some(ref callback) = on_token {
                            if reasoning_step_id.is_none() {
                                plan_step_counter += 1;
                                reasoning_step_id =
                                    Some(format!("reasoning_{}", plan_step_counter));
                            }

                            let step_event = A2UIStreamEvent::PlanStep(A2UIPlanStep {
                                id: reasoning_step_id.clone().unwrap(),
                                description: current_reasoning.trim().to_string(),
                                status: A2UIPlanStepStatus::InProgress,
                                tool_name: Some("think".to_string()),
                            });
                            callback(format!(
                                "<plan_step>{}</plan_step>",
                                serde_json::to_string(&step_event).unwrap_or_default()
                            ));
                        }
                    }
                    StreamedAssistantContent::Final(_) => {
                        if let (Some(callback), Some(step_id)) = (&on_token, &reasoning_step_id) {
                            let step_event = A2UIStreamEvent::PlanStep(A2UIPlanStep {
                                id: step_id.clone(),
                                description: current_reasoning.trim().to_string(),
                                status: A2UIPlanStepStatus::Completed,
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

            // Complete any remaining reasoning step
            if let (Some(callback), Some(step_id)) = (&on_token, &reasoning_step_id) {
                let step_event = A2UIStreamEvent::PlanStep(A2UIPlanStep {
                    id: step_id.clone(),
                    description: current_reasoning.trim().to_string(),
                    status: A2UIPlanStepStatus::Completed,
                    tool_name: Some("think".to_string()),
                });
                callback(format!(
                    "<plan_step>{}</plan_step>",
                    serde_json::to_string(&step_event).unwrap_or_default()
                ));
            }

            // Mark iteration analysis as complete
            if let Some(ref callback) = on_token {
                let step_event = A2UIStreamEvent::PlanStep(A2UIPlanStep {
                    id: format!("iteration_{}", iteration),
                    description: if iteration == 0 {
                        "Analysis complete".to_string()
                    } else {
                        "Tool results processed".to_string()
                    },
                    status: A2UIPlanStepStatus::Completed,
                    tool_name: Some("analyze".to_string()),
                });
                callback(format!(
                    "<plan_step>{}</plan_step>",
                    serde_json::to_string(&step_event).unwrap_or_default()
                ));
            }

            full_response.push_str(&iteration_text);

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

            if !tool_calls_found {
                break;
            }

            // Add assistant message to history
            let assistant_content = if response_contents.is_empty() {
                OneOrMany::one(AssistantContent::Text(rig::message::Text {
                    text: iteration_text.clone(),
                }))
            } else {
                OneOrMany::many(response_contents.clone())
                    .unwrap_or_else(|_| OneOrMany::one(AssistantContent::Text(rig::message::Text {
                        text: iteration_text.clone(),
                    })))
            };

            current_history.push(rig::message::Message::Assistant {
                id: None,
                content: assistant_content,
            });

            // Execute tool calls
            let mut tool_results: Vec<(String, String, RigToolResult)> = Vec::new();

            for tool_call in tool_calls {
                plan_step_counter += 1;
                let step_id = format!("tool_{}", plan_step_counter);

                // Describe what each tool does
                let tool_description = match tool_call.function.name.as_str() {
                    "get_component_schema" => {
                        let component_type = tool_call.function.arguments
                            .get("component_type")
                            .and_then(|v| v.as_str())
                            .unwrap_or("component");
                        format!("Looking up {} schema...", component_type)
                    },
                    "get_style_examples" => {
                        let category = tool_call.function.arguments
                            .get("category")
                            .and_then(|v| v.as_str())
                            .unwrap_or("styles");
                        format!("Fetching {} style examples...", category)
                    },
                    "modify_component" => {
                        let component_id = tool_call.function.arguments
                            .get("component_id")
                            .and_then(|v| v.as_str())
                            .unwrap_or("component");
                        format!("Modifying {}...", component_id)
                    },
                    "emit_surface" => "Creating components...".to_string(),
                    "think" => "Reasoning...".to_string(),
                    other => format!("Executing {}...", other),
                };

                if let Some(ref callback) = on_token {
                    // Send tool_call notification for frontend
                    callback(format!("tool_call:{}", tool_call.function.name));

                    let step_event = A2UIStreamEvent::PlanStep(A2UIPlanStep {
                        id: step_id.clone(),
                        description: tool_description,
                        status: A2UIPlanStepStatus::InProgress,
                        tool_name: Some(tool_call.function.name.clone()),
                    });
                    callback(format!(
                        "<plan_step>{}</plan_step>",
                        serde_json::to_string(&step_event).unwrap_or_default()
                    ));
                }

                let result = self.execute_tool(&tool_call.function.name, &tool_call.function.arguments, current_surface).await;

                // Describe completion based on tool type
                let completion_description = match tool_call.function.name.as_str() {
                    "get_component_schema" => "Schema loaded".to_string(),
                    "get_style_examples" => "Style examples ready".to_string(),
                    "modify_component" => "Component modified".to_string(),
                    "emit_surface" => {
                        if let Ok(args) = serde_json::from_value::<EmitSurfaceArgs>(tool_call.function.arguments.clone()) {
                            generated_components = args.components.clone();
                            format!("Created {} components", args.components.len())
                        } else {
                            "Components created".to_string()
                        }
                    },
                    _ => "Done".to_string(),
                };

                // Mark step as completed
                if let Some(ref callback) = on_token {
                    let step_event = A2UIStreamEvent::PlanStep(A2UIPlanStep {
                        id: step_id.clone(),
                        description: completion_description,
                        status: A2UIPlanStepStatus::Completed,
                        tool_name: Some(tool_call.function.name.clone()),
                    });
                    callback(format!(
                        "<plan_step>{}</plan_step>",
                        serde_json::to_string(&step_event).unwrap_or_default()
                    ));
                    // Send tool_result notification for frontend
                    callback("tool_result:done".to_string());
                }

                let result_content = match &result {
                    Ok(value) => ToolResultContent::Text(rig::message::Text {
                        text: value.clone(),
                    }),
                    Err(e) => ToolResultContent::Text(rig::message::Text {
                        text: format!("Error: {}", e),
                    }),
                };

                let tool_result = RigToolResult {
                    id: tool_call.id.clone(),
                    call_id: None,
                    content: OneOrMany::one(result_content),
                };
                tool_results.push((tool_call.id.clone(), tool_call.function.name.clone(), tool_result));
            }

            // Add tool results to history (each as a separate User message)
            for (_tool_id, _tool_name, tool_result) in tool_results {
                current_history.push(rig::message::Message::User {
                    content: OneOrMany::one(UserContent::ToolResult(tool_result)),
                });
            }
        }

        Ok(A2UICopilotResponse {
            message: full_response,
            components: generated_components,
            suggestions: vec![],
        })
    }

    fn prepare_context(
        &self,
        current_surface: Option<&Vec<SurfaceComponent>>,
        selected_component_ids: &[String],
    ) -> Result<A2UIContext> {
        let components = current_surface
            .map(|s| {
                s.iter()
                    .map(|c| ComponentContext {
                        id: c.id.clone(),
                        component_type: c.get_component_type_name(),
                        style_classes: c.style.as_ref().and_then(|s| s.class_name.clone()),
                        is_selected: selected_component_ids.contains(&c.id),
                    })
                    .collect()
            })
            .unwrap_or_default();

        Ok(A2UIContext {
            components,
            selected_ids: selected_component_ids.to_vec(),
            component_count: current_surface.map(|s| s.len()).unwrap_or(0),
        })
    }

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
            .build(&bit, self.state.clone(), token)
            .await?;
        let default_model = model.default_model().await.unwrap_or("gpt-4o".to_string());
        let provider = model.provider().await?;
        let client = provider.client();
        let completion = client
            .as_completion()
            .ok_or_else(|| flow_like_types::anyhow!("Model does not support completion"))?;

        Ok((default_model, completion))
    }

    async fn execute_tool(
        &self,
        tool_name: &str,
        arguments: &serde_json::Value,
        current_surface: Option<&Vec<SurfaceComponent>>,
    ) -> Result<String> {
        match tool_name {
            "get_component_schema" => {
                let args: GetComponentSchemaArgs = serde_json::from_value(arguments.clone())?;
                Ok(get_component_schema(&args.component_type))
            }
            "get_style_examples" => {
                let args: GetStyleExamplesArgs = serde_json::from_value(arguments.clone())?;
                Ok(get_style_examples(&args.category))
            }
            "modify_component" => {
                let args: ModifyComponentArgs = serde_json::from_value(arguments.clone())?;
                Ok(modify_component(&args, current_surface))
            }
            "emit_surface" => {
                let args: EmitSurfaceArgs = serde_json::from_value(arguments.clone())?;
                // Validate the components
                if args.components.is_empty() {
                    return Err(flow_like_types::anyhow!("emit_surface requires at least one component"));
                }
                Ok(format!("Surface emitted with {} components", args.components.len()))
            }
            _ => Err(flow_like_types::anyhow!("Unknown tool: {}", tool_name)),
        }
    }

    fn build_system_prompt(context_json: &str) -> String {
        format!(
            r#"You are FlowPilot, an AI assistant for generating A2UI interfaces. Generate UI components directly without asking questions.

## Current Context
```json
{context}
```

## Component Format
```json
{{"id": "unique-id", "style": {{"className": "tailwind"}}, "component": {{"type": "componentType", ...props}}}}
```

## BoundValue Format (for all component props)
- String: {{"literalString": "text"}}
- Number: {{"literalNumber": 42}}
- Boolean: {{"literalBool": true}}
- Options array: {{"literalOptions": [{{"value": "v1", "label": "Label 1"}}]}}
- Data binding: {{"path": "$.data.field", "defaultValue": "fallback"}}

## Children Format
```json
"children": {{"explicitList": ["child-id-1", "child-id-2"]}}
```

---
## LAYOUT COMPONENTS

### column
Vertical flex container.
| Property | Type | Description |
|----------|------|-------------|
| gap | BoundValue | "4px", "1rem", etc. |
| align | BoundValue | "start", "center", "end", "stretch", "baseline" |
| justify | BoundValue | "start", "center", "end", "between", "around", "evenly" |
| wrap | BoundValue | boolean |
| reverse | BoundValue | boolean |
| children | Children | Child component IDs |

### row
Horizontal flex container.
| Property | Type | Description |
|----------|------|-------------|
| gap | BoundValue | "4px", "1rem", etc. |
| align | BoundValue | "start", "center", "end", "stretch", "baseline" |
| justify | BoundValue | "start", "center", "end", "between", "around", "evenly" |
| wrap | BoundValue | boolean |
| reverse | BoundValue | boolean |
| children | Children | Child component IDs |

### grid
CSS Grid container.
| Property | Type | Description |
|----------|------|-------------|
| columns | BoundValue | "repeat(3, 1fr)", "1fr 2fr" |
| rows | BoundValue | "repeat(2, 1fr)", "auto" |
| gap | BoundValue | "16px", "1rem" |
| columnGap | BoundValue | Column-specific gap |
| rowGap | BoundValue | Row-specific gap |
| autoFlow | BoundValue | "row", "column", "dense", "rowDense", "columnDense" |
| children | Children | Child component IDs |

### stack
Z-axis layering. REQUIRES min-height in style!
| Property | Type | Description |
|----------|------|-------------|
| align | BoundValue | "start", "center", "end", "stretch" |
| children | Children | Stacked component IDs |

### scrollArea
Scrollable container.
| Property | Type | Description |
|----------|------|-------------|
| direction | BoundValue | "vertical", "horizontal", "both" |
| children | Children | Child component IDs |

### absolute
Free positioning container.
| Property | Type | Description |
|----------|------|-------------|
| width | BoundValue | "100px", "50%" |
| height | BoundValue | "100px", "50%" |
| children | Children | Child component IDs |

### aspectRatio
Maintain aspect ratio container.
| Property | Type | Description |
|----------|------|-------------|
| ratio | BoundValue | "16/9", "4/3", "1/1" (REQUIRED) |
| children | Children | Child component IDs |

### overlay
Position items over a base component.
| Property | Type | Description |
|----------|------|-------------|
| baseComponentId | string | Component ID for the base (REQUIRED) |
| overlays | array | {{componentId, anchor?, offsetX?, offsetY?, zIndex?}} |

Anchor values: "topLeft", "topCenter", "topRight", "centerLeft", "center", "centerRight", "bottomLeft", "bottomCenter", "bottomRight"

### box
Generic container with semantic HTML element.
| Property | Type | Description |
|----------|------|-------------|
| as | BoundValue | "div", "section", "header", "footer", "main", "aside", "nav", "article", "figure", "span" |
| children | Children | Child component IDs |

### center
Center content horizontally/vertically.
| Property | Type | Description |
|----------|------|-------------|
| inline | BoundValue | boolean - center inline elements |
| children | Children | Child component IDs |

### spacer
Flexible or fixed space.
| Property | Type | Description |
|----------|------|-------------|
| size | BoundValue | Fixed size: "20px", "2rem" |
| flex | BoundValue | Flex grow value (default: 1 if no size) |

---
## DISPLAY COMPONENTS

### text
Text display with typography control.
| Property | Type | Description |
|----------|------|-------------|
| content | BoundValue | Text content (REQUIRED) |
| variant | BoundValue | "body", "h1", "h2", "h3", "h4", "h5", "h6", "caption", "code", "label" |
| size | BoundValue | "xs", "sm", "md", "lg", "xl", "2xl", "3xl", "4xl" |
| weight | BoundValue | "light", "normal", "medium", "semibold", "bold" |
| color | BoundValue | Tailwind class like "text-primary" |
| align | BoundValue | "left", "center", "right", "justify" |
| truncate | BoundValue | boolean |
| maxLines | BoundValue | number |

### image
Image display.
| Property | Type | Description |
|----------|------|-------------|
| src | BoundValue | URL string (REQUIRED) |
| alt | BoundValue | Alt text |
| fit | BoundValue | "contain", "cover", "fill", "none", "scaleDown" |
| fallback | BoundValue | Fallback image URL |
| loading | BoundValue | "lazy", "eager" |
| aspectRatio | BoundValue | "16/9", "4/3", "1/1" |

### icon
Lucide icon display.
| Property | Type | Description |
|----------|------|-------------|
| name | BoundValue | Lucide icon name (REQUIRED): "user", "settings", "chevron-right", "home", "search", "menu", "x", "plus", "minus", "check", "alert-circle", "info", "mail", "phone", "calendar", "clock", "star", "heart", "bookmark", "share", "download", "upload", "edit", "trash", "copy", "link", "external-link", "eye", "eye-off", "lock", "unlock", "filter", "sort", "refresh", "loader" |
| size | BoundValue | "xs", "sm", "md", "lg", "xl" or number |
| color | BoundValue | Tailwind class |
| strokeWidth | BoundValue | number (default 2) |

### video
Video player.
| Property | Type | Description |
|----------|------|-------------|
| src | BoundValue | Video URL (REQUIRED) |
| poster | BoundValue | Poster image URL |
| autoplay | BoundValue | boolean |
| loop | BoundValue | boolean |
| muted | BoundValue | boolean |
| controls | BoundValue | boolean |
| width | BoundValue | string |
| height | BoundValue | string |

### lottie
Lottie animation player.
| Property | Type | Description |
|----------|------|-------------|
| src | BoundValue | Lottie JSON URL (REQUIRED) |
| autoplay | BoundValue | boolean |
| loop | BoundValue | boolean |
| speed | BoundValue | number (1 = normal) |
| width | BoundValue | string |
| height | BoundValue | string |

### markdown
Markdown content renderer.
| Property | Type | Description |
|----------|------|-------------|
| content | BoundValue | Markdown string (REQUIRED) |
| allowHtml | BoundValue | boolean |

### badge
Small label/tag.
| Property | Type | Description |
|----------|------|-------------|
| content | BoundValue | Badge text (REQUIRED) |
| variant | BoundValue | "default", "secondary", "destructive", "outline" |
| color | BoundValue | Tailwind class |

### avatar
User avatar display.
| Property | Type | Description |
|----------|------|-------------|
| src | BoundValue | Image URL |
| fallback | BoundValue | Fallback initials |
| size | BoundValue | "sm", "md", "lg", "xl" |

### progress
Progress indicator.
| Property | Type | Description |
|----------|------|-------------|
| value | BoundValue | Current value (REQUIRED) |
| max | BoundValue | Maximum value (default 100) |
| showLabel | BoundValue | boolean |
| variant | BoundValue | "default", "success", "warning", "error" |
| color | BoundValue | Tailwind class |

### spinner
Loading spinner.
| Property | Type | Description |
|----------|------|-------------|
| size | BoundValue | "sm", "md", "lg" |
| color | BoundValue | Tailwind class |

### divider
Visual separator.
| Property | Type | Description |
|----------|------|-------------|
| orientation | BoundValue | "horizontal", "vertical" |
| thickness | BoundValue | string |
| color | BoundValue | Tailwind class |

### skeleton
Loading placeholder.
| Property | Type | Description |
|----------|------|-------------|
| width | BoundValue | string |
| height | BoundValue | string |
| rounded | BoundValue | boolean |

### iframe
Embedded external content.
| Property | Type | Description |
|----------|------|-------------|
| src | BoundValue | URL (REQUIRED) |
| title | BoundValue | Frame title |
| width | BoundValue | string |
| height | BoundValue | string |
| sandbox | BoundValue | Sandbox restrictions |
| allow | BoundValue | Permissions policy |
| loading | BoundValue | "lazy", "eager" |

### table
Data table.
| Property | Type | Description |
|----------|------|-------------|
| columns | BoundValue | Array of {{id, header, accessor?, width?, align?, sortable?}} (REQUIRED) |
| data | BoundValue | Array of row objects (REQUIRED) |
| caption | BoundValue | Table caption |
| striped | BoundValue | boolean |
| bordered | BoundValue | boolean |
| hoverable | BoundValue | boolean |
| compact | BoundValue | boolean |
| stickyHeader | BoundValue | boolean |
| sortable | BoundValue | boolean |
| searchable | BoundValue | boolean |
| paginated | BoundValue | boolean |
| pageSize | BoundValue | number |
| selectable | BoundValue | boolean |

### plotlyChart
Interactive Plotly charts.
| Property | Type | Description |
|----------|------|-------------|
| chartType | BoundValue | "line", "bar", "scatter", "pie", "area", "histogram" |
| title | BoundValue | Chart title |
| data | BoundValue | Plotly data array |
| layout | BoundValue | Plotly layout object |
| width | BoundValue | Chart width |
| height | BoundValue | Chart height |
| responsive | BoundValue | boolean |
| showLegend | BoundValue | boolean |
| legendPosition | BoundValue | "top", "bottom", "left", "right" |

### nivoChart
Nivo chart library (bar, line, pie, radar, heatmap, scatter, funnel, treemap, sunburst, calendar, sankey, chord, etc.)
| Property | Type | Description |
|----------|------|-------------|
| chartType | BoundValue | "bar", "line", "pie", "radar", "heatmap", "scatter", "funnel", "treemap", "sunburst", "calendar", "bump", "areaBump", "sankey", "chord", etc. (REQUIRED) |
| title | BoundValue | Chart title |
| data | BoundValue | Chart data (format depends on chartType) |
| height | BoundValue | Chart height (default "400px") |
| colors | BoundValue | Color scheme ("nivo", "paired") or array of colors |
| animate | BoundValue | boolean |
| showLegend | BoundValue | boolean |
| legendPosition | BoundValue | "top", "bottom", "left", "right" |
| indexBy | BoundValue | Key for indexing (bar, radar) |
| keys | BoundValue | Data keys to display (bar, radar) |
| margin | BoundValue | {{top, right, bottom, left}} |
| axisBottom | BoundValue | Bottom axis config |
| axisLeft | BoundValue | Left axis config |

### filePreview
Generic file preview (images, videos, PDFs, etc.)
| Property | Type | Description |
|----------|------|-------------|
| src | BoundValue | File URL (REQUIRED) |
| showControls | BoundValue | boolean |
| fit | BoundValue | "contain", "cover", "fill", "none", "scaleDown" |
| fallbackText | BoundValue | Fallback text |

### boundingBoxOverlay
Display bounding boxes on an image.
| Property | Type | Description |
|----------|------|-------------|
| src | BoundValue | Image URL (REQUIRED) |
| alt | BoundValue | Alt text |
| boxes | BoundValue | Array of {{id?, x, y, width, height, label?, confidence?, color?}} (REQUIRED) |
| showLabels | BoundValue | boolean |
| showConfidence | BoundValue | boolean |
| strokeWidth | BoundValue | number |
| fontSize | BoundValue | number |
| fit | BoundValue | "contain", "cover", "fill" |
| normalized | BoundValue | boolean - if true, coordinates are 0-1 |
| interactive | BoundValue | boolean - enable click events |

---
## INTERACTIVE COMPONENTS

### button
Clickable button.
| Property | Type | Description |
|----------|------|-------------|
| label | BoundValue | Button text (REQUIRED) |
| variant | BoundValue | "default", "secondary", "outline", "ghost", "destructive", "link" |
| size | BoundValue | "sm", "md", "lg", "icon" |
| disabled | BoundValue | boolean |
| loading | BoundValue | boolean |
| icon | BoundValue | Lucide icon name |
| iconPosition | BoundValue | "left", "right" |
| tooltip | BoundValue | Hover tooltip |

### textField
Text input field.
| Property | Type | Description |
|----------|------|-------------|
| value | BoundValue | Current value (REQUIRED) |
| placeholder | BoundValue | Placeholder text |
| label | BoundValue | Field label |
| helperText | BoundValue | Helper text below |
| error | BoundValue | Error message |
| disabled | BoundValue | boolean |
| inputType | BoundValue | "text", "email", "password", "number", "tel", "url", "search" |
| multiline | BoundValue | boolean - textarea |
| rows | BoundValue | number |
| maxLength | BoundValue | number |
| required | BoundValue | boolean |

### select
Dropdown selection.
| Property | Type | Description |
|----------|------|-------------|
| value | BoundValue | Selected value (REQUIRED) |
| options | BoundValue | Array of {{value, label}} (REQUIRED) |
| placeholder | BoundValue | Placeholder text |
| label | BoundValue | Field label |
| disabled | BoundValue | boolean |
| multiple | BoundValue | boolean |
| searchable | BoundValue | boolean |

### slider
Range slider.
| Property | Type | Description |
|----------|------|-------------|
| value | BoundValue | Current value (REQUIRED) |
| min | BoundValue | number |
| max | BoundValue | number |
| step | BoundValue | number |
| disabled | BoundValue | boolean |
| showValue | BoundValue | boolean |
| label | BoundValue | string |

### checkbox
Boolean toggle.
| Property | Type | Description |
|----------|------|-------------|
| checked | BoundValue | boolean (REQUIRED) |
| label | BoundValue | string |
| disabled | BoundValue | boolean |
| indeterminate | BoundValue | boolean |

### switch
Toggle switch.
| Property | Type | Description |
|----------|------|-------------|
| checked | BoundValue | boolean (REQUIRED) |
| label | BoundValue | string |
| disabled | BoundValue | boolean |

### radioGroup
Radio button group.
| Property | Type | Description |
|----------|------|-------------|
| value | BoundValue | Selected value (REQUIRED) |
| options | BoundValue | Array of {{value, label}} (REQUIRED) |
| disabled | BoundValue | boolean |
| orientation | BoundValue | "horizontal", "vertical" |
| label | BoundValue | Group label |

### dateTimeInput
Date/time picker.
| Property | Type | Description |
|----------|------|-------------|
| value | BoundValue | ISO string (REQUIRED) |
| mode | BoundValue | "date", "time", "datetime" |
| min | BoundValue | ISO string |
| max | BoundValue | ISO string |
| disabled | BoundValue | boolean |
| label | BoundValue | string |

### fileInput
File upload.
| Property | Type | Description |
|----------|------|-------------|
| value | BoundValue | File data |
| label | BoundValue | Field label |
| helperText | BoundValue | Helper text |
| accept | BoundValue | ".pdf,.doc" etc. |
| multiple | BoundValue | boolean |
| maxSize | BoundValue | number (bytes) |
| maxFiles | BoundValue | number |
| disabled | BoundValue | boolean |
| error | BoundValue | Error message |

### imageInput
Image upload with preview.
| Property | Type | Description |
|----------|------|-------------|
| value | BoundValue | Image data |
| label | BoundValue | Field label |
| accept | BoundValue | Accepted types |
| multiple | BoundValue | boolean |
| maxSize | BoundValue | number |
| aspectRatio | BoundValue | Crop ratio |
| showPreview | BoundValue | boolean |
| disabled | BoundValue | boolean |

### link
Navigation link.
| Property | Type | Description |
|----------|------|-------------|
| href | BoundValue | URL (REQUIRED) |
| label | BoundValue | Link text |
| route | BoundValue | Internal route |
| external | boolean | External link |
| target | string | "_blank", "_self" |
| variant | string | "default", "muted", "primary", "destructive" |
| underline | string | "always", "hover", "none" |
| disabled | BoundValue | boolean |

### imageLabeler
Draw bounding boxes on images for labeling.
| Property | Type | Description |
|----------|------|-------------|
| src | BoundValue | Image URL (REQUIRED) |
| alt | BoundValue | Alt text |
| boxes | BoundValue | Initial boxes: {{id, x, y, width, height, label}}[] |
| labels | BoundValue | Available labels: string[] (REQUIRED) |
| disabled | BoundValue | boolean |
| showLabels | BoundValue | boolean |
| minBoxSize | BoundValue | Minimum box size in pixels |

### imageHotspot
Interactive image with clickable hotspots.
| Property | Type | Description |
|----------|------|-------------|
| src | BoundValue | Image URL (REQUIRED) |
| alt | BoundValue | Alt text |
| hotspots | BoundValue | Array of {{id, x, y, size?, color?, icon?, label?, description?, action?, disabled?}} (REQUIRED) |
| showMarkers | BoundValue | boolean |
| markerStyle | BoundValue | "pulse", "dot", "ring", "square", "diamond", "none" |
| fit | BoundValue | "contain", "cover", "fill" |
| normalized | BoundValue | boolean - if true, coordinates are 0-1 |
| showTooltips | BoundValue | boolean |

---
## CONTAINER COMPONENTS

### card
Content container.
| Property | Type | Description |
|----------|------|-------------|
| title | BoundValue | Card title |
| description | BoundValue | Card description |
| footer | BoundValue | Footer content |
| hoverable | BoundValue | boolean |
| clickable | BoundValue | boolean |
| variant | BoundValue | "default", "bordered", "elevated" |
| padding | BoundValue | string |
| headerImage | BoundValue | URL |
| headerIcon | BoundValue | Icon name |
| children | Children | Card body content |

### modal
Dialog overlay.
| Property | Type | Description |
|----------|------|-------------|
| open | BoundValue | boolean (REQUIRED) |
| title | BoundValue | string |
| description | BoundValue | string |
| closeOnOverlay | BoundValue | boolean |
| closeOnEscape | BoundValue | boolean |
| showCloseButton | BoundValue | boolean |
| size | BoundValue | "sm", "md", "lg", "xl", "full" |
| centered | BoundValue | boolean |
| children | Children | Modal content |

### tabs
Tabbed content.
| Property | Type | Description |
|----------|------|-------------|
| value | BoundValue | Active tab ID (REQUIRED) |
| tabs | array | {{id, label, icon?, disabled?, contentComponentId}} (REQUIRED) |
| orientation | BoundValue | "horizontal", "vertical" |
| variant | BoundValue | "default", "pills", "underline" |

### accordion
Collapsible sections.
| Property | Type | Description |
|----------|------|-------------|
| items | array | {{id, title, contentComponentId}} (REQUIRED) |
| multiple | BoundValue | boolean |
| defaultExpanded | BoundValue | array of IDs |
| collapsible | BoundValue | boolean |

### drawer
Slide-out panel.
| Property | Type | Description |
|----------|------|-------------|
| open | BoundValue | boolean (REQUIRED) |
| side | BoundValue | "left", "right", "top", "bottom" |
| title | BoundValue | string |
| size | BoundValue | string |
| overlay | BoundValue | boolean |
| closable | BoundValue | boolean |
| children | Children | Drawer content |

### tooltip
Hover tooltip.
| Property | Type | Description |
|----------|------|-------------|
| content | BoundValue | Tooltip text (REQUIRED) |
| side | BoundValue | "top", "right", "bottom", "left" |
| delayMs | BoundValue | number |
| maxWidth | BoundValue | string |
| children | Children | Trigger element |

### popover
Click popover.
| Property | Type | Description |
|----------|------|-------------|
| open | BoundValue | boolean |
| contentComponentId | string | Component ID (REQUIRED) |
| side | BoundValue | "top", "right", "bottom", "left" |
| trigger | BoundValue | "click", "hover" |
| closeOnClickOutside | BoundValue | boolean |
| children | Children | Trigger element |

---
## Styling Rules
ALWAYS use shadcn theme variables: bg-background, text-foreground, bg-muted, text-muted-foreground, bg-primary, text-primary-foreground, bg-secondary, text-secondary-foreground, bg-accent, bg-card, border-border, ring-ring
NEVER use hardcoded colors (bg-white, text-black, bg-gray-*, text-gray-*)

## CUSTOM CSS INJECTION
You CAN use `canvasSettings.customCss` for advanced effects not achievable with Tailwind classes:
```json
{{"canvasSettings": {{"backgroundColor": "bg-background", "padding": "1rem", "customCss": ".my-class {{ animation: pulse 2s infinite; }} @keyframes pulse {{ 0%,100%{{ opacity:1 }} 50%{{ opacity:0.5 }} }}"}}}}
```
**Good use cases for customCss:**
- Custom keyframe animations
- Complex gradients with ::before/::after
- Hover/focus states beyond Tailwind
- CSS variables for theming
- Pseudo-elements for decorative effects

**Prefer Tailwind first** - Only use customCss when standard classes won't work.

## RESPONSIVE DESIGN (CRITICAL)
Always design mobile-first with responsive breakpoints:
- Base styles: mobile (< 640px)
- sm: ≥ 640px, md: ≥ 768px, lg: ≥ 1024px, xl: ≥ 1280px, 2xl: ≥ 1536px

Examples: `grid-cols-1 sm:grid-cols-2 lg:grid-cols-3`, `flex-col md:flex-row`, `text-sm md:text-base lg:text-lg`, `p-4 md:p-6 lg:p-8`, `hidden md:block`

## CRITICAL EFFICIENCY
1. Call emit_surface ONCE with ALL components - do NOT call multiple times
2. Skip get_component_schema and get_style_examples - you know the patterns
3. Generate complete component trees in a single response
4. Aim for 1-2 tool calls total (emit_surface is usually sufficient)"#,
            context = context_json
        )
    }
}
