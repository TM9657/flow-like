use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WebWatcherSink {
    pub url: String,
    pub check_interval: u64,
    pub selector: Option<String>,
    pub headers: Option<Vec<(String, String)>>,
    pub method: Option<String>,
    pub last_content_hash: Option<String>,
}

// Implementation in stubs.rs
