use crate::{
    config::{PlacementConfig, ProjectSource},
    supervisor, vault,
};
use anyhow::{Context, Result, ensure};
use flow_like_device_protocol::PROJECT_ARTIFACT_MAX_FILES;
#[cfg(unix)]
use flow_like_types_contracts::storage::is_legacy_training_ledger_path;
use serde::{Deserialize, Serialize};
use std::{
    fs::{File, OpenOptions},
    io::{Read, Write},
    path::{Path, PathBuf},
};
use tokio_util::sync::CancellationToken;

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Binding {
    version: u32,
    placement_id: String,
    project_id: String,
    deployment_id: String,
    source: ProjectSource,
    initial_snapshot: PathBuf,
}

fn directory(path: &Path) -> Result<PathBuf> {
    #[cfg(unix)]
    {
        use std::os::unix::fs::DirBuilderExt;
        match std::fs::DirBuilder::new().mode(0o700).create(path) {
            Ok(()) => {
                if let Some(parent) = path.parent() {
                    File::open(parent)?.sync_all()?;
                }
            }
            Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => {}
            Err(error) => return Err(error.into()),
        }
        existing_directory(path)
    }
    #[cfg(not(unix))]
    anyhow::bail!("Placement data requires Unix file permissions");
}

fn existing_directory(path: &Path) -> Result<PathBuf> {
    #[cfg(unix)]
    {
        use std::os::unix::fs::MetadataExt;
        let metadata = std::fs::symlink_metadata(path)?;
        ensure!(
            metadata.is_dir()
                && !metadata.file_type().is_symlink()
                && metadata.uid() == unsafe { libc::geteuid() }
                && metadata.mode() & 0o077 == 0,
            "Placement data directory must be private and owned"
        );
        Ok(path.to_path_buf())
    }
    #[cfg(not(unix))]
    anyhow::bail!("Placement data requires Unix file permissions");
}

fn validate_binding(path: &Path, config: &PlacementConfig) -> Result<PathBuf> {
    existing_directory(path)?;
    let binding: Binding =
        serde_json::from_slice(&vault::read_private(&path.join("binding.json"))?)?;
    ensure!(
        binding.version == 1
            && binding.placement_id == config.id
            && binding.project_id == config.project_id
            && binding.deployment_id == config.deployment_id
            && binding.source == config.source,
        "Placement data identity or source type changed"
    );
    existing_directory(&path.join("store"))
}

fn record_initialized(parent: &Path, current: &Path) -> Result<()> {
    let binding = vault::read_private(&current.join("binding.json"))?;
    let receipt = parent.join("initialized.json");
    if receipt.try_exists()? {
        ensure!(
            *vault::read_private(&receipt)? == *binding,
            "Placement initialization receipt differs"
        );
    } else {
        let temporary = parent.join(format!(".initialized-{}.tmp", uuid::Uuid::new_v4()));
        let result = (|| -> Result<()> {
            vault::write_new_private(&temporary, &binding)?;
            std::fs::rename(&temporary, &receipt)?;
            File::open(parent)?.sync_all()?;
            Ok(())
        })();
        if result.is_err() {
            let _ = std::fs::remove_file(&temporary);
        }
        result?;
    }
    Ok(())
}

/// Validate a supervisor-selected data root without accepting a path from a deployment manifest.
pub(crate) fn validate_root(path: &Path, config: &PlacementConfig) -> Result<()> {
    ensure!(
        path.is_absolute() && path.file_name().is_some_and(|name| name == "store"),
        "Invalid placement data root"
    );
    let parent = path
        .parent()
        .context("Placement data root is missing its binding")?;
    ensure!(
        parent.file_name().is_some_and(|name| name == "current")
            && parent
                .parent()
                .and_then(Path::file_name)
                .is_some_and(|name| name == config.id.as_str()),
        "Placement data root identity differs"
    );
    ensure!(
        validate_binding(parent, config)? == path,
        "Placement data binding differs"
    );
    Ok(())
}

/// Snapshot directory holding the selected user's project data, which offline workers run
/// as the `local` user.
const USER_DATA_NAMESPACE: &str = "deployment-user-data";

#[derive(Default)]
struct Bounds {
    files: usize,
    directories: usize,
    bytes: u64,
}

#[cfg(unix)]
fn stamp(metadata: &std::fs::Metadata) -> (u64, u64, u64, i64, i64, i64, i64) {
    use std::os::unix::fs::MetadataExt;
    (
        metadata.dev(),
        metadata.ino(),
        metadata.len(),
        metadata.mtime(),
        metadata.mtime_nsec(),
        metadata.ctime(),
        metadata.ctime_nsec(),
    )
}

#[cfg(unix)]
fn count_directory(bounds: &mut Bounds) -> Result<()> {
    bounds.directories += 1;
    ensure!(
        bounds.directories <= PROJECT_ARTIFACT_MAX_FILES,
        "Initial data has too many directories"
    );
    Ok(())
}

#[cfg(unix)]
fn count_file(bounds: &mut Bounds, metadata: &std::fs::Metadata) -> Result<()> {
    use std::os::unix::fs::MetadataExt;
    ensure!(
        metadata.is_file() && metadata.nlink() == 1,
        "Initial data must contain regular files without hardlinks"
    );
    bounds.files += 1;
    bounds.bytes = bounds
        .bytes
        .checked_add(metadata.len())
        .context("Initial data size overflow")?;
    ensure!(
        bounds.files <= PROJECT_ARTIFACT_MAX_FILES,
        "Initial placement data exceeds its file limit"
    );
    Ok(())
}

/// A snapshot directory entry under the name its normalized artifact path gives it.
#[cfg(unix)]
struct SnapshotEntry {
    name: String,
    relative: String,
    path: PathBuf,
    metadata: std::fs::Metadata,
}

/// Lists `source` under the seeding rules: UTF-8 names that normalize as artifact paths,
/// no case-insensitive collisions and no symlinks. Entries whose normalized path `skip`
/// accepts are left out before any other check.
#[cfg(unix)]
fn snapshot_entries(
    source: &Path,
    relative: &str,
    project: &str,
    cancel: &CancellationToken,
    skip: impl Fn(&str) -> bool,
) -> Result<Vec<SnapshotEntry>> {
    let mut names = std::collections::HashSet::new();
    let mut entries = Vec::new();
    for entry in std::fs::read_dir(source)? {
        ensure!(
            !cancel.is_cancelled(),
            "Placement data initialization cancelled"
        );
        let entry = entry?;
        let name = entry.file_name();
        let name = name
            .to_str()
            .context("Initial data file name must be UTF-8")?;
        let relative = flow_like_device_protocol::normalize_artifact_path(
            project,
            &format!("{relative}/{name}"),
        )?;
        if skip(&relative) {
            continue;
        }
        let name = relative
            .rsplit('/')
            .next()
            .context("Initial data file name missing")?
            .to_owned();
        ensure!(
            names.insert(name.to_lowercase()),
            "Initial data has colliding file names"
        );
        let path = entry.path();
        let metadata = std::fs::symlink_metadata(&path)?;
        ensure!(
            !metadata.file_type().is_symlink(),
            "Initial data must not contain symlinks"
        );
        entries.push(SnapshotEntry {
            name,
            relative,
            path,
            metadata,
        });
    }
    Ok(entries)
}

