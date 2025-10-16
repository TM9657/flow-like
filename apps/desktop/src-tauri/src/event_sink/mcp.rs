use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MCPSink {
    pub server_url: String,
    pub api_key: Option<String>,
    pub event_type: String,
    pub filters: Option<Vec<(String, String)>>,
    pub last_event_id: Option<String>,
    pub last_event_timestamp: Option<String>,
}
