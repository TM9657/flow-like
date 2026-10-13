use super::auto_training_tables::TableDatasetRequest;
use flow_like::flow::{
    execution::context::ExecutionContext,
    node::{Node, NodeLogic},
};
use flow_like_ml_core::{ComputeConfig, TaskKind};
use flow_like_ml_runtime::experiment::*;
use flow_like_types::{Result, Value, anyhow};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

/// A search has one pinned source and one evaluation contract for every candidate.
#[derive(Clone, Serialize, Deserialize, JsonSchema)]
pub struct AutoTrainRequest {
    pub dataset: TableDatasetRequest,
    #[serde(default)]
    pub compute: ComputeConfig,
    #[serde(default)]
    pub search: AutoSearchConfig,
    #[serde(default)]
    pub budget: ExperimentBudget,
    #[serde(default)]
    pub goals: Option<ExperimentGoals>,
    #[serde(default)]
    pub tables: super::auto_training_tables::TableWritePolicy,
    #[serde(default)]
    pub ordered_target: bool,
    /// Optional task-compatible recipes, including architectures requiring an explicit input layout.
    #[serde(default)]
    pub initial_candidates: Vec<flow_like_ml_runtime::TrainingRequest>,
    /// Include a fine-tuning candidate initialized from an authorized registry source.
    #[serde(default)]
    pub pretrained: Option<flow_like_ml_runtime::PretrainedSourceRef>,
    #[serde(default)]
    pub feature_plan: Option<super::feature_engineering::FeaturePlan>,
    #[serde(default)]
    pub feature_sources: Vec<super::feature_engineering::FeatureSourceInput>,
    #[serde(default)]
    pub learning_cycle_id: Option<String>,
}

#[derive(Clone, Serialize, Deserialize, JsonSchema)]
pub struct ExperimentRequest {
    pub experiment_id: String,
}

#[derive(Clone, Serialize, Deserialize, JsonSchema)]
pub struct PredictAutoModelRequest {
    pub experiment_id: String,
    pub rows: Vec<Value>,
    #[serde(default)]
    pub compute: ComputeConfig,
    #[serde(default)]
    pub stateful_features: bool,
    #[serde(default)]
    pub feature_state: Option<flow_like_ml_native::feature_engineering::FeatureStreamState>,
    /// The extractor of PatchCore or PaDiM embeddings, as Extract Image Features, Extract
    /// Pretrained Image Features or Feature Map to Patches output it. Rows may instead carry it
    /// as `feature_extractor` inside their tensor value. Prediction fails when it differs from
    /// the extractor the model was fitted on.
    #[serde(default)]
    pub feature_extractor: Option<super::lifecycle::FeatureExtractorRef>,
}

#[derive(Clone, Serialize, Deserialize, JsonSchema)]
pub struct AutoModelPredictions {
    pub experiment_id: String,
    pub model_artifact_id: String,
    pub predictions: Vec<Value>,
    pub feature_state: Option<flow_like_ml_native::feature_engineering::FeatureStreamState>,
    /// Anomaly experiments only: the decision rule the final audit measured, for each row.
    #[serde(default)]
    pub anomaly: Option<AnomalyDecisions>,
}

/// Raw anomaly scores are calibrated as score / (score + scale) with the selected trial's
/// scale; a calibrated probability at or above the threshold is an anomaly.
#[derive(Clone, Serialize, Deserialize, JsonSchema)]
pub struct AnomalyDecisions {
    pub scale: f64,
    pub threshold: f64,
    pub rows: Vec<AnomalyDecision>,
}

#[derive(Clone, Serialize, Deserialize, JsonSchema)]
pub struct AnomalyDecision {
    pub score: f64,
    pub probability: f64,
    pub is_anomaly: bool,
}

#[derive(Clone, Serialize, Deserialize, JsonSchema)]
pub struct ExportAutoModelRequest {
    pub experiment_id: String,
    pub weights: flow_like_catalog_core::FlowPath,
    pub manifest: flow_like_catalog_core::FlowPath,
}

#[derive(Clone, Serialize, Deserialize, JsonSchema)]
pub struct ExportAutoModelResult {
    pub model_artifact_id: String,
    pub weights: flow_like_catalog_core::FlowPath,
    pub manifest: flow_like_catalog_core::FlowPath,
    pub sha256: String,
    pub bytes: u64,
}

#[derive(Clone, Serialize, Deserialize, JsonSchema)]
pub struct AutoTrainingResult {
    pub experiment_id: String,
    pub status: ExperimentStatus,
    pub stop_reason: Option<String>,
    pub goal_met: bool,
    pub unmet_constraints: Vec<String>,
    pub best_trial_id: Option<String>,
    pub model_artifact_id: Option<String>,
    pub model_bundle_ref: Option<flow_like_ml_runtime::ModelArtifact>,
    pub preprocessing_manifest: Value,
    pub source_table_versions: Vec<Value>,
    pub created_tables: Vec<Value>,
    pub updated_table_versions: Vec<Value>,
    pub dataset_snapshot_ids: Vec<String>,
    pub leaderboard: Vec<ExperimentTrial>,
    pub validation_metrics: std::collections::BTreeMap<String, f64>,
    pub final_test_metrics: std::collections::BTreeMap<String, f64>,
    pub predictions_table: Option<Value>,
    pub other_artifacts: Vec<Value>,
    pub budget_used: ExperimentUsage,
}

