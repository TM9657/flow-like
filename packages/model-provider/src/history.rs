// Implementation according to
// https://modelcontextprotocol.io/docs/concepts/sampling/

use flow_like_types::{Value, json};
use schemars::JsonSchema;
use serde::{Deserialize, Deserializer, Serialize};
use std::collections::HashMap;
use std::fmt;

use crate::response::{Annotation, Response};

#[derive(Debug, Deserialize, Serialize, JsonSchema, Clone)]
pub struct ToolCall {
    pub id: String,
    pub r#type: String,
    pub function: ToolCallFunction,
}

#[derive(Debug, Deserialize, Serialize, JsonSchema, Clone)]
pub struct ToolCallFunction {
    //#[serde(skip_serializing_if = "Option::is_none")]
    pub name: String,
    #[serde(deserialize_with = "arguments_as_str")]
    pub arguments: String,
}

/// Handles arguments incoming as str (e.g. for cloud-based LLM providers) or map (local LLM providers)
fn arguments_as_str<'de, D>(deserializer: D) -> Result<String, D::Error>
where
    D: Deserializer<'de>,
{
    let v = Value::deserialize(deserializer)?;
    match v {
        Value::String(s) => Ok(s), // already a string
        other => json::to_string(&other).map_err(serde::de::Error::custom), // object/array/number → stringified JSON
    }
}

#[derive(Serialize, Deserialize, JsonSchema, Debug, Clone, PartialEq)]
#[serde(untagged)]
pub enum MessageContent {
    String(String),
    Contents(Vec<Content>),
}

#[derive(Serialize, Deserialize, JsonSchema, Debug, Clone)]
#[serde(rename_all = "lowercase")]
pub struct HistoryMessage {
    pub role: Role,
    pub content: MessageContent,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tool_calls: Option<Vec<ToolCall>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tool_call_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub annotations: Option<Vec<Annotation>>,
}

impl HistoryMessage {
    pub fn from_string(role: Role, content: &str) -> Self {
        Self {
            role,
            content: MessageContent::Contents(vec![Content::Text {
                content_type: ContentType::Text,
                text: content.to_string(),
            }]),
            name: None,
            tool_call_id: None,
            tool_calls: None,
            annotations: None,
        }
    }

    pub fn from_response(response: Response) -> Self {
        let first_choice = response.choices.first();

        let content = match first_choice {
            Some(choice) => choice.message.content.clone(),
            None => None,
        };
        let annotations = match first_choice {
            Some(choice) => choice.message.annotations.clone(),
            None => None,
        };

        let role: Role = match first_choice {
            Some(choice) => match choice.message.role.as_str() {
                "user" => Role::User,
                "assistant" => Role::Assistant,
                "system" => Role::System,
                _ => Role::Assistant,
            },
            None => Role::Assistant,
        };

        Self {
            role,
            content: MessageContent::Contents(vec![Content::Text {
                content_type: ContentType::Text,
                text: content.unwrap_or_default(),
            }]),
            name: None,
            tool_call_id: None,
            tool_calls: None,
            annotations,
        }
    }
}

impl HistoryMessage {
    /// Returns a copy of the entire text-related content as single String
    pub fn as_str(&self) -> String {
        match &self.content {
            MessageContent::String(s) => s.clone(),
            MessageContent::Contents(contents) => contents
                .iter()
                .filter_map(|content| {
                    if let Content::Text { text, .. } = content {
                        Some(text.as_str())
                    } else {
                        None
                    }
                })
                .collect::<Vec<&str>>()
                .join("\n"),
        }
    }
}

impl fmt::Display for History {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if !self.messages.is_empty() {
            let mut history_str = String::from("| ");
            for message in self.messages.iter() {
                let m = match message.role {
                    Role::Assistant => " A |",
                    Role::System => " S |",
                    Role::Tool => " T |",
                    Role::User => " H |",
                    Role::Function => " F |",
                };
                history_str.push_str(m);
            }
            write!(f, "{}", history_str)
        } else {
            write!(f, "[]")
        }
    }
}

#[derive(Serialize, Deserialize, JsonSchema, Debug, Clone, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum Role {
    System,
    User,
    Assistant,
    Function,
    Tool,
}

#[derive(Serialize, Deserialize, JsonSchema, Debug, Clone, PartialEq)]
pub struct ImageUrl {
    pub url: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub detail: Option<String>,
}

#[derive(Serialize, Deserialize, JsonSchema, Debug, Clone, PartialEq)]
#[serde(rename_all = "lowercase")]
#[serde(untagged)]
pub enum Content {
    Text {
        #[serde(rename = "type")]
        content_type: ContentType,
        text: String,
    },
    Image {
        #[serde(rename = "type")]
        content_type: ContentType,
        image_url: ImageUrl,
    },
}

#[derive(Serialize, Deserialize, JsonSchema, Debug, Clone, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum ContentType {
    Text,
    #[serde(rename = "image_url")]
    ImageUrl,
}

