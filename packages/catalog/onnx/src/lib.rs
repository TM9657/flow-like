//! ONNX/TFLite inference catalog for Flow-Like
//!
//! This crate contains ONNX and TFLite inference nodes for:
//! - Object detection
//! - Image classification
//! - Feature extraction
//! - Teachable Machine models
//! - Depth estimation
//! - Face detection and recognition
//! - OCR (text detection and recognition)
//! - Audio processing (VAD)
//! - Batch inference
//! - Named Entity Recognition (NER)

use std::sync::Arc;

pub use flow_like_catalog_core::{NodeConstructor, NodeLogic, inventory, register_node};

#[path = "onnx.rs"]
pub mod onnx;
pub mod teachable_machine;

pub use onnx::*;

// Re-export submodules for external access
pub use onnx::{
    audio, batch, classification, depth, detection, face, feature, load, ner, ocr, pose,
    segmentation,
};

pub fn get_catalog() -> Vec<Arc<dyn NodeLogic>> {
    flow_like_catalog_core::get_catalog()
}
