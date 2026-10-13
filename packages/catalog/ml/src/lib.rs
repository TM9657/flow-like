//! Machine Learning catalog for Flow-Like
//!
//! This crate contains traditional ML nodes based on linfa (clustering, SVM, regression, etc.)
//! Does NOT include ONNX inference - see flow-like-catalog-onnx for that.
// Training node futures reach Lance and object-store readers, which need deeper Send trait evaluation.
#![recursion_limit = "256"]

extern crate flow_like_runtime as flow_like;

use std::sync::Arc;

pub use flow_like_catalog_core::{NodeConstructor, NodeLogic, register_node};

#[path = "ml.rs"]
pub mod ml;

#[cfg(test)]
mod tests;

pub use ml::*;

include!(concat!(env!("OUT_DIR"), "/node_registry.rs"));

pub fn get_catalog() -> Vec<Arc<dyn NodeLogic>> {
    collect_nodes()
}
