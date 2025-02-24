use crate::flow::node::NodeLogic;
use std::sync::Arc;
use tokio::sync::Mutex;

pub mod abs;
pub mod add;
pub mod ceil;
pub mod clamp;
pub mod divide;
pub mod equal;
pub mod floor;
pub mod gt;
pub mod gte;
pub mod lt;
pub mod lte;
pub mod max;
pub mod min;
pub mod multiply;
pub mod pow;
pub mod random_range;
pub mod root;
pub mod round;
pub mod subtract;
pub mod unequal;

pub async fn register_functions() -> Vec<Arc<Mutex<dyn NodeLogic>>> {
    let items: Vec<Arc<Mutex<dyn NodeLogic>>> = vec![
        Arc::new(Mutex::new(add::AddFloatNode::default())),
        Arc::new(Mutex::new(ceil::CeilFloatNode::default())),
        Arc::new(Mutex::new(clamp::ClampFloatNode::default())),
        Arc::new(Mutex::new(divide::DivideFloatNode::default())),
        Arc::new(Mutex::new(equal::EqualFloatNode::default())),
        Arc::new(Mutex::new(floor::FloorFloatNode::default())),
        Arc::new(Mutex::new(gt::GreaterThanFloatNode::default())),
        Arc::new(Mutex::new(gte::GreaterThanOrEqualFloatNode::default())),
        Arc::new(Mutex::new(lt::LessThanFloatNode::default())),
        Arc::new(Mutex::new(lte::LessThanOrEqualFloatNode::default())),
        Arc::new(Mutex::new(max::MaxFloatNode::default())),
        Arc::new(Mutex::new(min::MinFloatNode::default())),
        Arc::new(Mutex::new(multiply::MultiplyFloatNode::default())),
        Arc::new(Mutex::new(random_range::RandomFloatInRangeNode::default())),
        Arc::new(Mutex::new(round::RoundFloatNode::default())),
        Arc::new(Mutex::new(subtract::SubtractFloatNode::default())),
        Arc::new(Mutex::new(unequal::UnequalFloatNode::default())),
        Arc::new(Mutex::new(abs::AbsFloatNode::default())),
        Arc::new(Mutex::new(pow::PowerFloatNode::default())),
        Arc::new(Mutex::new(root::RootFloatNode::default())),
    ];

    items
}
