//! Publishes ledger models as FlowPath files under `models/`. Publishing never fails the
//! calling node: experiments record events, champions report a blocking next action and
//! deployments and SAM 2 sources log a warning.

use super::auto_training::AutoTrainingResult;
use super::auto_training_tables::{
    SourceScope, TableLocator, experiment_source_locators, learning_source_locators, scope_digests,
};
use super::lifecycle::{LEDGER_SCOPE_KEY, LedgerScope, StreamKind};
use super::published::{
    AnomalyCalibration, ChampionPointer, DeploymentPointer, ExperimentPointer, MANIFEST_FILE,
    MANIFEST_FORMAT_VERSION, MAXIMUM_MANIFEST_BYTES, MAXIMUM_POINTER_BYTES,
    MAXIMUM_SERVED_WEIGHTS_BYTES, ModelProvenance, ModelPublisher, ModelStorage,
    POINTER_FORMAT_VERSION, PREPROCESSING_FILE, PREPROCESSING_FORMAT_VERSION,
    PREPROCESSING_MARKER_KEY, PublishedDependencies, PublishedDependency, PublishedFile,
    PublishedModelManifest, PublishedModelRef, PublishedPreprocessing, PublishedPreprocessingRef,
    RETAINED_PREVIOUS_ARTIFACTS, WEIGHTS_FILE, artifact_dir, child, dependency_dir,
    deployment_pointer_path, experiment_pointer_path, folder_of, is_plain_file_name,
    learning_pointer_path, model_root, preprocessing_cap, run_map,
};
use flow_like::flow::execution::{LogLevel, context::ExecutionContext};
use flow_like_catalog_core::FlowPath;
use flow_like_ml_runtime::{
    Deployment, EvaluationTask, Experiment, LearningProject, LearningState, ModelArtifact,
    PretrainedSource, TrainingRepository, now_ms,
};
use flow_like_storage::{
    display_file_name,
    files::{store::FlowLikeStore, upload::put_payload},
    normalize_object_path,
    object_store::{self, ObjectMeta, ObjectStoreExt, PutPayload},
};
use flow_like_types::{
    Bytes, Result, Value, anyhow,
    json::{self, Map, json},
    storage_paths::PUBLISHED_MODELS_DIR,
};
use futures::StreamExt;
use serde::{Serialize, de::DeserializeOwned};
use std::{
    collections::{BTreeMap, BTreeSet, HashMap},
    sync::{Arc, LazyLock, Mutex, PoisonError},
};

#[derive(Clone, Debug)]
pub(crate) enum PublishOutcome {
    Published(PublishedModelRef),
    Unchanged,
    Skipped { reason: String },
    Failed { error: String },
}

/// Why a publish runs. Self-repair runs at most once per run per record (run-cache key
/// `published-models-checked-v1`) and returns at once while this process publishes the same
/// record; a state change always rebuilds the pointer from the ledger and publishes, waiting
/// for a concurrent publish of the same record first.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum PublishReason {
    /// Read paths: Get Learning Project, Get Active Student, Get and Predict fallbacks.
    SelfRepair,
    /// Experiment finalization, learning cycles, promote, rollback, pause and resume.
    StateChange,
}

/// A ledger record that Export Model writes as a model folder.
#[derive(Clone, Debug)]
pub(crate) enum LedgerModel {
    Experiment { experiment_id: String },
    LearningChampion { project_id: String },
    Artifact { artifact_id: String },
    PretrainedSource { source_id: String },
}

/// Limits of one publication; tests lower the serving cap.
#[derive(Clone, Copy, Debug)]
pub(crate) struct PublishLimits {
    pub(crate) serving_weights_bytes: u64,
}

impl Default for PublishLimits {
    fn default() -> Self {
        Self {
            serving_weights_bytes: MAXIMUM_SERVED_WEIGHTS_BYTES,
        }
    }
}

pub(crate) const PUBLISHED_EVENT: &str = "model_published";
pub(crate) const FAILED_EVENT: &str = "model_publish_failed";
pub(crate) const SKIPPED_EVENT: &str = "model_publish_skipped";
pub(crate) const SOURCE_SCOPE_UNCLASSIFIABLE: &str = "source_scope_unclassifiable";
pub(crate) const WEIGHTS_EXCEED_SERVING_CAP: &str = "weights_exceed_serving_cap";
pub(crate) const MANIFEST_TOO_LARGE: &str = "manifest_too_large";
pub(crate) const PREPROCESSING_TOO_LARGE: &str = "preprocessing_too_large";
pub(crate) const POINTER_TOO_LARGE: &str = "pointer_too_large";
pub(crate) const CHECKED_KEY: &str = "published-models-checked-v1";
const MAXIMUM_CHECKED: usize = 4096;
const MAXIMUM_EVENT_ERROR_BYTES: usize = 4096;
const MAXIMUM_PRUNE_POINTERS: usize = 1000;
/// Explicit exports may hold larger weights than automatic publication serves.
const MAXIMUM_EXPORT_WEIGHTS_BYTES: u64 = 2 * 1024 * 1024 * 1024;
const PRETRAINED_PREFIX: &str = "pretrained:";

fn warn(context: &mut ExecutionContext, message: &str) {
    tracing::warn!("{message}");
    context.log_message(message, LogLevel::Warn);
}

/// Error text an event keeps: at most 4096 bytes.
fn bounded(text: &str) -> String {
    const SUFFIX: &str = " [truncated]";
    if text.len() <= MAXIMUM_EVENT_ERROR_BYTES {
        return text.to_owned();
    }
    let mut end = MAXIMUM_EVENT_ERROR_BYTES - SUFFIX.len();
    while !text.is_char_boundary(end) {
        end -= 1;
    }
    format!("{}{SUFFIX}", &text[..end])
}

/// Runs SQLite and blob work on a blocking thread.
async fn on_ledger<T, F>(repo: &TrainingRepository, work: F) -> Result<T>
where
    T: std::any::Any + Send,
    F: FnOnce(&TrainingRepository) -> Result<T> + std::any::Any + Send,
{
    let repo = repo.clone();
    tokio::task::spawn_blocking(move || work(&repo)).await?
}

fn optional<T>(result: flow_like_ml_runtime::Result<T>) -> Result<Option<T>> {
    match result {
        Ok(value) => Ok(Some(value)),
        Err(flow_like_ml_runtime::Error::NotFound(_)) => Ok(None),
        Err(error) => Err(error.into()),
    }
}

async fn already_checked(context: &ExecutionContext, key: &str) -> bool {
    run_map::<()>(context, CHECKED_KEY)
        .await
        .is_ok_and(|checked| checked.get(key).is_some())
}

async fn mark_checked(context: &ExecutionContext, key: String) {
    if let Ok(checked) = run_map::<()>(context, CHECKED_KEY).await {
        checked.insert(key, (), MAXIMUM_CHECKED);
    }
}

type FlightLock = Arc<tokio::sync::Mutex<()>>;

static FLIGHTS: LazyLock<Mutex<HashMap<String, FlightLock>>> = LazyLock::new(Default::default);

/// Held while one publication of a record runs in this process. A concurrent state change of
/// the same record waits for it and then checks the ledger again; a concurrent self-repair
/// returns at once.
struct Flight {
    key: String,
    _guard: tokio::sync::OwnedMutexGuard<()>,
}

impl Drop for Flight {
    fn drop(&mut self) {
        let mut flights = FLIGHTS.lock().unwrap_or_else(PoisonError::into_inner);
        // Without waiters only the map and this guard still hold the lock.
        if flights
            .get(&self.key)
            .is_some_and(|lock| Arc::strong_count(lock) <= 2)
        {
            flights.remove(&self.key);
        }
    }
}

async fn single_flight(key: String) -> Flight {
    let lock = FLIGHTS
        .lock()
        .unwrap_or_else(PoisonError::into_inner)
        .entry(key.clone())
        .or_default()
        .clone();
    Flight {
        key,
        _guard: lock.lock_owned().await,
    }
}

/// The flight of `key`; None while another publication holds it. The map stays locked until
/// the attempt ends, so a failed attempt never keeps the holder's entry alive.
fn try_single_flight(key: String) -> Option<Flight> {
    let mut flights = FLIGHTS.lock().unwrap_or_else(PoisonError::into_inner);
    let guard = flights
        .entry(key.clone())
        .or_default()
        .clone()
        .try_lock_owned()
        .ok()?;
    Some(Flight { key, _guard: guard })
}

fn flight_key(kind: &str, repo: &TrainingRepository, id: &str) -> String {
    format!("{kind}\n{}\n{id}", repo.path().display())
}

/// Holds the flight a running publication of the `kind` record `id` holds.
#[cfg(test)]
pub(crate) async fn hold_flight(kind: &str, repo: &TrainingRepository, id: &str) -> Box<dyn Send> {
    Box::new(single_flight(flight_key(kind, repo, id)).await)
}

/// Holds the lock a pointer write in `target`'s root holds.
#[cfg(test)]
pub(crate) async fn hold_root(target: &WriteTarget) -> Box<dyn Send> {
    Box::new(target.prune_lock().await)
}

/// Credential-backed roots write through the undecorated content store, so a write reaches
/// the cloud or fails instead of queueing offline; other roots use the run's store. A shadow
/// run's store refuses every write.
pub(crate) async fn writer_store(
    context: &mut ExecutionContext,
    root: &FlowPath,
) -> Result<FlowLikeStore> {
    let credentials = context.credentials.clone();
    let store = match (credentials, &root.cache_store_ref) {
        (Some(credentials), Some(_)) => credentials.to_content_store_undecorated().await?,
        _ => root.to_store(context).await?,
    };
    let shadow = context
        .execution_cache
        .as_ref()
        .is_some_and(|cache| cache.shadow);
    Ok(if shadow { store.read_only() } else { store })
}

/// The `.s3flowEtag` file FlowPath keeps beside a cached copy.
fn etag_sidecar(path: &FlowPath) -> flow_like_storage::Path {
    let key = path.object_path();
    let text = key.as_ref();
    let base = key
        .extension()
        .and_then(|extension| text.strip_suffix(&format!(".{extension}")))
        .unwrap_or(text);
    normalize_object_path(&format!("{base}.s3flowEtag"))
}

/// A stored file, copied into the Local mirror when its publication ends.
pub(crate) struct Mirrored {
    path: FlowPath,
    payload: PutPayload,
    e_tag: Option<String>,
}

/// Copies written files into their FlowPath's Local cache layer with the cloud ETag. A
/// failure only warns, because the cloud copy is complete.
pub(crate) async fn mirror(context: &mut ExecutionContext, written: Vec<Mirrored>) {
    for file in written {
        if let Err(error) = file
            .path
            .write_cache_file(context, &file.payload, file.e_tag)
            .await
        {
            warn(
                context,
                &format!(
                    "Copying {} into local storage failed: {error}",
                    file.path.path
                ),
            );
        }
    }
}

/// Where a publication writes: below `root`, through `store`, mirrored into the root's Local
/// cache layer when it has one.
pub(crate) struct WriteTarget {
    pub(crate) root: FlowPath,
    pub(crate) store: FlowLikeStore,
    cache: Option<Arc<FlowLikeStore>>,
}

impl WriteTarget {
    pub(crate) async fn new(context: &mut ExecutionContext, root: FlowPath) -> Result<Self> {
        let store = writer_store(context, &root).await?;
        let cache = root
            .to_cache_layer(context)
            .await?
            .filter(|cache| matches!(**cache, FlowLikeStore::Local(_)));
        Ok(Self { root, store, cache })
    }

    async fn for_storage(context: &mut ExecutionContext, storage: ModelStorage) -> Result<Self> {
        let root = model_root(context, storage).await?;
        Self::new(context, root).await
    }

    pub(crate) fn file(&self, relative: &str) -> FlowPath {
        child(&self.root, relative)
    }

