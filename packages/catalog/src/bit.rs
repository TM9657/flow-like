pub mod from_string;
pub mod is_bit_of_type;
pub mod switch_on_bit;

use flow_like::flow::node::NodeLogic;
use std::sync::Arc;

pub async fn register_functions() -> Vec<Arc<dyn NodeLogic>> {
    vec![
        Arc::new(from_string::BitFromStringNode::default()) as Arc<dyn NodeLogic>,
        Arc::new(switch_on_bit::SwitchOnBitNode::default()) as Arc<dyn NodeLogic>,
        Arc::new(is_bit_of_type::IsBitOfTypeNode::default()) as Arc<dyn NodeLogic>,
    ]
}
