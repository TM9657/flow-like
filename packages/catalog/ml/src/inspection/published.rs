//! Trained models published as FlowPath files under `models/` of the app storage or of the
//! executing user's directory, so runtimes without a training ledger can run them.

use super::auto_training::AnomalyDecisions;
use super::lifecycle::LedgerScope;
use flow_like_catalog_core::FlowPath;
use flow_like_ml_native::feature_engineering::FeatureStreamState;
use flow_like_ml_runtime::{
    Deployment, EvaluationTask, ModelArtifact, PretrainedSource, StreamKey,
};
use flow_like_types::{
    Value,
    storage_paths::{PUBLISHED_MODELS_DIR, storage_path_segment},
};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

#[cfg(feature = "inference")]
use super::lifecycle::{
    CachedPredictor, CatalogLoadedPredictor, FeatureExtractorRef, InspectionContext,
};
#[cfg(feature = "inference")]
use flow_like::flow::execution::context::ExecutionContext;
#[cfg(feature = "inference")]
use flow_like_ml_core::{ComputeConfig, TensorData};
#[cfg(feature = "inference")]
use flow_like_ml_runtime::engines::FeatureExtractor;
#[cfg(feature = "inference")]
use flow_like_storage::{
    files::store::FlowLikeStore,
    join_object_path,
    object_store::{self, ObjectMeta, ObjectStoreExt},
};
#[cfg(feature = "inference")]
use flow_like_types::{Result, anyhow, json};
#[cfg(feature = "inference")]
use std::{
    collections::VecDeque,
    sync::{Arc, LazyLock, Mutex, MutexGuard, PoisonError},
};

pub const INFERENCE_UNAVAILABLE: &str = "Running trained models needs a build with ML inference. The desktop app, device placements and the AWS, Kubernetes, local and Docker Compose executors include it; Azure and GCP executors and the mobile apps do not in this version.";

pub fn inference_unavailable() -> flow_like_types::Error {
    flow_like_types::Error::msg(INFERENCE_UNAVAILABLE)
}

pub const MANIFEST_FILE: &str = "manifest.json";
pub const WEIGHTS_FILE: &str = "weights.bin";
pub const PREPROCESSING_FILE: &str = "preprocessing.json";
pub const MANIFEST_FORMAT_VERSION: u32 = 2;
pub const POINTER_FORMAT_VERSION: u32 = 1;
pub const PREPROCESSING_FORMAT_VERSION: u32 = 1;
pub const MAXIMUM_MANIFEST_BYTES: u64 = 1024 * 1024;
pub const MAXIMUM_POINTER_BYTES: u64 = 64 * 1024;
pub const MAXIMUM_SERVED_WEIGHTS_BYTES: u64 = 512 * 1024 * 1024;
pub const RETAINED_PREVIOUS_ARTIFACTS: usize = 3;
/// Replaces `artifact.manifest.preprocessing_manifest` in a published manifest; its value is
/// the preprocessing file name, where the moved manifest lives.
pub const PREPROCESSING_MARKER_KEY: &str = "$published";

/// Size cap of a preprocessing file whose feature pipeline may read `maximum_bytes`.
pub fn preprocessing_cap(maximum_bytes: u64) -> u64 {
    maximum_bytes.saturating_mul(2).saturating_add(1024 * 1024)
}

fn seg(id: &str) -> String {
    storage_path_segment(id, "model")
}

/// Folder of a published model or pretrained source, relative to a model root.
pub fn artifact_dir(id: &str) -> String {
    format!("{PUBLISHED_MODELS_DIR}/artifacts/{}", seg(id))
}

/// Pointer to a standalone Auto Train winner, relative to a model root.
pub fn experiment_pointer_path(id: &str) -> String {
    format!("{PUBLISHED_MODELS_DIR}/experiments/{}.json", seg(id))
}

/// Pointer to a learning project's champion, relative to a model root.
pub fn learning_pointer_path(id: &str) -> String {
    format!("{PUBLISHED_MODELS_DIR}/learning/{}.json", seg(id))
}

/// Pointer to a lifecycle deployment's active model, relative to a model root.
pub fn deployment_pointer_path(id: &str) -> String {
    format!("{PUBLISHED_MODELS_DIR}/deployments/{}.json", seg(id))
}

/// Dependency folder inside an exported model folder.
pub fn dependency_dir(source_id: &str) -> String {
    format!("dependencies/{}", seg(source_id))
}

/// Which model root holds published files: the app's storage or the executing user's
/// directory for models trained on private data.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum ModelStorage {
    #[default]
    App,
    User,
}

impl From<LedgerScope> for ModelStorage {
    fn from(scope: LedgerScope) -> Self {
        match scope {
            LedgerScope::App => Self::App,
            LedgerScope::User => Self::User,
        }
    }
}

impl From<ModelStorage> for LedgerScope {
    fn from(storage: ModelStorage) -> Self {
        match storage {
            ModelStorage::App => Self::App,
            ModelStorage::User => Self::User,
        }
    }
}

/// A published model to run. Without `storage`, the app storage is searched before the
/// executing user's directory.
#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum PublishedModelSource {
    /// A manifest file, for example one written by Export Model or Export Auto Model.
    Files {
        manifest: FlowPath,
        /// The weights file when the manifest does not name one in its own folder.
        #[serde(default)]
        weights: Option<FlowPath>,
    },
    /// The winner of a standalone Auto Train experiment.
    Experiment {
        experiment_id: String,
        #[serde(default)]
        storage: Option<ModelStorage>,
    },
    /// The current champion of a learning project.
    LearningChampion {
        project_id: String,
        #[serde(default)]
        storage: Option<ModelStorage>,
    },
    /// The active model of a lifecycle deployment.
    Deployment {
        deployment_id: String,
        #[serde(default)]
        storage: Option<ModelStorage>,
    },
    /// A model artifact or pretrained source by id.
    Artifact {
        artifact_id: String,
        #[serde(default)]
        storage: Option<ModelStorage>,
    },
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum ModelPublisher {
    /// Written by automatic publication; retention may prune it.
    Auto,
    /// Written by Export Model or Export Auto Model; never pruned.
    Export,
}

/// `manifest.json` of a published model folder. Exactly one of `artifact` and
/// `pretrained_source` is set.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct PublishedModelManifest {
    pub format_version: u32,
    #[serde(default)]
    pub artifact: Option<ModelArtifact>,
    #[serde(default)]
    pub pretrained_source: Option<PretrainedSource>,
    pub weights: PublishedFile,
    #[serde(default)]
    pub preprocessing: Option<PublishedPreprocessingRef>,
    #[serde(default)]
    pub task: Option<EvaluationTask>,
    /// Anomaly models only: the decision rule the final audit measured.
    #[serde(default)]
    pub calibration: Option<AnomalyCalibration>,
    #[serde(default)]
    pub goal_met: Option<bool>,
    #[serde(default)]
    pub validation_metrics: BTreeMap<String, f64>,
    #[serde(default)]
    pub final_test_metrics: BTreeMap<String, f64>,
    #[serde(default)]
    pub source_table_versions: Vec<Value>,
    #[serde(default)]
    pub dependencies: PublishedDependencies,
    #[serde(default)]
    pub provenance: Option<ModelProvenance>,
}

impl PublishedModelManifest {
    /// The model artifact's id, or the pretrained source's id for a source folder.
    pub fn artifact_id(&self) -> &str {
        self.artifact
            .as_ref()
            .map(|artifact| artifact.id.as_str())
            .or_else(|| {
                self.pretrained_source
                    .as_ref()
                    .map(|source| source.id.as_str())
            })
            .unwrap_or_default()
    }

    pub fn engine(&self) -> &str {
        self.artifact
            .as_ref()
            .map(|artifact| &artifact.manifest)
            .or_else(|| {
                self.pretrained_source
                    .as_ref()
                    .map(|source| &source.manifest)
            })
            .and_then(|manifest| manifest.get("engine"))
            .and_then(Value::as_str)
            .unwrap_or_default()
    }
}

/// A file of a published model. `file` is a plain name in the manifest's folder; it is None
/// only when Export Auto Model wrote the weights into another folder.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct PublishedFile {
    pub file: Option<String>,
    pub sha256: String,
    pub bytes: u64,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct PublishedPreprocessingRef {
    pub file: String,
    pub sha256: String,
    pub bytes: u64,
    /// Feature pipeline read limit the file's cap was derived from.
    pub maximum_bytes: u64,
}

/// `preprocessing.json` of a published model folder.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct PublishedPreprocessing {
    pub format_version: u32,
    #[serde(default)]
    pub preprocessing: Option<Value>,
    /// The artifact manifest's `preprocessing_manifest`, restored before loading.
    #[serde(default)]
    pub artifact_preprocessing_manifest: Option<Value>,
}

/// probability = score / (score + scale); an anomaly at or above `threshold`.
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct AnomalyCalibration {
    pub scale: f64,
    pub threshold: f64,
}

#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct PublishedDependencies {
    /// The model that computed this model's embeddings.
    #[serde(default)]
    pub feature_extractor: Option<PublishedDependency>,
}

