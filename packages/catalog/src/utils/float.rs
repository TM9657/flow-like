use flow_like::flow::node::NodeLogic;
use std::sync::Arc;

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

pub async fn register_functions() -> Vec<Arc<dyn NodeLogic>> {
    let items: Vec<Arc<dyn NodeLogic>> = vec![
        Arc::new(add::AddFloatNode::default()),
        Arc::new(ceil::CeilFloatNode::default()),
        Arc::new(clamp::ClampFloatNode::default()),
        Arc::new(divide::DivideFloatNode::default()),
        Arc::new(equal::EqualFloatNode::default()),
        Arc::new(floor::FloorFloatNode::default()),
        Arc::new(gt::GreaterThanFloatNode::default()),
        Arc::new(gte::GreaterThanOrEqualFloatNode::default()),
        Arc::new(lt::LessThanFloatNode::default()),
        Arc::new(lte::LessThanOrEqualFloatNode::default()),
        Arc::new(max::MaxFloatNode::default()),
        Arc::new(min::MinFloatNode::default()),
        Arc::new(multiply::MultiplyFloatNode::default()),
        Arc::new(random_range::RandomFloatInRangeNode::default()),
        Arc::new(round::RoundFloatNode::default()),
        Arc::new(subtract::SubtractFloatNode::default()),
        Arc::new(unequal::UnequalFloatNode::default()),
        Arc::new(abs::AbsFloatNode::default()),
        Arc::new(pow::PowerFloatNode::default()),
        Arc::new(root::RootFloatNode::default()),
    ];

    items
}
