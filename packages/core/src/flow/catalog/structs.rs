pub mod make;
pub mod fields;

use crate::flow::node::NodeLogic;
use std::sync::Arc;
use tokio::sync::Mutex;

pub async fn register_functions() -> Vec<Arc<Mutex<dyn NodeLogic>>> {
    vec![
        Arc::new(Mutex::new(make::MakeStructNode::default())) as Arc<Mutex<dyn NodeLogic>>,
        Arc::new(Mutex::new(fields::has_field::HasStructFieldNode::default())) as Arc<Mutex<dyn NodeLogic>>,
        Arc::new(Mutex::new(fields::get_field::GetStructFieldNode::default())) as Arc<Mutex<dyn NodeLogic>>,
        Arc::new(Mutex::new(fields::set_field::SetStructFieldNode::default())) as Arc<Mutex<dyn NodeLogic>>,
    ]
}
