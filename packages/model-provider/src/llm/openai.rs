use std::sync::Arc;

use super::{LLMCallback, ModelLogic};
use crate::{
    history::History,
    provider::{ModelProvider, ModelProviderConfiguration},
    response::Response,
};
use flow_like_types::{Result, async_trait, sync::Mutex};
use openai_api_rs::v1::chat_completion::ChatCompletionRequest;
mod client;
mod history;
mod response;

pub struct OpenAIModel {
    client: Arc<Mutex<client::OpenAIClient>>,
    provider: ModelProvider,
}

impl OpenAIModel {
    pub async fn new(
        provider: &ModelProvider,
        config: &ModelProviderConfiguration,
    ) -> flow_like_types::Result<Self> {
        let openai_config = config
            .openai_config
            .clone()
            .ok_or_else(|| flow_like_types::anyhow!("OpenAI configuration is missing"))?;
        let mut client = client::OpenAIClient::builder();

        if let Some(api_key) = &openai_config.api_key {
            client = client.with_api_key(api_key);
        }

        if let Some(organization_id) = &openai_config.organization {
            client = client.with_organization(organization_id.clone());
        }

        if let Some(endpoint) = &openai_config.endpoint {
            client = client.with_endpoint(endpoint.clone());
        }

        if let Some(proxy) = &openai_config.proxy {
            client = client.with_proxy(proxy.clone());
        }

        let client = client
            .build()
            .map_err(|e| flow_like_types::anyhow!("Failed to create OpenAI client: {}", e))?;

        Ok(OpenAIModel {
            client: Arc::new(Mutex::new(client)),
            provider: provider.clone(),
        })
    }
}

#[async_trait]
impl ModelLogic for OpenAIModel {
    async fn invoke(&self, history: &History, callback: Option<LLMCallback>) -> Result<Response> {
        let model_id = self
            .provider
            .model_id
            .clone()
            .ok_or_else(|| flow_like_types::anyhow!("Model ID is missing"))?;
        let mut request = ChatCompletionRequest::from(history.clone());
        request.model = model_id;

        let completion = {
            let mut client = self.client.lock().await;
            client.chat_completion(request, callback).await?
        };
        let response = Response::from(completion);
        Ok(response)
    }
}
