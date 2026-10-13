use super::*;
use async_trait::async_trait;
use bytes::Bytes;
use futures::stream::BoxStream;
use object_store::{
    CopyOptions, GetOptions, GetResult, ListResult, MultipartUpload, ObjectMeta, ObjectStore,
    ObjectStoreExt, PutMultipartOptions, PutOptions, UploadPart, memory::InMemory,
};
use std::{
    fmt::{Display, Formatter},
    sync::{Arc, Mutex},
};

#[derive(Clone, Copy, Debug, Default)]
enum Failure {
    #[default]
    None,
    Unsupported,
    NotImplemented,
    Denied,
    Part,
    Complete,
}

#[derive(Clone, Debug, Default)]
struct Stats {
    puts: usize,
    multipart_starts: usize,
    part_sizes: Vec<usize>,
    completes: usize,
    aborts: usize,
    active_parts: usize,
    max_active_parts: usize,
}

#[derive(Debug, Default)]
struct TrackingStore {
    inner: Arc<InMemory>,
    stats: Arc<Mutex<Stats>>,
    failure: Failure,
    fail_abort: bool,
}

impl Display for TrackingStore {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.write_str("TrackingStore")
    }
}

fn injected_error(message: &'static str) -> object_store::Error {
    object_store::Error::Generic {
        store: "upload test",
        source: message.into(),
    }
}

fn validate_frames(payload: &PutPayload) -> object_store::Result<()> {
    if payload.iter().any(|frame| frame.len() > UPLOAD_CHUNK_SIZE) {
        return Err(injected_error("transport frame exceeds 64 KiB"));
    }
    Ok(())
}

fn with_version(mut result: PutResult) -> PutResult {
    result.version = Some("uploaded-version".to_string());
    result
}

#[derive(Debug)]
struct TrackingUpload {
    inner: Box<dyn MultipartUpload>,
    stats: Arc<Mutex<Stats>>,
    failure: Failure,
    fail_abort: bool,
}

struct ActivePart(Arc<Mutex<Stats>>);

impl ActivePart {
    fn new(stats: Arc<Mutex<Stats>>) -> Self {
        {
            let mut state = stats.lock().unwrap();
            state.active_parts += 1;
            state.max_active_parts = state.max_active_parts.max(state.active_parts);
        }
        Self(stats)
    }
}

impl Drop for ActivePart {
    fn drop(&mut self) {
        self.0.lock().unwrap().active_parts -= 1;
    }
}

#[async_trait]
impl MultipartUpload for TrackingUpload {
    fn put_part(&mut self, payload: PutPayload) -> UploadPart {
        let index = {
            let mut stats = self.stats.lock().unwrap();
            stats.part_sizes.push(payload.content_length());
            stats.part_sizes.len() - 1
        };
        let validation = validate_frames(&payload);
        let pending = self.inner.put_part(payload);
        let stats = self.stats.clone();
        let fail = matches!(self.failure, Failure::Part) && index == 1;
        Box::pin(async move {
            let _active = ActivePart::new(stats);
            tokio::task::yield_now().await;
            validation?;
            if fail {
                return Err(injected_error("part failed"));
            }
            pending.await
        })
    }

    async fn complete(&mut self) -> object_store::Result<PutResult> {
        self.stats.lock().unwrap().completes += 1;
        if matches!(self.failure, Failure::Complete) {
            return Err(injected_error("completion failed"));
        }
        self.inner.complete().await.map(with_version)
    }

    async fn abort(&mut self) -> object_store::Result<()> {
        self.stats.lock().unwrap().aborts += 1;
        self.inner.abort().await?;
        if self.fail_abort {
            return Err(injected_error("abort failed"));
        }
        Ok(())
    }
}

#[async_trait]
impl ObjectStore for TrackingStore {
    async fn put_opts(
        &self,
        location: &Path,
        payload: PutPayload,
        options: PutOptions,
    ) -> object_store::Result<PutResult> {
        self.stats.lock().unwrap().puts += 1;
        validate_frames(&payload)?;
        self.inner
            .put_opts(location, payload, options)
            .await
            .map(with_version)
    }

