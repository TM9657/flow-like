use std::{any::Any, sync::Arc};

use super::{LLMCallback, ModelLogic};
use crate::{
    embedding::{EmbeddingModelLogic, GeneralTextSplitter}, history::History, llm::ModelConstructor, provider::{ModelProvider, ModelProviderConfiguration}, response::Response
};
use flow_like_types::{Cacheable, Result, async_trait, sync::Mutex};
use rig::client::ProviderClient;
use text_splitter::{ChunkConfig, MarkdownSplitter, TextSplitter};
use crate::provider::random_provider;
pub struct HuggingfaceModel {
    client: Arc<Box<dyn ProviderClient>>,
    provider: ModelProvider,
    default_model: Option<String>
}

impl HuggingfaceModel {
    pub async fn new(
        provider: &ModelProvider,
        config: &ModelProviderConfiguration,
    ) -> flow_like_types::Result<Self> {
        let huggingface_config = random_provider(&config.huggingface_config)?;
        let api_key = huggingface_config.api_key.clone().unwrap_or_default();
        let model_id = provider.model_id.clone();

        let mut builder = rig::providers::huggingface::Client::builder(&api_key);

        if let Some(sub_provider) = huggingface_config.sub_provider.as_deref() {
            builder = builder.sub_provider(sub_provider);
        }

        if let Some(endpoint) = huggingface_config.endpoint.as_deref() {
                builder = builder.base_url(endpoint);
            }

        let client = builder.build()?.boxed();

        Ok(HuggingfaceModel {
            client: Arc::new(client),
            provider: provider.clone(),
            default_model: model_id,
        })
    }

    pub async fn from_provider(provider: &ModelProvider) -> flow_like_types::Result<Self> {
        let params = provider.params.clone().unwrap_or_default();
        let api_key = params.get("api_key").cloned().unwrap_or_default();
        let api_key = api_key.as_str().unwrap_or_default();
        let model_id = params.get("model_id").cloned().and_then(|v| v.as_str().map(|s| s.to_string()));

        let mut builder = rig::providers::huggingface::Client::builder(api_key);
        if let Some(endpoint) = params.get("endpoint").and_then(|v| v.as_str()) {
                builder = builder.base_url(endpoint);
            }

        if let Some(sub_provider) = params.get("sub_provider").and_then(|v| v.as_str()) {
            builder = builder.sub_provider(sub_provider);
        }

        let client = builder.build()?.boxed();

        Ok(HuggingfaceModel {
            client: Arc::new(client),
            default_model: model_id,
            provider: provider.clone(),
        })
    }
}

impl Cacheable for HuggingfaceModel {
    fn as_any(&self) -> &dyn Any {
        self
    }

    fn as_any_mut(&mut self) -> &mut dyn Any {
        self
    }
}

#[async_trait]
impl ModelLogic for HuggingfaceModel {
    async fn provider(&self) -> Result<ModelConstructor> {
        Ok(ModelConstructor {
            inner: self.client.clone()
        })
    }

    async fn default_model(&self) -> Option<String> {
        self.default_model.clone()
    }
}