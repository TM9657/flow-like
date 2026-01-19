//! Standard catalog for Flow-Like
//!
//! This crate contains standard nodes:
//! - Math operations
//! - Control flow
//! - Variables
//! - Structs/data structures
//! - Logging
//! - Utilities

use std::sync::Arc;

pub use flow_like_catalog_core::{NodeConstructor, NodeLogic, inventory, register_node};

pub mod control;
pub mod logging;
pub mod math;
pub mod structs;
pub mod utils;
pub mod variables;

pub fn get_catalog() -> Vec<Arc<dyn NodeLogic>> {
    flow_like_catalog_core::get_catalog()
}
