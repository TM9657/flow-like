use crate::history::{
    self, Content as HistoryContent, History, MessageContent, ResponseFormat, Role, ToolChoice,
};
use openai_api_rs::v1::{
    chat_completion::{
        ChatCompletionMessage, ChatCompletionRequest, Content, ContentType, ImageUrl, ImageUrlType,
        MessageRole, Tool, ToolCall, ToolCallFunction, ToolType,
    },
    types::{Function, FunctionParameters, JSONSchemaDefine, JSONSchemaType},
};

impl From<History> for ChatCompletionRequest {
    fn from(history: History) -> Self {
        // Convert HistoryMessage to ChatCompletionMessage
        let messages = history
            .messages
            .into_iter()
            .map(|msg| ChatCompletionMessage {
                role: match msg.role {
                    Role::System => MessageRole::system,
                    Role::User => MessageRole::user,
                    Role::Assistant => MessageRole::assistant,
                    Role::Function => MessageRole::function,
                    Role::Tool => MessageRole::tool,
                },
                content: {
                    let mut content_vec: Vec<HistoryContent> = match msg.content {
                        MessageContent::String(ref text) => vec![HistoryContent::Text {
                            content_type: history::ContentType::Text,
                            text: text.clone(),
                        }],
                        MessageContent::Contents(ref contents) => contents.clone(),
                    };

                    if content_vec.len() == 1 {
                        match content_vec.pop().unwrap() {
                            HistoryContent::Text {
                                content_type: _,
                                text,
                            } => Content::Text(text),
                            HistoryContent::Image {
                                content_type: _,
                                image_url,
                                ..
                            } => Content::ImageUrl(vec![ImageUrl {
                                r#type: ContentType::image_url,
                                text: None,
                                image_url: Some(ImageUrlType { url: image_url.url }),
                            }]),
                        }
                    } else {
                        let image_urls = content_vec
                            .into_iter()
                            .filter_map(|c| match c {
                                HistoryContent::Text {
                                    content_type: _,
                                    text,
                                } => Some(ImageUrl {
                                    r#type: ContentType::text,
                                    text: Some(text),
                                    image_url: None,
                                }),
                                HistoryContent::Image {
                                    content_type: _,
                                    image_url,
                                    ..
                                } => Some(ImageUrl {
                                    r#type: ContentType::image_url,
                                    text: None,
                                    image_url: Some(ImageUrlType { url: image_url.url }),
                                }),
                            })
                            .collect::<Vec<ImageUrl>>();
                        Content::ImageUrl(image_urls)
                    }
                },
                name: msg.name.clone(),
                tool_calls: msg.tool_calls.clone().map(|tool_calls| {
                    tool_calls
                        .into_iter()
                        .map(|tool_call| ToolCall {
                            function: ToolCallFunction {
                                name: tool_call.function.name,
                                arguments: tool_call.function.arguments,
                            },
                            id: tool_call.id,
                            r#type: tool_call.r#type,
                        })
                        .collect()
                }),
                tool_call_id: msg.tool_call_id.clone(),
            })
            .collect();

        let tools = history
            .tools
            .map(|tools| tools.into_iter().map(Tool::from).collect());

        let tool_choice = history.tool_choice.map(|tc| match tc {
            ToolChoice::None => openai_api_rs::v1::chat_completion::ToolChoiceType::None,
            ToolChoice::Auto => openai_api_rs::v1::chat_completion::ToolChoiceType::Auto,
            ToolChoice::Required => openai_api_rs::v1::chat_completion::ToolChoiceType::Required,
            ToolChoice::Specific { r#type, function } => {
                openai_api_rs::v1::chat_completion::ToolChoiceType::ToolChoice {
                    tool: Tool {
                        r#type: match r#type {
                            crate::history::ToolType::Function => ToolType::Function,
                        },
                        function: Function {
                            description: function.description,
                            name: function.name,
                            parameters: FunctionParameters::from(function.parameters),
                        },
                    },
                }
            }
        });

        // Map fields directly where possible
        ChatCompletionRequest {
            model: history.model,
            messages,
            reasoning: None,
            temperature: history.temperature.map(|t| t as f64),
            top_p: history.top_p.map(|t| t as f64),
            n: history.n.map(|n| n as i64),
            response_format: history.response_format.map(|rf| match rf {
                ResponseFormat::String(s) => flow_like_types::Value::String(s),
                ResponseFormat::Object(v) => v, // Assuming flow_like_types::Value implements Into<serde_json::Value>
            }),
            stream: history.stream,
            stop: history.stop,
            max_tokens: history.max_completion_tokens.map(|m| m as i64),
            presence_penalty: history.presence_penalty.map(|p| p as f64),
            frequency_penalty: history.frequency_penalty.map(|f| f as f64),
            logit_bias: None,
            user: history.user,
            seed: history.seed.map(|s| s as i64), // u32 to i64
            tools,
            parallel_tool_calls: None,
            tool_choice,
        }
    }
}

impl From<crate::history::Tool> for Tool {
    fn from(tool: crate::history::Tool) -> Self {
        Tool {
            r#type: match tool.tool_type {
                crate::history::ToolType::Function => ToolType::Function,
            },
            function: Function {
                name: tool.function.name,
                description: tool.function.description,
                parameters: FunctionParameters::from(tool.function.parameters),
            },
        }
    }
}

impl From<crate::history::HistoryJSONSchemaType> for JSONSchemaType {
    fn from(history_type: crate::history::HistoryJSONSchemaType) -> Self {
        match history_type {
            crate::history::HistoryJSONSchemaType::Object => JSONSchemaType::Object,
            crate::history::HistoryJSONSchemaType::Number => JSONSchemaType::Number,
            crate::history::HistoryJSONSchemaType::String => JSONSchemaType::String,
            crate::history::HistoryJSONSchemaType::Array => JSONSchemaType::Array,
            crate::history::HistoryJSONSchemaType::Null => JSONSchemaType::Null,
            crate::history::HistoryJSONSchemaType::Boolean => JSONSchemaType::Boolean,
        }
    }
}

impl From<crate::history::HistoryJSONSchemaDefine> for JSONSchemaDefine {
    fn from(history_define: crate::history::HistoryJSONSchemaDefine) -> Self {
        JSONSchemaDefine {
            schema_type: history_define.schema_type.map(|t| t.into()),
            description: history_define.description,
            enum_values: history_define.enum_values,
            properties: history_define.properties.map(|props| {
                props
                    .into_iter()
                    .map(|(k, v)| (k, Box::new((*v).into())))
                    .collect()
            }),
            required: history_define.required,
            items: history_define.items.map(|item| Box::new((*item).into())),
        }
    }
}

impl From<crate::history::HistoryFunctionParameters> for FunctionParameters {
    fn from(params: crate::history::HistoryFunctionParameters) -> Self {
        FunctionParameters {
            schema_type: params.schema_type.into(),
            properties: params.properties.map(|props| {
                props
                    .into_iter()
                    .map(|(k, v)| (k, Box::new((*v).into())))
                    .collect()
            }),
            required: params.required,
        }
    }
}
