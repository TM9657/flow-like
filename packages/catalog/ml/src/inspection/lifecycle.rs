#[cfg(feature = "training")]
use super::publisher::{PublishOutcome, PublishReason};
use flow_like::flow::{execution::context::ExecutionContext, node::NodeLogic};
use flow_like_ml_core::{ComputeConfig, InspectionSpec, Sample, TensorData};
#[cfg(feature = "inference")]
use flow_like_ml_runtime as runtime;
use flow_like_ml_runtime::*;
#[cfg(any(feature = "inference", test))]
use flow_like_types::anyhow;
#[cfg(feature = "inference")]
use flow_like_types::json::json;
use flow_like_types::{Result, Value};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
#[cfg(feature = "training")]
use std::path::{Path, PathBuf};

/// Which device-local ledger holds a record: the app's shared ledger or the executing
/// user's private ledger. The serde form is the `ledger_scope` marker value.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum LedgerScope {
    #[default]
    App,
    User,
}

/// Streams minted by Auto Train (`auto:{uuid}`) and learning projects (`learning:{uuid}`).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum StreamKind {
    Auto,
    Learning,
}

/// Marker key at experiment `request.context` and learning project `request.source`.
pub const LEDGER_SCOPE_KEY: &str = "ledger_scope";

const USER_STREAM_PREFIX: &str = "user:";
const DEPLOYMENT_PREFIX: &str = "deployment:";
const USER_PROJECT_SUFFIX: &str = ":user";

impl LedgerScope {
    /// Only canonical minted user streams (`user:auto:{uuid}`, `user:learning:{uuid}`) are
    /// private; manual streams such as `user:camera` stay in the App ledger.
    pub fn of_stream(stream_id: &str) -> Self {
        match stream_id.strip_prefix(USER_STREAM_PREFIX) {
            Some(minted) if StreamKind::of_minted(minted).is_some() => Self::User,
            _ => Self::App,
        }
    }

    pub fn of_deployment(deployment_id: &str) -> Self {
        deployment_id
            .strip_prefix(DEPLOYMENT_PREFIX)
            .map_or(Self::App, Self::of_stream)
    }

    pub fn of_project(project_id: &str) -> Self {
        match project_id.strip_suffix(USER_PROJECT_SUFFIX) {
            Some(app_and_board) if app_and_board.contains(':') => Self::User,
            _ => Self::App,
        }
    }

    pub fn project_id(self, app_id: &str, board_id: &str) -> String {
        match self {
            Self::App => format!("{app_id}:{board_id}"),
            Self::User => format!("{app_id}:{board_id}{USER_PROJECT_SUFFIX}"),
        }
    }

    pub fn new_stream_id(self, kind: StreamKind) -> String {
        let scope = match self {
            Self::App => "",
            Self::User => USER_STREAM_PREFIX,
        };
        format!("{scope}{}{}", kind.prefix(), uuid::Uuid::new_v4())
    }
}

impl StreamKind {
    /// The kind of a minted stream id in either scope; manual streams have none.
    pub fn of_stream(stream_id: &str) -> Option<Self> {
        Self::of_minted(
            stream_id
                .strip_prefix(USER_STREAM_PREFIX)
                .unwrap_or(stream_id),
        )
    }

    fn of_minted(stream_id: &str) -> Option<Self> {
        [Self::Auto, Self::Learning].into_iter().find(|kind| {
            stream_id
                .strip_prefix(kind.prefix())
                .is_some_and(is_canonical_uuid)
        })
    }

    fn prefix(self) -> &'static str {
        match self {
            Self::Auto => "auto:",
            Self::Learning => "learning:",
        }
    }
}

fn is_canonical_uuid(value: &str) -> bool {
    value.len() == 36
        && value.bytes().enumerate().all(|(index, byte)| match index {
            8 | 13 | 18 | 23 => byte == b'-',
            _ => matches!(byte, b'0'..=b'9' | b'a'..=b'f'),
        })
}

pub const TRAINING_UNAVAILABLE: &str = "Training needs a device-local training ledger, which only the desktop app and offline device placements keep. Server executors, online device placements and desktop runs of online apps without a hub connection cannot train. Models already published under storage/models still run here through Predict Auto Model, Predict Published Model, Student Model Inference, Extract Image Features, Extract Pretrained Image Features, Extract Published Image Features, Route Learning Project, Route Inspection, Get Active Student and Segment with SAM 2.";

/// The refusal of every training operation on a runtime without a device-local ledger.
#[derive(Debug)]
pub struct TrainingUnavailable;

impl std::fmt::Display for TrainingUnavailable {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str(TRAINING_UNAVAILABLE)
    }
}

impl std::error::Error for TrainingUnavailable {}

pub fn training_unavailable() -> flow_like_types::Error {
    flow_like_types::Error::new(TrainingUnavailable)
}

pub fn is_training_unavailable(error: &flow_like_types::Error) -> bool {
    error.downcast_ref::<TrainingUnavailable>().is_some()
}

/// A changed specification creates a new training stream, including changed label order.
#[derive(Clone, Serialize, Deserialize, JsonSchema)]
pub struct InspectionContext {
    pub stream_id: String,
    pub spec: InspectionSpec,
}
impl InspectionContext {
    #[cfg(any(feature = "inference", test))]
    pub(crate) fn key(&self, project_id: String) -> Result<StreamKey> {
        let inspection_version = self.inspection_version()?;
        if self.stream_id.trim().is_empty() {
            return Err(anyhow!("Inspection stream ID is required"));
        }
        Ok(StreamKey {
            project_id,
            stream_id: self.stream_id.clone(),
            inspection_version,
        })
    }

    /// Digest of the validated specification; equal specifications share a version.
    #[cfg_attr(not(any(feature = "inference", test)), allow(dead_code))]
    pub(crate) fn inspection_version(&self) -> Result<String> {
        self.spec.validate()?;
        Ok(flow_like_ml_core::content_digest(
            &flow_like_types::json::to_vec(&self.spec)?,
        ))
    }
}

/// Where a ledger database lives in the Local app-storage store.
#[cfg(feature = "training")]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum LedgerSite {
    /// `training-ledgers/apps/{app}/project`
    App,
    /// `apps/{app}/storage/inspection`, used in place while no App ledger exists (D16)
    LegacyApp,
    /// `training-ledgers/apps/{app}/users/{sub}`
    User,
}

/// An app-storage location taken from the execution context. Resolving it touches the
/// filesystem, so it can move to a blocking thread.
#[cfg(feature = "training")]
pub(crate) struct LocalStorageTarget {
    store: std::sync::Arc<flow_like_storage::files::store::local_store::LocalObjectStore>,
    location: flow_like_storage::Path,
}
#[cfg(feature = "training")]
impl LocalStorageTarget {
    /// Shadow runs and app stores that are not Local cannot hold a ledger.
    pub(crate) fn new(context: &ExecutionContext, site: LedgerSite) -> Result<Self> {
        use flow_like_storage::files::store::FlowLikeStore;
        use flow_like_types::storage_paths::training_ledger_dir;
        let cache = context
            .execution_cache
            .as_ref()
            .ok_or_else(|| anyhow!("Execution storage context is unavailable"))?;
        if cache.shadow {
            return Err(anyhow!(
                "Training lifecycle operations require a live workflow. Run model comparisons through the explicit prediction recording node in a live workflow"
            ));
        }
        let Some(FlowLikeStore::Local(store)) = &cache.stores.app_storage_store else {
            return Err(training_unavailable());
        };
        let directory = match site {
            LedgerSite::App => {
                flow_like_storage::Path::from(training_ledger_dir(&cache.app_id, None))
            }
            LedgerSite::LegacyApp => cache.get_storage(false)?.join("inspection"),
            LedgerSite::User => flow_like_storage::Path::from(training_ledger_dir(
                &cache.app_id,
                Some(cache.sub.as_str()),
            )),
        };
        Ok(Self {
            store: store.clone(),
            location: directory.join(LEDGER_DATABASE_FILE),
        })
    }

    pub(crate) fn resolve(&self) -> Result<std::path::PathBuf> {
        let root = self
            .store
            .directory_to_filesystem(&flow_like_storage::Path::from(""))?
            .canonicalize()?;
        let path = self.store.directory_to_filesystem(&self.location)?;
        let mut ancestor = path.as_path();
        while !ancestor.try_exists()? {
            if std::fs::symlink_metadata(ancestor).is_ok() {
                return Err(anyhow!("Training path contains a dangling symlink"));
            }
            ancestor = ancestor
                .parent()
                .ok_or_else(|| anyhow!("Training path has no parent"))?;
        }
        if !ancestor.canonicalize()?.starts_with(root) {
            return Err(anyhow!("Training path escapes app storage"));
        }
        Ok(path)
    }
}

/// The workflow's training ledger, captured on the executor and opened on a blocking thread.
#[cfg(feature = "training")]
pub(crate) struct LedgerLocation {
    target: LocalStorageTarget,
    /// App scope only: the legacy ledger, used in place while `target` does not exist (D16).
    legacy: Option<LocalStorageTarget>,
    project: String,
    scope: LedgerScope,
}
#[cfg(feature = "training")]
impl LedgerLocation {
    /// Captures the locations without touching the filesystem.
    pub(crate) fn new(context: &ExecutionContext, scope: LedgerScope) -> Result<Self> {
        let (target, legacy) = match scope {
            LedgerScope::App => (
                LocalStorageTarget::new(context, LedgerSite::App)?,
                Some(LocalStorageTarget::new(context, LedgerSite::LegacyApp)?),
            ),
            LedgerScope::User => (LocalStorageTarget::new(context, LedgerSite::User)?, None),
        };
        let cache = context
            .execution_cache
            .as_ref()
            .ok_or_else(|| anyhow!("Execution storage context is unavailable"))?;
        Ok(Self {
            target,
            legacy,
            project: scope.project_id(&cache.app_id, &cache.board_id),
            scope,
        })
    }

    /// The ledger a lifecycle stream routes to.
    pub(crate) fn for_stream(context: &ExecutionContext, stream_id: &str) -> Result<Self> {
        Self::new(context, LedgerScope::of_stream(stream_id))
    }

    pub(crate) fn scope(&self) -> LedgerScope {
        self.scope
    }

    pub(crate) fn project(&self) -> &str {
        &self.project
    }

    /// Resolves and checks the database path; creates nothing. The App scope takes the App
    /// ledger when it exists, else the legacy ledger when that exists, else the App ledger.
    pub(crate) fn database_path(&self) -> Result<PathBuf> {
        let path = self.target.resolve()?;
        let Some(legacy) = &self.legacy else {
            return Ok(path);
        };
        if path.try_exists()? {
            return Ok(path);
        }
        let legacy = legacy.resolve()?;
        Ok(if legacy.try_exists()? { legacy } else { path })
    }

    /// The directory of the database, its blob directory and its locks; creates nothing.
    pub(crate) fn directory(&self) -> Result<std::path::PathBuf> {
        let database = self.database_path()?;
        database
            .parent()
            .map(std::path::Path::to_path_buf)
            .ok_or_else(|| {
                anyhow!(
                    "Training ledger path {} has no parent directory",
                    database.display()
                )
            })
    }

    pub(crate) fn exists(&self) -> Result<bool> {
        Ok(self.database_path()?.try_exists()?)
    }

    /// Blocks on the filesystem and, the first time this process opens the ledger, on SQLite.
    /// Creates the ledger when it is missing.
    pub(crate) fn open(self) -> Result<(TrainingRepository, String)> {
        Ok((open_ledger(self.database_path()?)?, self.project))
    }

    /// Opens the ledger only when its database exists; creates nothing otherwise.
    pub(crate) fn open_existing(self) -> Result<Option<(TrainingRepository, String)>> {
        if !self.exists()? {
            return Ok(None);
        }
        self.open().map(Some)
    }
}

/// Opens the workflow's App ledger from async code. The first open of a ledger in a process
/// waits on SQLite, so this executor thread hands its other tasks on while it waits.
#[cfg(feature = "training")]
pub(crate) fn repository(context: &ExecutionContext) -> Result<(TrainingRepository, String)> {
    scoped_repository(context, LedgerScope::App)
}

/// Opens the ledger of `scope` from async code, creating it when missing.
#[cfg(feature = "training")]
pub(crate) fn scoped_repository(
    context: &ExecutionContext,
    scope: LedgerScope,
) -> Result<(TrainingRepository, String)> {
    let ledger = LedgerLocation::new(context, scope)?;
    blocking(|| ledger.open())
}

/// Refuses shadow runs and runs without a Local app store before any data is read.
#[cfg(feature = "training")]
pub(crate) fn ensure_training_available(context: &ExecutionContext) -> Result<()> {
    LocalStorageTarget::new(context, LedgerSite::App)?;
    Ok(())
}

/// Database paths of this run's App ledger and of the executing user's private ledger,
/// for diagnostics. `user` is None when the private ledger cannot be resolved.
#[cfg(feature = "training")]
pub(crate) struct LedgerPaths {
    pub(crate) app: std::path::PathBuf,
    pub(crate) user: Option<std::path::PathBuf>,
}

/// Resolves both ledger paths without creating anything.
#[cfg(feature = "training")]
pub(crate) fn ledger_paths(context: &ExecutionContext) -> Result<LedgerPaths> {
    let app = LedgerLocation::new(context, LedgerScope::App)?.database_path()?;
    let user = LedgerLocation::new(context, LedgerScope::User)
        .and_then(|ledger| ledger.database_path())
        .ok();
    Ok(LedgerPaths { app, user })
}

/// A ledger that holds a looked-up record, with the project string of its records. Only the
/// training feature finds ledgers.
#[cfg(feature = "inference")]
#[cfg_attr(not(feature = "training"), allow(dead_code))]
pub(crate) struct LocatedLedger {
    pub(crate) repo: TrainingRepository,
    pub(crate) project: String,
    pub(crate) scope: LedgerScope,
}

/// The first existing ledger whose `holds(repo, project)` is true: the resolved App ledger,
/// then the legacy App ledger when it exists and differs, then the private ledger when it
/// exists. Nothing is created.
#[cfg(feature = "training")]
pub(crate) fn locate_repository(
    context: &ExecutionContext,
    holds: impl Fn(&TrainingRepository, &str) -> Result<bool>,
) -> Result<Option<LocatedLedger>> {
    let chain = LedgerChain::new(context)?;
    blocking(|| chain.locate(holds))
}

/// `locate_repository` for inference paths. None without the training feature, in shadow
/// runs, for non-Local app stores and when no ledger holds the record.
#[cfg(feature = "inference")]
pub(crate) fn locate_repository_for_inference(
    context: &ExecutionContext,
    holds: impl Fn(&TrainingRepository, &str) -> Result<bool>,
) -> Result<Option<LocatedLedger>> {
    #[cfg(feature = "training")]
    {
        match inference_location(context, LedgerChain::new)? {
            Some(chain) => blocking(|| chain.locate(holds)),
            None => Ok(None),
        }
    }
    #[cfg(not(feature = "training"))]
    {
        let _ = (context, holds);
        Ok(None)
    }
}

/// The routed ledger of an inspection stream when it exists, for inference nodes. None where
/// `locate_repository_for_inference` gives None; nothing is created.
#[cfg(feature = "training")]
fn routed_ledger_for_inference(
    context: &ExecutionContext,
    stream_id: &str,
) -> Result<Option<LocatedLedger>> {
    let Some(ledger) = inference_location(context, |context| {
        LedgerLocation::for_stream(context, stream_id)
    })?
    else {
        return Ok(None);
    };
    let scope = ledger.scope();
    Ok(blocking(|| ledger.open_existing())?
        .map(|(repo, project)| LocatedLedger {
            repo,
            project,
            scope,
        }))
}

/// A ledger location for an inference read: None in shadow runs and without a Local app store.
#[cfg(feature = "training")]
fn inference_location<T>(
    context: &ExecutionContext,
    locate: impl FnOnce(&ExecutionContext) -> Result<T>,
) -> Result<Option<T>> {
    if context
        .execution_cache
        .as_ref()
        .is_some_and(|cache| cache.shadow)
    {
        return Ok(None);
    }
    match locate(context) {
        Ok(location) => Ok(Some(location)),
        Err(error) if is_training_unavailable(&error) => Ok(None),
        Err(error) => Err(error),
    }
}

/// The ledgers a record looked up by id may live in.
#[cfg(feature = "training")]
struct LedgerChain {
    app: LedgerLocation,
    user: LedgerLocation,
}
#[cfg(feature = "training")]
impl LedgerChain {
    fn new(context: &ExecutionContext) -> Result<Self> {
        Ok(Self {
            app: LedgerLocation::new(context, LedgerScope::App)?,
            user: LedgerLocation::new(context, LedgerScope::User)?,
        })
    }

    /// The database paths that exist, in lookup order, with the ledger each belongs to.
    fn lookup_ledgers(&self) -> Result<Vec<(PathBuf, &LedgerLocation)>> {
        let app = self.app.database_path()?;
        let mut candidates = vec![(app.clone(), &self.app)];
        if let Some(legacy) = &self.app.legacy {
            let legacy = legacy.resolve()?;
            if legacy != app {
                candidates.push((legacy, &self.app));
            }
        }
        candidates.push((self.user.database_path()?, &self.user));
        let mut existing = Vec::with_capacity(candidates.len());
        for (path, ledger) in candidates {
            if path.try_exists()? {
                existing.push((path, ledger));
            }
        }
        Ok(existing)
    }

    fn locate(
        &self,
        holds: impl Fn(&TrainingRepository, &str) -> Result<bool>,
    ) -> Result<Option<LocatedLedger>> {
        for (path, ledger) in self.lookup_ledgers()? {
            let repo = open_ledger(path.clone()).map_err(|error| {
                error.context(format!("Opening training ledger {}", path.display()))
            })?;
            if holds(&repo, ledger.project())? {
                return Ok(Some(LocatedLedger {
                    repo,
                    project: ledger.project.clone(),
                    scope: ledger.scope,
                }));
            }
        }
        Ok(None)
    }
}

/// Whether a ledger holds model artifact `id` of its own project; another project's row
/// counts as missing.
#[cfg(feature = "inference")]
pub(crate) fn holds_artifact(repo: &TrainingRepository, project: &str, id: &str) -> Result<bool> {
    match repo.get_artifact(id) {
        Ok(artifact) => Ok(artifact.stream.project_id == project),
        Err(runtime::Error::NotFound(_)) => Ok(false),
        Err(error) => Err(error.into()),
    }
}

/// Whether a ledger holds pretrained source `id` of its own project; another project's
/// source counts as missing.
#[cfg(feature = "inference")]
pub(crate) fn holds_pretrained_source(
    repo: &TrainingRepository,
    project: &str,
    id: &str,
) -> Result<bool> {
    match repo.get_pretrained_source(id) {
        Ok(source) => Ok(source.project_id == project),
        Err(runtime::Error::NotFound(_)) => Ok(false),
        Err(error) => Err(error.into()),
    }
}

/// Logs a publish that did not happen; publishing never fails the calling node.
#[cfg(feature = "training")]
pub(crate) fn warn_unpublished(
    context: &mut ExecutionContext,
    model: &str,
    outcome: &PublishOutcome,
) {
    let message = match outcome {
        PublishOutcome::Failed { error } => {
            format!("{model} could not be published as model files: {error}")
        }
        PublishOutcome::Skipped { reason } => {
            format!("{model} was not published as model files: {reason}")
        }
        PublishOutcome::Published(_) | PublishOutcome::Unchanged => return,
    };
    tracing::warn!("{message}");
    context.log_message(&message, flow_like::flow::execution::LogLevel::Warn);
}

/// Training of one app that a drain cancelled, and what was still running at its deadline.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct DrainReport {
    pub cancelled: usize,
    pub still_running: usize,
}

/// Stops accepting new training for `app_id`, cancels its queued and running jobs and
/// experiments in its ledgers under `project_root` (`training_ledger_app_dir(app_id)` and the
/// legacy `apps/{app_id}/storage/inspection/jobs.sqlite`), and waits for them up to
/// `timeout`. A no-op without the training feature.
pub async fn drain_app_training(
    project_root: &std::path::Path,
    app_id: &str,
    timeout: std::time::Duration,
) -> DrainReport {
    #[cfg(feature = "training")]
    {
        const LONGEST_DRAIN: std::time::Duration = std::time::Duration::from_secs(24 * 60 * 60);
        let prefixes = drain_prefixes(project_root, app_id);
        let deadline = std::time::Instant::now() + timeout.min(LONGEST_DRAIN);
        let _closed = JOBS.close(&prefixes);
        let jobs = tokio::task::spawn_blocking({
            let prefixes = prefixes.clone();
            move || drain_jobs(&prefixes, deadline)
        });
        let experiments = tokio::task::spawn_blocking(move || {
            super::auto_training::drain_experiments(&prefixes, deadline)
        });
        let (jobs, experiments) = tokio::join!(jobs, experiments);
        [jobs, experiments]
            .into_iter()
            .map(|drained| {
                drained.unwrap_or_else(|error| {
                    tracing::warn!(app_id, %error, "Draining the app's training stopped unexpectedly");
                    DrainReport {
                        cancelled: 0,
                        still_running: 1,
                    }
                })
            })
            .fold(DrainReport::default(), |total, report| DrainReport {
                cancelled: total.cancelled + report.cancelled,
                still_running: total.still_running + report.still_running,
            })
    }
    #[cfg(not(feature = "training"))]
    {
        let _ = (project_root, app_id, timeout);
        DrainReport::default()
    }
}

/// Cancels the lifecycle jobs queued or running on `ledgers` and waits until `deadline`.
/// `ledgers` are canonical prefixes: a ledger directory tree or a ledger database file.
/// Waiting jobs are withdrawn from the worker, and no job starts under them meanwhile.
#[cfg(feature = "training")]
pub(crate) fn drain_jobs(ledgers: &[PathBuf], deadline: std::time::Instant) -> DrainReport {
    let _closed = JOBS.close(ledgers);
    JOBS.drain(ledgers, deadline)
}

/// The canonical prefixes of an app's ledgers under `project_root`: its directory under
/// `training-ledgers` and its legacy ledger file.
#[cfg(feature = "training")]
fn drain_prefixes(project_root: &Path, app_id: &str) -> Vec<PathBuf> {
    let ledgers = flow_like_types::storage_paths::training_ledger_app_dir(app_id)
        .split('/')
        .fold(project_root.to_path_buf(), |path, part| path.join(part));
    let mut prefixes = vec![canonical_prefix(&ledgers)];
    let mut parts = Path::new(app_id).components();
    if matches!(
        (parts.next(), parts.next()),
        (Some(std::path::Component::Normal(_)), None)
    ) {
        let legacy = project_root
            .join("apps")
            .join(app_id)
            .join("storage")
            .join("inspection")
            .join(LEDGER_DATABASE_FILE);
        prefixes.push(canonical_prefix(&legacy));
    }
    prefixes
}

/// `path` with its deepest existing ancestor canonicalized, so it compares equal to the
/// canonical paths of ledgers created below it later.
#[cfg(feature = "training")]
fn canonical_prefix(path: &Path) -> PathBuf {
    let mut missing = Vec::new();
    let mut existing = path;
    loop {
        if let Ok(canonical) = existing.canonicalize() {
            return missing
                .iter()
                .rev()
                .fold(canonical, |path, part| path.join(part));
        }
        match (existing.parent(), existing.file_name()) {
            (Some(parent), Some(name)) => {
                missing.push(name);
                existing = parent;
            }
            _ => return path.to_path_buf(),
        }
    }
}

/// A job dispatched to this process's training worker.
#[cfg(feature = "training")]
struct QueuedJob {
    /// Canonical path of the ledger database, compared with drain prefixes.
    ledger: PathBuf,
    repo: TrainingRepository,
    job_id: String,
}

/// Jobs dispatched to the training worker by queue key, the ones it runs, and the ledger
/// prefixes a drain closed to new jobs.
#[cfg(feature = "training")]
#[derive(Default)]
struct JobQueue {
    state: std::sync::Mutex<QueueState>,
    changed: std::sync::Condvar,
}

#[cfg(feature = "training")]
#[derive(Default)]
struct QueueState {
    admitted: std::collections::HashMap<String, QueuedJob>,
    running: std::collections::HashSet<String>,
    draining: std::collections::HashMap<PathBuf, usize>,
}

#[cfg(feature = "training")]
static JOBS: std::sync::LazyLock<JobQueue> = std::sync::LazyLock::new(JobQueue::default);

/// Keeps ledger prefixes closed to new jobs while it lives.
#[cfg(feature = "training")]
struct ClosedPrefixes<'a> {
    queue: &'a JobQueue,
    prefixes: Vec<PathBuf>,
}

#[cfg(feature = "training")]
impl Drop for ClosedPrefixes<'_> {
    fn drop(&mut self) {
        let mut state = self.queue.state();
        for prefix in &self.prefixes {
            if let Some(holders) = state.draining.get_mut(prefix) {
                *holders -= 1;
                if *holders == 0 {
                    state.draining.remove(prefix);
                }
            }
        }
    }
}

