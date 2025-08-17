use flow_like::flow::node::NodeLogic;
use std::sync::Arc;

pub mod embed_image;

pub async fn register_functions() -> Vec<Arc<dyn NodeLogic>> {
    vec![
        Arc::new(embed_image::EmbedImageNode::default()),
    ]
}
