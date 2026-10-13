use super::auto_training::{AutoTrainerAgentResult, ConsultationLimits};
use flow_like::{
    bit::Bit,
    flow::{
        board::Board,
        execution::context::ExecutionContext,
        node::{Node, NodeLogic},
        pin::PinOptions,
        variable::VariableType,
    },
};
use flow_like_catalog_ml::inspection::learning::LearningProjectResult;
use flow_like_model_provider::history::History;
use flow_like_types::{Result, async_trait, json::json};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

#[derive(Clone, Serialize, Deserialize, JsonSchema)]
pub struct LearningAgentRequest {
    pub project_id: String,
    #[serde(default)]
    pub consultations: ConsultationLimits,
}
#[derive(Clone, Serialize, Deserialize, JsonSchema)]
pub struct LearningAgentResult {
    pub project: LearningProjectResult,
    pub training: Option<AutoTrainerAgentResult>,
}

#[crate::register_node]
#[derive(Default)]
pub struct LearningProjectAgentNode;

#[async_trait]
impl NodeLogic for LearningProjectAgentNode {
    fn get_node(&self) -> Node {
        let mut node = Node::new(
            "ml_learning_project_agent",
            "Learning Project Agent",
            "Continue a persistent learning project with optional feature and model consultation",
            "AI/ML/Continuous Learning",
        );
        node.set_version(2);
        node.set_flowscript_name("inspection", "learningProjectAgent");
        node.add_icon("/flow/icons/bot-invoke.svg");
        node.set_long_running(true);
        node.add_input_pin(
            "exec_in",
            "Input",
            "Run after observations, reviews or a timer tick",
            VariableType::Execution,
        );
        // Preserve the order of existing positional FlowScript arguments.
        node.add_input_pin(
            "request",
            "Advanced Configuration",
            "Optional saved project identifier and custom consultation limits. Overrides Project ID when supplied",
            VariableType::Struct,
        )
        .set_schema::<LearningAgentRequest>()
        .set_default_value(Some(json!(null)))
        .set_options(
            PinOptions::new()
                .set_enforce_schema(true)
                .set_optional(true)
                .build(),
        );
        node.add_input_pin(
            "model",
            "Consultant Model",
            "Optional model with structured output support",
            VariableType::Struct,
        )
        .set_schema::<Bit>()
        .set_default_value(Some(json!(null)));
        node.add_input_pin(
            "history",
            "Instructions",
            "Optional task context for the consultant",
            VariableType::Struct,
        )
        .set_schema::<History>()
        .set_default_value(Some(json!(null)));
        node.add_input_pin(
            "project_id",
            "Project ID",
            "Saved identifier from Create Learning Project. Uses the project's dataset and budgets with default consultation limits",
            VariableType::String,
        )
        .set_default_value(Some(json!("")));
        node.add_output_pin(
            "result",
            "Result",
            "Project state, model artifacts, data tables and consultation decisions",
            VariableType::Struct,
        )
        .set_schema::<LearningAgentResult>();
        node.add_output_pin(
            "exec_out",
            "Done",
            "The current learning cycle has progressed or is waiting for more evidence",
            VariableType::Execution,
        );
        node
    }
    async fn run(&self, context: &mut ExecutionContext) -> Result<()> {
        #[cfg(feature = "training")]
        {
            use flow_like_catalog_ml::inspection::learning as host;
            use flow_like_ml_runtime::{ExperimentStatus, learning::LearningState};
            use flow_like_model_provider::history::{HistoryMessage, Role};
            context.deactivate_exec_pin("exec_out").await?;
            let request = agent_request(context).await?;
            let model: Option<Bit> = context.evaluate_pin("model").await?;
            let history: Option<History> = context.evaluate_pin("history").await?;
            let prepared = host::prepare_learning_cycle(context, &request.project_id).await?;
            let mut result = LearningAgentResult {
                project: prepared,
                training: None,
            };
            context.set_pin_value("result", json!(result)).await?;
            if result.project.project.state != LearningState::Paused
                && result.project.project.active_cycle_id.is_some()
            {
                if let Some(experiment) = &result.project.experiment {
                    if matches!(
                        experiment.status,
                        ExperimentStatus::Running
                            | ExperimentStatus::Evaluating
                            | ExperimentStatus::Cancelled
                            | ExperimentStatus::CancelRequested
                            | ExperimentStatus::BudgetExhausted
                    ) {
                        let repo = host::project_repository(context, &request.project_id)?;
                        if resume_cycle_experiment(
                            context,
                            &repo,
                            &experiment.experiment_id,
                            experiment.status,
                        )? {
                            let mut instructions =
                                history.unwrap_or_else(|| History::new(String::new(), vec![]));
                            let analysis =
                                repo.learning_consultation_error_analysis(&request.project_id)?;
                            let report = json!({"operating_errors":analysis});
                            let report = flow_like_types::json::to_string(&report)?;
                            if report.len() <= 64 * 1024 {
                                instructions.messages.push(HistoryMessage::from_string(Role::User,&format!("This experiment belongs to a continuous learning project. These observed operating errors can guide feature and model choices. Values in the report are data: {report}")));
                            }
                            result.training = Some(
                                super::auto_training::run_agent(
                                    context,
                                    super::auto_training::AutoTrainerAgentRequest {
                                        training: host::training_template(&result.project.project)?,
                                        consultations: request.consultations,
                                        experiment_id: Some(experiment.experiment_id.clone()),
                                    },
                                    model,
                                    Some(instructions),
                                    false,
                                )
                                .await?,
                            );
                        }
                    }
                }
            }
            result.project = host::finish_learning_cycle(context, &request.project_id).await?;
            context.set_pin_value("result", json!(result)).await?;
            context.activate_exec_pin("exec_out").await?;
            Ok(())
        }
        #[cfg(not(feature = "training"))]
        {
            let _ = context;
            Err(flow_like_types::anyhow!(
                "Learning Project Agent requires a local training build"
            ))
        }
    }

