use serde::Deserialize;
use std::{
    fs::{self, OpenOptions},
    io::Write,
    path::{Path, PathBuf},
    sync::OnceLock,
};

const MARKER: &str = ".flowpilot-e2e-owner.json";
const IDENTIFIER: &str = "com.flow-like.e2e";
static DATA_ROOT: OnceLock<PathBuf> = OnceLock::new();

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Owner {
    schema: String,
    run_id: String,
    runner_pid: u32,
}

pub(crate) fn data_root() -> Option<PathBuf> {
    DATA_ROOT.get().cloned()
}

fn validate_root(root: &Path, temp: &Path, run_id: &str) -> anyhow::Result<()> {
    anyhow::ensure!(root.is_absolute(), "E2E data root must be absolute");
    let canonical = root.canonicalize()?;
    anyhow::ensure!(canonical == root, "E2E data root must be canonical");
    anyhow::ensure!(
        canonical.parent() == Some(temp.canonicalize()?.as_path()),
        "E2E data root must be a direct temporary-directory child"
    );
    anyhow::ensure!(
        root.file_name()
            .and_then(|name| name.to_str())
            .is_some_and(|name| name.starts_with("flow-like-flowpilot-e2e-")),
        "E2E data root must use the runner's temporary-directory prefix"
    );
    let root_metadata = fs::symlink_metadata(root)?;
    anyhow::ensure!(root_metadata.is_dir(), "E2E data root is not a directory");
    let marker_path = root.join(MARKER);
    let marker_metadata = fs::symlink_metadata(&marker_path)?;
    anyhow::ensure!(
        marker_metadata.is_file() && marker_metadata.len() <= 4096,
        "E2E ownership marker must be a small regular file"
    );
    #[cfg(unix)]
    {
        use std::os::unix::fs::{MetadataExt, PermissionsExt};
        anyhow::ensure!(
            root_metadata.permissions().mode() & 0o077 == 0
                && marker_metadata.permissions().mode() & 0o077 == 0
                && root_metadata.uid() == marker_metadata.uid(),
            "E2E data root and marker must have the same owner and private permissions"
        );
    }
    let owner: Owner = serde_json::from_slice(&fs::read(&marker_path)?)?;
    anyhow::ensure!(
        owner.schema == "flowpilot-e2e-isolation/v1"
            && !run_id.is_empty()
            && owner.run_id == run_id
            && owner.runner_pid != 0,
        "E2E ownership marker does not match this run"
    );
    for entry in fs::read_dir(root)? {
        anyhow::ensure!(
            entry?.file_name() == MARKER,
            "E2E data root must be fresh and contain only its ownership marker"
        );
    }
    Ok(())
}

fn claim_root(root: &Path) -> anyhow::Result<()> {
    let mut options = OpenOptions::new();
    options.write(true).create_new(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        options.mode(0o600);
    }
    let mut owner = options.open(root.join(".native-owner"))?;
    writeln!(owner, "{}", std::process::id())?;
    Ok(())
}

pub(crate) fn initialize(config: &tauri::utils::config::Config) -> anyhow::Result<()> {
    let Some(root) = std::env::var_os("FLOWPILOT_E2E_DATA_ROOT") else {
        return Ok(());
    };
    anyhow::ensure!(
        config.identifier == IDENTIFIER
            && config.app.windows.len() == 1
            && config.app.windows[0].label == "main"
            && config.app.windows[0].incognito,
        "Isolated E2E requires its own application identifier and an incognito main window"
    );
    anyhow::ensure!(
        std::env::args().any(|arg| arg == "--flowpilot-e2e-cli"),
        "Isolated E2E requires the CLI launch marker"
    );
    let root = PathBuf::from(root);
    let run_id = std::env::var("FLOWPILOT_E2E_CLI_RUN_ID")?;
    validate_root(&root, &std::env::temp_dir(), &run_id)?;
    for (key, expected) in [
        ("CACHE_DIR", root.join("cache")),
        (
            "FLOW_LIKE_FLOWPILOT_DRAFT_DIR",
            root.join("projects/.flowpilot-drafts"),
        ),
    ] {
        anyhow::ensure!(
            std::env::var_os(key).map(PathBuf::from) == Some(expected),
            "Isolated E2E requires its own {key}"
        );
    }
    claim_root(&root)?;
    DATA_ROOT
        .set(root)
        .map_err(|_| anyhow::anyhow!("E2E isolation was already initialized"))?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    struct Fixture(PathBuf);
    impl Fixture {
        fn new() -> Self {
            let root = std::env::temp_dir()
                .canonicalize()
                .unwrap()
                .join(format!("flow-like-flowpilot-e2e-{}", uuid::Uuid::new_v4()));
            fs::create_dir(&root).unwrap();
            #[cfg(unix)]
            {
                use std::os::unix::fs::PermissionsExt;
                fs::set_permissions(&root, fs::Permissions::from_mode(0o700)).unwrap();
            }
            let marker = root.join(MARKER);
            fs::write(
                &marker,
                r#"{"schema":"flowpilot-e2e-isolation/v1","run_id":"test-run","runner_pid":1}"#,
            )
            .unwrap();
            #[cfg(unix)]
            {
                use std::os::unix::fs::PermissionsExt;
                fs::set_permissions(marker, fs::Permissions::from_mode(0o600)).unwrap();
            }
            Self(root)
        }

        fn validate(&self) -> anyhow::Result<()> {
            validate_root(&self.0, &std::env::temp_dir(), "test-run")
        }
    }
    impl Drop for Fixture {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.0);
        }
    }

    #[test]
    fn e2e_isolation_accepts_owned_fresh_root_and_claims_it_once() {
        let fixture = Fixture::new();
        fixture.validate().unwrap();
        claim_root(&fixture.0).unwrap();
        assert!(claim_root(&fixture.0).is_err());
        assert!(fixture.validate().is_err());
    }

    #[test]
    fn e2e_isolation_rejects_wrong_run_existing_data_and_relative_paths() {
        let fixture = Fixture::new();
        assert!(validate_root(&fixture.0, &std::env::temp_dir(), "other-run").is_err());
        assert!(validate_root(Path::new("relative"), &std::env::temp_dir(), "test-run").is_err());
        fs::write(fixture.0.join("global-settings.json"), "{}").unwrap();
        assert!(fixture.validate().is_err());
    }

    #[test]
    fn e2e_isolation_rejects_a_root_outside_the_temporary_parent() {
        let fixture = Fixture::new();
        assert!(validate_root(&fixture.0, &fixture.0, "test-run").is_err());
    }

    #[cfg(unix)]
    #[test]
    fn e2e_isolation_rejects_symlink_markers_and_public_permissions() {
        use std::os::unix::fs::{PermissionsExt, symlink};
        let fixture = Fixture::new();
        fs::set_permissions(&fixture.0, fs::Permissions::from_mode(0o755)).unwrap();
        assert!(fixture.validate().is_err());
        fs::set_permissions(&fixture.0, fs::Permissions::from_mode(0o700)).unwrap();
        let marker = fixture.0.join(MARKER);
        fs::rename(&marker, fixture.0.join("marker-target")).unwrap();
        symlink(fixture.0.join("marker-target"), marker).unwrap();
        assert!(fixture.validate().is_err());
    }
}
