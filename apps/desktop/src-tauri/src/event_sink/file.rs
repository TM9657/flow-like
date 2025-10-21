use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileSink {
    pub path: String,
    pub watch_type: FileWatchType,
    pub file_pattern: Option<String>,
    pub recursive: bool,
    pub last_modified: Option<String>,
    pub processed_files: Option<Vec<String>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum FileWatchType {
    Create,
    Modify,
    Delete,
    Any,
}

// Implementation in stubs.rs