impl From<ExperimentResult> for AutoTrainingResult {
    fn from(result: ExperimentResult) -> Self {
        let exp = result.experiment;
        let best = exp
            .selected_trial_id
            .as_ref()
            .or(exp.best_trial_id.as_ref());
        let trial = result.trials.iter().find(|trial| Some(&trial.id) == best);
        let validation_metrics = trial
            .map(|trial| trial.validation_metrics.clone())
            .unwrap_or_default();
        let preprocessing_manifest = trial
            .and_then(|trial| {
                result
                    .datasets
                    .iter()
                    .find(|data| data.snapshot_id == trial.dataset_snapshot_id)
            })
            .map(|data| data.preprocessing_manifest.clone())
            .unwrap_or(exp.request.preprocessing_manifest);
        let final_test_metrics = result
            .final_evaluation
            .as_ref()
            .map(|evaluation| evaluation.metrics.clone())
            .unwrap_or_default();
        let mut other_artifacts = result.events;
        if let Some(evaluation) = result.final_evaluation {
            other_artifacts.push(flow_like_types::json::json!(evaluation));
        }
        let predictions_table = exp
            .request
            .created_tables
            .iter()
            .chain(&exp.request.updated_table_versions)
            .find(|table| table.get("partition").and_then(Value::as_str) == Some("predictions"))
            .cloned();
        Self {
            experiment_id: exp.id,
            status: exp.status,
            stop_reason: exp.stop_reason,
            goal_met: exp.target_met,
            unmet_constraints: exp.unmet_constraints,
            best_trial_id: exp.selected_trial_id.or(exp.best_trial_id),
            model_artifact_id: result
                .best_artifact
                .as_ref()
                .map(|artifact| artifact.id.clone()),
            model_bundle_ref: result.best_artifact,
            preprocessing_manifest,
            source_table_versions: exp.request.source_table_versions,
            created_tables: exp.request.created_tables,
            updated_table_versions: exp.request.updated_table_versions,
            dataset_snapshot_ids: result
                .datasets
                .into_iter()
                .map(|data| data.snapshot_id)
                .collect(),
            leaderboard: result.trials,
            validation_metrics,
            final_test_metrics,
            predictions_table,
            other_artifacts,
            budget_used: exp.usage,
        }
    }
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum AutoTrainTask {
    Tabular,
    Forecast,
    Vision,
    Anomaly,
}

fn validate_task(facade: AutoTrainTask, task: &TaskKind) -> Result<()> {
    let allowed = match facade {
        AutoTrainTask::Tabular => matches!(
            task,
            TaskKind::SensorClassification | TaskKind::SensorRegression
        ),
        AutoTrainTask::Forecast => matches!(
            task,
            TaskKind::SequenceForecast
                | TaskKind::SensorClassification
                | TaskKind::VisualSequenceClassification
        ),
        AutoTrainTask::Vision => matches!(
            task,
            TaskKind::ImageClassification
                | TaskKind::ObjectDetection
                | TaskKind::Segmentation
                | TaskKind::InstanceSegmentation
                | TaskKind::Fusion
        ),
        AutoTrainTask::Anomaly => matches!(
            task,
            TaskKind::SensorAnomaly | TaskKind::VisualAnomaly | TaskKind::SequenceAutoencoder
        ),
    };
    if !allowed {
        return Err(anyhow!(
            "The selected auto trainer does not support this inspection task"
        ));
    }
    Ok(())
}

macro_rules! task_node {
    ($node:ident, $id:literal, $name:literal, $function:literal, $task:ident) => {
        #[flow_like_types::async_trait]
        impl NodeLogic for $node {
            fn get_node(&self) -> Node {
                super::operation_node::<AutoTrainRequest, AutoTrainingResult>(
                    $id,
                    $name,
                    "Pin a dataset and start a durable, bounded model search",
                    $function,
                    "AI/ML/Auto Training",
                )
            }
            async fn run(&self, context: &mut ExecutionContext) -> Result<()> {
                #[cfg(feature = "training")]
                {
                    context.deactivate_exec_pin("exec_out").await?;
                    let request: AutoTrainRequest = context.evaluate_pin("request").await?;
                    validate_task(AutoTrainTask::$task, &request.dataset.spec.task)?;
                    let result = start_auto_training(context, request, true).await?;
                    context
                        .set_pin_value("result", flow_like_types::json::json!(result))
                        .await?;
                    context.activate_exec_pin("exec_out").await?;
                    Ok(())
                }
                #[cfg(not(feature = "training"))]
                {
                    let _ = context;
                    Err(anyhow!("Auto training requires a local training build"))
                }
            }
        }
    };
}

#[crate::register_node]
#[derive(Default)]
pub struct AutoTrainTabularNode;
#[crate::register_node]
#[derive(Default)]
pub struct AutoTrainForecastNode;
#[crate::register_node]
#[derive(Default)]
pub struct AutoTrainVisionNode;
#[crate::register_node]
#[derive(Default)]
pub struct AutoTrainAnomalyNode;

task_node!(
    AutoTrainTabularNode,
    "ml_auto_train_tabular",
    "Auto Train Tabular",
    "autoTrainTabular",
    Tabular
);
task_node!(
    AutoTrainForecastNode,
    "ml_auto_train_forecast",
    "Auto Train Forecast",
    "autoTrainForecast",
    Forecast
);
task_node!(
    AutoTrainVisionNode,
    "ml_auto_train_vision",
    "Auto Train Vision",
    "autoTrainVision",
    Vision
);
task_node!(
    AutoTrainAnomalyNode,
    "ml_auto_train_anomaly",
    "Auto Train Anomaly",
    "autoTrainAnomaly",
    Anomaly
);

#[cfg(feature = "training")]
pub fn experiment_repository(
    context: &ExecutionContext,
    id: &str,
) -> Result<flow_like_ml_runtime::TrainingRepository> {
    let (repo, project) = super::lifecycle::repository(context)?;
    let experiment = repo.get_experiment(id)?;
    if experiment.request.stream.project_id != project {
        return Err(anyhow!(
            "The experiment belongs to a different app or board"
        ));
    }
    Ok(repo)
}

#[cfg(feature = "training")]
pub fn controller(
    repo: flow_like_ml_runtime::TrainingRepository,
    budget: &ExperimentBudget,
) -> Result<flow_like_ml_runtime::auto_training::AutoTrainingController> {
    use flow_like_ml_runtime::{
        auto_training::AutoTrainingController, engines, worker::TrainingWorker,
    };
    let mut worker = TrainingWorker::new(repo.clone(), budget.worker_limits.clone())?;
    engines::register_native_engines(&mut worker)?;
    engines::register_burn_engine(&mut worker)?;
    engines::register_efficient_ad_engine(&mut worker)?;
    super::auto_training_legacy::register_legacy_engine(&mut worker)?;
    Ok(
        AutoTrainingController::new(repo, worker).with_predictor_factory(std::sync::Arc::new(
            super::auto_training_legacy::CatalogSearchPredictorFactory,
        )),
    )
}

#[cfg(feature = "training")]
fn snapshot_from_table(
    prepared: &super::auto_training_tables::PreparedTableDataset,
    stream: flow_like_ml_runtime::StreamKey,
    policy: flow_like_ml_runtime::SplitPolicy,
) -> Result<flow_like_ml_runtime::DatasetSnapshot> {
    use flow_like_ml_core::{Annotation, LabelProvenance};
    use flow_like_ml_runtime::{DatasetSnapshot, LabelSource, TrainingSample};
    prepared.split.validate(&prepared.samples)?;
    let convert = |indices: &[usize]| -> Result<Vec<TrainingSample>> {
        indices
            .iter()
            .map(|&index| {
                let mut sample = prepared.samples[index].clone();
                sample.stream_id = stream.stream_id.clone();
                let (source, available) = match &sample.provenance {
                    LabelProvenance::Teacher { .. } => (LabelSource::Teacher, sample.timestamp_ms),
                    LabelProvenance::Reviewed { reviewed_at_ms, .. } => {
                        (LabelSource::Reviewed, *reviewed_at_ms)
                    }
                    LabelProvenance::Measured { .. } => (
                        LabelSource::ObservedOutcome,
                        sample
                            .outcome
                            .as_ref()
                            .map_or(sample.timestamp_ms, |outcome| outcome.available_at_ms),
                    ),
                    LabelProvenance::Unlabeled => {
                        return Err(anyhow!("Training rows must have labels with provenance"));
                    }
                };
                if matches!(sample.annotation, Annotation::Unlabeled) {
                    return Err(anyhow!("A training row is unlabelled"));
                }
                let mut payload = flow_like_types::json::json!(sample);
                if let Some(row) = prepared.raw_rows.get(index) {
                    payload["feature_row"] = row.clone();
                }
                if let Annotation::Class { class_id } = &sample.annotation {
                    payload["label"] = flow_like_types::json::json!(
                        prepared
                            .spec
                            .labels
                            .get(*class_id as usize)
                            .ok_or_else(|| anyhow!("Class exceeds the declared label order"))?
                    );
                }
                Ok(TrainingSample {
                    id: sample.id.clone(),
                    annotation_revision: u64::try_from(
                        prepared
                            .annotation_revisions
                            .get(&sample.id)
                            .copied()
                            .unwrap_or(1),
                    )
                    .map_err(|_| anyhow!("Annotation revision must be positive"))?,
                    group_id: sample.group_id.clone(),
                    captured_at_ms: sample.timestamp_ms,
                    label_available_at_ms: available.max(
                        prepared
                            .label_available_at_ms
                            .get(&sample.id)
                            .copied()
                            .unwrap_or(available),
                    ),
                    content_digest: prepared
                        .raw_content_digests
                        .get(&sample.id)
                        .cloned()
                        .unwrap_or(flow_like_ml_core::content_digest(
                            &flow_like_types::json::to_vec(&sample.input)?,
                        )),
                    source,
                    accepted: true,
                    payload,
                })
            })
            .collect()
    };
    Ok(DatasetSnapshot {
        id: String::new(),
        digest: String::new(),
        stream: stream.clone(),
        as_of_ms: flow_like_ml_runtime::now_ms(),
        cutoff_sequence: prepared.samples.len() as i64,
        policy,
        train: convert(&prepared.split.train)?,
        validation: convert(&prepared.split.validation)?,
        test: convert(&prepared.split.test)?,
        excluded: prepared
            .split
            .purged
            .iter()
            .map(|&index| prepared.samples[index].id.clone())
            .collect(),
    })
}

#[cfg(feature = "training")]
fn table_split_policy(
    policy: &super::auto_training_tables::TableSplitPolicy,
) -> Result<flow_like_ml_runtime::SplitPolicy> {
    use super::auto_training_tables::TableSplitPolicy;
    Ok(match policy {
        TableSplitPolicy::Group {
            train_fraction,
            validation_fraction,
            seed,
        } => flow_like_ml_runtime::SplitPolicy::Group {
            train_fraction: *train_fraction,
            validation_fraction: *validation_fraction,
            seed: *seed,
        },
        TableSplitPolicy::Time {
            train_end_ms,
            validation_end_ms,
            embargo_ms,
        } => flow_like_ml_runtime::SplitPolicy::Time {
            train_end_ms: *train_end_ms,
            validation_end_ms: *validation_end_ms,
            embargo_ms: i64::try_from(*embargo_ms)
                .map_err(|_| anyhow!("Temporal embargo exceeds the supported range"))?,
        },
    })
}

#[cfg(feature = "training")]
pub async fn start_auto_training(
    context: &mut ExecutionContext,
    request: AutoTrainRequest,
    background: bool,
) -> Result<AutoTrainingResult> {
    start_auto_training_scoped(context, request, background, None).await
}

#[cfg(feature = "training")]
pub async fn start_auto_training_scoped(
    context: &mut ExecutionContext,
    mut request: AutoTrainRequest,
    background: bool,
    stream: Option<flow_like_ml_runtime::StreamKey>,
) -> Result<AutoTrainingResult> {
    use flow_like_ml_runtime::auto_training::{default_goals, validate_goals, validate_search};
    use flow_like_types::json::json;
    let (repo, project) = super::lifecycle::repository(context)?;
    let original_spec = request.dataset.spec.clone();
    let scoped_stream = stream
        .as_ref()
        .map(|value| {
            super::lifecycle::InspectionContext {
                stream_id: value.stream_id.clone(),
                spec: original_spec.clone(),
            }
            .key(project.clone())
        })
        .transpose()?;
    if stream
        .as_ref()
        .is_some_and(|value| Some(value) != scoped_stream.as_ref())
    {
        return Err(anyhow!(
            "Training stream belongs to another project or inspection contract"
        ));
    }
    if let Some(cycle_id) = &request.learning_cycle_id {
        if background {
            return Err(anyhow!(
                "Learning cycles must attach their experiment before dispatch"
            ));
        }
        let cycle = repo.get_learning_cycle(cycle_id)?;
        let learning = repo.get_learning_project(&cycle.project_id)?;
        if scoped_stream.as_ref() != Some(&learning.request.stream) {
            return Err(anyhow!("Learning cycle does not own this training stream"));
        }
    }
    if background {
        background::ensure_capacity()?;
    }
    let mut prepared = if let Some(plan) = &request.feature_plan {
        super::feature_engineering::prepare_feature_dataset(
            context,
            &request.dataset,
            plan,
            &request.feature_sources,
        )
        .await?
    } else {
        super::auto_training_tables::prepare_table_dataset(context, &request.dataset).await?
    };
    if request.feature_plan.is_none() {
        prepared.feature_sources =
            super::feature_engineering::pin_feature_sources(context, &request.feature_sources)
                .await?;
    }
    request.dataset.spec = prepared.spec.clone();
    request.dataset.source = prepared.source.database.clone();
    let expected_stream = super::lifecycle::InspectionContext {
        stream_id: format!("auto:{}", uuid::Uuid::new_v4()),
        spec: prepared.spec.clone(),
    }
    .key(project)?;
    let stream = scoped_stream.unwrap_or(expected_stream);
    let split = table_split_policy(&request.dataset.split)?;
    let (prepared, snapshot, extractor) = {
        let stream = stream.clone();
        tokio::task::spawn_blocking(move || -> Result<_> {
            let snapshot = snapshot_from_table(&prepared, stream, split)?;
            let extractor = requested_extractor(&prepared.preprocessing, &prepared.raw_rows, None)?;
            Ok((prepared, snapshot, extractor))
        })
        .await??
    };
    request.search.maximum_candidates = request
        .search
        .maximum_candidates
        .min(request.budget.maximum_trials);
    if let Some(sample) = prepared
        .split
        .train
        .first()
        .map(|&index| &prepared.samples[index])
    {
        request.search.output_features = match &sample.annotation {
            flow_like_ml_core::Annotation::Values { values } => values.len(),
            _ => 1,
        };
    }
    validate_search(&prepared.spec, &request.search, &request.compute)?;
    let mut candidates = start_candidates(&repo, &stream.project_id, &prepared.spec, &request)?;
    record_feature_extractor(&mut candidates, extractor.as_ref())?;
    let mut goals = request
        .goals
        .clone()
        .unwrap_or_else(|| default_goals(&prepared.spec));
    if request.ordered_target && request.goals.is_none() {
        goals.primary_metric = "mean_rank_error".into();
        goals.direction = MetricDirection::Minimize;
    }
    validate_goals(&prepared.spec, &goals)?;
    validate_table_policy(&prepared, &request.tables)?;
    // The split and its training view are both charged to the dataset budget.
    let snapshot_bytes = super::auto_training_tables::serialized_json_size(&snapshot)? as u64;
    if snapshot_bytes.saturating_mul(2) > request.budget.maximum_dataset_bytes {
        return Err(anyhow!(
            "The prepared split serializes to {snapshot_bytes} bytes and is stored twice, which exceeds budget.maximum_dataset_bytes ({}); raise the dataset budget or select fewer rows or columns",
            request.budget.maximum_dataset_bytes
        ));
    }
    let experiment_request = flow_like_ml_runtime::experiment::ExperimentRequest {
        stream: stream.clone(),
        snapshot_id: String::new(),
        spec: json!(prepared.spec),
        candidates,
        goals,
        budget: request.budget.clone(),
        source_table_versions: std::iter::once(json!(prepared.source.reference))
            .chain(
                prepared.feature_sources.iter().map(
                    |source| json!({"alias":source.alias,"reference":source.source.reference}),
                ),
            )
            .collect(),
        created_tables: vec![],
        updated_table_versions: vec![],
        preprocessing_manifest: json!(prepared.preprocessing),
        context: json!({"training_profile":prepared.profile,"search":request.search,"compute":request.compute,"pretrained":request.pretrained,"dataset_request":request.dataset,"table_policy":request.tables,"pinned_source":prepared.source,"ordered_target":request.ordered_target,"feature_plan":request.feature_plan,"feature_sources":prepared.feature_sources,"learning_cycle_id":request.learning_cycle_id}),
    };
    let experiment = create_started_experiment(
        context.get_cancellation_token(),
        &repo,
        &stream,
        snapshot,
        experiment_request,
    )
    .await?;
    // A start dropped while its tables are written must not leave an undriven Running experiment.
    let guard = super::cancellation::CancelOnDrop::new({
        let (repo, id) = (repo.clone(), experiment.id.clone());
        move || {
            let _ = repo.cancel_experiment(&id, flow_like_ml_runtime::now_ms());
        }
    });
    let activated = activate_experiment(
        context,
        &repo,
        &prepared,
        &experiment.id,
        &request.tables,
        background,
    )
    .await;
    guard.disarm();
    if let Err(error) = activated {
        abandon_experiment(&repo, &experiment.id, &error);
        return Err(anyhow!(
            "Auto training experiment {} was saved but could not start, so it was cancelled: {error}. Fix the cause and resume it by ID, or start a new experiment",
            experiment.id
        ));
    }
    repo.experiment_result(&experiment.id)
        .map(AutoTrainingResult::from)
        .map_err(|error| {
            anyhow!(
                "Auto training experiment {} started, but its result could not be read: {error}",
                experiment.id
            )
        })
}

/// Explicit, pretrained, classical and generated candidates, validated before anything is
/// written to the ledger.
#[cfg(feature = "training")]
fn start_candidates(
    repo: &flow_like_ml_runtime::TrainingRepository,
    project_id: &str,
    spec: &flow_like_ml_core::InspectionSpec,
    request: &AutoTrainRequest,
) -> Result<Vec<flow_like_ml_runtime::TrainingRequest>> {
    let mut candidates = request.initial_candidates.clone();
    if let Some(pretrained) = &request.pretrained {
        candidates.insert(
            0,
            pretrained_candidate(
                repo,
                project_id,
                spec,
                pretrained,
                &request.search,
                &request.compute,
            )?,
        );
    }
    for candidate in &candidates {
        if candidate.engine == "burn" {
            let config: flow_like_ml_burn::TrainingConfig = flow_like_types::json::from_value(
                candidate
                    .recipe
                    .get("config")
                    .cloned()
                    .ok_or_else(|| anyhow!("Burn candidate needs a training config"))?,
            )?;
            // A fine-tune keeps its source architecture; explicit recipes rely on validate().
            if candidate
                .recipe
                .get("pretrained")
                .is_some_and(|pretrained| !pretrained.is_null())
            {
                config.validate_fine_tune_planning_budget()?;
            }
            flow_like_ml_burn::validate_inspection_config(spec, &config)?;
            validate_candidate_pretrained(
                repo,
                project_id,
                candidate,
                &spec.labels,
                request.budget.worker_limits.memory_budget_bytes,
            )?;
        }
    }
    if spec.task == TaskKind::SensorClassification && spec.input_shape.len() == 1 {
        candidates.extend(legacy_candidates(
            spec,
            &request.compute,
            request.ordered_target,
        )?);
    } else if request.ordered_target {
        return Err(anyhow!(
            "Ordered targets require a tabular classification task with declared labels"
        ));
    }
    let compute = generation_compute(&request.compute, &request.budget);
    match flow_like_ml_runtime::auto_training::generate_candidates(spec, &request.search, &compute)
    {
        Ok(generated) => candidates.extend(generated),
        Err(error) if candidates.is_empty() => return Err(error.into()),
        Err(_) => {}
    }
    candidates.truncate(request.search.maximum_candidates);
    Ok(candidates)
}

/// Generated batches are fitted to the memory a trial can actually reserve.
#[cfg(feature = "training")]
fn generation_compute(compute: &ComputeConfig, budget: &ExperimentBudget) -> ComputeConfig {
    ComputeConfig {
        memory_limit_bytes: compute
            .memory_limit_bytes
            .min(budget.worker_limits.memory_budget_bytes),
        ..compute.clone()
    }
}

/// The deterministic table-write rules, checked before the experiment exists.
#[cfg(feature = "training")]
fn validate_table_policy(
    prepared: &super::auto_training_tables::PreparedTableDataset,
    policy: &super::auto_training_tables::TableWritePolicy,
) -> Result<()> {
    if !policy.create_tables && policy.allowed_destination.is_none() {
        return Ok(());
    }
    if policy.maximum_rows > 1_000_000 || policy.maximum_bytes > 1024 * 1024 * 1024 {
        return Err(anyhow!(
            "Table write policy allows at most 1,000,000 rows and 1 GiB per table, not {} rows and {} bytes",
            policy.maximum_rows,
            policy.maximum_bytes
        ));
    }
    let split = &prepared.split;
    let rows = if policy.allowed_destination.is_some() {
        if policy.expected_destination.is_none() {
            return Err(anyhow!(
                "Existing table writes require its expected durable reference"
            ));
        }
        split.train.len() + split.validation.len() + split.test.len()
    } else {
        let prefix = policy.table_prefix.as_deref().unwrap_or("ml_experiment");
        if prefix.is_empty()
            || prefix.len() > 48
            || !prefix
                .bytes()
                .all(|byte| byte.is_ascii_alphanumeric() || byte == b'_')
        {
            return Err(anyhow!(
                "Table prefix '{prefix}' must be 1 to 48 ASCII letters, digits or underscores"
            ));
        }
        split
            .train
            .len()
            .max(split.validation.len())
            .max(split.test.len())
    };
    if rows > policy.maximum_rows {
        return Err(anyhow!(
            "A prepared table of {rows} rows exceeds the table write policy's maximum_rows ({})",
            policy.maximum_rows
        ));
    }
    Ok(())
}

/// Record the samples and create the experiment off the async executor. A cancelled run, or a
/// node future the executor dropped, stops between sample batches.
#[cfg(feature = "training")]
async fn create_started_experiment(
    token: Option<flow_like_types::tokio_util::sync::CancellationToken>,
    repo: &flow_like_ml_runtime::TrainingRepository,
    stream: &flow_like_ml_runtime::StreamKey,
    snapshot: flow_like_ml_runtime::DatasetSnapshot,
    request: flow_like_ml_runtime::experiment::ExperimentRequest,
) -> Result<Experiment> {
    let stop = super::cancellation::CancelFlag::default();
    let guard = super::cancellation::CancelOnDrop::new({
        let stop = stop.clone();
        move || stop.cancel()
    });
    let (repo, stream) = (repo.clone(), stream.clone());
    let work = tokio::task::spawn_blocking(move || {
        let cancelled =
            || stop.is_cancelled() || token.as_ref().is_some_and(|token| token.is_cancelled());
        record_and_create(&repo, &stream, snapshot, request, &cancelled)
    });
    let created = work.await;
    guard.disarm();
    created?
}

#[cfg(feature = "training")]
fn record_and_create(
    repo: &flow_like_ml_runtime::TrainingRepository,
    stream: &flow_like_ml_runtime::StreamKey,
    snapshot: flow_like_ml_runtime::DatasetSnapshot,
    request: flow_like_ml_runtime::experiment::ExperimentRequest,
    cancelled: &dyn Fn() -> bool,
) -> Result<Experiment> {
    const SAMPLE_BATCH: usize = 256;
    let stopped = || anyhow!("Auto training start was cancelled before its experiment was created");
    for (index, sample) in snapshot
        .train
        .iter()
        .chain(&snapshot.validation)
        .chain(&snapshot.test)
        .enumerate()
    {
        if index % SAMPLE_BATCH == 0 && cancelled() {
            return Err(stopped());
        }
        repo.record_prepared_sample(stream, sample)?;
    }
    if cancelled() {
        return Err(stopped());
    }
    let experiment =
        repo.create_experiment_with_snapshot(snapshot, request, flow_like_ml_runtime::now_ms())?;
    if cancelled() {
        let _ = repo.cancel_experiment(&experiment.id, flow_like_ml_runtime::now_ms());
        return Err(anyhow!(
            "Auto training start was cancelled; experiment {} was saved, cancelled and can be resumed",
            experiment.id
        ));
    }
    Ok(experiment)
}

/// Steps after creation; a failure here leaves a saved experiment the caller must stop.
#[cfg(feature = "training")]
async fn activate_experiment(
    context: &mut ExecutionContext,
    repo: &flow_like_ml_runtime::TrainingRepository,
    prepared: &super::auto_training_tables::PreparedTableDataset,
    id: &str,
    policy: &super::auto_training_tables::TableWritePolicy,
    background: bool,
) -> Result<()> {
    let tables =
        super::auto_training_tables::materialize_prepared_tables(context, prepared, id, policy)
            .await?;
    let references = tables
        .into_iter()
        .map(|table| flow_like_types::json::json!(table))
        .collect();
    if policy.allowed_destination.is_some() {
        repo.update_experiment_tables(id, vec![], references, flow_like_ml_runtime::now_ms())?;
    } else {
        repo.update_experiment_tables(id, references, vec![], flow_like_ml_runtime::now_ms())?;
    }
    if background {
        enqueue_experiment(repo.clone(), id.to_string())?;
    }
    Ok(())
}

/// A saved experiment that cannot start is cancelled, so it never looks driven while nothing
/// runs it; the event keeps the cause.
#[cfg(feature = "training")]
fn abandon_experiment(
    repo: &flow_like_ml_runtime::TrainingRepository,
    id: &str,
    error: &flow_like_types::Error,
) {
    note_experiment(
        repo,
        id,
        "start_failed",
        flow_like_types::json::json!({"error": bounded_text(&error.to_string())}),
    );
    if let Err(cancel) = repo.cancel_experiment(id, flow_like_ml_runtime::now_ms()) {
        tracing::warn!(experiment_id = %id, error = %cancel, "Could not cancel an experiment whose start failed");
    }
}

/// Diagnostic events never change experiment state; a failed write only loses the note.
#[cfg(feature = "training")]
fn note_experiment(
    repo: &flow_like_ml_runtime::TrainingRepository,
    id: &str,
    kind: &str,
    body: Value,
) {
    if let Err(error) = repo.record_experiment_event(id, kind, body, flow_like_ml_runtime::now_ms())
    {
        tracing::warn!(experiment_id = %id, kind, %error, "Could not record an experiment event");
    }
}

/// Error text kept in events stays far below the 64 KiB event limit.
#[cfg(feature = "training")]
fn bounded_text(text: &str) -> String {
    const LIMIT: usize = 4096;
    if text.len() <= LIMIT {
        return text.to_owned();
    }
    let mut end = LIMIT;
    while !text.is_char_boundary(end) {
        end -= 1;
    }
    format!("{} [truncated]", &text[..end])
}

#[cfg(feature = "training")]
fn validate_candidate_pretrained(
    repo: &flow_like_ml_runtime::TrainingRepository,
    project_id: &str,
    candidate: &flow_like_ml_runtime::TrainingRequest,
    labels: &[String],
    worker_memory_bytes: u64,
) -> Result<()> {
    let recipe: flow_like_ml_runtime::engines::BurnEngineRecipe =
        flow_like_types::json::from_value(candidate.recipe.clone())?;
    if let Some(pretrained) = &recipe.pretrained {
        let compute: ComputeConfig = flow_like_types::json::from_value(candidate.compute.clone())?;
        compute.validate()?;
        let source = flow_like_ml_runtime::engines::validate_burn_pretrained_source(
            repo,
            project_id,
            pretrained,
            &recipe.config,
            labels,
            (compute.memory_limit_bytes.min(worker_memory_bytes) / 4).min(512 * 1024 * 1024),
        )?;
        flow_like_ml_runtime::engines::validate_burn_pretrained_preprocessing(
            &source,
            &recipe.preprocessing,
        )?;
    }
    Ok(())
}

#[cfg(feature = "training")]
fn pretrained_candidate(
    repo: &flow_like_ml_runtime::TrainingRepository,
    project_id: &str,
    spec: &flow_like_ml_core::InspectionSpec,
    pretrained: &flow_like_ml_runtime::PretrainedSourceRef,
    search: &AutoSearchConfig,
    compute: &ComputeConfig,
) -> Result<flow_like_ml_runtime::TrainingRequest> {
    let source = repo.get_pretrained_source(&pretrained.source_id)?;
    if source.project_id != project_id {
        return Err(anyhow!("Pretrained source belongs to a different project"));
    }
    let mut config: flow_like_ml_burn::TrainingConfig =
        flow_like_types::json::from_value(source.manifest["config"].clone())?;
    if pretrained.replace_head && !spec.labels.is_empty() {
        let mut recipe = flow_like_types::json::to_value(&config.recipe)?;
        let classes = flow_like_types::json::json!(spec.labels.len());
        if recipe.get("classes").is_some() {
            recipe["classes"] = classes;
        } else if recipe.pointer("/config/classes").is_some() {
            recipe["config"]["classes"] = classes;
        } else {
            return Err(anyhow!(
                "Automatic fine-tuning requires a replaceable class head"
            ));
        }
        config.recipe = flow_like_types::json::from_value(recipe)?;
    }
    config.backend = flow_like_ml_burn::BackendChoice::Auto;
    config.epochs = search.min_epochs;
    config.batch_size = search.batch_size;
    // A frozen backbone trains only the new head, which must leave its initialization within
    // the first search rung to survive pruning.
    config.learning_rate = if pretrained.freeze_backbone {
        0.001
    } else {
        0.0001
    };
    config.seed = search.seed;
    config.gradient_clip = 5.;
    flow_like_ml_burn::validate_inspection_config(spec, &config)?;
    Ok(flow_like_ml_runtime::TrainingRequest {
        engine: "burn".into(),
        recipe: flow_like_types::json::json!({
            "config":config,"labels":spec.labels,"inspection_task":spec.task,
            "pretrained":pretrained,
            "search":{"rung":0,"min_epochs":search.min_epochs,"max_epochs":search.max_epochs,"reduction_factor":search.reduction_factor},
        }),
        compute: flow_like_types::json::to_value(compute)?,
    })
}

#[cfg(feature = "training")]
fn legacy_candidates(
    spec: &flow_like_ml_core::InspectionSpec,
    compute: &ComputeConfig,
    ordered: bool,
) -> Result<Vec<flow_like_ml_runtime::TrainingRequest>> {
    use super::auto_training_legacy::{LegacyAlgorithm, LegacyRecipe};
    let algorithms = if ordered {
        vec![
            LegacyAlgorithm::OrdinalRidge { alpha: 1.0 },
            LegacyAlgorithm::OrdinalLogistic {
                alpha: 1.0,
                max_iterations: 500,
            },
        ]
    } else {
        vec![
            LegacyAlgorithm::GaussianNaiveBayes,
            LegacyAlgorithm::DecisionTree {
                max_depth: 8,
                min_leaf: 2,
            },
            LegacyAlgorithm::Logistic {
                alpha: 1.0,
                max_iterations: 200,
            },
        ]
    };
    algorithms
        .into_iter()
        .map(|algorithm| {
            Ok(flow_like_ml_runtime::TrainingRequest {
                engine: "legacy_classical".into(),
                recipe: flow_like_types::json::to_value(LegacyRecipe {
                    algorithm,
                    labels: spec.labels.clone(),
                })?,
                compute: flow_like_types::json::json!(compute),
            })
        })
        .collect()
}

#[cfg(feature = "training")]
pub fn consultation_context(
    repo: &flow_like_ml_runtime::TrainingRepository,
    id: &str,
) -> Result<Value> {
    let state = repo.experiment_result(id)?;
    let experiment = state.experiment;
    let pretrained_sources = repo.list_pretrained_sources_for_engine(&experiment.request.stream.project_id, 16, "burn")?
        .into_iter()
        .map(|source| flow_like_types::json::json!({
            "source_id":source.id,"source_kind":source.source_kind,
            "recipe":source.manifest.pointer("/config/recipe"),
            "input_shape":source.manifest.get("input_shape"),
            "label_count":source.manifest.get("labels").and_then(Value::as_array).map_or(0,Vec::len),
            "head_pretrained":source.manifest.pointer("/imported/classification_head_pretrained"),
            "recommended_preprocessing":source.manifest.get("recommended_preprocessing"),
            "imported_provenance":source.manifest.get("imported").filter(|value| !value.is_null())
                .or_else(|| source.manifest.pointer("/pretrained/training/imported"))
                .or_else(|| source.manifest.pointer("/pretrained/imported")),
            "bytes":source.blob.bytes,"sha256":source.blob.sha256,
        })).collect::<Vec<_>>();
    let trials = state.trials.iter().map(|trial| -> Result<(String, Value)> {
        let job = repo.get_job(&trial.job_id)?;
        let config = if job.request.engine == "burn" {
            flow_like_types::json::from_value::<flow_like_ml_burn::TrainingConfig>(job.request.recipe["config"].clone()).ok().map(|config| flow_like_types::json::json!(config)).unwrap_or(Value::Null)
        } else {
            let mut summary = flow_like_types::json::Map::new();
            for key in ["objective","estimators","max_depth","max_bins","min_leaf","learning_rate","l2","trees","sample_size","seed","epochs","batch_size","autoencoder_channels","hard_quantile","gradient_clip"] {
                if let Some(value) = job.request.recipe["config"].get(key).filter(|value| !value.is_object() && !value.is_array()) { summary.insert(key.into(),value.clone()); }
            }
            if let Ok(recipe) = flow_like_types::json::from_value::<super::auto_training_legacy::LegacyRecipe>(job.request.recipe.clone()) {
                summary.insert("algorithm".into(),flow_like_types::json::json!(recipe.algorithm));
            }
            Value::Object(summary)
        };
        Ok((trial.id.clone(), flow_like_types::json::json!({
            "trial_id":trial.id,"candidate_index":trial.candidate_index,"status":trial.status,
            "engine":job.request.engine,"config":config,"pretrained":job.request.recipe.get("pretrained"),"dataset_snapshot_id":trial.dataset_snapshot_id,
            "validation_metrics":trial.validation_metrics,"error":trial.error,
            "training_time_ms":trial.training_time_ms,"progress":job.progress,
        })))
    }).collect::<Result<Vec<_>>>()?;
    let datasets = state.datasets.iter().map(|dataset| {
        let preprocessing = flow_like_types::json::from_value::<super::auto_training_tables::TablePreprocessing>(dataset.preprocessing_manifest.clone()).ok();
        let (mut shape, numeric, categorical) = match preprocessing.as_ref().map(super::auto_training_tables::base_preprocessing) {
            Some(super::auto_training_tables::TablePreprocessing::Tabular { fitted }) => {
                use flow_like_ml_native::preprocessing::FittedColumn;
                let numeric:Vec<_> = fitted.columns.iter().filter_map(|column| if let FittedColumn::Numeric { name,.. } = column {Some(name)} else {None}).cloned().collect();
                let categorical:Vec<_> = fitted.columns.iter().filter_map(|column| if let FittedColumn::Categorical { name,.. } = column {Some(name)} else {None}).cloned().collect();
                (vec![fitted.output_features.len()],numeric,categorical)
            },
            Some(super::auto_training_tables::TablePreprocessing::Tensor { input_shape,.. } | super::auto_training_tables::TablePreprocessing::Sample { input_shape,.. }) => (input_shape.clone(),vec![],vec![]),
            _ => (vec![],vec![],vec![]),
        };
        let plan=if let Some(super::auto_training_tables::TablePreprocessing::Engineered {projection,plan,..})=&preprocessing {
            if let Some(projection)=projection {shape=vec![projection.components.len()];}
            plan.as_deref()
        }else{None};
        flow_like_types::json::json!({"dataset_snapshot_id":dataset.snapshot_id,"input_shape":shape,"numeric_columns":numeric,"categorical_columns":categorical,"feature_plan":plan,"digest":dataset.digest})
    }).collect::<Vec<_>>();
    let parts = ConsultationParts {
        profile: experiment
            .request
            .context
            .get("training_profile")
            .and_then(|profile| flow_like_types::json::from_value(profile.clone()).ok()),
        columns: consultant_columns(&experiment.request.context),
        trials,
        datasets,
        sources: pretrained_sources,
    };
    // A report over budget is projected smaller instead of failing the consultation.
    for (level, limits) in REPORT_LIMITS.into_iter().enumerate() {
        let report = consultation_report(&experiment, &parts, limits, level > 0);
        if super::auto_training_tables::serialized_json_size(&report)? <= CONSULTATION_REPORT_BYTES
        {
            return Ok(report);
        }
    }
    Ok(minimal_consultation_report(&experiment))
}

#[cfg(feature = "training")]
const CONSULTATION_REPORT_BYTES: usize = 128 * 1024;

/// One projection of the consultation report; later entries keep less of each list.
#[cfg(feature = "training")]
#[derive(Clone, Copy)]
struct ReportLimits {
    examples: usize,
    trials: usize,
    columns: usize,
    sources: usize,
}

#[cfg(feature = "training")]
const REPORT_LIMITS: [ReportLimits; 5] = [
    ReportLimits {
        examples: 5,
        trials: usize::MAX,
        columns: usize::MAX,
        sources: 16,
    },
    ReportLimits {
        examples: 2,
        trials: 32,
        columns: 512,
        sources: 16,
    },
    ReportLimits {
        examples: 0,
        trials: 16,
        columns: 128,
        sources: 8,
    },
    ReportLimits {
        examples: 0,
        trials: 8,
        columns: 32,
        sources: 4,
    },
    ReportLimits {
        examples: 0,
        trials: 4,
        columns: 8,
        sources: 1,
    },
];

#[cfg(feature = "training")]
struct ConsultationParts {
    profile: Option<super::auto_training_tables::TableProfile>,
    columns: Option<ConsultantColumns>,
    trials: Vec<(String, Value)>,
    datasets: Vec<Value>,
    sources: Vec<Value>,
}

/// Columns the consultant may see. Unmapped source columns never leave the controller,
/// whatever the stored profile holds.
#[cfg(feature = "training")]
struct ConsultantColumns {
    visible: std::collections::HashSet<String>,
    targets: std::collections::HashSet<String>,
    metadata: Vec<String>,
}

#[cfg(feature = "training")]
fn consultant_columns(context: &Value) -> Option<ConsultantColumns> {
    use super::auto_training_tables::{FeatureSource, TargetSource};
    let request: TableDatasetRequest =
        flow_like_types::json::from_value(context.get("dataset_request")?.clone()).ok()?;
    let mapping = request.mapping;
    let targets: std::collections::HashSet<String> = match mapping.target {
        TargetSource::Column { column } | TargetSource::Annotation { column } => {
            std::iter::once(column).collect()
        }
        TargetSource::Columns { columns } => columns.into_iter().collect(),
        TargetSource::Sample => Default::default(),
    };
    let mut visible: std::collections::HashSet<String> = match mapping.features {
        FeatureSource::Tabular { options } => options
            .numeric_columns
            .into_iter()
            .chain(options.categorical_columns)
            .collect(),
        FeatureSource::Tensor { column } | FeatureSource::Sample { column } => {
            std::iter::once(column).collect()
        }
    };
    if let Some(plan) = context.get("feature_plan").filter(|plan| !plan.is_null()) {
        let plan: super::feature_engineering::FeaturePlan =
            flow_like_types::json::from_value(plan.clone()).ok()?;
        visible.extend(plan.pipeline.generated_columns());
    }
    visible.extend(targets.iter().cloned());
    let metadata = std::iter::once(mapping.row_id)
        .chain(
            [
                mapping.group_id,
                mapping.timestamp_ms,
                mapping.window_start_ms,
                mapping.window_end_ms,
            ]
            .into_iter()
            .flatten(),
        )
        .collect();
    Some(ConsultantColumns {
        visible,
        targets,
        metadata,
    })
}

/// Only this explicit projection is sent to the consultant. It has no test rows or test scores.
#[cfg(feature = "training")]
fn consultation_report(
    experiment: &Experiment,
    parts: &ConsultationParts,
    limits: ReportLimits,
    truncated: bool,
) -> Value {
    use flow_like_types::json::json;
    let context = &experiment.request.context;
    let profile = match (&parts.profile, &parts.columns) {
        (Some(profile), Some(columns)) => consultant_profile(profile, columns, limits),
        _ => Value::Null,
    };
    let mut allowed = context
        .pointer("/dataset_request/mapping/features")
        .cloned()
        .unwrap_or(Value::Null);
    if let Some(options) = allowed.get_mut("options") {
        truncate_columns(options, limits.columns);
    }
    let mut plan = context.get("feature_plan").cloned().unwrap_or(Value::Null);
    truncate_columns(&mut plan, limits.columns);
    let feature_sources = context
        .get("feature_sources")
        .and_then(Value::as_array)
        .map(|sources| {
            sources
                .iter()
                .map(|source| {
                    let mut source = json!({"alias":source.get("alias"),"allowed_columns":source.get("allowed_columns")});
                    truncate_list(&mut source, "allowed_columns", limits.columns);
                    source
                })
                .collect::<Vec<_>>()
        });
    let (trials, trials_omitted) = select_trials(
        &parts.trials,
        experiment.best_trial_id.as_deref(),
        limits.trials,
    );
    let datasets = parts
        .datasets
        .iter()
        .map(|dataset| {
            let mut dataset = dataset.clone();
            truncate_columns(&mut dataset, limits.columns);
            if let Some(plan) = dataset.get_mut("feature_plan") {
                truncate_columns(plan, limits.columns);
            }
            dataset
        })
        .collect::<Vec<_>>();
    let sources = &parts.sources[..parts.sources.len().min(limits.sources)];
    let mut report = json!({
        "spec": experiment.request.spec,
        "goals": experiment.request.goals,
        "budget": experiment.request.budget,
        "budget_used": experiment.usage,
        "training_profile": profile,
        "allowed_features": allowed,
        "metadata_columns": parts.columns.as_ref().map(|columns| &columns.metadata),
        "feature_plan": plan,
        "feature_sources": feature_sources,
        "feature_operations":["derive_columns","window_features","join_sources","train_only_imputation","train_only_scaling","train_only_pca"],
        "ordered_target": context.get("ordered_target"),
        "trials": trials,
        "registered_datasets": datasets,
        "pretrained_sources": sources,
    });
    if truncated {
        report["report_truncated"] = json!({
            "reason": "the full report exceeds its 128 KiB budget",
            "examples_per_column": limits.examples,
            "columns_per_list": limits.columns,
            "trials_omitted": trials_omitted,
            "pretrained_sources_omitted": parts.sources.len() - sources.len(),
        });
    }
    report
}

#[cfg(feature = "training")]
fn minimal_consultation_report(experiment: &Experiment) -> Value {
    flow_like_types::json::json!({
        "spec": experiment.request.spec,
        "goals": experiment.request.goals,
        "budget": experiment.request.budget,
        "budget_used": experiment.usage,
        "ordered_target": experiment.request.context.get("ordered_target"),
        "report_truncated": {
            "reason": "even the smallest projection exceeds the 128 KiB budget; profile, features, trials, datasets and pretrained sources were omitted",
        },
    })
}

#[cfg(feature = "training")]
fn consultant_profile(
    profile: &super::auto_training_tables::TableProfile,
    columns: &ConsultantColumns,
    limits: ReportLimits,
) -> Value {
    let mut kept = Vec::new();
    let (mut features, mut omitted) = (0usize, 0usize);
    for column in &profile.columns {
        if !columns.visible.contains(&column.name) {
            continue;
        }
        if !columns.targets.contains(&column.name) {
            if features >= limits.columns {
                omitted += 1;
                continue;
            }
            features += 1;
        }
        let mut column = column.clone();
        column.examples.truncate(limits.examples);
        kept.push(column);
    }
    let mut value = flow_like_types::json::json!({
        "partition": profile.partition,
        "rows": profile.rows,
        "columns": kept,
    });
    if omitted > 0 {
        value["columns_omitted"] = flow_like_types::json::json!(omitted);
    }
    value
}

/// At most `limit` trials: the best one and the newest others, in their original order.
#[cfg(feature = "training")]
fn select_trials(
    trials: &[(String, Value)],
    best: Option<&str>,
    limit: usize,
) -> (Vec<Value>, usize) {
    if trials.len() <= limit {
        return (trials.iter().map(|(_, trial)| trial.clone()).collect(), 0);
    }
    let best = best.and_then(|best| trials.iter().position(|(id, _)| id == best));
    let mut start = trials.len() - limit;
    if best.is_some_and(|best| best < start) {
        start += 1;
    }
    let kept = trials
        .iter()
        .enumerate()
        .filter(|(index, _)| *index >= start || Some(*index) == best)
        .map(|(_, (_, trial))| trial.clone())
        .collect::<Vec<_>>();
    let omitted = trials.len() - kept.len();
    (kept, omitted)
}

#[cfg(feature = "training")]
fn truncate_columns(value: &mut Value, limit: usize) {
    truncate_list(value, "numeric_columns", limit);
    truncate_list(value, "categorical_columns", limit);
}

/// Keeps the first `limit` entries of a list and records how many were left out.
#[cfg(feature = "training")]
fn truncate_list(value: &mut Value, key: &str, limit: usize) {
    let Some(object) = value.as_object_mut() else {
        return;
    };
    let Some(list) = object.get_mut(key).and_then(Value::as_array_mut) else {
        return;
    };
    if list.len() <= limit {
        return;
    }
    let omitted = list.len() - limit;
    list.truncate(limit);
    object.insert(
        format!("{key}_omitted"),
        flow_like_types::json::json!(omitted),
    );
}

#[cfg(feature = "training")]
pub fn append_burn_candidate(
    repo: &flow_like_ml_runtime::TrainingRepository,
    id: &str,
    config: flow_like_ml_burn::TrainingConfig,
    dataset_snapshot_id: Option<String>,
) -> Result<()> {
    append_burn_candidate_with_pretrained(repo, id, config, dataset_snapshot_id, None)
}

#[cfg(feature = "training")]
pub fn append_burn_candidate_with_pretrained(
    repo: &flow_like_ml_runtime::TrainingRepository,
    id: &str,
    config: flow_like_ml_burn::TrainingConfig,
    dataset_snapshot_id: Option<String>,
    pretrained: Option<flow_like_ml_runtime::PretrainedSourceRef>,
) -> Result<()> {
    let state = repo.experiment_result(id)?;
    let experiment = state.experiment;
    let dataset_id = dataset_snapshot_id
        .as_deref()
        .unwrap_or(&experiment.request.snapshot_id);
    let dataset = state
        .datasets
        .iter()
        .find(|dataset| dataset.snapshot_id == dataset_id)
        .ok_or_else(|| anyhow!("Candidate dataset is not registered in this experiment"))?;
    let training = repo.get_snapshot_limited(
        &dataset.training_snapshot_id,
        experiment.request.budget.maximum_dataset_bytes,
    )?;
    let row = training
        .train
        .first()
        .ok_or_else(|| anyhow!("Candidate dataset has no training rows"))?;
    let sample: flow_like_ml_core::Sample = flow_like_types::json::from_value(
        row.payload.get("sample").unwrap_or(&row.payload).clone(),
    )?;
    let mut spec: flow_like_ml_core::InspectionSpec =
        flow_like_types::json::from_value(experiment.request.spec.clone())?;
    spec.input_shape = sample.input.shape;
    // A consultant fine-tune keeps the source's architecture, so only its schedule is capped.
    if pretrained.is_some() {
        config.validate_fine_tune_planning_budget()?;
    } else {
        config.validate_planning_budget()?;
    }
    flow_like_ml_burn::validate_inspection_config(&spec, &config)?;
    let compute: ComputeConfig = flow_like_types::json::from_value(
        experiment
            .request
            .context
            .get("compute")
            .cloned()
            .unwrap_or_else(|| flow_like_types::json::json!({})),
    )?;
    let candidate = flow_like_ml_runtime::TrainingRequest {
        engine: "burn".into(),
        recipe: flow_like_types::json::json!({"config":config,"labels":spec.labels,"preprocessing":[],"pretrained":pretrained,"inspection_task":spec.task,"minimum_examples":spec.minimum_examples,"minimum_examples_per_class":spec.minimum_examples_per_class,"dataset_snapshot_id":dataset_id}),
        compute: flow_like_types::json::json!(compute),
    };
    validate_candidate_pretrained(
        repo,
        &experiment.request.stream.project_id,
        &candidate,
        &spec.labels,
        experiment.request.budget.worker_limits.memory_budget_bytes,
    )?;
    repo.append_experiment_candidates(id, vec![candidate], flow_like_ml_runtime::now_ms())?;
    Ok(())
}

#[cfg(feature = "training")]
pub async fn derive_feature_candidate(
    context: &mut ExecutionContext,
    id: &str,
    numeric_columns: Vec<String>,
    categorical_columns: Vec<String>,
    standardize: bool,
) -> Result<()> {
    use super::auto_training_tables::FeatureSource;
    let repo = experiment_repository(context, id)?;
    let experiment = repo.get_experiment(id)?;
    let request: TableDatasetRequest =
        flow_like_types::json::from_value(experiment.request.context["dataset_request"].clone())?;
    let FeatureSource::Tabular { options } = request.mapping.features else {
        return Err(anyhow!("Feature selection requires a tabular mapping"));
    };
    let original_allowed: std::collections::HashSet<_> = options
        .numeric_columns
        .iter()
        .chain(&options.categorical_columns)
        .cloned()
        .collect();
    let mut plan = experiment
        .request
        .context
        .get("feature_plan")
        .filter(|value| !value.is_null())
        .map(|value| {
            flow_like_types::json::from_value::<super::feature_engineering::FeaturePlan>(
                value.clone(),
            )
        })
        .transpose()?
        .unwrap_or(super::feature_engineering::FeaturePlan {
            pipeline: Default::default(),
            numeric_columns: options.numeric_columns,
            categorical_columns: options.categorical_columns,
            imputation: options.imputation,
            standardize: options.standardize,
            pca_components: None,
            limits: Default::default(),
        });
    let requested: Vec<_> = numeric_columns.iter().chain(&categorical_columns).collect();
    if requested.iter().any(|name| {
        !original_allowed.contains(*name) && !plan.pipeline.generated_columns().contains(*name)
    }) {
        let stored =
            repo.experiment_result(id)?
                .datasets
                .into_iter()
                .filter_map(|dataset| {
                    flow_like_types::json::from_value::<
                        super::auto_training_tables::TablePreprocessing,
                    >(dataset.preprocessing_manifest)
                    .ok()
                })
                .find_map(|preprocessing| {
                    if let super::auto_training_tables::TablePreprocessing::Engineered {
                        plan: Some(plan),
                        ..
                    } = preprocessing
                    {
                        let generated = plan.pipeline.generated_columns();
                        if requested.iter().all(|name| {
                            original_allowed.contains(*name) || generated.contains(*name)
                        }) {
                            return Some(*plan);
                        }
                    }
                    None
                })
                .ok_or_else(|| {
                    anyhow!("Selected engineered columns require a registered feature pipeline")
                })?;
        plan = stored;
    }
    plan.numeric_columns = numeric_columns;
    plan.categorical_columns = categorical_columns;
    plan.standardize = standardize;
    super::feature_engineering::derive_pipeline_candidate(context, id, plan).await
}

#[cfg(feature = "training")]
pub(crate) async fn register_prepared_candidate(
    context: &mut ExecutionContext,
    id: &str,
    prepared: super::auto_training_tables::PreparedTableDataset,
) -> Result<()> {
    use super::auto_training_tables::TableWritePolicy;
    use flow_like_types::json::json;
    let repo = experiment_repository(context, id)?;
    let experiment = repo.get_experiment(id)?;
    let setting = |key: &str| {
        experiment
            .request
            .context
            .get(key)
            .cloned()
            .unwrap_or_else(|| json!({}))
    };
    // A rejected proposal must leave no dataset or candidate behind, so every check runs
    // before the derived dataset and its candidate are registered together.
    let mut policy: TableWritePolicy = flow_like_types::json::from_value(setting("table_policy"))?;
    if policy.allowed_destination.is_some() {
        if !policy.create_tables {
            return Err(anyhow!(
                "Derived candidates require authorization to create a separate versioned table"
            ));
        }
        policy.allowed_destination = None;
        policy.expected_destination = None;
    }
    if experiment.status != ExperimentStatus::Running || experiment.selected_trial_id.is_some() {
        return Err(anyhow!(
            "Experiment {id} no longer accepts candidates; its status is {:?}",
            experiment.status
        ));
    }
    if experiment.request.candidates.len() >= experiment.request.budget.maximum_trials {
        return Err(anyhow!(
            "Experiment {id} already holds {} candidates, its maximum_trials budget",
            experiment.request.candidates.len()
        ));
    }
    let mut search: AutoSearchConfig = flow_like_types::json::from_value(setting("search"))?;
    search.maximum_candidates = 1;
    let compute: ComputeConfig = flow_like_types::json::from_value(setting("compute"))?;
    let derived_id = uuid::Uuid::new_v4().to_string();
    let mut candidates = flow_like_ml_runtime::auto_training::generate_candidates(
        &prepared.spec,
        &search,
        &generation_compute(&compute, &experiment.request.budget),
    )?;
    for candidate in &mut candidates {
        candidate.recipe["dataset_snapshot_id"] = json!(derived_id);
    }
    let (prepared, derived) = {
        let repo = repo.clone();
        let snapshot_id = experiment.request.snapshot_id.clone();
        let maximum_bytes = experiment.request.budget.maximum_dataset_bytes;
        let derived_id = derived_id.clone();
        tokio::task::spawn_blocking(move || -> Result<_> {
            let derived =
                derived_snapshot(&repo, &snapshot_id, maximum_bytes, &prepared, derived_id)?;
            Ok((prepared, derived))
        })
        .await??
    };
    let tables = super::auto_training_tables::materialize_prepared_tables(
        context,
        &prepared,
        &format!("{id}-{derived_id}"),
        &policy,
    )
    .await?;
    repo.register_experiment_candidates(
        id,
        derived,
        json!(prepared.preprocessing),
        candidates,
        flow_like_ml_runtime::now_ms(),
    )?;
    let references = tables.into_iter().map(|table| json!(table)).collect();
    repo.update_experiment_tables(id, references, vec![], flow_like_ml_runtime::now_ms())?;
    Ok(())
}

/// The experiment's split with every input replaced by the derived features. Membership,
/// identity and targets stay as recorded; the runtime verifies that on registration.
#[cfg(feature = "training")]
fn derived_snapshot(
    repo: &flow_like_ml_runtime::TrainingRepository,
    snapshot_id: &str,
    maximum_bytes: u64,
    prepared: &super::auto_training_tables::PreparedTableDataset,
    derived_id: String,
) -> Result<flow_like_ml_runtime::DatasetSnapshot> {
    let mut snapshot = repo.get_snapshot_limited(snapshot_id, maximum_bytes)?;
    snapshot.id = derived_id;
    snapshot.digest.clear();
    let inputs: std::collections::HashMap<_, _> = prepared
        .samples
        .iter()
        .map(|sample| (sample.id.as_str(), &sample.input))
        .collect();
    for sample in snapshot
        .train
        .iter_mut()
        .chain(&mut snapshot.validation)
        .chain(&mut snapshot.test)
    {
        let input = inputs.get(sample.id.as_str()).ok_or_else(|| {
            anyhow!(
                "The derived dataset has no row for sample {}; derived features must keep row membership",
                sample.id
            )
        })?;
        let payload = if sample.payload.get("sample").is_some() {
            &mut sample.payload["sample"]
        } else {
            &mut sample.payload
        };
        payload["input"] = flow_like_types::json::json!(input);
    }
    Ok(snapshot)
}

#[cfg(feature = "training")]
pub async fn run_next_auto_training(
    context: &ExecutionContext,
    id: &str,
) -> Result<AutoTrainingResult> {
    let repo = experiment_repository(context, id)?;
    let experiment = repo.get_experiment(id)?;
    let controller = controller(repo.clone(), &experiment.request.budget)?;
    let owned_id = id.to_string();
    let work = tokio::task::spawn_blocking(move || controller.run_next(&owned_id));
    await_experiment_work(
        context.get_cancellation_token(),
        &repo,
        id,
        work,
        "Auto training was cancelled; its experiment ID can be resumed",
    )
    .await?;
    Ok(repo.experiment_result(id)?.into())
}

/// Wait for durable work on an experiment. On a cancelled run the executor drops this node's
/// future without polling it again, so a drop guard cancels the experiment too; the worker
/// then stops at its next cancellation check instead of training on detached.
#[cfg(feature = "training")]
async fn await_experiment_work(
    token: Option<flow_like_types::tokio_util::sync::CancellationToken>,
    repo: &flow_like_ml_runtime::TrainingRepository,
    id: &str,
    mut work: tokio::task::JoinHandle<flow_like_ml_runtime::Result<Experiment>>,
    cancelled: &str,
) -> Result<()> {
    let guard = super::cancellation::CancelOnDrop::new({
        let (repo, id) = (repo.clone(), id.to_string());
        move || {
            let _ = repo.cancel_experiment(&id, flow_like_ml_runtime::now_ms());
        }
    });
    let finished = match token {
        Some(token) => tokio::select! {
            finished = &mut work => finished,
            _ = token.cancelled() => {
                guard.disarm();
                repo.cancel_experiment(id, flow_like_ml_runtime::now_ms())?;
                return Err(anyhow!("{cancelled}"));
            }
        },
        None => (&mut work).await,
    };
    guard.disarm();
    finished??;
    Ok(())
}

#[cfg(feature = "training")]
pub fn resume_saved_experiment(
    repo: &flow_like_ml_runtime::TrainingRepository,
    id: &str,
) -> Result<()> {
    let experiment = repo.get_experiment(id)?;
    for trial in repo.list_experiment_trials(id)? {
        let job = repo.get_job(&trial.job_id)?;
        if matches!(
            job.status,
            flow_like_ml_runtime::JobStatus::Running
                | flow_like_ml_runtime::JobStatus::CancelRequested
        ) && job
            .lease_expires_at_ms
            .is_some_and(|expiry| expiry < flow_like_ml_runtime::now_ms())
        {
            flow_like_ml_runtime::worker::reconcile_expired_job(repo, &job.id)?;
        }
    }
    if matches!(
        experiment.status,
        ExperimentStatus::Cancelled | ExperimentStatus::CancelRequested
    ) {
        if experiment.selected_trial_id.is_some() {
            repo.resume_frozen_experiment_audit(id, flow_like_ml_runtime::now_ms())?;
        } else {
            repo.resume_experiment(id, flow_like_ml_runtime::now_ms())?;
        }
    }
    Ok(())
}

#[cfg(feature = "training")]
pub async fn finalize_auto_training(
    context: &ExecutionContext,
    id: &str,
) -> Result<AutoTrainingResult> {
    let repo = experiment_repository(context, id)?;
    let experiment = repo.get_experiment(id)?;
    let controller = controller(repo.clone(), &experiment.request.budget)?;
    let owned_id = id.to_string();
    let work = tokio::task::spawn_blocking(move || controller.finalize(&owned_id));
    await_experiment_work(
        context.get_cancellation_token(),
        &repo,
        id,
        work,
        "Final audit was cancelled; resume the experiment to continue auditing the same frozen model",
    )
    .await?;
    Ok(repo.experiment_result(id)?.into())
}

#[cfg(feature = "training")]
pub async fn materialize_final_predictions(
    context: &mut ExecutionContext,
    id: &str,
) -> Result<AutoTrainingResult> {
    let repo = experiment_repository(context, id)?;
    let result: AutoTrainingResult = repo.experiment_result(id)?.into();
    if result.predictions_table.is_some() || result.final_test_metrics.is_empty() {
        return Ok(result);
    }
    match write_predictions_table(context, &repo, id, &result).await {
        Ok(false) => Ok(result),
        Ok(true) => Ok(repo.experiment_result(id)?.into()),
        // The table copies durable predictions; a source connection that is unavailable in this
        // run must not hide the experiment result from Get, Cancel, Resume or Finalize.
        Err(error) => {
            let error = bounded_text(&format!("Final predictions table was not written: {error}"));
            tracing::warn!(experiment_id = %id, %error, "Final predictions table was not written");
            let recorded = result.other_artifacts.iter().any(|event| {
                event.get("kind").and_then(Value::as_str) == Some("predictions_table_failed")
                    && event.pointer("/body/error").and_then(Value::as_str) == Some(error.as_str())
            });
            if recorded {
                return Ok(result);
            }
            note_experiment(
                &repo,
                id,
                "predictions_table_failed",
                flow_like_types::json::json!({ "error": error }),
            );
            Ok(repo
                .experiment_result(id)
                .map(AutoTrainingResult::from)
                .unwrap_or(result))
        }
    }
}

/// Write the final test predictions as a versioned table. Returns whether a table was written.
#[cfg(feature = "training")]
async fn write_predictions_table(
    context: &mut ExecutionContext,
    repo: &flow_like_ml_runtime::TrainingRepository,
    id: &str,
    result: &AutoTrainingResult,
) -> Result<bool> {
    use super::auto_training_tables::{PinnedTableSource, TableWritePolicy};
    let experiment = repo.get_experiment(id)?;
    let mut policy: TableWritePolicy = flow_like_types::json::from_value(
        experiment
            .request
            .context
            .get("table_policy")
            .cloned()
            .unwrap_or_else(|| flow_like_types::json::json!({})),
    )?;
    if !policy.create_tables {
        return Ok(false);
    }
    policy.allowed_destination = None;
    policy.expected_destination = None;
    let Some(artifact_id) = &result.model_artifact_id else {
        return Ok(false);
    };
    let rows = repo.predictions(artifact_id)?.into_iter().map(|record| Ok(flow_like_types::json::json!({"sample_id":record.sample_id,"record_json":flow_like_types::json::to_string(&record)?}))).collect::<Result<Vec<_>>>()?;
    if rows.is_empty() {
        return Ok(false);
    }
    let mut source: PinnedTableSource = flow_like_types::json::from_value(
        experiment
            .request
            .context
            .get("pinned_source")
            .cloned()
            .ok_or_else(|| anyhow!("Experiment has no source table locator"))?,
    )?;
    source.database = super::auto_training_tables::reopen_pinned_source(context, &source).await?;
    let digest = flow_like_ml_core::content_digest(&flow_like_types::json::to_vec(
        &result.preprocessing_manifest,
    )?);
    let table = super::auto_training_tables::write_experiment_rows(
        context,
        &source,
        rows,
        id,
        "predictions",
        &digest,
        &policy,
    )
    .await?;
    repo.update_experiment_tables(
        id,
        vec![flow_like_types::json::json!(table)],
        vec![],
        flow_like_ml_runtime::now_ms(),
    )?;
    Ok(true)
}

#[derive(Clone, Copy)]
enum LifecycleAction {
    Status,
    Step,
    Cancel,
    Resume,
    Finalize,
}

#[cfg(feature = "training")]
async fn lifecycle(
    context: &mut ExecutionContext,
    id: &str,
    action: LifecycleAction,
) -> Result<AutoTrainingResult> {
    if matches!(action, LifecycleAction::Step) {
        return run_next_auto_training(context, id).await;
    }
    let repo = experiment_repository(context, id)?;
    match action {
        LifecycleAction::Status => {}
        LifecycleAction::Step => unreachable!(),
        LifecycleAction::Cancel => {
            repo.cancel_experiment(id, flow_like_ml_runtime::now_ms())?;
        }
        LifecycleAction::Resume => {
            resume_saved_experiment(&repo, id)?;
            enqueue_experiment(repo.clone(), id.to_string())?;
        }
        LifecycleAction::Finalize => {
            finalize_auto_training(context, id).await?;
        }
    }
    materialize_final_predictions(context, id).await
}

macro_rules! lifecycle_node {
    ($node:ident, $id:literal, $name:literal, $description:literal, $function:literal, $action:ident) => {
        #[flow_like_types::async_trait]
        impl NodeLogic for $node {
            fn get_node(&self) -> Node {
                let mut node = super::operation_node::<ExperimentRequest, AutoTrainingResult>(
                    $id,
                    $name,
                    $description,
                    $function,
                    "AI/ML/Auto Training",
                );
                node.set_long_running(matches!(
                    LifecycleAction::$action,
                    LifecycleAction::Step | LifecycleAction::Finalize
                ));
                node
            }
            async fn run(&self, context: &mut ExecutionContext) -> Result<()> {
                #[cfg(feature = "training")]
                {
                    context.deactivate_exec_pin("exec_out").await?;
                    let request: ExperimentRequest = context.evaluate_pin("request").await?;
                    let result =
                        lifecycle(context, &request.experiment_id, LifecycleAction::$action)
                            .await?;
                    context
                        .set_pin_value("result", flow_like_types::json::json!(result))
                        .await?;
                    context.activate_exec_pin("exec_out").await?;
                    Ok(())
                }
                #[cfg(not(feature = "training"))]
                {
                    let _ = context;
                    Err(anyhow!("Auto training requires a local training build"))
                }
            }
        }
    };
}

#[crate::register_node]
#[derive(Default)]
pub struct GetAutoTrainingNode;
#[crate::register_node]
#[derive(Default)]
pub struct StepAutoTrainingNode;
#[crate::register_node]
#[derive(Default)]
pub struct CancelAutoTrainingNode;
#[crate::register_node]
#[derive(Default)]
pub struct ResumeAutoTrainingNode;
#[crate::register_node]
#[derive(Default)]
pub struct FinalizeAutoTrainingNode;

lifecycle_node!(
    GetAutoTrainingNode,
    "ml_get_auto_training",
    "Get Auto Training Result",
    "Read the durable experiment, leaderboard, table versions and best model bundle",
    "getAutoTraining",
    Status
);
lifecycle_node!(
    StepAutoTrainingNode,
    "ml_step_auto_training",
    "Step Auto Training",
    "Run one candidate and return its measured validation results",
    "stepAutoTraining",
    Step
);
lifecycle_node!(
    CancelAutoTrainingNode,
    "ml_cancel_auto_training",
    "Cancel Auto Training",
    "Cancel active trials and preserve the experiment and checkpoints",
    "cancelAutoTraining",
    Cancel
);
lifecycle_node!(
    ResumeAutoTrainingNode,
    "ml_resume_auto_training",
    "Resume Auto Training",
    "Resume a saved experiment in the background within its remaining budget",
    "resumeAutoTraining",
    Resume
);
lifecycle_node!(
    FinalizeAutoTrainingNode,
    "ml_finalize_auto_training",
    "Finalize Auto Training",
    "Freeze the selected candidate and audit it against the untouched test partition",
    "finalizeAutoTraining",
    Finalize
);

#[crate::register_node]
#[derive(Default)]
pub struct PredictAutoModelNode;

#[flow_like_types::async_trait]
impl NodeLogic for PredictAutoModelNode {
    fn get_node(&self) -> Node {
        let mut node = super::operation_node::<PredictAutoModelRequest, AutoModelPredictions>(
            "ml_predict_auto_model",
            "Predict Auto Model",
            "Apply the winning model's saved preprocessing to raw rows and load its durable weights; anomaly models also return the audited calibrated probability and decision",
            "predictAutoModel",
            "AI/ML/Auto Training",
        );
        node.set_version(2);
        node
    }
    async fn run(&self, context: &mut ExecutionContext) -> Result<()> {
        #[cfg(feature = "training")]
        {
            context.deactivate_exec_pin("exec_out").await?;
            let request: PredictAutoModelRequest = context.evaluate_pin("request").await?;
            let result = predict_auto_model(context, request).await?;
            context
                .set_pin_value("result", flow_like_types::json::json!(result))
                .await?;
            context.activate_exec_pin("exec_out").await?;
            Ok(())
        }
        #[cfg(not(feature = "training"))]
        {
            let _ = context;
            Err(anyhow!(
                "Auto model inference requires a local training build"
            ))
        }
    }
}

#[crate::register_node]
#[derive(Default)]
pub struct ExportAutoModelNode;

#[flow_like_types::async_trait]
impl NodeLogic for ExportAutoModelNode {
    fn get_node(&self) -> Node {
        super::operation_node::<ExportAutoModelRequest, ExportAutoModelResult>(
            "ml_export_auto_model",
            "Export Auto Model",
            "Write native model weights and their task, preprocessing and evaluation manifest",
            "exportAutoModel",
            "AI/ML/Auto Training",
        )
    }
    async fn run(&self, context: &mut ExecutionContext) -> Result<()> {
        #[cfg(feature = "training")]
        {
            context.deactivate_exec_pin("exec_out").await?;
            let request: ExportAutoModelRequest = context.evaluate_pin("request").await?;
            let result = export_auto_model(context, request).await?;
            context
                .set_pin_value("result", flow_like_types::json::json!(result))
                .await?;
            context.activate_exec_pin("exec_out").await?;
            Ok(())
        }
        #[cfg(not(feature = "training"))]
        {
            let _ = context;
            Err(anyhow!("Auto model export requires a local training build"))
        }
    }
}

/// Predict raw rows with the experiment's selected model and the preprocessing of the dataset
/// it was trained on. Anomaly scores also get the decision rule the final audit measured.
#[cfg(feature = "training")]
pub async fn predict_auto_model(
    context: &ExecutionContext,
    request: PredictAutoModelRequest,
) -> Result<AutoModelPredictions> {
    if request.rows.len() > 100_000 {
        return Err(anyhow!(
            "Prediction batch of {} rows exceeds 100,000 rows",
            request.rows.len()
        ));
    }
    let repo = experiment_repository(context, &request.experiment_id)?;
    let state = repo.experiment_result(&request.experiment_id)?;
    let threshold = match state.experiment.request.goals.task {
        flow_like_ml_runtime::EvaluationTask::Anomaly { threshold } => Some(threshold),
        _ => None,
    };
    let result: AutoTrainingResult = state.into();
    let artifact = result.model_bundle_ref.clone().ok_or_else(|| {
        anyhow!(
            "Experiment {} has no successful model yet",
            request.experiment_id
        )
    })?;
    // The final audit calibrates with the scale measured for the selected trial.
    let calibration = threshold.map(|threshold| {
        let scale = result
            .validation_metrics
            .get("anomaly_scale")
            .copied()
            .unwrap_or(1.0);
        (scale, threshold)
    });
    let predictor = cached_auto_model(context, &repo, &artifact, &request.compute).await?;
    let preprocessing = result.preprocessing_manifest;
    tokio::task::spawn_blocking(move || -> Result<AutoModelPredictions> {
        let preprocessing: super::auto_training_tables::TablePreprocessing =
            flow_like_types::json::from_value(preprocessing)?;
        let extractor = requested_extractor(
            &preprocessing,
            &request.rows,
            request.feature_extractor.as_ref(),
        )?;
        let (inputs, feature_state) = super::auto_training_tables::transform_table_rows_with_state(
            &preprocessing,
            &request.rows,
            32 * 1024 * 1024,
            request.feature_state,
            request.stateful_features,
        )?;
        let predictions = inputs
            .into_iter()
            .map(|mut input| {
                input.shape.insert(0, 1);
                predictor.predict_checked(&input, extractor.as_ref())
            })
            .collect::<Result<Vec<_>>>()?;
        let anomaly = calibration
            .map(|(scale, threshold)| anomaly_decisions(&predictions, scale, threshold))
            .transpose()?;
        Ok(AutoModelPredictions {
            experiment_id: request.experiment_id,
            model_artifact_id: artifact.id,
            predictions,
            feature_state,
            anomaly,
        })
    })
    .await?
}

/// The final audit's anomaly rule: probability = score / (score + scale), an anomaly at or
/// above the threshold.
#[cfg(feature = "training")]
fn anomaly_decisions(
    predictions: &[Value],
    scale: f64,
    threshold: f64,
) -> Result<AnomalyDecisions> {
    if !scale.is_finite() || scale <= 0.0 {
        return Err(anyhow!(
            "Anomaly scale {scale} of the selected trial is not a positive finite number"
        ));
    }
    let rows = predictions
        .iter()
        .enumerate()
        .map(|(row, raw)| {
            let scores = ["scores", "reconstruction_error"]
                .iter()
                .find_map(|key| raw.get(*key).and_then(Value::as_array))
                .ok_or_else(|| {
                    anyhow!(
                        "Row {row}: the anomaly model returned neither scores nor reconstruction errors"
                    )
                })?;
            let score = scores
                .first()
                .and_then(Value::as_f64)
                .filter(|score| score.is_finite() && *score >= 0.0)
                .ok_or_else(|| {
                    anyhow!("Row {row}: the anomaly score is not a finite non-negative number")
                })?;
            let probability = score / (score + scale);
            Ok(AnomalyDecision {
                score,
                probability,
                is_anomaly: probability >= threshold,
            })
        })
        .collect::<Result<Vec<_>>>()?;
    Ok(AnomalyDecisions {
        scale,
        threshold,
        rows,
    })
}

/// Embeddings name their extractor on the request or as `feature_extractor` inside each row's
/// tensor value; every named extractor must be the same one.
#[cfg(feature = "training")]
fn requested_extractor(
    preprocessing: &super::auto_training_tables::TablePreprocessing,
    rows: &[Value],
    explicit: Option<&super::lifecycle::FeatureExtractorRef>,
) -> Result<Option<flow_like_ml_runtime::engines::FeatureExtractor>> {
    let mut named = explicit
        .map(super::lifecycle::FeatureExtractorRef::to_runtime)
        .transpose()?
        .into_iter()
        .collect::<Vec<_>>();
    if let super::auto_training_tables::TablePreprocessing::Tensor { column, .. } =
        super::auto_training_tables::base_preprocessing(preprocessing)
    {
        for row in rows {
            // Tensor cells hold the tensor object or its JSON text, as table decoding accepts.
            let carried = match row.get(column) {
                Some(Value::String(text)) => flow_like_types::json::from_str::<Value>(text)
                    .ok()
                    .and_then(|tensor| tensor.get("feature_extractor").cloned()),
                Some(tensor) => tensor.get("feature_extractor").cloned(),
                None => None,
            };
            if let Some(value) = carried.filter(|value| !value.is_null()) {
                named.push(super::lifecycle::runtime_extractor(&value)?);
            }
        }
    }
    let mut named = named.into_iter();
    let Some(first) = named.next() else {
        return Ok(None);
    };
    if let Some(other) = named.find(|extractor| extractor != &first) {
        return Err(anyhow!(
            "Embeddings come from different feature extractors ({} and {}); predict each extractor's rows separately",
            first.source_id,
            other.source_id
        ));
    }
    Ok(Some(first))
}

/// PatchCore and PaDiM memory banks only score embeddings from the feature space they were
/// fitted in, so the extractor named by the training rows is recorded with their recipes.
#[cfg(feature = "training")]
fn record_feature_extractor(
    candidates: &mut [flow_like_ml_runtime::TrainingRequest],
    extractor: Option<&flow_like_ml_runtime::engines::FeatureExtractor>,
) -> Result<()> {
    let Some(extractor) = extractor else {
        return Ok(());
    };
    let extractor = flow_like_types::json::to_value(extractor)?;
    for candidate in candidates
        .iter_mut()
        .filter(|candidate| matches!(candidate.engine.as_str(), "patchcore" | "padim"))
    {
        if let Some(recipe) = candidate.recipe.as_object_mut() {
            recipe.insert("feature_extractor".into(), extractor.clone());
        }
    }
    Ok(())
}

#[cfg(feature = "training")]
type CachedAutoModel = std::sync::Arc<super::lifecycle::CatalogLoadedPredictor>;

#[cfg(feature = "training")]
#[derive(Clone, Default)]
struct AutoModelCache {
    models: std::sync::Arc<tokio::sync::Mutex<Vec<(String, CachedAutoModel, u64)>>>,
}

#[cfg(feature = "training")]
impl flow_like_types::Cacheable for AutoModelCache {
    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
    fn as_any_mut(&mut self) -> &mut dyn std::any::Any {
        self
    }
}

/// Loaded models are reused within one workflow run, as Student Model Inference does, so a
/// loop over rows does not unpack and rebuild the model on every call.
#[cfg(feature = "training")]
async fn cached_auto_model(
    context: &ExecutionContext,
    repo: &flow_like_ml_runtime::TrainingRepository,
    artifact: &flow_like_ml_runtime::ModelArtifact,
    compute: &ComputeConfig,
) -> Result<CachedAutoModel> {
    compute.validate()?;
    let key = flow_like_ml_core::content_digest(&flow_like_types::json::to_vec(&(
        repo.path(),
        &artifact.id,
        &artifact.blob.sha256,
        compute,
    ))?);
    let cache = {
        let mut run_cache = context.cache.write().await;
        let value = run_cache
            .entry("auto-model-predictors-v1".into())
            .or_insert_with(|| std::sync::Arc::new(AutoModelCache::default()));
        value
            .as_any()
            .downcast_ref::<AutoModelCache>()
            .ok_or_else(|| anyhow!("Auto model predictor cache has an unexpected type"))?
            .clone()
    };
    let mut models = cache.models.lock().await;
    if let Some(index) = models.iter().position(|(cached, _, _)| cached == &key) {
        let entry = models.remove(index);
        let predictor = entry.1.clone();
        models.push(entry);
        return Ok(predictor);
    }
    let budget = compute.memory_limit_bytes;
    let reserved = artifact
        .blob
        .bytes
        .checked_mul(4)
        .ok_or_else(|| anyhow!("Memory estimate of model {} overflows", artifact.id))?;
    if reserved >= budget {
        return Err(anyhow!(
            "Model {} needs about {reserved} bytes to load, above the inference memory budget of {budget} bytes",
            artifact.id
        ));
    }
    while !models.is_empty()
        && (models.len() >= 4
            || models
                .iter()
                .fold(reserved, |total, entry| total.saturating_add(entry.2))
                > budget)
    {
        models.remove(0);
    }
    let predictor = {
        let (repo, artifact_id, compute) = (repo.clone(), artifact.id.clone(), compute.clone());
        std::sync::Arc::new(
            tokio::task::spawn_blocking(move || {
                super::lifecycle::CatalogLoadedPredictor::load(&repo, &artifact_id, &compute)
            })
            .await??,
        )
    };
    models.push((key, predictor.clone(), reserved));
    Ok(predictor)
}

#[cfg(feature = "training")]
pub async fn export_auto_model(
    context: &mut ExecutionContext,
    request: ExportAutoModelRequest,
) -> Result<ExportAutoModelResult> {
    if flow_like_types::json::to_value(&request.weights)?
        == flow_like_types::json::to_value(&request.manifest)?
    {
        return Err(anyhow!(
            "Weights and manifest must have different destinations"
        ));
    }
    let repo = experiment_repository(context, &request.experiment_id)?;
    let result: AutoTrainingResult = repo.experiment_result(&request.experiment_id)?.into();
    let artifact = result.model_bundle_ref.clone().ok_or_else(|| {
        anyhow!(
            "Experiment {} has no model to export",
            request.experiment_id
        )
    })?;
    let blob = artifact.blob.clone();
    let maximum = repo
        .get_experiment(&request.experiment_id)?
        .request
        .budget
        .worker_limits
        .maximum_artifact_bytes;
    let bytes =
        tokio::task::spawn_blocking(move || repo.read_blob_limited(&blob, maximum)).await??;
    let manifest = flow_like_types::json::to_vec_pretty(&flow_like_types::json::json!({
        "format_version":1,"artifact":artifact,"preprocessing":result.preprocessing_manifest,
        "source_table_versions":result.source_table_versions,"validation_metrics":result.validation_metrics,
        "final_test_metrics":result.final_test_metrics,"goal_met":result.goal_met
    }))?;
    request.weights.put(context, bytes, false).await?;
    request.manifest.put(context, manifest, false).await?;
    Ok(ExportAutoModelResult {
        model_artifact_id: artifact.id,
        weights: request.weights,
        manifest: request.manifest,
        sha256: artifact.blob.sha256,
        bytes: artifact.blob.bytes,
    })
}

#[cfg(feature = "training")]
pub fn enqueue_experiment(
    repo: flow_like_ml_runtime::TrainingRepository,
    id: String,
) -> Result<()> {
    background::enqueue(repo, id, 0)
}

/// Background experiments run one at a time on a dedicated thread, which is replaced when it
/// has stopped. Wall time is charged only while the controller works on an experiment, so a
/// queued experiment keeps its budget.
#[cfg(feature = "training")]
mod background {
    use super::{bounded_text, controller, note_experiment};
    use flow_like_ml_runtime::{ExperimentStatus, ExperimentTrialStatus, TrainingRepository};
    use flow_like_types::{Result, anyhow, json::json};
    use std::{
        collections::HashSet,
        sync::{
            LazyLock, Mutex, MutexGuard, PoisonError,
            mpsc::{Receiver, SyncSender, TrySendError, sync_channel},
        },
        time::Duration,
    };

