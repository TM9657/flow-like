//! Flow-Like Catalog - Aggregate crate for all catalog modules
//!
//! This crate re-exports all catalog sub-crates for convenience.
//! You can also depend on individual catalog crates directly.

use std::sync::Arc;

use flow_like_catalog_core::NodeConstructor;

// Re-export core types and utilities
pub use flow_like_catalog_core::{
    Attachment, BoundingBox, CachedDB, FlowPath, FlowPathRuntime, FlowPathStore, NodeDBConnection,
    NodeImage, NodeImageWrapper, get_catalog as get_core_catalog, inventory, register_node,
};

// Re-export standard library
pub use flow_like_catalog_std::{control, logging, math, structs, utils, variables};

// Re-export data integrations
pub use flow_like_catalog_data::{data, events};

// Re-export web modules
pub use flow_like_catalog_web::{http, mail, web};

// Re-export media modules
pub use flow_like_catalog_media::{bit, image};

// Re-export ML module
pub use flow_like_catalog_ml::ml;

// Re-export ONNX module
pub use flow_like_catalog_onnx::{onnx, teachable_machine};

// Re-export LLM/GenAI modules
pub use flow_like_catalog_llm::generative;

// Re-export processing modules
pub use flow_like_catalog_processing::processing;

/// Get the full catalog from all sub-crates
pub fn get_catalog() -> Vec<Arc<dyn flow_like_catalog_core::NodeLogic>> {
    let mut catalog = flow_like_catalog_core::get_catalog();
    catalog.extend(flow_like_catalog_std::get_catalog());
    catalog.extend(flow_like_catalog_data::get_catalog());
    catalog.extend(flow_like_catalog_web::get_catalog());
    catalog.extend(flow_like_catalog_media::get_catalog());
    catalog.extend(flow_like_catalog_ml::get_catalog());
    catalog.extend(flow_like_catalog_onnx::get_catalog());
    catalog.extend(flow_like_catalog_llm::get_catalog());
    catalog.extend(flow_like_catalog_processing::get_catalog());
    catalog
}