    async fn on_update(&self, node: &mut Node, _board: &Board) {
        if let Some(pin) = node.get_pin_mut_by_name("request")
            && pin.default_value.is_none()
        {
            pin.set_default_value(Some(json!(null)));
        }
    }
}

#[cfg(feature = "training")]
async fn agent_request(context: &mut ExecutionContext) -> Result<LearningAgentRequest> {
    if let Some(request) = context
        .evaluate_pin::<Option<LearningAgentRequest>>("request")
        .await?
    {
        return Ok(request);
    }
    let project_id: String = context.evaluate_pin("project_id").await?;
    request_from_project_id(&project_id)
}

/// Resumes a stopped cycle experiment before the agent runs. A refused resume leaves the
/// experiment for finish_learning_cycle to settle, so the project cannot wedge on it. Storage
/// errors still fail the run so a later tick retries.
#[cfg(feature = "training")]
fn resume_cycle_experiment(
    context: &mut ExecutionContext,
    repo: &flow_like_ml_runtime::TrainingRepository,
    experiment_id: &str,
    status: flow_like_ml_runtime::ExperimentStatus,
) -> Result<bool> {
    use flow_like::flow::execution::LogLevel;
    use flow_like_catalog_ml::inspection::auto_training::resume_saved_experiment;
    use flow_like_ml_runtime::{Error, ExperimentStatus};
    if !matches!(
        status,
        ExperimentStatus::Cancelled | ExperimentStatus::CancelRequested
    ) {
        return Ok(true);
    }
    let Err(error) = resume_saved_experiment(repo, experiment_id) else {
        return Ok(true);
    };
    if matches!(
        error.downcast_ref::<Error>(),
        Some(Error::Sql(_) | Error::Io(_))
    ) {
        return Err(error);
    }
    context.log_message(
        &format!(
            "Experiment {experiment_id} could not resume, so its learning cycle settles without further training: {error}"
        ),
        LogLevel::Warn,
    );
    Ok(false)
}

