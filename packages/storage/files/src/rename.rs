use crate::decode_path_segment;
use futures::TryStreamExt;
use object_store::{ObjectMeta, ObjectStore, ObjectStoreExt, path::Path};
use std::{error::Error, fmt};

#[derive(Debug)]
pub enum RenameError {
    InvalidName,
    InvalidPath,
    NotFound,
    AlreadyExists,
    SourceChanged,
    Incomplete(Box<RenameError>),
    Storage(object_store::Error),
}

impl fmt::Display for RenameError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidName => {
                f.write_str("Enter a name without slashes, control characters, or '.' and '..'.")
            }
            Self::InvalidPath => f.write_str(
                "The storage root and paths outside this storage folder cannot be renamed.",
            ),
            Self::NotFound => f.write_str("The file or folder no longer exists."),
            Self::AlreadyExists => f.write_str("A file or folder with that name already exists."),
            Self::SourceChanged => {
                f.write_str("The file or folder changed during rename. Refresh and try again.")
            }
            Self::Incomplete(error) => write!(
                f,
                "Rename did not finish. Copied items may remain under the new name. Refresh before trying again. {error}"
            ),
            Self::Storage(_) => f.write_str("The storage provider could not complete the rename."),
        }
    }
}

impl Error for RenameError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::Storage(error) => Some(error),
            Self::Incomplete(error) => Some(error.as_ref()),
            _ => None,
        }
    }
}

impl From<object_store::Error> for RenameError {
    fn from(error: object_store::Error) -> Self {
        match error {
            object_store::Error::NotFound { .. } => Self::NotFound,
            object_store::Error::AlreadyExists { .. } => Self::AlreadyExists,
            other => Self::Storage(other),
        }
    }
}

/// Resolve a raw display name beside an existing, scoped object key.
pub fn rename_destination(base: &Path, source: &Path, new_name: &str) -> Result<Path, RenameError> {
    if new_name.trim().is_empty()
        || matches!(new_name, "." | "..")
        || new_name
            .chars()
            .any(|ch| matches!(ch, '/' | '\\') || ch.is_control())
    {
        return Err(RenameError::InvalidName);
    }
    let relative = source.prefix_match(base).ok_or(RenameError::InvalidPath)?;
    if source == base {
        return Err(RenameError::InvalidPath);
    }
    for part in relative {
        let decoded = decode_path_segment(part.as_ref());
        if matches!(decoded.as_ref(), "." | "..")
            || decoded
                .chars()
                .any(|ch| matches!(ch, '/' | '\\') || ch.is_control())
        {
            return Err(RenameError::InvalidPath);
        }
    }
    // A newly entered name is literal text, unlike a listed, encoded source key.
    Ok(source
        .parent()
        .ok_or(RenameError::InvalidPath)?
        .join(new_name))
}

async fn exact_object(
    store: &dyn ObjectStore,
    path: &Path,
) -> Result<Option<ObjectMeta>, RenameError> {
    match store.head(path).await {
        Ok(meta) => Ok(Some(meta)),
        Err(object_store::Error::NotFound { .. }) => Ok(None),
        Err(error) => Err(error.into()),
    }
}

async fn source_objects(
    store: &dyn ObjectStore,
    source: &Path,
    is_dir: bool,
) -> Result<Vec<ObjectMeta>, RenameError> {
    let mut objects = if is_dir {
        store.list(Some(source)).try_collect::<Vec<_>>().await?
    } else {
        Vec::new()
    };
    objects.retain(|meta| meta.location != *source && meta.location.prefix_matches(source));
    if !is_dir && let Some(meta) = exact_object(store, source).await? {
        objects.push(meta);
    }
    objects.sort_by(|left, right| left.location.cmp(&right.location));
    if objects.is_empty() {
        return Err(RenameError::NotFound);
    }
    Ok(objects)
}

