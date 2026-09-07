pub(super) fn write_settings_atomically(
    path: &std::path::Path,
    bytes: &[u8],
) -> std::io::Result<()> {
    use std::io::Write;

    let temporary = path.with_extension(format!("{}.tmp", uuid::Uuid::new_v4()));
    let result = (|| {
        let mut file = std::fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&temporary)?;
        file.write_all(bytes)?;
        file.sync_all()?;
        drop(file);
        std::fs::rename(&temporary, path)
    })();
    if result.is_err() {
        if let Err(error) = std::fs::remove_file(&temporary)
            && error.kind() != std::io::ErrorKind::NotFound
        {
            tracing::warn!(%error, "Could not remove a temporary settings file");
        }
    }
    result
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn replacing_settings_keeps_a_complete_file_and_cleans_temporary_files() {
        let directory = std::env::temp_dir().join(uuid::Uuid::new_v4().to_string());
        std::fs::create_dir(&directory).unwrap();
        let path = directory.join("settings.json");
        write_settings_atomically(&path, br#"{"home_layout":null}"#).unwrap();
        let customized = br#"{"home_layout":{"version":1,"widgets":[]}}"#;
        write_settings_atomically(&path, customized).unwrap();
        assert_eq!(std::fs::read(&path).unwrap(), customized);
        assert_eq!(std::fs::read_dir(&directory).unwrap().count(), 1);
        std::fs::remove_dir_all(directory).unwrap();
    }

    #[test]
    fn failed_replacement_reports_an_error_and_cleans_temporary_files() {
        let directory = std::env::temp_dir().join(uuid::Uuid::new_v4().to_string());
        let path = directory.join("settings.json");
        std::fs::create_dir_all(&path).unwrap();
        let existing = path.join("preserved");
        std::fs::write(&existing, b"previous data").unwrap();
        assert!(write_settings_atomically(&path, b"new data").is_err());
        assert_eq!(std::fs::read(existing).unwrap(), b"previous data");
        assert_eq!(std::fs::read_dir(&directory).unwrap().count(), 1);
        std::fs::remove_dir_all(directory).unwrap();
    }
}
