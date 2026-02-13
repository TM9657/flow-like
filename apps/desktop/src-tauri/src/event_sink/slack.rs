use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SlackSink {
    pub bot_token: String,
    pub app_token: Option<String>,
    pub channel_id: Option<String>,
    pub team_id: Option<String>,
    pub last_event_ts: Option<String>,
}

// Implementation in stubs.rs