#[cfg(feature = "training")]
impl JobQueue {
    fn state(&self) -> std::sync::MutexGuard<'_, QueueState> {
        self.state
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
    }

    /// False when the job already waits or runs; refused under a closed prefix.
    fn admit(&self, key: &str, job: QueuedJob) -> Result<bool> {
        let mut state = self.state();
        if let Some(prefix) = state
            .draining
            .keys()
            .find(|prefix| job.ledger.starts_with(prefix))
        {
            return Err(anyhow!(
                "Training job {} was not started because the app's training under {} is being stopped before its data is deleted",
                job.job_id,
                prefix.display()
            ));
        }
        if state.admitted.contains_key(key) {
            return Ok(false);
        }
        state.admitted.insert(key.to_owned(), job);
        Ok(true)
    }

    /// Whether the worker runs the job; a drain withdraws the jobs that still wait.
    fn start(&self, key: &str) -> bool {
        let mut state = self.state();
        state.admitted.contains_key(key) && state.running.insert(key.to_owned())
    }

    fn finish(&self, key: &str) {
        let mut state = self.state();
        state.admitted.remove(key);
        state.running.remove(key);
        drop(state);
        self.changed.notify_all();
    }

    fn close(&self, prefixes: &[PathBuf]) -> ClosedPrefixes<'_> {
        let mut state = self.state();
        for prefix in prefixes {
            *state.draining.entry(prefix.clone()).or_default() += 1;
        }
        ClosedPrefixes {
            queue: self,
            prefixes: prefixes.to_vec(),
        }
    }

    /// Withdraws the waiting jobs under `prefixes`, cancels them and the running ones, and
    /// waits until the running ones finish or `deadline` passes.
    fn drain(&self, prefixes: &[PathBuf], deadline: std::time::Instant) -> DrainReport {
        let mut drained = Vec::new();
        {
            let mut state = self.state();
            let keys: Vec<String> = state
                .admitted
                .iter()
                .filter(|(_, job)| prefixes.iter().any(|prefix| job.ledger.starts_with(prefix)))
                .map(|(key, _)| key.clone())
                .collect();
            for key in keys {
                let job = if state.running.contains(&key) {
                    state
                        .admitted
                        .get(&key)
                        .map(|job| (job.repo.clone(), job.job_id.clone()))
                } else {
                    state.admitted.remove(&key).map(|job| (job.repo, job.job_id))
                };
                drained.extend(job.map(|(repo, job_id)| (key, repo, job_id)));
            }
        }
        let cancelled = drained
            .iter()
            .filter(|(_, repo, job_id)| cancel_dispatched(repo, job_id))
            .count();
        let mut state = self.state();
        loop {
            let still_running = drained
                .iter()
                .filter(|(key, ..)| state.admitted.contains_key(key))
                .count();
            let remaining = deadline.saturating_duration_since(std::time::Instant::now());
            if still_running == 0 || remaining.is_zero() {
                return DrainReport {
                    cancelled,
                    still_running,
                };
            }
            state = self
                .changed
                .wait_timeout(state, remaining)
                .unwrap_or_else(std::sync::PoisonError::into_inner)
                .0;
        }
    }
}

/// Cancels a dispatched job that has not finished; false when it had or the cancel failed.
#[cfg(feature = "training")]
fn cancel_dispatched(repo: &TrainingRepository, job_id: &str) -> bool {
    let cancel = || -> runtime::Result<bool> {
        if repo.get_job(job_id)?.status.is_terminal() {
            return Ok(false);
        }
        let job = repo.request_cancel(job_id, runtime::now_ms())?;
        Ok(matches!(
            job.status,
            JobStatus::Cancelled | JobStatus::CancelRequested
        ))
    };
    cancel().unwrap_or_else(|error| {
        tracing::warn!(
            job_id,
            ledger = %repo.path().display(),
            %error,
            "Draining could not cancel a training job"
        );
        false
    })
}

/// Runs blocking work called from async code. On a multi-threaded runtime the executor moves
/// this thread's other tasks to another worker first; elsewhere the work runs in place.
#[cfg(feature = "training")]
fn blocking<T>(work: impl FnOnce() -> T) -> T {
    match tokio::runtime::Handle::try_current().map(|handle| handle.runtime_flavor()) {
        Ok(tokio::runtime::RuntimeFlavor::MultiThread) => tokio::task::block_in_place(work),
        _ => work(),
    }
}

/// Opening migrates and verifies the ledger under the SQLite write lock, so each file is
/// opened once per process. A removed or replaced file is opened, and so verified, again; a
/// reused handle still has its storage entries verified on every call.
#[cfg(feature = "training")]
fn open_ledger(path: std::path::PathBuf) -> Result<TrainingRepository> {
    use std::{
        collections::HashMap,
        sync::{LazyLock, Mutex, PoisonError},
    };
    type Opened = HashMap<std::path::PathBuf, (LedgerIdentity, TrainingRepository)>;
    static OPENED: LazyLock<Mutex<Opened>> = LazyLock::new(|| Mutex::new(HashMap::new()));
    if let Some(identity) = ledger_identity(&path) {
        let cached = OPENED
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .get(&path)
            .filter(|(known, _)| *known == identity)
            .map(|(_, repo)| repo.clone());
        if let Some(repo) = cached {
            repo.verify_storage()?;
            return Ok(repo);
        }
    }
    let repo = TrainingRepository::open(&path)?;
    if let Some(identity) = ledger_identity(&path) {
        OPENED
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .insert(path, (identity, repo.clone()));
    }
    Ok(repo)
}

/// The ledger file's device and inode, with its birth time where the filesystem records one,
/// so a new file that reuses a freed inode number differs. Without inode numbers the birth
/// time alone identifies the file.
#[cfg(feature = "training")]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct LedgerIdentity {
    file: (u64, u64),
    born: Option<std::time::SystemTime>,
}

#[cfg(feature = "training")]
fn ledger_identity(path: &std::path::Path) -> Option<LedgerIdentity> {
    let metadata = std::fs::symlink_metadata(path)
        .ok()
        .filter(std::fs::Metadata::is_file)?;
    let born = metadata.created().ok();
    #[cfg(unix)]
    let file = {
        use std::os::unix::fs::MetadataExt;
        (metadata.dev(), metadata.ino())
    };
    #[cfg(not(unix))]
    let file = born.map(|_| (0, 0))?;
    Some(LedgerIdentity { file, born })
}

#[cfg(any(feature = "inference", test))]
fn check_stream(expected: &StreamKey, actual: &StreamKey) -> Result<()> {
    if expected != actual {
        return Err(anyhow!(
            "The requested resource belongs to a different inspection stream or specification"
        ));
    }
    Ok(())
}

#[derive(Clone, Serialize, Deserialize, JsonSchema)]
pub struct RecordSampleRequest {
    pub inspection: InspectionContext,
    pub sample: Sample,
    pub annotation_revision: u64,
    pub accepted: bool,
    pub label_available_at_ms: i64,
}
#[cfg(feature = "training")]
fn record_sample(
    repo: TrainingRepository,
    project: String,
    input: RecordSampleRequest,
) -> Result<SampleReceipt> {
    input.inspection.spec.validate_sample(&input.sample)?;
    if input.accepted
        && matches!(
            input.sample.annotation,
            flow_like_ml_core::Annotation::Unlabeled
        )
    {
        return Err(anyhow!(
            "An unlabelled sample cannot count as an accepted training label"
        ));
    }
    let key = input.inspection.key(project)?;
    if input.sample.stream_id != key.stream_id {
        return Err(anyhow!("Sample and inspection stream IDs differ"));
    }
    let (source, available) = match &input.sample.provenance {
        flow_like_ml_core::LabelProvenance::Teacher { .. } => {
            (LabelSource::Teacher, input.sample.timestamp_ms)
        }
        flow_like_ml_core::LabelProvenance::Reviewed { reviewed_at_ms, .. } => {
            (LabelSource::Reviewed, *reviewed_at_ms)
        }
        flow_like_ml_core::LabelProvenance::Measured { .. } => (
            LabelSource::ObservedOutcome,
            input
                .sample
                .outcome
                .as_ref()
                .map_or(input.sample.timestamp_ms, |o| o.available_at_ms),
        ),
        _ => {
            return Err(anyhow!(
                "Only labelled samples with provenance can enter training storage"
            ));
        }
    };
    if input.label_available_at_ms < available || input.label_available_at_ms > runtime::now_ms() {
        return Err(anyhow!(
            "Label availability must follow capture/review/outcome and cannot be in the future"
        ));
    }
    let digest = flow_like_ml_core::content_digest(&flow_like_types::json::to_vec(&(
        &input.sample.stream_id,
        input.sample.window_start_ms,
        input.sample.window_end_ms,
        &input.sample.input,
    ))?);
    let mut payload = json!(input.sample);
    if let flow_like_ml_core::Annotation::Class { class_id } = input.sample.annotation {
        payload["label"] = json!(input.inspection.spec.labels[class_id as usize]);
    }
    Ok(repo.record_sample(
        &key,
        &TrainingSample {
            id: input.sample.id,
            annotation_revision: input.annotation_revision,
            group_id: input.sample.group_id,
            captured_at_ms: input.sample.timestamp_ms,
            label_available_at_ms: input.label_available_at_ms,
            content_digest: digest,
            source,
            accepted: input.accepted,
            payload,
        },
    )?)
}

/// The model and image steps that computed patch embeddings, as output by Extract Image
/// Features. PatchCore and PaDiM models record it and reject embeddings from another one.
#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema, PartialEq)]
pub struct FeatureExtractorRef {
    pub source_id: String,
    pub blob_sha256: String,
    #[serde(default)]
    pub image_steps: Vec<flow_like_ml_core::PreprocessingStep>,
}
#[cfg(feature = "inference")]
impl FeatureExtractorRef {
    /// Fails unless the extractor names a source and a SHA-256 weights digest.
    pub(crate) fn to_runtime(&self) -> Result<runtime::engines::FeatureExtractor> {
        let extractor = runtime::engines::FeatureExtractor {
            source_id: self.source_id.clone(),
            blob_sha256: self.blob_sha256.clone(),
            image_steps: self.image_steps.clone(),
        };
        extractor.validate()?;
        Ok(extractor)
    }
}
#[cfg(feature = "inference")]
impl From<runtime::engines::FeatureExtractor> for FeatureExtractorRef {
    fn from(extractor: runtime::engines::FeatureExtractor) -> Self {
        Self {
            source_id: extractor.source_id,
            blob_sha256: extractor.blob_sha256,
            image_steps: extractor.image_steps,
        }
    }
}
/// Parses an extractor carried as JSON, such as in a recipe or inside a tensor value.
#[cfg(feature = "inference")]
pub(crate) fn runtime_extractor(value: &Value) -> Result<runtime::engines::FeatureExtractor> {
    flow_like_types::json::from_value::<FeatureExtractorRef>(value.clone())
        .map_err(|error| anyhow!("Invalid feature extractor: {error}"))?
        .to_runtime()
}

#[derive(Clone, Serialize, Deserialize, JsonSchema)]
pub struct ScheduleTrainingRequest {
    pub inspection: InspectionContext,
    pub after_new_samples: usize,
    pub training: TrainingRequest,
    pub split: SplitPolicy,
    pub worker_limits: WorkerLimits,
    /// PatchCore and PaDiM only: the extractor of the recorded embeddings. It replaces a
    /// `feature_extractor` in the recipe.
    #[serde(default)]
    pub feature_extractor: Option<FeatureExtractorRef>,
}
#[derive(Clone, Serialize, Deserialize, JsonSchema)]
pub struct TrainBurnRequest {
    pub inspection: InspectionContext,
    pub after_new_samples: usize,
    pub config: flow_like_ml_burn::TrainingConfig,
    #[serde(default)]
    pub compute: ComputeConfig,
    pub split: SplitPolicy,
    pub worker_limits: WorkerLimits,
    #[serde(default)]
    pub preprocessing: Vec<flow_like_ml_core::PreprocessingStep>,
    #[serde(default)]
    pub pretrained: Option<PretrainedSourceRef>,
}
#[cfg(any(feature = "training", test))]
fn validate_native_schedule_compute(engine: &str, compute: &ComputeConfig) -> Result<()> {
    if matches!(
        engine,
        "isolation_forest" | "histogram_gradient_boosting" | "patchcore" | "padim"
    ) && !matches!(
        compute.backend,
        flow_like_ml_core::ComputeBackend::Cpu | flow_like_ml_core::ComputeBackend::Auto
    ) && !compute.allow_cpu_fallback
    {
        return Err(anyhow!(
            "Native classical models require CPU or explicit CPU fallback"
        ));
    }
    Ok(())
}
#[cfg(feature = "training")]
fn schedule(
    repo: TrainingRepository,
    project: String,
    input: ScheduleTrainingRequest,
) -> Result<Option<TrainingJob>> {
    schedule_with(repo, project, input, enqueue)
}
#[cfg(feature = "training")]
fn schedule_with(
    repo: TrainingRepository,
    project: String,
    mut input: ScheduleTrainingRequest,
    dispatch: impl Fn(&TrainingRepository, &str, &WorkerLimits) -> Result<()>,
) -> Result<Option<TrainingJob>> {
    let key = input.inspection.key(project)?;
    let compute: ComputeConfig = flow_like_types::json::from_value(input.training.compute.clone())?;
    compute.validate()?;
    validate_native_schedule_compute(&input.training.engine, &compute)?;
    runtime::worker::TrainingWorker::new(repo.clone(), input.worker_limits.clone())?;
    match input.training.engine.as_str() {
        "burn" => {
            let recipe: runtime::engines::BurnEngineRecipe =
                flow_like_types::json::from_value(input.training.recipe.clone())?;
            flow_like_ml_burn::validate_inspection_config(&input.inspection.spec, &recipe.config)?;
            if let Some(pretrained) = &recipe.pretrained {
                let source = runtime::engines::validate_burn_pretrained_source(
                    &repo,
                    &key.project_id,
                    pretrained,
                    &recipe.config,
                    &recipe.labels,
                    (compute
                        .memory_limit_bytes
                        .min(input.worker_limits.memory_budget_bytes)
                        / 4)
                    .min(512 * 1024 * 1024),
                )?;
                runtime::engines::validate_burn_pretrained_preprocessing(
                    &source,
                    &recipe.preprocessing,
                )?;
            }
        }
        "efficient_ad" => {
            let recipe: runtime::engines::EfficientAdEngineRecipe =
                flow_like_types::json::from_value(input.training.recipe.clone())?;
            recipe.config.validate()?;
            if input.inspection.spec.task != flow_like_ml_core::TaskKind::VisualAnomaly
                || input.inspection.spec.input_shape
                    != [recipe.config.teacher.input_channels, 256, 256]
            {
                return Err(anyhow!(
                    "EfficientAD requires a visual anomaly inspection with normalized [teacher channels,256,256] images"
                ));
            }
        }
        "isolation_forest" => {
            if !matches!(
                input.inspection.spec.task,
                flow_like_ml_core::TaskKind::SensorAnomaly
                    | flow_like_ml_core::TaskKind::VisualAnomaly
            ) {
                return Err(anyhow!("Isolation Forest requires an anomaly inspection"));
            }
        }
        "patchcore" | "padim" => {
            if input.inspection.spec.task != flow_like_ml_core::TaskKind::VisualAnomaly
                || input.inspection.spec.input_shape.len() != 2
            {
                return Err(anyhow!(
                    "PatchCore and PaDiM require visual anomaly embeddings shaped [patches,channels]"
                ));
            }
        }
        "histogram_gradient_boosting" => {
            let config: flow_like_ml_native::models::BoostConfig =
                flow_like_types::json::from_value(input.training.recipe["config"].clone())?;
            let compatible = match config.objective {
                flow_like_ml_native::models::BoostObjective::BinaryLogLoss => {
                    input.inspection.spec.task == flow_like_ml_core::TaskKind::SensorClassification
                        && input.inspection.spec.labels.len() == 2
                }
                flow_like_ml_native::models::BoostObjective::SquaredError => {
                    input.inspection.spec.task == flow_like_ml_core::TaskKind::SensorRegression
                }
            };
            if !compatible {
                return Err(anyhow!(
                    "Histogram boosting objective must match binary sensor classification or scalar sensor regression"
                ));
            }
        }
        _ => return Err(anyhow!("Unsupported inspection training engine")),
    }
    let labels = input
        .training
        .recipe
        .get("labels")
        .and_then(Value::as_array)
        .ok_or_else(|| anyhow!("Training recipe must include labels in inspection order"))?;
    if *labels
        != input
            .inspection
            .spec
            .labels
            .iter()
            .map(|s| json!(s))
            .collect::<Vec<_>>()
    {
        return Err(anyhow!(
            "Training labels differ from the inspection specification"
        ));
    }
    input.training.recipe["minimum_examples"] = json!(input.inspection.spec.minimum_examples);
    input.training.recipe["inspection_task"] = json!(input.inspection.spec.task);
    input.training.recipe["minimum_examples_per_class"] =
        json!(input.inspection.spec.minimum_examples_per_class);
    if let Some(extractor) = input.feature_extractor.take() {
        if !matches!(input.training.engine.as_str(), "patchcore" | "padim") {
            return Err(anyhow!(
                "A feature extractor applies to PatchCore and PaDiM embeddings, not to engine {}",
                input.training.engine
            ));
        }
        input.training.recipe["feature_extractor"] = json!(extractor);
    }
    let extractor = input
        .training
        .recipe
        .get("feature_extractor")
        .filter(|value| !value.is_null())
        .map(runtime_extractor)
        .transpose()?;
    drive_or_trigger(&repo, &key, input, extractor.as_ref(), dispatch)
}

/// Drives the stream's waiting job, or triggers a new one once N new samples arrived.
#[cfg(feature = "training")]
fn drive_or_trigger(
    repo: &TrainingRepository,
    key: &StreamKey,
    input: ScheduleTrainingRequest,
    extractor: Option<&runtime::engines::FeatureExtractor>,
    dispatch: impl Fn(&TrainingRepository, &str, &WorkerLimits) -> Result<()>,
) -> Result<Option<TrainingJob>> {
    let now = runtime::now_ms();
    let mut jobs = settled_jobs(repo, key, now)?;
    if let Some(index) = jobs.iter().position(restartable) {
        return drive_pending(repo, jobs.swap_remove(index), &input, now, &dispatch).map(Some);
    }
    if !trigger_due(repo, key, &jobs, &input, extractor, now)? {
        return Ok(None);
    }
    let job = repo.trigger_after_n(
        key,
        input.after_new_samples,
        input.training,
        input.split,
        now,
    )?;
    if let Some(job) = &job {
        dispatch(repo, &job.id, &input.worker_limits)?;
    }
    Ok(job)
}

/// The waiting job takes this call's compute settings and is dispatched unless it would fail
/// the same way again.
#[cfg(feature = "training")]
fn drive_pending(
    repo: &TrainingRepository,
    job: TrainingJob,
    input: &ScheduleTrainingRequest,
    now: i64,
    dispatch: &impl Fn(&TrainingRepository, &str, &WorkerLimits) -> Result<()>,
) -> Result<TrainingJob> {
    let job = with_requested_compute(repo, job, &input.training, now)?;
    if !repeats_known_failure(repo, &job, &input.worker_limits)? {
        dispatch(repo, &job.id, &input.worker_limits)?;
    }
    Ok(job)
}

/// A trigger needs N new samples, enough of them to fill a first training partition, and
/// embeddings from the extractor the stream already trained on.
#[cfg(feature = "training")]
fn trigger_due(
    repo: &TrainingRepository,
    key: &StreamKey,
    jobs: &[TrainingJob],
    input: &ScheduleTrainingRequest,
    extractor: Option<&runtime::engines::FeatureExtractor>,
    now: i64,
) -> Result<bool> {
    if let Some(extractor) = extractor {
        check_stream_extractor(key, jobs, extractor)?;
    }
    // Counting reads no sample bodies; only a trigger loads them.
    let new_samples = repo.count_new_training_samples(key, now)?;
    Ok(new_samples >= input.after_new_samples
        && !first_trigger_short(jobs, new_samples, input.inspection.spec.minimum_examples))
}

/// The stream's jobs after settling those whose worker stopped. Settling reads every running
/// job's request again, so it only runs when a listed lease has run out.
#[cfg(feature = "training")]
fn settled_jobs(repo: &TrainingRepository, key: &StreamKey, now: i64) -> Result<Vec<TrainingJob>> {
    let jobs = repo.list_jobs(key)?;
    if !jobs.iter().any(|job| lease_ran_out(job, now)) {
        return Ok(jobs);
    }
    repo.recover_expired_jobs(key, now)?;
    Ok(repo.list_jobs(key)?)
}

#[cfg(feature = "training")]
fn lease_ran_out(job: &TrainingJob, now: i64) -> bool {
    matches!(job.status, JobStatus::Running | JobStatus::CancelRequested)
        && job
            .lease_expires_at_ms
            .is_some_and(|deadline| deadline <= now)
}

/// A waiting job of the same engine takes the compute settings of the call that drives it, so
/// a raised memory limit reaches a job that admission refused. A job that a worker is
/// admitting, and an experiment trial, keep their settings.
#[cfg(feature = "training")]
fn with_requested_compute(
    repo: &TrainingRepository,
    job: TrainingJob,
    request: &TrainingRequest,
    now: i64,
) -> Result<TrainingJob> {
    if job.request.engine != request.engine || job.request.compute == request.compute {
        return Ok(job);
    }
    match repo.reconfigure_job_compute(&job.id, request.compute.clone(), now) {
        Err(runtime::Error::Conflict(_)) => Ok(job),
        reconfigured => Ok(reconfigured?),
    }
}

/// Train After N fits every accepted sample of a stream together, so the stream keeps the
/// feature extractor its jobs recorded; embeddings from another extractor need a new stream.
#[cfg(feature = "training")]
fn check_stream_extractor(
    key: &StreamKey,
    jobs: &[TrainingJob],
    scheduled: &runtime::engines::FeatureExtractor,
) -> Result<()> {
    let recorded = jobs.iter().rev().find_map(|job| {
        job.request
            .recipe
            .get("feature_extractor")
            .filter(|value| !value.is_null())
    });
    let Some(recorded) = recorded.map(runtime_extractor).transpose()? else {
        return Ok(());
    };
    if recorded == *scheduled {
        return Ok(());
    }
    Err(anyhow!(
        "Stream {} was trained on embeddings from feature extractor {} ({}), so embeddings from {} ({}) cannot join it: Train After N fits all of a stream's samples together. Record the new embeddings under a new stream ID",
        key.stream_id,
        recorded.source_id,
        recorded.blob_sha256,
        scheduled.source_id,
        scheduled.blob_sha256
    ))
}

/// Before a stream's first trigger every eligible sample is new, and validation and test take
/// at least one each, so fewer than `minimum_examples + 2` cannot fill the training partition.
/// Returning early spares the trigger reading every sample body under the write lock.
#[cfg(feature = "training")]
fn first_trigger_short(jobs: &[TrainingJob], new_samples: usize, minimum_examples: usize) -> bool {
    jobs.is_empty() && new_samples < minimum_examples.saturating_add(2)
}

/// Waiting jobs and jobs whose worker stopped resume on dispatch; paused jobs wait for Resume.
#[cfg(feature = "training")]
fn restartable(job: &TrainingJob) -> bool {
    matches!(job.status, JobStatus::Queued | JobStatus::Interrupted)
}

/// A job whose last attempt in this process failed deterministically under these worker
/// limits, at admission or by exhausting device memory, fails the same way again. It waits
/// for other worker limits, new compute settings from the call that drives it, or Resume
/// Training, instead of being dispatched on every call.
#[cfg(feature = "training")]
fn repeats_known_failure(
    repo: &TrainingRepository,
    job: &TrainingJob,
    limits: &WorkerLimits,
) -> Result<bool> {
    if !restartable(job) || job.error.is_none() {
        return Ok(false);
    }
    let key = queue_key(repo, &job.id)?;
    let limits = flow_like_types::json::to_string(limits)?;
    Ok(with_known_failures(|failures| {
        failures.get(&key) == Some(&limits)
    }))
}

/// Worker limits of each job's last deterministic failure, by queue key.
#[cfg(feature = "training")]
type KnownFailures = std::collections::HashMap<String, String>;
#[cfg(feature = "training")]
fn with_known_failures<T>(use_failures: impl FnOnce(&mut KnownFailures) -> T) -> T {
    use std::sync::{LazyLock, Mutex, PoisonError};
    static FAILURES: LazyLock<Mutex<KnownFailures>> =
        LazyLock::new(|| Mutex::new(KnownFailures::new()));
    use_failures(&mut FAILURES.lock().unwrap_or_else(PoisonError::into_inner))
}

