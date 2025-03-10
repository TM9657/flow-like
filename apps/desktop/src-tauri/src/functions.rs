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

impl From<anyhow::Error> for TauriFunctionError {
    fn from(error: anyhow::Error) -> Self {
        Self {
            error: error.to_string(),
        }
    }
}