    pub(crate) async fn head(&self, path: &FlowPath) -> Result<Option<ObjectMeta>> {
        match self.store.as_generic().head(&path.object_path()).await {
            Ok(meta) => Ok(Some(meta)),
            Err(object_store::Error::NotFound { .. }) => Ok(None),
            Err(error) => Err(anyhow!(
                "Reading the metadata of {} failed: {error}",
                path.path
            )),
        }
    }

    /// The bytes and stored ETag of a file of at most `maximum` bytes; None when it is missing
    /// or larger. The GET's own size is checked before its body is read, so a file replaced by
    /// a larger one after the HEAD is never buffered.
    async fn fetch(
        &self,
        path: &FlowPath,
        maximum: u64,
    ) -> Result<Option<(Bytes, Option<String>)>> {
        let Some(meta) = self.head(path).await? else {
            return Ok(None);
        };
        if meta.size > maximum {
            return Ok(None);
        }
        let result = match self.store.as_generic().get(&path.object_path()).await {
            Ok(result) => result,
            Err(object_store::Error::NotFound { .. }) => return Ok(None),
            Err(error) => return Err(anyhow!("Reading {} failed: {error}", path.path)),
        };
        if result.meta.size > maximum {
            return Ok(None);
        }
        let e_tag = result.meta.e_tag.clone();
        let bytes = result
            .bytes()
            .await
            .map_err(|error| anyhow!("Reading {} failed: {error}", path.path))?;
        Ok(Some((bytes, e_tag)))
    }

    /// A JSON file of at most `maximum` bytes; None when it is missing, larger or unreadable,
    /// so the publication rewrites it.
    async fn read_json<T: DeserializeOwned>(
        &self,
        path: &FlowPath,
        maximum: u64,
    ) -> Result<Option<T>> {
        Ok(self
            .fetch(path, maximum)
            .await?
            .and_then(|(bytes, _)| json::from_slice(&bytes).ok()))
    }

    /// Writes `bytes`, then confirms the size and, when both report one, the ETag with a HEAD.
    pub(crate) async fn write(
        &self,
        path: &FlowPath,
        bytes: Bytes,
        mirrors: &mut Vec<Mirrored>,
    ) -> Result<Option<String>> {
        let payload = PutPayload::from_bytes(bytes);
        let size = payload.content_length() as u64;
        let written = put_payload(
            self.store.as_generic().as_ref(),
            &path.object_path(),
            payload.clone(),
        )
        .await
        .map_err(|error| anyhow!("Writing {} failed: {error}", path.path))?;
        let e_tag = self.confirm(path, size, written.e_tag).await?;
        if self.cache.is_some() {
            mirrors.push(Mirrored {
                path: path.clone(),
                payload,
                e_tag: e_tag.clone(),
            });
        }
        Ok(e_tag)
    }

    /// A HEAD after an upload: the size must match, and the ETag must equal the upload's when
    /// both exist. Returns the stored ETag.
    pub(crate) async fn confirm(
        &self,
        path: &FlowPath,
        size: u64,
        written: Option<String>,
    ) -> Result<Option<String>> {
        let stored = self
            .head(path)
            .await?
            .ok_or_else(|| anyhow!("{} is missing right after it was written", path.path))?;
        if stored.size != size {
            return Err(anyhow!(
                "{} holds {} bytes after {size} bytes were written",
                path.path,
                stored.size
            ));
        }
        if let (Some(written), Some(current)) = (&written, &stored.e_tag)
            && written != current
        {
            return Err(anyhow!(
                "{} changed while it was written: the upload returned ETag {written}, the store reports {current}",
                path.path
            ));
        }
        Ok(stored.e_tag.or(written))
    }

    async fn delete(&self, context: &mut ExecutionContext, path: &FlowPath) -> Result<()> {
        match self.store.as_generic().delete(&path.object_path()).await {
            Ok(()) | Err(object_store::Error::NotFound { .. }) => {}
            Err(error) => return Err(anyhow!("Deleting {} failed: {error}", path.path)),
        }
        if let Some(cache) = &self.cache {
            for location in [path.object_path(), etag_sidecar(path)] {
                if let Err(error) = cache.as_generic().delete(&location).await
                    && !matches!(error, object_store::Error::NotFound { .. })
                {
                    warn(
                        context,
                        &format!("Deleting the local copy {location} failed: {error}"),
                    );
                }
            }
        }
        Ok(())
    }

    /// Queues `path`, which this publication left in place, for the Local mirror when the
    /// mirror lacks it, with the stored ETag. `known` is its stored content when that is at
    /// hand; otherwise the stored file of at most `maximum` bytes is read.
    async fn mirror_stored(
        &self,
        path: &FlowPath,
        known: Option<&Bytes>,
        maximum: u64,
        mirrors: &mut Vec<Mirrored>,
    ) -> Result<()> {
        let Some(cache) = &self.cache else {
            return Ok(());
        };
        if mirrors.iter().any(|file| file.path.path == path.path) {
            return Ok(());
        }
        let lacks = matches!(
            cache.as_generic().head(&path.object_path()).await,
            Err(object_store::Error::NotFound { .. })
        );
        if !lacks {
            return Ok(());
        }
        let stored = match known {
            Some(bytes) => self
                .head(path)
                .await?
                .map(|meta| (bytes.clone(), meta.e_tag)),
            None => self.fetch(path, maximum).await?,
        };
        if let Some((bytes, e_tag)) = stored {
            mirrors.push(Mirrored {
                path: path.clone(),
                payload: PutPayload::from_bytes(bytes),
                e_tag,
            });
        }
        Ok(())
    }

    /// Held by pruning and by every pointer write below this root in this process, so pruning
    /// never deletes a folder that a pointer being written names. Other processes writing the
    /// same root are not covered.
    async fn prune_lock(&self) -> Flight {
        single_flight(format!("root\n{}\n{}", self.root.store_ref, self.root.path)).await
    }

    /// The folder's manifest when it records the same artifact and weights in plain file
    /// names, so the folder counts as published.
    async fn matching_manifest(
        &self,
        path: &FlowPath,
        plan: &FolderPlan,
    ) -> Result<Option<PublishedModelManifest>> {
        let manifest = self
            .read_json::<PublishedModelManifest>(path, MAXIMUM_MANIFEST_BYTES)
            .await?;
        Ok(manifest.filter(|manifest| {
            manifest.format_version == MANIFEST_FORMAT_VERSION
                && manifest.artifact_id() == plan.artifact_id()
                && manifest.weights.sha256 == plan.manifest.weights.sha256
                && manifest.weights.bytes == plan.manifest.weights.bytes
                && manifest
                    .weights
                    .file
                    .as_deref()
                    .is_some_and(is_plain_file_name)
                && manifest
                    .preprocessing
                    .as_ref()
                    .is_none_or(|reference| is_plain_file_name(&reference.file))
        }))
    }
}

/// What a model manifest carries beside its record.
#[derive(Clone, Default)]
struct ModelMetadata {
    task: Option<EvaluationTask>,
    calibration: Option<AnomalyCalibration>,
    goal_met: Option<bool>,
    validation_metrics: BTreeMap<String, f64>,
    final_test_metrics: BTreeMap<String, f64>,
    source_table_versions: Vec<Value>,
    table_preprocessing: Option<Value>,
}

impl ModelMetadata {
    /// The selected trial's validation scale calibrates anomaly scores, as the final audit
    /// did.
    fn of_experiment(experiment: &Experiment, result: &AutoTrainingResult) -> Self {
        let task = experiment.request.goals.task.clone();
        let calibration = match &task {
            EvaluationTask::Anomaly { threshold } => Some(AnomalyCalibration {
                scale: result
                    .validation_metrics
                    .get("anomaly_scale")
                    .copied()
                    .unwrap_or(1.0),
                threshold: *threshold,
            }),
            _ => None,
        };
        Self {
            task: Some(task),
            calibration,
            goal_met: Some(result.goal_met),
            validation_metrics: result.validation_metrics.clone(),
            final_test_metrics: result.final_test_metrics.clone(),
            source_table_versions: result.source_table_versions.clone(),
            table_preprocessing: Some(result.preprocessing_manifest.clone())
                .filter(|preprocessing| !preprocessing.is_null()),
        }
    }
}

fn provenance(
    publisher: ModelPublisher,
    ledger: LedgerScope,
    storage: Option<ModelStorage>,
) -> ModelProvenance {
    ModelProvenance {
        publisher,
        ledger,
        storage,
        dependency: false,
        experiment_id: None,
        selected_trial_id: None,
        learning_project_id: None,
        cycle_id: None,
        deployment_id: None,
        deployment_generation: None,
        published_at_ms: now_ms(),
    }
}

fn selected_trial(experiment: &Experiment) -> Option<String> {
    experiment
        .selected_trial_id
        .clone()
        .or_else(|| experiment.best_trial_id.clone())
}

struct ExperimentRecord {
    experiment: Experiment,
    result: AutoTrainingResult,
}

fn load_experiment(repo: &TrainingRepository, id: &str) -> Result<ExperimentRecord> {
    let state = repo.experiment_result(id)?;
    let experiment = state.experiment.clone();
    Ok(ExperimentRecord {
        experiment,
        result: state.into(),
    })
}

/// The experiment's metadata; `provenance` notes the experiment and its selected trial.
fn experiment_metadata(
    record: &ExperimentRecord,
    provenance: &mut ModelProvenance,
) -> ModelMetadata {
    provenance.experiment_id = Some(record.experiment.id.clone());
    provenance.selected_trial_id = selected_trial(&record.experiment);
    ModelMetadata::of_experiment(&record.experiment, &record.result)
}

/// The files of one model folder, built from a ledger record.
struct FolderPlan {
    weights: Bytes,
    preprocessing: Option<Bytes>,
    manifest: PublishedModelManifest,
    dependency: Option<Box<FolderPlan>>,
}

impl FolderPlan {
    fn artifact_id(&self) -> &str {
        self.manifest.artifact_id()
    }

    fn weights_file(&self) -> &str {
        self.manifest
            .weights
            .file
            .as_deref()
            .unwrap_or(WEIGHTS_FILE)
    }

    fn model_ref(&self, folder: &FlowPath, storage: Option<ModelStorage>) -> PublishedModelRef {
        PublishedModelRef {
            artifact_id: self.artifact_id().to_owned(),
            manifest: child(folder, MANIFEST_FILE),
            weights: child(folder, self.weights_file()),
            sha256: self.manifest.weights.sha256.clone(),
            bytes: self.manifest.weights.bytes,
            storage,
        }
    }
}

/// Why a ledger record is left unpublished; `code` is the reason its skip event records.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum SkipReason {
    SourceScopeUnclassifiable,
    WeightsExceedServingCap,
    ManifestTooLarge,
    PreprocessingTooLarge,
}

impl SkipReason {
    fn code(&self) -> &str {
        match self {
            Self::SourceScopeUnclassifiable => SOURCE_SCOPE_UNCLASSIFIABLE,
            Self::WeightsExceedServingCap => WEIGHTS_EXCEED_SERVING_CAP,
            Self::ManifestTooLarge => MANIFEST_TOO_LARGE,
            Self::PreprocessingTooLarge => PREPROCESSING_TOO_LARGE,
        }
    }

    /// Why Export Model refuses the record.
    fn export_text(&self) -> &str {
        match self {
            Self::SourceScopeUnclassifiable => {
                "its source tables match neither this project's database nor yours"
            }
            Self::WeightsExceedServingCap => "its weights exceed the 2 GiB export limit",
            Self::ManifestTooLarge => "its manifest would exceed 1 MiB",
            Self::PreprocessingTooLarge => {
                "its preprocessing exceeds the size its feature pipeline allows"
            }
        }
    }
}

enum Planned {
    Ready(FolderPlan),
    Skip(SkipReason),
}

