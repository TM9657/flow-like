//! Durable storage for resumable FlowPilot app builds.
//!
//! Build state is an opaque JSON envelope owned by the app-build engine. This module only
//! accepts a fixed schema version and identity. Generic stores use object-store preconditions.
//! Desktop-local stores use a per-build OS lock and an atomic filesystem overwrite.

mod local_lock;

use flow_like_storage::{
    Path,
    files::store::local_store::LocalObjectStore,
    object_store::{
        Error as ObjectStoreError, ObjectMeta, ObjectStore, ObjectStoreExt, PutMode, PutOptions,
        PutPayload, UpdateVersion, local::LocalFileSystem,
    },
};
use serde_json::Value;
use std::{any::TypeId, fmt, sync::Arc};

pub const APP_BUILD_SCHEMA_VERSION: u64 = 1;
pub const MAX_APP_BUILD_BYTES: usize = 1024 * 1024;
const MAX_ID_BYTES: usize = 128;

/// Converts a typed or type-erased object-store handle for [`AppBuildStore::new`].
///
/// This trait preserves the concrete type long enough to reject the two known local filesystem
/// stores. Callers that already hold `Arc<dyn ObjectStore>` must ensure it is a backend with real
/// create and update preconditions. Host adapters keep the typed `FlowLikeStore` variant until
/// they can select [`AppBuildStore::new_local`].
#[doc(hidden)]
pub trait IntoAppBuildObjectStore {
    fn supports_conditional_writes(&self) -> bool;
    fn into_object_store(self) -> Arc<dyn ObjectStore>;
}

impl<T: ObjectStore> IntoAppBuildObjectStore for Arc<T> {
    fn supports_conditional_writes(&self) -> bool {
        TypeId::of::<T>() != TypeId::of::<LocalObjectStore>()
            && TypeId::of::<T>() != TypeId::of::<LocalFileSystem>()
    }

    fn into_object_store(self) -> Arc<dyn ObjectStore> {
        self
    }
}

impl IntoAppBuildObjectStore for Arc<dyn ObjectStore> {
    fn supports_conditional_writes(&self) -> bool {
        true
    }

    fn into_object_store(self) -> Arc<dyn ObjectStore> {
        self
    }
}

/// A project-scoped store for FlowPilot build checkpoints.
#[derive(Clone)]
pub struct AppBuildStore {
    store: Arc<dyn ObjectStore>,
    local_store: Option<Arc<LocalObjectStore>>,
    generic_conditionals_supported: bool,
    app_id: String,
}

#[derive(Debug, Clone, PartialEq)]
pub enum AppBuildStoreError {
    InvalidId {
        field: &'static str,
        reason: String,
    },
    InvalidRecord {
        reason: String,
    },
    CorruptStoredRecord {
        path: String,
        reason: String,
    },
    TooLarge {
        actual: usize,
        maximum: usize,
    },
    Conflict {
        expected_revision: Option<u64>,
        current_revision: Option<u64>,
        current: Option<Value>,
    },
    ConditionalWriteUnsupported {
        operation: &'static str,
    },
    Storage {
        operation: &'static str,
        source: String,
    },
}

impl fmt::Display for AppBuildStoreError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidId { field, reason } => {
                write!(formatter, "invalid {field}: {reason}")
            }
            Self::InvalidRecord { reason } => write!(formatter, "invalid app build: {reason}"),
            Self::CorruptStoredRecord { path, reason } => {
                write!(formatter, "stored app build at {path} is invalid: {reason}")
            }
            Self::TooLarge { actual, maximum } => write!(
                formatter,
                "app build is {actual} bytes, exceeding the {maximum} byte limit"
            ),
            Self::Conflict {
                expected_revision,
                current_revision,
                ..
            } => write!(
                formatter,
                "app build revision conflict: expected {expected_revision:?}, current {current_revision:?}"
            ),
            Self::ConditionalWriteUnsupported { operation } => write!(
                formatter,
                "object store does not support the conditional write required for {operation}"
            ),
            Self::Storage { operation, source } => {
                write!(formatter, "failed to {operation} app build: {source}")
            }
        }
    }
}

