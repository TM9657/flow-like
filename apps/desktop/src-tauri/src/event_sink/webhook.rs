use serde::{Deserialize, Serialize};


#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WebhookSink {
    pub id: String,
    pub path: String,
    pub secret: Option<String>,
    pub allowed_ips: Option<Vec<String>>,
}

// Implementation in stubs.rs
