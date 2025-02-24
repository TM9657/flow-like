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

pub async fn register_functions() -> Vec<Arc<Mutex<dyn NodeLogic>>> {
    let items: Vec<Arc<Mutex<dyn NodeLogic>>> = vec![
        Arc::new(Mutex::new(abs::AbsoluteIntegerNode::default())),
        Arc::new(Mutex::new(add::AddIntegerNode::default())),
        Arc::new(Mutex::new(subtract::SubtractIntegerNode::default())),
        Arc::new(Mutex::new(multiply::MultiplyIntegerNode::default())),
        Arc::new(Mutex::new(divide::DivideIntegerNode::default())),
        Arc::new(Mutex::new(max::MaxIntegerNode::default())),
        Arc::new(Mutex::new(min::MinIntegerNode::default())),
        Arc::new(Mutex::new(equal::EqualIntegerNode::default())),
        Arc::new(Mutex::new(unequal::UnequalIntegerNode::default())),
        Arc::new(Mutex::new(gt::GreaterThanIntegerNode::default())),
        Arc::new(Mutex::new(gte::GreaterThanOrEqualIntegerNode::default())),
        Arc::new(Mutex::new(lt::LessThanIntegerNode::default())),
        Arc::new(Mutex::new(lte::LessThanOrEqualIntegerNode::default())),
        Arc::new(Mutex::new(random_range::RandomIntegerInRangeNode::default())),
        Arc::new(Mutex::new(clamp::ClampIntegerNode::default())),
        Arc::new(Mutex::new(modulo::ModuloIntegerNode::default())),
        Arc::new(Mutex::new(pow::PowerIntegerNode::default())),
        Arc::new(Mutex::new(root::RootIntegerNode::default())),
    ];

    items
}
