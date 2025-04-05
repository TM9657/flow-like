use flow_like_types::{Bytes, async_trait};
use futures::stream::BoxStream;
use object_store::local::LocalFileSystem;
use object_store::path::Path;
use object_store::{
    GetOptions, GetResult, ListResult, MultipartUpload, ObjectMeta, ObjectStore, PutMultipartOpts,
    PutOptions, PutPayload, PutResult, Result,
};
use std::fs;
use std::ops::Range;
use std::path::PathBuf;

#[derive(Debug)]
pub struct LocalObjectStore {
    store: LocalFileSystem,
}

impl std::fmt::Display for LocalObjectStore {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "LocalFileSystem({})", self.store)
    }
}

impl LocalObjectStore {
    pub fn new(prefix: PathBuf) -> Result<Self> {
        if !prefix.exists() {
            fs::create_dir_all(&prefix)
                .map(|_| ())
                .map_err(|_| object_store::Error::NotImplemented)?;
        }

        let store = LocalFileSystem::new_with_prefix(prefix)?.with_automatic_cleanup(true);
        Ok(Self { store })
    }

    pub fn path_to_filesystem(&self, location: &Path) -> Result<PathBuf> {
        let path = self.store.path_to_filesystem(location)?;
        Ok(path)
    }
}

#[async_trait]
impl ObjectStore for LocalObjectStore {
    async fn put(&self, location: &Path, payload: PutPayload) -> Result<PutResult> {
        let path = self.store.path_to_filesystem(location)?;
        if let Some(parent) = path.parent() {
            if !parent.exists() {
                fs::create_dir_all(parent)
                    .map(|_| ())
                    .map_err(|_| object_store::Error::NotImplemented)?;
            }
        }
        self.store.put(location, payload).await
    }

    async fn put_opts(
        &self,
        location: &Path,
        payload: PutPayload,
        opts: PutOptions,
    ) -> Result<PutResult> {
        let path = self.store.path_to_filesystem(location)?;
        if let Some(parent) = path.parent() {
            if !parent.exists() {
                fs::create_dir_all(parent)
                    .map(|_| ())
                    .map_err(|_| object_store::Error::NotImplemented)?;
            }
        }
        self.store.put_opts(location, payload, opts).await
    }

    async fn put_multipart(&self, location: &Path) -> Result<Box<dyn MultipartUpload>> {
        let path = self.store.path_to_filesystem(location)?;
        if let Some(parent) = path.parent() {
            if !parent.exists() {
                fs::create_dir_all(parent)
                    .map(|_| ())
                    .map_err(|_| object_store::Error::NotImplemented)?;
            }
        }
        self.store.put_multipart(location).await
    }

    async fn put_multipart_opts(
        &self,
        location: &Path,
        opts: PutMultipartOpts,
    ) -> Result<Box<dyn MultipartUpload>> {
        let path = self.store.path_to_filesystem(location)?;
        if let Some(parent) = path.parent() {
            if !parent.exists() {
                fs::create_dir_all(parent)
                    .map(|_| ())
                    .map_err(|_| object_store::Error::NotImplemented)?;
            }
        }
        self.store.put_multipart_opts(location, opts).await
    }

    async fn get(&self, location: &Path) -> Result<GetResult> {
        self.store.get(location).await
    }

    async fn get_opts(&self, location: &Path, opts: GetOptions) -> Result<GetResult> {
        self.store.get_opts(location, opts).await
    }

    async fn get_range(
        &self,
        location: &Path,
        range: Range<usize>,
    ) -> Result<flow_like_types::Bytes> {
        self.store.get_range(location, range).await
    }

    async fn get_ranges(&self, location: &Path, ranges: &[Range<usize>]) -> Result<Vec<Bytes>> {
        self.store.get_ranges(location, ranges).await
    }

    async fn head(&self, location: &Path) -> Result<ObjectMeta> {
        let path = self.store.path_to_filesystem(location)?;
        if let Some(parent) = path.parent() {
            if !parent.exists() {
                fs::create_dir_all(parent)
                    .map(|_| ())
                    .map_err(|_| object_store::Error::NotImplemented)?;
            }
        }
        self.store.head(location).await
    }

    async fn delete(&self, location: &Path) -> Result<()> {
        self.store.delete(location).await
    }

    fn delete_stream<'a>(
        &'a self,
        locations: BoxStream<'a, Result<Path>>,
    ) -> BoxStream<'a, Result<Path>> {
        self.store.delete_stream(locations)
    }

    fn list(&self, prefix: Option<&Path>) -> BoxStream<'_, Result<ObjectMeta>> {
        self.store.list(prefix)
    }

    fn list_with_offset(
        &self,
        prefix: Option<&Path>,
        offset: &Path,
    ) -> BoxStream<'_, Result<ObjectMeta>> {
        self.store.list_with_offset(prefix, offset)
    }

    async fn list_with_delimiter(&self, prefix: Option<&Path>) -> Result<ListResult> {
        self.store.list_with_delimiter(prefix).await
    }

    async fn copy(&self, from: &Path, to: &Path) -> Result<()> {
        self.store.copy(from, to).await
    }

    async fn rename(&self, from: &Path, to: &Path) -> Result<()> {
        self.store.rename(from, to).await
    }

    async fn copy_if_not_exists(&self, from: &Path, to: &Path) -> Result<()> {
        self.store.copy_if_not_exists(from, to).await
    }

    async fn rename_if_not_exists(&self, from: &Path, to: &Path) -> Result<()> {
        self.store.rename_if_not_exists(from, to).await
    }
}
