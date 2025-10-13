use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeeplinkSink {
    pub id: String,
    pub path_pattern: String,
}