/// Resolved under `models/artifacts/{source_id}` of the same root or `dependencies/{source_id}`
/// beside the manifest; both the id and the blob digest must match.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct PublishedDependency {
    pub source_id: String,
    pub blob_sha256: String,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct ModelProvenance {
    pub publisher: ModelPublisher,
    pub ledger: LedgerScope,
    #[serde(default)]
    pub storage: Option<ModelStorage>,
    /// Published as another model's feature-extractor dependency, not as a model of its own.
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub dependency: bool,
    #[serde(default)]
    pub experiment_id: Option<String>,
    #[serde(default)]
    pub selected_trial_id: Option<String>,
    #[serde(default)]
    pub learning_project_id: Option<String>,
    #[serde(default)]
    pub cycle_id: Option<String>,
    #[serde(default)]
    pub deployment_id: Option<String>,
    #[serde(default)]
    pub deployment_generation: Option<i64>,
    pub published_at_ms: i64,
}

/// `models/experiments/{experiment}.json`: a standalone Auto Train winner.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct ExperimentPointer {
    pub format_version: u32,
    pub experiment_id: String,
    pub artifact_id: String,
    pub sha256: String,
    pub bytes: u64,
    pub goal_met: bool,
    pub storage: ModelStorage,
    pub published_at_ms: i64,
}

/// `models/learning/{project}.json`. The Teacher decides while `artifact_id` is None or
/// `paused` is set.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct ChampionPointer {
    pub format_version: u32,
    pub project_id: String,
    pub deployment_id: String,
    pub deployment_generation: i64,
    pub project_generation: i64,
    pub artifact_id: Option<String>,
    pub sha256: Option<String>,
    pub paused: bool,
    pub cycle_id: Option<String>,
    /// Previous champion artifacts, newest first.
    #[serde(default)]
    pub retained: Vec<String>,
    pub storage: ModelStorage,
    pub published_at_ms: i64,
}

impl ChampionPointer {
    /// A pointer replaces another only with a strictly greater order.
    pub fn order(&self) -> (i64, i64) {
        (self.deployment_generation, self.project_generation)
    }
}

/// `models/deployments/{deployment}.json`; ordered by `generation`.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct DeploymentPointer {
    pub format_version: u32,
    pub deployment_id: String,
    pub generation: i64,
    pub active_artifact_id: String,
    pub paused: bool,
    pub stream: StreamKey,
    pub sha256: String,
    pub deployment: Deployment,
    /// Previous active artifacts, newest first.
    #[serde(default)]
    pub retained: Vec<String>,
    pub storage: ModelStorage,
    pub published_at_ms: i64,
}

/// Where a model was published or exported.
#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema)]
pub struct PublishedModelRef {
    pub artifact_id: String,
    pub manifest: FlowPath,
    pub weights: FlowPath,
    pub sha256: String,
    pub bytes: u64,
    #[serde(default)]
    pub storage: Option<ModelStorage>,
}

#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema)]
pub struct PublishedModelInfo {
    pub artifact_id: String,
    pub engine: String,
    pub sha256: String,
    pub bytes: u64,
    pub storage: Option<ModelStorage>,
    pub goal_met: Option<bool>,
    pub manifest: FlowPath,
}

#[derive(Clone, Serialize, Deserialize, JsonSchema)]
pub struct PublishedModelPredictions {
    pub model: PublishedModelInfo,
    pub predictions: Vec<Value>,
    pub feature_state: Option<FeatureStreamState>,
    /// Anomaly models only: the calibrated decision for each row.
    pub anomaly: Option<AnomalyDecisions>,
}

/// A published model whose manifest was read and checked.
#[cfg(feature = "inference")]
pub(crate) struct ResolvedModel {
    pub(crate) manifest: PublishedModelManifest,
    pub(crate) manifest_path: FlowPath,
    pub(crate) weights_path: FlowPath,
    pub(crate) storage: Option<ModelStorage>,
    /// The table preprocessing a version 1 export keeps inside its manifest.
    pub(crate) inline_preprocessing: Option<Value>,
}

#[cfg(feature = "inference")]
pub(crate) struct TableRowsInput {
    pub(crate) rows: Vec<Value>,
    pub(crate) feature_state: Option<FeatureStreamState>,
    pub(crate) stateful_features: bool,
    pub(crate) feature_extractor: Option<FeatureExtractorRef>,
}

#[cfg(feature = "inference")]
pub(crate) struct TableRowsOutput {
    pub(crate) predictions: Vec<Value>,
    pub(crate) feature_state: Option<FeatureStreamState>,
    pub(crate) anomaly: Option<AnomalyDecisions>,
}

#[cfg(feature = "inference")]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum PointerKind {
    Learning,
    Deployment,
}

/// Rows one Predict Published Model request may carry, as Predict Auto Model allows.
#[cfg(feature = "inference")]
pub(crate) const MAXIMUM_PREDICTION_ROWS: usize = 100_000;
#[cfg(feature = "inference")]
const PREPROCESSING_CACHE_KEY: &str = "published-preprocessing-v1";
#[cfg(feature = "inference")]
const GENERATION_PROBE_KEY: &str = "published-generations-v1";
#[cfg(feature = "inference")]
const MAXIMUM_CACHED_DOCUMENTS: usize = 4;
#[cfg(feature = "inference")]
const MAXIMUM_GENERATION_PROBES: usize = 256;
#[cfg(feature = "inference")]
const MAXIMUM_CACHED_PREDICTORS: usize = 8;
#[cfg(feature = "inference")]
const MAXIMUM_VERIFIED_OBJECTS: usize = 256;
#[cfg(feature = "inference")]
const DEFAULT_PREDICTOR_BUDGET: u64 = 4 * 1024 * 1024 * 1024;

#[cfg(feature = "inference")]
fn storage_name(storage: ModelStorage) -> &'static str {
    match storage {
        ModelStorage::App => "app",
        ModelStorage::User => "user",
    }
}

/// `apps/{app}/storage` for App and `users/{sub}/apps/{app}` for User. Everything below it is
/// built by the path helpers from constants and sanitized ids.
#[cfg(feature = "inference")]
pub(crate) async fn model_root(
    context: &mut ExecutionContext,
    storage: ModelStorage,
) -> Result<FlowPath> {
    let root = match storage {
        ModelStorage::App => FlowPath::from_storage_dir(context, false).await,
        ModelStorage::User => FlowPath::from_user_dir(context, false).await,
    };
    root.map_err(|error| {
        anyhow!(
            "The {} model root is unavailable in this run: {error}",
            storage_name(storage)
        )
    })
}

/// `relative` below `base`, in the same stores.
#[cfg(feature = "inference")]
pub(crate) fn child(base: &FlowPath, relative: &str) -> FlowPath {
    FlowPath {
        path: join_object_path(&base.object_path(), relative)
            .as_ref()
            .to_string(),
        store_ref: base.store_ref.clone(),
        cache_store_ref: base.cache_store_ref.clone(),
    }
}

/// The folder holding `path`.
#[cfg(feature = "inference")]
pub(crate) fn folder_of(path: &FlowPath) -> FlowPath {
    let key = path.object_path();
    FlowPath {
        path: key
            .as_ref()
            .rsplit_once('/')
            .map_or("", |(folder, _)| folder)
            .to_string(),
        store_ref: path.store_ref.clone(),
        cache_store_ref: path.cache_store_ref.clone(),
    }
}

/// A file name a manifest may hold: one plain name in the manifest's own folder.
#[cfg(feature = "inference")]
pub(crate) fn is_plain_file_name(name: &str) -> bool {
    !name.is_empty() && name != "." && name != ".." && !name.contains(['/', '\\'])
}

#[cfg(feature = "inference")]
fn plain_file_name(name: &str, field: &str, manifest: &FlowPath) -> Result<()> {
    if !is_plain_file_name(name) {
        return Err(anyhow!(
            "The model manifest {} names '{name}' in {field}; it must be a plain file name in the manifest's folder",
            manifest.path
        ));
    }
    Ok(())
}

/// The roots a source searches: the given one, or the app's storage before the user's
/// directory.
#[cfg(feature = "inference")]
fn searched_roots(storage: Option<ModelStorage>) -> Vec<ModelStorage> {
    storage.map_or_else(|| vec![ModelStorage::App, ModelStorage::User], |storage| vec![storage])
}

/// The root of `storage`. While searching both roots, a root this run cannot build is noted
/// in `tried` and skipped; a requested root that cannot be built is an error.
#[cfg(feature = "inference")]
async fn searched_root(
    context: &mut ExecutionContext,
    storage: ModelStorage,
    requested: bool,
    tried: &mut Vec<String>,
) -> Result<Option<FlowPath>> {
    match model_root(context, storage).await {
        Ok(root) => Ok(Some(root)),
        Err(error) if !requested => {
            tried.push(error.to_string());
            Ok(None)
        }
        Err(error) => Err(error),
    }
}

#[cfg(feature = "inference")]
fn not_found(error: &object_store::Error) -> bool {
    matches!(error, object_store::Error::NotFound { .. })
}

/// A file located in its store, with the metadata read before its body.
#[cfg(feature = "inference")]
struct StoredFile {
    store: FlowLikeStore,
    meta: ObjectMeta,
}

/// HEADs `path` in its own store and refuses it above `maximum` before any byte is read.
/// None when nothing is stored there.
#[cfg(feature = "inference")]
async fn stat_capped(
    context: &mut ExecutionContext,
    path: &FlowPath,
    maximum: u64,
    what: &str,
) -> Result<Option<StoredFile>> {
    let store = path.to_store(context).await?;
    let location = path.object_path();
    let meta = match store.as_generic().head(&location).await {
        Ok(meta) => meta,
        Err(error) if not_found(&error) => return Ok(None),
        Err(error) => {
            return Err(anyhow!(
                "Reading the metadata of {what} {} from store '{}' failed: {error}",
                path.path,
                path.store_ref
            ));
        }
    };
    if meta.size > maximum {
        return Err(anyhow!(
            "The {what} {} is {} bytes, above its limit of {maximum} bytes",
            path.path,
            meta.size
        ));
    }
    Ok(Some(StoredFile { store, meta }))
}

