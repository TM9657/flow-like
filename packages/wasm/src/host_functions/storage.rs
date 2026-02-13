//! Storage host functions
//!
//! Provides storage access for WASM modules.

// Storage functions need async handling and are implemented in linker.rs
// This module can provide utilities for storage path validation etc.

/// Maximum file size for storage operations (10MB)
pub const MAX_STORAGE_FILE_SIZE: usize = 10 * 1024 * 1024;

/// Validate a storage path
pub fn validate_path(path: &str) -> bool {
    // Path should not contain ..
    !path.contains("..") &&
    // Path should not be absolute
    !path.starts_with('/') &&
    // Path should not be empty
    !path.is_empty()
}
