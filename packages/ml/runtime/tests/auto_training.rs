#![cfg(all(feature = "native", feature = "burn"))]
use flow_like_ml_burn as burn;
use flow_like_ml_core::{
    Annotation, ComputeBackend, ComputeConfig, InspectionSpec, LabelProvenance, Sample, TaskKind,
    TensorData,
};
use flow_like_ml_runtime::{auto_training::*, engines::*, worker::TrainingWorker, *};
use serde_json::json;
use sha2::{Digest, Sha256};
use std::{
    sync::{
        Arc,
        atomic::{AtomicBool, AtomicI64, AtomicUsize, Ordering},
    },
    time::{Duration, Instant},
};

#[derive(Clone, Copy)]
struct Fixture {
    anomaly: bool,
    test_size: usize,
    /// Narrows validation inputs around the middle of the training clusters. Shifting them
    /// instead leaves the training range, where isolation scores saturate at the edge values.
    validation_spread: f32,
}
impl Default for Fixture {
    fn default() -> Self {
        Self {
            anomaly: false,
            test_size: 16,
            validation_spread: 1.0,
        }
    }
}

fn setup(repo: &TrainingRepository) -> (ExperimentRequest, InspectionSpec) {
    fixture(repo, Fixture::default())
}

fn setup_task(repo: &TrainingRepository, anomaly: bool) -> (ExperimentRequest, InspectionSpec) {
    fixture(
        repo,
        Fixture {
            anomaly,
            ..Fixture::default()
        },
    )
}

fn fixture(repo: &TrainingRepository, options: Fixture) -> (ExperimentRequest, InspectionSpec) {
    let anomaly = options.anomaly;
    let stream = StreamKey {
        project_id: "automl".into(),
        stream_id: "quality".into(),
        inspection_version: "v1".into(),
    };
    let spec = InspectionSpec {
        id: "v1".into(),
        description: "detect negative and positive samples".into(),
        task: if anomaly {
            TaskKind::SensorAnomaly
        } else {
            TaskKind::SensorClassification
        },
        labels: vec!["negative".into(), "positive".into()],
        input_shape: vec![1],
        prediction_horizon_ms: None,
        minimum_examples: 1,
        minimum_examples_per_class: 1,
    };
    let mut partitions: [Vec<TrainingSample>; 3] = Default::default();
    for (partition, rows) in partitions.iter_mut().enumerate() {
        let count = if partition == 2 {
            options.test_size
        } else {
            16
        };
        let spread = if partition == 1 {
            options.validation_spread
        } else {
            1.0
        };
        for index in 0..count {
            let sign = if index % 2 == 0 { -1.0 } else { 1.0 };
            // Final labels intentionally differ, exposing accidental reuse of validation metrics.
            let class = if partition == 2 {
                1 - index % 2
            } else {
                index % 2
            };
            let sample = Sample {
                id: format!("{partition}-{index}"),
                group_id: format!("group-{partition}-{index}"),
                stream_id: stream.stream_id.clone(),
                timestamp_ms: 100,
                window_start_ms: 100,
                window_end_ms: 100,
                input: TensorData {
                    shape: vec![1],
                    values: vec![
                        sign * (1.0 + 0.07 * (1.0 - spread) + index as f32 * 0.01 * spread),
                    ],
                },
                annotation: if anomaly {
                    Annotation::Anomaly {
                        is_anomaly: class == 1,
                    }
                } else {
                    Annotation::Class {
                        class_id: class as u32,
                    }
                },
                provenance: LabelProvenance::Reviewed {
                    reviewer: "fixture".into(),
                    reviewed_at_ms: 100,
                },
                outcome: None,
            };
            let row = TrainingSample {
                id: sample.id.clone(),
                annotation_revision: 1,
                group_id: sample.group_id.clone(),
                captured_at_ms: 100,
                label_available_at_ms: 100,
                content_digest: format!(
                    "{:x}",
                    Sha256::digest(serde_json::to_vec(&sample).unwrap())
                ),
                source: LabelSource::Reviewed,
                accepted: true,
                payload: json!({"sample":sample,"label":spec.labels[class]}),
            };
            repo.record_sample(&stream, &row).unwrap();
            rows.push(row);
        }
    }
    let [train, validation, test] = partitions;
    let cutoff_sequence = (train.len() + validation.len() + test.len()) as i64;
    let snapshot = repo
        .import_experiment_snapshot(DatasetSnapshot {
            id: "".into(),
            digest: "".into(),
            stream: stream.clone(),
            as_of_ms: 200,
            cutoff_sequence,
            policy: SplitPolicy::Group {
                train_fraction: 1.0 / 3.0,
                validation_fraction: 1.0 / 3.0,
                seed: 7,
            },
            train,
            validation,
            test,
            excluded: vec![],
        })
        .unwrap();
    let mut budget = ExperimentBudget::default();
    budget.maximum_trials = 4;
    budget.worker_limits.maximum_duration_ms = 10_000;
    budget.maximum_training_time_ms = 40_000;
    let compute = ComputeConfig {
        backend: ComputeBackend::Cpu,
        ..Default::default()
    };
    let candidates = vec![TrainingRequest {
        engine: "histogram_gradient_boosting".into(),
        recipe: json!({"labels":spec.labels,"inspection_task":spec.task,"config":{"objective":"binary_log_loss","estimators":30,"max_depth":2,"max_bins":16,"min_leaf":1,"learning_rate":0.2,"l2":0.1}}),
        compute: serde_json::to_value(compute).unwrap(),
    }];
    let mut goals = default_goals(&spec);
    goals.minimum_audited_samples = 8;
    goals.bounds = vec![MetricBound {
        name: "accuracy".into(),
        minimum: Some(0.9),
        maximum: None,
    }];
    (
        ExperimentRequest {
            stream,
            snapshot_id: snapshot.id,
            spec: serde_json::to_value(&spec).unwrap(),
            candidates,
            goals,
            budget,
            source_table_versions: vec![json!({"id":"raw-v1"})],
            created_tables: vec![],
            updated_table_versions: vec![],
            preprocessing_manifest: json!({"fit_partition":"train","features":["signal"]}),
            context: json!({}),
        },
        spec,
    )
}

#[test]
fn anomaly_search_trains_only_normal_samples_but_scores_both_validation_classes() {
    let dir = tempfile::tempdir().unwrap();
    let repo = TrainingRepository::open(dir.path().join("training.sqlite")).unwrap();
    let (mut request, _) = setup_task(&repo, true);
    request.candidates = vec![TrainingRequest {
        engine: "isolation_forest".into(),
        recipe: json!({"labels":["negative","positive"],"inspection_task":"sensor_anomaly","config":{"trees":32,"sample_size":8,"seed":7}}),
        compute: serde_json::to_value(ComputeConfig {
            backend: ComputeBackend::Cpu,
            ..Default::default()
        })
        .unwrap(),
    }];
    let limits = request.budget.worker_limits.clone();
    let experiment = repo.create_experiment(request, now_ms()).unwrap();
    let controller = AutoTrainingController::new(repo.clone(), worker(&repo, &limits));
    controller.run_next(&experiment.id).unwrap();
    let trial = repo
        .list_experiment_trials(&experiment.id)
        .unwrap()
        .remove(0);
    assert_eq!(
        trial.status,
        ExperimentTrialStatus::Validated,
        "{:?}",
        trial.error
    );
    assert_eq!(trial.validation_metrics["audited_samples"], 16.0);
    assert!(trial.validation_metrics.contains_key("auroc"));
    let training = repo.get_snapshot(&trial.snapshot_id).unwrap();
    assert_eq!(training.train.len(), 8);
    assert_eq!(training.validation.len(), 8);
    assert!(training.test.is_empty());
    let result = controller.finalize(&experiment.id).unwrap();
    assert_eq!(result.status, ExperimentStatus::Completed);
    let evaluation = repo
        .get_evaluation(result.final_evaluation_id.as_ref().unwrap())
        .unwrap();
    assert_eq!(evaluation.audited_samples, 16);
    assert!(evaluation.metrics.contains_key("auroc"));
    assert_goal_metrics_reach_the_final_audit(&repo, &result);
}

/// Every goal metric validation measured for the winner is also a metric of its final audit.
fn assert_goal_metrics_reach_the_final_audit(repo: &TrainingRepository, result: &Experiment) {
    let spec: InspectionSpec = serde_json::from_value(result.request.spec.clone()).unwrap();
    let winner = repo
        .get_experiment_trial(result.selected_trial_id.as_ref().unwrap())
        .unwrap();
    let evaluation = repo
        .get_evaluation(result.final_evaluation_id.as_ref().unwrap())
        .unwrap();
    let mut goals = result.request.goals.clone();
    for name in winner.validation_metrics.keys() {
        goals.bounds = vec![MetricBound {
            name: name.clone(),
            minimum: Some(0.0),
            maximum: None,
        }];
        if validate_goals(&spec, &goals).is_ok() {
            assert!(
                evaluation.metrics.contains_key(name),
                "goal metric {name} is missing from the final audit: {:?}",
                evaluation.metrics.keys().collect::<Vec<_>>()
            );
        }
    }
}

fn worker(repo: &TrainingRepository, limits: &WorkerLimits) -> TrainingWorker {
    let mut worker = TrainingWorker::new(repo.clone(), limits.clone()).unwrap();
    register_native_engines(&mut worker).unwrap();
    register_burn_engine(&mut worker).unwrap();
    worker
}

