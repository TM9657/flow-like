//! Adapters between the durable worker contract and native algorithms/Burn models.
use crate::{worker::*, *};
#[cfg(feature = "burn")]
use flow_like_ml_core::ComputeBackend;
use flow_like_ml_core::{Annotation, ComputeConfig, PreprocessingStep, Sample, TensorData};
use rusqlite::{Connection, OptionalExtension};
use std::collections::{BTreeSet, HashMap, HashSet};
use std::sync::Arc;
mod inference;
pub use inference::{LoadedArtifactPredictor, verify_artifact_bytes};

#[cfg(feature = "burn")]
mod burn_adapter;
#[cfg(feature = "burn")]
mod cancellation_monitor;
#[cfg(feature = "burn")]
pub(crate) use cancellation_monitor::BurnCancellationMonitor;
#[cfg(feature = "burn")]
mod efficient_ad_adapter;
#[cfg(feature = "burn")]
mod sam2_adapter;
#[cfg(feature = "burn")]
pub use burn_adapter::{
    BurnEngineRecipe, CheckpointObserver, pretrained_features_from_parts,
    pretrained_source_features, publish_burn_pretrained_source, register_burn_engine,
    register_observed_burn_engine, validate_burn_pretrained_preprocessing,
    validate_burn_pretrained_source,
};
#[cfg(feature = "burn")]
pub use efficient_ad_adapter::{EfficientAdEngineRecipe, register_efficient_ad_engine};
#[cfg(feature = "burn")]
pub use sam2_adapter::{
    fine_tune_sam2_source, predict_sam2_from_parts, predict_sam2_source, publish_sam2_source,
};

#[cfg(feature = "burn")]
fn image_batch_shape(shape: &[usize], expected: &[usize]) -> Result<Vec<usize>> {
    if expected.len() == 3 && !expected.contains(&0) {
        if shape == expected {
            // Image preprocessing produces CHW; native inference consumes NCHW.
            let mut batch = vec![1];
            batch.extend_from_slice(shape);
            return Ok(batch);
        }
        if shape.len() == 4 && shape[0] > 0 && &shape[1..] == expected {
            return Ok(shape.to_vec());
        }
    }
    Err(invalid(
        "image input contract requires matching CHW or NCHW dimensions",
    ))
}

#[cfg(feature = "burn")]
pub fn artifact_features(
    repository: &TrainingRepository,
    artifact_id: &str,
    input: &TensorData,
    compute: &ComputeConfig,
) -> Result<TensorData> {
    LoadedArtifactPredictor::load(repository, artifact_id, compute)?.features(input)
}

#[cfg(feature = "onnx-export")]
pub fn export_artifact_onnx(
    repository: &TrainingRepository,
    artifact_id: &str,
    input: &TensorData,
    compute: &ComputeConfig,
) -> Result<(Vec<u8>, flow_like_ml_burn::OnnxExportReport)> {
    input.validate(16_777_216).map_err(engine_error)?;
    compute.validate().map_err(engine_error)?;
    let artifact = repository.get_artifact(artifact_id)?;
    if artifact.manifest["engine"] != "burn" {
        return Err(invalid("ONNX export requires a Burn model artifact"));
    }
    let limit = (compute.memory_limit_bytes / 4).min(512 * 1024 * 1024);
    burn_adapter::export(
        &repository.read_blob_limited(&artifact.blob, limit)?,
        input,
        compute,
    )
}

fn engine_error(error: impl std::fmt::Display) -> Error {
    Error::Engine(error.to_string())
}

/// Model and image steps that produced embeddings. Memory banks fitted in one extractor's
/// feature space give arbitrary anomaly scores for features from another extractor.
#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema, PartialEq)]
pub struct FeatureExtractor {
    /// Training artifact or pretrained source that computed the features.
    pub source_id: String,
    pub blob_sha256: String,
    #[serde(default)]
    pub image_steps: Vec<PreprocessingStep>,
}

impl FeatureExtractor {
    pub fn from_artifact(artifact: &ModelArtifact) -> Result<Self> {
        Ok(Self {
            source_id: artifact.id.clone(),
            blob_sha256: artifact.blob.sha256.clone(),
            image_steps: expected_image_steps(&artifact.manifest)?,
        })
    }

    pub fn from_source(source: &PretrainedSource) -> Result<Self> {
        Ok(Self {
            source_id: source.id.clone(),
            blob_sha256: source.blob.sha256.clone(),
            image_steps: expected_image_steps(&source.manifest)?,
        })
    }

    pub fn validate(&self) -> Result<()> {
        if self.source_id.trim().is_empty()
            || self.blob_sha256.len() != 64
            || !self.blob_sha256.bytes().all(|b| b.is_ascii_hexdigit())
        {
            return Err(invalid(
                "feature extractor needs a source ID and a SHA-256 weights digest",
            ));
        }
        Ok(())
    }
}

fn is_image_step(step: &PreprocessingStep) -> bool {
    matches!(
        step,
        PreprocessingStep::Resize { .. } | PreprocessingStep::Normalize { .. }
    )
}

/// Image steps a model expects: the steps declared when it was trained, otherwise the
/// recommendation of the weights it was imported or fine-tuned from.
fn expected_image_steps(manifest: &Value) -> Result<Vec<PreprocessingStep>> {
    for key in ["preprocessing", "recommended_preprocessing"] {
        let Some(steps) = manifest.get(key).filter(|value| !value.is_null()) else {
            continue;
        };
        let steps: Vec<PreprocessingStep> = serde_json::from_value(steps.clone())?;
        if steps.iter().any(is_image_step) {
            return Ok(steps);
        }
    }
    Ok(Vec::new())
}

const MAXIMUM_INITIALIZATION_DEPTH: usize = 32;

#[derive(Deserialize)]
struct SampleIdentity {
    group_id: String,
    content_digest: String,
}

/// Sample identities of a snapshot's partitions, decoded without the sample payloads.
#[derive(Deserialize)]
struct PartitionIdentities {
    train: Vec<SampleIdentity>,
    validation: Vec<SampleIdentity>,
    test: Vec<SampleIdentity>,
}

