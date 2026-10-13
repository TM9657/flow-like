use super::auto_training::{AutoTrainRequest, AutoTrainingResult};
#[cfg(feature = "training")]
use super::cancellation::{CancelFlag, CancelOnDrop};
#[cfg(feature = "training")]
use flow_like::flow::execution::LogLevel;
use flow_like::flow::{
    execution::context::ExecutionContext,
    node::{Node, NodeLogic},
};
use flow_like_ml_runtime::learning::*;
use flow_like_types::Result;
#[cfg(feature = "inference")]
use flow_like_types::{anyhow, json::json};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

#[cfg(feature = "training")]
#[path = "learning_inference.rs"]
mod inference;

#[cfg(all(test, feature = "training"))]
#[path = "learning_tests.rs"]
mod tests;

#[derive(Clone, Serialize, Deserialize, JsonSchema)]
pub struct CreateLearningProjectRequest {
    pub training: AutoTrainRequest,
    #[serde(default)]
    pub policy: LearningPolicy,
    #[serde(default)]
    pub budget: LearningBudget,
    /// Promote eligible candidates after the configured independent audit.
    #[serde(default)]
    pub automatic_promotion: bool,
    /// Advance temporal boundaries at each cycle while preserving sealed audit exclusions.
    #[serde(default)]
    pub rolling_time_split: Option<RollingTimeSplit>,
}

#[derive(Clone, Serialize, Deserialize, JsonSchema)]
pub struct RollingTimeSplit {
    pub validation_duration_ms: u64,
    pub test_duration_ms: u64,
}

#[cfg(feature = "training")]
impl RollingTimeSplit {
    fn apply(
        &self,
        split: &mut super::auto_training_tables::TableSplitPolicy,
        at_ms: i64,
    ) -> Result<()> {
        use super::auto_training_tables::TableSplitPolicy;
        let TableSplitPolicy::Time {
            train_end_ms,
            validation_end_ms,
            ..
        } = split
        else {
            return Err(anyhow!(
                "Rolling time splits require a temporal source split"
            ));
        };
        let validation = i64::try_from(self.validation_duration_ms)
            .map_err(|_| anyhow!("Validation duration exceeds the supported range"))?;
        let test = i64::try_from(self.test_duration_ms)
            .map_err(|_| anyhow!("Test duration exceeds the supported range"))?;
        if validation <= 0 || test <= 0 {
            return Err(anyhow!(
                "Rolling validation and test durations must be positive"
            ));
        }
        let validation_end = at_ms
            .checked_sub(test)
            .ok_or_else(|| anyhow!("Rolling validation boundary overflow"))?;
        let train_end = validation_end
            .checked_sub(validation)
            .ok_or_else(|| anyhow!("Rolling training boundary overflow"))?;
        *train_end_ms = train_end;
        *validation_end_ms = validation_end;
        Ok(())
    }
}
#[derive(Clone, Serialize, Deserialize, JsonSchema)]
pub struct LearningProjectId {
    pub project_id: String,
}
#[derive(Clone, Serialize, Deserialize, JsonSchema)]
pub struct LearningObservationsRequest {
    pub project_id: String,
    pub observations: Vec<LearningObservation>,
}
#[derive(Clone, Serialize, Deserialize, JsonSchema)]
pub struct LearningReviewRequest {
    pub project_id: String,
    pub review: LearningReview,
}
#[derive(Clone, Serialize, Deserialize, JsonSchema)]
pub struct LearningProductChangeRequest {
    pub project_id: String,
    pub change_id: String,
    pub description: String,
}
#[derive(Clone, Serialize, Deserialize, JsonSchema)]
pub struct LearningReviewSelectionRequest {
    pub project_id: String,
    pub selection_key: String,
}
#[derive(Clone, Serialize, Deserialize, JsonSchema)]
pub struct LearningRouteRequest {
    pub project_id: String,
    pub sample_id: String,
}
#[derive(Clone, Serialize, Deserialize, JsonSchema)]
pub struct RollbackLearningModelRequest {
    pub project_id: String,
    /// The active model to roll back. Defaults to the more recently trained of the deployment's
    /// active and previous model. Repeating the request after that model was rolled back returns
    /// the current deployment instead of restoring it.
    #[serde(default)]
    pub expected_artifact_id: Option<String>,
    /// Deployment generation the caller observed. The rollback fails when the deployment changed.
    #[serde(default)]
    pub expected_generation: Option<i64>,
}
#[derive(Clone, Serialize, Deserialize, JsonSchema)]
pub struct RejectLearningCandidateRequest {
    pub project_id: String,
    /// Learning project generation the caller observed. The rejection fails when the project
    /// changed in the meantime.
    #[serde(default)]
    pub expected_generation: Option<i64>,
}
#[derive(Clone, Serialize, Deserialize, JsonSchema)]
pub struct LearningProjectResult {
    pub project: LearningProject,
    pub next_actions: LearningNextActions,
    pub cycles: Vec<LearningCycle>,
    pub experiment: Option<AutoTrainingResult>,
    pub comparison: Option<LearningComparison>,
}

#[cfg(feature = "training")]
pub fn project_repository(
    context: &ExecutionContext,
    id: &str,
) -> Result<flow_like_ml_runtime::TrainingRepository> {
    project_ledger(context, id).map(|(repo, _)| repo)
}

/// The ledger holding learning project `id` with this run's project string, and its scope.
#[cfg(feature = "training")]
pub(crate) fn project_ledger(
    context: &ExecutionContext,
    id: &str,
) -> Result<(
    flow_like_ml_runtime::TrainingRepository,
    super::lifecycle::LedgerScope,
)> {
    use super::auto_training::{existing, missing_from_ledgers};
    let located = super::lifecycle::locate_repository(context, |repo, _| {
        Ok(existing(repo.get_learning_project(id))?.is_some())
    })?;
    let Some(ledger) = located else {
        return Err(missing_from_ledgers(
            context,
            &format!("Learning project {id}"),
            &super::published::learning_pointer_path(id),
            "Route Learning Project reads its published champion",
        ));
    };
    if ledger.repo.get_learning_project(id)?.request.stream.project_id != ledger.project {
        return Err(anyhow!(
            "Learning project belongs to a different app or board"
        ));
    }
    Ok((ledger.repo, ledger.scope))
}

#[cfg(feature = "training")]
pub fn training_template(project: &LearningProject) -> Result<AutoTrainRequest> {
    Ok(flow_like_types::json::from_value(
        project.request.source["training"].clone(),
    )?)
}