/// Snapshot, checkpoint, estimate, engine and device-memory errors repeat under the same
/// limits; storage errors, cancellation and resource contention do not.
#[cfg(feature = "training")]
fn note_attempt(key: &str, limits: &WorkerLimits, result: &runtime::Result<()>) {
    let deterministic = matches!(
        result,
        Err(runtime::Error::Invalid(_)
            | runtime::Error::Engine(_)
            | runtime::Error::NotFound(_)
            | runtime::Error::Json(_))
    );
    let limits = flow_like_types::json::to_string(limits).ok();
    with_known_failures(|failures| match limits {
        Some(limits) if deterministic => {
            failures.insert(key.to_owned(), limits);
        }
        _ => {
            failures.remove(key);
        }
    });
}

#[cfg(feature = "training")]
fn queue_key(repo: &TrainingRepository, job_id: &str) -> Result<String> {
    Ok(flow_like_ml_core::content_digest(
        &flow_like_types::json::to_vec(&(repo.path(), job_id))?,
    ))
}
#[cfg(feature = "training")]
fn train_burn(
    repo: TrainingRepository,
    project: String,
    input: TrainBurnRequest,
) -> Result<Option<TrainingJob>> {
    flow_like_ml_burn::validate_inspection_config(&input.inspection.spec, &input.config)?;
    let labels = input.inspection.spec.labels.clone();
    let request = TrainingRequest {
        engine: "burn".into(),
        recipe: json!({"config":input.config,"labels":labels,"preprocessing":input.preprocessing,"pretrained":input.pretrained}),
        compute: json!(input.compute),
    };
    schedule(
        repo,
        project,
        ScheduleTrainingRequest {
            inspection: input.inspection,
            after_new_samples: input.after_new_samples,
            training: request,
            split: input.split,
            worker_limits: input.worker_limits,
            feature_extractor: None,
        },
    )
}

#[cfg(feature = "training")]
fn enqueue(repo: &TrainingRepository, job_id: &str, limits: &WorkerLimits) -> Result<()> {
    use std::sync::{
        LazyLock,
        mpsc::{SyncSender, sync_channel},
    };
    type Work = (TrainingRepository, String, WorkerLimits, String);
    // One worker limits aggregate CPU/GPU memory use; SQLite remains the durable queue.
    static WORKER: LazyLock<std::result::Result<SyncSender<Work>, String>> = LazyLock::new(|| {
        let (sender, receiver) = sync_channel::<Work>(32);
        std::thread::Builder::new()
            .name("inspection-training".into())
            .spawn(move || {
                while let Ok((repo, id, limits, key)) = receiver.recv() {
                    if JOBS.start(&key) {
                        attempt(&repo, &id, &limits, &key);
                        JOBS.finish(&key);
                    }
                }
            })
            .map_err(|e| e.to_string())?;
        Ok(sender)
    });
    let sender = WORKER
        .as_ref()
        .map_err(|e| anyhow!("Cannot start training worker: {e}"))?;
    let key = queue_key(repo, job_id)?;
    let ledger = repo.path().canonicalize().map_err(|error| {
        anyhow!(
            "Training job {job_id} is saved, but its ledger {} cannot be resolved: {error}",
            repo.path().display()
        )
    })?;
    let job = QueuedJob {
        ledger,
        repo: repo.clone(),
        job_id: job_id.to_owned(),
    };
    if !JOBS.admit(&key, job)? {
        return Ok(());
    }
    if let Err(error) =
        sender.try_send((repo.clone(), job_id.to_owned(), limits.clone(), key.clone()))
    {
        JOBS.finish(&key);
        return Err(anyhow!(
            "Training job {job_id} is saved, but the worker queue is unavailable. Recover Training Jobs can retry it: {error}"
        ));
    }
    Ok(())
}

/// Runs one dispatched job and keeps a failure that left it waiting visible on the job.
#[cfg(feature = "training")]
fn attempt(repo: &TrainingRepository, job_id: &str, limits: &WorkerLimits, key: &str) {
    let result = run_training_job(repo, job_id, limits);
    note_attempt(key, limits, &result);
    let Err(error) = result else {
        return;
    };
    // The worker records admission failures itself; this adds those before admission.
    if !matches!(
        error,
        runtime::Error::Cancelled | runtime::Error::Conflict(_)
    ) {
        let _ = repo.record_job_attempt_error(job_id, &error.to_string(), runtime::now_ms());
    }
    tracing::warn!(job_id = %job_id, %error, "Inspection training worker stopped");
}

#[cfg(feature = "training")]
fn run_training_job(
    repo: &TrainingRepository,
    job_id: &str,
    limits: &WorkerLimits,
) -> runtime::Result<()> {
    let mut worker = runtime::worker::TrainingWorker::new(repo.clone(), limits.clone())?;
    runtime::engines::register_native_engines(&mut worker)?;
    runtime::engines::register_burn_engine(&mut worker)?;
    runtime::engines::register_efficient_ad_engine(&mut worker)?;
    worker.run(job_id)?;
    Ok(())
}

#[derive(Clone, Serialize, Deserialize, JsonSchema)]
pub struct JobRequest {
    pub inspection: InspectionContext,
    pub job_id: String,
}

#[derive(Clone, Serialize, Deserialize, JsonSchema)]
pub struct TrainEfficientAdRequest {
    pub inspection: InspectionContext,
    pub after_new_samples: usize,
    pub config: flow_like_ml_burn::EfficientAdConfig,
    #[serde(default)]
    pub compute: ComputeConfig,
    pub split: SplitPolicy,
    pub worker_limits: WorkerLimits,
    #[serde(default)]
    pub preprocessing: Vec<flow_like_ml_core::PreprocessingStep>,
}
#[cfg(feature = "training")]
fn train_efficient_ad(
    repo: TrainingRepository,
    project: String,
    input: TrainEfficientAdRequest,
) -> Result<Option<TrainingJob>> {
    input.config.validate()?;
    if input.inspection.spec.task != flow_like_ml_core::TaskKind::VisualAnomaly {
        return Err(anyhow!("EfficientAD requires a visual anomaly inspection"));
    }
    let request = TrainingRequest {
        engine: "efficient_ad".into(),
        recipe: json!({"config":input.config,"labels":input.inspection.spec.labels,"preprocessing":input.preprocessing}),
        compute: json!(input.compute),
    };
    schedule(
        repo,
        project,
        ScheduleTrainingRequest {
            inspection: input.inspection,
            after_new_samples: input.after_new_samples,
            training: request,
            split: input.split,
            worker_limits: input.worker_limits,
            feature_extractor: None,
        },
    )
}
#[cfg(feature = "training")]
fn job(repo: TrainingRepository, project: String, input: JobRequest) -> Result<TrainingJob> {
    let job = repo.get_job(&input.job_id)?;
    check_stream(&input.inspection.key(project)?, &job.stream)?;
    Ok(job)
}
/// A job whose worker died is settled first, so a cancellation never waits for it.
#[cfg(feature = "training")]
fn cancel(repo: TrainingRepository, project: String, input: JobRequest) -> Result<TrainingJob> {
    let key = input.inspection.key(project)?;
    check_stream(&key, &repo.get_job(&input.job_id)?.stream)?;
    repo.recover_expired_jobs(&key, runtime::now_ms())?;
    Ok(repo.request_cancel(&input.job_id, runtime::now_ms())?)
}
#[derive(Clone, Serialize, Deserialize, JsonSchema)]
pub struct ResumeRequest {
    pub inspection: InspectionContext,
    pub job_id: String,
    pub worker_limits: WorkerLimits,
    /// New compute settings, applied before the job runs again, including after Cancel or a
    /// failure. Running jobs and experiment trials refuse them.
    #[serde(default)]
    pub compute: Option<ComputeConfig>,
}
#[cfg(feature = "training")]
fn resume(repo: TrainingRepository, project: String, input: ResumeRequest) -> Result<TrainingJob> {
    resume_with(repo, project, input, enqueue)
}
/// A queued job, such as one that failed admission, is dispatched again with these limits.
#[cfg(feature = "training")]
fn resume_with(
    repo: TrainingRepository,
    project: String,
    input: ResumeRequest,
    dispatch: impl Fn(&TrainingRepository, &str, &WorkerLimits) -> Result<()>,
) -> Result<TrainingJob> {
    let key = input.inspection.key(project)?;
    let job = repo.get_job(&input.job_id)?;
    check_stream(&key, &job.stream)?;
    runtime::worker::TrainingWorker::new(repo.clone(), input.worker_limits.clone())?;
    let job = requeue_with_compute(&repo, job, input.compute.as_ref())?;
    dispatch(&repo, &job.id, &input.worker_limits)?;
    Ok(job)
}
/// A waiting job stays queued; a paused, interrupted, cancelled, failed or crashed job is
/// queued again.
#[cfg(feature = "training")]
fn requeue(repo: &TrainingRepository, job: TrainingJob) -> Result<TrainingJob> {
    if job.status == JobStatus::Queued {
        return Ok(job);
    }
    Ok(repo.resume_job(&job.id, runtime::now_ms())?)
}
/// Requeues the job, under new compute settings when given. Only waiting and stopped jobs
/// change compute: those take it before they are requeued, and a cancelled, failed or crashed
/// job is requeued first. Either way it is dispatched once, under the new settings.
#[cfg(feature = "training")]
fn requeue_with_compute(
    repo: &TrainingRepository,
    job: TrainingJob,
    compute: Option<&ComputeConfig>,
) -> Result<TrainingJob> {
    let Some(compute) = compute else {
        return requeue(repo, job);
    };
    compute.validate()?;
    validate_native_schedule_compute(&job.request.engine, compute)?;
    let compute = flow_like_types::json::to_value(compute)?;
    let reconfigure = move |job: TrainingJob| -> Result<TrainingJob> {
        Ok(repo.reconfigure_job_compute(&job.id, compute, runtime::now_ms())?)
    };
    if matches!(
        job.status,
        JobStatus::Queued | JobStatus::Paused | JobStatus::Interrupted
    ) {
        requeue(repo, reconfigure(job)?)
    } else {
        reconfigure(requeue(repo, job)?)
    }
}
#[derive(Clone, Serialize, Deserialize, JsonSchema)]
pub struct RecoverRequest {
    pub inspection: InspectionContext,
    pub worker_limits: WorkerLimits,
}
#[cfg(feature = "training")]
fn recover(
    repo: TrainingRepository,
    project: String,
    input: RecoverRequest,
) -> Result<Vec<TrainingJob>> {
    recover_with(repo, project, input, enqueue)
}
/// Settles jobs whose worker stopped, then restarts waiting and interrupted jobs.
#[cfg(feature = "training")]
fn recover_with(
    repo: TrainingRepository,
    project: String,
    input: RecoverRequest,
    dispatch: impl Fn(&TrainingRepository, &str, &WorkerLimits) -> Result<()>,
) -> Result<Vec<TrainingJob>> {
    let key = input.inspection.key(project)?;
    runtime::worker::TrainingWorker::new(repo.clone(), input.worker_limits.clone())?;
    let jobs = settled_jobs(&repo, &key, runtime::now_ms())?;
    for job in jobs.iter().filter(|job| restartable(job)) {
        if !repeats_known_failure(&repo, job, &input.worker_limits)? {
            dispatch(&repo, &job.id, &input.worker_limits)?;
        }
    }
    Ok(jobs)
}

#[derive(Clone, Serialize, Deserialize, JsonSchema)]
pub struct ArtifactRequest {
    pub inspection: InspectionContext,
    pub artifact_id: String,
}
#[cfg(feature = "inference")]
fn artifact(
    repo: TrainingRepository,
    project: String,
    input: ArtifactRequest,
) -> Result<ModelArtifact> {
    let artifact = repo.get_artifact(&input.artifact_id)?;
    check_stream(&input.inspection.key(project)?, &artifact.stream)?;
    Ok(artifact)
}
#[derive(Clone, Serialize, Deserialize, JsonSchema)]
pub struct PredictRequest {
    pub inspection: InspectionContext,
    pub artifact_id: String,
    pub input: TensorData,
    #[serde(default)]
    pub compute: ComputeConfig,
}
#[derive(Clone, Serialize, Deserialize, JsonSchema)]
pub struct StudentInferenceRequest {
    pub inspection: InspectionContext,
    pub artifact_id: String,
    pub input: TensorData,
    #[serde(default)]
    pub compute: ComputeConfig,
    /// The extractor of embedding inputs, from Extract Image Features. A PatchCore or PaDiM
    /// model that recorded its extractor refuses embeddings from another extractor or without
    /// one.
    #[serde(default)]
    pub feature_extractor: Option<FeatureExtractorRef>,
}
#[derive(Clone, Serialize, Deserialize, JsonSchema)]
pub struct RecordPredictionRequest {
    pub inspection: InspectionContext,
    pub prediction: PredictionRecord,
}
/// Comparisons are immutable and evaluation rejects evidence recorded after it runs, so a
/// timestamp outside the artifact's lifetime would block that artifact's evaluation.
#[cfg(feature = "training")]
fn record_prediction(
    repo: TrainingRepository,
    project: String,
    input: RecordPredictionRequest,
) -> Result<bool> {
    let model = artifact(
        repo.clone(),
        project,
        ArtifactRequest {
            inspection: input.inspection,
            artifact_id: input.prediction.artifact_id.clone(),
        },
    )?;
    let recorded = input.prediction.recorded_at_ms;
    let now = runtime::now_ms();
    if recorded < model.created_at_ms || recorded > now {
        return Err(anyhow!(
            "Comparison recorded_at_ms {recorded} must lie between the creation of artifact {} at {} and now ({now}); use epoch milliseconds",
            model.id,
            model.created_at_ms
        ));
    }
    Ok(repo.record_prediction(&input.prediction)?)
}
#[cfg(feature = "training")]
fn evaluate(
    repo: TrainingRepository,
    project: String,
    input: ArtifactRequest,
) -> Result<EvaluationReport> {
    artifact(repo.clone(), project, input.clone())?;
    Ok(repo.evaluate_classification(&input.artifact_id, runtime::now_ms())?)
}
#[derive(Clone, Serialize, Deserialize, JsonSchema)]
pub struct PromoteRequest {
    pub inspection: InspectionContext,
    pub deployment_id: String,
    pub expected_generation: i64,
    pub evaluation_id: String,
    pub policy: PromotionPolicy,
}

#[derive(Clone, Serialize, Deserialize, JsonSchema)]
pub struct EvaluateTaskRequest {
    pub inspection: InspectionContext,
    pub artifact_id: String,
    pub task: EvaluationTask,
}
#[cfg(feature = "training")]
fn evaluate_task(
    repo: TrainingRepository,
    project: String,
    input: EvaluateTaskRequest,
) -> Result<EvaluationReport> {
    artifact(
        repo.clone(),
        project,
        ArtifactRequest {
            inspection: input.inspection,
            artifact_id: input.artifact_id.clone(),
        },
    )?;
    Ok(repo.evaluate_task(&input.artifact_id, input.task, runtime::now_ms())?)
}
#[derive(Clone, Serialize, Deserialize, JsonSchema)]
pub struct PromoteMetricsRequest {
    pub inspection: InspectionContext,
    pub deployment_id: String,
    pub expected_generation: i64,
    pub evaluation_id: String,
    pub policy: MetricPromotionPolicy,
}
#[cfg(feature = "training")]
fn promote_metrics(
    repo: TrainingRepository,
    project: String,
    input: PromoteMetricsRequest,
) -> Result<Deployment> {
    let key = input.inspection.key(project)?;
    check_promotion_scope(&repo, &key, &input.evaluation_id, &input.deployment_id)?;
    Ok(repo.promote_metrics(
        &input.deployment_id,
        input.expected_generation,
        &input.evaluation_id,
        input.policy,
        runtime::now_ms(),
    )?)
}
#[cfg(feature = "training")]
fn promote(repo: TrainingRepository, project: String, input: PromoteRequest) -> Result<Deployment> {
    let key = input.inspection.key(project)?;
    check_promotion_scope(&repo, &key, &input.evaluation_id, &input.deployment_id)?;
    Ok(repo.promote(
        &input.deployment_id,
        input.expected_generation,
        &input.evaluation_id,
        input.policy,
        runtime::now_ms(),
    )?)
}
#[cfg(feature = "training")]
fn check_promotion_scope(
    repo: &TrainingRepository,
    key: &StreamKey,
    evaluation_id: &str,
    deployment_id: &str,
) -> Result<()> {
    let report = repo.get_evaluation(evaluation_id)?;
    check_stream(key, &repo.get_artifact(&report.artifact_id)?.stream)?;
    check_deployment_scope(repo, key, deployment_id)
}
#[derive(Clone, Serialize, Deserialize, JsonSchema)]
pub struct DeploymentRequest {
    pub inspection: InspectionContext,
    pub deployment_id: String,
}

#[derive(Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum InspectionMode {
    TeacherOnly,
    Shadow,
    StudentOnly,
}
#[derive(Clone, Serialize, Deserialize, JsonSchema)]
pub struct InspectionRoute {
    pub mode: InspectionMode,
    pub active_artifact_id: Option<String>,
    pub candidate_artifact_id: Option<String>,
    pub deployment_generation: Option<i64>,
}
/// The stream's deployment in this ledger and the route the ledger gives.
#[cfg(feature = "training")]
fn routing(
    repo: TrainingRepository,
    project: String,
    input: DeploymentRequest,
) -> Result<(Option<Deployment>, InspectionRoute)> {
    let key = input.inspection.key(project.clone())?;
    let deployment = deployment(repo.clone(), project, input)?;
    if let Some(active) = &deployment {
        if !active.paused {
            let route = InspectionRoute {
                mode: InspectionMode::StudentOnly,
                active_artifact_id: Some(active.active_artifact_id.clone()),
                candidate_artifact_id: None,
                deployment_generation: Some(active.generation),
            };
            return Ok((deployment, route));
        }
    }
    let candidate = repo
        .list_jobs(&key)?
        .into_iter()
        .filter(|j| j.status == JobStatus::Succeeded)
        .max_by_key(|j| j.updated_at_ms)
        .and_then(|j| j.artifact_id);
    let route = InspectionRoute {
        mode: if candidate.is_some() {
            InspectionMode::Shadow
        } else {
            InspectionMode::TeacherOnly
        },
        active_artifact_id: None,
        candidate_artifact_id: candidate,
        deployment_generation: deployment.as_ref().map(|d| d.generation),
    };
    Ok((deployment, route))
}

/// The route of a published pointer: the student unless the pointer is paused or missing.
/// Without a ledger there are no shadow candidates.
#[cfg(feature = "inference")]
fn published_route(pointer: Option<&super::published::DeploymentPointer>) -> InspectionRoute {
    let active = pointer.filter(|pointer| !pointer.paused);
    InspectionRoute {
        mode: if active.is_some() {
            InspectionMode::StudentOnly
        } else {
            InspectionMode::TeacherOnly
        },
        active_artifact_id: active.map(|pointer| pointer.active_artifact_id.clone()),
        candidate_artifact_id: None,
        deployment_generation: pointer.map(|pointer| pointer.generation),
    }
}

/// The model root of a stream's published models, or None for both roots, App first. Records
/// of streams minted in the App ledger without a scope marker publish to the executing user's
/// directory when they trained on private data.
#[cfg(feature = "inference")]
fn published_storage(stream_id: &str) -> Option<super::published::ModelStorage> {
    use super::published::ModelStorage;
    match LedgerScope::of_stream(stream_id) {
        LedgerScope::User => Some(ModelStorage::User),
        LedgerScope::App if StreamKind::of_stream(stream_id).is_some() => None,
        LedgerScope::App => Some(ModelStorage::App),
    }
}

/// The published pointer of the request's deployment when it serves the request's stream
/// and specification.
#[cfg(feature = "inference")]
async fn published_pointer(
    context: &mut ExecutionContext,
    input: &DeploymentRequest,
) -> Result<Option<super::published::DeploymentPointer>> {
    use super::published::ModelStorage;
    let version = input.inspection.inspection_version()?;
    let storages = match published_storage(&input.inspection.stream_id) {
        Some(storage) => vec![storage],
        None => vec![ModelStorage::App, ModelStorage::User],
    };
    for storage in storages {
        let pointer =
            super::published::published_deployment(context, &input.deployment_id, storage)
                .await?
                .filter(|pointer| {
                    pointer.stream.stream_id == input.inspection.stream_id
                        && pointer.stream.inspection_version == version
                });
        if pointer.is_some() {
            return Ok(pointer);
        }
    }
    Ok(None)
}

/// A published pointer that replaces the ledger's answer: one of a strictly greater
/// generation than the deployment the ledger holds, which a pointer in the Local store must
/// show first, or any pointer of this stream when the ledger holds none. Read failures keep
/// the ledger's answer.
#[cfg(feature = "training")]
async fn superseding_pointer(
    context: &mut ExecutionContext,
    input: &DeploymentRequest,
    held: Option<&Deployment>,
) -> Option<super::published::DeploymentPointer> {
    let generation = held.map_or(0, |deployment| deployment.generation);
    if held.is_some() {
        let scope = LedgerScope::of_stream(&input.inspection.stream_id);
        let newer = super::published::newer_published_generation(
            context,
            super::published::PointerKind::Deployment,
            scope,
            &input.deployment_id,
            (generation, 0),
        )
        .await;
        if !newer {
            return None;
        }
    }
    match published_pointer(context, input).await {
        Ok(pointer) => pointer.filter(|pointer| pointer.generation > generation),
        Err(error) => {
            let message = format!(
                "Published deployment {} of stream {} could not be read, so the training ledger decides: {error}",
                input.deployment_id, input.inspection.stream_id
            );
            tracing::warn!("{message}");
            context.log_message(&message, flow_like::flow::execution::LogLevel::Warn);
            None
        }
    }
}

/// Get Active Student: the routed ledger's deployment of this stream unless a published
/// pointer supersedes it, else the published pointer's deployment.
#[cfg(feature = "inference")]
async fn active_deployment(
    context: &mut ExecutionContext,
    input: &DeploymentRequest,
) -> Result<Option<Deployment>> {
    #[cfg(feature = "training")]
    if let Some(ledger) = routed_ledger_for_inference(context, &input.inspection.stream_id)? {
        let held = {
            let (repo, project) = (ledger.repo.clone(), ledger.project.clone());
            let input = input.clone();
            tokio::task::spawn_blocking(move || deployment(repo, project, input)).await??
        };
        if let Some(pointer) = superseding_pointer(context, input, held.as_ref()).await {
            return Ok(Some(pointer.deployment));
        }
        if let Some(deployment) = &held {
            publish_deployment(
                context,
                &ledger.repo,
                ledger.scope,
                deployment,
                PublishReason::SelfRepair,
            )
            .await;
        }
        return Ok(held);
    }
    Ok(published_pointer(context, input)
        .await?
        .map(|pointer| pointer.deployment))
}

/// Route Inspection: the routed ledger's route unless a published pointer supersedes its
/// deployment, else the published pointer's route.
#[cfg(feature = "inference")]
async fn inspection_route(
    context: &mut ExecutionContext,
    input: &DeploymentRequest,
) -> Result<InspectionRoute> {
    #[cfg(feature = "training")]
    if let Some(ledger) = routed_ledger_for_inference(context, &input.inspection.stream_id)? {
        let (held, route) = {
            let LocatedLedger { repo, project, .. } = ledger;
            let input = input.clone();
            tokio::task::spawn_blocking(move || routing(repo, project, input)).await??
        };
        return Ok(match superseding_pointer(context, input, held.as_ref()).await {
            Some(pointer) => published_route(Some(&pointer)),
            None => route,
        });
    }
    Ok(published_route(published_pointer(context, input).await?.as_ref()))
}

/// Publishes a deployment's pointer and active model; a failure is a warning.
#[cfg(feature = "training")]
async fn publish_deployment(
    context: &mut ExecutionContext,
    repo: &TrainingRepository,
    scope: LedgerScope,
    deployment: &Deployment,
    reason: PublishReason,
) {
    let outcome =
        super::publisher::ensure_deployment_published(context, repo, scope, deployment, reason)
            .await;
    let model = format!(
        "Deployment {} of stream {}",
        deployment.id, deployment.stream.stream_id
    );
    warn_unpublished(context, &model, &outcome);
}

/// The publish arm of lifecycle nodes that change a deployment.
#[cfg(feature = "training")]
async fn publish_changed_deployment(context: &mut ExecutionContext, routed: &Routed<Deployment>) {
    publish_deployment(
        context,
        &routed.repo,
        routed.scope,
        &routed.output,
        PublishReason::StateChange,
    )
    .await;
}