#[test]
fn search_uses_validation_then_freezes_one_winner_for_real_final_evidence() {
    let dir = tempfile::tempdir().unwrap();
    let repo = TrainingRepository::open(dir.path().join("training.sqlite")).unwrap();
    let (request, _) = setup(&repo);
    let limits = request.budget.worker_limits.clone();
    let experiment = repo.create_experiment(request, now_ms()).unwrap();
    let controller = AutoTrainingController::new(repo.clone(), worker(&repo, &limits));
    let running = controller.run_next(&experiment.id).unwrap();
    assert!(running.selected_trial_id.is_none());
    assert!(running.final_evaluation_id.is_none());
    let trials = repo.list_experiment_trials(&experiment.id).unwrap();
    assert_eq!(trials.len(), 1);
    assert_eq!(trials[0].validation_metrics["accuracy"], 1.0);
    let artifact = repo
        .get_artifact(trials[0].artifact_id.as_ref().unwrap())
        .unwrap();
    assert!(
        repo.get_snapshot(&artifact.snapshot_id)
            .unwrap()
            .test
            .is_empty()
    );
    assert!(repo.predictions(&artifact.id).unwrap().is_empty());
    assert_eq!(
        artifact.manifest["preprocessing_manifest"]["fit_partition"],
        "train"
    );
    let completed = controller.finalize(&experiment.id).unwrap();
    assert_eq!(completed.status, ExperimentStatus::Completed);
    assert!(!completed.target_met);
    assert_goal_metrics_reach_the_final_audit(&repo, &completed);
    let evaluation = repo
        .get_evaluation(completed.final_evaluation_id.as_ref().unwrap())
        .unwrap();
    assert_eq!(evaluation.metrics["accuracy"], 0.0);
    assert_eq!(evaluation.audited_samples, 16);
    assert_eq!(repo.predictions(&artifact.id).unwrap().len(), 16);
    assert!(
        repo.append_experiment_candidates(
            &experiment.id,
            vec![experiment.request.candidates[0].clone()],
            now_ms()
        )
        .is_err()
    );
    assert_eq!(
        controller
            .finalize(&experiment.id)
            .unwrap()
            .final_evaluation_id,
        completed.final_evaluation_id
    );
}

#[test]
fn completed_trial_recovers_without_retraining_and_cancelled_queue_resumes() {
    let dir = tempfile::tempdir().unwrap();
    let repo = TrainingRepository::open(dir.path().join("training.sqlite")).unwrap();
    let (request, _) = setup(&repo);
    let limits = request.budget.worker_limits.clone();
    let experiment = repo.create_experiment(request, now_ms()).unwrap();
    let trial = repo
        .submit_experiment_trial(&experiment.id, 0, "candidate-0", now_ms())
        .unwrap();
    repo.cancel_experiment(&experiment.id, now_ms()).unwrap();
    assert_eq!(
        repo.get_job(&trial.job_id).unwrap().status,
        JobStatus::Cancelled
    );
    repo.resume_experiment(&experiment.id, now_ms()).unwrap();
    let artifact = worker(&repo, &limits).run(&trial.job_id).unwrap();
    let generation = repo.get_job(&trial.job_id).unwrap().generation;
    let reopened = TrainingRepository::open(dir.path().join("training.sqlite")).unwrap();
    let controller = AutoTrainingController::new(reopened.clone(), worker(&reopened, &limits));
    controller.run_next(&experiment.id).unwrap();
    let validated = reopened.get_experiment_trial(&trial.id).unwrap();
    assert_eq!(validated.artifact_id.as_deref(), Some(artifact.id.as_str()));
    assert_eq!(
        reopened.get_job(&trial.job_id).unwrap().generation,
        generation
    );
    assert_eq!(validated.status, ExperimentTrialStatus::Validated);
}

struct InterceptFactory {
    experiment_id: String,
    limits: WorkerLimits,
    cancel: bool,
    triggered: Arc<AtomicBool>,
}
struct InterceptPredictor {
    inner: Box<dyn SearchPredictor>,
    repository: TrainingRepository,
    experiment_id: String,
    cancel: bool,
    triggered: Arc<AtomicBool>,
}
impl SearchPredictor for InterceptPredictor {
    fn predict(&mut self, input: &TensorData) -> Result<serde_json::Value> {
        if self.cancel && !self.triggered.swap(true, Ordering::SeqCst) {
            self.repository
                .cancel_experiment(&self.experiment_id, now_ms())?;
        }
        self.inner.predict(input)
    }
}
impl SearchPredictorFactory for InterceptFactory {
    fn load(
        &self,
        repository: &TrainingRepository,
        artifact_id: &str,
        compute: &ComputeConfig,
    ) -> Result<Box<dyn SearchPredictor>> {
        if !self.cancel {
            let concurrent =
                AutoTrainingController::new(repository.clone(), worker(repository, &self.limits));
            let before = repository.get_experiment(&self.experiment_id)?;
            assert_eq!(
                concurrent.run_next(&self.experiment_id)?.generation,
                before.generation
            );
            assert_eq!(
                concurrent.finalize(&self.experiment_id)?.generation,
                before.generation
            );
            self.triggered.store(true, Ordering::SeqCst);
        }
        Ok(Box::new(InterceptPredictor {
            inner: NativeSearchPredictorFactory.load(repository, artifact_id, compute)?,
            repository: repository.clone(),
            experiment_id: self.experiment_id.clone(),
            cancel: self.cancel,
            triggered: self.triggered.clone(),
        }))
    }
}

#[test]
fn concurrent_controller_cannot_repeat_validation_or_open_test_early() {
    let dir = tempfile::tempdir().unwrap();
    let repo = TrainingRepository::open(dir.path().join("training.sqlite")).unwrap();
    let (request, _) = setup(&repo);
    let limits = request.budget.worker_limits.clone();
    let experiment = repo.create_experiment(request, now_ms()).unwrap();
    let triggered = Arc::new(AtomicBool::new(false));
    let controller = AutoTrainingController::new(repo.clone(), worker(&repo, &limits))
        .with_predictor_factory(Arc::new(InterceptFactory {
            experiment_id: experiment.id.clone(),
            limits,
            cancel: false,
            triggered: triggered.clone(),
        }));
    controller.run_next(&experiment.id).unwrap();
    assert!(triggered.load(Ordering::SeqCst));
    let trial = repo
        .list_experiment_trials(&experiment.id)
        .unwrap()
        .remove(0);
    assert_eq!(trial.status, ExperimentTrialStatus::Validated);
    assert!(
        repo.predictions(trial.artifact_id.as_ref().unwrap())
            .unwrap()
            .is_empty()
    );
    assert!(
        repo.get_experiment(&experiment.id)
            .unwrap()
            .selected_trial_id
            .is_none()
    );
    assert_eq!(
        controller.finalize(&experiment.id).unwrap().status,
        ExperimentStatus::Completed
    );
}

#[test]
fn cancellation_during_validation_resumes_the_published_artifact() {
    let dir = tempfile::tempdir().unwrap();
    let repo = TrainingRepository::open(dir.path().join("training.sqlite")).unwrap();
    let (request, _) = setup(&repo);
    let limits = request.budget.worker_limits.clone();
    let experiment = repo.create_experiment(request, now_ms()).unwrap();
    let controller = AutoTrainingController::new(repo.clone(), worker(&repo, &limits))
        .with_predictor_factory(Arc::new(InterceptFactory {
            experiment_id: experiment.id.clone(),
            limits: limits.clone(),
            cancel: true,
            triggered: Arc::new(AtomicBool::new(false)),
        }));
    let stopped = controller.run_next(&experiment.id).unwrap();
    assert_eq!(stopped.status, ExperimentStatus::Cancelled);
    let trial = repo
        .list_experiment_trials(&experiment.id)
        .unwrap()
        .remove(0);
    let job = repo.get_job(&trial.job_id).unwrap();
    assert_eq!(job.status, JobStatus::Succeeded);
    assert_eq!(trial.status, ExperimentTrialStatus::Cancelled);
    repo.resume_experiment(&experiment.id, now_ms()).unwrap();
    let resumed = AutoTrainingController::new(repo.clone(), worker(&repo, &limits));
    resumed.run_next(&experiment.id).unwrap();
    let validated = repo.get_experiment_trial(&trial.id).unwrap();
    assert_eq!(validated.status, ExperimentTrialStatus::Validated);
    assert_eq!(validated.artifact_id, job.artifact_id);
    assert_eq!(
        repo.get_job(&trial.job_id).unwrap().generation,
        job.generation
    );
}

#[test]
fn cancellation_after_controller_interruption_settles_the_completed_worker() {
    let dir = tempfile::tempdir().unwrap();
    let repo = TrainingRepository::open(dir.path().join("training.sqlite")).unwrap();
    let (request, _) = setup(&repo);
    let limits = request.budget.worker_limits.clone();
    let experiment = repo.create_experiment(request, now_ms()).unwrap();
    let trial = repo
        .submit_experiment_trial(&experiment.id, 0, "candidate-0", now_ms())
        .unwrap();
    let artifact = worker(&repo, &limits).run(&trial.job_id).unwrap();
    let generation = repo.get_job(&trial.job_id).unwrap().generation;
    drop(repo);

    let reopened = TrainingRepository::open(dir.path().join("training.sqlite")).unwrap();
    reopened
        .cancel_experiment(&experiment.id, now_ms())
        .unwrap();
    let controller = AutoTrainingController::new(reopened.clone(), worker(&reopened, &limits));
    let cancelled = controller.run(&experiment.id).unwrap();
    assert_eq!(cancelled.status, ExperimentStatus::Cancelled);
    assert_eq!(cancelled.usage.reserved_training_time_ms, 0);
    assert_eq!(cancelled.usage.reserved_artifact_bytes, 0);
    assert_eq!(cancelled.usage.training_time_ms, limits.maximum_duration_ms);
    assert_eq!(cancelled.usage.artifact_bytes, artifact.blob.bytes);
    assert_eq!(
        reopened.get_experiment_trial(&trial.id).unwrap().status,
        ExperimentTrialStatus::Cancelled
    );

    let repeated = reopened
        .cancel_experiment(&experiment.id, now_ms())
        .unwrap();
    assert_eq!(
        serde_json::to_value(&repeated.usage).unwrap(),
        serde_json::to_value(&cancelled.usage).unwrap()
    );
    reopened
        .resume_experiment(&experiment.id, now_ms())
        .unwrap();
    controller.run_next(&experiment.id).unwrap();
    let validated = reopened.get_experiment_trial(&trial.id).unwrap();
    assert_eq!(validated.status, ExperimentTrialStatus::Validated);
    assert_eq!(validated.artifact_id.as_deref(), Some(artifact.id.as_str()));
    assert_eq!(
        reopened.get_job(&trial.job_id).unwrap().generation,
        generation
    );
    let resumed = reopened.get_experiment(&experiment.id).unwrap();
    assert_eq!(resumed.usage.artifact_bytes, cancelled.usage.artifact_bytes);
    assert_eq!(
        resumed.usage.training_time_ms,
        cancelled.usage.training_time_ms
    );
}

