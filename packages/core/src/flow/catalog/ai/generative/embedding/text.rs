pub mod chunk_text;
pub mod chunk_text_char;
pub mod embed_text_document;
pub mod embed_text_query;
pub mod embed_texts_document;
pub mod embed_texts_query;

use crate::flow::node::NodeLogic;
use std::sync::Arc;

pub async fn register_functions() -> Vec<Arc<dyn NodeLogic>> {
    vec![
        Arc::new(embed_text_document::EmbedDocumentNode::default()),
        Arc::new(embed_text_query::EmbedQueryNode::default()),
        Arc::new(chunk_text::ChunkText::default()),
        Arc::new(chunk_text_char::ChunkTextChar::default()),
    ]
}
