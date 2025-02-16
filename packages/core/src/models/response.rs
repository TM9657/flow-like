use super::response_chunk::{Delta, ResponseChunk};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, JsonSchema, Debug, Clone)]
pub struct FunctionCall {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub index: Option<i32>,
    pub id: String,
    #[serde(rename = "type")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tool_type: Option<String>,
    pub function: ResponseFunction,
}

impl Default for FunctionCall {
    fn default() -> Self {
        FunctionCall {
            index: None,
            id: "".to_string(),
            tool_type: None,
            function: ResponseFunction {
                name: None,
                arguments: None,
            },
        }
    }
}

#[derive(Serialize, Deserialize, JsonSchema, Debug, Clone)]
pub struct ResponseFunction {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub arguments: Option<String>,
}

#[derive(Serialize, Deserialize, JsonSchema, Debug, Clone)]
pub struct LogProbs {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub content: Option<Vec<TokenLogProbs>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub refusal: Option<Vec<TokenLogProbs>>,
}

#[derive(Serialize, Deserialize, JsonSchema, Debug, Clone)]
pub struct TokenLogProbs {
    pub token: String,
    pub logprob: f64,
    pub bytes: Option<Vec<u8>>,
    pub top_logprobs: Option<Vec<TopLogProbs>>,
}

#[derive(Serialize, Deserialize, JsonSchema, Debug, Clone)]
pub struct TopLogProbs {
    pub token: String,
    pub logprob: f64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub bytes: Option<Vec<u8>>,
}

#[derive(Serialize, Deserialize, JsonSchema, Debug, Clone)]
pub struct Choice {
    index: i32,
    finish_reason: String,
    message: ResponseMessage,
    #[serde(skip_serializing_if = "Option::is_none")]
    logprobs: Option<LogProbs>,
}

#[derive(Serialize, Deserialize, JsonSchema, Debug, Clone)]
pub struct ResponseMessage {
    pub role: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub content: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub refusal: Option<String>,

    #[serde(default)]
    pub tool_calls: Vec<FunctionCall>,
}

impl Default for ResponseMessage {
    fn default() -> Self {
        ResponseMessage {
            content: None,
            refusal: None,
            tool_calls: vec![],
            role: "".to_string(),
        }
    }
}

impl ResponseMessage {
    pub fn apply_delta(&mut self, delta: Delta) {
        if let Some(content) = delta.content {
            self.content = Some(self.content.as_deref().unwrap_or("").to_string() + &content);
        }

        if let Some(refusal) = delta.refusal {
            self.refusal = Some(self.refusal.as_deref().unwrap_or("").to_string() + &refusal);
        }

        if let Some(role) = delta.role {
            if role != self.role {
                self.role = self.role.to_string() + &role;
            }
        }

        if delta.tool_calls.is_none() {
            return;
        }

        for function_call in delta.tool_calls.unwrap() {
            // Check if a choice with the same index already exists
            if let Some(existing_tool_call) = self
                .tool_calls
                .iter_mut()
                .find(|c| c.index == function_call.index)
            {
                existing_tool_call.id = function_call.id;

                if let Some(tool_type) = function_call.tool_type {
                    existing_tool_call.tool_type = Some(
                        existing_tool_call
                            .tool_type
                            .as_deref()
                            .unwrap_or("")
                            .to_string()
                            + &tool_type,
                    );
                }

                if let Some(function_name) = function_call.function.name {
                    existing_tool_call.function.name = Some(
                        existing_tool_call
                            .function
                            .name
                            .as_deref()
                            .unwrap_or("")
                            .to_string()
                            + &function_name,
                    );
                }

                if let Some(arguments) = function_call.function.arguments {
                    existing_tool_call.function.arguments = Some(
                        existing_tool_call
                            .function
                            .arguments
                            .as_deref()
                            .unwrap_or("")
                            .to_string()
                            + &arguments,
                    );
                }

                return;
            }

            self.tool_calls.push(FunctionCall {
                index: function_call.index,
                id: function_call.id,
                tool_type: function_call.tool_type,
                function: ResponseFunction {
                    name: function_call.function.name,
                    arguments: function_call.function.arguments,
                },
            });
        }
    }
}

#[derive(Serialize, Deserialize, JsonSchema, Debug, Clone, Default)]
pub struct Usage {
    completion_tokens: u32,
    prompt_tokens: u32,
    total_tokens: u32,
    #[serde(skip_serializing_if = "Option::is_none")]
    prompt_tokens_details: Option<PromptTokenDetails>,
    #[serde(skip_serializing_if = "Option::is_none")]
    completion_tokens_details: Option<CompletionTokenDetails>,
}

#[derive(Serialize, Deserialize, JsonSchema, Debug, Clone)]
pub struct PromptTokenDetails {
    cached_tokens: u32,
    audio_tokens: u32,
}

#[derive(Serialize, Deserialize, JsonSchema, Debug, Clone)]
pub struct CompletionTokenDetails {
    accepted_prediction_tokens: u32,
    audio_tokens: u32,
    reasoning_tokens: u32,
    rejected_prediction_tokens: u32,
}

#[derive(Serialize, Deserialize, JsonSchema, Debug, Clone, Default)]
pub struct Response {
    #[serde(skip_serializing_if = "Option::is_none")]
    id: Option<String>,
    choices: Vec<Choice>,
    #[serde(skip_serializing_if = "Option::is_none")]
    created: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    model: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    service_tier: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    system_fingerprint: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    object: Option<String>,
    usage: Usage,
}

impl Response {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn last_message(&self) -> Option<&ResponseMessage> {
        self.choices.last().map(|c| &c.message)
    }

    pub fn push_chunk(&mut self, chunk: ResponseChunk) {
        // Update optional fields if present in the chunk
        if let Some(created) = chunk.created {
            self.created = Some(created);
        }

        if let Some(model) = chunk.model {
            self.model = Some(model);
        }

        if let Some(service_tier) = chunk.service_tier {
            self.service_tier = Some(service_tier);
        }

        if let Some(system_fingerprint) = chunk.system_fingerprint {
            self.system_fingerprint = Some(system_fingerprint);
        }

        if let Some(usage) = chunk.usage {
            self.usage.completion_tokens += usage.completion_tokens;
            self.usage.prompt_tokens += usage.prompt_tokens;
            self.usage.total_tokens += usage.total_tokens;
        }

        for choice in chunk.choices {
            // Check if a choice with the same index already exists
            if let Some(existing_choice) = self.choices.iter_mut().find(|c| c.index == choice.index)
            {
                // Update existing choice fields if present
                if let Some(delta) = choice.delta {
                    existing_choice.message.apply_delta(delta);
                }
                if let Some(logprobs) = choice.logprobs {
                    existing_choice.logprobs = Some(logprobs);
                }
                if let Some(finish_reason) = choice.finish_reason {
                    existing_choice.finish_reason = finish_reason;
                }

                return;
            }

            // Create a new choice if it doesn't exist
            let mut message = ResponseMessage::default();
            if let Some(delta) = choice.delta {
                message.apply_delta(delta);
            }

            self.choices.push(Choice {
                finish_reason: choice.finish_reason.unwrap_or_default(),
                index: choice.index,
                logprobs: choice.logprobs,
                message,
            });
        }
    }
}