    const CAPACITY: usize = 32;
    /// Waiting for a busy device or another controller is retried this many times in total.
    const ATTEMPTS: u32 = 5;

    struct Work {
        repo: TrainingRepository,
        id: String,
        key: String,
        attempt: u32,
    }

    static QUEUED: LazyLock<Mutex<HashSet<String>>> = LazyLock::new(Default::default);
    static WORKER: LazyLock<Mutex<Option<SyncSender<Work>>>> = LazyLock::new(Default::default);

    fn queued() -> MutexGuard<'static, HashSet<String>> {
        QUEUED.lock().unwrap_or_else(PoisonError::into_inner)
    }

    pub(super) fn ensure_capacity() -> Result<()> {
        let queued = queued().len();
        if queued >= CAPACITY {
            return Err(anyhow!(
                "The background training queue already holds {queued} experiments; wait for one to finish or start without background training"
            ));
        }
        Ok(())
    }

    pub(super) fn enqueue(repo: TrainingRepository, id: String, attempt: u32) -> Result<()> {
        let key =
            flow_like_ml_core::content_digest(&flow_like_types::json::to_vec(&(repo.path(), &id))?);
        let ahead = {
            let mut queued = queued();
            if !queued.insert(key.clone()) {
                return Ok(());
            }
            let work = Work {
                repo: repo.clone(),
                id: id.clone(),
                key: key.clone(),
                attempt,
            };
            let mut worker = WORKER.lock().unwrap_or_else(PoisonError::into_inner);
            let sent = send_or_respawn(&mut *worker, work, || {
                // Work buffered for a stopped thread is gone; its keys must not block Resume.
                queued.retain(|queued_key| queued_key == &key);
                spawn_worker()
            });
            drop(worker);
            if let Err(error) = sent {
                queued.remove(&key);
                return Err(anyhow!(
                    "Experiment {id} is saved but was not queued: {error}; Resume Auto Training can queue it again"
                ));
            }
            queued.len() - 1
        };
        if ahead > 0 {
            note_experiment(
                &repo,
                &id,
                "queued_behind_other_experiments",
                json!({ "experiments_ahead": ahead }),
            );
        }
        Ok(())
    }

