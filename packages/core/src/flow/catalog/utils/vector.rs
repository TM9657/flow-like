use crate::flow::node::NodeLogic;
use std::sync::Arc;
use tokio::sync::Mutex;

pub mod addition;
pub mod cosine_sim;
pub mod cross_product;
pub mod dot_product;
pub mod multiplication;
pub mod normalize;
pub mod subtraction;

pub async fn register_functions() -> Vec<Arc<dyn NodeLogic>> {
    vec![
        Arc::new(addition::FloatVectorAdditionNode::default()),
        Arc::new(cross_product::FloatVectorCrossProductNode::default()),
        Arc::new(dot_product::FloatVectorDotProductNode::default()),
        Arc::new(multiplication::FloatVectorMultiplicationNode::default()),
        Arc::new(subtraction::FloatVectorSubtractionNode::default()),
        Arc::new(cosine_sim::FloatVectorCosineSimilarityNode::default()),
        Arc::new(normalize::FloatVectorNormalizeNode::default()),
    ]
}
