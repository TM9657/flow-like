use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeeplinkSink {
    pub path_pattern: String,
}