/// Reads through the path's cache layer only when that layer is Local, so desktop runs use
/// their mirror while executors and online placements read the store and write no copy.
#[cfg(feature = "inference")]
async fn fetch(context: &mut ExecutionContext, path: &FlowPath) -> Result<Vec<u8>> {
    let through_local_cache = matches!(
        path.to_cache_layer(context).await?.as_deref(),
        Some(FlowLikeStore::Local(_))
    );
    path.get(context, !through_local_cache).await
}

/// The bytes of a file of at most `maximum` bytes, checked by HEAD first; None when it does
/// not exist.
#[cfg(feature = "inference")]
async fn read_capped(
    context: &mut ExecutionContext,
    path: &FlowPath,
    maximum: u64,
    what: &str,
) -> Result<Option<Vec<u8>>> {
    if stat_capped(context, path, maximum, what).await?.is_none() {
        return Ok(None);
    }
    let bytes = fetch(context, path).await?;
    if bytes.len() as u64 > maximum {
        return Err(anyhow!(
            "The {what} {} grew to {} bytes while it was read, above its limit of {maximum} bytes",
            path.path,
            bytes.len()
        ));
    }
    Ok(Some(bytes))
}

/// Reads a file whose size and sha256 a manifest records; `file` holds its HEAD.
#[cfg(feature = "inference")]
async fn read_verified(
    context: &mut ExecutionContext,
    path: &FlowPath,
    file: &StoredFile,
    bytes: u64,
    sha256: &str,
    what: &str,
) -> Result<Vec<u8>> {
    if file.meta.size != bytes {
        return Err(anyhow!(
            "The {what} {} is {} bytes, but its manifest records {bytes} bytes",
            path.path,
            file.meta.size
        ));
    }
    let content = fetch(context, path).await?;
    let (expected, shown, what) = (sha256.to_owned(), path.path.clone(), what.to_owned());
    tokio::task::spawn_blocking(move || -> Result<Vec<u8>> {
        if content.len() as u64 != bytes || flow_like_ml_core::content_digest(&content) != expected
        {
            return Err(anyhow!(
                "The {what} {shown} does not hold the {bytes} bytes with sha256 {expected} its manifest records"
            ));
        }
        Ok(content)
    })
    .await?
}

/// The store, object and version a requester reached, for proving it can read an object.
#[cfg(feature = "inference")]
fn object_identity(path: &FlowPath, file: &StoredFile) -> String {
    let version = file.meta.e_tag.clone().unwrap_or_else(|| {
        format!("{} bytes modified {}", file.meta.size, file.meta.last_modified)
    });
    format!(
        "{}\n{}\n{}\n{version}",
        path.store_ref,
        file.store.as_generic(),
        file.meta.location
    )
}

/// Per-run values in the run cache, oldest first.
#[cfg(feature = "inference")]
pub(crate) struct RunMap<T>(Arc<Mutex<VecDeque<(String, T)>>>);

#[cfg(feature = "inference")]
impl<T: Clone> RunMap<T> {
    pub(crate) fn get(&self, key: &str) -> Option<T> {
        self.entries()
            .iter()
            .find(|(known, _)| known == key)
            .map(|(_, value)| value.clone())
    }

    /// Replaces `key`, dropping the oldest entries beyond `maximum`.
    pub(crate) fn insert(&self, key: String, value: T, maximum: usize) {
        let mut entries = self.entries();
        entries.retain(|(known, _)| *known != key);
        while entries.len() >= maximum.max(1) {
            entries.pop_front();
        }
        entries.push_back((key, value));
    }

    fn entries(&self) -> MutexGuard<'_, VecDeque<(String, T)>> {
        self.0.lock().unwrap_or_else(PoisonError::into_inner)
    }
}

#[cfg(feature = "inference")]
impl<T: Send + 'static> flow_like_types::Cacheable for RunMap<T> {
    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
    fn as_any_mut(&mut self) -> &mut dyn std::any::Any {
        self
    }
}

/// The run's map under `key`, created on first use.
#[cfg(feature = "inference")]
pub(crate) async fn run_map<T: Clone + Send + 'static>(
    context: &ExecutionContext,
    key: &str,
) -> Result<RunMap<T>> {
    let mut cache = context.cache.write().await;
    let entry = cache
        .entry(key.to_string())
        .or_insert_with(|| Arc::new(RunMap::<T>(Arc::default())));
    entry
        .as_any()
        .downcast_ref::<RunMap<T>>()
        .map(|map| RunMap(map.0.clone()))
        .ok_or_else(|| anyhow!("Run cache entry {key} holds a value of another type"))
}

/// Version 1 is the manifest Export Auto Model wrote before published folders existed.
#[cfg(feature = "inference")]
#[derive(Deserialize)]
struct ExportManifestV1 {
    artifact: ModelArtifact,
    #[serde(default)]
    preprocessing: Value,
    #[serde(default)]
    source_table_versions: Vec<Value>,
    #[serde(default)]
    validation_metrics: BTreeMap<String, f64>,
    #[serde(default)]
    final_test_metrics: BTreeMap<String, f64>,
    #[serde(default)]
    goal_met: Option<bool>,
}

#[cfg(feature = "inference")]
struct ParsedManifest {
    manifest: PublishedModelManifest,
    inline_preprocessing: Option<Value>,
}

/// Version 2, or version 1 when the manifest came from a files source.
#[cfg(feature = "inference")]
fn parse_manifest(bytes: &[u8], path: &FlowPath, files_source: bool) -> Result<ParsedManifest> {
    let value: Value = json::from_slice(bytes)
        .map_err(|error| anyhow!("The model manifest {} is not valid JSON: {error}", path.path))?;
    let version = value.get("format_version").and_then(Value::as_u64);
    if version == Some(1) && files_source {
        let legacy: ExportManifestV1 = json::from_value(value).map_err(|error| {
            anyhow!(
                "The version 1 model manifest {} is invalid: {error}",
                path.path
            )
        })?;
        let weights = PublishedFile {
            file: None,
            sha256: legacy.artifact.blob.sha256.clone(),
            bytes: legacy.artifact.blob.bytes,
        };
        let manifest = PublishedModelManifest {
            format_version: 1,
            artifact: Some(legacy.artifact),
            pretrained_source: None,
            weights,
            preprocessing: None,
            task: None,
            calibration: None,
            goal_met: legacy.goal_met,
            validation_metrics: legacy.validation_metrics,
            final_test_metrics: legacy.final_test_metrics,
            source_table_versions: legacy.source_table_versions,
            dependencies: PublishedDependencies::default(),
            provenance: None,
        };
        return Ok(ParsedManifest {
            manifest,
            inline_preprocessing: Some(legacy.preprocessing).filter(|value| !value.is_null()),
        });
    }
    if version != Some(u64::from(MANIFEST_FORMAT_VERSION)) {
        let hint = if version == Some(1) {
            "; read version 1 exports through a files source with their weights file"
        } else {
            ""
        };
        return Err(anyhow!(
            "The model manifest {} has format_version {}, but this source reads version {MANIFEST_FORMAT_VERSION}{hint}",
            path.path,
            version.map_or_else(|| "none".to_string(), |version| version.to_string())
        ));
    }
    let manifest: PublishedModelManifest = json::from_value(value)
        .map_err(|error| anyhow!("The model manifest {} is invalid: {error}", path.path))?;
    validate_manifest(&manifest, path)?;
    Ok(ParsedManifest {
        manifest,
        inline_preprocessing: None,
    })
}

#[cfg(feature = "inference")]
fn validate_manifest(manifest: &PublishedModelManifest, path: &FlowPath) -> Result<()> {
    let (record, blob) = match (&manifest.artifact, &manifest.pretrained_source) {
        (Some(artifact), None) => ("artifact", &artifact.blob),
        (None, Some(source)) => ("pretrained source", &source.blob),
        _ => {
            return Err(anyhow!(
                "The model manifest {} must describe exactly one artifact or pretrained source",
                path.path
            ));
        }
    };
    if manifest.weights.sha256 != blob.sha256 || manifest.weights.bytes != blob.bytes {
        return Err(anyhow!(
            "The model manifest {} records weights {} ({} bytes), but its {record} has {} ({} bytes)",
            path.path,
            manifest.weights.sha256,
            manifest.weights.bytes,
            blob.sha256,
            blob.bytes
        ));
    }
    if let Some(file) = &manifest.weights.file {
        plain_file_name(file, "weights.file", path)?;
    }
    if let Some(reference) = &manifest.preprocessing {
        plain_file_name(&reference.file, "preprocessing.file", path)?;
        let cap = preprocessing_cap(reference.maximum_bytes);
        if reference.bytes > cap {
            return Err(anyhow!(
                "The model manifest {} lists a {}-byte preprocessing file, above its limit of {cap} bytes",
                path.path,
                reference.bytes
            ));
        }
    }
    Ok(())
}

#[cfg(feature = "inference")]
fn check_pointer_version(format_version: u32, path: &FlowPath) -> Result<()> {
    if format_version != POINTER_FORMAT_VERSION {
        return Err(anyhow!(
            "The model pointer {} has format_version {format_version}, but this runtime reads version {POINTER_FORMAT_VERSION}",
            path.path
        ));
    }
    Ok(())
}

