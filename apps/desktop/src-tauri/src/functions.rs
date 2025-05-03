use serde::Serialize;

pub mod ai;
pub mod app;
pub mod bit;
pub mod download;
pub mod file;
pub mod flow;
pub mod settings;
pub mod system;

#[derive(Debug, Serialize)]
pub struct TauriFunctionError {
    error: String,
}

impl TauriFunctionError {
    pub fn new(error: &str) -> Self {
        Self {
            error: error.to_string(),
        }
    }
}

impl From<flow_like::flow_like_storage::async_duckdb::Error> for TauriFunctionError {
    fn from(error: flow_like::flow_like_storage::async_duckdb::Error) -> Self {
        Self {
            error: error.to_string(),
        }
    }
}

impl From<flow_like_types::Error> for TauriFunctionError {
    fn from(error: flow_like_types::Error) -> Self {
        Self {
            error: error.to_string(),
        }
    }
}