/// Copies one counted regular file and refuses any change while it is read. `observe` sees
/// every copied chunk.
#[cfg(unix)]
fn copy_file(
    source: &Path,
    metadata: &std::fs::Metadata,
    destination: &Path,
    cancel: &CancellationToken,
    mut observe: impl FnMut(&[u8]),
) -> Result<()> {
    use std::os::unix::fs::OpenOptionsExt;
    let mut input = OpenOptions::new()
        .read(true)
        .custom_flags(libc::O_NOFOLLOW | libc::O_NONBLOCK)
        .open(source)?;
    ensure!(
        stamp(&input.metadata()?) == stamp(metadata),
        "Initial data changed during initialization"
    );
    let mut output = OpenOptions::new()
        .write(true)
        .create_new(true)
        .mode(0o600)
        .custom_flags(libc::O_NOFOLLOW)
        .open(destination)?;
    let mut bytes = [0u8; 128 * 1024];
    let mut copied = 0u64;
    loop {
        ensure!(
            !cancel.is_cancelled(),
            "Placement data initialization cancelled"
        );
        let count = input.read(&mut bytes)?;
        if count == 0 {
            break;
        }
        copied = copied
            .checked_add(count as u64)
            .context("Initial data size overflow")?;
        ensure!(
            copied <= metadata.len(),
            "Initial data changed during initialization"
        );
        observe(&bytes[..count]);
        output.write_all(&bytes[..count])?;
    }
    ensure!(
        copied == metadata.len()
            && stamp(&input.metadata()?) == stamp(metadata)
            && stamp(&std::fs::symlink_metadata(source)?) == stamp(metadata),
        "Initial data changed during initialization"
    );
    output.sync_all()?;
    Ok(())
}

/// The pre-device-local training ledger stays on the device that created it.
#[cfg(unix)]
fn is_legacy_ledger(project: &str, relative: &str) -> bool {
    relative
        .strip_prefix(&format!("apps/{project}/"))
        .is_some_and(is_legacy_training_ledger_path)
}

#[cfg(unix)]
fn copy_tree(
    source: &Path,
    destination: &Path,
    relative: &str,
    project: &str,
    bounds: &mut Bounds,
    cancel: &CancellationToken,
) -> Result<()> {
    ensure!(
        !cancel.is_cancelled(),
        "Placement data initialization cancelled"
    );
    let before = std::fs::symlink_metadata(source)?;
    ensure!(
        before.is_dir() && !before.file_type().is_symlink(),
        "Initial data must not contain symlinks"
    );
    count_directory(bounds)?;
    directory(destination)?;
    let entries = snapshot_entries(source, relative, project, cancel, |path| {
        is_legacy_ledger(project, path)
    })?;
    for entry in entries {
        let destination = destination.join(&entry.name);
        if entry.metadata.is_dir() {
            copy_tree(
                &entry.path,
                &destination,
                &entry.relative,
                project,
                bounds,
                cancel,
            )?;
            continue;
        }
        count_file(bounds, &entry.metadata)?;
        copy_file(&entry.path, &entry.metadata, &destination, cancel, |_| {})?;
    }
    ensure!(
        stamp(&std::fs::symlink_metadata(source)?) == stamp(&before),
        "Initial data directory changed during initialization"
    );
    File::open(destination)?.sync_all()?;
    Ok(())
}

/// Resolves `components` below `root` without following symlinks or case aliases; `None`
/// when one of them is missing.
fn snapshot_directory(root: &Path, components: &[&str]) -> Result<Option<PathBuf>> {
    let mut path = root.to_path_buf();
    for component in components {
        crate::config::ensure_unaliased_child(&path, component)?;
        path.push(component);
        match std::fs::symlink_metadata(&path) {
            Ok(metadata) => ensure!(
                metadata.is_dir() && !metadata.file_type().is_symlink(),
                "Initial project data path must not contain symlinks"
            ),
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(None),
            Err(error) => return Err(error.into()),
        }
    }
    Ok(Some(path))
}

/// The source must be a stopped project or a consistent database snapshot. Copying
/// files cannot manufacture a transactionally consistent snapshot of a live database.
/// The reconciler acquires model assets separately after this disk-copy slot is released.
pub(crate) fn prepare_for_launch(
    root: &Path,
    config: &PlacementConfig,
    revision: u64,
    intent: u64,
    cancel: &CancellationToken,
) -> Result<PathBuf> {
    prepare_data(root, config, revision, intent, cancel)
}

fn prepare_data(
    root: &Path,
    config: &PlacementConfig,
    revision: u64,
    intent: u64,
    cancel: &CancellationToken,
) -> Result<PathBuf> {
    prepare(root, config, cancel, || {
        let store = crate::state::StateStore::open(&root.join("management.sqlite"))?;
        store.connection.execute_batch("BEGIN IMMEDIATE")?;
        let record = store
            .get_placement(&config.id)?
            .context("Placement was removed while initializing data")?;
        ensure!(
            record.config_revision == revision
                && record.intent_revision == intent
                && record.desired_state == crate::state::DesiredState::Running,
            "Placement changed while initializing data"
        );
        Ok(store)
    })
}

fn prepare<G>(
    root: &Path,
    config: &PlacementConfig,
    cancel: &CancellationToken,
    before_publish: impl FnOnce() -> Result<G>,
) -> Result<PathBuf> {
    config.validate()?;
    let base = directory(&directory(root)?.join("placement-data"))?;
    crate::config::ensure_unaliased_child(&base, &config.id)?;
    let base = directory(&base.join(&config.id))?;
    // Admission precedes copying a bundle so an unprovisioned sandbox cannot
    // fill the agent filesystem before its worker is rejected.
    crate::isolation::preflight(config, root, &base)?;
    let target = base.join("current");
    let _lock = supervisor::lock_file(&base.join("initialize.lock"))?;
    if target.try_exists()? {
        #[cfg(unix)]
        let staged = published_models::stage(&base, &target, config, cancel)?;
        let _publication_guard = before_publish()?;
        ensure!(
            !cancel.is_cancelled(),
            "Placement data initialization cancelled"
        );
        let data = validate_binding(&target, config)?;
        #[cfg(unix)]
        published_models::publish(staged, &data, config);
        record_initialized(&base, &target)?;
        return Ok(data);
    }
    ensure!(
        !base.join("initialized.json").try_exists()?,
        "Previously initialized placement data is missing; restore it before starting"
    );
    // A surviving child from an older agent can still own the source database.
    let mut source_locks = Vec::new();
    for slot in 0..32 {
        let path = root.join(format!("placement-{}-slot-{slot}.lock", config.id));
        if path.try_exists()? {
            source_locks.push(supervisor::lock_file(&path)?);
        }
    }
    let staging = base.join("staging");
    if staging.try_exists()? {
        directory(&staging)?;
        std::fs::remove_dir_all(&staging)?;
    }
    directory(&staging)?;
    let result = (|| -> Result<PathBuf> {
        let store = directory(&staging.join("store"))?;
        let apps = directory(&store.join("apps"))?;
        let project = directory(&apps.join(&config.project_id))?;
        if config.source == ProjectSource::Offline {
            let snapshot = config.project_path.canonicalize()?;
            if let Some(source) = snapshot_directory(&snapshot, &["apps", &config.project_id])? {
                let mut bounds = Bounds::default();
                for namespace in ["storage", "files", "upload", "metadata"] {
                    crate::config::ensure_unaliased_child(&source, namespace)?;
                    let input = source.join(namespace);
                    match std::fs::symlink_metadata(&input) {
                        Ok(_) => {
                            #[cfg(unix)]
                            copy_tree(
                                &input,
                                &project.join(namespace),
                                &format!("apps/{}/{namespace}", config.project_id),
                                &config.project_id,
                                &mut bounds,
                                cancel,
                            )?;
                            #[cfg(not(unix))]
                            anyhow::bail!("Placement data initialization requires Unix");
                        }
                        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
                        Err(error) => return Err(error.into()),
                    }
                }
                // The exporter includes one selected user's project data. Offline
                // workers use `local`, so do not preserve the source account path.
                let namespace = USER_DATA_NAMESPACE;
                crate::config::ensure_unaliased_child(&source, namespace)?;
                let input = source.join(namespace);
                match std::fs::symlink_metadata(&input) {
                    Ok(_) => {
                        let user = directory(&store.join("user"))?;
                        let users = directory(&user.join("users"))?;
                        let local = directory(&users.join("local"))?;
                        let apps = directory(&local.join("apps"))?;
                        #[cfg(unix)]
                        copy_tree(
                            &input,
                            &apps.join(&config.project_id),
                            &format!("apps/{}/{namespace}", config.project_id),
                            &config.project_id,
                            &mut bounds,
                            cancel,
                        )?;
                        #[cfg(not(unix))]
                        anyhow::bail!("Placement data initialization requires Unix");
                    }
                    Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
                    Err(error) => return Err(error.into()),
                }
            }
        }
        for namespace in ["user", "logs", "tmp"] {
            directory(&store.join(namespace))?;
        }
        let binding = Binding {
            version: 1,
            placement_id: config.id.clone(),
            project_id: config.project_id.clone(),
            deployment_id: config.deployment_id.clone(),
            source: config.source,
            initial_snapshot: config.project_path.clone(),
        };
        vault::write_new_private(
            &staging.join("binding.json"),
            &serde_json::to_vec(&binding)?,
        )?;
        File::open(&store)?.sync_all()?;
        File::open(&staging)?.sync_all()?;
        ensure!(
            !cancel.is_cancelled(),
            "Placement data initialization cancelled"
        );
        let _publication_guard = before_publish()?;
        ensure!(
            !cancel.is_cancelled(),
            "Placement data initialization cancelled"
        );
        ensure!(!target.try_exists()?, "Placement data already exists");
        std::fs::rename(&staging, &target)?;
        File::open(&base)?.sync_all()?;
        let data = validate_binding(&target, config)?;
        record_initialized(&base, &target)?;
        Ok(data)
    })();
    if result.is_err() && staging.exists() {
        let _ = std::fs::remove_dir_all(&staging);
    }
    result
}