#[derive(Clone, Serialize, Deserialize, JsonSchema)]
pub struct PauseDeploymentRequest {
    pub inspection: InspectionContext,
    pub deployment_id: String,
    pub expected_generation: i64,
    pub paused: bool,
}
#[cfg(feature = "training")]
fn pause_deployment(
    repo: TrainingRepository,
    project: String,
    input: PauseDeploymentRequest,
) -> Result<Deployment> {
    let key = input.inspection.key(project)?;
    check_deployment_scope(&repo, &key, &input.deployment_id)?;
    Ok(repo.set_deployment_paused(
        &input.deployment_id,
        input.expected_generation,
        input.paused,
        runtime::now_ms(),
    )?)
}
/// Deployment IDs are shared by every stream in the app ledger. Another stream's deployment
/// under this ID is not a student of this stream, so the teacher keeps its decisions.
#[cfg(feature = "training")]
fn deployment(
    repo: TrainingRepository,
    project: String,
    input: DeploymentRequest,
) -> Result<Option<Deployment>> {
    let key = input.inspection.key(project)?;
    let stored = stored_deployment(&repo, &input.deployment_id)?;
    Ok(stored.filter(|deployment| deployment.stream == key))
}
#[cfg(feature = "training")]
fn stored_deployment(repo: &TrainingRepository, deployment_id: &str) -> Result<Option<Deployment>> {
    match repo.get_deployment(deployment_id) {
        Ok(deployment) => Ok(Some(deployment)),
        Err(runtime::Error::NotFound(_)) => Ok(None),
        Err(error) => Err(error.into()),
    }
}
/// Changing another stream's deployment, including after a specification edit, is refused.
#[cfg(feature = "training")]
fn check_deployment_scope(
    repo: &TrainingRepository,
    key: &StreamKey,
    deployment_id: &str,
) -> Result<()> {
    let stored = stored_deployment(repo, deployment_id)?;
    if stored.is_some_and(|deployment| deployment.stream != *key) {
        return Err(anyhow!(
            "Deployment {deployment_id} serves a different inspection stream or specification; use a separate deployment ID for stream {}",
            key.stream_id
        ));
    }
    Ok(())
}
#[derive(Clone, Serialize, Deserialize, JsonSchema)]
pub struct RollbackRequest {
    pub inspection: InspectionContext,
    pub deployment_id: String,
    pub expected_generation: i64,
}
#[cfg(feature = "training")]
fn rollback(
    repo: TrainingRepository,
    project: String,
    input: RollbackRequest,
) -> Result<Deployment> {
    let key = input.inspection.key(project)?;
    check_deployment_scope(&repo, &key, &input.deployment_id)?;
    Ok(repo.rollback(
        &input.deployment_id,
        input.expected_generation,
        runtime::now_ms(),
    )?)
}

/// Requests of nodes with an inspection context, which run on the ledger of their stream.
#[cfg(feature = "training")]
trait LedgerRouted {
    fn stream_id(&self) -> &str;
}

#[cfg(feature = "training")]
macro_rules! ledger_routed {
    ($($request:ty),+ $(,)?) => {
        $(
            impl LedgerRouted for $request {
                fn stream_id(&self) -> &str {
                    &self.inspection.stream_id
                }
            }
        )+
    };
}

#[cfg(feature = "training")]
ledger_routed!(
    RecordSampleRequest,
    ScheduleTrainingRequest,
    TrainBurnRequest,
    JobRequest,
    TrainEfficientAdRequest,
    ResumeRequest,
    RecoverRequest,
    ArtifactRequest,
    PredictRequest,
    StudentInferenceRequest,
    RecordPredictionRequest,
    PromoteRequest,
    EvaluateTaskRequest,
    PromoteMetricsRequest,
    DeploymentRequest,
    PauseDeploymentRequest,
    RollbackRequest,
);

#[cfg(feature = "training")]
impl LedgerRouted for ExportOnnxRequest {
    fn stream_id(&self) -> &str {
        &self.prediction.inspection.stream_id
    }
}

/// A lifecycle handler's output and the ledger it ran on.
#[cfg(feature = "training")]
struct Routed<O> {
    output: O,
    repo: TrainingRepository,
    scope: LedgerScope,
}

/// Runs `handler` on the ledger of the request's stream, creating the ledger when missing.
#[cfg(feature = "training")]
async fn run_routed<I, O>(
    context: &ExecutionContext,
    input: I,
    handler: fn(TrainingRepository, String, I) -> Result<O>,
) -> Result<Routed<O>>
where
    I: LedgerRouted + Send + 'static,
    O: Send + 'static,
{
    let ledger = LedgerLocation::for_stream(context, input.stream_id())?;
    let scope = ledger.scope();
    tokio::task::spawn_blocking(move || {
        let (repo, project) = ledger.open()?;
        let output = handler(repo.clone(), project, input)?;
        Ok(Routed {
            output,
            repo,
            scope,
        })
    })
    .await?
}

macro_rules! lifecycle_node {
    ($node:ident, $id:literal, $name:literal, $description:literal, $function:literal, $input:ty, $output:ty, $handler:path $(, version = $version:literal)? $(, publish = $publish:path)?) => {
        #[flow_like_types::async_trait]
        impl NodeLogic for $node {
            fn get_node(&self) -> flow_like::flow::node::Node {
                let node = super::operation_node::<$input, $output>(
                    $id,
                    $name,
                    $description,
                    $function,
                    "AI/ML/Lifecycle",
                );
                $(
                    let mut node = node;
                    node.set_version($version);
                )?
                node
            }
            async fn run(&self, context: &mut ExecutionContext) -> Result<()> {
                #[cfg(feature = "training")]
                {
                    context.deactivate_exec_pin("exec_out").await?;
                    let input: $input = context.evaluate_pin("request").await?;
                    let routed = run_routed::<$input, $output>(context, input, $handler).await?;
                    $($publish(context, &routed).await;)?
                    context.set_pin_value("result", json!(routed.output)).await?;
                    context.activate_exec_pin("exec_out").await?;
                    Ok(())
                }
                #[cfg(not(feature = "training"))]
                {
                    let _ = context;
                    Err(training_unavailable())
                }
            }
        }
    };
}
#[crate::register_node]
#[derive(Default)]
pub struct RecordTrainingSampleNode;
lifecycle_node!(
    RecordTrainingSampleNode,
    "ml_record_training_sample",
    "Record Training Sample",
    "Persist a versioned annotation with capture time and label provenance",
    "recordSample",
    RecordSampleRequest,
    SampleReceipt,
    record_sample
);

#[crate::register_node]
#[derive(Default)]
pub struct ScheduleTrainingNode;
lifecycle_node!(
    ScheduleTrainingNode,
    "ml_schedule_training",
    "Train After N Samples",
    "Atomically snapshot new labels and start a durable background training job",
    "scheduleTraining",
    ScheduleTrainingRequest,
    Option<TrainingJob>,
    schedule,
    version = 1
);

#[crate::register_node]
#[derive(Default)]
pub struct TrainBurnModelNode;
lifecycle_node!(
    TrainBurnModelNode,
    "ml_train_burn_model",
    "Train Burn Model",
    "Train a selected neural architecture on CPU or an optional GPU backend",
    "trainBurn",
    TrainBurnRequest,
    Option<TrainingJob>,
    train_burn
);

#[crate::register_node]
#[derive(Default)]
pub struct TrainingJobStatusNode;
lifecycle_node!(
    TrainingJobStatusNode,
    "ml_training_job_status",
    "Training Job Status",
    "Read progress, checkpoint, failure and artifact state",
    "trainingStatus",
    JobRequest,
    TrainingJob,
    job
);

#[crate::register_node]
#[derive(Default)]
pub struct CancelTrainingNode;
lifecycle_node!(
    CancelTrainingNode,
    "ml_cancel_training",
    "Cancel Training",
    "Request cooperative cancellation and retain the last completed checkpoint",
    "cancelTraining",
    JobRequest,
    TrainingJob,
    cancel
);

#[crate::register_node]
#[derive(Default)]
pub struct ResumeTrainingNode;
lifecycle_node!(
    ResumeTrainingNode,
    "ml_resume_training",
    "Resume Training",
    "Resume an interrupted, cancelled or waiting job from its saved checkpoint under the given worker limits, optionally with new compute settings",
    "resumeTraining",
    ResumeRequest,
    TrainingJob,
    resume,
    version = 2
);

#[crate::register_node]
#[derive(Default)]
pub struct RecoverTrainingJobsNode;
lifecycle_node!(
    RecoverTrainingJobsNode,
    "ml_recover_training_jobs",
    "Recover Training Jobs",
    "Settle jobs whose worker stopped and restart queued or interrupted jobs after a process restart",
    "recoverTraining",
    RecoverRequest,
    Vec<TrainingJob>,
    recover
);

#[crate::register_node]
#[derive(Default)]
pub struct GetModelArtifactNode;
lifecycle_node!(
    GetModelArtifactNode,
    "ml_get_model_artifact",
    "Get Model Artifact",
    "Resolve a durable artifact and verify its inspection scope",
    "artifact",
    ArtifactRequest,
    ModelArtifact,
    artifact
);

#[crate::register_node]
#[derive(Default)]
pub struct StudentInferenceNode;

#[cfg(feature = "inference")]
pub(crate) type CachedPredictor = std::sync::Arc<CatalogLoadedPredictor>;
#[cfg(feature = "inference")]
pub(crate) enum CatalogLoadedPredictor {
    Builtin(runtime::engines::LoadedArtifactPredictor),
    Classical {
        artifact: ModelArtifact,
        predictor: std::sync::Mutex<super::auto_training_legacy::LegacyPredictor>,
    },
}
#[cfg(feature = "inference")]
impl CatalogLoadedPredictor {
    pub(crate) fn load(
        repo: &TrainingRepository,
        artifact_id: &str,
        compute: &ComputeConfig,
    ) -> Result<Self> {
        let artifact = repo.get_artifact(artifact_id)?;
        if artifact.manifest["engine"] == "legacy_classical" {
            let predictor =
                super::auto_training_legacy::LegacyPredictor::load(repo, artifact_id, compute)?;
            Ok(Self::Classical {
                artifact,
                predictor: std::sync::Mutex::new(predictor),
            })
        } else {
            Ok(Self::Builtin(
                runtime::engines::LoadedArtifactPredictor::load(repo, artifact_id, compute)?,
            ))
        }
    }
    /// Loads an artifact whose blob bytes come from outside a ledger, such as a published
    /// model folder. It never checks a project id.
    pub(crate) fn from_parts(
        artifact: ModelArtifact,
        bytes: &[u8],
        compute: &ComputeConfig,
    ) -> Result<Self> {
        if artifact.manifest["engine"] == "legacy_classical" {
            let predictor =
                super::auto_training_legacy::LegacyPredictor::from_parts(&artifact, bytes, compute)?;
            Ok(Self::Classical {
                artifact,
                predictor: std::sync::Mutex::new(predictor),
            })
        } else {
            Ok(Self::Builtin(
                runtime::engines::LoadedArtifactPredictor::from_parts(artifact, bytes, compute)?,
            ))
        }
    }
    pub(crate) fn artifact(&self) -> &ModelArtifact {
        match self {
            Self::Builtin(predictor) => predictor.artifact(),
            Self::Classical { artifact, .. } => artifact,
        }
    }
    /// Inputs that name no feature extractor; a model that recorded the extractor of its
    /// training embeddings refuses them.
    pub(crate) fn predict(&self, input: &TensorData) -> Result<Value> {
        match self {
            Self::Builtin(predictor) => {
                refuse_unnamed_embeddings(predictor.artifact())?;
                Ok(predictor.predict(input)?)
            }
            Self::Classical { predictor, .. } => {
                use runtime::auto_training::SearchPredictor;
                let mut predictor = predictor
                    .lock()
                    .map_err(|_| anyhow!("Classical predictor lock is poisoned"))?;
                Ok(predictor.predict(input)?)
            }
        }
    }
    /// With an extractor, a model fitted on another extractor's embeddings refuses them.
    pub(crate) fn predict_checked(
        &self,
        input: &TensorData,
        extractor: Option<&runtime::engines::FeatureExtractor>,
    ) -> Result<Value> {
        match (self, extractor) {
            (Self::Builtin(predictor), Some(extractor)) => {
                Ok(predictor.predict_with_extractor(input, extractor)?)
            }
            // Classical models record no extractor; `predict` checks inputs that name none.
            _ => self.predict(input),
        }
    }
    pub(crate) fn features(&self, input: &TensorData) -> Result<TensorData> {
        match self {
            Self::Builtin(predictor) => Ok(predictor.features(input)?),
            Self::Classical { .. } => {
                Err(anyhow!("Classical models do not expose image feature maps"))
            }
        }
    }
    pub(crate) fn feature_extractor(&self) -> Result<FeatureExtractorRef> {
        match self {
            Self::Builtin(predictor) => Ok(predictor.feature_extractor()?.into()),
            Self::Classical { .. } => {
                Err(anyhow!("Classical models do not expose image feature maps"))
            }
        }
    }
}
/// Embeddings that name no extractor cannot be checked against the one a PatchCore or PaDiM
/// model recorded, and another extractor's embeddings give arbitrary anomaly scores.
#[cfg(feature = "inference")]
fn refuse_unnamed_embeddings(model: &ModelArtifact) -> Result<()> {
    let Some(recorded) = model
        .manifest
        .get("feature_extractor")
        .filter(|value| !value.is_null())
    else {
        return Ok(());
    };
    let recorded = runtime_extractor(recorded)?;
    Err(anyhow!(
        "Model {} was fitted on embeddings from feature extractor {} ({}); send the extractor of these embeddings, the Feature Extractor output of Extract Image Features, with the request",
        model.id,
        recorded.source_id,
        recorded.blob_sha256
    ))
}
#[cfg(feature = "inference")]
#[derive(Clone, Default)]
struct PredictionCache {
    models: std::sync::Arc<tokio::sync::Mutex<Vec<(String, CachedPredictor, u64)>>>,
}
#[cfg(feature = "inference")]
fn trim_prediction_cache(
    models: &mut Vec<(String, CachedPredictor, u64)>,
    incoming: u64,
    budget: u64,
) {
    while !models.is_empty()
        && (models.len() >= 4
            || models
                .iter()
                .fold(incoming, |total, entry| total.saturating_add(entry.2))
                > budget)
    {
        models.remove(0);
    }
}
#[cfg(feature = "inference")]
impl flow_like_types::Cacheable for PredictionCache {
    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
    fn as_any_mut(&mut self) -> &mut dyn std::any::Any {
        self
    }
}
/// The model of `artifact_id` from the ledger that holds it for this run's project, else
/// from its published folder in the stream's model root.
#[cfg(feature = "inference")]
async fn cached_predictor(
    context: &mut ExecutionContext,
    inspection: &InspectionContext,
    artifact_id: &str,
    compute: &ComputeConfig,
) -> Result<CachedPredictor> {
    let compute = compute.for_host();
    compute.validate()?;
    if let Some(predictor) = ledger_predictor(context, inspection, artifact_id, &compute).await? {
        return Ok(predictor);
    }
    let source = super::published::PublishedModelSource::Artifact {
        artifact_id: artifact_id.to_owned(),
        storage: published_storage(&inspection.stream_id),
    };
    let model = super::published::resolve_model(context, &source).await?;
    super::published::check_stream(&model.manifest, inspection)?;
    super::published::published_predictor(context, &model, &compute).await
}

/// The ledger path of `cached_predictor`, reused within the run. None when no ledger holds
/// the artifact for this run's project.
#[cfg(feature = "inference")]
async fn ledger_predictor(
    context: &mut ExecutionContext,
    inspection: &InspectionContext,
    artifact_id: &str,
    compute: &ComputeConfig,
) -> Result<Option<CachedPredictor>> {
    let located = locate_repository_for_inference(context, |repo, project| {
        holds_artifact(repo, project, artifact_id)
    })?;
    let Some(LocatedLedger { repo, project, .. }) = located else {
        return Ok(None);
    };
    let request = ArtifactRequest {
        inspection: inspection.clone(),
        artifact_id: artifact_id.to_owned(),
    };
    let (repo, model) = tokio::task::spawn_blocking(move || -> Result<_> {
        let model = artifact(repo.clone(), project, request)?;
        Ok((repo, model))
    })
    .await??;
    let key = flow_like_ml_core::content_digest(&flow_like_types::json::to_vec(&(
        repo.path(),
        &model.id,
        &model.blob.sha256,
        compute,
    ))?);
    let cache = {
        let mut run_cache = context.cache.write().await;
        let value = run_cache
            .entry("inspection-predictors-v1".into())
            .or_insert_with(|| std::sync::Arc::new(PredictionCache::default()));
        value
            .as_any()
            .downcast_ref::<PredictionCache>()
            .ok_or_else(|| anyhow!("Inspection predictor cache has an unexpected type"))?
            .clone()
    };
    let mut models = cache.models.lock().await;
    if let Some(index) = models.iter().position(|(id, _, _)| id == &key) {
        let entry = models.remove(index);
        let predictor = entry.1.clone();
        trim_prediction_cache(&mut models, entry.2, compute.memory_limit_bytes);
        models.push(entry);
        return Ok(Some(predictor));
    }
    let reserved = model
        .blob
        .bytes
        .checked_mul(4)
        .ok_or_else(|| anyhow!("Model cache size overflows"))?;
    let budget = compute.memory_limit_bytes;
    if reserved >= budget {
        return Err(anyhow!("Model exceeds the inference memory budget"));
    }
    trim_prediction_cache(&mut models, reserved, budget);
    let artifact_id = model.id;
    let compute = compute.clone();
    let predictor = std::sync::Arc::new(
        tokio::task::spawn_blocking(move || {
            CatalogLoadedPredictor::load(&repo, &artifact_id, &compute)
        })
        .await??,
    );
    models.push((key, predictor.clone(), reserved));
    Ok(Some(predictor))
}

#[flow_like_types::async_trait]
impl NodeLogic for StudentInferenceNode {
    fn get_node(&self) -> flow_like::flow::node::Node {
        let mut node = super::operation_node::<StudentInferenceRequest, Value>(
            "ml_student_inference",
            "Student Model Inference",
            "Predict with a versioned model reused within this workflow execution",
            "predict",
            "AI/ML/Lifecycle",
        );
        node.set_version(2);
        node
    }
    async fn run(&self, context: &mut ExecutionContext) -> Result<()> {
        #[cfg(feature = "inference")]
        {
            context.deactivate_exec_pin("exec_out").await?;
            let mut input: StudentInferenceRequest = context.evaluate_pin("request").await?;
            let extractor = input
                .feature_extractor
                .as_ref()
                .map(FeatureExtractorRef::to_runtime)
                .transpose()?;
            let predictor = cached_predictor(
                context,
                &input.inspection,
                &input.artifact_id,
                &input.compute,
            )
            .await?;
            if matches!(
                predictor.artifact().manifest["engine"].as_str(),
                Some("burn" | "efficient_ad" | "legacy_classical")
            ) && input.input.shape == input.inspection.spec.input_shape
            {
                input.input.shape.insert(0, 1);
            }
            let output = tokio::task::spawn_blocking(move || {
                predictor.predict_checked(&input.input, extractor.as_ref())
            })
            .await??;
            context.set_pin_value("result", json!(output)).await?;
            context.activate_exec_pin("exec_out").await?;
            Ok(())
        }
        #[cfg(not(feature = "inference"))]
        {
            let _ = context;
            Err(super::published::inference_unavailable())
        }
    }
}

#[crate::register_node]
#[derive(Default)]
pub struct RecordModelComparisonNode;
lifecycle_node!(
    RecordModelComparisonNode,
    "ml_record_model_comparison",
    "Record Model Comparison",
    "Store aligned teacher and student predictions with audited outcomes",
    "recordComparison",
    RecordPredictionRequest,
    bool,
    record_prediction
);

#[crate::register_node]
#[derive(Default)]
pub struct EvaluateStudentNode;
lifecycle_node!(
    EvaluateStudentNode,
    "ml_evaluate_student",
    "Evaluate Student",
    "Measure audited classification quality separately from teacher agreement",
    "evaluateStudent",
    ArtifactRequest,
    EvaluationReport,
    evaluate
);

#[crate::register_node]
#[derive(Default)]
pub struct PromoteStudentNode;
lifecycle_node!(
    PromoteStudentNode,
    "ml_promote_student",
    "Promote Student",
    "Atomically promote a model that passes the audited quality and latency policy",
    "promoteStudent",
    PromoteRequest,
    Deployment,
    promote,
    publish = publish_changed_deployment
);

#[crate::register_node]
#[derive(Default)]
pub struct GetDeploymentNode;
#[flow_like_types::async_trait]
impl NodeLogic for GetDeploymentNode {
    fn get_node(&self) -> flow_like::flow::node::Node {
        super::operation_node::<DeploymentRequest, Option<Deployment>>(
            "ml_get_deployment",
            "Get Active Student",
            "Resolve the promoted artifact; an empty result means the teacher still owns decisions",
            "deployment",
            "AI/ML/Lifecycle",
        )
    }
    async fn run(&self, context: &mut ExecutionContext) -> Result<()> {
        #[cfg(feature = "inference")]
        {
            context.deactivate_exec_pin("exec_out").await?;
            let input: DeploymentRequest = context.evaluate_pin("request").await?;
            let active = active_deployment(context, &input).await?;
            context.set_pin_value("result", json!(active)).await?;
            context.activate_exec_pin("exec_out").await?;
            Ok(())
        }
        #[cfg(not(feature = "inference"))]
        {
            let _ = context;
            Err(super::published::inference_unavailable())
        }
    }
}

#[crate::register_node]
#[derive(Default)]
pub struct RollbackStudentNode;
lifecycle_node!(
    RollbackStudentNode,
    "ml_rollback_student",
    "Rollback Student",
    "Atomically restore the previous model and its evaluation policy",
    "rollbackStudent",
    RollbackRequest,
    Deployment,
    rollback,
    publish = publish_changed_deployment
);

#[crate::register_node]
#[derive(Default)]
pub struct EvaluateInspectionTaskNode;
lifecycle_node!(
    EvaluateInspectionTaskNode,
    "ml_evaluate_inspection_task",
    "Evaluate Inspection Task",
    "Derive audited regression, detection, segmentation or event metrics from held-out evidence",
    "evaluateTask",
    EvaluateTaskRequest,
    EvaluationReport,
    evaluate_task
);
#[crate::register_node]
#[derive(Default)]
pub struct PromoteByMetricsNode;
lifecycle_node!(
    PromoteByMetricsNode,
    "ml_promote_by_metrics",
    "Promote by Audited Metrics",
    "Promote an artifact only when its sealed evaluation meets named metric bounds",
    "promoteByMetrics",
    PromoteMetricsRequest,
    Deployment,
    promote_metrics,
    publish = publish_changed_deployment
);

#[crate::register_node]
#[derive(Default)]
pub struct ExtractPatchFeaturesNode;
#[flow_like_types::async_trait]
impl NodeLogic for ExtractPatchFeaturesNode {
    fn get_node(&self) -> flow_like::flow::node::Node {
        use flow_like::flow::variable::VariableType;
        let mut node = super::operation_node::<PredictRequest, TensorData>(
            "ml_extract_patch_features",
            "Extract Image Features",
            "Extract spatial features from a Burn classifier reused within this workflow execution",
            "imageFeatures",
            "AI/ML/Lifecycle",
        );
        node.add_output_pin(
            "extractor",
            "Feature Extractor",
            "Model and image steps that computed these features. Pass it to PatchCore or PaDiM training and inference so features from another extractor are refused",
            VariableType::Struct,
        )
        .set_schema::<FeatureExtractorRef>();
        node.set_version(1);
        node
    }
    async fn run(&self, context: &mut ExecutionContext) -> Result<()> {
        #[cfg(feature = "inference")]
        {
            context.deactivate_exec_pin("exec_out").await?;
            let mut input: PredictRequest = context.evaluate_pin("request").await?;
            let predictor = cached_predictor(
                context,
                &input.inspection,
                &input.artifact_id,
                &input.compute,
            )
            .await?;
            if input.input.shape == input.inspection.spec.input_shape {
                input.input.shape.insert(0, 1);
            }
            let (features, extractor) = tokio::task::spawn_blocking(move || -> Result<_> {
                Ok((
                    predictor.features(&input.input)?,
                    predictor.feature_extractor()?,
                ))
            })
            .await??;
            context.set_pin_value("result", json!(features)).await?;
            context.set_pin_value("extractor", json!(extractor)).await?;
            context.activate_exec_pin("exec_out").await?;
            Ok(())
        }
        #[cfg(not(feature = "inference"))]
        {
            let _ = context;
            Err(super::published::inference_unavailable())
        }
    }
}

