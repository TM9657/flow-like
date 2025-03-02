pub mod from_string;
pub mod is_bit_of_type;
pub mod switch_on_bit;

use crate::flow::node::NodeLogic;
use std::sync::Arc;
use tokio::sync::Mutex;

pub async fn register_functions() -> Vec<Arc<Mutex<dyn NodeLogic>>> {
    vec![
        Arc::new(Mutex::new(from_string::BitFromStringNode::default()))
            as Arc<Mutex<dyn NodeLogic>>,
        Arc::new(Mutex::new(switch_on_bit::SwitchOnBitNode::default()))
            as Arc<Mutex<dyn NodeLogic>>,
        Arc::new(Mutex::new(is_bit_of_type::IsBitOfTypeNode::default()))
            as Arc<Mutex<dyn NodeLogic>>,
    ]
}