/// A pointer of at most 64 KiB; None when it does not exist.
#[cfg(feature = "inference")]
async fn read_pointer<T: serde::de::DeserializeOwned>(
    context: &mut ExecutionContext,
    path: &FlowPath,
) -> Result<Option<T>> {
    let Some(bytes) = read_capped(context, path, MAXIMUM_POINTER_BYTES, "model pointer").await?
    else {
        return Ok(None);
    };
    json::from_slice(&bytes)
        .map(Some)
        .map_err(|error| anyhow!("The model pointer {} is invalid: {error}", path.path))
}

/// The first pointer at `relative` in the searched roots, with the root it was found in.
#[cfg(feature = "inference")]
async fn find_pointer<T: serde::de::DeserializeOwned>(
    context: &mut ExecutionContext,
    storage: Option<ModelStorage>,
    relative: &str,
    tried: &mut Vec<String>,
) -> Result<Option<(T, ModelStorage, FlowPath, FlowPath)>> {
    for candidate in searched_roots(storage) {
        let Some(root) = searched_root(context, candidate, storage.is_some(), tried).await? else {
            continue;
        };
        let path = child(&root, relative);
        tried.push(path.path.clone());
        if let Some(pointer) = read_pointer::<T>(context, &path).await? {
            return Ok(Some((pointer, candidate, root, path)));
        }
    }
    Ok(None)
}

/// The artifact folder of `artifact_id` under `root`; None when its manifest is missing.
#[cfg(feature = "inference")]
async fn resolve_artifact(
    context: &mut ExecutionContext,
    root: &FlowPath,
    storage: ModelStorage,
    artifact_id: &str,
    pointed_sha256: Option<&str>,
    tried: &mut Vec<String>,
) -> Result<Option<ResolvedModel>> {
    let folder = child(root, &artifact_dir(artifact_id));
    let manifest_path = child(&folder, MANIFEST_FILE);
    tried.push(manifest_path.path.clone());
    let Some(bytes) = read_capped(
        context,
        &manifest_path,
        MAXIMUM_MANIFEST_BYTES,
        "model manifest",
    )
    .await?
    else {
        return Ok(None);
    };
    let parsed = parse_manifest(&bytes, &manifest_path, false)?;
    if parsed.manifest.artifact_id() != artifact_id {
        return Err(anyhow!(
            "The model manifest {} describes {}, not {artifact_id}",
            manifest_path.path,
            parsed.manifest.artifact_id()
        ));
    }
    if let Some(pointed) = pointed_sha256
        && pointed != parsed.manifest.weights.sha256
    {
        return Err(anyhow!(
            "A pointer names weights {pointed} for model {artifact_id}, but {} records {}",
            manifest_path.path,
            parsed.manifest.weights.sha256
        ));
    }
    let file = parsed.manifest.weights.file.as_deref().ok_or_else(|| {
        anyhow!(
            "The model manifest {} names no weights file in its folder",
            manifest_path.path
        )
    })?;
    Ok(Some(ResolvedModel {
        weights_path: child(&folder, file),
        manifest: parsed.manifest,
        manifest_path,
        storage: Some(storage),
        inline_preprocessing: None,
    }))
}

#[cfg(feature = "inference")]
async fn resolve_pointed(
    context: &mut ExecutionContext,
    root: &FlowPath,
    storage: ModelStorage,
    artifact_id: &str,
    sha256: Option<&str>,
    tried: &mut Vec<String>,
) -> Result<ResolvedModel> {
    resolve_artifact(context, root, storage, artifact_id, sha256, tried)
        .await?
        .ok_or_else(|| {
            anyhow!(
                "Published model {artifact_id} is missing; looked for {}",
                tried.join(", ")
            )
        })
}

#[cfg(feature = "inference")]
async fn resolve_files(
    context: &mut ExecutionContext,
    manifest: &FlowPath,
    weights: Option<&FlowPath>,
) -> Result<ResolvedModel> {
    let bytes = read_capped(context, manifest, MAXIMUM_MANIFEST_BYTES, "model manifest")
        .await?
        .ok_or_else(|| anyhow!("The model manifest {} does not exist", manifest.path))?;
    let parsed = parse_manifest(&bytes, manifest, true)?;
    let weights_path = match (weights, parsed.manifest.weights.file.as_deref()) {
        (Some(weights), _) => weights.clone(),
        (None, Some(file)) => child(&folder_of(manifest), file),
        (None, None) => {
            return Err(anyhow!(
                "The model manifest {} names no weights file in its folder; pass the weights file with the files source",
                manifest.path
            ));
        }
    };
    Ok(ResolvedModel {
        manifest: parsed.manifest,
        manifest_path: manifest.clone(),
        weights_path,
        storage: None,
        inline_preprocessing: parsed.inline_preprocessing,
    })
}

/// Reads a published model's manifest: a files source directly, the other sources through
/// their pointer or artifact folder in `storage`, or the app's storage before the user's
/// directory when `storage` is None. Errors list every path tried.
#[cfg(feature = "inference")]
pub(crate) async fn resolve_model(
    context: &mut ExecutionContext,
    source: &PublishedModelSource,
) -> Result<ResolvedModel> {
    let mut tried = Vec::new();
    match source {
        PublishedModelSource::Files { manifest, weights } => {
            resolve_files(context, manifest, weights.as_ref()).await
        }
        PublishedModelSource::Artifact {
            artifact_id,
            storage,
        } => {
            for candidate in searched_roots(*storage) {
                let Some(root) =
                    searched_root(context, candidate, storage.is_some(), &mut tried).await?
                else {
                    continue;
                };
                if let Some(model) =
                    resolve_artifact(context, &root, candidate, artifact_id, None, &mut tried)
                        .await?
                {
                    return Ok(model);
                }
            }
            Err(anyhow!(
                "Model {artifact_id} is not published; looked for {}",
                tried.join(", ")
            ))
        }
        PublishedModelSource::Experiment {
            experiment_id,
            storage,
        } => {
            let relative = experiment_pointer_path(experiment_id);
            let Some((pointer, found, root, path)) =
                find_pointer::<ExperimentPointer>(context, *storage, &relative, &mut tried).await?
            else {
                return Err(anyhow!(
                    "Auto Train experiment {experiment_id} has no published winner; looked for {}",
                    tried.join(", ")
                ));
            };
            check_pointer_version(pointer.format_version, &path)?;
            if pointer.experiment_id != *experiment_id {
                return Err(anyhow!(
                    "The model pointer {} belongs to experiment {}, not {experiment_id}",
                    path.path,
                    pointer.experiment_id
                ));
            }
            resolve_pointed(
                context,
                &root,
                found,
                &pointer.artifact_id,
                Some(&pointer.sha256),
                &mut tried,
            )
            .await
        }
        PublishedModelSource::LearningChampion {
            project_id,
            storage,
        } => {
            let relative = learning_pointer_path(project_id);
            let Some((pointer, found, root, path)) =
                find_pointer::<ChampionPointer>(context, *storage, &relative, &mut tried).await?
            else {
                return Err(anyhow!(
                    "Learning project {project_id} has no published champion; looked for {}",
                    tried.join(", ")
                ));
            };
            check_pointer_version(pointer.format_version, &path)?;
            if pointer.project_id != *project_id {
                return Err(anyhow!(
                    "The model pointer {} belongs to learning project {}, not {project_id}",
                    path.path,
                    pointer.project_id
                ));
            }
            let artifact_id = pointer.artifact_id.as_deref().ok_or_else(|| {
                anyhow!(
                    "Learning project {project_id} has no champion model yet, so the teacher decides ({})",
                    path.path
                )
            })?;
            resolve_pointed(
                context,
                &root,
                found,
                artifact_id,
                pointer.sha256.as_deref(),
                &mut tried,
            )
            .await
        }
        PublishedModelSource::Deployment {
            deployment_id,
            storage,
        } => {
            let relative = deployment_pointer_path(deployment_id);
            let Some((pointer, found, root, path)) =
                find_pointer::<DeploymentPointer>(context, *storage, &relative, &mut tried)
                    .await?
            else {
                return Err(anyhow!(
                    "Deployment {deployment_id} has no published model; looked for {}",
                    tried.join(", ")
                ));
            };
            check_pointer_version(pointer.format_version, &path)?;
            if pointer.deployment_id != *deployment_id {
                return Err(anyhow!(
                    "The model pointer {} belongs to deployment {}, not {deployment_id}",
                    path.path,
                    pointer.deployment_id
                ));
            }
            resolve_pointed(
                context,
                &root,
                found,
                &pointer.active_artifact_id,
                Some(&pointer.sha256),
                &mut tried,
            )
            .await
        }
    }
}

/// The memory budget a load is planned against when the request names none.
#[cfg(feature = "inference")]
fn host_load_budget() -> u64 {
    ComputeConfig::default().for_host().memory_limit_bytes
}

/// Largest weights file a request may load: a quarter of its memory, at most 512 MiB.
#[cfg(feature = "inference")]
pub(crate) fn served_weights_limit(compute: &ComputeConfig) -> u64 {
    (compute.memory_limit_bytes / 4).min(MAXIMUM_SERVED_WEIGHTS_BYTES)
}

/// HEADs the weights and refuses a size other than the manifest's or above `maximum`.
#[cfg(feature = "inference")]
async fn stat_weights(
    context: &mut ExecutionContext,
    model: &ResolvedModel,
    maximum: u64,
) -> Result<StoredFile> {
    let bytes = model.manifest.weights.bytes;
    if bytes > maximum {
        return Err(anyhow!(
            "Model {} has {bytes} bytes of weights, above the limit of {maximum} bytes for this request",
            model.manifest.artifact_id()
        ));
    }
    let file = stat_capped(context, &model.weights_path, maximum, "weights file")
        .await?
        .ok_or_else(|| {
            anyhow!(
                "The weights file {} of model {} is missing",
                model.weights_path.path,
                model.manifest.artifact_id()
            )
        })?;
    if file.meta.size != bytes {
        return Err(anyhow!(
            "The weights file {} is {} bytes, but its manifest records {bytes} bytes",
            model.weights_path.path,
            file.meta.size
        ));
    }
    Ok(file)
}