#[cfg(feature = "training")]
fn result(
    repo: &flow_like_ml_runtime::TrainingRepository,
    id: &str,
) -> Result<LearningProjectResult> {
    let project = repo.get_learning_project(id)?;
    let cycles = repo.learning_cycles(id)?;
    let latest = project
        .active_cycle_id
        .as_ref()
        .and_then(|cycle_id| cycles.iter().find(|cycle| &cycle.id == cycle_id))
        .or_else(|| cycles.last());
    let experiment = latest
        .and_then(|cycle| cycle.experiment_id.as_ref())
        .map(|id| repo.experiment_result(id).map(AutoTrainingResult::from))
        .transpose()?;
    let comparison = latest
        .map(|cycle| match repo.get_learning_comparison(&cycle.id) {
            Ok(value) => Ok(Some(value)),
            Err(flow_like_ml_runtime::Error::NotFound(_)) => Ok(None),
            Err(error) => Err(error),
        })
        .transpose()?
        .flatten();
    Ok(LearningProjectResult {
        project,
        next_actions: repo.learning_next_actions(id, flow_like_ml_runtime::now_ms())?,
        cycles,
        experiment,
        comparison,
    })
}

/// Creates the project in the ledger its sources call for: the App ledger when every source
/// reads the project database, otherwise the executing user's private ledger. It starts with a
/// published Teacher pointer.
#[cfg(feature = "training")]
async fn create(
    context: &mut ExecutionContext,
    mut request: CreateLearningProjectRequest,
) -> Result<LearningProjectResult> {
    use super::lifecycle::{LEDGER_SCOPE_KEY, LedgerScope, StreamKind};
    super::lifecycle::ensure_training_available(context)?;
    if let Some(rolling) = &request.rolling_time_split {
        rolling.apply(
            &mut request.training.dataset.split,
            flow_like_ml_runtime::now_ms(),
        )?;
    }
    let spec = &request.training.dataset.spec;
    let mut goals = request
        .training
        .goals
        .clone()
        .unwrap_or_else(|| flow_like_ml_runtime::auto_training::default_goals(spec));
    if request.training.ordered_target && request.training.goals.is_none() {
        goals.primary_metric = "mean_rank_error".into();
        goals.direction = flow_like_ml_runtime::MetricDirection::Minimize;
    }
    goals.minimum_audited_samples = goals
        .minimum_audited_samples
        .max(request.policy.minimum_audit_samples);
    if goals.bounds.is_empty() {
        return Err(anyhow!(
            "Learning projects require explicit quality bounds; set training.goals.bounds"
        ));
    }
    flow_like_ml_runtime::auto_training::validate_goals(spec, &goals)?;
    let pinned =
        super::auto_training_tables::pin_table_source(context, &request.training.dataset.source)
            .await?;
    let pinned_feature_sources =
        super::feature_engineering::pin_feature_sources(context, &request.training.feature_sources)
            .await?;
    let digests = super::auto_training_tables::scope_digests(context).await?;
    let scope =
        super::auto_training::source_ledger_scope(&digests, &pinned, &pinned_feature_sources);
    if scope == LedgerScope::User {
        super::auto_training::ensure_private_table_writes(
            context,
            &digests,
            &pinned,
            &request.training.tables,
        )
        .await?;
    }
    let (repo, project_string) = super::lifecycle::scoped_repository(context, scope)?;
    let stream = super::lifecycle::InspectionContext {
        stream_id: scope.new_stream_id(StreamKind::Learning),
        spec: spec.clone(),
    }
    .key(project_string)?;
    if scope == LedgerScope::User {
        let named = request
            .training
            .pretrained
            .as_ref()
            .map(|source| source.source_id.as_str());
        super::auto_training::adopt_pretrained_sources(
            context,
            &repo,
            &stream.project_id,
            named,
            None,
        )
        .await?;
    }
    let mut source = json!({"training":request.training,"pinned_source":pinned,"pinned_feature_sources":pinned_feature_sources,"automatic_promotion":request.automatic_promotion,"rolling_time_split":request.rolling_time_split});
    source[LEDGER_SCOPE_KEY] = json!(scope);
    let project = repo.create_learning_project(
        LearningProjectRequest {
            deployment_id: format!("deployment:{}", stream.stream_id),
            stream,
            spec: json!(spec),
            source,
            goals,
            policy: request.policy,
            budget: request.budget,
        },
        flow_like_ml_runtime::now_ms(),
    )?;
    let created = result(&repo, &project.id)?;
    published_champion(
        context,
        &repo,
        scope,
        &project.id,
        super::publisher::PublishReason::StateChange,
        created,
    )
    .await
}

#[cfg(feature = "training")]
async fn get(
    context: &mut ExecutionContext,
    request: LearningProjectId,
) -> Result<LearningProjectResult> {
    let (repo, scope) = project_ledger(context, &request.project_id)?;
    let read = result(&repo, &request.project_id)?;
    published_champion(
        context,
        &repo,
        scope,
        &request.project_id,
        super::publisher::PublishReason::SelfRepair,
        read,
    )
    .await
}

/// Publishes the project's champion pointer, and the champion when there is one, after the
/// project changed or, with `SelfRepair`, when a read finds the files missing or stale. A
/// failure blocks with `champion_model_not_published` and never fails the caller.
#[cfg(feature = "training")]
async fn published_champion(
    context: &mut ExecutionContext,
    repo: &flow_like_ml_runtime::TrainingRepository,
    scope: super::lifecycle::LedgerScope,
    id: &str,
    reason: super::publisher::PublishReason,
    mut output: LearningProjectResult,
) -> Result<LearningProjectResult> {
    use super::publisher::{PublishOutcome, ensure_champion_published};
    match ensure_champion_published(context, repo, scope, id, reason).await {
        PublishOutcome::Failed { error } => output
            .next_actions
            .blocking
            .push(format!("champion_model_not_published: {error}")),
        PublishOutcome::Skipped { reason: skipped } => context.log_message(
            &format!("The champion of learning project {id} was not published: {skipped}"),
            LogLevel::Warn,
        ),
        PublishOutcome::Published(_) | PublishOutcome::Unchanged => {}
    }
    Ok(output)
}

#[cfg(feature = "training")]
async fn observe(
    context: &mut ExecutionContext,
    request: LearningObservationsRequest,
) -> Result<LearningProjectResult> {
    let repo = project_repository(context, &request.project_id)?;
    if request.observations.len() > 4096 {
        return Err(anyhow!(
            "An observation batch may contain at most 4096 rows"
        ));
    }
    let project = repo.get_learning_project(&request.project_id)?;
    // Canary replay rebuilds candidate traffic and window history from each raw row.
    if project.request.policy.canary_fraction > 0. {
        let mapping = training_template(&project)?.dataset.mapping;
        let units = inference::pinned_temporal_units(&project.request.source)?;
        for observation in &request.observations {
            inference::validate_canary_sample(
                observation,
                &mapping,
                &project.request.stream.stream_id,
                &units,
            )?;
        }
    }
    for observation in request.observations {
        repo.record_learning_observation(&request.project_id, &observation)?;
    }
    result(&repo, &request.project_id)
}