impl PartitionIdentities {
    fn of(snapshot: &DatasetSnapshot) -> Self {
        let identities = |samples: &[TrainingSample]| -> Vec<SampleIdentity> {
            samples
                .iter()
                .map(|sample| SampleIdentity {
                    group_id: sample.group_id.clone(),
                    content_digest: sample.content_digest.clone(),
                })
                .collect()
        };
        Self {
            train: identities(&snapshot.train),
            validation: identities(&snapshot.validation),
            test: identities(&snapshot.test),
        }
    }

    fn fitted(&self) -> impl Iterator<Item = &SampleIdentity> {
        self.train.iter().chain(&self.validation)
    }

    fn all(&self) -> impl Iterator<Item = &SampleIdentity> {
        self.fitted().chain(&self.test)
    }

    fn links(&self) -> impl Iterator<Item = (&str, &str)> {
        self.all()
            .map(|sample| (sample.group_id.as_str(), sample.content_digest.as_str()))
    }
}

fn stored_partitions(connection: &Connection, snapshot_id: &str) -> Result<PartitionIdentities> {
    let body: String = connection
        .query_row(
            "SELECT body FROM snapshots WHERE id=?",
            [snapshot_id],
            |row| row.get(0),
        )
        .optional()?
        .ok_or_else(|| Error::NotFound(format!("snapshot {snapshot_id}")))?;
    Ok(serde_json::from_str(&body)?)
}

/// Partitions a training run saw. Experiment trials train on a filtered snapshot, while
/// selection also scores the validation partition of the trial's full dataset snapshot.
fn run_partitions(
    connection: &Connection,
    job_id: &str,
    snapshot_id: &str,
    own: PartitionIdentities,
) -> Result<Vec<PartitionIdentities>> {
    let full: Option<String> = connection
        .query_row(
            "SELECT json_extract(body,'$.dataset_snapshot_id') FROM experiment_trials WHERE job_id=?",
            [job_id],
            |row| row.get::<_, Option<String>>(0),
        )
        .optional()?
        .flatten();
    let mut runs = vec![own];
    if let Some(full) = full.filter(|full| full != snapshot_id) {
        runs.push(stored_partitions(connection, &full)?);
    }
    Ok(runs)
}

/// In-project training artifact a model was fine-tuned from, if any.
fn initialization_artifact(manifest: &Value) -> Result<Option<String>> {
    let Some(pretrained) = manifest.get("pretrained").filter(|value| !value.is_null()) else {
        return Ok(None);
    };
    let kind = pretrained
        .get("source_kind")
        .map(|kind| serde_json::from_value::<PretrainedSourceKind>(kind.clone()))
        .transpose()?;
    if kind != Some(PretrainedSourceKind::TrainingArtifact) {
        return Ok(None);
    }
    pretrained["source_id"]
        .as_str()
        .map(|id| Some(id.to_owned()))
        .ok_or_else(|| invalid("training-artifact initialization has no source ID"))
}

/// Content that a training artifact, and every training artifact it was initialized from,
/// was fitted on, and content those models were only validated on.
pub(crate) struct LineageContent {
    trained: Consumption,
    validated: Consumption,
}

/// Content digests link a lineage to the groups of any stream. Group IDs name the same lot
/// or part across the snapshots of one stream, so there the lineage's own groups count even
/// when none of its samples remain.
#[derive(Default)]
struct Consumption {
    digests: HashSet<String>,
    groups: HashMap<GroupScope, HashSet<String>>,
}

/// Project and stream ID: group IDs keep their meaning across inspection versions.
type GroupScope = (String, String);

fn group_scope(stream: &StreamKey) -> GroupScope {
    (stream.project_id.clone(), stream.stream_id.clone())
}

impl Consumption {
    fn record(&mut self, stream: &StreamKey, samples: &[SampleIdentity]) {
        let groups = self.groups.entry(group_scope(stream)).or_default();
        for sample in samples {
            self.digests.insert(sample.content_digest.clone());
            groups.insert(sample.group_id.clone());
        }
    }

    /// Consumed groups of `stream`: groups of `samples` holding consumed content, and every
    /// group consumed in `stream` itself.
    fn occupied<'a>(
        &'a self,
        stream: &StreamKey,
        samples: impl IntoIterator<Item = (&'a str, &'a str)>,
    ) -> HashSet<&'a str> {
        let mut occupied: HashSet<&'a str> = linked_groups(samples, &self.digests).collect();
        if let Some(groups) = self.groups.get(&group_scope(stream)) {
            occupied.extend(groups.iter().map(String::as_str));
        }
        occupied
    }
}

impl LineageContent {
    pub(crate) fn load(connection: &Connection, artifact_id: &str) -> Result<Self> {
        let mut lineage = Self {
            trained: Consumption::default(),
            validated: Consumption::default(),
        };
        let mut visited = HashSet::new();
        let mut next = Some(artifact_id.to_owned());
        while let Some(id) = next {
            if visited.len() == MAXIMUM_INITIALIZATION_DEPTH || !visited.insert(id.clone()) {
                return Err(invalid(format!(
                    "initialization lineage of {artifact_id} exceeds {MAXIMUM_INITIALIZATION_DEPTH} models"
                )));
            }
            let body: Option<String> = connection
                .query_row("SELECT body FROM artifacts WHERE id=?", [&id], |row| {
                    row.get(0)
                })
                .optional()?;
            let Some(body) = body else {
                let adopted = LineageDigests::adopted(connection, &id)?;
                lineage.trained.digests.extend(adopted.trained);
                lineage.validated.digests.extend(adopted.validated);
                break;
            };
            let artifact: ModelArtifact = serde_json::from_str(&body)?;
            let own = stored_partitions(connection, &artifact.snapshot_id)?;
            for run in run_partitions(connection, &artifact.job_id, &artifact.snapshot_id, own)? {
                lineage.trained.record(&artifact.stream, &run.train);
                lineage.validated.record(&artifact.stream, &run.validation);
            }
            next = initialization_artifact(&artifact.manifest)?;
        }
        Ok(lineage)
    }

    #[cfg(feature = "native")]
    fn consumed(self) -> Consumption {
        let mut consumed = self.trained;
        consumed.digests.extend(self.validated.digests);
        for (scope, groups) in self.validated.groups {
            consumed.groups.entry(scope).or_default().extend(groups);
        }
        consumed
    }
}