/// The weights of a published model of at most `maximum` bytes, checked against the size and
/// sha256 its manifest records.
#[cfg(feature = "inference")]
pub(crate) async fn published_weights(
    context: &mut ExecutionContext,
    model: &ResolvedModel,
    maximum: u64,
) -> Result<Vec<u8>> {
    let file = stat_weights(context, model, maximum).await?;
    read_verified(
        context,
        &model.weights_path,
        &file,
        model.manifest.weights.bytes,
        &model.manifest.weights.sha256,
        "weights file",
    )
    .await
}

/// The preprocessing file a manifest lists, read once per run per object and digest.
#[cfg(feature = "inference")]
async fn preprocessing_document(
    context: &mut ExecutionContext,
    model: &ResolvedModel,
) -> Result<Option<Arc<PublishedPreprocessing>>> {
    let Some(reference) = model.manifest.preprocessing.clone() else {
        return Ok(None);
    };
    let path = child(&folder_of(&model.manifest_path), &reference.file);
    let cap = preprocessing_cap(reference.maximum_bytes).min(host_load_budget() / 4);
    if reference.bytes > cap {
        return Err(anyhow!(
            "The preprocessing file {} is {} bytes, above its limit of {cap} bytes",
            path.path,
            reference.bytes
        ));
    }
    let file = stat_capped(context, &path, cap, "preprocessing file")
        .await?
        .ok_or_else(|| {
            anyhow!(
                "The preprocessing file {} that {} lists is missing",
                path.path,
                model.manifest_path.path
            )
        })?;
    let key = format!("{}\n{}", object_identity(&path, &file), reference.sha256);
    let documents = run_map::<Arc<PublishedPreprocessing>>(context, PREPROCESSING_CACHE_KEY).await?;
    if let Some(document) = documents.get(&key) {
        return Ok(Some(document));
    }
    let bytes = read_verified(
        context,
        &path,
        &file,
        reference.bytes,
        &reference.sha256,
        "preprocessing file",
    )
    .await?;
    let document: PublishedPreprocessing = json::from_slice(&bytes)
        .map_err(|error| anyhow!("The preprocessing file {} is invalid: {error}", path.path))?;
    if document.format_version != PREPROCESSING_FORMAT_VERSION {
        return Err(anyhow!(
            "The preprocessing file {} has format_version {}, but this runtime reads version {PREPROCESSING_FORMAT_VERSION}",
            path.path,
            document.format_version
        ));
    }
    let document = Arc::new(document);
    documents.insert(key, document.clone(), MAXIMUM_CACHED_DOCUMENTS);
    Ok(Some(document))
}

/// The preprocessing file a published artifact manifest's marker names.
#[cfg(feature = "inference")]
fn preprocessing_marker(manifest: &Value) -> Option<String> {
    let marker = manifest.get("preprocessing_manifest")?.as_object()?;
    if marker.len() != 1 {
        return None;
    }
    marker
        .get(PREPROCESSING_MARKER_KEY)?
        .as_str()
        .map(str::to_owned)
}

/// The ledger's artifact record: a published manifest moves the artifact's preprocessing
/// manifest into the preprocessing file and leaves a marker, which this reverses.
#[cfg(feature = "inference")]
pub(crate) async fn restore_artifact(
    context: &mut ExecutionContext,
    model: &ResolvedModel,
    mut artifact: ModelArtifact,
) -> Result<ModelArtifact> {
    let Some(file) = preprocessing_marker(&artifact.manifest) else {
        return Ok(artifact);
    };
    if model
        .manifest
        .preprocessing
        .as_ref()
        .is_none_or(|reference| reference.file != file)
    {
        return Err(anyhow!(
            "Model {} moved its preprocessing manifest to {file}, but {} does not list that file",
            artifact.id,
            model.manifest_path.path
        ));
    }
    let document = preprocessing_document(context, model).await?.ok_or_else(|| {
        anyhow!(
            "The preprocessing file {file} of model {} is missing",
            artifact.id
        )
    })?;
    let restored = document
        .artifact_preprocessing_manifest
        .clone()
        .or_else(|| document.preprocessing.clone())
        .ok_or_else(|| {
            anyhow!(
                "The preprocessing file {file} of model {} holds no preprocessing manifest",
                artifact.id
            )
        })?;
    artifact.manifest["preprocessing_manifest"] = restored;
    Ok(artifact)
}

/// Loaded predictors shared by every run in this process.
#[cfg(feature = "inference")]
struct CachedEntry {
    key: String,
    predictor: CachedPredictor,
    reserved: u64,
}

/// At most eight predictors, oldest first. A cached predictor is served only to a requester
/// whose own HEAD of its weights object matches an object verified to hold the same sha256,
/// so a crafted manifest cannot borrow a model its requester could not read.
#[cfg(feature = "inference")]
#[derive(Default)]
struct PredictorCache {
    entries: VecDeque<CachedEntry>,
    verified: VecDeque<(String, String)>,
}

#[cfg(feature = "inference")]
impl PredictorCache {
    fn touch(&mut self, key: &str) -> Option<CachedPredictor> {
        let index = self.entries.iter().position(|entry| entry.key == key)?;
        let entry = self.entries.remove(index)?;
        let predictor = entry.predictor.clone();
        self.entries.push_back(entry);
        Some(predictor)
    }

    fn is_verified(&self, identity: &str, sha256: &str) -> bool {
        self.verified
            .iter()
            .any(|(object, digest)| object == identity && digest == sha256)
    }

    /// The predictor for `key` when `identity` was verified to hold `sha256`.
    fn verified_hit(&mut self, key: &str, identity: &str, sha256: &str) -> Option<CachedPredictor> {
        if !self.is_verified(identity, sha256) {
            return None;
        }
        self.touch(key)
    }

    /// Records that `identity` holds `sha256` and returns any predictor cached for `key`.
    fn verify(&mut self, identity: String, sha256: String, key: &str) -> Option<CachedPredictor> {
        self.verified.retain(|(object, _)| *object != identity);
        while self.verified.len() >= MAXIMUM_VERIFIED_OBJECTS {
            self.verified.pop_front();
        }
        self.verified.push_back((identity, sha256));
        self.touch(key)
    }

    /// Keeps the predictor when its reservation fits `budget`, evicting the oldest entries
    /// until at most eight remain and their reservations fit.
    fn insert(&mut self, key: String, predictor: CachedPredictor, reserved: u64, budget: u64) {
        if reserved > budget {
            return;
        }
        self.entries.retain(|entry| entry.key != key);
        while !self.entries.is_empty()
            && (self.entries.len() >= MAXIMUM_CACHED_PREDICTORS
                || self
                    .entries
                    .iter()
                    .fold(reserved, |total, entry| total.saturating_add(entry.reserved))
                    > budget)
        {
            self.entries.pop_front();
        }
        self.entries.push_back(CachedEntry {
            key,
            predictor,
            reserved,
        });
    }
}

#[cfg(feature = "inference")]
static PREDICTORS: LazyLock<Mutex<PredictorCache>> = LazyLock::new(Default::default);

#[cfg(feature = "inference")]
fn predictors() -> MutexGuard<'static, PredictorCache> {
    PREDICTORS.lock().unwrap_or_else(PoisonError::into_inner)
}

/// Half of the detected container memory, or 4 GiB when none is detected.
#[cfg(feature = "inference")]
fn predictor_budget() -> u64 {
    flow_like_ml_core::container_memory_limit_bytes()
        .map_or(DEFAULT_PREDICTOR_BUDGET, |bytes| bytes / 2)
}

/// The cache key of a predictor: its weights, its artifact manifest and the compute it was
/// loaded for. The manifest digest keeps models that share weights apart.
#[cfg(feature = "inference")]
fn predictor_key(sha256: &str, artifact: &ModelArtifact, compute: &ComputeConfig) -> Result<String> {
    let manifest = flow_like_ml_core::content_digest(&json::to_vec(&artifact.manifest)?);
    Ok(flow_like_ml_core::content_digest(&json::to_vec(&(
        sha256, manifest, compute,
    ))?))
}