/// Additive refresh of published models on existing offline placements. Staging copies a
/// revision's new artifact folders and its pointer files under `models-staging` while
/// `initialize.lock` is held; under the management guard the refresh only renames.
#[cfg(unix)]
mod published_models {
    use super::*;
    use flow_like_types_contracts::storage::{PUBLISHED_MODELS_DIR, storage_path_segment};
    use sha2::{Digest, Sha256};

    pub(super) const MODELS_STAGING: &str = "models-staging";
    // Layout and caps of flow-like-catalog-ml's `inspection::published`, which the agent does
    // not link in every build.
    const ARTIFACTS_DIR: &str = "artifacts";
    const MANIFEST_FILE: &str = "manifest.json";
    const WEIGHTS_FILE: &str = "weights.bin";
    const PREPROCESSING_FILE: &str = "preprocessing.json";
    const MAXIMUM_MANIFEST_BYTES: u64 = 1024 * 1024;
    const MAXIMUM_POINTER_BYTES: u64 = 64 * 1024;

    #[derive(Clone, Copy, PartialEq, Eq)]
    enum PointerKind {
        Experiment,
        Learning,
        Deployment,
    }

    impl PointerKind {
        const ALL: [Self; 3] = [Self::Experiment, Self::Learning, Self::Deployment];

        fn directory(&self) -> &str {
            match self {
                Self::Experiment => "experiments",
                Self::Learning => "learning",
                Self::Deployment => "deployments",
            }
        }
    }

    /// Experiment pointers are published before learning and deployment pointers.
    const PUBLICATION_ORDER: [&[PointerKind]; 2] = [
        &[PointerKind::Experiment],
        &[PointerKind::Learning, PointerKind::Deployment],
    ];

    #[derive(Deserialize)]
    struct ExperimentPointer {
        artifact_id: String,
    }

    #[derive(Deserialize)]
    struct ChampionPointer {
        deployment_generation: i64,
        project_generation: i64,
        artifact_id: Option<String>,
    }

    #[derive(Deserialize)]
    struct DeploymentPointer {
        generation: i64,
        active_artifact_id: String,
    }

    /// A pointer's order and the artifact it names, read with the published module's shapes.
    struct PointerFacts {
        order: (i64, i64),
        artifact: Option<String>,
    }

    fn pointer_facts(kind: PointerKind, bytes: &[u8]) -> Result<PointerFacts> {
        Ok(match kind {
            PointerKind::Experiment => {
                let pointer: ExperimentPointer = serde_json::from_slice(bytes)?;
                PointerFacts {
                    order: (0, 0),
                    artifact: Some(pointer.artifact_id),
                }
            }
            PointerKind::Learning => {
                let pointer: ChampionPointer = serde_json::from_slice(bytes)?;
                PointerFacts {
                    order: (pointer.deployment_generation, pointer.project_generation),
                    artifact: pointer.artifact_id,
                }
            }
            PointerKind::Deployment => {
                let pointer: DeploymentPointer = serde_json::from_slice(bytes)?;
                PointerFacts {
                    order: (pointer.generation, 0),
                    artifact: Some(pointer.active_artifact_id),
                }
            }
        })
    }

    #[derive(Deserialize)]
    struct Manifest {
        weights: ManifestFile,
        #[serde(default)]
        preprocessing: Option<ManifestFile>,
    }

    #[derive(Deserialize)]
    struct ManifestFile {
        file: Option<String>,
        sha256: String,
        bytes: u64,
    }

    /// A published-model root, relative to the snapshot and to the placement store.
    struct ModelRoot {
        label: String,
        source: String,
        target: String,
    }

    fn model_roots(project: &str) -> [ModelRoot; 2] {
        [
            ModelRoot {
                label: "app".into(),
                source: format!("apps/{project}/storage/{PUBLISHED_MODELS_DIR}"),
                target: format!("apps/{project}/storage/{PUBLISHED_MODELS_DIR}"),
            },
            ModelRoot {
                label: "user".into(),
                source: format!("apps/{project}/{USER_DATA_NAMESPACE}/{PUBLISHED_MODELS_DIR}"),
                target: format!("user/users/local/apps/{project}/{PUBLISHED_MODELS_DIR}"),
            },
        ]
    }

    /// Models staged for publication; dropping them removes the staging directory.
    pub(super) struct StagedModels {
        directory: PathBuf,
        roots: Vec<StagedRoot>,
    }

    struct StagedRoot {
        staging: PathBuf,
        target: String,
        artifacts: Vec<String>,
        pointers: Vec<(PointerKind, String)>,
    }

    impl Drop for StagedModels {
        fn drop(&mut self) {
            if let Err(error) = std::fs::remove_dir_all(&self.directory)
                && error.kind() != std::io::ErrorKind::NotFound
            {
                tracing::warn!(
                    staging = %self.directory.display(),
                    %error,
                    "Failed to remove staged published models"
                );
            }
        }
    }

    #[derive(Default)]
    pub(super) struct RefreshReport {
        artifacts: usize,
        pointers: usize,
        kept: usize,
    }

    /// Stages a revision's published models for an existing offline placement. Failures
    /// other than cancellation are logged and never block the start.
    pub(super) fn stage(
        base: &Path,
        current: &Path,
        config: &PlacementConfig,
        cancel: &CancellationToken,
    ) -> Result<Option<StagedModels>> {
        if config.source != ProjectSource::Offline {
            return Ok(None);
        }
        let Ok(store) = validate_binding(current, config) else {
            return Ok(None);
        };
        match refresh_published_models(
            &config.project_path,
            base,
            &store,
            &config.project_id,
            cancel,
        ) {
            Err(error) if !cancel.is_cancelled() => {
                tracing::warn!(
                    placement = %config.id,
                    revision = %config.revision,
                    "Published models of this revision were not staged: {error:#}"
                );
                Ok(None)
            }
            result => result,
        }
    }

