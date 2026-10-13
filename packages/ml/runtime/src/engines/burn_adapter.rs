use super::*;
use flow_like_ml_burn as burn;
use std::{fs, path::Path};

#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema)]
pub struct BurnEngineRecipe {
    pub config: burn::TrainingConfig,
    #[serde(default)]
    pub labels: Vec<String>,
    #[serde(default)]
    pub preprocessing: Vec<flow_like_ml_core::PreprocessingStep>,
    #[serde(default)]
    pub pretrained: Option<PretrainedSourceRef>,
}

fn fine_tune_options(source: &PretrainedSourceRef) -> burn::FineTuneOptions {
    burn::FineTuneOptions {
        replace_head: source.replace_head,
        freeze_backbone: source.freeze_backbone,
    }
}

fn authorized_source(
    repository: &TrainingRepository,
    project_id: &str,
    source_id: &str,
    maximum_bytes: u64,
) -> Result<PretrainedSource> {
    if project_id.trim().is_empty() || source_id.trim().is_empty() {
        return Err(invalid(
            "pretrained source requires a project and source ID",
        ));
    }
    let source = repository.get_pretrained_source(source_id)?;
    if source.project_id != project_id {
        return Err(invalid("pretrained source belongs to a different project"));
    }
    if source.manifest["engine"] != "burn"
        || source.manifest["format_version"].as_u64() != Some(1)
        || source.blob.bytes == 0
        || source.blob.bytes > maximum_bytes
    {
        return Err(invalid(
            "pretrained source format or byte budget is incompatible",
        ));
    }
    Ok(source)
}

/// Packed state files print f32 values in their shortest form, while manifests built with
/// `json!` widen them to f64, so configs are compared as typed values.
pub(super) fn same_training_config(left: &Value, right: &Value) -> bool {
    match (
        serde_json::from_value::<burn::TrainingConfig>(left.clone()),
        serde_json::from_value::<burn::TrainingConfig>(right.clone()),
    ) {
        (Ok(left), Ok(right)) => left == right,
        _ => false,
    }
}

fn unpack_source(
    repository: &TrainingRepository,
    source: &PretrainedSource,
    maximum_bytes: u64,
) -> Result<tempfile::TempDir> {
    let directory = tempfile::tempdir()?;
    let bytes = repository.read_blob_limited(&source.blob, maximum_bytes)?;
    unpack(&bytes, directory.path(), maximum_bytes)?;
    let state: Value =
        serde_json::from_slice(&fs::read(directory.path().join(burn::MANIFEST_FILE))?)?;
    if !same_training_config(&state["config"], &source.manifest["config"])
        || state["input_shape"] != source.manifest["input_shape"]
        || state.get("imported").filter(|value| !value.is_null())
            != source
                .manifest
                .get("imported")
                .filter(|value| !value.is_null())
    {
        return Err(invalid(
            "pretrained manifest differs from its packed model state",
        ));
    }
    Ok(directory)
}

/// Extract spatial features from project-owned weights without creating training evidence.
pub fn pretrained_source_features(
    repository: &TrainingRepository,
    project_id: &str,
    source_id: &str,
    input: &TensorData,
    compute: &ComputeConfig,
) -> Result<TensorData> {
    compute.validate().map_err(engine_error)?;
    let source = authorized_source(repository, project_id, source_id, source_limit(compute))?;
    let config: burn::TrainingConfig = serde_json::from_value(source.manifest["config"].clone())?;
    config.validate_stored().map_err(engine_error)?;
    if !matches!(
        config.recipe,
        burn::Recipe::ResNet18 { .. }
            | burn::Recipe::MobileNetV2 { .. }
            | burn::Recipe::EfficientNet { .. }
            | burn::Recipe::DinoV2 { .. }
    ) {
        return Err(invalid(
            "Spatial features require ResNet18, MobileNetV2, EfficientNet or DINOv2",
        ));
    }
    let input_limit = (compute.memory_limit_bytes / 64).min(64 * 1024 * 1024) as usize;
    input.validate(input_limit).map_err(engine_error)?;
    let expected: Vec<usize> = serde_json::from_value(source.manifest["input_shape"].clone())?;
    let batch_shape = image_batch_shape(&input.shape, &expected)?;
    config
        .recipe
        .validate_shape(&batch_shape)
        .map_err(engine_error)?;
    let workspace = match attention_workspace_bytes(&config.recipe, &expected) {
        Some(per_image) => u64::try_from(per_image * batch_shape[0] as u128)
            .map_err(|_| invalid("feature workspace estimate overflows"))?,
        None => config
            .recipe
            .estimated_training_bytes(&expected, batch_shape[0])
            .map_err(engine_error)?,
    };
    let estimate = source
        .blob
        .bytes
        .checked_mul(4)
        .and_then(|bytes| bytes.checked_add(workspace))
        .ok_or_else(|| invalid("feature memory estimate overflows"))?;
    if estimate > compute.memory_limit_bytes {
        return Err(invalid(format!(
            "Feature extraction needs an estimated {estimate} bytes, exceeding the configured {} byte memory limit",
            compute.memory_limit_bytes
        )));
    }
    let directory = unpack_source(repository, &source, source_limit(compute))?;
    let predictor =
        burn::Predictor::load(directory.path(), backend(compute)?).map_err(engine_error)?;
    let output = predictor
        .features(&burn::TensorData {
            shape: batch_shape,
            values: input.values.clone(),
        })
        .map_err(engine_error)?;
    let output = TensorData {
        shape: output.shape,
        values: output.values,
    };
    output.validate(input_limit).map_err(engine_error)?;
    Ok(output)
}

/// Resolve an initialization model within the owning project before admitting training.
pub fn validate_burn_pretrained_source(
    repository: &TrainingRepository,
    project_id: &str,
    reference: &PretrainedSourceRef,
    config: &burn::TrainingConfig,
    labels: &[String],
    maximum_bytes: u64,
) -> Result<PretrainedSource> {
    let source = authorized_source(repository, project_id, &reference.source_id, maximum_bytes)?;
    let source_config: burn::TrainingConfig =
        serde_json::from_value(source.manifest["config"].clone())?;
    source_config.validate_stored().map_err(engine_error)?;
    burn::validate_fine_tune(
        &source_config.recipe,
        &config.recipe,
        &fine_tune_options(reference),
    )
    .map_err(engine_error)?;
    let source_labels: Vec<String> = serde_json::from_value(
        source
            .manifest
            .get("labels")
            .cloned()
            .unwrap_or_else(|| serde_json::json!([])),
    )?;
    flow_like_ml_core::validate_labels(&source_labels).map_err(engine_error)?;
    if !reference.replace_head && source_labels != labels {
        return Err(invalid(
            "retaining a pretrained head requires the same label order",
        ));
    }
    if !reference.replace_head
        && source
            .manifest
            .pointer("/imported/classification_head_pretrained")
            .and_then(Value::as_bool)
            == Some(false)
    {
        return Err(invalid(
            "imported backbone requires a new classification head",
        ));
    }
    Ok(source)
}

/// Reject declared input normalization that clearly differs from the normalization the
/// source's weights expect. Tensors carry no provenance, so undeclared steps are not checked.
pub fn validate_burn_pretrained_preprocessing(
    source: &PretrainedSource,
    preprocessing: &[PreprocessingStep],
) -> Result<()> {
    let (Some(expected), Some(declared)) = (
        input_normalization(&expected_image_steps(&source.manifest)?)?,
        input_normalization(preprocessing)?,
    ) else {
        return Ok(());
    };
    let channels = expected.0.len().max(declared.0.len());
    let compatible = (0..channels).all(|index| {
        match (
            channel(&expected.0, index),
            channel(&expected.1, index),
            channel(&declared.0, index),
            channel(&declared.1, index),
        ) {
            (Some(mean), Some(deviation), Some(declared_mean), Some(declared_deviation)) => {
                (mean - declared_mean).abs() <= 0.1
                    && declared_deviation > 0.0
                    && (deviation / declared_deviation).max(declared_deviation / deviation) <= 2.0
            }
            _ => false,
        }
    });
    if !compatible {
        return Err(invalid(format!(
            "pretrained source {} expects images normalized with mean [{}] and std [{}], but the dataset declares mean [{}] and std [{}]; prepare inputs with the source's recommended preprocessing",
            source.id,
            channels_text(&expected.0),
            channels_text(&expected.1),
            channels_text(&declared.0),
            channels_text(&declared.1),
        )));
    }
    Ok(())
}

