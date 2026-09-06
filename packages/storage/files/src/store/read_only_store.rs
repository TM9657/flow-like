//! Read-only [`ObjectStore`] decorator for shadow/replay runs.
//!
//! Every mutating operation fails loudly with a `NotSupported` error instead of
//! silently succeeding. A fabricated success would hide changes in a shadow-run diff.
//! All read operations delegate to the wrapped store unchanged.

use async_trait::async_trait;
use bytes::Bytes;
use futures::stream::BoxStream;
use object_store::path::Path;
use object_store::{
    CopyOptions, GetOptions, GetResult, ListResult, MultipartUpload, ObjectMeta, ObjectStore,
    PutMultipartOptions, PutOptions, PutPayload, PutResult, RenameOptions, Result,
};
use std::ops::Range;
use std::sync::Arc;

#[derive(Debug)]
pub struct ReadOnlyStore {
    inner: Arc<dyn ObjectStore>,
}

impl ReadOnlyStore {
    pub fn new(inner: Arc<dyn ObjectStore>) -> Self {
        Self { inner }
    }

    fn write_denied(operation: &str, location: &str) -> object_store::Error {
        object_store::Error::NotSupported {
            source: format!(
                "shadow runs cannot write app storage: {operation} on '{location}' was blocked by the read-only store"
            )
            .into(),
        }
    }
}

impl std::fmt::Display for ReadOnlyStore {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "ReadOnlyStore({})", self.inner)
    }
}

#[async_trait]
impl ObjectStore for ReadOnlyStore {
    async fn put_opts(
        &self,
        location: &Path,
        _payload: PutPayload,
        _opts: PutOptions,
    ) -> Result<PutResult> {
        Err(Self::write_denied("put", location.as_ref()))
    }

    async fn put_multipart_opts(
        &self,
        location: &Path,
        _opts: PutMultipartOptions,
    ) -> Result<Box<dyn MultipartUpload>> {
        Err(Self::write_denied("multipart upload", location.as_ref()))
    }

    async fn get_opts(&self, location: &Path, opts: GetOptions) -> Result<GetResult> {
        self.inner.get_opts(location, opts).await
    }

    async fn get_ranges(&self, location: &Path, ranges: &[Range<u64>]) -> Result<Vec<Bytes>> {
        self.inner.get_ranges(location, ranges).await
    }

    fn delete_stream(
        &self,
        locations: BoxStream<'static, Result<Path>>,
    ) -> BoxStream<'static, Result<Path>> {
        Box::pin(futures::StreamExt::map(locations, |location| {
            let location = location?;
            Err(Self::write_denied("delete", location.as_ref()))
        }))
    }

    fn list(&self, prefix: Option<&Path>) -> BoxStream<'static, Result<ObjectMeta>> {
        self.inner.list(prefix)
    }

    fn list_with_offset(
        &self,
        prefix: Option<&Path>,
        offset: &Path,
    ) -> BoxStream<'static, Result<ObjectMeta>> {
        self.inner.list_with_offset(prefix, offset)
    }

    async fn list_with_delimiter(&self, prefix: Option<&Path>) -> Result<ListResult> {
        self.inner.list_with_delimiter(prefix).await
    }

    async fn copy_opts(&self, from: &Path, to: &Path, _options: CopyOptions) -> Result<()> {
        Err(Self::write_denied(
            "copy",
            &format!("{from} -> {to}", from = from.as_ref(), to = to.as_ref()),
        ))
    }

    async fn rename_opts(&self, from: &Path, to: &Path, _options: RenameOptions) -> Result<()> {
        Err(Self::write_denied(
            "rename",
            &format!("{from} -> {to}", from = from.as_ref(), to = to.as_ref()),
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use futures::TryStreamExt;
    use object_store::{ObjectStoreExt, memory::InMemory};

    fn store_with_object() -> (Arc<InMemory>, ReadOnlyStore) {
        let inner = Arc::new(InMemory::new());
        let read_only = ReadOnlyStore::new(inner.clone());
        (inner, read_only)
    }

    #[tokio::test]
    async fn every_write_operation_fails_loudly() {
        let (inner, store) = store_with_object();
        let path = Path::from("apps/app-1/file.txt");
        inner
            .put(&path, PutPayload::from_static(b"live"))
            .await
            .expect("seed inner store");

        let put = store.put(&path, PutPayload::from_static(b"x")).await;
        assert!(
            put.as_ref().is_err_and(|e| e
                .to_string()
                .contains("shadow runs cannot write app storage")),
            "put must be denied with the shadow message, got {put:?}"
        );
        assert!(store.put_multipart(&path).await.is_err());
        assert!(store.delete(&path).await.is_err());
        assert!(store.copy(&path, &Path::from("copy.txt")).await.is_err());
        assert!(store.rename(&path, &Path::from("moved.txt")).await.is_err());
        assert!(
            store
                .copy_if_not_exists(&path, &Path::from("copy.txt"))
                .await
                .is_err()
        );
        assert!(
            store
                .rename_if_not_exists(&path, &Path::from("moved.txt"))
                .await
                .is_err()
        );

        let streamed: Vec<_> = store
            .delete_stream(Box::pin(futures::stream::iter([Ok(path.clone())])))
            .try_collect::<Vec<_>>()
            .await
            .err()
            .into_iter()
            .collect();
        assert!(!streamed.is_empty(), "delete_stream must error per path");

        // The inner object survived every attempt untouched.
        let bytes = inner.get(&path).await.unwrap().bytes().await.unwrap();
        assert_eq!(bytes.as_ref(), b"live");
    }

    #[tokio::test]
    async fn reads_delegate_to_the_inner_store() {
        let (inner, store) = store_with_object();
        let path = Path::from("apps/app-1/file.txt");
        inner
            .put(&path, PutPayload::from_static(b"live"))
            .await
            .expect("seed inner store");

        let bytes = store.get(&path).await.unwrap().bytes().await.unwrap();
        assert_eq!(bytes.as_ref(), b"live");
        assert_eq!(store.head(&path).await.unwrap().size, 4);
        let listed: Vec<_> = store.list(None).try_collect().await.unwrap();
        assert_eq!(listed.len(), 1);
        assert_eq!(listed[0].location, path);
    }
}