    /// Publishes staged models under the management guard; a failure is logged.
    pub(super) fn publish(staged: Option<StagedModels>, store: &Path, config: &PlacementConfig) {
        let Some(staged) = staged else {
            return;
        };
        match publish_refreshed_models(staged, store) {
            Ok(report) => tracing::info!(
                placement = %config.id,
                revision = %config.revision,
                artifacts = report.artifacts,
                pointers = report.pointers,
                kept = report.kept,
                "Refreshed published models"
            ),
            Err(error) => tracing::warn!(
                placement = %config.id,
                revision = %config.revision,
                "Published models were not refreshed: {error:#}"
            ),
        }
    }

    /// Copies what the placement lacks from the published models of the snapshot into
    /// `base/models-staging`: artifact folders absent from `store`, each verified against its
    /// manifest, and every pointer file.
    pub(super) fn refresh_published_models(
        snapshot: &Path,
        base: &Path,
        store: &Path,
        project_id: &str,
        cancel: &CancellationToken,
    ) -> Result<Option<StagedModels>> {
        let snapshot = snapshot.canonicalize()?;
        let mut staged = StagedModels::fresh(base.join(MODELS_STAGING))?;
        let mut stager = Stager {
            project: project_id.to_owned(),
            cancel: cancel.clone(),
            bounds: Bounds::default(),
        };
        for root in model_roots(project_id) {
            let components = root.source.split('/').collect::<Vec<_>>();
            let Some(source) = snapshot_directory(&snapshot, &components)? else {
                continue;
            };
            let target = join_relative(store, &root.target);
            let staged_root = stager.root(&source, &staged.directory, &target, &root)?;
            if !staged_root.artifacts.is_empty() || !staged_root.pointers.is_empty() {
                staged.roots.push(staged_root);
            }
        }
        Ok((!staged.roots.is_empty()).then_some(staged))
    }

    impl StagedModels {
        /// An empty staging directory; a stale one from an interrupted start is removed.
        fn fresh(path: PathBuf) -> Result<Self> {
            if path.try_exists()? {
                directory(&path)?;
                std::fs::remove_dir_all(&path)?;
            }
            directory(&path)?;
            Ok(Self {
                directory: path,
                roots: Vec::new(),
            })
        }
    }

    /// An artifact folder that is incomplete or does not match its manifest. It is left out
    /// while the rest of the revision is refreshed; every other staging error aborts.
    #[derive(Debug, thiserror::Error)]
    #[error("{0}")]
    struct Rejected(String);

    fn rejected(reason: String) -> anyhow::Error {
        Rejected(reason).into()
    }

    /// Leaves a rejected artifact folder out of the refresh.
    fn discard(artifact: &str, staged: &Path, reason: &anyhow::Error) -> Result<()> {
        tracing::warn!(
            artifact,
            "Published model folder is not refreshed: {reason}"
        );
        if present(staged)? {
            std::fs::remove_dir_all(staged)?;
        }
        Ok(())
    }

    /// Copies snapshot files into staging under the seeding rules and one set of bounds.
    struct Stager {
        project: String,
        cancel: CancellationToken,
        bounds: Bounds,
    }

    impl Stager {
        fn root(
            &mut self,
            source: &Path,
            staging: &Path,
            target: &Path,
            root: &ModelRoot,
        ) -> Result<StagedRoot> {
            let staging = directory(&staging.join(&root.label))?;
            let artifacts = self.artifacts(source, &staging, target, root)?;
            let mut pointers = Vec::new();
            for kind in PointerKind::ALL {
                pointers.extend(self.pointers(source, &staging, root, kind)?);
            }
            Ok(StagedRoot {
                staging,
                target: root.target.clone(),
                artifacts,
                pointers,
            })
        }

        /// Stages the artifact folders that `target` lacks and returns their names.
        fn artifacts(
            &mut self,
            source: &Path,
            staging: &Path,
            target: &Path,
            root: &ModelRoot,
        ) -> Result<Vec<String>> {
            let Some(artifacts) = snapshot_directory(source, &[ARTIFACTS_DIR])? else {
                return Ok(Vec::new());
            };
            count_directory(&mut self.bounds)?;
            let folders = directory(&staging.join(ARTIFACTS_DIR))?;
            let relative = format!("{}/{ARTIFACTS_DIR}", root.source);
            let mut staged = Vec::new();
            for entry in self.entries(&artifacts, &relative)? {
                if !entry.metadata.is_dir()
                    || present(&target.join(ARTIFACTS_DIR).join(&entry.name))?
                {
                    continue;
                }
                count_directory(&mut self.bounds)?;
                let folder = folders.join(&entry.name);
                match self.artifact(&entry.path, &folder) {
                    Ok(()) => staged.push(entry.name),
                    Err(error) if error.is::<Rejected>() => {
                        discard(&entry.relative, &folder, &error)?
                    }
                    Err(error) => return Err(error),
                }
            }
            Ok(staged)
        }

        /// Stages every pointer file of one kind; pointers are small, so publication decides.
        fn pointers(
            &mut self,
            source: &Path,
            staging: &Path,
            root: &ModelRoot,
            kind: PointerKind,
        ) -> Result<Vec<(PointerKind, String)>> {
            let Some(pointers) = snapshot_directory(source, &[kind.directory()])? else {
                return Ok(Vec::new());
            };
            count_directory(&mut self.bounds)?;
            let destination = directory(&staging.join(kind.directory()))?;
            let relative = format!("{}/{}", root.source, kind.directory());
            let mut staged = Vec::new();
            for entry in self.entries(&pointers, &relative)? {
                if !entry.metadata.is_file() || !entry.name.ends_with(".json") {
                    continue;
                }
                if entry.metadata.len() > MAXIMUM_POINTER_BYTES {
                    tracing::warn!(
                        pointer = %entry.relative,
                        bytes = entry.metadata.len(),
                        "Published model pointer exceeds {MAXIMUM_POINTER_BYTES} bytes and is not refreshed"
                    );
                    continue;
                }
                self.file(&entry.path, &entry.metadata, &destination.join(&entry.name))?;
                staged.push((kind, entry.name));
            }
            File::open(&destination)?.sync_all()?;
            Ok(staged)
        }

        /// Stages one artifact folder: its manifest, then the weights and preprocessing files
        /// the manifest names.
        fn artifact(&mut self, folder: &Path, staging: &Path) -> Result<()> {
            let manifest = self.manifest(folder, staging)?;
            let files = std::iter::once((WEIGHTS_FILE, manifest.weights)).chain(
                manifest
                    .preprocessing
                    .map(|preprocessing| (PREPROCESSING_FILE, preprocessing)),
            );
            for (name, declared) in files {
                self.declared_file(folder, staging, name, &declared)?;
            }
            File::open(staging)?.sync_all()?;
            Ok(())
        }

        fn manifest(&mut self, folder: &Path, staging: &Path) -> Result<Manifest> {
            let (path, metadata) = artifact_file(folder, MANIFEST_FILE)?
                .ok_or_else(|| rejected(format!("{MANIFEST_FILE} is missing")))?;
            if !metadata.is_file() || metadata.len() > MAXIMUM_MANIFEST_BYTES {
                return Err(rejected(format!(
                    "{MANIFEST_FILE} is not a file of at most {MAXIMUM_MANIFEST_BYTES} bytes"
                )));
            }
            directory(staging)?;
            self.file(&path, &metadata, &staging.join(MANIFEST_FILE))?;
            serde_json::from_slice(&std::fs::read(staging.join(MANIFEST_FILE))?).map_err(|error| {
                rejected(format!(
                    "{MANIFEST_FILE} is not a published model manifest: {error}"
                ))
            })
        }

