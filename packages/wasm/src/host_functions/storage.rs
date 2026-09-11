//! Storage host functions
//!
//! Provides storage access for WASM modules.

use flow_like_storage::normalize_object_path;
use flow_like_storage::object_store::ObjectStoreExt;
use flow_like_storage::object_store::{PutPayload, path::Path};
use flow_like_types::Bytes;
use std::collections::HashMap;

use super::StorageContext;

pub const MAX_STORAGE_FILE_SIZE: usize = 10 * 1024 * 1024;
pub const MAX_PENDING_WRITES: usize = 8;
pub const MAX_TOTAL_WRITE_SIZE: usize = 512 * 1024 * 1024;

#[derive(Debug)]
pub struct PendingWrite {
    pub flow_path: StorageFlowPath,
    pub buffer: Vec<u8>,
    pub total_size: u64,
}

#[derive(Debug, serde::Serialize, serde::Deserialize, Clone)]
pub struct StorageFlowPath {
    pub path: String,
    pub store_ref: String,
    pub cache_store_ref: Option<String>,
}

impl StorageFlowPath {
    /// Canonical object key. Guests may send a raw name or a key that came out
    /// of a list response; both resolve to the same object.
    pub fn object_path(&self) -> Path {
        normalize_object_path(&self.path)
    }
}

pub fn validate_path(path: &str) -> bool {
    !path.contains("..") && !path.starts_with('/') && !path.is_empty()
}

pub fn start_write(
    pending: &mut HashMap<String, PendingWrite>,
    flow_path: StorageFlowPath,
    total_size: u64,
) -> Option<String> {
    if pending.len() >= MAX_PENDING_WRITES {
        tracing::warn!("[wasm write-start] rejected: too many pending writes");
        return None;
    }
    if total_size as usize > MAX_TOTAL_WRITE_SIZE {
        tracing::warn!(
            "[wasm write-start] rejected: total_size {} exceeds max {}",
            total_size,
            MAX_TOTAL_WRITE_SIZE
        );
        return None;
    }
    let id = format!(
        "cw_{:x}_{:x}",
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_nanos(),
        pending.len()
    );
    pending.insert(
        id.clone(),
        PendingWrite {
            flow_path,
            buffer: Vec::with_capacity(total_size as usize),
            total_size,
        },
    );
    Some(id)
}

pub fn append_chunk(
    pending: &mut HashMap<String, PendingWrite>,
    write_id: &str,
    data: &[u8],
) -> bool {
    if data.len() > MAX_STORAGE_FILE_SIZE {
        tracing::warn!(
            "[wasm write-chunk] rejected: chunk size {} exceeds max {}",
            data.len(),
            MAX_STORAGE_FILE_SIZE
        );
        return false;
    }
    let Some(pw) = pending.get_mut(write_id) else {
        tracing::warn!("[wasm write-chunk] rejected: unknown write_id {write_id}");
        return false;
    };
    if pw.buffer.len() + data.len() > pw.total_size as usize {
        tracing::warn!("[wasm write-chunk] rejected: would exceed declared total_size");
        return false;
    }
    pw.buffer.extend_from_slice(data);
    true
}

pub async fn put_flow_path(
    storage_ctx: &StorageContext,
    flow_path: &StorageFlowPath,
    data: Vec<u8>,
    log_prefix: &str,
) -> bool {
    let Some(store) = storage_ctx.resolve_store(&flow_path.store_ref) else {
        tracing::warn!(
            "[{log_prefix}] rejected: unresolved store_ref={}",
            flow_path.store_ref
        );
        return false;
    };

    let path = flow_path.object_path();
    let payload = PutPayload::from_bytes(Bytes::from(data));

    if let Err(e) = store.as_generic().put(&path, payload.clone()).await {
        tracing::warn!(
            "[{log_prefix}] put failed for path={} store_ref={}: {e}",
            flow_path.path,
            flow_path.store_ref
        );
        return false;
    }

    if let Some(cache_store_ref) = flow_path.cache_store_ref.as_deref() {
        if cache_store_ref == flow_path.store_ref {
            return true;
        }

        let Some(cache_store) = storage_ctx.resolve_store(cache_store_ref) else {
            tracing::warn!(
                "[{log_prefix}] cache write skipped: unresolved cache_store_ref={cache_store_ref}"
            );
            return true;
        };

        if let Err(e) = cache_store.as_generic().put(&path, payload).await {
            tracing::warn!(
                "[{log_prefix}] cache put failed for path={} cache_store_ref={}: {e}",
                flow_path.path,
                cache_store_ref
            );
        }
    }

    true
}

pub async fn finish_write(
    pending: &mut HashMap<String, PendingWrite>,
    write_id: &str,
    storage_ctx: &StorageContext,
) -> bool {
    let Some(pw) = pending.remove(write_id) else {
        tracing::warn!("[wasm write-finish] rejected: unknown write_id {write_id}");
        return false;
    };
    put_flow_path(storage_ctx, &pw.flow_path, pw.buffer, "wasm write-finish").await
}

#[cfg(test)]
mod tests {
    use super::*;
    use flow_like_storage::display_file_name;

    const NAME: &str = "Übersicht (2)#1.pdf";

    fn flow_path(path: impl Into<String>) -> StorageFlowPath {
        StorageFlowPath {
            path: path.into(),
            store_ref: "store".into(),
            cache_store_ref: None,
        }
    }

    #[test]
    fn raw_and_listed_paths_resolve_to_the_same_key() {
        let expected = Path::from("apps/a/upload").join(NAME);
        let raw = flow_path(format!("apps/a/upload/{NAME}"));
        let listed = flow_path(expected.as_ref());

        assert_eq!(raw.object_path(), expected);
        assert_eq!(listed.object_path(), expected);
        assert_eq!(
            display_file_name(&listed.object_path()).as_deref(),
            Some(NAME)
        );
    }

    #[test]
    fn traversal_in_guest_path_stays_inert() {
        assert_eq!(
            flow_path("apps/a/../../etc/passwd").object_path().as_ref(),
            "apps/a/%2E%2E/%2E%2E/etc/passwd"
        );
    }
}
