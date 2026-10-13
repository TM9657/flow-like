use super::auto_training_tables::TableDatasetRequest;
use flow_like::flow::{
    execution::context::ExecutionContext,
    node::{Node, NodeLogic},
};
use flow_like_ml_core::ComputeConfig;
#[cfg(any(feature = "training", test))]
use flow_like_ml_core::TaskKind;
use flow_like_ml_runtime::experiment::*;
#[cfg(any(feature = "inference", test))]
use flow_like_types::anyhow;
use flow_like_types::{Result, Value};
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
    /// PatchCore and PaDiM only: the extractor that computed the embedding column, as Extract
    /// Image Features, Extract Pretrained Image Features or Feature Map to Patches output it. The
    /// model records it, so Predict Auto Model refuses embeddings from another extractor.
    #[serde(default)]
    pub feature_extractor: Option<super::lifecycle::FeatureExtractorRef>,
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
    /// as `feature_extractor` inside their tensor value. A model that recorded the extractor it
    /// was fitted on refuses embeddings that name no extractor or another one.
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

#[cfg(any(feature = "training", test))]
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
                let mut node = super::operation_node::<AutoTrainRequest, AutoTrainingResult>(
                    $id,
                    $name,
                    "Pin a dataset and start a durable, bounded model search",
                    $function,
                    "AI/ML/Auto Training",
                );
                node.set_version(1);
                node
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
                    Err(super::lifecycle::training_unavailable())
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
    experiment_ledger(context, id).map(|(repo, _)| repo)
}

/// The ledger holding experiment `id` with this run's project string, and its scope.
#[cfg(feature = "training")]
pub(crate) fn experiment_ledger(
    context: &ExecutionContext,
    id: &str,
) -> Result<(
    flow_like_ml_runtime::TrainingRepository,
    super::lifecycle::LedgerScope,
)> {
    let located = super::lifecycle::locate_repository(context, |repo, _| {
        Ok(existing(repo.get_experiment(id))?.is_some())
    })?;
    let Some(ledger) = located else {
        return Err(missing_from_ledgers(
            context,
            &format!("Experiment {id}"),
            &super::published::experiment_pointer_path(id),
            "Predict Auto Model reads its published winner",
        ));
    };
    if ledger.repo.get_experiment(id)?.request.stream.project_id != ledger.project {
        return Err(anyhow!(
            "The experiment belongs to a different app or board"
        ));
    }
    Ok((ledger.repo, ledger.scope))
}

/// A record that no training ledger of this app and board on this device holds. The error
/// names both ledgers and the published file that other runtimes read instead.
#[cfg(feature = "training")]
pub(crate) fn missing_from_ledgers(
    context: &ExecutionContext,
    record: &str,
    published: &str,
    reader: &str,
) -> flow_like_types::Error {
    let (app, user) = match super::lifecycle::ledger_paths(context) {
        Ok(paths) => (
            paths.app.display().to_string(),
            paths
                .user
                .map_or_else(|| "none".to_owned(), |path| path.display().to_string()),
        ),
        Err(error) => (format!("unavailable ({error})"), "none".to_owned()),
    };
    anyhow!(
        "{record} is not in this app and board's training ledgers on this device (app ledger {app}, private ledger {user}); it belongs to a different app or board or was created on another device. {reader} from storage/{published}"
    )
}

/// `Ok(None)` for a record the ledger does not hold; other ledger errors stay errors.
#[cfg(feature = "inference")]
pub(crate) fn existing<T>(found: flow_like_ml_runtime::Result<T>) -> Result<Option<T>> {
    match found {
        Ok(value) => Ok(Some(value)),
        Err(flow_like_ml_runtime::Error::NotFound(_)) => Ok(None),
        Err(error) => Err(error.into()),
    }
}

/// Cancels the experiments queued or running on `ledgers` and waits until `deadline`.
#[cfg(feature = "training")]
pub(crate) fn drain_experiments(
    ledgers: &[std::path::PathBuf],
    deadline: std::time::Instant,
) -> super::lifecycle::DrainReport {
    background::drain(ledgers, deadline)
}

/// Keeps background experiments from being queued or driven on the closed ledgers while it
/// lives.
#[cfg(feature = "training")]
pub(crate) struct ExperimentsClosed {
    _draining: background::Draining,
}