fn forecast_spec() -> InspectionSpec {
    InspectionSpec {
        id: "forecast".into(),
        description: "Forecast the next readings".into(),
        task: TaskKind::SequenceForecast,
        labels: vec![],
        input_shape: vec![1],
        prediction_horizon_ms: Some(1),
        minimum_examples: 1,
        minimum_examples_per_class: 1,
    }
}

/// A reviewed forecast window whose outcome is measured 1 ms and available 2 ms after it.
fn forecast_row(
    stream: &StreamKey,
    partition: usize,
    index: usize,
    targets: Vec<f32>,
) -> TrainingSample {
    let timestamp = (partition * 2 + index) as i64 * 10;
    let sample = Sample {
        id: format!("{partition}-{index}"),
        group_id: format!("group-{partition}-{index}"),
        stream_id: stream.stream_id.clone(),
        timestamp_ms: timestamp,
        window_start_ms: timestamp,
        window_end_ms: timestamp,
        input: TensorData {
            shape: vec![1],
            values: vec![index as f32],
        },
        annotation: Annotation::Values { values: targets },
        provenance: LabelProvenance::Reviewed {
            reviewer: "fixture".into(),
            reviewed_at_ms: timestamp + 2,
        },
        outcome: Some(flow_like_ml_core::Outcome {
            target_start_ms: timestamp + 1,
            target_end_ms: timestamp + 1,
            available_at_ms: timestamp + 2,
        }),
    };
    forecast_spec().validate_sample_at(&sample, 100).unwrap();
    TrainingSample {
        id: sample.id.clone(),
        annotation_revision: 1,
        group_id: sample.group_id.clone(),
        captured_at_ms: timestamp,
        label_available_at_ms: timestamp + 2,
        content_digest: format!("{:x}", Sha256::digest(serde_json::to_vec(&sample).unwrap())),
        source: LabelSource::Reviewed,
        accepted: true,
        payload: serde_json::to_value(sample).unwrap(),
    }
}

/// Two reviewed forecast windows per partition, labelled `targets(partition, index)`.
fn forecast_snapshot(
    repo: &TrainingRepository,
    targets: impl Fn(usize, usize) -> Vec<f32>,
) -> DatasetSnapshot {
    let stream = StreamKey {
        project_id: "forecast".into(),
        stream_id: "readings".into(),
        inspection_version: "v1".into(),
    };
    let mut partitions: [Vec<TrainingSample>; 3] = Default::default();
    for (partition, rows) in partitions.iter_mut().enumerate() {
        for index in 0..2 {
            let row = forecast_row(&stream, partition, index, targets(partition, index));
            repo.record_sample(&stream, &row).unwrap();
            rows.push(row);
        }
    }
    let [train, validation, test] = partitions;
    repo.import_experiment_snapshot(DatasetSnapshot {
        id: String::new(),
        digest: String::new(),
        stream,
        as_of_ms: 100,
        cutoff_sequence: 6,
        policy: SplitPolicy::Group {
            train_fraction: 0.4,
            validation_fraction: 0.3,
            seed: 42,
        },
        train,
        validation,
        test,
        excluded: vec![],
    })
    .unwrap()
}

/// A forecast search over `forecast_snapshot` with `candidates` under an rmse bound of 100.
fn forecast_experiment(
    repo: &TrainingRepository,
    targets: impl Fn(usize, usize) -> Vec<f32>,
    candidates: Vec<TrainingRequest>,
) -> Experiment {
    let snapshot = forecast_snapshot(repo, targets);
    let spec = forecast_spec();
    let mut goals = default_goals(&spec);
    goals.bounds = vec![MetricBound {
        name: "rmse".into(),
        minimum: None,
        maximum: Some(100.),
    }];
    repo.create_experiment(
        ExperimentRequest {
            stream: snapshot.stream,
            snapshot_id: snapshot.id,
            spec: serde_json::to_value(spec).unwrap(),
            candidates,
            goals,
            budget: ExperimentBudget::default(),
            source_table_versions: vec![],
            created_tables: vec![],
            updated_table_versions: vec![],
            preprocessing_manifest: json!({}),
            context: json!({}),
        },
        now_ms(),
    )
    .unwrap()
}

#[test]
fn forecast_search_checks_each_target_width_before_flattening() {
    for ragged_partition in [Some(0), Some(1), None] {
        let dir = tempfile::tempdir().unwrap();
        let repo = TrainingRepository::open(dir.path().join("forecast.sqlite")).unwrap();
        let experiment = forecast_experiment(
            &repo,
            |partition, index| {
                if ragged_partition != Some(partition) {
                    vec![10., 20.]
                } else if index == 0 {
                    vec![10.]
                } else {
                    vec![20., 30., 40.]
                }
            },
            vec![TrainingRequest {
                engine: "burn".into(),
                recipe: json!({"config":{"recipe":{"architecture":"mlp","input_features":1,"hidden":4,"outputs":2,"objective":"regression"},"backend":{"backend":"cpu"},"epochs":1,"batch_size":2,"learning_rate":0.01,"seed":42,"gradient_clip":5.},"labels":[],"inspection_task":"sequence_forecast"}),
                compute: json!({"backend":"cpu"}),
            }],
        );
        let limits = experiment.request.budget.worker_limits.clone();
        let result = AutoTrainingController::new(repo.clone(), worker(&repo, &limits))
            .run(&experiment.id)
            .unwrap();
        let trial = repo
            .list_experiment_trials(&experiment.id)
            .unwrap()
            .remove(0);
        if ragged_partition.is_some() {
            assert_eq!(result.status, ExperimentStatus::Failed);
            assert!(
                trial
                    .error
                    .as_deref()
                    .unwrap()
                    .contains("target width 1; model requires 2")
            );
            assert!(trial.artifact_id.is_none());
            assert!(repo.get_job(&trial.job_id).unwrap().checkpoint.is_none());
        } else {
            assert_eq!(result.status, ExperimentStatus::Completed);
            assert!(result.target_met);
            assert!(trial.artifact_id.is_some());
            assert!(result.final_evaluation_id.is_some());
        }
    }
}

#[test]
fn single_step_forecasts_search_boosting_but_wider_horizons_do_not() {
    let boosting = |output_features: usize| {
        let search = AutoSearchConfig {
            maximum_candidates: 32,
            output_features,
            ..Default::default()
        };
        generate_candidates(&forecast_spec(), &search, &cpu())
            .unwrap()
            .into_iter()
            .filter(|candidate| candidate.engine == "histogram_gradient_boosting")
            .collect::<Vec<_>>()
    };
    assert!(boosting(2).is_empty(), "boosting fits one target value");
    let candidates = boosting(1);
    assert_eq!(candidates.len(), 3);
    let dir = tempfile::tempdir().unwrap();
    let repo = TrainingRepository::open(dir.path().join("forecast.sqlite")).unwrap();
    let experiment = forecast_experiment(&repo, |_, index| vec![10. + index as f32], candidates);
    let limits = experiment.request.budget.worker_limits.clone();
    let result = AutoTrainingController::new(repo.clone(), worker(&repo, &limits))
        .run(&experiment.id)
        .unwrap();
    assert_eq!(
        result.status,
        ExperimentStatus::Completed,
        "{:?}",
        result.stop_reason
    );
    assert!(result.target_met, "{:?}", result.unmet_constraints);
    let trials = repo.list_experiment_trials(&experiment.id).unwrap();
    assert_eq!(trials.len(), 3);
    for trial in trials {
        assert_eq!(
            trial.status,
            ExperimentTrialStatus::Validated,
            "{:?}",
            trial.error
        );
        assert!(trial.validation_metrics.contains_key("rmse"));
    }
    let evaluation = repo
        .get_evaluation(result.final_evaluation_id.as_ref().unwrap())
        .unwrap();
    assert_eq!(evaluation.audited_samples, 2);
    assert_goal_metrics_reach_the_final_audit(&repo, &result);
}

#[test]
fn controller_recovers_an_expired_worker_after_process_restart() {
    let dir = tempfile::tempdir().unwrap();
    let repo = TrainingRepository::open(dir.path().join("training.sqlite")).unwrap();
    let (request, _) = setup(&repo);
    let limits = request.budget.worker_limits.clone();
    let experiment = repo.create_experiment(request, now_ms()).unwrap();
    let trial = repo
        .submit_experiment_trial(&experiment.id, 0, "restart", now_ms())
        .unwrap();
    let lease = repo
        .claim_job(&trial.job_id, "old-process", 1000, now_ms() - 2000)
        .unwrap();
    let reopened = TrainingRepository::open(repo.path()).unwrap();
    let controller = AutoTrainingController::new(reopened.clone(), worker(&reopened, &limits));
    controller.run_next(&experiment.id).unwrap();
    assert_eq!(
        reopened.get_experiment_trial(&trial.id).unwrap().status,
        ExperimentTrialStatus::Validated
    );
    assert!(reopened.get_job(&trial.job_id).unwrap().generation > lease.generation);
    assert!(
        reopened
            .get_experiment(&experiment.id)
            .unwrap()
            .usage
            .training_time_ms
            >= limits.maximum_duration_ms,
        "recovery must charge the interrupted worker slice before training again"
    );
}

#[test]
fn recovery_cannot_restart_a_worker_after_its_training_budget_is_consumed() {
    let dir = tempfile::tempdir().unwrap();
    let repo = TrainingRepository::open(dir.path().join("training.sqlite")).unwrap();
    let (mut request, _) = setup(&repo);
    request.budget.maximum_training_time_ms = request.budget.worker_limits.maximum_duration_ms;
    let limits = request.budget.worker_limits.clone();
    let experiment = repo.create_experiment(request, now_ms()).unwrap();
    let trial = repo
        .submit_experiment_trial(&experiment.id, 0, "exhausted-restart", now_ms())
        .unwrap();
    repo.claim_job(&trial.job_id, "old-process", 1000, now_ms() - 2000)
        .unwrap();
    let controller = AutoTrainingController::new(repo.clone(), worker(&repo, &limits));
    let result = controller.run_next(&experiment.id).unwrap();
    let stopped = repo.get_job(&trial.job_id).unwrap();
    assert_eq!(stopped.status, JobStatus::Failed);
    assert!(stopped.artifact_id.is_none());
    assert!(result.usage.training_time_ms >= limits.maximum_duration_ms);
    assert_eq!(result.usage.reserved_training_time_ms, 0);
    assert_eq!(result.status, ExperimentStatus::BudgetExhausted);
}

