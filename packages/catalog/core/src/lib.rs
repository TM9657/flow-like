//! Core catalog types for Flow-Like
//!
//! This crate contains shared types used across all catalog crates:
//! - NodeImage, BoundingBox
//! - FlowPath, FlowPathRuntime, FlowPathStore
//! - NodeDBConnection, CachedDB
//! - Attachment
//! - NodeConstructor and get_catalog()

use std::sync::Arc;

pub use flow_like::flow::node::NodeLogic;

pub use flow_like_catalog_macros::register_node;
pub use inventory;

mod types;

pub use types::attachment::Attachment;
pub use types::bounding_box::BoundingBox;
pub use types::db_connection::{CachedDB, NodeDBConnection};
pub use types::flow_path::{FlowPath, FlowPathRuntime, FlowPathStore};
pub use types::node_image::{NodeImage, NodeImageWrapper};

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