/// How a ledger record becomes a folder plan.
struct PlanSpec {
    metadata: ModelMetadata,
    provenance: ModelProvenance,
    preprocessing_file: String,
    weights_file: Option<String>,
    limit: u64,
}

impl PlanSpec {
    fn published(metadata: ModelMetadata, provenance: ModelProvenance, limit: u64) -> Self {
        Self {
            metadata,
            provenance,
            preprocessing_file: PREPROCESSING_FILE.into(),
            weights_file: Some(WEIGHTS_FILE.into()),
            limit,
        }
    }
}

/// Moves the artifact manifest's preprocessing manifest out, leaving the marker that names
/// the file it moves to.
fn take_artifact_preprocessing(artifact: &mut ModelArtifact, file: &str) -> Option<Value> {
    let slot = artifact.manifest.get_mut("preprocessing_manifest")?;
    if slot.is_null() {
        return None;
    }
    let mut marker = Map::new();
    marker.insert(
        PREPROCESSING_MARKER_KEY.to_owned(),
        Value::String(file.to_owned()),
    );
    Some(std::mem::replace(slot, Value::Object(marker)))
}

/// `L`, the read limit of a table preprocessing's feature pipeline: an engineered pipeline's
/// own limit, otherwise the default 64 MiB.
fn feature_read_limit(preprocessing: Option<&Value>) -> u64 {
    preprocessing
        .filter(|value| value.get("kind").and_then(Value::as_str) == Some("engineered"))
        .and_then(|value| value.pointer("/pipeline/limits/maximum_bytes"))
        .and_then(Value::as_u64)
        .unwrap_or(
            flow_like_ml_native::feature_engineering::FeatureLimits::default().maximum_bytes as u64,
        )
}

fn recorded_dependency(manifest: &Value) -> Result<Option<PublishedDependency>> {
    let Some(value) = manifest
        .get("feature_extractor")
        .filter(|value| !value.is_null())
    else {
        return Ok(None);
    };
    let extractor = super::lifecycle::runtime_extractor(value)?;
    Ok(Some(PublishedDependency {
        source_id: extractor.source_id,
        blob_sha256: extractor.blob_sha256,
    }))
}

fn finish_plan(
    weights: Vec<u8>,
    preprocessing: Option<Vec<u8>>,
    manifest: PublishedModelManifest,
) -> Result<Planned> {
    if json::to_vec(&manifest)?.len() as u64 > MAXIMUM_MANIFEST_BYTES {
        return Ok(Planned::Skip(SkipReason::ManifestTooLarge));
    }
    Ok(Planned::Ready(FolderPlan {
        weights: Bytes::from(weights),
        preprocessing: preprocessing.map(Bytes::from),
        manifest,
        dependency: None,
    }))
}

/// A preprocessing file and the read limit of its feature pipeline.
struct PreprocessingFile {
    bytes: Vec<u8>,
    limit: u64,
}

/// The preprocessing file holding `table` and, when it differs, the artifact's moved
/// preprocessing manifest. None when it exceeds what the table's feature pipeline allows.
fn preprocessing_file(table: &Value, moved: Option<Value>) -> Result<Option<PreprocessingFile>> {
    let limit = feature_read_limit(Some(table));
    let document = PublishedPreprocessing {
        format_version: PREPROCESSING_FORMAT_VERSION,
        artifact_preprocessing_manifest: moved.filter(|manifest| manifest != table),
        preprocessing: Some(table.clone()),
    };
    let bytes = json::to_vec(&document)?;
    if bytes.len() as u64 > preprocessing_cap(limit) {
        return Ok(None);
    }
    Ok(Some(PreprocessingFile { bytes, limit }))
}

fn artifact_manifest(
    artifact: ModelArtifact,
    spec: PlanSpec,
    preprocessing: Option<PublishedPreprocessingRef>,
    feature_extractor: Option<PublishedDependency>,
) -> PublishedModelManifest {
    let weights = PublishedFile {
        file: spec.weights_file,
        sha256: artifact.blob.sha256.clone(),
        bytes: artifact.blob.bytes,
    };
    PublishedModelManifest {
        format_version: MANIFEST_FORMAT_VERSION,
        artifact: Some(artifact),
        pretrained_source: None,
        weights,
        preprocessing,
        task: spec.metadata.task,
        calibration: spec.metadata.calibration,
        goal_met: spec.metadata.goal_met,
        validation_metrics: spec.metadata.validation_metrics,
        final_test_metrics: spec.metadata.final_test_metrics,
        source_table_versions: spec.metadata.source_table_versions,
        dependencies: PublishedDependencies { feature_extractor },
        provenance: Some(spec.provenance),
    }
}

/// A model artifact's folder. Its preprocessing manifest moves into the preprocessing file,
/// which also holds the table preprocessing; when both are equal it is stored once.
fn artifact_plan(
    repo: &TrainingRepository,
    mut artifact: ModelArtifact,
    spec: PlanSpec,
) -> Result<Planned> {
    if artifact.blob.bytes > spec.limit {
        return Ok(Planned::Skip(SkipReason::WeightsExceedServingCap));
    }
    let weights = repo.read_blob_limited(&artifact.blob, spec.limit)?;
    let dependency = recorded_dependency(&artifact.manifest)?;
    let moved = take_artifact_preprocessing(&mut artifact, &spec.preprocessing_file);
    let table = spec
        .metadata
        .table_preprocessing
        .clone()
        .or_else(|| moved.clone());
    let preprocessing = match &table {
        None => None,
        Some(table) => match preprocessing_file(table, moved)? {
            Some(file) => Some(file),
            None => return Ok(Planned::Skip(SkipReason::PreprocessingTooLarge)),
        },
    };
    let reference = preprocessing
        .as_ref()
        .map(|file| PublishedPreprocessingRef {
            file: spec.preprocessing_file.clone(),
            sha256: flow_like_ml_core::content_digest(&file.bytes),
            bytes: file.bytes.len() as u64,
            maximum_bytes: file.limit,
        });
    let manifest = artifact_manifest(artifact, spec, reference, dependency);
    finish_plan(weights, preprocessing.map(|file| file.bytes), manifest)
}

/// A pretrained source's folder: its weights and a manifest with the source record.
fn source_plan(
    repo: &TrainingRepository,
    source: PretrainedSource,
    provenance: ModelProvenance,
    limit: u64,
) -> Result<Planned> {
    if source.blob.bytes > limit {
        return Ok(Planned::Skip(SkipReason::WeightsExceedServingCap));
    }
    let weights = repo.read_blob_limited(&source.blob, limit)?;
    let file = PublishedFile {
        file: Some(WEIGHTS_FILE.into()),
        sha256: source.blob.sha256.clone(),
        bytes: source.blob.bytes,
    };
    let manifest = PublishedModelManifest {
        format_version: MANIFEST_FORMAT_VERSION,
        artifact: None,
        pretrained_source: Some(source),
        weights: file,
        preprocessing: None,
        task: None,
        calibration: None,
        goal_met: None,
        validation_metrics: BTreeMap::new(),
        final_test_metrics: BTreeMap::new(),
        source_table_versions: Vec::new(),
        dependencies: PublishedDependencies::default(),
        provenance: Some(provenance),
    };
    finish_plan(weights, None, manifest)
}

enum DependencyLookup {
    Found(FolderPlan),
    Missing,
    /// Only the executing user's private ledger holds it, and the model goes to app storage.
    Private,
    TooLarge,
}

fn check_dependency_digest(dependency: &PublishedDependency, sha256: &str) -> Result<()> {
    if dependency.blob_sha256 != sha256 {
        return Err(anyhow!(
            "Feature extractor {} holds weights {sha256} in the training ledger, but the model records {}",
            dependency.source_id,
            dependency.blob_sha256
        ));
    }
    Ok(())
}

/// The folder of a model's feature extractor from `repo`: a pretrained source, or for a
/// training artifact id its full artifact record, so a model deployed later shares the folder.
fn dependency_plan(
    repo: &TrainingRepository,
    dependency: &PublishedDependency,
    provenance: ModelProvenance,
    limit: u64,
) -> Result<DependencyLookup> {
    let planned = if dependency.source_id.starts_with(PRETRAINED_PREFIX) {
        let Some(source) = optional(repo.get_pretrained_source(&dependency.source_id))? else {
            return Ok(DependencyLookup::Missing);
        };
        check_dependency_digest(dependency, &source.blob.sha256)?;
        source_plan(repo, source, provenance, limit)?
    } else {
        let Some(artifact) = optional(repo.get_artifact(&dependency.source_id))? else {
            return Ok(DependencyLookup::Missing);
        };
        check_dependency_digest(dependency, &artifact.blob.sha256)?;
        artifact_plan(
            repo,
            artifact,
            PlanSpec::published(ModelMetadata::default(), provenance, limit),
        )?
    };
    Ok(match planned {
        Planned::Ready(plan) => DependencyLookup::Found(plan),
        Planned::Skip(_) => DependencyLookup::TooLarge,
    })
}

fn holds_extractor(repo: &TrainingRepository, source_id: &str) -> Result<bool> {
    Ok(if source_id.starts_with(PRETRAINED_PREFIX) {
        optional(repo.get_pretrained_source(source_id))?.is_some()
    } else {
        optional(repo.get_artifact(source_id))?.is_some()
    })
}

/// A dependency's provenance: its dependent's, in the ledger that holds the dependency.
fn dependency_provenance(base: &ModelProvenance, ledger: LedgerScope) -> ModelProvenance {
    ModelProvenance {
        ledger,
        dependency: true,
        experiment_id: None,
        selected_trial_id: None,
        learning_project_id: None,
        cycle_id: None,
        deployment_id: None,
        deployment_generation: None,
        ..base.clone()
    }
}

async fn plan_dependency_in(
    repo: &TrainingRepository,
    dependency: &PublishedDependency,
    provenance: ModelProvenance,
    limit: u64,
) -> Result<DependencyLookup> {
    let dependency = dependency.clone();
    on_ledger(repo, move |repo| {
        dependency_plan(repo, &dependency, provenance, limit)
    })
    .await
}

/// The extractor's folder from the record's own ledger, else from the first ledger of this
/// run's App chain that holds it. `storage` is the model root, None for an explicit export;
/// an extractor found only in the private ledger never goes to app storage.
async fn lookup_dependency(
    context: &mut ExecutionContext,
    repo: &TrainingRepository,
    scope: LedgerScope,
    storage: Option<ModelStorage>,
    dependency: &PublishedDependency,
    base: &ModelProvenance,
    limit: u64,
) -> Result<DependencyLookup> {
    let own = dependency_provenance(base, scope);
    let lookup = plan_dependency_in(repo, dependency, own, limit).await?;
    if !matches!(lookup, DependencyLookup::Missing) {
        return Ok(lookup);
    }
    let source_id = dependency.source_id.clone();
    let located = super::lifecycle::locate_repository(context, |candidate, _| {
        holds_extractor(candidate, &source_id)
    });
    match located {
        Ok(Some(located))
            if located.scope == LedgerScope::User && storage == Some(ModelStorage::App) =>
        {
            Ok(DependencyLookup::Private)
        }
        Ok(Some(located)) => {
            let provenance = dependency_provenance(base, located.scope);
            plan_dependency_in(&located.repo, dependency, provenance, limit).await
        }
        Ok(None) => Ok(DependencyLookup::Missing),
        Err(error) => {
            warn(
                context,
                &format!(
                    "Looking up feature extractor {} in this app's training ledgers failed: {error}",
                    dependency.source_id
                ),
            );
            Ok(DependencyLookup::Missing)
        }
    }
}

