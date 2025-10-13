use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ShortcutSink {
    pub id: String,
    pub key_combination: String,
}