impl std::error::Error for AppBuildStoreError {}

#[derive(Debug)]
struct StoredBuild {
    record: Value,
    revision: u64,
    meta: ObjectMeta,
}

impl AppBuildStore {
    pub fn new<S: IntoAppBuildObjectStore>(
        store: S,
        app_id: impl Into<String>,
    ) -> Result<Self, AppBuildStoreError> {
        let app_id = app_id.into();
        validate_id("app_id", &app_id)?;
        let generic_conditionals_supported = store.supports_conditional_writes();
        Ok(Self {
            store: store.into_object_store(),
            local_store: None,
            generic_conditionals_supported,
            app_id,
        })
    }

    /// Use filesystem locking for a desktop-local store.
    ///
    /// `LocalObjectStore` cannot provide a real conditional update. Every create and update
    /// therefore takes a persistent, per-build sidecar lock, checks the current revision while
    /// holding it, and commits with `PutMode::Overwrite`. The underlying `LocalFileSystem`
    /// implements that overwrite with a staged file followed by an atomic rename.
    pub fn new_local(
        store: Arc<LocalObjectStore>,
        app_id: impl Into<String>,
    ) -> Result<Self, AppBuildStoreError> {
        let app_id = app_id.into();
        validate_id("app_id", &app_id)?;
        Ok(Self {
            store: store.clone() as Arc<dyn ObjectStore>,
            local_store: Some(store),
            generic_conditionals_supported: false,
            app_id,
        })
    }

    /// Read a build checkpoint. A missing object is the only case returned as `None`.
    pub async fn read(&self, build_id: &str) -> Result<Option<Value>, AppBuildStoreError> {
        Ok(self
            .read_with_meta(build_id)
            .await?
            .map(|stored| stored.record))
    }

    /// Create revision zero. If another writer has already created the build, returns its
    /// current state in [`AppBuildStoreError::Conflict`].
    pub async fn create(&self, build_id: &str, record: Value) -> Result<Value, AppBuildStoreError> {
        validate_id("build_id", build_id)?;
        let revision = validate_record(&record, &self.app_id, build_id)
            .map_err(|reason| AppBuildStoreError::InvalidRecord { reason })?;
        if revision != 0 {
            return Err(AppBuildStoreError::InvalidRecord {
                reason: "a newly created build must have revision 0".to_string(),
            });
        }
        let bytes = serialize_record(&record)?;

        if self.local_store.is_some() {
            let store = self.clone();
            let build_id = build_id.to_string();
            let operation = flow_like_types::tokio::spawn(async move {
                store.create_local(&build_id, record, bytes).await
            });
            return await_local_operation(operation).await;
        }
        if !self.generic_conditionals_supported {
            return Err(AppBuildStoreError::ConditionalWriteUnsupported {
                operation: "create",
            });
        }

        let path = self.path(build_id);
        let options = PutOptions {
            mode: PutMode::Create,
            ..Default::default()
        };

        match self
            .store
            .put_opts(&path, PutPayload::from(bytes), options)
            .await
        {
            Ok(_) => Ok(record),
            Err(ObjectStoreError::AlreadyExists { .. })
            | Err(ObjectStoreError::Precondition { .. }) => {
                Err(self.conflict_with_current(build_id, None).await?)
            }
            Err(ObjectStoreError::NotImplemented { .. })
            | Err(ObjectStoreError::NotSupported { .. }) => {
                Err(AppBuildStoreError::ConditionalWriteUnsupported {
                    operation: "create",
                })
            }
            Err(error) => Err(storage_error("create", error)),
        }
    }