#[derive(Clone, Serialize, Deserialize, JsonSchema)]
pub struct ExportOnnxRequest {
    pub prediction: PredictRequest,
    pub destination: flow_like_catalog_core::FlowPath,
}
#[derive(Clone, Serialize, Deserialize, JsonSchema)]
pub struct ExportOnnxResult {
    pub path: flow_like_catalog_core::FlowPath,
    pub sha256: String,
    pub bytes: u64,
    pub opset: u32,
    pub input_shape: Vec<usize>,
    pub output_semantics: String,
}
#[crate::register_node]
#[derive(Default)]
pub struct ExportStudentOnnxNode;
#[flow_like_types::async_trait]
impl NodeLogic for ExportStudentOnnxNode {
    fn get_node(&self) -> flow_like::flow::node::Node {
        super::operation_node::<ExportOnnxRequest, ExportOnnxResult>(
            "ml_export_student_onnx",
            "Export Student to ONNX",
            "Export a supported inference graph with fixed dimensions and explicit output semantics",
            "exportOnnx",
            "AI/ML/Export",
        )
    }
    async fn run(&self, context: &mut ExecutionContext) -> Result<()> {
        #[cfg(feature = "onnx-export")]
        {
            context.deactivate_exec_pin("exec_out").await?;
            let input: ExportOnnxRequest = context.evaluate_pin("request").await?;
            let ledger = LedgerLocation::for_stream(context, input.stream_id())?;
            let mut prediction = input.prediction;
            if prediction.input.shape == prediction.inspection.spec.input_shape {
                prediction.input.shape.insert(0, 1);
            }
            let (bytes, report) = tokio::task::spawn_blocking(move || -> Result<_> {
                let Some((repo, project)) = ledger.open_existing()? else {
                    return Err(anyhow!(
                        "Artifact {} cannot be exported: stream {} has no training ledger on this device",
                        prediction.artifact_id,
                        prediction.inspection.stream_id
                    ));
                };
                artifact(
                    repo.clone(),
                    project,
                    ArtifactRequest {
                        inspection: prediction.inspection,
                        artifact_id: prediction.artifact_id.clone(),
                    },
                )?;
                Ok(runtime::engines::export_artifact_onnx(
                    &repo,
                    &prediction.artifact_id,
                    &prediction.input,
                    &prediction.compute,
                )?)
            })
            .await??;
            let output = ExportOnnxResult {
                path: input.destination.clone(),
                sha256: flow_like_ml_core::content_digest(&bytes),
                bytes: bytes.len() as u64,
                opset: report.opset,
                input_shape: report.input_shape,
                output_semantics: report.output_semantics,
            };
            input.destination.put(context, bytes, false).await?;
            context.set_pin_value("result", json!(output)).await?;
            context.activate_exec_pin("exec_out").await?;
            Ok(())
        }
        #[cfg(all(feature = "training", not(feature = "onnx-export")))]
        {
            let _ = context;
            Err(anyhow!("This executor needs the onnx-export feature"))
        }
        #[cfg(not(feature = "training"))]
        {
            let _ = context;
            Err(training_unavailable())
        }
    }
}
#[cfg(feature = "training")]
fn train_mlp(
    repo: TrainingRepository,
    project: String,
    input: TrainBurnRequest,
) -> Result<Option<TrainingJob>> {
    if !matches!(input.config.recipe, flow_like_ml_burn::Recipe::Mlp { .. }) {
        return Err(anyhow!("Select the MLP recipe for this node"));
    }
    train_burn(repo, project, input)
}
#[crate::register_node]
#[derive(Default)]
pub struct TrainMlpNode;
lifecycle_node!(
    TrainMlpNode,
    "ml_train_mlp",
    "Train MLP",
    "Schedule MLP training with durable checkpoints and explicit device selection",
    "trainMlp",
    TrainBurnRequest,
    Option<TrainingJob>,
    train_mlp
);

#[cfg(feature = "training")]
fn train_lstm(
    repo: TrainingRepository,
    project: String,
    input: TrainBurnRequest,
) -> Result<Option<TrainingJob>> {
    if !matches!(input.config.recipe, flow_like_ml_burn::Recipe::Lstm { .. }) {
        return Err(anyhow!("Select the LSTM recipe for this node"));
    }
    train_burn(repo, project, input)
}
#[crate::register_node]
#[derive(Default)]
pub struct TrainLstmNode;
lifecycle_node!(
    TrainLstmNode,
    "ml_train_lstm",
    "Train LSTM",
    "Schedule LSTM training with durable checkpoints and explicit device selection",
    "trainLstm",
    TrainBurnRequest,
    Option<TrainingJob>,
    train_lstm
);

#[cfg(feature = "training")]
fn train_gru(
    repo: TrainingRepository,
    project: String,
    input: TrainBurnRequest,
) -> Result<Option<TrainingJob>> {
    if !matches!(input.config.recipe, flow_like_ml_burn::Recipe::Gru { .. }) {
        return Err(anyhow!("Select the GRU recipe for this node"));
    }
    train_burn(repo, project, input)
}
#[crate::register_node]
#[derive(Default)]
pub struct TrainGruNode;
lifecycle_node!(
    TrainGruNode,
    "ml_train_gru",
    "Train GRU",
    "Schedule GRU training with durable checkpoints and explicit device selection",
    "trainGru",
    TrainBurnRequest,
    Option<TrainingJob>,
    train_gru
);

#[cfg(feature = "training")]
fn train_cnn1d(
    repo: TrainingRepository,
    project: String,
    input: TrainBurnRequest,
) -> Result<Option<TrainingJob>> {
    if !matches!(input.config.recipe, flow_like_ml_burn::Recipe::Cnn1d { .. }) {
        return Err(anyhow!("Select the 1D CNN recipe for this node"));
    }
    train_burn(repo, project, input)
}
#[crate::register_node]
#[derive(Default)]
pub struct TrainCnn1dNode;
lifecycle_node!(
    TrainCnn1dNode,
    "ml_train_cnn1d",
    "Train 1D CNN",
    "Schedule 1D CNN training with durable checkpoints and explicit device selection",
    "trainCnn1d",
    TrainBurnRequest,
    Option<TrainingJob>,
    train_cnn1d
);

#[cfg(feature = "training")]
fn train_tcn(
    repo: TrainingRepository,
    project: String,
    input: TrainBurnRequest,
) -> Result<Option<TrainingJob>> {
    if !matches!(input.config.recipe, flow_like_ml_burn::Recipe::Tcn { .. }) {
        return Err(anyhow!("Select the TCN recipe for this node"));
    }
    train_burn(repo, project, input)
}
#[crate::register_node]
#[derive(Default)]
pub struct TrainTcnNode;
lifecycle_node!(
    TrainTcnNode,
    "ml_train_tcn",
    "Train TCN",
    "Schedule TCN training with durable checkpoints and explicit device selection",
    "trainTcn",
    TrainBurnRequest,
    Option<TrainingJob>,
    train_tcn
);

#[cfg(feature = "training")]
fn train_dense_autoencoder(
    repo: TrainingRepository,
    project: String,
    input: TrainBurnRequest,
) -> Result<Option<TrainingJob>> {
    if !matches!(
        input.config.recipe,
        flow_like_ml_burn::Recipe::DenseAutoencoder { .. }
    ) {
        return Err(anyhow!("Select the Dense Autoencoder recipe for this node"));
    }
    train_burn(repo, project, input)
}
#[crate::register_node]
#[derive(Default)]
pub struct TrainDenseAutoencoderNode;
lifecycle_node!(
    TrainDenseAutoencoderNode,
    "ml_train_dense_autoencoder",
    "Train Dense Autoencoder",
    "Schedule Dense Autoencoder training with durable checkpoints and explicit device selection",
    "trainDenseAutoencoder",
    TrainBurnRequest,
    Option<TrainingJob>,
    train_dense_autoencoder
);

#[cfg(feature = "training")]
fn train_conv1d_autoencoder(
    repo: TrainingRepository,
    project: String,
    input: TrainBurnRequest,
) -> Result<Option<TrainingJob>> {
    if !matches!(
        input.config.recipe,
        flow_like_ml_burn::Recipe::Conv1dAutoencoder { .. }
    ) {
        return Err(anyhow!(
            "Select the Convolutional Sequence Autoencoder recipe for this node"
        ));
    }
    train_burn(repo, project, input)
}
#[crate::register_node]
#[derive(Default)]
pub struct TrainConv1dAutoencoderNode;
lifecycle_node!(
    TrainConv1dAutoencoderNode,
    "ml_train_conv1d_autoencoder",
    "Train Convolutional Sequence Autoencoder",
    "Schedule Convolutional Sequence Autoencoder training with durable checkpoints and explicit device selection",
    "trainConv1dAutoencoder",
    TrainBurnRequest,
    Option<TrainingJob>,
    train_conv1d_autoencoder
);

#[cfg(feature = "training")]
fn train_res_net18(
    repo: TrainingRepository,
    project: String,
    input: TrainBurnRequest,
) -> Result<Option<TrainingJob>> {
    if !matches!(
        input.config.recipe,
        flow_like_ml_burn::Recipe::ResNet18 { .. }
    ) {
        return Err(anyhow!("Select the ResNet-18 recipe for this node"));
    }
    train_burn(repo, project, input)
}
#[crate::register_node]
#[derive(Default)]
pub struct TrainResNet18Node;
lifecycle_node!(
    TrainResNet18Node,
    "ml_train_res_net18",
    "Train ResNet-18",
    "Schedule ResNet-18 training with durable checkpoints and explicit device selection",
    "trainResNet18",
    TrainBurnRequest,
    Option<TrainingJob>,
    train_res_net18
);

#[cfg(feature = "training")]
fn train_dino_v2(
    repo: TrainingRepository,
    project: String,
    input: TrainBurnRequest,
) -> Result<Option<TrainingJob>> {
    if !matches!(
        input.config.recipe,
        flow_like_ml_burn::Recipe::DinoV2 { .. }
    ) {
        return Err(anyhow!("Select the DINOv2 recipe for this node"));
    }
    train_burn(repo, project, input)
}
#[crate::register_node]
#[derive(Default)]
pub struct TrainDinoV2Node;
lifecycle_node!(
    TrainDinoV2Node,
    "ml_train_dino_v2",
    "Train DINOv2",
    "Train a DINOv2 image classifier from scratch or compatible pretrained weights",
    "trainDinoV2",
    TrainBurnRequest,
    Option<TrainingJob>,
    train_dino_v2
);

#[cfg(feature = "training")]
fn train_rtdetr_v2(
    repo: TrainingRepository,
    project: String,
    input: TrainBurnRequest,
) -> Result<Option<TrainingJob>> {
    if !matches!(
        input.config.recipe,
        flow_like_ml_burn::Recipe::RtdetrV2 { .. }
    ) {
        return Err(anyhow!("Select the RT-DETRv2 recipe for this node"));
    }
    train_burn(repo, project, input)
}
#[crate::register_node]
#[derive(Default)]
pub struct TrainRtdetrV2Node;
lifecycle_node!(
    TrainRtdetrV2Node,
    "ml_train_rtdetr_v2",
    "Train RT-DETRv2",
    "Train a native RT-DETRv2 object detector from scratch or compatible pretrained weights",
    "trainRtdetrV2",
    TrainBurnRequest,
    Option<TrainingJob>,
    train_rtdetr_v2
);

#[cfg(feature = "training")]
fn train_rf_detr(
    repo: TrainingRepository,
    project: String,
    input: TrainBurnRequest,
) -> Result<Option<TrainingJob>> {
    if !matches!(
        input.config.recipe,
        flow_like_ml_burn::Recipe::RfDetr { .. }
    ) {
        return Err(anyhow!("Select the RF-DETR recipe for this node"));
    }
    train_burn(repo, project, input)
}
#[crate::register_node]
#[derive(Default)]
pub struct TrainRfDetrNode;
lifecycle_node!(
    TrainRfDetrNode,
    "ml_train_rf_detr",
    "Train RF-DETR",
    "Train a native RF-DETR object detector with grouped queries and compatible pretrained weights",
    "trainRfDetr",
    TrainBurnRequest,
    Option<TrainingJob>,
    train_rf_detr
);

#[cfg(feature = "training")]
fn train_dfine_nano(
    repo: TrainingRepository,
    project: String,
    input: TrainBurnRequest,
) -> Result<Option<TrainingJob>> {
    if !matches!(
        input.config.recipe,
        flow_like_ml_burn::Recipe::DfineNano { .. }
    ) {
        return Err(anyhow!("Select the D-FINE Nano recipe for this node"));
    }
    train_burn(repo, project, input)
}
#[crate::register_node]
#[derive(Default)]
pub struct TrainDfineNanoNode;
lifecycle_node!(
    TrainDfineNanoNode,
    "ml_train_dfine_nano",
    "Train D-FINE Nano",
    "Fine-tune a native D-FINE Nano detector with matching and auxiliary classification and box losses",
    "trainDfineNano",
    TrainBurnRequest,
    Option<TrainingJob>,
    train_dfine_nano
);

#[cfg(feature = "training")]
fn train_mobile_net_v2(
    repo: TrainingRepository,
    project: String,
    input: TrainBurnRequest,
) -> Result<Option<TrainingJob>> {
    if !matches!(
        input.config.recipe,
        flow_like_ml_burn::Recipe::MobileNetV2 { .. }
    ) {
        return Err(anyhow!("Select the MobileNetV2 recipe for this node"));
    }
    train_burn(repo, project, input)
}
#[crate::register_node]
#[derive(Default)]
pub struct TrainMobileNetV2Node;
lifecycle_node!(
    TrainMobileNetV2Node,
    "ml_train_mobile_net_v2",
    "Train MobileNetV2",
    "Schedule MobileNetV2 training with durable checkpoints and explicit device selection",
    "trainMobileNetV2",
    TrainBurnRequest,
    Option<TrainingJob>,
    train_mobile_net_v2
);

#[cfg(feature = "training")]
fn train_efficient_net(
    repo: TrainingRepository,
    project: String,
    input: TrainBurnRequest,
) -> Result<Option<TrainingJob>> {
    if !matches!(
        input.config.recipe,
        flow_like_ml_burn::Recipe::EfficientNet { .. }
    ) {
        return Err(anyhow!("Select the EfficientNet recipe for this node"));
    }
    train_burn(repo, project, input)
}
#[crate::register_node]
#[derive(Default)]
pub struct TrainEfficientNetNode;
lifecycle_node!(
    TrainEfficientNetNode,
    "ml_train_efficient_net",
    "Train EfficientNet",
    "Schedule EfficientNet training with durable checkpoints and explicit device selection",
    "trainEfficientNet",
    TrainBurnRequest,
    Option<TrainingJob>,
    train_efficient_net
);

#[cfg(feature = "training")]
fn train_yolo_x(
    repo: TrainingRepository,
    project: String,
    input: TrainBurnRequest,
) -> Result<Option<TrainingJob>> {
    if !matches!(input.config.recipe, flow_like_ml_burn::Recipe::YoloX { .. }) {
        return Err(anyhow!("Select the YOLOX recipe for this node"));
    }
    train_burn(repo, project, input)
}
#[crate::register_node]
#[derive(Default)]
pub struct TrainYoloXNode;
lifecycle_node!(
    TrainYoloXNode,
    "ml_train_yolo_x",
    "Train YOLOX",
    "Schedule YOLOX training with durable checkpoints and explicit device selection",
    "trainYoloX",
    TrainBurnRequest,
    Option<TrainingJob>,
    train_yolo_x
);

#[cfg(feature = "training")]
fn train_u_net(
    repo: TrainingRepository,
    project: String,
    input: TrainBurnRequest,
) -> Result<Option<TrainingJob>> {
    if !matches!(input.config.recipe, flow_like_ml_burn::Recipe::UNet { .. }) {
        return Err(anyhow!("Select the U-Net recipe for this node"));
    }
    train_burn(repo, project, input)
}
#[crate::register_node]
#[derive(Default)]
pub struct TrainUNetNode;
lifecycle_node!(
    TrainUNetNode,
    "ml_train_u_net",
    "Train U-Net",
    "Schedule U-Net training with durable checkpoints and explicit device selection",
    "trainUNet",
    TrainBurnRequest,
    Option<TrainingJob>,
    train_u_net
);
#[cfg(feature = "training")]
fn train_lstm_autoencoder(
    repo: TrainingRepository,
    project: String,
    input: TrainBurnRequest,
) -> Result<Option<TrainingJob>> {
    if !matches!(
        input.config.recipe,
        flow_like_ml_burn::Recipe::LstmAutoencoder { .. }
    ) {
        return Err(anyhow!("Select the LSTM Autoencoder recipe for this node"));
    }
    train_burn(repo, project, input)
}
#[crate::register_node]
#[derive(Default)]
pub struct TrainLstmAutoencoderNode;
lifecycle_node!(
    TrainLstmAutoencoderNode,
    "ml_train_lstm_autoencoder",
    "Train LSTM Autoencoder",
    "Schedule LSTM Autoencoder with durable checkpoints and explicit device selection",
    "trainLstmAutoencoder",
    TrainBurnRequest,
    Option<TrainingJob>,
    train_lstm_autoencoder
);

#[cfg(feature = "training")]
fn train_cnn_lstm(
    repo: TrainingRepository,
    project: String,
    input: TrainBurnRequest,
) -> Result<Option<TrainingJob>> {
    if !matches!(
        input.config.recipe,
        flow_like_ml_burn::Recipe::CnnLstm { .. }
    ) {
        return Err(anyhow!(
            "Select the CNN-LSTM Video Model recipe for this node"
        ));
    }
    train_burn(repo, project, input)
}
#[crate::register_node]
#[derive(Default)]
pub struct TrainCnnLstmNode;
lifecycle_node!(
    TrainCnnLstmNode,
    "ml_train_cnn_lstm",
    "Train CNN-LSTM Video Model",
    "Schedule CNN-LSTM Video Model with durable checkpoints and explicit device selection",
    "trainCnnLstm",
    TrainBurnRequest,
    Option<TrainingJob>,
    train_cnn_lstm
);

#[cfg(feature = "training")]
fn train_image_sensor_fusion(
    repo: TrainingRepository,
    project: String,
    input: TrainBurnRequest,
) -> Result<Option<TrainingJob>> {
    if !matches!(
        input.config.recipe,
        flow_like_ml_burn::Recipe::ImageSensorFusion { .. }
    ) {
        return Err(anyhow!(
            "Select the Image/Sensor Fusion Model recipe for this node"
        ));
    }
    train_burn(repo, project, input)
}
#[crate::register_node]
#[derive(Default)]
pub struct TrainImageSensorFusionNode;
lifecycle_node!(
    TrainImageSensorFusionNode,
    "ml_train_image_sensor_fusion",
    "Train Image/Sensor Fusion Model",
    "Schedule Image/Sensor Fusion Model with durable checkpoints and explicit device selection",
    "trainImageSensorFusion",
    TrainBurnRequest,
    Option<TrainingJob>,
    train_image_sensor_fusion
);

#[crate::register_node]
#[derive(Default)]
pub struct PauseStudentNode;
#[crate::register_node]
#[derive(Default)]
pub struct TrainEfficientAdNode;
#[crate::register_node]
#[derive(Default)]
pub struct TrainMaskRcnnNode;
#[cfg(feature = "training")]
fn train_mask_rcnn(
    repo: TrainingRepository,
    project: String,
    input: TrainBurnRequest,
) -> Result<Option<TrainingJob>> {
    if !matches!(
        input.config.recipe,
        flow_like_ml_burn::Recipe::MaskRcnn { .. }
    ) {
        return Err(anyhow!("Select the Mask R-CNN recipe for this node"));
    }
    train_burn(repo, project, input)
}
lifecycle_node!(
    TrainMaskRcnnNode,
    "ml_train_mask_rcnn",
    "Train Mask R-CNN",
    "Train a two-stage detector with region proposals, ROIAlign, boxes and instance masks",
    "trainMaskRcnn",
    TrainBurnRequest,
    Option<TrainingJob>,
    train_mask_rcnn
);
lifecycle_node!(
    TrainEfficientAdNode,
    "ml_train_efficient_ad",
    "Train EfficientAD",
    "Train a PDN teacher/student and autoencoder anomaly model from supplied pretrained teacher weights and normal images",
    "trainEfficientAd",
    TrainEfficientAdRequest,
    Option<TrainingJob>,
    train_efficient_ad
);
lifecycle_node!(
    PauseStudentNode,
    "ml_pause_student",
    "Pause/Resume Student Routing",
    "Return decisions to the teacher after drift or resume an audited deployment",
    "pauseStudent",
    PauseDeploymentRequest,
    Deployment,
    pause_deployment,
    publish = publish_changed_deployment
);

#[crate::register_node]
#[derive(Default)]
pub struct RouteInspectionNode;
#[flow_like_types::async_trait]
impl NodeLogic for RouteInspectionNode {
    fn get_node(&self) -> flow_like::flow::node::Node {
        use flow_like::flow::variable::VariableType;
        let mut node = super::operation_node::<DeploymentRequest, InspectionRoute>(
            "ml_route_inspection",
            "Route Inspection",
            "Route to the teacher during collection/shadow evaluation and to the audited student after promotion",
            "route",
            "AI/ML/Lifecycle",
        );
        node.add_output_pin(
            "teacher",
            "Teacher",
            "Teacher owns this decision",
            VariableType::Execution,
        );
        node.add_output_pin(
            "student",
            "Student",
            "Promoted student owns this decision",
            VariableType::Execution,
        );
        node.add_output_pin(
            "shadow",
            "Shadow Candidate",
            "Run the candidate for comparison while the teacher owns the decision",
            VariableType::Execution,
        );
        node
    }
    async fn run(&self, context: &mut ExecutionContext) -> Result<()> {
        #[cfg(feature = "inference")]
        {
            for pin in ["teacher", "student", "shadow", "exec_out"] {
                context.deactivate_exec_pin(pin).await?;
            }
            let input: DeploymentRequest = context.evaluate_pin("request").await?;
            let route = inspection_route(context, &input).await?;
            context.set_pin_value("result", json!(route)).await?;
            match route.mode {
                InspectionMode::StudentOnly => context.activate_exec_pin("student").await?,
                InspectionMode::TeacherOnly => context.activate_exec_pin("teacher").await?,
                InspectionMode::Shadow => {
                    context.activate_exec_pin("teacher").await?;
                    context.activate_exec_pin("shadow").await?;
                }
            }
            context.activate_exec_pin("exec_out").await?;
            Ok(())
        }
        #[cfg(not(feature = "inference"))]
        {
            let _ = context;
            Err(super::published::inference_unavailable())
        }
    }
}

#[cfg(test)]
mod scope_tests {
    use super::*;

    #[test]
    fn stream_authorization_includes_project_stream_and_canonical_specification() {
        let inspection = InspectionContext {
            stream_id: "camera-1".into(),
            spec: InspectionSpec {
                id: "surface".into(),
                description: "Surface classification".into(),
                task: flow_like_ml_core::TaskKind::ImageClassification,
                labels: vec!["good".into(), "scratch".into()],
                input_shape: vec![3, 16, 16],
                prediction_horizon_ms: None,
                minimum_examples: 10,
                minimum_examples_per_class: 2,
            },
        };
        let expected = inspection.key("project-a".into()).unwrap();
        let reordered: InspectionContext = flow_like_types::json::from_value(flow_like_types::json::json!({
            "spec": {"input_shape":[3,16,16], "minimum_examples_per_class":2,"minimum_examples":10,
                "labels":["good","scratch"],"task":"image_classification","description":"Surface classification",
                "prediction_horizon_ms":null,"id":"surface"}, "stream_id":"camera-1"
        })).unwrap();
        check_stream(&expected, &reordered.key("project-a".into()).unwrap()).unwrap();
        assert!(check_stream(&expected, &inspection.key("project-b".into()).unwrap()).is_err());
        let mut changed = inspection.clone();
        changed.stream_id = "camera-2".into();
        assert!(check_stream(&expected, &changed.key("project-a".into()).unwrap()).is_err());
        changed = inspection;
        changed.spec.labels.swap(0, 1);
        assert!(check_stream(&expected, &changed.key("project-a".into()).unwrap()).is_err());
    }

    #[test]
    fn native_gpu_requests_fail_before_scheduling_unless_fallback_is_explicit() {
        for engine in [
            "isolation_forest",
            "histogram_gradient_boosting",
            "patchcore",
            "padim",
        ] {
            let mut compute = ComputeConfig {
                backend: flow_like_ml_core::ComputeBackend::Wgpu,
                allow_cpu_fallback: false,
                ..Default::default()
            };
            assert!(validate_native_schedule_compute(engine, &compute).is_err());
            compute.allow_cpu_fallback = true;
            validate_native_schedule_compute(engine, &compute).unwrap();
            validate_native_schedule_compute(engine, &ComputeConfig::default()).unwrap();
        }
    }

    const UUID: &str = "0f8e5c1a-3b2d-4c6e-9a7f-1d2e3f4a5b6c";