/// Per-channel mean and standard deviation of the composed Normalize steps, or `None` when
/// the steps declare no image preprocessing at all.
fn input_normalization(steps: &[PreprocessingStep]) -> Result<Option<(Vec<f64>, Vec<f64>)>> {
    if !steps.iter().any(is_image_step) {
        return Ok(None);
    }
    let (mut mean, mut deviation) = (vec![0.0], vec![1.0]);
    for step in steps {
        let PreprocessingStep::Normalize {
            mean: step_mean,
            std: step_deviation,
        } = step
        else {
            continue;
        };
        let step_mean: Vec<f64> = step_mean.iter().map(|v| f64::from(*v)).collect();
        let step_deviation: Vec<f64> = step_deviation.iter().map(|v| f64::from(*v)).collect();
        let channels = mean.len().max(step_mean.len());
        let mut next = (Vec::with_capacity(channels), Vec::with_capacity(channels));
        for index in 0..channels {
            // (x - a) / b, then (y - c) / d, equals (x - (a + c b)) / (b d).
            let (Some(a), Some(b), Some(c), Some(d)) = (
                channel(&mean, index),
                channel(&deviation, index),
                channel(&step_mean, index),
                channel(&step_deviation, index),
            ) else {
                return Err(invalid(
                    "image normalization steps have incompatible channel counts",
                ));
            };
            next.0.push(a + c * b);
            next.1.push(b * d);
        }
        (mean, deviation) = next;
    }
    Ok(Some((mean, deviation)))
}

fn channel(values: &[f64], index: usize) -> Option<f64> {
    match values {
        [single] => Some(*single),
        _ => values.get(index).copied(),
    }
}

fn channels_text(values: &[f64]) -> String {
    values
        .iter()
        .map(|value| format!("{value:.3}"))
        .collect::<Vec<_>>()
        .join(", ")
}

/// Multiple of the largest value a source's normalization gives an image scaled to [0, 1]
/// beyond which inputs cannot have been prepared for it, such as raw 0-255 pixels.
const INPUT_SCALE_TOLERANCE: f64 = 8.0;

/// Tensors carry no record of their preprocessing, but inputs far outside the range the
/// source's normalization produces were not scaled for it, whatever the run declares.
fn validate_pretrained_input_range(
    source: &PretrainedSource,
    inputs: &[&burn::TensorData],
) -> Result<()> {
    let steps = expected_image_steps(&source.manifest)?;
    if !steps
        .iter()
        .any(|step| matches!(step, PreprocessingStep::Normalize { .. }))
    {
        return Ok(());
    }
    let Some((mean, deviation)) = input_normalization(&steps)? else {
        return Ok(());
    };
    let bound = (0..mean.len().max(deviation.len()))
        .filter_map(|index| {
            let (mean, deviation) = (channel(&mean, index)?, channel(&deviation, index)?);
            Some(mean.abs().max((1.0 - mean).abs()) / deviation.abs())
        })
        .fold(0.0, f64::max);
    let largest = inputs
        .iter()
        .flat_map(|input| &input.values)
        .fold(0.0, |largest: f64, value| largest.max(f64::from(value.abs())));
    if largest > bound * INPUT_SCALE_TOLERANCE {
        return Err(invalid(format!(
            "pretrained source {} expects images scaled to [0, 1] and normalized with mean [{}] and std [{}], which keeps values within ±{bound:.2}, but the training inputs reach {largest:.1}; they look unscaled, for example raw 0-255 pixels; prepare inputs with the source's recommended preprocessing",
            source.id,
            channels_text(&mean),
            channels_text(&deviation),
        )));
    }
    Ok(())
}

/// Fine-tuning from an in-project model must not select on data that model's lineage was
/// fitted on, nor audit on any of its lineage's data, or the metrics stop being independent.
fn ensure_independent_initialization(work: &TrainingWork, source: &PretrainedSource) -> Result<()> {
    if source.source_kind != PretrainedSourceKind::TrainingArtifact {
        return Ok(());
    }
    let connection = work.repository.connection()?;
    let lineage = LineageContent::load(&connection, &source.id)?;
    let groups = contaminated_groups(&connection, &work.job.id, &work.snapshot, &lineage)?;
    if let Some(example) = groups.first() {
        return Err(invalid(format!(
            "pretrained source {} or a model it was initialized from consumed data in {} validation/test groups of this run (for example {example}); initialize from imported weights or keep that data in training",
            source.id,
            groups.len()
        )));
    }
    Ok(())
}

/// Register a converted Burn model as initialization weights without inventing training evidence.
pub fn publish_burn_pretrained_source(
    repository: &TrainingRepository,
    project_id: &str,
    directory: &Path,
    labels: Vec<String>,
    origin: Value,
    maximum_bytes: u64,
    at_ms: i64,
) -> Result<PretrainedSource> {
    flow_like_ml_core::validate_labels(&labels).map_err(engine_error)?;
    let model_bytes = pack(directory, false, maximum_bytes)?;
    let state: Value = serde_json::from_slice(&fs::read(directory.join(burn::MANIFEST_FILE))?)?;
    let config: burn::TrainingConfig = serde_json::from_value(state["config"].clone())?;
    config.validate_stored().map_err(engine_error)?;
    let input_shape: Vec<usize> = serde_json::from_value(state["input_shape"].clone())?;
    let mut batch_shape = vec![1];
    batch_shape.extend(&input_shape);
    config
        .recipe
        .validate_shape(&batch_shape)
        .map_err(engine_error)?;
    if !labels.is_empty() && labels.len() != config.recipe.outputs() {
        return Err(invalid(
            "pretrained labels must match the model output count",
        ));
    }
    let recommended_preprocessing = match state.get("imported").filter(|value| !value.is_null()) {
        Some(provenance) => burn::ImportedModelInfo {
            recipe: config.recipe.clone(),
            input_shape: input_shape.clone(),
            provenance: serde_json::from_value(provenance.clone())?,
        }
        .recommended_preprocessing(),
        None => Vec::new(),
    };
    repository.register_pretrained_source(
        project_id,
        &model_bytes,
        serde_json::json!({
            "engine":"burn", "format_version":1, "config":state["config"],
            "input_shape":input_shape, "labels":labels,
            "imported":state["imported"], "pretrained":state["pretrained"],
            "recommended_preprocessing":recommended_preprocessing,
        }),
        origin,
        maximum_bytes,
        at_ms,
    )
}

fn source_limit(compute: &ComputeConfig) -> u64 {
    (compute.memory_limit_bytes / 4).min(512 * 1024 * 1024)
}

/// Called with the job and step after each durable Burn epoch checkpoint is published.
pub type CheckpointObserver = Arc<dyn Fn(&TrainingJob, u64) + Send + Sync>;

pub fn register_burn_engine(worker: &mut TrainingWorker) -> Result<()> {
    worker.register("burn", Arc::new(BurnEngine { observer: None }))
}

pub fn register_observed_burn_engine(
    worker: &mut TrainingWorker,
    observer: CheckpointObserver,
) -> Result<()> {
    worker.register(
        "burn",
        Arc::new(BurnEngine {
            observer: Some(observer),
        }),
    )
}

struct BurnEngine {
    observer: Option<CheckpointObserver>,
}