    /// Replace a checkpoint only when its stored revision and object identity still match.
    pub async fn compare_and_swap(
        &self,
        build_id: &str,
        expected_revision: u64,
        next: Value,
    ) -> Result<Value, AppBuildStoreError> {
        validate_id("build_id", build_id)?;
        let revision = validate_record(&next, &self.app_id, build_id)
            .map_err(|reason| AppBuildStoreError::InvalidRecord { reason })?;
        let required_revision =
            expected_revision
                .checked_add(1)
                .ok_or_else(|| AppBuildStoreError::InvalidRecord {
                    reason: "revision cannot advance beyond an unsigned 64-bit integer".to_string(),
                })?;
        if revision != required_revision {
            return Err(AppBuildStoreError::InvalidRecord {
                reason: format!(
                    "update revision must be {required_revision} when expected_revision is {expected_revision}"
                ),
            });
        }

        if self.local_store.is_some() {
            let store = self.clone();
            let build_id = build_id.to_string();
            let operation = flow_like_types::tokio::spawn(async move {
                store
                    .compare_and_swap_local(&build_id, expected_revision, next)
                    .await
            });
            return await_local_operation(operation).await;
        }
        if !self.generic_conditionals_supported {
            return Err(AppBuildStoreError::ConditionalWriteUnsupported {
                operation: "compare-and-swap update",
            });
        }

        let Some(current) = self.read_with_meta(build_id).await? else {
            return Err(AppBuildStoreError::Conflict {
                expected_revision: Some(expected_revision),
                current_revision: None,
                current: None,
            });
        };
        if current.revision != expected_revision {
            return Err(AppBuildStoreError::Conflict {
                expected_revision: Some(expected_revision),
                current_revision: Some(current.revision),
                current: Some(current.record),
            });
        }

        let update_version = UpdateVersion {
            e_tag: current.meta.e_tag,
            version: current.meta.version,
        };
        if update_version.e_tag.is_none() && update_version.version.is_none() {
            return Err(AppBuildStoreError::ConditionalWriteUnsupported {
                operation: "compare-and-swap update",
            });
        }

        let bytes = serialize_record(&next)?;
        let path = self.path(build_id);
        let options = PutOptions {
            mode: PutMode::Update(update_version),
            ..Default::default()
        };
        match self
            .store
            .put_opts(&path, PutPayload::from(bytes), options)
            .await
        {
            Ok(_) => Ok(next),
            Err(ObjectStoreError::Precondition { .. })
            | Err(ObjectStoreError::NotModified { .. })
            | Err(ObjectStoreError::NotFound { .. }) => Err(self
                .conflict_with_current(build_id, Some(expected_revision))
                .await?),
            Err(ObjectStoreError::NotImplemented { .. })
            | Err(ObjectStoreError::NotSupported { .. }) => {
                Err(AppBuildStoreError::ConditionalWriteUnsupported {
                    operation: "compare-and-swap update",
                })
            }
            Err(error) => Err(storage_error("update", error)),
        }
    }

    async fn create_local(
        &self,
        build_id: &str,
        record: Value,
        bytes: Vec<u8>,
    ) -> Result<Value, AppBuildStoreError> {
        let _lock = self.acquire_local_lock(build_id).await?;
        if let Some(current) = self.read_with_meta(build_id).await? {
            return Err(AppBuildStoreError::Conflict {
                expected_revision: None,
                current_revision: Some(current.revision),
                current: Some(current.record),
            });
        }
        self.put_local_overwrite(build_id, bytes, "create").await?;
        Ok(record)
    }

    async fn compare_and_swap_local(
        &self,
        build_id: &str,
        expected_revision: u64,
        next: Value,
    ) -> Result<Value, AppBuildStoreError> {
        let bytes = serialize_record(&next)?;
        let _lock = self.acquire_local_lock(build_id).await?;
        let Some(current) = self.read_with_meta(build_id).await? else {
            return Err(AppBuildStoreError::Conflict {
                expected_revision: Some(expected_revision),
                current_revision: None,
                current: None,
            });
        };
        if current.revision != expected_revision {
            return Err(AppBuildStoreError::Conflict {
                expected_revision: Some(expected_revision),
                current_revision: Some(current.revision),
                current: Some(current.record),
            });
        }
        self.put_local_overwrite(build_id, bytes, "update").await?;
        Ok(next)
    }

