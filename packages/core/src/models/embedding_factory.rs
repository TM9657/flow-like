use std::{collections::HashMap, sync::Arc, time::SystemTime};

use flow_like_model_provider::{
    embedding::{openai::OpenAIEmbeddingModel, EmbeddingModelLogic}, image_embedding::ImageEmbeddingModelLogic,
};
use flow_like_types::sync::Mutex;

use crate::{bit::Bit, state::FlowLikeState};

use super::{
    embedding::local::LocalEmbeddingModel, image_embedding::local::LocalImageEmbeddingModel,
};

pub struct EmbeddingFactory {
    pub cached_text_models: HashMap<String, Arc<dyn EmbeddingModelLogic>>,
    pub cached_image_models: HashMap<String, Arc<dyn ImageEmbeddingModelLogic>>,
    pub ttl_list: HashMap<String, SystemTime>,
}

impl Default for EmbeddingFactory {
    fn default() -> Self {
        Self::new()
    }
}

impl EmbeddingFactory {
    pub fn new() -> Self {
        Self {
            cached_text_models: HashMap::new(),
            cached_image_models: HashMap::new(),
            ttl_list: HashMap::new(),
        }
    }

    pub async fn build_text(
        &mut self,
        bit: &Bit,
        app_state: Arc<Mutex<FlowLikeState>>,
    ) -> flow_like_types::Result<Arc<dyn EmbeddingModelLogic>> {
        let provider_config = app_state.lock().await.model_provider_config.clone();

        let provider = bit.try_to_embedding_provider().ok_or(
            flow_like_types::anyhow!("Model type not supported"),
        )?;
        let embedding_provider = bit.try_to_embedding().ok_or(
            flow_like_types::anyhow!("Model type not supported"),
        )?;
        let provider_name = provider.provider_name;

        if provider_name == "Local" {
            if let Some(model) = self.cached_text_models.get(&bit.id) {
                // update last used time
                self.ttl_list.insert(bit.id.clone(), SystemTime::now());
                return Ok(model.clone());
            }

            let local_model = LocalEmbeddingModel::new(bit, app_state).await?;
            self.ttl_list.insert(bit.id.clone(), SystemTime::now());
            self.cached_text_models
                .insert(bit.id.clone(), local_model.clone());
            return Ok(local_model);
        }

        if provider_name == "openai" || provider_name == "azure" {
            let local_model = OpenAIEmbeddingModel::new(&embedding_provider, &provider_config).await?;
            return Ok(Arc::new(local_model));
        }

        Err(flow_like_types::anyhow!("Model type not supported"))
    }

    pub async fn build_image(
        &mut self,
        bit: &Bit,
        app_state: Arc<Mutex<FlowLikeState>>,
    ) -> flow_like_types::Result<Arc<dyn ImageEmbeddingModelLogic>> {
        let provider = bit.try_to_provider();
        if provider.is_none() {
            return Err(flow_like_types::anyhow!("Model type not supported"));
        }

        let provider = provider.ok_or(flow_like_types::anyhow!("Model type not supported"))?;
        let provider = provider.provider_name;

        if provider == "Local" {
            if let Some(model) = self.cached_image_models.get(&bit.id) {
                self.ttl_list.insert(bit.id.clone(), SystemTime::now());
                return Ok(model.clone());
            }

            let local_model = LocalImageEmbeddingModel::new(bit, app_state, self).await?;
            self.ttl_list.insert(bit.id.clone(), SystemTime::now());
            self.cached_image_models
                .insert(bit.id.clone(), local_model.clone());
            return Ok(local_model);
        }

        Err(flow_like_types::anyhow!("Model type not supported"))
    }

    pub fn gc(&mut self) {
        let mut to_remove = Vec::new();
        for id in self.cached_image_models.keys() {
            // check if the model was not used for 5 minutes
            let ttl = self.ttl_list.get(id).unwrap();
            if ttl.elapsed().unwrap().as_secs() > 300 {
                to_remove.push(id.clone());
            }
        }

        for id in self.cached_text_models.keys() {
            // check if the model was not used for 5 minutes
            let ttl = self.ttl_list.get(id).unwrap();
            if ttl.elapsed().unwrap().as_secs() > 300 {
                to_remove.push(id.clone());
            }
        }

        for id in to_remove {
            self.cached_text_models.remove(&id);
            self.cached_image_models.remove(&id);
            self.ttl_list.remove(&id);
        }
    }
}
