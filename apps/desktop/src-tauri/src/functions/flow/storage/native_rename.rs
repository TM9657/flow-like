use std::{fs, io, path::Path};

/// Rename within the scoped storage directory without replacing an existing item.
pub(super) fn rename_local_item(
    base: &Path,
    source: &Path,
    destination: &Path,
    is_dir: bool,
) -> io::Result<()> {
    let base = base.canonicalize()?;
    let parent = source
        .parent()
        .ok_or_else(|| io::Error::other("The storage root cannot be renamed"))?
        .canonicalize()?;
    let destination_parent = destination
        .parent()
        .ok_or_else(|| io::Error::other("A destination folder is required"))?
        .canonicalize()?;
    if !parent.starts_with(&base) || parent != destination_parent {
        return Err(io::Error::new(
            io::ErrorKind::PermissionDenied,
            "The item must stay in its storage folder",
        ));
    }
    let metadata = fs::symlink_metadata(source)?;
    if metadata.file_type().is_symlink()
        || metadata.is_dir() != is_dir
        || (!is_dir && !metadata.is_file())
    {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "The item type changed; refresh the folder and try again",
        ));
    }
    let resolved_source = source.canonicalize()?;
    if resolved_source == base || !resolved_source.starts_with(&base) {
        return Err(io::Error::new(
            io::ErrorKind::PermissionDenied,
            "The item is outside its storage folder",
        ));
    }
    let source_name = source
        .file_name()
        .ok_or_else(|| io::Error::other("A source name is required"))?;
    let destination_name = destination
        .file_name()
        .ok_or_else(|| io::Error::other("A destination name is required"))?;
    if source_name == destination_name {
        return Ok(());
    }

    // Native directory renames preserve empty descendants and metadata. The OS
    // rejects an occupied destination even if it appears after our validation.
    #[cfg(unix)]
    {
        let directory = fs::File::open(&parent)?;
        rustix::fs::renameat_with(
            &directory,
            source_name,
            &directory,
            destination_name,
            rustix::fs::RenameFlags::NOREPLACE,
        )?;
        Ok(())
    }
    #[cfg(windows)]
    {
        // MoveFileExW without MOVEFILE_REPLACE_EXISTING rejects existing targets.
        use std::os::windows::ffi::OsStrExt;
        use windows::Win32::Storage::FileSystem::{MOVE_FILE_FLAGS, MoveFileExW};
        use windows::core::PCWSTR;

        let source: Vec<u16> = parent
            .join(source_name)
            .as_os_str()
            .encode_wide()
            .chain(Some(0))
            .collect();
        let destination: Vec<u16> = parent
            .join(destination_name)
            .as_os_str()
            .encode_wide()
            .chain(Some(0))
            .collect();
        // Both paths remain valid, null-terminated UTF-16 throughout the call.
        unsafe {
            MoveFileExW(
                PCWSTR(source.as_ptr()),
                PCWSTR(destination.as_ptr()),
                MOVE_FILE_FLAGS(0),
            )
        }
        .map_err(|_| io::Error::last_os_error())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;
    use std::time::{SystemTime, UNIX_EPOCH};

    struct StorageDirectory(PathBuf);

    impl StorageDirectory {
        fn new() -> Self {
            let nanos = SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_nanos();
            let path = std::env::temp_dir().join(format!(
                "flow-like-storage-rename-{}-{nanos}",
                std::process::id()
            ));
            fs::create_dir_all(&path).unwrap();
            Self(path)
        }
    }

    impl Drop for StorageDirectory {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.0);
        }
    }

    #[test]
    fn renames_files_and_preserves_bytes() {
        let root = StorageDirectory::new();
        let source = root.0.join("old.txt");
        let destination = root.0.join("new ünicode 100%.txt");
        fs::write(&source, b"keep these bytes").unwrap();
        rename_local_item(&root.0, &source, &destination, false).unwrap();
        assert!(!source.exists());
        assert_eq!(fs::read(destination).unwrap(), b"keep these bytes");
    }

    #[test]
    fn renames_folders_with_files_and_empty_descendants() {
        let root = StorageDirectory::new();
        let source = root.0.join("old");
        let destination = root.0.join("new");
        fs::create_dir_all(source.join("nested/empty")).unwrap();
        fs::write(source.join("nested/data.txt"), b"nested").unwrap();
        rename_local_item(&root.0, &source, &destination, true).unwrap();
        assert!(!source.exists());
        assert!(destination.join("nested/empty").is_dir());
        assert_eq!(
            fs::read(destination.join("nested/data.txt")).unwrap(),
            b"nested"
        );
    }

    #[test]
    fn renames_an_empty_folder() {
        let root = StorageDirectory::new();
        let source = root.0.join("old");
        let destination = root.0.join("new");
        fs::create_dir(&source).unwrap();
        rename_local_item(&root.0, &source, &destination, true).unwrap();
        assert!(!source.exists());
        assert!(destination.is_dir());
    }

    #[test]
    fn rejects_existing_files_and_empty_folders_without_replacing_them() {
        for is_dir in [false, true] {
            let root = StorageDirectory::new();
            let source = root.0.join("old");
            let destination = root.0.join("existing");
            if is_dir {
                fs::create_dir(&source).unwrap();
                fs::create_dir(&destination).unwrap();
            } else {
                fs::write(&source, b"source").unwrap();
                fs::write(&destination, b"destination").unwrap();
            }
            let error = rename_local_item(&root.0, &source, &destination, is_dir).unwrap_err();
            assert_eq!(error.kind(), io::ErrorKind::AlreadyExists);
            assert!(source.exists());
            assert!(destination.exists());
            if !is_dir {
                assert_eq!(fs::read(source).unwrap(), b"source");
                assert_eq!(fs::read(destination).unwrap(), b"destination");
            }
        }
    }

    #[test]
    fn rejects_a_changed_item_type() {
        let root = StorageDirectory::new();
        let source = root.0.join("file");
        let destination = root.0.join("new");
        fs::write(&source, b"source").unwrap();
        assert_eq!(
            rename_local_item(&root.0, &source, &destination, true)
                .unwrap_err()
                .kind(),
            io::ErrorKind::InvalidInput
        );
        assert!(source.exists());
        assert!(!destination.exists());
    }

    #[test]
    fn rejects_the_storage_root_and_moves_between_folders() {
        let root = StorageDirectory::new();
        let sibling = root.0.with_extension("renamed");
        assert!(rename_local_item(&root.0, &root.0, &sibling, true).is_err());
        let source = root.0.join("file");
        fs::write(&source, b"source").unwrap();
        fs::create_dir(root.0.join("another")).unwrap();
        assert!(rename_local_item(&root.0, &source, &root.0.join("another/file"), false).is_err());
        assert!(source.exists());
    }

    #[cfg(unix)]
    #[test]
    fn rejects_symlinks_and_folders_that_escape_the_scoped_storage() {
        use std::os::unix::fs::symlink;

        let root = StorageDirectory::new();
        let outside = StorageDirectory::new();
        fs::write(outside.0.join("file"), b"outside").unwrap();
        symlink(&outside.0, root.0.join("linked")).unwrap();
        assert!(
            rename_local_item(
                &root.0,
                &root.0.join("linked/file"),
                &root.0.join("linked/new"),
                false,
            )
            .is_err()
        );
        symlink(outside.0.join("file"), root.0.join("link")).unwrap();
        assert!(
            rename_local_item(&root.0, &root.0.join("link"), &root.0.join("new"), false).is_err()
        );
        assert_eq!(fs::read(outside.0.join("file")).unwrap(), b"outside");
    }
}
