use std::sync::Arc;

use flow_like::flow::node::NodeLogic;

pub mod ai;
pub mod bit;
pub mod control;
pub mod events;
pub mod http;
pub mod logging;
pub mod math;
pub mod storage;
pub mod structs;
pub mod utils;
pub mod variables;
pub mod web;

pub async fn get_catalog() -> Vec<Arc<dyn NodeLogic>> {
    let catalog: Vec<Arc<dyn NodeLogic>> = vec![
        ai::register_functions().await,
        control::register_functions().await,
        variables::register_functions().await,
        logging::register_functions().await,
        events::register_functions().await,
        utils::register_functions().await,
        structs::register_functions().await,
        storage::register_functions().await,
        bit::register_functions().await,
        web::register_functions().await,
    ]
    .into_iter()
    .flatten()
    .collect();

    catalog
}
