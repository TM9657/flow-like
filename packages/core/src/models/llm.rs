pub mod local;

use crate::{bit::Bit, state::FlowLikeState, utils::device::get_vram};
use flow_like_model_provider::llm::{ModelLogic, openai::OpenAIModel};
use flow_like_types::{Result, sync::Mutex, tokio::time::interval};
use local::LocalModel;
use serde::{Deserialize, Serialize};
use std::{
    collections::HashMap,
    sync::Arc,
    time::{Duration, SystemTime},
};

#[derive(Serialize, Deserialize, Debug, Clone, Hash, PartialEq, Eq)]
pub struct ExecutionSettings {
    pub gpu_mode: bool,
    pub max_context_size: usize,
}

impl Default for ExecutionSettings {
    fn default() -> Self {
        ExecutionSettings::new()
    }
}

impl ExecutionSettings {
    pub fn new() -> Self {
        let vram = get_vram().unwrap_or(0);

        Self {
            gpu_mode: vram > 6_000_000_000,
            max_context_size: if vram > 6_000_000_000 { 32_000 } else { 8192 },
        }
    }
}

// TODO: implement DashMap
pub struct ModelFactory {
    pub cached_models: HashMap<String, Arc<dyn ModelLogic>>,
    pub ttl_list: HashMap<String, SystemTime>,
    pub execution_settings: ExecutionSettings,
}

impl Default for ModelFactory {
    fn default() -> Self {
        Self::new()
    }
}

impl ModelFactory {
    pub fn new() -> Self {
        Self {
            cached_models: HashMap::new(),
            ttl_list: HashMap::new(),
            execution_settings: ExecutionSettings::new(),
        }
    }

    pub fn set_execution_settings(&mut self, settings: ExecutionSettings) {
        self.execution_settings = settings;
    }

    pub async fn build(
        &mut self,
        bit: &Bit,
        app_state: Arc<Mutex<FlowLikeState>>,
    ) -> Result<Arc<dyn ModelLogic>> {
        let provider_config = app_state.lock().await.model_provider_config.clone();
        let settings = self.execution_settings.clone();
        let provider = bit.try_to_provider();
        if provider.is_none() {
            return Err(flow_like_types::anyhow!("Model type not supported"));
        }

        let model_provider =
            provider.ok_or(flow_like_types::anyhow!("Model type not supported"))?;
        let provider = model_provider.provider_name.clone();

        if provider == "Local" {
            if let Some(model) = self.cached_models.get(&bit.id) {
                self.ttl_list.insert(bit.id.clone(), SystemTime::now());
                return Ok(model.clone());
            }

            let local_model = LocalModel::new(bit, app_state, &settings).await;
            let local_model = match local_model {
                Ok(local_model) => local_model,
                Err(e) => return Err(e),
            };
            let local_model: Arc<LocalModel> = Arc::new(local_model);
            self.ttl_list.insert(bit.id.clone(), SystemTime::now());
            self.cached_models
                .insert(bit.id.clone(), local_model.clone());
            return Ok(local_model);
        }

        if provider == "azure" || provider == "openai" {
            if let Some(model) = self.cached_models.get(&bit.id) {
                self.ttl_list.insert(bit.id.clone(), SystemTime::now());
                return Ok(model.clone());
            }

            let model = OpenAIModel::new(&model_provider, &provider_config).await?;

            let model = Arc::new(model);
            self.ttl_list.insert(bit.id.clone(), SystemTime::now());
            self.cached_models.insert(bit.id.clone(), model.clone());
            return Ok(model);
        }

        Err(flow_like_types::anyhow!("Model type not supported"))
    }

    pub fn gc(&mut self) {
        let mut to_remove = Vec::new();
        for id in self.cached_models.keys() {
            // check if the model was not used for 5 minutes
            let ttl = self.ttl_list.get(id).unwrap();
            if ttl.elapsed().unwrap().as_secs() > 300 {
                to_remove.push(id.clone());
            }
        }

        for id in to_remove {
            self.cached_models.remove(&id);
            self.ttl_list.remove(&id);
        }
    }
}

pub async fn start_gc(state: Arc<Mutex<ModelFactory>>) {
    let mut interval = interval(Duration::from_secs(1));

    loop {
        interval.tick().await;

        {
            let state = state.try_lock();
            if let Ok(mut state) = state {
                state.gc();
            }
        }
    }
}