#[cfg(feature = "training")]
async fn review(
    context: &mut ExecutionContext,
    request: LearningReviewRequest,
) -> Result<LearningProjectResult> {
    let repo = project_repository(context, &request.project_id)?;
    repo.record_learning_review(&request.project_id, &request.review)?;
    result(&repo, &request.project_id)
}

#[cfg(feature = "training")]
async fn select_reviews(
    context: &mut ExecutionContext,
    request: LearningReviewSelectionRequest,
) -> Result<Vec<LearningReviewItem>> {
    Ok(
        project_repository(context, &request.project_id)?.select_learning_reviews(
            &request.project_id,
            &request.selection_key,
            flow_like_ml_runtime::now_ms(),
        )?,
    )
}

#[cfg(feature = "training")]
async fn errors(
    context: &mut ExecutionContext,
    request: LearningProjectId,
) -> Result<LearningErrorAnalysis> {
    Ok(project_repository(context, &request.project_id)?
        .learning_error_analysis(&request.project_id)?)
}

#[cfg(feature = "training")]
async fn changed(
    context: &mut ExecutionContext,
    request: LearningProductChangeRequest,
) -> Result<LearningProjectResult> {
    let repo = project_repository(context, &request.project_id)?;
    repo.record_learning_product_change(
        &request.project_id,
        &request.change_id,
        &request.description,
        flow_like_ml_runtime::now_ms(),
    )?;
    result(&repo, &request.project_id)
}

#[cfg(feature = "training")]
async fn pause(
    context: &mut ExecutionContext,
    request: LearningProjectId,
) -> Result<LearningProjectResult> {
    let (repo, scope) = project_ledger(context, &request.project_id)?;
    let project = repo.pause_learning_project(
        &request.project_id,
        repo.get_learning_project(&request.project_id)?.generation,
        flow_like_ml_runtime::now_ms(),
    )?;
    if let Some(cycle_id) = project.active_cycle_id {
        if let Some(experiment_id) = repo.get_learning_cycle(&cycle_id)?.experiment_id {
            repo.cancel_experiment(&experiment_id, flow_like_ml_runtime::now_ms())?;
        }
    }
    let paused = result(&repo, &request.project_id)?;
    published_champion(
        context,
        &repo,
        scope,
        &request.project_id,
        super::publisher::PublishReason::StateChange,
        paused,
    )
    .await
}

#[cfg(feature = "training")]
async fn resume(
    context: &mut ExecutionContext,
    request: LearningProjectId,
) -> Result<LearningProjectResult> {
    let (repo, scope) = project_ledger(context, &request.project_id)?;
    repo.resume_learning_project(
        &request.project_id,
        repo.get_learning_project(&request.project_id)?.generation,
        flow_like_ml_runtime::now_ms(),
    )?;
    let resumed = result(&repo, &request.project_id)?;
    published_champion(
        context,
        &repo,
        scope,
        &request.project_id,
        super::publisher::PublishReason::StateChange,
        resumed,
    )
    .await
}

/// The ledger decides while it holds the project under this run's project string and no
/// champion pointer in the run's local store orders after it; otherwise the published champion
/// pointer routes, to the champion or, while paused or without one, to the teacher.
#[cfg(feature = "inference")]
async fn route(
    context: &mut ExecutionContext,
    request: LearningRouteRequest,
) -> Result<LearningRoute> {
    use super::published::{
        ModelStorage, PointerKind, newer_published_generation, published_learning_route,
    };
    let id = &request.project_id;
    let located = super::lifecycle::locate_repository_for_inference(context, |repo, project| {
        Ok(super::auto_training::existing(repo.get_learning_project(id))?
            .is_some_and(|learning| learning.request.stream.project_id == project))
    })?;
    let mut storage: Option<ModelStorage> = None;
    if let Some(ledger) = located {
        let project = ledger.repo.get_learning_project(id)?;
        let order = (
            deployment_generation(&ledger.repo, &project)?,
            project.generation,
        );
        if !newer_published_generation(context, PointerKind::Learning, ledger.scope, id, order)
            .await
        {
            return Ok(ledger.repo.learning_route(id, &request.sample_id)?);
        }
        storage = Some(ledger.scope.into());
    }
    published_learning_route(context, id, storage)
        .await?
        .ok_or_else(|| {
            anyhow!(
                "Learning project {id} is not in this device's training ledgers and has no published champion at storage/{}",
                super::published::learning_pointer_path(id)
            )
        })
}

#[cfg(feature = "training")]
async fn rollback(
    context: &mut ExecutionContext,
    request: RollbackLearningModelRequest,
) -> Result<LearningProjectResult> {
    let (repo, scope) = project_ledger(context, &request.project_id)?;
    let at_ms = flow_like_ml_runtime::now_ms();
    match (&request.expected_artifact_id, request.expected_generation) {
        (Some(_), Some(_)) => {
            return Err(anyhow!(
                "Rollback Learning Model accepts expected_artifact_id or expected_generation, not both"
            ));
        }
        (Some(artifact_id), None) => {
            repo.rollback_learning_project_from(&request.project_id, artifact_id, at_ms)?;
        }
        (None, Some(generation)) => {
            repo.rollback_learning_project(&request.project_id, generation, at_ms)?;
        }
        (None, None) => {
            let project = repo.get_learning_project(&request.project_id)?;
            let model = newest_deployed_model(&repo, &project)?;
            repo.rollback_learning_project_from(&request.project_id, &model, at_ms)?;
        }
    }
    let rolled_back = result(&repo, &request.project_id)?;
    published_champion(
        context,
        &repo,
        scope,
        &request.project_id,
        super::publisher::PublishReason::StateChange,
        rolled_back,
    )
    .await
}

/// Whichever of the deployment's active and previous model the project trained last: the active
/// one after a promotion, the previous one after a rollback. Rolling it back is retry-safe.
#[cfg(feature = "training")]
fn newest_deployed_model(
    repo: &flow_like_ml_runtime::TrainingRepository,
    project: &LearningProject,
) -> Result<String> {
    let deployment = match repo.get_deployment(&project.request.deployment_id) {
        Ok(deployment) => deployment,
        Err(flow_like_ml_runtime::Error::NotFound(_)) => {
            return Err(anyhow!(
                "Learning project {} has no deployed model to roll back",
                project.id
            ));
        }
        Err(error) => return Err(error.into()),
    };
    let cycles = repo.learning_cycles(&project.id)?;
    let trained = |artifact_id: &str| {
        cycles
            .iter()
            .position(|cycle| cycle.artifact_id.as_deref() == Some(artifact_id))
    };
    Ok(match deployment.previous_artifact_id {
        Some(previous)
            if trained(previous.as_str()) > trained(deployment.active_artifact_id.as_str()) =>
        {
            previous
        }
        _ => deployment.active_artifact_id,
    })
}

