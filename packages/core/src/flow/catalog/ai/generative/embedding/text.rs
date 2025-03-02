pub mod embed_text_document;
pub mod embed_text_query;
pub mod embed_texts_document;
pub mod embed_texts_query;

use crate::flow::node::NodeLogic;
use std::sync::Arc;
use tokio::sync::Mutex;

pub async fn register_functions() -> Vec<Arc<Mutex<dyn NodeLogic>>> {
    vec![
        Arc::new(Mutex::new(embed_text_document::EmbedDocumentNode::default())),
        Arc::new(Mutex::new(embed_text_query::EmbedQueryNode::default())),
    ]
}
