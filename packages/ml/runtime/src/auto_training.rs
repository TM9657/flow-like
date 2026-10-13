//! Bounded model search. Validation ranks candidates; the final holdout is read after selection.
use crate::{
    engines::{BurnEngineRecipe, LoadedArtifactPredictor, SNAPSHOT_MEMORY_FACTOR, serialized_size},
    worker::{LocalRunLock, TrainingWorker},
    *,
};
use flow_like_ml_burn as burn;
use flow_like_ml_core::{
    Annotation, ComputeBackend, ComputeConfig, InspectionSpec, LabelProvenance, Sample, TaskKind,
    TensorData,
};
use flow_like_ml_native::evaluation as metrics;
use std::{
    collections::{BTreeMap, BTreeSet, HashMap, HashSet},
    sync::Arc,
    time::{Duration, Instant},
};

/// Marks a candidate appended with a smaller batch in place of the oversized original.
const RESIZED_FROM: &str = "resized_from_candidate";
/// Event naming what kept a scheduled trial from starting.
const TRIAL_BLOCKED: &str = "trial_blocked";
/// Retries cannot fill the 256-event log, which also records skipped candidates.
const MAXIMUM_BLOCKED_EVENTS: usize = 32;
/// Pause before the first retry of a held-back step; later pauses double.
const FIRST_RETRY_DELAY_MS: u64 = 250;
/// Longest pause between retries, which also bounds how late `run` sees a cancellation.
const MAXIMUM_RETRY_DELAY_MS: u64 = 10_000;

pub trait SearchPredictor {
    /// Input has a leading batch dimension. Return native scores or Burn-shaped predictions.
    fn predict(&mut self, input: &TensorData) -> Result<Value>;
}

pub trait SearchPredictorFactory: Send + Sync {
    fn load(
        &self,
        repository: &TrainingRepository,
        artifact_id: &str,
        compute: &ComputeConfig,
    ) -> Result<Box<dyn SearchPredictor>>;
}

#[derive(Default)]
pub struct NativeSearchPredictorFactory;
impl SearchPredictor for LoadedArtifactPredictor {
    fn predict(&mut self, input: &TensorData) -> Result<Value> {
        LoadedArtifactPredictor::predict(self, input)
    }
}
impl SearchPredictorFactory for NativeSearchPredictorFactory {
    fn load(
        &self,
        repository: &TrainingRepository,
        artifact_id: &str,
        compute: &ComputeConfig,
    ) -> Result<Box<dyn SearchPredictor>> {
        Ok(Box::new(LoadedArtifactPredictor::load(
            repository,
            artifact_id,
            compute,
        )?))
    }
}

pub fn default_goals(spec: &InspectionSpec) -> ExperimentGoals {
    let (task, primary, direction) = match spec.task {
        TaskKind::SensorRegression | TaskKind::SequenceForecast => (
            EvaluationTask::Regression,
            "rmse",
            MetricDirection::Minimize,
        ),
        TaskKind::Fusion if spec.labels.is_empty() => (
            EvaluationTask::Regression,
            "rmse",
            MetricDirection::Minimize,
        ),
        TaskKind::SensorAnomaly | TaskKind::VisualAnomaly | TaskKind::SequenceAutoencoder => (
            EvaluationTask::Anomaly { threshold: 0.5 },
            "auroc",
            MetricDirection::Maximize,
        ),
        TaskKind::ObjectDetection => (
            EvaluationTask::Detection {
                classes: spec.labels.len(),
                iou_threshold: 0.5,
            },
            "mean_average_precision",
            MetricDirection::Maximize,
        ),
        TaskKind::Segmentation => (
            EvaluationTask::Segmentation {
                classes: spec.labels.len(),
            },
            "mean_iou",
            MetricDirection::Maximize,
        ),
        TaskKind::InstanceSegmentation => (
            EvaluationTask::InstanceSegmentation {
                classes: spec.labels.len(),
                iou_threshold: 0.5,
            },
            "mask_mean_average_precision",
            MetricDirection::Maximize,
        ),
        _ => (
            EvaluationTask::Classification,
            "accuracy",
            MetricDirection::Maximize,
        ),
    };
    ExperimentGoals {
        task,
        primary_metric: primary.into(),
        direction,
        minimum_audited_samples: 1,
        bounds: Vec::new(),
    }
}

/// Validate search limits independently of whether the task has generated candidates.
pub fn validate_search(
    spec: &InspectionSpec,
    search: &AutoSearchConfig,
    compute: &ComputeConfig,
) -> Result<()> {
    spec.validate().map_err(engine_error)?;
    compute.validate().map_err(engine_error)?;
    if search.maximum_candidates == 0
        || search.maximum_candidates > 64
        || search.min_epochs == 0
        || search.max_epochs < search.min_epochs
        || search.max_epochs > 1000
        || search.batch_size == 0
        || search.batch_size > 128
        || search.output_features == 0
        || search.output_features > 4096
        || !(2..=8).contains(&search.reduction_factor)
    {
        return Err(invalid(
            "search requires 1..64 candidates, ordered epochs up to 1000, batch 1..128 and bounded output width",
        ));
    }
    Ok(())
}

/// Reject a selection metric or bound that validation never measures for this task. Selection
/// would otherwise rank every trial as ineligible and leave the bound to the final audit.
pub fn validate_goals(spec: &InspectionSpec, goals: &ExperimentGoals) -> Result<()> {
    for name in std::iter::once(&goals.primary_metric).chain(goals.bounds.iter().map(|b| &b.name)) {
        if !validation_measures(&goals.task, &spec.labels, name) {
            return Err(invalid(format!(
                "goal metric {name} is not measured on the validation partition for this task"
            )));
        }
    }
    Ok(())
}

fn validation_measures(task: &EvaluationTask, labels: &[String], name: &str) -> bool {
    if matches!(
        name,
        "audited_samples" | "mean_latency_ms" | "failure_rate" | "artifact_bytes"
    ) {
        return true;
    }
    let class = |value: &str, classes: usize| value.parse::<usize>().is_ok_and(|c| c < classes);
    let per_class = |prefixes: &[&str], classes: usize| {
        prefixes.iter().any(|prefix| {
            name.strip_prefix(*prefix)
                .is_some_and(|value| class(value, classes))
        })
    };
    match task {
        EvaluationTask::Classification => {
            matches!(
                name,
                "accuracy"
                    | "accuracy_wilson_lower_95"
                    | "macro_f1"
                    | "balanced_accuracy"
                    | "mean_rank_error"
                    | "macro_rank_error"
                    | "quadratic_kappa"
            ) || ["support/", "recall/"].iter().any(|prefix| {
                name.strip_prefix(*prefix)
                    .is_some_and(|label| labels.iter().any(|known| known == label))
            }) || name
                .strip_prefix("class/")
                .and_then(|rest| rest.split_once('/'))
                .is_some_and(|(index, metric)| {
                    class(index, labels.len()) && matches!(metric, "support" | "recall")
                })
        }
        EvaluationTask::Regression => matches!(name, "rmse" | "mae" | "r2"),
        EvaluationTask::Anomaly { .. } => matches!(
            name,
            "auroc"
                | "average_precision"
                | "accuracy"
                | "accuracy_wilson_lower_95"
                | "brier_score"
                | "log_loss"
                | "precision"
                | "recall"
                | "false_positive_rate"
                | "support/normal"
                | "support/anomaly"
                | "anomaly_scale"
        ),
        EvaluationTask::Detection { classes, .. } => {
            name == "mean_average_precision"
                || per_class(
                    &["support/", "recall/", "precision/", "average_precision/"],
                    *classes,
                )
        }
        EvaluationTask::Segmentation { classes } => {
            matches!(name, "mean_iou" | "mean_dice") || per_class(&["iou/", "dice/"], *classes)
        }
        EvaluationTask::InstanceSegmentation { classes, .. } => {
            matches!(name, "mask_mean_average_precision" | "mask_mean_iou")
                || per_class(
                    &[
                        "support/",
                        "predicted/",
                        "precision/",
                        "recall/",
                        "mask_average_precision/",
                        "mask_iou/",
                    ],
                    *classes,
                )
        }
        EvaluationTask::Events { .. } => false,
    }
}

