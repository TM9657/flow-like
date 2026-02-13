use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NotionSink {
    pub integration_token: String,
    pub database_id: Option<String>,
    pub page_id: Option<String>,
    pub poll_interval: u64,
    pub last_edited_time: Option<String>,
}

// Implementation in stubs.rs
