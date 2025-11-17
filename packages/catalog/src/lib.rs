use std::sync::Arc;

use flow_like::flow::node::NodeLogic;

// Re-export for use in the macro
pub use flow_like_catalog_macros::register_node;
pub use inventory;

pub mod ai;
pub mod bit;
pub mod control;
pub mod data;
pub mod events;
pub mod http;
pub mod image;
pub mod logging;
pub mod mail;
pub mod math;
pub mod structs;
pub mod utils;
pub mod variables;
pub mod web;

/// A node constructor function type
pub struct NodeConstructor {
    constructor: fn() -> Arc<dyn NodeLogic>,
}

impl NodeConstructor {
    pub const fn new(constructor: fn() -> Arc<dyn NodeLogic>) -> Self {
        Self { constructor }
    }

    pub fn construct(&self) -> Arc<dyn NodeLogic> {
        (self.constructor)()
    }
}

inventory::collect!(NodeConstructor);

pub fn get_catalog() -> Vec<Arc<dyn NodeLogic>> {
    inventory::iter::<NodeConstructor>()
        .map(|nc| nc.construct())
        .collect()
}