/// Generate a reproducible, bounded set of actual supported training recipes. Burn batches are
/// halved until the model fits the compute memory limit; admission repeats the check with the
/// snapshot, pretrained source and worker budget.
pub fn generate_candidates(
    spec: &InspectionSpec,
    search: &AutoSearchConfig,
    compute: &ComputeConfig,
) -> Result<Vec<TrainingRequest>> {
    validate_search(spec, search, compute)?;
    let native_allowed = matches!(compute.backend, ComputeBackend::Cpu | ComputeBackend::Auto)
        || compute.allow_cpu_fallback;
    let memory_budget = compute.memory_limit_bytes;
    let shape = &spec.input_shape;
    let classification = !spec.labels.is_empty()
        && !matches!(
            spec.task,
            TaskKind::SensorAnomaly | TaskKind::VisualAnomaly | TaskKind::SequenceAutoencoder
        );
    let objective = if classification {
        burn::Objective::Classification
    } else {
        burn::Objective::Regression
    };
    let outputs = if classification {
        spec.labels.len()
    } else {
        search.output_features
    };
    let mut recipes = Vec::new();
    let mut candidates = Vec::new();
    let compute = serde_json::to_value(compute)?;
    let native = |engine: &str, parameters: Value| TrainingRequest {
        engine: engine.into(),
        recipe: parameters,
        compute: compute.clone(),
    };
    match spec.task {
        TaskKind::SensorClassification
        | TaskKind::SensorRegression
        | TaskKind::SequenceForecast => {
            if (classification && outputs == 2) || (!classification && outputs == 1) {
                for depth in [2, 4, 6] {
                    candidates.push(native("histogram_gradient_boosting", serde_json::json!({"labels":spec.labels,"inspection_task":spec.task,"config":{
                        "objective":if classification {"binary_log_loss"} else {"squared_error"},
                        "estimators":64,"max_depth":depth,"max_bins":32,"min_leaf":2,"learning_rate":0.08,"l2":1.0
                    }})));
                }
            }
            match shape.as_slice() {
                [features] => {
                    for hidden in [16, 32, 64] {
                        recipes.push(burn::Recipe::Mlp {
                            input_features: *features,
                            hidden,
                            outputs,
                            objective,
                        });
                    }
                }
                [_, features] => {
                    for hidden in [16, 32] {
                        recipes.extend([
                            burn::Recipe::Lstm {
                                input_features: *features,
                                hidden,
                                outputs,
                                objective,
                            },
                            burn::Recipe::Gru {
                                input_features: *features,
                                hidden,
                                outputs,
                                objective,
                            },
                            burn::Recipe::Cnn1d {
                                input_features: *features,
                                hidden,
                                outputs,
                                objective,
                            },
                            burn::Recipe::Tcn {
                                input_features: *features,
                                hidden,
                                outputs,
                                objective,
                                levels: 3,
                            },
                        ]);
                    }
                }
                _ => {
                    return Err(invalid(
                        "sensor search expects feature vectors or [time, features] windows",
                    ));
                }
            }
        }
        TaskKind::SensorAnomaly | TaskKind::VisualAnomaly | TaskKind::SequenceAutoencoder => {
            for trees in [64, 128] {
                candidates.push(native("isolation_forest", serde_json::json!({"labels":spec.labels,"inspection_task":spec.task,"config":{"trees":trees,"sample_size":256,"seed":search.seed}})));
            }
            match shape.as_slice() {
                [features] => {
                    for hidden in [16, 32] {
                        recipes.push(burn::Recipe::DenseAutoencoder {
                            input_features: *features,
                            hidden,
                            latent: 8,
                        });
                    }
                }
                [_, features] if spec.task != TaskKind::VisualAnomaly => {
                    for hidden in [16, 32] {
                        recipes.extend([
                            burn::Recipe::LstmAutoencoder {
                                input_features: *features,
                                hidden,
                                latent: 8,
                            },
                            burn::Recipe::Conv1dAutoencoder {
                                input_features: *features,
                                hidden,
                                latent: 8,
                            },
                        ]);
                    }
                }
                [_, _] => {
                    candidates.push(native("patchcore", serde_json::json!({"labels":spec.labels,"inspection_task":spec.task,"coreset_size":128})));
                    candidates.push(native("padim", serde_json::json!({"labels":spec.labels,"inspection_task":spec.task,"regularization":0.01})));
                }
                _ => {}
            }
        }
        TaskKind::ImageClassification => {
            let [channels, _, _] = shape.as_slice() else {
                return Err(invalid("image search expects [channels,height,width]"));
            };
            recipes.extend([
                burn::Recipe::ResNet18 {
                    input_channels: *channels,
                    classes: outputs,
                    base_channels: 16,
                },
                burn::Recipe::ResNet18 {
                    input_channels: *channels,
                    classes: outputs,
                    base_channels: 32,
                },
                burn::Recipe::MobileNetV2 {
                    input_channels: *channels,
                    classes: outputs,
                    width_multiplier: 0.5,
                },
                burn::Recipe::EfficientNet {
                    input_channels: *channels,
                    classes: outputs,
                    width_multiplier: 0.5,
                },
            ]);
        }
        TaskKind::ObjectDetection => {
            let [channels, _, _] = shape.as_slice() else {
                return Err(invalid("detection search expects image tensors"));
            };
            for width in [0.25, 0.5] {
                recipes.push(burn::Recipe::YoloX {
                    input_channels: *channels,
                    classes: outputs,
                    width_multiplier: width,
                    depth_multiplier: 0.33,
                });
            }
        }
        TaskKind::Segmentation => {
            let [channels, _, _] = shape.as_slice() else {
                return Err(invalid("segmentation search expects image tensors"));
            };
            for channels_base in [8, 16, 32] {
                recipes.push(burn::Recipe::UNet {
                    input_channels: *channels,
                    classes: outputs,
                    base_channels: channels_base,
                    depth: 2,
                });
            }
        }
        TaskKind::InstanceSegmentation => {
            let [channels, _, _] = shape.as_slice() else {
                return Err(invalid("instance search expects image tensors"));
            };
            for channels_base in [8, 16] {
                recipes.push(burn::Recipe::MaskRcnn {
                    config: burn::MaskRcnnConfig {
                        input_channels: *channels,
                        classes: outputs,
                        base_channels: channels_base,
                        ..Default::default()
                    },
                });
            }
        }
        TaskKind::VisualSequenceClassification => {
            let [_, channels, _, _] = shape.as_slice() else {
                return Err(invalid(
                    "visual sequence search expects [time,channels,height,width]",
                ));
            };
            for hidden in [16, 32] {
                recipes.push(burn::Recipe::CnnLstm {
                    input_channels: *channels,
                    cnn_channels: 16,
                    hidden,
                    outputs,
                    objective,
                });
            }
        }
        TaskKind::Fusion => {
            return Err(invalid(
                "automatic fusion search needs an explicit recipe defining image dimensions and sensor features",
            ));
        }
    }
    // Native models run on CPU only; a strict GPU request must not silently become CPU training.
    let skipped_native = !native_allowed && !candidates.is_empty();
    if skipped_native {
        candidates.clear();
    }
    let mut rng = SearchRng(search.seed);
    let mut smallest_unfit: Option<u64> = None;
    for recipe in recipes {
        let mut config = burn::TrainingConfig {
            recipe,
            backend: burn::BackendChoice::Auto,
            epochs: search.min_epochs,
            batch_size: search
                .batch_size
                .min(if spec.task == TaskKind::InstanceSegmentation {
                    32
                } else {
                    128
                }),
            learning_rate: if rng.next() % 2 == 0 { 0.003 } else { 0.01 },
            seed: search.seed,
            gradient_clip: 5.0,
        };
        if burn::validate_inspection_config(spec, &config).is_err() {
            continue;
        }
        match fit_batch(&config.recipe, shape, config.batch_size, 0, memory_budget) {
            Some(Ok(batch)) => config.batch_size = batch,
            Some(Err(estimate)) => {
                smallest_unfit = Some(smallest_unfit.map_or(estimate, |s| s.min(estimate)));
                continue;
            }
            None => {}
        }
        let family = digest(&serde_json::to_vec(&config)?);
        candidates.push(TrainingRequest {engine:"burn".into(),recipe:serde_json::json!({"config":config,"labels":spec.labels,"inspection_task":spec.task,"search":{"family":family,"rung":0,"min_epochs":search.min_epochs,"max_epochs":search.max_epochs,"reduction_factor":search.reduction_factor}}),compute:compute.clone()});
    }
    for i in (1..candidates.len()).rev() {
        let j = rng.next() as usize % (i + 1);
        candidates.swap(i, j);
    }
    candidates.truncate(search.maximum_candidates);
    if candidates.is_empty() {
        let message = if let Some(estimate) = smallest_unfit {
            unfit_message(memory_budget, estimate)
        } else if skipped_native {
            "native candidates need CPU or allow_cpu_fallback, and no GPU recipe fits this task"
                .to_string()
        } else {
            "no supported candidate matches this shape and task".to_string()
        };
        return Err(invalid(message));
    }
    Ok(candidates)
}

fn unfit_message(budget: u64, estimate: u64) -> String {
    format!(
        "no candidate fits the memory budget of {budget} bytes; \
         the smallest estimate is {estimate} bytes"
    )
}

/// Halve `batch` until the training estimate plus `fixed_bytes` fits `budget`. Returns the
/// batch, the estimate at batch 1 when nothing fits, or None when the estimate is undefined.
fn fit_batch(
    recipe: &burn::Recipe,
    sample_shape: &[usize],
    mut batch: usize,
    fixed_bytes: u64,
    budget: u64,
) -> Option<std::result::Result<usize, u64>> {
    loop {
        let estimate = recipe
            .estimated_training_bytes(sample_shape, batch)
            .ok()?
            .saturating_add(fixed_bytes);
        if estimate <= budget {
            return Some(Ok(batch));
        }
        if batch <= 1 {
            return Some(Err(estimate));
        }
        batch /= 2;
    }
}

struct SearchRng(u64);
impl SearchRng {
    fn next(&mut self) -> u64 {
        self.0 = self.0.wrapping_add(0x9e3779b97f4a7c15);
        let mut x = self.0;
        x = (x ^ (x >> 30)).wrapping_mul(0xbf58476d1ce4e5b9);
        x = (x ^ (x >> 27)).wrapping_mul(0x94d049bb133111eb);
        x ^ (x >> 31)
    }
}

fn engine_error(error: impl std::fmt::Display) -> Error {
    Error::Engine(error.to_string())
}

struct Observation {
    sample: Sample,
    predicted_class: Option<u32>,
    evidence: Option<EvidencePrediction>,
    latency_ms: f64,
}

fn sealed_sample(
    row: &TrainingSample,
    spec: &InspectionSpec,
    as_of_ms: i64,
    audited: bool,
) -> Result<Sample> {
    let sample: Sample =
        serde_json::from_value(row.payload.get("sample").unwrap_or(&row.payload).clone())?;
    spec.validate_sample_at(&sample, as_of_ms)
        .map_err(engine_error)?;
    if !row.accepted
        || row.id != sample.id
        || row.group_id != sample.group_id
        || row.label_available_at_ms > as_of_ms
    {
        return Err(invalid(
            "snapshot contains an unavailable, unaccepted or mismatched sample",
        ));
    }
    if audited
        && !matches!(
            (&row.source, &sample.provenance),
            (LabelSource::Reviewed, LabelProvenance::Reviewed { .. })
                | (
                    LabelSource::ObservedOutcome,
                    LabelProvenance::Measured { .. }
                )
        )
    {
        return Err(invalid(
            "model selection and final evaluation require reviewed or measured labels",
        ));
    }
    Ok(sample)
}

fn predict_sample(predictor: &mut dyn SearchPredictor, sample: &Sample) -> Result<Value> {
    let mut input = sample.input.clone();
    input.shape.insert(0, 1);
    predictor.predict(&input)
}

fn floats(value: &Value) -> Result<Vec<f64>> {
    let values: Vec<f64> = serde_json::from_value(value.clone())?;
    if values.is_empty() || values.iter().any(|v| !v.is_finite()) {
        return Err(invalid(
            "prediction must contain finite nonempty numeric values",
        ));
    }
    Ok(values)
}

fn scalar_score(raw: &Value) -> Result<f64> {
    for key in ["scores", "reconstruction_error"] {
        if raw.get(key).is_some_and(Value::is_array) {
            return Ok(floats(&raw[key])?[0]);
        }
    }
    Err(invalid(
        "anomaly model did not return anomaly scores or reconstruction errors",
    ))
}

