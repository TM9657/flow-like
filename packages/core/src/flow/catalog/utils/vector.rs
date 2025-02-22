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

pub async fn register_functions() -> Vec<Arc<Mutex<dyn NodeLogic>>> {
    vec![
        Arc::new(Mutex::new(addition::FloatVectorAdditionNode::default())),
        Arc::new(Mutex::new(
            cross_product::FloatVectorCrossProductNode::default(),
        )),
        Arc::new(Mutex::new(dot_product::FloatVectorDotProductNode::default())),
        Arc::new(Mutex::new(
            multiplication::FloatVectorMultiplicationNode::default(),
        )),
        Arc::new(Mutex::new(
            subtraction::FloatVectorSubtractionNode::default(),
        )),
        Arc::new(Mutex::new(
            cosine_sim::FloatVectorCosineSimilarityNode::default(),
        )),
        Arc::new(Mutex::new(normalize::FloatVectorNormalizeNode::default())),
    ]
}
