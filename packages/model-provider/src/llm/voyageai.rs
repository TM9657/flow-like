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

pub struct VoyageAIModel {
    client: Arc<Box<dyn ProviderClient>>,
    provider: ModelProvider,
    default_model: Option<String>,
}

impl VoyageAIModel {
    pub async fn new(
        provider: &ModelProvider,
        config: &ModelProviderConfiguration,
    ) -> flow_like_types::Result<Self> {
        let voyageai_config = random_provider(&config.voyageai_config)?;
        let api_key = voyageai_config.api_key.clone().unwrap_or_default();
        let model_id = provider.model_id.clone();

        let mut builder = rig::providers::openai::Client::builder(&api_key);

        if let Some(endpoint) = voyageai_config.endpoint.as_deref() {
            builder = builder.base_url(endpoint);
        }

        let client = builder.build().boxed();

        Ok(VoyageAIModel {
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

        let mut builder = rig::providers::openai::Client::builder(api_key);
        if let Some(endpoint) = params.get("endpoint").and_then(|v| v.as_str()) {
            builder = builder.base_url(endpoint);
        }

        let client = builder.build().boxed();

        Ok(VoyageAIModel {
            client: Arc::new(client),
            default_model: model_id,
            provider: provider.clone(),
        })
    }
}

impl Cacheable for VoyageAIModel {
    fn as_any(&self) -> &dyn Any {
        self
    }

    fn as_any_mut(&mut self) -> &mut dyn Any {
        self
    }
}

#[async_trait]
impl ModelLogic for VoyageAIModel {
    async fn provider(&self) -> Result<ModelConstructor> {
        Ok(ModelConstructor {
            inner: self.client.clone(),
        })
    }

    async fn default_model(&self) -> Option<String> {
        self.default_model.clone()
    }
}