fn decode_prediction(
    sample: Sample,
    raw: &Value,
    task: &EvaluationTask,
    labels: usize,
    anomaly_scale: f64,
    latency_ms: f64,
) -> Result<Observation> {
    let mut observation = Observation {
        sample,
        predicted_class: None,
        evidence: None,
        latency_ms,
    };
    observation.evidence = match task {
        EvaluationTask::Classification => {
            let class = if let Some(class) = raw
                .get("classes")
                .and_then(Value::as_array)
                .and_then(|v| v.first())
                .and_then(Value::as_u64)
            {
                class as u32
            } else if raw.get("scores").is_some() && labels == 2 {
                u32::from(floats(&raw["scores"])?[0] >= 0.5)
            } else {
                let values = floats(&raw["output"]["values"])?;
                if values.len() != labels {
                    return Err(invalid("class scores differ from the label order"));
                }
                values
                    .iter()
                    .enumerate()
                    .max_by(|a, b| a.1.total_cmp(b.1))
                    .map(|(i, _)| i as u32)
                    .ok_or_else(|| invalid("empty class prediction"))?
            };
            if class as usize >= labels {
                return Err(invalid("predicted class is outside the label schema"));
            }
            observation.predicted_class = Some(class);
            None
        }
        EvaluationTask::Regression => Some(EvidencePrediction::Regression {
            values: floats(raw.get("scores").unwrap_or(&raw["output"]["values"]))?,
        }),
        EvaluationTask::Anomaly { .. } => {
            let score = scalar_score(raw)?;
            if score < 0.0 || !anomaly_scale.is_finite() || anomaly_scale <= 0.0 {
                return Err(invalid("invalid anomaly score calibration"));
            }
            Some(EvidencePrediction::Anomaly {
                probability: score / (score + anomaly_scale),
            })
        }
        EvaluationTask::Detection { .. } => {
            let detections: Vec<Vec<burn::Detection>> =
                serde_json::from_value(raw["detections"].clone())?;
            if detections.len() != 1 {
                return Err(invalid(
                    "detection predictor returned a different batch size",
                ));
            }
            // Boxes clamped to the image border can collapse to zero area; no truth can match
            // them and box validation would reject the whole partition. NaN boxes stay, so
            // validation still rejects a broken model.
            let boxes = detections
                .into_iter()
                .next()
                .unwrap_or_default()
                .into_iter()
                .filter(|d| !(d.x_max <= d.x_min || d.y_max <= d.y_min))
                .map(|d| DetectionEvidence {
                    class_id: d.class_id as u32,
                    x_min: d.x_min,
                    y_min: d.y_min,
                    x_max: d.x_max,
                    y_max: d.y_max,
                    confidence: d.confidence as f64,
                })
                .collect();
            Some(EvidencePrediction::Detection { boxes })
        }
        EvaluationTask::Segmentation { classes } => {
            let shape: Vec<usize> = serde_json::from_value(raw["output"]["shape"].clone())?;
            let [1, channels, height, width] = shape.as_slice() else {
                return Err(invalid(
                    "segmentation output must be [1,classes,height,width]",
                ));
            };
            if channels != classes {
                return Err(invalid("segmentation output class count differs"));
            }
            let values = floats(&raw["output"]["values"])?;
            let area = height
                .checked_mul(*width)
                .ok_or_else(|| invalid("segmentation area overflow"))?;
            if values.len() != area * channels {
                return Err(invalid("segmentation output length differs"));
            }
            let found = (0..area)
                .map(|pixel| {
                    (0..*channels)
                        .max_by(|a, b| {
                            values[a * area + pixel].total_cmp(&values[b * area + pixel])
                        })
                        .unwrap_or(0) as u32
                })
                .collect();
            Some(EvidencePrediction::Segmentation {
                width: *width,
                height: *height,
                classes: found,
            })
        }
        EvaluationTask::InstanceSegmentation { .. } => {
            let predictions: Vec<Vec<burn::InstanceDetection>> =
                serde_json::from_value(raw["instances"].clone())?;
            if predictions.len() != 1 {
                return Err(invalid(
                    "instance predictor returned a different batch size",
                ));
            }
            let mut instances = Vec::new();
            for instance in predictions.into_iter().next().unwrap_or_default() {
                let [height, width] = instance.mask.shape.as_slice() else {
                    return Err(invalid("instance mask must be an image grid"));
                };
                instances.push(InstanceEvidence {
                    class_id: instance.detection.class_id as u32,
                    confidence: instance.detection.confidence as f64,
                    width: *width,
                    height: *height,
                    foreground: instance.mask.values.into_iter().map(|v| v >= 0.5).collect(),
                });
            }
            Some(EvidencePrediction::Instances { instances })
        }
        EvaluationTask::Events { .. } => {
            return Err(invalid(
                "automatic event-interval scoring needs an explicit alarm policy; use classification or regression search",
            ));
        }
    };
    Ok(observation)
}

/// Convert an actual model output into the same evidence format used by final audits.
pub fn prediction_record_from_output(
    row: &TrainingSample,
    spec: &InspectionSpec,
    task: &EvaluationTask,
    artifact_id: &str,
    raw: &Value,
    anomaly_scale: f64,
    latency_ms: f64,
) -> Result<PredictionRecord> {
    let sample = sealed_sample(row, spec, now_ms(), true)?;
    let observation = decode_prediction(
        sample,
        raw,
        task,
        spec.labels.len(),
        anomaly_scale,
        latency_ms,
    )?;
    let actual_label = if let Annotation::Class { class_id } = observation.sample.annotation {
        Some(
            spec.labels
                .get(class_id as usize)
                .ok_or_else(|| invalid("audit class outside label order"))?
                .clone(),
        )
    } else {
        None
    };
    Ok(PredictionRecord {
        sample_id: row.id.clone(),
        artifact_id: artifact_id.into(),
        predicted_label: observation
            .predicted_class
            .map(|class| spec.labels[class as usize].clone()),
        teacher_label: None,
        actual_label,
        actual_source: Some(row.source),
        latency_ms,
        failed: false,
        recorded_at_ms: now_ms(),
        details: serde_json::json!({"prediction":observation.evidence,"partition":"learning_audit"}),
    })
}

fn measure(
    observations: &[Observation],
    task: &EvaluationTask,
    labels: &[String],
) -> Result<BTreeMap<String, f64>> {
    let classes = labels.len();
    if observations.is_empty() {
        return Err(invalid("held-out partition is empty"));
    }
    let mut values = BTreeMap::from([
        ("audited_samples".into(), observations.len() as f64),
        (
            "mean_latency_ms".into(),
            observations.iter().map(|v| v.latency_ms).sum::<f64>() / observations.len() as f64,
        ),
        ("failure_rate".into(), 0.0),
    ]);
    match task {
        EvaluationTask::Classification => {
            let actual = observations
                .iter()
                .map(|v| match v.sample.annotation {
                    Annotation::Class { class_id } => Ok(class_id),
                    _ => Err(invalid(
                        "classification validation requires class annotations",
                    )),
                })
                .collect::<Result<Vec<_>>>()?;
            let predicted = observations
                .iter()
                .map(|v| {
                    v.predicted_class
                        .ok_or_else(|| invalid("missing predicted class"))
                })
                .collect::<Result<Vec<_>>>()?;
            let result = metrics::classification_metrics(&actual, &predicted, classes)
                .map_err(engine_error)?;
            let ordinal =
                metrics::ordinal_metrics(&actual, &predicted, classes).map_err(engine_error)?;
            values.extend([
                ("mean_rank_error".into(), ordinal.mean_rank_error),
                ("macro_rank_error".into(), ordinal.macro_rank_error),
                ("quadratic_kappa".into(), ordinal.quadratic_kappa),
            ]);
            values.extend([
                ("accuracy".into(), result.accuracy),
                (
                    "accuracy_wilson_lower_95".into(),
                    result.accuracy_interval_95.lower,
                ),
                ("macro_f1".into(), result.macro_f1),
                ("balanced_accuracy".into(), result.balanced_accuracy),
            ]);
            for class in result.per_class {
                let label = &labels[class.class_id as usize];
                values.insert(format!("support/{label}"), class.support as f64);
                values.insert(
                    format!("class/{}/support", class.class_id),
                    class.support as f64,
                );
                if let Some(recall) = class.recall {
                    values.insert(format!("recall/{label}"), recall);
                    values.insert(format!("class/{}/recall", class.class_id), recall);
                }
            }
        }
        EvaluationTask::Regression => {
            let mut actual = Vec::new();
            let mut predicted = Vec::new();
            for observation in observations {
                let truth = match &observation.sample.annotation {
                    Annotation::Scalar { value } => vec![*value],
                    Annotation::Values { values } => values.iter().map(|v| *v as f64).collect(),
                    _ => return Err(invalid("regression validation requires numeric targets")),
                };
                let Some(EvidencePrediction::Regression { values: found }) = &observation.evidence
                else {
                    return Err(invalid("missing regression evidence"));
                };
                if truth.len() != found.len() {
                    return Err(invalid("regression output width differs from target"));
                }
                actual.extend(truth);
                predicted.extend(found);
            }
            let result = metrics::regression_metrics(&actual, &predicted).map_err(engine_error)?;
            values.insert("rmse".into(), result.rmse);
            values.insert("mae".into(), result.mae);
            if let Some(r2) = result.r2 {
                values.insert("r2".into(), r2);
            }
        }
        EvaluationTask::Anomaly { threshold } => {
            let mut actual = Vec::new();
            let mut scores = Vec::new();
            for observation in observations {
                let Annotation::Anomaly { is_anomaly } = observation.sample.annotation else {
                    return Err(invalid(
                        "anomaly validation needs reviewed normal/anomaly labels",
                    ));
                };
                let Some(EvidencePrediction::Anomaly { probability }) = observation.evidence else {
                    return Err(invalid("missing anomaly evidence"));
                };
                actual.push(is_anomaly);
                scores.push(probability);
            }
            // Same metric set as the final anomaly audit, so goal bounds can steer selection.
            let curve = metrics::binary_curve(&actual, &scores).map_err(engine_error)?;
            if let Some(auc) = curve.roc_auc {
                values.insert("auroc".into(), auc);
            }
            if let Some(ap) = curve.average_precision {
                values.insert("average_precision".into(), ap);
            }
            let truth: Vec<_> = actual.iter().map(|v| u32::from(*v)).collect();
            let predicted: Vec<_> = scores.iter().map(|v| u32::from(v >= threshold)).collect();
            let result =
                metrics::classification_metrics(&truth, &predicted, 2).map_err(engine_error)?;
            values.insert("accuracy".into(), result.accuracy);
            values.insert(
                "accuracy_wilson_lower_95".into(),
                result.accuracy_interval_95.lower,
            );
            values.insert("brier_score".into(), curve.brier_score);
            values.insert("log_loss".into(), curve.log_loss);
            let (normal, anomaly) = (&result.per_class[0], &result.per_class[1]);
            if let Some(precision) = anomaly.precision {
                values.insert("precision".into(), precision);
            }
            if let Some(recall) = anomaly.recall {
                values.insert("recall".into(), recall);
            }
            if normal.support > 0 {
                values.insert(
                    "false_positive_rate".into(),
                    anomaly.false_positive as f64 / normal.support as f64,
                );
            }
            values.insert("support/normal".into(), normal.support as f64);
            values.insert("support/anomaly".into(), anomaly.support as f64);
        }
        EvaluationTask::Detection {
            classes,
            iou_threshold,
        } => {
            let mut actual = Vec::new();
            let mut predicted = Vec::new();
            for observation in observations {
                let Annotation::Boxes { boxes } = &observation.sample.annotation else {
                    return Err(invalid("detection validation needs box annotations"));
                };
                let Some(EvidencePrediction::Detection { boxes: found }) = &observation.evidence
                else {
                    return Err(invalid("missing detection evidence"));
                };
                actual.push(boxes.clone());
                predicted.push(
                    found
                        .iter()
                        .map(|b| metrics::ScoredBox {
                            bounds: flow_like_ml_core::BoundingBox {
                                class_id: b.class_id,
                                x_min: b.x_min,
                                y_min: b.y_min,
                                x_max: b.x_max,
                                y_max: b.y_max,
                            },
                            confidence: b.confidence,
                        })
                        .collect(),
                );
            }
            let result = metrics::detection_metrics(&actual, &predicted, *classes, *iou_threshold)
                .map_err(engine_error)?;
            if let Some(ap) = result.mean_average_precision {
                values.insert("mean_average_precision".into(), ap);
            }
            for class in result.per_class {
                let id = class.class_id;
                values.insert(format!("support/{id}"), class.actual as f64);
                if let Some(recall) = class.recall {
                    values.insert(format!("recall/{id}"), recall);
                }
                if let Some(precision) = class.precision {
                    values.insert(format!("precision/{id}"), precision);
                }
                if let Some(ap) = class.average_precision {
                    values.insert(format!("average_precision/{id}"), ap);
                }
            }
        }
        EvaluationTask::Segmentation { classes } => {
            let mut actual = Vec::new();
            let mut predicted = Vec::new();
            for observation in observations {
                let Annotation::Mask {
                    width,
                    height,
                    classes: truth,
                } = &observation.sample.annotation
                else {
                    return Err(invalid("segmentation validation needs masks"));
                };
                let Some(EvidencePrediction::Segmentation {
                    width: pw,
                    height: ph,
                    classes: found,
                }) = &observation.evidence
                else {
                    return Err(invalid("missing segmentation evidence"));
                };
                if width != pw || height != ph || truth.len() != found.len() {
                    return Err(invalid("segmentation masks do not align"));
                }
                actual.extend(truth);
                predicted.extend(found);
            }
            let result = metrics::segmentation_metrics(&actual, &predicted, *classes)
                .map_err(engine_error)?;
            values.insert("mean_iou".into(), result.mean_iou);
            values.insert("mean_dice".into(), result.mean_dice);
            for (class, iou) in result.per_class_iou.into_iter().enumerate() {
                if let Some(iou) = iou {
                    values.insert(format!("iou/{class}"), iou);
                }
            }
            for (class, dice) in result.per_class_dice.into_iter().enumerate() {
                if let Some(dice) = dice {
                    values.insert(format!("dice/{class}"), dice);
                }
            }
        }
        EvaluationTask::InstanceSegmentation {
            classes,
            iou_threshold,
        } => {
            let mut actual = Vec::new();
            let mut predicted = Vec::new();
            for observation in observations {
                let Annotation::InstanceMasks { instances } = &observation.sample.annotation else {
                    return Err(invalid("instance validation needs masks"));
                };
                let Some(EvidencePrediction::Instances { instances: found }) =
                    &observation.evidence
                else {
                    return Err(invalid("missing instance evidence"));
                };
                actual.push(instances.clone());
                predicted.push(found.clone());
            }
            crate::evaluation::instance_metrics(
                &actual,
                &predicted,
                *classes,
                *iou_threshold,
                &mut values,
                &mut BTreeMap::new(),
            )?;
        }
        EvaluationTask::Events { .. } => {
            return Err(invalid("event intervals require an explicit alarm policy"));
        }
    }
    Ok(values)
}

