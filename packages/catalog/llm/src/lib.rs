//! LLM/Generative AI catalog for Flow-Like
//!
//! This crate contains LLM invocation, embedding, agents, and MCP tools.

use std::sync::Arc;

pub use flow_like_catalog_core::{NodeConstructor, NodeLogic, inventory, register_node};

#[path = "generative.rs"]
pub mod generative;

pub use generative::*;

pub fn get_catalog() -> Vec<Arc<dyn NodeLogic>> {
    flow_like_catalog_core::get_catalog()
}