/// Lineage content digests that an adopted copy of a training artifact keeps, because the
/// adopting ledger holds neither the artifact nor its snapshots. The lineage's groups are left
/// out: they only count in streams of the artifact's own project, which adoption rewrites.
#[derive(Serialize, Deserialize)]
pub(crate) struct LineageDigests {
    trained: BTreeSet<String>,
    validated: BTreeSet<String>,
}

impl LineageDigests {
    pub(crate) fn of(connection: &Connection, artifact_id: &str) -> Result<Self> {
        let lineage = LineageContent::load(connection, artifact_id)?;
        Ok(Self {
            trained: lineage.trained.digests.into_iter().collect(),
            validated: lineage.validated.digests.into_iter().collect(),
        })
    }

    /// The digests that the adopted copy of training artifact `id` keeps in this ledger.
    fn adopted(connection: &Connection, id: &str) -> Result<Self> {
        let digests = connection
            .query_row(
                "SELECT json_extract(body,?2) FROM pretrained_sources WHERE id=?1 AND json_extract(body,'$.source_kind')='training_artifact'",
                [id, crate::repository::ADOPTED_LINEAGE],
                |row| row.get::<_, Option<String>>(0),
            )
            .optional()?
            .flatten()
            .ok_or_else(|| {
                Error::NotFound(format!(
                    "initialization artifact {id}, or the lineage its adopted copy keeps"
                ))
            })?;
        Ok(serde_json::from_str(&digests)?)
    }
}

/// Groups of `samples` holding any of the `lineage` content digests.
fn linked_groups<'a>(
    samples: impl IntoIterator<Item = (&'a str, &'a str)>,
    lineage: &HashSet<String>,
) -> impl Iterator<Item = &'a str> {
    samples
        .into_iter()
        .filter(move |(_, digest)| lineage.contains(*digest))
        .map(|(group, _)| group)
}

/// Groups that fitting or selecting an artifact consumed, including the groups its
/// initialization lineage consumed. Snapshots are immutable and can be large, so load this
/// before taking a write lock.
#[cfg(feature = "native")]
pub(crate) struct ConsumedGroups {
    groups: HashSet<String>,
    lineage: HashSet<String>,
}

#[cfg(feature = "native")]
impl ConsumedGroups {
    pub(crate) fn load(
        connection: &Connection,
        artifact: &ModelArtifact,
        snapshot: &DatasetSnapshot,
    ) -> Result<Self> {
        let runs = run_partitions(
            connection,
            &artifact.job_id,
            &snapshot.id,
            PartitionIdentities::of(snapshot),
        )?;
        let lineage = match initialization_artifact(&artifact.manifest)? {
            Some(source) => LineageContent::load(connection, &source)?.consumed(),
            None => Consumption::default(),
        };
        let mut groups: HashSet<String> = runs
            .iter()
            .flat_map(|run| run.fitted())
            .map(|sample| sample.group_id.clone())
            .collect();
        // Audited samples come from the artifact's own stream.
        groups.extend(
            lineage
                .occupied(&artifact.stream, runs.iter().flat_map(|run| run.links()))
                .into_iter()
                .map(str::to_owned),
        );
        Ok(Self {
            groups,
            lineage: lineage.digests,
        })
    }

    /// Audited samples must lie outside every consumed group. An audited sample holding
    /// lineage content also consumes its own group for the other audited samples.
    pub(crate) fn ensure_independent<'a>(
        &self,
        audited: impl IntoIterator<Item = &'a TrainingSample>,
    ) -> Result<()> {
        let audited: Vec<&TrainingSample> = audited.into_iter().collect();
        let linked: HashSet<&str> = linked_groups(
            audited
                .iter()
                .map(|sample| (sample.group_id.as_str(), sample.content_digest.as_str())),
            &self.lineage,
        )
        .collect();
        if let Some(sample) = audited.iter().find(|sample| {
            self.groups.contains(&sample.group_id) || linked.contains(sample.group_id.as_str())
        }) {
            return Err(invalid(format!(
                "audited sample {} shares a training/validation group with this model or a model it was initialized from",
                sample.id
            )));
        }
        Ok(())
    }
}

/// Held-out groups of a run, including an experiment trial's full snapshot, that would make
/// its metrics dependent on the lineage: validation groups the lineage was fitted on, and
/// audit groups it consumed in any way.
#[cfg(feature = "burn")]
fn contaminated_groups(
    connection: &Connection,
    job_id: &str,
    snapshot: &DatasetSnapshot,
    lineage: &LineageContent,
) -> Result<std::collections::BTreeSet<String>> {
    let runs = run_partitions(
        connection,
        job_id,
        &snapshot.id,
        PartitionIdentities::of(snapshot),
    )?;
    let links = || runs.iter().flat_map(|run| run.links());
    let trained = lineage.trained.occupied(&snapshot.stream, links());
    let validated = lineage.validated.occupied(&snapshot.stream, links());
    let validation = runs
        .iter()
        .flat_map(|run| &run.validation)
        .map(|sample| sample.group_id.as_str())
        .filter(|group| trained.contains(group));
    let test = runs
        .iter()
        .flat_map(|run| &run.test)
        .map(|sample| sample.group_id.as_str())
        .filter(|group| trained.contains(group) || validated.contains(group));
    Ok(validation.chain(test).map(str::to_owned).collect())
}

/// Training memory the Burn and EfficientAD engines charge per serialized snapshot byte: the
/// parsed snapshot, its decoded samples and the tensors built from them.
#[cfg(feature = "burn")]
pub(crate) const SNAPSHOT_MEMORY_FACTOR: u64 = 12;

pub(crate) fn serialized_size<T: Serialize + ?Sized>(value: &T) -> Result<u64> {
    struct Counter(u64);
    impl std::io::Write for Counter {
        fn write(&mut self, bytes: &[u8]) -> std::io::Result<usize> {
            self.0 = self
                .0
                .checked_add(bytes.len() as u64)
                .ok_or_else(|| std::io::Error::other("serialized size exceeds u64"))?;
            Ok(bytes.len())
        }
        fn flush(&mut self) -> std::io::Result<()> {
            Ok(())
        }
    }
    let mut counter = Counter(0);
    serde_json::to_writer(&mut counter, value)?;
    Ok(counter.0)
}