    #[test]
    fn only_canonical_minted_user_streams_route_to_the_private_ledger() {
        for kind in ["auto", "learning"] {
            assert_eq!(
                LedgerScope::of_stream(&format!("user:{kind}:{UUID}")),
                LedgerScope::User
            );
            assert_eq!(
                LedgerScope::of_stream(&format!("{kind}:{UUID}")),
                LedgerScope::App
            );
        }
        let manual = [
            "user:camera".to_string(),
            "user:auto:not-a-uuid".to_string(),
            format!("user:auto:{}", UUID.to_uppercase()),
            format!("user:auto:{UUID}-extra"),
            format!("user:auto:{}", UUID.replace('-', "")),
            format!("user:other:{UUID}"),
            format!("user:user:auto:{UUID}"),
            "camera-1".to_string(),
            String::new(),
        ];
        for stream in &manual {
            assert_eq!(LedgerScope::of_stream(stream), LedgerScope::App, "{stream}");
        }
        assert_eq!(
            StreamKind::of_stream(&format!("auto:{UUID}")),
            Some(StreamKind::Auto)
        );
        assert_eq!(
            StreamKind::of_stream(&format!("user:learning:{UUID}")),
            Some(StreamKind::Learning)
        );
        assert_eq!(StreamKind::of_stream("auto:camera"), None);
        assert_eq!(StreamKind::of_stream("user:camera"), None);
    }

    #[test]
    fn deployment_and_project_ids_carry_the_ledger_scope() {
        assert_eq!(
            LedgerScope::of_deployment(&format!("deployment:user:learning:{UUID}")),
            LedgerScope::User
        );
        assert_eq!(
            LedgerScope::of_deployment(&format!("deployment:learning:{UUID}")),
            LedgerScope::App
        );
        assert_eq!(
            LedgerScope::of_deployment(&format!("user:learning:{UUID}")),
            LedgerScope::App
        );
        assert_eq!(LedgerScope::of_deployment("line-3"), LedgerScope::App);
        for scope in [LedgerScope::App, LedgerScope::User] {
            let project = scope.project_id("app-1", "board-1");
            assert_eq!(LedgerScope::of_project(&project), scope, "{project}");
        }
        assert_eq!(LedgerScope::User.project_id("a", "b"), "a:b:user");
        assert_eq!(LedgerScope::App.project_id("a", "b"), "a:b");
        assert_eq!(LedgerScope::of_project("app:user"), LedgerScope::App);
        assert_eq!(LedgerScope::of_project(":user"), LedgerScope::App);
    }

    #[test]
    fn minted_stream_ids_are_recognized_in_their_scope() {
        for scope in [LedgerScope::App, LedgerScope::User] {
            for kind in [StreamKind::Auto, StreamKind::Learning] {
                let stream = scope.new_stream_id(kind);
                assert_eq!(LedgerScope::of_stream(&stream), scope, "{stream}");
                assert_eq!(StreamKind::of_stream(&stream), Some(kind), "{stream}");
                assert_eq!(
                    LedgerScope::of_deployment(&format!("deployment:{stream}")),
                    scope
                );
            }
        }
        assert_eq!(
            flow_like_types::json::to_value(LedgerScope::User).unwrap(),
            flow_like_types::json::json!("user")
        );
        assert_eq!(LedgerScope::default(), LedgerScope::App);
    }

    #[test]
    fn training_refusals_are_recognizable_after_wrapping() {
        let error = training_unavailable();
        assert_eq!(error.to_string(), TRAINING_UNAVAILABLE);
        assert!(is_training_unavailable(&error));
        assert!(is_training_unavailable(
            &error.context("Auto Train Tabular")
        ));
        assert!(!is_training_unavailable(&anyhow!(TRAINING_UNAVAILABLE)));
    }
}

/// Node runs on a device whose Local app and user stores live in one temporary directory.
/// Runs belong to app `app`, board `board` and user `user-1`, so App records carry the
/// project `app:board`.
#[cfg(all(test, feature = "training"))]
pub(crate) mod test_device {
    use super::*;
    use ahash::AHashMap;
    use flow_like::{
        flow::{
            board::ExecutionStage,
            execution::{
                LogLevel, Run, context::ExecutionContextCache, internal_node::InternalNode,
                internal_pin::InternalPin,
            },
        },
        profile::Profile,
        state::{FlowLikeConfig, FlowLikeState, FlowLikeStores},
        utils::http::HTTPClient,
    };
    use flow_like_ml_core::{Annotation, LabelProvenance, TaskKind};
    use flow_like_storage::{
        files::store::{FlowLikeStore, local_store::LocalObjectStore},
        object_store::{ObjectStoreExt, memory::InMemory},
    };
    use flow_like_types::{
        Cacheable,
        sync::{Mutex, RwLock},
    };
    use std::sync::{Arc, Weak};

    pub(crate) const BOARD: &str = "board";
    /// App-scope models, relative to the app store.
    pub(crate) const APP_MODELS: &str = "apps/app/storage/models";
    /// The executing user's models, relative to the user store.
    pub(crate) const USER_MODELS: &str = "users/user-1/apps/app/models";

    /// The cache of one run; contexts that share it belong to the same run.
    pub(crate) type RunCache = Arc<RwLock<AHashMap<String, Arc<dyn Cacheable>>>>;

    pub(crate) struct Device {
        directory: tempfile::TempDir,
        pub(crate) app: FlowLikeStore,
        pub(crate) user: FlowLikeStore,
    }

    impl Device {
        pub(crate) fn new() -> Self {
            let directory = tempfile::tempdir().unwrap();
            let local = |name: &str| {
                let store = LocalObjectStore::new(directory.path().join(name)).unwrap();
                FlowLikeStore::Local(Arc::new(store))
            };
            let (app, user) = (local("store"), local("user"));
            Self {
                directory,
                app,
                user,
            }
        }

        /// The canonical root of the app store.
        pub(crate) fn root(&self) -> PathBuf {
            self.directory.path().join("store").canonicalize().unwrap()
        }

        /// A directory outside both stores.
        pub(crate) fn scratch(&self, name: &str) -> PathBuf {
            self.directory.path().join(name)
        }

        pub(crate) fn stores(&self) -> FlowLikeStores {
            stores(Some(self.app.clone()), Some(self.user.clone()))
        }
    }

    pub(crate) fn stores(app: Option<FlowLikeStore>, user: Option<FlowLikeStore>) -> FlowLikeStores {
        let mut stores = FlowLikeStores::default();
        stores.app_storage_store = app;
        stores.user_store = user;
        stores
    }

    pub(crate) fn memory_store() -> FlowLikeStore {
        FlowLikeStore::Memory(Arc::new(InMemory::new()))
    }

    /// A Memory store with the objects under `prefix` of `store`, as a runtime without a
    /// ledger reads published files.
    pub(crate) async fn published_copy(store: &FlowLikeStore, prefix: &str) -> FlowLikeStore {
        use futures::TryStreamExt;
        let source = store.as_generic();
        let copy = Arc::new(InMemory::new());
        let objects: Vec<_> = source
            .list(Some(&flow_like_storage::Path::from(prefix)))
            .try_collect()
            .await
            .unwrap();
        assert!(!objects.is_empty(), "nothing is published under {prefix}");
        for object in objects {
            let bytes = source.get(&object.location).await.unwrap().bytes().await;
            copy.put(&object.location, bytes.unwrap().into())
                .await
                .unwrap();
        }
        FlowLikeStore::Memory(copy)
    }

    /// A context of a run on `board` that runs `logic` with `request` on its request pin.
    pub(crate) async fn node_context(
        logic: Arc<dyn NodeLogic>,
        request: Value,
        stores: FlowLikeStores,
        run: &RunCache,
        board: &str,
    ) -> ExecutionContext {
        let node = logic.get_node();
        let pins = node
            .pins
            .values()
            .map(|pin| (pin.id.clone(), Arc::new(InternalPin::new(pin, false))))
            .collect();
        let node = Arc::new(InternalNode::new(node, pins, logic, AHashMap::new()));
        if let Ok(pin) = node.get_pin_by_name("request").await {
            pin.set_value(request).await;
        }
        let state = Arc::new(FlowLikeState::new(
            FlowLikeConfig::new(),
            HTTPClient::new_without_refetch(),
        ));
        let detached: Weak<Mutex<Run>> = Weak::new();
        let mut context = ExecutionContext::new(
            Arc::new(AHashMap::from_iter([(
                node.node_id().to_string(),
                node.clone(),
            )])),
            &detached,
            &state,
            &node,
            &Arc::new(Mutex::new(AHashMap::new())),
            run,
            LogLevel::Debug,
            ExecutionStage::Dev,
            Arc::new(Profile::default()),
            None,
            Arc::new(RwLock::new(Vec::new())),
            None,
            None,
            Arc::new(AHashMap::new()),
            None,
        )
        .await;
        context.execution_cache = Some(ExecutionContextCache {
            stores,
            app_id: "app".into(),
            model_usage_app_id: None,
            board_dir: flow_like_storage::Path::from("apps").join("app"),
            board_id: board.into(),
            node_id: node.shared_node_id(),
            sub: "user-1".into(),
            shadow: false,
        });
        context
    }

    /// A context of a new run on `board` that runs no node.
    pub(crate) async fn plain_context(stores: FlowLikeStores) -> ExecutionContext {
        let logic: Arc<dyn NodeLogic> = Arc::new(GetModelArtifactNode);
        node_context(logic, Value::Null, stores, &RunCache::default(), BOARD).await
    }

    /// Runs `logic` like a run on `board` would and returns its context for the outputs.
    pub(crate) async fn run_on(
        logic: impl NodeLogic + 'static,
        request: Value,
        stores: FlowLikeStores,
        run: &RunCache,
        board: &str,
    ) -> Result<ExecutionContext> {
        let logic: Arc<dyn NodeLogic> = Arc::new(logic);
        let mut context = node_context(logic.clone(), request, stores, run, board).await;
        logic.run(&mut context).await?;
        Ok(context)
    }

    pub(crate) async fn output(context: &ExecutionContext, pin: &str) -> Value {
        let pin = context.get_pin_by_name(pin).await.unwrap();
        pin.get_raw_value().await.unwrap_or(Value::Null)
    }

    /// The result of `logic` run in a new run on board `board`.
    pub(crate) async fn run_node(
        logic: impl NodeLogic + 'static,
        request: Value,
        stores: FlowLikeStores,
    ) -> Result<Value> {
        let context = run_on(logic, request, stores, &RunCache::default(), BOARD).await?;
        Ok(output(&context, "result").await)
    }

    /// Exports the artifact's folder to `models/artifacts/{id}` of the app storage, where the
    /// App scope resolves it.
    pub(crate) async fn publish_artifact_folder(
        context: &mut ExecutionContext,
        repo: &TrainingRepository,
        artifact_id: &str,
    ) {
        let root = flow_like_catalog_core::FlowPath::from_storage_dir(context, false)
            .await
            .unwrap();
        let destination = flow_like_catalog_core::FlowPath {
            path: format!(
                "{}/{}",
                root.path,
                super::super::published::artifact_dir(artifact_id)
            ),
            ..root
        };
        let model = super::super::publisher::LedgerModel::Artifact {
            artifact_id: artifact_id.into(),
        };
        super::super::publisher::export_model_folder(
            context,
            repo,
            LedgerScope::App,
            &model,
            &destination,
        )
        .await
        .unwrap();
    }

    /// A ResNet-18 image classifier of stream `stream_id`, small enough for unit tests and
    /// able to extract spatial features.
    pub(crate) fn tiny_resnet(
        repo: &TrainingRepository,
        project: &str,
        stream_id: &str,
    ) -> (InspectionContext, ModelArtifact) {
        let labels = vec!["good".to_owned(), "bad".to_owned()];
        let inspection = InspectionContext {
            stream_id: stream_id.into(),
            spec: InspectionSpec {
                id: "tiny".into(),
                description: "Tiny image classifier".into(),
                task: TaskKind::ImageClassification,
                labels: labels.clone(),
                input_shape: vec![1, 8, 8],
                prediction_horizon_ms: None,
                minimum_examples: 1,
                minimum_examples_per_class: 0,
            },
        };
        for index in 0..12 {
            let time = 1_000 + index;
            let sample = Sample {
                id: format!("{stream_id}-{index}"),
                group_id: format!("{stream_id}-group-{index}"),
                stream_id: stream_id.into(),
                timestamp_ms: time,
                window_start_ms: time,
                window_end_ms: time,
                input: TensorData {
                    shape: vec![1, 8, 8],
                    values: vec![index as f32 / 12.0; 64],
                },
                annotation: Annotation::Class {
                    class_id: (index % 2) as u32,
                },
                provenance: LabelProvenance::Reviewed {
                    reviewer: "fixture".into(),
                    reviewed_at_ms: 2_000,
                },
                outcome: None,
            };
            let request = RecordSampleRequest {
                inspection: inspection.clone(),
                sample,
                annotation_revision: 1,
                accepted: true,
                label_available_at_ms: 2_000,
            };
            record_sample(repo.clone(), project.into(), request).unwrap();
        }
        let config = flow_like_ml_burn::TrainingConfig {
            recipe: flow_like_ml_burn::Recipe::ResNet18 {
                input_channels: 1,
                classes: 2,
                base_channels: 2,
            },
            backend: flow_like_ml_burn::BackendChoice::Cpu,
            epochs: 1,
            batch_size: 2,
            learning_rate: 0.001,
            seed: 17,
            gradient_clip: 0.1,
        };
        let request = ScheduleTrainingRequest {
            inspection: inspection.clone(),
            after_new_samples: 12,
            training: TrainingRequest {
                engine: "burn".into(),
                recipe: json!({
                    "config": config, "labels": labels, "preprocessing": [], "pretrained": null
                }),
                compute: json!({"backend": "cpu"}),
            },
            split: SplitPolicy::Group {
                train_fraction: 0.5,
                validation_fraction: 0.25,
                seed: 7,
            },
            worker_limits: WorkerLimits::default(),
            feature_extractor: None,
        };
        let ignore = |_: &TrainingRepository, _: &str, _: &WorkerLimits| -> Result<()> { Ok(()) };
        let job = schedule_with(repo.clone(), project.into(), request, ignore)
            .unwrap()
            .unwrap();
        run_training_job(repo, &job.id, &WorkerLimits::default()).unwrap();
        let artifact_id = repo.get_job(&job.id).unwrap().artifact_id.unwrap();
        (inspection, repo.get_artifact(&artifact_id).unwrap())
    }
}

#[cfg(all(test, feature = "training"))]
mod lifecycle_tests {
    use super::test_device::*;
    use super::*;
    use flow_like_ml_core::{Annotation, LabelProvenance, Outcome, TaskKind};
    use flow_like_types::json::{from_slice, from_value};
    use std::cell::RefCell;
    use std::time::{Duration, Instant};

    const PROJECT: &str = "app:board";

    fn ledger() -> (tempfile::TempDir, TrainingRepository) {
        let directory = tempfile::tempdir().unwrap();
        let repo = TrainingRepository::open(directory.path().join("jobs.sqlite")).unwrap();
        (directory, repo)
    }

    fn inspection_context(
        stream_id: &str,
        task: TaskKind,
        labels: &[&str],
        input_shape: &[usize],
    ) -> InspectionContext {
        InspectionContext {
            stream_id: stream_id.into(),
            spec: InspectionSpec {
                id: "surface".into(),
                description: "Surface inspection".into(),
                task,
                labels: labels.iter().map(|label| (*label).to_owned()).collect(),
                input_shape: input_shape.to_vec(),
                prediction_horizon_ms: None,
                minimum_examples: 1,
                minimum_examples_per_class: 0,
            },
        }
    }

    fn reviewed() -> LabelProvenance {
        LabelProvenance::Reviewed {
            reviewer: "inspector".into(),
            reviewed_at_ms: 2_000,
        }
    }

    fn sample(
        inspection: &InspectionContext,
        index: usize,
        annotation: Annotation,
        provenance: LabelProvenance,
    ) -> Sample {
        let time = 1_000 + index as i64;
        let elements = inspection.spec.input_shape.iter().product::<usize>();
        Sample {
            id: format!("sample-{index}"),
            group_id: format!("group-{index}"),
            stream_id: inspection.stream_id.clone(),
            timestamp_ms: time,
            window_start_ms: time,
            window_end_ms: time,
            input: TensorData {
                shape: inspection.spec.input_shape.clone(),
                values: (0..elements)
                    .map(|element| (index * elements + element) as f32)
                    .collect(),
            },
            annotation,
            provenance,
            outcome: None,
        }
    }

    fn record(
        repo: &TrainingRepository,
        inspection: &InspectionContext,
        sample: Sample,
        label_available_at_ms: i64,
    ) -> Result<SampleReceipt> {
        record_sample(
            repo.clone(),
            PROJECT.into(),
            RecordSampleRequest {
                inspection: inspection.clone(),
                sample,
                annotation_revision: 1,
                accepted: true,
                label_available_at_ms,
            },
        )
    }

    fn record_normal(
        repo: &TrainingRepository,
        inspection: &InspectionContext,
        indices: std::ops::Range<usize>,
    ) {
        for index in indices {
            let annotation = Annotation::Anomaly { is_anomaly: false };
            let normal = sample(inspection, index, annotation, reviewed());
            record(repo, inspection, normal, 2_000).unwrap();
        }
    }

    fn split() -> SplitPolicy {
        SplitPolicy::Group {
            train_fraction: 0.5,
            validation_fraction: 0.25,
            seed: 7,
        }
    }

    fn fixture_request() -> TrainingRequest {
        TrainingRequest {
            engine: "fixture".into(),
            recipe: json!({}),
            compute: json!({}),
        }
    }

    /// The ledger allows one active job per stream, so every job gets a stream of its own.
    fn queued_job(repo: &TrainingRepository, stream_id: &str) -> (InspectionContext, TrainingJob) {
        let inspection = inspection_context(stream_id, TaskKind::SensorAnomaly, &[], &[2]);
        record_normal(repo, &inspection, 0..3);
        let key = inspection.key(PROJECT.into()).unwrap();
        let job = repo
            .trigger_after_n(&key, 3, fixture_request(), split(), runtime::now_ms())
            .unwrap()
            .unwrap();
        (inspection, job)
    }

    fn interrupt(repo: &TrainingRepository, job: &TrainingJob) {
        let lease = repo.claim_job(&job.id, "crashed", 1_000, 1_000).unwrap();
        repo.mark_interrupted(&job.id, lease.generation, 3_000)
            .unwrap();
    }

    fn pause(repo: &TrainingRepository, job: &TrainingJob) {
        let now = runtime::now_ms();
        let lease = repo.claim_job(&job.id, "worker", 60_000, now).unwrap();
        let metadata = json!({"engine": "fixture"});
        repo.publish_checkpoint(&lease, b"checkpoint", 1, metadata, true, now)
            .unwrap();
    }

    /// The worker's lease expires while cancellation is pending, as after a crash or sleep.
    fn request_cancel_then_crash(repo: &TrainingRepository, job: &TrainingJob) {
        repo.claim_job(&job.id, "crashed", 1_000, 1_000).unwrap();
        let requested = repo.request_cancel(&job.id, 1_500).unwrap();
        assert_eq!(requested.status, JobStatus::CancelRequested);
    }

    fn remember_failure(repo: &TrainingRepository, job: &TrainingJob, limits: &WorkerLimits) {
        let failure =
            runtime::Error::Engine("training needs an estimated 9 bytes; budget is 4".into());
        repo.record_job_attempt_error(&job.id, &failure.to_string(), runtime::now_ms())
            .unwrap();
        note_attempt(&queue_key(repo, &job.id).unwrap(), limits, &Err(failure));
    }

    fn larger(limits: &WorkerLimits) -> WorkerLimits {
        WorkerLimits {
            memory_budget_bytes: limits.memory_budget_bytes * 2,
            ..limits.clone()
        }
    }

    #[test]
    fn ledger_registry_reopens_and_verifies_a_removed_ledger() {
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("inspection").join("jobs.sqlite");
        let first = open_ledger(path.clone()).unwrap();
        let identity = ledger_identity(&path).unwrap();
        assert_eq!(open_ledger(path.clone()).unwrap().path(), first.path());
        assert_eq!(ledger_identity(&path), Some(identity));
        for suffix in ["", "-wal", "-shm"] {
            let mut file = path.clone().into_os_string();
            file.push(suffix);
            let _ = std::fs::remove_file(file);
        }
        assert_eq!(ledger_identity(&path), None);
        let reopened = open_ledger(path.clone()).unwrap();
        let key = inspection_context("ledger", TaskKind::SensorAnomaly, &[], &[2])
            .key(PROJECT.into())
            .unwrap();
        assert!(reopened.list_jobs(&key).unwrap().is_empty());
    }

    #[cfg(unix)]
    #[test]
    fn reused_ledger_handles_refuse_a_blob_directory_swapped_for_a_symlink() {
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("inspection").join("jobs.sqlite");
        open_ledger(path.clone()).unwrap();
        let blobs = path.with_extension("artifacts");
        std::fs::remove_dir_all(&blobs).unwrap();
        let elsewhere = tempfile::tempdir().unwrap();
        std::os::unix::fs::symlink(elsewhere.path(), &blobs).unwrap();
        let error = open_ledger(path).unwrap_err().to_string();
        assert!(error.contains("must be a real directory"), "{error}");
    }