#[derive(Serialize, Deserialize, JsonSchema, Debug, Clone)]
#[serde(untagged)]
pub enum ResponseFormat {
    String(String),
    Object(flow_like_types::Value),
}

#[derive(Serialize, Deserialize, JsonSchema, Debug, Clone)]
pub struct StreamOptions {
    pub include_usage: bool,
}

#[derive(Serialize, Deserialize, JsonSchema, Debug, Clone)]
pub struct Usage {
    pub include: bool,
}

#[derive(Serialize, Deserialize, JsonSchema, Debug, Clone)]
pub struct History {
    pub model: String,
    pub messages: Vec<HistoryMessage>,

    pub preset: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub stream: Option<bool>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub stream_options: Option<StreamOptions>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_completion_tokens: Option<u32>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub top_p: Option<f32>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub temperature: Option<f32>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub seed: Option<u32>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub presence_penalty: Option<f32>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub frequency_penalty: Option<f32>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub user: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub stop: Option<Vec<String>>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub response_format: Option<ResponseFormat>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub n: Option<u32>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub tools: Option<Vec<Tool>>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub tool_choice: Option<ToolChoice>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub usage: Option<Usage>,
}

impl History {
    pub fn new(model: String, messages: Vec<HistoryMessage>) -> Self {
        Self {
            model,
            messages,
            preset: None,
            stream: Some(true),
            stream_options: None,
            max_completion_tokens: None,
            top_p: None,
            temperature: None,
            seed: None,
            presence_penalty: None,
            frequency_penalty: None,
            user: None,
            stop: None,
            response_format: None,
            n: None,
            tools: None,
            tool_choice: None,
            usage: None,
        }
    }

    pub fn push_message(&mut self, message: HistoryMessage) {
        self.messages.push(message);
    }

    pub fn get_system_prompt_index(&self) -> Option<usize> {
        self.messages
            .iter()
            .position(|message| message.role == Role::System)
    }

    pub fn get_system_prompt(&self) -> Option<String> {
        if let Some(index) = self.get_system_prompt_index() {
            match &self.messages[index].content {
                MessageContent::Contents(contents) => {
                    let mut prompt = String::new();
                    for content in contents {
                        if let Content::Text {
                            content_type: _,
                            text,
                        } = content
                        {
                            prompt.push_str(text);
                        }
                    }
                    return Some(prompt);
                }
                MessageContent::String(content) => return Some(content.to_string()),
            }
        }
        None
    }

    pub fn set_system_prompt(&mut self, prompt: String) {
        if let Some(index) = self.get_system_prompt_index() {
            self.messages[index].content = MessageContent::Contents(vec![Content::Text {
                content_type: ContentType::Text,
                text: prompt,
            }]);
            return;
        }

        self.messages.insert(
            0,
            HistoryMessage {
                role: Role::System,
                content: MessageContent::Contents(vec![Content::Text {
                    content_type: ContentType::Text,
                    text: prompt,
                }]),
                name: None,
                tool_call_id: None,
                tool_calls: None,
                annotations: None,
            },
        );
    }

    pub fn set_stream(&mut self, stream: bool) {
        self.stream = Some(stream);
    }
}

#[derive(Serialize, Deserialize, JsonSchema, Debug, Clone)]
pub struct Tool {
    #[serde(rename = "type")]
    pub tool_type: ToolType,
    pub function: HistoryFunction,
}

#[derive(Serialize, Deserialize, JsonSchema, Debug, Clone)]
#[serde(rename_all = "lowercase")]
pub enum ToolType {
    Function,
}

#[derive(Serialize, Deserialize, JsonSchema, Debug, Clone)]
pub struct HistoryFunction {
    pub name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    pub parameters: HistoryFunctionParameters,
}

#[derive(Serialize, Deserialize, JsonSchema, Debug, Clone)]
pub struct HistoryFunctionParameters {
    #[serde(rename = "type")]
    pub schema_type: HistoryJSONSchemaType,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub properties: Option<HashMap<String, Box<HistoryJSONSchemaDefine>>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub required: Option<Vec<String>>,
}

#[derive(Serialize, Deserialize, JsonSchema, Debug, Clone)]
#[serde(rename_all = "lowercase")]
pub enum HistoryJSONSchemaType {
    Object,
    Number,
    String,
    Array,
    Null,
    Boolean,
}

#[derive(Serialize, Deserialize, JsonSchema, Debug, Clone)]
pub struct HistoryJSONSchemaDefine {
    #[serde(rename = "type")]
    pub schema_type: Option<HistoryJSONSchemaType>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none", rename = "enum")]
    pub enum_values: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub properties: Option<HashMap<String, Box<HistoryJSONSchemaDefine>>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub required: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub items: Option<Box<HistoryJSONSchemaDefine>>,
}

#[derive(Serialize, Deserialize, JsonSchema, Debug, Clone)]
#[serde(rename_all = "lowercase", untagged)]
pub enum ToolChoice {
    None,
    Auto,
    Required,
    Specific {
        r#type: ToolType,
        function: HistoryFunction,
    },
}