    async fn put_local_overwrite(
        &self,
        build_id: &str,
        bytes: Vec<u8>,
        operation: &'static str,
    ) -> Result<(), AppBuildStoreError> {
        let options = PutOptions {
            mode: PutMode::Overwrite,
            ..Default::default()
        };
        self.store
            .put_opts(&self.path(build_id), PutPayload::from(bytes), options)
            .await
            .map(|_| ())
            .map_err(|error| storage_error(operation, error))
    }

    async fn acquire_local_lock(
        &self,
        build_id: &str,
    ) -> Result<std::fs::File, AppBuildStoreError> {
        let local_store = self
            .local_store
            .as_ref()
            .expect("local lock acquisition requires a local store");
        local_lock::acquire(local_store, &self.lock_path(build_id)).await
    }

    fn path(&self, build_id: &str) -> Path {
        Path::from("apps")
            .join(self.app_id.as_str())
            .join("flowpilot-builds")
            .join(format!("{build_id}.json"))
    }

    fn lock_path(&self, build_id: &str) -> Path {
        Path::from("apps")
            .join(self.app_id.as_str())
            .join("flowpilot-builds")
            .join(".locks")
            .join(format!("{build_id}.lock"))
    }

    async fn read_with_meta(
        &self,
        build_id: &str,
    ) -> Result<Option<StoredBuild>, AppBuildStoreError> {
        validate_id("build_id", build_id)?;
        let path = self.path(build_id);
        let result = match self.store.get(&path).await {
            Ok(result) => result,
            Err(ObjectStoreError::NotFound { .. }) => return Ok(None),
            Err(error) => return Err(storage_error("read", error)),
        };
        let meta = result.meta.clone();
        let actual = usize::try_from(meta.size).unwrap_or(usize::MAX);
        if actual > MAX_APP_BUILD_BYTES {
            return Err(AppBuildStoreError::TooLarge {
                actual,
                maximum: MAX_APP_BUILD_BYTES,
            });
        }
        let bytes = result
            .bytes()
            .await
            .map_err(|error| storage_error("read", error))?;
        if bytes.len() > MAX_APP_BUILD_BYTES {
            return Err(AppBuildStoreError::TooLarge {
                actual: bytes.len(),
                maximum: MAX_APP_BUILD_BYTES,
            });
        }
        let record: Value = serde_json::from_slice(&bytes).map_err(|error| {
            AppBuildStoreError::CorruptStoredRecord {
                path: path.to_string(),
                reason: format!("malformed JSON: {error}"),
            }
        })?;
        let revision = validate_record(&record, &self.app_id, build_id).map_err(|reason| {
            AppBuildStoreError::CorruptStoredRecord {
                path: path.to_string(),
                reason,
            }
        })?;

        Ok(Some(StoredBuild {
            record,
            revision,
            meta,
        }))
    }

    async fn conflict_with_current(
        &self,
        build_id: &str,
        expected_revision: Option<u64>,
    ) -> Result<AppBuildStoreError, AppBuildStoreError> {
        let current = self.read_with_meta(build_id).await?;
        Ok(AppBuildStoreError::Conflict {
            expected_revision,
            current_revision: current.as_ref().map(|stored| stored.revision),
            current: current.map(|stored| stored.record),
        })
    }
}

async fn await_local_operation(
    operation: flow_like_types::tokio::task::JoinHandle<Result<Value, AppBuildStoreError>>,
) -> Result<Value, AppBuildStoreError> {
    operation
        .await
        .map_err(|error| AppBuildStoreError::Storage {
            operation: "complete local transaction",
            source: error.to_string(),
        })?
}

fn validate_id(field: &'static str, value: &str) -> Result<(), AppBuildStoreError> {
    if value.is_empty() {
        return Err(AppBuildStoreError::InvalidId {
            field,
            reason: "must not be empty".to_string(),
        });
    }
    if value.len() > MAX_ID_BYTES {
        return Err(AppBuildStoreError::InvalidId {
            field,
            reason: format!("must not exceed {MAX_ID_BYTES} bytes"),
        });
    }
    if !value
        .bytes()
        .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_'))
    {
        return Err(AppBuildStoreError::InvalidId {
            field,
            reason: "may contain only ASCII letters, digits, '-' and '_'".to_string(),
        });
    }
    Ok(())
}