/// Packs a resumable checkpoint. An experiment trial's final artifact must fit its artifact
/// reservation, so the model weights a checkpoint resumes must fit it as well. The optimizer
/// state is bounded by the checkpoint limit and charged to the experiment once the trial
/// settles.
fn pack_checkpoint(
    repository: &TrainingRepository,
    job_id: &str,
    directory: &Path,
    manifest: &str,
    maximum_bytes: u64,
) -> Result<Vec<u8>> {
    let bytes = pack_named(directory, manifest, true, maximum_bytes)?;
    let weights = checkpoint_model_bytes(&bytes);
    repository
        .check_experiment_artifact_budget(job_id, weights)
        .map_err(|error| match error {
            Error::Invalid(_) => invalid(format!(
                "model weights of {weights} bytes exceed the experiment trial's artifact reservation"
            )),
            other => other,
        })?;
    Ok(bytes)
}

/// Size of the inference artifact a packed checkpoint holds: the envelope without its
/// optimizer state.
fn checkpoint_model_bytes(checkpoint: &[u8]) -> u64 {
    let optimizer = checkpoint
        .get(24..32)
        .and_then(|length| <[u8; 8]>::try_from(length).ok())
        .map_or(0, u64::from_le_bytes);
    (checkpoint.len() as u64).saturating_sub(optimizer)
}

pub(super) fn publish_checkpoint(
    work: &TrainingWork,
    control: &WorkerControl,
    directory: &Path,
    manifest: &str,
    step: u64,
    metadata: Value,
) -> Result<()> {
    let bytes = pack_checkpoint(
        &work.repository,
        &work.job.id,
        directory,
        manifest,
        control.limits().maximum_checkpoint_bytes,
    )?;
    control.checkpoint(&bytes, step, metadata)?;
    Ok(())
}

/// After a run stopped, persists the last completed batch that the engine checkpointed
/// before returning, under the checks of an epoch checkpoint. Callers report why the run
/// stopped, so they ignore a failure here.
pub(super) fn publish_stop_checkpoint(
    work: &TrainingWork,
    control: &WorkerControl,
    directory: &Path,
    manifest: &str,
    mut metadata: Value,
) -> Result<()> {
    let state_path = directory.join(manifest);
    if !state_path.exists() {
        return Ok(());
    }
    let state: Value = serde_json::from_slice(&fs::read(state_path)?)?;
    let step = state
        .pointer("/report/steps")
        .and_then(Value::as_u64)
        .unwrap_or(0);
    let bytes = pack_checkpoint(
        &work.repository,
        &work.job.id,
        directory,
        manifest,
        control.limits().maximum_checkpoint_bytes,
    )?;
    metadata["steps"] = step.into();
    control.checkpoint_after_stop(&bytes, step, metadata)?;
    Ok(())
}

/// Peak inference workspace of one image for attention backbones, whose activations grow
/// quadratically with the token count. Inference retains one transformer block at a time:
/// attention matrices, projected tokens, normalization and input/output copies.
pub(super) fn attention_workspace_bytes(
    recipe: &burn::Recipe,
    sample_shape: &[usize],
) -> Option<u128> {
    let [_, height, width] = sample_shape else {
        return None;
    };
    // RF-DETR's windowed backbone is a ViT-S whose global blocks attend over all patches.
    let (patch, heads, embedding) = match recipe {
        burn::Recipe::DinoV2 { config } => (
            burn::DinoV2Config::PATCH_SIZE,
            config.variant.heads(),
            config.variant.embedding_dim(),
        ),
        burn::Recipe::RfDetr { config } => (config.variant.patch_size(), 6, 384),
        _ => return None,
    };
    let tokens = (*height / patch) as u128 * (*width / patch) as u128 + 1;
    let elements = sample_shape.iter().map(|d| *d as u128).product::<u128>();
    Some(tokens * tokens * heads as u128 * 16 + tokens * embedding as u128 * 128 + elements * 16)
}

/// Images per forward pass that keep attention workspaces inside the memory left after
/// loading a model. Other recipes are bounded by the input element budget alone.
pub(super) fn inference_batch_limit(
    recipe: &burn::Recipe,
    sample_shape: &[usize],
    available_bytes: u64,
) -> Result<usize> {
    let Some(per_image) = attention_workspace_bytes(recipe, sample_shape) else {
        return Ok(usize::MAX);
    };
    let limit = u128::from(available_bytes) / per_image.max(1);
    if limit == 0 {
        return Err(invalid(format!(
            "inference needs an estimated {per_image} bytes per image, but only {available_bytes} bytes remain after loading the model"
        )));
    }
    Ok(usize::try_from(limit).unwrap_or(usize::MAX))
}