#[test]
fn artifact_size_can_be_the_primary_goal_with_final_evidence() {
    let dir = tempfile::tempdir().unwrap();
    let repo = TrainingRepository::open(dir.path().join("training.sqlite")).unwrap();
    let (mut request, _) = setup(&repo);
    request.goals.primary_metric = "artifact_bytes".into();
    request.goals.direction = MetricDirection::Minimize;
    request.goals.bounds = vec![MetricBound {
        name: "artifact_bytes".into(),
        minimum: None,
        maximum: Some(1_000_000.0),
    }];
    let limits = request.budget.worker_limits.clone();
    let experiment = repo.create_experiment(request, now_ms()).unwrap();
    let controller = AutoTrainingController::new(repo.clone(), worker(&repo, &limits));
    let result = controller.run(&experiment.id).unwrap();
    assert!(result.target_met, "{:?}", result.unmet_constraints);
    let trial = repo
        .get_experiment_trial(result.selected_trial_id.as_ref().unwrap())
        .unwrap();
    let artifact = repo
        .get_artifact(trial.artifact_id.as_ref().unwrap())
        .unwrap();
    let evaluation = repo
        .get_evaluation(result.final_evaluation_id.as_ref().unwrap())
        .unwrap();
    assert_eq!(
        trial.validation_metrics["artifact_bytes"],
        artifact.blob.bytes as f64
    );
    assert_eq!(
        evaluation.metrics["artifact_bytes"],
        artifact.blob.bytes as f64
    );
    assert!(evaluation.metrics["mean_latency_ms"] >= 0.0);
}

#[test]
fn generated_candidates_are_seeded_and_include_real_temporal_architectures() {
    let spec = InspectionSpec {
        id: "sensor".into(),
        description: "classify sensor windows".into(),
        task: TaskKind::SensorClassification,
        labels: vec!["a".into(), "b".into(), "c".into()],
        input_shape: vec![8, 3],
        prediction_horizon_ms: None,
        minimum_examples: 1,
        minimum_examples_per_class: 1,
    };
    let config = AutoSearchConfig {
        maximum_candidates: 32,
        ..Default::default()
    };
    let a = generate_candidates(&spec, &config, &ComputeConfig::default()).unwrap();
    let b = generate_candidates(&spec, &config, &ComputeConfig::default()).unwrap();
    assert_eq!(a, b);
    for name in ["lstm", "gru", "tcn", "cnn1d"] {
        assert!(
            a.iter()
                .any(|r| r.recipe["config"]["recipe"]["architecture"] == name),
            "missing {name}"
        );
    }
}

#[test]
fn explicit_candidate_tasks_still_validate_search_and_compute_limits() {
    let spec = InspectionSpec {
        id: "fusion".into(),
        description: "classify combined image and sensor features".into(),
        task: TaskKind::Fusion,
        labels: vec!["pass".into(), "fail".into()],
        input_shape: vec![16],
        prediction_horizon_ms: None,
        minimum_examples: 1,
        minimum_examples_per_class: 1,
    };
    let search = AutoSearchConfig::default();
    let compute = ComputeConfig::default();
    validate_search(&spec, &search, &compute).unwrap();
    assert!(
        generate_candidates(&spec, &search, &compute).is_err(),
        "fusion still needs an explicit recipe"
    );
    for invalid in [
        AutoSearchConfig {
            maximum_candidates: 0,
            ..search.clone()
        },
        AutoSearchConfig {
            max_epochs: 0,
            ..search.clone()
        },
        AutoSearchConfig {
            reduction_factor: 1,
            ..search.clone()
        },
        AutoSearchConfig {
            batch_size: 129,
            ..search.clone()
        },
        AutoSearchConfig {
            output_features: 0,
            ..search.clone()
        },
    ] {
        assert!(validate_search(&spec, &invalid, &compute).is_err());
    }
    assert!(
        validate_search(
            &spec,
            &search,
            &ComputeConfig {
                memory_limit_bytes: 0,
                ..compute
            }
        )
        .is_err()
    );
}

#[test]
fn successive_halving_resumes_only_the_selected_neural_survivor() {
    let dir = tempfile::tempdir().unwrap();
    let repo = TrainingRepository::open(dir.path().join("training.sqlite")).unwrap();
    let (mut request, spec) = setup(&repo);
    request.budget.maximum_trials = 4;
    request.budget.maximum_training_time_ms = 120_000;
    request.budget.worker_limits.maximum_duration_ms = 30_000;
    let search = AutoSearchConfig {
        maximum_candidates: 32,
        min_epochs: 1,
        max_epochs: 2,
        reduction_factor: 3,
        batch_size: 8,
        ..Default::default()
    };
    request.candidates = generate_candidates(
        &spec,
        &search,
        &ComputeConfig {
            backend: ComputeBackend::Cpu,
            ..Default::default()
        },
    )
    .unwrap()
    .into_iter()
    .filter(|candidate| candidate.engine == "burn")
    .collect();
    assert_eq!(request.candidates.len(), 3);
    let limits = request.budget.worker_limits.clone();
    let experiment = repo.create_experiment(request, now_ms()).unwrap();
    let controller = AutoTrainingController::new(repo.clone(), worker(&repo, &limits));
    let result = controller.run(&experiment.id).unwrap();
    assert_eq!(result.status, ExperimentStatus::Completed);
    let trials = repo.list_experiment_trials(&experiment.id).unwrap();
    assert_eq!(trials.len(), 4);
    let child = trials
        .iter()
        .find(|trial| repo.get_job(&trial.job_id).unwrap().request.recipe["search"]["rung"] == 1)
        .unwrap();
    assert_eq!(child.status, ExperimentTrialStatus::Validated);
    let job = repo.get_job(&child.job_id).unwrap();
    assert_eq!(job.request.recipe["config"]["epochs"], 2);
    let parent = repo
        .get_experiment_trial(
            job.request.recipe["search"]["parent_trial_id"]
                .as_str()
                .unwrap(),
        )
        .unwrap();
    assert!(repo.get_job(&parent.job_id).unwrap().checkpoint.is_some());
    assert!(
        job.checkpoint.as_ref().unwrap().step
            > repo
                .get_job(&parent.job_id)
                .unwrap()
                .checkpoint
                .unwrap()
                .step
    );
    assert_eq!(child.snapshot_id, parent.snapshot_id);
}

#[test]
fn halving_finishes_each_survivor_and_preserves_distinct_learning_rates() {
    let dir = tempfile::tempdir().unwrap();
    let repo = TrainingRepository::open(dir.path().join("training.sqlite")).unwrap();
    let (mut request, spec) = setup(&repo);
    request.budget.maximum_trials = 8;
    request.budget.maximum_training_time_ms = 240_000;
    request.budget.worker_limits.maximum_duration_ms = 30_000;
    let search = AutoSearchConfig {
        maximum_candidates: 32,
        min_epochs: 1,
        max_epochs: 4,
        reduction_factor: 2,
        batch_size: 8,
        ..Default::default()
    };
    let template = generate_candidates(
        &spec,
        &search,
        &ComputeConfig {
            backend: ComputeBackend::Cpu,
            ..Default::default()
        },
    )
    .unwrap()
    .into_iter()
    .find(|candidate| candidate.engine == "burn")
    .unwrap();
    request.candidates = [0.001, 0.003, 0.01]
        .into_iter()
        .map(|rate| {
            let mut candidate = template.clone();
            candidate.recipe["config"]["learning_rate"] = json!(rate);
            candidate
        })
        .collect();
    request.candidates.push(request.candidates[0].clone());
    let limits = request.budget.worker_limits.clone();
    let experiment = repo.create_experiment(request, now_ms()).unwrap();
    let controller = AutoTrainingController::new(repo.clone(), worker(&repo, &limits));
    assert_eq!(
        controller.run(&experiment.id).unwrap().status,
        ExperimentStatus::Completed
    );
    let trials = repo.list_experiment_trials(&experiment.id).unwrap();
    assert_eq!(
        trials.len(),
        7,
        "four roots, two distinct survivors, one final continuation"
    );
    let mut rungs = [0usize; 3];
    let mut first_survivor_rates = vec![];
    for trial in &trials {
        assert_eq!(
            trial.status,
            ExperimentTrialStatus::Validated,
            "{:?}",
            trial.error
        );
        let job = repo.get_job(&trial.job_id).unwrap();
        let rung = job.request.recipe["search"]["rung"].as_u64().unwrap() as usize;
        rungs[rung] += 1;
        if rung == 1 {
            first_survivor_rates.push(
                job.request.recipe["config"]["learning_rate"]
                    .as_f64()
                    .unwrap(),
            );
        }
        if let Some(parent_id) = job.request.recipe["search"]["parent_trial_id"].as_str() {
            let parent = repo.get_experiment_trial(parent_id).unwrap();
            let parent_job = repo.get_job(&parent.job_id).unwrap();
            let mut parent_config = parent_job.request.recipe["config"].clone();
            let mut child_config = job.request.recipe["config"].clone();
            parent_config.as_object_mut().unwrap().remove("epochs");
            child_config.as_object_mut().unwrap().remove("epochs");
            assert_eq!(
                parent_config, child_config,
                "optimizer continuation must retain architecture, learning rate, and seed"
            );
            assert!(job.checkpoint.unwrap().step > parent_job.checkpoint.unwrap().step);
            assert_eq!(trial.dataset_snapshot_id, parent.dataset_snapshot_id);
        }
    }
    assert_eq!(rungs, [4, 2, 1]);
    assert_ne!(
        first_survivor_rates[0], first_survivor_rates[1],
        "duplicate candidates must not displace a distinct learning rate"
    );
}