    async fn put_multipart_opts(
        &self,
        location: &Path,
        options: PutMultipartOptions,
    ) -> object_store::Result<Box<dyn MultipartUpload>> {
        self.stats.lock().unwrap().multipart_starts += 1;
        match self.failure {
            Failure::Unsupported => {
                return Err(object_store::Error::NotSupported {
                    source: "multipart unavailable".into(),
                });
            }
            Failure::NotImplemented => {
                return Err(object_store::Error::NotImplemented {
                    operation: "put_multipart_opts".to_string(),
                    implementer: "HTTP test store".to_string(),
                });
            }
            Failure::Denied => {
                return Err(object_store::Error::PermissionDenied {
                    path: location.to_string(),
                    source: "multipart denied".into(),
                });
            }
            _ => {}
        }
        Ok(Box::new(TrackingUpload {
            inner: self.inner.put_multipart_opts(location, options).await?,
            stats: self.stats.clone(),
            failure: self.failure,
            fail_abort: self.fail_abort,
        }))
    }

    async fn get_opts(&self, path: &Path, options: GetOptions) -> object_store::Result<GetResult> {
        self.inner.get_opts(path, options).await
    }

    fn delete_stream(
        &self,
        locations: BoxStream<'static, object_store::Result<Path>>,
    ) -> BoxStream<'static, object_store::Result<Path>> {
        self.inner.delete_stream(locations)
    }

    fn list(&self, prefix: Option<&Path>) -> BoxStream<'static, object_store::Result<ObjectMeta>> {
        self.inner.list(prefix)
    }

    async fn list_with_delimiter(&self, prefix: Option<&Path>) -> object_store::Result<ListResult> {
        self.inner.list_with_delimiter(prefix).await
    }

    async fn copy_opts(
        &self,
        from: &Path,
        to: &Path,
        options: CopyOptions,
    ) -> object_store::Result<()> {
        self.inner.copy_opts(from, to, options).await
    }
}

fn bytes(len: usize) -> Bytes {
    (0..len).map(|index| (index % 251) as u8).collect()
}

async fn assert_saved(store: &dyn ObjectStore, path: &Path, expected: &Bytes, result: PutResult) {
    let stored = store.get(path).await.unwrap();
    assert_eq!(result.e_tag, stored.meta.e_tag);
    assert_eq!(result.version.as_deref(), Some("uploaded-version"));
    assert_eq!(stored.bytes().await.unwrap(), *expected);
}

#[tokio::test]
async fn small_and_reported_pdf_sizes_use_one_put_with_bounded_frames() {
    for size in [0, 1_400_000, 2_800_000, MULTIPART_PART_SIZE] {
        let store = TrackingStore::default();
        let path = Path::from("upload/report.pdf");
        let data = bytes(size);
        let result = put_payload(&store, &path, data.clone().into())
            .await
            .unwrap();
        assert_saved(&store, &path, &data, result).await;
        let stats = store.stats.lock().unwrap();
        assert_eq!(stats.puts, 1, "payload size {size}");
        assert_eq!(stats.multipart_starts, 0, "payload size {size}");
    }
}

#[tokio::test]
async fn multipart_preserves_uneven_input_chunks_and_limits_concurrency() {
    let store = TrackingStore::default();
    let path = Path::from("upload/large.bin");
    let data = bytes((MAX_CONCURRENT_PARTS + 1) * MULTIPART_PART_SIZE + 333);
    let boundaries = [
        0,
        13,
        MULTIPART_PART_SIZE / 2,
        MULTIPART_PART_SIZE + 5,
        data.len(),
    ];
    let payload = std::iter::once(Bytes::new())
        .chain(
            boundaries
                .windows(2)
                .map(|range| data.slice(range[0]..range[1])),
        )
        .chain(std::iter::once(Bytes::new()))
        .collect();
    let result = put_payload(&store, &path, payload).await.unwrap();
    assert_saved(&store, &path, &data, result).await;

    let stats = store.stats.lock().unwrap();
    let mut expected_sizes = vec![MULTIPART_PART_SIZE; MAX_CONCURRENT_PARTS + 1];
    expected_sizes.push(333);
    assert_eq!(stats.part_sizes, expected_sizes);
    assert_eq!(stats.puts, 0);
    assert_eq!(stats.multipart_starts, 1);
    assert_eq!(stats.completes, 1);
    assert_eq!(stats.aborts, 0);
    assert_eq!(stats.active_parts, 0);
    assert!(stats.max_active_parts <= MAX_CONCURRENT_PARTS);
}