#[cfg(feature = "training")]
async fn reject(
    context: &mut ExecutionContext,
    request: RejectLearningCandidateRequest,
) -> Result<LearningProjectResult> {
    let repo = project_repository(context, &request.project_id)?;
    repo.reject_learning_candidate(
        &request.project_id,
        request.expected_generation,
        flow_like_ml_runtime::now_ms(),
    )?;
    result(&repo, &request.project_id)
}

#[cfg(feature = "inference")]
fn deployment_generation(
    repo: &flow_like_ml_runtime::TrainingRepository,
    project: &LearningProject,
) -> Result<i64> {
    match repo.get_deployment(&project.request.deployment_id) {
        Ok(value) => Ok(value.generation),
        Err(flow_like_ml_runtime::Error::NotFound(_)) => Ok(0),
        Err(error) => Err(error.into()),
    }
}

/// The ledger's run lock for `learning:{project}`, at the path the runtime's LocalRunLock uses
/// for that key. `None` while another step of the project holds it, in any process.
#[cfg(feature = "training")]
fn try_lock_learning_project(
    repo: &flow_like_ml_runtime::TrainingRepository,
    project_id: &str,
) -> Result<Option<std::fs::File>> {
    let database = repo.path().canonicalize()?;
    let directory = database
        .parent()
        .ok_or_else(|| {
            anyhow!(
                "Training ledger {} has no parent directory",
                database.display()
            )
        })?
        .join(".training-locks");
    std::fs::create_dir_all(&directory)?;
    if !std::fs::symlink_metadata(&directory)?.is_dir() {
        return Err(anyhow!(
            "Training lock directory {} must be a real directory",
            directory.display()
        ));
    }
    let mut identity = database.as_os_str().as_encoded_bytes().to_vec();
    identity.push(0);
    identity.extend_from_slice(format!("learning:{project_id}").as_bytes());
    let path = directory.join(format!(
        "{}.lock",
        flow_like_ml_core::content_digest(&identity)
    ));
    if std::fs::symlink_metadata(&path).is_ok_and(|metadata| !metadata.is_file()) {
        return Err(anyhow!(
            "Training lock {} must be a regular file",
            path.display()
        ));
    }
    let file = std::fs::OpenOptions::new()
        .read(true)
        .write(true)
        .create(true)
        .truncate(false)
        .open(&path)?;
    match file.try_lock() {
        Ok(()) => Ok(Some(file)),
        Err(std::fs::TryLockError::WouldBlock) => Ok(None),
        Err(std::fs::TryLockError::Error(error)) => Err(anyhow!(
            "Could not lock learning project {project_id} at {}: {error}",
            path.display()
        )),
    }
}

/// Prepare at most one experiment. A workflow timer can call this again after new evidence arrives.
#[cfg(feature = "training")]
pub async fn prepare_learning_cycle(
    context: &mut ExecutionContext,
    id: &str,
) -> Result<LearningProjectResult> {
    use super::auto_training_tables::{PinnedTableSource, TableLabelOverride};
    use flow_like_ml_runtime::{LabelSource, now_ms};
    let repo = project_repository(context, id)?;
    // An overlapping step would re-read the sources and import another snapshot.
    let Some(_preparing) = try_lock_learning_project(&repo, id)? else {
        return result(&repo, id);
    };
    let project = repo.get_learning_project(id)?;
    if matches!(
        project.state,
        LearningState::Paused
            | LearningState::Shadow
            | LearningState::Canary
            | LearningState::BudgetExhausted
    ) {
        return result(&repo, id);
    }
    let mut template = training_template(&project)?;
    let cycle = if let Some(cycle_id) = &project.active_cycle_id {
        repo.get_learning_cycle(cycle_id)?
    } else {
        if !repo.learning_next_actions(id, now_ms())?.ready_to_train {
            return result(&repo, id);
        }
        repo.reserve_learning_cycle(
            id,
            &format!("cycle:{}", project.usage.cycles_started),
            template.budget.clone(),
            now_ms(),
        )?
    };
    if cycle.experiment_id.is_some() {
        return result(&repo, id);
    }
    if let Some(experiment) = repo.find_learning_cycle_experiment(&cycle.id)? {
        repo.attach_learning_experiment(&cycle.id, &experiment.id, now_ms())?;
        return result(&repo, id);
    }
    let prepared: Result<String> = async {
        let pinned: PinnedTableSource =
            flow_like_types::json::from_value(project.request.source["pinned_source"].clone())?;
        template.dataset.source =
            super::auto_training_tables::reopen_latest_source(context, &pinned).await?;
        let sides: Vec<super::feature_engineering::PinnedFeatureSource> =
            flow_like_types::json::from_value(
                project
                    .request
                    .source
                    .get("pinned_feature_sources")
                    .cloned()
                    .unwrap_or_else(|| json!([])),
            )?;
        template.feature_sources.clear();
        for side in sides {
            template
                .feature_sources
                .push(super::feature_engineering::FeatureSourceInput {
                    alias: side.alias,
                    allowed_columns: side.allowed_columns,
                    source: super::auto_training_tables::reopen_latest_source(
                        context,
                        &side.source,
                    )
                    .await?,
                });
        }
        let exclusions = repo.learning_dataset_exclusions(id)?;
        merge_dataset_exclusions(&mut template.dataset.selection, exclusions);
        if let Some(value) = project
            .request
            .source
            .get("rolling_time_split")
            .filter(|value| !value.is_null())
        {
            let rolling: RollingTimeSplit = flow_like_types::json::from_value(value.clone())?;
            rolling.apply(&mut template.dataset.split, cycle.created_at_ms)?;
        }
        for review in repo.learning_training_reviews_at(id, now_ms())? {
            let provenance = match review.source {
                LabelSource::Reviewed => flow_like_ml_core::LabelProvenance::Reviewed {
                    reviewer: review.reviewer,
                    reviewed_at_ms: review.available_at_ms,
                },
                LabelSource::ObservedOutcome => flow_like_ml_core::LabelProvenance::Measured {
                    source: review.reviewer,
                },
                LabelSource::Teacher => {
                    return Err(anyhow!(
                        "Teacher labels cannot enter a reviewed outcome overlay"
                    ));
                }
            };
            template.dataset.label_overrides.insert(
                review.sample_id,
                TableLabelOverride {
                    annotation: flow_like_types::json::from_value(review.annotation)?,
                    provenance,
                    annotation_revision: i64::try_from(
                        review
                            .revision
                            .checked_add(1)
                            .ok_or_else(|| anyhow!("Review revision overflow"))?,
                    )
                    .map_err(|_| anyhow!("Review revision exceeds the supported range"))?,
                    available_at_ms: Some(review.available_at_ms),
                    outcome: review
                        .outcome
                        .map(flow_like_types::json::from_value)
                        .transpose()?,
                },
            );
        }
        template.goals = Some(project.request.goals.clone());
        template.budget = cycle.budget.clone();
        template.learning_cycle_id = Some(cycle.id.clone());
        let experiment = super::auto_training::start_auto_training_scoped(
            context,
            template,
            false,
            Some(project.request.stream.clone()),
        )
        .await?;
        repo.attach_learning_experiment(&cycle.id, &experiment.experiment_id, now_ms())?;
        Ok(experiment.experiment_id)
    }
    .await;
    if let Err(error) = prepared {
        // A saved experiment keeps its reservation and can be attached after a restart.
        if repo.find_learning_cycle_experiment(&cycle.id)?.is_none() {
            repo.abort_learning_cycle(&cycle.id, &error.to_string(), now_ms())?;
        }
        return Err(error);
    }
    result(&repo, id)
}