/// Adds the folder of the model's feature extractor; a missing, private or oversized
/// extractor only warns, and the model is published without it. `storage` is the model root,
/// None for an explicit export, which may write a private extractor anywhere.
async fn attach_dependency(
    context: &mut ExecutionContext,
    repo: &TrainingRepository,
    scope: LedgerScope,
    storage: Option<ModelStorage>,
    plan: &mut FolderPlan,
    limit: u64,
) -> Result<()> {
    let Some(dependency) = plan.manifest.dependencies.feature_extractor.clone() else {
        return Ok(());
    };
    let base = plan
        .manifest
        .provenance
        .clone()
        .unwrap_or_else(|| provenance(ModelPublisher::Export, scope, None));
    let lookup =
        lookup_dependency(context, repo, scope, storage, &dependency, &base, limit).await?;
    let problem = match lookup {
        DependencyLookup::Found(found) => {
            plan.dependency = Some(Box::new(found));
            return Ok(());
        }
        DependencyLookup::Missing => "is in no training ledger of this app".to_owned(),
        DependencyLookup::Private => {
            "is only in your private training ledger, which app storage never receives".to_owned()
        }
        DependencyLookup::TooLarge => format!("has more than {limit} bytes of weights"),
    };
    warn(
        context,
        &format!(
            "Feature extractor {} of model {} {problem}, so it is not published beside the model",
            dependency.source_id,
            plan.artifact_id()
        ),
    );
    Ok(())
}

/// The model root a record publishes to: the user's for the private ledger, the marker's for
/// marked App records, and for App records from before ledger scopes the scope their source
/// tables classify to. None when those sources cannot be classified.
async fn record_storage(
    context: &ExecutionContext,
    scope: LedgerScope,
    marker: Option<&Value>,
    locators: &[Option<TableLocator>],
) -> Result<Option<ModelStorage>> {
    if scope == LedgerScope::User {
        return Ok(Some(ModelStorage::User));
    }
    if let Some(marker) = marker.filter(|marker| !marker.is_null()) {
        let marked: LedgerScope = json::from_value(marker.clone())
            .map_err(|error| anyhow!("The ledger_scope marker {marker} is invalid: {error}"))?;
        return Ok(Some(marked.into()));
    }
    let digests = scope_digests(context).await?;
    Ok(match digests.combine(locators) {
        SourceScope::Project => Some(ModelStorage::App),
        SourceScope::User => Some(ModelStorage::User),
        SourceScope::Unknown => None,
    })
}

