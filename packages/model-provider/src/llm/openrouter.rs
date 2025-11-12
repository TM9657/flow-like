use std::{any::Any, sync::Arc};

use super::{LLMCallback, ModelLogic};
use crate::provider::random_provider;
use crate::{
    embedding::{EmbeddingModelLogic, GeneralTextSplitter},
    history::History,
    llm::ModelConstructor,
    provider::{ModelProvider, ModelProviderConfiguration},
    response::Response,
};
use flow_like_types::{Cacheable, Result, async_trait, sync::Mutex};
use rig::client::ProviderClient;
use text_splitter::{ChunkConfig, MarkdownSplitter, TextSplitter};

pub struct OpenRouterModel {
    client: Arc<Box<dyn ProviderClient>>,
    provider: ModelProvider,
    default_model: Option<String>,
}

impl OpenRouterModel {
    pub async fn new(
        provider: &ModelProvider,
        config: &ModelProviderConfiguration,
    ) -> flow_like_types::Result<Self> {
        let openrouter_config = random_provider(&config.openrouter_config)?;
        let api_key = openrouter_config.api_key.clone().unwrap_or_default();
        let model_id = provider.model_id.clone();

        let mut builder = rig::providers::openrouter::Client::builder(&api_key);

        if let Some(endpoint) = openrouter_config.endpoint.as_deref() {
            builder = builder.base_url(endpoint);
        }

        let client = builder.build().boxed();

        Ok(OpenRouterModel {
            client: Arc::new(client),
            provider: provider.clone(),
            default_model: model_id,
        })
    }

    pub async fn from_provider(provider: &ModelProvider) -> flow_like_types::Result<Self> {
        let params = provider.params.clone().unwrap_or_default();
        let api_key = params.get("api_key").cloned().unwrap_or_default();
        let api_key = api_key.as_str().unwrap_or_default();
        let model_id = params
            .get("model_id")
            .cloned()
            .and_then(|v| v.as_str().map(|s| s.to_string()));

        let endpoint = params.get("endpoint").and_then(|v| v.as_str());

        let mut builder = rig::providers::openrouter::Client::builder(api_key);
        if let Some(endpoint) = endpoint {
            builder = builder.base_url(endpoint);
        }

        let client = builder.build().boxed();

        Ok(OpenRouterModel {
            client: Arc::new(client),
            default_model: model_id.clone(),
            provider: provider.clone(),
        })
    }
}

impl Cacheable for OpenRouterModel {
    fn as_any(&self) -> &dyn Any {
        self
    }

    fn as_any_mut(&mut self) -> &mut dyn Any {
        self
    }
}

#[async_trait]
impl ModelLogic for OpenRouterModel {
    async fn provider(&self) -> Result<ModelConstructor> {
        Ok(ModelConstructor {
            inner: self.client.clone(),
        })
    }

    async fn default_model(&self) -> Option<String> {
        self.default_model.clone()
    }
}