fn cpu() -> ComputeConfig {
    ComputeConfig {
        backend: ComputeBackend::Cpu,
        ..Default::default()
    }
}

/// Controller clock shifted by a test-controlled offset from the system clock.
fn shifted_clock(offset: &Arc<AtomicI64>) -> Arc<dyn Fn() -> i64 + Send + Sync> {
    let offset = offset.clone();
    Arc::new(move || now_ms() + offset.load(Ordering::SeqCst))
}

fn add_deeper_boosting_candidate(request: &mut ExperimentRequest) {
    let mut deeper = request.candidates[0].clone();
    deeper.recipe["config"]["max_depth"] = json!(3);
    request.candidates.push(deeper);
}

fn isolation_forest() -> TrainingRequest {
    TrainingRequest {
        engine: "isolation_forest".into(),
        recipe: json!({"labels":["negative","positive"],"inspection_task":"sensor_anomaly","config":{"trees":32,"sample_size":8,"seed":7}}),
        compute: serde_json::to_value(cpu()).unwrap(),
    }
}

#[test]
fn queued_time_before_the_first_controller_step_is_not_charged() {
    let dir = tempfile::tempdir().unwrap();
    let repo = TrainingRepository::open(dir.path().join("training.sqlite")).unwrap();
    let (request, _) = setup(&repo);
    let limits = request.budget.worker_limits.clone();
    let wall = request.budget.maximum_wall_time_ms;
    let queued_since = now_ms() - 2 * wall as i64;
    let experiment = repo.create_experiment(request, queued_since).unwrap();
    let result = AutoTrainingController::new(repo.clone(), worker(&repo, &limits))
        .run(&experiment.id)
        .unwrap();
    assert_eq!(
        result.status,
        ExperimentStatus::Completed,
        "{:?}",
        result.stop_reason
    );
    assert!(result.final_evaluation_id.is_some());
    assert!(result.wall_elapsed_ms(now_ms()) < wall);
}

#[test]
fn cancelled_time_is_not_charged_and_resume_keeps_the_remaining_wall_budget() {
    let dir = tempfile::tempdir().unwrap();
    let repo = TrainingRepository::open(dir.path().join("training.sqlite")).unwrap();
    let (request, _) = setup(&repo);
    let limits = request.budget.worker_limits.clone();
    let wall = request.budget.maximum_wall_time_ms as i64;
    let experiment = repo.create_experiment(request, now_ms()).unwrap();
    let offset = Arc::new(AtomicI64::new(0));
    let controller = AutoTrainingController::new(repo.clone(), worker(&repo, &limits))
        .with_clock(shifted_clock(&offset));
    controller.run_next(&experiment.id).unwrap();
    let cancelled = repo.cancel_experiment(&experiment.id, now_ms()).unwrap();
    assert_eq!(cancelled.status, ExperimentStatus::Cancelled);
    let at = now_ms();
    assert_eq!(
        cancelled.wall_elapsed_ms(at + wall),
        cancelled.wall_elapsed_ms(at),
        "a stopped experiment must not be charged wall time"
    );
    // A whole wall budget passes while the experiment stays cancelled.
    offset.store(wall, Ordering::SeqCst);
    let resumed = repo
        .resume_experiment(&experiment.id, now_ms() + wall)
        .unwrap();
    assert_eq!(resumed.status, ExperimentStatus::Running);
    let result = controller.run(&experiment.id).unwrap();
    assert_eq!(
        result.status,
        ExperimentStatus::Completed,
        "{:?}",
        result.stop_reason
    );
    assert!(result.final_evaluation_id.is_some());
}

#[test]
fn a_clock_that_moves_backwards_never_stops_the_search_as_budget_exhausted() {
    let dir = tempfile::tempdir().unwrap();
    let repo = TrainingRepository::open(dir.path().join("training.sqlite")).unwrap();
    let (mut request, _) = setup(&repo);
    add_deeper_boosting_candidate(&mut request);
    let limits = request.budget.worker_limits.clone();
    let experiment = repo.create_experiment(request, now_ms()).unwrap();
    let offset = Arc::new(AtomicI64::new(0));
    let controller = AutoTrainingController::new(repo.clone(), worker(&repo, &limits))
        .with_clock(shifted_clock(&offset));
    controller.run_next(&experiment.id).unwrap();
    offset.store(-60_000, Ordering::SeqCst);
    // The ledger either orders the step after its latest state or reports a retryable conflict.
    if let Err(error) = controller.run_next(&experiment.id) {
        assert!(matches!(error, Error::Conflict(_)), "{error}");
        assert!(!error.to_string().contains("budget"), "{error}");
    }
    let after = repo.get_experiment(&experiment.id).unwrap();
    assert_eq!(after.status, ExperimentStatus::Running);
    assert!(after.stop_reason.is_none());
    offset.store(0, Ordering::SeqCst);
    let result = controller.run(&experiment.id).unwrap();
    assert_eq!(
        result.status,
        ExperimentStatus::Completed,
        "{:?}",
        result.stop_reason
    );
    assert_eq!(
        repo.list_experiment_trials(&experiment.id).unwrap().len(),
        2
    );
}

#[test]
fn a_search_bound_by_wall_time_keeps_its_final_audit() {
    for past_deadline in [false, true] {
        let dir = tempfile::tempdir().unwrap();
        let repo = TrainingRepository::open(dir.path().join("training.sqlite")).unwrap();
        let (mut request, _) = setup(&repo);
        add_deeper_boosting_candidate(&mut request);
        let limits = request.budget.worker_limits.clone();
        let wall = request.budget.maximum_wall_time_ms as i64;
        let experiment = repo.create_experiment(request, now_ms()).unwrap();
        let offset = Arc::new(AtomicI64::new(0));
        let controller = AutoTrainingController::new(repo.clone(), worker(&repo, &limits))
            .with_clock(shifted_clock(&offset));
        controller.run_next(&experiment.id).unwrap();
        // Leave less than the audit allowance, a tenth of the wall budget, or nothing at all.
        offset.store(
            if past_deadline {
                wall + 1_000
            } else {
                wall - wall / 20
            },
            Ordering::SeqCst,
        );
        let result = controller.run(&experiment.id).unwrap();
        assert_eq!(
            result.status,
            ExperimentStatus::Completed,
            "{past_deadline}: {:?}",
            result.stop_reason
        );
        assert!(result.final_evaluation_id.is_some());
        assert_eq!(
            result.stop_reason.as_deref(),
            Some("wall_time_budget_exhausted")
        );
        assert_eq!(
            repo.list_experiment_trials(&experiment.id).unwrap().len(),
            1,
            "no trial may start inside the audit window"
        );
    }
}

#[test]
fn a_goal_bound_validation_or_the_final_audit_never_measures_fails_before_any_training() {
    // Validation never measures the first; the final audit never reports the second.
    for name in ["teacher_agreement", "audited_samples"] {
        let dir = tempfile::tempdir().unwrap();
        let repo = TrainingRepository::open(dir.path().join("training.sqlite")).unwrap();
        let (mut request, _) = setup(&repo);
        request.goals.bounds.push(MetricBound {
            name: name.into(),
            minimum: Some(0.9),
            maximum: None,
        });
        let limits = request.budget.worker_limits.clone();
        let experiment = repo.create_experiment(request, now_ms()).unwrap();
        let result = AutoTrainingController::new(repo.clone(), worker(&repo, &limits))
            .run(&experiment.id)
            .unwrap();
        assert_eq!(result.status, ExperimentStatus::Failed, "{name}");
        assert!(
            result
                .stop_reason
                .as_deref()
                .is_some_and(|reason| reason.contains(name)),
            "{:?}",
            result.stop_reason
        );
        assert!(
            repo.list_experiment_trials(&experiment.id)
                .unwrap()
                .is_empty()
        );
    }
}

#[test]
fn a_final_holdout_below_the_audited_minimum_is_recorded_instead_of_wedging() {
    let dir = tempfile::tempdir().unwrap();
    let repo = TrainingRepository::open(dir.path().join("training.sqlite")).unwrap();
    let (mut request, _) = fixture(
        &repo,
        Fixture {
            test_size: 8,
            ..Fixture::default()
        },
    );
    request.goals.minimum_audited_samples = 12;
    let limits = request.budget.worker_limits.clone();
    let experiment = repo.create_experiment(request, now_ms()).unwrap();
    let result = AutoTrainingController::new(repo.clone(), worker(&repo, &limits))
        .run(&experiment.id)
        .unwrap();
    assert_eq!(result.status, ExperimentStatus::Completed);
    assert!(!result.target_met);
    assert!(
        result
            .unmet_constraints
            .iter()
            .any(|constraint| constraint == "audited_samples: 8 < 12"),
        "{:?}",
        result.unmet_constraints
    );
}

struct FailingPredictor;
impl SearchPredictor for FailingPredictor {
    fn predict(&mut self, _: &TensorData) -> Result<serde_json::Value> {
        Err(Error::Engine("fixture prediction failure".into()))
    }
}

/// Validates normally, then fails every prediction of the final audit.
struct FailingAuditFactory {
    loads: AtomicUsize,
}
impl SearchPredictorFactory for FailingAuditFactory {
    fn load(
        &self,
        repository: &TrainingRepository,
        artifact_id: &str,
        compute: &ComputeConfig,
    ) -> Result<Box<dyn SearchPredictor>> {
        if self.loads.fetch_add(1, Ordering::SeqCst) == 0 {
            return NativeSearchPredictorFactory.load(repository, artifact_id, compute);
        }
        Ok(Box::new(FailingPredictor))
    }
}