fn validate_record(record: &Value, app_id: &str, build_id: &str) -> Result<u64, String> {
    let object = record
        .as_object()
        .ok_or_else(|| "the envelope must be a JSON object".to_string())?;
    let schema_version = object
        .get("schema_version")
        .and_then(Value::as_u64)
        .ok_or_else(|| "schema_version must be a non-negative integer".to_string())?;
    if schema_version != APP_BUILD_SCHEMA_VERSION {
        return Err(format!(
            "unsupported schema_version {schema_version}; expected {APP_BUILD_SCHEMA_VERSION}"
        ));
    }
    let record_app_id = object
        .get("app_id")
        .and_then(Value::as_str)
        .ok_or_else(|| "app_id must be a string".to_string())?;
    if record_app_id != app_id {
        return Err(format!("app_id does not match the addressed app {app_id}"));
    }
    let record_build_id = object
        .get("build_id")
        .and_then(Value::as_str)
        .ok_or_else(|| "build_id must be a string".to_string())?;
    if record_build_id != build_id {
        return Err(format!(
            "build_id does not match the addressed build {build_id}"
        ));
    }
    object
        .get("revision")
        .and_then(Value::as_u64)
        .ok_or_else(|| "revision must be a non-negative integer".to_string())
}

fn serialize_record(record: &Value) -> Result<Vec<u8>, AppBuildStoreError> {
    let bytes = serde_json::to_vec(record).map_err(|error| AppBuildStoreError::InvalidRecord {
        reason: format!("could not serialize JSON: {error}"),
    })?;
    if bytes.len() > MAX_APP_BUILD_BYTES {
        return Err(AppBuildStoreError::TooLarge {
            actual: bytes.len(),
            maximum: MAX_APP_BUILD_BYTES,
        });
    }
    Ok(bytes)
}