fn payload_sample(sample: &TrainingSample, classes: usize) -> Result<Sample> {
    let value = sample
        .payload
        .get("sample")
        .unwrap_or(&sample.payload)
        .clone();
    let parsed: Sample = serde_json::from_value(value)?;
    parsed
        .validate(classes, 64 * 1024 * 1024)
        .map_err(engine_error)?;
    if parsed.id != sample.id || parsed.group_id != sample.group_id {
        return Err(invalid(
            "sample payload identity/group differs from durable ledger",
        ));
    }
    if parsed
        .outcome
        .as_ref()
        .is_some_and(|outcome| outcome.available_at_ms > sample.label_available_at_ms)
    {
        return Err(invalid(
            "outcome was unavailable at the recorded label availability",
        ));
    }
    Ok(parsed)
}

#[cfg(feature = "native")]
fn labels(recipe: &Value) -> Result<Vec<String>> {
    let labels: Vec<String> = recipe
        .get("labels")
        .cloned()
        .map(serde_json::from_value)
        .transpose()?
        .unwrap_or_default();
    flow_like_ml_core::validate_labels(&labels).map_err(engine_error)?;
    Ok(labels)
}

/// Native algorithms currently use CPU. Refuse a GPU request unless the caller opted into
/// CPU fallback, so a backend pin cannot silently become a different execution contract.
#[cfg(feature = "native")]
fn validate_native_compute(value: &Value) -> Result<()> {
    let compute: ComputeConfig = serde_json::from_value(value.clone())?;
    compute.validate().map_err(engine_error)?;
    if !matches!(
        compute.backend,
        flow_like_ml_core::ComputeBackend::Cpu | flow_like_ml_core::ComputeBackend::Auto
    ) && !compute.allow_cpu_fallback
    {
        return Err(invalid(
            "native classical models require CPU or explicit CPU fallback",
        ));
    }
    Ok(())
}

#[cfg(feature = "native")]
pub fn register_native_engines(worker: &mut TrainingWorker) -> Result<()> {
    for name in [
        "isolation_forest",
        "histogram_gradient_boosting",
        "patchcore",
        "padim",
    ] {
        worker.register(name, Arc::new(NativeEngine { name }))?;
    }
    Ok(())
}

#[cfg(feature = "native")]
#[derive(Serialize, Deserialize)]
#[serde(tag = "engine", content = "model", rename_all = "snake_case")]
enum NativeModel {
    IsolationForest(flow_like_ml_native::models::IsolationForest),
    HistogramGradientBoosting(flow_like_ml_native::models::HistogramGradientBoosting),
    Patchcore(flow_like_ml_native::models::PatchCore),
    Padim(flow_like_ml_native::models::Padim),
}

#[cfg(feature = "native")]
struct NativeEngine {
    name: &'static str,
}
#[cfg(feature = "native")]
impl TrainingEngine for NativeEngine {
    fn resource_key(&self, _: &TrainingWork) -> Result<String> {
        Ok("cpu:0".into())
    }
    fn estimated_memory_bytes(&self, work: &TrainingWork) -> Result<u64> {
        let base = serialized_size(&work.snapshot.train)?;
        let model = match self.name {
            "isolation_forest" => work
                .job
                .request
                .recipe
                .pointer("/config/trees")
                .and_then(Value::as_u64)
                .unwrap_or(100)
                .saturating_mul(65536),
            "histogram_gradient_boosting" => {
                let config = &work.job.request.recipe["config"];
                let depth = config["max_depth"].as_u64().unwrap_or(3).min(16);
                config["estimators"]
                    .as_u64()
                    .unwrap_or(100)
                    .saturating_mul(1u64 << depth)
                    .saturating_mul(64)
            }
            "padim" => {
                let sample = payload_sample(
                    work.snapshot
                        .train
                        .first()
                        .ok_or_else(|| invalid("empty training set"))?,
                    2,
                )?;
                if sample.input.shape.len() != 2 {
                    return Err(invalid("PaDiM input must be [patches, embedding channels]"));
                }
                (sample.input.shape[0] as u64)
                    .saturating_mul(sample.input.shape[1] as u64)
                    .saturating_mul(sample.input.shape[1] as u64)
                    .saturating_mul(16)
            }
            _ => base.saturating_mul(2),
        };
        Ok(base.saturating_mul(8).saturating_add(model))
    }
    fn train(&self, work: &TrainingWork, control: &WorkerControl) -> Result<EngineOutput> {
        use flow_like_ml_native::models::*;
        validate_native_compute(&work.job.request.compute)?;
        control.check_cancel()?;
        let labels = labels(&work.job.request.recipe)?;
        let extractor = recipe_feature_extractor(&work.job.request.recipe)?;
        let samples: Vec<_> = work
            .snapshot
            .train
            .iter()
            .map(|sample| payload_sample(sample, labels.len().max(2)))
            .collect::<Result<_>>()?;
        let input_shape = samples
            .first()
            .ok_or_else(|| invalid("empty training data"))?
            .input
            .shape
            .clone();
        if samples.iter().any(|s| s.input.shape != input_shape) {
            return Err(invalid("training inputs have different shapes"));
        }
        let rows: Vec<Vec<f64>> = samples
            .iter()
            .map(|s| s.input.values.iter().map(|v| *v as f64).collect())
            .collect();
        let progress = |step| {
            control
                .progress(serde_json::json!({"step":step}))
                .map_err(|error| flow_like_ml_core::Error::Invalid(error.to_string()))
        };
        let trained: std::result::Result<NativeModel, flow_like_ml_core::Error> = match self.name {
            "isolation_forest" => {
                let config: IsolationForestConfig = serde_json::from_value(
                    work.job
                        .request
                        .recipe
                        .get("config")
                        .cloned()
                        .unwrap_or_else(
                            || serde_json::json!({"trees":100,"sample_size":256,"seed":42}),
                        ),
                )?;
                IsolationForest::fit_with_progress(&rows, &config, progress)
                    .map(NativeModel::IsolationForest)
            }
            "histogram_gradient_boosting" => {
                let config: BoostConfig =
                    serde_json::from_value(work.job.request.recipe["config"].clone())?;
                let targets = samples
                    .iter()
                    .map(|s| boost_target(&s.annotation))
                    .collect::<Result<Vec<_>>>()?;
                HistogramGradientBoosting::fit_with_progress(&rows, &targets, &config, progress)
                    .map(NativeModel::HistogramGradientBoosting)
            }
            "patchcore" => {
                ensure_normal(&samples)?;
                let images = patch_images(&samples)?;
                let patches = images.into_iter().flatten().collect::<Vec<_>>();
                let count = work.job.request.recipe["coreset_size"]
                    .as_u64()
                    .unwrap_or(256) as usize;
                PatchCore::fit_with_progress(&patches, count, progress).map(NativeModel::Patchcore)
            }
            "padim" => {
                ensure_normal(&samples)?;
                let images = patch_images(&samples)?;
                let regularization = work.job.request.recipe["regularization"]
                    .as_f64()
                    .unwrap_or(0.01);
                Padim::fit_with_progress(&images, regularization, progress).map(NativeModel::Padim)
            }
            _ => return Err(invalid("unsupported native engine")),
        };
        // Preserve cancellation/lease failure rather than wrapping it as a fit error.
        control.check_cancel()?;
        let trained = trained.map_err(engine_error)?;
        let model_bytes = serde_json::to_vec(&trained)?;
        control.checkpoint(
            &model_bytes,
            1,
            serde_json::json!({"engine":self.name,"resume_mode":"completed_model"}),
        )?;
        let mut manifest = serde_json::json!({"engine":self.name,"format_version":1,"labels":labels,"input_shape":input_shape,"recipe":work.job.request.recipe,"backend":"cpu"});
        if self.name == "histogram_gradient_boosting"
            && samples
                .iter()
                .any(|sample| matches!(sample.annotation, Annotation::Values { .. }))
        {
            manifest["output_shape"] = serde_json::json!([1]);
        }
        if let Some(extractor) = extractor {
            manifest["feature_extractor"] = serde_json::to_value(extractor)?;
        }
        Ok(EngineOutput {
            model_bytes,
            manifest,
        })
    }
}