#[tokio::test]
async fn just_above_threshold_uses_multipart() {
    let store = TrackingStore::default();
    let path = Path::from("threshold.bin");
    let data = bytes(MULTIPART_PART_SIZE + 1);
    let result = put_payload(&store, &path, data.clone().into())
        .await
        .unwrap();
    assert_saved(&store, &path, &data, result).await;
    let stats = store.stats.lock().unwrap();
    assert_eq!(stats.puts, 0);
    assert_eq!(stats.part_sizes, [MULTIPART_PART_SIZE, 1]);
}

#[tokio::test]
async fn unsupported_multipart_falls_back_to_chunked_put() {
    for failure in [Failure::Unsupported, Failure::NotImplemented] {
        let store = TrackingStore {
            failure,
            ..Default::default()
        };
        let path = Path::from("unsupported.bin");
        let data = bytes(MULTIPART_PART_SIZE + 17);
        let result = put_payload(&store, &path, data.clone().into())
            .await
            .unwrap();
        assert_saved(&store, &path, &data, result).await;
        let stats = store.stats.lock().unwrap();
        assert_eq!(stats.multipart_starts, 1);
        assert_eq!(stats.puts, 1);
        assert_eq!(stats.aborts, 0);
    }
}

#[tokio::test]
async fn multipart_permission_denial_does_not_fall_back() {
    let store = TrackingStore {
        failure: Failure::Denied,
        ..Default::default()
    };
    let path = Path::from("denied.bin");
    let error = put_payload(&store, &path, bytes(MULTIPART_PART_SIZE + 1).into())
        .await
        .unwrap_err();
    assert!(matches!(
        error,
        object_store::Error::PermissionDenied { .. }
    ));
    assert_eq!(store.stats.lock().unwrap().puts, 0);
    assert!(store.head(&path).await.is_err());
}

#[tokio::test]
async fn failures_abort_without_overwriting_the_original_error_or_object() {
    for (failure, message, completes) in [
        (Failure::Part, "part failed", 0),
        (Failure::Complete, "completion failed", 1),
    ] {
        let store = TrackingStore {
            failure,
            fail_abort: true,
            ..Default::default()
        };
        let path = Path::from("existing.bin");
        store.inner.put(&path, "original".into()).await.unwrap();
        let error = put_payload(&store, &path, bytes(MULTIPART_PART_SIZE + 1).into())
            .await
            .unwrap_err();
        assert!(error.to_string().contains(message), "{error}");
        assert_eq!(
            store.get(&path).await.unwrap().bytes().await.unwrap(),
            "original"
        );
        let stats = store.stats.lock().unwrap();
        assert_eq!(stats.aborts, 1);
        assert_eq!(stats.completes, completes);
        assert_eq!(stats.puts, 0);
        assert_eq!(stats.active_parts, 0);
    }
}

#[test]
#[cfg(target_pointer_width = "64")]
fn multipart_part_size_stays_aligned_and_within_the_part_limit() {
    for size in [
        0,
        MULTIPART_PART_SIZE,
        MULTIPART_PART_SIZE * MAX_UPLOAD_PARTS,
        MULTIPART_PART_SIZE * MAX_UPLOAD_PARTS + 1,
        MULTIPART_PART_SIZE * MAX_UPLOAD_PARTS * 2 + 123,
    ] {
        let part_size = multipart_part_size(size);
        assert!(part_size >= MULTIPART_PART_SIZE);
        assert_eq!(part_size % UPLOAD_CHUNK_SIZE, 0);
        assert!(size.div_ceil(part_size) <= MAX_UPLOAD_PARTS);
    }
}

#[tokio::test]
async fn local_uploads_preserve_bytes_and_replace_existing_contents() {
    let directory = tempfile::tempdir().unwrap();
    let store = crate::store::local_store::LocalObjectStore::new(directory.path().into()).unwrap();
    let path = Path::from("nested/report.pdf");
    for size in [MULTIPART_PART_SIZE + 17, 2_800_000, 0] {
        let data = bytes(size);
        let result = put_payload(&store, &path, data.clone().into())
            .await
            .unwrap();
        let stored = store.get(&path).await.unwrap();
        assert_eq!(result.e_tag, stored.meta.e_tag);
        assert_eq!(stored.bytes().await.unwrap(), data);
    }
}
