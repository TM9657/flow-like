use flow_like_types::Value;
use flow_like_types::json::{self as serde_json, json};
use flow_like_types::reqwest;
use rig::{
    OneOrMany,
    client::{
        ClientBuilderError, CompletionClient as RigCompletionClient, ProviderClient, ProviderValue,
    },
    completion::{self, CompletionError, CompletionRequest, GetTokenUsage, Usage},
    impl_conversion_traits,
    message::{self},
    streaming,
};
use serde::{Deserialize, Serialize};
use std::convert::TryFrom;

#[derive(Clone, Debug)]
pub struct LlamaCppClient {
    base_url: String,
    http_client: reqwest::Client,
}

impl LlamaCppClient {
    pub fn new(base_url: &str) -> Self {
        Self {
            base_url: base_url.to_string(),
            http_client: reqwest::Client::new(),
        }
    }

    fn post(&self, path: &str) -> Result<reqwest::RequestBuilder, ClientBuilderError> {
        let url = format!("{}/{}", self.base_url, path);
        Ok(self.http_client.post(url))
    }
}

impl ProviderClient for LlamaCppClient {
    fn from_env() -> Self {
        Self::new("http://localhost:8080")
    }

    fn from_val(_val: ProviderValue) -> Self {
        Self::new("http://localhost:8080")
    }
}

impl RigCompletionClient for LlamaCppClient {
    type CompletionModel = CompletionModel;

    fn completion_model(&self, model: &str) -> CompletionModel {
        CompletionModel::new(self.clone(), model)
    }
}

impl_conversion_traits!(
    AsTranscription,
    AsImageGeneration,
    AsAudioGeneration,
    AsEmbeddings for LlamaCppClient
);

