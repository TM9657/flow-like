use openai_api_rs::v1::chat_completion::{ChatCompletionResponse, FinishReason, MessageRole};

use crate::response::{Choice, FunctionCall, Response, ResponseFunction, ResponseMessage, Usage};

impl From<ChatCompletionResponse> for Response {
    fn from(chat_completion_response: ChatCompletionResponse) -> Self {
        let choices = chat_completion_response
            .choices
            .into_iter()
            .map(|chat_choice| {
                let index = chat_choice.index as i32;

                let finish_reason = chat_choice
                    .finish_reason
                    .map(|fr| match fr {
                        FinishReason::stop => "stop".to_string(),
                        FinishReason::length => "length".to_string(),
                        FinishReason::content_filter => "content_filter".to_string(),
                        FinishReason::tool_calls => "tool_calls".to_string(),
                        FinishReason::null => "null".to_string(),
                    })
                    .unwrap_or_default();

                let message = {
                    let role = match chat_choice.message.role {
                        MessageRole::user => "user".to_string(),
                        MessageRole::system => "system".to_string(),
                        MessageRole::assistant => "assistant".to_string(),
                        MessageRole::function => "function".to_string(),
                        MessageRole::tool => "tool".to_string(),
                    };
                    let content = chat_choice.message.content;
                    let refusal = None; // No direct mapping; set to None

                    let tool_calls = chat_choice
                        .message
                        .tool_calls
                        .unwrap_or_default()
                        .into_iter()
                        .enumerate()
                        .map(|(i, tool_call)| FunctionCall {
                            index: Some(i as i32), // Assign index based on position
                            id: tool_call.id,
                            tool_type: Some(tool_call.r#type), // Map type to tool_type
                            function: ResponseFunction {
                                name: tool_call.function.name.unwrap_or("".to_string()),
                                arguments: tool_call.function.arguments.unwrap_or("".to_string()),
                            },
                        })
                        .collect();

                    ResponseMessage {
                        annotations: None,
                        audio: None,
                        role,
                        content,
                        refusal,
                        tool_calls,
                    }
                };

                Choice {
                    index,
                    finish_reason,
                    message,
                    logprobs: None,
                }
            })
            .collect();

        Response {
            id: chat_completion_response.id,
            choices,
            created: Some(chat_completion_response.created as u64),
            model: Some(chat_completion_response.model),
            service_tier: None,
            system_fingerprint: chat_completion_response.system_fingerprint,
            object: Some(chat_completion_response.object),
            usage: Usage {
                prompt_tokens: chat_completion_response.usage.prompt_tokens as u32,
                completion_tokens: chat_completion_response.usage.completion_tokens as u32,
                total_tokens: chat_completion_response.usage.total_tokens as u32,
                prompt_tokens_details: None,
                completion_tokens_details: None,
            },
        }
    }
}
