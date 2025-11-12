use std::{any::Any, sync::Arc};

use super::ModelLogic;
use crate::provider::random_provider;
use crate::{
    llm::ModelConstructor,
    provider::{ModelProvider, ModelProviderConfiguration},
};
use flow_like_types::{Cacheable, Result, async_trait};
use rig::client::ProviderClient;

pub struct PerplexityModel {
    client: Arc<Box<dyn ProviderClient>>,
    provider: ModelProvider,
    default_model: Option<String>,
}

impl PerplexityModel {
    pub async fn new(
        provider: &ModelProvider,
        config: &ModelProviderConfiguration,
    ) -> flow_like_types::Result<Self> {
        let perplexity_config = random_provider(&config.perplexity_config)?;
        let api_key = perplexity_config.api_key.clone().unwrap_or_default();
        let model_id = provider.model_id.clone();

        let mut builder = rig::providers::perplexity::Client::builder(&api_key);

        if let Some(endpoint) = perplexity_config.endpoint.as_deref() {
            builder = builder.base_url(endpoint);
        }

        let client = builder.build().boxed();

        Ok(PerplexityModel {
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

        let mut builder = rig::providers::perplexity::Client::builder(api_key);
        if let Some(endpoint) = params.get("endpoint").and_then(|v| v.as_str()) {
            builder = builder.base_url(endpoint);
        }

        let client = builder.build().boxed();

        Ok(PerplexityModel {
            client: Arc::new(client),
            default_model: model_id,
            provider: provider.clone(),
        })
    }
}

impl Cacheable for PerplexityModel {
    fn as_any(&self) -> &dyn Any {
        self
    }

    fn as_any_mut(&mut self) -> &mut dyn Any {
        self
    }
}

#[async_trait]
impl ModelLogic for PerplexityModel {
    async fn provider(&self) -> Result<ModelConstructor> {
        Ok(ModelConstructor {
            inner: self.client.clone(),
        })
    }

    async fn default_model(&self) -> Option<String> {
        self.default_model.clone()
    }
}