async fn ensure_destination_absent(
    store: &dyn ObjectStore,
    destination: &Path,
) -> Result<(), RenameError> {
    if exact_object(store, destination).await?.is_some() {
        return Err(RenameError::AlreadyExists);
    }
    let mut objects = store.list(Some(destination));
    while let Some(meta) = objects.try_next().await? {
        if meta.location.prefix_matches(destination) {
            return Err(RenameError::AlreadyExists);
        }
    }
    Ok(())
}

fn same_revision(expected: &ObjectMeta, current: &ObjectMeta) -> bool {
    expected.location == current.location
        && expected.size == current.size
        // LIST may include fractional seconds that HTTP HEAD does not retain.
        && expected.last_modified.timestamp() == current.last_modified.timestamp()
        && expected.e_tag.as_ref().is_none_or(|tag| current.e_tag.as_ref() == Some(tag))
        // Some providers include a version in HEAD but omit it from LIST.
        && expected.version.as_ref().is_none_or(|version| current.version.as_ref() == Some(version))
}

/// Rename one object or a complete folder inside `base`, without overwriting.
/// All copies finish before any source is deleted. Multi-object renames are
/// not atomic and concurrent edits are unsupported. Errors retain destination
/// copies because unconditional rollback could delete another writer's changes.
pub async fn rename_storage_item(
    store: &dyn ObjectStore,
    base: &Path,
    source: &Path,
    new_name: &str,
    is_dir: bool,
) -> Result<Path, RenameError> {
    let destination = rename_destination(base, source, new_name)?;
    let objects = source_objects(store, source, is_dir).await?;
    if destination == *source {
        return Ok(destination);
    }
    ensure_destination_absent(store, &destination).await?;

    for (copied, object) in objects.iter().enumerate() {
        let mut target = destination.clone();
        target.extend(
            object
                .location
                .prefix_match(source)
                .ok_or(RenameError::InvalidPath)?,
        );
        if let Err(error) = store.copy_if_not_exists(&object.location, &target).await {
            let error = RenameError::from(error);
            return Err(if copied > 0 {
                RenameError::Incomplete(Box::new(error))
            } else {
                error
            });
        }
    }

    // Do not delete a source modified or added while its folder was copied.
    let current = match source_objects(store, source, is_dir).await {
        Ok(current) => current,
        Err(error) => {
            return Err(RenameError::Incomplete(Box::new(error)));
        }
    };
    if current != objects {
        return Err(RenameError::Incomplete(Box::new(
            RenameError::SourceChanged,
        )));
    }
    for object in objects {
        // Generic stores have no conditional delete. Recheck immediately before
        // each removal to narrow the remaining race with a concurrent writer.
        let current = exact_object(store, &object.location)
            .await
            .map_err(|error| RenameError::Incomplete(Box::new(error)))?;
        if !current
            .as_ref()
            .is_some_and(|current| same_revision(&object, current))
        {
            return Err(RenameError::Incomplete(Box::new(
                RenameError::SourceChanged,
            )));
        }
        store
            .delete(&object.location)
            .await
            .map_err(|error| RenameError::Incomplete(Box::new(error.into())))?;
    }
    Ok(destination)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::join_object_path;
    use async_trait::async_trait;
    use futures::stream::BoxStream;
    use object_store::{
        CopyOptions, GetOptions, GetResult, ListResult, MultipartUpload, PutMultipartOptions,
        PutOptions, PutPayload, PutResult, memory::InMemory,
    };
    use std::sync::atomic::{AtomicUsize, Ordering};

    fn base() -> Path {
        Path::from("apps/app/upload")
    }

    fn path(relative: &str) -> Path {
        join_object_path(&base(), relative)
    }

    async fn put(store: &dyn ObjectStore, relative: &str, body: &'static str) {
        store.put(&path(relative), body.into()).await.unwrap();
    }

    async fn contents(store: &dyn ObjectStore, relative: &str) -> String {
        String::from_utf8(
            store
                .get(&path(relative))
                .await
                .unwrap()
                .bytes()
                .await
                .unwrap()
                .to_vec(),
        )
        .unwrap()
    }

    #[test]
    fn rejects_root_outside_paths_and_traversal() {
        for source in [
            base(),
            Path::from("apps/other/upload/file"),
            join_object_path(&base(), "../file"),
            join_object_path(&base(), "%2E%2E/file"),
        ] {
            assert!(matches!(
                rename_destination(&base(), &source, "new"),
                Err(RenameError::InvalidPath)
            ));
        }
        for name in ["", " ", ".", "..", "a/b", "a\\b", "a\0b", "a\nb"] {
            assert!(matches!(
                rename_destination(&base(), &path("file"), name),
                Err(RenameError::InvalidName)
            ));
        }
    }

    #[test]
    fn preserves_encoded_source_parent_and_literal_new_name() {
        let source = path("Übersicht #1").join("old.txt");
        assert_eq!(
            rename_destination(&base(), &source, "100%20 Ü.txt").unwrap(),
            path("Übersicht #1").join("100%20 Ü.txt"),
        );
    }

    #[test]
    fn revision_checks_accept_head_precision_and_additional_version_metadata() {
        let listed = ObjectMeta {
            location: path("old/a"),
            size: 10,
            last_modified: "2026-10-12T12:00:00.123Z".parse().unwrap(),
            e_tag: Some("original".into()),
            version: None,
        };
        let mut head = listed.clone();
        head.last_modified = "2026-10-12T12:00:00Z".parse().unwrap();
        head.version = Some("provider-version".into());
        assert!(same_revision(&listed, &head));
        head.e_tag = Some("changed".into());
        assert!(!same_revision(&listed, &head));
    }

    #[tokio::test]
    async fn renames_only_the_selected_file() {
        let store = InMemory::new();
        put(&store, "notes.txt", "file").await;
        put(&store, "notes.txt/child", "folder").await;
        rename_storage_item(&store, &base(), &path("notes.txt"), "new.txt", false)
            .await
            .unwrap();
        assert_eq!(contents(&store, "new.txt").await, "file");
        assert_eq!(contents(&store, "notes.txt/child").await, "folder");
        assert!(store.head(&path("notes.txt")).await.is_err());
    }

    #[tokio::test]
    async fn folder_rename_keeps_exact_file_and_similar_prefixes() {
        let store = InMemory::new();
        put(&store, "docs", "same-name file").await;
        put(&store, "docs/a", "a").await;
        put(&store, "docs/nested/Ü #1.txt", "b").await;
        put(&store, "docs-old/c", "c").await;
        put(&store, "archive-old/d", "d").await;
        rename_storage_item(&store, &base(), &path("docs"), "archive", true)
            .await
            .unwrap();
        assert_eq!(contents(&store, "docs").await, "same-name file");
        assert_eq!(contents(&store, "archive/a").await, "a");
        assert_eq!(contents(&store, "archive/nested/Ü #1.txt").await, "b");
        assert_eq!(contents(&store, "docs-old/c").await, "c");
        assert_eq!(contents(&store, "archive-old/d").await, "d");
        assert!(store.head(&path("docs/a")).await.is_err());
    }

    #[tokio::test]
    async fn collisions_and_missing_sources_do_not_change_storage() {
        for is_dir in [false, true] {
            for existing in ["taken", "taken/child"] {
                let store = InMemory::new();
                let source = if is_dir { "old/child" } else { "old" };
                put(&store, source, "original").await;
                put(&store, existing, "existing").await;
                assert!(matches!(
                    rename_storage_item(&store, &base(), &path("old"), "taken", is_dir).await,
                    Err(RenameError::AlreadyExists)
                ));
                assert_eq!(contents(&store, source).await, "original");
                assert_eq!(contents(&store, existing).await, "existing");
                assert!(matches!(
                    rename_storage_item(&store, &base(), &path("missing"), "new", is_dir).await,
                    Err(RenameError::NotFound)
                ));
            }
        }
    }

    #[derive(Debug, Default)]
    struct InterruptedStore {
        inner: InMemory,
        copies: AtomicUsize,
        fail_copy: Option<usize>,
        change_source: Option<Path>,
        overwrite_target_on_failure: Option<Path>,
    }

    impl fmt::Display for InterruptedStore {
        fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
            f.write_str("InterruptedStore")
        }
    }

    #[async_trait]
    impl ObjectStore for InterruptedStore {
        async fn put_opts(
            &self,
            location: &Path,
            payload: PutPayload,
            opts: PutOptions,
        ) -> object_store::Result<PutResult> {
            self.inner.put_opts(location, payload, opts).await
        }

        async fn put_multipart_opts(
            &self,
            location: &Path,
            opts: PutMultipartOptions,
        ) -> object_store::Result<Box<dyn MultipartUpload>> {
            self.inner.put_multipart_opts(location, opts).await
        }

        async fn get_opts(
            &self,
            location: &Path,
            opts: GetOptions,
        ) -> object_store::Result<GetResult> {
            self.inner.get_opts(location, opts).await
        }

        fn delete_stream(
            &self,
            locations: BoxStream<'static, object_store::Result<Path>>,
        ) -> BoxStream<'static, object_store::Result<Path>> {
            self.inner.delete_stream(locations)
        }

        fn list(
            &self,
            prefix: Option<&Path>,
        ) -> BoxStream<'static, object_store::Result<ObjectMeta>> {
            self.inner.list(prefix)
        }

        async fn list_with_delimiter(
            &self,
            prefix: Option<&Path>,
        ) -> object_store::Result<ListResult> {
            self.inner.list_with_delimiter(prefix).await
        }

        async fn copy_opts(
            &self,
            from: &Path,
            to: &Path,
            options: CopyOptions,
        ) -> object_store::Result<()> {
            let number = self.copies.fetch_add(1, Ordering::SeqCst) + 1;
            if self.fail_copy == Some(number) {
                if let Some(path) = &self.overwrite_target_on_failure {
                    self.inner
                        .put(path, "concurrent target update".into())
                        .await?;
                }
                return Err(object_store::Error::Generic {
                    store: "InterruptedStore",
                    source: "copy interrupted".into(),
                });
            }
            self.inner.copy_opts(from, to, options).await?;
            if let Some(path) = &self.change_source {
                self.inner.put(path, "concurrent update".into()).await?;
            }
            Ok(())
        }
    }

    #[tokio::test]
    async fn a_later_copy_failure_keeps_every_source_and_partial_copies() {
        let store = InterruptedStore {
            fail_copy: Some(2),
            ..Default::default()
        };
        put(&store, "old/a", "a").await;
        put(&store, "old/b", "b").await;
        assert!(matches!(
            rename_storage_item(&store, &base(), &path("old"), "new", true).await,
            Err(RenameError::Incomplete(_))
        ));
        assert_eq!(contents(&store, "old/a").await, "a");
        assert_eq!(contents(&store, "old/b").await, "b");
        assert_eq!(contents(&store, "new/a").await, "a");
        assert!(store.head(&path("new/b")).await.is_err());
    }

    #[tokio::test]
    async fn concurrent_source_updates_are_preserved() {
        let store = InterruptedStore {
            change_source: Some(path("old")),
            ..Default::default()
        };
        put(&store, "old", "original").await;
        let error = rename_storage_item(&store, &base(), &path("old"), "new", false)
            .await
            .unwrap_err();
        assert!(
            matches!(error, RenameError::Incomplete(cause) if matches!(*cause, RenameError::SourceChanged))
        );
        assert_eq!(contents(&store, "old").await, "concurrent update");
        assert_eq!(contents(&store, "new").await, "original");
    }

    #[tokio::test]
    async fn later_failures_do_not_remove_another_writers_destination_update() {
        let store = InterruptedStore {
            fail_copy: Some(2),
            overwrite_target_on_failure: Some(path("new/a")),
            ..Default::default()
        };
        put(&store, "old/a", "a").await;
        put(&store, "old/b", "b").await;
        assert!(matches!(
            rename_storage_item(&store, &base(), &path("old"), "new", true).await,
            Err(RenameError::Incomplete(_))
        ));
        assert_eq!(contents(&store, "old/a").await, "a");
        assert_eq!(contents(&store, "old/b").await, "b");
        assert_eq!(contents(&store, "new/a").await, "concurrent target update");
    }
}