    #[test]
    fn ledger_identity_carries_the_birth_time_the_filesystem_reports() {
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("jobs.sqlite");
        std::fs::write(&path, b"ledger").unwrap();
        let born = std::fs::metadata(&path).unwrap().created().ok();
        let identity = ledger_identity(&path).map(|identity| identity.born);
        assert_eq!(identity, Some(born));
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 1)]
    async fn blocking_ledger_work_hands_other_tasks_to_another_worker() {
        let (started, blocked) = tokio::sync::oneshot::channel();
        let (release, released) = std::sync::mpsc::channel::<()>();
        let waiting = tokio::spawn(async move {
            blocking(move || {
                let _ = started.send(());
                released.recv_timeout(std::time::Duration::from_secs(10))
            })
        });
        blocked.await.unwrap();
        // The runtime's only worker waits above, so this task runs only if it was handed on.
        tokio::spawn(async move { release.send(()) })
            .await
            .unwrap()
            .unwrap();
        waiting.await.unwrap().unwrap();
    }

    #[test]
    fn record_sample_maps_label_provenance_and_rejects_unavailable_labels() {
        let (_directory, repo) = ledger();
        let inspection = inspection_context(
            "provenance",
            TaskKind::SensorClassification,
            &["good", "bad"],
            &[2],
        );
        let class = |class_id: u32| Annotation::Class { class_id };
        let teacher = LabelProvenance::Teacher {
            model: "vision-teacher".into(),
            prompt_digest: "prompt".into(),
            confidence: Some(0.9),
        };
        let measured = LabelProvenance::Measured {
            source: "gauge".into(),
        };
        let labelled = sample(&inspection, 0, class(0), teacher.clone());
        record(&repo, &inspection, labelled, 1_500).unwrap();
        let labelled = sample(&inspection, 1, class(1), reviewed());
        record(&repo, &inspection, labelled, 2_000).unwrap();
        let mut observed = sample(&inspection, 2, class(0), measured);
        observed.outcome = Some(Outcome {
            available_at_ms: 1_900,
            target_start_ms: 1_002,
            target_end_ms: 1_800,
        });
        record(&repo, &inspection, observed.clone(), 1_900).unwrap();
        let key = inspection.key(PROJECT.into()).unwrap();
        let snapshot = repo.snapshot(&key, split(), runtime::now_ms()).unwrap();
        let mut stored: Vec<_> = snapshot
            .train
            .iter()
            .chain(&snapshot.validation)
            .chain(&snapshot.test)
            .map(|sample| {
                (
                    sample.id.clone(),
                    sample.source,
                    sample.label_available_at_ms,
                )
            })
            .collect();
        stored.sort_by(|left, right| left.0.cmp(&right.0));
        assert_eq!(
            stored,
            [
                ("sample-0".to_owned(), LabelSource::Teacher, 1_500),
                ("sample-1".to_owned(), LabelSource::Reviewed, 2_000),
                ("sample-2".to_owned(), LabelSource::ObservedOutcome, 1_900),
            ]
        );

        let unlabeled = sample(&inspection, 3, class(0), LabelProvenance::Unlabeled);
        assert!(record(&repo, &inspection, unlabeled, 2_000).is_err());
        let future = sample(&inspection, 4, class(0), teacher);
        assert!(record(&repo, &inspection, future, runtime::now_ms() + 60_000).is_err());
        let before_review = sample(&inspection, 5, class(0), reviewed());
        assert!(record(&repo, &inspection, before_review, 1_999).is_err());
        let mut before_outcome = observed;
        before_outcome.id = "sample-6".into();
        before_outcome.group_id = "group-6".into();
        assert!(record(&repo, &inspection, before_outcome, 1_899).is_err());
    }

    #[test]
    fn recover_settles_stopped_workers_and_restarts_waiting_and_interrupted_jobs() {
        let (_directory, repo) = ledger();
        let (waiting, queued) = queued_job(&repo, "queued");
        let (interrupted, stopped) = queued_job(&repo, "interrupted");
        interrupt(&repo, &stopped);
        let (paused, pausing) = queued_job(&repo, "paused");
        pause(&repo, &pausing);
        let (cancelling, cancelled) = queued_job(&repo, "cancel-requested");
        request_cancel_then_crash(&repo, &cancelled);
        let (running, crashed) = queued_job(&repo, "running");
        repo.claim_job(&crashed.id, "crashed", 1_000, 1_000)
            .unwrap();
        let (exhausted, failing) = queued_job(&repo, "out-of-memory");
        interrupt(&repo, &failing);
        let limits = WorkerLimits::default();
        remember_failure(&repo, &failing, &limits);
        let dispatched = RefCell::new(Vec::new());
        let dispatch = |_: &TrainingRepository, job_id: &str, _: &WorkerLimits| -> Result<()> {
            dispatched.borrow_mut().push(job_id.to_owned());
            Ok(())
        };
        let recover = |inspection: &InspectionContext, job: &TrainingJob, limits: &WorkerLimits| {
            let request = RecoverRequest {
                inspection: inspection.clone(),
                worker_limits: limits.clone(),
            };
            recover_with(repo.clone(), PROJECT.into(), request, &dispatch)
                .unwrap()
                .into_iter()
                .find(|recovered| recovered.id == job.id)
                .unwrap()
                .status
        };
        let statuses = [
            recover(&waiting, &queued, &limits),
            recover(&interrupted, &stopped, &limits),
            recover(&paused, &pausing, &limits),
            recover(&cancelling, &cancelled, &limits),
            recover(&running, &crashed, &limits),
            recover(&exhausted, &failing, &limits),
        ];
        assert_eq!(
            statuses,
            [
                JobStatus::Queued,
                JobStatus::Interrupted,
                JobStatus::Paused,
                JobStatus::Cancelled,
                JobStatus::Interrupted,
                JobStatus::Interrupted,
            ]
        );
        assert_eq!(
            *dispatched.borrow(),
            [queued.id.clone(), stopped.id.clone(), crashed.id.clone()]
        );
        recover(&exhausted, &failing, &larger(&limits));
        assert_eq!(dispatched.borrow().last(), Some(&failing.id));
    }

    #[test]
    fn cancel_settles_jobs_whose_worker_stopped_so_the_stream_trains_again() {
        let (_directory, repo) = ledger();
        let (running, crashed) = queued_job(&repo, "running");
        repo.claim_job(&crashed.id, "crashed", 1_000, 1_000)
            .unwrap();
        let (cancelling, cancelled) = queued_job(&repo, "cancel-requested");
        request_cancel_then_crash(&repo, &cancelled);
        for (inspection, job) in [(&running, &crashed), (&cancelling, &cancelled)] {
            let request = JobRequest {
                inspection: inspection.clone(),
                job_id: job.id.clone(),
            };
            let settled = cancel(repo.clone(), PROJECT.into(), request).unwrap();
            assert_eq!(settled.status, JobStatus::Cancelled);
        }
        record_normal(&repo, &cancelling, 3..6);
        let key = cancelling.key(PROJECT.into()).unwrap();
        let retrained = repo
            .trigger_after_n(&key, 3, fixture_request(), split(), runtime::now_ms())
            .unwrap();
        assert!(retrained.is_some_and(|job| job.status == JobStatus::Queued));
    }

    #[test]
    fn resume_requeues_stopped_jobs_and_redispatches_waiting_ones_with_new_limits() {
        let (_directory, repo) = ledger();
        let (waiting, queued) = queued_job(&repo, "queued");
        let (interrupted, stopped) = queued_job(&repo, "interrupted");
        interrupt(&repo, &stopped);
        let (paused, pausing) = queued_job(&repo, "paused");
        pause(&repo, &pausing);
        let (cancelling, cancelled) = queued_job(&repo, "cancel-requested");
        request_cancel_then_crash(&repo, &cancelled);
        let (live, running) = queued_job(&repo, "running");
        let now = runtime::now_ms();
        repo.claim_job(&running.id, "worker", 60_000, now).unwrap();
        let limits = larger(&WorkerLimits::default());
        let dispatched = RefCell::new(Vec::new());
        let dispatch =
            |_: &TrainingRepository, job_id: &str, limits: &WorkerLimits| -> Result<()> {
                dispatched
                    .borrow_mut()
                    .push((job_id.to_owned(), limits.memory_budget_bytes));
                Ok(())
            };
        let resume = |inspection: &InspectionContext, job: &TrainingJob| {
            let request = ResumeRequest {
                inspection: inspection.clone(),
                job_id: job.id.clone(),
                worker_limits: limits.clone(),
                compute: None,
            };
            resume_with(repo.clone(), PROJECT.into(), request, &dispatch)
        };
        for (inspection, job) in [
            (&waiting, &queued),
            (&interrupted, &stopped),
            (&paused, &pausing),
            (&cancelling, &cancelled),
        ] {
            assert_eq!(resume(inspection, job).unwrap().status, JobStatus::Queued);
        }
        assert!(resume(&live, &running).is_err());
        let expected: Vec<_> = [&queued, &stopped, &pausing, &cancelled]
            .into_iter()
            .map(|job| (job.id.clone(), limits.memory_budget_bytes))
            .collect();
        assert_eq!(*dispatched.borrow(), expected);
    }

    fn small_cpu_compute() -> ComputeConfig {
        ComputeConfig {
            backend: flow_like_ml_core::ComputeBackend::Cpu,
            memory_limit_bytes: 256 * 1024 * 1024,
            ..Default::default()
        }
    }

    /// Resumes a job with new compute and records the jobs it dispatches.
    fn resume_on(
        repo: &TrainingRepository,
        inspection: &InspectionContext,
        job: &TrainingJob,
        compute: &ComputeConfig,
        dispatched: &RefCell<Vec<String>>,
    ) -> Result<TrainingJob> {
        let request = ResumeRequest {
            inspection: inspection.clone(),
            job_id: job.id.clone(),
            worker_limits: WorkerLimits::default(),
            compute: Some(compute.clone()),
        };
        let dispatch = |_: &TrainingRepository, job_id: &str, _: &WorkerLimits| -> Result<()> {
            dispatched.borrow_mut().push(job_id.to_owned());
            Ok(())
        };
        resume_with(repo.clone(), PROJECT.into(), request, dispatch)
    }

    #[test]
    fn resume_writes_new_compute_to_waiting_and_stopped_jobs_before_dispatch() {
        let (_directory, repo) = ledger();
        let (waiting, queued) = queued_job(&repo, "queued");
        let (interrupted, stopped) = queued_job(&repo, "interrupted");
        interrupt(&repo, &stopped);
        let (paused, pausing) = queued_job(&repo, "paused");
        pause(&repo, &pausing);
        let (running, crashed) = queued_job(&repo, "running");
        repo.claim_job(&crashed.id, "crashed", 1_000, 1_000)
            .unwrap();
        let compute = small_cpu_compute();
        let dispatched = RefCell::new(Vec::new());
        for (inspection, job) in [
            (&waiting, &queued),
            (&interrupted, &stopped),
            (&paused, &pausing),
            (&running, &crashed),
        ] {
            let resumed = resume_on(&repo, inspection, job, &compute, &dispatched).unwrap();
            assert_eq!(resumed.status, JobStatus::Queued);
            assert_eq!(resumed.request.compute, json!(compute));
            let stored = repo.get_job(&job.id).unwrap();
            assert_eq!(stored.request.compute, json!(compute));
        }
        assert!(repo.get_job(&pausing.id).unwrap().checkpoint.is_some());
        let expected = [&queued, &stopped, &pausing, &crashed].map(|job| job.id.clone());
        assert_eq!(*dispatched.borrow(), expected);
    }

    #[test]
    fn resume_gives_cancelled_and_failed_jobs_new_compute_and_refuses_invalid_compute() {
        let (_directory, repo) = ledger();
        let (waiting, queued) = queued_job(&repo, "queued");
        let (cancelling, cancelled) = queued_job(&repo, "cancelled");
        repo.request_cancel(&cancelled.id, runtime::now_ms())
            .unwrap();
        let (failing, failed) = queued_job(&repo, "failed");
        let lease = repo.claim_job(&failed.id, "worker", 60_000, runtime::now_ms());
        repo.fail_job(&lease.unwrap(), "engine stopped", runtime::now_ms())
            .unwrap();
        let (live, running) = queued_job(&repo, "running");
        repo.claim_job(&running.id, "worker", 60_000, runtime::now_ms())
            .unwrap();
        let dispatched = RefCell::new(Vec::new());
        let without_memory = ComputeConfig {
            memory_limit_bytes: 0,
            ..small_cpu_compute()
        };
        assert!(resume_on(&repo, &waiting, &queued, &without_memory, &dispatched).is_err());
        let compute = small_cpu_compute();
        assert!(resume_on(&repo, &live, &running, &compute, &dispatched).is_err());
        for job in [&queued, &running] {
            assert_eq!(repo.get_job(&job.id).unwrap().request.compute, json!({}));
        }
        for (inspection, job) in [(&cancelling, &cancelled), (&failing, &failed)] {
            let resumed = resume_on(&repo, inspection, job, &compute, &dispatched).unwrap();
            assert_eq!(resumed.status, JobStatus::Queued);
            let stored = repo.get_job(&job.id).unwrap();
            assert_eq!(stored.request.compute, json!(compute));
        }
        assert_eq!(
            *dispatched.borrow(),
            [cancelled.id.clone(), failed.id.clone()]
        );
    }

    #[test]
    fn train_after_n_counts_new_samples_and_holds_a_job_that_failed_under_the_same_limits() {
        let (_directory, repo) = ledger();
        let inspection = inspection_context("schedule", TaskKind::SensorAnomaly, &[], &[2]);
        record_normal(&repo, &inspection, 0..3);
        let request =
            |after_new_samples: usize, worker_limits: WorkerLimits| ScheduleTrainingRequest {
                inspection: inspection.clone(),
                after_new_samples,
                training: TrainingRequest {
                    engine: "isolation_forest".into(),
                    recipe: json!({"labels": []}),
                    compute: json!({"backend": "cpu"}),
                },
                split: split(),
                worker_limits,
                feature_extractor: None,
            };
        let dispatched = RefCell::new(Vec::new());
        let dispatch = |_: &TrainingRepository, job_id: &str, _: &WorkerLimits| -> Result<()> {
            dispatched.borrow_mut().push(job_id.to_owned());
            Ok(())
        };
        let schedule = |request: ScheduleTrainingRequest| {
            schedule_with(repo.clone(), PROJECT.into(), request, &dispatch).unwrap()
        };
        let limits = WorkerLimits::default();
        assert!(schedule(request(4, limits.clone())).is_none());
        let job = schedule(request(3, limits.clone())).unwrap();
        assert_eq!(job.status, JobStatus::Queued);
        remember_failure(&repo, &job, &limits);
        let held = schedule(request(3, limits.clone())).unwrap();
        assert_eq!(held.id, job.id);
        assert!(
            held.error
                .as_deref()
                .is_some_and(|error| error.contains("budget is 4"))
        );
        assert_eq!(*dispatched.borrow(), [job.id.clone()]);
        let retried = schedule(request(3, larger(&limits))).unwrap();
        assert_eq!(retried.id, job.id);
        assert_eq!(*dispatched.borrow(), [job.id.clone(), job.id.clone()]);
    }

    fn forest_request(inspection: &InspectionContext, compute: Value) -> ScheduleTrainingRequest {
        ScheduleTrainingRequest {
            inspection: inspection.clone(),
            after_new_samples: 1,
            training: TrainingRequest {
                engine: "isolation_forest".into(),
                recipe: json!({"labels": []}),
                compute,
            },
            split: split(),
            worker_limits: WorkerLimits::default(),
            feature_extractor: None,
        }
    }

    #[test]
    fn train_after_n_gives_its_held_job_raised_compute_and_dispatches_it_again() {
        let (_directory, repo) = ledger();
        let inspection = inspection_context("compute", TaskKind::SensorAnomaly, &[], &[2]);
        record_normal(&repo, &inspection, 0..3);
        let dispatched = RefCell::new(Vec::new());
        let dispatch = |_: &TrainingRepository, job_id: &str, _: &WorkerLimits| -> Result<()> {
            dispatched.borrow_mut().push(job_id.to_owned());
            Ok(())
        };
        let schedule = |compute: &Value| {
            let request = forest_request(&inspection, compute.clone());
            schedule_with(repo.clone(), PROJECT.into(), request, &dispatch)
                .unwrap()
                .unwrap()
        };
        let small = json!({"backend": "cpu", "memory_limit_bytes": 1024});
        let job = schedule(&small);
        remember_failure(&repo, &job, &WorkerLimits::default());
        assert!(schedule(&small).error.is_some());
        assert_eq!(*dispatched.borrow(), [job.id.clone()]);
        let raised = json!({"backend": "cpu", "memory_limit_bytes": 1_073_741_824});
        let reconfigured = schedule(&raised);
        assert_eq!(reconfigured.id, job.id);
        assert_eq!(reconfigured.error, None);
        assert_eq!(repo.get_job(&job.id).unwrap().request.compute, raised);
        assert_eq!(*dispatched.borrow(), [job.id.clone(), job.id.clone()]);
    }

    #[test]
    fn train_after_n_skips_first_triggers_that_cannot_fill_a_training_partition() {
        assert!(first_trigger_short(&[], 3, 2));
        assert!(!first_trigger_short(&[], 4, 2));
        let (_directory, repo) = ledger();
        let mut inspection = inspection_context("warm-up", TaskKind::SensorAnomaly, &[], &[2]);
        inspection.spec.minimum_examples = 2;
        record_normal(&repo, &inspection, 0..3);
        // An invalid split fails only inside a trigger, so the error shows that one ran.
        let ignore = |_: &TrainingRepository, _: &str, _: &WorkerLimits| -> Result<()> { Ok(()) };
        let schedule = || {
            let mut request = forest_request(&inspection, json!({"backend": "cpu"}));
            request.split = SplitPolicy::Group {
                train_fraction: 1.0,
                validation_fraction: 0.5,
                seed: 7,
            };
            schedule_with(repo.clone(), PROJECT.into(), request, &ignore)
        };
        assert!(schedule().unwrap().is_none());
        record_normal(&repo, &inspection, 3..4);
        let error = schedule().unwrap_err().to_string();
        assert!(error.contains("group split"), "{error}");
    }

    #[test]
    fn worker_attempts_keep_admission_failures_and_hold_the_job_under_the_same_limits() {
        let (_directory, repo) = ledger();
        let (inspection, job) = queued_job(&repo, "admission");
        let limits = WorkerLimits::default();
        attempt(&repo, &job.id, &limits, &queue_key(&repo, &job.id).unwrap());
        let waiting = repo.get_job(&job.id).unwrap();
        assert_eq!(waiting.status, JobStatus::Queued);
        let error = waiting.error.unwrap_or_default();
        assert!(
            error.contains("engine fixture is not registered"),
            "{error}"
        );
        let dispatched = RefCell::new(Vec::new());
        let dispatch =
            |_: &TrainingRepository, job_id: &str, limits: &WorkerLimits| -> Result<()> {
                dispatched
                    .borrow_mut()
                    .push((job_id.to_owned(), limits.memory_budget_bytes));
                Ok(())
            };
        let drive = |worker_limits: &WorkerLimits| {
            let recover = RecoverRequest {
                inspection: inspection.clone(),
                worker_limits: worker_limits.clone(),
            };
            recover_with(repo.clone(), PROJECT.into(), recover, &dispatch).unwrap();
            let mut request = forest_request(&inspection, json!({"backend": "cpu"}));
            request.worker_limits = worker_limits.clone();
            schedule_with(repo.clone(), PROJECT.into(), request, &dispatch).unwrap();
        };
        drive(&limits);
        assert!(dispatched.borrow().is_empty());
        let raised = larger(&limits);
        drive(&raised);
        let expected = vec![(job.id.clone(), raised.memory_budget_bytes); 2];
        assert_eq!(*dispatched.borrow(), expected);
    }

    #[test]
    fn patch_models_record_their_feature_extractor_and_refuse_other_embeddings() {
        let (_directory, repo) = ledger();
        let inspection = inspection_context("patches", TaskKind::VisualAnomaly, &[], &[4, 3]);
        record_normal(&repo, &inspection, 0..3);
        let extractor = FeatureExtractorRef {
            source_id: "backbone".into(),
            blob_sha256: "a".repeat(64),
            image_steps: Vec::new(),
        };
        let request = |engine: &str, feature_extractor: Option<FeatureExtractorRef>| {
            ScheduleTrainingRequest {
                inspection: inspection.clone(),
                after_new_samples: 3,
                training: TrainingRequest {
                    engine: engine.into(),
                    recipe: json!({"labels": [], "coreset_size": 8}),
                    compute: json!({"backend": "cpu"}),
                },
                split: split(),
                worker_limits: WorkerLimits::default(),
                feature_extractor,
            }
        };
        let ignore = |_: &TrainingRepository, _: &str, _: &WorkerLimits| -> Result<()> { Ok(()) };
        let schedule = |request: ScheduleTrainingRequest| {
            schedule_with(repo.clone(), PROJECT.into(), request, &ignore)
        };
        assert!(schedule(request("isolation_forest", Some(extractor.clone()))).is_err());
        let unverifiable = FeatureExtractorRef {
            blob_sha256: "not-a-digest".into(),
            ..extractor.clone()
        };
        assert!(schedule(request("patchcore", Some(unverifiable))).is_err());
        let job = schedule(request("patchcore", Some(extractor.clone())))
            .unwrap()
            .unwrap();
        assert_eq!(job.request.recipe["feature_extractor"], json!(extractor));
        run_training_job(&repo, &job.id, &WorkerLimits::default()).unwrap();
        let artifact_id = repo.get_job(&job.id).unwrap().artifact_id.unwrap();
        let predictor =
            CatalogLoadedPredictor::load(&repo, &artifact_id, &ComputeConfig::default()).unwrap();
        let embeddings = TensorData {
            shape: vec![4, 3],
            values: vec![0.5; 12],
        };
        let predict = |extractor: Option<&FeatureExtractorRef>| {
            let extractor = extractor.map(|extractor| extractor.to_runtime().unwrap());
            predictor.predict_checked(&embeddings, extractor.as_ref())
        };
        predict(Some(&extractor)).unwrap();
        let unnamed = predict(None).unwrap_err().to_string();
        assert!(unnamed.contains("feature extractor backbone"), "{unnamed}");
        let other = FeatureExtractorRef {
            source_id: "other-backbone".into(),
            ..extractor
        };
        let error = predict(Some(&other)).unwrap_err().to_string();
        assert!(error.contains("other-backbone"), "{error}");
    }

    /// PatchCore over three new embeddings, from the named extractor when there is one.
    fn patch_request(
        inspection: &InspectionContext,
        source_id: Option<&str>,
    ) -> ScheduleTrainingRequest {
        ScheduleTrainingRequest {
            inspection: inspection.clone(),
            after_new_samples: 3,
            training: TrainingRequest {
                engine: "patchcore".into(),
                recipe: json!({"labels": [], "coreset_size": 8}),
                compute: json!({"backend": "cpu"}),
            },
            split: split(),
            worker_limits: WorkerLimits::default(),
            feature_extractor: source_id.map(|source_id| FeatureExtractorRef {
                source_id: source_id.into(),
                blob_sha256: "a".repeat(64),
                image_steps: Vec::new(),
            }),
        }
    }

    #[test]
    fn patch_models_fitted_without_an_extractor_still_score_unnamed_embeddings() {
        let (_directory, repo) = ledger();
        let inspection = inspection_context("plain", TaskKind::VisualAnomaly, &[], &[4, 3]);
        record_normal(&repo, &inspection, 0..3);
        let ignore = |_: &TrainingRepository, _: &str, _: &WorkerLimits| -> Result<()> { Ok(()) };
        let request = patch_request(&inspection, None);
        let job = schedule_with(repo.clone(), PROJECT.into(), request, ignore);
        let job = job.unwrap().unwrap();
        run_training_job(&repo, &job.id, &WorkerLimits::default()).unwrap();
        let artifact_id = repo.get_job(&job.id).unwrap().artifact_id.unwrap();
        let predictor =
            CatalogLoadedPredictor::load(&repo, &artifact_id, &ComputeConfig::default()).unwrap();
        let embeddings = TensorData {
            shape: vec![4, 3],
            values: vec![0.5; 12],
        };
        predictor.predict_checked(&embeddings, None).unwrap();
    }

    #[test]
    fn train_after_n_keeps_one_feature_extractor_per_stream() {
        let (_directory, repo) = ledger();
        let inspection = inspection_context("swap", TaskKind::VisualAnomaly, &[], &[4, 3]);
        record_normal(&repo, &inspection, 0..3);
        let ignore = |_: &TrainingRepository, _: &str, _: &WorkerLimits| -> Result<()> { Ok(()) };
        let schedule = |source_id: &str| {
            let request = patch_request(&inspection, Some(source_id));
            schedule_with(repo.clone(), PROJECT.into(), request, &ignore)
        };
        let first = schedule("backbone").unwrap().unwrap();
        repo.request_cancel(&first.id, runtime::now_ms()).unwrap();
        record_normal(&repo, &inspection, 3..6);
        let error = schedule("promoted-backbone").unwrap_err().to_string();
        assert!(error.contains("promoted-backbone"), "{error}");
        assert!(error.contains("new stream ID"), "{error}");
        let retrained = schedule("backbone").unwrap().unwrap();
        assert_ne!(retrained.id, first.id);
    }

    #[test]
    fn burn_fine_tuning_refuses_normalization_that_differs_from_the_source() {
        let (_directory, repo) = ledger();
        let inspection = inspection_context(
            "fine-tune",
            TaskKind::SensorClassification,
            &["good", "bad"],
            &[2],
        );
        let config = json!({
            "recipe": {"architecture":"mlp","input_features":2,"hidden":4,"outputs":2,"objective":"classification"},
            "backend": {"backend":"cpu"},
            "epochs": 1,
            "batch_size": 1,
            "learning_rate": 0.01,
            "seed": 7
        });
        let source = repo
            .register_pretrained_source(
                PROJECT,
                b"pretrained weights",
                json!({
                    "engine": "burn",
                    "format_version": 1,
                    "config": config,
                    "input_shape": [2],
                    "labels": ["good", "bad"],
                    "recommended_preprocessing": [{"kind":"normalize","mean":[0.5],"std":[0.25]}]
                }),
                json!({"source": "fixture"}),
                1024,
                runtime::now_ms(),
            )
            .unwrap();
        let request = |mean: f32, deviation: f32| ScheduleTrainingRequest {
            inspection: inspection.clone(),
            after_new_samples: 1,
            training: TrainingRequest {
                engine: "burn".into(),
                recipe: json!({
                    "config": config,
                    "labels": ["good", "bad"],
                    "preprocessing": [{"kind":"normalize","mean":[mean],"std":[deviation]}],
                    "pretrained": {"source_id": source.id, "replace_head": false, "freeze_backbone": false}
                }),
                compute: json!({"backend": "cpu"}),
            },
            split: split(),
            worker_limits: WorkerLimits::default(),
            feature_extractor: None,
        };
        let ignore = |_: &TrainingRepository, _: &str, _: &WorkerLimits| -> Result<()> { Ok(()) };
        let error = schedule_with(repo.clone(), PROJECT.into(), request(0.0, 1.0), &ignore)
            .unwrap_err()
            .to_string();
        assert!(error.contains("expects images normalized"), "{error}");
        let matching = schedule_with(repo.clone(), PROJECT.into(), request(0.5, 0.25), &ignore);
        assert!(matching.unwrap().is_none());
    }

    #[test]
    fn comparisons_stay_in_the_artifact_lifetime_and_other_stream_deployments_stay_teacher_only() {
        let (_directory, repo) = ledger();
        let labels = ["good", "bad"];
        let inspection =
            inspection_context("camera", TaskKind::SensorClassification, &labels, &[2]);
        for index in 0..3 {
            let class = Annotation::Class {
                class_id: (index % 2) as u32,
            };
            let labelled = sample(&inspection, index, class, reviewed());
            record(&repo, &inspection, labelled, 2_000).unwrap();
        }
        let now = runtime::now_ms();
        let key = inspection.key(PROJECT.into()).unwrap();
        let job = repo
            .trigger_after_n(&key, 3, fixture_request(), split(), now)
            .unwrap()
            .unwrap();
        let lease = repo.claim_job(&job.id, "worker", 60_000, now).unwrap();
        let model = repo
            .finish_training(
                &lease,
                b"model",
                json!({"engine":"fixture","labels":labels}),
                now,
            )
            .unwrap();
        let audited = repo
            .get_snapshot(&model.snapshot_id)
            .unwrap()
            .test
            .remove(0);
        let label = audited.payload["label"].as_str().unwrap().to_owned();
        let comparison = |recorded_at_ms: i64| RecordPredictionRequest {
            inspection: inspection.clone(),
            prediction: PredictionRecord {
                sample_id: audited.id.clone(),
                artifact_id: model.id.clone(),
                predicted_label: Some(label.clone()),
                teacher_label: None,
                actual_label: Some(label.clone()),
                actual_source: Some(LabelSource::Reviewed),
                latency_ms: 1.0,
                failed: false,
                recorded_at_ms,
                details: json!({}),
            },
        };
        for recorded_at_ms in [
            now + 3_600_000,
            model.created_at_ms - 1,
            model.created_at_ms * 1_000,
        ] {
            let error = record_prediction(repo.clone(), PROJECT.into(), comparison(recorded_at_ms))
                .unwrap_err()
                .to_string();
            assert!(error.contains(&recorded_at_ms.to_string()), "{error}");
        }
        let recorded = comparison(model.created_at_ms);
        assert!(record_prediction(repo.clone(), PROJECT.into(), recorded).unwrap());
        let report = repo
            .evaluate_classification(&model.id, runtime::now_ms())
            .unwrap();
        let promote_request = PromoteRequest {
            inspection: inspection.clone(),
            deployment_id: "main".into(),
            expected_generation: 0,
            evaluation_id: report.id,
            policy: PromotionPolicy {
                minimum_audited_samples: 1,
                minimum_accuracy: 0.5,
                minimum_class_recall: Default::default(),
                maximum_mean_latency_ms: 10.0,
                maximum_failure_rate: 0.0,
            },
        };
        let promoted = promote(repo.clone(), PROJECT.into(), promote_request).unwrap();
        let request = |inspection: &InspectionContext| DeploymentRequest {
            inspection: inspection.clone(),
            deployment_id: "main".into(),
        };
        let active = deployment(repo.clone(), PROJECT.into(), request(&inspection)).unwrap();
        assert_eq!(
            active.map(|found| found.generation),
            Some(promoted.generation)
        );
        let (_, route) = routing(repo.clone(), PROJECT.into(), request(&inspection)).unwrap();
        assert!(matches!(route.mode, InspectionMode::StudentOnly));

        let mut edited = inspection.clone();
        edited.spec.description = "Surface inspection after a specification edit".into();
        assert!(
            deployment(repo.clone(), PROJECT.into(), request(&edited))
                .unwrap()
                .is_none()
        );
        let (_, route) = routing(repo.clone(), PROJECT.into(), request(&edited)).unwrap();
        assert!(matches!(route.mode, InspectionMode::TeacherOnly));
        assert_eq!(route.deployment_generation, None);
        let pause_request = PauseDeploymentRequest {
            inspection: edited.clone(),
            deployment_id: "main".into(),
            expected_generation: promoted.generation,
            paused: true,
        };
        let error = pause_deployment(repo.clone(), PROJECT.into(), pause_request)
            .unwrap_err()
            .to_string();
        assert!(error.contains("different inspection stream"), "{error}");
        let rollback_request = RollbackRequest {
            inspection: edited.clone(),
            deployment_id: "main".into(),
            expected_generation: promoted.generation,
        };
        assert!(rollback(repo.clone(), PROJECT.into(), rollback_request).is_err());
        let edited_key = edited.key(PROJECT.into()).unwrap();
        assert!(check_deployment_scope(&repo, &edited_key, "main").is_err());
        check_deployment_scope(&repo, &edited_key, "edited").unwrap();
    }

    const APP_LEDGER: &str = "training-ledgers/apps/app/project/jobs.sqlite";
    const LEGACY_LEDGER: &str = "apps/app/storage/inspection/jobs.sqlite";
    const USER_LEDGER: &str = "training-ledgers/apps/app/users/user-1/jobs.sqlite";

    #[tokio::test]
    async fn app_ledgers_use_the_new_path_then_a_legacy_ledger_in_place() {
        let device = Device::new();
        let context = plain_context(device.stores()).await;
        let (new, legacy) = (device.root().join(APP_LEDGER), device.root().join(LEGACY_LEDGER));
        let app = || LedgerLocation::new(&context, LedgerScope::App).unwrap();
        assert_eq!(app().database_path().unwrap(), new);
        assert!(!app().exists().unwrap());
        TrainingRepository::open(&legacy).unwrap();
        assert_eq!(app().database_path().unwrap(), legacy);
        assert_eq!(app().directory().unwrap(), legacy.parent().unwrap());
        assert_eq!(app().open().unwrap().0.path(), legacy);
        TrainingRepository::open(&new).unwrap();
        assert_eq!(app().database_path().unwrap(), new);
        assert_eq!(app().project(), PROJECT);
        let user = LedgerLocation::new(&context, LedgerScope::User).unwrap();
        let private = device.root().join(USER_LEDGER);
        assert_eq!(user.database_path().unwrap(), private);
        assert_eq!(
            (user.scope(), user.project()),
            (LedgerScope::User, "app:board:user")
        );
        let paths = ledger_paths(&context).unwrap();
        assert_eq!((paths.app, paths.user), (new, Some(private)));
    }

    #[tokio::test]
    async fn training_needs_a_live_run_with_a_local_app_store() {
        let context = plain_context(stores(Some(memory_store()), None)).await;
        let refusal = LedgerLocation::new(&context, LedgerScope::App).err().unwrap();
        assert!(is_training_unavailable(&refusal), "{refusal}");
        let refusal = ensure_training_available(&context).unwrap_err();
        assert!(is_training_unavailable(&refusal), "{refusal}");
        let found = locate_repository_for_inference(&context, |_, _| Ok(true)).unwrap();
        assert!(found.is_none());
        let device = Device::new();
        let mut shadow = plain_context(device.stores()).await;
        shadow.execution_cache.as_mut().unwrap().shadow = true;
        let refusal = ensure_training_available(&shadow).unwrap_err();
        assert!(!is_training_unavailable(&refusal));
        assert!(
            refusal
                .to_string()
                .starts_with("Training lifecycle operations require a live workflow"),
            "{refusal}"
        );
        let found = locate_repository_for_inference(&shadow, |_, _| Ok(true)).unwrap();
        assert!(found.is_none());
        assert!(!device.root().join("training-ledgers").exists());
    }

    #[tokio::test]
    async fn lookups_create_no_ledger() {
        let device = Device::new();
        let context = plain_context(device.stores()).await;
        assert!(
            locate_repository(&context, |_, _| Ok(true))
                .unwrap()
                .is_none()
        );
        let found = locate_repository_for_inference(&context, |_, _| Ok(true)).unwrap();
        assert!(found.is_none());
        for scope in [LedgerScope::App, LedgerScope::User] {
            let ledger = LedgerLocation::new(&context, scope).unwrap();
            ledger.directory().unwrap();
            assert!(!ledger.exists().unwrap());
            assert!(ledger.open_existing().unwrap().is_none());
        }
        assert!(ledger_paths(&context).unwrap().user.is_some());
        assert!(!device.root().join("training-ledgers").exists());
        assert!(!device.root().join("apps").exists());
    }

    fn register_source(repo: &TrainingRepository, project: &str) -> String {
        let manifest = json!({
            "engine": "burn", "format_version": 1, "config": {}, "input_shape": [1]
        });
        repo.register_pretrained_source(
            project,
            b"weights",
            manifest,
            json!({"source": "fixture"}),
            1024,
            runtime::now_ms(),
        )
        .unwrap()
        .id
    }

    #[tokio::test]
    async fn lookups_walk_the_app_legacy_and_private_ledgers_in_order() {
        let device = Device::new();
        let context = plain_context(device.stores()).await;
        let legacy = open_ledger(device.root().join(LEGACY_LEDGER)).unwrap();
        let in_legacy = register_source(&legacy, PROJECT);
        let app = open_ledger(device.root().join(APP_LEDGER)).unwrap();
        let in_app = register_source(&app, PROJECT);
        let other_board = register_source(&app, "app:other-board");
        let (user, user_project) = scoped_repository(&context, LedgerScope::User).unwrap();
        let in_user = register_source(&user, &user_project);
        let locate = |id: &str| {
            locate_repository(&context, |repo, project| {
                holds_pretrained_source(repo, project, id)
            })
            .unwrap()
            .map(|found| (found.repo.path().to_path_buf(), found.scope, found.project))
        };
        let app_hit = (app.path().to_path_buf(), LedgerScope::App, PROJECT.to_owned());
        assert_eq!(locate(&in_app), Some(app_hit));
        let legacy_hit = (legacy.path().to_path_buf(), LedgerScope::App, PROJECT.to_owned());
        assert_eq!(locate(&in_legacy), Some(legacy_hit));
        let user_hit = (user.path().to_path_buf(), LedgerScope::User, user_project);
        assert_eq!(locate(&in_user), Some(user_hit));
        assert_eq!(locate(&other_board), None);
        assert_eq!(locate("pretrained:missing"), None);
    }

    #[tokio::test]
    async fn minted_user_streams_record_into_the_private_ledger() {
        let device = Device::new();
        let (app_ledger, user_ledger) = (
            device.root().join(APP_LEDGER),
            device.root().join(USER_LEDGER),
        );
        let recording = |stream: &str| {
            let inspection = inspection_context(stream, TaskKind::SensorAnomaly, &[], &[2]);
            let normal = Annotation::Anomaly { is_anomaly: false };
            let request = json!({
                "inspection": inspection, "sample": sample(&inspection, 0, normal, reviewed()),
                "annotation_revision": 1, "accepted": true, "label_available_at_ms": 2_000
            });
            (inspection, request)
        };
        let counted = |repo: &TrainingRepository, inspection: &InspectionContext, project| {
            let key = inspection.key(project).unwrap();
            repo.count_new_training_samples(&key, runtime::now_ms())
                .unwrap()
        };
        let minted = LedgerScope::User.new_stream_id(StreamKind::Auto);
        let (private, request) = recording(&minted);
        run_node(RecordTrainingSampleNode, request, device.stores())
            .await
            .unwrap();
        assert!(user_ledger.exists() && !app_ledger.exists());
        let context = plain_context(device.stores()).await;
        let (repo, project) = scoped_repository(&context, LedgerScope::User).unwrap();
        assert_eq!(counted(&repo, &private, project), 1);
        let (manual, request) = recording("user:camera");
        run_node(RecordTrainingSampleNode, request, device.stores())
            .await
            .unwrap();
        let (repo, project) = repository(&context).unwrap();
        assert_eq!(repo.path(), app_ledger);
        assert_eq!(counted(&repo, &manual, project), 1);
    }

    /// Admits a queued job to the training worker's queue as dispatch would.
    fn admit(repo: &TrainingRepository, job: &TrainingJob) -> String {
        let key = queue_key(repo, &job.id).unwrap();
        let queued = QueuedJob {
            ledger: repo.path().canonicalize().unwrap(),
            repo: repo.clone(),
            job_id: job.id.clone(),
        };
        assert!(JOBS.admit(&key, queued).unwrap());
        key
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn draining_an_app_cancels_its_jobs_and_keeps_other_apps_jobs() {
        use flow_like_types::storage_paths::training_ledger_dir;
        let directory = tempfile::tempdir().unwrap();
        let root = directory.path().canonicalize().unwrap();
        let ledger_of = |app: &str| {
            let directory = root.join(training_ledger_dir(app, None));
            open_ledger(directory.join(LEDGER_DATABASE_FILE)).unwrap()
        };
        let (a, b) = (ledger_of("app-a"), ledger_of("app-b"));
        let (_, running) = queued_job(&a, "running");
        let (_, waiting) = queued_job(&a, "waiting");
        let (_, other) = queued_job(&b, "other");
        let (running_key, waiting_key, other_key) =
            (admit(&a, &running), admit(&a, &waiting), admit(&b, &other));
        assert!(JOBS.start(&running_key));
        let lease = a
            .claim_job(&running.id, "worker", 60_000, runtime::now_ms())
            .unwrap();
        let worker = {
            let (a, job_id, key) = (a.clone(), running.id.clone(), running_key.clone());
            std::thread::spawn(move || {
                let deadline = Instant::now() + Duration::from_secs(30);
                while a.get_job(&job_id).unwrap().status != JobStatus::CancelRequested {
                    assert!(Instant::now() < deadline, "the drain never cancelled {job_id}");
                    std::thread::sleep(Duration::from_millis(5));
                }
                a.acknowledge_cancel(&lease, runtime::now_ms()).unwrap();
                JOBS.finish(&key);
            })
        };
        let report = drain_app_training(&root, "app-a", Duration::from_secs(30)).await;
        worker.join().unwrap();
        assert_eq!(
            report,
            DrainReport {
                cancelled: 2,
                still_running: 0
            }
        );
        for job in [&running, &waiting] {
            assert_eq!(a.get_job(&job.id).unwrap().status, JobStatus::Cancelled);
        }
        assert!(!JOBS.start(&waiting_key));
        assert_eq!(b.get_job(&other.id).unwrap().status, JobStatus::Queued);
        assert!(JOBS.start(&other_key));
        JOBS.finish(&other_key);

        let prefixes = drain_prefixes(&root, "app-a");
        assert_eq!(
            prefixes,
            [
                root.join("training-ledgers/apps/app-a"),
                root.join("apps/app-a/storage/inspection/jobs.sqlite")
            ]
        );
        assert_eq!(drain_prefixes(&root, "../escape").len(), 1);
        let (_, later) = queued_job(&a, "later");
        let closed = JOBS.close(&prefixes);
        let key = queue_key(&a, &later.id).unwrap();
        let queued = || QueuedJob {
            ledger: a.path().canonicalize().unwrap(),
            repo: a.clone(),
            job_id: later.id.clone(),
        };
        assert!(JOBS.admit(&key, queued()).is_err());
        drop(closed);
        assert!(JOBS.admit(&key, queued()).unwrap());
        JOBS.finish(&key);
    }

    /// An Isolation Forest artifact of stream `stream_id` in `repo`.
    fn forest_artifact(repo: &TrainingRepository, stream_id: &str) -> (InspectionContext, String) {
        let inspection = inspection_context(stream_id, TaskKind::SensorAnomaly, &[], &[2]);
        record_normal(repo, &inspection, 0..12);
        let mut request = forest_request(&inspection, json!({"backend": "cpu"}));
        request.training.recipe =
            json!({"labels": [], "config": {"trees": 16, "sample_size": 8, "seed": 5}});
        let ignore = |_: &TrainingRepository, _: &str, _: &WorkerLimits| -> Result<()> { Ok(()) };
        let job = schedule_with(repo.clone(), PROJECT.into(), request, ignore)
            .unwrap()
            .unwrap();
        run_training_job(repo, &job.id, &WorkerLimits::default()).unwrap();
        (inspection, repo.get_job(&job.id).unwrap().artifact_id.unwrap())
    }

    /// The stores of a runtime without a ledger that reads the app's published models.
    async fn without_ledger(device: &Device) -> flow_like::state::FlowLikeStores {
        let published = published_copy(&device.app, APP_MODELS).await;
        stores(Some(published), Some(memory_store()))
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn student_inference_serves_published_files_without_a_ledger() {
        let device = Device::new();
        let mut context = plain_context(device.stores()).await;
        let (repo, _) = repository(&context).unwrap();
        let (inspection, artifact_id) = forest_artifact(&repo, "camera");
        let request = json!({
            "inspection": inspection, "artifact_id": artifact_id,
            "input": {"shape": [3, 2], "values": [0.0, 1.0, 2.0, 3.0, 40.0, -50.0]}
        });
        let ledger = run_node(StudentInferenceNode, request.clone(), device.stores())
            .await
            .unwrap();
        publish_artifact_folder(&mut context, &repo, &artifact_id).await;
        let files = run_node(
            StudentInferenceNode,
            request.clone(),
            without_ledger(&device).await,
        )
        .await
        .unwrap();
        assert_eq!(files, ledger);
        let run = RunCache::default();
        let elsewhere = run_on(
            StudentInferenceNode,
            request.clone(),
            device.stores(),
            &run,
            "other-board",
        )
        .await
        .unwrap();
        assert_eq!(output(&elsewhere, "result").await, ledger);
        let mut renamed = request;
        renamed["inspection"]["stream_id"] = json!("camera-2");
        let refused =
            run_node(StudentInferenceNode, renamed, without_ledger(&device).await).await;
        assert!(refused.is_err());
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn extracted_image_features_name_the_same_extractor_without_a_ledger() {
        let device = Device::new();
        let mut context = plain_context(device.stores()).await;
        let (repo, project) = repository(&context).unwrap();
        let (inspection, artifact) = tiny_resnet(&repo, &project, "camera");
        let request = json!({
            "inspection": inspection, "artifact_id": artifact.id,
            "input": {"shape": [1, 8, 8], "values": vec![0.25f32; 64]}
        });
        let run = RunCache::default();
        let ledger = run_on(
            ExtractPatchFeaturesNode,
            request.clone(),
            device.stores(),
            &run,
            BOARD,
        )
        .await
        .unwrap();
        publish_artifact_folder(&mut context, &repo, &artifact.id).await;
        let files = run_on(
            ExtractPatchFeaturesNode,
            request,
            without_ledger(&device).await,
            &RunCache::default(),
            BOARD,
        )
        .await
        .unwrap();
        for pin in ["extractor", "result"] {
            assert_eq!(output(&files, pin).await, output(&ledger, pin).await, "{pin}");
        }
    }

    /// A fixture model of stream `camera` with an audited evaluation that passes `policy()`.
    fn evaluated_fixture(
        repo: &TrainingRepository,
    ) -> (InspectionContext, ModelArtifact, EvaluationReport) {
        let labels = ["good", "bad"];
        let inspection =
            inspection_context("camera", TaskKind::SensorClassification, &labels, &[2]);
        for index in 0..3 {
            let class = Annotation::Class {
                class_id: (index % 2) as u32,
            };
            let labelled = sample(&inspection, index, class, reviewed());
            record(repo, &inspection, labelled, 2_000).unwrap();
        }
        let now = runtime::now_ms();
        let key = inspection.key(PROJECT.into()).unwrap();
        let job = repo
            .trigger_after_n(&key, 3, fixture_request(), split(), now)
            .unwrap()
            .unwrap();
        let lease = repo.claim_job(&job.id, "worker", 60_000, now).unwrap();
        let manifest = json!({"engine": "fixture", "labels": labels});
        let model = repo
            .finish_training(&lease, b"model", manifest, now)
            .unwrap();
        let audited = repo
            .get_snapshot(&model.snapshot_id)
            .unwrap()
            .test
            .remove(0);
        let label = audited.payload["label"].as_str().map(str::to_owned);
        let comparison = RecordPredictionRequest {
            inspection: inspection.clone(),
            prediction: PredictionRecord {
                sample_id: audited.id.clone(),
                artifact_id: model.id.clone(),
                predicted_label: label.clone(),
                teacher_label: None,
                actual_label: label,
                actual_source: Some(LabelSource::Reviewed),
                latency_ms: 1.0,
                failed: false,
                recorded_at_ms: model.created_at_ms,
                details: json!({}),
            },
        };
        record_prediction(repo.clone(), PROJECT.into(), comparison).unwrap();
        let report = repo
            .evaluate_classification(&model.id, runtime::now_ms())
            .unwrap();
        (inspection, model, report)
    }

    fn policy() -> Value {
        json!({"minimum_audited_samples": 1, "minimum_accuracy": 0.5, "minimum_class_recall": {},
            "maximum_mean_latency_ms": 10.0, "maximum_failure_rate": 0.0})
    }

    fn pointer_file(device: &Device) -> PathBuf {
        let pointer = super::super::published::deployment_pointer_path("main");
        device.root().join("apps/app/storage").join(pointer)
    }

    fn read_pointer(device: &Device) -> super::super::published::DeploymentPointer {
        from_slice(&std::fs::read(pointer_file(device)).unwrap()).unwrap()
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn deployment_pointers_follow_the_ledger_and_serve_runtimes_without_one() {
        let device = Device::new();
        let context = plain_context(device.stores()).await;
        let (repo, _) = repository(&context).unwrap();
        let (inspection, model, report) = evaluated_fixture(&repo);
        let promote = json!({
            "inspection": inspection, "deployment_id": "main", "expected_generation": 0,
            "evaluation_id": report.id, "policy": policy()
        });
        let promoted = run_node(PromoteStudentNode, promote, device.stores())
            .await
            .unwrap();
        let promoted: Deployment = from_value(promoted).unwrap();
        let pointer = read_pointer(&device);
        assert_eq!(
            (pointer.active_artifact_id, pointer.generation, pointer.paused),
            (model.id.clone(), promoted.generation, false)
        );
        let pausing = |generation: i64, paused: bool| {
            json!({"inspection": inspection, "deployment_id": "main",
                "expected_generation": generation, "paused": paused})
        };
        let paused = run_node(
            PauseStudentNode,
            pausing(promoted.generation, true),
            device.stores(),
        )
        .await
        .unwrap();
        let paused: Deployment = from_value(paused).unwrap();
        let pointer = read_pointer(&device);
        assert!(pointer.paused);
        assert_eq!(pointer.generation, promoted.generation + 1);

        let lookup = json!({"inspection": inspection, "deployment_id": "main"});
        let active = run_node(GetDeploymentNode, lookup.clone(), without_ledger(&device).await)
            .await
            .unwrap();
        assert_eq!(
            from_value::<Option<Deployment>>(active).unwrap(),
            Some(paused.clone())
        );
        let route = run_node(
            RouteInspectionNode,
            lookup.clone(),
            without_ledger(&device).await,
        )
        .await
        .unwrap();
        let route: InspectionRoute = from_value(route).unwrap();
        assert!(matches!(route.mode, InspectionMode::TeacherOnly));
        assert_eq!(route.deployment_generation, Some(paused.generation));

        let resumed = run_node(
            PauseStudentNode,
            pausing(paused.generation, false),
            device.stores(),
        )
        .await
        .unwrap();
        let resumed: Deployment = from_value(resumed).unwrap();
        let route = run_node(
            RouteInspectionNode,
            lookup.clone(),
            without_ledger(&device).await,
        )
        .await
        .unwrap();
        let route: InspectionRoute = from_value(route).unwrap();
        assert!(matches!(route.mode, InspectionMode::StudentOnly));
        assert_eq!(route.active_artifact_id, Some(model.id.clone()));
        assert_eq!(route.deployment_generation, Some(resumed.generation));

        std::fs::remove_file(pointer_file(&device)).unwrap();
        let active = run_node(GetDeploymentNode, lookup.clone(), device.stores())
            .await
            .unwrap();
        assert_eq!(
            from_value::<Option<Deployment>>(active).unwrap(),
            Some(resumed.clone())
        );
        assert_eq!(read_pointer(&device).generation, resumed.generation);

        let mut newer: Value = from_slice(&std::fs::read(pointer_file(&device)).unwrap()).unwrap();
        let generation = resumed.generation + 5;
        newer["generation"] = json!(generation);
        newer["deployment"]["generation"] = json!(generation);
        std::fs::write(pointer_file(&device), newer.to_string()).unwrap();
        let active = run_node(GetDeploymentNode, lookup.clone(), device.stores())
            .await
            .unwrap();
        let active: Option<Deployment> = from_value(active).unwrap();
        assert_eq!(active.map(|found| found.generation), Some(generation));
        let route = run_node(RouteInspectionNode, lookup, device.stores())
            .await
            .unwrap();
        let route: InspectionRoute = from_value(route).unwrap();
        assert!(matches!(route.mode, InspectionMode::StudentOnly));
        assert_eq!(route.deployment_generation, Some(generation));
        assert_eq!(read_pointer(&device).generation, generation);
    }

    #[test]
    fn published_pointers_route_to_the_student_unless_paused_or_missing() {
        let pointer = |paused: bool| -> super::super::published::DeploymentPointer {
            let stream = json!({
                "project_id": PROJECT, "stream_id": "camera", "inspection_version": "v1"
            });
            from_value(json!({
                "format_version": 1, "deployment_id": "main", "generation": 4,
                "active_artifact_id": "model-4", "paused": paused, "stream": stream,
                "sha256": "a".repeat(64), "storage": "app", "published_at_ms": 0,
                "deployment": {
                    "id": "main", "stream": stream, "generation": 4,
                    "active_artifact_id": "model-4", "paused": paused,
                    "previous_artifact_id": null, "evaluation_id": "evaluation-4",
                    "policy": null, "metric_policy": null, "updated_at_ms": 0
                }
            }))
            .unwrap()
        };
        let active = published_route(Some(&pointer(false)));
        assert!(matches!(active.mode, InspectionMode::StudentOnly));
        assert_eq!(active.active_artifact_id.as_deref(), Some("model-4"));
        assert_eq!(active.deployment_generation, Some(4));
        let paused = published_route(Some(&pointer(true)));
        assert!(matches!(paused.mode, InspectionMode::TeacherOnly));
        assert_eq!(paused.active_artifact_id, None);
        assert_eq!(paused.deployment_generation, Some(4));
        let missing = published_route(None);
        assert!(matches!(missing.mode, InspectionMode::TeacherOnly));
        assert_eq!(missing.deployment_generation, None);
        assert!(missing.candidate_artifact_id.is_none());
    }
}
