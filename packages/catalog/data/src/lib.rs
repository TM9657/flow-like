//! Data integration catalog for Flow-Like
//!
//! This crate contains data integration nodes:
//! - Google services
//! - Microsoft services
//! - GitHub integration
//! - Excel/CSV processing
//! - Database connections
//! - Path/file operations
//! - Events

use std::sync::Arc;

pub use flow_like_catalog_core::{NodeConstructor, NodeLogic, inventory, register_node};

pub mod data;
pub mod events;
pub mod interaction;

pub use data::*;

pub fn get_catalog() -> Vec<Arc<dyn NodeLogic>> {
    flow_like_catalog_core::get_catalog()
}
