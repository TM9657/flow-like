pub mod fields;
pub mod make;

use crate::flow::node::NodeLogic;
use std::sync::Arc;

pub async fn register_functions() -> Vec<Arc<dyn NodeLogic>> {
    vec![
        Arc::new(make::MakeStructNode::default()),
        Arc::new(fields::has_field::HasStructFieldNode::default()),
        Arc::new(fields::get_field::GetStructFieldNode::default()),
        Arc::new(fields::set_field::SetStructFieldNode::default()),
    ]
}