fn storage_error(operation: &'static str, error: ObjectStoreError) -> AppBuildStoreError {
    AppBuildStoreError::Storage {
        operation,
        source: error.to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use flow_like_storage::object_store::{local::LocalFileSystem, memory::InMemory};
    use serde_json::json;

    fn record(app_id: &str, build_id: &str, revision: u64) -> Value {
        json!({
            "schema_version": APP_BUILD_SCHEMA_VERSION,
            "app_id": app_id,
            "build_id": build_id,
            "revision": revision,
            "phase": "provisioning"
        })
    }

    #[tokio::test]
    async fn create_read_and_compare_and_swap_preserve_the_envelope() {
        let store = AppBuildStore::new(Arc::new(InMemory::new()), "app_1").unwrap();
        let initial = record("app_1", "build_1", 0);
        assert_eq!(
            store.create("build_1", initial.clone()).await.unwrap(),
            initial
        );
        assert_eq!(store.read("build_1").await.unwrap(), Some(initial));

        let next = record("app_1", "build_1", 1);
        assert_eq!(
            store
                .compare_and_swap("build_1", 0, next.clone())
                .await
                .unwrap(),
            next
        );
        assert_eq!(store.read("build_1").await.unwrap(), Some(next));
    }

    #[tokio::test]
    async fn stale_updates_return_the_current_revision_and_state() {
        let store = AppBuildStore::new(Arc::new(InMemory::new()), "app_1").unwrap();
        store
            .create("build_1", record("app_1", "build_1", 0))
            .await
            .unwrap();
        let current = record("app_1", "build_1", 1);
        store
            .compare_and_swap("build_1", 0, current.clone())
            .await
            .unwrap();

        let error = store
            .compare_and_swap("build_1", 0, record("app_1", "build_1", 1))
            .await
            .unwrap_err();
        assert_eq!(
            error,
            AppBuildStoreError::Conflict {
                expected_revision: Some(0),
                current_revision: Some(1),
                current: Some(current),
            }
        );
    }

    #[tokio::test]
    async fn local_filesystem_create_compare_and_swap_and_stale_update_are_serialized() {
        let directory = tempfile::tempdir().unwrap();
        let local = Arc::new(LocalObjectStore::new(directory.path().to_path_buf()).unwrap());
        let store = AppBuildStore::new_local(local, "app_1").unwrap();
        let initial = record("app_1", "build_1", 0);
        store.create("build_1", initial).await.unwrap();

        let current = record("app_1", "build_1", 1);
        store
            .compare_and_swap("build_1", 0, current.clone())
            .await
            .unwrap();
        let error = store
            .compare_and_swap("build_1", 0, record("app_1", "build_1", 1))
            .await
            .unwrap_err();

        assert_eq!(
            error,
            AppBuildStoreError::Conflict {
                expected_revision: Some(0),
                current_revision: Some(1),
                current: Some(current.clone()),
            }
        );
        assert_eq!(store.read("build_1").await.unwrap(), Some(current));
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 4)]
    async fn concurrent_local_writers_have_exactly_one_winner() {
        let directory = tempfile::tempdir().unwrap();
        let local = Arc::new(LocalObjectStore::new(directory.path().to_path_buf()).unwrap());
        let store = AppBuildStore::new_local(local, "app_1").unwrap();
        store
            .create("build_1", record("app_1", "build_1", 0))
            .await
            .unwrap();

        let barrier = Arc::new(tokio::sync::Barrier::new(3));
        let writer = |phase: &'static str| {
            let store = store.clone();
            let barrier = barrier.clone();
            tokio::spawn(async move {
                let mut next = record("app_1", "build_1", 1);
                next["phase"] = Value::String(phase.to_string());
                barrier.wait().await;
                store.compare_and_swap("build_1", 0, next).await
            })
        };
        let first = writer("first");
        let second = writer("second");
        barrier.wait().await;
        let first = first.await.unwrap();
        let second = second.await.unwrap();

        assert_eq!(
            [first.is_ok(), second.is_ok()]
                .into_iter()
                .filter(|won| *won)
                .count(),
            1
        );
        let loser = first.err().or_else(|| second.err()).unwrap();
        assert!(matches!(
            loser,
            AppBuildStoreError::Conflict {
                expected_revision: Some(0),
                current_revision: Some(1),
                ..
            }
        ));
        assert_eq!(
            store.read("build_1").await.unwrap().unwrap()["revision"],
            json!(1)
        );
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn cancelling_local_caller_does_not_cancel_the_locked_update() {
        let directory = tempfile::tempdir().unwrap();
        let local = Arc::new(LocalObjectStore::new(directory.path().to_path_buf()).unwrap());
        let store = AppBuildStore::new_local(local, "app_1").unwrap();
        store
            .create("build_1", record("app_1", "build_1", 0))
            .await
            .unwrap();

        let held_lock = store.acquire_local_lock("build_1").await.unwrap();
        {
            let update = store.compare_and_swap("build_1", 0, record("app_1", "build_1", 1));
            tokio::pin!(update);
            assert!(futures::poll!(update.as_mut()).is_pending());
        }
        drop(held_lock);

        tokio::time::timeout(std::time::Duration::from_secs(2), async {
            loop {
                let current = store.read("build_1").await.unwrap().unwrap();
                if current["revision"] == json!(1) {
                    break;
                }
                tokio::time::sleep(std::time::Duration::from_millis(10)).await;
            }
        })
        .await
        .expect("detached local update did not finish after its caller was cancelled");
    }

    #[tokio::test]
    async fn generic_local_object_store_create_and_update_fail_closed() {
        let directory = tempfile::tempdir().unwrap();
        let raw = Arc::new(LocalObjectStore::new(directory.path().to_path_buf()).unwrap());
        let store = AppBuildStore::new(raw.clone(), "app_1").unwrap();
        let initial = record("app_1", "build_1", 0);
        assert!(matches!(
            store.create("build_1", initial.clone()).await,
            Err(AppBuildStoreError::ConditionalWriteUnsupported {
                operation: "create"
            })
        ));
        assert_eq!(store.read("build_1").await.unwrap(), None);
        raw.put(
            &store.path("build_1"),
            PutPayload::from(serialize_record(&initial).unwrap()),
        )
        .await
        .unwrap();

        let error = store
            .compare_and_swap("build_1", 0, record("app_1", "build_1", 1))
            .await
            .unwrap_err();

        assert!(matches!(
            error,
            AppBuildStoreError::ConditionalWriteUnsupported {
                operation: "compare-and-swap update"
            }
        ));
        assert_eq!(store.read("build_1").await.unwrap(), Some(initial));
    }

    #[tokio::test]
    async fn generic_raw_local_filesystem_create_and_update_fail_closed() {
        let directory = tempfile::tempdir().unwrap();
        let raw = Arc::new(LocalFileSystem::new_with_prefix(directory.path()).unwrap());
        let store = AppBuildStore::new(raw.clone(), "app_1").unwrap();
        let initial = record("app_1", "build_1", 0);
        assert!(matches!(
            store.create("build_1", initial.clone()).await,
            Err(AppBuildStoreError::ConditionalWriteUnsupported {
                operation: "create"
            })
        ));
        assert_eq!(store.read("build_1").await.unwrap(), None);
        raw.put(
            &store.path("build_1"),
            PutPayload::from(serialize_record(&initial).unwrap()),
        )
        .await
        .unwrap();

        let error = store
            .compare_and_swap("build_1", 0, record("app_1", "build_1", 1))
            .await
            .unwrap_err();

        assert!(matches!(
            error,
            AppBuildStoreError::ConditionalWriteUnsupported {
                operation: "compare-and-swap update"
            }
        ));
        assert_eq!(store.read("build_1").await.unwrap(), Some(initial));
    }

    #[tokio::test]
    async fn duplicate_create_returns_the_current_state_without_overwriting_it() {
        let store = AppBuildStore::new(Arc::new(InMemory::new()), "app_1").unwrap();
        let current = record("app_1", "build_1", 0);
        store.create("build_1", current.clone()).await.unwrap();

        let mut replacement = record("app_1", "build_1", 0);
        replacement["phase"] = Value::String("different".to_string());
        let error = store.create("build_1", replacement).await.unwrap_err();

        assert_eq!(
            error,
            AppBuildStoreError::Conflict {
                expected_revision: None,
                current_revision: Some(0),
                current: Some(current.clone()),
            }
        );
        assert_eq!(store.read("build_1").await.unwrap(), Some(current));
    }

    #[tokio::test]
    async fn create_rejects_cross_app_and_cross_build_records() {
        let store = AppBuildStore::new(Arc::new(InMemory::new()), "app_1").unwrap();
        for invalid in [record("app_2", "build_1", 0), record("app_1", "build_2", 0)] {
            assert!(matches!(
                store.create("build_1", invalid).await,
                Err(AppBuildStoreError::InvalidRecord { .. })
            ));
        }
        assert_eq!(store.read("build_1").await.unwrap(), None);
    }

    #[tokio::test]
    async fn malformed_stored_json_is_an_error_not_a_missing_build() {
        let raw: Arc<dyn ObjectStore> = Arc::new(InMemory::new());
        let store = AppBuildStore::new(raw.clone(), "app_1").unwrap();
        raw.put(
            &store.path("build_1"),
            PutPayload::from_static(b"{not-json"),
        )
        .await
        .unwrap();

        assert!(matches!(
            store.read("build_1").await,
            Err(AppBuildStoreError::CorruptStoredRecord { .. })
        ));
    }

    #[tokio::test]
    async fn oversized_records_are_rejected_before_writing() {
        let store = AppBuildStore::new(Arc::new(InMemory::new()), "app_1").unwrap();
        let mut value = record("app_1", "build_1", 0);
        value["payload"] = Value::String("x".repeat(MAX_APP_BUILD_BYTES));
        assert!(matches!(
            store.create("build_1", value).await,
            Err(AppBuildStoreError::TooLarge { .. })
        ));
        assert_eq!(store.read("build_1").await.unwrap(), None);
    }

    #[test]
    fn path_ids_are_single_safe_segments() {
        for invalid in ["", "../other", "a/b", "a.b", "space here", "café"] {
            assert!(AppBuildStore::new(Arc::new(InMemory::new()), invalid).is_err());
        }
    }
}