pub struct AutoTrainingController {
    repository: TrainingRepository,
    worker: TrainingWorker,
    predictors: Arc<dyn SearchPredictorFactory>,
    clock: Arc<dyn Fn() -> i64 + Send + Sync>,
}
impl AutoTrainingController {
    pub fn new(repository: TrainingRepository, worker: TrainingWorker) -> Self {
        Self {
            repository,
            worker,
            predictors: Arc::new(NativeSearchPredictorFactory),
            clock: Arc::new(now_ms),
        }
    }
    pub fn with_predictor_factory(mut self, predictors: Arc<dyn SearchPredictorFactory>) -> Self {
        self.predictors = predictors;
        self
    }
    /// Experiment timestamps and wall-time checks use this clock; worker leases keep the
    /// system clock.
    pub fn with_clock(mut self, clock: Arc<dyn Fn() -> i64 + Send + Sync>) -> Self {
        self.clock = clock;
        self
    }
    fn now(&self) -> i64 {
        (self.clock)()
    }

    /// Execute at most one candidate. Reopening the repository resumes a scheduled job or
    /// validates its already-published artifact without retraining it. A trial whose job,
    /// lease or resource budget another worker holds stays scheduled, and the reason is
    /// recorded as a `trial_blocked` experiment event.
    pub fn run_next(&self, experiment_id: &str) -> Result<Experiment> {
        let _lock =
            match LocalRunLock::acquire(&self.repository, &format!("experiment:{experiment_id}")) {
                Ok(lock) => lock,
                Err(Error::Conflict(_)) => return self.repository.get_experiment(experiment_id),
                Err(error) => return Err(error),
            };
        // Wall time is charged from the first controller step, not from creation.
        self.repository
            .mark_experiment_active(experiment_id, self.now())?;
        let experiment = self.repository.get_experiment(experiment_id)?;
        if experiment.status != ExperimentStatus::Running || experiment.selected_trial_id.is_some()
        {
            return Ok(experiment);
        }
        let limits = self.worker.limits();
        let allowed = &experiment.request.budget.worker_limits;
        if limits.memory_budget_bytes > allowed.memory_budget_bytes
            || limits.maximum_duration_ms > allowed.maximum_duration_ms
            || limits.maximum_artifact_bytes > allowed.maximum_artifact_bytes
            || limits.maximum_checkpoint_bytes > allowed.maximum_checkpoint_bytes
        {
            return Err(invalid(
                "worker limits exceed the experiment's reserved resource limits",
            ));
        }
        let trials = self.repository.list_experiment_trials(experiment_id)?;
        let trial = if let Some(trial) = trials
            .iter()
            .find(|t| t.status == ExperimentTrialStatus::Scheduled)
        {
            trial.clone()
        } else {
            if trials.is_empty()
                && let Ok(spec) =
                    serde_json::from_value::<InspectionSpec>(experiment.request.spec.clone())
                && let Err(Error::Invalid(message)) =
                    validate_goals(&spec, &experiment.request.goals)
            {
                return self.repository.finalize_experiment(
                    experiment_id,
                    None,
                    &message,
                    self.now(),
                );
            }
            if experiment.usage.submitted_trials >= experiment.request.budget.maximum_trials {
                return self.repository.stop_experiment_budget(
                    experiment_id,
                    "trial_budget_exhausted",
                    self.now(),
                );
            }
            if !search_window_open(&experiment, self.now()) {
                return self.repository.stop_experiment_budget(
                    experiment_id,
                    "wall_time_budget_exhausted",
                    self.now(),
                );
            }
            let submitted = match self.next_fresh_candidate(&experiment, &trials) {
                Ok(Some(index)) => self
                    .repository
                    .submit_experiment_trial(
                        experiment_id,
                        index,
                        &format!("candidate-{index}"),
                        self.now(),
                    )
                    .map(Some),
                Ok(None) if trials.is_empty() => match self.unfit_memory_reason(experiment_id)? {
                    Some(reason) => {
                        return self.repository.stop_experiment_budget(
                            experiment_id,
                            &reason,
                            self.now(),
                        );
                    }
                    None => Ok(None),
                },
                Ok(None) => self.next_halving_trial(&experiment, &trials),
                Err(error) => Err(error),
            };
            match submitted {
                Ok(Some(trial)) => trial,
                Ok(None) => return self.repository.get_experiment(experiment_id),
                // A clock that moved backwards is a retryable Conflict, never a budget stop.
                Err(Error::Invalid(message)) if message.contains("budget exhausted") => {
                    return self.repository.stop_experiment_budget(
                        experiment_id,
                        &message,
                        self.now(),
                    );
                }
                Err(error) => return Err(error),
            }
        };
        let recovering_completed =
            self.repository.get_job(&trial.job_id)?.status == JobStatus::Succeeded;
        let started = Instant::now();
        let result = (|| -> Result<(ModelArtifact, BTreeMap<String, f64>)> {
            self.check_cancel(experiment_id)?;
            let job = self.repository.get_job(&trial.job_id)?;
            if job.status == JobStatus::Running
                && job
                    .lease_expires_at_ms
                    .is_none_or(|deadline| deadline > now_ms())
            {
                return Err(Error::Conflict(
                    "candidate is already owned by a training worker".into(),
                ));
            }
            let dataset = self
                .repository
                .get_experiment_search_snapshot(&trial.id, snapshot_limit(&experiment))?;
            if !dataset.test.is_empty() {
                return Err(invalid(
                    "candidate training snapshot exposes the final holdout",
                ));
            }
            let mut spec: InspectionSpec = serde_json::from_value(experiment.request.spec.clone())?;
            let first: Sample = serde_json::from_value(
                dataset
                    .train
                    .first()
                    .ok_or_else(|| invalid("empty training partition"))?
                    .payload
                    .get("sample")
                    .unwrap_or(&dataset.train[0].payload)
                    .clone(),
            )?;
            spec.input_shape = first.input.shape;
            validate_task(&spec, &experiment.request.goals.task)?;
            validate_recipe(&spec, &job.request)?;
            if dataset.validation.len() < experiment.request.goals.minimum_audited_samples {
                return Err(invalid(
                    "validation partition has fewer samples than the audited minimum",
                ));
            }
            for sample in &dataset.validation {
                sealed_sample(sample, &spec, dataset.as_of_ms, true)?;
            }
            let artifact = if job.status == JobStatus::Succeeded {
                self.repository.get_artifact(
                    job.artifact_id
                        .as_deref()
                        .ok_or_else(|| invalid("succeeded candidate has no artifact"))?,
                )?
            } else if matches!(
                job.status,
                JobStatus::Queued | JobStatus::Paused | JobStatus::Interrupted | JobStatus::Running
            ) {
                self.worker.run(&job.id)?
            } else {
                return Err(Error::Engine(format!(
                    "candidate job is {:?}: {}",
                    job.status,
                    job.error.unwrap_or_default()
                )));
            };
            let compute = inference_compute(&job.request.compute, &experiment)?;
            let mut predictor = self
                .predictors
                .load(&self.repository, &artifact.id, &compute)?;
            let scale = if matches!(
                experiment.request.goals.task,
                EvaluationTask::Anomaly { .. }
            ) {
                let (scale, in_sample) =
                    self.anomaly_scale(experiment_id, &mut *predictor, &dataset, &spec)?;
                if in_sample {
                    self.note(
                        experiment_id,
                        "anomaly_calibration_in_sample",
                        serde_json::json!({
                            "trial_id": trial.id,
                            "reason": "validation has no normal samples, so the decision scale \
                                       was fitted on training scores",
                        }),
                    );
                }
                scale
            } else {
                1.0
            };
            let observations = self.observe(
                experiment_id,
                &mut *predictor,
                &dataset.validation,
                &spec,
                dataset.as_of_ms,
                &experiment.request.goals.task,
                scale,
            )?;
            let mut metrics = measure(&observations, &experiment.request.goals.task, &spec.labels)?;
            metrics.insert("artifact_bytes".into(), artifact.blob.bytes as f64);
            if !metrics.contains_key(&experiment.request.goals.primary_metric) {
                return Err(invalid(
                    "primary metric is undefined for the validation labels or task",
                ));
            }
            if matches!(
                experiment.request.goals.task,
                EvaluationTask::Anomaly { .. }
            ) {
                metrics.insert("anomaly_scale".into(), scale);
            }
            // Recording would reject the set and leave the trial scheduled forever.
            if metrics.values().any(|value| !value.is_finite()) {
                return Err(invalid("validation produced a non-finite metric"));
            }
            Ok((artifact, metrics))
        })();
        let elapsed = if recovering_completed {
            trial.reserved_training_time_ms
        } else {
            started.elapsed().as_millis().min(u64::MAX as u128) as u64
        };
        match result {
            Ok((_, metrics)) => {
                self.repository.record_experiment_trial_validation(
                    &trial.id,
                    &metrics,
                    elapsed,
                    self.now(),
                )?;
            }
            Err(error @ (Error::Conflict(_) | Error::LeaseLost)) => {
                self.note_blocked(experiment_id, &trial, &error.to_string());
                return self.repository.get_experiment(experiment_id);
            }
            Err(error) => {
                self.repository.fail_experiment_trial(
                    &trial.id,
                    &error.to_string(),
                    elapsed,
                    self.now(),
                )?;
            }
        }
        let updated = self.repository.get_experiment(experiment_id)?;
        if updated.status == ExperimentStatus::Running && !search_window_open(&updated, self.now())
        {
            // Another scheduled trial keeps the search open until it settles.
            return match self.repository.stop_experiment_budget(
                experiment_id,
                "wall_time_budget_exhausted",
                self.now(),
            ) {
                Err(Error::Conflict(_)) => Ok(updated),
                stopped => stopped,
            };
        }
        Ok(updated)
    }