#[test]
fn failed_test_predictions_count_as_failures_and_settle_the_experiment() {
    for anomaly in [false, true] {
        let dir = tempfile::tempdir().unwrap();
        let repo = TrainingRepository::open(dir.path().join("training.sqlite")).unwrap();
        let (mut request, _) = setup_task(&repo, anomaly);
        if anomaly {
            request.candidates = vec![isolation_forest()];
        }
        let limits = request.budget.worker_limits.clone();
        let experiment = repo.create_experiment(request, now_ms()).unwrap();
        let result = AutoTrainingController::new(repo.clone(), worker(&repo, &limits))
            .with_predictor_factory(Arc::new(FailingAuditFactory {
                loads: AtomicUsize::new(0),
            }))
            .run(&experiment.id)
            .unwrap();
        if anomaly {
            // Anomaly audits cannot score a failed prediction, so the experiment ends without one.
            assert_eq!(result.status, ExperimentStatus::Failed);
            assert!(result.final_evaluation_id.is_none());
            assert!(
                result
                    .stop_reason
                    .as_deref()
                    .is_some_and(|reason| reason.starts_with("final_evaluation_failed")),
                "{:?}",
                result.stop_reason
            );
            continue;
        }
        assert_eq!(
            result.status,
            ExperimentStatus::Completed,
            "{:?}",
            result.stop_reason
        );
        let evaluation = repo
            .get_evaluation(result.final_evaluation_id.as_ref().unwrap())
            .unwrap();
        assert_eq!(evaluation.metrics["failure_rate"], 1.0);
        assert_eq!(evaluation.metrics["accuracy"], 0.0);
        let winner = repo
            .get_experiment_trial(result.selected_trial_id.as_ref().unwrap())
            .unwrap();
        let predictions = repo
            .predictions(winner.artifact_id.as_ref().unwrap())
            .unwrap();
        assert_eq!(predictions.len(), 16);
        assert!(
            predictions
                .iter()
                .all(|prediction| prediction.failed && prediction.details["error"].is_string())
        );
    }
}

/// Validates normally, fails the final audit's first predictor load as a full disk would, then
/// loads normally again.
struct StorageFailureAtAudit {
    loads: AtomicUsize,
}
impl SearchPredictorFactory for StorageFailureAtAudit {
    fn load(
        &self,
        repository: &TrainingRepository,
        artifact_id: &str,
        compute: &ComputeConfig,
    ) -> Result<Box<dyn SearchPredictor>> {
        if self.loads.fetch_add(1, Ordering::SeqCst) == 1 {
            return Err(Error::Io(std::io::Error::other("no space left on device")));
        }
        NativeSearchPredictorFactory.load(repository, artifact_id, compute)
    }
}

#[test]
fn a_storage_failure_in_the_final_audit_leaves_it_to_a_later_finalize() {
    let dir = tempfile::tempdir().unwrap();
    let repo = TrainingRepository::open(dir.path().join("training.sqlite")).unwrap();
    let (request, _) = setup(&repo);
    let limits = request.budget.worker_limits.clone();
    let experiment = repo.create_experiment(request, now_ms()).unwrap();
    let controller = AutoTrainingController::new(repo.clone(), worker(&repo, &limits))
        .with_predictor_factory(Arc::new(StorageFailureAtAudit {
            loads: AtomicUsize::new(0),
        }));
    let error = controller.run(&experiment.id).unwrap_err();
    assert!(matches!(error, Error::Io(_)), "{error}");
    let waiting = repo.get_experiment(&experiment.id).unwrap();
    assert_eq!(waiting.status, ExperimentStatus::Evaluating);
    assert!(waiting.stop_reason.is_none(), "{:?}", waiting.stop_reason);
    let result = controller.finalize(&experiment.id).unwrap();
    assert_eq!(
        result.status,
        ExperimentStatus::Completed,
        "{:?}",
        result.stop_reason
    );
    assert!(result.final_evaluation_id.is_some());
}

#[test]
fn the_final_audit_never_reuses_predictions_it_did_not_record() {
    let dir = tempfile::tempdir().unwrap();
    let repo = TrainingRepository::open(dir.path().join("training.sqlite")).unwrap();
    let (request, spec) = setup(&repo);
    let limits = request.budget.worker_limits.clone();
    let experiment = repo.create_experiment(request, now_ms()).unwrap();
    let controller = AutoTrainingController::new(repo.clone(), worker(&repo, &limits));
    controller.run_next(&experiment.id).unwrap();
    let artifact_id = repo
        .list_experiment_trials(&experiment.id)
        .unwrap()
        .remove(0)
        .artifact_id
        .unwrap();
    // A shadow prediction for a holdout sample claims a label the audit never produced.
    repo.record_prediction(&PredictionRecord {
        sample_id: "2-0".into(),
        artifact_id: artifact_id.clone(),
        predicted_label: Some(spec.labels[1].clone()),
        teacher_label: None,
        actual_label: None,
        actual_source: None,
        latency_ms: 1.0,
        failed: false,
        recorded_at_ms: now_ms(),
        details: json!({"partition":"shadow"}),
    })
    .unwrap();
    let result = controller.finalize(&experiment.id).unwrap();
    assert_eq!(result.status, ExperimentStatus::Failed);
    assert!(result.final_evaluation_id.is_none());
    assert!(
        result
            .stop_reason
            .as_deref()
            .is_some_and(|reason| reason.contains("2-0")),
        "{:?}",
        result.stop_reason
    );
}

/// What the fifth prediction of the final audit does to the experiment.
#[derive(Clone)]
enum Interruption {
    Cancel { experiment_id: String },
    AdvanceClock { offset: Arc<AtomicI64>, by_ms: i64 },
}
impl Interruption {
    fn apply(&self, repository: &TrainingRepository) -> Result<()> {
        match self {
            Self::Cancel { experiment_id } => repository
                .cancel_experiment(experiment_id, now_ms())
                .map(drop),
            Self::AdvanceClock { offset, by_ms } => {
                offset.fetch_add(*by_ms, Ordering::SeqCst);
                Ok(())
            }
        }
    }
}

struct InterruptingPredictor {
    inner: Box<dyn SearchPredictor>,
    repository: TrainingRepository,
    remaining: usize,
    interruption: Option<Interruption>,
}
impl SearchPredictor for InterruptingPredictor {
    fn predict(&mut self, input: &TensorData) -> Result<serde_json::Value> {
        if self.remaining == 0
            && let Some(interruption) = self.interruption.take()
        {
            interruption.apply(&self.repository)?;
        }
        self.remaining = self.remaining.saturating_sub(1);
        self.inner.predict(input)
    }
}

/// Interrupts the final audit (the second load) once, during its fifth prediction.
struct InterruptedAudit {
    loads: AtomicUsize,
    interruption: Interruption,
}
impl InterruptedAudit {
    fn new(interruption: Interruption) -> Self {
        Self {
            loads: AtomicUsize::new(0),
            interruption,
        }
    }
}
impl SearchPredictorFactory for InterruptedAudit {
    fn load(
        &self,
        repository: &TrainingRepository,
        artifact_id: &str,
        compute: &ComputeConfig,
    ) -> Result<Box<dyn SearchPredictor>> {
        let inner = NativeSearchPredictorFactory.load(repository, artifact_id, compute)?;
        if self.loads.fetch_add(1, Ordering::SeqCst) != 1 {
            return Ok(inner);
        }
        Ok(Box::new(InterruptingPredictor {
            inner,
            repository: repository.clone(),
            remaining: 4,
            interruption: Some(self.interruption.clone()),
        }))
    }
}

#[test]
fn an_audit_cut_off_by_the_wall_budget_keeps_its_predictions_without_final_evidence() {
    let dir = tempfile::tempdir().unwrap();
    let repo = TrainingRepository::open(dir.path().join("training.sqlite")).unwrap();
    let (request, _) = setup(&repo);
    let limits = request.budget.worker_limits.clone();
    // Past the wall budget and the audit allowance that follows the frozen winner.
    let jump_ms = 2 * request.budget.maximum_wall_time_ms as i64;
    let experiment = repo.create_experiment(request, now_ms()).unwrap();
    let offset = Arc::new(AtomicI64::new(0));
    let result = AutoTrainingController::new(repo.clone(), worker(&repo, &limits))
        .with_clock(shifted_clock(&offset))
        .with_predictor_factory(Arc::new(InterruptedAudit::new(
            Interruption::AdvanceClock {
                offset: offset.clone(),
                by_ms: jump_ms,
            },
        )))
        .run(&experiment.id)
        .unwrap();
    assert_eq!(
        result.status,
        ExperimentStatus::BudgetExhausted,
        "{:?}",
        result.stop_reason
    );
    assert_eq!(
        result.stop_reason.as_deref(),
        Some("wall_time_budget_exhausted")
    );
    assert!(result.final_evaluation_id.is_none());
    assert!(
        result
            .unmet_constraints
            .iter()
            .any(|constraint| constraint == "final independent evaluation is unavailable"),
        "{:?}",
        result.unmet_constraints
    );
    let winner = repo
        .get_experiment_trial(result.selected_trial_id.as_ref().unwrap())
        .unwrap();
    let recorded = repo
        .predictions(winner.artifact_id.as_ref().unwrap())
        .unwrap();
    assert_eq!(recorded.len(), 5, "the audit stops at the next holdout row");
}

#[test]
fn a_resumed_final_audit_reuses_only_its_own_recorded_predictions() {
    let dir = tempfile::tempdir().unwrap();
    let repo = TrainingRepository::open(dir.path().join("training.sqlite")).unwrap();
    let (request, _) = setup(&repo);
    let limits = request.budget.worker_limits.clone();
    let experiment = repo.create_experiment(request, now_ms()).unwrap();
    let controller = AutoTrainingController::new(repo.clone(), worker(&repo, &limits))
        .with_predictor_factory(Arc::new(InterruptedAudit::new(Interruption::Cancel {
            experiment_id: experiment.id.clone(),
        })));
    controller.run_next(&experiment.id).unwrap();
    assert!(matches!(
        controller.finalize(&experiment.id),
        Err(Error::Cancelled)
    ));
    let winner = repo
        .get_experiment_trial(
            repo.get_experiment(&experiment.id)
                .unwrap()
                .selected_trial_id
                .as_ref()
                .unwrap(),
        )
        .unwrap();
    let artifact_id = winner.artifact_id.unwrap();
    let partial = repo.predictions(&artifact_id).unwrap();
    assert_eq!(partial.len(), 5);
    repo.resume_frozen_experiment_audit(&experiment.id, now_ms())
        .unwrap();
    let result = controller.finalize(&experiment.id).unwrap();
    assert_eq!(
        result.status,
        ExperimentStatus::Completed,
        "{:?}",
        result.stop_reason
    );
    let predictions = repo.predictions(&artifact_id).unwrap();
    assert_eq!(predictions.len(), 16);
    for earlier in &partial {
        assert!(predictions.contains(earlier), "{}", earlier.sample_id);
    }
}