#[cfg(feature = "training")]
fn merge_dataset_exclusions(
    selection: &mut super::auto_training_tables::DatasetSelection,
    exclusions: LearningDatasetExclusions,
) {
    for (existing, additional) in [
        (
            &mut selection.excluded_ids,
            exclusions.sealed_audit_sample_ids,
        ),
        (
            &mut selection.excluded_groups,
            exclusions.sealed_audit_group_ids,
        ),
        (
            &mut selection.ineligible_test_ids,
            exclusions.previously_trained_sample_ids,
        ),
        (
            &mut selection.ineligible_test_groups,
            exclusions.previously_trained_group_ids,
        ),
    ] {
        existing.extend(additional);
        existing.sort();
        existing.dedup();
    }
}

#[cfg(feature = "training")]
fn audit_champion(
    repo: &flow_like_ml_runtime::TrainingRepository,
    project: &LearningProject,
    cycle: &LearningCycle,
    cancel: &CancelFlag,
) -> Result<()> {
    use flow_like_ml_runtime::{
        ExperimentResult,
        auto_training::{SearchPredictorFactory, prediction_record_from_output},
    };
    let Some(artifact_id) = &project.champion_artifact_id else {
        return Ok(());
    };
    if cycle.artifact_id.as_ref() == Some(artifact_id) {
        return Ok(());
    }
    let recorded: std::collections::HashSet<_> = repo
        .predictions(artifact_id)?
        .into_iter()
        .map(|prediction| prediction.sample_id)
        .collect();
    // The comparison reads champion predictions for the sealed audit samples only.
    if cycle
        .audit_sample_ids
        .iter()
        .all(|sample_id| recorded.contains(sample_id))
    {
        return Ok(());
    }
    let template = training_template(project)?;
    let mut budget = ReplayBudget::new(
        "Champion audit replay",
        cancel,
        template.budget.worker_limits.maximum_duration_ms,
    );
    budget.check()?;
    let experiment_id = cycle
        .experiment_id
        .as_ref()
        .ok_or_else(|| anyhow!("Learning cycle has no experiment"))?;
    let state: ExperimentResult = repo.experiment_result(experiment_id)?;
    let selected = state
        .experiment
        .selected_trial_id
        .as_ref()
        .ok_or_else(|| anyhow!("A candidate must be frozen before comparing its champion"))?;
    let trial = state
        .trials
        .iter()
        .find(|trial| &trial.id == selected)
        .ok_or_else(|| anyhow!("Selected learning trial is missing"))?;
    let snapshot = repo.get_snapshot_limited(
        &trial.dataset_snapshot_id,
        state.experiment.request.budget.maximum_dataset_bytes,
    )?;
    let artifact = repo.get_artifact(artifact_id)?;
    let preprocessing: super::auto_training_tables::TablePreprocessing =
        flow_like_types::json::from_value(artifact.manifest["preprocessing_manifest"].clone())?;
    let mut spec: flow_like_ml_core::InspectionSpec =
        flow_like_types::json::from_value(artifact.manifest["inspection_spec"].clone())?;
    let rows = snapshot
        .test
        .iter()
        .map(|sample| {
            sample.payload.get("feature_row").cloned().ok_or_else(|| {
                anyhow!(
                    "This dataset predates raw-feature audit replay; prepare a new learning cycle"
                )
            })
        })
        .collect::<Result<Vec<_>>>()?;
    let inputs = super::auto_training_tables::transform_table_rows(
        &preprocessing,
        &rows,
        template.dataset.budget.maximum_tensor_elements,
    )?;
    let mut anomaly_scale = 1.;
    for previous in repo.learning_cycles(&project.id)? {
        if previous.artifact_id.as_ref() != Some(artifact_id) {
            continue;
        }
        if let Some(id) = previous.experiment_id {
            if let Some(trial) = repo
                .experiment_result(&id)?
                .trials
                .iter()
                .find(|trial| trial.artifact_id.as_ref() == Some(artifact_id))
            {
                anomaly_scale = trial
                    .validation_metrics
                    .get("anomaly_scale")
                    .copied()
                    .unwrap_or(1.);
            }
        }
    }
    let mut predictor = None;
    for (row, input) in snapshot.test.iter().zip(inputs) {
        if recorded.contains(&row.id) {
            continue;
        }
        budget.check()?;
        let mut prepared = row.clone();
        spec.input_shape = input.shape.clone();
        let sample_payload = if prepared.payload.get("sample").is_some() {
            &mut prepared.payload["sample"]
        } else {
            &mut prepared.payload
        };
        sample_payload["input"] = json!(input);
        let mut batched = input;
        batched.shape.insert(0, 1);
        if predictor.is_none() {
            predictor = Some(
                super::auto_training_legacy::CatalogSearchPredictorFactory.load(
                    repo,
                    artifact_id,
                    &template.compute,
                )?,
            );
        }
        let model = predictor
            .as_mut()
            .ok_or_else(|| anyhow!("Champion model {artifact_id} is not loaded"))?;
        let predicted = std::time::Instant::now();
        let output = model.predict(&batched)?;
        let record = prediction_record_from_output(
            &prepared,
            &spec,
            &project.request.goals.task,
            artifact_id,
            &output,
            anomaly_scale,
            predicted.elapsed().as_secs_f64() * 1000.,
        )?;
        repo.record_prediction(&record)?;
        budget.recorded();
    }
    Ok(())
}

/// Bounds one audit replay call. Cancellation stops it before its next prediction. The step
/// budget counts from the call's first recorded prediction, so setup and model loading slower
/// than the budget still let every call record one.
#[cfg(feature = "training")]
struct ReplayBudget<'a> {
    replay: &'static str,
    cancel: &'a CancelFlag,
    limit_ms: u64,
    first_recorded: Option<std::time::Instant>,
}