pub(super) fn resource_key(compute: &ComputeConfig) -> Result<String> {
    Ok(match backend(compute)? {
        burn::BackendChoice::Auto => return Err(engine_error("backend resolution returned Auto")),
        burn::BackendChoice::Cpu => "cpu:0".into(),
        burn::BackendChoice::Wgpu => "wgpu:0".into(),
        burn::BackendChoice::WgpuAdapter { kind, device } => format!("wgpu:{kind:?}:{device}"),
        burn::BackendChoice::Cuda { device } => format!("cuda:{device}"),
        burn::BackendChoice::Rocm { device } => format!("rocm:{device}"),
    })
}
impl TrainingEngine for BurnEngine {
    fn resource_key(&self, work: &TrainingWork) -> Result<String> {
        let compute = compute_config(&work.job.request.compute)?;
        resource_key(&compute)
    }
    fn estimated_memory_bytes(&self, work: &TrainingWork) -> Result<u64> {
        let recipe: BurnEngineRecipe = serde_json::from_value(work.job.request.recipe.clone())?;
        recipe.config.validate().map_err(engine_error)?;
        let sample = work
            .snapshot
            .train
            .first()
            .ok_or_else(|| invalid("empty tensor partition"))?;
        let sample = payload_sample(sample, recipe.labels.len())?;
        let model = recipe
            .config
            .recipe
            .estimated_training_bytes(&sample.input.shape, recipe.config.batch_size)
            .map_err(engine_error)?;
        let dataset = serialized_size(&work.snapshot)?;
        let source_memory = if let Some(reference) = &recipe.pretrained {
            let compute = compute_config(&work.job.request.compute)?;
            let source = validate_burn_pretrained_source(
                &work.repository,
                &work.job.stream.project_id,
                reference,
                &recipe.config,
                &recipe.labels,
                source_limit(&compute),
            )?;
            if work.resume_checkpoint.is_some() {
                0
            } else {
                let config: burn::TrainingConfig =
                    serde_json::from_value(source.manifest["config"].clone())?;
                let shape: Vec<usize> =
                    serde_json::from_value(source.manifest["input_shape"].clone())?;
                source.blob.bytes.saturating_mul(4).saturating_add(
                    config
                        .recipe
                        .estimated_training_bytes(&shape, 1)
                        .map_err(engine_error)?,
                )
            }
        } else {
            0
        };
        Ok(model
            .saturating_add(dataset.saturating_mul(SNAPSHOT_MEMORY_FACTOR))
            .saturating_add(source_memory))
    }
    fn snapshot_byte_limit(&self, budget: u64) -> u64 {
        budget / SNAPSHOT_MEMORY_FACTOR
    }
    fn train(&self, work: &TrainingWork, control: &WorkerControl) -> Result<EngineOutput> {
        let mut recipe: BurnEngineRecipe = serde_json::from_value(work.job.request.recipe.clone())?;
        flow_like_ml_core::validate_labels(&recipe.labels).map_err(engine_error)?;
        if recipe.config.recipe.objective() == Some(burn::Objective::Classification)
            && recipe.labels.len() != recipe.config.recipe.outputs()
        {
            return Err(invalid(
                "classification labels must match the model output count",
            ));
        }
        let compute = compute_config(&work.job.request.compute)?;
        recipe.config.backend = backend(&compute)?;
        let training = dataset(
            &work.snapshot.train,
            &recipe.config.recipe,
            recipe.labels.len(),
        )?;
        let validation = dataset(
            &work.snapshot.validation,
            &recipe.config.recipe,
            recipe.labels.len(),
        )?;
        let directory = tempfile::tempdir()?;
        if let Some(bytes) = &work.resume_checkpoint {
            unpack(
                bytes,
                directory.path(),
                control.limits().maximum_checkpoint_bytes,
            )?;
        }
        let pretrained = recipe
            .pretrained
            .as_ref()
            .map(|reference| {
                validate_burn_pretrained_source(
                    &work.repository,
                    &work.job.stream.project_id,
                    reference,
                    &recipe.config,
                    &recipe.labels,
                    source_limit(&compute),
                )
            })
            .transpose()?;
        if let Some(source) = &pretrained {
            validate_burn_pretrained_preprocessing(source, &recipe.preprocessing)?;
            validate_pretrained_input_range(source, &[&training.inputs, &validation.inputs])?;
            ensure_independent_initialization(work, source)?;
        }
        let source_directory = if work.resume_checkpoint.is_none() {
            pretrained
                .as_ref()
                .map(|source| unpack_source(&work.repository, source, source_limit(&compute)))
                .transpose()?
        } else {
            None
        };
        let cancellation = burn::CancellationToken::new();
        let cancellation_monitor = BurnCancellationMonitor::start(control, &cancellation)?;
        let mut interruption = None;
        let mut checkpoint_error = None;
        let mut latest_published = 0;
        let mut callback = |progress: burn::TrainingProgress| {
            if let Err(error) =
                control.progress(serde_json::to_value(&progress).unwrap_or(Value::Null))
            {
                interruption = Some(error);
                cancellation.cancel();
                return;
            }
            // Burn publishes its epoch checkpoint before emitting validation progress.
            if progress.validation_loss.is_some() && progress.steps > latest_published {
                let result = publish_checkpoint(
                    work,
                    control,
                    directory.path(),
                    burn::MANIFEST_FILE,
                    progress.steps as u64,
                    serde_json::json!({"engine":"burn","format_version":1,"epoch":progress.epoch,"steps":progress.steps}),
                );
                if let Err(error) = result {
                    checkpoint_error = Some(error);
                    cancellation.cancel();
                } else {
                    latest_published = progress.steps;
                    if let Some(observer) = &self.observer {
                        observer(&work.job, progress.steps as u64);
                        // An observer may have stopped the job; honor that before the next batch.
                        if let Err(error) = control.check_cancel_now() {
                            interruption = Some(error);
                            cancellation.cancel();
                        }
                    }
                }
            }
        };
        let trained = if work.resume_checkpoint.is_some() {
            burn::resume(
                &recipe.config,
                &training,
                Some(&validation),
                directory.path(),
                &cancellation,
                &mut callback,
            )
        } else if let Some(source_directory) = &source_directory {
            burn::train_from_pretrained(
                &recipe.config,
                &training,
                Some(&validation),
                source_directory.path(),
                &fine_tune_options(recipe.pretrained.as_ref().unwrap()),
                directory.path(),
                &cancellation,
                &mut callback,
            )
        } else {
            burn::train(
                &recipe.config,
                &training,
                Some(&validation),
                directory.path(),
                &cancellation,
                &mut callback,
            )
        };
        let cancellation_reason = cancellation_monitor.finish();
        if trained.is_err() {
            // Burn checkpoints the last completed batch on cooperative cancellation.
            let _ = publish_stop_checkpoint(
                work,
                control,
                directory.path(),
                burn::MANIFEST_FILE,
                serde_json::json!({"engine":"burn","format_version":1}),
            );
        }
        if let Some(error) = interruption.or(checkpoint_error).or(cancellation_reason) {
            return Err(error);
        }
        let report = trained.map_err(|error| {
            if matches!(error, burn::Error::Cancelled) {
                Error::Cancelled
            } else {
                engine_error(error)
            }
        })?;
        control.check_cancel()?;
        publish_checkpoint(
            work,
            control,
            directory.path(),
            burn::MANIFEST_FILE,
            report.steps as u64,
            serde_json::json!({"engine":"burn","format_version":1,"steps":report.steps}),
        )?;
        let model_bytes = pack(
            directory.path(),
            false,
            control.limits().maximum_artifact_bytes,
        )?;
        let state: Value =
            serde_json::from_slice(&fs::read(directory.path().join(burn::MANIFEST_FILE))?)?;
        let recommended = match &pretrained {
            Some(source) if input_normalization(&recipe.preprocessing)?.is_none() => {
                expected_image_steps(&source.manifest)?
            }
            _ => Vec::new(),
        };
        let initialization = pretrained.map(|source| {
            serde_json::json!({
                "source_id":source.id, "source_kind":source.source_kind,
                "source_blob":source.blob, "source_origin":source.origin,
                "options":recipe.pretrained, "training":state["pretrained"],
            })
        });
        // The packed state's own config keeps loading exact for non-dyadic f32 values.
        let mut manifest = serde_json::json!({"engine":"burn","format_version":1,"config":state["config"],"labels":recipe.labels,"input_shape":training.inputs.shape[1..],"preprocessing":recipe.preprocessing,"report":report,"pretrained":initialization});
        if !recommended.is_empty() {
            manifest["recommended_preprocessing"] = serde_json::to_value(&recommended)?;
        }
        Ok(EngineOutput {
            model_bytes,
            manifest,
        })
    }
}

fn dataset(
    samples: &[TrainingSample],
    recipe: &burn::Recipe,
    classes: usize,
) -> Result<burn::TensorDataset> {
    let samples = samples
        .iter()
        .map(|sample| payload_sample(sample, classes))
        .collect::<Result<Vec<_>>>()?;
    let first = samples
        .first()
        .ok_or_else(|| invalid("empty tensor partition"))?;
    if samples
        .iter()
        .any(|sample| sample.input.shape != first.input.shape)
    {
        return Err(invalid("training samples have inconsistent shapes"));
    }
    let mut shape = vec![samples.len()];
    shape.extend_from_slice(&first.input.shape);
    let inputs = burn::TensorData {
        shape,
        values: samples
            .iter()
            .flat_map(|sample| sample.input.values.iter().copied())
            .collect(),
    };
    let box_target = |bounds: &flow_like_ml_core::BoundingBox| burn::BoundingBox {
        class_id: bounds.class_id as usize,
        x_min: bounds.x_min,
        y_min: bounds.y_min,
        x_max: bounds.x_max,
        y_max: bounds.y_max,
    };
    let targets = if matches!(recipe, burn::Recipe::MaskRcnn { .. }) {
        burn::Targets::Instances {
            values: samples
                .iter()
                .map(|sample| {
                    let Annotation::InstanceMasks { instances } = &sample.annotation else {
                        return Err(invalid("Mask R-CNN requires instance masks"));
                    };
                    Ok(instances
                        .iter()
                        .map(|instance| burn::InstanceTarget {
                            bbox: box_target(&instance.bounds),
                            mask: burn::TensorData {
                                shape: vec![instance.height, instance.width],
                                values: instance
                                    .foreground
                                    .iter()
                                    .map(|value| if *value { 1.0 } else { 0.0 })
                                    .collect(),
                            },
                        })
                        .collect())
                })
                .collect::<Result<_>>()?,
        }
    } else if recipe.is_detection() {
        burn::Targets::Boxes {
            values: samples
                .iter()
                .map(|sample| {
                    let Annotation::Boxes { boxes } = &sample.annotation else {
                        return Err(invalid("Object detection requires box annotations"));
                    };
                    Ok(boxes.iter().map(&box_target).collect())
                })
                .collect::<Result<_>>()?,
        }
    } else if recipe.is_segmentation() {
        let mut shape = vec![samples.len()];
        let mut values = Vec::new();
        let mut mask_shape = None;
        for sample in &samples {
            let Annotation::Mask {
                width,
                height,
                classes,
            } = &sample.annotation
            else {
                return Err(invalid("segmentation requires dense masks"));
            };
            if mask_shape.is_some_and(|dims| dims != (*height, *width)) {
                return Err(invalid("segmentation mask dimensions differ"));
            }
            mask_shape = Some((*height, *width));
            values.extend(classes.iter().map(|c| *c as i64));
        }
        let (height, width) = mask_shape.unwrap();
        shape.extend([height, width]);
        burn::Targets::Segmentation { shape, values }
    } else {
        match recipe.objective() {
            Some(burn::Objective::Classification) => burn::Targets::Classes {
                values: samples
                    .iter()
                    .map(|sample| match sample.annotation {
                        Annotation::Class { class_id } => Ok(class_id as i64),
                        _ => Err(invalid("classification requires class annotations")),
                    })
                    .collect::<Result<_>>()?,
            },
            Some(burn::Objective::Regression) => {
                let values = samples
                    .iter()
                    .map(|sample| {
                        let values = match &sample.annotation {
                            Annotation::Scalar { value } => vec![*value as f32],
                            Annotation::Values { values } => values.clone(),
                            _ => {
                                return Err(invalid(
                                    "regression requires scalar/dense annotations",
                                ));
                            }
                        };
                        if values.len() != recipe.outputs() {
                            return Err(invalid(format!(
                                "regression sample {} has target width {}; model requires {}",
                                sample.id,
                                values.len(),
                                recipe.outputs()
                            )));
                        }
                        Ok(values)
                    })
                    .collect::<Result<Vec<_>>>()?
                    .into_iter()
                    .flatten()
                    .collect();
                burn::Targets::Dense {
                    tensor: burn::TensorData {
                        shape: vec![samples.len(), recipe.outputs()],
                        values,
                    },
                }
            }
            None => {
                if samples.iter().any(|sample| {
                    matches!(sample.annotation, Annotation::Anomaly { is_anomaly: true })
                }) {
                    return Err(invalid(
                        "reconstruction anomaly training requires normal examples",
                    ));
                }
                burn::Targets::Reconstruction
            }
        }
    };
    let dataset = burn::TensorDataset { inputs, targets };
    dataset.validate(recipe).map_err(engine_error)?;
    Ok(dataset)
}