        /// Stages a file the manifest names after checking its name and size, then checks the
        /// sha256 of the copy.
        fn declared_file(
            &mut self,
            folder: &Path,
            staging: &Path,
            name: &str,
            declared: &ManifestFile,
        ) -> Result<()> {
            if declared.file.as_deref() != Some(name) {
                return Err(rejected(format!(
                    "the manifest names {:?} instead of {name}",
                    declared.file
                )));
            }
            let (path, metadata) = artifact_file(folder, name)?
                .ok_or_else(|| rejected(format!("{name} is missing")))?;
            if !metadata.is_file() || metadata.len() != declared.bytes {
                return Err(rejected(format!(
                    "{name} has {} bytes where the manifest declares {}",
                    metadata.len(),
                    declared.bytes
                )));
            }
            let sha256 = self.file(&path, &metadata, &staging.join(name))?;
            if !sha256.eq_ignore_ascii_case(&declared.sha256) {
                return Err(rejected(format!(
                    "{name} has sha256 {sha256} where the manifest declares {}",
                    declared.sha256
                )));
            }
            Ok(())
        }

        fn entries(&self, parent: &Path, relative: &str) -> Result<Vec<SnapshotEntry>> {
            snapshot_entries(parent, relative, &self.project, &self.cancel, |_| false)
        }

        /// Copies a snapshot file into staging and returns its sha256.
        fn file(
            &mut self,
            source: &Path,
            metadata: &std::fs::Metadata,
            destination: &Path,
        ) -> Result<String> {
            count_file(&mut self.bounds, metadata)?;
            let mut hash = Sha256::new();
            copy_file(source, metadata, destination, &self.cancel, |bytes| {
                hash.update(bytes)
            })?;
            Ok(format!("{:x}", hash.finalize()))
        }
    }

    /// A file of a snapshot artifact folder; a symlink or a case alias is refused.
    fn artifact_file(folder: &Path, name: &str) -> Result<Option<(PathBuf, std::fs::Metadata)>> {
        crate::config::ensure_unaliased_child(folder, name)?;
        let path = folder.join(name);
        match std::fs::symlink_metadata(&path) {
            Ok(metadata) => {
                ensure!(
                    !metadata.file_type().is_symlink(),
                    "Initial data must not contain symlinks"
                );
                Ok(Some((path, metadata)))
            }
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(None),
            Err(error) => Err(error.into()),
        }
    }

    /// Whether anything, a symlink included, exists at `path`.
    fn present(path: &Path) -> Result<bool> {
        match std::fs::symlink_metadata(path) {
            Ok(_) => Ok(true),
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(false),
            Err(error) => Err(error.into()),
        }
    }

    fn join_relative(root: &Path, relative: &str) -> PathBuf {
        relative
            .split('/')
            .fold(root.to_path_buf(), |path, part| path.join(part))
    }

    /// Creates the missing directories of `relative` below `root`. Each must be a directory
    /// the agent owns and not a symlink; workers create them with their own umask, so the
    /// private-mode check of `directory` does not apply.
    fn store_directories(root: &Path, relative: &str) -> Result<PathBuf> {
        use std::os::unix::fs::{DirBuilderExt, MetadataExt};
        let mut path = root.to_path_buf();
        for component in relative.split('/') {
            let parent = path.clone();
            path.push(component);
            match std::fs::DirBuilder::new().mode(0o700).create(&path) {
                Ok(()) => File::open(&parent)?.sync_all()?,
                Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => {}
                Err(error) => return Err(error.into()),
            }
            let metadata = std::fs::symlink_metadata(&path)?;
            ensure!(
                metadata.is_dir()
                    && !metadata.file_type().is_symlink()
                    && metadata.uid() == unsafe { libc::geteuid() },
                "{} must be a directory the agent owns, not a symlink",
                path.display()
            );
        }
        Ok(path)
    }

    /// Publishes staged models into `store` and only renames: artifact folders absent from
    /// the store, then experiment pointers absent from it, then learning and deployment
    /// pointers that are absent or strictly newer. A pointer is published only when the
    /// artifact it names is on the device, and nothing on the device is deleted.
    pub(super) fn publish_refreshed_models(
        staged: StagedModels,
        store: &Path,
    ) -> Result<RefreshReport> {
        let mut report = RefreshReport::default();
        let mut roots = Vec::with_capacity(staged.roots.len());
        for root in &staged.roots {
            roots.push((root, store_directories(store, &root.target)?));
        }
        for (root, models) in &roots {
            publish_artifacts(root, models, &mut report)?;
        }
        for kinds in PUBLICATION_ORDER {
            for (root, models) in &roots {
                for &kind in kinds {
                    publish_pointers(root, models, kind, &mut report)?;
                }
            }
        }
        Ok(report)
    }

    fn publish_artifacts(
        root: &StagedRoot,
        models: &Path,
        report: &mut RefreshReport,
    ) -> Result<()> {
        if root.artifacts.is_empty() {
            return Ok(());
        }
        let artifacts = store_directories(models, ARTIFACTS_DIR)?;
        for folder in &root.artifacts {
            let target = artifacts.join(folder);
            if present(&target)? {
                report.kept += 1;
                continue;
            }
            match std::fs::rename(root.staging.join(ARTIFACTS_DIR).join(folder), &target) {
                Ok(()) => report.artifacts += 1,
                Err(error)
                    if matches!(
                        error.kind(),
                        std::io::ErrorKind::AlreadyExists | std::io::ErrorKind::DirectoryNotEmpty
                    ) =>
                {
                    report.kept += 1
                }
                Err(error) => return Err(error.into()),
            }
        }
        File::open(&artifacts)?.sync_all()?;
        Ok(())
    }

    fn publish_pointers(
        root: &StagedRoot,
        models: &Path,
        kind: PointerKind,
        report: &mut RefreshReport,
    ) -> Result<()> {
        let names = root
            .pointers
            .iter()
            .filter(|(staged, _)| *staged == kind)
            .map(|(_, name)| name)
            .collect::<Vec<_>>();
        if names.is_empty() {
            return Ok(());
        }
        let pointers = store_directories(models, kind.directory())?;
        let artifacts = models.join(ARTIFACTS_DIR);
        for name in names {
            let staged = root.staging.join(kind.directory()).join(name);
            let target = pointers.join(name);
            match replaces(kind, &staged, &target, &artifacts) {
                Ok(true) => {
                    std::fs::rename(&staged, &target)?;
                    report.pointers += 1;
                }
                Ok(false) => report.kept += 1,
                Err(error) => {
                    tracing::warn!(
                        pointer = %target.display(),
                        "Published model pointer is not refreshed: {error:#}"
                    );
                    report.kept += 1;
                }
            }
        }
        File::open(&pointers)?.sync_all()?;
        Ok(())
    }

    /// Whether the staged pointer takes `target`'s place: an experiment pointer only while
    /// `target` is absent, a learning or deployment pointer also when its order is strictly
    /// greater. A pointer whose artifact is not on the device never does.
    fn replaces(kind: PointerKind, staged: &Path, target: &Path, artifacts: &Path) -> Result<bool> {
        let incoming = pointer_facts(kind, &std::fs::read(staged)?)?;
        if let Some(artifact) = &incoming.artifact {
            let manifest = artifacts
                .join(storage_path_segment(artifact, "model"))
                .join(MANIFEST_FILE);
            ensure!(
                present(&manifest)?,
                "its artifact {artifact} is not on the device"
            );
        }
        let Some(existing) = read_pointer(target)? else {
            return Ok(true);
        };
        if kind == PointerKind::Experiment {
            return Ok(false);
        }
        Ok(incoming.order > pointer_facts(kind, &existing)?.order)
    }