#[test]
fn strict_gpu_compute_generates_no_native_candidates() {
    let strict = ComputeConfig {
        backend: ComputeBackend::Cuda,
        allow_cpu_fallback: false,
        ..Default::default()
    };
    let sensors = InspectionSpec {
        id: "sensors".into(),
        description: "classify sensor features".into(),
        task: TaskKind::SensorClassification,
        labels: vec!["ok".into(), "defect".into()],
        input_shape: vec![4],
        prediction_horizon_ms: None,
        minimum_examples: 1,
        minimum_examples_per_class: 1,
    };
    let search = AutoSearchConfig::default();
    let candidates = generate_candidates(&sensors, &search, &strict).unwrap();
    assert!(!candidates.is_empty());
    assert!(
        candidates
            .iter()
            .all(|candidate| candidate.engine == "burn")
    );
    assert!(
        generate_candidates(&sensors, &search, &ComputeConfig::default())
            .unwrap()
            .iter()
            .any(|candidate| candidate.engine == "histogram_gradient_boosting")
    );
    let patches = InspectionSpec {
        id: "patches".into(),
        description: "score patch embeddings".into(),
        task: TaskKind::VisualAnomaly,
        labels: vec![],
        input_shape: vec![16, 8],
        ..sensors
    };
    let error = generate_candidates(&patches, &search, &strict)
        .unwrap_err()
        .to_string();
    assert!(error.contains("allow_cpu_fallback"), "{error}");
}

#[test]
fn generated_vision_batches_are_the_largest_halving_that_fits_the_memory_limit() {
    let spec = InspectionSpec {
        id: "images".into(),
        description: "classify images".into(),
        task: TaskKind::ImageClassification,
        labels: vec!["ok".into(), "defect".into()],
        input_shape: vec![3, 224, 224],
        prediction_horizon_ms: None,
        minimum_examples: 1,
        minimum_examples_per_class: 1,
    };
    let search = AutoSearchConfig::default();
    let compute = ComputeConfig::default();
    let candidates = generate_candidates(&spec, &search, &compute).unwrap();
    assert!(!candidates.is_empty());
    for candidate in &candidates {
        let config: burn::TrainingConfig =
            serde_json::from_value(candidate.recipe["config"].clone()).unwrap();
        let estimate = |batch| {
            config
                .recipe
                .estimated_training_bytes(&spec.input_shape, batch)
                .unwrap()
        };
        assert!(estimate(config.batch_size) <= compute.memory_limit_bytes);
        assert!(
            config.batch_size == search.batch_size
                || estimate(config.batch_size * 2) > compute.memory_limit_bytes
        );
    }
    let resnet = burn::Recipe::ResNet18 {
        input_channels: 3,
        classes: 2,
        base_channels: 16,
    };
    let tight = ComputeConfig {
        memory_limit_bytes: resnet.estimated_training_bytes(&[3, 224, 224], 4).unwrap(),
        ..Default::default()
    };
    let chosen = generate_candidates(&spec, &search, &tight)
        .unwrap()
        .into_iter()
        .find(|candidate| {
            candidate.recipe["config"]["recipe"]["architecture"] == "res_net18"
                && candidate.recipe["config"]["recipe"]["base_channels"] == 16
        })
        .unwrap();
    assert_eq!(chosen.recipe["config"]["batch_size"], 4);
    let huge = InspectionSpec {
        input_shape: vec![3, 2048, 2048],
        ..spec
    };
    let error = generate_candidates(&huge, &search, &compute)
        .unwrap_err()
        .to_string();
    assert!(
        error.contains("memory budget of 2147483648 bytes") && error.contains("smallest estimate"),
        "{error}"
    );
}

/// The fixture search with one generated Burn MLP at batch 8, its config, and the serialized
/// size of the worker view, which every experiment over the fixture snapshot shares.
fn generated_mlp_search(
    repo: &TrainingRepository,
) -> (ExperimentRequest, burn::TrainingConfig, u64) {
    let (mut request, spec) = setup(repo);
    let search = AutoSearchConfig {
        min_epochs: 1,
        max_epochs: 1,
        batch_size: 8,
        ..Default::default()
    };
    let candidate = generate_candidates(&spec, &search, &cpu())
        .unwrap()
        .into_iter()
        .find(|candidate| candidate.engine == "burn")
        .unwrap();
    let config: burn::TrainingConfig =
        serde_json::from_value(candidate.recipe["config"].clone()).unwrap();
    assert_eq!(config.batch_size, 8);
    request.candidates = vec![candidate];
    let probe = repo.create_experiment(request.clone(), now_ms()).unwrap();
    let view = repo
        .experiment_result(&probe.id)
        .unwrap()
        .datasets
        .remove(0)
        .training_snapshot_id;
    let view_bytes = serde_json::to_vec(&repo.get_snapshot(&view).unwrap())
        .unwrap()
        .len() as u64;
    (request, config, view_bytes)
}

#[test]
fn a_full_candidate_list_names_the_slot_a_resized_copy_needs() {
    let dir = tempfile::tempdir().unwrap();
    let repo = TrainingRepository::open(dir.path().join("training.sqlite")).unwrap();
    let (mut request, config, view_bytes) = generated_mlp_search(&repo);
    request.budget.maximum_trials = 1;
    request.budget.worker_limits.memory_budget_bytes =
        config.recipe.estimated_training_bytes(&[1], 1).unwrap() + 12 * view_bytes;
    let limits = request.budget.worker_limits.clone();
    let experiment = repo.create_experiment(request, now_ms()).unwrap();
    let result = AutoTrainingController::new(repo.clone(), worker(&repo, &limits))
        .run(&experiment.id)
        .unwrap();
    assert_eq!(
        result.status,
        ExperimentStatus::BudgetExhausted,
        "{:?}",
        result.stop_reason
    );
    let reason = result.stop_reason.unwrap_or_default();
    assert!(
        reason.contains("batch 1") && reason.contains("maximum_trials"),
        "{reason}"
    );
    assert!(
        repo.list_experiment_trials(&experiment.id)
            .unwrap()
            .is_empty()
    );
}

#[test]
fn a_worker_view_beyond_a_twelfth_of_the_budget_reports_what_training_needs() {
    let dir = tempfile::tempdir().unwrap();
    let repo = TrainingRepository::open(dir.path().join("training.sqlite")).unwrap();
    let (mut request, config, view_bytes) = generated_mlp_search(&repo);
    let budget = 12 * view_bytes - 1;
    request.budget.worker_limits.memory_budget_bytes = budget;
    let limits = request.budget.worker_limits.clone();
    let experiment = repo.create_experiment(request, now_ms()).unwrap();
    let result = AutoTrainingController::new(repo.clone(), worker(&repo, &limits))
        .run(&experiment.id)
        .unwrap();
    assert_eq!(
        result.status,
        ExperimentStatus::BudgetExhausted,
        "{:?}",
        result.stop_reason
    );
    let needed = config.recipe.estimated_training_bytes(&[1], 1).unwrap() + 12 * view_bytes;
    let reason = result.stop_reason.unwrap_or_default();
    assert!(
        reason.contains(&format!(
            "budget of {budget} bytes; the smallest estimate is {needed} bytes"
        )) && reason.contains(&format!("{view_bytes} byte training view")),
        "{reason}"
    );
    let skipped = repo
        .experiment_events(&experiment.id)
        .unwrap()
        .into_iter()
        .find(|event| event["kind"] == "candidate_skipped")
        .unwrap();
    assert_eq!(skipped["body"]["view_bytes"], view_bytes);
    assert!(
        repo.list_experiment_trials(&experiment.id)
            .unwrap()
            .is_empty()
    );
}

#[test]
fn an_oversized_candidate_trains_with_the_largest_batch_that_fits_the_worker() {
    let dir = tempfile::tempdir().unwrap();
    let repo = TrainingRepository::open(dir.path().join("training.sqlite")).unwrap();
    let (mut request, config, view_bytes) = generated_mlp_search(&repo);
    request.budget.worker_limits.memory_budget_bytes =
        config.recipe.estimated_training_bytes(&[1], 1).unwrap() + 12 * view_bytes;
    let limits = request.budget.worker_limits.clone();
    let experiment = repo.create_experiment(request, now_ms()).unwrap();
    let result = AutoTrainingController::new(repo.clone(), worker(&repo, &limits))
        .run(&experiment.id)
        .unwrap();
    assert_eq!(
        result.status,
        ExperimentStatus::Completed,
        "{:?}",
        result.stop_reason
    );
    let trials = repo.list_experiment_trials(&experiment.id).unwrap();
    assert_eq!(trials.len(), 1);
    assert_eq!(trials[0].candidate_index, 1);
    assert_eq!(
        trials[0].status,
        ExperimentTrialStatus::Validated,
        "{:?}",
        trials[0].error
    );
    assert_eq!(
        repo.get_job(&trials[0].job_id).unwrap().request.recipe["config"]["batch_size"],
        1
    );
    assert!(
        repo.experiment_events(&experiment.id)
            .unwrap()
            .iter()
            .any(|event| event["kind"] == "candidate_resized"
                && event["body"]["candidate_index"] == 0)
    );
}

#[test]
fn candidates_that_never_fit_fail_fast_naming_the_budget_and_smallest_estimate() {
    let dir = tempfile::tempdir().unwrap();
    let repo = TrainingRepository::open(dir.path().join("training.sqlite")).unwrap();
    let (mut request, spec) = setup(&repo);
    let search = AutoSearchConfig {
        min_epochs: 1,
        max_epochs: 1,
        ..Default::default()
    };
    request.candidates = generate_candidates(&spec, &search, &cpu())
        .unwrap()
        .into_iter()
        .filter(|candidate| candidate.engine == "burn")
        .collect();
    // Burn reserves more than this for its runtime alone.
    request.budget.worker_limits.memory_budget_bytes = 8 * 1024 * 1024;
    let limits = request.budget.worker_limits.clone();
    let experiment = repo.create_experiment(request, now_ms()).unwrap();
    let result = AutoTrainingController::new(repo.clone(), worker(&repo, &limits))
        .run(&experiment.id)
        .unwrap();
    assert_eq!(result.status, ExperimentStatus::BudgetExhausted);
    let reason = result.stop_reason.unwrap_or_default();
    assert!(
        reason.contains("memory budget of 8388608 bytes") && reason.contains("smallest estimate"),
        "{reason}"
    );
    assert!(
        repo.list_experiment_trials(&experiment.id)
            .unwrap()
            .is_empty()
    );
}