/// `record_storage`, warning that the `model` of `record` stays unpublished when its source
/// tables cannot be classified.
async fn classified_storage(
    context: &mut ExecutionContext,
    scope: LedgerScope,
    marker: Option<&Value>,
    locators: &[Option<TableLocator>],
    record: &str,
    model: &str,
) -> Result<Option<ModelStorage>> {
    let storage = record_storage(context, scope, marker, locators).await?;
    if storage.is_none() {
        warn(
            context,
            &format!(
                "{record} predates ledger scopes and reads source tables that match neither this project's database nor yours, so its {model} is not published"
            ),
        );
    }
    Ok(storage)
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum FolderMode {
    /// A model folder: kept when its manifest records the same weights, repaired otherwise.
    Model,
    /// A dependency or pretrained source: left alone when its manifest records the same
    /// weights.
    ContentAddressed,
    /// An explicit export: every file is written.
    Export,
}

/// A written folder: whether any of its files or its dependency's changed, and the manifests a
/// pointer to it relies on, its own and its dependency's.
struct FolderWrite {
    any: bool,
    weights_e_tag: Option<String>,
    manifests: Vec<FlowPath>,
}

/// A folder whose data files are in place, whether its manifest must be written, and the
/// matching manifest it already holds.
#[derive(Default)]
struct FolderFiles {
    any: bool,
    weights_e_tag: Option<String>,
    rewrite_manifest: bool,
    kept: Option<PublishedModelManifest>,
}

/// Whether a folder's manifest must be written: it holds no matching one, or the one it keeps
/// lacks the planned preprocessing reference.
fn manifest_outdated(kept: Option<&PublishedModelManifest>, plan: &FolderPlan) -> bool {
    kept.is_none_or(|kept| kept.preprocessing.is_none() && plan.manifest.preprocessing.is_some())
}

/// The stored object at `path`, looked up only when the folder's files are checked.
async fn present(
    target: &WriteTarget,
    path: &FlowPath,
    checks: bool,
) -> Result<Option<ObjectMeta>> {
    if checks {
        target.head(path).await
    } else {
        Ok(None)
    }
}

/// Writes the planned preprocessing file unless the file the kept manifest lists is present
/// with its recorded size. Some(whether that listing changes) when it was written.
async fn write_preprocessing(
    target: &WriteTarget,
    folder: &FlowPath,
    plan: &FolderPlan,
    kept: Option<&PublishedModelManifest>,
    checks: bool,
    mirrors: &mut Vec<Mirrored>,
) -> Result<Option<bool>> {
    let (Some(bytes), Some(planned)) = (&plan.preprocessing, &plan.manifest.preprocessing) else {
        return Ok(None);
    };
    let listed = kept
        .and_then(|manifest| manifest.preprocessing.as_ref())
        .unwrap_or(planned);
    let stored = present(target, &child(folder, &listed.file), checks).await?;
    if stored.is_some_and(|meta| meta.size == listed.bytes) {
        return Ok(None);
    }
    target
        .write(&child(folder, &planned.file), bytes.clone(), mirrors)
        .await?;
    Ok(Some(listed != planned))
}

/// Writes a folder's weights and preprocessing unless they are present with the size its
/// manifest records; a content-addressed folder that is already published is left alone. A
/// kept manifest without the planned preprocessing reference is rewritten.
async fn write_folder_files(
    target: &WriteTarget,
    folder: &FlowPath,
    plan: &FolderPlan,
    mode: FolderMode,
    mirrors: &mut Vec<Mirrored>,
) -> Result<FolderFiles> {
    let checks = mode != FolderMode::Export;
    let kept = if checks {
        target
            .matching_manifest(&child(folder, MANIFEST_FILE), plan)
            .await?
    } else {
        None
    };
    if mode == FolderMode::ContentAddressed && kept.is_some() {
        return Ok(FolderFiles {
            kept,
            ..FolderFiles::default()
        });
    }
    let described = kept.as_ref().unwrap_or(&plan.manifest);
    let weights_path = child(
        folder,
        described.weights.file.as_deref().unwrap_or(WEIGHTS_FILE),
    );
    let stored = present(target, &weights_path, checks).await?;
    let mut any = false;
    let weights_e_tag = match stored.filter(|meta| meta.size == plan.manifest.weights.bytes) {
        Some(meta) => meta.e_tag,
        None => {
            any = true;
            target
                .write(&weights_path, plan.weights.clone(), mirrors)
                .await?
        }
    };
    let mut rewrite_manifest = manifest_outdated(kept.as_ref(), plan);
    if let Some(changed) =
        write_preprocessing(target, folder, plan, kept.as_ref(), checks, mirrors).await?
    {
        any = true;
        rewrite_manifest |= changed;
    }
    Ok(FolderFiles {
        any,
        weights_e_tag,
        rewrite_manifest,
        kept,
    })
}

async fn write_folder_manifest(
    target: &WriteTarget,
    folder: &FlowPath,
    plan: &FolderPlan,
    mirrors: &mut Vec<Mirrored>,
) -> Result<()> {
    target
        .write(
            &child(folder, MANIFEST_FILE),
            Bytes::from(json::to_vec(&plan.manifest)?),
            mirrors,
        )
        .await?;
    Ok(())
}

/// Where a folder's dependency goes: `dependencies/{source}` inside an export, otherwise its
/// own content-addressed folder below the root.
fn dependency_location(
    target: &WriteTarget,
    folder: &FlowPath,
    mode: FolderMode,
    dependency: &FolderPlan,
) -> (FlowPath, FolderMode) {
    if mode == FolderMode::Export {
        (
            child(folder, &dependency_dir(dependency.artifact_id())),
            FolderMode::Export,
        )
    } else {
        (
            target.file(&artifact_dir(dependency.artifact_id())),
            FolderMode::ContentAddressed,
        )
    }
}

/// Queues the files a folder's manifest names that this publication left in place and the
/// Local mirror lacks; that manifest is the planned one when it was written.
async fn mirror_folder(
    target: &WriteTarget,
    folder: &FlowPath,
    plan: &FolderPlan,
    files: &FolderFiles,
    mirrors: &mut Vec<Mirrored>,
) -> Result<()> {
    if target.cache.is_none() {
        return Ok(());
    }
    let manifest = if files.rewrite_manifest {
        Some(&plan.manifest)
    } else {
        files.kept.as_ref()
    };
    let Some(manifest) = manifest else {
        return Ok(());
    };
    let weights_file = manifest.weights.file.as_deref().unwrap_or(WEIGHTS_FILE);
    let weights = child(folder, weights_file);
    let size = plan.manifest.weights.bytes;
    target
        .mirror_stored(&weights, Some(&plan.weights), size, mirrors)
        .await?;
    if let Some(reference) = &manifest.preprocessing {
        let path = child(folder, &reference.file);
        let known = plan
            .preprocessing
            .as_ref()
            .filter(|_| plan.manifest.preprocessing.as_ref() == Some(reference));
        target
            .mirror_stored(&path, known, reference.bytes, mirrors)
            .await?;
    }
    let manifest_path = child(folder, MANIFEST_FILE);
    target
        .mirror_stored(&manifest_path, None, MAXIMUM_MANIFEST_BYTES, mirrors)
        .await
}

/// Writes the folder's manifest when it must change, which makes the folder visible, and
/// queues the files the Local mirror lacks. Returns whether anything in the folder changed.
async fn finish_folder(
    target: &WriteTarget,
    folder: &FlowPath,
    plan: &FolderPlan,
    files: &FolderFiles,
    mirrors: &mut Vec<Mirrored>,
) -> Result<bool> {
    if files.rewrite_manifest {
        write_folder_manifest(target, folder, plan, mirrors).await?;
    }
    mirror_folder(target, folder, plan, files, mirrors).await?;
    Ok(files.any || files.rewrite_manifest)
}

/// Writes a folder in order: weights, preprocessing, the dependency folder, then the
/// manifest. Files that are already present with the size the manifest records are kept.
async fn write_model_folder(
    target: &WriteTarget,
    folder: &FlowPath,
    plan: &FolderPlan,
    mode: FolderMode,
    mirrors: &mut Vec<Mirrored>,
) -> Result<FolderWrite> {
    let files = write_folder_files(target, folder, plan, mode, mirrors).await?;
    let mut manifests = vec![child(folder, MANIFEST_FILE)];
    let mut dependency_written = false;
    if let Some(dependency) = &plan.dependency {
        let (dependency_folder, dependency_mode) =
            dependency_location(target, folder, mode, dependency);
        let written = write_folder_files(
            target,
            &dependency_folder,
            dependency,
            dependency_mode,
            mirrors,
        )
        .await?;
        dependency_written =
            finish_folder(target, &dependency_folder, dependency, &written, mirrors).await?;
        manifests.push(child(&dependency_folder, MANIFEST_FILE));
    }
    let any = finish_folder(target, folder, plan, &files, mirrors).await? || dependency_written;
    Ok(FolderWrite {
        any,
        weights_e_tag: files.weights_e_tag,
        manifests,
    })
}

/// Equal apart from the publication time.
fn same_pointer<T: Serialize>(left: &T, right: &T) -> bool {
    let comparable = |pointer: &T| {
        let mut value = json::to_value(pointer).ok();
        if let Some(Value::Object(fields)) = &mut value {
            fields.remove("published_at_ms");
        }
        value
    };
    comparable(left) == comparable(right)
}

/// Previous artifacts a pointer keeps, newest first: the previous pointer's artifact, then its
/// retained ids, without the new artifact, at most three.
pub(crate) fn next_retained(
    previous: Option<(Option<&str>, &[String])>,
    current: Option<&str>,
) -> Vec<String> {
    let Some((previous_current, previous_retained)) = previous else {
        return Vec::new();
    };
    let mut retained: Vec<String> = Vec::new();
    for id in previous_current
        .into_iter()
        .chain(previous_retained.iter().map(String::as_str))
    {
        if Some(id) != current && !retained.iter().any(|kept| kept.as_str() == id) {
            retained.push(id.to_owned());
        }
    }
    retained.truncate(RETAINED_PREVIOUS_ARTIFACTS);
    retained
}

/// Ids the previous pointer referenced that the new one no longer does.
pub(crate) fn released_ids(
    previous: Option<(Option<&str>, &[String])>,
    current: Option<&str>,
    retained: &[String],
) -> Vec<String> {
    let Some((previous_current, previous_retained)) = previous else {
        return Vec::new();
    };
    previous_current
        .into_iter()
        .chain(previous_retained.iter().map(String::as_str))
        .filter(|id| Some(*id) != current && !retained.iter().any(|kept| kept.as_str() == *id))
        .map(str::to_owned)
        .collect()
}

/// The pointer files below the root; None above 1000.
async fn pointer_locations(target: &WriteTarget) -> Result<Option<Vec<flow_like_storage::Path>>> {
    let mut pointers = Vec::new();
    for kind in ["experiments", "learning", "deployments"] {
        let prefix = target
            .file(&format!("{PUBLISHED_MODELS_DIR}/{kind}"))
            .object_path();
        let mut listing = target.store.as_generic().list(Some(&prefix));
        while let Some(meta) = listing.next().await {
            let meta = meta.map_err(|error| anyhow!("Listing {prefix} failed: {error}"))?;
            if meta.location.as_ref().ends_with(".json") {
                pointers.push(meta.location);
            }
            if pointers.len() > MAXIMUM_PRUNE_POINTERS {
                return Ok(None);
            }
        }
    }
    Ok(Some(pointers))
}

/// The ids a pointer of any kind names as current or retained.
fn pointer_references(pointer: &Value) -> Vec<String> {
    let current = ["artifact_id", "active_artifact_id"]
        .into_iter()
        .filter_map(|key| pointer.get(key).and_then(Value::as_str));
    let retained = pointer
        .get("retained")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .filter_map(Value::as_str);
    current.chain(retained).map(str::to_owned).collect()
}

/// Ids every pointer of the root names as current or retained; None above 1000 pointers.
async fn referenced_artifacts(target: &WriteTarget) -> Result<Option<BTreeSet<String>>> {
    let Some(pointers) = pointer_locations(target).await? else {
        return Ok(None);
    };
    let mut referenced = BTreeSet::new();
    for location in pointers {
        let path = FlowPath {
            path: location.as_ref().to_string(),
            store_ref: target.root.store_ref.clone(),
            cache_store_ref: target.root.cache_store_ref.clone(),
        };
        let pointer = target
            .read_json::<Value>(&path, MAXIMUM_POINTER_BYTES)
            .await?
            .ok_or_else(|| anyhow!("The model pointer {} cannot be read", path.path))?;
        referenced.extend(pointer_references(&pointer));
    }
    Ok(Some(referenced))
}

/// The extractors the manifests of `referenced` folders name.
async fn protected_dependencies(
    target: &WriteTarget,
    referenced: &BTreeSet<String>,
) -> Result<BTreeSet<String>> {
    let mut protected = BTreeSet::new();
    for id in referenced {
        let path = target.file(&format!("{}/{MANIFEST_FILE}", artifact_dir(id)));
        if let Some(manifest) = target
            .read_json::<PublishedModelManifest>(&path, MAXIMUM_MANIFEST_BYTES)
            .await?
            && let Some(dependency) = manifest.dependencies.feature_extractor
        {
            protected.insert(dependency.source_id);
        }
    }
    Ok(protected)
}

/// Deletes the folder of `id`, manifest first, when automatic publication wrote it, and
/// returns the extractor it names.
async fn delete_auto_folder(
    context: &mut ExecutionContext,
    target: &WriteTarget,
    id: &str,
) -> Result<Option<String>> {
    let folder = target.file(&artifact_dir(id));
    let manifest_path = child(&folder, MANIFEST_FILE);
    let Some(manifest) = target
        .read_json::<PublishedModelManifest>(&manifest_path, MAXIMUM_MANIFEST_BYTES)
        .await?
    else {
        return Ok(None);
    };
    if manifest
        .provenance
        .as_ref()
        .is_none_or(|provenance| provenance.publisher != ModelPublisher::Auto)
    {
        return Ok(None);
    }
    target.delete(context, &manifest_path).await?;
    let files = manifest
        .preprocessing
        .as_ref()
        .map(|reference| reference.file.as_str())
        .into_iter()
        .chain(Some(
            manifest.weights.file.as_deref().unwrap_or(WEIGHTS_FILE),
        ));
    for file in files.filter(|file| is_plain_file_name(file)) {
        target.delete(context, &child(&folder, file)).await?;
    }
    Ok(manifest
        .dependencies
        .feature_extractor
        .map(|dependency| dependency.source_id))
}

/// Deletes released folders that automatic publication wrote and that no pointer and no kept
/// manifest names, then their dependencies under the same rule. Pointer writes in this process
/// wait for it, and it waits for them.
pub(crate) async fn prune_released(
    context: &mut ExecutionContext,
    target: &WriteTarget,
    released: Vec<String>,
) -> Result<()> {
    let _pruning = target.prune_lock().await;
    let Some(referenced) = referenced_artifacts(target).await? else {
        warn(
            context,
            &format!(
                "{} holds more than {MAXIMUM_PRUNE_POINTERS} model pointers, so released model folders are kept",
                target.root.path
            ),
        );
        return Ok(());
    };
    let protected = protected_dependencies(target, &referenced).await?;
    let mut queue = released;
    let mut visited = BTreeSet::new();
    while let Some(id) = queue.pop() {
        if referenced.contains(&id) || protected.contains(&id) || !visited.insert(id.clone()) {
            continue;
        }
        if let Some(dependency) = delete_auto_folder(context, target, &id).await? {
            queue.push(dependency);
        }
    }
    Ok(())
}

async fn prune(context: &mut ExecutionContext, target: &WriteTarget, released: Vec<String>) {
    if released.is_empty() {
        return;
    }
    if let Err(error) = prune_released(context, target, released).await {
        warn(
            context,
            &format!(
                "Pruning released model folders under {} failed: {error}",
                target.root.path
            ),
        );
    }
}

/// Appends a publication event unless it repeats: a failure while the newest publication event
/// of the artifact is a failure, or a skip with a reason already recorded for it.
fn note_publication(
    repo: &TrainingRepository,
    experiment_id: &str,
    artifact_id: &str,
    kind: &str,
    body: Value,
) -> Result<()> {
    let events = repo.experiment_events(experiment_id)?;
    let publications = events
        .iter()
        .filter(|event| {
            event.pointer("/body/artifact_id").and_then(Value::as_str) == Some(artifact_id)
                && event
                    .get("kind")
                    .and_then(Value::as_str)
                    .is_some_and(|kind| {
                        [PUBLISHED_EVENT, FAILED_EVENT, SKIPPED_EVENT].contains(&kind)
                    })
        })
        .collect::<Vec<_>>();
    let repeated = match kind {
        FAILED_EVENT => publications
            .last()
            .is_some_and(|event| event["kind"] == FAILED_EVENT),
        SKIPPED_EVENT => publications.iter().any(|event| {
            event["kind"] == SKIPPED_EVENT && event["body"]["reason"] == body["reason"]
        }),
        _ => false,
    };
    if repeated {
        return Ok(());
    }
    repo.record_experiment_event(experiment_id, kind, body, now_ms())?;
    Ok(())
}

async fn record_event(
    context: &mut ExecutionContext,
    repo: &TrainingRepository,
    experiment_id: &str,
    artifact_id: &str,
    kind: &str,
    body: Value,
) {
    let (experiment, artifact, event) = (
        experiment_id.to_owned(),
        artifact_id.to_owned(),
        kind.to_owned(),
    );
    let recorded = on_ledger(repo, move |repo| {
        note_publication(repo, &experiment, &artifact, &event, body)
    })
    .await;
    if let Err(error) = recorded {
        warn(
            context,
            &format!("Recording the {kind} event of experiment {experiment_id} failed: {error}"),
        );
    }
}

/// The run memo key and the held flight of a record's publication. A state change waits for
/// any publication of the same record in this process. A self-repair gets None, without
/// waiting and without marking the run memo, while one runs, and None once a self-repair
/// checked the record in this run.
async fn begin_publication(
    context: &ExecutionContext,
    repo: &TrainingRepository,
    kind: &str,
    scope: LedgerScope,
    id: &str,
    reason: PublishReason,
) -> Option<(String, Flight)> {
    let memo = format!("{kind}\n{scope:?}\n{id}");
    let key = flight_key(kind, repo, id);
    if reason == PublishReason::StateChange {
        return Some((memo, single_flight(key).await));
    }
    if already_checked(context, &memo).await {
        return None;
    }
    let flight = try_single_flight(key)?;
    if already_checked(context, &memo).await {
        return None;
    }
    Some((memo, flight))
}

/// A record's publication: the ledger it is read from and the model root it writes to.
struct Publication {
    repo: TrainingRepository,
    scope: LedgerScope,
    storage: ModelStorage,
    target: WriteTarget,
}

impl Publication {
    async fn new(
        context: &mut ExecutionContext,
        repo: &TrainingRepository,
        scope: LedgerScope,
        storage: ModelStorage,
    ) -> Result<Self> {
        Ok(Self {
            repo: repo.clone(),
            scope,
            storage,
            target: WriteTarget::for_storage(context, storage).await?,
        })
    }

    fn provenance(&self) -> ModelProvenance {
        provenance(ModelPublisher::Auto, self.scope, Some(self.storage))
    }
}

enum FolderOutcome {
    Written {
        reference: PublishedModelRef,
        written: FolderWrite,
    },
    Skipped(SkipReason),
}

/// Plans an artifact's folder from the ledger, adds its extractor and writes the folder below
/// the publication's root.
async fn publish_artifact_folder(
    context: &mut ExecutionContext,
    publication: &Publication,
    artifact: ModelArtifact,
    spec: PlanSpec,
    mirrors: &mut Vec<Mirrored>,
) -> Result<FolderOutcome> {
    let limit = spec.limit;
    let planned = on_ledger(&publication.repo, move |repo| {
        artifact_plan(repo, artifact, spec)
    })
    .await?;
    let mut plan = match planned {
        Planned::Ready(plan) => plan,
        Planned::Skip(reason) => return Ok(FolderOutcome::Skipped(reason)),
    };
    attach_dependency(
        context,
        &publication.repo,
        publication.scope,
        Some(publication.storage),
        &mut plan,
        limit,
    )
    .await?;
    let target = &publication.target;
    let folder = target.file(&artifact_dir(plan.artifact_id()));
    let written = write_model_folder(target, &folder, &plan, FolderMode::Model, mirrors).await?;
    Ok(FolderOutcome::Written {
        reference: plan.model_ref(&folder, Some(publication.storage)),
        written,
    })
}

/// Fails when a manifest that `folder` relies on is no longer stored, so the pointer at `path`
/// is not written.
async fn confirm_still_stored(
    target: &WriteTarget,
    folder: Option<&FolderWrite>,
    path: &FlowPath,
) -> Result<()> {
    for manifest in folder.iter().flat_map(|written| &written.manifests) {
        if target.head(manifest).await?.is_none() {
            return Err(anyhow!(
                "{} was pruned while this publication ran, so {} is not written; the next publication writes both",
                manifest.path,
                path.path
            ));
        }
    }
    Ok(())
}

/// Writes `pointer` unless its `folder` is unchanged and the published pointer equals it apart
/// from the publication time. The write holds the root's prune lock and first confirms that
/// the manifests the folder relies on are still stored, so it never names a folder pruned
/// meanwhile. Returns whether it was written.
async fn write_pointer<T: Serialize>(
    target: &WriteTarget,
    path: &FlowPath,
    pointer: &T,
    current: Option<&T>,
    folder: Option<&FolderWrite>,
    mirrors: &mut Vec<Mirrored>,
) -> Result<bool> {
    let folder_written = folder.is_some_and(|folder| folder.any);
    if !folder_written && current.is_some_and(|current| same_pointer(current, pointer)) {
        target
            .mirror_stored(path, None, MAXIMUM_POINTER_BYTES, mirrors)
            .await?;
        return Ok(false);
    }
    let _pruning = target.prune_lock().await;
    confirm_still_stored(target, folder, path).await?;
    target
        .write(path, Bytes::from(json::to_vec(pointer)?), mirrors)
        .await?;
    Ok(true)
}

enum Winner {
    Published {
        reference: PublishedModelRef,
        weights_e_tag: Option<String>,
    },
    Unchanged,
    Skipped(SkipReason),
}

fn experiment_pointer(
    record: &ExperimentRecord,
    reference: &PublishedModelRef,
    storage: ModelStorage,
) -> ExperimentPointer {
    ExperimentPointer {
        format_version: POINTER_FORMAT_VERSION,
        experiment_id: record.experiment.id.clone(),
        artifact_id: reference.artifact_id.clone(),
        sha256: reference.sha256.clone(),
        bytes: reference.bytes,
        goal_met: record.result.goal_met,
        storage,
        published_at_ms: now_ms(),
    }
}

/// Writes a standalone Auto Train winner: its folder, then `models/experiments/{id}.json`.
async fn publish_winner(
    context: &mut ExecutionContext,
    repo: &TrainingRepository,
    scope: LedgerScope,
    record: &ExperimentRecord,
    artifact: ModelArtifact,
    limits: PublishLimits,
    mirrors: &mut Vec<Mirrored>,
) -> Result<Winner> {
    let experiment = &record.experiment;
    let locators = experiment_source_locators(&experiment.request.context);
    let marker = experiment.request.context.get(LEDGER_SCOPE_KEY);
    let name = format!("Auto Train experiment {}", experiment.id);
    let Some(storage) =
        classified_storage(context, scope, marker, &locators, &name, "winner").await?
    else {
        return Ok(Winner::Skipped(SkipReason::SourceScopeUnclassifiable));
    };
    let publication = Publication::new(context, repo, scope, storage).await?;
    let mut provenance = publication.provenance();
    let metadata = experiment_metadata(record, &mut provenance);
    let spec = PlanSpec::published(metadata, provenance, limits.serving_weights_bytes);
    let (reference, written) =
        match publish_artifact_folder(context, &publication, artifact, spec, mirrors).await? {
            FolderOutcome::Written { reference, written } => (reference, written),
            FolderOutcome::Skipped(reason) => return Ok(Winner::Skipped(reason)),
        };
    let target = &publication.target;
    let pointer = experiment_pointer(record, &reference, storage);
    let pointer_path = target.file(&experiment_pointer_path(&experiment.id));
    let current = target
        .read_json::<ExperimentPointer>(&pointer_path, MAXIMUM_POINTER_BYTES)
        .await?;
    let pointer_written = write_pointer(
        target,
        &pointer_path,
        &pointer,
        current.as_ref(),
        Some(&written),
        mirrors,
    )
    .await?;
    Ok(if pointer_written {
        Winner::Published {
            reference,
            weights_e_tag: written.weights_e_tag,
        }
    } else {
        Winner::Unchanged
    })
}

/// The event a publication's outcome records, and that outcome.
async fn record_outcome(
    context: &mut ExecutionContext,
    repo: &TrainingRepository,
    experiment_id: &str,
    artifact_id: &str,
    winner: Result<Winner>,
) -> PublishOutcome {
    match winner {
        Ok(Winner::Published {
            reference,
            weights_e_tag,
        }) => {
            let body = json!({
                "artifact_id": artifact_id,
                "storage": reference.storage,
                "manifest": reference.manifest.path,
                "sha256": reference.sha256,
                "bytes": reference.bytes,
                "weights_etag": weights_e_tag,
            });
            record_event(
                context,
                repo,
                experiment_id,
                artifact_id,
                PUBLISHED_EVENT,
                body,
            )
            .await;
            PublishOutcome::Published(reference)
        }
        Ok(Winner::Unchanged) => PublishOutcome::Unchanged,
        Ok(Winner::Skipped(reason)) => {
            let body = json!({ "artifact_id": artifact_id, "reason": reason.code() });
            record_event(
                context,
                repo,
                experiment_id,
                artifact_id,
                SKIPPED_EVENT,
                body,
            )
            .await;
            PublishOutcome::Skipped {
                reason: reason.code().to_owned(),
            }
        }
        Err(error) => {
            let error = bounded(&error.to_string());
            let body = json!({ "artifact_id": artifact_id, "error": error });
            record_event(
                context,
                repo,
                experiment_id,
                artifact_id,
                FAILED_EVENT,
                body,
            )
            .await;
            PublishOutcome::Failed { error }
        }
    }
}

/// `publish_experiment` with the default serving cap.
pub(crate) async fn ensure_experiment_published(
    context: &mut ExecutionContext,
    repo: &TrainingRepository,
    scope: LedgerScope,
    experiment_id: &str,
    reason: PublishReason,
) -> PublishOutcome {
    publish_experiment(
        context,
        repo,
        scope,
        experiment_id,
        reason,
        PublishLimits::default(),
    )
    .await
}

/// Publishes a finished standalone Auto Train winner, whether or not it met its goals, and
/// records `model_published`, `model_publish_failed` or `model_publish_skipped`. Experiments
/// that are not final yet are left for a later call.
pub(crate) async fn publish_experiment(
    context: &mut ExecutionContext,
    repo: &TrainingRepository,
    scope: LedgerScope,
    experiment_id: &str,
    reason: PublishReason,
    limits: PublishLimits,
) -> PublishOutcome {
    let Some((memo, _flight)) =
        begin_publication(context, repo, "experiment", scope, experiment_id, reason).await
    else {
        return PublishOutcome::Unchanged;
    };
    let id = experiment_id.to_owned();
    let record = match on_ledger(repo, move |repo| load_experiment(repo, &id)).await {
        Ok(record) => record,
        Err(error) => {
            return PublishOutcome::Failed {
                error: bounded(&format!(
                    "Reading experiment {experiment_id} from the training ledger failed: {error}"
                )),
            };
        }
    };
    if record.result.final_test_metrics.is_empty() {
        return PublishOutcome::Unchanged;
    }
    mark_checked(context, memo).await;
    let stream = &record.experiment.request.stream.stream_id;
    if StreamKind::of_stream(stream) != Some(StreamKind::Auto) {
        return PublishOutcome::Skipped {
            reason: format!(
                "experiment {experiment_id} trains stream {stream}, not a standalone Auto Train stream, so only its learning project's champion is published"
            ),
        };
    }
    let Some(artifact) = record.result.model_bundle_ref.clone() else {
        return PublishOutcome::Unchanged;
    };
    let artifact_id = artifact.id.clone();
    let mut mirrors = Vec::new();
    let winner = publish_winner(
        context,
        repo,
        scope,
        &record,
        artifact,
        limits,
        &mut mirrors,
    )
    .await;
    mirror(context, mirrors).await;
    record_outcome(context, repo, experiment_id, &artifact_id, winner).await
}

struct ChampionRecord {
    project: LearningProject,
    deployment: Option<Deployment>,
    champion: Option<ModelArtifact>,
    cycle_id: Option<String>,
    experiment: Option<ExperimentRecord>,
}

/// The newest cycle of learning project `project_id` that produced `artifact_id`: its id and
/// the experiment that trained the artifact.
fn producing_cycle(
    repo: &TrainingRepository,
    project_id: &str,
    artifact_id: &str,
) -> Result<(Option<String>, Option<ExperimentRecord>)> {
    let cycle = repo
        .learning_cycles(project_id)?
        .into_iter()
        .rev()
        .find(|cycle| cycle.artifact_id.as_deref() == Some(artifact_id));
    let experiment = cycle
        .as_ref()
        .and_then(|cycle| cycle.experiment_id.as_deref())
        .map(|id| load_experiment(repo, id))
        .transpose()?;
    Ok((cycle.map(|cycle| cycle.id), experiment))
}

fn load_champion(repo: &TrainingRepository, project_id: &str) -> Result<ChampionRecord> {
    let project = repo.get_learning_project(project_id)?;
    let deployment = optional(repo.get_deployment(&project.request.deployment_id))?;
    let champion = project
        .champion_artifact_id
        .as_deref()
        .map(|id| repo.get_artifact(id))
        .transpose()?;
    let (cycle_id, experiment) = match &champion {
        Some(artifact) => producing_cycle(repo, project_id, &artifact.id)?,
        None => (None, None),
    };
    Ok(ChampionRecord {
        project,
        deployment,
        champion,
        cycle_id,
        experiment,
    })
}

/// The champion's metadata from the cycle experiment that trained it, or the project's task.
fn champion_metadata(record: &ChampionRecord, provenance: &mut ModelProvenance) -> ModelMetadata {
    provenance.learning_project_id = Some(record.project.id.clone());
    provenance.cycle_id = record.cycle_id.clone();
    provenance.deployment_id = Some(record.project.request.deployment_id.clone());
    provenance.deployment_generation = Some(
        record
            .deployment
            .as_ref()
            .map_or(0, |deployment| deployment.generation),
    );
    match &record.experiment {
        Some(experiment) => experiment_metadata(experiment, provenance),
        None => ModelMetadata {
            task: Some(record.project.request.goals.task.clone()),
            ..ModelMetadata::default()
        },
    }
}

fn champion_pointer(
    record: &ChampionRecord,
    storage: ModelStorage,
    previous: Option<(Option<&str>, &[String])>,
) -> ChampionPointer {
    let champion_id = record.champion.as_ref().map(|artifact| artifact.id.clone());
    ChampionPointer {
        format_version: POINTER_FORMAT_VERSION,
        project_id: record.project.id.clone(),
        deployment_id: record.project.request.deployment_id.clone(),
        deployment_generation: record
            .deployment
            .as_ref()
            .map_or(0, |deployment| deployment.generation),
        project_generation: record.project.generation,
        sha256: record
            .champion
            .as_ref()
            .map(|artifact| artifact.blob.sha256.clone()),
        paused: record.project.state == LearningState::Paused
            || record
                .deployment
                .as_ref()
                .is_some_and(|deployment| deployment.paused),
        cycle_id: record.cycle_id.clone(),
        retained: next_retained(previous, champion_id.as_deref()),
        artifact_id: champion_id,
        storage,
        published_at_ms: now_ms(),
    }
}

/// The champion's folder and its outcome: Unchanged without a folder when the project has no
/// champion. A skipped champion warns and is cleared from `pointer`, so readers route to the
/// teacher, and the ids of `previous` stay retained.
async fn champion_folder(
    context: &mut ExecutionContext,
    publication: &Publication,
    record: &ChampionRecord,
    pointer: &mut ChampionPointer,
    previous: Option<(Option<&str>, &[String])>,
    mirrors: &mut Vec<Mirrored>,
) -> Result<(PublishOutcome, Option<FolderWrite>)> {
    let Some(artifact) = record.champion.clone() else {
        return Ok((PublishOutcome::Unchanged, None));
    };
    let mut provenance = publication.provenance();
    let metadata = champion_metadata(record, &mut provenance);
    let spec = PlanSpec::published(metadata, provenance, MAXIMUM_SERVED_WEIGHTS_BYTES);
    let outcome = publish_artifact_folder(context, publication, artifact, spec, mirrors).await?;
    let reason = match outcome {
        FolderOutcome::Written { reference, written } => {
            return Ok((PublishOutcome::Published(reference), Some(written)));
        }
        FolderOutcome::Skipped(reason) => reason.code().to_owned(),
    };
    warn(
        context,
        &format!(
            "The champion of learning project {} is not published: {reason}",
            record.project.id
        ),
    );
    pointer.artifact_id = None;
    pointer.sha256 = None;
    pointer.retained = next_retained(previous, None);
    Ok((PublishOutcome::Skipped { reason }, None))
}

/// A folder's outcome once its pointer is settled: Published becomes Unchanged when the pointer
/// was not written either.
fn settled(outcome: PublishOutcome, pointer_written: bool) -> PublishOutcome {
    match outcome {
        PublishOutcome::Published(_) if !pointer_written => PublishOutcome::Unchanged,
        outcome => outcome,
    }
}

/// Writes the champion folder, when the project has a champion, then
/// `models/learning/{id}.json`, unless a pointer of greater order is already published. A
/// champion whose folder is skipped is published as no champion, so devices without the
/// training ledger route to the teacher.
async fn publish_champion(
    context: &mut ExecutionContext,
    repo: &TrainingRepository,
    scope: LedgerScope,
    project_id: &str,
    mirrors: &mut Vec<Mirrored>,
) -> Result<PublishOutcome> {
    let id = project_id.to_owned();
    let record = on_ledger(repo, move |repo| load_champion(repo, &id)).await?;
    let source = &record.project.request.source;
    let locators = learning_source_locators(source);
    let marker = source.get(LEDGER_SCOPE_KEY);
    let name = format!("Learning project {project_id}");
    let Some(storage) =
        classified_storage(context, scope, marker, &locators, &name, "champion").await?
    else {
        return Ok(PublishOutcome::Skipped {
            reason: SOURCE_SCOPE_UNCLASSIFIABLE.into(),
        });
    };
    let publication = Publication::new(context, repo, scope, storage).await?;
    let target = &publication.target;
    let pointer_path = target.file(&learning_pointer_path(project_id));
    let current = target
        .read_json::<ChampionPointer>(&pointer_path, MAXIMUM_POINTER_BYTES)
        .await?;
    let previous = current
        .as_ref()
        .map(|pointer| (pointer.artifact_id.as_deref(), pointer.retained.as_slice()));
    let mut pointer = champion_pointer(&record, storage, previous);
    if let Some(current) = &current
        && current.order() > pointer.order()
    {
        return Ok(PublishOutcome::Skipped {
            reason: format!(
                "the published champion pointer {} has order {:?}, newer than the ledger's {:?}",
                pointer_path.path,
                current.order(),
                pointer.order()
            ),
        });
    }
    let (outcome, folder) = champion_folder(
        context,
        &publication,
        &record,
        &mut pointer,
        previous,
        mirrors,
    )
    .await?;
    let written = write_pointer(
        target,
        &pointer_path,
        &pointer,
        current.as_ref(),
        folder.as_ref(),
        mirrors,
    )
    .await?;
    if written {
        let released = released_ids(previous, pointer.artifact_id.as_deref(), &pointer.retained);
        prune(context, target, released).await;
    }
    Ok(settled(outcome, written))
}

/// Publishes a learning project's champion pointer and, when it has a champion, the champion's
/// folder. Published names the champion folder; a pointer of a project without a champion
/// reports Unchanged even when it was written, and one whose champion folder is skipped
/// reports Skipped. Whatever was written reaches the Local mirror, even when a later step
/// fails.
pub(crate) async fn ensure_champion_published(
    context: &mut ExecutionContext,
    repo: &TrainingRepository,
    scope: LedgerScope,
    project_id: &str,
    reason: PublishReason,
) -> PublishOutcome {
    let Some((memo, _flight)) =
        begin_publication(context, repo, "learning", scope, project_id, reason).await
    else {
        return PublishOutcome::Unchanged;
    };
    let mut mirrors = Vec::new();
    let outcome = publish_champion(context, repo, scope, project_id, &mut mirrors)
        .await
        .unwrap_or_else(|error| PublishOutcome::Failed {
            error: bounded(&format!(
                "Publishing the champion of learning project {project_id} failed: {error}"
            )),
        });
    mirror(context, mirrors).await;
    mark_checked(context, memo).await;
    outcome
}

struct DeploymentRecord {
    artifact: ModelArtifact,
    owner: Option<(Option<Value>, Vec<Option<TableLocator>>)>,
    experiment: Option<ExperimentRecord>,
}

/// The active artifact, and for a minted stream the learning project or experiment that owns
/// it: its scope marker and source locators, and the experiment that trained the artifact, so
/// a deployment publishes the same metadata as the champion it serves.
fn load_deployment(repo: &TrainingRepository, deployment: &Deployment) -> Result<DeploymentRecord> {
    let artifact = repo.get_artifact(&deployment.active_artifact_id)?;
    let (owner, experiment) = match StreamKind::of_stream(&deployment.stream.stream_id) {
        Some(StreamKind::Learning) => {
            let project = repo
                .learning_projects(&deployment.stream.project_id)?
                .into_iter()
                .find(|project| {
                    project.request.deployment_id == deployment.id
                        || project.request.stream == deployment.stream
                });
            let (_, experiment) = match &project {
                Some(project) => producing_cycle(repo, &project.id, &artifact.id)?,
                None => (None, None),
            };
            let owner = project.map(|project| {
                (
                    project.request.source.get(LEDGER_SCOPE_KEY).cloned(),
                    learning_source_locators(&project.request.source),
                )
            });
            (owner, experiment)
        }
        Some(StreamKind::Auto) => {
            let experiment = repo
                .list_experiments(&deployment.stream)?
                .into_iter()
                .next();
            let record = experiment
                .as_ref()
                .map(|experiment| load_experiment(repo, &experiment.id))
                .transpose()?;
            let owner = experiment.map(|experiment| {
                (
                    experiment.request.context.get(LEDGER_SCOPE_KEY).cloned(),
                    experiment_source_locators(&experiment.request.context),
                )
            });
            (owner, record)
        }
        None => (None, None),
    };
    Ok(DeploymentRecord {
        artifact,
        owner,
        experiment,
    })
}

/// The root a deployment publishes to. A deployment of a minted stream follows the root of
/// the learning project or experiment that owns it; None while that owner cannot be
/// classified.
async fn deployment_storage(
    context: &ExecutionContext,
    scope: LedgerScope,
    deployment: &Deployment,
    record: &DeploymentRecord,
) -> Result<Option<ModelStorage>> {
    Ok(
        match (scope, StreamKind::of_stream(&deployment.stream.stream_id)) {
            (LedgerScope::User, _) => Some(ModelStorage::User),
            (LedgerScope::App, None) => Some(ModelStorage::App),
            (LedgerScope::App, Some(_)) => match &record.owner {
                Some((marker, locators)) => {
                    record_storage(context, scope, marker.as_ref(), locators).await?
                }
                None => None,
            },
        },
    )
}

/// The active model's folder plan input: the experiment's metadata when it trained that model.
fn deployment_spec(
    publication: &Publication,
    deployment: &Deployment,
    record: &DeploymentRecord,
) -> PlanSpec {
    let mut provenance = publication.provenance();
    provenance.deployment_id = Some(deployment.id.clone());
    provenance.deployment_generation = Some(deployment.generation);
    let metadata = match &record.experiment {
        Some(experiment)
            if experiment.result.model_artifact_id.as_deref()
                == Some(record.artifact.id.as_str()) =>
        {
            experiment_metadata(experiment, &mut provenance)
        }
        _ => ModelMetadata::default(),
    };
    PlanSpec::published(metadata, provenance, MAXIMUM_SERVED_WEIGHTS_BYTES)
}

/// Skipped when the published pointer has a greater generation than the ledger's deployment.
fn newer_deployment(
    current: Option<&DeploymentPointer>,
    deployment: &Deployment,
    path: &FlowPath,
) -> Option<PublishOutcome> {
    let current = current.filter(|current| current.generation > deployment.generation)?;
    Some(PublishOutcome::Skipped {
        reason: format!(
            "the published deployment pointer {} has generation {}, newer than the ledger's {}",
            path.path, current.generation, deployment.generation
        ),
    })
}

fn deployment_pointer(
    deployment: &Deployment,
    sha256: String,
    storage: ModelStorage,
    previous: Option<(Option<&str>, &[String])>,
) -> DeploymentPointer {
    DeploymentPointer {
        format_version: POINTER_FORMAT_VERSION,
        deployment_id: deployment.id.clone(),
        generation: deployment.generation,
        active_artifact_id: deployment.active_artifact_id.clone(),
        paused: deployment.paused,
        stream: deployment.stream.clone(),
        sha256,
        deployment: deployment.clone(),
        retained: next_retained(previous, Some(deployment.active_artifact_id.as_str())),
        storage,
        published_at_ms: now_ms(),
    }
}

/// The active model's folder and its outcome. A skipped folder pauses `pointer` and the
/// deployment it carries, so readers route to the teacher and never load the unpublished model.
async fn deployment_folder(
    context: &mut ExecutionContext,
    publication: &Publication,
    record: &DeploymentRecord,
    pointer: &mut DeploymentPointer,
    mirrors: &mut Vec<Mirrored>,
) -> Result<(PublishOutcome, Option<FolderWrite>)> {
    let spec = deployment_spec(publication, &pointer.deployment, record);
    let artifact = record.artifact.clone();
    let outcome = publish_artifact_folder(context, publication, artifact, spec, mirrors).await?;
    let skipped = match outcome {
        FolderOutcome::Written { reference, written } => {
            return Ok((PublishOutcome::Published(reference), Some(written)));
        }
        FolderOutcome::Skipped(reason) => reason,
    };
    pointer.paused = true;
    pointer.deployment.paused = true;
    let reason = format!(
        "{}: the active model {} of deployment {} is not published, so its published pointer routes to the teacher",
        skipped.code(),
        pointer.active_artifact_id,
        pointer.deployment_id
    );
    Ok((PublishOutcome::Skipped { reason }, None))
}

/// Skipped when `pointer` would exceed the size every reader of it accepts.
fn oversized_pointer<T: Serialize>(pointer: &T, path: &FlowPath) -> Result<Option<PublishOutcome>> {
    let bytes = json::to_vec(pointer)?.len() as u64;
    if bytes <= MAXIMUM_POINTER_BYTES {
        return Ok(None);
    }
    Ok(Some(PublishOutcome::Skipped {
        reason: format!(
            "{POINTER_TOO_LARGE}: the model pointer {} would hold {bytes} bytes, above the {MAXIMUM_POINTER_BYTES} bytes its readers accept",
            path.path
        ),
    }))
}

/// Writes the active artifact's folder, then `models/deployments/{id}.json`, unless a pointer
/// of greater generation is already published or the pointer would be too large to read. A
/// deployment whose active folder is skipped is published paused, so devices without the
/// training ledger route to the teacher. A deployment is never published to app storage while
/// `deployment_storage` cannot classify it.
async fn publish_deployment(
    context: &mut ExecutionContext,
    repo: &TrainingRepository,
    scope: LedgerScope,
    deployment: &Deployment,
    mirrors: &mut Vec<Mirrored>,
) -> Result<PublishOutcome> {
    let loaded = deployment.clone();
    let record = on_ledger(repo, move |repo| load_deployment(repo, &loaded)).await?;
    let Some(storage) = deployment_storage(context, scope, deployment, &record).await? else {
        return Ok(PublishOutcome::Skipped {
            reason: format!(
                "{SOURCE_SCOPE_UNCLASSIFIABLE}: deployment {} serves stream {}, whose learning project or experiment predates ledger scopes and reads source tables that match neither this project's database nor yours",
                deployment.id, deployment.stream.stream_id
            ),
        });
    };
    let publication = Publication::new(context, repo, scope, storage).await?;
    let target = &publication.target;
    let pointer_path = target.file(&deployment_pointer_path(&deployment.id));
    let current = target
        .read_json::<DeploymentPointer>(&pointer_path, MAXIMUM_POINTER_BYTES)
        .await?;
    if let Some(skipped) = newer_deployment(current.as_ref(), deployment, &pointer_path) {
        return Ok(skipped);
    }
    let previous = current.as_ref().map(|pointer| {
        (
            Some(pointer.active_artifact_id.as_str()),
            pointer.retained.as_slice(),
        )
    });
    let sha256 = record.artifact.blob.sha256.clone();
    let mut pointer = deployment_pointer(deployment, sha256, storage, previous);
    if let Some(skipped) = oversized_pointer(&pointer, &pointer_path)? {
        return Ok(skipped);
    }
    let (outcome, folder) =
        deployment_folder(context, &publication, &record, &mut pointer, mirrors).await?;
    let written = write_pointer(
        target,
        &pointer_path,
        &pointer,
        current.as_ref(),
        folder.as_ref(),
        mirrors,
    )
    .await?;
    if written {
        let active = Some(deployment.active_artifact_id.as_str());
        let released = released_ids(previous, active, &pointer.retained);
        prune(context, target, released).await;
    }
    Ok(settled(outcome, written))
}

/// Publishes a lifecycle deployment's active model and its deployment pointer. Skipped
/// carries a readable reason for the caller's warning. Whatever was written reaches the Local
/// mirror, even when a later step fails.
pub(crate) async fn ensure_deployment_published(
    context: &mut ExecutionContext,
    repo: &TrainingRepository,
    scope: LedgerScope,
    deployment: &Deployment,
    reason: PublishReason,
) -> PublishOutcome {
    let Some((memo, _flight)) =
        begin_publication(context, repo, "deployment", scope, &deployment.id, reason).await
    else {
        return PublishOutcome::Unchanged;
    };
    let mut mirrors = Vec::new();
    let outcome = publish_deployment(context, repo, scope, deployment, &mut mirrors)
        .await
        .unwrap_or_else(|error| PublishOutcome::Failed {
            error: bounded(&format!(
                "Publishing deployment {} failed: {error}",
                deployment.id
            )),
        });
    mirror(context, mirrors).await;
    mark_checked(context, memo).await;
    outcome
}

async fn publish_source(
    context: &mut ExecutionContext,
    repo: &TrainingRepository,
    scope: LedgerScope,
    source: &PretrainedSource,
    mirrors: &mut Vec<Mirrored>,
) -> Result<PublishOutcome> {
    let storage = ModelStorage::from(scope);
    let provenance = provenance(ModelPublisher::Auto, scope, Some(storage));
    let record = source.clone();
    let plan = match on_ledger(repo, move |repo| {
        source_plan(repo, record, provenance, MAXIMUM_SERVED_WEIGHTS_BYTES)
    })
    .await?
    {
        Planned::Ready(plan) => plan,
        Planned::Skip(reason) => {
            return Ok(PublishOutcome::Skipped {
                reason: format!(
                    "{}: pretrained source {} is not published",
                    reason.code(),
                    source.id
                ),
            });
        }
    };
    let target = WriteTarget::for_storage(context, storage).await?;
    let folder = target.file(&artifact_dir(plan.artifact_id()));
    let written = write_model_folder(
        &target,
        &folder,
        &plan,
        FolderMode::ContentAddressed,
        mirrors,
    )
    .await?;
    Ok(if written.any {
        PublishOutcome::Published(plan.model_ref(&folder, Some(storage)))
    } else {
        PublishOutcome::Unchanged
    })
}

/// Content-addressed: an existing folder with the same blob digest is left as it is. Whatever
/// was written reaches the Local mirror, even when a later step fails.
pub(crate) async fn publish_pretrained_source(
    context: &mut ExecutionContext,
    repo: &TrainingRepository,
    scope: LedgerScope,
    source: &PretrainedSource,
) -> PublishOutcome {
    let _flight = single_flight(flight_key("source", repo, &source.id)).await;
    let mut mirrors = Vec::new();
    let outcome = publish_source(context, repo, scope, source, &mut mirrors)
        .await
        .unwrap_or_else(|error| PublishOutcome::Failed {
            error: bounded(&format!(
                "Publishing pretrained source {} failed: {error}",
                source.id
            )),
        });
    mirror(context, mirrors).await;
    outcome
}

fn exported(planned: Planned, model: &LedgerModel) -> Result<FolderPlan> {
    match planned {
        Planned::Ready(plan) => Ok(plan),
        Planned::Skip(reason) => Err(anyhow!(
            "{model:?} cannot be exported: {}",
            reason.export_text()
        )),
    }
}

/// An experiment's model with its metadata; `provenance` notes the experiment.
fn experiment_model(
    repo: &TrainingRepository,
    id: &str,
    provenance: &mut ModelProvenance,
) -> Result<(ModelArtifact, ModelMetadata)> {
    let record = load_experiment(repo, id)?;
    let artifact = record
        .result
        .model_bundle_ref
        .clone()
        .ok_or_else(|| anyhow!("Experiment {id} has no model to export yet"))?;
    Ok((artifact, experiment_metadata(&record, provenance)))
}

fn export_plan(
    repo: &TrainingRepository,
    model: &LedgerModel,
    mut provenance: ModelProvenance,
) -> Result<Planned> {
    let (artifact, metadata) = match model {
        LedgerModel::Experiment { experiment_id } => {
            experiment_model(repo, experiment_id, &mut provenance)?
        }
        LedgerModel::LearningChampion { project_id } => {
            let record = load_champion(repo, project_id)?;
            let artifact = record.champion.clone().ok_or_else(|| {
                anyhow!("Learning project {project_id} has no champion model to export yet")
            })?;
            (artifact, champion_metadata(&record, &mut provenance))
        }
        LedgerModel::Artifact { artifact_id } => {
            let artifact = repo.get_artifact(artifact_id).map_err(|error| {
                anyhow!("Reading artifact {artifact_id} from the training ledger failed: {error}")
            })?;
            (artifact, ModelMetadata::default())
        }
        LedgerModel::PretrainedSource { source_id } => {
            let source = repo.get_pretrained_source(source_id).map_err(|error| {
                anyhow!(
                    "Reading pretrained source {source_id} from the training ledger failed: {error}"
                )
            })?;
            return source_plan(repo, source, provenance, MAXIMUM_EXPORT_WEIGHTS_BYTES);
        }
    };
    let spec = PlanSpec::published(metadata, provenance, MAXIMUM_EXPORT_WEIGHTS_BYTES);
    artifact_plan(repo, artifact, spec)
}

/// Writes the model folder, with `dependencies/{source}` for its extractor, to `destination`.
pub(crate) async fn export_model_folder(
    context: &mut ExecutionContext,
    repo: &TrainingRepository,
    scope: LedgerScope,
    model: &LedgerModel,
    destination: &FlowPath,
) -> Result<PublishedModelRef> {
    let provenance = provenance(ModelPublisher::Export, scope, None);
    let requested = model.clone();
    let planned = on_ledger(repo, move |repo| export_plan(repo, &requested, provenance)).await?;
    let mut plan = exported(planned, model)?;
    attach_dependency(
        context,
        repo,
        scope,
        None,
        &mut plan,
        MAXIMUM_EXPORT_WEIGHTS_BYTES,
    )
    .await?;
    let target = WriteTarget::new(context, destination.clone()).await?;
    let mut mirrors = Vec::new();
    write_model_folder(
        &target,
        destination,
        &plan,
        FolderMode::Export,
        &mut mirrors,
    )
    .await?;
    mirror(context, mirrors).await;
    Ok(plan.model_ref(destination, None))
}

fn same_location(left: &FlowPath, right: &FlowPath) -> bool {
    left.store_ref == right.store_ref && left.object_path() == right.object_path()
}

/// The preprocessing file name `{manifest stem}.preprocessing.json`, and the weights file name
/// the manifest records when weights and manifest share a folder.
fn auto_export_names(weights: &FlowPath, manifest: &FlowPath) -> Result<(String, Option<String>)> {
    if same_location(weights, manifest) {
        return Err(anyhow!(
            "Weights and manifest must have different destinations, but both are {}",
            manifest.path
        ));
    }
    let manifest_name = display_file_name(&manifest.object_path())
        .ok_or_else(|| anyhow!("The manifest destination {} names no file", manifest.path))?;
    let weights_name = display_file_name(&weights.object_path())
        .ok_or_else(|| anyhow!("The weights destination {} names no file", weights.path))?;
    let stem = manifest_name
        .rsplit_once('.')
        .map_or(manifest_name.as_str(), |(stem, _)| stem);
    let preprocessing_file = format!("{stem}.preprocessing.json");
    let weights_file = (same_location(&folder_of(weights), &folder_of(manifest))
        && is_plain_file_name(&weights_name))
    .then_some(weights_name);
    Ok((preprocessing_file, weights_file))
}

/// Writes the weights, then the preprocessing file and extractor folder beside the manifest,
/// then the manifest.
async fn write_auto_export(
    context: &mut ExecutionContext,
    plan: &FolderPlan,
    weights: &FlowPath,
    manifest: &FlowPath,
) -> Result<()> {
    let manifest_folder = folder_of(manifest);
    let weights_target = WriteTarget::new(context, weights.clone()).await?;
    let manifest_target = WriteTarget::new(context, manifest.clone()).await?;
    let mut mirrors = Vec::new();
    weights_target
        .write(weights, plan.weights.clone(), &mut mirrors)
        .await?;
    if let (Some(bytes), Some(reference)) = (&plan.preprocessing, &plan.manifest.preprocessing) {
        let path = child(&manifest_folder, &reference.file);
        manifest_target
            .write(&path, bytes.clone(), &mut mirrors)
            .await?;
    }
    if let Some(dependency) = &plan.dependency {
        let folder = child(&manifest_folder, &dependency_dir(dependency.artifact_id()));
        write_model_folder(
            &manifest_target,
            &folder,
            dependency,
            FolderMode::Export,
            &mut mirrors,
        )
        .await?;
    }
    manifest_target
        .write(
            manifest,
            Bytes::from(json::to_vec(&plan.manifest)?),
            &mut mirrors,
        )
        .await?;
    mirror(context, mirrors).await;
    Ok(())
}

/// Export Auto Model's files: the weights at `weights`, the preprocessing as
/// `{manifest stem}.preprocessing.json` beside the manifest and a v2 manifest at `manifest`.
/// The manifest names the weights file only when both share a folder.
pub(crate) async fn export_auto_model_files(
    context: &mut ExecutionContext,
    repo: &TrainingRepository,
    scope: LedgerScope,
    experiment_id: &str,
    weights: &FlowPath,
    manifest: &FlowPath,
) -> Result<PublishedModelRef> {
    let (preprocessing_file, weights_file) = auto_export_names(weights, manifest)?;
    let id = experiment_id.to_owned();
    let planned = on_ledger(repo, move |repo| {
        let mut provenance = provenance(ModelPublisher::Export, scope, None);
        let (artifact, metadata) = experiment_model(repo, &id, &mut provenance)?;
        let spec = PlanSpec {
            metadata,
            provenance,
            preprocessing_file,
            weights_file,
            limit: MAXIMUM_EXPORT_WEIGHTS_BYTES,
        };
        artifact_plan(repo, artifact, spec)
    })
    .await?;
    let model = LedgerModel::Experiment {
        experiment_id: experiment_id.to_owned(),
    };
    let mut plan = exported(planned, &model)?;
    attach_dependency(
        context,
        repo,
        scope,
        None,
        &mut plan,
        MAXIMUM_EXPORT_WEIGHTS_BYTES,
    )
    .await?;
    write_auto_export(context, &plan, weights, manifest).await?;
    Ok(PublishedModelRef {
        artifact_id: plan.artifact_id().to_owned(),
        manifest: manifest.clone(),
        weights: weights.clone(),
        sha256: plan.manifest.weights.sha256.clone(),
        bytes: plan.manifest.weights.bytes,
        storage: None,
    })
}