/// The model's predictor, shared across runs while it stays cached. The request's compute is
/// clamped to the host first; the weights must fit a quarter of that memory, at most 512 MiB,
/// and are checked by HEAD before they are read and by sha256 before they are decoded.
#[cfg(feature = "inference")]
pub(crate) async fn published_predictor(
    context: &mut ExecutionContext,
    model: &ResolvedModel,
    compute: &ComputeConfig,
) -> Result<CachedPredictor> {
    let compute = compute.for_host();
    compute.validate()?;
    let artifact = model.manifest.artifact.clone().ok_or_else(|| {
        anyhow!(
            "{} describes pretrained source {}, not a trained model; run it with Extract Published Image Features or register it with Import Model",
            model.manifest_path.path,
            model.manifest.artifact_id()
        )
    })?;
    let bytes = model.manifest.weights.bytes;
    let reserved = bytes
        .checked_mul(4)
        .ok_or_else(|| anyhow!("The memory estimate of model {} overflows", artifact.id))?;
    if reserved >= compute.memory_limit_bytes {
        return Err(anyhow!(
            "Model {} needs about {reserved} bytes to load, above the inference memory budget of {} bytes",
            artifact.id,
            compute.memory_limit_bytes
        ));
    }
    let file = stat_weights(context, model, served_weights_limit(&compute)).await?;
    let artifact = restore_artifact(context, model, artifact).await?;
    let key = predictor_key(&model.manifest.weights.sha256, &artifact, &compute)?;
    let identity = object_identity(&model.weights_path, &file);
    let sha256 = model.manifest.weights.sha256.clone();
    let cached = predictors().verified_hit(&key, &identity, &sha256);
    if let Some(predictor) = cached {
        return Ok(predictor);
    }
    let content = read_verified(
        context,
        &model.weights_path,
        &file,
        bytes,
        &sha256,
        "weights file",
    )
    .await?;
    let cached = predictors().verify(identity, sha256, &key);
    if let Some(predictor) = cached {
        return Ok(predictor);
    }
    let loaded = tokio::task::spawn_blocking(move || {
        CatalogLoadedPredictor::from_parts(artifact, &content, &compute)
    })
    .await??;
    let predictor: CachedPredictor = Arc::new(loaded);
    predictors().insert(key, predictor.clone(), reserved, predictor_budget());
    Ok(predictor)
}

/// The table preprocessing of a published model: a version 1 export's inline value or the
/// `preprocessing` of its preprocessing file. None for models whose rows are tensors.
#[cfg(feature = "inference")]
pub(crate) async fn published_preprocessing(
    context: &mut ExecutionContext,
    model: &ResolvedModel,
) -> Result<Option<Value>> {
    if let Some(inline) = &model.inline_preprocessing {
        return Ok(Some(inline.clone()));
    }
    Ok(preprocessing_document(context, model)
        .await?
        .and_then(|document| document.preprocessing.clone()))
}

/// The manifest's stream must match the inspection's stream id and specification version.
/// The project string is not compared, because published files leave their ledger.
#[cfg(feature = "inference")]
pub(crate) fn check_stream(
    manifest: &PublishedModelManifest,
    inspection: &InspectionContext,
) -> Result<()> {
    let artifact = manifest.artifact.as_ref().ok_or_else(|| {
        anyhow!(
            "Published model {} is a pretrained source, not a model trained on an inspection stream",
            manifest.artifact_id()
        )
    })?;
    let version = inspection.inspection_version()?;
    if artifact.stream.stream_id != inspection.stream_id
        || artifact.stream.inspection_version != version
    {
        return Err(anyhow!(
            "Published model {} was trained on stream {} with specification {}, not on stream {} with specification {version}",
            artifact.id,
            artifact.stream.stream_id,
            artifact.stream.inspection_version,
            inspection.stream_id
        ));
    }
    Ok(())
}

/// Rows of a model without table preprocessing are tensors; a row with exactly the trained
/// input shape gets a batch dimension. Rows may name their extractor inside the tensor value.
#[cfg(feature = "inference")]
fn tensor_rows(
    artifact: &ModelArtifact,
    rows: &[Value],
    explicit: Option<&FeatureExtractorRef>,
) -> Result<(Vec<TensorData>, Option<FeatureExtractor>)> {
    let input_shape: Vec<usize> = artifact
        .manifest
        .get("input_shape")
        .cloned()
        .and_then(|shape| json::from_value(shape).ok())
        .unwrap_or_default();
    let mut named = explicit
        .map(FeatureExtractorRef::to_runtime)
        .transpose()?
        .into_iter()
        .collect::<Vec<_>>();
    let mut inputs = Vec::with_capacity(rows.len());
    for (index, row) in rows.iter().enumerate() {
        if let Some(carried) = row
            .get("feature_extractor")
            .filter(|value| !value.is_null())
        {
            named.push(super::lifecycle::runtime_extractor(carried)?);
        }
        let mut tensor: TensorData = json::from_value(row.clone()).map_err(|error| {
            anyhow!(
                "Row {index} is not a tensor with shape and values, which model {} expects without table preprocessing: {error}",
                artifact.id
            )
        })?;
        if tensor.shape == input_shape {
            tensor.shape.insert(0, 1);
        }
        inputs.push(tensor);
    }
    let mut named = named.into_iter();
    let first = named.next();
    if let Some(first) = &first
        && let Some(other) = named.find(|extractor| extractor != first)
    {
        return Err(anyhow!(
            "Embeddings come from different feature extractors ({} and {}); predict each extractor's rows separately",
            first.source_id,
            other.source_id
        ));
    }
    Ok((inputs, first))
}

/// Predicts as Predict Auto Model does on the ledger: table rows go through the saved
/// preprocessing with a batch dimension per row, and anomaly models apply `calibration`.
/// Synchronous; callers run it in `spawn_blocking`.
#[cfg(feature = "inference")]
pub(crate) fn predict_table_rows(
    predictor: &CatalogLoadedPredictor,
    preprocessing: Option<&Value>,
    calibration: Option<AnomalyCalibration>,
    input: TableRowsInput,
) -> Result<TableRowsOutput> {
    if input.rows.len() > MAXIMUM_PREDICTION_ROWS {
        return Err(anyhow!(
            "Prediction batch of {} rows exceeds {MAXIMUM_PREDICTION_ROWS} rows",
            input.rows.len()
        ));
    }
    let artifact = predictor.artifact();
    let (inputs, extractor, feature_state) = match preprocessing {
        Some(preprocessing) => {
            let preprocessing: super::auto_training_tables::TablePreprocessing =
                json::from_value(preprocessing.clone()).map_err(|error| {
                    anyhow!(
                        "The table preprocessing of model {} is invalid: {error}",
                        artifact.id
                    )
                })?;
            let extractor = super::auto_training::requested_extractor(
                &preprocessing,
                &input.rows,
                input.feature_extractor.as_ref(),
            )?;
            super::auto_training::ensure_extractor_named(
                &artifact.id,
                &artifact.manifest,
                extractor.as_ref(),
            )?;
            let (mut inputs, feature_state) =
                super::auto_training_tables::transform_table_rows_with_state(
                    &preprocessing,
                    &input.rows,
                    32 * 1024 * 1024,
                    input.feature_state,
                    input.stateful_features,
                )?;
            for tensor in &mut inputs {
                tensor.shape.insert(0, 1);
            }
            (inputs, extractor, feature_state)
        }
        None => {
            let (inputs, extractor) =
                tensor_rows(artifact, &input.rows, input.feature_extractor.as_ref())?;
            super::auto_training::ensure_extractor_named(
                &artifact.id,
                &artifact.manifest,
                extractor.as_ref(),
            )?;
            (inputs, extractor, input.feature_state)
        }
    };
    let predictions = inputs
        .iter()
        .map(|tensor| predictor.predict_checked(tensor, extractor.as_ref()))
        .collect::<Result<Vec<_>>>()?;
    let anomaly = calibration
        .map(|calibration| {
            super::auto_training::anomaly_decisions(
                &predictions,
                calibration.scale,
                calibration.threshold,
            )
        })
        .transpose()?;
    Ok(TableRowsOutput {
        predictions,
        feature_state,
        anomaly,
    })
}

/// The `models/artifacts` folder holding `manifest`, when it sits in an artifact folder.
#[cfg(feature = "inference")]
fn artifacts_folder_of(manifest: &FlowPath) -> Option<FlowPath> {
    let key = manifest.object_path();
    let parts = key.as_ref().split('/').collect::<Vec<_>>();
    let count = parts.len();
    (count >= 4 && parts[count - 3] == "artifacts" && parts[count - 4] == PUBLISHED_MODELS_DIR).then(
        || FlowPath {
            path: parts[..count - 2].join("/"),
            store_ref: manifest.store_ref.clone(),
            cache_store_ref: manifest.cache_store_ref.clone(),
        },
    )
}

/// The folder of a model's feature-extractor dependency: the sibling artifact folder of a
/// model root, then `dependencies/{source}` beside the manifest. Only these two places are
/// read, and the dependency's id and weights digest must match the record.
#[cfg(feature = "inference")]
async fn resolve_dependency(
    context: &mut ExecutionContext,
    model: &ResolvedModel,
    dependency: &PublishedDependency,
) -> Result<ResolvedModel> {
    let mut folders = Vec::new();
    if let Some(storage) = model.storage {
        let root = model_root(context, storage).await?;
        folders.push(child(&root, &artifact_dir(&dependency.source_id)));
    } else if let Some(artifacts) = artifacts_folder_of(&model.manifest_path) {
        folders.push(child(&artifacts, &seg(&dependency.source_id)));
    }
    folders.push(child(
        &folder_of(&model.manifest_path),
        &dependency_dir(&dependency.source_id),
    ));
    let mut tried = Vec::new();
    for folder in folders {
        let manifest_path = child(&folder, MANIFEST_FILE);
        tried.push(manifest_path.path.clone());
        let Some(bytes) = read_capped(
            context,
            &manifest_path,
            MAXIMUM_MANIFEST_BYTES,
            "model manifest",
        )
        .await?
        else {
            continue;
        };
        let parsed = parse_manifest(&bytes, &manifest_path, false)?;
        if parsed.manifest.artifact_id() != dependency.source_id
            || parsed.manifest.weights.sha256 != dependency.blob_sha256
        {
            return Err(anyhow!(
                "{} holds {} with weights {}, not the feature extractor {} with weights {} that model {} records",
                manifest_path.path,
                parsed.manifest.artifact_id(),
                parsed.manifest.weights.sha256,
                dependency.source_id,
                dependency.blob_sha256,
                model.manifest.artifact_id()
            ));
        }
        let file = parsed.manifest.weights.file.as_deref().ok_or_else(|| {
            anyhow!(
                "The model manifest {} names no weights file in its folder",
                manifest_path.path
            )
        })?;
        return Ok(ResolvedModel {
            weights_path: child(&folder, file),
            manifest: parsed.manifest,
            manifest_path,
            storage: model.storage,
            inline_preprocessing: None,
        });
    }
    Err(anyhow!(
        "Feature extractor {} of model {} is not published; looked for {}",
        dependency.source_id,
        model.manifest.artifact_id(),
        tried.join(", ")
    ))
}