/// Single-step forecasts carry one-element value targets, which boosting fits as scalars.
#[cfg(feature = "native")]
fn boost_target(annotation: &Annotation) -> Result<f64> {
    match annotation {
        Annotation::Class { class_id } => Ok(*class_id as f64),
        Annotation::Scalar { value } => Ok(*value),
        Annotation::Values { values } if values.len() == 1 => Ok(values[0] as f64),
        _ => Err(invalid(
            "boosting needs class, scalar or single-value annotations",
        )),
    }
}

#[cfg(feature = "native")]
fn recipe_feature_extractor(recipe: &Value) -> Result<Option<FeatureExtractor>> {
    recipe
        .get("feature_extractor")
        .filter(|value| !value.is_null())
        .map(|value| -> Result<FeatureExtractor> {
            let extractor: FeatureExtractor = serde_json::from_value(value.clone())?;
            extractor.validate()?;
            Ok(extractor)
        })
        .transpose()
}

#[cfg(feature = "native")]
fn ensure_normal(samples: &[Sample]) -> Result<()> {
    if samples
        .iter()
        .any(|sample| !matches!(sample.annotation, Annotation::Anomaly { is_anomaly: false }))
    {
        return Err(invalid(
            "PatchCore/PaDiM fit requires explicitly normal embeddings",
        ));
    }
    Ok(())
}
#[cfg(feature = "native")]
fn patch_images(samples: &[Sample]) -> Result<Vec<Vec<Vec<f64>>>> {
    samples
        .iter()
        .map(|sample| {
            if sample.input.shape.len() != 2 {
                return Err(invalid(
                    "image embeddings must have shape [patches, embedding channels]",
                ));
            }
            Ok(sample
                .input
                .values
                .chunks(sample.input.shape[1])
                .map(|patch| patch.iter().map(|v| *v as f64).collect())
                .collect())
        })
        .collect()
}

/// Input includes the batch dimension for dense/temporal/image models. Patch anomaly
/// models accept one image as [patches, channels] or a batch as [batch, patches, channels].
pub fn load_artifact_predictions(
    repository: &TrainingRepository,
    artifact_id: &str,
    input: &TensorData,
    compute: &ComputeConfig,
) -> Result<Value> {
    LoadedArtifactPredictor::load(repository, artifact_id, compute)?.predict(input)
}

#[cfg(all(test, feature = "native"))]
mod native_compute_tests {
    use super::*;

    #[test]
    fn automatic_and_defaulted_native_compute_use_cpu_with_explicit_strictness() {
        let request: TrainingRequest =
            serde_json::from_value(serde_json::json!({"engine":"isolation_forest","recipe":{}}))
                .unwrap();
        assert_eq!(request.compute, serde_json::json!({}));
        validate_native_compute(&request.compute).unwrap();
        for value in [
            serde_json::json!({}),
            serde_json::json!({"backend":"auto","allow_cpu_fallback":false}),
            serde_json::json!({"backend":"cuda"}),
            serde_json::json!({"backend":"rocm","allow_cpu_fallback":true}),
        ] {
            validate_native_compute(&value).unwrap();
        }
        for backend in ["cuda", "rocm", "wgpu"] {
            assert!(
                validate_native_compute(
                    &serde_json::json!({"backend":backend,"allow_cpu_fallback":false})
                )
                .is_err()
            );
        }
        assert!(validate_native_compute(&serde_json::json!({"backend":"unknown"})).is_err());
        assert!(validate_native_compute(&serde_json::json!({"memory_limit_bytes":0})).is_err());
    }

    #[test]
    fn boosting_targets_accept_single_step_value_annotations_only() {
        assert_eq!(
            boost_target(&Annotation::Values { values: vec![2.5] }).unwrap(),
            2.5
        );
        assert_eq!(
            boost_target(&Annotation::Scalar { value: -1. }).unwrap(),
            -1.
        );
        assert_eq!(
            boost_target(&Annotation::Class { class_id: 1 }).unwrap(),
            1.
        );
        assert!(
            boost_target(&Annotation::Values {
                values: vec![1., 2.]
            })
            .is_err()
        );
        assert!(boost_target(&Annotation::Anomaly { is_anomaly: true }).is_err());
    }
}

