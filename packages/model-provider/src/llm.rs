use flow_like_types::Result;
use flow_like_types::async_trait;
use std::{future::Future, pin::Pin, sync::Arc};

use super::{history::History, response::Response, response_chunk::ResponseChunk};

pub mod bedrock;
pub mod openai;

pub type LLMCallback = Arc<
    dyn Fn(ResponseChunk) -> Pin<Box<dyn Future<Output = Result<()>> + Send>>
        + Send
        + Sync
        + 'static,
>;

#[async_trait]
pub trait ModelLogic: Send + Sync {
    async fn invoke(&self, history: &History, lambda: Option<LLMCallback>) -> Result<Response>;
}