#[cfg(feature = "training")]
impl<'a> ReplayBudget<'a> {
    fn new(replay: &'static str, cancel: &'a CancelFlag, limit_ms: u64) -> Self {
        Self {
            replay,
            cancel,
            limit_ms,
            first_recorded: None,
        }
    }

    fn check(&self) -> Result<()> {
        if self.cancel.is_cancelled() {
            return Err(anyhow!(
                "{} was cancelled; recorded predictions resume on the next step",
                self.replay
            ));
        }
        if self
            .first_recorded
            .is_some_and(|at| at.elapsed().as_millis() > u128::from(self.limit_ms))
        {
            return Err(anyhow!(
                "{} exceeded its {} ms step budget; recorded predictions resume on the next step",
                self.replay,
                self.limit_ms
            ));
        }
        Ok(())
    }

    fn recorded(&mut self) {
        self.first_recorded
            .get_or_insert_with(std::time::Instant::now);
    }
}

/// The executor drops a node future when its run is cancelled, before a select on the token
/// can run, so the guard also stops the detached blocking work in that case.
#[cfg(feature = "training")]
async fn run_cancellable<T: Send + 'static>(
    token: Option<flow_like_types::tokio_util::sync::CancellationToken>,
    on_cancel: impl FnOnce() + Send + 'static,
    cancelled: &str,
    work: impl FnOnce() -> Result<T> + Send + 'static,
) -> Result<T> {
    let guard = CancelOnDrop::new(on_cancel);
    let mut task = tokio::task::spawn_blocking(work);
    let done = match token {
        Some(token) => tokio::select! {
            done = &mut task => done,
            _ = token.cancelled() => {
                drop(guard);
                return Err(anyhow!("{cancelled}"));
            }
        },
        None => task.await,
    };
    guard.disarm();
    done?
}

/// Audit replays record one prediction at a time and stop before the next once cancelled.
#[cfg(feature = "training")]
async fn run_audit(
    context: &ExecutionContext,
    work: impl FnOnce(CancelFlag) -> Result<()> + Send + 'static,
) -> Result<()> {
    let cancel = CancelFlag::default();
    let stop = cancel.clone();
    run_cancellable(
        context.get_cancellation_token(),
        move || stop.cancel(),
        "Learning audit replay was cancelled; recorded predictions resume on the next step",
        move || work(cancel),
    )
    .await
}

/// A stopped experiment that cannot resume settles with its cycle, so the project can start
/// another. Storage failures are retried by the next step instead.
#[cfg(feature = "training")]
fn settles_after_failed_resume(
    status: flow_like_ml_runtime::ExperimentStatus,
    error: &flow_like_types::Error,
) -> bool {
    use flow_like_ml_runtime::{Error, ExperimentStatus};
    matches!(
        status,
        ExperimentStatus::Cancelled | ExperimentStatus::CancelRequested
    ) && !matches!(
        error.downcast_ref::<Error>(),
        Some(Error::Sql(_) | Error::Io(_))
    )
}

/// Whether a review of one of the cycle's audit samples became available after `since_ms`.
#[cfg(feature = "training")]
fn audit_reviewed_since(
    repo: &flow_like_ml_runtime::TrainingRepository,
    cycle: &LearningCycle,
    since_ms: i64,
    at_ms: i64,
) -> Result<bool> {
    let audit: std::collections::HashSet<&str> =
        cycle.audit_sample_ids.iter().map(String::as_str).collect();
    Ok(repo
        .learning_training_reviews_at(&cycle.project_id, at_ms)?
        .iter()
        .any(|review| {
            review.available_at_ms > since_ms && audit.contains(review.sample_id.as_str())
        }))
}

/// Settles, compares and promotes the active cycle as far as its evidence allows, then
/// publishes the champion the project ends with.
#[cfg(feature = "training")]
pub async fn finish_learning_cycle(
    context: &mut ExecutionContext,
    id: &str,
) -> Result<LearningProjectResult> {
    let finished = finish_with(
        context,
        id,
        flow_like_ml_runtime::TrainingRepository::compare_learning_cycle,
    )
    .await?;
    let (repo, scope) = project_ledger(context, id)?;
    published_champion(
        context,
        &repo,
        scope,
        id,
        super::publisher::PublishReason::StateChange,
        finished,
    )
    .await
}

/// `compare` evaluates the settled candidate and the champion on the cycle's sealed audit.
#[cfg(feature = "training")]
async fn finish_with(
    context: &mut ExecutionContext,
    id: &str,
    compare: fn(
        &flow_like_ml_runtime::TrainingRepository,
        &str,
        i64,
    ) -> flow_like_ml_runtime::Result<LearningComparison>,
) -> Result<LearningProjectResult> {
    use flow_like_ml_runtime::{ExperimentStatus, now_ms};
    let repo = project_repository(context, id)?;
    let project = repo.get_learning_project(id)?;
    if project.state == LearningState::Paused {
        return result(&repo, id);
    }
    let cycles = repo.learning_cycles(id)?;
    let Some(mut cycle) = project
        .active_cycle_id
        .as_ref()
        .and_then(|cycle_id| cycles.iter().find(|cycle| &cycle.id == cycle_id))
        .or_else(|| cycles.last())
        .cloned()
    else {
        return result(&repo, id);
    };
    let Some(experiment_id) = &cycle.experiment_id else {
        return result(&repo, id);
    };
    let experiment = repo.get_experiment(experiment_id)?;
    if matches!(
        experiment.status,
        ExperimentStatus::Running
            | ExperimentStatus::Evaluating
            | ExperimentStatus::CancelRequested
    ) {
        return result(&repo, id);
    }
    if cycle.state == LearningCycleState::Attached {
        cycle = repo.settle_learning_cycle(&cycle.id, now_ms())?;
    }
    if cycle.evaluation_id.is_none() {
        return result(&repo, id);
    }
    let fresh = repo.get_learning_project(id)?;
    if fresh.candidate_artifact_id != cycle.artifact_id {
        return result(&repo, id);
    }
    let saved = match repo.get_learning_comparison(&cycle.id) {
        Ok(saved) => Some(saved),
        Err(flow_like_ml_runtime::Error::NotFound(_)) => None,
        Err(error) => return Err(error.into()),
    };
    // A waiting candidate keeps its sealed comparison until a review of one of its audit samples
    // becomes available after it; compare checks that review against the truth the cycle sealed.
    // Promotion compares again before deciding.
    let waiting =
        saved.filter(|_| matches!(fresh.state, LearningState::Shadow | LearningState::Canary));
    let reviewed = match &waiting {
        Some(saved) => audit_reviewed_since(&repo, &cycle, saved.evaluated_at_ms, now_ms())?,
        None => false,
    };
    let comparison = match waiting {
        Some(saved) if !reviewed => saved,
        waiting => {
            let work_repo = repo.clone();
            let work_project = fresh.clone();
            let work_cycle = cycle.clone();
            run_audit(context, move |cancel| {
                audit_champion(&work_repo, &work_project, &work_cycle, &cancel)
            })
            .await?;
            match (compare(&repo, &cycle.id, now_ms()), waiting) {
                (Ok(comparison), _) => comparison,
                // Any review recorded while compare ran moves the project generation. The sealed
                // comparison keeps the canary audit going until the next step compares again.
                (Err(flow_like_ml_runtime::Error::Conflict(reason)), Some(saved)) => {
                    context.log_message(
                        &format!(
                            "Learning cycle {} keeps its sealed comparison until the next step compares again: {reason}",
                            cycle.id
                        ),
                        LogLevel::Warn,
                    );
                    saved
                }
                (Err(error), _) => return Err(error.into()),
            }
        }
    };
    if comparison.eligible && fresh.state == LearningState::Canary {
        let work_repo = repo.clone();
        let work_project = fresh.clone();
        run_audit(context, move |cancel| {
            inference::audit_canary(&work_repo, &work_project, &cancel)
        })
        .await?;
        if repo.get_learning_project(id)?.candidate_artifact_id != cycle.artifact_id {
            return result(&repo, id);
        }
    }
    let can_promote =
        fresh.state != LearningState::Canary || repo.learning_canary_ready(id, now_ms())?;
    if comparison.eligible
        && can_promote
        && fresh.request.source["automatic_promotion"].as_bool() == Some(true)
    {
        if let Err(error) =
            repo.promote_learning_cycle(&cycle.id, deployment_generation(&repo, &fresh)?, now_ms())
        {
            let current = repo.get_learning_project(id)?;
            let audited = repo.get_learning_comparison(&cycle.id)?;
            if current.candidate_artifact_id.is_some() || audited.eligible {
                return Err(error.into());
            }
            // A failed canary gate is a recorded decision; the champion continues serving.
            return result(&repo, id);
        }
    }
    let mut output = result(&repo, id)?;
    if comparison.eligible && !can_promote {
        output
            .next_actions
            .blocking
            .push("canary_requires_reviewed_traffic_and_both_model_predictions".into());
    }
    Ok(output)
}