#[cfg(all(test, feature = "native"))]
mod independence_tests {
    use super::*;

    fn sample(id: &str, group: &str, content: u32) -> TrainingSample {
        TrainingSample {
            id: id.into(),
            annotation_revision: 1,
            group_id: group.into(),
            captured_at_ms: 0,
            label_available_at_ms: 0,
            content_digest: format!("{content:064x}"),
            source: LabelSource::Reviewed,
            accepted: true,
            payload: serde_json::json!({}),
        }
    }

    fn stream(id: &str) -> StreamKey {
        StreamKey {
            project_id: "factory".into(),
            stream_id: id.into(),
            inspection_version: "v1".into(),
        }
    }

    fn snapshot(
        id: &str,
        train: Vec<TrainingSample>,
        validation: Vec<TrainingSample>,
        test: Vec<TrainingSample>,
    ) -> DatasetSnapshot {
        DatasetSnapshot {
            id: id.into(),
            digest: String::new(),
            stream: stream(id),
            as_of_ms: 0,
            cutoff_sequence: 0,
            policy: SplitPolicy::Group {
                train_fraction: 0.6,
                validation_fraction: 0.2,
                seed: 1,
            },
            train,
            validation,
            test,
            excluded: Vec::new(),
        }
    }

    fn store_snapshot(connection: &Connection, snapshot: &DatasetSnapshot) {
        connection
            .execute(
                "INSERT INTO snapshots(id,scope,body) VALUES (?,?,?)",
                rusqlite::params![
                    snapshot.id,
                    "test",
                    serde_json::to_string(snapshot).unwrap()
                ],
            )
            .unwrap();
    }

    fn artifact(id: &str, snapshot_id: &str, manifest: Value) -> ModelArtifact {
        ModelArtifact {
            id: id.into(),
            job_id: format!("job-{id}"),
            stream: stream(id),
            snapshot_id: snapshot_id.into(),
            dataset_digest: String::new(),
            blob: BlobRef {
                sha256: "0".repeat(64),
                bytes: 1,
                path: format!("{}.blob", "0".repeat(64)),
            },
            manifest,
            created_at_ms: 0,
        }
    }

    fn store_artifact(connection: &Connection, artifact: &ModelArtifact) {
        connection
            .execute(
                "INSERT INTO artifacts(id,scope,body) VALUES (?,?,?)",
                rusqlite::params![
                    artifact.id,
                    "test",
                    serde_json::to_string(artifact).unwrap()
                ],
            )
            .unwrap();
    }

    fn fine_tuned_from(source: &str) -> Value {
        serde_json::json!({"pretrained":{"source_kind":"training_artifact","source_id":source}})
    }

    fn repository() -> (tempfile::TempDir, Connection) {
        let directory = tempfile::tempdir().unwrap();
        let repository =
            TrainingRepository::open(directory.path().join("training.sqlite")).unwrap();
        let connection = repository.connection().unwrap();
        (directory, connection)
    }

    fn ensure_independent_audit(
        connection: &Connection,
        artifact: &ModelArtifact,
        snapshot: &DatasetSnapshot,
        audited: &[&TrainingSample],
    ) -> Result<()> {
        ConsumedGroups::load(connection, artifact, snapshot)?
            .ensure_independent(audited.iter().copied())
    }

    #[test]
    fn experiment_selection_anomalies_are_consumed_by_the_trained_artifact() {
        let (_directory, connection) = repository();
        let training = snapshot(
            "training",
            vec![sample("a", "g-a", 1)],
            vec![sample("b", "g-b", 2)],
            Vec::new(),
        );
        let full = snapshot(
            "full",
            vec![sample("a", "g-a", 1)],
            vec![sample("b", "g-b", 2), sample("c", "g-anomaly", 3)],
            vec![sample("d", "g-d", 4)],
        );
        store_snapshot(&connection, &training);
        store_snapshot(&connection, &full);
        let model = artifact("model", "training", serde_json::json!({}));
        let leaked = sample("c-review", "g-anomaly", 5);
        ensure_independent_audit(&connection, &model, &training, &[&leaked]).unwrap();
        connection
            .execute(
                "INSERT INTO experiment_trials(id,experiment_id,job_id,idempotency_key,candidate_index,body) VALUES ('trial','experiment',?,'key',0,?)",
                rusqlite::params![
                    model.job_id,
                    serde_json::json!({"dataset_snapshot_id":"full"}).to_string()
                ],
            )
            .unwrap();
        let error = ensure_independent_audit(&connection, &model, &training, &[&leaked])
            .unwrap_err()
            .to_string();
        assert!(error.contains("c-review"), "{error}");
        ensure_independent_audit(&connection, &model, &training, &[&sample("d", "g-d", 4)])
            .unwrap();
    }

    #[test]
    fn initialization_lineage_links_audit_groups_through_shared_content_only() {
        let (_directory, connection) = repository();
        let origin = snapshot(
            "origin",
            vec![sample("o1", "lot-1", 10)],
            vec![sample("o2", "lot-2", 11)],
            vec![sample("o3", "lot-3", 12)],
        );
        store_snapshot(&connection, &origin);
        store_artifact(
            &connection,
            &artifact("origin-model", "origin", serde_json::json!({})),
        );
        let middle = snapshot(
            "middle",
            vec![sample("m1", "lot-1", 20)],
            Vec::new(),
            Vec::new(),
        );
        store_snapshot(&connection, &middle);
        store_artifact(
            &connection,
            &artifact("middle-model", "middle", fine_tuned_from("origin-model")),
        );
        let target = snapshot(
            "target",
            vec![sample("t1", "lot-1", 30)],
            vec![sample("t2", "lot-4", 31)],
            vec![sample("copy", "lot-7", 10), sample("mate", "lot-7", 32)],
        );
        let model = artifact("target-model", "target", fine_tuned_from("middle-model"));
        // Equal group names from another stream are unrelated without shared content.
        ensure_independent_audit(&connection, &model, &target, &[&sample("t3", "lot-2", 33)])
            .unwrap();
        ensure_independent_audit(&connection, &model, &target, &[&sample("t4", "lot-3", 12)])
            .unwrap();
        for leaked in [sample("copy", "lot-7", 10), sample("mate", "lot-7", 32)] {
            assert!(
                ensure_independent_audit(&connection, &model, &target, &[&leaked]).is_err(),
                "{} shares a group with content the origin model trained on",
                leaked.id
            );
        }
        let unseen = sample("late", "lot-9", 11);
        assert!(ensure_independent_audit(&connection, &model, &target, &[&unseen]).is_err());
        let mate = sample("late-mate", "lot-9", 34);
        ensure_independent_audit(&connection, &model, &target, &[&mate]).unwrap();
        let error = ensure_independent_audit(&connection, &model, &target, &[&mate, &unseen])
            .unwrap_err()
            .to_string();
        assert!(error.contains("late-mate"), "{error}");
    }