/// Features of one published folder: a pretrained Burn source or a Burn image classifier.
#[cfg(feature = "inference")]
async fn folder_features(
    context: &mut ExecutionContext,
    model: &ResolvedModel,
    mut input: TensorData,
    compute: &ComputeConfig,
) -> Result<(TensorData, FeatureExtractorRef)> {
    if let Some(source) = model.manifest.pretrained_source.clone() {
        if source.manifest["engine"] != "burn" {
            return Err(anyhow!(
                "Published source {} is a {} model; image features need a Burn backbone",
                source.id,
                model.manifest.engine()
            ));
        }
        let bytes = published_weights(context, model, served_weights_limit(compute)).await?;
        let extractor: FeatureExtractorRef = FeatureExtractor::from_source(&source)?.into();
        let compute = compute.clone();
        let features = tokio::task::spawn_blocking(move || {
            flow_like_ml_runtime::engines::pretrained_features_from_parts(
                &source, &bytes, &input, &compute,
            )
        })
        .await??;
        return Ok((features, extractor));
    }
    let predictor = published_predictor(context, model, compute).await?;
    let input_shape: Vec<usize> = predictor
        .artifact()
        .manifest
        .get("input_shape")
        .cloned()
        .and_then(|shape| json::from_value(shape).ok())
        .unwrap_or_default();
    if input.shape == input_shape {
        input.shape.insert(0, 1);
    }
    tokio::task::spawn_blocking(move || -> Result<_> {
        Ok((predictor.features(&input)?, predictor.feature_extractor()?))
    })
    .await?
}

/// Spatial features of a published model: through its feature-extractor dependency when it
/// records one, otherwise from the pretrained source or Burn classifier itself.
#[cfg(feature = "inference")]
pub(crate) async fn published_features(
    context: &mut ExecutionContext,
    source: &PublishedModelSource,
    input: TensorData,
    compute: &ComputeConfig,
) -> Result<(TensorData, FeatureExtractorRef)> {
    let compute = compute.for_host();
    compute.validate()?;
    let model = resolve_model(context, source).await?;
    let model = match model.manifest.dependencies.feature_extractor.clone() {
        Some(dependency) => resolve_dependency(context, &model, &dependency).await?,
        None => model,
    };
    folder_features(context, &model, input, &compute).await
}

/// The champion route a learning project's pointer gives: Teacher while it is paused or has
/// no champion. None when no searched root holds a pointer.
#[cfg(feature = "inference")]
pub(crate) async fn published_learning_route(
    context: &mut ExecutionContext,
    project_id: &str,
    storage: Option<ModelStorage>,
) -> Result<Option<flow_like_ml_runtime::LearningRoute>> {
    let mut tried = Vec::new();
    let relative = learning_pointer_path(project_id);
    let Some((pointer, _, _, path)) =
        find_pointer::<ChampionPointer>(context, storage, &relative, &mut tried).await?
    else {
        return Ok(None);
    };
    check_pointer_version(pointer.format_version, &path)?;
    if pointer.project_id != project_id {
        return Err(anyhow!(
            "The model pointer {} belongs to learning project {}, not {project_id}",
            path.path,
            pointer.project_id
        ));
    }
    Ok(Some(match pointer.artifact_id {
        Some(artifact_id) if !pointer.paused => {
            flow_like_ml_runtime::LearningRoute::Champion { artifact_id }
        }
        _ => flow_like_ml_runtime::LearningRoute::Teacher,
    }))
}

/// The deployment pointer in `storage`; None when it does not exist.
#[cfg(feature = "inference")]
pub(crate) async fn published_deployment(
    context: &mut ExecutionContext,
    deployment_id: &str,
    storage: ModelStorage,
) -> Result<Option<DeploymentPointer>> {
    let root = model_root(context, storage).await?;
    let path = child(&root, &deployment_pointer_path(deployment_id));
    let Some(pointer) = read_pointer::<DeploymentPointer>(context, &path).await? else {
        return Ok(None);
    };
    check_pointer_version(pointer.format_version, &path)?;
    if pointer.deployment_id != deployment_id {
        return Err(anyhow!(
            "The model pointer {} belongs to deployment {}, not {deployment_id}",
            path.path,
            pointer.deployment_id
        ));
    }
    Ok(Some(pointer))
}

/// SAM 2 masks from a published SAM 2 source, searched in the app's storage, then the user's.
#[cfg(feature = "inference")]
pub(crate) async fn published_sam2_prediction(
    context: &mut ExecutionContext,
    source_id: &str,
    images: &TensorData,
    prompts: &[flow_like_ml_burn::Sam2Prompt],
    compute: &ComputeConfig,
) -> Result<flow_like_ml_burn::Sam2Prediction> {
    let compute = compute.for_host();
    compute.validate()?;
    let source = PublishedModelSource::Artifact {
        artifact_id: source_id.to_string(),
        storage: None,
    };
    let model = resolve_model(context, &source).await?;
    let sam2 = model
        .manifest
        .pretrained_source
        .clone()
        .filter(|source| source.manifest["engine"] == "sam2")
        .ok_or_else(|| {
            anyhow!(
                "Published model {source_id} at {} is a {} model, not a SAM 2 source",
                model.manifest_path.path,
                model.manifest.engine()
            )
        })?;
    let limit = compute
        .memory_limit_bytes
        .min(MAXIMUM_SERVED_WEIGHTS_BYTES);
    let bytes = published_weights(context, &model, limit).await?;
    let (images, prompts) = (images.clone(), prompts.to_vec());
    Ok(tokio::task::spawn_blocking(move || {
        flow_like_ml_runtime::engines::predict_sam2_from_parts(
            &sam2, &bytes, &images, &prompts, &compute,
        )
    })
    .await??)
}

/// The order of a pointer in a Local app or user store, read without the run's credentials.
#[cfg(feature = "inference")]
async fn local_pointer_order(
    context: &ExecutionContext,
    kind: PointerKind,
    scope: LedgerScope,
    id: &str,
) -> Result<Option<(i64, i64)>> {
    let key = format!("{kind:?}\n{scope:?}\n{id}");
    let probes = run_map::<Option<(i64, i64)>>(context, GENERATION_PROBE_KEY).await?;
    if let Some(order) = probes.get(&key) {
        return Ok(order);
    }
    let execution = context
        .execution_cache
        .as_ref()
        .ok_or_else(|| anyhow!("Execution storage context is unavailable"))?;
    let (store, base) = match scope {
        LedgerScope::App => (
            execution.stores.app_storage_store.clone(),
            execution.get_storage(false)?,
        ),
        LedgerScope::User => (
            execution.stores.user_store.clone(),
            execution.get_user_dir(false)?,
        ),
    };
    let relative = match kind {
        PointerKind::Learning => learning_pointer_path(id),
        PointerKind::Deployment => deployment_pointer_path(id),
    };
    let order = match store {
        Some(store @ FlowLikeStore::Local(_)) => {
            let location = join_object_path(&base, &relative);
            let generic = store.as_generic();
            match generic.head(&location).await {
                Ok(meta) if meta.size > MAXIMUM_POINTER_BYTES => {
                    return Err(anyhow!(
                        "The model pointer {location} is {} bytes, above its limit of {MAXIMUM_POINTER_BYTES} bytes",
                        meta.size
                    ));
                }
                Ok(_) => {
                    let bytes = generic.get(&location).await?.bytes().await?;
                    Some(match kind {
                        PointerKind::Learning => json::from_slice::<ChampionPointer>(&bytes)?.order(),
                        PointerKind::Deployment => {
                            (json::from_slice::<DeploymentPointer>(&bytes)?.generation, 0)
                        }
                    })
                }
                Err(error) if not_found(&error) => None,
                Err(error) => return Err(error.into()),
            }
        }
        _ => None,
    };
    probes.insert(key, order, MAXIMUM_GENERATION_PROBES);
    Ok(order)
}

/// Whether a pointer in the run's Local store orders strictly after `ledger_order`: a
/// champion by (deployment generation, project generation), a deployment by its generation
/// against `ledger_order.0`. Only Local app and user stores are read, once per run per
/// pointer, and any error reads as false.
#[cfg(feature = "inference")]
pub(crate) async fn newer_published_generation(
    context: &mut ExecutionContext,
    kind: PointerKind,
    scope: LedgerScope,
    id: &str,
    ledger_order: (i64, i64),
) -> bool {
    match local_pointer_order(context, kind, scope, id).await {
        Ok(Some(order)) => match kind {
            PointerKind::Learning => order > ledger_order,
            PointerKind::Deployment => order.0 > ledger_order.0,
        },
        Ok(None) => false,
        Err(error) => {
            tracing::debug!(%error, id, "Reading a local model pointer failed");
            false
        }
    }
}

#[cfg(test)]
#[path = "published_tests.rs"]
mod tests;

#[cfg(test)]
mod format_tests {
    use super::*;
    use flow_like_types::json::{self, json};