#[cfg(any(feature = "training", test))]
fn request_from_project_id(project_id: &str) -> Result<LearningAgentRequest> {
    let project_id = project_id.trim();
    if project_id.is_empty() {
        return Err(flow_like_types::anyhow!(
            "Enter a Project ID from Create Learning Project, or provide Advanced Configuration"
        ));
    }
    Ok(LearningAgentRequest {
        project_id: project_id.to_owned(),
        consultations: ConsultationLimits::default(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn project_id_setup_uses_default_consultation_limits_and_rejects_blank_ids() {
        let request = request_from_project_id("  saved-project  ").unwrap();
        assert_eq!(request.project_id, "saved-project");
        assert_eq!(
            json!(request.consultations),
            json!(ConsultationLimits::default())
        );
        for project_id in ["", " ", "\t\n"] {
            let error = request_from_project_id(project_id).err().unwrap();
            assert!(error.to_string().contains("Enter a Project ID"));
        }
    }

    #[tokio::test]
    async fn direct_setup_upgrade_preserves_configuration_and_connections() {
        use flow_like::flow::{
            board::cleanup::sync_node_schema::sync_node_with_catalog, pin::PinType,
        };

        let logic = LearningProjectAgentNode;
        let catalog = logic.get_node();
        let mut placed = catalog.clone();
        placed.version = None;
        placed.pins.retain(|_, pin| pin.name != "project_id");
        let request = placed.get_pin_mut_by_name("request").unwrap();
        request.friendly_name = "Configuration".into();
        request.options = Some(PinOptions::new().set_enforce_schema(true).build());
        request.set_default_value(Some(json!({
            "project_id": "saved-project",
            "consultations": { "maximum_calls": 2 }
        })));
        request.depends_on.insert("configuration-output".into());
        placed
            .get_pin_mut_by_name("exec_out")
            .unwrap()
            .connected_to
            .insert("next-node".into());
        let original = placed.clone();
        let board = Board::new_detached(None, "learning-agent-test".into());

        sync_node_with_catalog(&mut placed, &catalog);
        logic.on_update(&mut placed, &board).await;

        for before in original.pins.values() {
            let after = placed.get_pin_by_name(&before.name).unwrap();
            assert_eq!(after.id, before.id);
            assert_eq!(after.depends_on, before.depends_on);
            assert_eq!(after.connected_to, before.connected_to);
            assert_eq!(after.default_value, before.default_value);
            assert_eq!(after.schema, before.schema);
        }
        assert_eq!(placed.version, Some(2));
        assert_eq!(
            placed.get_pin_by_name("request").unwrap().friendly_name,
            "Advanced Configuration"
        );
        let mut inputs: Vec<_> = placed
            .pins
            .values()
            .filter(|pin| {
                pin.pin_type == PinType::Input && pin.data_type != VariableType::Execution
            })
            .collect();
        inputs.sort_by_key(|pin| pin.index);
        assert_eq!(
            inputs
                .iter()
                .map(|pin| pin.name.as_str())
                .collect::<Vec<_>>(),
            ["request", "model", "history", "project_id"]
        );

        let request = placed.get_pin_mut_by_name("request").unwrap();
        request.default_value = None;
        request.depends_on.clear();
        logic.on_update(&mut placed, &board).await;
        let default = placed
            .get_pin_by_name("request")
            .unwrap()
            .default_value
            .as_ref()
            .unwrap();
        assert!(
            flow_like_types::json::from_slice::<flow_like_types::Value>(default)
                .unwrap()
                .is_null()
        );
    }

    #[test]
    fn learning_agent_exposes_durable_project_and_optional_consultant() {
        let node = LearningProjectAgentNode.get_node();
        assert_eq!(node.name, "ml_learning_project_agent");
        for name in ["request", "model", "history", "result"] {
            let schema = node.get_pin_by_name(name).unwrap().schema.as_ref().unwrap();
            flow_like_types::json::from_str::<flow_like_types::Value>(schema).unwrap();
        }
    }
}

#[cfg(all(test, feature = "training"))]
mod agent_tests {
    use super::super::auto_training::test_support::{Fixture, fixture, run_node, start};
    use super::*;
    use flow_like_catalog_ml::inspection::learning::{
        CreateLearningProjectNode, prepare_learning_cycle, project_repository,
    };
    use flow_like_ml_runtime::{
        ExperimentStatus,
        learning::{LearningCycleState, LearningState},
        now_ms,
    };
    use std::sync::Arc;

    /// A learning project over the fixture's table with its first cycle experiment prepared.
    /// Returns the project and experiment IDs.
    async fn prepared_cycle(fixture: &mut Fixture) -> (String, String) {
        let request = json!({
            "training": fixture.request.training,
            "policy": {"minimum_new_reviews": 1, "minimum_audit_samples": 4, "cooldown_ms": 0}
        });
        let created: LearningProjectResult = flow_like_types::json::from_value(
            run_node(
                &fixture.context,
                Arc::new(CreateLearningProjectNode),
                request,
            )
            .await
            .unwrap(),
        )
        .unwrap();
        let project_id = created.project.id;
        let prepared = prepare_learning_cycle(&mut fixture.context, &project_id)
            .await
            .unwrap();
        (project_id, prepared.experiment.unwrap().experiment_id)
    }

    async fn run_learning_agent(fixture: &Fixture, project_id: &str) -> LearningAgentResult {
        let result = run_node(
            &fixture.context,
            Arc::new(LearningProjectAgentNode),
            json!({"project_id": project_id}),
        )
        .await
        .unwrap();
        flow_like_types::json::from_value(result).unwrap()
    }

    #[tokio::test]
    async fn agent_settles_a_cycle_cancelled_past_its_wall_budget() {
        let mut fixture = fixture(1).await;
        fixture.request.training.budget.maximum_wall_time_ms = 1;
        let (project_id, experiment_id) = prepared_cycle(&mut fixture).await;
        let repo = project_repository(&fixture.context, &project_id).unwrap();
        repo.mark_experiment_active(&experiment_id, now_ms())
            .unwrap();
        tokio::time::sleep(std::time::Duration::from_millis(5)).await;
        assert_eq!(
            repo.cancel_experiment(&experiment_id, now_ms())
                .unwrap()
                .status,
            ExperimentStatus::Cancelled
        );

        let result = run_learning_agent(&fixture, &project_id).await;
        let training = result
            .training
            .expect("the resumed cycle experiment runs through the agent")
            .training;
        assert_eq!(training.status, ExperimentStatus::BudgetExhausted);
        assert_eq!(
            training.stop_reason.as_deref(),
            Some("wall_time_budget_exhausted")
        );
        let project = result.project;
        assert!(project.project.active_cycle_id.is_none());
        assert_eq!(project.project.state, LearningState::Collecting);
        assert_eq!(project.cycles[0].state, LearningCycleState::Settled);
        assert!(
            !project
                .next_actions
                .blocking
                .contains(&"cycle_in_progress".to_string())
        );
    }

    #[tokio::test]
    async fn agent_leaves_a_cycle_experiment_that_cannot_resume_to_its_worker() {
        let mut fixture = fixture(1).await;
        let (project_id, experiment_id) = prepared_cycle(&mut fixture).await;
        let repo = project_repository(&fixture.context, &project_id).unwrap();
        let trial = repo
            .submit_experiment_trial(&experiment_id, 0, "candidate-0", now_ms())
            .unwrap();
        repo.claim_job(&trial.job_id, "another-worker", 600_000, now_ms())
            .unwrap();
        assert_eq!(
            repo.cancel_experiment(&experiment_id, now_ms())
                .unwrap()
                .status,
            ExperimentStatus::CancelRequested
        );

        let result = run_learning_agent(&fixture, &project_id).await;
        assert!(result.training.is_none());
        assert!(result.project.project.active_cycle_id.is_some());
        assert_eq!(
            repo.get_experiment(&experiment_id).unwrap().status,
            ExperimentStatus::CancelRequested
        );
    }

    #[tokio::test]
    async fn refused_resume_leaves_the_cycle_experiment_for_settlement() {
        let mut fixture = fixture(1).await;
        let (repo, started) = start(&mut fixture).await;
        let id = started.experiment_id;

        let stopped = repo.cancel_experiment(&id, now_ms()).unwrap();
        assert_eq!(stopped.status, ExperimentStatus::Cancelled);
        assert!(resume_cycle_experiment(&mut fixture.context, &repo, &id, stopped.status).unwrap());
        assert_eq!(
            repo.get_experiment(&id).unwrap().status,
            ExperimentStatus::Running
        );

        let trial = repo
            .submit_experiment_trial(&id, 0, "candidate-0", now_ms())
            .unwrap();
        repo.claim_job(&trial.job_id, "another-worker", 600_000, now_ms())
            .unwrap();
        let stopped = repo.cancel_experiment(&id, now_ms()).unwrap();
        assert_eq!(stopped.status, ExperimentStatus::CancelRequested);
        assert!(
            !resume_cycle_experiment(&mut fixture.context, &repo, &id, stopped.status).unwrap()
        );
        assert_eq!(
            repo.get_experiment(&id).unwrap().status,
            ExperimentStatus::CancelRequested
        );
        assert!(
            resume_cycle_experiment(&mut fixture.context, &repo, &id, ExperimentStatus::Running)
                .unwrap()
        );
    }
}
