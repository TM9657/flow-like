use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TelegramSink {
    pub bot_token: String,
    pub chat_id: Option<String>,
    pub allowed_updates: Option<Vec<String>>,
    pub last_update_id: Option<i64>,
}

// Implementation in stubs.rs