    /// Run the fixed candidate list, then evaluate one frozen winner. Hosts can call run_next
    /// instead to inspect validation results and append additional bounded candidates.
    /// A step held back by another worker, lease, resource reservation or concurrent writer is
    /// retried with backoff for at most one per-trial duration cap without progress; then the
    /// experiment is returned still running, or the last conflict is returned.
    pub fn run(&self, experiment_id: &str) -> Result<Experiment> {
        let limit_ms = self.worker.limits().maximum_duration_ms;
        let mut backoff = Backoff::new();
        loop {
            let before = self.repository.get_experiment(experiment_id)?;
            if matches!(
                before.status,
                ExperimentStatus::Completed
                    | ExperimentStatus::Cancelled
                    | ExperimentStatus::Failed
                    | ExperimentStatus::CancelRequested
            ) {
                return Ok(before);
            }
            if before.selected_trial_id.is_some() {
                return self.finalize(experiment_id);
            }
            let after = match self.run_next(experiment_id) {
                Ok(after) => after,
                Err(Error::Conflict(_)) if backoff.wait(limit_ms) => continue,
                Err(error) => return Err(error),
            };
            if after.generation != before.generation {
                backoff = Backoff::new();
                continue;
            }
            if !self
                .repository
                .list_experiment_trials(experiment_id)?
                .iter()
                .any(|t| t.status == ExperimentTrialStatus::Scheduled)
            {
                return self.finalize(experiment_id);
            }
            // The scheduled trial is held elsewhere; a trial_blocked event names the holder.
            if !backoff.wait(limit_ms) {
                return Ok(after);
            }
        }
    }

    /// Freeze the validation winner before opening its final test partition. Repeated calls
    /// reuse durable prediction evidence and the final evaluation, never select another winner.
    /// Once the winner is frozen every outcome settles the experiment, except a cancellation or
    /// a busy ledger, which a later call retries.
    pub fn finalize(&self, experiment_id: &str) -> Result<Experiment> {
        let _lock =
            match LocalRunLock::acquire(&self.repository, &format!("experiment:{experiment_id}")) {
                Ok(lock) => lock,
                Err(Error::Conflict(_)) => return self.repository.get_experiment(experiment_id),
                Err(error) => return Err(error),
            };
        self.repository
            .mark_experiment_active(experiment_id, self.now())?;
        let experiment = self.repository.get_experiment(experiment_id)?;
        if experiment.final_evaluation_id.is_some()
            || matches!(
                experiment.status,
                ExperimentStatus::Cancelled
                    | ExperimentStatus::CancelRequested
                    | ExperimentStatus::Failed
            )
        {
            return Ok(experiment);
        }
        if experiment.best_trial_id.is_none() {
            let reason = experiment
                .stop_reason
                .clone()
                .unwrap_or_else(|| "no_candidate_met_validation_requirements".into());
            return self
                .repository
                .finalize_experiment(experiment_id, None, &reason, self.now());
        }
        let experiment = self
            .repository
            .select_experiment_winner(experiment_id, self.now())?;
        if experiment.status != ExperimentStatus::Evaluating {
            let reason = experiment
                .stop_reason
                .clone()
                .unwrap_or_else(|| "final_evaluation_unavailable".into());
            return self
                .repository
                .finalize_experiment(experiment_id, None, &reason, self.now());
        }
        let reason = experiment
            .stop_reason
            .clone()
            .unwrap_or_else(|| "candidate_search_completed".into());
        let outcome = self.audit(&experiment).and_then(|audit| match audit {
            Audit::Evaluated(evaluation_id) => self.repository.finalize_experiment(
                experiment_id,
                Some(&evaluation_id),
                &reason,
                self.now(),
            ),
            Audit::Ended(ended) => {
                self.repository
                    .finalize_experiment(experiment_id, None, &ended, self.now())
            }
        });
        match outcome {
            Err(error) if !matches!(error, Error::Cancelled) && !ledger_busy(&error) => {
                self.repository.finalize_experiment(
                    experiment_id,
                    None,
                    &format!("final_evaluation_failed: {error}"),
                    self.now(),
                )
            }
            outcome => outcome,
        }
    }

    /// Score the frozen winner on the final holdout, reusing only predictions this audit
    /// recorded. A failed prediction counts as a miss where the task can score one.
    fn audit(&self, experiment: &Experiment) -> Result<Audit> {
        let experiment_id = experiment.id.as_str();
        let task = &experiment.request.goals.task;
        let trial = self.repository.get_experiment_trial(
            experiment
                .selected_trial_id
                .as_deref()
                .ok_or_else(|| invalid("winner was not frozen"))?,
        )?;
        let artifact_id = trial
            .artifact_id
            .as_deref()
            .ok_or_else(|| invalid("selected trial has no model artifact"))?;
        let dataset = self
            .repository
            .get_snapshot_limited(&trial.dataset_snapshot_id, snapshot_limit(experiment))?;
        let mut spec: InspectionSpec = serde_json::from_value(experiment.request.spec.clone())?;
        let first: Sample = serde_json::from_value(
            dataset
                .train
                .first()
                .ok_or_else(|| invalid("empty training partition"))?
                .payload
                .get("sample")
                .unwrap_or(&dataset.train[0].payload)
                .clone(),
        )?;
        spec.input_shape = first.input.shape;
        validate_task(&spec, task)?;
        let job = self.repository.get_job(&trial.job_id)?;
        let compute = inference_compute(&job.request.compute, experiment)?;
        let holdout: BTreeSet<String> = dataset.test.iter().map(|row| row.id.clone()).collect();
        let recorded: BTreeMap<String, PredictionRecord> = self
            .repository
            .predictions(artifact_id)?
            .into_iter()
            .filter(|prediction| holdout.contains(&prediction.sample_id))
            .map(|prediction| (prediction.sample_id.clone(), prediction))
            .collect();
        let scale = trial
            .validation_metrics
            .get("anomaly_scale")
            .copied()
            .unwrap_or(1.0);
        let mut predictor: Option<Box<dyn SearchPredictor>> = None;
        for row in &dataset.test {
            let current = self.repository.get_experiment(experiment_id)?;
            if matches!(
                current.status,
                ExperimentStatus::Cancelled | ExperimentStatus::CancelRequested
            ) {
                return Err(Error::Cancelled);
            }
            if !audit_window_open(&current, self.now()) {
                return Ok(Audit::Ended("wall_time_budget_exhausted".into()));
            }
            if let Some(existing) = recorded.get(&row.id) {
                if existing.details.get("partition").and_then(Value::as_str) == Some("final_test")
                    && existing
                        .details
                        .get("experiment_id")
                        .and_then(Value::as_str)
                        == Some(experiment_id)
                {
                    continue;
                }
                return Ok(Audit::Ended(format!(
                    "final_evaluation_failed: holdout sample {} already has a prediction that this audit did not record",
                    row.id
                )));
            }
            let sample = sealed_sample(row, &spec, dataset.as_of_ms, true)?;
            let actual_label = audited_label(row, &sample, &spec)?;
            if predictor.is_none() {
                predictor = Some(
                    self.predictors
                        .load(&self.repository, artifact_id, &compute)?,
                );
            }
            let model = predictor
                .as_deref_mut()
                .ok_or_else(|| invalid("final audit predictor is unavailable"))?;
            let started = Instant::now();
            let observed = match predict_sample(model, &sample) {
                Ok(raw) => decode_prediction(
                    sample,
                    &raw,
                    task,
                    spec.labels.len(),
                    scale,
                    started.elapsed().as_secs_f64() * 1000.0,
                ),
                Err(error) => Err(error),
            };
            let (predicted_label, evidence, latency_ms, failure) = match observed {
                Ok(observation) => (
                    observation
                        .predicted_class
                        .map(|class| spec.labels[class as usize].clone()),
                    serde_json::to_value(&observation.evidence)?,
                    observation.latency_ms,
                    None,
                ),
                Err(error) => match failed_evidence(task) {
                    Some(evidence) => (
                        None,
                        evidence,
                        started.elapsed().as_secs_f64() * 1000.0,
                        Some(error.to_string()),
                    ),
                    None => {
                        return Ok(Audit::Ended(format!(
                            "final_evaluation_failed: test sample {} could not be scored: {error}",
                            row.id
                        )));
                    }
                },
            };
            let mut details = serde_json::json!({
                "prediction": evidence,
                "experiment_id": experiment_id,
                "partition": "final_test",
            });
            if let Some(failure) = &failure {
                details["error"] = serde_json::json!(failure);
            }
            self.repository.record_prediction(&PredictionRecord {
                sample_id: row.id.clone(),
                artifact_id: artifact_id.into(),
                predicted_label,
                teacher_label: None,
                actual_label,
                actual_source: Some(row.source),
                latency_ms,
                failed: failure.is_some(),
                recorded_at_ms: self.now(),
                details,
            })?;
        }
        let evaluation = self.repository.evaluate_task_cohort(
            artifact_id,
            task.clone(),
            Some(&holdout),
            self.now(),
        )?;
        Ok(Audit::Evaluated(evaluation.id))
    }

    fn check_cancel(&self, experiment_id: &str) -> Result<()> {
        let experiment = self.repository.get_experiment(experiment_id)?;
        if matches!(
            experiment.status,
            ExperimentStatus::Cancelled | ExperimentStatus::CancelRequested
        ) {
            Err(Error::Cancelled)
        } else if experiment.wall_remaining_ms(self.now()) == 0 {
            Err(Error::Engine(
                "experiment wall time budget exhausted".into(),
            ))
        } else {
            Ok(())
        }
    }

    /// Diagnostic events never block the search; a full event log only loses the note.
    fn note(&self, experiment_id: &str, kind: &str, body: Value) {
        let _ = self
            .repository
            .record_experiment_event(experiment_id, kind, body, self.now());
    }