pub(super) fn compute_config(value: &Value) -> Result<ComputeConfig> {
    let mut default = serde_json::to_value(ComputeConfig::default())?;
    let incoming = value
        .as_object()
        .ok_or_else(|| invalid("compute must be an object"))?;
    for (key, value) in incoming {
        default[key] = value.clone();
    }
    Ok(serde_json::from_value(default)?)
}
pub(super) fn backend(compute: &ComputeConfig) -> Result<burn::BackendChoice> {
    backend_with(compute, burn::resolve_backend_with_fallback)
}

pub(super) fn concrete_compute(
    requested: &ComputeConfig,
    choice: &burn::BackendChoice,
) -> Result<ComputeConfig> {
    let (backend, device_index) = match choice {
        burn::BackendChoice::Auto => return Err(engine_error("backend resolution returned Auto")),
        burn::BackendChoice::Cpu => (ComputeBackend::Cpu, 0),
        burn::BackendChoice::Wgpu => (ComputeBackend::Wgpu, 0),
        burn::BackendChoice::WgpuAdapter { .. } => {
            return Err(engine_error(
                "runtime compute cannot describe a typed WGPU adapter",
            ));
        }
        burn::BackendChoice::Cuda { device } => (ComputeBackend::Cuda, *device),
        burn::BackendChoice::Rocm { device } => (ComputeBackend::Rocm, *device),
    };
    Ok(ComputeConfig {
        backend,
        device_index,
        allow_cpu_fallback: false,
        memory_limit_bytes: requested.memory_limit_bytes,
    })
}

fn backend_with(
    compute: &ComputeConfig,
    resolve: impl FnOnce(&burn::BackendChoice, bool) -> burn::Result<burn::BackendChoice>,
) -> Result<burn::BackendChoice> {
    compute.validate().map_err(engine_error)?;
    if matches!(
        compute.backend,
        ComputeBackend::Wgpu | ComputeBackend::Auto | ComputeBackend::Cpu
    ) && compute.device_index != 0
    {
        return Err(invalid(
            "generic CPU/WGPU/Auto selection requires device_index 0; use an explicit Burn WgpuAdapter with the lower-level engine for adapter selection",
        ));
    }
    let choice = match compute.backend {
        ComputeBackend::Cpu => burn::BackendChoice::Cpu,
        ComputeBackend::Wgpu => burn::BackendChoice::Wgpu,
        ComputeBackend::Cuda => burn::BackendChoice::Cuda {
            device: compute.device_index,
        },
        ComputeBackend::Rocm => burn::BackendChoice::Rocm {
            device: compute.device_index,
        },
        ComputeBackend::Auto => burn::BackendChoice::Auto,
    };
    resolve(&choice, compute.allow_cpu_fallback).map_err(engine_error)
}

#[cfg(test)]
pub(super) fn predict(bytes: &[u8], input: &TensorData, compute: &ComputeConfig) -> Result<Value> {
    let directory = tempfile::tempdir()?;
    unpack(bytes, directory.path(), compute.memory_limit_bytes)?;
    let predictor =
        burn::Predictor::load(directory.path(), backend(compute)?).map_err(engine_error)?;
    let prediction = predictor
        .predict(&burn::TensorData {
            shape: input.shape.clone(),
            values: input.values.clone(),
        })
        .map_err(engine_error)?;
    Ok(serde_json::to_value(prediction)?)
}

#[cfg(feature = "onnx-export")]
pub(super) fn export(
    bytes: &[u8],
    input: &TensorData,
    compute: &ComputeConfig,
) -> Result<(Vec<u8>, burn::OnnxExportReport)> {
    let directory = tempfile::tempdir()?;
    unpack(bytes, directory.path(), compute.memory_limit_bytes)?;
    let predictor =
        burn::Predictor::load(directory.path(), backend(compute)?).map_err(engine_error)?;
    let path = directory.path().join("model.onnx");
    let report = predictor
        .export_onnx(
            &burn::TensorData {
                shape: input.shape.clone(),
                values: input.values.clone(),
            },
            &path,
        )
        .map_err(engine_error)?;
    if fs::metadata(&path)?.len() > compute.memory_limit_bytes {
        return Err(invalid(
            "ONNX artifact exceeds the configured memory budget",
        ));
    }
    Ok((fs::read(path)?, report))
}

