use flow_like_types::async_trait;
use flow_like_types::{Result, anyhow};
use futures::StreamExt;
use rig::client::ProviderClient;
pub use rig::client::completion::CompletionModelHandle;
use rig::completion::{CompletionModel, CompletionRequestBuilder, Message, Usage as RigUsage};
use rig::streaming::StreamedAssistantContent;
use std::{future::Future, pin::Pin, sync::Arc};

use super::{
    history::History,
    response::{Response, Usage as ResponseUsage},
    response_chunk::ResponseChunk,
};

// pub mod bedrock;
pub mod anthropic;
pub mod cohere;
pub mod deepseek;
pub mod galadriel;
pub mod gemini;
pub mod groq;
pub mod huggingface;
pub mod hyperbolic;
pub mod llamacpp;
pub mod mira;
pub mod mistral;
pub mod moonshot;
pub mod ollama;
pub mod openai;
pub mod openrouter;
pub mod perplexity;
pub mod together;
pub mod voyageai;
pub mod xai;

pub type LLMCallback = Arc<
    dyn Fn(ResponseChunk) -> Pin<Box<dyn Future<Output = Result<()>> + Send>>
        + Send
        + Sync
        + 'static,
>;

#[async_trait]
pub trait ModelLogic: Send + Sync {
    async fn provider(&self) -> Result<ModelConstructor>;
    async fn default_model(&self) -> Option<String>;
    fn additional_params(&self, _history: &Option<History>) -> Option<flow_like_types::Value> {
        None
    }

    fn transform_history(&self, _history: &mut History) {}

    /// Get the underlying rig CompletionModelHandle for use with external libraries
    async fn completion_model_handle(
        &self,
        model_name: Option<&str>,
    ) -> Result<CompletionModelHandle<'static>> {
        let default = self.default_model().await;
        let model_name = model_name
            .map(|s| s.to_string())
            .or(default)
            .ok_or_else(|| anyhow!("No model name provided and no default model available"))?;

        let constructor = self.provider().await?;
        let client = constructor.client();
        let completion_client = client
            .as_ref()
            .as_completion()
            .ok_or_else(|| anyhow!("Provider does not support completion"))?;

        let completion_model = completion_client.completion_model(&model_name);
        Ok(CompletionModelHandle {
            inner: Arc::from(completion_model),
        })
    }

    async fn invoke(&self, history: &History, lambda: Option<LLMCallback>) -> Result<Response> {
        let model_name = self
            .default_model()
            .await
            .unwrap_or_else(|| history.model.clone());

        let constructor = self.provider().await?;
        let client = constructor.client();
        let completion_client = client
            .as_ref()
            .as_completion()
            .ok_or_else(|| anyhow!("Provider does not support completion"))?;

        let completion_model = completion_client.completion_model(&model_name);
        let completion_handle = CompletionModelHandle {
            inner: Arc::from(completion_model),
        };

        let (prompt, chat_history) = history
            .extract_prompt_and_history()
            .map_err(|e| anyhow!("Failed to convert history into rig messages: {e}"))?;

        let mut builder = completion_handle
            .completion_request(prompt)
            .messages(chat_history);

        if let Some(temp) = history.temperature {
            builder = builder.temperature(temp as f64);
        }

        if let Some(max_tokens) = history.max_completion_tokens {
            builder = builder.max_tokens(max_tokens as u64);
        }

        if history.tools.is_some() {
            let tool_definitions = history.tools_to_rig()?;
            if !tool_definitions.is_empty() {
                builder = builder.tools(tool_definitions);
            }
        }

        if let Some(choice) = history.tool_choice_to_rig() {
            builder = builder.tool_choice(choice);
        }

        // Note: We call self.additional_params() later which may need to merge with history params
        // Some providers (like Gemini) need to filter certain fields from history params
        // So we let the model implementation handle the merging in additional_params()
        // Only add history params here if the model doesn't provide custom params
        let model_additional_params = self.additional_params(&Some(history.clone()));

        if model_additional_params.is_none() {
            // Model doesn't provide custom params, use history params directly
            if let Some(params) = history.build_additional_params()? {
                builder = builder.additional_params(params);
            }
        }

        if let Some(callback) = lambda {
            invoke_with_stream(builder, callback, &model_name, model_additional_params).await
        } else {
            invoke_without_stream(builder, &model_name, model_additional_params).await
        }
    }
}

