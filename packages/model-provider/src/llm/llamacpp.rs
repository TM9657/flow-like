use std::{any::Any, sync::Arc};

use super::{ModelConstructor, ModelLogic};
use crate::provider::ModelProvider;
use flow_like_types::{Cacheable, Result, async_trait};
use rig::client::ProviderClient;

mod client;
pub use client::LlamaCppClient;

pub struct LlamaCppModel {
    client: Arc<Box<dyn ProviderClient>>,
    #[allow(dead_code)]
    provider: ModelProvider,
    default_model: Option<String>,
    port: u16,
}

impl LlamaCppModel {
    pub async fn new(provider: &ModelProvider, port: u16) -> flow_like_types::Result<Self> {
        let model_id = provider.model_id.clone();
        let base_url = format!("http://localhost:{}", port);

        let client = Box::new(LlamaCppClient::new(&base_url));

        Ok(LlamaCppModel {
            client: Arc::new(client),
            provider: provider.clone(),
            default_model: model_id,
            port,
        })
    }

    pub fn port(&self) -> u16 {
        self.port
    }
}

impl Cacheable for LlamaCppModel {
    fn as_any(&self) -> &dyn Any {
        self
    }

    fn as_any_mut(&mut self) -> &mut dyn Any {
        self
    }
}

#[async_trait]
impl ModelLogic for LlamaCppModel {
    async fn provider(&self) -> Result<ModelConstructor> {
        Ok(ModelConstructor {
            inner: self.client.clone(),
        })
    }

    async fn default_model(&self) -> Option<String> {
        self.default_model.clone()
    }
}
