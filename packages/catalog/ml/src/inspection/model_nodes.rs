//! Nodes that run published models without a training ledger, and nodes that move models
//! between a training ledger and FlowPath files explicitly.

use super::lifecycle::{FeatureExtractorRef, InspectionContext};
use super::published::{PublishedModelPredictions, PublishedModelRef, PublishedModelSource};
use flow_like::flow::{execution::context::ExecutionContext, node::NodeLogic};
use flow_like_catalog_core::FlowPath;
use flow_like_ml_core::{ComputeConfig, TensorData};
use flow_like_ml_native::feature_engineering::FeatureStreamState;
use flow_like_types::{Result, Value};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

#[derive(Clone, Serialize, Deserialize, JsonSchema)]
pub struct PredictPublishedModelRequest {
    pub model: PublishedModelSource,
    /// Raw table rows when the model has saved table preprocessing, as Predict Auto Model
    /// takes them; otherwise tensors (`shape` and `values`), each given a batch dimension when
    /// its shape equals the trained input shape.
    pub rows: Vec<Value>,
    #[serde(default)]
    pub compute: ComputeConfig,
    #[serde(default)]
    pub stateful_features: bool,
    #[serde(default)]
    pub feature_state: Option<FeatureStreamState>,
    /// The extractor of PatchCore or PaDiM embeddings, as the feature nodes output it. A model
    /// that recorded its extractor refuses embeddings that name none or another one.
    #[serde(default)]
    pub feature_extractor: Option<FeatureExtractorRef>,
}

#[derive(Clone, Serialize, Deserialize, JsonSchema)]
pub struct PublishedFeaturesRequest {
    pub model: PublishedModelSource,
    /// A CHW image or NCHW batch prepared with the model's image steps.
    pub input: TensorData,
    #[serde(default)]
    pub compute: ComputeConfig,
}

/// A model in a training ledger of this app.
#[derive(Clone, Serialize, Deserialize, JsonSchema)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum LedgerModelSource {
    /// The winner of an Auto Train experiment.
    Experiment { experiment_id: String },
    /// The current champion of a learning project.
    LearningChampion { project_id: String },
    /// A lifecycle artifact of the given inspection stream.
    Artifact {
        inspection: InspectionContext,
        artifact_id: String,
    },
    /// An imported or fine-tuned pretrained source.
    PretrainedSource { source_id: String },
}

#[derive(Clone, Serialize, Deserialize, JsonSchema)]
pub struct ExportModelRequest {
    pub source: LedgerModelSource,
    /// Folder that receives weights.bin, preprocessing.json, dependencies/ and manifest.json.
    pub destination: FlowPath,
}

fn default_import_limit() -> u64 {
    512 * 1024 * 1024
}

#[derive(Clone, Serialize, Deserialize, JsonSchema)]
pub struct ImportModelRequest {
    /// A published Burn or SAM 2 model, for example an Export Model folder's manifest.
    pub model: PublishedModelSource,
    /// Register it in the executing user's private ledger instead of the app's. A model read
    /// from your private storage, or trained on private data, needs it unless the model source
    /// names app storage explicitly.
    #[serde(default)]
    pub private: bool,
    #[serde(default = "default_import_limit")]
    pub maximum_bytes: u64,
}

/// Predicts rows with a published model, applying its saved preprocessing and, for anomaly
/// models, its audited calibration.
#[cfg(feature = "inference")]
pub(crate) async fn predict_published_model(
    context: &mut ExecutionContext,
    request: PredictPublishedModelRequest,
) -> Result<PublishedModelPredictions> {
    use super::published::{
        MAXIMUM_PREDICTION_ROWS, PublishedModelInfo, TableRowsInput, predict_table_rows,
        published_predictor, published_preprocessing, resolve_model,
    };
    if request.rows.len() > MAXIMUM_PREDICTION_ROWS {
        return Err(flow_like_types::anyhow!(
            "Prediction batch of {} rows exceeds {MAXIMUM_PREDICTION_ROWS} rows",
            request.rows.len()
        ));
    }
    let model = resolve_model(context, &request.model).await?;
    let predictor = published_predictor(context, &model, &request.compute).await?;
    let preprocessing = published_preprocessing(context, &model).await?;
    let calibration = model.manifest.calibration;
    let info = PublishedModelInfo {
        artifact_id: model.manifest.artifact_id().to_owned(),
        engine: model.manifest.engine().to_owned(),
        sha256: model.manifest.weights.sha256.clone(),
        bytes: model.manifest.weights.bytes,
        storage: model.storage,
        goal_met: model.manifest.goal_met,
        manifest: model.manifest_path.clone(),
    };
    let input = TableRowsInput {
        rows: request.rows,
        feature_state: request.feature_state,
        stateful_features: request.stateful_features,
        feature_extractor: request.feature_extractor,
    };
    let output = tokio::task::spawn_blocking(move || {
        predict_table_rows(&predictor, preprocessing.as_ref(), calibration, input)
    })
    .await??;
    Ok(PublishedModelPredictions {
        model: info,
        predictions: output.predictions,
        feature_state: output.feature_state,
        anomaly: output.anomaly,
    })
}

