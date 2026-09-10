use super::{AppBuildStoreError, storage_error};
use flow_like_storage::{Path, files::store::local_store::LocalObjectStore};
use std::{
    fs::{File, OpenOptions, TryLockError},
    path::PathBuf,
    time::{Duration, Instant},
};

const LOCK_TIMEOUT: Duration = Duration::from_secs(5);
const LOCK_RETRY_DELAY: Duration = Duration::from_millis(10);

/// Acquire the persistent sidecar lock for one build without blocking an async worker.
pub(super) async fn acquire(
    store: &LocalObjectStore,
    lock_path: &Path,
) -> Result<File, AppBuildStoreError> {
    let lock_path = store
        .path_to_filesystem(lock_path)
        .map_err(|error| storage_error("resolve local lock path", error))?;

    flow_like_types::tokio::task::spawn_blocking(move || acquire_blocking(lock_path))
        .await
        .map_err(|error| AppBuildStoreError::Storage {
            operation: "acquire local lock",
            source: format!("lock task failed: {error}"),
        })?
}

fn acquire_blocking(path: PathBuf) -> Result<File, AppBuildStoreError> {
    let Some(parent) = path.parent() else {
        return Err(AppBuildStoreError::Storage {
            operation: "acquire local lock",
            source: format!("lock path {} has no parent", path.display()),
        });
    };
    std::fs::create_dir_all(parent).map_err(|error| AppBuildStoreError::Storage {
        operation: "create local lock directory",
        source: error.to_string(),
    })?;
    let file = OpenOptions::new()
        .create(true)
        .read(true)
        .write(true)
        // A lock file carries holder metadata; opening must never discard it.
        .truncate(false)
        .open(&path)
        .map_err(|error| AppBuildStoreError::Storage {
            operation: "open local lock",
            source: error.to_string(),
        })?;
    let started = Instant::now();
    loop {
        match file.try_lock() {
            Ok(()) => return Ok(file),
            Err(TryLockError::WouldBlock) if started.elapsed() < LOCK_TIMEOUT => {
                std::thread::sleep(LOCK_RETRY_DELAY);
            }
            Err(TryLockError::WouldBlock) => {
                return Err(AppBuildStoreError::Storage {
                    operation: "acquire local lock",
                    source: format!(
                        "timed out after {} ms waiting for {}",
                        LOCK_TIMEOUT.as_millis(),
                        path.display()
                    ),
                });
            }
            Err(TryLockError::Error(error)) => {
                return Err(AppBuildStoreError::Storage {
                    operation: "acquire local lock",
                    source: error.to_string(),
                });
            }
        }
    }
}