    /// Record what kept a scheduled trial from starting, once per trial and reason.
    fn note_blocked(&self, experiment_id: &str, trial: &ExperimentTrial, reason: &str) {
        let Ok(events) = self.repository.experiment_events(experiment_id) else {
            return;
        };
        let blocked: Vec<&Value> = events
            .iter()
            .filter(|event| event["kind"] == TRIAL_BLOCKED)
            .collect();
        let recorded = blocked.iter().any(|event| {
            event["body"]["trial_id"] == trial.id.as_str() && event["body"]["reason"] == reason
        });
        if recorded || blocked.len() >= MAXIMUM_BLOCKED_EVENTS {
            return;
        }
        self.note(
            experiment_id,
            TRIAL_BLOCKED,
            serde_json::json!({
                "trial_id": trial.id,
                "job_id": trial.job_id,
                "candidate_index": trial.candidate_index,
                "reason": reason,
            }),
        );
    }

    /// The next unsubmitted candidate that fits this worker's memory. An oversized Burn
    /// candidate is replaced by a copy with the largest fitting batch, or skipped with its
    /// estimate recorded when even one sample does not fit.
    fn next_fresh_candidate(
        &self,
        experiment: &Experiment,
        trials: &[ExperimentTrial],
    ) -> Result<Option<usize>> {
        let submitted: HashSet<usize> = trials.iter().map(|t| t.candidate_index).collect();
        let mut skipped = self.skipped_candidates(experiment)?;
        let mut experiment = experiment.clone();
        let mut data = HashMap::new();
        loop {
            let Some(index) = (0..experiment.request.candidates.len())
                .find(|i| !submitted.contains(i) && !skipped.contains(i))
            else {
                return Ok(None);
            };
            let candidate = experiment.request.candidates[index].clone();
            match self.admit(&experiment, &candidate, &mut data)? {
                Admission::Fits => return Ok(Some(index)),
                Admission::Resize {
                    mut request,
                    batch,
                    estimate,
                    budget,
                } => {
                    request.recipe[RESIZED_FROM] = serde_json::json!(index);
                    match self.repository.append_experiment_candidates(
                        &experiment.id,
                        vec![request],
                        self.now(),
                    ) {
                        Ok(updated) => {
                            self.note(
                                &experiment.id,
                                "candidate_resized",
                                serde_json::json!({
                                    "candidate_index": index,
                                    "replacement_index": updated.request.candidates.len() - 1,
                                    "batch_size": batch,
                                    "estimated_bytes": estimate,
                                    "memory_budget_bytes": budget,
                                }),
                            );
                            experiment = updated;
                        }
                        Err(Error::Invalid(message)) if message.contains("candidate count") => {
                            self.note(
                                &experiment.id,
                                "candidate_skipped",
                                serde_json::json!({
                                    "candidate_index": index,
                                    "reason": format!("batch {batch} would fit, but {message}"),
                                }),
                            );
                        }
                        Err(error) => return Err(error),
                    }
                    skipped.insert(index);
                }
                Admission::Unfit { estimate, budget } => {
                    self.note(
                        &experiment.id,
                        "candidate_skipped",
                        serde_json::json!({
                            "candidate_index": index,
                            "reason": "training does not fit the worker memory budget at batch 1",
                            "estimated_bytes": estimate,
                            "memory_budget_bytes": budget,
                        }),
                    );
                    skipped.insert(index);
                }
            }
        }
    }

    fn skipped_candidates(&self, experiment: &Experiment) -> Result<HashSet<usize>> {
        let mut skipped: HashSet<usize> = experiment
            .request
            .candidates
            .iter()
            .filter_map(|candidate| candidate.recipe.get(RESIZED_FROM)?.as_u64())
            .map(|index| index as usize)
            .collect();
        for event in self.repository.experiment_events(&experiment.id)? {
            if event["kind"] == "candidate_skipped"
                && let Some(index) = event["body"]["candidate_index"].as_u64()
            {
                skipped.insert(index as usize);
            }
        }
        Ok(skipped)
    }

    /// Mirror the Burn worker's admission estimate: model and optimizer at the batch, the
    /// engine's per-byte charge for the serialized training view and the unpacked pretrained
    /// source.
    fn admit(
        &self,
        experiment: &Experiment,
        candidate: &TrainingRequest,
        data: &mut HashMap<(String, u64), TrainingData>,
    ) -> Result<Admission> {
        if candidate.engine != "burn" {
            return Ok(Admission::Fits);
        }
        let Ok(recipe) = serde_json::from_value::<BurnEngineRecipe>(candidate.recipe.clone())
        else {
            return Ok(Admission::Fits);
        };
        let worker_budget = self.worker.limits().memory_budget_bytes;
        let budget = candidate
            .compute
            .get("memory_limit_bytes")
            .and_then(Value::as_u64)
            .unwrap_or(worker_budget)
            .min(worker_budget);
        let dataset_id = candidate
            .recipe
            .get("dataset_snapshot_id")
            .and_then(Value::as_str)
            .unwrap_or(&experiment.request.snapshot_id)
            .to_owned();
        // A larger view exceeds the budget whatever the model needs.
        let limit = budget / SNAPSHOT_MEMORY_FACTOR;
        let key = (dataset_id, limit);
        if !data.contains_key(&key) {
            let measured = self.training_data(&experiment.id, &key.0, limit)?;
            data.insert(key.clone(), measured);
        }
        let (dataset_bytes, shape) = match &data[&key] {
            TrainingData::Measured { bytes, shape } => (*bytes, shape.clone()),
            TrainingData::Exceeds => {
                return Ok(Admission::Unfit {
                    estimate: limit
                        .saturating_add(1)
                        .saturating_mul(SNAPSHOT_MEMORY_FACTOR),
                    budget,
                });
            }
            TrainingData::Unknown => return Ok(Admission::Fits),
        };
        let source_bytes = match &recipe.pretrained {
            Some(reference) => match self.pretrained_source_bytes(reference) {
                Some(bytes) => bytes,
                None => return Ok(Admission::Fits),
            },
            None => 0,
        };
        let fixed = dataset_bytes
            .saturating_mul(SNAPSHOT_MEMORY_FACTOR)
            .saturating_add(source_bytes);
        Ok(
            match fit_batch(
                &recipe.config.recipe,
                &shape,
                recipe.config.batch_size,
                fixed,
                budget,
            ) {
                None => Admission::Fits,
                Some(Ok(batch)) if batch == recipe.config.batch_size => Admission::Fits,
                Some(Ok(batch)) => {
                    let mut request = candidate.clone();
                    request.recipe["config"]["batch_size"] = serde_json::json!(batch);
                    if request.recipe.get("search").is_some_and(Value::is_object) {
                        let mut config = recipe.config.clone();
                        config.batch_size = batch;
                        request.recipe["search"]["family"] =
                            serde_json::json!(digest(&serde_json::to_vec(&config)?));
                    }
                    let estimate = recipe
                        .config
                        .recipe
                        .estimated_training_bytes(&shape, batch)
                        .map_err(engine_error)?
                        .saturating_add(fixed);
                    Admission::Resize {
                        request,
                        batch,
                        estimate,
                        budget,
                    }
                }
                Some(Err(estimate)) => Admission::Unfit { estimate, budget },
            },
        )
    }

    /// Serialized size and sample shape of a dataset's worker view, read only up to `limit`.
    fn training_data(
        &self,
        experiment_id: &str,
        dataset_id: &str,
        limit: u64,
    ) -> Result<TrainingData> {
        let Some(dataset) = self
            .repository
            .experiment_result(experiment_id)?
            .datasets
            .into_iter()
            .find(|dataset| dataset.snapshot_id == dataset_id)
        else {
            return Ok(TrainingData::Unknown);
        };
        let snapshot = match self
            .repository
            .get_snapshot_limited(&dataset.training_snapshot_id, limit)
        {
            Ok(snapshot) => snapshot,
            Err(Error::Invalid(_)) => return Ok(TrainingData::Exceeds),
            Err(error) => return Err(error),
        };
        let Some(first) = snapshot.train.first() else {
            return Ok(TrainingData::Unknown);
        };
        let Ok(sample) = serde_json::from_value::<Sample>(
            first
                .payload
                .get("sample")
                .unwrap_or(&first.payload)
                .clone(),
        ) else {
            return Ok(TrainingData::Unknown);
        };
        Ok(TrainingData::Measured {
            bytes: serialized_size(&snapshot)?,
            shape: sample.input.shape,
        })
    }

    fn pretrained_source_bytes(&self, reference: &PretrainedSourceRef) -> Option<u64> {
        let source = self
            .repository
            .get_pretrained_source(&reference.source_id)
            .ok()?;
        let config: burn::TrainingConfig =
            serde_json::from_value(source.manifest["config"].clone()).ok()?;
        let shape: Vec<usize> =
            serde_json::from_value(source.manifest["input_shape"].clone()).ok()?;
        Some(
            source
                .blob
                .bytes
                .saturating_mul(4)
                .saturating_add(config.recipe.estimated_training_bytes(&shape, 1).ok()?),
        )
    }

    /// Explain an experiment whose every candidate was skipped for memory.
    fn unfit_memory_reason(&self, experiment_id: &str) -> Result<Option<String>> {
        let mut smallest: Option<(u64, u64)> = None;
        for event in self.repository.experiment_events(experiment_id)? {
            if event["kind"] != "candidate_skipped" {
                continue;
            }
            let body = &event["body"];
            if let (Some(estimate), Some(budget)) = (
                body["estimated_bytes"].as_u64(),
                body["memory_budget_bytes"].as_u64(),
            ) && smallest.is_none_or(|(least, _)| estimate < least)
            {
                smallest = Some((estimate, budget));
            }
        }
        Ok(smallest.map(|(estimate, budget)| unfit_message(budget, estimate)))
    }