#[crate::register_node]
#[derive(Default)]
pub struct PredictPublishedModelNode;

#[flow_like_types::async_trait]
impl NodeLogic for PredictPublishedModelNode {
    fn get_node(&self) -> flow_like::flow::node::Node {
        super::operation_node::<PredictPublishedModelRequest, PublishedModelPredictions>(
            "ml_predict_published_model",
            "Predict Published Model",
            "Predict with a model published under storage/models or exported to files, without a training ledger: the saved preprocessing turns raw rows into model inputs and anomaly models also return their audited calibrated decision",
            "predictPublishedModel",
            "AI/ML/Inference",
        )
    }

    async fn run(&self, context: &mut ExecutionContext) -> Result<()> {
        #[cfg(feature = "inference")]
        {
            context.deactivate_exec_pin("exec_out").await?;
            let request: PredictPublishedModelRequest = context.evaluate_pin("request").await?;
            let result = predict_published_model(context, request).await?;
            context
                .set_pin_value("result", flow_like_types::json::json!(result))
                .await?;
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
pub struct ExtractPublishedFeaturesNode;

#[flow_like_types::async_trait]
impl NodeLogic for ExtractPublishedFeaturesNode {
    fn get_node(&self) -> flow_like::flow::node::Node {
        let mut node = super::operation_node::<PublishedFeaturesRequest, TensorData>(
            "ml_extract_published_features",
            "Extract Published Image Features",
            "Extract NCHW spatial features with a published model without a training ledger: through the feature extractor a PatchCore or PaDiM model records, with a published pretrained source, or with a Burn image classifier",
            "publishedImageFeatures",
            "AI/ML/Inference",
        );
        node.add_output_pin(
            "extractor",
            "Feature Extractor",
            "Model and image steps that computed these features. Pass it to PatchCore or PaDiM inference so features from another extractor are refused",
            flow_like::flow::variable::VariableType::Struct,
        )
        .set_schema::<FeatureExtractorRef>();
        node
    }

    async fn run(&self, context: &mut ExecutionContext) -> Result<()> {
        #[cfg(feature = "inference")]
        {
            context.deactivate_exec_pin("exec_out").await?;
            let request: PublishedFeaturesRequest = context.evaluate_pin("request").await?;
            let (features, extractor) = super::published::published_features(
                context,
                &request.model,
                request.input,
                &request.compute,
            )
            .await?;
            context
                .set_pin_value("result", flow_like_types::json::json!(features))
                .await?;
            context
                .set_pin_value("extractor", flow_like_types::json::json!(extractor))
                .await?;
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

#[cfg(feature = "training")]
type Ledger = (
    flow_like_ml_runtime::TrainingRepository,
    super::lifecycle::LedgerScope,
);

/// The ledger of the inspection's stream, which must hold `artifact_id` for that stream.
#[cfg(feature = "training")]
async fn artifact_ledger(
    context: &ExecutionContext,
    inspection: InspectionContext,
    artifact_id: &str,
) -> Result<Ledger> {
    use flow_like_types::anyhow;
    let location = super::lifecycle::LedgerLocation::for_stream(context, &inspection.stream_id)?;
    let scope = location.scope();
    let requested = artifact_id.to_owned();
    let repo = tokio::task::spawn_blocking(move || -> Result<_> {
        let Some((repo, project)) = location.open_existing()? else {
            return Err(anyhow!(
                "No training ledger of this app holds artifact {requested} of stream {}",
                inspection.stream_id
            ));
        };
        let artifact = repo.get_artifact(&requested).map_err(|error| {
            anyhow!("Reading artifact {requested} from the training ledger failed: {error}")
        })?;
        if artifact.stream != inspection.key(project)? {
            return Err(anyhow!(
                "Artifact {requested} belongs to another inspection stream or specification than {}",
                inspection.stream_id
            ));
        }
        Ok(repo)
    })
    .await??;
    Ok((repo, scope))
}

/// The first ledger of this app that holds pretrained source `source_id`.
#[cfg(feature = "training")]
fn source_ledger(context: &ExecutionContext, source_id: &str) -> Result<Ledger> {
    let located = super::lifecycle::locate_repository(context, |repo, project| {
        match repo.get_pretrained_source(source_id) {
            Ok(source) => Ok(source.project_id == project),
            Err(flow_like_ml_runtime::Error::NotFound(_)) => Ok(false),
            Err(error) => Err(error.into()),
        }
    })?
    .ok_or_else(|| {
        flow_like_types::anyhow!(
            "No training ledger of this app holds pretrained source {source_id}"
        )
    })?;
    Ok((located.repo, located.scope))
}

/// Finds the ledger holding the requested model and writes its folder to the destination.
#[cfg(feature = "training")]
pub(crate) async fn export_model(
    context: &mut ExecutionContext,
    request: ExportModelRequest,
) -> Result<PublishedModelRef> {
    use super::publisher::{LedgerModel, export_model_folder};
    let ((repo, scope), model) = match request.source {
        LedgerModelSource::Experiment { experiment_id } => (
            super::auto_training::experiment_ledger(context, &experiment_id)?,
            LedgerModel::Experiment { experiment_id },
        ),
        LedgerModelSource::LearningChampion { project_id } => (
            super::learning::project_ledger(context, &project_id)?,
            LedgerModel::LearningChampion { project_id },
        ),
        LedgerModelSource::Artifact {
            inspection,
            artifact_id,
        } => (
            artifact_ledger(context, inspection, &artifact_id).await?,
            LedgerModel::Artifact { artifact_id },
        ),
        LedgerModelSource::PretrainedSource { source_id } => (
            source_ledger(context, &source_id)?,
            LedgerModel::PretrainedSource { source_id },
        ),
    };
    export_model_folder(context, &repo, scope, &model, &request.destination).await
}

#[crate::register_node]
#[derive(Default)]
pub struct ExportModelNode;

#[flow_like_types::async_trait]
impl NodeLogic for ExportModelNode {
    fn get_node(&self) -> flow_like::flow::node::Node {
        super::operation_node::<ExportModelRequest, PublishedModelRef>(
            "ml_export_model",
            "Export Model",
            "Write a ledger model as a model folder (weights.bin, preprocessing.json, dependencies/ and manifest.json, written last) to any FlowPath, including app storage, so Predict Published Model and Import Model can use it anywhere",
            "exportModel",
            "AI/ML/Export",
        )
    }

    async fn run(&self, context: &mut ExecutionContext) -> Result<()> {
        #[cfg(feature = "training")]
        {
            context.deactivate_exec_pin("exec_out").await?;
            let request: ExportModelRequest = context.evaluate_pin("request").await?;
            let result = export_model(context, request).await?;
            context
                .set_pin_value("result", flow_like_types::json::json!(result))
                .await?;
            context.activate_exec_pin("exec_out").await?;
            Ok(())
        }
        #[cfg(not(feature = "training"))]
        {
            let _ = context;
            Err(super::lifecycle::training_unavailable())
        }
    }
}

#[cfg(feature = "training")]
const MAXIMUM_IMPORT_BYTES: u64 = 2 * 1024 * 1024 * 1024;

/// The pretrained-source manifest of a published Burn or SAM 2 model; other engines are
/// refused.
#[cfg(feature = "training")]
async fn importable_manifest(
    context: &mut ExecutionContext,
    model: &super::published::ResolvedModel,
) -> Result<Value> {
    use flow_like_types::anyhow;
    let engine = model.manifest.engine();
    if !matches!(engine, "burn" | "sam2") {
        return Err(anyhow!(
            "Import Model registers Burn and SAM 2 models as pretrained sources, but {} is a {} model",
            model.manifest.artifact_id(),
            if engine.is_empty() { "unknown" } else { engine }
        ));
    }
    match (&model.manifest.artifact, &model.manifest.pretrained_source) {
        (Some(artifact), _) => {
            let restored =
                super::published::restore_artifact(context, model, artifact.clone()).await?;
            Ok(restored.manifest)
        }
        (None, Some(source)) => Ok(source.manifest.clone()),
        (None, None) => Err(anyhow!(
            "{} describes neither a model nor a pretrained source",
            model.manifest_path.path
        )),
    }
}

/// Whether a model source names app storage itself: `storage: app` on an id source, or a files
/// source whose files all lie below app storage.
#[cfg(feature = "training")]
fn names_app_storage(
    source: &PublishedModelSource,
    files: &[flow_like_storage::Path],
    app_root: &flow_like_storage::Path,
) -> bool {
    use super::published::ModelStorage;
    match source {
        PublishedModelSource::Files { .. } => {
            files.iter().all(|file| file.prefix_matches(app_root))
        }
        PublishedModelSource::Experiment { storage, .. }
        | PublishedModelSource::LearningChampion { storage, .. }
        | PublishedModelSource::Deployment { storage, .. }
        | PublishedModelSource::Artifact { storage, .. } => *storage == Some(ModelStorage::App),
    }
}

/// Keeps private models out of the app's shared ledger: a model with a file in the executing
/// user's directory is refused, and so is one whose manifest records a private ledger unless
/// the request names app storage explicitly.
#[cfg(feature = "training")]
fn ensure_shareable(
    context: &ExecutionContext,
    source: &PublishedModelSource,
    model: &super::published::ResolvedModel,
) -> Result<()> {
    use super::lifecycle::LedgerScope;
    let cache = context
        .execution_cache
        .as_ref()
        .ok_or_else(|| flow_like_types::anyhow!("Execution storage context is unavailable"))?;
    let (app_root, user_root) = (cache.get_storage(false)?, cache.get_user_dir(false)?);
    let files = [
        model.manifest_path.object_path(),
        model.weights_path.object_path(),
    ];
    let from_user_root = files.iter().any(|file| file.prefix_matches(&user_root));
    let private_ledger = model
        .manifest
        .provenance
        .as_ref()
        .is_some_and(|provenance| provenance.ledger == LedgerScope::User);
    if !from_user_root && (!private_ledger || names_app_storage(source, &files, &app_root)) {
        return Ok(());
    }
    Err(private_model_refusal(model, from_user_root))
}

#[cfg(feature = "training")]
fn private_model_refusal(
    model: &super::published::ResolvedModel,
    from_user_root: bool,
) -> flow_like_types::Error {
    let reason = if from_user_root {
        "is read from your private (user-scoped) storage"
    } else {
        "was trained on private (user-scoped) data"
    };
    flow_like_types::anyhow!(
        "Model {} at {} {reason}, so Import Model cannot register it in the app's shared training ledger; set private to true, or import a copy shared in app storage by naming it explicitly (storage \"app\", or a files manifest below app storage)",
        model.manifest.artifact_id(),
        model.manifest_path.path
    )
}

/// Registers a published Burn or SAM 2 model as a pretrained source of this app's ledger, or
/// of the executing user's private ledger.
#[cfg(feature = "training")]
pub(crate) async fn import_model(
    context: &mut ExecutionContext,
    request: ImportModelRequest,
) -> Result<flow_like_ml_runtime::PretrainedSource> {
    use super::lifecycle::LedgerScope;
    use super::published::{published_weights, resolve_model};
    use flow_like_types::{anyhow, json::json};
    if !(1..=MAXIMUM_IMPORT_BYTES).contains(&request.maximum_bytes) {
        return Err(anyhow!(
            "Import Model's byte limit must be between 1 byte and 2 GiB, not {}",
            request.maximum_bytes
        ));
    }
    let model = resolve_model(context, &request.model).await?;
    if !request.private {
        ensure_shareable(context, &request.model, &model)?;
    }
    let manifest = importable_manifest(context, &model).await?;
    let bytes = published_weights(context, &model, request.maximum_bytes).await?;
    let origin = json!({
        "published": {
            "id": model.manifest.artifact_id(),
            "engine": model.manifest.engine(),
            "sha256": model.manifest.weights.sha256,
            "manifest": model.manifest_path.path,
        }
    });
    let scope = if request.private {
        LedgerScope::User
    } else {
        LedgerScope::App
    };
    let (repo, project) = super::lifecycle::scoped_repository(context, scope)?;
    let maximum = request.maximum_bytes;
    Ok(tokio::task::spawn_blocking(move || {
        repo.register_pretrained_source(
            &project,
            &bytes,
            manifest,
            origin,
            maximum,
            flow_like_ml_runtime::now_ms(),
        )
    })
    .await??)
}

#[crate::register_node]
#[derive(Default)]
pub struct ImportModelNode;

#[flow_like_types::async_trait]
impl NodeLogic for ImportModelNode {
    fn get_node(&self) -> flow_like::flow::node::Node {
        super::operation_node::<ImportModelRequest, flow_like_ml_runtime::PretrainedSource>(
            "ml_import_model",
            "Import Model",
            "Register a published Burn or SAM 2 model as a pretrained source of this app's training ledger, or of your private ledger, to fine-tune it or extract features with it; other engines are refused",
            "importModel",
            "AI/ML/Export",
        )
    }

    async fn run(&self, context: &mut ExecutionContext) -> Result<()> {
        #[cfg(feature = "training")]
        {
            context.deactivate_exec_pin("exec_out").await?;
            let request: ImportModelRequest = context.evaluate_pin("request").await?;
            let result = import_model(context, request).await?;
            context
                .set_pin_value("result", flow_like_types::json::json!(result))
                .await?;
            context.activate_exec_pin("exec_out").await?;
            Ok(())
        }
        #[cfg(not(feature = "training"))]
        {
            let _ = context;
            Err(super::lifecycle::training_unavailable())
        }
    }
}