pub struct ModelConstructor {
    pub inner: Arc<Box<dyn ProviderClient>>,
}

impl ModelConstructor {
    pub fn into(self) -> Arc<Box<dyn ProviderClient>> {
        self.inner.clone()
    }

    pub fn client(&self) -> Arc<Box<dyn ProviderClient>> {
        self.inner.clone()
    }
}

async fn invoke_without_stream<'a>(
    builder: CompletionRequestBuilder<CompletionModelHandle<'a>>,
    model_name: &str,
    additional_params: Option<flow_like_types::Value>,
) -> Result<Response> {
    let builder = if let Some(params) = additional_params {
        builder.additional_params(params)
    } else {
        builder
    };

    let completion = builder
        .send()
        .await
        .map_err(|e| anyhow!("Rig completion error: {e}"))?;

    let message = Message::Assistant {
        id: None,
        content: completion.choice.clone(),
    };

    let mut response = Response::from_rig_message(message)?;
    response.model = Some(model_name.to_string());
    response.usage = ResponseUsage::from_rig(completion.usage);
    Ok(response)
}

async fn invoke_with_stream<'a>(
    builder: CompletionRequestBuilder<CompletionModelHandle<'a>>,
    callback: LLMCallback,
    model_name: &str,
    additional_params: Option<flow_like_types::Value>,
) -> Result<Response> {
    let builder = if let Some(params) = additional_params {
        builder.additional_params(params)
    } else {
        builder
    };

    let mut stream = builder.stream().await.map_err(|e| {
        // Extract more detailed error information
        let error_msg = format!("{:?}", e);
        anyhow!("Rig streaming error: {} | Details: {}", e, error_msg)
    })?;

    let mut response = Response::new();
    response.model = Some(model_name.to_string());

    let mut final_usage: Option<RigUsage> = None;

    while let Some(item) = stream.next().await {
        let content = item.map_err(|e| {
            let error_msg = format!("{:?}", e);
            anyhow!("Rig streaming error: {} | Details: {}", e, error_msg)
        })?;
        match content {
            StreamedAssistantContent::Text(text) => {
                let chunk = ResponseChunk::from_text(&text.text, model_name);
                response.push_chunk(chunk.clone());
                callback(chunk).await?;
            }
            StreamedAssistantContent::ToolCall(tool_call) => {
                let chunk = ResponseChunk::from_tool_call(&tool_call, model_name);
                response.push_chunk(chunk.clone());
                callback(chunk).await?;
            }
            StreamedAssistantContent::ToolCallDelta { id, delta } => {
                let chunk = ResponseChunk::from_tool_call_delta(&id, &delta, model_name);
                response.push_chunk(chunk.clone());
                callback(chunk).await?;
            }
            StreamedAssistantContent::Reasoning(reasoning) => {
                // Join the reasoning vec into a single string
                let reasoning_text = reasoning.reasoning.join("\n");
                let chunk = ResponseChunk::from_reasoning(&reasoning_text, model_name);
                response.push_chunk(chunk.clone());
                callback(chunk).await?;
            }
            StreamedAssistantContent::Final(final_resp) => {
                final_usage = final_resp.usage;
            }
        }
    }

    let finish_chunk = ResponseChunk::finish(model_name, final_usage.as_ref());
    response.push_chunk(finish_chunk.clone());
    callback(finish_chunk).await?;

    if let Some(usage) = final_usage {
        response.usage = ResponseUsage::from_rig(usage);
    }

    Ok(response)
}
