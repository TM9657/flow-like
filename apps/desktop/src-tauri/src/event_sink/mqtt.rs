use serde::{Deserialize, Serialize};


#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MQTTSink {
    pub id: String,
    pub broker_url: String,
    pub topic: String,
    pub client_id: Option<String>,
    pub username: Option<String>,
    pub password: Option<String>,
    pub qos: Option<u8>,
    pub use_tls: bool,
    pub last_message_id: Option<String>,
}

// Implementation in stubs.rs