/// Closes `ledgers`, canonical prefixes as for [`drain_experiments`], to background
/// experiments until the returned guard drops.
#[cfg(feature = "training")]
pub(crate) fn close_experiments(ledgers: &[std::path::PathBuf]) -> ExperimentsClosed {
    ExperimentsClosed {
        _draining: background::begin_drain(ledgers).0,
    }
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
    use super::lifecycle::{LEDGER_SCOPE_KEY, LedgerScope};
    use flow_like_ml_runtime::auto_training::{default_goals, validate_goals, validate_search};
    use flow_like_types::json::json;
    super::lifecycle::ensure_training_available(context)?;
    validate_request_rules(&request)?;
    let scoped = match stream {
        Some(stream) => Some(StartLedger::for_stream(
            context, &request, background, stream,
        )?),
        None => {
            if request.learning_cycle_id.is_some() {
                return Err(anyhow!(if background {
                    "Learning cycles must attach their experiment before dispatch"
                } else {
                    "Learning cycle does not own this training stream"
                }));
            }
            None
        }
    };
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
    let (scope, digests) = match &scoped {
        Some(scoped) if scoped.scope == LedgerScope::App => (LedgerScope::App, Default::default()),
        Some(scoped) => (
            scoped.scope,
            super::auto_training_tables::scope_digests(context).await?,
        ),
        None => {
            let digests = super::auto_training_tables::scope_digests(context).await?;
            let scope = source_ledger_scope(&digests, &prepared.source, &prepared.feature_sources);
            (scope, digests)
        }
    };
    if scope == LedgerScope::User {
        ensure_private_table_writes(context, &digests, &prepared.source, &request.tables).await?;
    }
    let StartLedger {
        repo,
        stream,
        marker,
        ..
    } = match scoped {
        Some(scoped) => scoped,
        None => StartLedger::minted(context, scope, &prepared.spec)?,
    };
    let split = table_split_policy(&request.dataset.split)?;
    let PreparedSplit {
        prepared,
        snapshot,
        extractor,
        dataset_bytes,
    } = {
        let stream = stream.clone();
        let named = request.feature_extractor.clone();
        tokio::task::spawn_blocking(move || prepare_split(prepared, stream, split, named.as_ref()))
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
    if scope == LedgerScope::User {
        adopt_pretrained_sources(
            context,
            &repo,
            &stream.project_id,
            request
                .pretrained
                .as_ref()
                .map(|source| source.source_id.as_str()),
            extractor
                .as_ref()
                .map(|extractor| extractor.source_id.as_str()),
        )
        .await?;
    }
    let (mut candidates, unchecked) =
        start_candidates(&repo, &stream.project_id, &prepared.spec, &request)?;
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
    if dataset_bytes > request.budget.maximum_dataset_bytes {
        return Err(anyhow!(
            "The prepared split and its training view serialize to {dataset_bytes} bytes, which exceeds budget.maximum_dataset_bytes ({}); raise the dataset budget or select fewer rows or columns",
            request.budget.maximum_dataset_bytes
        ));
    }
    let mut experiment_context = json!({"training_profile":prepared.profile,"search":request.search,"compute":request.compute,"pretrained":request.pretrained,"dataset_request":request.dataset,"table_policy":request.tables,"pinned_source":prepared.source,"ordered_target":request.ordered_target,"feature_plan":request.feature_plan,"feature_sources":prepared.feature_sources,"learning_cycle_id":request.learning_cycle_id});
    if let Some(marker) = marker {
        experiment_context[LEDGER_SCOPE_KEY] = marker;
    }
    let experiment_request =
        flow_like_ml_runtime::experiment::ExperimentRequest {
            stream: stream.clone(),
            snapshot_id: String::new(),
            spec: json!(prepared.spec),
            candidates,
            goals,
            budget: request.budget.clone(),
            source_table_versions: std::iter::once(json!(prepared.source.reference))
                .chain(prepared.feature_sources.iter().map(
                    |source| json!({"alias":source.alias,"reference":source.source.reference}),
                ))
                .collect(),
            created_tables: vec![],
            updated_table_versions: vec![],
            preprocessing_manifest: json!(prepared.preprocessing),
            context: experiment_context,
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
    for (index, note) in unchecked {
        note_unchecked_preprocessing(&repo, &experiment.id, index, Some(note));
    }
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

/// The D10 refusal of a private experiment that would write into the project database.
#[cfg(feature = "training")]
pub(crate) const PRIVATE_TABLE_WRITES: &str = "This experiment trains on private (user-scoped) data, so it cannot write tables into the shared project database; set tables.create_tables to false or train from a user-scoped main table.";

/// Where a started experiment lives: its ledger and that ledger's scope, its stream, and the
/// `ledger_scope` marker its context records.
#[cfg(feature = "training")]
struct StartLedger {
    repo: flow_like_ml_runtime::TrainingRepository,
    scope: super::lifecycle::LedgerScope,
    stream: flow_like_ml_runtime::StreamKey,
    marker: Option<Value>,
}

#[cfg(feature = "training")]
impl StartLedger {
    /// A learning cycle's stream, in the ledger that holds the cycle, which may be the legacy
    /// App ledger; a stream without a cycle uses the ledger its project string names. A cycle's
    /// experiment copies its project's marker, which projects created before markers lack.
    fn for_stream(
        context: &ExecutionContext,
        request: &AutoTrainRequest,
        background: bool,
        stream: flow_like_ml_runtime::StreamKey,
    ) -> Result<Self> {
        let (repo, project, scope) = match &request.learning_cycle_id {
            Some(cycle_id) => {
                let ledger = cycle_ledger(context, cycle_id, &stream)?;
                (ledger.repo, ledger.project, ledger.scope)
            }
            None => {
                let scope = super::lifecycle::LedgerScope::of_project(&stream.project_id);
                let (repo, project) = super::lifecycle::scoped_repository(context, scope)?;
                (repo, project, scope)
            }
        };
        let scoped = super::lifecycle::InspectionContext {
            stream_id: stream.stream_id.clone(),
            spec: request.dataset.spec.clone(),
        }
        .key(project)?;
        if scoped != stream {
            return Err(anyhow!(
                "Training stream belongs to another project or inspection contract"
            ));
        }
        let marker = match &request.learning_cycle_id {
            Some(cycle_id) => cycle_marker(&repo, cycle_id, background, &scoped)?,
            None => Some(flow_like_types::json::json!(scope)),
        };
        Ok(Self {
            repo,
            scope,
            stream: scoped,
            marker,
        })
    }

    /// A new `auto` stream in the ledger of `scope`, which the experiment records as its marker.
    fn minted(
        context: &ExecutionContext,
        scope: super::lifecycle::LedgerScope,
        spec: &flow_like_ml_core::InspectionSpec,
    ) -> Result<Self> {
        let (repo, project) = super::lifecycle::scoped_repository(context, scope)?;
        let stream = super::lifecycle::InspectionContext {
            stream_id: scope.new_stream_id(super::lifecycle::StreamKind::Auto),
            spec: spec.clone(),
        }
        .key(project)?;
        Ok(Self {
            repo,
            scope,
            stream,
            marker: Some(flow_like_types::json::json!(scope)),
        })
    }
}

/// A learning cycle attaches its experiment before dispatch and must own `stream`. Its
/// experiment copies the project's marker, which projects created before markers lack.
#[cfg(feature = "training")]
fn cycle_marker(
    repo: &flow_like_ml_runtime::TrainingRepository,
    cycle_id: &str,
    background: bool,
    stream: &flow_like_ml_runtime::StreamKey,
) -> Result<Option<Value>> {
    if background {
        return Err(anyhow!(
            "Learning cycles must attach their experiment before dispatch"
        ));
    }
    let cycle = repo.get_learning_cycle(cycle_id)?;
    let learning = repo.get_learning_project(&cycle.project_id)?;
    if learning.request.stream != *stream {
        return Err(anyhow!("Learning cycle does not own this training stream"));
    }
    Ok(learning
        .request
        .source
        .get(super::lifecycle::LEDGER_SCOPE_KEY)
        .cloned())
}

/// The ledger of `stream`'s project that holds learning cycle `cycle_id`: the one its learning
/// project was found in, which may be the legacy App ledger next to a newer App ledger.
#[cfg(feature = "training")]
fn cycle_ledger(
    context: &ExecutionContext,
    cycle_id: &str,
    stream: &flow_like_ml_runtime::StreamKey,
) -> Result<super::lifecycle::LocatedLedger> {
    let located = super::lifecycle::locate_repository(context, |repo, project| {
        if project != stream.project_id {
            return Ok(false);
        }
        Ok(existing(repo.get_learning_cycle(cycle_id))?.is_some())
    })?;
    located.ok_or_else(|| {
        anyhow!(
            "Learning cycle {cycle_id} of stream {} is in no training ledger of project {} on this device",
            stream.stream_id,
            stream.project_id
        )
    })
}

/// App only when every source reads the run's project database; any other source, including
/// one that cannot be classified, keeps the training private.
#[cfg(feature = "training")]
pub(crate) fn source_ledger_scope(
    digests: &super::auto_training_tables::ScopeDigests,
    main: &super::auto_training_tables::PinnedTableSource,
    sides: &[super::feature_engineering::PinnedFeatureSource],
) -> super::lifecycle::LedgerScope {
    use super::auto_training_tables::{SourceScope, pinned_source_locators};
    use super::lifecycle::LedgerScope;
    match digests.combine(&pinned_source_locators(main, sides)) {
        SourceScope::Project => LedgerScope::App,
        SourceScope::User | SourceScope::Unknown => LedgerScope::User,
    }
}

/// D10: private training writes no tables into the project database, neither tables created
/// beside a project main source nor rows merged into a project destination. While the project
/// database has no digest, a database that cannot be classified counts as the project's.
#[cfg(feature = "training")]
pub(crate) async fn ensure_private_table_writes(
    context: &mut ExecutionContext,
    digests: &super::auto_training_tables::ScopeDigests,
    main: &super::auto_training_tables::PinnedTableSource,
    policy: &super::auto_training_tables::TableWritePolicy,
) -> Result<()> {
    use super::auto_training_tables::{SourceScope, connection_scope};
    let project_like = |scope: SourceScope| match scope {
        SourceScope::Project => true,
        SourceScope::Unknown => digests.project.is_none(),
        SourceScope::User => false,
    };
    let main_scope = digests.classify(main.locator.as_ref());
    let beside_main = policy.create_tables && project_like(main_scope);
    let into_destination = match &policy.allowed_destination {
        Some(destination) => project_like(connection_scope(context, destination, digests).await?),
        None => false,
    };
    if beside_main || into_destination {
        return Err(anyhow!(PRIVATE_TABLE_WRITES));
    }
    Ok(())
}

/// D11: private training fine-tunes only sources of its own ledger. The source a request names
/// must be found in the App ledger chain and is adopted; the extractor of its embeddings and the
/// newest App `burn` sources, which the consultant may name, are adopted when present. Adoption
/// keeps each id and blob, rewrites the project and repeats harmlessly.
#[cfg(feature = "training")]
pub(crate) async fn adopt_pretrained_sources(
    context: &ExecutionContext,
    repo: &flow_like_ml_runtime::TrainingRepository,
    project: &str,
    named: Option<&str>,
    extractor: Option<&str>,
) -> Result<()> {
    if let Some(source_id) = named {
        adopt_named_source(context, repo, project, source_id)?;
    }
    if let Some(source_id) = extractor {
        adopt_extractor_source(context, repo, project, source_id)?;
    }
    adopt_consultant_sources(context, repo, project).await
}

#[cfg(feature = "training")]
fn adopt_named_source(
    context: &ExecutionContext,
    repo: &flow_like_ml_runtime::TrainingRepository,
    project: &str,
    source_id: &str,
) -> Result<()> {
    let Some(ledger) = locate_pretrained_source(context, source_id)? else {
        return Err(anyhow!(
            "Pretrained source {source_id} is in neither this app's training ledger nor your private one, so private training cannot fine-tune from it"
        ));
    };
    if ledger.scope == super::lifecycle::LedgerScope::App {
        repo.adopt_pretrained_source(&ledger.repo, source_id, project)?;
    }
    Ok(())
}

/// Embeddings may name an extractor that only exists as published files, which is no error.
#[cfg(feature = "training")]
fn adopt_extractor_source(
    context: &ExecutionContext,
    repo: &flow_like_ml_runtime::TrainingRepository,
    project: &str,
    source_id: &str,
) -> Result<()> {
    let Some(ledger) = locate_pretrained_source(context, source_id)? else {
        return Ok(());
    };
    if ledger.scope == super::lifecycle::LedgerScope::App
        && let Err(error) = repo.adopt_pretrained_source(&ledger.repo, source_id, project)
    {
        tracing::warn!(source_id, %error, "The feature extractor of private training was not adopted into its ledger");
    }
    Ok(())
}

/// The newest App `burn` sources, the ones the consultant may name.
#[cfg(feature = "training")]
async fn adopt_consultant_sources(
    context: &ExecutionContext,
    repo: &flow_like_ml_runtime::TrainingRepository,
    project: &str,
) -> Result<()> {
    use super::lifecycle::{LedgerLocation, LedgerScope};
    let app = LedgerLocation::new(context, LedgerScope::App)?;
    let (repo, project) = (repo.clone(), project.to_owned());
    tokio::task::spawn_blocking(move || -> Result<()> {
        let Some((app, app_project)) = app.open_existing()? else {
            return Ok(());
        };
        for source in app.list_pretrained_sources_for_engine(&app_project, 16, "burn")? {
            if let Err(error) = repo.adopt_pretrained_source(&app, &source.id, &project) {
                tracing::warn!(source_id = %source.id, %error, "A pretrained source the consultant can name was not adopted into the private ledger");
            }
        }
        Ok(())
    })
    .await?
}

/// The first ledger of the chain that holds `source_id` under its own project string.
#[cfg(feature = "training")]
fn locate_pretrained_source(
    context: &ExecutionContext,
    source_id: &str,
) -> Result<Option<super::lifecycle::LocatedLedger>> {
    super::lifecycle::locate_repository(context, |ledger, project| {
        Ok(existing(ledger.get_pretrained_source(source_id))?
            .is_some_and(|source| source.project_id == project))
    })
}

/// Explicit, pretrained, classical and generated candidates, validated before anything is
/// written to the ledger, and by index the fine-tunes whose input normalization went unchecked.
#[cfg(feature = "training")]
fn start_candidates(
    repo: &flow_like_ml_runtime::TrainingRepository,
    project_id: &str,
    spec: &flow_like_ml_core::InspectionSpec,
    request: &AutoTrainRequest,
) -> Result<(
    Vec<flow_like_ml_runtime::TrainingRequest>,
    Vec<(usize, Value)>,
)> {
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
    let mut unchecked = validate_burn_candidates(
        repo,
        project_id,
        spec,
        &candidates,
        request.budget.worker_limits.memory_budget_bytes,
    )?;
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
    unchecked.retain(|(index, _)| *index < candidates.len());
    Ok((candidates, unchecked))
}

/// Burn candidates must fit the task and, when fine-tuned, their source. Returns by index the
/// fine-tunes whose input normalization could not be compared with their source's.
#[cfg(feature = "training")]
fn validate_burn_candidates(
    repo: &flow_like_ml_runtime::TrainingRepository,
    project_id: &str,
    spec: &flow_like_ml_core::InspectionSpec,
    candidates: &[flow_like_ml_runtime::TrainingRequest],
    worker_memory_bytes: u64,
) -> Result<Vec<(usize, Value)>> {
    let mut unchecked = Vec::new();
    let burn = candidates
        .iter()
        .enumerate()
        .filter(|(_, candidate)| candidate.engine == "burn");
    for (index, candidate) in burn {
        flow_like_ml_burn::validate_inspection_config(spec, &burn_candidate_config(candidate)?)?;
        if let Some(note) = validate_candidate_pretrained(
            repo,
            project_id,
            candidate,
            &spec.labels,
            worker_memory_bytes,
        )? {
            unchecked.push((index, note));
        }
    }
    Ok(unchecked)
}

#[cfg(feature = "training")]
fn burn_candidate_config(
    candidate: &flow_like_ml_runtime::TrainingRequest,
) -> Result<flow_like_ml_burn::TrainingConfig> {
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
    Ok(config)
}

/// Records a fine-tune whose input normalization could not be compared with its source's.
#[cfg(feature = "training")]
fn note_unchecked_preprocessing(
    repo: &flow_like_ml_runtime::TrainingRepository,
    id: &str,
    candidate_index: usize,
    note: Option<Value>,
) {
    let Some(mut note) = note else {
        return;
    };
    note["candidate_index"] = flow_like_types::json::json!(candidate_index);
    note_experiment(repo, id, "pretrained_preprocessing_unchecked", note);
}

/// Request rules the ledger enforces only after every sample is recorded, checked before any
/// table is read.
#[cfg(feature = "training")]
fn validate_request_rules(request: &AutoTrainRequest) -> Result<()> {
    validate_budget(&request.budget)?;
    if let Some(goals) = &request.goals {
        validate_goal_rules(goals)?;
    }
    validate_candidate_requests(&request.initial_candidates)
}

/// The split as the ledger will record it, checked against the rules the ledger applies only
/// after recording, with the extractor its embeddings name and the bytes it is charged.
#[cfg(feature = "training")]
struct PreparedSplit {
    prepared: super::auto_training_tables::PreparedTableDataset,
    snapshot: flow_like_ml_runtime::DatasetSnapshot,
    extractor: Option<flow_like_ml_runtime::engines::FeatureExtractor>,
    dataset_bytes: u64,
}

#[cfg(feature = "training")]
fn prepare_split(
    prepared: super::auto_training_tables::PreparedTableDataset,
    stream: flow_like_ml_runtime::StreamKey,
    split: flow_like_ml_runtime::SplitPolicy,
    named: Option<&super::lifecycle::FeatureExtractorRef>,
) -> Result<PreparedSplit> {
    let snapshot = snapshot_from_table(&prepared, stream, split)?;
    ensure_labels_available(&snapshot)?;
    ensure_training_ready(&prepared)?;
    let extractor = training_extractor(&prepared.preprocessing, &prepared.raw_rows, named)?;
    let dataset_bytes = dataset_charge(&snapshot, &prepared.spec.task)?;
    Ok(PreparedSplit {
        prepared,
        snapshot,
        extractor,
        dataset_bytes,
    })
}

/// The ledger's experiment budget rules.
#[cfg(feature = "training")]
fn validate_budget(budget: &ExperimentBudget) -> Result<()> {
    let limits = &budget.worker_limits;
    let rules = [
        (
            (1..=10_000).contains(&budget.maximum_trials),
            "maximum_trials must be 1 to 10,000",
        ),
        (
            (1..=budget.maximum_trials).contains(&budget.maximum_parallel_trials),
            "maximum_parallel_trials must be 1 to maximum_trials",
        ),
        (
            budget.maximum_wall_time_ms > 0
                && budget.maximum_training_time_ms > 0
                && budget.maximum_dataset_bytes > 0
                && budget.maximum_artifact_bytes > 0,
            "wall time, training time, dataset and artifact budgets must be positive",
        ),
        (
            limits.maximum_duration_ms <= budget.maximum_training_time_ms,
            "worker_limits.maximum_duration_ms, the time reserved per trial, must not exceed maximum_training_time_ms",
        ),
        (
            limits.maximum_artifact_bytes <= budget.maximum_artifact_bytes,
            "worker_limits.maximum_artifact_bytes must not exceed maximum_artifact_bytes",
        ),
        (
            limits.memory_budget_bytes > 0
                && limits.maximum_artifact_bytes > 0
                && limits.maximum_checkpoint_bytes > 0
                && limits.maximum_duration_ms > 0,
            "worker_limits memory, artifact, checkpoint and duration limits must be positive",
        ),
        (
            limits.lease_duration_ms >= 300,
            "worker_limits.lease_duration_ms must be at least 300",
        ),
    ];
    if let Some((_, rule)) = rules.iter().find(|(holds, _)| !holds) {
        return Err(anyhow!(
            "Invalid experiment budget: {rule}; the budget is {}",
            flow_like_types::json::json!(budget)
        ));
    }
    Ok(())
}

/// The ledger's goal rules that `validate_goals` leaves out.
#[cfg(feature = "training")]
fn validate_goal_rules(goals: &ExperimentGoals) -> Result<()> {
    if goals.primary_metric.trim().is_empty() || goals.minimum_audited_samples == 0 {
        return Err(anyhow!(
            "Goals need a primary metric and a positive minimum_audited_samples, not '{}' and {}",
            goals.primary_metric,
            goals.minimum_audited_samples
        ));
    }
    if let Some(bound) = goals.bounds.iter().find(|bound| !valid_bound(bound)) {
        return Err(anyhow!(
            "Goal bound '{}' needs a finite minimum or maximum, with the minimum at most the maximum; it has {:?} and {:?}",
            bound.name,
            bound.minimum,
            bound.maximum
        ));
    }
    Ok(())
}

#[cfg(feature = "training")]
fn valid_bound(bound: &flow_like_ml_runtime::MetricBound) -> bool {
    let finite = |value: Option<f64>| value.is_none_or(f64::is_finite);
    !bound.name.trim().is_empty()
        && (bound.minimum.is_some() || bound.maximum.is_some())
        && finite(bound.minimum)
        && finite(bound.maximum)
        && !bound
            .minimum
            .zip(bound.maximum)
            .is_some_and(|(minimum, maximum)| minimum > maximum)
}

#[cfg(feature = "training")]
fn validate_candidate_requests(candidates: &[flow_like_ml_runtime::TrainingRequest]) -> Result<()> {
    if let Some(index) = candidates.iter().position(|candidate| {
        candidate.engine.trim().is_empty()
            || !candidate.recipe.is_object()
            || !candidate.compute.is_object()
    }) {
        return Err(anyhow!(
            "Initial candidate {index} needs an engine name and object-valued recipe and compute"
        ));
    }
    Ok(())
}

/// Labels the dataset's as-of time cannot see would be refused once every sample is recorded.
#[cfg(feature = "training")]
fn ensure_labels_available(snapshot: &flow_like_ml_runtime::DatasetSnapshot) -> Result<()> {
    if let Some(sample) = snapshot
        .train
        .iter()
        .chain(&snapshot.validation)
        .chain(&snapshot.test)
        .find(|sample| sample.label_available_at_ms > snapshot.as_of_ms)
    {
        return Err(anyhow!(
            "The label of row {} becomes available at {} ms, after the dataset was prepared at {} ms; exclude the row until its label is available",
            sample.id,
            sample.label_available_at_ms,
            snapshot.as_of_ms
        ));
    }
    Ok(())
}

#[cfg(feature = "training")]
fn is_anomaly_task(task: &TaskKind) -> bool {
    matches!(
        task,
        TaskKind::SensorAnomaly | TaskKind::VisualAnomaly | TaskKind::SequenceAutoencoder
    )
}

/// The ledger's readiness rule for the worker view, which keeps only normal examples of anomaly
/// tasks: enough training examples, normal examples to validate on, and each label's support.
#[cfg(feature = "training")]
fn ensure_training_ready(
    prepared: &super::auto_training_tables::PreparedTableDataset,
) -> Result<()> {
    ensure_enough_examples(prepared)?;
    let spec = &prepared.spec;
    if is_anomaly_task(&spec.task)
        || spec.labels.is_empty()
        || matches!(
            spec.task,
            TaskKind::SensorRegression | TaskKind::SequenceForecast
        )
    {
        return Ok(());
    }
    ensure_label_support(prepared)
}

#[cfg(feature = "training")]
fn ensure_enough_examples(
    prepared: &super::auto_training_tables::PreparedTableDataset,
) -> Result<()> {
    let spec = &prepared.spec;
    let normals = |indices: &[usize]| {
        indices
            .iter()
            .filter(|&&index| {
                matches!(
                    prepared.samples[index].annotation,
                    flow_like_ml_core::Annotation::Anomaly { is_anomaly: false }
                )
            })
            .count()
    };
    let (train, required) = if is_anomaly_task(&spec.task) {
        let (train, validation) = (
            normals(&prepared.split.train),
            normals(&prepared.split.validation),
        );
        if train == 0 || validation == 0 {
            return Err(anyhow!(
                "Anomaly training needs normal examples in training and validation; the split has {train} and {validation}"
            ));
        }
        (
            train,
            spec.minimum_examples.max(spec.minimum_examples_per_class),
        )
    } else {
        (prepared.split.train.len(), spec.minimum_examples)
    };
    if train < required {
        return Err(anyhow!(
            "The training partition has {train} usable examples but the specification requires {required} (minimum_examples, and minimum_examples_per_class for anomaly tasks); add rows or lower the minimum"
        ));
    }
    Ok(())
}

#[cfg(feature = "training")]
fn ensure_label_support(
    prepared: &super::auto_training_tables::PreparedTableDataset,
) -> Result<()> {
    let spec = &prepared.spec;
    let mut support = vec![0usize; spec.labels.len()];
    for &index in &prepared.split.train {
        for class in annotation_classes(&prepared.samples[index].annotation) {
            if let Some(count) = support.get_mut(class as usize) {
                *count += 1;
            }
        }
    }
    if let Some((label, count)) = spec
        .labels
        .iter()
        .zip(support)
        .find(|(_, count)| *count < spec.minimum_examples_per_class)
    {
        return Err(anyhow!(
            "Label '{label}' has {count} training examples, below minimum_examples_per_class ({}); add examples of it or lower the minimum",
            spec.minimum_examples_per_class
        ));
    }
    Ok(())
}

/// Each class an annotation shows, once, as the ledger counts a label's support.
#[cfg(feature = "training")]
fn annotation_classes(
    annotation: &flow_like_ml_core::Annotation,
) -> std::collections::BTreeSet<u32> {
    use flow_like_ml_core::Annotation;
    match annotation {
        Annotation::Class { class_id } => std::iter::once(*class_id).collect(),
        Annotation::Boxes { boxes } => boxes.iter().map(|value| value.class_id).collect(),
        Annotation::Mask { classes, .. } => classes.iter().copied().collect(),
        Annotation::InstanceMasks { instances } => instances
            .iter()
            .map(|instance| instance.bounds.class_id)
            .collect(),
        _ => Default::default(),
    }
}

/// The ledger keeps an example in an anomaly task's worker view only when it is labelled normal.
#[cfg(feature = "training")]
fn labelled_normal(sample: &flow_like_ml_runtime::TrainingSample) -> bool {
    let payload = sample.payload.get("sample").unwrap_or(&sample.payload);
    payload
        .pointer("/annotation/is_anomaly")
        .and_then(Value::as_bool)
        == Some(false)
}

/// Bytes the ledger charges to register a split: the split plus its worker view, which drops
/// the final test partition and, for anomaly tasks, every example not labelled normal. Both are
/// stored with a UUID, unless the split already has an ID, and a hex SHA-256 digest.
#[cfg(feature = "training")]
fn dataset_charge(
    snapshot: &flow_like_ml_runtime::DatasetSnapshot,
    task: &TaskKind,
) -> Result<u64> {
    use super::auto_training_tables::serialized_json_size as size;
    // JSON string lengths, quotes included.
    const UUID: usize = 38;
    const DIGEST: usize = 66;
    let id = if snapshot.id.is_empty() {
        UUID
    } else {
        size(&snapshot.id)?
    };
    let shared = size(snapshot)? - size(&snapshot.id)? - size(&snapshot.digest)?;
    let view = shared - size(&snapshot.test)? + "[]".len() + UUID + DIGEST
        - dropped_example_bytes(snapshot, task)?;
    Ok((shared + id + DIGEST + view) as u64)
}

/// The training and validation examples an anomaly task's worker view drops, with their commas.
#[cfg(feature = "training")]
fn dropped_example_bytes(
    snapshot: &flow_like_ml_runtime::DatasetSnapshot,
    task: &TaskKind,
) -> Result<usize> {
    if !is_anomaly_task(task) {
        return Ok(0);
    }
    snapshot
        .train
        .iter()
        .chain(&snapshot.validation)
        .filter(|sample| !labelled_normal(sample))
        .map(|sample| -> Result<usize> {
            Ok(super::auto_training_tables::serialized_json_size(sample)? + ",".len())
        })
        .sum()
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
/// node future the executor dropped, stops between sample batches; a future dropped after the
/// experiment exists, before the caller can guard it, cancels that experiment.
#[cfg(feature = "training")]
async fn create_started_experiment(
    token: Option<flow_like_types::tokio_util::sync::CancellationToken>,
    repo: &flow_like_ml_runtime::TrainingRepository,
    stream: &flow_like_ml_runtime::StreamKey,
    snapshot: flow_like_ml_runtime::DatasetSnapshot,
    request: flow_like_ml_runtime::experiment::ExperimentRequest,
) -> Result<Experiment> {
    use std::sync::{Arc, Mutex, PoisonError};
    let stop = super::cancellation::CancelFlag::default();
    let created = Arc::new(Mutex::new(None::<String>));
    let guard = super::cancellation::CancelOnDrop::new({
        let (stop, created, repo) = (stop.clone(), created.clone(), repo.clone());
        move || {
            stop.cancel();
            let id = created
                .lock()
                .unwrap_or_else(PoisonError::into_inner)
                .take();
            if let Some(id) = id {
                let _ = repo.cancel_experiment(&id, flow_like_ml_runtime::now_ms());
            }
        }
    });
    let (repo, stream) = (repo.clone(), stream.clone());
    let work = tokio::task::spawn_blocking(move || {
        let cancelled =
            || stop.is_cancelled() || token.as_ref().is_some_and(|token| token.is_cancelled());
        let publish =
            |id: &str| *created.lock().unwrap_or_else(PoisonError::into_inner) = Some(id.into());
        record_and_create(&repo, &stream, snapshot, request, &cancelled, &publish)
    });
    match work.await {
        Ok(outcome) => {
            guard.disarm();
            outcome
        }
        // The guard cancels an experiment the work created before it panicked.
        Err(failed) => Err(anyhow!(
            "Recording the training samples stopped unexpectedly: {failed}"
        )),
    }
}

/// `created` receives the experiment ID as soon as the experiment exists.
#[cfg(feature = "training")]
fn record_and_create(
    repo: &flow_like_ml_runtime::TrainingRepository,
    stream: &flow_like_ml_runtime::StreamKey,
    snapshot: flow_like_ml_runtime::DatasetSnapshot,
    request: flow_like_ml_runtime::experiment::ExperimentRequest,
    cancelled: &dyn Fn() -> bool,
    created: &dyn Fn(&str),
) -> Result<Experiment> {
    const SAMPLE_BATCH: usize = 256;
    let stopped = || anyhow!("Auto training start was cancelled before its experiment was created");
    for partition in [&snapshot.train, &snapshot.validation, &snapshot.test] {
        for batch in partition.chunks(SAMPLE_BATCH) {
            if cancelled() {
                return Err(stopped());
            }
            repo.record_prepared_samples(stream, batch)?;
        }
    }
    if cancelled() {
        return Err(stopped());
    }
    let experiment =
        repo.create_experiment_with_snapshot(snapshot, request, flow_like_ml_runtime::now_ms())?;
    created(&experiment.id);
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

/// A fine-tune's source must be authorized and fit its recipe. The note reports a recipe whose
/// input normalization could not be compared with the source's.
#[cfg(feature = "training")]
fn validate_candidate_pretrained(
    repo: &flow_like_ml_runtime::TrainingRepository,
    project_id: &str,
    candidate: &flow_like_ml_runtime::TrainingRequest,
    labels: &[String],
    worker_memory_bytes: u64,
) -> Result<Option<Value>> {
    let recipe: flow_like_ml_runtime::engines::BurnEngineRecipe =
        flow_like_types::json::from_value(candidate.recipe.clone())?;
    let Some(pretrained) = &recipe.pretrained else {
        return Ok(None);
    };
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
    unchecked_pretrained_preprocessing(&source, &recipe.preprocessing)
}

/// Normalization is only compared when the recipe declares image steps. Without them, the steps
/// the source's weights expect are reported, since differently prepared inputs degrade the
/// fine-tune without failing it.
#[cfg(feature = "training")]
fn unchecked_pretrained_preprocessing(
    source: &flow_like_ml_runtime::PretrainedSource,
    declared: &[flow_like_ml_core::PreprocessingStep],
) -> Result<Option<Value>> {
    use flow_like_ml_core::PreprocessingStep;
    if declared.iter().any(|step| {
        matches!(
            step,
            PreprocessingStep::Resize { .. } | PreprocessingStep::Normalize { .. }
        )
    }) {
        return Ok(None);
    }
    let expected =
        flow_like_ml_runtime::engines::FeatureExtractor::from_source(source)?.image_steps;
    if expected.is_empty() {
        return Ok(None);
    }
    Ok(Some(flow_like_types::json::json!({
        "source_id": source.id,
        "recommended_preprocessing": expected,
        "reason": "the candidate declares no image preprocessing, so its input normalization was not compared with the source's; prepare inputs with the recommended steps",
    })))
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
    let unchecked = validate_candidate_pretrained(
        repo,
        &experiment.request.stream.project_id,
        &candidate,
        &spec.labels,
        experiment.request.budget.worker_limits.memory_budget_bytes,
    )?;
    let appended =
        repo.append_experiment_candidates(id, vec![candidate], flow_like_ml_runtime::now_ms())?;
    let index = appended.request.candidates.len().saturating_sub(1);
    note_unchecked_preprocessing(repo, id, index, unchecked);
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
    ensure_accepts_derived(&experiment, None)?;
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
    let (prepared, derived, dataset_bytes) = {
        let repo = repo.clone();
        let snapshot_id = experiment.request.snapshot_id.clone();
        let maximum_bytes = experiment.request.budget.maximum_dataset_bytes;
        let derived_id = derived_id.clone();
        tokio::task::spawn_blocking(move || -> Result<_> {
            let derived =
                derived_snapshot(&repo, &snapshot_id, maximum_bytes, &prepared, derived_id)?;
            let bytes = dataset_charge(&derived, &prepared.spec.task)?;
            Ok((prepared, derived, bytes))
        })
        .await??
    };
    // The ledger refuses these only after the tables exist, so they are checked before writing.
    ensure_accepts_derived(&repo.get_experiment(id)?, Some(dataset_bytes))?;
    let tables = super::auto_training_tables::materialize_prepared_tables(
        context,
        &prepared,
        &format!("{id}-{derived_id}"),
        &policy,
    )
    .await?;
    let references: Vec<Value> = tables.into_iter().map(|table| json!(table)).collect();
    let manifest = json!(prepared.preprocessing);
    let (repo, id) = (repo.clone(), id.to_owned());
    tokio::task::spawn_blocking(move || {
        register_derived(&repo, &id, derived, manifest, candidates, references)
    })
    .await?
}

/// What the ledger checks when it registers a derived dataset: the experiment still runs, has
/// room for a candidate and wall time left, and, once its size is known, budget for the dataset.
#[cfg(feature = "training")]
fn ensure_accepts_derived(experiment: &Experiment, dataset_bytes: Option<u64>) -> Result<()> {
    let (id, budget) = (&experiment.id, &experiment.request.budget);
    if experiment.status != ExperimentStatus::Running || experiment.selected_trial_id.is_some() {
        return Err(anyhow!(
            "Experiment {id} no longer accepts candidates; its status is {:?}",
            experiment.status
        ));
    }
    if experiment.request.candidates.len() >= budget.maximum_trials {
        return Err(anyhow!(
            "Experiment {id} already holds {} candidates, its maximum_trials budget",
            experiment.request.candidates.len()
        ));
    }
    let elapsed = experiment.wall_elapsed_ms(flow_like_ml_runtime::now_ms());
    if elapsed > budget.maximum_wall_time_ms {
        return Err(anyhow!(
            "Experiment {id} has used {elapsed} ms of its {} ms budget.maximum_wall_time_ms and accepts no more candidates",
            budget.maximum_wall_time_ms
        ));
    }
    let used = experiment.usage.dataset_bytes;
    if let Some(bytes) =
        dataset_bytes.filter(|bytes| used.saturating_add(*bytes) > budget.maximum_dataset_bytes)
    {
        return Err(anyhow!(
            "The derived dataset and its training view need {bytes} bytes, but experiment {id} already uses {used} of its {} byte budget.maximum_dataset_bytes",
            budget.maximum_dataset_bytes
        ));
    }
    Ok(())
}

/// Register a derived dataset with its candidates and list its tables. Tables written for a
/// dataset the ledger then refuses stay listed in created_tables, next to an event naming the
/// refusal, so they can be found and removed.
#[cfg(feature = "training")]
fn register_derived(
    repo: &flow_like_ml_runtime::TrainingRepository,
    id: &str,
    derived: flow_like_ml_runtime::DatasetSnapshot,
    preprocessing_manifest: Value,
    candidates: Vec<flow_like_ml_runtime::TrainingRequest>,
    tables: Vec<Value>,
) -> Result<()> {
    use flow_like_ml_runtime::now_ms;
    let derived_id = derived.id.clone();
    let registered = repo.register_experiment_candidates(
        id,
        derived,
        preprocessing_manifest,
        candidates,
        now_ms(),
    );
    let error = match registered {
        Ok(_) if tables.is_empty() => return Ok(()),
        Ok(_) => {
            repo.update_experiment_tables(id, tables, vec![], now_ms())?;
            return Ok(());
        }
        Err(error) => error,
    };
    if tables.is_empty() {
        return Err(anyhow!(
            "Experiment {id} refused the derived dataset {derived_id}: {error}"
        ));
    }
    note_experiment(
        repo,
        id,
        "derived_dataset_rejected",
        flow_like_types::json::json!({
            "dataset_snapshot_id": derived_id,
            "error": bounded_text(&error.to_string()),
            "tables": tables,
        }),
    );
    if let Err(listed) = repo.update_experiment_tables(id, tables, vec![], now_ms()) {
        tracing::warn!(experiment_id = %id, error = %listed, "Could not list the tables of a refused derived dataset");
    }
    Err(anyhow!(
        "Experiment {id} refused the derived dataset {derived_id}: {error}; its written tables are listed in created_tables"
    ))
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

/// Publishes an audited winner after a state change, then writes its final predictions table.
#[cfg(feature = "training")]
pub async fn materialize_final_predictions(
    context: &mut ExecutionContext,
    id: &str,
) -> Result<AutoTrainingResult> {
    materialize_with(context, id, super::publisher::PublishReason::StateChange).await
}

#[cfg(feature = "training")]
async fn materialize_with(
    context: &mut ExecutionContext,
    id: &str,
    reason: super::publisher::PublishReason,
) -> Result<AutoTrainingResult> {
    let (repo, scope) = experiment_ledger(context, id)?;
    let mut result: AutoTrainingResult = repo.experiment_result(id)?.into();
    if !result.final_test_metrics.is_empty() {
        result = publish_winner(context, &repo, scope, id, reason).await?;
    }
    materialize_predictions_table(context, &repo, id, result).await
}

/// Writes an audited experiment's final predictions table once and returns the result read
/// after it.
#[cfg(feature = "training")]
async fn materialize_predictions_table(
    context: &mut ExecutionContext,
    repo: &flow_like_ml_runtime::TrainingRepository,
    id: &str,
    result: AutoTrainingResult,
) -> Result<AutoTrainingResult> {
    if result.predictions_table.is_some() || result.final_test_metrics.is_empty() {
        return Ok(result);
    }
    match write_predictions_table(context, repo, id, &result).await {
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
                repo,
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

/// Publishes the audited winner of a standalone experiment and returns the result read after
/// the publication events. Publishing never fails the caller; the publisher records failures
/// as experiment events.
#[cfg(feature = "training")]
async fn publish_winner(
    context: &mut ExecutionContext,
    repo: &flow_like_ml_runtime::TrainingRepository,
    scope: super::lifecycle::LedgerScope,
    id: &str,
    reason: super::publisher::PublishReason,
) -> Result<AutoTrainingResult> {
    use super::publisher::{PublishOutcome, ensure_experiment_published};
    if let PublishOutcome::Failed { error } =
        ensure_experiment_published(context, repo, scope, id, reason).await
    {
        context.log_message(
            &format!("The winner of experiment {id} was not published: {error}"),
            flow_like::flow::execution::LogLevel::Warn,
        );
    }
    Ok(repo.experiment_result(id)?.into())
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

/// Get, Cancel and Resume repair a missing publication at most once per run; a step that ends
/// the search and Finalize publish the audited winner they produced.
#[cfg(feature = "training")]
async fn lifecycle(
    context: &mut ExecutionContext,
    id: &str,
    action: LifecycleAction,
) -> Result<AutoTrainingResult> {
    use super::publisher::PublishReason;
    if matches!(action, LifecycleAction::Step) {
        let stepped = run_next_auto_training(context, id).await?;
        if stepped.final_test_metrics.is_empty() {
            return Ok(stepped);
        }
        let (repo, scope) = experiment_ledger(context, id)?;
        return publish_winner(context, &repo, scope, id, PublishReason::StateChange).await;
    }
    let repo = experiment_repository(context, id)?;
    let reason = match action {
        LifecycleAction::Status => PublishReason::SelfRepair,
        LifecycleAction::Step => unreachable!(),
        LifecycleAction::Cancel => {
            repo.cancel_experiment(id, flow_like_ml_runtime::now_ms())?;
            PublishReason::SelfRepair
        }
        LifecycleAction::Resume => {
            resume_saved_experiment(&repo, id)?;
            enqueue_experiment(repo.clone(), id.to_string())?;
            PublishReason::SelfRepair
        }
        LifecycleAction::Finalize => {
            finalize_auto_training(context, id).await?;
            PublishReason::StateChange
        }
    };
    materialize_with(context, id, reason).await
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
                    Err(super::lifecycle::training_unavailable())
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
            "Apply the winning model's saved preprocessing to raw rows and load its durable weights; anomaly models also return the audited calibrated probability and decision. Without the experiment in this device's training ledger, it runs the winner published under storage/models/experiments",
            "predictAutoModel",
            "AI/ML/Auto Training",
        );
        node.set_version(3);
        node
    }
    async fn run(&self, context: &mut ExecutionContext) -> Result<()> {
        #[cfg(feature = "inference")]
        {
            context.deactivate_exec_pin("exec_out").await?;
            let request: PredictAutoModelRequest = context.evaluate_pin("request").await?;
            let result = predict_auto_model_or_published(context, request).await?;
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
            Err(super::lifecycle::training_unavailable())
        }
    }
}

/// Predict raw rows with the experiment's selected model and the preprocessing of the dataset
/// it was trained on, from this device's training ledger only. Anomaly scores also get the
/// decision rule the final audit measured.
#[cfg(feature = "training")]
pub async fn predict_auto_model(
    context: &ExecutionContext,
    mut request: PredictAutoModelRequest,
) -> Result<AutoModelPredictions> {
    check_prediction_batch(&request)?;
    request.compute = request.compute.for_host();
    let repo = experiment_repository(context, &request.experiment_id)?;
    predict_from_ledger(context, &repo, request).await
}

/// Predict Auto Model: the ledger when it holds the experiment under this run's project string,
/// otherwise the winner published under `models/experiments` of the app storage or, after it,
/// of the executing user's directory.
#[cfg(feature = "inference")]
pub(crate) async fn predict_auto_model_or_published(
    context: &mut ExecutionContext,
    mut request: PredictAutoModelRequest,
) -> Result<AutoModelPredictions> {
    check_prediction_batch(&request)?;
    request.compute = request.compute.for_host();
    let id = request.experiment_id.clone();
    let ledger = super::lifecycle::locate_repository_for_inference(context, |repo, project| {
        Ok(existing(repo.get_experiment(&id))?
            .is_some_and(|experiment| experiment.request.stream.project_id == project))
    })?;
    match ledger {
        Some(ledger) => predict_from_ledger(context, &ledger.repo, request).await,
        None => predict_published(context, request).await,
    }
}

#[cfg(feature = "inference")]
fn check_prediction_batch(request: &PredictAutoModelRequest) -> Result<()> {
    if request.rows.len() > 100_000 {
        return Err(anyhow!(
            "Prediction batch of {} rows exceeds 100,000 rows",
            request.rows.len()
        ));
    }
    Ok(())
}

/// The experiment's selected model from the ledger that holds it.
#[cfg(feature = "inference")]
async fn predict_from_ledger(
    context: &ExecutionContext,
    repo: &flow_like_ml_runtime::TrainingRepository,
    request: PredictAutoModelRequest,
) -> Result<AutoModelPredictions> {
    use super::published::AnomalyCalibration;
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
    let calibration = threshold.map(|threshold| AnomalyCalibration {
        scale: result
            .validation_metrics
            .get("anomaly_scale")
            .copied()
            .unwrap_or(1.0),
        threshold,
    });
    let predictor = cached_auto_model(context, repo, &artifact, &request.compute).await?;
    let preprocessing = result.preprocessing_manifest;
    predict_rows(
        predictor,
        Some(preprocessing),
        calibration,
        artifact.id,
        request,
    )
    .await
}

/// The winner published for the experiment, with the calibration its manifest records.
#[cfg(feature = "inference")]
async fn predict_published(
    context: &mut ExecutionContext,
    request: PredictAutoModelRequest,
) -> Result<AutoModelPredictions> {
    use super::published::{
        PublishedModelSource, published_predictor, published_preprocessing, resolve_model,
    };
    let source = PublishedModelSource::Experiment {
        experiment_id: request.experiment_id.clone(),
        storage: None,
    };
    let model = resolve_model(context, &source).await?;
    let predictor = published_predictor(context, &model, &request.compute).await?;
    let preprocessing = published_preprocessing(context, &model).await?;
    predict_rows(
        predictor,
        preprocessing,
        model.manifest.calibration,
        model.manifest.artifact_id().to_owned(),
        request,
    )
    .await
}

/// Runs the shared table-row prediction off the async executor.
#[cfg(feature = "inference")]
async fn predict_rows(
    predictor: super::lifecycle::CachedPredictor,
    preprocessing: Option<Value>,
    calibration: Option<super::published::AnomalyCalibration>,
    model_artifact_id: String,
    request: PredictAutoModelRequest,
) -> Result<AutoModelPredictions> {
    let PredictAutoModelRequest {
        experiment_id,
        rows,
        stateful_features,
        feature_state,
        feature_extractor,
        ..
    } = request;
    tokio::task::spawn_blocking(move || -> Result<AutoModelPredictions> {
        let output = super::published::predict_table_rows(
            &predictor,
            preprocessing.as_ref(),
            calibration,
            super::published::TableRowsInput {
                rows,
                feature_state,
                stateful_features,
                feature_extractor,
            },
        )?;
        Ok(AutoModelPredictions {
            experiment_id,
            model_artifact_id,
            predictions: output.predictions,
            feature_state: output.feature_state,
            anomaly: output.anomaly,
        })
    })
    .await?
}

/// The final audit's anomaly rule: probability = score / (score + scale), an anomaly at or
/// above the threshold.
#[cfg(feature = "inference")]
pub(crate) fn anomaly_decisions(
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

/// The extractor that prediction embeddings name; every named extractor must be the same one.
#[cfg(feature = "inference")]
pub(crate) fn requested_extractor(
    preprocessing: &super::auto_training_tables::TablePreprocessing,
    rows: &[Value],
    explicit: Option<&super::lifecycle::FeatureExtractorRef>,
) -> Result<Option<flow_like_ml_runtime::engines::FeatureExtractor>> {
    named_extractor(preprocessing, rows, explicit, |first, other| {
        anyhow!(
            "Embeddings come from different feature extractors ({first} and {other}); predict each extractor's rows separately"
        )
    })
}

/// The extractor that training embeddings name; every named extractor must be the same one.
#[cfg(feature = "training")]
fn training_extractor(
    preprocessing: &super::auto_training_tables::TablePreprocessing,
    rows: &[Value],
    explicit: Option<&super::lifecycle::FeatureExtractorRef>,
) -> Result<Option<flow_like_ml_runtime::engines::FeatureExtractor>> {
    named_extractor(preprocessing, rows, explicit, |first, other| {
        anyhow!(
            "Training embeddings come from different feature extractors ({first} and {other}); train one model per extractor"
        )
    })
}

/// Embeddings name their extractor on the request or as `feature_extractor` inside each row's
/// tensor value. `mixed` describes two different extractors by their source IDs.
#[cfg(feature = "inference")]
fn named_extractor(
    preprocessing: &super::auto_training_tables::TablePreprocessing,
    rows: &[Value],
    explicit: Option<&super::lifecycle::FeatureExtractorRef>,
    mixed: impl FnOnce(&str, &str) -> flow_like_types::Error,
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
        return Err(mixed(&first.source_id, &other.source_id));
    }
    Ok(Some(first))
}

/// A model that recorded the extractor of its training embeddings cannot check embeddings that
/// name none, so it refuses them instead of scoring a possibly different feature space.
#[cfg(feature = "inference")]
pub(crate) fn ensure_extractor_named(
    artifact_id: &str,
    manifest: &Value,
    named: Option<&flow_like_ml_runtime::engines::FeatureExtractor>,
) -> Result<()> {
    let Some(recorded) = manifest
        .get("feature_extractor")
        .filter(|value| !value.is_null())
    else {
        return Ok(());
    };
    if named.is_some() {
        return Ok(());
    }
    let recorded = super::lifecycle::runtime_extractor(recorded)?;
    Err(anyhow!(
        "Model {artifact_id} was fitted on embeddings from feature extractor {} ({}); pass that extractor as feature_extractor, or inside each row's tensor value, to predict with it",
        recorded.source_id,
        recorded.blob_sha256
    ))
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

#[cfg(feature = "inference")]
#[derive(Clone, Default)]
struct AutoModelCache {
    models:
        std::sync::Arc<tokio::sync::Mutex<Vec<(String, super::lifecycle::CachedPredictor, u64)>>>,
}

#[cfg(feature = "inference")]
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
#[cfg(feature = "inference")]
async fn cached_auto_model(
    context: &ExecutionContext,
    repo: &flow_like_ml_runtime::TrainingRepository,
    artifact: &flow_like_ml_runtime::ModelArtifact,
    compute: &ComputeConfig,
) -> Result<super::lifecycle::CachedPredictor> {
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
    let (repo, scope) = experiment_ledger(context, &request.experiment_id)?;
    let exported = super::publisher::export_auto_model_files(
        context,
        &repo,
        scope,
        &request.experiment_id,
        &request.weights,
        &request.manifest,
    )
    .await?;
    Ok(ExportAutoModelResult {
        model_artifact_id: exported.artifact_id,
        weights: request.weights,
        manifest: request.manifest,
        sha256: exported.sha256,
        bytes: exported.bytes,
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
    use crate::inspection::lifecycle::DrainReport;
    use flow_like_ml_runtime::{
        ExperimentStatus, ExperimentTrialStatus, JobStatus, TrainingRepository,
    };
    use flow_like_types::{Result, anyhow, json::json};
    use std::{
        collections::HashMap,
        path::{Path, PathBuf},
        sync::{
            LazyLock, Mutex, MutexGuard, PoisonError,
            mpsc::{Receiver, SyncSender, TrySendError, sync_channel},
        },
        time::{Duration, Instant},
    };

    const CAPACITY: usize = 32;
    /// Waiting for a busy device or another controller is retried this many times in total.
    pub(super) const ATTEMPTS: u32 = 5;

    pub(super) struct Work {
        repo: TrainingRepository,
        id: String,
        pub(super) key: String,
        /// The ledger's canonical path, which drains match by prefix.
        ledger: PathBuf,
        attempt: u32,
    }

    impl Work {
        pub(super) fn new(repo: TrainingRepository, id: String, attempt: u32) -> Result<Self> {
            let key = flow_like_ml_core::content_digest(&flow_like_types::json::to_vec(&(
                repo.path(),
                &id,
            ))?);
            let ledger = repo
                .path()
                .canonicalize()
                .unwrap_or_else(|_| repo.path().to_path_buf());
            Ok(Self {
                repo,
                id,
                key,
                ledger,
                attempt,
            })
        }
    }

    /// An experiment queued for, or being driven by, the background worker.
    #[derive(Clone)]
    pub(super) struct Queued {
        repo: TrainingRepository,
        ledger: PathBuf,
        experiment_id: String,
    }

    impl Queued {
        pub(super) fn of(work: &Work) -> Self {
            Self {
                repo: work.repo.clone(),
                ledger: work.ledger.clone(),
                experiment_id: work.id.clone(),
            }
        }
    }

    /// Queued and driven experiments by work key, and the ledger prefixes of apps being drained,
    /// under which nothing is queued or driven.
    #[derive(Default)]
    pub(super) struct Queue {
        pub(super) entries: HashMap<String, Queued>,
        draining: Vec<PathBuf>,
    }

    impl Queue {
        fn draining(&self, ledger: &Path) -> bool {
            self.draining
                .iter()
                .any(|prefix| ledger.starts_with(prefix))
        }
    }

    static QUEUE: LazyLock<Mutex<Queue>> = LazyLock::new(Default::default);
    static WORKER: LazyLock<Mutex<Option<SyncSender<Work>>>> = LazyLock::new(Default::default);

    pub(super) fn queue() -> MutexGuard<'static, Queue> {
        QUEUE.lock().unwrap_or_else(PoisonError::into_inner)
    }

    pub(super) fn ensure_capacity() -> Result<()> {
        let queued = queue().entries.len();
        if queued >= CAPACITY {
            return Err(anyhow!(
                "The background training queue already holds {queued} experiments; wait for one to finish or start without background training"
            ));
        }
        Ok(())
    }

    pub(super) fn enqueue(repo: TrainingRepository, id: String, attempt: u32) -> Result<()> {
        let work = Work::new(repo.clone(), id.clone(), attempt)?;
        let key = work.key.clone();
        let ahead = {
            let mut queue = queue();
            if queue.draining(&work.ledger) {
                return Err(anyhow!(
                    "Experiment {id} was not queued because its app's training is being stopped for deletion"
                ));
            }
            if queue.entries.contains_key(&key) {
                return Ok(());
            }
            queue.entries.insert(key.clone(), Queued::of(&work));
            let mut worker = WORKER.lock().unwrap_or_else(PoisonError::into_inner);
            let sent = send_or_respawn(&mut *worker, work, || {
                // Work buffered for a stopped thread is gone; its keys must not block Resume.
                queue.entries.retain(|queued_key, _| queued_key == &key);
                spawn_worker()
            });
            drop(worker);
            if let Err(error) = sent {
                queue.entries.remove(&key);
                return Err(anyhow!(
                    "Experiment {id} is saved but was not queued: {error}; Resume Auto Training can queue it again"
                ));
            }
            queue.entries.len() - 1
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
            process(work, drive);
        }
    }

    /// Drive one queued experiment and settle the outcome. Neither a panic while driving nor one
    /// while settling stops the worker, and the queue key is released however the item ends.
    pub(super) fn process(work: Work, drive: impl FnOnce(&TrainingRepository, &str) -> Driven) {
        let _queued = QueuedKey(work.key.clone());
        if undrivable(&work) {
            return;
        }
        let driven =
            std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| drive(&work.repo, &work.id)))
                .unwrap_or_else(|payload| {
                    Driven::Failed(format!(
                        "the background training worker panicked: {}",
                        panic_text(&*payload)
                    ))
                });
        let _ = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| settle(work, driven)));
    }

    /// Work of an app being drained, which the drain cancelled already, and work whose ledger
    /// was deleted are dropped without touching the ledger.
    fn undrivable(work: &Work) -> bool {
        queue().draining(&work.ledger) || !work.repo.path().try_exists().unwrap_or(true)
    }

    /// Releases the queue key however the work item ends, so Resume can queue it again.
    struct QueuedKey(String);
    impl Drop for QueuedKey {
        fn drop(&mut self) {
            queue().entries.remove(&self.0);
        }
    }

    /// Cancels every experiment queued or driven on a ledger under `prefixes` and waits until
    /// their work ends or `deadline` passes. Nothing is queued under the prefixes meanwhile.
    pub(super) fn drain(prefixes: &[PathBuf], deadline: Instant) -> DrainReport {
        let (_draining, targets) = begin_drain(prefixes);
        let mut cancelled = 0;
        for target in targets.values() {
            match target
                .repo
                .cancel_experiment(&target.experiment_id, flow_like_ml_runtime::now_ms())
            {
                Ok(_) => cancelled += 1,
                Err(error) => {
                    tracing::warn!(experiment_id = %target.experiment_id, %error, "A drained experiment could not be cancelled")
                }
            }
        }
        loop {
            let still_running = {
                let queue = queue();
                targets
                    .keys()
                    .filter(|key| queue.entries.contains_key(*key))
                    .count()
            };
            let now = Instant::now();
            if still_running == 0 || now >= deadline {
                return DrainReport {
                    cancelled,
                    still_running,
                };
            }
            std::thread::sleep((deadline - now).min(Duration::from_millis(50)));
        }
    }

    /// Marks `prefixes` as draining until the returned guard drops, and returns the work queued
    /// or driven under them.
    pub(super) fn begin_drain(prefixes: &[PathBuf]) -> (Draining, HashMap<String, Queued>) {
        let mut queue = queue();
        queue.draining.extend(prefixes.iter().cloned());
        let targets = queue
            .entries
            .iter()
            .filter(|(_, entry)| {
                prefixes
                    .iter()
                    .any(|prefix| entry.ledger.starts_with(prefix))
            })
            .map(|(key, entry)| (key.clone(), entry.clone()))
            .collect();
        (Draining(prefixes.to_vec()), targets)
    }

    /// Removes each prefix of its drain once, so an overlapping drain keeps its own.
    pub(super) struct Draining(Vec<PathBuf>);
    impl Drop for Draining {
        fn drop(&mut self) {
            let mut queue = queue();
            for prefix in &self.0 {
                if let Some(index) = queue.draining.iter().position(|kept| kept == prefix) {
                    queue.draining.remove(index);
                }
            }
        }
    }

    pub(super) enum Driven {
        Settled,
        /// Nothing progresses: a trial waits for a device or a reservation, or the ledger is busy.
        Waiting(String),
        /// Another controller or worker drives the experiment.
        Held(String),
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
                unsettled(repo, id, None)
            }
            Ok(_) | Err(flow_like_ml_runtime::Error::Cancelled) => Driven::Settled,
            // Another holder, a lost lease or a busy ledger clears up without intervention.
            Err(
                error @ (flow_like_ml_runtime::Error::Conflict(_)
                | flow_like_ml_runtime::Error::LeaseLost
                | flow_like_ml_runtime::Error::Sql(_)),
            ) => unsettled(repo, id, Some(error.to_string())),
            Err(error) => Driven::Failed(error.to_string()),
        }
    }

    /// Why the controller left an experiment unsettled. Without a scheduled trial it returns only
    /// when another controller holds the experiment, and a scheduled trial whose job holds a live
    /// lease trains under another worker. Anything else, including an error the controller kept
    /// meeting, waits for a device, a reservation or the ledger.
    pub(super) fn unsettled(repo: &TrainingRepository, id: &str, error: Option<String>) -> Driven {
        let trials = match repo.list_experiment_trials(id) {
            Ok(trials) => trials,
            Err(read) => {
                return Driven::Waiting(
                    error.unwrap_or_else(|| format!("its trials could not be read: {read}")),
                );
            }
        };
        let scheduled = trials
            .into_iter()
            .find(|trial| trial.status == ExperimentTrialStatus::Scheduled);
        let Some(trial) = scheduled else {
            return match error {
                Some(error) => Driven::Waiting(error),
                None => Driven::Held("another controller holds this experiment".into()),
            };
        };
        let job = match repo.get_job(&trial.job_id) {
            Ok(job) => job,
            Err(read) => {
                return Driven::Waiting(format!(
                    "trial {} job could not be read: {read}",
                    trial.id
                ));
            }
        };
        let leased = matches!(job.status, JobStatus::Running | JobStatus::CancelRequested)
            && job
                .lease_expires_at_ms
                .is_some_and(|expiry| expiry > flow_like_ml_runtime::now_ms());
        let reason = match (error, job.error) {
            (Some(error), _) => format!("trial {}: {error}", trial.id),
            (None, Some(waiting)) => format!("trial {} is waiting: {waiting}", trial.id),
            (None, None) => format!("trial {} has a {:?} job", trial.id, job.status),
        };
        if leased {
            Driven::Held(reason)
        } else {
            Driven::Waiting(reason)
        }
    }

    fn settle(work: Work, driven: Driven) {
        let Work {
            repo, id, attempt, ..
        } = work;
        match driven {
            Driven::Settled => {}
            Driven::Waiting(reason) | Driven::Held(reason) if attempt + 1 < ATTEMPTS => {
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
            // Another controller or worker drives it, so the experiment is left as it is.
            Driven::Held(reason) => note_experiment(
                &repo,
                &id,
                "background_worker_stopped",
                json!({
                    "attempts": attempt + 1,
                    "reason": bounded_text(&reason),
                    "resume": "Resume Auto Training queues the experiment again",
                }),
            ),
            Driven::Waiting(reason) => {
                note_experiment(
                    &repo,
                    &id,
                    "background_worker_stopped",
                    json!({
                        "attempts": attempt + 1,
                        "reason": bounded_text(&reason),
                        "cancelled": true,
                        "resume": "Resume Auto Training continues it within its remaining budget",
                    }),
                );
                stop(&repo, &id);
            }
            Driven::Failed(error) => {
                note_experiment(
                    &repo,
                    &id,
                    "background_worker_failed",
                    json!({ "error": bounded_text(&error) }),
                );
                stop(&repo, &id);
            }
        }
    }

    /// Nothing drives the experiment any more. Cancelled, it shows that, stops charging wall
    /// time and stays resumable.
    fn stop(repo: &TrainingRepository, id: &str) {
        if let Err(cancel) = repo.cancel_experiment(id, flow_like_ml_runtime::now_ms()) {
            tracing::warn!(experiment_id = %id, error = %cancel, "Could not stop an experiment its background worker gave up on");
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

        /// Work for experiment `id` on a fresh ledger in `directory`.
        fn ledger_work(directory: &std::path::Path, id: &str) -> Work {
            let repo = TrainingRepository::open(directory.join("jobs.sqlite")).unwrap();
            Work::new(repo, id.into(), 0).unwrap()
        }

        #[test]
        fn the_queue_key_is_released_when_work_panics() {
            let directory = tempfile::tempdir().unwrap();
            let work = ledger_work(directory.path(), &format!("test-{}", uuid::Uuid::new_v4()));
            let key = work.key.clone();
            queue().entries.insert(key.clone(), Queued::of(&work));
            let guard = QueuedKey(key.clone());
            let outcome = std::panic::catch_unwind(std::panic::AssertUnwindSafe(move || {
                let _guard = guard;
                panic!("work failed");
            }));
            assert!(outcome.is_err());
            assert!(!queue().entries.contains_key(&key));
        }

        #[test]
        fn draining_refuses_and_skips_only_work_under_its_prefixes() {
            let (drained, kept) = (tempfile::tempdir().unwrap(), tempfile::tempdir().unwrap());
            let prefix = drained.path().canonicalize().unwrap();
            let (inside, outside) = (
                ledger_work(drained.path(), "inside"),
                ledger_work(kept.path(), "outside"),
            );
            let (draining, targets) = begin_drain(std::slice::from_ref(&prefix));
            assert!(targets.is_empty());
            let error = enqueue(inside.repo.clone(), "inside".into(), 0)
                .unwrap_err()
                .to_string();
            assert!(error.contains("being stopped for deletion"), "{error}");
            assert!(!queue().entries.contains_key(&inside.key));
            assert!(undrivable(&inside));
            assert!(!undrivable(&outside));
            let (overlapping, _) = begin_drain(std::slice::from_ref(&prefix));
            drop(draining);
            assert!(undrivable(&inside), "an overlapping drain keeps its prefix");
            drop(overlapping);
            assert!(!undrivable(&inside));
            let deleted = tempfile::tempdir().unwrap();
            let gone = ledger_work(deleted.path(), "gone");
            drop(deleted);
            assert!(undrivable(&gone));
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

    #[test]
    fn auto_train_requests_take_the_extractor_that_feature_nodes_output() {
        for node in [
            AutoTrainTabularNode.get_node(),
            AutoTrainForecastNode.get_node(),
            AutoTrainVisionNode.get_node(),
            AutoTrainAnomalyNode.get_node(),
        ] {
            assert_eq!(node.version, Some(1), "{}", node.name);
            let request = node.get_pin_by_name("request").unwrap();
            let schema: Value =
                flow_like_types::json::from_str(request.schema.as_deref().unwrap()).unwrap();
            assert!(
                schema["properties"].get("feature_extractor").is_some()
                    && schema["$defs"].get("FeatureExtractorRef").is_some(),
                "{}",
                node.name
            );
        }
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
        use crate::inspection::lifecycle::{LEDGER_SCOPE_KEY, LedgerScope, StreamKind};
        use crate::inspection::published::{
            AnomalyCalibration, ExperimentPointer, ModelStorage, PublishedModelManifest,
            PublishedPreprocessing, artifact_dir, experiment_pointer_path,
        };
        use flow_like_catalog_core::{FlowPath, NodeDBConnection};
        use flow_like_ml_runtime::{
            DatasetSnapshot, EvaluationTask, PredictionRecord, StreamKey, TrainingRepository,
            TrainingSample, now_ms,
        };
        use flow_like_storage::files::store::FlowLikeStore;
        use flow_like_types::json::{from_slice, from_value, json};
        use std::{collections::BTreeMap, sync::Arc};

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

        /// The database a harness table lives in.
        #[derive(Clone, Copy)]
        enum TableHome {
            /// The board's shared project database.
            Project,
            /// The executing user's private database.
            User,
        }

        impl Harness {
            fn database(&self, home: TableHome) -> String {
                let directory = match home {
                    TableHome::Project => "database",
                    TableHome::User => "user-database",
                };
                self._scratch
                    .0
                    .join(directory)
                    .to_string_lossy()
                    .into_owned()
            }

            /// A file under the app storage or under the executing user's directory.
            fn stored(&self, scope: LedgerScope, relative: &str) -> std::path::PathBuf {
                match scope {
                    LedgerScope::App => self._scratch.0.join("board").join("storage"),
                    LedgerScope::User => self
                        ._scratch
                        .0
                        .join("user-store")
                        .join("users")
                        .join("tester")
                        .join("apps")
                        .join("auto-test"),
                }
                .join(relative)
            }

            /// Writes `rows` as table `name` of `home` and opens it as run connection `cache_key`.
            async fn table(
                &mut self,
                home: TableHome,
                name: &str,
                rows: Vec<Value>,
                cache_key: &str,
            ) -> NodeDBConnection {
                use flow_like_catalog_core::CachedDB;
                use flow_like_storage::databases::vector::{
                    VectorStore, buffered::BufferedVectorStore, lancedb::LanceDBVectorStore,
                };
                use flow_like_types::sync::RwLock;
                use std::sync::Arc;
                let connection = flow_like_storage::lancedb::connect(&self.database(home))
                    .execute()
                    .await
                    .unwrap();
                let mut store = LanceDBVectorStore::from_connection(connection, name.into()).await;
                store.insert(rows).await.unwrap();
                let source = NodeDBConnection {
                    cache_key: cache_key.into(),
                };
                self.context.cache.write().await.insert(
                    source.cache_key.clone(),
                    Arc::new(CachedDB {
                        db: Arc::new(RwLock::new(BufferedVectorStore::new(store, 0))),
                    }),
                );
                source
            }
        }

        /// A live workflow context for board "auto-test:board" whose project table holds `rows`.
        async fn harness(rows: Vec<Value>) -> Harness {
            harness_in(rows, TableHome::Project).await
        }

        /// A live workflow context for board "auto-test:board" of user "tester" with local app
        /// storage, a local user store, and project and user databases; `home` holds the source
        /// table of `rows`.
        async fn harness_in(rows: Vec<Value>, home: TableHome) -> Harness {
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
            use flow_like_storage::{
                files::store::{FlowLikeStore, local_store::LocalObjectStore},
                object_store::path::Path,
            };
            use flow_like_types::sync::{Mutex, RwLock};
            use std::sync::{Arc, Weak};
            let scratch = Scratch::new("auto-model");
            let user_store = scratch.0.join("user-store");
            std::fs::create_dir_all(&user_store).unwrap();
            let mut config = FlowLikeConfig::new();
            let project = scratch.0.join("database").to_string_lossy().into_owned();
            config.register_build_project_database(Arc::new(move |_| {
                flow_like_storage::lancedb::connect(&project)
            }));
            let user = scratch
                .0
                .join("user-database")
                .to_string_lossy()
                .into_owned();
            config.register_build_user_database(Arc::new(move |_| {
                flow_like_storage::lancedb::connect(&user)
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
            stores.user_store = Some(FlowLikeStore::Local(Arc::new(
                LocalObjectStore::new(user_store).unwrap(),
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
            let mut harness = Harness {
                context,
                source: NodeDBConnection {
                    cache_key: "auto-source".into(),
                },
                _scratch: scratch,
            };
            harness.source = harness
                .table(home, "measurements", rows, "auto-source")
                .await;
            harness
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

        /// The stream and split of `prepared` in `project`, and the request creating them.
        fn experiment_parts(
            project: &str,
            request: &AutoTrainRequest,
            prepared: &PreparedTableDataset,
            context: Value,
        ) -> (
            StreamKey,
            DatasetSnapshot,
            flow_like_ml_runtime::experiment::ExperimentRequest,
        ) {
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
            let experiment = flow_like_ml_runtime::experiment::ExperimentRequest {
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
            };
            (stream, snapshot, experiment)
        }

        fn create_experiment(
            repo: &TrainingRepository,
            project: &str,
            request: &AutoTrainRequest,
            prepared: &PreparedTableDataset,
            context: Value,
        ) -> Experiment {
            let (stream, snapshot, experiment) =
                experiment_parts(project, request, prepared, context);
            record_and_create(repo, &stream, snapshot, experiment, &|| false, &|_| {}).unwrap()
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
            let written: PublishedModelManifest =
                from_slice(&manifest.get(&mut harness.context, true).await.unwrap()).unwrap();
            assert_eq!(written.format_version, 2);
            assert_eq!(written.artifact_id(), artifact);
            assert_eq!(written.weights.file.as_deref(), Some("model.bin"));
            assert_eq!(written.weights.sha256, exported.sha256);
            assert_eq!(written.final_test_metrics, result.final_test_metrics);
            assert_eq!(
                written.calibration,
                Some(AnomalyCalibration {
                    scale: anomaly.scale,
                    threshold: anomaly.threshold
                })
            );
            // The preprocessing moves beside the manifest, named after its stem.
            assert_eq!(
                written.preprocessing.unwrap().file,
                "model.preprocessing.json"
            );
            let moved: PublishedPreprocessing = from_slice(
                &export_path("model.preprocessing.json")
                    .get(&mut harness.context, true)
                    .await
                    .unwrap(),
            )
            .unwrap();
            assert_eq!(moved.preprocessing, Some(result.preprocessing_manifest));
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

        /// Starts the ledger would refuse only after recording every sample, with the error
        /// text the earlier refusal names.
        fn rejected_start_cases(source: &NodeDBConnection) -> Vec<(AutoTrainRequest, &str)> {
            let anomaly_goals = |primary_metric: &str, minimum_audited_samples| ExperimentGoals {
                task: EvaluationTask::Anomaly { threshold: 0.5 },
                primary_metric: primary_metric.into(),
                direction: MetricDirection::Maximize,
                minimum_audited_samples,
                bounds: vec![],
            };
            let mut destination = anomaly_request(source);
            destination.tables.allowed_destination = Some(source.clone());
            let mut small_budget = anomaly_request(source);
            small_budget.budget.maximum_dataset_bytes = 4096;
            let mut unmeasured = anomaly_request(source);
            unmeasured.goals = Some(anomaly_goals("mean_iou", 1));
            // The default per-trial reservation of 60 s exceeds this training budget.
            let mut short_training = anomaly_request(source);
            short_training.budget.maximum_training_time_ms = 30_000;
            let mut unready = anomaly_request(source);
            unready.dataset.spec.minimum_examples = 1_000;
            let mut unaudited = anomaly_request(source);
            unaudited.goals = Some(anomaly_goals("auroc", 0));
            vec![
                (destination, "expected durable reference"),
                (small_budget, "maximum_dataset_bytes"),
                (unmeasured, "not measured"),
                (short_training, "maximum_duration_ms"),
                (unready, "minimum_examples"),
                (unaudited, "minimum_audited_samples"),
            ]
        }

        #[tokio::test]
        async fn rejected_starts_leave_no_experiment_or_ledger_samples() {
            let mut harness = harness(anomaly_rows()).await;
            let (repo, project) =
                crate::inspection::lifecycle::repository(&harness.context).unwrap();
            for (request, expected) in rejected_start_cases(&harness.source) {
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

        /// Polls `ready` until it yields a value, for at most ten seconds.
        async fn eventually<T>(mut ready: impl FnMut() -> Option<T>) -> T {
            for _ in 0..1000 {
                if let Some(value) = ready() {
                    return value;
                }
                tokio::time::sleep(std::time::Duration::from_millis(10)).await;
            }
            panic!("the condition did not hold within ten seconds");
        }

        fn scoped_stream(project: &str, request: &AutoTrainRequest) -> StreamKey {
            crate::inspection::lifecycle::InspectionContext {
                stream_id: request.dataset.stream_id.clone(),
                spec: request.dataset.spec.clone(),
            }
            .key(project.into())
            .unwrap()
        }

        #[test]
        fn a_cancelled_start_stops_between_sample_batches() {
            let scratch = Scratch::new("batched-start");
            let repo = TrainingRepository::open(scratch.0.join("training.sqlite")).unwrap();
            let (request, prepared) = distance_dataset();
            let (stream, snapshot, experiment) =
                experiment_parts("project", &request, &prepared, json!({}));
            let train = snapshot.train.len();
            let checks = std::cell::Cell::new(0);
            let cancelled = || {
                checks.set(checks.get() + 1);
                checks.get() > 1
            };
            let error =
                record_and_create(&repo, &stream, snapshot, experiment, &cancelled, &|_| {})
                    .unwrap_err()
                    .to_string();
            assert!(
                error.contains("before its experiment was created"),
                "{error}"
            );
            assert!(repo.list_experiments(&stream).unwrap().is_empty());
            assert_eq!(
                repo.count_new_training_samples(&stream, now_ms()).unwrap(),
                train
            );
        }

        #[tokio::test]
        async fn a_start_dropped_after_its_experiment_exists_cancels_it() {
            let scratch = Scratch::new("dropped-start");
            let repo = TrainingRepository::open(scratch.0.join("training.sqlite")).unwrap();
            let (request, prepared) = distance_dataset();
            let (stream, snapshot, experiment) =
                experiment_parts("project", &request, &prepared, json!({}));
            let mut start = Box::pin(create_started_experiment(
                None, &repo, &stream, snapshot, experiment,
            ));
            let pending = std::future::poll_fn(|cx| {
                std::task::Poll::Ready(std::future::Future::poll(start.as_mut(), cx).is_pending())
            })
            .await;
            assert!(pending);
            let created =
                eventually(|| repo.list_experiments(&stream).unwrap().into_iter().next()).await;
            // The blocking work returns right after creating it; the start is never polled again.
            tokio::time::sleep(std::time::Duration::from_millis(200)).await;
            drop(start);
            eventually(|| {
                (repo.get_experiment(&created.id).unwrap().status == ExperimentStatus::Cancelled)
                    .then_some(())
            })
            .await;
        }

        #[tokio::test]
        async fn the_dataset_budget_covers_the_split_and_its_training_view_exactly() {
            let mut harness = harness(anomaly_rows()).await;
            let (repo, project) =
                crate::inspection::lifecycle::repository(&harness.context).unwrap();
            // Stream IDs of one length give splits of one size.
            let scoped = |stream_id: &str, maximum_dataset_bytes: Option<u64>| {
                let mut request = anomaly_request(&harness.source);
                request.dataset.stream_id = stream_id.into();
                if let Some(bytes) = maximum_dataset_bytes {
                    request.budget.maximum_dataset_bytes = bytes;
                }
                let stream = scoped_stream(&project, &request);
                (request, stream)
            };
            let (request, stream) = scoped("sensors", None);
            let charged =
                start_auto_training_scoped(&mut harness.context, request, false, Some(stream))
                    .await
                    .unwrap()
                    .budget_used
                    .dataset_bytes;
            let (request, stream) = scoped("sensorz", Some(charged));
            let exact =
                start_auto_training_scoped(&mut harness.context, request, false, Some(stream))
                    .await
                    .unwrap();
            assert_eq!(exact.budget_used.dataset_bytes, charged);
            let (request, stream) = scoped("sensorx", Some(charged - 1));
            let error = start_auto_training_scoped(
                &mut harness.context,
                request,
                false,
                Some(stream.clone()),
            )
            .await
            .err()
            .expect("a budget one byte short is refused")
            .to_string();
            assert!(error.contains("training view"), "{error}");
            assert!(repo.list_experiments(&stream).unwrap().is_empty());
            assert_eq!(
                repo.count_new_training_samples(&stream, now_ms()).unwrap(),
                0
            );
        }

        #[test]
        fn readiness_needs_examples_per_label_and_normal_examples_to_validate() {
            let (_, mut distance) = distance_dataset();
            ensure_training_ready(&distance).unwrap();
            distance.spec.minimum_examples_per_class = 1_000;
            let error = ensure_training_ready(&distance).unwrap_err().to_string();
            assert!(error.contains("Label 'near'"), "{error}");
            let source: PinnedTableSource = from_value(json!({"database":{"cache_key":"source"},"reference":{"table":"measurements","branch":"main","version":1,"read_only":true,"pinned":true},"locator":null})).unwrap();
            let request = anomaly_request(&source.database);
            let mut sensors = prepare_rows(source, &request.dataset, &anomaly_rows()).unwrap();
            ensure_training_ready(&sensors).unwrap();
            for index in sensors.split.validation.clone() {
                sensors.samples[index].annotation =
                    flow_like_ml_core::Annotation::Anomaly { is_anomaly: true };
            }
            let error = ensure_training_ready(&sensors).unwrap_err().to_string();
            assert!(error.contains("normal examples"), "{error}");
        }

        #[tokio::test]
        async fn a_derived_dataset_over_the_dataset_budget_writes_no_tables() {
            let mut harness = harness(distance_rows()).await;
            let (repo, project) =
                crate::inspection::lifecycle::repository(&harness.context).unwrap();
            let mut request = distance_request(&harness.source);
            let prepared = crate::inspection::auto_training_tables::prepare_table_dataset(
                &mut harness.context,
                &request.dataset,
            )
            .await
            .unwrap();
            let (_, split, _) = experiment_parts(&project, &request, &prepared, json!({}));
            let charge = dataset_charge(&split, &prepared.spec.task).unwrap();
            // The split fits; a second dataset of its size does not.
            request.budget.maximum_dataset_bytes = charge + charge / 2;
            let experiment = create_experiment(
                &repo,
                &project,
                &request,
                &prepared,
                json!({
                    "search":request.search,"compute":request.compute,"dataset_request":request.dataset,
                    "pinned_source":prepared.source,"table_policy":{"create_tables":true},
                }),
            );
            assert_eq!(experiment.usage.dataset_bytes, charge);
            let error =
                register_prepared_candidate(&mut harness.context, &experiment.id, prepared.clone())
                    .await
                    .unwrap_err()
                    .to_string();
            assert!(error.contains("maximum_dataset_bytes"), "{error}");
            let state = repo.experiment_result(&experiment.id).unwrap();
            assert_eq!(state.datasets.len(), 1);
            assert!(state.experiment.request.created_tables.is_empty());
            let database = harness._scratch.0.join("database");
            let tables = flow_like_storage::lancedb::connect(&database.to_string_lossy())
                .execute()
                .await
                .unwrap()
                .table_names()
                .execute()
                .await
                .unwrap();
            assert_eq!(tables, ["measurements"]);
        }

        #[test]
        fn tables_of_a_refused_derived_dataset_stay_listed() {
            let scratch = Scratch::new("refused-derived");
            let repo = TrainingRepository::open(scratch.0.join("training.sqlite")).unwrap();
            let (request, prepared) = distance_dataset();
            let experiment = create_experiment(&repo, "project", &request, &prepared, json!({}));
            repo.cancel_experiment(&experiment.id, now_ms()).unwrap();
            let derived_id = uuid::Uuid::new_v4().to_string();
            let derived = derived_snapshot(
                &repo,
                &experiment.request.snapshot_id,
                u64::MAX,
                &prepared,
                derived_id.clone(),
            )
            .unwrap();
            let mut candidate = request.initial_candidates[0].clone();
            candidate.recipe["dataset_snapshot_id"] = json!(derived_id);
            let table = json!({"partition":"train","rows":72});
            let error = register_derived(
                &repo,
                &experiment.id,
                derived,
                json!(prepared.preprocessing),
                vec![candidate],
                vec![table.clone()],
            )
            .unwrap_err()
            .to_string();
            assert!(error.contains("created_tables"), "{error}");
            let state = repo.experiment_result(&experiment.id).unwrap();
            assert_eq!(state.datasets.len(), 1);
            assert_eq!(state.experiment.request.created_tables, [table]);
            assert!(state.events.iter().any(|event| {
                event["kind"] == "derived_dataset_rejected"
                    && event["body"]["dataset_snapshot_id"] == json!(derived_id)
            }));
        }

        #[tokio::test]
        async fn a_background_worker_that_gives_up_stops_the_clock_unless_another_drives() {
            let mut harness = harness(anomaly_rows()).await;
            let started = start_auto_training(
                &mut harness.context,
                anomaly_request(&harness.source),
                false,
            )
            .await
            .unwrap();
            let id = started.experiment_id;
            let repo = experiment_repository(&harness.context, &id).unwrap();
            assert!(matches!(
                background::unsettled(&repo, &id, None),
                background::Driven::Held(_)
            ));
            // A queued trial that nobody leases waits, here for a device another job holds.
            repo.submit_experiment_trial(&id, 0, "candidate-0", now_ms())
                .unwrap();
            assert!(matches!(
                background::unsettled(&repo, &id, None),
                background::Driven::Waiting(_)
            ));
            let last = background::ATTEMPTS - 1;
            background::process(
                background::Work::new(repo.clone(), id.clone(), last).unwrap(),
                |_, _| background::Driven::Held("another controller holds this experiment".into()),
            );
            let held = repo.get_experiment(&id).unwrap();
            assert_eq!(held.status, ExperimentStatus::Running);
            assert!(held.clock.unwrap().active_since_ms.is_some());
            background::process(
                background::Work::new(repo.clone(), id.clone(), last).unwrap(),
                |repo, id| background::unsettled(repo, id, None),
            );
            let stopped = repo.experiment_result(&id).unwrap();
            assert_eq!(stopped.experiment.status, ExperimentStatus::Cancelled);
            assert!(stopped.experiment.clock.unwrap().active_since_ms.is_none());
            let cancelled: Vec<_> = stopped
                .events
                .iter()
                .filter(|event| event["kind"] == "background_worker_stopped")
                .map(|event| event["body"]["cancelled"].as_bool())
                .collect();
            assert_eq!(cancelled, [None, Some(true)]);
        }

        #[tokio::test]
        async fn a_panicking_background_drive_cancels_its_experiment_and_frees_its_slot() {
            let mut harness = harness(anomaly_rows()).await;
            let started = start_auto_training(
                &mut harness.context,
                anomaly_request(&harness.source),
                false,
            )
            .await
            .unwrap();
            let id = started.experiment_id;
            let repo = experiment_repository(&harness.context, &id).unwrap();
            let work = background::Work::new(repo.clone(), id.clone(), 0).unwrap();
            let key = work.key.clone();
            background::queue()
                .entries
                .insert(key.clone(), background::Queued::of(&work));
            background::process(work, |_, _| panic!("drive failed on purpose"));
            assert!(!background::queue().entries.contains_key(&key));
            let state = repo.experiment_result(&id).unwrap();
            assert_eq!(state.experiment.status, ExperimentStatus::Cancelled);
            assert!(state.events.iter().any(|event| {
                event["kind"] == "background_worker_failed"
                    && event["body"]["error"]
                        .as_str()
                        .is_some_and(|error| error.contains("drive failed on purpose"))
            }));
        }

        #[tokio::test]
        async fn an_unwritable_predictions_table_is_reported_once_while_get_succeeds() {
            let mut harness = harness(anomaly_rows()).await;
            let mut request = anomaly_request(&harness.source);
            request.tables.create_tables = true;
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
            // The pinned view now holds the live table, whose reference differs from the pin.
            let pinned = experiment.request.context["pinned_source"]["database"]["cache_key"]
                .as_str()
                .unwrap()
                .to_owned();
            {
                let mut cache = harness.context.cache.write().await;
                let live = cache.get(&harness.source.cache_key).unwrap().clone();
                cache.insert(pinned, live);
            }
            for _ in 0..2 {
                let result = lifecycle(
                    &mut harness.context,
                    &experiment.id,
                    LifecycleAction::Status,
                )
                .await
                .unwrap();
                assert!(result.predictions_table.is_none());
            }
            let failures = repo
                .experiment_events(&experiment.id)
                .unwrap()
                .iter()
                .filter(|event| event["kind"] == "predictions_table_failed")
                .count();
            assert_eq!(failures, 1);
        }

        /// The execution scopes of another board of the same app and of another app.
        const FOREIGN_SCOPES: [(&str, &str); 2] =
            [("auto-test", "other-board"), ("other-app", "board")];

        fn enter_scope(harness: &mut Harness, app: &str, board: &str) {
            let scope = harness.context.execution_cache.as_mut().unwrap();
            scope.app_id = app.into();
            scope.board_id = board.into();
        }

        /// A learning project of the harness board over the anomaly readings.
        fn create_learning_project(harness: &Harness) -> String {
            let (repo, project) =
                crate::inspection::lifecycle::repository(&harness.context).unwrap();
            let mut training = anomaly_request(&harness.source);
            training.dataset.stream_id = "learning:scope".into();
            let mut goals =
                flow_like_ml_runtime::auto_training::default_goals(&training.dataset.spec);
            goals.bounds = vec![flow_like_ml_runtime::MetricBound {
                name: "auroc".into(),
                minimum: Some(0.5),
                maximum: None,
            }];
            repo.create_learning_project(
                flow_like_ml_runtime::LearningProjectRequest {
                    stream: scoped_stream(&project, &training),
                    spec: json!(training.dataset.spec),
                    source: json!({}),
                    goals,
                    policy: Default::default(),
                    budget: Default::default(),
                    deployment_id: "deployment:scope".into(),
                },
                now_ms(),
            )
            .unwrap()
            .id
        }

        #[tokio::test]
        async fn learning_projects_stay_in_their_app_and_board() {
            use crate::inspection::learning;
            let mut harness = harness(anomaly_rows()).await;
            let project_id = create_learning_project(&harness);
            learning::project_repository(&harness.context, &project_id).unwrap();
            for (app, board) in FOREIGN_SCOPES {
                enter_scope(&mut harness, app, board);
                let refusals = [
                    refusal(
                        "project",
                        learning::project_repository(&harness.context, &project_id),
                    ),
                    refusal(
                        "prepare cycle",
                        learning::prepare_learning_cycle(&mut harness.context, &project_id).await,
                    ),
                    refusal(
                        "finish cycle",
                        learning::finish_learning_cycle(&mut harness.context, &project_id).await,
                    ),
                ];
                // Another board's ledger row has another project string; another app's ledgers
                // do not hold the project at all.
                for (entry, error) in refusals {
                    assert!(
                        error.contains("belongs to a different app or board"),
                        "{entry} from {app}:{board}: {error}"
                    );
                }
            }
        }

        #[tokio::test]
        async fn pinned_sources_reopen_only_in_their_app_and_board() {
            use crate::inspection::auto_training_tables::reopen_pinned_source;
            let mut harness = harness(anomaly_rows()).await;
            let started = start_auto_training(
                &mut harness.context,
                anomaly_request(&harness.source),
                false,
            )
            .await
            .unwrap();
            let repo = experiment_repository(&harness.context, &started.experiment_id).unwrap();
            let pinned: PinnedTableSource = from_value(
                repo.get_experiment(&started.experiment_id)
                    .unwrap()
                    .request
                    .context["pinned_source"]
                    .clone(),
            )
            .unwrap();
            reopen_pinned_source(&mut harness.context, &pinned)
                .await
                .unwrap();
            for (app, board) in FOREIGN_SCOPES {
                enter_scope(&mut harness, app, board);
                let (_, error) = refusal(
                    "reopen source",
                    reopen_pinned_source(&mut harness.context, &pinned).await,
                );
                assert!(
                    error.contains("different execution scope"),
                    "{app}:{board}: {error}"
                );
            }
        }

        #[cfg(unix)]
        #[tokio::test]
        async fn a_ledger_directory_linked_outside_app_storage_is_refused() {
            let harness = harness(anomaly_rows()).await;
            let outside = Scratch::new("outside-app-storage");
            let link = harness._scratch.0.join("training-ledgers");
            std::os::unix::fs::symlink(&outside.0, &link).unwrap();
            let (_, error) = refusal(
                "escape",
                crate::inspection::lifecycle::repository(&harness.context),
            );
            assert!(error.contains("escapes app storage"), "{error}");
            std::fs::remove_file(&link).unwrap();
            std::os::unix::fs::symlink(outside.0.join("missing"), &link).unwrap();
            let (_, error) = refusal(
                "dangling",
                crate::inspection::lifecycle::repository(&harness.context),
            );
            assert!(error.contains("dangling symlink"), "{error}");
            assert!(!outside.0.join("apps").exists());
        }

        #[test]
        fn training_embeddings_must_name_one_feature_extractor() {
            use crate::inspection::lifecycle::FeatureExtractorRef;
            let preprocessing = TablePreprocessing::Tensor {
                column: "features".into(),
                input_shape: vec![4, 8],
            };
            let extractor = |source: &str| FeatureExtractorRef {
                source_id: source.into(),
                blob_sha256: "c".repeat(64),
                image_steps: vec![],
            };
            let row = |source: &str| json!({"features":{"shape":[4,8],"values":[],"feature_extractor":extractor(source)}});
            let named = training_extractor(
                &preprocessing,
                &[json!({"features":{}})],
                Some(&extractor("dino")),
            )
            .unwrap()
            .unwrap();
            assert_eq!(named.source_id, "dino");
            for (rows, requested) in [
                (vec![row("dino"), row("resnet")], None),
                (vec![row("resnet")], Some(extractor("dino"))),
            ] {
                let error = training_extractor(&preprocessing, &rows, requested.as_ref())
                    .unwrap_err()
                    .to_string();
                assert!(
                    error.contains("train one model per extractor") && !error.contains("predict"),
                    "{error}"
                );
            }
        }

        /// Normal and anomalous [4, 8] patch embeddings, stored as tensor JSON text.
        fn embedding_rows() -> Vec<Value> {
            (0..48)
                .map(|i| {
                    let anomaly = i % 4 == 3;
                    let values: Vec<f32> = (0..32)
                        .map(|j| {
                            if anomaly {
                                4.0
                            } else {
                                ((i + j) % 5) as f32 / 10.0
                            }
                        })
                        .collect();
                    json!({"id":i.to_string(),"group":(i / 4).to_string(),"features":json!({"shape":[4,8],"values":values}).to_string(),"anomaly":anomaly})
                })
                .collect()
        }

        #[tokio::test]
        async fn patch_models_record_the_extractor_the_request_names() {
            use crate::inspection::lifecycle::FeatureExtractorRef;
            let mut harness = harness(embedding_rows()).await;
            let extractor = FeatureExtractorRef {
                source_id: "dino".into(),
                blob_sha256: "d".repeat(64),
                image_steps: vec![],
            };
            let request: AutoTrainRequest = from_value(json!({
                "dataset":{"source":harness.source,"stream_id":"patches",
                    "spec":{"id":"patch-anomaly","description":"Detect anomalous patch embeddings","task":"visual_anomaly","labels":[],"input_shape":[4,8],"prediction_horizon_ms":null,"minimum_examples":2,"minimum_examples_per_class":1},
                    "mapping":{"row_id":"id","group_id":"group","features":{"kind":"tensor","column":"features"},"target":{"kind":"column","column":"anomaly"},"provenance":{"kind":"measured","source":"gauge"}},
                    "split":{"kind":"group","train_fraction":0.6,"validation_fraction":0.2,"seed":11}},
                "compute":{"backend":"cpu"},"search":{"maximum_candidates":1},"budget":{"maximum_trials":1},
                "tables":{"create_tables":false},
                "initial_candidates":[{"engine":"patchcore","recipe":{"labels":[],"inspection_task":"visual_anomaly","coreset_size":16},"compute":{"backend":"cpu"}}],
                "feature_extractor":extractor
            }))
            .unwrap();
            let started = start_auto_training(&mut harness.context, request, false)
                .await
                .unwrap();
            let candidates = experiment_repository(&harness.context, &started.experiment_id)
                .unwrap()
                .get_experiment(&started.experiment_id)
                .unwrap()
                .request
                .candidates;
            assert_eq!(candidates[0].engine, "patchcore");
            assert_eq!(candidates[0].recipe["feature_extractor"], json!(extractor));
        }

        #[test]
        fn a_model_that_recorded_its_extractor_refuses_unnamed_embeddings() {
            let recorded =
                json!({"source_id":"dino","blob_sha256":"e".repeat(64),"image_steps":[]});
            let manifest = json!({"engine":"patchcore","feature_extractor":recorded});
            let error = ensure_extractor_named("artifact", &manifest, None)
                .unwrap_err()
                .to_string();
            assert!(error.contains("dino"), "{error}");
            let named = crate::inspection::lifecycle::runtime_extractor(&recorded).unwrap();
            ensure_extractor_named("artifact", &manifest, Some(&named)).unwrap();
            ensure_extractor_named("artifact", &json!({"engine":"patchcore"}), None).unwrap();
            ensure_extractor_named(
                "artifact",
                &json!({"engine":"patchcore","feature_extractor":null}),
                None,
            )
            .unwrap();
        }

        #[test]
        fn fine_tunes_without_image_steps_report_the_source_preprocessing() {
            use flow_like_ml_core::PreprocessingStep;
            let source = |manifest: Value| flow_like_ml_runtime::PretrainedSource {
                id: "pretrained:fixture".into(),
                project_id: "project".into(),
                source_kind: flow_like_ml_runtime::PretrainedSourceKind::Imported,
                blob: flow_like_ml_runtime::BlobRef {
                    sha256: "0".repeat(64),
                    bytes: 1,
                    path: "fixture.blob".into(),
                },
                manifest,
                origin: json!({}),
                created_at_ms: 0,
            };
            let steps = json!([
                {"kind":"resize","width":8,"height":8},
                {"kind":"normalize","mean":[0.5],"std":[0.25]}
            ]);
            let recommending = source(json!({ "recommended_preprocessing": steps }));
            let note = unchecked_pretrained_preprocessing(&recommending, &[])
                .unwrap()
                .expect("an unchecked normalization is reported");
            assert_eq!(note["source_id"], "pretrained:fixture");
            assert_eq!(note["recommended_preprocessing"], steps);
            let declared = [PreprocessingStep::Resize {
                width: 8,
                height: 8,
            }];
            assert!(
                unchecked_pretrained_preprocessing(&recommending, &declared)
                    .unwrap()
                    .is_none()
            );
            assert!(
                unchecked_pretrained_preprocessing(&source(json!({})), &[])
                    .unwrap()
                    .is_none()
            );
        }

        /// Starts `request`, steps it until its search ends and finalizes it.
        async fn finalized_experiment(harness: &mut Harness, request: AutoTrainRequest) -> String {
            let id = start_auto_training(&mut harness.context, request, false)
                .await
                .unwrap()
                .experiment_id;
            for _ in 0..8 {
                let stepped = lifecycle(&mut harness.context, &id, LifecycleAction::Step)
                    .await
                    .unwrap();
                if stepped.status != ExperimentStatus::Running {
                    break;
                }
            }
            let finalized = lifecycle(&mut harness.context, &id, LifecycleAction::Finalize)
                .await
                .unwrap();
            assert_eq!(
                finalized.status,
                ExperimentStatus::Completed,
                "{:?}",
                finalized.stop_reason
            );
            assert!(!finalized.final_test_metrics.is_empty());
            id
        }

        fn published_events(repo: &TrainingRepository, id: &str) -> usize {
            repo.experiment_events(id)
                .unwrap()
                .iter()
                .filter(|event| event["kind"] == "model_published")
                .count()
        }

        #[tokio::test]
        async fn finalize_publishes_the_winner_once_and_gets_rewrite_nothing() {
            use crate::inspection::published::WEIGHTS_FILE;
            let mut harness = harness(anomaly_rows()).await;
            let request = anomaly_request(&harness.source);
            let id = finalized_experiment(&mut harness, request).await;
            let (repo, scope) = experiment_ledger(&harness.context, &id).unwrap();
            assert_eq!(scope, LedgerScope::App);
            let winner = AutoTrainingResult::from(repo.experiment_result(&id).unwrap())
                .model_bundle_ref
                .unwrap();
            let pointer_path = harness.stored(LedgerScope::App, &experiment_pointer_path(&id));
            let pointer = std::fs::read(&pointer_path).unwrap();
            let parsed: ExperimentPointer = from_slice(&pointer).unwrap();
            assert_eq!(parsed.artifact_id, winner.id);
            assert_eq!(parsed.sha256, winner.blob.sha256);
            assert_eq!(parsed.storage, ModelStorage::App);
            let weights = harness.stored(
                LedgerScope::App,
                &format!("{}/{WEIGHTS_FILE}", artifact_dir(&winner.id)),
            );
            assert_eq!(
                flow_like_ml_core::content_digest(&std::fs::read(weights).unwrap()),
                winner.blob.sha256
            );
            assert_eq!(published_events(&repo, &id), 1);
            for _ in 0..2 {
                lifecycle(&mut harness.context, &id, LifecycleAction::Status)
                    .await
                    .unwrap();
            }
            assert_eq!(published_events(&repo, &id), 1);
            assert_eq!(std::fs::read(&pointer_path).unwrap(), pointer);
        }

        #[tokio::test]
        async fn training_on_a_user_table_stays_in_the_private_ledger_and_folder() {
            let mut harness = harness_in(anomaly_rows(), TableHome::User).await;
            let request = anomaly_request(&harness.source);
            let id = finalized_experiment(&mut harness, request).await;
            let read = lifecycle(&mut harness.context, &id, LifecycleAction::Status)
                .await
                .unwrap();
            assert_eq!(read.experiment_id, id);
            let (repo, scope) = experiment_ledger(&harness.context, &id).unwrap();
            assert_eq!(scope, LedgerScope::User);
            let experiment = repo.get_experiment(&id).unwrap();
            let stream = &experiment.request.stream;
            assert!(
                stream.stream_id.starts_with("user:auto:"),
                "{}",
                stream.stream_id
            );
            assert_eq!(
                StreamKind::of_stream(&stream.stream_id),
                Some(StreamKind::Auto)
            );
            assert_eq!(stream.project_id, "auto-test:board:user");
            assert_eq!(experiment.request.context[LEDGER_SCOPE_KEY], "user");
            let ledgers = crate::inspection::lifecycle::ledger_paths(&harness.context).unwrap();
            assert!(!ledgers.app.exists(), "{}", ledgers.app.display());
            assert!(ledgers.user.unwrap().is_file());
            assert!(
                harness
                    .stored(LedgerScope::User, &experiment_pointer_path(&id))
                    .is_file()
            );
            assert!(!harness.stored(LedgerScope::App, "models").exists());
        }

        #[tokio::test]
        async fn private_training_writes_no_tables_into_the_project_database() {
            use crate::inspection::feature_engineering::FeatureSourceInput;
            let mut harness = harness(anomaly_rows()).await;
            let notes = harness
                .table(
                    TableHome::User,
                    "operator_notes",
                    anomaly_rows(),
                    "user-notes",
                )
                .await;
            let mut request = anomaly_request(&harness.source);
            request.feature_sources = vec![FeatureSourceInput {
                alias: "notes".into(),
                source: notes,
                allowed_columns: vec!["id".into(), "x".into()],
            }];
            request.tables.create_tables = true;
            let error = start_auto_training(&mut harness.context, request.clone(), false)
                .await
                .err()
                .expect("private training cannot create tables beside a project table")
                .to_string();
            assert_eq!(error, PRIVATE_TABLE_WRITES);
            let ledgers = crate::inspection::lifecycle::ledger_paths(&harness.context).unwrap();
            let user = ledgers.user.clone().unwrap();
            assert!(!ledgers.app.exists() && !user.exists());
            request.tables.create_tables = false;
            let started = start_auto_training(&mut harness.context, request, false)
                .await
                .unwrap();
            let (_, scope) = experiment_ledger(&harness.context, &started.experiment_id).unwrap();
            assert_eq!(scope, LedgerScope::User);
            assert!(!ledgers.app.exists() && user.is_file());
        }

        #[tokio::test]
        async fn private_table_writes_fail_closed_without_the_project_database_digest() {
            use crate::inspection::auto_training_tables::{
                ScopeDigests, TableWritePolicy, pin_table_source, scope_digests,
            };
            let mut harness = harness(anomaly_rows()).await;
            let notes = harness
                .table(
                    TableHome::User,
                    "operator_notes",
                    anomaly_rows(),
                    "user-notes",
                )
                .await;
            let project_main = pin_table_source(&mut harness.context, &harness.source)
                .await
                .unwrap();
            let user_main = pin_table_source(&mut harness.context, &notes)
                .await
                .unwrap();
            let unknown = ScopeDigests {
                project: None,
                ..scope_digests(&harness.context).await.unwrap()
            };
            let beside_main = TableWritePolicy::default();
            let refused = ensure_private_table_writes(
                &mut harness.context,
                &unknown,
                &project_main,
                &beside_main,
            )
            .await
            .unwrap_err();
            assert_eq!(refused.to_string(), PRIVATE_TABLE_WRITES);
            ensure_private_table_writes(&mut harness.context, &unknown, &user_main, &beside_main)
                .await
                .unwrap();
            let into_project = TableWritePolicy {
                create_tables: false,
                allowed_destination: Some(harness.source.clone()),
                ..TableWritePolicy::default()
            };
            let refused = ensure_private_table_writes(
                &mut harness.context,
                &unknown,
                &user_main,
                &into_project,
            )
            .await
            .unwrap_err();
            assert_eq!(refused.to_string(), PRIVATE_TABLE_WRITES);
        }

        #[tokio::test]
        async fn cycle_starts_use_the_ledger_that_holds_their_learning_cycle() {
            use flow_like_ml_runtime::{
                LearningProjectRequest, MetricBound, auto_training::default_goals,
            };
            let harness = harness(anomaly_rows()).await;
            let (app, project) =
                crate::inspection::lifecycle::repository(&harness.context).unwrap();
            let legacy_path = harness.stored(LedgerScope::App, "inspection/jobs.sqlite");
            let legacy = TrainingRepository::open(legacy_path).unwrap();
            let request = anomaly_request(&harness.source);
            let stream = scoped_stream(&project, &request);
            let mut goals = default_goals(&request.dataset.spec);
            goals.bounds = vec![MetricBound {
                name: goals.primary_metric.clone(),
                minimum: Some(0.5),
                maximum: None,
            }];
            let learning = legacy
                .create_learning_project(
                    LearningProjectRequest {
                        stream: stream.clone(),
                        spec: json!(request.dataset.spec),
                        source: json!({}),
                        goals,
                        policy: Default::default(),
                        budget: Default::default(),
                        deployment_id: "deployment:learning".into(),
                    },
                    now_ms(),
                )
                .unwrap();
            let cycle = legacy
                .reserve_learning_cycle(&learning.id, "cycle:0", request.budget.clone(), now_ms())
                .unwrap();
            let mut cycled = request.clone();
            cycled.learning_cycle_id = Some(cycle.id.clone());
            let started =
                StartLedger::for_stream(&harness.context, &cycled, false, stream.clone()).unwrap();
            let held = started.repo.get_learning_cycle(&cycle.id).unwrap();
            assert_eq!(held.id, cycle.id);
            assert_eq!((started.scope, started.marker), (LedgerScope::App, None));
            assert!(app.get_learning_cycle(&cycle.id).is_err());
            let without_cycle =
                StartLedger::for_stream(&harness.context, &request, false, stream).unwrap();
            assert_eq!(without_cycle.repo.path(), app.path());
        }

        #[tokio::test]
        async fn private_training_adopts_the_app_sources_it_may_fine_tune() {
            let mut harness = harness_in(distance_rows(), TableHome::User).await;
            let (app, app_project) =
                crate::inspection::lifecycle::repository(&harness.context).unwrap();
            let config = json!({
                "recipe": {"architecture":"mlp","input_features":2,"hidden":4,"outputs":2,"objective":"classification"},
                "backend": {"backend":"cpu"},
                "epochs": 1, "batch_size": 1, "learning_rate": 0.01, "seed": 7
            });
            let source = app
                .register_pretrained_source(
                    &app_project,
                    b"pretrained weights",
                    json!({"engine":"burn","format_version":1,"config":config,"input_shape":[2],"labels":["near","far"]}),
                    json!({"source":"fixture"}),
                    1024,
                    now_ms(),
                )
                .unwrap();
            let mut request = distance_request(&harness.source);
            request.tables.create_tables = false;
            request.pretrained = Some(
                from_value(
                    json!({"source_id":source.id,"replace_head":false,"freeze_backbone":false}),
                )
                .unwrap(),
            );
            let started = start_auto_training(&mut harness.context, request, false)
                .await
                .unwrap();
            let (repo, scope) =
                experiment_ledger(&harness.context, &started.experiment_id).unwrap();
            assert_eq!(scope, LedgerScope::User);
            let adopted = repo.get_pretrained_source(&source.id).unwrap();
            assert_eq!(adopted.project_id, "auto-test:board:user");
            assert_eq!(adopted.blob.sha256, source.blob.sha256);
            let candidates = repo
                .get_experiment(&started.experiment_id)
                .unwrap()
                .request
                .candidates;
            assert_eq!(candidates[0].engine, "burn");
            assert_eq!(
                candidates[0].recipe["pretrained"]["source_id"],
                json!(source.id)
            );
            let report = consultation_context(&repo, &started.experiment_id).unwrap();
            let listed = report["pretrained_sources"].as_array().unwrap();
            assert!(
                listed
                    .iter()
                    .any(|listed| listed["source_id"] == json!(source.id)),
                "{report}"
            );
        }

        #[tokio::test]
        async fn runs_without_a_local_app_store_refuse_training_before_reading_sources() {
            use crate::inspection::lifecycle::{TRAINING_UNAVAILABLE, is_training_unavailable};
            let mut harness = harness(anomaly_rows()).await;
            harness
                .context
                .execution_cache
                .as_mut()
                .unwrap()
                .stores
                .app_storage_store = Some(FlowLikeStore::Memory(Arc::new(
                flow_like_storage::object_store::memory::InMemory::new(),
            )));
            let mut request = anomaly_request(&harness.source);
            request.dataset.source = NodeDBConnection {
                cache_key: "unreadable-source".into(),
            };
            let error = start_auto_training(&mut harness.context, request, false)
                .await
                .err()
                .expect("training needs a local ledger");
            assert!(is_training_unavailable(&error), "{error}");
            assert_eq!(error.to_string(), TRAINING_UNAVAILABLE);
        }

        fn assert_same_predictions(expected: &AutoModelPredictions, served: &AutoModelPredictions) {
            assert_eq!(served.model_artifact_id, expected.model_artifact_id);
            assert_eq!(served.predictions, expected.predictions);
            assert_eq!(json!(served.anomaly), json!(expected.anomaly));
        }

        /// Copies every file below `directory` into a new memory store, below `prefix`.
        async fn memory_copy(
            context: &mut ExecutionContext,
            directory: &std::path::Path,
            prefix: &str,
        ) -> FlowLikeStore {
            let store = FlowLikeStore::Memory(Arc::new(
                flow_like_storage::object_store::memory::InMemory::new(),
            ));
            context
                .cache
                .write()
                .await
                .insert("published-copy".into(), Arc::new(store.clone()));
            let mut pending = vec![directory.to_path_buf()];
            while let Some(current) = pending.pop() {
                for entry in std::fs::read_dir(&current).unwrap() {
                    let path = entry.unwrap().path();
                    if path.is_dir() {
                        pending.push(path);
                        continue;
                    }
                    let relative = path
                        .strip_prefix(directory)
                        .unwrap()
                        .to_string_lossy()
                        .replace('\\', "/");
                    FlowPath {
                        path: format!("{prefix}/{relative}"),
                        store_ref: "published-copy".into(),
                        cache_store_ref: None,
                    }
                    .put(context, std::fs::read(&path).unwrap(), true)
                    .await
                    .unwrap();
                }
            }
            store
        }

        #[tokio::test]
        async fn predict_auto_model_falls_back_to_the_published_winner() {
            let mut harness = harness(anomaly_rows()).await;
            let request = anomaly_request(&harness.source);
            let id = finalized_experiment(&mut harness, request).await;
            let repo = experiment_repository(&harness.context, &id).unwrap();
            let snapshot_id = repo.get_experiment(&id).unwrap().request.snapshot_id;
            let test = repo.get_snapshot(&snapshot_id).unwrap().test;
            let ledger = predict_auto_model(&harness.context, raw_rows_request(&id, &test))
                .await
                .unwrap();
            assert!(ledger.anomaly.is_some());
            // Another board's runs see the ledger row under a foreign project string, which
            // counts as missing, so they read the published files.
            enter_scope(&mut harness, "auto-test", "other-board");
            let foreign =
                predict_auto_model_or_published(&mut harness.context, raw_rows_request(&id, &test))
                    .await
                    .unwrap();
            assert_same_predictions(&ledger, &foreign);
            // A runtime without a local ledger reads the same files from its own app store.
            enter_scope(&mut harness, "auto-test", "board");
            let models = harness.stored(LedgerScope::App, "models");
            let copy = memory_copy(&mut harness.context, &models, "board/storage/models").await;
            harness.context.cache.write().await.clear();
            harness
                .context
                .execution_cache
                .as_mut()
                .unwrap()
                .stores
                .app_storage_store = Some(copy);
            let served =
                predict_auto_model_or_published(&mut harness.context, raw_rows_request(&id, &test))
                    .await
                    .unwrap();
            assert_same_predictions(&ledger, &served);
        }

        #[tokio::test]
        async fn unmarked_app_experiments_on_user_tables_resume_and_publish_privately() {
            let mut harness = harness_in(anomaly_rows(), TableHome::User).await;
            let (repo, project) =
                crate::inspection::lifecycle::repository(&harness.context).unwrap();
            let mut request = anomaly_request(&harness.source);
            request.dataset.stream_id = LedgerScope::App.new_stream_id(StreamKind::Auto);
            let prepared = crate::inspection::auto_training_tables::prepare_table_dataset(
                &mut harness.context,
                &request.dataset,
            )
            .await
            .unwrap();
            // Created before markers existed: an App-ledger record without `ledger_scope`.
            let unmarked = create_experiment(
                &repo,
                &project,
                &request,
                &prepared,
                json!({
                    "search":request.search,"compute":request.compute,"dataset_request":request.dataset,
                    "pinned_source":prepared.source,"table_policy":request.tables,
                }),
            );
            assert!(unmarked.request.context.get(LEDGER_SCOPE_KEY).is_none());
            repo.cancel_experiment(&unmarked.id, now_ms()).unwrap();
            let resumed = lifecycle(&mut harness.context, &unmarked.id, LifecycleAction::Resume)
                .await
                .unwrap();
            assert_ne!(resumed.status, ExperimentStatus::Cancelled);
            eventually(|| {
                (repo.get_experiment(&unmarked.id).unwrap().status == ExperimentStatus::Completed)
                    .then_some(())
            })
            .await;
            lifecycle(&mut harness.context, &unmarked.id, LifecycleAction::Status)
                .await
                .unwrap();
            assert!(
                harness
                    .stored(LedgerScope::User, &experiment_pointer_path(&unmarked.id))
                    .is_file()
            );
            assert!(!harness.stored(LedgerScope::App, "models").exists());
        }

        #[test]
        fn draining_cancels_only_the_experiments_under_its_prefix() {
            let (drained, kept) = (Scratch::new("drained-ledger"), Scratch::new("kept-ledger"));
            let (request, prepared) = distance_dataset();
            let queued = [&drained, &kept].map(|scratch| {
                let repo = TrainingRepository::open(scratch.0.join("jobs.sqlite")).unwrap();
                let experiment =
                    create_experiment(&repo, "project", &request, &prepared, json!({}));
                let work = background::Work::new(repo.clone(), experiment.id.clone(), 0).unwrap();
                background::queue()
                    .entries
                    .insert(work.key.clone(), background::Queued::of(&work));
                (repo, experiment.id, work.key)
            });
            let report = drain_experiments(
                &[drained.0.canonicalize().unwrap()],
                std::time::Instant::now() + std::time::Duration::from_millis(100),
            );
            for (_, _, key) in &queued {
                background::queue().entries.remove(key);
            }
            // Nothing drives these entries, so the drained one is still queued at the deadline.
            assert_eq!(
                report,
                crate::inspection::lifecycle::DrainReport {
                    cancelled: 1,
                    still_running: 1
                }
            );
            let status = |(repo, id, _): &(TrainingRepository, String, String)| {
                repo.get_experiment(id).unwrap().status
            };
            assert_eq!(status(&queued[0]), ExperimentStatus::Cancelled);
            assert_eq!(status(&queued[1]), ExperimentStatus::Running);
        }
    }
}
