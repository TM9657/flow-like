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
use flow_like_types::{Result, anyhow};

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

#[cfg(feature = "inference")]
pub(crate) async fn resolve_model(
    _context: &mut ExecutionContext,
    _source: &PublishedModelSource,
) -> Result<ResolvedModel> {
    Err(anyhow!("resolve_model lands in L4"))
}

#[cfg(feature = "inference")]
pub(crate) async fn published_predictor(
    _context: &mut ExecutionContext,
    _model: &ResolvedModel,
    _compute: &ComputeConfig,
) -> Result<CachedPredictor> {
    Err(anyhow!("published_predictor lands in L4"))
}

#[cfg(feature = "inference")]
pub(crate) async fn published_preprocessing(
    _context: &mut ExecutionContext,
    _model: &ResolvedModel,
) -> Result<Option<Value>> {
    Err(anyhow!("published_preprocessing lands in L4"))
}

/// The manifest's stream must match the inspection's stream id and specification version.
#[cfg(feature = "inference")]
pub(crate) fn check_stream(
    _manifest: &PublishedModelManifest,
    _inspection: &InspectionContext,
) -> Result<()> {
    Err(anyhow!("published::check_stream lands in L4"))
}

/// Synchronous; callers run it in `spawn_blocking`.
#[cfg(feature = "inference")]
pub(crate) fn predict_table_rows(
    _predictor: &CatalogLoadedPredictor,
    _preprocessing: Option<&Value>,
    _calibration: Option<AnomalyCalibration>,
    _input: TableRowsInput,
) -> Result<TableRowsOutput> {
    Err(anyhow!("predict_table_rows lands in L4"))
}

#[cfg(feature = "inference")]
pub(crate) async fn published_features(
    _context: &mut ExecutionContext,
    _source: &PublishedModelSource,
    _input: TensorData,
    _compute: &ComputeConfig,
) -> Result<(TensorData, FeatureExtractorRef)> {
    Err(anyhow!("published_features lands in L4"))
}

#[cfg(feature = "inference")]
pub(crate) async fn published_learning_route(
    _context: &mut ExecutionContext,
    _project_id: &str,
    _storage: Option<ModelStorage>,
) -> Result<Option<flow_like_ml_runtime::LearningRoute>> {
    Err(anyhow!("published_learning_route lands in L4"))
}

#[cfg(feature = "inference")]
pub(crate) async fn published_deployment(
    _context: &mut ExecutionContext,
    _deployment_id: &str,
    _storage: ModelStorage,
) -> Result<Option<DeploymentPointer>> {
    Err(anyhow!("published_deployment lands in L4"))
}

#[cfg(feature = "inference")]
pub(crate) async fn published_sam2_prediction(
    _context: &mut ExecutionContext,
    _source_id: &str,
    _images: &TensorData,
    _prompts: &[flow_like_ml_burn::Sam2Prompt],
    _compute: &ComputeConfig,
) -> Result<flow_like_ml_burn::Sam2Prediction> {
    Err(anyhow!("published_sam2_prediction lands in L4"))
}

/// Whether a pointer in the run's Local store orders strictly after `ledger_order`.
/// False on any error.
#[cfg(feature = "inference")]
pub(crate) async fn newer_published_generation(
    _context: &mut ExecutionContext,
    _kind: PointerKind,
    _scope: LedgerScope,
    _id: &str,
    _ledger_order: (i64, i64),
) -> bool {
    false
}

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
        assert!(pointer(3, 12).order() == pointer(3, 12).order());
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
