use crate::config::Config;
use flow_like_storage::files::store::FlowLikeStore;

pub fn create_store(config: &Config) -> Result<FlowLikeStore, StorageError> {
    // For the API, we typically just need the content store
    config
        .storage_config
        .build_store(&config.bucket_config.content)
        .map_err(|e| StorageError::Build(e.to_string()))
}

#[derive(Debug)]
pub enum StorageError {
    Build(String),
}

impl std::fmt::Display for StorageError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            StorageError::Build(msg) => write!(f, "Failed to build store: {}", msg),
        }
    }
}

impl std::error::Error for StorageError {}