    fn next_halving_trial(
        &self,
        experiment: &Experiment,
        trials: &[ExperimentTrial],
    ) -> Result<Option<ExperimentTrial>> {
        let mut rungs = Vec::new();
        for trial in trials {
            let request = &self.repository.get_job(&trial.job_id)?.request;
            if request.engine != "burn" {
                continue;
            }
            let Some(search) = request.recipe.get("search") else {
                continue;
            };
            let factor = search["reduction_factor"]
                .as_u64()
                .ok_or_else(|| invalid("invalid halving reduction factor"))?
                as usize;
            if !(2..=8).contains(&factor) {
                return Err(invalid("halving reduction factor must be 2..8"));
            }
            let mut config = request.recipe["config"].clone();
            config
                .as_object_mut()
                .ok_or_else(|| invalid("missing trial config"))?
                .remove("epochs");
            let family = digest(&serde_json::to_vec(&serde_json::json!({
                "config": config,
                "compute": request.compute,
                "dataset": trial.dataset_snapshot_id,
                "maximum": search["max_epochs"],
                "factor": factor,
            }))?);
            rungs.push(HalvingCandidate {
                trial: trial.clone(),
                rung: search["rung"].as_u64().unwrap_or(0),
                factor,
                epochs: request.recipe["config"]["epochs"]
                    .as_u64()
                    .ok_or_else(|| invalid("missing trial epochs"))?
                    as usize,
                maximum: search["max_epochs"]
                    .as_u64()
                    .ok_or_else(|| invalid("missing maximum epochs"))?
                    as usize,
                parent: search["parent_trial_id"].as_str().map(str::to_owned),
                family,
            });
        }
        let Some(highest) = rungs.iter().map(|v| v.rung).max() else {
            return Ok(None);
        };
        // Finish every selected survivor of the previous rung before pruning the current one.
        for rung in [highest.checked_sub(1), Some(highest)]
            .into_iter()
            .flatten()
        {
            let mut parents = rungs
                .iter()
                .filter(|v| v.rung == rung && v.trial.status == ExperimentTrialStatus::Validated)
                .collect::<Vec<_>>();
            if parents.is_empty() {
                continue;
            }
            parents.sort_by(|a, b| rank_candidates(&a.trial, &b.trial, &experiment.request.goals));
            let mut families = HashSet::new();
            parents.retain(|parent| families.insert(parent.family.as_str()));
            let survivors = parents.len().div_ceil(parents[0].factor);
            for parent in parents.into_iter().take(survivors) {
                if parent.epochs >= parent.maximum {
                    continue;
                }
                let children = rungs
                    .iter()
                    .filter(|child| {
                        child.parent.as_deref() == Some(parent.trial.id.as_str())
                            || (child.rung == parent.rung + 1 && child.family == parent.family)
                    })
                    .collect::<Vec<_>>();
                // One retry with half the extra epochs after a failed continuation.
                let target = match children.as_slice() {
                    [] => parent
                        .epochs
                        .saturating_mul(parent.factor)
                        .min(parent.maximum),
                    [failed] if failed.trial.status == ExperimentTrialStatus::Failed => {
                        parent.epochs + failed.epochs.saturating_sub(parent.epochs) / 2
                    }
                    _ => continue,
                };
                let started_from = rungs
                    .iter()
                    .find(|grandparent| {
                        parent.parent.as_deref() == Some(grandparent.trial.id.as_str())
                    })
                    .map_or(0, |grandparent| grandparent.epochs);
                let fitting = epochs_within_trial_cap(
                    self.worker.limits().maximum_duration_ms,
                    parent.trial.training_time_ms,
                    parent.epochs.saturating_sub(started_from),
                );
                let epochs = target.min(parent.epochs.saturating_add(fitting));
                if epochs <= parent.epochs {
                    continue;
                }
                return self
                    .repository
                    .continue_experiment_trial(&parent.trial.id, epochs, self.now())
                    .map(Some);
            }
        }
        Ok(None)
    }

    /// Fit the decision scale on held-out normal validation scores. The model was fitted on
    /// the training rows, so their scores are a fallback, reported as in-sample.
    fn anomaly_scale(
        &self,
        id: &str,
        predictor: &mut dyn SearchPredictor,
        dataset: &DatasetSnapshot,
        spec: &InspectionSpec,
    ) -> Result<(f64, bool)> {
        let mut scores = self.normal_scores(
            id,
            predictor,
            &dataset.validation,
            spec,
            dataset.as_of_ms,
            true,
        )?;
        let in_sample = scores.is_empty();
        if in_sample {
            scores =
                self.normal_scores(id, predictor, &dataset.train, spec, dataset.as_of_ms, false)?;
        }
        if scores.is_empty() {
            return Err(invalid(
                "anomaly calibration requires explicitly normal examples",
            ));
        }
        scores.sort_by(f64::total_cmp);
        Ok((
            scores[((scores.len() - 1) as f64 * 0.95).floor() as usize].max(1e-8),
            in_sample,
        ))
    }

    fn normal_scores(
        &self,
        id: &str,
        predictor: &mut dyn SearchPredictor,
        rows: &[TrainingSample],
        spec: &InspectionSpec,
        as_of_ms: i64,
        audited: bool,
    ) -> Result<Vec<f64>> {
        let mut scores = Vec::new();
        for row in rows {
            self.check_cancel(id)?;
            let sample = sealed_sample(row, spec, as_of_ms, audited)?;
            if matches!(sample.annotation, Annotation::Anomaly { is_anomaly: false }) {
                scores.push(scalar_score(&predict_sample(predictor, &sample)?)?);
                if scores.len() == 256 {
                    break;
                }
            }
        }
        Ok(scores)
    }

    fn observe(
        &self,
        id: &str,
        predictor: &mut dyn SearchPredictor,
        rows: &[TrainingSample],
        spec: &InspectionSpec,
        at_ms: i64,
        task: &EvaluationTask,
        scale: f64,
    ) -> Result<Vec<Observation>> {
        let mut observations = Vec::with_capacity(rows.len());
        for row in rows {
            self.check_cancel(id)?;
            let sample = sealed_sample(row, spec, at_ms, true)?;
            let started = Instant::now();
            let raw = predict_sample(predictor, &sample)?;
            let mut observation = decode_prediction(
                sample,
                &raw,
                task,
                spec.labels.len(),
                scale,
                started.elapsed().as_secs_f64() * 1000.0,
            )?;
            observation.sample.input.values = Vec::new();
            observations.push(observation);
        }
        Ok(observations)
    }
}

/// New trials start only while more than the audit allowance of wall time remains.
fn search_window_open(experiment: &Experiment, at_ms: i64) -> bool {
    experiment.wall_remaining_ms(at_ms) > experiment.audit_allowance_ms()
}

/// The audit may use the remaining wall time and at least its allowance after the winner
/// froze, so a trial stopped at the deadline does not cost the final evaluation.
fn audit_window_open(experiment: &Experiment, at_ms: i64) -> bool {
    experiment.wall_remaining_ms(at_ms) > 0
        || experiment.selected_at_ms.is_some_and(|selected| {
            (at_ms as i128 - selected as i128) < experiment.audit_allowance_ms() as i128
        })
}

/// Extra epochs expected to finish within 80% of the per-trial duration cap, estimated from
/// the time the parent trial spent on the epochs it trained itself.
fn epochs_within_trial_cap(cap_ms: u64, trained_ms: u64, trained_epochs: usize) -> usize {
    let per_epoch_ms = trained_ms.div_ceil(trained_epochs.max(1) as u64);
    if per_epoch_ms == 0 {
        return usize::MAX;
    }
    usize::try_from(cap_ms / 5 * 4 / per_epoch_ms).unwrap_or(usize::MAX)
}

/// Doubling pauses whose total stays within a limit.
struct Backoff {
    delay_ms: u64,
    waited_ms: u64,
}
impl Backoff {
    fn new() -> Self {
        Self {
            delay_ms: FIRST_RETRY_DELAY_MS,
            waited_ms: 0,
        }
    }

    /// The next pause, or None once it would take the total past `limit_ms`.
    fn next_pause(&mut self, limit_ms: u64) -> Option<Duration> {
        let waited_ms = self.waited_ms.saturating_add(self.delay_ms);
        if waited_ms > limit_ms {
            return None;
        }
        let pause = Duration::from_millis(self.delay_ms);
        self.waited_ms = waited_ms;
        self.delay_ms = self.delay_ms.saturating_mul(2).min(MAXIMUM_RETRY_DELAY_MS);
        Some(pause)
    }

    /// Sleeps for the next pause; false once the total would pass `limit_ms`.
    fn wait(&mut self, limit_ms: u64) -> bool {
        let Some(pause) = self.next_pause(limit_ms) else {
            return false;
        };
        std::thread::sleep(pause);
        true
    }
}

fn ledger_busy(error: &Error) -> bool {
    matches!(
        error,
        Error::Sql(rusqlite::Error::SqliteFailure(failure, _))
            if matches!(
                failure.code,
                rusqlite::ErrorCode::DatabaseBusy | rusqlite::ErrorCode::DatabaseLocked
            )
    )
}

/// Evidence for a failed test prediction where the task scores a failure as a miss.
fn failed_evidence(task: &EvaluationTask) -> Option<Value> {
    match task {
        EvaluationTask::Classification => Some(Value::Null),
        EvaluationTask::Detection { .. } => {
            serde_json::to_value(EvidencePrediction::Detection { boxes: Vec::new() }).ok()
        }
        EvaluationTask::InstanceSegmentation { .. } => {
            serde_json::to_value(EvidencePrediction::Instances {
                instances: Vec::new(),
            })
            .ok()
        }
        _ => None,
    }
}

fn audited_label(
    row: &TrainingSample,
    sample: &Sample,
    spec: &InspectionSpec,
) -> Result<Option<String>> {
    let Annotation::Class { class_id } = sample.annotation else {
        return Ok(None);
    };
    let expected = spec
        .labels
        .get(class_id as usize)
        .ok_or_else(|| invalid("test class outside label order"))?;
    if row.payload.get("label").and_then(Value::as_str) != Some(expected.as_str()) {
        return Err(invalid(
            "test ledger label differs from the inspection class order",
        ));
    }
    Ok(Some(expected.clone()))
}

enum Audit {
    Evaluated(String),
    Ended(String),
}

enum Admission {
    Fits,
    Resize {
        request: TrainingRequest,
        batch: usize,
        estimate: u64,
        budget: u64,
    },
    Unfit {
        estimate: u64,
        budget: u64,
    },
}

enum TrainingData {
    Measured { bytes: u64, shape: Vec<usize> },
    Exceeds,
    Unknown,
}

struct HalvingCandidate {
    trial: ExperimentTrial,
    rung: u64,
    factor: usize,
    epochs: usize,
    maximum: usize,
    parent: Option<String>,
    family: String,
}

fn rank_candidates(
    a: &ExperimentTrial,
    b: &ExperimentTrial,
    goals: &ExperimentGoals,
) -> std::cmp::Ordering {
    let eligible = |trial: &ExperimentTrial| {
        trial
            .validation_metrics
            .get("audited_samples")
            .is_some_and(|samples| *samples >= goals.minimum_audited_samples as f64)
            && goals.bounds.iter().all(|bound| {
                trial
                    .validation_metrics
                    .get(&bound.name)
                    .is_some_and(|value| {
                        value.is_finite()
                            && bound.minimum.is_none_or(|minimum| *value >= minimum)
                            && bound.maximum.is_none_or(|maximum| *value <= maximum)
                    })
            })
    };
    eligible(b)
        .cmp(&eligible(a))
        .then_with(|| {
            let a = a
                .validation_metrics
                .get(&goals.primary_metric)
                .copied()
                .unwrap_or(f64::NAN);
            let b = b
                .validation_metrics
                .get(&goals.primary_metric)
                .copied()
                .unwrap_or(f64::NAN);
            match goals.direction {
                MetricDirection::Maximize => b.total_cmp(&a),
                MetricDirection::Minimize => a.total_cmp(&b),
            }
        })
        .then(a.candidate_index.cmp(&b.candidate_index))
}

fn inference_compute(request: &Value, experiment: &Experiment) -> Result<ComputeConfig> {
    let mut compute: ComputeConfig = serde_json::from_value(request.clone())?;
    compute.memory_limit_bytes = compute
        .memory_limit_bytes
        .min(experiment.request.budget.worker_limits.memory_budget_bytes);
    compute.validate().map_err(engine_error)?;
    Ok(compute)
}

fn snapshot_limit(experiment: &Experiment) -> u64 {
    experiment
        .request
        .budget
        .maximum_dataset_bytes
        .min(experiment.request.budget.worker_limits.memory_budget_bytes / 8)
}