#[derive(Debug, Serialize, Deserialize)]
pub struct CompletionResponse {
    pub id: String,
    pub object: String,
    pub created: u64,
    pub model: String,
    pub choices: Vec<Choice>,
    pub usage: ApiUsage,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Choice {
    pub index: u32,
    pub message: ResponseMessage,
    pub finish_reason: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ResponseMessage {
    pub role: String,
    pub content: Option<String>,
    #[serde(default)]
    pub tool_calls: Vec<ToolCall>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ToolCall {
    pub id: String,
    pub r#type: String,
    pub function: FunctionCall,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct FunctionCall {
    pub name: String,
    pub arguments: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ApiUsage {
    pub prompt_tokens: u64,
    pub completion_tokens: u64,
    pub total_tokens: u64,
}

impl TryFrom<CompletionResponse> for completion::CompletionResponse<CompletionResponse> {
    type Error = CompletionError;

    fn try_from(resp: CompletionResponse) -> Result<Self, Self::Error> {
        let first_choice = resp
            .choices
            .first()
            .ok_or_else(|| CompletionError::ResponseError("No choices in response".to_string()))?;

        let mut assistant_contents = Vec::new();

        if let Some(content) = &first_choice.message.content {
            if !content.is_empty() {
                assistant_contents.push(completion::AssistantContent::text(content));
            }
        }

        for tc in &first_choice.message.tool_calls {
            let args_value: Value =
                serde_json::from_str(&tc.function.arguments).unwrap_or_else(|_| json!({}));
            assistant_contents.push(completion::AssistantContent::tool_call(
                tc.id.clone(),
                tc.function.name.clone(),
                args_value,
            ));
        }

        let choice = OneOrMany::many(assistant_contents)
            .map_err(|_| CompletionError::ResponseError("No content provided".to_owned()))?;

        Ok(completion::CompletionResponse {
            choice,
            usage: Usage {
                input_tokens: resp.usage.prompt_tokens,
                output_tokens: resp.usage.completion_tokens,
                total_tokens: resp.usage.total_tokens,
            },
            raw_response: resp,
        })
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct StreamingFunction {
    #[serde(default)]
    pub name: Option<String>,
    #[serde(default)]
    pub arguments: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct StreamingToolCall {
    pub index: usize,
    pub id: Option<String>,
    pub function: StreamingFunction,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct StreamingDelta {
    #[serde(default)]
    pub content: Option<String>,
    #[serde(default)]
    pub tool_calls: Vec<StreamingToolCall>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct StreamingChoice {
    pub delta: StreamingDelta,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct StreamingChunk {
    pub choices: Vec<StreamingChoice>,
    pub usage: Option<ApiUsage>,
}

#[derive(Clone)]
pub struct CompletionModel {
    client: LlamaCppClient,
    pub model: String,
}

impl CompletionModel {
    pub fn new(client: LlamaCppClient, model: &str) -> Self {
        Self {
            client,
            model: model.to_owned(),
        }
    }

    fn create_completion_request(
        &self,
        completion_request: CompletionRequest,
    ) -> Result<Value, CompletionError> {
        let mut messages = Vec::new();

        if let Some(preamble) = &completion_request.preamble {
            messages.push(json!({
                "role": "system",
                "content": preamble,
            }));
        }

        if !completion_request.documents.is_empty() {
            let doc_content = completion_request
                .documents
                .iter()
                .map(|d| d.text.clone())
                .collect::<Vec<_>>()
                .join("\n\n");
            messages.push(json!({
                "role": "system",
                "content": format!("Context documents:\n{}", doc_content),
            }));
        }

        for msg in completion_request.chat_history.iter() {
            let converted = self.convert_message(msg.clone())?;
            if let Some(msgs) = converted.as_array() {
                messages.extend(msgs.iter().cloned());
            } else {
                messages.push(converted);
            }
        }

        let temperature = completion_request.temperature.unwrap_or(0.7);

        let mut request_payload = json!({
            "model": self.model,
            "messages": messages,
            "temperature": temperature,
            "stream": false,
        });

        if let Some(max_tokens) = completion_request.max_tokens {
            request_payload["max_tokens"] = json!(max_tokens);
        }

        if !completion_request.tools.is_empty() {
            request_payload["tools"] = json!(
                completion_request
                    .tools
                    .into_iter()
                    .map(|tool| json!({
                        "type": "function",
                        "function": {
                            "name": tool.name,
                            "description": tool.description,
                            "parameters": tool.parameters,
                        }
                    }))
                    .collect::<Vec<_>>()
            );
        }

        if let Some(extra) = completion_request.additional_params {
            if let Some(obj) = request_payload.as_object_mut() {
                if let Some(extra_obj) = extra.as_object() {
                    for (k, v) in extra_obj {
                        obj.insert(k.clone(), v.clone());
                    }
                }
            }
        }

        Ok(request_payload)
    }

    fn convert_message(&self, msg: message::Message) -> Result<Value, CompletionError> {
        match msg {
            message::Message::User { content, .. } => {
                let mut text_parts = Vec::new();
                let mut tool_results = Vec::new();

                for c in content.iter() {
                    match c {
                        message::UserContent::Text(t) => {
                            text_parts.push(t.text.clone());
                        }
                        message::UserContent::ToolResult(tr) => {
                            let mut result_texts = Vec::new();
                            for content_item in tr.content.iter() {
                                match content_item {
                                    message::ToolResultContent::Text(t) => {
                                        result_texts.push(t.text.clone());
                                    }
                                    _ => {}
                                }
                            }
                            let result_text = result_texts.join(" ");

                            tool_results.push(json!({
                                "role": "tool",
                                "tool_call_id": tr.id,
                                "content": result_text
                            }));
                        }
                        _ => {}
                    }
                }

                let text = text_parts.join(" ");

                if !tool_results.is_empty() && text.is_empty() {
                    return Ok(json!(tool_results));
                }

                if !tool_results.is_empty() {
                    let mut result = tool_results;
                    result.push(json!({
                        "role": "user",
                        "content": text,
                    }));
                    return Ok(json!(result));
                }

                let content_text = if text.is_empty() {
                    "[No text content]".to_string()
                } else {
                    text
                };

                Ok(json!({
                    "role": "user",
                    "content": content_text,
                }))
            }
            message::Message::Assistant { content, .. } => {
                let mut text_parts = Vec::new();
                let mut tool_calls = Vec::new();

                for c in content.iter() {
                    match c {
                        completion::AssistantContent::Text(t) => {
                            text_parts.push(t.text.clone());
                        }
                        completion::AssistantContent::ToolCall(tc) => {
                            tool_calls.push(json!({
                                "id": tc.id,
                                "type": "function",
                                "function": {
                                    "name": tc.function.name,
                                    "arguments": serde_json::to_string(&tc.function.arguments).unwrap_or_default()
                                }
                            }));
                        }
                        _ => {}
                    }
                }

                let text = text_parts.join(" ");

                let mut message = json!({
                    "role": "assistant",
                });

                message["content"] = if text.is_empty() {
                    json!(null)
                } else {
                    json!(text)
                };

                if !tool_calls.is_empty() {
                    message["tool_calls"] = json!(tool_calls);
                }

                Ok(message)
            }
        }
    }
}

#[derive(Clone, Serialize, Deserialize, Debug)]
pub struct StreamingCompletionResponse {
    pub prompt_tokens: u64,
    pub completion_tokens: u64,
    pub total_tokens: u64,
}

impl GetTokenUsage for StreamingCompletionResponse {
    fn token_usage(&self) -> Option<Usage> {
        Some(Usage {
            input_tokens: self.prompt_tokens,
            output_tokens: self.completion_tokens,
            total_tokens: self.total_tokens,
        })
    }
}

impl completion::CompletionModel for CompletionModel {
    type Response = CompletionResponse;
    type StreamingResponse = StreamingCompletionResponse;

    async fn completion(
        &self,
        completion_request: CompletionRequest,
    ) -> Result<completion::CompletionResponse<Self::Response>, CompletionError> {
        let request = self.create_completion_request(completion_request)?;

        let response = self
            .client
            .post("v1/chat/completions")
            .map_err(|e| CompletionError::ProviderError(e.to_string()))?
            .json(&request)
            .send()
            .await
            .map_err(|e| CompletionError::ProviderError(e.to_string()))?;

        if !response.status().is_success() {
            return Err(CompletionError::ProviderError(
                response.text().await.unwrap_or_default(),
            ));
        }

        let bytes = response.bytes().await.map_err(|e| {
            CompletionError::ProviderError(format!("Failed to read response: {}", e))
        })?;

        let response_data: CompletionResponse = serde_json::from_slice(&bytes)
            .map_err(|e| CompletionError::ResponseError(e.to_string()))?;

        response_data.try_into()
    }

    async fn stream(
        &self,
        completion_request: CompletionRequest,
    ) -> Result<streaming::StreamingCompletionResponse<Self::StreamingResponse>, CompletionError>
    {
        use flow_like_types::async_stream::stream;
        use flow_like_types::futures::StreamExt;
        use flow_like_types::reqwest_eventsource::{Event, RequestBuilderExt};
        use std::collections::HashMap;

        let mut request = self.create_completion_request(completion_request)?;
        request["stream"] = json!(true);

        let builder = self
            .client
            .post("v1/chat/completions")
            .map_err(|e| CompletionError::ProviderError(e.to_string()))?
            .json(&request);

        let mut event_source = builder.eventsource().map_err(|e| {
            CompletionError::ProviderError(format!("Failed to create event source: {}", e))
        })?;

        let stream = Box::pin(stream! {
            let mut tool_calls: HashMap<usize, (String, String, String)> = HashMap::new();
            let mut final_usage: Option<ApiUsage> = None;

            while let Some(event_result) = event_source.next().await {
                match event_result {
                    Ok(Event::Open) => {
                        continue;
                    }
                    Ok(Event::Message(message)) => {
                        if message.data.trim().is_empty() || message.data == "[DONE]" {
                            continue;
                        }

                        let chunk: Result<StreamingChunk, _> = serde_json::from_str(&message.data);
                        let Ok(chunk) = chunk else {
                            continue;
                        };

                        if let Some(choice) = chunk.choices.first() {
                            let delta = &choice.delta;

                            if let Some(content) = &delta.content {
                                if !content.is_empty() {
                                    yield Ok(streaming::RawStreamingChoice::Message(content.clone()));
                                }
                            }

                            if !delta.tool_calls.is_empty() {
                                for tool_call in &delta.tool_calls {
                                    let function = &tool_call.function;

                                    if function.name.is_some() && function.arguments.is_empty() {
                                        let id = tool_call.id.clone().unwrap_or_default();
                                        tool_calls.insert(
                                            tool_call.index,
                                            (id, function.name.clone().unwrap(), String::new()),
                                        );
                                    }
                                    else if function.name.is_none() && !function.arguments.is_empty() {
                                        if let Some((id, name, args)) = tool_calls.get(&tool_call.index) {
                                            let new_args = format!("{}{}", args, &function.arguments);
                                            tool_calls.insert(
                                                tool_call.index,
                                                (id.clone(), name.clone(), new_args),
                                            );
                                        }
                                    }
                                }
                            }
                        }

                        if let Some(usage) = chunk.usage {
                            final_usage = Some(usage);
                        }
                    }
                    Err(e) => {
                        let error_str = e.to_string();
                        if error_str.contains("Stream ended") {
                            break;
                        }

                        yield Err(CompletionError::ProviderError(format!("Stream error: {}", e)));
                        break;
                    }
                }
            }

            for (_, (id, name, args)) in tool_calls {
                if let Ok(arguments) = serde_json::from_str(&args) {
                    yield Ok(streaming::RawStreamingChoice::ToolCall {
                        id,
                        call_id: None,
                        name,
                        arguments,
                    });
                }
            }

            if let Some(usage) = final_usage {
                yield Ok(streaming::RawStreamingChoice::FinalResponse(
                    StreamingCompletionResponse {
                        prompt_tokens: usage.prompt_tokens,
                        completion_tokens: usage.completion_tokens,
                        total_tokens: usage.total_tokens,
                    }
                ));
            }

            event_source.close();
        });

        Ok(streaming::StreamingCompletionResponse::stream(stream))
    }
}
