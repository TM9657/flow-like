use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NFCSink {
    pub tag_id: Option<String>,
    pub read_mode: NFCReadMode,
    pub last_tag_id: Option<String>,
    pub last_read_time: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum NFCReadMode {
    Any,
    NDEF,
    ISO14443A,
    ISO14443B,
    ISO15693,
}
