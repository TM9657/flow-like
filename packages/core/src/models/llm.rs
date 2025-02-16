pub mod local;
pub mod openai;

use crate::{
    bit::{Bit, BitProvider},
    state::FlowLikeState,
    utils::device::get_vram,
};
use anyhow::Result;
use async_trait::async_trait;
use local::LocalModel;
use serde::{Deserialize, Serialize};
use std::{collections::HashMap, future::Future, pin::Pin, sync::Arc, time::SystemTime};
use tokio::sync::Mutex;
use tokio::time::{interval, Duration};

use super::{history::History, response::Response, response_chunk::ResponseChunk};

pub type LLMCallback = Arc<
    dyn Fn(ResponseChunk) -> Pin<Box<dyn Future<Output = Result<()>> + Send>>
        + Send
        + Sync
        + 'static,
>;

#[async_trait]
pub trait ModelLogic: Send + Sync {
    async fn invoke(&self, history: &History, lambda: Option<LLMCallback>) -> Result<Response>;
    fn get_bit(&self) -> Bit;
}

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

pub struct ModelFactory {
    pub cached_models: HashMap<String, Arc<LocalModel>>,
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
        let settings = self.execution_settings.clone();
        let provider = bit.try_to_provider();
        if provider.is_none() {
            return Err(anyhow::anyhow!("Model type not supported"));
        }

        let provider = provider.ok_or(anyhow::anyhow!("Model type not supported"))?;
        let provider = provider.provider_name;

        if provider == BitProvider::Local {
            if let Some(model) = self.cached_models.get(&bit.id) {
                // update last used time
                self.ttl_list.insert(bit.id.clone(), SystemTime::now());
                return Ok(model.clone());
            }

            let local_model = LocalModel::new(bit, app_state, &settings).await;
            let local_model = match local_model {
                Some(local_model) => local_model,
                None => return Err(anyhow::anyhow!("Model not found")),
            };
            let local_model: Arc<LocalModel> = Arc::new(local_model);
            self.ttl_list.insert(bit.id.clone(), SystemTime::now());
            self.cached_models
                .insert(bit.id.clone(), local_model.clone());
            return Ok(local_model);
        }

        Err(anyhow::anyhow!("Model type not supported"))
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

        // Lock the state, call gc(), and release the lock
        {
            let mut state = state.lock().await;
            state.gc();
        }
    }
}
