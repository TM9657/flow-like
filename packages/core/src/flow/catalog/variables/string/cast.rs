// use crate::flow::{execution::InternalRun, nodes::{Node, NodeLogic, Pin}};
// use async_trait::async_trait;
// use tauri::AppHandle;
// use std::sync::Arc;
// use serde_json::Value;
// use tokio::sync::Mutex;

// #[derive(Default, Clone)]
// pub struct CastStringFloat {

// }

// #[async_trait]
// impl NodeLogic for CastStringFloat {
//     async fn run(&self, node: Arc<Mutex<Node>>, handler: &FlowLikeState, internal_run: &mut InternalRun) {
//         let input_string_pin = node.lock().await.get_pin("input_string").unwrap();

//     }

//     async fn instantiate(&self, handler: &FlowLikeState, node: &Node) -> Arc<Mutex<dyn NodeLogic>> {
//         Arc::new(Mutex::new(self.clone()))
//     }

//     async fn get_node(&self, handler: &FlowLikeState) -> Node {

//         let mut pins = vec![];

//         Node {
//             id: "".to_string(),
//             name: "cast_string_float".to_string(),
//             friendly_name: "Cast String to Float".to_string(),
//             description: "Tries to parse a string number into a float".to_string(),
//             coordinates: None,
//             category: "variables/string".to_string(),
//             pins: pins,
//             start: false,
//         }
//     }

//     async fn is_ready(&self, handler: &FlowLikeState) -> bool {

//     }

//     async fn set_pin(&self, handler: &FlowLikeState, pin_id: &str, value: Value) {

//     }
// }
// pub struct CastStringInt {}