    const SHA: &str = "4a5f0c2e9b8d7a6f5e4d3c2b1a0f9e8d7c6b5a4f3e2d1c0b9a8f7e6d5c4b3a29";

    fn round_trips<T: Serialize + serde::de::DeserializeOwned>(value: Value) -> T {
        let parsed: T = json::from_value(value.clone()).unwrap();
        assert_eq!(json::to_value(&parsed).unwrap(), value);
        parsed
    }

    fn stream() -> Value {
        json!({"project_id":"app:board","stream_id":"auto:0f8e5c1a-3b2d-4c6e-9a7f-1d2e3f4a5b6c","inspection_version":"v1"})
    }

    #[test]
    fn manifest_v2_round_trips_without_loss() {
        let manifest: PublishedModelManifest = round_trips(json!({
            "format_version": 2,
            "artifact": {
                "id": "artifact-1", "job_id": "job-1", "stream": stream(),
                "snapshot_id": "snapshot-1", "dataset_digest": SHA,
                "blob": {"sha256": SHA, "bytes": 1, "path": format!("{SHA}.blob")},
                "manifest": {"engine": "isolation_forest", "preprocessing_manifest": {"$published": "preprocessing.json"}},
                "created_at_ms": 0
            },
            "pretrained_source": null,
            "weights": {"file": "weights.bin", "sha256": SHA, "bytes": 1},
            "preprocessing": {"file": "preprocessing.json", "sha256": SHA, "bytes": 1, "maximum_bytes": 67108864},
            "task": {"kind": "anomaly", "threshold": 0.5},
            "calibration": {"scale": 0.8, "threshold": 0.5},
            "goal_met": false,
            "validation_metrics": {}, "final_test_metrics": {}, "source_table_versions": [],
            "dependencies": {"feature_extractor": {"source_id": "pretrained:resnet", "blob_sha256": SHA}},
            "provenance": {
                "publisher": "auto", "ledger": "app", "storage": "app",
                "experiment_id": "experiment-1", "selected_trial_id": "trial-1",
                "learning_project_id": null, "cycle_id": null, "deployment_id": null,
                "deployment_generation": null, "published_at_ms": 0
            }
        }));
        assert_eq!(manifest.artifact_id(), "artifact-1");
        assert_eq!(manifest.engine(), "isolation_forest");
        assert_eq!(manifest.format_version, MANIFEST_FORMAT_VERSION);
        assert_eq!(
            manifest.calibration,
            Some(AnomalyCalibration {
                scale: 0.8,
                threshold: 0.5
            })
        );
        assert_eq!(
            manifest.artifact.unwrap().manifest["preprocessing_manifest"][PREPROCESSING_MARKER_KEY],
            PREPROCESSING_FILE
        );
        let provenance = manifest.provenance.unwrap();
        assert!(!provenance.dependency);
        assert_eq!(provenance.ledger, LedgerScope::App);
    }

    #[test]
    fn minimal_manifests_default_their_optional_parts() {
        let manifest: PublishedModelManifest = json::from_value(json!({
            "format_version": 2,
            "pretrained_source": {
                "id": "pretrained:resnet", "project_id": "app:board", "source_kind": "imported",
                "blob": {"sha256": SHA, "bytes": 3, "path": format!("{SHA}.blob")},
                "manifest": {"engine": "burn"}, "origin": {}, "created_at_ms": 0
            },
            "weights": {"file": null, "sha256": SHA, "bytes": 3}
        }))
        .unwrap();
        assert_eq!(manifest.artifact_id(), "pretrained:resnet");
        assert_eq!(manifest.engine(), "burn");
        assert!(manifest.dependencies.feature_extractor.is_none());
        assert!(manifest.provenance.is_none() && manifest.preprocessing.is_none());
        let dependency: ModelProvenance = round_trips(json!({
            "publisher": "auto", "ledger": "user", "storage": "user", "dependency": true,
            "experiment_id": null, "selected_trial_id": null, "learning_project_id": null,
            "cycle_id": null, "deployment_id": null, "deployment_generation": null,
            "published_at_ms": 7
        }));
        assert!(dependency.dependency);
    }

    #[test]
    fn preprocessing_and_pointers_round_trip_without_loss() {
        let preprocessing: PublishedPreprocessing = round_trips(json!({
            "format_version": 1, "preprocessing": {"kind": "tabular"},
            "artifact_preprocessing_manifest": null
        }));
        assert_eq!(preprocessing.format_version, PREPROCESSING_FORMAT_VERSION);
        let experiment: ExperimentPointer = round_trips(json!({
            "format_version": 1, "experiment_id": "experiment-1", "artifact_id": "artifact-1",
            "sha256": SHA, "bytes": 1, "goal_met": true, "storage": "app", "published_at_ms": 0
        }));
        assert_eq!(experiment.storage, ModelStorage::App);
        let champion: ChampionPointer = round_trips(json!({
            "format_version": 1, "project_id": "project-1",
            "deployment_id": "deployment:learning:project-1", "deployment_generation": 3,
            "project_generation": 12, "artifact_id": "artifact-1", "sha256": SHA,
            "paused": false, "cycle_id": "cycle-1", "retained": ["older-1", "older-2"],
            "storage": "app", "published_at_ms": 0
        }));
        assert_eq!(champion.order(), (3, 12));
        let deployment: DeploymentPointer = round_trips(json!({
            "format_version": 1, "deployment_id": "deployment-1", "generation": 5,
            "active_artifact_id": "artifact-5", "paused": false, "stream": stream(),
            "sha256": SHA,
            "deployment": {
                "id": "deployment-1", "stream": stream(), "generation": 5,
                "active_artifact_id": "artifact-5", "paused": false,
                "previous_artifact_id": "artifact-4", "evaluation_id": "evaluation-5",
                "policy": null, "metric_policy": null, "updated_at_ms": 10
            },
            "retained": ["artifact-4"], "storage": "user", "published_at_ms": 0
        }));
        assert_eq!(deployment.deployment.generation, deployment.generation);
        let teacher: ChampionPointer = json::from_value(json!({
            "format_version": 1, "project_id": "project-1", "deployment_id": "deployment-1",
            "deployment_generation": 0, "project_generation": 1, "artifact_id": null,
            "sha256": null, "paused": false, "cycle_id": null, "storage": "app",
            "published_at_ms": 0
        }))
        .unwrap();
        assert!(teacher.retained.is_empty() && teacher.artifact_id.is_none());
    }

    #[test]
    fn champion_order_compares_deployment_then_project_generation() {
        let pointer = |deployment_generation, project_generation| ChampionPointer {
            format_version: POINTER_FORMAT_VERSION,
            project_id: "project".into(),
            deployment_id: "deployment".into(),
            deployment_generation,
            project_generation,
            artifact_id: None,
            sha256: None,
            paused: false,
            cycle_id: None,
            retained: Vec::new(),
            storage: ModelStorage::App,
            published_at_ms: 0,
        };
        assert!(pointer(3, 12).order() < pointer(4, 0).order());
        assert!(pointer(3, 12).order() > pointer(3, 11).order());
        assert_eq!(pointer(3, 12).order(), (3, 12));
    }

    #[test]
    fn sources_default_to_searching_both_roots() {
        let source: PublishedModelSource =
            json::from_value(json!({"kind": "artifact", "artifact_id": "artifact-1"})).unwrap();
        assert!(matches!(
            source,
            PublishedModelSource::Artifact { ref artifact_id, storage: None } if artifact_id == "artifact-1"
        ));
        let source: PublishedModelSource = json::from_value(json!({
            "kind": "learning_champion", "project_id": "project-1", "storage": "user"
        }))
        .unwrap();
        assert!(matches!(
            source,
            PublishedModelSource::LearningChampion {
                storage: Some(ModelStorage::User),
                ..
            }
        ));
        let source: PublishedModelSource = json::from_value(json!({
            "kind": "files",
            "manifest": {"path": "models/x/manifest.json", "store_ref": "store", "cache_store_ref": null}
        }))
        .unwrap();
        assert!(matches!(
            source,
            PublishedModelSource::Files { weights: None, .. }
        ));
        for scope in [LedgerScope::App, LedgerScope::User] {
            assert_eq!(LedgerScope::from(ModelStorage::from(scope)), scope);
        }
    }

    #[test]
    fn model_paths_are_built_from_sanitized_segments() {
        assert_eq!(artifact_dir("artifact-1"), "models/artifacts/artifact-1");
        assert_eq!(
            experiment_pointer_path("experiment-1"),
            "models/experiments/experiment-1.json"
        );
        assert_eq!(learning_pointer_path("p"), "models/learning/p.json");
        assert_eq!(deployment_pointer_path("d"), "models/deployments/d.json");
        assert_eq!(dependency_dir("source"), "dependencies/source");
        for hostile in ["pretrained:resnet", "../../escape", "a/b", "", ".."] {
            let folder = artifact_dir(hostile);
            let segment = folder.strip_prefix("models/artifacts/").unwrap();
            assert!(
                !segment.is_empty()
                    && !segment.contains(['/', '\\', ':'])
                    && segment != "."
                    && segment != "..",
                "{hostile} -> {folder}"
            );
        }
        assert_ne!(artifact_dir("a:b"), artifact_dir("a_b"));
        assert_eq!(preprocessing_cap(64 * 1024 * 1024), 129 * 1024 * 1024);
        assert_eq!(preprocessing_cap(u64::MAX), u64::MAX);
    }

    #[test]
    fn inference_refusal_names_the_runtimes_that_can_run_models() {
        assert_eq!(inference_unavailable().to_string(), INFERENCE_UNAVAILABLE);
    }
}