#[cfg(feature = "training")]
async fn step(
    context: &mut ExecutionContext,
    request: LearningProjectId,
) -> Result<LearningProjectResult> {
    step_with(
        context,
        request,
        super::auto_training::resume_saved_experiment,
    )
    .await
}

/// `resume` readies the active cycle's saved experiment before its controller runs again.
#[cfg(feature = "training")]
async fn step_with(
    context: &mut ExecutionContext,
    request: LearningProjectId,
    resume: fn(&flow_like_ml_runtime::TrainingRepository, &str) -> Result<()>,
) -> Result<LearningProjectResult> {
    use flow_like_ml_runtime::{ExperimentStatus, now_ms};
    let prepared = prepare_learning_cycle(context, &request.project_id).await?;
    if prepared.project.state == LearningState::Paused {
        return Ok(prepared);
    }
    if let Some(experiment) = prepared.experiment
        && matches!(
            experiment.status,
            ExperimentStatus::Running
                | ExperimentStatus::Evaluating
                | ExperimentStatus::Cancelled
                | ExperimentStatus::CancelRequested
                | ExperimentStatus::BudgetExhausted
        )
        && prepared.project.active_cycle_id.is_some()
    {
        let repo = project_repository(context, &request.project_id)?;
        let id = experiment.experiment_id;
        match resume(&repo, &id) {
            Ok(()) => {
                let budget = repo.get_experiment(&id)?.request.budget;
                let controller = super::auto_training::controller(repo.clone(), &budget)?;
                let cancel_repo = repo.clone();
                let cancel_id = id.clone();
                let run_id = id.clone();
                run_cancellable(
                    context.get_cancellation_token(),
                    move || {
                        let _ = cancel_repo.cancel_experiment(&cancel_id, now_ms());
                    },
                    "Learning cycle cancelled; its saved project can continue it later",
                    move || Ok(controller.run(&run_id)?),
                )
                .await?;
                super::auto_training::materialize_final_predictions(context, &id).await?;
            }
            Err(error) if !settles_after_failed_resume(experiment.status, &error) => {
                return Err(error);
            }
            Err(error) => context.log_message(
                &format!(
                    "Learning experiment {id} cannot resume; its cycle settles once the experiment has stopped: {error}"
                ),
                LogLevel::Warn,
            ),
        }
    }
    finish_learning_cycle(context, &request.project_id).await
}

#[cfg(feature = "training")]
async fn promote(
    context: &mut ExecutionContext,
    request: LearningProjectId,
) -> Result<LearningProjectResult> {
    let (repo, scope) = project_ledger(context, &request.project_id)?;
    let promoted = promote_settled_candidate(context, &repo, &request.project_id).await?;
    published_champion(
        context,
        &repo,
        scope,
        &request.project_id,
        super::publisher::PublishReason::StateChange,
        promoted,
    )
    .await
}

/// Audits the settled candidate against the champion on fresh evidence and promotes it when
/// the project's policy allows.
#[cfg(feature = "training")]
async fn promote_settled_candidate(
    context: &ExecutionContext,
    repo: &flow_like_ml_runtime::TrainingRepository,
    id: &str,
) -> Result<LearningProjectResult> {
    let project = repo.get_learning_project(id)?;
    let cycles = repo.learning_cycles(id)?;
    let cycle = cycles
        .iter()
        .rev()
        .find(|cycle| {
            cycle.artifact_id == project.candidate_artifact_id
                && cycle.state == LearningCycleState::Settled
        })
        .ok_or_else(|| anyhow!("Project has no settled candidate"))?;
    let work_repo = repo.clone();
    let work_project = project.clone();
    let work_cycle = cycle.clone();
    run_audit(context, move |cancel| {
        audit_champion(&work_repo, &work_project, &work_cycle, &cancel)?;
        inference::audit_canary(&work_repo, &work_project, &cancel)
    })
    .await?;
    if repo.get_learning_project(id)?.candidate_artifact_id != cycle.artifact_id {
        return result(repo, id);
    }
    repo.compare_learning_cycle(&cycle.id, flow_like_ml_runtime::now_ms())?;
    repo.promote_learning_cycle(
        &cycle.id,
        deployment_generation(repo, &project)?,
        flow_like_ml_runtime::now_ms(),
    )?;
    result(repo, id)
}

