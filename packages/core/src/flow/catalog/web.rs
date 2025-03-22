pub mod api;
pub mod scrape;

use crate::flow::node::NodeLogic;
use std::sync::Arc;

pub async fn register_functions() -> Vec<Arc<dyn NodeLogic>> {
    let mut out = Vec::new();

    out.extend(api::register_functions().await);

    out
}