    /// Send to the current worker, or to a new one when the previous thread has stopped.
    fn send_or_respawn<W>(
        slot: &mut Option<SyncSender<W>>,
        work: W,
        spawn: impl FnOnce() -> Result<SyncSender<W>>,
    ) -> Result<()> {
        let work = match slot.as_ref() {
            Some(sender) => match sender.try_send(work) {
                Ok(()) => return Ok(()),
                Err(TrySendError::Full(_)) => {
                    return Err(anyhow!(
                        "the background training queue is full ({CAPACITY} experiments)"
                    ));
                }
                Err(TrySendError::Disconnected(work)) => work,
            },
            None => work,
        };
        let sender = spawn()?;
        sender.try_send(work).map_err(|error| {
            anyhow!("the new background training worker did not accept work: {error}")
        })?;
        *slot = Some(sender);
        Ok(())
    }

    fn spawn_worker() -> Result<SyncSender<Work>> {
        let (sender, receiver) = sync_channel::<Work>(CAPACITY);
        std::thread::Builder::new()
            .name("auto-training".into())
            .spawn(move || run_worker(receiver))
            .map_err(|error| anyhow!("Cannot start the background training worker: {error}"))?;
        Ok(sender)
    }

    fn run_worker(receiver: Receiver<Work>) {
        while let Ok(work) = receiver.recv() {
            let _queued = QueuedKey(work.key.clone());
            let driven = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                drive(&work.repo, &work.id)
            }))
            .unwrap_or_else(|payload| {
                Driven::Failed(format!(
                    "the background training worker panicked: {}",
                    panic_text(&*payload)
                ))
            });
            // Recording the outcome must not stop the worker either.
            let _ = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| settle(work, driven)));
        }
    }

    /// Releases the queue key however the work item ends, so Resume can queue it again.
    struct QueuedKey(String);
    impl Drop for QueuedKey {
        fn drop(&mut self) {
            queued().remove(&self.0);
        }
    }

    enum Driven {
        Settled,
        Waiting(String),
        Failed(String),
    }

    fn drive(repo: &TrainingRepository, id: &str) -> Driven {
        let experiment = match repo.get_experiment(id) {
            Ok(experiment) => experiment,
            Err(error) => return Driven::Failed(error.to_string()),
        };
        let controller = match controller(repo.clone(), &experiment.request.budget) {
            Ok(controller) => controller,
            Err(error) => return Driven::Failed(error.to_string()),
        };
        match controller.run(id) {
            Ok(experiment)
                if matches!(
                    experiment.status,
                    ExperimentStatus::Running | ExperimentStatus::Evaluating
                ) =>
            {
                Driven::Waiting(waiting_reason(repo, id))
            }
            Ok(_) | Err(flow_like_ml_runtime::Error::Cancelled) => Driven::Settled,
            // Another holder, a lost lease or a busy ledger clears up without intervention.
            Err(
                error @ (flow_like_ml_runtime::Error::Conflict(_)
                | flow_like_ml_runtime::Error::LeaseLost
                | flow_like_ml_runtime::Error::Sql(_)),
            ) => Driven::Waiting(error.to_string()),
            Err(error) => Driven::Failed(error.to_string()),
        }
    }

    /// Why an unsettled experiment stopped: usually a scheduled trial whose job waits for a
    /// device or is owned by another worker.
    fn waiting_reason(repo: &TrainingRepository, id: &str) -> String {
        let scheduled = repo
            .list_experiment_trials(id)
            .unwrap_or_default()
            .into_iter()
            .find(|trial| trial.status == ExperimentTrialStatus::Scheduled);
        let Some(trial) = scheduled else {
            return "another controller holds this experiment".into();
        };
        match repo.get_job(&trial.job_id) {
            Ok(job) => match job.error {
                Some(error) => format!("trial {} is waiting: {error}", trial.id),
                None => format!("trial {} has a {:?} job", trial.id, job.status),
            },
            Err(error) => format!("trial {} job could not be read: {error}", trial.id),
        }
    }

    fn settle(work: Work, driven: Driven) {
        let Work {
            repo, id, attempt, ..
        } = work;
        match driven {
            Driven::Settled => {}
            Driven::Waiting(reason) if attempt + 1 < ATTEMPTS => {
                let delay = retry_delay(attempt);
                note_experiment(
                    &repo,
                    &id,
                    "background_worker_waiting",
                    json!({
                        "attempt": attempt + 1,
                        "retry_in_ms": delay.as_millis() as u64,
                        "reason": bounded_text(&reason),
                    }),
                );
                retry_later(repo, id, attempt + 1, delay);
            }
            // Another controller may still drive it, so the experiment is left as it is.
            Driven::Waiting(reason) => note_experiment(
                &repo,
                &id,
                "background_worker_stopped",
                json!({
                    "attempts": attempt + 1,
                    "reason": bounded_text(&reason),
                    "resume": "Resume Auto Training queues the experiment again",
                }),
            ),
            Driven::Failed(error) => {
                note_experiment(
                    &repo,
                    &id,
                    "background_worker_failed",
                    json!({ "error": bounded_text(&error) }),
                );
                // A cancelled experiment shows that nothing drives it and stays resumable.
                if let Err(cancel) = repo.cancel_experiment(&id, flow_like_ml_runtime::now_ms()) {
                    tracing::warn!(experiment_id = %id, error = %cancel, "Could not stop an experiment its background worker failed on");
                }
            }
        }
    }

    fn retry_delay(attempt: u32) -> Duration {
        Duration::from_secs((5u64 << attempt.min(4)).min(60))
    }

    fn retry_later(repo: TrainingRepository, id: String, attempt: u32, delay: Duration) {
        let spawned = std::thread::Builder::new()
            .name("auto-training-retry".into())
            .spawn({
                let (repo, id) = (repo.clone(), id.clone());
                move || {
                    std::thread::sleep(delay);
                    if let Err(error) = enqueue(repo.clone(), id.clone(), attempt) {
                        note_experiment(
                            &repo,
                            &id,
                            "background_worker_stopped",
                            json!({ "error": bounded_text(&error.to_string()) }),
                        );
                    }
                }
            });
        if let Err(error) = spawned {
            note_experiment(
                &repo,
                &id,
                "background_worker_stopped",
                json!({ "error": bounded_text(&format!("cannot schedule a retry: {error}")) }),
            );
        }
    }

    fn panic_text(payload: &(dyn std::any::Any + Send)) -> String {
        payload
            .downcast_ref::<&str>()
            .map(|text| (*text).to_owned())
            .or_else(|| payload.downcast_ref::<String>().cloned())
            .unwrap_or_else(|| "unknown panic payload".into())
    }

    #[cfg(test)]
    mod tests {
        use super::*;

        #[test]
        fn a_stopped_worker_is_replaced_and_receives_the_work() {
            let (stopped, receiver) = sync_channel::<u32>(1);
            drop(receiver);
            let mut slot = Some(stopped);
            let (live, inbox) = sync_channel::<u32>(2);
            send_or_respawn(&mut slot, 7, || Ok(live)).unwrap();
            assert_eq!(inbox.try_recv().unwrap(), 7);
            send_or_respawn(&mut slot, 8, || panic!("a live worker is never replaced")).unwrap();
            assert_eq!(inbox.try_recv().unwrap(), 8);
        }

        #[test]
        fn a_full_queue_is_reported_without_replacing_the_worker() {
            let (sender, inbox) = sync_channel::<u32>(1);
            let mut slot = Some(sender);
            send_or_respawn(&mut slot, 1, || panic!("a live worker is never replaced")).unwrap();
            let error = send_or_respawn(&mut slot, 2, || panic!("a live worker is never replaced"))
                .unwrap_err();
            assert!(error.to_string().contains("full"), "{error}");
            assert_eq!(inbox.try_recv().unwrap(), 1);
        }

        #[test]
        fn the_queue_key_is_released_when_work_panics() {
            let key = format!("test-{}", uuid::Uuid::new_v4());
            queued().insert(key.clone());
            let guard = QueuedKey(key.clone());
            let outcome = std::panic::catch_unwind(std::panic::AssertUnwindSafe(move || {
                let _guard = guard;
                panic!("work failed");
            }));
            assert!(outcome.is_err());
            assert!(!queued().contains(&key));
        }

        #[test]
        fn waiting_retries_back_off_up_to_a_minute() {
            let delays: Vec<_> = (0..6)
                .map(|attempt| retry_delay(attempt).as_secs())
                .collect();
            assert_eq!(delays, [5, 10, 20, 40, 60, 60]);
        }

        #[test]
        fn panic_payloads_keep_their_message() {
            let text =
                std::panic::catch_unwind(|| -> u8 { panic!("shape {} mismatch", 3) }).unwrap_err();
            assert_eq!(panic_text(&*text), "shape 3 mismatch");
            let text = std::panic::catch_unwind(|| -> u8 { panic!("static message") }).unwrap_err();
            assert_eq!(panic_text(&*text), "static message");
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn task_facades_reject_incompatible_objectives() {
        assert!(validate_task(AutoTrainTask::Tabular, &TaskKind::SensorRegression).is_ok());
        assert!(validate_task(AutoTrainTask::Forecast, &TaskKind::SequenceForecast).is_ok());
        assert!(validate_task(AutoTrainTask::Vision, &TaskKind::ObjectDetection).is_ok());
        assert!(validate_task(AutoTrainTask::Anomaly, &TaskKind::SequenceAutoencoder).is_ok());
        assert!(validate_task(AutoTrainTask::Tabular, &TaskKind::ObjectDetection).is_err());
    }

    #[cfg(feature = "training")]
    #[test]
    fn ordinal_table_search_returns_a_durable_bundle_with_fitted_preprocessing() {
        use super::super::auto_training_tables::{
            PinnedTableSource, prepare_rows, transform_table_rows,
        };
        use flow_like_ml_runtime::{
            MetricBound, StreamKey, TrainingRepository, auto_training::SearchPredictorFactory,
        };
        use flow_like_types::json::{from_value, json};
        let directory =
            std::env::temp_dir().join(format!("flow-like-ordinal-search-{}", uuid::Uuid::new_v4()));
        let repo = TrainingRepository::open(directory.join("training.sqlite")).unwrap();
        let source: PinnedTableSource = from_value(json!({"database":{"cache_key":"source"},"reference":{"table":"measurements","branch":"main","version":1,"read_only":true,"pinned":true},"locator":null})).unwrap();
        let request: TableDatasetRequest = from_value(json!({
            "source":{"cache_key":"source"},"stream_id":"quality",
            "spec":{"id":"ordered-quality","description":"Predict quality level","task":"sensor_classification","labels":["low","medium","high"],"input_shape":[1],"prediction_horizon_ms":null,"minimum_examples":3,"minimum_examples_per_class":1},
            "mapping":{"row_id":"id","group_id":"group","features":{"kind":"tabular","options":{"numeric_columns":["measurement"],"categorical_columns":[]}},"target":{"kind":"column","column":"target"},"provenance":{"kind":"measured","source":"gauge"}},
            "split":{"kind":"group","train_fraction":0.6,"validation_fraction":0.2,"seed":42}
        })).unwrap();
        let rows: Vec<_> = (0..90).map(|i| json!({"id":i,"group":i/3,"measurement":(i%3) as f64 * 20.0 + i as f64 / 100.0,"target":(["low","medium","high"][i%3])})).collect();
        let prepared = prepare_rows(source, &request, &rows).unwrap();
        let stream = StreamKey {
            project_id: "test".into(),
            stream_id: "quality".into(),
            inspection_version: "v1".into(),
        };
        let snapshot = snapshot_from_table(
            &prepared,
            stream.clone(),
            table_split_policy(&request.split).unwrap(),
        )
        .unwrap();
        for sample in snapshot
            .train
            .iter()
            .chain(&snapshot.validation)
            .chain(&snapshot.test)
        {
            repo.record_sample(&stream, sample).unwrap();
        }
        let snapshot = repo.import_experiment_snapshot(snapshot).unwrap();
        let mut goals = flow_like_ml_runtime::auto_training::default_goals(&prepared.spec);
        goals.primary_metric = "mean_rank_error".into();
        goals.direction = MetricDirection::Minimize;
        goals.bounds = vec![MetricBound {
            name: "mean_rank_error".into(),
            minimum: None,
            maximum: Some(0.5),
        }];
        let compute = ComputeConfig {
            backend: flow_like_ml_core::ComputeBackend::Cpu,
            ..Default::default()
        };
        let mut budget = ExperimentBudget::default();
        budget.maximum_trials = 2;
        let experiment = repo
            .create_experiment(
                flow_like_ml_runtime::experiment::ExperimentRequest {
                    stream,
                    snapshot_id: snapshot.id,
                    spec: json!(prepared.spec),
                    candidates: legacy_candidates(&prepared.spec, &compute, true).unwrap(),
                    goals,
                    budget: budget.clone(),
                    source_table_versions: vec![],
                    created_tables: vec![],
                    updated_table_versions: vec![],
                    preprocessing_manifest: json!(prepared.preprocessing),
                    context: json!({}),
                },
                flow_like_ml_runtime::now_ms(),
            )
            .unwrap();
        controller(repo.clone(), &budget)
            .unwrap()
            .run(&experiment.id)
            .unwrap();
        let reopened = TrainingRepository::open(directory.join("training.sqlite")).unwrap();
        let consultation = consultation_context(&reopened, &experiment.id).unwrap();
        assert!(consultation.get("final_test_metrics").is_none());
        assert!(consultation.get("final_evaluation").is_none());
        assert!(consultation.get("samples").is_none());
        assert!(
            consultation["trials"]
                .as_array()
                .unwrap()
                .iter()
                .all(|trial| {
                    trial.get("validation_metrics").is_some()
                        && trial.get("weights").is_none()
                        && trial.get("test").is_none()
                })
        );
        assert_eq!(
            consultation["registered_datasets"][0]["input_shape"],
            json!([1])
        );
        assert_eq!(
            consultation["registered_datasets"][0]["numeric_columns"],
            json!(["measurement"])
        );
        let result: AutoTrainingResult = reopened.experiment_result(&experiment.id).unwrap().into();
        assert!(result.goal_met, "{:?}", result.unmet_constraints);
        assert_eq!(result.status, ExperimentStatus::Completed);
        assert!(result.final_test_metrics["mean_rank_error"] <= 0.5);
        assert_eq!(result.preprocessing_manifest, json!(prepared.preprocessing));
        let artifact = result.model_bundle_ref.unwrap();
        assert_eq!(
            artifact.manifest["preprocessing_manifest"],
            json!(prepared.preprocessing)
        );
        let mut predictor = super::super::auto_training_legacy::CatalogSearchPredictorFactory
            .load(&reopened, &artifact.id, &compute)
            .unwrap();
        let mut input =
            transform_table_rows(&prepared.preprocessing, &[json!({"measurement":20.2})], 16)
                .unwrap()
                .remove(0);
        input.shape.insert(0, 1);
        assert_eq!(predictor.predict(&input).unwrap()["classes"][0], 1);
        std::fs::remove_dir_all(directory).unwrap();
    }

    #[cfg(feature = "training")]
    mod scenarios {
        use super::super::*;
        use crate::inspection::auto_training_tables::{
            PinnedTableSource, PreparedTableDataset, TablePreprocessing, prepare_rows,
        };
        use flow_like_catalog_core::{FlowPath, NodeDBConnection};
        use flow_like_ml_runtime::{
            EvaluationTask, PredictionRecord, StreamKey, TrainingRepository, TrainingSample,
        };
        use flow_like_types::json::{from_slice, from_value, json};
        use std::collections::BTreeMap;

        struct Scratch(std::path::PathBuf);
        impl Scratch {
            fn new(name: &str) -> Self {
                let path =
                    std::env::temp_dir().join(format!("flow-like-{name}-{}", uuid::Uuid::new_v4()));
                std::fs::create_dir_all(&path).unwrap();
                Self(path)
            }
        }
        impl Drop for Scratch {
            fn drop(&mut self) {
                let _ = std::fs::remove_dir_all(&self.0);
            }
        }

        struct Harness {
            context: ExecutionContext,
            source: NodeDBConnection,
            _scratch: Scratch,
        }

        /// A live workflow context for board "auto-test:board" whose source table holds `rows`.
        async fn harness(rows: Vec<Value>) -> Harness {
            use ahash::AHashMap;
            use flow_like::{
                flow::{
                    board::ExecutionStage,
                    execution::{
                        LogLevel, Run, context::ExecutionContextCache, internal_node::InternalNode,
                    },
                },
                profile::Profile,
                state::{FlowLikeConfig, FlowLikeState, FlowLikeStores},
                utils::http::HTTPClient,
            };
            use flow_like_catalog_core::CachedDB;
            use flow_like_storage::{
                databases::vector::{
                    VectorStore, buffered::BufferedVectorStore, lancedb::LanceDBVectorStore,
                },
                files::store::{FlowLikeStore, local_store::LocalObjectStore},
                object_store::path::Path,
            };
            use flow_like_types::sync::{Mutex, RwLock};
            use std::sync::{Arc, Weak};
            let scratch = Scratch::new("auto-model");
            let db_location = scratch.0.join("database").to_string_lossy().into_owned();
            let mut config = FlowLikeConfig::new();
            let reopen = db_location.clone();
            config.register_build_project_database(Arc::new(move |_| {
                flow_like_storage::lancedb::connect(&reopen)
            }));
            let state = Arc::new(FlowLikeState::new(
                config,
                HTTPClient::new_without_refetch(),
            ));
            let logic: Arc<dyn NodeLogic> = Arc::new(GetAutoTrainingNode);
            let node = Arc::new(InternalNode::new(
                logic.get_node(),
                AHashMap::new(),
                logic,
                AHashMap::new(),
            ));
            let mut nodes = AHashMap::new();
            nodes.insert(node.node_id().to_string(), node.clone());
            let run: Weak<Mutex<Run>> = Weak::new();
            let mut context = ExecutionContext::new(
                Arc::new(nodes),
                &run,
                &state,
                &node,
                &Arc::new(Mutex::new(AHashMap::new())),
                &Arc::new(RwLock::new(AHashMap::new())),
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
            let mut stores = FlowLikeStores::default();
            stores.app_storage_store = Some(FlowLikeStore::Local(Arc::new(
                LocalObjectStore::new(scratch.0.clone()).unwrap(),
            )));
            context.execution_cache = Some(ExecutionContextCache {
                stores,
                app_id: "auto-test".into(),
                model_usage_app_id: None,
                board_dir: Path::from("board"),
                board_id: "board".into(),
                node_id: node.shared_node_id(),
                sub: "tester".into(),
                shadow: false,
            });
            let connection = flow_like_storage::lancedb::connect(&db_location)
                .execute()
                .await
                .unwrap();
            let mut store =
                LanceDBVectorStore::from_connection(connection, "measurements".into()).await;
            store.insert(rows).await.unwrap();
            let source = NodeDBConnection {
                cache_key: "auto-source".into(),
            };
            context.cache.write().await.insert(
                source.cache_key.clone(),
                Arc::new(CachedDB {
                    db: Arc::new(RwLock::new(BufferedVectorStore::new(store, 0))),
                }),
            );
            Harness {
                context,
                source,
                _scratch: scratch,
            }
        }

        /// One reading in four per group is anomalous, so every partition holds both classes.
        fn anomaly_rows() -> Vec<Value> {
            (0..96)
                .map(|i| {
                    let anomaly = i % 4 == 3;
                    let (x, y) = if anomaly {
                        (6.0 + (i % 3) as f64, -6.0 - (i % 5) as f64)
                    } else {
                        ((i % 7) as f64 / 10.0, (i % 5) as f64 / 10.0)
                    };
                    json!({"id":i.to_string(),"group":(i / 4).to_string(),"x":x,"y":y,"anomaly":anomaly})
                })
                .collect()
        }

        fn anomaly_request(source: &NodeDBConnection) -> AutoTrainRequest {
            from_value(json!({
                "dataset":{"source":source,"stream_id":"sensors",
                    "spec":{"id":"sensor-anomaly","description":"Detect anomalous sensor readings","task":"sensor_anomaly","labels":[],"input_shape":[2],"prediction_horizon_ms":null,"minimum_examples":2,"minimum_examples_per_class":1},
                    "mapping":{"row_id":"id","group_id":"group","features":{"kind":"tabular","options":{"numeric_columns":["x","y"],"categorical_columns":[]}},"target":{"kind":"column","column":"anomaly"},"provenance":{"kind":"measured","source":"gauge"}},
                    "split":{"kind":"group","train_fraction":0.6,"validation_fraction":0.2,"seed":11}},
                "compute":{"backend":"cpu"},"search":{"maximum_candidates":1},"budget":{"maximum_trials":1},
                "tables":{"create_tables":false},
                "initial_candidates":[{"engine":"isolation_forest","recipe":{"labels":[],"inspection_task":"sensor_anomaly","config":{"trees":64,"sample_size":256,"seed":5}},"compute":{"backend":"cpu"}}]
            }))
            .unwrap()
        }

        /// The target depends on |x| only, so a linear model on x cannot separate it while the
        /// derived square of x separates it exactly.
        fn distance_rows() -> Vec<Value> {
            (0..120)
                .map(|i| {
                    let x = (i % 8) as f64 - 3.5;
                    json!({"id":i.to_string(),"group":(i / 2).to_string(),"x":x,"z":((i * 7) % 11) as f64,"target":if x.abs() > 2.0 {"far"} else {"near"}})
                })
                .collect()
        }

        fn distance_request(source: &NodeDBConnection) -> AutoTrainRequest {
            from_value(json!({
                "dataset":{"source":source,"stream_id":"distance",
                    "spec":{"id":"distance","description":"Classify the distance from a setpoint","task":"sensor_classification","labels":["near","far"],"input_shape":[2],"prediction_horizon_ms":null,"minimum_examples":2,"minimum_examples_per_class":1},
                    "mapping":{"row_id":"id","group_id":"group","features":{"kind":"tabular","options":{"numeric_columns":["x","z"],"categorical_columns":[]}},"target":{"kind":"column","column":"target"},"provenance":{"kind":"measured","source":"gauge"}},
                    "split":{"kind":"group","train_fraction":0.6,"validation_fraction":0.2,"seed":3}},
                "compute":{"backend":"cpu"},"search":{"maximum_candidates":1},"budget":{"maximum_trials":2},
                "initial_candidates":[{"engine":"legacy_classical","recipe":{"algorithm":{"kind":"logistic","alpha":1.0,"max_iterations":200},"labels":["near","far"]},"compute":{"backend":"cpu"}}]
            }))
            .unwrap()
        }

        /// A search seed whose single generated candidate for two features is gradient
        /// boosting, which separates the derived feature exactly.
        fn boosting_seed(request: &AutoTrainRequest) -> u64 {
            (0..256)
                .find(|seed| {
                    let search = AutoSearchConfig {
                        seed: *seed,
                        maximum_candidates: 1,
                        ..request.search.clone()
                    };
                    flow_like_ml_runtime::auto_training::generate_candidates(
                        &request.dataset.spec,
                        &search,
                        &request.compute,
                    )
                    .is_ok_and(|candidates| candidates[0].engine == "histogram_gradient_boosting")
                })
                .expect("a search seed generates gradient boosting first")
        }

        fn distance_dataset() -> (AutoTrainRequest, PreparedTableDataset) {
            let source: PinnedTableSource = from_value(json!({"database":{"cache_key":"source"},"reference":{"table":"measurements","branch":"main","version":1,"read_only":true,"pinned":true},"locator":null})).unwrap();
            let request = distance_request(&source.database);
            let prepared = prepare_rows(source, &request.dataset, &distance_rows()).unwrap();
            (request, prepared)
        }

        fn create_experiment(
            repo: &TrainingRepository,
            project: &str,
            request: &AutoTrainRequest,
            prepared: &PreparedTableDataset,
            context: Value,
        ) -> Experiment {
            let stream = StreamKey {
                project_id: project.into(),
                stream_id: request.dataset.stream_id.clone(),
                inspection_version: "v1".into(),
            };
            let snapshot = snapshot_from_table(
                prepared,
                stream.clone(),
                table_split_policy(&request.dataset.split).unwrap(),
            )
            .unwrap();
            record_and_create(
                repo,
                &stream,
                snapshot,
                flow_like_ml_runtime::experiment::ExperimentRequest {
                    stream: stream.clone(),
                    snapshot_id: String::new(),
                    spec: json!(prepared.spec),
                    candidates: request.initial_candidates.clone(),
                    goals: flow_like_ml_runtime::auto_training::default_goals(&prepared.spec),
                    budget: request.budget.clone(),
                    source_table_versions: vec![],
                    created_tables: vec![],
                    updated_table_versions: vec![],
                    preprocessing_manifest: json!(prepared.preprocessing),
                    context,
                },
                &|| false,
            )
            .unwrap()
        }

        async fn run_to_completion(
            context: &ExecutionContext,
            id: &str,
        ) -> (TrainingRepository, Experiment) {
            let repo = experiment_repository(context, id).unwrap();
            let budget = repo.get_experiment(id).unwrap().request.budget;
            let (worker, owned) = (repo.clone(), id.to_string());
            let experiment = tokio::task::spawn_blocking(move || {
                controller(worker, &budget).unwrap().run(&owned)
            })
            .await
            .unwrap()
            .unwrap();
            (repo, experiment)
        }

        fn final_test_predictions(
            repo: &TrainingRepository,
            artifact: &str,
        ) -> BTreeMap<String, PredictionRecord> {
            repo.predictions(artifact)
                .unwrap()
                .into_iter()
                .filter(|record| record.details["partition"] == "final_test")
                .inspect(|record| assert!(!record.failed, "{:?}", record.details))
                .map(|record| (record.sample_id.clone(), record))
                .collect()
        }

        fn raw_rows_request(id: &str, test: &[TrainingSample]) -> PredictAutoModelRequest {
            PredictAutoModelRequest {
                experiment_id: id.into(),
                rows: test
                    .iter()
                    .map(|sample| sample.payload["feature_row"].clone())
                    .collect(),
                compute: ComputeConfig {
                    backend: flow_like_ml_core::ComputeBackend::Cpu,
                    ..Default::default()
                },
                stateful_features: false,
                feature_state: None,
                feature_extractor: None,
            }
        }

        fn export_path(name: &str) -> FlowPath {
            FlowPath {
                path: format!("exports/{name}"),
                store_ref: "export-store".into(),
                cache_store_ref: None,
            }
        }

        /// The class the final audit decodes from a raw classification output.
        fn audited_label(raw: &Value, labels: &[&str]) -> String {
            let class = if let Some(class) = raw
                .get("classes")
                .and_then(Value::as_array)
                .and_then(|classes| classes.first())
                .and_then(Value::as_u64)
            {
                class as usize
            } else if raw.get("scores").is_some() && labels.len() == 2 {
                usize::from(raw["scores"][0].as_f64().unwrap() >= 0.5)
            } else {
                raw["output"]["values"]
                    .as_array()
                    .unwrap()
                    .iter()
                    .enumerate()
                    .max_by(|a, b| a.1.as_f64().unwrap().total_cmp(&b.1.as_f64().unwrap()))
                    .unwrap()
                    .0
            };
            labels[class].to_string()
        }

        #[tokio::test]
        async fn anomaly_predictions_follow_the_audited_rule_and_exports_round_trip() {
            let mut harness = harness(anomaly_rows()).await;
            let request = anomaly_request(&harness.source);
            let started = start_auto_training(&mut harness.context, request, false)
                .await
                .unwrap();
            let (repo, experiment) =
                run_to_completion(&harness.context, &started.experiment_id).await;
            assert_eq!(
                experiment.status,
                ExperimentStatus::Completed,
                "{:?}",
                experiment.stop_reason
            );
            let result: AutoTrainingResult = repo.experiment_result(&experiment.id).unwrap().into();
            let artifact = result.model_artifact_id.clone().unwrap();
            let audited = final_test_predictions(&repo, &artifact);
            let test = repo
                .get_snapshot(&experiment.request.snapshot_id)
                .unwrap()
                .test;
            assert_eq!(audited.len(), test.len());
            let served =
                predict_auto_model(&harness.context, raw_rows_request(&experiment.id, &test))
                    .await
                    .unwrap();
            let anomaly = served
                .anomaly
                .expect("anomaly experiments return the audited decision rule");
            assert_eq!(anomaly.threshold, 0.5);
            assert_eq!(anomaly.scale, result.validation_metrics["anomaly_scale"]);
            assert_eq!(anomaly.rows.len(), test.len());
            for (sample, decision) in test.iter().zip(&anomaly.rows) {
                let probability = audited[&sample.id].details["prediction"]["probability"]
                    .as_f64()
                    .unwrap();
                assert!(
                    (decision.probability - probability).abs() < 1e-12,
                    "sample {}: served {} but audited {probability}",
                    sample.id,
                    decision.probability
                );
                assert_eq!(decision.is_anomaly, probability >= 0.5, "{}", sample.id);
            }

            harness.context.cache.write().await.insert(
                "export-store".into(),
                std::sync::Arc::new(flow_like_storage::files::store::FlowLikeStore::Memory(
                    std::sync::Arc::new(flow_like_storage::object_store::memory::InMemory::new()),
                )),
            );
            let (weights, manifest) = (export_path("model.bin"), export_path("model.json"));
            let exported = export_auto_model(
                &mut harness.context,
                ExportAutoModelRequest {
                    experiment_id: experiment.id.clone(),
                    weights: weights.clone(),
                    manifest: manifest.clone(),
                },
            )
            .await
            .unwrap();
            assert_eq!(exported.model_artifact_id, artifact);
            let bytes = weights.get(&mut harness.context, true).await.unwrap();
            assert_eq!(bytes.len() as u64, exported.bytes);
            assert_eq!(flow_like_ml_core::content_digest(&bytes), exported.sha256);
            let written: Value =
                from_slice(&manifest.get(&mut harness.context, true).await.unwrap()).unwrap();
            assert_eq!(written["artifact"]["id"], json!(artifact));
            assert_eq!(written["preprocessing"], result.preprocessing_manifest);
            assert_eq!(
                written["final_test_metrics"],
                json!(result.final_test_metrics)
            );
        }

        #[tokio::test]
        async fn a_derived_feature_winner_serves_its_own_preprocessing() {
            let mut harness = harness(distance_rows()).await;
            let mut request = distance_request(&harness.source);
            request.search.seed = boosting_seed(&request);
            let started = start_auto_training(&mut harness.context, request, false)
                .await
                .unwrap();
            let plan: crate::inspection::feature_engineering::FeaturePlan = from_value(json!({
                "pipeline":{"steps":[{"kind":"derive_columns","columns":[{"name":"x_squared","expression":{"kind":"unary","op":"square","value":{"kind":"column","name":"x"}}}]}]},
                "numeric_columns":["x","x_squared"],"categorical_columns":[],"standardize":true
            }))
            .unwrap();
            crate::inspection::feature_engineering::derive_pipeline_candidate(
                &mut harness.context,
                &started.experiment_id,
                plan,
            )
            .await
            .unwrap();
            let (repo, experiment) =
                run_to_completion(&harness.context, &started.experiment_id).await;
            assert_eq!(
                experiment.status,
                ExperimentStatus::Completed,
                "{:?}",
                experiment.stop_reason
            );
            let state = repo.experiment_result(&experiment.id).unwrap();
            let winner = state
                .trials
                .iter()
                .find(|trial| experiment.selected_trial_id.as_ref() == Some(&trial.id))
                .unwrap();
            assert_ne!(
                winner.dataset_snapshot_id, experiment.request.snapshot_id,
                "the derived dataset must win: {:?}",
                state.trials
            );
            let result: AutoTrainingResult = state.into();
            assert_ne!(
                result.preprocessing_manifest,
                experiment.request.preprocessing_manifest
            );
            let artifact = result.model_artifact_id.clone().unwrap();
            let audited = final_test_predictions(&repo, &artifact);
            let test = repo
                .get_snapshot(&experiment.request.snapshot_id)
                .unwrap()
                .test;
            assert_eq!(audited.len(), test.len());
            let served =
                predict_auto_model(&harness.context, raw_rows_request(&experiment.id, &test))
                    .await
                    .unwrap();
            assert!(served.anomaly.is_none());
            assert_eq!(served.predictions.len(), test.len());
            for (sample, raw) in test.iter().zip(&served.predictions) {
                assert_eq!(
                    Some(audited_label(raw, &["near", "far"])),
                    audited[&sample.id].predicted_label,
                    "sample {}",
                    sample.id
                );
            }
        }

        fn refusal<T>(entry: &'static str, result: Result<T>) -> (&'static str, String) {
            let message = match result {
                Ok(_) => "the call succeeded".to_string(),
                Err(error) => error.to_string(),
            };
            (entry, message)
        }

        async fn entry_point_errors(
            context: &mut ExecutionContext,
            id: &str,
        ) -> Vec<(&'static str, String)> {
            vec![
                refusal("get", lifecycle(context, id, LifecycleAction::Status).await),
                refusal("step", lifecycle(context, id, LifecycleAction::Step).await),
                refusal(
                    "cancel",
                    lifecycle(context, id, LifecycleAction::Cancel).await,
                ),
                refusal(
                    "resume",
                    lifecycle(context, id, LifecycleAction::Resume).await,
                ),
                refusal(
                    "finalize",
                    lifecycle(context, id, LifecycleAction::Finalize).await,
                ),
                refusal(
                    "materialize",
                    materialize_final_predictions(context, id).await,
                ),
                refusal(
                    "predict",
                    predict_auto_model(context, raw_rows_request(id, &[])).await,
                ),
                refusal(
                    "export",
                    export_auto_model(
                        context,
                        ExportAutoModelRequest {
                            experiment_id: id.into(),
                            weights: export_path("model.bin"),
                            manifest: export_path("model.json"),
                        },
                    )
                    .await,
                ),
                refusal(
                    "select features",
                    derive_feature_candidate(context, id, vec!["x".into()], vec![], true).await,
                ),
            ]
        }

        #[tokio::test]
        async fn other_boards_and_apps_cannot_reach_an_experiment() {
            let mut harness = harness(anomaly_rows()).await;
            let request = anomaly_request(&harness.source);
            let started = start_auto_training(&mut harness.context, request, false)
                .await
                .unwrap();
            let id = started.experiment_id;
            for (app, board) in [("auto-test", "other-board"), ("other-app", "board")] {
                let scope = harness.context.execution_cache.as_mut().unwrap();
                scope.app_id = app.into();
                scope.board_id = board.into();
                for (entry, error) in entry_point_errors(&mut harness.context, &id).await {
                    assert!(
                        error.contains("different app or board"),
                        "{entry} from {app}:{board}: {error}"
                    );
                }
            }
            let scope = harness.context.execution_cache.as_mut().unwrap();
            scope.app_id = "auto-test".into();
            scope.board_id = "board".into();
            let status = lifecycle(&mut harness.context, &id, LifecycleAction::Status)
                .await
                .unwrap();
            assert_eq!(status.status, ExperimentStatus::Running);
        }

        #[tokio::test]
        async fn rejected_starts_leave_no_experiment_or_ledger_samples() {
            let mut harness = harness(anomaly_rows()).await;
            let mut destination = anomaly_request(&harness.source);
            destination.tables.allowed_destination = Some(harness.source.clone());
            let mut small_budget = anomaly_request(&harness.source);
            small_budget.budget.maximum_dataset_bytes = 4096;
            let mut unmeasured = anomaly_request(&harness.source);
            unmeasured.goals = Some(ExperimentGoals {
                task: EvaluationTask::Anomaly { threshold: 0.5 },
                primary_metric: "mean_iou".into(),
                direction: MetricDirection::Maximize,
                minimum_audited_samples: 1,
                bounds: vec![],
            });
            let (repo, project) =
                crate::inspection::lifecycle::repository(&harness.context).unwrap();
            for (request, expected) in [
                (destination, "expected durable reference"),
                (small_budget, "maximum_dataset_bytes"),
                (unmeasured, "not measured"),
            ] {
                let stream = crate::inspection::lifecycle::InspectionContext {
                    stream_id: request.dataset.stream_id.clone(),
                    spec: request.dataset.spec.clone(),
                }
                .key(project.clone())
                .unwrap();
                let error = start_auto_training_scoped(
                    &mut harness.context,
                    request,
                    false,
                    Some(stream.clone()),
                )
                .await
                .err()
                .expect("the start is rejected")
                .to_string();
                assert!(error.contains(expected), "{error}");
                assert!(repo.list_experiments(&stream).unwrap().is_empty());
                assert_eq!(
                    repo.count_new_training_samples(&stream, flow_like_ml_runtime::now_ms())
                        .unwrap(),
                    0
                );
            }
        }

        #[tokio::test]
        async fn a_rejected_derived_candidate_registers_nothing() {
            let mut harness = harness(distance_rows()).await;
            let (repo, project) =
                crate::inspection::lifecycle::repository(&harness.context).unwrap();
            let request = distance_request(&harness.source);
            let prepared = crate::inspection::auto_training_tables::prepare_table_dataset(
                &mut harness.context,
                &request.dataset,
            )
            .await
            .unwrap();
            let experiment = create_experiment(
                &repo,
                &project,
                &request,
                &prepared,
                json!({
                    "search":request.search,"compute":request.compute,"dataset_request":request.dataset,
                    "pinned_source":prepared.source,
                    "table_policy":{"create_tables":false,"allowed_destination":harness.source,"expected_destination":prepared.source.reference},
                }),
            );
            let error =
                register_prepared_candidate(&mut harness.context, &experiment.id, prepared.clone())
                    .await
                    .unwrap_err();
            assert!(
                error.to_string().contains("separate versioned table"),
                "{error}"
            );
            let state = repo.experiment_result(&experiment.id).unwrap();
            assert_eq!(
                state.experiment.request.candidates.len(),
                experiment.request.candidates.len()
            );
            assert_eq!(state.datasets.len(), 1);
            assert_eq!(
                state.experiment.usage.dataset_bytes,
                experiment.usage.dataset_bytes
            );
        }

        #[tokio::test]
        async fn a_dropped_wait_cancels_the_experiment_and_a_finished_one_does_not() {
            let mut harness = harness(anomaly_rows()).await;
            let request = anomaly_request(&harness.source);
            let started = start_auto_training(&mut harness.context, request, false)
                .await
                .unwrap();
            let id = started.experiment_id;
            let repo = experiment_repository(&harness.context, &id).unwrap();
            let (worker, owned) = (repo.clone(), id.clone());
            let finished = tokio::task::spawn_blocking(move || worker.get_experiment(&owned));
            await_experiment_work(None, &repo, &id, finished, "cancelled")
                .await
                .unwrap();
            assert_eq!(
                repo.get_experiment(&id).unwrap().status,
                ExperimentStatus::Running
            );
            // The blocking work waits until `release` is dropped, so the wait stays pending.
            let (release, blocked) = std::sync::mpsc::channel::<()>();
            let (worker, owned) = (repo.clone(), id.clone());
            let blocked_work = tokio::task::spawn_blocking(move || {
                let _ = blocked.recv();
                worker.get_experiment(&owned)
            });
            let mut waiting = Box::pin(await_experiment_work(
                None,
                &repo,
                &id,
                blocked_work,
                "cancelled",
            ));
            let pending = std::future::poll_fn(|cx| {
                std::task::Poll::Ready(std::future::Future::poll(waiting.as_mut(), cx).is_pending())
            })
            .await;
            drop(waiting);
            let status = repo.get_experiment(&id).unwrap().status;
            drop(release);
            assert!(pending);
            assert_eq!(status, ExperimentStatus::Cancelled);
        }

        #[test]
        fn consultation_reports_only_mapped_columns_within_their_budget() {
            let scratch = Scratch::new("consultation");
            let repo = TrainingRepository::open(scratch.0.join("training.sqlite")).unwrap();
            let (request, prepared) = distance_dataset();
            let column = |name: &str, examples: Value| json!({"name":name,"missing":0,"numeric":72,"boolean":0,"text":0,"other":0,"minimum":0.0,"maximum":1.0,"examples":examples});
            let narrow = create_experiment(
                &repo,
                "project",
                &request,
                &prepared,
                json!({"dataset_request":request.dataset,"training_profile":{"partition":"train","rows":72,"columns":[
                    column("email", json!(["secret@example.com"])),
                    column("group", json!(["7"])),
                    column("id", json!(["14"])),
                    column("target", json!(["near","far"])),
                    column("x", json!([-3.5,-2.5,-1.5])),
                    column("z", json!([1.0,2.0])),
                ]}}),
            );
            let report = consultation_context(&repo, &narrow.id).unwrap();
            assert!(report.get("report_truncated").is_none());
            let names: Vec<_> = report["training_profile"]["columns"]
                .as_array()
                .unwrap()
                .iter()
                .map(|column| column["name"].as_str().unwrap().to_owned())
                .collect();
            assert_eq!(names, ["target", "x", "z"]);
            assert_eq!(report["metadata_columns"], json!(["id", "group"]));
            assert!(!report.to_string().contains("secret@example.com"));

            let sensors: Vec<String> = (0..3000).map(|i| format!("sensor_{i:04}")).collect();
            let mapped: Vec<String> = std::iter::once("x".to_string())
                .chain(sensors.iter().cloned())
                .collect();
            let mut dataset_request = json!(request.dataset);
            dataset_request["mapping"]["features"]["options"]["numeric_columns"] = json!(mapped);
            let mut columns = vec![
                column("email", json!(["secret@example.com"])),
                column("target", json!(["near", "far"])),
                column("x", json!([-3.5, -2.5, -1.5, 1.5, 3.5])),
            ];
            columns.extend(
                sensors
                    .iter()
                    .map(|name| column(name, json!([0.125, 0.25, 0.375, 0.5, 0.625]))),
            );
            let wide = create_experiment(
                &repo,
                "project",
                &request,
                &prepared,
                json!({"dataset_request":dataset_request,"training_profile":{"partition":"train","rows":72,"columns":columns}}),
            );
            let report = consultation_context(&repo, &wide.id).unwrap();
            let text = report.to_string();
            assert!(text.len() <= 128 * 1024, "{} bytes", text.len());
            assert!(!text.contains("secret@example.com"));
            assert!(report.get("report_truncated").is_some());
            assert!(
                report["training_profile"]["columns_omitted"]
                    .as_u64()
                    .unwrap()
                    > 0
            );
            assert!(
                report["allowed_features"]["options"]["numeric_columns_omitted"]
                    .as_u64()
                    .unwrap()
                    > 0
            );
            assert_eq!(report["training_profile"]["columns"][0]["name"], "target");
        }

        #[test]
        fn trial_lists_keep_the_best_and_the_newest() {
            let trials: Vec<_> = (0..5)
                .map(|index| (format!("trial-{index}"), json!(index)))
                .collect();
            assert_eq!(
                select_trials(&trials, Some("trial-0"), 3),
                (vec![json!(0), json!(3), json!(4)], 2)
            );
            assert_eq!(
                select_trials(&trials, None, 2),
                (vec![json!(3), json!(4)], 3)
            );
            assert_eq!(select_trials(&trials, None, 8).1, 0);
        }

        #[test]
        fn anomaly_decisions_apply_the_audited_calibration() {
            let predictions = [
                json!({"scores":[0.5]}),
                json!({"reconstruction_error":[0.1]}),
            ];
            let decisions = anomaly_decisions(&predictions, 0.5, 0.5).unwrap();
            assert_eq!(decisions.rows[0].probability, 0.5);
            assert!(decisions.rows[0].is_anomaly);
            assert!((decisions.rows[1].probability - 0.1 / 0.6).abs() < 1e-12);
            assert!(!decisions.rows[1].is_anomaly);
            assert!(anomaly_decisions(&[json!({"scores":[-1.0]})], 0.5, 0.5).is_err());
            assert!(anomaly_decisions(&[json!({"classes":[1]})], 0.5, 0.5).is_err());
            assert!(anomaly_decisions(&predictions, 0.0, 0.5).is_err());
        }

        #[test]
        fn embeddings_must_name_one_feature_extractor() {
            use crate::inspection::lifecycle::FeatureExtractorRef;
            let preprocessing = TablePreprocessing::Tensor {
                column: "features".into(),
                input_shape: vec![4, 8],
            };
            let extractor = |source: &str| FeatureExtractorRef {
                source_id: source.into(),
                blob_sha256: "a".repeat(64),
                image_steps: vec![],
            };
            let row = |source: &str| json!({"features":{"shape":[4,8],"values":[],"feature_extractor":extractor(source)}});
            assert!(
                requested_extractor(&preprocessing, &[json!({"features":{}})], None)
                    .unwrap()
                    .is_none()
            );
            let carried = requested_extractor(&preprocessing, &[row("dino"), row("dino")], None)
                .unwrap()
                .unwrap();
            assert_eq!(carried.source_id, "dino");
            assert!(
                requested_extractor(&preprocessing, &[row("dino")], Some(&extractor("dino")))
                    .unwrap()
                    .is_some()
            );
            assert!(
                requested_extractor(&preprocessing, &[row("dino"), row("resnet")], None).is_err()
            );
            assert!(
                requested_extractor(&preprocessing, &[row("dino")], Some(&extractor("resnet")))
                    .is_err()
            );
            let invalid = FeatureExtractorRef {
                blob_sha256: "short".into(),
                ..extractor("dino")
            };
            assert!(requested_extractor(&preprocessing, &[], Some(&invalid)).is_err());
            let malformed = json!({"features":{"feature_extractor":{"source_id":"dino"}}});
            assert!(requested_extractor(&preprocessing, &[malformed], None).is_err());
        }

        #[test]
        fn predict_requests_take_the_extractor_that_feature_nodes_output() {
            use crate::inspection::lifecycle::FeatureExtractorRef;
            let output = json!({"source_id":"dino","blob_sha256":"d".repeat(64),"image_steps":[]});
            let request: PredictAutoModelRequest = from_value(json!({
                "experiment_id": "experiment",
                "rows": [],
                "feature_extractor": output
            }))
            .unwrap();
            let extractor: FeatureExtractorRef = from_value(output.clone()).unwrap();
            assert_eq!(request.feature_extractor, Some(extractor));
            let unnamed: PredictAutoModelRequest = from_value(json!({
                "experiment_id": "experiment",
                "rows": [],
                "feature_extractor": null
            }))
            .unwrap();
            assert!(unnamed.feature_extractor.is_none());
        }

        #[test]
        fn patch_models_record_the_extractor_their_embeddings_carry() {
            let preprocessing = TablePreprocessing::Tensor {
                column: "features".into(),
                input_shape: vec![4, 8],
            };
            let extractor =
                json!({"source_id":"dino","blob_sha256":"b".repeat(64),"image_steps":[]});
            let tensor = json!({"shape":[4,8],"values":[],"feature_extractor":extractor});
            let rows = [
                json!({ "features": tensor.to_string() }),
                json!({ "features": tensor }),
            ];
            let candidate = |engine: &str| flow_like_ml_runtime::TrainingRequest {
                engine: engine.into(),
                recipe: json!({ "labels": [] }),
                compute: json!({}),
            };
            let mut candidates = vec![
                candidate("patchcore"),
                candidate("padim"),
                candidate("isolation_forest"),
            ];
            let carried = requested_extractor(&preprocessing, &rows, None).unwrap();
            record_feature_extractor(&mut candidates, carried.as_ref()).unwrap();
            assert_eq!(candidates[0].recipe["feature_extractor"], extractor);
            assert_eq!(candidates[1].recipe["feature_extractor"], extractor);
            assert!(candidates[2].recipe.get("feature_extractor").is_none());
        }
    }
}