/// Fails the validation of every continuation so the controller must decide about retries.
struct FailingContinuations;
impl SearchPredictorFactory for FailingContinuations {
    fn load(
        &self,
        repository: &TrainingRepository,
        artifact_id: &str,
        compute: &ComputeConfig,
    ) -> Result<Box<dyn SearchPredictor>> {
        let artifact = repository.get_artifact(artifact_id)?;
        if repository.get_job(&artifact.job_id)?.request.recipe["search"]["rung"].as_u64() > Some(0)
        {
            return Err(Error::Engine(
                "fixture continuation validation failure".into(),
            ));
        }
        NativeSearchPredictorFactory.load(repository, artifact_id, compute)
    }
}

#[test]
fn a_failed_continuation_is_retried_once_with_fewer_epochs() {
    let dir = tempfile::tempdir().unwrap();
    let repo = TrainingRepository::open(dir.path().join("training.sqlite")).unwrap();
    let (mut request, spec) = setup(&repo);
    request.budget.maximum_trials = 6;
    request.budget.maximum_training_time_ms = 360_000;
    request.budget.worker_limits.maximum_duration_ms = 60_000;
    let search = AutoSearchConfig {
        maximum_candidates: 32,
        min_epochs: 1,
        max_epochs: 8,
        reduction_factor: 4,
        batch_size: 8,
        ..Default::default()
    };
    request.candidates = generate_candidates(&spec, &search, &cpu())
        .unwrap()
        .into_iter()
        .filter(|candidate| candidate.engine == "burn")
        .collect();
    assert_eq!(request.candidates.len(), 3);
    let limits = request.budget.worker_limits.clone();
    let experiment = repo.create_experiment(request, now_ms()).unwrap();
    let result = AutoTrainingController::new(repo.clone(), worker(&repo, &limits))
        .with_predictor_factory(Arc::new(FailingContinuations))
        .run(&experiment.id)
        .unwrap();
    assert_eq!(
        result.status,
        ExperimentStatus::Completed,
        "{:?}",
        result.stop_reason
    );
    let continuations = repo
        .list_experiment_trials(&experiment.id)
        .unwrap()
        .into_iter()
        .map(|trial| {
            let recipe = repo.get_job(&trial.job_id).unwrap().request.recipe;
            (trial.status, recipe)
        })
        .filter(|(_, recipe)| recipe["search"]["rung"] == 1)
        .collect::<Vec<_>>();
    assert_eq!(
        continuations
            .iter()
            .map(|(_, recipe)| recipe["config"]["epochs"].as_u64().unwrap())
            .collect::<Vec<_>>(),
        vec![4, 2]
    );
    assert!(
        continuations
            .iter()
            .all(|(status, _)| *status == ExperimentTrialStatus::Failed)
    );
}

/// The 95th percentile of normal-sample scores, as the controller calibrates it.
fn normal_score_percentile(predictor: &mut dyn SearchPredictor, rows: &[TrainingSample]) -> f64 {
    let mut scores = rows
        .iter()
        .filter_map(|row| {
            let sample: Sample = serde_json::from_value(row.payload["sample"].clone()).unwrap();
            matches!(sample.annotation, Annotation::Anomaly { is_anomaly: false }).then(|| {
                let mut input = sample.input;
                input.shape.insert(0, 1);
                predictor.predict(&input).unwrap()["scores"][0]
                    .as_f64()
                    .unwrap()
            })
        })
        .collect::<Vec<_>>();
    scores.sort_by(f64::total_cmp);
    scores[((scores.len() - 1) as f64 * 0.95).floor() as usize].max(1e-8)
}

#[test]
fn the_anomaly_decision_scale_is_fitted_on_held_out_normal_samples() {
    let dir = tempfile::tempdir().unwrap();
    let repo = TrainingRepository::open(dir.path().join("training.sqlite")).unwrap();
    let (mut request, _) = fixture(
        &repo,
        Fixture {
            anomaly: true,
            validation_spread: 0.5,
            ..Fixture::default()
        },
    );
    request.candidates = vec![isolation_forest()];
    let limits = request.budget.worker_limits.clone();
    let experiment = repo.create_experiment(request, now_ms()).unwrap();
    AutoTrainingController::new(repo.clone(), worker(&repo, &limits))
        .run_next(&experiment.id)
        .unwrap();
    let trial = repo
        .list_experiment_trials(&experiment.id)
        .unwrap()
        .remove(0);
    assert_eq!(
        trial.status,
        ExperimentTrialStatus::Validated,
        "{:?}",
        trial.error
    );
    let mut predictor = NativeSearchPredictorFactory
        .load(&repo, trial.artifact_id.as_ref().unwrap(), &cpu())
        .unwrap();
    let snapshot = repo.get_snapshot(&experiment.request.snapshot_id).unwrap();
    let held_out = normal_score_percentile(&mut *predictor, &snapshot.validation);
    let in_sample = normal_score_percentile(&mut *predictor, &snapshot.train);
    assert_ne!(held_out, in_sample, "the fixture must separate the two");
    assert_eq!(trial.validation_metrics["anomaly_scale"], held_out);
}

/// Another experiment's trial job reserves the whole native CPU budget until its lease is
/// failed.
fn occupy_native_budget(repo: &TrainingRepository, request: &ExperimentRequest) -> JobLease {
    let holder = repo.create_experiment(request.clone(), now_ms()).unwrap();
    let trial = repo
        .submit_experiment_trial(&holder.id, 0, "holder", now_ms())
        .unwrap();
    let budget = request.budget.worker_limits.memory_budget_bytes;
    repo.claim_job_with_resources(
        &trial.job_id,
        "holder",
        600_000,
        now_ms(),
        Some(ResourceRequest {
            key: "cpu:0".into(),
            estimated_bytes: budget,
            budget_bytes: budget,
        }),
    )
    .unwrap()
}

fn blocked_events(repo: &TrainingRepository, experiment_id: &str) -> Vec<serde_json::Value> {
    repo.experiment_events(experiment_id)
        .unwrap()
        .into_iter()
        .filter(|event| event["kind"] == "trial_blocked")
        .map(|event| event["body"].clone())
        .collect()
}

/// Releases the holder half a second after the experiment's first trial failed to claim the
/// budget, past the immediate retry that follows the submission, so only a backoff retry can
/// claim it.
fn release_once_blocked(repo: &TrainingRepository, id: &str, holder: &JobLease) -> bool {
    let deadline = Instant::now() + Duration::from_secs(30);
    while Instant::now() < deadline {
        let blocked = repo
            .list_experiment_trials(id)
            .unwrap()
            .first()
            .is_some_and(|trial| repo.get_job(&trial.job_id).unwrap().error.is_some());
        if blocked {
            std::thread::sleep(Duration::from_millis(500));
            repo.fail_job(holder, "released by the fixture", now_ms())
                .unwrap();
            return true;
        }
        std::thread::sleep(Duration::from_millis(5));
    }
    false
}

#[test]
fn a_step_held_back_by_an_occupied_budget_names_the_holder_once() {
    let dir = tempfile::tempdir().unwrap();
    let repo = TrainingRepository::open(dir.path().join("training.sqlite")).unwrap();
    let (request, _) = setup(&repo);
    let limits = request.budget.worker_limits.clone();
    let holder = occupy_native_budget(&repo, &request);
    let experiment = repo.create_experiment(request, now_ms()).unwrap();
    let controller = AutoTrainingController::new(repo.clone(), worker(&repo, &limits));
    for _ in 0..2 {
        let result = controller.run_next(&experiment.id).unwrap();
        assert_eq!(result.status, ExperimentStatus::Running);
    }
    let trial = repo
        .list_experiment_trials(&experiment.id)
        .unwrap()
        .remove(0);
    assert_eq!(trial.status, ExperimentTrialStatus::Scheduled);
    let job = repo.get_job(&trial.job_id).unwrap();
    assert_eq!(job.status, JobStatus::Queued);
    assert!(
        job.error
            .as_deref()
            .is_some_and(|error| error.contains("is occupied")),
        "{:?}",
        job.error
    );
    let blocked = blocked_events(&repo, &experiment.id);
    assert_eq!(blocked.len(), 1, "{blocked:?}");
    assert_eq!(blocked[0]["trial_id"], trial.id.as_str());
    assert!(
        blocked[0]["reason"]
            .as_str()
            .is_some_and(|reason| reason.contains(&holder.job_id)),
        "{blocked:?}"
    );
}

#[test]
fn a_run_waits_for_an_occupied_budget_instead_of_returning_with_a_scheduled_trial() {
    let dir = tempfile::tempdir().unwrap();
    let repo = TrainingRepository::open(dir.path().join("training.sqlite")).unwrap();
    let (request, _) = setup(&repo);
    let limits = request.budget.worker_limits.clone();
    let holder = occupy_native_budget(&repo, &request);
    let experiment = repo.create_experiment(request, now_ms()).unwrap();
    let release = std::thread::spawn({
        let repo = repo.clone();
        let experiment_id = experiment.id.clone();
        move || release_once_blocked(&repo, &experiment_id, &holder)
    });
    let result = AutoTrainingController::new(repo.clone(), worker(&repo, &limits))
        .run(&experiment.id)
        .unwrap();
    assert!(
        release.join().unwrap(),
        "the trial never reported the occupied budget"
    );
    assert_eq!(
        result.status,
        ExperimentStatus::Completed,
        "{:?}",
        result.stop_reason
    );
    assert_eq!(blocked_events(&repo, &experiment.id).len(), 1);
}