// The envelope contains three fixed payloads, never archive paths. Its state points at
// one fixed directory on extraction, so model-supplied traversal cannot escape a temp root.
const MAGIC: &[u8; 8] = b"FLMLB001";
fn pack(directory: &Path, optimizer: bool, limit: u64) -> Result<Vec<u8>> {
    pack_named(directory, burn::MANIFEST_FILE, optimizer, limit)
}
pub(super) fn pack_named(
    directory: &Path,
    manifest: &str,
    optimizer: bool,
    limit: u64,
) -> Result<Vec<u8>> {
    let state_path = directory.join(manifest);
    if fs::metadata(&state_path)?.len() > 16 * 1024 * 1024 {
        return Err(invalid("model state exceeds metadata size limit"));
    }
    let mut state: Value = serde_json::from_slice(&fs::read(state_path)?)?;
    let checkpoint = state["checkpoint"]
        .as_str()
        .ok_or_else(|| invalid("checkpoint pointer is missing"))?;
    let checkpoint_prefix = if manifest == burn::SAM2_MANIFEST_FILE {
        "sam2-checkpoint-"
    } else {
        "checkpoint-"
    };
    if !checkpoint.starts_with(checkpoint_prefix)
        || checkpoint.contains('/')
        || checkpoint.contains('\\')
        || checkpoint.contains("..")
    {
        return Err(invalid("invalid checkpoint directory"));
    }
    let checkpoint = directory.join(checkpoint);
    let model_path = checkpoint.join("model.bpk");
    let optimizer_path = checkpoint.join("optimizer.bpk");
    let model_len = fs::metadata(&model_path)?.len();
    let optimizer_len = if optimizer {
        fs::metadata(&optimizer_path)?.len()
    } else {
        0
    };
    state["checkpoint"] = Value::String(format!("{checkpoint_prefix}runtime"));
    let state = serde_json::to_vec(&state)?;
    let total = 32u64
        .checked_add(state.len() as u64)
        .and_then(|n| n.checked_add(model_len))
        .and_then(|n| n.checked_add(optimizer_len))
        .ok_or_else(|| invalid("artifact length overflow"))?;
    if total > limit || total > usize::MAX as u64 {
        return Err(invalid("artifact exceeds byte budget"));
    }
    let mut output = Vec::with_capacity(total as usize);
    output.extend_from_slice(MAGIC);
    for length in [state.len() as u64, model_len, optimizer_len] {
        output.extend_from_slice(&length.to_le_bytes());
    }
    output.extend_from_slice(&state);
    output.extend_from_slice(&fs::read(model_path)?);
    if optimizer {
        output.extend_from_slice(&fs::read(optimizer_path)?);
    }
    Ok(output)
}
pub(super) fn unpack(bytes: &[u8], directory: &Path, limit: u64) -> Result<()> {
    unpack_named(bytes, directory, burn::MANIFEST_FILE, limit)
}
pub(super) fn unpack_named(
    bytes: &[u8],
    directory: &Path,
    manifest: &str,
    limit: u64,
) -> Result<()> {
    if bytes.len() < 32 || bytes.len() as u64 > limit || &bytes[..8] != MAGIC {
        return Err(invalid("invalid or oversized Burn artifact envelope"));
    }
    let lengths: [u64; 3] = std::array::from_fn(|i| {
        u64::from_le_bytes(bytes[8 + i * 8..16 + i * 8].try_into().unwrap())
    });
    let total = lengths
        .iter()
        .try_fold(32u64, |n, length| n.checked_add(*length))
        .ok_or_else(|| invalid("artifact length overflow"))?;
    if total != bytes.len() as u64 || lengths[0] > 16 * 1024 * 1024 || lengths[1] == 0 {
        return Err(invalid("artifact lengths are inconsistent"));
    }
    let state_end = 32 + lengths[0] as usize;
    let model_end = state_end + lengths[1] as usize;
    let mut state: Value = serde_json::from_slice(&bytes[32..state_end])?;
    let checkpoint_name = if manifest == burn::SAM2_MANIFEST_FILE {
        "sam2-checkpoint-runtime"
    } else {
        "checkpoint-runtime"
    };
    state["checkpoint"] = Value::String(checkpoint_name.into());
    let checkpoint = directory.join(checkpoint_name);
    fs::create_dir_all(&checkpoint)?;
    fs::write(checkpoint.join("model.bpk"), &bytes[state_end..model_end])?;
    if lengths[2] > 0 {
        fs::write(checkpoint.join("optimizer.bpk"), &bytes[model_end..])?;
    }
    fs::write(directory.join(manifest), serde_json::to_vec(&state)?)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn runtime_backend_selection_preserves_auto_fallback_and_device_contracts() {
        let compute = ComputeConfig {
            backend: ComputeBackend::Auto,
            allow_cpu_fallback: false,
            ..Default::default()
        };
        let result = backend_with(&compute, |choice, fallback| {
            assert_eq!(choice, &burn::BackendChoice::Auto);
            assert!(!fallback);
            Ok(burn::BackendChoice::Cpu)
        })
        .unwrap();
        assert_eq!(result, burn::BackendChoice::Cpu);

        for (backend, expected) in [
            (
                ComputeBackend::Cuda,
                burn::BackendChoice::Cuda { device: 2 },
            ),
            (
                ComputeBackend::Rocm,
                burn::BackendChoice::Rocm { device: 2 },
            ),
        ] {
            let mut compute = ComputeConfig {
                backend,
                device_index: 2,
                allow_cpu_fallback: false,
                ..Default::default()
            };
            assert!(
                backend_with(&compute, |choice, fallback| {
                    assert_eq!(choice, &expected);
                    assert!(!fallback);
                    Err(burn::Error::Backend("driver unavailable".into()))
                })
                .is_err()
            );
            compute.allow_cpu_fallback = true;
            assert_eq!(
                backend_with(&compute, |choice, fallback| {
                    assert_eq!(choice, &expected);
                    assert!(fallback);
                    Ok(burn::BackendChoice::Wgpu)
                })
                .unwrap(),
                burn::BackendChoice::Wgpu
            );
        }
        for backend in [
            ComputeBackend::Cpu,
            ComputeBackend::Wgpu,
            ComputeBackend::Auto,
        ] {
            assert!(
                backend_with(
                    &ComputeConfig {
                        backend,
                        device_index: 1,
                        ..Default::default()
                    },
                    |_, _| {
                        panic!("invalid generic device indices must fail before initialization")
                    }
                )
                .is_err()
            );
        }
    }

    #[test]
    fn cpu_selection_and_resource_reservation_resolve_the_same_backend() {
        let compute = ComputeConfig {
            backend: ComputeBackend::Cpu,
            ..Default::default()
        };
        assert_eq!(backend(&compute).unwrap(), burn::BackendChoice::Cpu);
        assert_eq!(resource_key(&compute).unwrap(), "cpu:0");
        assert_eq!(backend(&compute).unwrap(), burn::BackendChoice::Cpu);
        if burn::compiled_backends() == vec!["cpu"] {
            let automatic = ComputeConfig {
                backend: ComputeBackend::Auto,
                allow_cpu_fallback: false,
                ..Default::default()
            };
            assert_eq!(backend(&automatic).unwrap(), burn::BackendChoice::Cpu);
            assert_eq!(resource_key(&automatic).unwrap(), "cpu:0");
        }
    }

    #[test]
    fn artifact_envelope_rejects_truncation_overflow_and_extra_bytes() {
        let dir = tempfile::tempdir().unwrap();
        assert!(unpack(b"FLMLB001", dir.path(), 1024).is_err());
        let mut corrupt = MAGIC.to_vec();
        for _ in 0..3 {
            corrupt.extend_from_slice(&u64::MAX.to_le_bytes());
        }
        assert!(unpack(&corrupt, dir.path(), 1024).is_err());
    }

    #[test]
    fn packed_state_configs_match_manifests_that_widened_f32_values() {
        let config = burn::TrainingConfig {
            recipe: burn::Recipe::Mlp {
                input_features: 1,
                hidden: 2,
                outputs: 1,
                objective: burn::Objective::Regression,
            },
            backend: burn::BackendChoice::Cpu,
            epochs: 1,
            batch_size: 1,
            learning_rate: 0.01,
            seed: 7,
            gradient_clip: 0.1,
        };
        let widened = serde_json::json!(config);
        let shortest: Value =
            serde_json::from_str(&serde_json::to_string_pretty(&config).unwrap()).unwrap();
        assert!(same_training_config(&shortest, &widened));
        let mut changed = widened.clone();
        changed["seed"] = serde_json::json!(8);
        assert!(!same_training_config(&shortest, &changed));
        assert!(!same_training_config(&shortest, &serde_json::json!({})));
    }

    #[test]
    fn mask_rcnn_sources_saved_above_the_roi_budget_still_initialize_fine_tunes() {
        let directory = tempfile::tempdir().unwrap();
        let repository = TrainingRepository::open(directory.path().join("jobs.sqlite")).unwrap();
        let saved = burn::TrainingConfig {
            recipe: burn::Recipe::MaskRcnn {
                config: burn::MaskRcnnConfig::default(),
            },
            backend: burn::BackendChoice::Cpu,
            epochs: 1,
            batch_size: 21,
            learning_rate: 0.001,
            seed: 1,
            gradient_clip: 5.0,
        };
        assert!(
            saved.validate().is_err(),
            "the saved batch exceeds the budget"
        );
        let labels = vec!["scratch".to_string(), "dent".to_string()];
        let source = repository
            .register_pretrained_source(
                "project",
                b"weights",
                serde_json::json!({"engine":"burn","format_version":1,"config":saved,"input_shape":[3,64,64],"labels":labels}),
                serde_json::json!({}),
                1024,
                0,
            )
            .unwrap();
        let target = burn::TrainingConfig {
            batch_size: 2,
            ..saved
        };
        target.validate().unwrap();
        let reference = PretrainedSourceRef {
            source_id: source.id,
            replace_head: false,
            freeze_backbone: false,
        };
        validate_burn_pretrained_source(&repository, "project", &reference, &target, &labels, 1024)
            .unwrap();
    }

    fn source_with(manifest: Value) -> PretrainedSource {
        PretrainedSource {
            id: "pretrained:fixture".into(),
            project_id: "project".into(),
            source_kind: PretrainedSourceKind::Imported,
            blob: BlobRef {
                sha256: "0".repeat(64),
                bytes: 1,
                path: format!("{}.blob", "0".repeat(64)),
            },
            manifest,
            origin: serde_json::json!({}),
            created_at_ms: 0,
        }
    }

    fn normalize(mean: f32, std: f32) -> PreprocessingStep {
        PreprocessingStep::Normalize {
            mean: vec![mean],
            std: vec![std],
        }
    }

    #[test]
    fn declared_image_normalization_must_match_the_source_recommendation() {
        let imagenet = source_with(serde_json::json!({"recommended_preprocessing":[
            {"kind":"resize","width":8,"height":8},
            {"kind":"normalize","mean":[0.485,0.456,0.406],"std":[0.229,0.224,0.225]}
        ]}));
        let resize = PreprocessingStep::Resize {
            width: 8,
            height: 8,
        };
        let error = validate_burn_pretrained_preprocessing(&imagenet, &[resize.clone()])
            .unwrap_err()
            .to_string();
        assert!(error.contains("expects images normalized"), "{error}");
        assert!(
            validate_burn_pretrained_preprocessing(
                &imagenet,
                &[resize.clone(), normalize(0.5, 0.5)]
            )
            .is_err()
        );
        validate_burn_pretrained_preprocessing(
            &imagenet,
            &[
                resize.clone(),
                PreprocessingStep::Normalize {
                    mean: vec![0.485, 0.456, 0.406],
                    std: vec![0.229, 0.224, 0.225],
                },
            ],
        )
        .unwrap();
        // (x - 0.4) / 0.25 followed by (y - 0.3) / 0.9 is (x - 0.475) / 0.225.
        validate_burn_pretrained_preprocessing(
            &imagenet,
            &[normalize(0.4, 0.25), normalize(0.3, 0.9)],
        )
        .unwrap();
        validate_burn_pretrained_preprocessing(&imagenet, &[]).unwrap();
        validate_burn_pretrained_preprocessing(
            &imagenet,
            &[PreprocessingStep::Features {
                names: vec!["feature".into()],
            }],
        )
        .unwrap();
        validate_burn_pretrained_preprocessing(&source_with(serde_json::json!({})), &[resize])
            .unwrap();
    }

    #[test]
    fn fine_tune_inputs_far_outside_the_source_normalization_are_rejected() {
        let imagenet = source_with(serde_json::json!({"recommended_preprocessing":[
            {"kind":"resize","width":2,"height":1},
            {"kind":"normalize","mean":[0.485,0.456,0.406],"std":[0.229,0.224,0.225]}
        ]}));
        let images = |values: Vec<f32>| burn::TensorData {
            shape: vec![1, 3, 1, 2],
            values,
        };
        let normalized = images(vec![-2.1, 2.2, -2.0, 2.4, -1.8, 2.6]);
        // Unnormalized [0, 1] pixels stay in range; the probe only rejects what cannot fit.
        let unit = images(vec![0.0, 1.0, 0.2, 0.8, 0.5, 0.5]);
        validate_pretrained_input_range(&imagenet, &[&normalized, &unit]).unwrap();
        let raw = images(vec![0.0, 255.0, 12.0, 200.0, 90.0, 30.0]);
        let error = validate_pretrained_input_range(&imagenet, &[&normalized, &raw])
            .unwrap_err()
            .to_string();
        assert!(error.contains("raw 0-255 pixels"), "{error}");
        let identity = source_with(serde_json::json!({"recommended_preprocessing":[
            {"kind":"normalize","mean":[0.0],"std":[1.0]}
        ]}));
        assert!(validate_pretrained_input_range(&identity, &[&raw]).is_err());
        validate_pretrained_input_range(&identity, &[&normalized]).unwrap();
        let unnormalized = source_with(serde_json::json!({"recommended_preprocessing":[
            {"kind":"resize","width":2,"height":1}
        ]}));
        validate_pretrained_input_range(&unnormalized, &[&raw]).unwrap();
        validate_pretrained_input_range(&source_with(serde_json::json!({})), &[&raw]).unwrap();
    }

    fn reserve_trial(repository: &TrainingRepository, job_id: &str, reserved_artifact_bytes: u64) {
        let trial = ExperimentTrial {
            id: format!("trial-{job_id}"),
            experiment_id: format!("experiment-{job_id}"),
            candidate_index: 0,
            idempotency_key: "candidate".into(),
            job_id: job_id.into(),
            dataset_snapshot_id: "dataset".into(),
            snapshot_id: "training".into(),
            artifact_id: None,
            status: ExperimentTrialStatus::Scheduled,
            validation_metrics: Default::default(),
            training_time_ms: 0,
            reserved_training_time_ms: 0,
            reserved_artifact_bytes,
            error: None,
            created_at_ms: 0,
            updated_at_ms: 0,
        };
        repository
            .connection()
            .unwrap()
            .execute(
                "INSERT INTO experiment_trials(id,experiment_id,job_id,idempotency_key,candidate_index,body) VALUES (?,?,?,?,0,?)",
                rusqlite::params![
                    trial.id,
                    trial.experiment_id,
                    trial.job_id,
                    trial.idempotency_key,
                    serde_json::to_string(&trial).unwrap()
                ],
            )
            .unwrap();
    }

    #[test]
    fn trial_checkpoints_charge_only_their_model_weights_to_the_artifact_reservation() {
        let directory = tempfile::tempdir().unwrap();
        let repository = TrainingRepository::open(directory.path().join("jobs.sqlite")).unwrap();
        let state = tempfile::tempdir().unwrap();
        let checkpoint = state.path().join("checkpoint-3");
        fs::create_dir(&checkpoint).unwrap();
        fs::write(checkpoint.join("model.bpk"), vec![1u8; 1000]).unwrap();
        fs::write(checkpoint.join("optimizer.bpk"), vec![2u8; 2000]).unwrap();
        fs::write(
            state.path().join(burn::MANIFEST_FILE),
            br#"{"checkpoint":"checkpoint-3"}"#,
        )
        .unwrap();
        let weights = pack(state.path(), false, u64::MAX).unwrap().len() as u64;
        let pack_for = |job_id: &str, maximum_bytes: u64| {
            pack_checkpoint(
                &repository,
                job_id,
                state.path(),
                burn::MANIFEST_FILE,
                maximum_bytes,
            )
        };
        let whole = pack_for("lifecycle", u64::MAX).unwrap();
        assert_eq!(checkpoint_model_bytes(&whole), weights);
        assert_eq!(whole.len() as u64, weights + 2000);
        reserve_trial(&repository, "fits", weights);
        assert_eq!(pack_for("fits", u64::MAX).unwrap(), whole);
        reserve_trial(&repository, "oversized", weights - 1);
        let error = pack_for("oversized", u64::MAX).unwrap_err().to_string();
        assert!(error.contains("artifact reservation"), "{error}");
        assert!(
            pack_for("fits", whole.len() as u64 - 1).is_err(),
            "the checkpoint limit still bounds the optimizer state"
        );
    }

    /// Records a reviewed sample at time `index`, which also names its own group.
    fn record(
        repository: &TrainingRepository,
        stream: &StreamKey,
        index: i64,
        input: TensorData,
        annotation: Annotation,
    ) {
        let sample = Sample {
            id: format!("{}-{index}", stream.stream_id),
            group_id: format!("{}-group-{index}", stream.stream_id),
            stream_id: stream.stream_id.clone(),
            timestamp_ms: index,
            window_start_ms: index,
            window_end_ms: index,
            input,
            annotation,
            provenance: flow_like_ml_core::LabelProvenance::Reviewed {
                reviewer: "fixture".into(),
                reviewed_at_ms: index,
            },
            outcome: None,
        };
        repository
            .record_sample(
                stream,
                &TrainingSample {
                    id: sample.id.clone(),
                    annotation_revision: 1,
                    group_id: sample.group_id.clone(),
                    captured_at_ms: index,
                    label_available_at_ms: index,
                    content_digest: format!("{index:064x}"),
                    source: LabelSource::Reviewed,
                    accepted: true,
                    payload: serde_json::to_value(sample).unwrap(),
                },
            )
            .unwrap();
    }

    /// Queues `engine` on the recorded samples, split at times 5 and 8, as an experiment trial
    /// whose artifact reservation cannot hold the model.
    fn undersized_trial(
        repository: &TrainingRepository,
        stream: &StreamKey,
        samples: usize,
        engine: &str,
        config: Value,
    ) -> TrainingJob {
        let job = repository
            .trigger_after_n(
                stream,
                samples,
                TrainingRequest {
                    engine: engine.into(),
                    recipe: serde_json::json!({"config":config,"labels":[]}),
                    compute: serde_json::to_value(ComputeConfig {
                        backend: ComputeBackend::Cpu,
                        ..Default::default()
                    })
                    .unwrap(),
                },
                SplitPolicy::Time {
                    train_end_ms: 5,
                    validation_end_ms: 8,
                    embargo_ms: 0,
                },
                100,
            )
            .unwrap()
            .unwrap();
        reserve_trial(repository, &job.id, 64);
        job
    }

    fn assert_failed_without_checkpoint(repository: &TrainingRepository, job_id: &str) {
        let mut worker = TrainingWorker::new(repository.clone(), WorkerLimits::default()).unwrap();
        register_burn_engine(&mut worker).unwrap();
        register_efficient_ad_engine(&mut worker).unwrap();
        let error = worker.run(job_id).unwrap_err().to_string();
        assert!(error.contains("artifact reservation"), "{error}");
        let failed = repository.get_job(job_id).unwrap();
        assert_eq!(failed.status, JobStatus::Failed);
        assert!(
            failed.checkpoint.is_none(),
            "the stop path retained the checkpoint the epoch publish refused"
        );
    }

    #[test]
    fn trials_whose_weights_exceed_their_reservation_fail_without_retaining_a_checkpoint() {
        let directory = tempfile::tempdir().unwrap();
        let repository = TrainingRepository::open(directory.path().join("jobs.sqlite")).unwrap();
        let stream = StreamKey {
            project_id: "project".into(),
            stream_id: "line".into(),
            inspection_version: "v1".into(),
        };
        for index in 0..12 {
            let x = index as f32 / 12.0;
            let input = TensorData {
                shape: vec![1],
                values: vec![x],
            };
            let target = Annotation::Scalar {
                value: f64::from(2.0 * x),
            };
            record(&repository, &stream, index, input, target);
        }
        let config = burn::TrainingConfig {
            recipe: burn::Recipe::Mlp {
                input_features: 1,
                hidden: 4,
                outputs: 1,
                objective: burn::Objective::Regression,
            },
            backend: burn::BackendChoice::Cpu,
            epochs: 2,
            batch_size: 4,
            learning_rate: 0.01,
            seed: 3,
            gradient_clip: 1.0,
        };
        let job = undersized_trial(
            &repository,
            &stream,
            12,
            "burn",
            serde_json::to_value(config).unwrap(),
        );
        assert_failed_without_checkpoint(&repository, &job.id);
    }

    /// The smallest valid EfficientAD model: a one-channel PDN-small teacher.
    fn smallest_efficient_ad() -> burn::EfficientAdConfig {
        let convolution = |shape: [usize; 4]| burn::ConvWeights {
            weight: burn::TensorData {
                shape: shape.to_vec(),
                values: vec![0.01; shape.iter().product()],
            },
            bias: burn::TensorData {
                shape: vec![shape[0]],
                values: vec![0.01; shape[0]],
            },
        };
        burn::EfficientAdConfig {
            teacher: burn::PdnTeacherWeights {
                input_channels: 1,
                base_channels: 1,
                feature_channels: 1,
                convolutions: [[1, 1, 4, 4], [2, 1, 4, 4], [2, 2, 3, 3], [1, 2, 4, 4]]
                    .into_iter()
                    .map(convolution)
                    .collect(),
            },
            backend: burn::BackendChoice::Cpu,
            epochs: 1,
            batch_size: 1,
            learning_rate: 0.001,
            seed: 42,
            autoencoder_channels: 2,
            hard_quantile: 0.99,
            gradient_clip: 5.0,
        }
    }

    #[test]
    fn efficient_ad_trials_whose_weights_exceed_their_reservation_retain_no_checkpoint() {
        let directory = tempfile::tempdir().unwrap();
        let repository = TrainingRepository::open(directory.path().join("jobs.sqlite")).unwrap();
        let stream = StreamKey {
            project_id: "project".into(),
            stream_id: "surface".into(),
            inspection_version: "v1".into(),
        };
        for index in [1, 6, 9] {
            let input = TensorData {
                shape: vec![1, 256, 256],
                values: (0..256 * 256)
                    .map(|i| ((i % 256) as f32 * 0.03 + index as f32).sin())
                    .collect(),
            };
            let normal = Annotation::Anomaly { is_anomaly: false };
            record(&repository, &stream, index, input, normal);
        }
        let config = serde_json::to_value(smallest_efficient_ad()).unwrap();
        let job = undersized_trial(&repository, &stream, 3, "efficient_ad", config);
        assert_failed_without_checkpoint(&repository, &job.id);
    }

    #[test]
    fn attention_backbones_split_inference_batches_by_their_quadratic_workspace() {
        let dinov2 = burn::Recipe::DinoV2 {
            config: burn::DinoV2Config::default(),
        };
        let large = [3, 896, 896];
        let per_image = attention_workspace_bytes(&dinov2, &large).unwrap();
        let tokens = 64u128 * 64 + 1;
        assert!(per_image > tokens * tokens * 6 * 16);
        let budget = 2 * 1024 * 1024 * 1024u64;
        let limit = inference_batch_limit(&dinov2, &large, budget).unwrap();
        assert_eq!(limit as u128, u128::from(budget) / per_image);
        assert!(limit < 11, "11 such images must not share one forward pass");
        assert!(
            inference_batch_limit(&dinov2, &large, 154 * 1024 * 1024)
                .unwrap_err()
                .to_string()
                .contains("per image")
        );
        let small = [3, 28, 28];
        assert!(inference_batch_limit(&dinov2, &small, budget).unwrap() > 1000);
        let resnet = burn::Recipe::ResNet18 {
            input_channels: 3,
            classes: 2,
            base_channels: 8,
        };
        assert_eq!(attention_workspace_bytes(&resnet, &large), None);
        assert_eq!(
            inference_batch_limit(&resnet, &large, 1).unwrap(),
            usize::MAX
        );
    }
}
