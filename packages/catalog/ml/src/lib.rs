//! Machine Learning catalog for Flow-Like
//!
//! This crate contains traditional ML nodes based on linfa (clustering, SVM, regression, etc.)
//! Does NOT include ONNX inference - see flow-like-catalog-onnx for that.

use std::sync::Arc;

pub use flow_like_catalog_core::{NodeConstructor, NodeLogic, inventory, register_node};

#[path = "ml.rs"]
pub mod ml;

#[cfg(test)]
mod tests;

pub use ml::*;

pub fn get_catalog() -> Vec<Arc<dyn NodeLogic>> {
    flow_like_catalog_core::get_catalog()
}