fn validate_task(spec: &InspectionSpec, task: &EvaluationTask) -> Result<()> {
    let compatible = match (&default_goals(spec).task, task) {
        (EvaluationTask::Classification, EvaluationTask::Classification)
        | (EvaluationTask::Regression, EvaluationTask::Regression)
        | (EvaluationTask::Anomaly { .. }, EvaluationTask::Anomaly { .. }) => true,
        (
            EvaluationTask::Detection { classes: a, .. },
            EvaluationTask::Detection { classes: b, .. },
        )
        | (
            EvaluationTask::Segmentation { classes: a },
            EvaluationTask::Segmentation { classes: b },
        )
        | (
            EvaluationTask::InstanceSegmentation { classes: a, .. },
            EvaluationTask::InstanceSegmentation { classes: b, .. },
        ) => a == b,
        _ => false,
    };
    if compatible {
        Ok(())
    } else {
        Err(invalid("evaluation task differs from the inspection task"))
    }
}

fn validate_recipe(spec: &InspectionSpec, request: &TrainingRequest) -> Result<()> {
    if request.engine == "burn" {
        let config: burn::TrainingConfig =
            serde_json::from_value(request.recipe["config"].clone())?;
        burn::validate_inspection_config(spec, &config).map_err(engine_error)?;
    }
    let labels: Vec<String> = serde_json::from_value(
        request
            .recipe
            .get("labels")
            .cloned()
            .unwrap_or_else(|| serde_json::json!([])),
    )?;
    if labels != spec.labels {
        return Err(invalid("candidate label order differs from the inspection"));
    }
    if request.recipe.get("inspection_task").is_some() {
        let task: TaskKind = serde_json::from_value(request.recipe["inspection_task"].clone())?;
        if task != spec.task {
            return Err(invalid("candidate task differs from the experiment"));
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use flow_like_ml_core::{BoundingBox, InstanceMask};

    fn observed(
        annotation: Annotation,
        evidence: Option<EvidencePrediction>,
        predicted_class: Option<u32>,
    ) -> Observation {
        Observation {
            sample: Sample {
                id: "sample".into(),
                group_id: "group".into(),
                stream_id: "stream".into(),
                timestamp_ms: 0,
                window_start_ms: 0,
                window_end_ms: 0,
                input: TensorData {
                    shape: vec![1],
                    values: vec![0.0],
                },
                annotation,
                provenance: LabelProvenance::Reviewed {
                    reviewer: "fixture".into(),
                    reviewed_at_ms: 0,
                },
                outcome: None,
            },
            predicted_class,
            evidence,
            latency_ms: 1.0,
        }
    }

    fn centered_box(class_id: u32) -> BoundingBox {
        BoundingBox {
            class_id,
            x_min: 0.25,
            y_min: 0.25,
            x_max: 0.75,
            y_max: 0.75,
        }
    }

    #[test]
    fn validation_reports_the_final_audit_metrics_and_goal_checks_accept_them() {
        let labels = vec!["normal".to_string(), "defect".to_string()];
        let anomaly = |is_anomaly, probability| {
            observed(
                Annotation::Anomaly { is_anomaly },
                Some(EvidencePrediction::Anomaly { probability }),
                None,
            )
        };
        let cases = vec![
            (
                EvaluationTask::Anomaly { threshold: 0.5 },
                vec![anomaly(false, 0.2), anomaly(true, 0.8), anomaly(false, 0.6)],
                vec![
                    "accuracy_wilson_lower_95",
                    "log_loss",
                    "precision",
                    "recall",
                    "false_positive_rate",
                    "support/normal",
                    "support/anomaly",
                ],
            ),
            (
                EvaluationTask::Detection {
                    classes: 2,
                    iou_threshold: 0.5,
                },
                vec![observed(
                    Annotation::Boxes {
                        boxes: vec![centered_box(0), centered_box(1)],
                    },
                    Some(EvidencePrediction::Detection {
                        boxes: vec![DetectionEvidence {
                            class_id: 0,
                            x_min: 0.25,
                            y_min: 0.25,
                            x_max: 0.75,
                            y_max: 0.75,
                            confidence: 0.9,
                        }],
                    }),
                    None,
                )],
                vec![
                    "mean_average_precision",
                    "support/1",
                    "recall/0",
                    "precision/0",
                    "average_precision/1",
                ],
            ),
            (
                EvaluationTask::Segmentation { classes: 2 },
                vec![observed(
                    Annotation::Mask {
                        width: 2,
                        height: 1,
                        classes: vec![0, 1],
                    },
                    Some(EvidencePrediction::Segmentation {
                        width: 2,
                        height: 1,
                        classes: vec![0, 1],
                    }),
                    None,
                )],
                vec!["iou/0", "iou/1", "dice/0", "dice/1"],
            ),
            (
                EvaluationTask::InstanceSegmentation {
                    classes: 1,
                    iou_threshold: 0.5,
                },
                vec![observed(
                    Annotation::InstanceMasks {
                        instances: vec![InstanceMask {
                            instance_id: "instance".into(),
                            bounds: centered_box(0),
                            width: 2,
                            height: 1,
                            foreground: vec![true, false],
                        }],
                    },
                    Some(EvidencePrediction::Instances {
                        instances: vec![InstanceEvidence {
                            class_id: 0,
                            confidence: 0.9,
                            width: 2,
                            height: 1,
                            foreground: vec![true, false],
                        }],
                    }),
                    None,
                )],
                vec!["mask_mean_average_precision", "mask_iou/0", "recall/0"],
            ),
            (
                EvaluationTask::Classification,
                vec![
                    observed(Annotation::Class { class_id: 0 }, None, Some(0)),
                    observed(Annotation::Class { class_id: 1 }, None, Some(0)),
                ],
                vec!["macro_f1", "recall/defect", "class/1/support"],
            ),
            (
                EvaluationTask::Regression,
                vec![
                    observed(
                        Annotation::Scalar { value: 1.0 },
                        Some(EvidencePrediction::Regression { values: vec![1.5] }),
                        None,
                    ),
                    observed(
                        Annotation::Scalar { value: 2.0 },
                        Some(EvidencePrediction::Regression { values: vec![2.5] }),
                        None,
                    ),
                ],
                vec!["rmse", "mae", "r2"],
            ),
        ];
        for (task, observations, expected) in cases {
            let metrics = measure(&observations, &task, &labels).unwrap();
            for name in expected {
                assert!(
                    metrics.contains_key(name),
                    "{task:?} validation lacks {name}"
                );
            }
            for name in metrics.keys() {
                assert!(
                    validation_measures(&task, &labels, name),
                    "{task:?} goal check rejects measured {name}"
                );
            }
        }
    }

    #[test]
    fn goals_naming_metrics_validation_never_measures_are_rejected() {
        let spec = InspectionSpec {
            id: "spec".into(),
            description: "classify windows".into(),
            task: TaskKind::SensorClassification,
            labels: vec!["ok".into(), "defect".into()],
            input_shape: vec![4],
            prediction_horizon_ms: None,
            minimum_examples: 1,
            minimum_examples_per_class: 1,
        };
        let mut goals = default_goals(&spec);
        let bound = |name: &str| MetricBound {
            name: name.into(),
            minimum: Some(0.5),
            maximum: None,
        };
        for name in [
            "recall/defect",
            "class/1/recall",
            "accuracy_wilson_lower_95",
            "artifact_bytes",
        ] {
            goals.bounds = vec![bound(name)];
            validate_goals(&spec, &goals).unwrap();
        }
        for name in [
            "teacher_agreement",
            "recall/missing",
            "class/2/recall",
            "accuracy ",
        ] {
            goals.bounds = vec![bound(name)];
            assert!(validate_goals(&spec, &goals).is_err(), "{name}");
        }
        goals.bounds.clear();
        goals.primary_metric = "mean_iou".into();
        assert!(validate_goals(&spec, &goals).is_err());
    }

    #[test]
    fn continuation_epochs_stay_within_eighty_percent_of_the_trial_cap() {
        assert_eq!(epochs_within_trial_cap(60_000, 15_000, 3), 9);
        assert_eq!(epochs_within_trial_cap(60_000, 60_000, 3), 2);
        assert_eq!(epochs_within_trial_cap(60_000, 100_000, 1), 0);
        assert_eq!(epochs_within_trial_cap(60_000, 0, 3), usize::MAX);
    }

    #[test]
    fn batches_halve_until_the_estimate_fits() {
        let recipe = burn::Recipe::ResNet18 {
            input_channels: 3,
            classes: 2,
            base_channels: 16,
        };
        let shape: [usize; 3] = [3, 64, 64];
        let at = |batch| recipe.estimated_training_bytes(&shape, batch).unwrap();
        assert_eq!(fit_batch(&recipe, &shape, 16, 0, at(16)), Some(Ok(16)));
        assert_eq!(fit_batch(&recipe, &shape, 16, 0, at(4)), Some(Ok(4)));
        assert_eq!(fit_batch(&recipe, &shape, 16, 100, at(4) + 99), Some(Ok(2)));
        assert_eq!(
            fit_batch(&recipe, &shape, 16, 0, at(1) - 1),
            Some(Err(at(1)))
        );
    }

    #[test]
    fn boxes_clamped_to_zero_area_are_not_scored() {
        let raw = serde_json::json!({"detections":[[
            {"class_id":0,"confidence":0.9,"x_min":0.2,"y_min":0.2,"x_max":0.6,"y_max":0.6},
            {"class_id":0,"confidence":0.8,"x_min":1.0,"y_min":0.3,"x_max":1.0,"y_max":0.7}
        ]]});
        let sample = observed(
            Annotation::Boxes {
                boxes: vec![centered_box(0)],
            },
            None,
            None,
        )
        .sample;
        let observation = decode_prediction(
            sample,
            &raw,
            &EvaluationTask::Detection {
                classes: 1,
                iou_threshold: 0.5,
            },
            1,
            1.0,
            1.0,
        )
        .unwrap();
        let Some(EvidencePrediction::Detection { boxes }) = observation.evidence else {
            panic!("detection evidence is missing");
        };
        assert_eq!(boxes.len(), 1);
        assert_eq!(boxes[0].x_max, 0.6);
    }

    #[test]
    fn retry_pauses_double_to_their_cap_and_stop_at_the_total_limit() {
        let pauses = |limit_ms: u64| {
            let mut backoff = Backoff::new();
            std::iter::from_fn(|| backoff.next_pause(limit_ms))
                .map(|pause| pause.as_millis() as u64)
                .collect::<Vec<_>>()
        };
        assert_eq!(pauses(2_000), vec![250, 500, 1_000]);
        assert!(pauses(100).is_empty());
        let long = pauses(60_000);
        assert_eq!(long[..7], [250, 500, 1_000, 2_000, 4_000, 8_000, 10_000]);
        assert_eq!(long.len(), 10);
        assert!(
            long[6..]
                .iter()
                .all(|pause| *pause == MAXIMUM_RETRY_DELAY_MS)
        );
        assert!(long.iter().sum::<u64>() <= 60_000);
    }
}