    fn in_stream(
        stream_id: &str,
        inspection_version: &str,
        artifact: ModelArtifact,
    ) -> ModelArtifact {
        ModelArtifact {
            stream: StreamKey {
                inspection_version: inspection_version.into(),
                ..stream(stream_id)
            },
            ..artifact
        }
    }

    #[test]
    fn same_stream_lineage_groups_stay_consumed_after_their_samples_leave_the_snapshot() {
        let (_directory, connection) = repository();
        let origin = snapshot(
            "origin",
            vec![sample("s1", "lot-7", 1)],
            vec![sample("v1", "lot-8", 2)],
            Vec::new(),
        );
        store_snapshot(&connection, &origin);
        let origin_model = artifact("origin-model", "origin", serde_json::json!({}));
        store_artifact(&connection, &in_stream("camera", "v1", origin_model));
        // s1 was withdrawn, so the fine-tune's snapshot only holds new images of its lot.
        let target = snapshot(
            "target",
            vec![sample("t1", "lot-1", 10)],
            vec![sample("t2", "lot-2", 11)],
            vec![sample("s2", "lot-7", 3)],
        );
        let fine_tuned = artifact("target-model", "target", fine_tuned_from("origin-model"));
        let model = in_stream("camera", "v2", fine_tuned);
        for audited in [sample("s2", "lot-7", 3), sample("v2", "lot-8", 4)] {
            assert!(
                ensure_independent_audit(&connection, &model, &target, &[&audited]).is_err(),
                "{} belongs to a lot the origin model consumed",
                audited.id
            );
        }
        ensure_independent_audit(&connection, &model, &target, &[&sample("n1", "lot-9", 5)])
            .unwrap();
        // Lot names of another stream are unrelated without shared content.
        let elsewhere = in_stream("press", "v1", model);
        ensure_independent_audit(
            &connection,
            &elsewhere,
            &target,
            &[&sample("s2", "lot-7", 3)],
        )
        .unwrap();
    }

    #[test]
    #[cfg(feature = "burn")]
    fn same_stream_lineage_groups_contaminate_held_out_partitions_without_shared_content() {
        let (_directory, connection) = repository();
        let origin = snapshot(
            "origin",
            vec![sample("o1", "lot-1", 10)],
            vec![sample("o2", "lot-2", 11)],
            Vec::new(),
        );
        store_snapshot(&connection, &origin);
        let origin_model = artifact("origin-model", "origin", serde_json::json!({}));
        store_artifact(&connection, &in_stream("camera", "v1", origin_model));
        let lineage = LineageContent::load(&connection, "origin-model").unwrap();
        let held_out = |stream_id: &str| DatasetSnapshot {
            stream: stream(stream_id),
            ..snapshot(
                "run",
                vec![sample("r1", "lot-3", 20)],
                vec![sample("r2", "lot-1", 21)],
                vec![sample("r3", "lot-2", 22), sample("r4", "lot-4", 23)],
            )
        };
        assert_eq!(
            contaminated_groups(&connection, "job-run", &held_out("camera"), &lineage).unwrap(),
            std::collections::BTreeSet::from(["lot-1".to_owned(), "lot-2".to_owned()])
        );
        assert!(
            contaminated_groups(&connection, "job-run", &held_out("press"), &lineage)
                .unwrap()
                .is_empty()
        );
    }

    #[test]
    #[cfg(feature = "burn")]
    fn initialization_rejects_selection_on_trained_content_and_audits_on_any_lineage_content() {
        let (_directory, connection) = repository();
        let origin = snapshot(
            "origin",
            vec![sample("o1", "lot-1", 10)],
            vec![sample("o2", "lot-2", 11)],
            Vec::new(),
        );
        store_snapshot(&connection, &origin);
        store_artifact(
            &connection,
            &artifact("origin-model", "origin", serde_json::json!({})),
        );
        let lineage = LineageContent::load(&connection, "origin-model").unwrap();
        // Reusing the origin's validation rows for validation only repeats its selection data.
        let aligned = snapshot(
            "aligned",
            vec![sample("a1", "a", 10)],
            vec![sample("a2", "b", 11)],
            vec![sample("a3", "c", 12)],
        );
        assert!(
            contaminated_groups(&connection, "job-aligned", &aligned, &lineage)
                .unwrap()
                .is_empty()
        );
        let reshuffled = snapshot(
            "reshuffled",
            vec![sample("r1", "a", 20)],
            vec![sample("r2", "b", 10)],
            vec![sample("r3", "c", 11), sample("r4", "d", 21)],
        );
        assert_eq!(
            contaminated_groups(&connection, "job-reshuffled", &reshuffled, &lineage).unwrap(),
            std::collections::BTreeSet::from(["b".to_owned(), "c".to_owned()])
        );
    }