    /// The pointer at `path`, at most `MAXIMUM_POINTER_BYTES`; a symlink is not followed.
    fn read_pointer(path: &Path) -> Result<Option<Vec<u8>>> {
        use std::os::unix::fs::OpenOptionsExt;
        let file = match OpenOptions::new()
            .read(true)
            .custom_flags(libc::O_NOFOLLOW | libc::O_NONBLOCK)
            .open(path)
        {
            Ok(file) => file,
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(None),
            Err(error) => return Err(error.into()),
        };
        let mut bytes = Vec::new();
        file.take(MAXIMUM_POINTER_BYTES + 1)
            .read_to_end(&mut bytes)?;
        ensure!(
            bytes.len() as u64 <= MAXIMUM_POINTER_BYTES,
            "{} exceeds {MAXIMUM_POINTER_BYTES} bytes",
            path.display()
        );
        Ok(Some(bytes))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn state_root() -> Result<tempfile::TempDir> {
        let directory = tempfile::tempdir()?;
        supervisor::prepare_state_dir(directory.path())?;
        Ok(directory)
    }
    fn config(source: &Path) -> PlacementConfig {
        serde_json::from_value(serde_json::json!({"id":"placement","project_id":"project","deployment_id":"deployment","revision":"one","source":"offline","project_path":source,"events":[{"event_id":"event","event_version":[1,0,0],"board_version":[1,0,0]}]})).unwrap()
    }
    fn seed(root: &Path, value: &[u8]) {
        let storage = root.join("apps/project/storage/db");
        std::fs::create_dir_all(&storage).unwrap();
        std::fs::write(storage.join("table"), value).unwrap();
        std::fs::write(root.join("apps/project/manifest.app"), b"pinned metadata").unwrap();
        std::fs::create_dir_all(root.join("user")).unwrap();
        std::fs::write(root.join("user/private"), b"never imported").unwrap();
    }
    /// Publishes `weights` under a manifest that records the size and sha256 of `trained`.
    #[cfg(unix)]
    fn model_folder(models: &Path, artifact: &str, weights: &[u8], trained: &[u8]) {
        use sha2::{Digest, Sha256};
        let folder = models.join("artifacts").join(artifact);
        std::fs::create_dir_all(&folder).unwrap();
        std::fs::write(folder.join("weights.bin"), weights).unwrap();
        let manifest = serde_json::json!({
            "format_version": 2,
            "artifact": {"id": artifact},
            "weights": {
                "file": "weights.bin",
                "sha256": format!("{:x}", Sha256::digest(trained)),
                "bytes": trained.len(),
            },
        });
        std::fs::write(
            folder.join("manifest.json"),
            serde_json::to_vec(&manifest).unwrap(),
        )
        .unwrap();
    }
    #[cfg(unix)]
    fn pointer(models: &Path, kind: &str, id: &str, value: serde_json::Value) {
        let directory = models.join(kind);
        std::fs::create_dir_all(&directory).unwrap();
        std::fs::write(
            directory.join(format!("{id}.json")),
            serde_json::to_vec(&value).unwrap(),
        )
        .unwrap();
    }
    #[cfg(unix)]
    fn champion(models: &Path, project: &str, order: (i64, i64), artifact: Option<&str>) {
        pointer(
            models,
            "learning",
            project,
            serde_json::json!({
                "format_version": 1,
                "project_id": project,
                "deployment_id": format!("deployment:learning:{project}"),
                "deployment_generation": order.0,
                "project_generation": order.1,
                "artifact_id": artifact,
                "sha256": null,
                "paused": false,
                "cycle_id": null,
                "retained": [],
                "storage": "app",
                "published_at_ms": 0
            }),
        );
    }
    #[cfg(unix)]
    fn deployment(models: &Path, id: &str, generation: i64, artifact: &str) {
        pointer(
            models,
            "deployments",
            id,
            serde_json::json!({
                "format_version": 1,
                "deployment_id": id,
                "generation": generation,
                "active_artifact_id": artifact,
                "paused": false,
                "stream": {
                    "project_id": "project:board",
                    "stream_id": "auto:stream",
                    "inspection_version": "version"
                },
                "sha256": "digest",
                "deployment": {},
                "retained": [],
                "storage": "app",
                "published_at_ms": 0
            }),
        );
    }
    #[cfg(unix)]
    fn experiment(models: &Path, id: &str, artifact: &str) {
        pointer(
            models,
            "experiments",
            id,
            serde_json::json!({
                "format_version": 1,
                "experiment_id": id,
                "artifact_id": artifact,
                "sha256": "digest",
                "bytes": 1,
                "goal_met": true,
                "storage": "app",
                "published_at_ms": 0
            }),
        );
    }
    #[cfg(unix)]
    fn read_json(path: &Path) -> serde_json::Value {
        serde_json::from_slice(&std::fs::read(path).unwrap()).unwrap()
    }
    #[cfg(unix)]
    fn next_revision(config: &mut PlacementConfig, snapshot: &Path) {
        config.project_path = snapshot.into();
        config.revision = "two".into();
    }
    #[cfg(unix)]
    fn app_models(root: &Path) -> PathBuf {
        root.join("apps/project/storage/models")
    }
    /// An offline placement that starts from the `first` snapshot and is updated to `second`.
    #[cfg(unix)]
    struct Revisions {
        root: tempfile::TempDir,
        first: tempfile::TempDir,
        second: tempfile::TempDir,
        cancel: CancellationToken,
    }
    #[cfg(unix)]
    impl Revisions {
        fn new() -> Result<Self> {
            let revisions = Self {
                root: state_root()?,
                first: tempfile::tempdir()?,
                second: tempfile::tempdir()?,
                cancel: CancellationToken::new(),
            };
            seed(revisions.first.path(), b"initial");
            seed(revisions.second.path(), b"initial");
            Ok(revisions)
        }
        /// Starts the placement, lets `on_device` change its store, then prepares the second
        /// revision with `guard` as the management guard. Returns the store.
        fn update<G>(
            &self,
            on_device: impl FnOnce(&Path) -> Result<()>,
            guard: impl FnOnce() -> Result<G>,
        ) -> Result<PathBuf> {
            let mut config = config(self.first.path());
            let data = prepare(self.root.path(), &config, &self.cancel, || Ok(()))?;
            on_device(data.as_path())?;
            next_revision(&mut config, self.second.path());
            prepare(self.root.path(), &config, &self.cancel, guard)
        }
        fn store(&self) -> PathBuf {
            self.root
                .path()
                .join("placement-data/placement/current/store")
        }
        fn staging(&self) -> PathBuf {
            self.root
                .path()
                .join("placement-data/placement")
                .join(published_models::MODELS_STAGING)
        }
    }
    #[test]
    fn revisions_restarts_and_replicas_preserve_live_data_and_other_placements_are_isolated()
    -> Result<()> {
        let root = state_root()?;
        let first = tempfile::tempdir()?;
        let second = tempfile::tempdir()?;
        seed(first.path(), b"initial");
        seed(second.path(), b"new snapshot");
        let mut config = config(first.path());
        let cancel = CancellationToken::new();
        let data = prepare(root.path(), &config, &cancel, || Ok(()))?;
        let file = data.join("apps/project/storage/db/table");
        assert_eq!(std::fs::read(&file)?, b"initial");
        assert!(!data.join("apps/project/manifest.app").exists());
        assert!(!data.join("user/private").exists());
        std::fs::write(&file, b"live write")?;
        let wrapper = data.parent().unwrap().parent().unwrap();
        std::fs::remove_file(wrapper.join("initialized.json"))?;
        vault::write_new_private(
            &wrapper.join(format!(".initialized-{}.tmp", uuid::Uuid::new_v4())),
            b"partial receipt",
        )?;
        assert_eq!(prepare(root.path(), &config, &cancel, || Ok(()))?, data);
        assert!(wrapper.join("initialized.json").is_file());
        assert_eq!(std::fs::read(&file)?, b"live write");
        config.project_path = second.path().into();
        config.revision = "two".into();
        assert_eq!(prepare(root.path(), &config, &cancel, || Ok(()))?, data);
        assert_eq!(std::fs::read(&file)?, b"live write");
        config.id = "second".into();
        let other = prepare(root.path(), &config, &cancel, || Ok(()))?;
        assert_ne!(other, data);
        assert_eq!(
            std::fs::read(other.join("apps/project/storage/db/table"))?,
            b"new snapshot"
        );
        config.id = "placement".into();
        config.source = ProjectSource::Online;
        config.resource_grant = Some(crate::config::ResourceGrantRef {
            grant_id: "grant".into(),
            authz_version: 1,
            billing_grant_id: None,
            billing_authz_version: None,
        });
        assert!(prepare(root.path(), &config, &cancel, || Ok(())).is_err());
        assert_eq!(std::fs::read(&file)?, b"live write");
        Ok(())
    }
    #[test]
    fn selected_user_snapshot_is_seeded_only_into_the_offline_execution_identity() -> Result<()> {
        let root = state_root()?;
        let source = tempfile::tempdir()?;
        seed(source.path(), b"project");
        let user = source.path().join("apps/project/deployment-user-data");
        std::fs::create_dir_all(user.join("db/table.lance"))?;
        std::fs::write(user.join("file.txt"), b"selected user")?;
        std::fs::write(user.join("db/table.lance/data"), b"selected rows")?;
        let data = prepare(
            root.path(),
            &config(source.path()),
            &CancellationToken::new(),
            || Ok(()),
        )?;
        assert_eq!(
            std::fs::read(data.join("user/users/local/apps/project/file.txt"))?,
            b"selected user"
        );
        assert_eq!(
            std::fs::read(data.join("user/users/local/apps/project/db/table.lance/data"))?,
            b"selected rows"
        );
        assert!(!data.join("user/private").exists());
        assert!(!data.join("apps/project/deployment-user-data").exists());
        Ok(())
    }

    #[test]
    #[cfg(unix)]
    fn failed_or_interrupted_initialization_never_publishes_partial_data() -> Result<()> {
        let root = state_root()?;
        let source = tempfile::tempdir()?;
        seed(source.path(), b"initial");
        let config = config(source.path());
        let cancel = CancellationToken::new();
        let link = source.path().join("apps/project/storage/link");
        std::os::unix::fs::symlink(source.path().join("user"), &link)?;
        assert!(prepare(root.path(), &config, &cancel, || Ok(())).is_err());
        assert!(
            !root
                .path()
                .join("placement-data/placement/current")
                .exists()
        );
        std::fs::remove_file(link)?;
        assert!(
            prepare(root.path(), &config, &cancel, || -> Result<()> {
                anyhow::bail!("stale launch")
            })
            .is_err()
        );
        assert!(
            !root
                .path()
                .join("placement-data/placement/current")
                .exists()
        );
        let staging = root.path().join("placement-data/placement/staging");
        directory(&staging)?;
        vault::write_new_private(&staging.join("partial"), b"interrupted")?;
        let data = prepare(root.path(), &config, &cancel, || Ok(()))?;
        assert_eq!(
            std::fs::read(data.join("apps/project/storage/db/table"))?,
            b"initial"
        );
        assert!(!staging.exists());
        Ok(())
    }

    #[test]
    fn damaged_published_data_never_reseeds_or_creates_an_empty_store() -> Result<()> {
        let root = state_root()?;
        let source = tempfile::tempdir()?;
        seed(source.path(), b"initial");
        let config = config(source.path());
        let cancel = CancellationToken::new();
        let data = prepare(root.path(), &config, &cancel, || Ok(()))?;
        std::fs::remove_dir_all(&data)?;
        assert!(validate_root(&data, &config).is_err());
        assert!(prepare(root.path(), &config, &cancel, || Ok(())).is_err());
        assert!(!data.exists());
        std::fs::remove_dir_all(data.parent().unwrap())?;
        assert!(prepare(root.path(), &config, &cancel, || Ok(())).is_err());
        assert!(!data.exists());
        Ok(())
    }

    #[test]
    fn publication_guard_survives_rename_and_cancellation_after_acquiring_it_prevents_publish()
    -> Result<()> {
        use std::sync::{
            Arc,
            atomic::{AtomicBool, Ordering},
        };
        struct Guard {
            current: PathBuf,
            observed: Arc<AtomicBool>,
        }
        impl Drop for Guard {
            fn drop(&mut self) {
                self.observed.store(
                    self.current.join("binding.json").is_file(),
                    Ordering::SeqCst,
                );
            }
        }
        let root = state_root()?;
        let source = tempfile::tempdir()?;
        seed(source.path(), b"initial");
        let config = config(source.path());
        let cancel = CancellationToken::new();
        let observed = Arc::new(AtomicBool::new(false));
        let current = root.path().join("placement-data/placement/current");
        prepare(root.path(), &config, &cancel, || {
            Ok(Guard {
                current,
                observed: observed.clone(),
            })
        })?;
        assert!(observed.load(Ordering::SeqCst));
        let other = state_root()?;
        assert!(
            prepare(other.path(), &config, &cancel, || {
                cancel.cancel();
                Ok(())
            })
            .is_err()
        );
        assert!(
            !other
                .path()
                .join("placement-data/placement/current")
                .exists()
        );
        Ok(())
    }

    #[test]
    #[cfg(unix)]
    fn first_seeding_leaves_the_legacy_training_ledger_on_its_device() -> Result<()> {
        let root = state_root()?;
        let source = tempfile::tempdir()?;
        seed(source.path(), b"initial");
        let inspection = source.path().join("apps/project/storage/inspection");
        for file in [
            "jobs.sqlite",
            "jobs.sqlite-wal",
            "jobs.sqlite-shm",
            "jobs.sqlite-journal",
            "jobs.artifacts/blob.blob",
            ".training-locks/job.lock",
            "pki/trusted/cert.der",
            "jobs.sqlite.bak",
        ] {
            let path = inspection.join(file);
            std::fs::create_dir_all(path.parent().unwrap())?;
            std::fs::write(path, b"ledger")?;
        }
        let data = prepare(
            root.path(),
            &config(source.path()),
            &CancellationToken::new(),
            || Ok(()),
        )?;
        let seeded = data.join("apps/project/storage/inspection");
        for skipped in [
            "jobs.sqlite",
            "jobs.sqlite-wal",
            "jobs.sqlite-shm",
            "jobs.sqlite-journal",
            "jobs.artifacts",
            ".training-locks",
        ] {
            assert!(
                !seeded.join(skipped).exists(),
                "{skipped} stays on its device"
            );
        }
        assert_eq!(
            std::fs::read(seeded.join("pki/trusted/cert.der"))?,
            b"ledger"
        );
        assert_eq!(std::fs::read(seeded.join("jobs.sqlite.bak"))?, b"ledger");
        assert_eq!(
            std::fs::read(data.join("apps/project/storage/db/table"))?,
            b"initial"
        );
        Ok(())
    }

    #[test]
    #[cfg(unix)]
    fn new_revisions_publish_new_model_folders_and_newer_pointers_by_renaming() -> Result<()> {
        use std::os::unix::fs::MetadataExt;
        let revisions = Revisions::new()?;
        let first = app_models(revisions.first.path());
        model_folder(&first, "model-a", b"alpha", b"alpha");
        champion(&first, "project-l", (1, 1), Some("model-a"));
        let second = app_models(revisions.second.path());
        model_folder(&second, "model-a", b"alpha", b"alpha");
        model_folder(&second, "model-b", b"beta", b"beta");
        champion(&second, "project-l", (1, 2), Some("model-b"));
        experiment(&second, "experiment-f", "model-b");
        let staged = revisions.staging().join("app");
        let models = app_models(&revisions.store());
        let inode = |path: PathBuf| std::fs::metadata(path).map(|metadata| metadata.ino());
        let staged_inodes = std::cell::Cell::new((0, 0));
        revisions.update(
            |_| Ok(()),
            || {
                for file in [
                    "artifacts/model-b/manifest.json",
                    "experiments/experiment-f.json",
                ] {
                    assert!(
                        staged.join(file).is_file(),
                        "{file} is staged before the guard"
                    );
                }
                assert!(!staged.join("artifacts/model-a").exists());
                assert!(!models.join("artifacts/model-b").exists());
                staged_inodes.set((
                    inode(staged.join("artifacts/model-b/weights.bin"))?,
                    inode(staged.join("learning/project-l.json"))?,
                ));
                Ok(())
            },
        )?;
        assert_eq!(
            staged_inodes.get(),
            (
                inode(models.join("artifacts/model-b/weights.bin"))?,
                inode(models.join("learning/project-l.json"))?,
            ),
            "the guard only renames staged files"
        );
        assert_eq!(
            std::fs::read(models.join("artifacts/model-b/weights.bin"))?,
            b"beta"
        );
        let learning = read_json(&models.join("learning/project-l.json"));
        assert_eq!(learning["project_generation"], 2);
        let winner = read_json(&models.join("experiments/experiment-f.json"));
        assert_eq!(winner["artifact_id"], "model-b");
        assert!(!revisions.staging().exists());
        Ok(())
    }

    #[test]
    #[cfg(unix)]
    fn new_revisions_never_replace_newer_device_state() -> Result<()> {
        let revisions = Revisions::new()?;
        let first = app_models(revisions.first.path());
        model_folder(&first, "model-a", b"alpha", b"alpha");
        champion(&first, "project-m", (2, 0), Some("model-a"));
        deployment(&first, "deployment-d", 5, "model-a");
        experiment(&first, "experiment-e", "model-a");
        let second = app_models(revisions.second.path());
        model_folder(&second, "model-b", b"beta", b"beta");
        champion(&second, "project-m", (1, 9), Some("model-b"));
        deployment(&second, "deployment-d", 6, "model-b");
        experiment(&second, "experiment-e", "model-b");
        let data = revisions.update(
            |data| {
                let models = app_models(data);
                std::fs::write(data.join("apps/project/storage/db/table"), b"live write")?;
                std::fs::create_dir_all(models.join("artifacts/device-x"))?;
                std::fs::write(models.join("artifacts/device-x/weights.bin"), b"device")?;
                deployment(&models, "deployment-d", 7, "device-x");
                Ok(())
            },
            || Ok(()),
        )?;
        let models = app_models(&data);
        for (file, field, kept) in [
            (
                "learning/project-m.json",
                "deployment_generation",
                serde_json::json!(2),
            ),
            (
                "deployments/deployment-d.json",
                "generation",
                serde_json::json!(7),
            ),
            (
                "experiments/experiment-e.json",
                "artifact_id",
                serde_json::json!("model-a"),
            ),
        ] {
            assert_eq!(
                read_json(&models.join(file))[field],
                kept,
                "{file} keeps its state"
            );
        }
        assert_eq!(
            std::fs::read(models.join("artifacts/model-b/weights.bin"))?,
            b"beta"
        );
        let device = std::fs::read(models.join("artifacts/device-x/weights.bin"))?;
        assert_eq!(device, b"device");
        let table = std::fs::read(data.join("apps/project/storage/db/table"))?;
        assert_eq!(table, b"live write");
        Ok(())
    }

    #[test]
    #[cfg(unix)]
    fn selected_user_models_refresh_into_the_offline_execution_identity() -> Result<()> {
        let revisions = Revisions::new()?;
        let user_models = revisions
            .second
            .path()
            .join("apps/project")
            .join(USER_DATA_NAMESPACE)
            .join("models");
        model_folder(&user_models, "private-model", b"private", b"private");
        champion(
            &user_models,
            "private-project",
            (1, 1),
            Some("private-model"),
        );
        let data = revisions.update(|_| Ok(()), || Ok(()))?;
        let local = data.join("user/users/local/apps/project/models");
        let weights = std::fs::read(local.join("artifacts/private-model/weights.bin"))?;
        assert_eq!(weights, b"private");
        let learning = read_json(&local.join("learning/private-project.json"));
        assert_eq!(learning["artifact_id"], "private-model");
        assert!(!app_models(&data).exists());
        assert!(!data.join("apps/project").join(USER_DATA_NAMESPACE).exists());
        Ok(())
    }

    #[test]
    #[cfg(unix)]
    fn artifact_folders_failing_verification_and_pointers_to_them_are_not_refreshed() -> Result<()>
    {
        let revisions = Revisions::new()?;
        let models = app_models(revisions.second.path());
        model_folder(&models, "tampered", b"changed", b"trained");
        champion(&models, "project-tampered", (1, 1), Some("tampered"));
        model_folder(&models, "intact", b"trained", b"trained");
        champion(&models, "project-intact", (1, 1), Some("intact"));
        champion(&models, "project-missing", (1, 1), Some("missing"));
        champion(&models, "project-teacher", (1, 1), None);
        let published = app_models(&revisions.update(|_| Ok(()), || Ok(()))?);
        for missing in [
            "artifacts/tampered",
            "learning/project-tampered.json",
            "learning/project-missing.json",
        ] {
            assert!(
                !published.join(missing).exists(),
                "{missing} is not refreshed"
            );
        }
        for present in [
            "artifacts/intact/weights.bin",
            "learning/project-intact.json",
            "learning/project-teacher.json",
        ] {
            assert!(published.join(present).is_file(), "{present} is refreshed");
        }
        Ok(())
    }

    #[test]
    #[cfg(unix)]
    fn cancellation_after_staging_publishes_no_models() -> Result<()> {
        let revisions = Revisions::new()?;
        let second = app_models(revisions.second.path());
        model_folder(&second, "model-b", b"beta", b"beta");
        let cancelled = revisions.update(
            |_| Ok(()),
            || {
                revisions.cancel.cancel();
                Ok(())
            },
        );
        assert!(cancelled.is_err());
        assert!(!app_models(&revisions.store()).exists());
        assert!(!revisions.staging().exists());
        Ok(())
    }

    #[test]
    #[cfg(unix)]
    fn online_placements_never_refresh_published_models() -> Result<()> {
        let root = state_root()?;
        let empty = tempfile::tempdir()?;
        let snapshot = tempfile::tempdir()?;
        seed(snapshot.path(), b"cloud");
        model_folder(&app_models(snapshot.path()), "model-a", b"alpha", b"alpha");
        let mut config = config(empty.path());
        config.source = ProjectSource::Online;
        config.online_metadata_sha256 = Some("a".repeat(64));
        config.resource_grant = Some(crate::config::ResourceGrantRef {
            grant_id: "grant".into(),
            authz_version: 1,
            billing_grant_id: None,
            billing_authz_version: None,
        });
        let cancel = CancellationToken::new();
        let data = prepare(root.path(), &config, &cancel, || Ok(()))?;
        next_revision(&mut config, snapshot.path());
        assert_eq!(prepare(root.path(), &config, &cancel, || Ok(()))?, data);
        assert!(!app_models(&data).exists());
        assert!(
            !root
                .path()
                .join("placement-data/placement")
                .join(published_models::MODELS_STAGING)
                .exists()
        );
        Ok(())
    }
}
