//! Automation catalog for Flow-Like
//!
//! This crate contains automation nodes for:
//! - Browser automation (CDP-based)
//! - Desktop/computer automation (mouse, keyboard, screenshots)
//! - Selectors and element fingerprinting
//! - Vision/template matching
//! - LLM-assisted self-healing
//! - RPA reliability primitives

use std::sync::Arc;

pub use flow_like_catalog_core::{NodeConstructor, NodeLogic, inventory, register_node};

pub mod types;

pub mod browser;
pub mod computer;
pub mod fingerprint;
pub mod llm;
pub mod rpa;
pub mod selector;
pub mod session;
pub mod vision;

pub fn get_catalog() -> Vec<Arc<dyn NodeLogic>> {
    flow_like_catalog_core::get_catalog()
}
