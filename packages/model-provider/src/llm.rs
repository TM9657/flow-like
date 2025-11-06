use flow_like_types::Result;
use flow_like_types::async_trait;
use flow_like_types::rand;
use flow_like_types::rand::Rng;
use rig::client::ProviderClient;
use std::{future::Future, pin::Pin, sync::Arc};

use super::{history::History, response::Response, response_chunk::ResponseChunk};

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
}

pub struct ModelConstructor {
    inner: Arc<Box<dyn ProviderClient>>,
}
