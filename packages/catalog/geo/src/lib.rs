//! Geo catalog for Flow-Like
//!
//! This crate contains geolocation-related nodes:
//! - Map image fetching
//! - Geocoding (forward and reverse)
//! - Route planning
//! - H3 geospatial indexing

pub use flow_like_catalog_core::{NodeConstructor, NodeLogic, inventory, register_node};
use std::sync::Arc;

pub mod geo;

pub fn get_catalog() -> Vec<Arc<dyn NodeLogic>> {
    flow_like_catalog_core::get_catalog()
}
