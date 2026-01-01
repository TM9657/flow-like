//! ONNX/TFLite inference catalog for Flow-Like
//!
//! This crate contains ONNX and TFLite inference nodes for:
//! - Object detection
//! - Image classification
//! - Feature extraction
//! - Teachable Machine models

use std::sync::Arc;

pub use flow_like_catalog_core::{NodeConstructor, NodeLogic, inventory, register_node};

#[path = "onnx.rs"]
pub mod onnx;
pub mod teachable_machine;

pub use onnx::*;

pub fn get_catalog() -> Vec<Arc<dyn NodeLogic>> {
    flow_like_catalog_core::get_catalog()
}
