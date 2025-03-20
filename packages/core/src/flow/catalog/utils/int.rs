use crate::flow::node::NodeLogic;
use std::sync::Arc;
use tokio::sync::Mutex;

pub mod abs;
pub mod add;
pub mod clamp;
pub mod divide;
pub mod equal;
pub mod gt;
pub mod gte;
pub mod lt;
pub mod lte;
pub mod max;
pub mod min;
pub mod modulo;
pub mod multiply;
pub mod pow;
pub mod random_range;
pub mod root;
pub mod subtract;
pub mod unequal;

pub async fn register_functions() -> Vec<Arc<dyn NodeLogic>> {
    let items: Vec<Arc<dyn NodeLogic>> = vec![
        Arc::new(abs::AbsoluteIntegerNode::default()),
        Arc::new(add::AddIntegerNode::default()),
        Arc::new(subtract::SubtractIntegerNode::default()),
        Arc::new(multiply::MultiplyIntegerNode::default()),
        Arc::new(divide::DivideIntegerNode::default()),
        Arc::new(max::MaxIntegerNode::default()),
        Arc::new(min::MinIntegerNode::default()),
        Arc::new(equal::EqualIntegerNode::default()),
        Arc::new(unequal::UnequalIntegerNode::default()),
        Arc::new(gt::GreaterThanIntegerNode::default()),
        Arc::new(gte::GreaterThanOrEqualIntegerNode::default()),
        Arc::new(lt::LessThanIntegerNode::default()),
        Arc::new(lte::LessThanOrEqualIntegerNode::default()),
        Arc::new(random_range::RandomIntegerInRangeNode::default()),
        Arc::new(clamp::ClampIntegerNode::default()),
        Arc::new(modulo::ModuloIntegerNode::default()),
        Arc::new(pow::PowerIntegerNode::default()),
        Arc::new(root::RootIntegerNode::default()),
    ];

    items
}