macro_rules! learning_node {
    ($type:ident,$id:literal,$title:literal,$description:literal,$function:literal,$input:ty,$output:ty,$handler:ident $(, version = $version:literal)?) => {
        #[flow_like_types::async_trait]
        impl NodeLogic for $type {
            fn get_node(&self) -> Node {
                let mut node = super::operation_node::<$input, $output>(
                    $id,
                    $title,
                    $description,
                    $function,
                    "AI/ML/Continuous Learning",
                );
                node.set_long_running(true);
                $(node.set_version($version);)?
                node
            }
            async fn run(&self, context: &mut ExecutionContext) -> Result<()> {
                #[cfg(feature = "training")]
                {
                    context.deactivate_exec_pin("exec_out").await?;
                    let request: $input = context.evaluate_pin("request").await?;
                    let output = $handler(context, request).await?;
                    context.set_pin_value("result", json!(output)).await?;
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
pub struct CreateLearningProjectNode;
#[crate::register_node]
#[derive(Default)]
pub struct GetLearningProjectNode;
#[crate::register_node]
#[derive(Default)]
pub struct ObserveLearningSampleNode;
#[crate::register_node]
#[derive(Default)]
pub struct ReviewLearningSampleNode;
#[crate::register_node]
#[derive(Default)]
pub struct SelectSamplesForReviewNode;
#[crate::register_node]
#[derive(Default)]
pub struct AnalyzeModelErrorsNode;
#[crate::register_node]
#[derive(Default)]
pub struct NotifyLearningProductChangeNode;
#[crate::register_node]
#[derive(Default)]
pub struct StepLearningProjectNode;
#[crate::register_node]
#[derive(Default)]
pub struct PromoteLearningModelNode;
#[crate::register_node]
#[derive(Default)]
pub struct RollbackLearningModelNode;
#[crate::register_node]
#[derive(Default)]
pub struct RejectLearningCandidateNode;
#[crate::register_node]
#[derive(Default)]
pub struct PauseLearningProjectNode;
#[crate::register_node]
#[derive(Default)]
pub struct ResumeLearningProjectNode;
#[crate::register_node]
#[derive(Default)]
pub struct RouteLearningProjectNode;

learning_node!(
    CreateLearningProjectNode,
    "ml_create_learning_project",
    "Create Learning Project",
    "Persist a task, dataset source, review policy and budget across training experiments",
    "createLearningProject",
    CreateLearningProjectRequest,
    LearningProjectResult,
    create,
    version = 1
);
learning_node!(
    GetLearningProjectNode,
    "ml_get_learning_project",
    "Get Learning Project",
    "Read project state, budget, experiment history and retraining reasons",
    "getLearningProject",
    LearningProjectId,
    LearningProjectResult,
    get,
    version = 1
);
learning_node!(
    ObserveLearningSampleNode,
    "ml_observe_learning_sample",
    "Observe Learning Samples",
    "Record sample predictions, embeddings and operating conditions for review and drift detection. With a positive canary fraction every observation needs its raw source row in sample",
    "observeLearningSamples",
    LearningObservationsRequest,
    LearningProjectResult,
    observe,
    version = 1
);
learning_node!(
    ReviewLearningSampleNode,
    "ml_review_learning_sample",
    "Review Learning Sample",
    "Record a reviewed annotation or measured outcome for a collected sample",
    "reviewLearningSample",
    LearningReviewRequest,
    LearningProjectResult,
    review,
    version = 1
);
learning_node!(
    SelectSamplesForReviewNode,
    "ml_select_samples_for_review",
    "Select Samples for Review",
    "Select uncertain, disagreeing, diverse and underrepresented samples",
    "selectSamplesForReview",
    LearningReviewSelectionRequest,
    Vec<LearningReviewItem>,
    select_reviews
);
learning_node!(
    AnalyzeModelErrorsNode,
    "ml_analyze_model_errors",
    "Analyze Model Errors",
    "Measure errors by class and declared operating slices using reviewed outcomes",
    "analyzeModelErrors",
    LearningProjectId,
    LearningErrorAnalysis,
    errors
);
learning_node!(
    NotifyLearningProductChangeNode,
    "ml_notify_learning_product_change",
    "Notify Learning Product Change",
    "Record an idempotent product change that can trigger another training cycle",
    "notifyLearningProductChange",
    LearningProductChangeRequest,
    LearningProjectResult,
    changed,
    version = 1
);
learning_node!(
    StepLearningProjectNode,
    "ml_step_learning_project",
    "Step Learning Project",
    "Prepare or continue a training cycle, compare models on fresh audit evidence and apply the deployment policy",
    "stepLearningProject",
    LearningProjectId,
    LearningProjectResult,
    step,
    version = 1
);
learning_node!(
    PromoteLearningModelNode,
    "ml_promote_learning_model",
    "Promote Learning Model",
    "Promote an eligible model using fresh audited evidence and the project quality policy",
    "promoteLearningModel",
    LearningProjectId,
    LearningProjectResult,
    promote,
    version = 1
);
learning_node!(
    RollbackLearningModelNode,
    "ml_rollback_learning_model",
    "Rollback Learning Model",
    "Restore the previous model in the project deployment. By default it rolls back the more recently trained of the active and previous model, so a repeated request keeps the restored model serving",
    "rollbackLearningModel",
    RollbackLearningModelRequest,
    LearningProjectResult,
    rollback,
    version = 1
);
learning_node!(
    RejectLearningCandidateNode,
    "ml_reject_learning_candidate",
    "Reject Learning Candidate",
    "Discard the shadow or canary candidate without promoting it. The champion keeps serving, canary samples that judged the candidate stay sealed for audits and the next training cycle can start",
    "rejectLearningCandidate",
    RejectLearningCandidateRequest,
    LearningProjectResult,
    reject
);
learning_node!(
    PauseLearningProjectNode,
    "ml_pause_learning_project",
    "Pause Learning Project",
    "Pause project training and route decisions through its fallback",
    "pauseLearningProject",
    LearningProjectId,
    LearningProjectResult,
    pause,
    version = 1
);
learning_node!(
    ResumeLearningProjectNode,
    "ml_resume_learning_project",
    "Resume Learning Project",
    "Resume the project with its saved budget, datasets and model state",
    "resumeLearningProject",
    LearningProjectId,
    LearningProjectResult,
    resume,
    version = 1
);

/// Its own node so it runs on inference-only builds, from published files when this device has
/// no training ledger for the project.
#[flow_like_types::async_trait]
impl NodeLogic for RouteLearningProjectNode {
    fn get_node(&self) -> Node {
        let mut node = super::operation_node::<LearningRouteRequest, LearningRoute>(
            "ml_route_learning_project",
            "Route Learning Project",
            "Resolve teacher, champion, shadow or deterministic canary routing for a sample",
            "routeLearningProject",
            "AI/ML/Continuous Learning",
        );
        node.set_long_running(true);
        node
    }
    async fn run(&self, context: &mut ExecutionContext) -> Result<()> {
        #[cfg(feature = "inference")]
        {
            context.deactivate_exec_pin("exec_out").await?;
            let request: LearningRouteRequest = context.evaluate_pin("request").await?;
            let output = route(context, request).await?;
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