    /// A finished training artifact of `repo`, fitted on six samples of `stream_id` whose
    /// content digests are `first..first + 6`.
    fn trained_artifact(
        repo: &TrainingRepository,
        stream_id: &str,
        first: u32,
        manifest: Value,
    ) -> ModelArtifact {
        let line = stream(stream_id);
        for content in first..first + 6 {
            let id = format!("{stream_id}-{content}");
            repo.record_sample(&line, &sample(&id, &format!("{id}-lot"), content))
                .unwrap();
        }
        let request = TrainingRequest {
            engine: "fixture".into(),
            recipe: serde_json::json!({}),
            compute: serde_json::json!({}),
        };
        let policy = SplitPolicy::Group {
            train_fraction: 0.5,
            validation_fraction: 0.25,
            seed: 7,
        };
        let job = repo
            .trigger_after_n(&line, 6, request, policy, now_ms())
            .unwrap()
            .unwrap();
        let lease = repo.claim_job(&job.id, "worker", 60_000, now_ms()).unwrap();
        repo.finish_training(&lease, stream_id.as_bytes(), manifest, now_ms())
            .unwrap()
    }

    #[test]
    fn adopted_training_artifacts_keep_their_whole_lineage_without_the_app_ledger() {
        let root = tempfile::tempdir().unwrap();
        let ledger = |name: &str| root.path().join(name).join(LEDGER_DATABASE_FILE);
        let app = TrainingRepository::open(ledger("project")).unwrap();
        let origin = trained_artifact(&app, "camera", 0, serde_json::json!({}));
        let middle = trained_artifact(&app, "press", 10, fine_tuned_from(&origin.id));
        let expected = LineageContent::load(&app.connection().unwrap(), &middle.id).unwrap();
        let origin_trained = app.get_snapshot(&origin.snapshot_id).unwrap().train;
        let user = TrainingRepository::open(ledger("user")).unwrap();
        user.adopt_pretrained_source(&app, &middle.id, "factory:user")
            .unwrap();
        drop(app);
        std::fs::remove_dir_all(root.path().join("project")).unwrap();

        let connection = user.connection().unwrap();
        let adopted = LineageContent::load(&connection, &middle.id).unwrap();
        assert_eq!(adopted.trained.digests, expected.trained.digests);
        assert_eq!(adopted.validated.digests, expected.validated.digests);
        let leaked = TrainingSample {
            content_digest: origin_trained[0].content_digest.clone(),
            ..sample("leaked", "leaked-lot", 0)
        };
        assert!(adopted.trained.digests.contains(&leaked.content_digest));
        // A private fine-tune of the adopted model audits against its whole App lineage.
        let target = snapshot(
            "tuned",
            vec![sample("t1", "t-lot-1", 900)],
            vec![sample("t2", "t-lot-2", 901)],
            Vec::new(),
        );
        let model = artifact("tuned", "tuned", fine_tuned_from(&middle.id));
        let error = ensure_independent_audit(&connection, &model, &target, &[&leaked])
            .unwrap_err()
            .to_string();
        assert!(error.contains("leaked"), "{error}");
        let fresh = sample("fresh", "fresh-lot", 902);
        ensure_independent_audit(&connection, &model, &target, &[&fresh]).unwrap();
        #[cfg(feature = "burn")]
        {
            let held_out = snapshot(
                "held-out",
                vec![sample("r1", "r-lot-1", 903)],
                vec![leaked],
                vec![fresh],
            );
            assert_eq!(
                contaminated_groups(&connection, "job-held-out", &held_out, &adopted).unwrap(),
                std::collections::BTreeSet::from(["leaked-lot".to_owned()])
            );
        }
    }

    #[test]
    fn adopting_again_records_a_lineage_that_an_earlier_adoption_left_out() {
        let root = tempfile::tempdir().unwrap();
        let ledger = |name: &str| root.path().join(name).join(LEDGER_DATABASE_FILE);
        let app = TrainingRepository::open(ledger("project")).unwrap();
        let origin = trained_artifact(&app, "camera", 0, serde_json::json!({}));
        let user = TrainingRepository::open(ledger("user")).unwrap();
        let adopted = user
            .adopt_pretrained_source(&app, &origin.id, "factory:user")
            .unwrap();
        let connection = user.connection().unwrap();
        connection
            .execute(
                "UPDATE pretrained_sources SET body=json_remove(body,?1)",
                [crate::repository::ADOPTED_LINEAGE],
            )
            .unwrap();
        assert!(matches!(
            LineageContent::load(&connection, &origin.id),
            Err(Error::NotFound(_))
        ));
        assert_eq!(
            user.adopt_pretrained_source(&app, &origin.id, "factory:user")
                .unwrap(),
            adopted
        );
        let expected = LineageContent::load(&app.connection().unwrap(), &origin.id).unwrap();
        let recorded = LineageContent::load(&connection, &origin.id).unwrap();
        assert_eq!(recorded.trained.digests, expected.trained.digests);
        assert_eq!(recorded.validated.digests, expected.validated.digests);
    }

    #[test]
    fn feature_extractor_identity_requires_a_digest_and_prefers_declared_steps() {
        let mut manifest = serde_json::json!({
            "preprocessing":[{"kind":"features","names":["a"]}],
            "recommended_preprocessing":[{"kind":"resize","width":4,"height":4}],
        });
        assert_eq!(
            expected_image_steps(&manifest).unwrap(),
            vec![PreprocessingStep::Resize {
                width: 4,
                height: 4
            }]
        );
        manifest["preprocessing"] =
            serde_json::json!([{"kind":"normalize","mean":[0.5],"std":[0.25]}]);
        assert_eq!(
            expected_image_steps(&manifest).unwrap(),
            vec![PreprocessingStep::Normalize {
                mean: vec![0.5],
                std: vec![0.25]
            }]
        );
        let model = artifact("model", "snapshot", manifest);
        let extractor = FeatureExtractor::from_artifact(&model).unwrap();
        extractor.validate().unwrap();
        assert_eq!(extractor.source_id, "model");
        assert!(
            FeatureExtractor {
                blob_sha256: "short".into(),
                ..extractor
            }
            .validate()
            .is_err()
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn serialized_size_counts_utf8_escapes_and_nested_values() {
        let value = serde_json::json!({
            "text": "é😀\n\"\\",
            "nested": [null, true, -123.456, {"empty": []}],
            "samples": [1, 2, 3]
        });
        assert_eq!(
            serialized_size(&value).unwrap(),
            serde_json::to_vec(&value).unwrap().len() as u64
        );
    }
}
