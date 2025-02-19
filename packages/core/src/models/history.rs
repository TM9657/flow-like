// Implementation according to
// https://modelcontextprotocol.io/docs/concepts/sampling/

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
#[derive(Serialize, Deserialize, JsonSchema, Debug, Clone)]
#[serde(rename_all = "lowercase")]
pub struct HistoryMessage {
    pub role: Role,
    pub content: Vec<Content>,
}

impl HistoryMessage {
    pub fn from_string(role: Role, content: &str) -> Self {
        Self {
            role,
            content: vec![Content::Text {
                content_type: ContentType::Text,
                text: content.to_string(),
            }],
        }
    }
}

#[derive(Serialize, Deserialize, JsonSchema, Debug, Clone, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum Role {
    System,
    User,
    Assistant,
}

#[derive(Serialize, Deserialize, JsonSchema, Debug, Clone)]
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
        data: String,
        mime_type: String,
    },
}

#[derive(Serialize, Deserialize, JsonSchema, Debug, Clone)]
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
    Object(serde_json::Value),
}

#[derive(Serialize, Deserialize, JsonSchema, Debug, Clone)]
pub struct StreamOptions {
    pub include_usage: bool,
}

#[derive(Serialize, Deserialize, JsonSchema, Debug, Clone)]
pub struct History {
    pub model: String,
    pub messages: Vec<HistoryMessage>,

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
}

impl History {
    pub fn new(model: String, messages: Vec<HistoryMessage>) -> Self {
        Self {
            model,
            messages,
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
        }
    }

    pub fn push_message(&mut self, message: HistoryMessage) {
        self.messages.push(message);
    }

    pub fn set_system_prompt(&mut self, prompt: String) {
        let system_prompt_index = self
            .messages
            .iter()
            .position(|message| message.role == Role::System);

        if let Some(index) = system_prompt_index {
            self.messages[index].content = vec![Content::Text {
                content_type: ContentType::Text,
                text: prompt,
            }];
            return;
        }

        self.messages.insert(
            0,
            HistoryMessage {
                role: Role::System,
                content: vec![Content::Text {
                    content_type: ContentType::Text,
                    text: prompt,
                }],
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
    #[serde(skip_serializing_if = "Option::is_none")]
    pub parameters: Option<serde_json::Value>,
    pub strict: Option<bool>,
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
