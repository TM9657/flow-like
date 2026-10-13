use super::*;
use serde_json::json;
use std::sync::{Arc, Barrier};

fn setup() -> (
    tempfile::TempDir,
    TrainingRepository,
    DatasetSnapshot,
    ExperimentRequest,
) {
    let directory = tempfile::tempdir().unwrap();
    let repository = TrainingRepository::open(directory.path().join("training.sqlite")).unwrap();
    let stream = StreamKey {
        project_id: "app:board".into(),
        stream_id: "inspection".into(),
        inspection_version: "v1".into(),
    };
    for index in 0..20 {
        repository.record_sample(&stream,&TrainingSample {
            id:format!("sample-{index}"),annotation_revision:1,group_id:format!("lot-{index}"),captured_at_ms:index,label_available_at_ms:index,content_digest:format!("{index:064x}"),source:LabelSource::Reviewed,accepted:true,
            payload:json!({"input":{"shape":[1],"values":[index]},"label":"good","annotation":{"kind":"class","class_id":0},"provenance":{"kind":"reviewed","reviewer":"operator"}}),
        }).unwrap();
    }
    let snapshot = repository
        .snapshot(
            &stream,
            SplitPolicy::Group {
                train_fraction: 0.5,
                validation_fraction: 0.25,
                seed: 42,
            },
            100,
        )
        .unwrap();
    let candidate = TrainingRequest {
        engine: "fixture".into(),
        recipe: json!({"labels":["good"]}),
        compute: json!({}),
    };
    let request = ExperimentRequest {
        stream,
        snapshot_id: snapshot.id.clone(),
        spec: json!({"labels":["good"],"input_shape":[1]}),
        candidates: vec![candidate.clone(), candidate],
        goals: ExperimentGoals {
            task: EvaluationTask::Classification,
            primary_metric: "accuracy".into(),
            direction: MetricDirection::Maximize,
            minimum_audited_samples: 2,
            bounds: vec![MetricBound {
                name: "accuracy".into(),
                minimum: Some(0.9),
                maximum: None,
            }],
        },
        budget: ExperimentBudget {
            maximum_trials: 4,
            maximum_llm_tokens: 100,
            maximum_llm_cost_micros: 100,
            maximum_training_time_ms: 3000,
            maximum_wall_time_ms: 10000,
            maximum_artifact_bytes: 100,
            worker_limits: WorkerLimits {
                maximum_duration_ms: 1000,
                maximum_artifact_bytes: 30,
                ..Default::default()
            },
            ..Default::default()
        },
        source_table_versions: vec![json!({"table":"source","branch":"main","version":7})],
        created_tables: vec![],
        updated_table_versions: vec![],
        preprocessing_manifest: json!({"kind":"identity"}),
        context: Value::Null,
    };
    (directory, repository, snapshot, request)
}

fn trained(repo: &TrainingRepository, trial: &ExperimentTrial, time: i64) -> ModelArtifact {
    let lease = repo.claim_job(&trial.job_id, "worker", 1000, time).unwrap();
    repo.finish_training(&lease, b"weights", json!({"engine":"fixture"}), time + 1)
        .unwrap()
}

fn validate(
    repo: &TrainingRepository,
    trial: &ExperimentTrial,
    accuracy: f64,
    time: i64,
) -> ExperimentTrial {
    repo.record_experiment_trial_validation(
        &trial.id,
        &BTreeMap::from([
            ("accuracy".into(), accuracy),
            ("audited_samples".into(), 5.),
        ]),
        10,
        time,
    )
    .unwrap()
}

#[test]
fn frozen_holdout_is_hidden_from_workers_and_survives_restart() {
    let (directory, repo, snapshot, request) = setup();
    let experiment = repo.create_experiment(request, 100).unwrap();
    let trial = repo
        .submit_experiment_trial(&experiment.id, 0, "first", 101)
        .unwrap();
    let job = repo.get_job(&trial.job_id).unwrap();
    let worker_data = repo.get_snapshot(&job.snapshot_id).unwrap();
    assert_eq!(worker_data.train, snapshot.train);
    assert_eq!(worker_data.validation, snapshot.validation);
    assert!(worker_data.test.is_empty());
    assert_ne!(worker_data.digest, snapshot.digest);
    assert_eq!(job.stream, snapshot.stream);
    assert_eq!(
        job.request.recipe["preprocessing_manifest"],
        json!({"kind":"identity"})
    );
    let artifact = trained(&repo, &trial, 102);
    assert_eq!(artifact.stream, snapshot.stream);
    let repo = TrainingRepository::open(directory.path().join("training.sqlite")).unwrap();
    assert_eq!(
        repo.get_snapshot(&trial.dataset_snapshot_id).unwrap(),
        snapshot
    );
    validate(&repo, &trial, 1., 104);
    assert_eq!(
        repo.experiment_result(&experiment.id)
            .unwrap()
            .best_artifact
            .unwrap()
            .id,
        artifact.id
    );
}

#[test]
fn experiment_events_are_bounded_append_only_and_survive_finalization() {
    let (directory, repo, _snapshot, request) = setup();
    let experiment = repo.create_experiment(request, 100).unwrap();
    assert!(
        repo.record_experiment_event(&experiment.id, "decision", json!({}), 99)
            .is_err()
    );
    assert!(
        repo.record_experiment_event(
            &experiment.id,
            "decision",
            json!("x".repeat(64 * 1024)),
            100
        )
        .is_err()
    );
    assert!(
        repo.shadow_telemetry()
            .record_experiment_event(&experiment.id, "decision", json!({}), 100)
            .is_err()
    );
    for index in 0..255 {
        repo.record_experiment_event(&experiment.id, "decision", json!({"index":index}), 100)
            .unwrap();
    }
    let finished = repo
        .finalize_experiment(&experiment.id, None, "no successful candidate", 101)
        .unwrap();
    let before = serde_json::to_value(&finished).unwrap();
    let barrier = Arc::new(Barrier::new(3));
    let mut handles = vec![];
    for index in 0..2 {
        let repo = repo.clone();
        let barrier = barrier.clone();
        let id = experiment.id.clone();
        handles.push(std::thread::spawn(move || {
            barrier.wait();
            repo.record_experiment_event(&id, "completion", json!({"index":index}), 102)
        }));
    }
    barrier.wait();
    assert_eq!(
        handles
            .into_iter()
            .map(|handle| handle.join().unwrap())
            .filter(Result::is_ok)
            .count(),
        1
    );
    let reopened = TrainingRepository::open(directory.path().join("training.sqlite")).unwrap();
    let events = reopened.experiment_events(&experiment.id).unwrap();
    assert_eq!(events.len(), 256);
    for (index, event) in events.iter().enumerate() {
        assert_eq!(event["sequence"], index);
    }
    assert_eq!(events[255]["kind"], "completion");
    let result = reopened.experiment_result(&experiment.id).unwrap();
    assert_eq!(result.events, events);
    assert_eq!(serde_json::to_value(result.experiment).unwrap(), before);
}

#[test]
fn selection_prefers_feasible_models_then_stable_candidate_order() {
    let (_directory, repo, _snapshot, mut request) = setup();
    request.candidates.push(request.candidates[0].clone());
    request.budget.maximum_parallel_trials = 3;
    request.goals.bounds.push(MetricBound {
        name: "mean_latency_ms".into(),
        minimum: None,
        maximum: Some(10.),
    });
    let experiment = repo.create_experiment(request, 100).unwrap();
    let mut trials = vec![];
    for index in 0..3 {
        let trial = repo
            .submit_experiment_trial(&experiment.id, index, &format!("candidate-{index}"), 101)
            .unwrap();
        trained(&repo, &trial, 102);
        trials.push(trial);
    }
    for (index, accuracy, latency) in [(2, 1., 20.), (1, 0.9, 2.), (0, 0.9, 2.)] {
        repo.record_experiment_trial_validation(
            &trials[index].id,
            &BTreeMap::from([
                ("accuracy".into(), accuracy),
                ("audited_samples".into(), 5.),
                ("mean_latency_ms".into(), latency),
            ]),
            10,
            104,
        )
        .unwrap();
        assert_eq!(
            repo.get_experiment(&experiment.id)
                .unwrap()
                .best_trial_id
                .as_deref(),
            Some(trials[index].id.as_str())
        );
    }
}

#[test]
fn consultation_call_reservations_are_atomic_and_survive_restart() {
    let (directory, repo, _snapshot, request) = setup();
    let experiment = repo.create_experiment(request, 100).unwrap();
    let barrier = Arc::new(Barrier::new(3));
    let handles: Vec<_> = (0..2)
        .map(|_| {
            let repo = repo.clone();
            let id = experiment.id.clone();
            let barrier = barrier.clone();
            std::thread::spawn(move || {
                barrier.wait();
                repo.reserve_experiment_consultation(&id, 10, 3, 1, 101)
            })
        })
        .collect();
    barrier.wait();
    assert_eq!(
        handles
            .into_iter()
            .map(|thread| thread.join().unwrap())
            .filter(Result::is_ok)
            .count(),
        1
    );
    let reopened = TrainingRepository::open(directory.path().join("training.sqlite")).unwrap();
    assert!(
        reopened
            .reserve_experiment_consultation(&experiment.id, 10, 3, 1, 102)
            .is_err()
    );
    let events = reopened.experiment_events(&experiment.id).unwrap();
    assert_eq!(events.len(), 1);
    assert_eq!(events[0]["kind"], "consultation_reserved");
    let usage = reopened.get_experiment(&experiment.id).unwrap().usage;
    assert_eq!(usage.llm_tokens, 10);
    assert_eq!(usage.llm_cost_micros, 3);
    assert!(
        reopened
            .reserve_experiment_consultation(&experiment.id, 100, 1, 2, 102)
            .is_err()
    );
    assert_eq!(reopened.experiment_events(&experiment.id).unwrap().len(), 1);
}

#[test]
fn request_metadata_budget_rejects_oversized_candidates_without_partial_append() {
    let (_directory, repo, _snapshot, mut request) = setup();
    request.budget.maximum_dataset_bytes = 100_000;
    let candidate = TrainingRequest {
        engine: "fixture".into(),
        recipe: json!({"large":"x".repeat(100_000)}),
        compute: json!({}),
    };
    let mut oversized = request.clone();
    oversized.candidates = vec![candidate.clone()];
    assert!(repo.create_experiment(oversized, 100).is_err());
    assert!(repo.list_experiments(&request.stream).unwrap().is_empty());
    let experiment = repo.create_experiment(request, 100).unwrap();
    assert!(
        repo.append_experiment_candidates(&experiment.id, vec![candidate], 101)
            .is_err()
    );
    assert_eq!(
        repo.get_experiment(&experiment.id)
            .unwrap()
            .request
            .candidates
            .len(),
        2
    );
}

#[test]
fn validation_keeps_its_trial_slot_until_trusted_results_settle() {
    let (_directory, repo, _snapshot, request) = setup();
    let experiment = repo.create_experiment(request, 100).unwrap();
    let first = repo
        .submit_experiment_trial(&experiment.id, 0, "first", 101)
        .unwrap();
    trained(&repo, &first, 102);
    assert!(
        repo.submit_experiment_trial(&experiment.id, 1, "second", 104)
            .is_err()
    );
    assert!(
        repo.stop_experiment_budget(&experiment.id, "budget", 104)
            .is_err()
    );
    assert!(
        repo.finalize_experiment(&experiment.id, None, "completed", 104)
            .is_err()
    );
    validate(&repo, &first, 0.95, 105);
    let second = repo
        .submit_experiment_trial(&experiment.id, 1, "second", 106)
        .unwrap();
    trained(&repo, &second, 107);
    assert!(repo.select_experiment_winner(&experiment.id, 109).is_err());
    validate(&repo, &second, 0.9, 110);
    assert_eq!(
        repo.select_experiment_winner(&experiment.id, 111)
            .unwrap()
            .selected_trial_id,
        Some(first.id)
    );
}

#[test]
fn cancelled_final_audit_resumes_only_its_frozen_winner_and_evidence() {
    let (_directory, repo, snapshot, request) = setup();
    let experiment = repo.create_experiment(request, 100).unwrap();
    let trial = repo
        .submit_experiment_trial(&experiment.id, 0, "first", 101)
        .unwrap();
    let artifact = trained(&repo, &trial, 102);
    validate(&repo, &trial, 1., 104);
    let selected = repo.select_experiment_winner(&experiment.id, 105).unwrap();
    let sample = &snapshot.test[0];
    repo.record_prediction(&PredictionRecord {
        sample_id: sample.id.clone(),
        artifact_id: artifact.id.clone(),
        predicted_label: Some("good".into()),
        teacher_label: None,
        actual_label: Some("good".into()),
        actual_source: Some(LabelSource::Reviewed),
        latency_ms: 1.,
        failed: false,
        recorded_at_ms: 106,
        details: Value::Null,
    })
    .unwrap();
    repo.cancel_experiment(&experiment.id, 107).unwrap();
    assert!(matches!(
        repo.finalize_experiment(&experiment.id, None, "completed", 108),
        Err(Error::Cancelled)
    ));
    assert!(matches!(
        repo.select_experiment_winner(&experiment.id, 108),
        Err(Error::Cancelled)
    ));
    assert!(repo.resume_experiment(&experiment.id, 108).is_err());
    let resumed = repo
        .resume_frozen_experiment_audit(&experiment.id, 108)
        .unwrap();
    assert_eq!(resumed.status, ExperimentStatus::Evaluating);
    assert_eq!(resumed.selected_trial_id, selected.selected_trial_id);
    assert_eq!(resumed.selected_at_ms, selected.selected_at_ms);
    assert_eq!(repo.predictions(&artifact.id).unwrap().len(), 1);
    assert!(
        repo.append_experiment_candidates(&experiment.id, vec![], 109)
            .is_err()
    );
    assert!(
        repo.submit_experiment_trial(&experiment.id, 1, "another", 109)
            .is_err()
    );
}

#[test]
fn interrupted_attempts_consume_training_budget_before_requeue() {
    let (_directory, repo, _snapshot, mut request) = setup();
    request.budget.maximum_training_time_ms = 2000;
    let experiment = repo.create_experiment(request, 100).unwrap();
    let trial = repo
        .submit_experiment_trial(&experiment.id, 0, "candidate", 101)
        .unwrap();
    let resource = ResourceRequest {
        key: "cpu".into(),
        estimated_bytes: 10,
        budget_bytes: 100,
    };
    let first = repo
        .claim_job_with_resources(&trial.job_id, "crashed", 10, 102, Some(resource.clone()))
        .unwrap();
    assert_eq!(
        repo.reconcile_stopped_job(&trial.job_id, 111)
            .unwrap()
            .status,
        JobStatus::Running
    );
    assert_eq!(
        repo.reconcile_stopped_job(&trial.job_id, 113)
            .unwrap()
            .status,
        JobStatus::Interrupted
    );
    assert!(
        repo.finish_training(&first, b"stale", json!({}), 113)
            .is_err()
    );
    assert_eq!(
        repo.resume_stopped_job(&trial.job_id, 114).unwrap().status,
        JobStatus::Queued
    );
    let usage = repo.get_experiment(&experiment.id).unwrap().usage;
    assert_eq!(usage.training_time_ms, 1000);
    assert_eq!(usage.reserved_training_time_ms, 1000);
    repo.claim_job_with_resources(&trial.job_id, "crashed-again", 10, 115, Some(resource))
        .unwrap();
    repo.reconcile_stopped_job(&trial.job_id, 126).unwrap();
    assert!(
        matches!(repo.resume_stopped_job(&trial.job_id,127),Err(Error::Invalid(reason)) if reason.contains("budget"))
    );
    let experiment = repo.get_experiment(&experiment.id).unwrap();
    assert_eq!(experiment.status, ExperimentStatus::BudgetExhausted);
    assert_eq!(experiment.usage.training_time_ms, 2000);
    assert_eq!(experiment.usage.reserved_training_time_ms, 0);
    assert_eq!(experiment.usage.reserved_artifact_bytes, 0);
    assert_eq!(
        repo.get_job(&trial.job_id).unwrap().status,
        JobStatus::Failed
    );
    let trial = repo.get_experiment_trial(&trial.id).unwrap();
    assert_eq!(trial.status, ExperimentTrialStatus::Failed);
    assert_eq!(trial.training_time_ms, 2000);
    assert!(repo.resume_job(&trial.job_id, 128).is_err());
    let resources: usize = repo
        .connection()
        .unwrap()
        .query_row(
            "SELECT count(*) FROM resource_reservations WHERE job_id=?",
            [&trial.job_id],
            |row| row.get(0),
        )
        .unwrap();
    assert_eq!(resources, 0);
}

#[test]
fn cancellation_and_expired_reconciliation_cannot_requeue_a_cancelled_worker() {
    let (_directory, repo, _snapshot, request) = setup();
    let experiment = repo.create_experiment(request, 100).unwrap();
    let trial = repo
        .submit_experiment_trial(&experiment.id, 0, "candidate", 101)
        .unwrap();
    repo.claim_job(&trial.job_id, "stopped", 10, 102).unwrap();
    let barrier = Arc::new(Barrier::new(3));
    let mut threads = vec![];
    for cancel in [true, false] {
        let repo = repo.clone();
        let job_id = trial.job_id.clone();
        let barrier = barrier.clone();
        threads.push(std::thread::spawn(move || {
            barrier.wait();
            if cancel {
                repo.request_cancel(&job_id, 113)
            } else {
                repo.reconcile_stopped_job(&job_id, 113)
            }
        }));
    }
    barrier.wait();
    for thread in threads {
        thread.join().unwrap().unwrap();
    }
    assert_eq!(
        repo.get_job(&trial.job_id).unwrap().status,
        JobStatus::Cancelled
    );
    assert!(repo.resume_stopped_job(&trial.job_id, 114).is_err());
}

#[test]
fn duplicate_dispatch_and_llm_admission_are_atomic_across_connections() {
    let (_directory, repo, _snapshot, request) = setup();
    let experiment = repo.create_experiment(request, 100).unwrap();
    let barrier = Arc::new(Barrier::new(3));
    let mut threads = vec![];
    for _ in 0..2 {
        let repo = repo.clone();
        let experiment = experiment.clone();
        let barrier = barrier.clone();
        threads.push(std::thread::spawn(move || {
            barrier.wait();
            repo.submit_experiment_trial(&experiment.id, 0, "same", 101)
                .unwrap()
        }));
    }
    barrier.wait();
    let first = threads.remove(0).join().unwrap();
    let second = threads.remove(0).join().unwrap();
    assert_eq!(first.id, second.id);
    assert_eq!(
        repo.get_experiment(&experiment.id)
            .unwrap()
            .usage
            .submitted_trials,
        1
    );
    assert!(
        repo.submit_experiment_trial(&experiment.id, 0, "different", 102)
            .is_err()
    );
    assert!(
        repo.submit_experiment_trial(&experiment.id, 1, "parallel", 102)
            .is_err()
    );
    let barrier = Arc::new(Barrier::new(3));
    let mut threads = vec![];
    for _ in 0..2 {
        let repo = repo.clone();
        let experiment = experiment.clone();
        let barrier = barrier.clone();
        threads.push(std::thread::spawn(move || {
            barrier.wait();
            repo.consume_experiment_llm_budget(&experiment.id, 60, 60, 103)
        }));
    }
    barrier.wait();
    assert_eq!(
        threads
            .into_iter()
            .filter(|thread| thread.thread().id() != std::thread::current().id())
            .map(|thread| thread.join().unwrap().is_ok() as usize)
            .sum::<usize>(),
        1
    );
    assert_eq!(
        repo.get_experiment(&experiment.id)
            .unwrap()
            .usage
            .llm_tokens,
        60
    );
}

#[test]
fn derived_features_preserve_split_targets_and_provenance() {
    let (_directory, repo, snapshot, request) = setup();
    let experiment = repo.create_experiment(request, 100).unwrap();
    let mut derived = snapshot.clone();
    derived.id.clear();
    derived.digest.clear();
    for sample in derived
        .train
        .iter_mut()
        .chain(&mut derived.validation)
        .chain(&mut derived.test)
    {
        sample.payload["input"] = json!({"shape":[2],"values":[1.,2.]});
    }
    let dataset = repo
        .register_experiment_dataset(
            &experiment.id,
            derived.clone(),
            json!({"features":["x","x_squared"]}),
            101,
        )
        .unwrap();
    assert_ne!(dataset.digest, snapshot.digest);
    let mut moved = derived.clone();
    std::mem::swap(&mut moved.train[0], &mut moved.test[0]);
    assert!(
        repo.register_experiment_dataset(&experiment.id, moved, Value::Null, 102)
            .is_err()
    );
    let mut relabelled = derived.clone();
    relabelled.test[0].payload["label"] = json!("bad");
    assert!(
        repo.register_experiment_dataset(&experiment.id, relabelled, Value::Null, 102)
            .is_err()
    );
    let mut forged = derived;
    forged.test[0].source = LabelSource::Teacher;
    assert!(
        repo.register_experiment_dataset(&experiment.id, forged, Value::Null, 102)
            .is_err()
    );
    let candidate = TrainingRequest {
        engine: "fixture".into(),
        recipe: json!({"dataset_snapshot_id":dataset.snapshot_id}),
        compute: json!({}),
    };
    repo.append_experiment_candidates(&experiment.id, vec![candidate], 102)
        .unwrap();
    let trial = repo
        .submit_experiment_trial(&experiment.id, 2, "features", 103)
        .unwrap();
    let job = repo.get_job(&trial.job_id).unwrap();
    assert_eq!(
        job.request.recipe["inspection_spec"]["input_shape"],
        json!([2])
    );
}

#[test]
fn cancellation_prevents_new_work_and_resume_preserves_artifact() {
    let (_directory, repo, _snapshot, request) = setup();
    let experiment = repo.create_experiment(request, 100).unwrap();
    let trial = repo
        .submit_experiment_trial(&experiment.id, 0, "first", 101)
        .unwrap();
    let artifact = trained(&repo, &trial, 102);
    assert_eq!(
        repo.cancel_experiment(&experiment.id, 104).unwrap().status,
        ExperimentStatus::Cancelled
    );
    repo.fail_experiment_trial(&trial.id, "cancelled during validation", 10, 105)
        .unwrap();
    assert_eq!(
        repo.get_experiment_trial(&trial.id).unwrap().status,
        ExperimentTrialStatus::Cancelled
    );
    assert!(
        repo.submit_experiment_trial(&experiment.id, 1, "next", 105)
            .is_err()
    );
    repo.resume_experiment(&experiment.id, 106).unwrap();
    assert_eq!(
        repo.get_job(&trial.job_id).unwrap().status,
        JobStatus::Succeeded
    );
    let trial = validate(&repo, &trial, 1., 107);
    assert_eq!(trial.artifact_id.as_deref(), Some(artifact.id.as_str()));
    assert_eq!(
        repo.get_experiment(&experiment.id)
            .unwrap()
            .usage
            .artifact_bytes,
        7
    );
}

#[test]
fn artifact_and_training_budget_are_reserved_before_dispatch() {
    let (_directory, repo, _snapshot, mut request) = setup();
    request.budget.maximum_parallel_trials = 2;
    request.budget.maximum_artifact_bytes = 30;
    let experiment = repo.create_experiment(request, 100).unwrap();
    let first = repo
        .submit_experiment_trial(&experiment.id, 0, "first", 101)
        .unwrap();
    assert!(
        repo.submit_experiment_trial(&experiment.id, 1, "second", 102)
            .is_err()
    );
    let lease = repo.claim_job(&first.job_id, "worker", 1000, 102).unwrap();
    assert!(
        repo.finish_training(&lease, &[0; 31], json!({}), 103)
            .is_err()
    );
    repo.finish_training(&lease, b"weights", json!({}), 103)
        .unwrap();
    validate(&repo, &first, 1., 104);
    assert!(
        repo.submit_experiment_trial(&experiment.id, 1, "second", 105)
            .is_err()
    );
    assert_eq!(
        repo.get_experiment(&experiment.id)
            .unwrap()
            .usage
            .submitted_trials,
        1
    );
    assert!(
        repo.consume_experiment_llm_budget(&experiment.id, 1, 1, 20000)
            .is_err()
    );
}

#[test]
fn winner_is_frozen_before_final_evidence_and_goal_failure_still_returns_best() {
    let (_directory, repo, snapshot, request) = setup();
    let experiment = repo.create_experiment(request, 100).unwrap();
    let trial = repo
        .submit_experiment_trial(&experiment.id, 0, "first", 101)
        .unwrap();
    let artifact = trained(&repo, &trial, 102);
    validate(&repo, &trial, 0.8, 104);
    repo.select_experiment_winner(&experiment.id, 105).unwrap();
    assert!(
        repo.append_experiment_candidates(&experiment.id, vec![], 106)
            .is_err()
    );
    assert!(
        repo.submit_experiment_trial(&experiment.id, 1, "too_late", 106)
            .is_err()
    );
    for sample in &snapshot.test {
        repo.record_prediction(&PredictionRecord {
            sample_id: sample.id.clone(),
            artifact_id: artifact.id.clone(),
            predicted_label: Some("bad".into()),
            teacher_label: None,
            actual_label: Some("good".into()),
            actual_source: Some(LabelSource::Reviewed),
            latency_ms: 1.,
            failed: false,
            recorded_at_ms: 106,
            details: Value::Null,
        })
        .unwrap();
    }
    let report = repo.evaluate_classification(&artifact.id, 107).unwrap();
    let finished = repo
        .finalize_experiment(&experiment.id, Some(&report.id), "completed", 108)
        .unwrap();
    assert_eq!(finished.status, ExperimentStatus::Completed);
    assert!(!finished.target_met);
    assert!(!finished.unmet_constraints.is_empty());
    let result = repo.experiment_result(&experiment.id).unwrap();
    assert_eq!(result.best_artifact.unwrap().id, artifact.id);
    assert_eq!(result.final_evaluation.unwrap().id, report.id);
    assert!(repo.resume_experiment(&experiment.id, 109).is_err());
}

#[test]
fn prepared_import_rejects_forged_labels_and_temporal_leakage() {
    let (_directory, repo, mut snapshot, _request) = setup();
    snapshot.id.clear();
    snapshot.digest = "caller_digest_is_ignored".into();
    let imported = repo.import_experiment_snapshot(snapshot.clone()).unwrap();
    assert_eq!(imported.digest.len(), 64);
    snapshot.test[0].payload["label"] = json!("forged");
    assert!(repo.import_experiment_snapshot(snapshot.clone()).is_err());
    snapshot.test[0].payload["label"] = json!("good");
    snapshot.policy = SplitPolicy::Time {
        train_end_ms: 5,
        validation_end_ms: 10,
        embargo_ms: 1,
    };
    assert!(repo.import_experiment_snapshot(snapshot).is_err());
}

#[test]
fn shadow_cannot_mutate_experiment_state() {
    let (_directory, repo, _snapshot, request) = setup();
    let experiment = repo.create_experiment(request.clone(), 100).unwrap();
    let shadow = repo.shadow_telemetry();
    assert!(matches!(
        shadow.create_experiment(request, 100),
        Err(Error::ShadowWrite)
    ));
    assert!(matches!(
        shadow.submit_experiment_trial(&experiment.id, 0, "first", 101),
        Err(Error::ShadowWrite)
    ));
    assert!(matches!(
        shadow.consume_experiment_llm_budget(&experiment.id, 1, 1, 101),
        Err(Error::ShadowWrite)
    ));
    assert!(shadow.get_experiment(&experiment.id).is_ok());
}

#[test]
fn successive_halving_continuation_is_atomic_and_preserves_optimizer_checkpoint() {
    let (_directory, repo, _snapshot, mut request) = setup();
    request.candidates[0] = TrainingRequest {
        engine: "burn".into(),
        recipe: json!({"config":{"epochs":2},"search":{"family":"mlp-small","rung":0,"max_epochs":9,"reduction_factor":3}}),
        compute: json!({}),
    };
    let experiment = repo.create_experiment(request, 100).unwrap();
    let parent = repo
        .submit_experiment_trial(&experiment.id, 0, "initial", 101)
        .unwrap();
    let lease = repo.claim_job(&parent.job_id, "worker", 1000, 102).unwrap();
    let checkpoint = repo
        .publish_checkpoint(
            &lease,
            b"optimizer-state",
            4,
            json!({"epoch":2}),
            false,
            103,
        )
        .unwrap();
    repo.finish_training(&lease, b"weights", json!({}), 104)
        .unwrap();
    validate(&repo, &parent, 0.95, 105);
    assert!(repo.continue_experiment_trial(&parent.id, 2, 106).is_err());
    assert!(repo.continue_experiment_trial(&parent.id, 10, 106).is_err());
    let child = repo.continue_experiment_trial(&parent.id, 6, 106).unwrap();
    assert_eq!(
        repo.continue_experiment_trial(&parent.id, 6, 107)
            .unwrap()
            .id,
        child.id
    );
    assert_eq!(child.dataset_snapshot_id, parent.dataset_snapshot_id);
    assert_eq!(child.snapshot_id, parent.snapshot_id);
    let job = repo.get_job(&child.job_id).unwrap();
    assert_eq!(job.request.recipe["config"]["epochs"], json!(6));
    assert_eq!(job.request.recipe["search"]["family"], json!("mlp-small"));
    assert_eq!(job.request.recipe["search"]["rung"], json!(1));
    let copied = job.checkpoint.unwrap();
    assert_eq!(copied.blob, checkpoint.blob);
    assert_eq!(
        copied.request_digest,
        digest(&serde_json::to_vec(&job.request).unwrap())
    );
    assert_ne!(copied.request_digest, checkpoint.request_digest);
    assert_eq!(
        repo.get_experiment(&experiment.id)
            .unwrap()
            .usage
            .submitted_trials,
        2
    );
}

#[test]
fn anomaly_training_filters_defects_but_search_retains_validation_classes() {
    let (_directory, repo, mut snapshot, mut request) = setup();
    for partition in [
        &mut snapshot.train,
        &mut snapshot.validation,
        &mut snapshot.test,
    ] {
        for (index, sample) in partition.iter_mut().enumerate() {
            sample.annotation_revision = 2;
            sample.payload["annotation"] = json!({"kind":"anomaly","is_anomaly":index%2==1});
            repo.record_sample(&snapshot.stream, sample).unwrap();
        }
    }
    snapshot.id.clear();
    snapshot.digest.clear();
    let snapshot = repo.import_experiment_snapshot(snapshot).unwrap();
    request.snapshot_id = snapshot.id.clone();
    request.spec["task"] = json!("sensor_anomaly");
    let experiment = repo.create_experiment(request, 100).unwrap();
    let trial = repo
        .submit_experiment_trial(&experiment.id, 0, "anomaly", 101)
        .unwrap();
    let training = repo.get_snapshot(&trial.snapshot_id).unwrap();
    assert!(
        training
            .train
            .iter()
            .chain(&training.validation)
            .all(|sample| sample.payload["annotation"]["is_anomaly"] == json!(false))
    );
    let search = repo
        .get_experiment_search_snapshot(&trial.id, 1_000_000)
        .unwrap();
    assert_eq!(search.validation, snapshot.validation);
    assert!(search.test.is_empty());
    assert!(repo.get_experiment_search_snapshot(&trial.id, 1).is_err());
}

fn interrupt(repo: &TrainingRepository, job_id: &str) {
    let mut job = repo.get_job(job_id).unwrap();
    job.status = JobStatus::Interrupted;
    job.lease_owner = None;
    job.lease_expires_at_ms = None;
    update_job(&repo.connection().unwrap(), &job).unwrap();
}

fn reservations(repo: &TrainingRepository, job_id: &str) -> usize {
    repo.connection()
        .unwrap()
        .query_row(
            "SELECT count(*) FROM resource_reservations WHERE job_id=?",
            [job_id],
            |row| row.get(0),
        )
        .unwrap()
}

#[test]
fn failing_an_interrupted_trial_settles_its_job_and_charges_its_slice() {
    let (_directory, repo, _snapshot, request) = setup();
    let experiment = repo.create_experiment(request, 100).unwrap();
    let trial = repo
        .submit_experiment_trial(&experiment.id, 0, "candidate", 101)
        .unwrap();
    let resource = ResourceRequest {
        key: "cpu".into(),
        estimated_bytes: 10,
        budget_bytes: 100,
    };
    repo.claim_job_with_resources(&trial.job_id, "crashed", 10, 102, Some(resource))
        .unwrap();
    interrupt(&repo, &trial.job_id);
    assert_eq!(reservations(&repo, &trial.job_id), 1);
    let failed = repo
        .fail_experiment_trial(&trial.id, "experiment wall time budget exhausted", 0, 114)
        .unwrap();
    assert_eq!(failed.status, ExperimentTrialStatus::Failed);
    assert_eq!(failed.training_time_ms, 1000);
    let job = repo.get_job(&trial.job_id).unwrap();
    assert_eq!(job.status, JobStatus::Failed);
    assert_eq!(
        job.error.as_deref(),
        Some("experiment wall time budget exhausted")
    );
    assert_eq!(reservations(&repo, &trial.job_id), 0);
    let usage = repo.get_experiment(&experiment.id).unwrap().usage;
    assert_eq!(usage.training_time_ms, 1000);
    assert_eq!(usage.reserved_training_time_ms, 0);
    assert_eq!(usage.reserved_artifact_bytes, 0);
    let stopped = repo
        .stop_experiment_budget(&experiment.id, "wall_time_budget_exhausted", 115)
        .unwrap();
    assert_eq!(stopped.status, ExperimentStatus::BudgetExhausted);
}

#[test]
fn failing_an_interrupted_trial_while_cancelling_completes_the_cancellation() {
    let (_directory, repo, _snapshot, request) = setup();
    let experiment = repo.create_experiment(request, 100).unwrap();
    let trial = repo
        .submit_experiment_trial(&experiment.id, 0, "candidate", 101)
        .unwrap();
    repo.claim_job(&trial.job_id, "crashed", 10, 102).unwrap();
    assert_eq!(
        repo.cancel_experiment(&experiment.id, 103).unwrap().status,
        ExperimentStatus::CancelRequested
    );
    interrupt(&repo, &trial.job_id);
    let cancelled = repo
        .fail_experiment_trial(&trial.id, "cancelled", 0, 114)
        .unwrap();
    assert_eq!(cancelled.status, ExperimentTrialStatus::Cancelled);
    assert_eq!(
        repo.get_job(&trial.job_id).unwrap().status,
        JobStatus::Cancelled
    );
    let value = repo.get_experiment(&experiment.id).unwrap();
    assert_eq!(value.status, ExperimentStatus::Cancelled);
    assert_eq!(value.usage.training_time_ms, 1000);
    assert_eq!(value.usage.reserved_training_time_ms, 0);
}

#[test]
fn cancelling_charges_stopped_attempts_and_refunds_queued_ones() {
    let (_directory, repo, _snapshot, mut request) = setup();
    request.budget.maximum_parallel_trials = 2;
    let experiment = repo.create_experiment(request, 100).unwrap();
    let attempted = repo
        .submit_experiment_trial(&experiment.id, 0, "attempted", 101)
        .unwrap();
    let queued = repo
        .submit_experiment_trial(&experiment.id, 1, "queued", 101)
        .unwrap();
    repo.claim_job(&attempted.job_id, "crashed", 10, 102)
        .unwrap();
    assert_eq!(
        repo.reconcile_stopped_job(&attempted.job_id, 113)
            .unwrap()
            .status,
        JobStatus::Interrupted
    );
    let cancelled = repo.cancel_experiment(&experiment.id, 114).unwrap();
    assert_eq!(cancelled.status, ExperimentStatus::Cancelled);
    assert_eq!(cancelled.usage.training_time_ms, 1000);
    assert_eq!(cancelled.usage.reserved_training_time_ms, 0);
    assert_eq!(cancelled.usage.reserved_artifact_bytes, 0);
    assert_eq!(
        repo.get_experiment_trial(&attempted.id)
            .unwrap()
            .training_time_ms,
        1000
    );
    assert_eq!(
        repo.get_experiment_trial(&queued.id)
            .unwrap()
            .training_time_ms,
        0
    );
    let resumed = repo.resume_experiment(&experiment.id, 115).unwrap();
    assert_eq!(resumed.status, ExperimentStatus::Running);
    assert_eq!(resumed.usage.training_time_ms, 1000);
    assert_eq!(resumed.usage.reserved_training_time_ms, 2000);
}

#[test]
fn resume_settles_trials_the_remaining_budget_cannot_cover() {
    let (_directory, repo, _snapshot, mut request) = setup();
    request.budget.maximum_training_time_ms = 1500;
    let experiment = repo.create_experiment(request, 100).unwrap();
    let trial = repo
        .submit_experiment_trial(&experiment.id, 0, "candidate", 101)
        .unwrap();
    repo.claim_job(&trial.job_id, "crashed", 10, 102).unwrap();
    repo.reconcile_stopped_job(&trial.job_id, 113).unwrap();
    repo.cancel_experiment(&experiment.id, 114).unwrap();
    let settled = repo.resume_experiment(&experiment.id, 115).unwrap();
    assert_eq!(settled.status, ExperimentStatus::BudgetExhausted);
    assert_eq!(
        settled.stop_reason.as_deref(),
        Some("resource_budget_exhausted")
    );
    assert_eq!(settled.usage.reserved_training_time_ms, 0);
    assert_eq!(settled.usage.reserved_artifact_bytes, 0);
    let failed = repo.get_experiment_trial(&trial.id).unwrap();
    assert_eq!(failed.status, ExperimentTrialStatus::Failed);
    assert_eq!(failed.error.as_deref(), Some("budget exhausted on resume"));
    assert_eq!(
        repo.get_job(&trial.job_id).unwrap().status,
        JobStatus::Cancelled
    );
    let finished = repo
        .finalize_experiment(
            &experiment.id,
            None,
            "no_candidate_met_validation_requirements",
            116,
        )
        .unwrap();
    assert_eq!(finished.status, ExperimentStatus::BudgetExhausted);
    assert_eq!(
        finished.stop_reason.as_deref(),
        Some("resource_budget_exhausted")
    );
}

#[test]
fn resume_past_the_wall_budget_settles_instead_of_failing() {
    let (_directory, repo, _snapshot, request) = setup();
    let experiment = repo.create_experiment(request, 100).unwrap();
    let trial = repo
        .submit_experiment_trial(&experiment.id, 0, "candidate", 101)
        .unwrap();
    repo.cancel_experiment(&experiment.id, 10_200).unwrap();
    let settled = repo.resume_experiment(&experiment.id, 10_300).unwrap();
    assert_eq!(settled.status, ExperimentStatus::BudgetExhausted);
    assert_eq!(
        settled.stop_reason.as_deref(),
        Some("wall_time_budget_exhausted")
    );
    assert_eq!(
        repo.get_experiment_trial(&trial.id).unwrap().status,
        ExperimentTrialStatus::Failed
    );
    assert_eq!(
        repo.get_job(&trial.job_id).unwrap().status,
        JobStatus::Cancelled
    );
    let finished = repo
        .finalize_experiment(
            &experiment.id,
            None,
            "no_candidate_met_validation_requirements",
            10_400,
        )
        .unwrap();
    assert_eq!(finished.status, ExperimentStatus::BudgetExhausted);
    assert_eq!(
        finished.stop_reason.as_deref(),
        Some("wall_time_budget_exhausted")
    );
}

#[test]
fn frozen_audit_past_the_wall_budget_settles_for_finalization() {
    let (_directory, repo, _snapshot, request) = setup();
    let experiment = repo.create_experiment(request, 100).unwrap();
    let trial = repo
        .submit_experiment_trial(&experiment.id, 0, "first", 101)
        .unwrap();
    trained(&repo, &trial, 102);
    validate(&repo, &trial, 1., 104);
    let selected = repo.select_experiment_winner(&experiment.id, 105).unwrap();
    repo.cancel_experiment(&experiment.id, 10_200).unwrap();
    let settled = repo
        .resume_frozen_experiment_audit(&experiment.id, 10_300)
        .unwrap();
    assert_eq!(settled.status, ExperimentStatus::BudgetExhausted);
    let finished = repo
        .finalize_experiment(&experiment.id, None, "wall_time_budget_exhausted", 10_400)
        .unwrap();
    assert_eq!(finished.status, ExperimentStatus::BudgetExhausted);
    assert_eq!(finished.selected_trial_id, selected.selected_trial_id);
    assert!(
        finished
            .unmet_constraints
            .contains(&"final independent evaluation is unavailable".to_string())
    );
}

#[test]
fn wall_clock_charges_only_active_time() {
    let (_directory, repo, _snapshot, request) = setup();
    let experiment = repo.create_experiment(request, 100).unwrap();
    assert_eq!(experiment.clock, Some(ExperimentClock::default()));
    assert_eq!(experiment.wall_elapsed_ms(50_000), 0);
    repo.consume_experiment_llm_budget(&experiment.id, 1, 1, 50_000)
        .unwrap();
    let active = repo.mark_experiment_active(&experiment.id, 50_000).unwrap();
    assert_eq!(active.clock.as_ref().unwrap().active_since_ms, Some(50_000));
    assert_eq!(
        repo.mark_experiment_active(&experiment.id, 52_000)
            .unwrap()
            .clock,
        active.clock
    );
    let cancelled = repo.cancel_experiment(&experiment.id, 55_000).unwrap();
    assert_eq!(
        cancelled.clock,
        Some(ExperimentClock {
            active_since_ms: None,
            elapsed_ms: 5_000,
        })
    );
    let resumed = repo.resume_experiment(&experiment.id, 90_000).unwrap();
    assert_eq!(resumed.status, ExperimentStatus::Running);
    assert_eq!(resumed.wall_elapsed_ms(95_000), 5_000);
    repo.mark_experiment_active(&experiment.id, 95_000).unwrap();
    repo.consume_experiment_llm_budget(&experiment.id, 1, 1, 100_000)
        .unwrap();
    assert!(matches!(
        repo.consume_experiment_llm_budget(&experiment.id, 1, 1, 100_001),
        Err(Error::Invalid(reason)) if reason.contains("wall time budget exhausted")
    ));
}

#[test]
fn final_audit_after_a_budget_stop_restarts_the_wall_clock() {
    let (_directory, repo, _snapshot, request) = setup();
    let experiment = repo.create_experiment(request, 100).unwrap();
    let trial = repo
        .submit_experiment_trial(&experiment.id, 0, "first", 101)
        .unwrap();
    trained(&repo, &trial, 102);
    validate(&repo, &trial, 1., 104);
    let stopped = repo
        .stop_experiment_budget(&experiment.id, "trial_budget_exhausted", 1_101)
        .unwrap();
    assert_eq!(
        stopped.clock,
        Some(ExperimentClock {
            active_since_ms: None,
            elapsed_ms: 1_000,
        })
    );
    let selected = repo
        .select_experiment_winner(&experiment.id, 50_000)
        .unwrap();
    assert_eq!(
        selected.clock.as_ref().unwrap().active_since_ms,
        Some(50_000)
    );
    assert_eq!(selected.wall_elapsed_ms(52_000), 3_000);
    let finished = repo
        .finalize_experiment(&experiment.id, None, "wall_time_budget_exhausted", 53_000)
        .unwrap();
    assert_eq!(
        finished.clock,
        Some(ExperimentClock {
            active_since_ms: None,
            elapsed_ms: 4_000,
        })
    );
}

#[test]
fn records_without_a_clock_count_from_creation_until_paused() {
    let (_directory, repo, _snapshot, request) = setup();
    let experiment = repo.create_experiment(request, 100).unwrap();
    repo.connection()
        .unwrap()
        .execute(
            "UPDATE experiments SET body=json_remove(body,'$.clock') WHERE id=?",
            [&experiment.id],
        )
        .unwrap();
    let legacy = repo.get_experiment(&experiment.id).unwrap();
    assert!(legacy.clock.is_none());
    assert_eq!(legacy.wall_elapsed_ms(600), 500);
    assert_eq!(legacy.wall_remaining_ms(600), 9_500);
    assert!(
        repo.mark_experiment_active(&experiment.id, 600)
            .unwrap()
            .clock
            .is_none()
    );
    let cancelled = repo.cancel_experiment(&experiment.id, 700).unwrap();
    assert_eq!(
        cancelled.clock,
        Some(ExperimentClock {
            active_since_ms: None,
            elapsed_ms: 600,
        })
    );
}

#[test]
fn resumed_jobs_get_the_deadline_of_the_restarted_clock() {
    let (_directory, repo, _snapshot, request) = setup();
    let experiment = repo.create_experiment(request, 100).unwrap();
    repo.mark_experiment_active(&experiment.id, 1_000).unwrap();
    let trial = repo
        .submit_experiment_trial(&experiment.id, 0, "candidate", 2_000)
        .unwrap();
    // 9_000 ms remain at dispatch; the final audit keeps a tenth of the 10_000 ms budget.
    assert_eq!(
        repo.get_job(&trial.job_id).unwrap().request.recipe["experiment_deadline_ms"],
        json!(10_000)
    );
    let lease = repo
        .claim_job(&trial.job_id, "worker", 10_000, 2_001)
        .unwrap();
    repo.publish_checkpoint(
        &lease,
        b"optimizer-state",
        1,
        json!({"epoch":1}),
        false,
        2_002,
    )
    .unwrap();
    repo.cancel_experiment(&experiment.id, 3_000).unwrap();
    repo.acknowledge_cancel(&lease, 3_001).unwrap();
    repo.fail_experiment_trial(&trial.id, "cancelled", 1_000, 3_002)
        .unwrap();
    assert_eq!(
        repo.get_experiment(&experiment.id).unwrap().status,
        ExperimentStatus::Cancelled
    );
    repo.resume_experiment(&experiment.id, 50_000).unwrap();
    let job = repo.get_job(&trial.job_id).unwrap();
    assert_eq!(job.status, JobStatus::Queued);
    assert_eq!(job.request.recipe["experiment_deadline_ms"], json!(57_000));
    assert_eq!(
        job.checkpoint.unwrap().request_digest,
        digest(&serde_json::to_vec(&job.request).unwrap())
    );
    repo.mark_experiment_active(&experiment.id, 70_000).unwrap();
    let job = repo.get_job(&trial.job_id).unwrap();
    assert_eq!(job.request.recipe["experiment_deadline_ms"], json!(77_000));
    assert_eq!(
        job.checkpoint.unwrap().request_digest,
        digest(&serde_json::to_vec(&job.request).unwrap())
    );
}

#[test]
fn deadline_refresh_never_adopts_a_foreign_checkpoint() {
    let (_directory, repo, _snapshot, request) = setup();
    let experiment = repo.create_experiment(request, 100).unwrap();
    let trial = repo
        .submit_experiment_trial(&experiment.id, 0, "candidate", 101)
        .unwrap();
    let mut job = repo.get_job(&trial.job_id).unwrap();
    job.checkpoint = Some(CheckpointRef {
        blob: BlobRef {
            sha256: "0".repeat(64),
            bytes: 1,
            path: "foreign".into(),
        },
        step: 1,
        metadata: json!({}),
        request_digest: "foreign".into(),
    });
    assert!(refresh_job_deadline(&mut job, 42).unwrap());
    assert_eq!(job.request.recipe["experiment_deadline_ms"], json!(42));
    assert_eq!(job.checkpoint.unwrap().request_digest, "foreign");
}

#[test]
fn writes_ordered_after_a_later_commit_neither_fail_nor_regress() {
    let (_directory, repo, _snapshot, request) = setup();
    let experiment = repo.create_experiment(request, 100).unwrap();
    repo.reserve_experiment_consultation(&experiment.id, 10, 3, 4, 5_000)
        .unwrap();
    let trial = repo
        .submit_experiment_trial(&experiment.id, 0, "late-clock", 4_000)
        .unwrap();
    assert_eq!(trial.created_at_ms, 5_000);
    let cancelled = repo.cancel_experiment(&experiment.id, 4_500).unwrap();
    assert_eq!(cancelled.status, ExperimentStatus::Cancelled);
    assert_eq!(cancelled.updated_at_ms, 5_000);
    let running = Experiment {
        status: ExperimentStatus::Running,
        ..cancelled
    };
    assert!(matches!(
        ensure_running(&running, 4_999),
        Err(Error::Conflict(reason)) if reason == "experiment clock moved backwards"
    ));
}

#[test]
fn consultation_settlement_replaces_each_reservation_once() {
    let (_directory, repo, _snapshot, request) = setup();
    let experiment = repo.create_experiment(request, 100).unwrap();
    repo.reserve_experiment_consultation(&experiment.id, 60, 30, 4, 101)
        .unwrap();
    let refunded = repo
        .settle_experiment_consultation(&experiment.id, 60, 30, 10, 5, 102)
        .unwrap();
    assert_eq!(
        (refunded.usage.llm_tokens, refunded.usage.llm_cost_micros),
        (10, 5)
    );
    assert!(matches!(
        repo.settle_experiment_consultation(&experiment.id, 60, 30, 10, 5, 103),
        Err(Error::Conflict(_))
    ));
    assert!(matches!(
        repo.settle_experiment_consultation(&experiment.id, 7, 3, 1, 1, 103),
        Err(Error::Conflict(_))
    ));
    repo.reserve_experiment_consultation(&experiment.id, 60, 30, 4, 104)
        .unwrap();
    repo.stop_experiment_budget(&experiment.id, "trial_budget_exhausted", 105)
        .unwrap();
    let charged = repo
        .settle_experiment_consultation(&experiment.id, 60, 30, 95, 40, 106)
        .unwrap();
    assert_eq!(charged.status, ExperimentStatus::BudgetExhausted);
    assert_eq!(
        (charged.usage.llm_tokens, charged.usage.llm_cost_micros),
        (100, 45)
    );
    let kinds: Vec<String> = repo
        .experiment_events(&experiment.id)
        .unwrap()
        .iter()
        .map(|event| event["kind"].as_str().unwrap().to_owned())
        .collect();
    assert_eq!(
        kinds,
        [
            "consultation_reserved",
            "consultation_settled",
            "consultation_reserved",
            "consultation_settled"
        ]
    );
}

#[test]
fn derived_candidates_register_atomically_with_their_dataset() {
    let (_directory, repo, snapshot, request) = setup();
    let experiment = repo.create_experiment(request, 100).unwrap();
    let mut derived = snapshot.clone();
    derived.id = "derived-features".into();
    derived.digest.clear();
    for sample in derived
        .train
        .iter_mut()
        .chain(&mut derived.validation)
        .chain(&mut derived.test)
    {
        sample.payload["input"] = json!({"shape":[2],"values":[1.,2.]});
    }
    let candidate = TrainingRequest {
        engine: "fixture".into(),
        recipe: json!({"dataset_snapshot_id":"derived-features"}),
        compute: json!({}),
    };
    let bytes = experiment.usage.dataset_bytes;
    assert!(
        repo.register_experiment_candidates(
            &experiment.id,
            derived.clone(),
            Value::Null,
            vec![candidate.clone(); 3],
            101,
        )
        .is_err()
    );
    let unregistered = TrainingRequest {
        recipe: json!({"dataset_snapshot_id":"elsewhere"}),
        ..candidate.clone()
    };
    assert!(
        repo.register_experiment_candidates(
            &experiment.id,
            derived.clone(),
            Value::Null,
            vec![candidate.clone(), unregistered],
            101,
        )
        .is_err()
    );
    let mut unnamed = derived.clone();
    unnamed.id.clear();
    assert!(
        repo.register_experiment_candidates(
            &experiment.id,
            unnamed,
            Value::Null,
            vec![candidate.clone()],
            101,
        )
        .is_err()
    );
    let unchanged = repo.experiment_result(&experiment.id).unwrap();
    assert_eq!(unchanged.datasets.len(), 1);
    assert_eq!(unchanged.experiment.usage.dataset_bytes, bytes);
    assert_eq!(unchanged.experiment.request.candidates.len(), 2);
    assert!(repo.get_snapshot("derived-features").is_err());
    let (dataset, updated) = repo
        .register_experiment_candidates(
            &experiment.id,
            derived,
            json!({"features":["x","x_squared"]}),
            vec![candidate],
            101,
        )
        .unwrap();
    assert_eq!(dataset.snapshot_id, "derived-features");
    assert_eq!(updated.request.candidates.len(), 3);
    assert!(updated.usage.dataset_bytes > bytes);
    let trial = repo
        .submit_experiment_trial(&experiment.id, 2, "features", 102)
        .unwrap();
    assert_eq!(trial.dataset_snapshot_id, "derived-features");
}

#[test]
fn prepared_samples_follow_raw_row_edits_but_keep_targets_fixed() {
    let (_directory, repo, snapshot, _request) = setup();
    let stream = snapshot.stream.clone();
    let scope = serde_json::to_string(&stream).unwrap();
    let mut sample = snapshot.train[0].clone();
    sample.payload["feature_row"] = json!({"x":1});
    assert!(
        !repo
            .record_prepared_sample(&stream, &sample)
            .unwrap()
            .inserted
    );
    sample.payload["feature_row"] = json!({"x":2,"added_column":"note"});
    sample.payload["input"] = json!({"shape":[1],"values":[2.]});
    let receipt = repo.record_prepared_sample(&stream, &sample).unwrap();
    assert!(!receipt.inserted);
    assert!(receipt.accepted_sequence.is_some());
    let stored = recorded_annotation(&repo.connection().unwrap(), &scope, &sample)
        .unwrap()
        .unwrap();
    assert_eq!(stored, sample);
    let mut relabelled = sample.clone();
    relabelled.payload["label"] = json!("bad");
    assert!(matches!(
        repo.record_prepared_sample(&stream, &relabelled),
        Err(Error::Conflict(_))
    ));
    let mut rejected = sample.clone();
    rejected.accepted = false;
    assert!(repo.record_prepared_sample(&stream, &rejected).is_err());
    let mut edited = snapshot;
    edited.id.clear();
    edited.train[0] = sample;
    assert!(repo.import_experiment_snapshot(edited).is_ok());
}

#[test]
fn snapshot_import_with_creation_leaves_nothing_behind_on_failure() {
    let (_directory, repo, mut snapshot, mut request) = setup();
    snapshot.id.clear();
    snapshot.digest.clear();
    let count = || -> usize {
        repo.connection()
            .unwrap()
            .query_row("SELECT count(*) FROM snapshots", [], |row| row.get(0))
            .unwrap()
    };
    let before = count();
    let mut small = request.clone();
    small.budget.maximum_dataset_bytes = 1024;
    assert!(
        repo.create_experiment_with_snapshot(snapshot.clone(), small, 100)
            .is_err()
    );
    let mut unregistered = request.clone();
    unregistered.candidates[0].recipe = json!({"dataset_snapshot_id":"missing"});
    assert!(
        repo.create_experiment_with_snapshot(snapshot.clone(), unregistered, 100)
            .is_err()
    );
    assert_eq!(count(), before);
    request.snapshot_id = "replaced".into();
    let experiment = repo
        .create_experiment_with_snapshot(snapshot, request, 100)
        .unwrap();
    assert_ne!(experiment.request.snapshot_id, "replaced");
    assert_eq!(
        repo.experiment_result(&experiment.id).unwrap().datasets[0].snapshot_id,
        experiment.request.snapshot_id
    );
    assert_eq!(count(), before + 2);
}

fn frozen_winner(
    repo: &TrainingRepository,
    request: ExperimentRequest,
) -> (Experiment, ModelArtifact) {
    let experiment = repo.create_experiment(request, 100).unwrap();
    let trial = repo
        .submit_experiment_trial(&experiment.id, 0, "first", 101)
        .unwrap();
    let artifact = trained(repo, &trial, 102);
    validate(repo, &trial, 1., 104);
    let selected = repo.select_experiment_winner(&experiment.id, 105).unwrap();
    (selected, artifact)
}

fn reviewed_prediction(artifact: &ModelArtifact, sample: &TrainingSample) -> PredictionRecord {
    PredictionRecord {
        sample_id: sample.id.clone(),
        artifact_id: artifact.id.clone(),
        predicted_label: Some("good".into()),
        teacher_label: None,
        actual_label: Some("good".into()),
        actual_source: Some(LabelSource::Reviewed),
        latency_ms: 1.,
        failed: false,
        recorded_at_ms: 106,
        details: Value::Null,
    }
}

#[test]
fn shadow_predictions_outside_the_frozen_holdout_do_not_void_the_final_audit() {
    let (_directory, repo, snapshot, request) = setup();
    let (experiment, artifact) = frozen_winner(&repo, request);
    for sample in &snapshot.test {
        repo.record_prediction(&reviewed_prediction(&artifact, sample))
            .unwrap();
    }
    let shadow = PredictionRecord {
        actual_label: None,
        actual_source: None,
        ..reviewed_prediction(&artifact, &snapshot.train[0])
    };
    repo.record_prediction(&shadow).unwrap();
    let holdout: BTreeSet<String> = snapshot
        .test
        .iter()
        .map(|sample| sample.id.clone())
        .collect();
    let report = repo
        .evaluate_classification_cohort(&artifact.id, Some(&holdout), 107)
        .unwrap();
    let finished = repo
        .finalize_experiment(&experiment.id, Some(&report.id), "completed", 108)
        .unwrap();
    assert_eq!(finished.status, ExperimentStatus::Completed);
    assert_eq!(
        finished.final_evaluation_id.as_deref(),
        Some(report.id.as_str())
    );
}

#[test]
fn final_evidence_must_still_cover_the_whole_frozen_holdout() {
    let (_directory, repo, snapshot, request) = setup();
    assert!(snapshot.test.len() > 1);
    let (experiment, artifact) = frozen_winner(&repo, request);
    for sample in &snapshot.test[1..] {
        repo.record_prediction(&reviewed_prediction(&artifact, sample))
            .unwrap();
    }
    let report = repo.evaluate_classification(&artifact.id, 107).unwrap();
    assert!(matches!(
        repo.finalize_experiment(&experiment.id, Some(&report.id), "completed", 108),
        Err(Error::Invalid(reason)) if reason.contains("frozen holdout")
    ));
}

#[test]
fn trial_deadlines_leave_the_audit_allowance_to_the_frozen_winner() {
    let (_directory, repo, _snapshot, request) = setup();
    let deadline = |experiment: &Experiment, at_ms: i64| {
        let trial = repo
            .submit_experiment_trial(&experiment.id, 0, "candidate", at_ms)
            .unwrap();
        let job = repo.get_job(&trial.job_id).unwrap();
        job.request.recipe["experiment_deadline_ms"].clone()
    };
    let mut long = request.clone();
    long.budget.maximum_wall_time_ms = 10_000_000;
    let long = repo.create_experiment(long, 100).unwrap();
    assert_eq!(long.audit_allowance_ms(), 300_000);
    assert_eq!(deadline(&long, 1_000), json!(9_701_000));
    let short = repo.create_experiment(request, 100).unwrap();
    assert_eq!(short.audit_allowance_ms(), 1_000);
    repo.mark_experiment_active(&short.id, 100).unwrap();
    // Only 500 ms remain, all of them reserved for the final audit.
    assert_eq!(deadline(&short, 9_600), json!(9_600));
}

#[test]
fn settled_trials_charge_their_retained_checkpoint_until_a_winner_is_selected() {
    let (_directory, repo, _snapshot, mut request) = setup();
    request.budget.maximum_artifact_bytes = 60;
    let experiment = repo.create_experiment(request, 100).unwrap();
    let trial = repo
        .submit_experiment_trial(&experiment.id, 0, "first", 101)
        .unwrap();
    let lease = repo.claim_job(&trial.job_id, "worker", 1000, 102).unwrap();
    let checkpoint = repo
        .publish_checkpoint(&lease, &[1; 25], 1, json!({"epoch":1}), false, 103)
        .unwrap();
    repo.finish_training(&lease, b"weights", json!({}), 104)
        .unwrap();
    validate(&repo, &trial, 1., 105);
    let usage = repo.get_experiment(&experiment.id).unwrap().usage;
    assert_eq!(usage.checkpoint_bytes, checkpoint.blob.bytes);
    assert_eq!(usage.artifact_bytes, 7 + checkpoint.blob.bytes);
    // 32 retained bytes and another 30 byte reservation exceed the 60 byte budget.
    assert!(matches!(
        repo.submit_experiment_trial(&experiment.id, 1, "second", 106),
        Err(Error::Invalid(reason)) if reason.contains("artifact budget exhausted")
    ));
    let selected = repo.select_experiment_winner(&experiment.id, 107).unwrap();
    assert_eq!(selected.usage.checkpoint_bytes, 0);
    assert_eq!(selected.usage.artifact_bytes, 7);
}

#[test]
fn checkpoint_blobs_shared_with_artifacts_or_continuations_are_charged_once() {
    let (_directory, repo, _snapshot, mut request) = setup();
    request.candidates[0] = TrainingRequest {
        engine: "burn".into(),
        recipe: json!({"config":{"epochs":2},"search":{"family":"mlp-small","rung":0,"max_epochs":9,"reduction_factor":3}}),
        compute: json!({}),
    };
    request.budget.maximum_parallel_trials = 2;
    let experiment = repo.create_experiment(request, 100).unwrap();
    // Native engines checkpoint the finished model, which is the artifact blob itself.
    let native = repo
        .submit_experiment_trial(&experiment.id, 1, "native", 101)
        .unwrap();
    let lease = repo.claim_job(&native.job_id, "worker", 1000, 102).unwrap();
    repo.publish_checkpoint(&lease, b"weights", 1, json!({}), false, 103)
        .unwrap();
    repo.finish_training(&lease, b"weights", json!({}), 104)
        .unwrap();
    validate(&repo, &native, 0.9, 105);
    let usage = repo.get_experiment(&experiment.id).unwrap().usage;
    assert_eq!((usage.artifact_bytes, usage.checkpoint_bytes), (7, 0));
    let parent = repo
        .submit_experiment_trial(&experiment.id, 0, "initial", 106)
        .unwrap();
    let lease = repo.claim_job(&parent.job_id, "worker", 1000, 107).unwrap();
    let checkpoint = repo
        .publish_checkpoint(&lease, b"optimizer-state", 2, json!({}), false, 108)
        .unwrap();
    repo.finish_training(&lease, b"parent-weights", json!({}), 109)
        .unwrap();
    validate(&repo, &parent, 0.95, 110);
    // The continuation starts from the parent's blob until it publishes its own checkpoint.
    let child = repo.continue_experiment_trial(&parent.id, 6, 111).unwrap();
    let copied = repo.get_job(&child.job_id).unwrap().checkpoint.unwrap();
    assert_eq!(copied.blob, checkpoint.blob);
    let cancelled = repo.cancel_experiment(&experiment.id, 112).unwrap();
    assert_eq!(
        repo.get_experiment_trial(&child.id).unwrap().status,
        ExperimentTrialStatus::Cancelled
    );
    assert_eq!(cancelled.usage.checkpoint_bytes, checkpoint.blob.bytes);
    assert_eq!(
        cancelled.usage.artifact_bytes,
        7 + 14 + checkpoint.blob.bytes
    );
}

#[test]
fn resuming_or_finalizing_releases_the_retained_checkpoint_charge() {
    let (_directory, repo, _snapshot, request) = setup();
    let experiment = repo.create_experiment(request, 100).unwrap();
    let trial = repo
        .submit_experiment_trial(&experiment.id, 0, "candidate", 101)
        .unwrap();
    let lease = repo.claim_job(&trial.job_id, "worker", 1000, 102).unwrap();
    let checkpoint = repo
        .publish_checkpoint(&lease, b"optimizer-state", 1, json!({}), false, 103)
        .unwrap();
    let retained = checkpoint.blob.bytes;
    repo.cancel_experiment(&experiment.id, 104).unwrap();
    repo.acknowledge_cancel(&lease, 105).unwrap();
    repo.fail_experiment_trial(&trial.id, "cancelled", 0, 106)
        .unwrap();
    let cancelled = repo.get_experiment(&experiment.id).unwrap();
    assert_eq!(cancelled.status, ExperimentStatus::Cancelled);
    assert_eq!(
        (
            cancelled.usage.artifact_bytes,
            cancelled.usage.checkpoint_bytes
        ),
        (retained, retained)
    );
    // The requeued trial's checkpoint is covered by its new reservation.
    let resumed = repo.resume_experiment(&experiment.id, 107).unwrap();
    assert_eq!(resumed.status, ExperimentStatus::Running);
    assert_eq!(
        (resumed.usage.artifact_bytes, resumed.usage.checkpoint_bytes),
        (0, 0)
    );
    repo.fail_experiment_trial(&trial.id, "worker crashed", 0, 108)
        .unwrap();
    assert_eq!(
        repo.get_experiment(&experiment.id)
            .unwrap()
            .usage
            .checkpoint_bytes,
        retained
    );
    let finished = repo
        .finalize_experiment(
            &experiment.id,
            None,
            "no_candidate_met_validation_requirements",
            109,
        )
        .unwrap();
    assert_eq!(finished.status, ExperimentStatus::Failed);
    assert_eq!(
        (
            finished.usage.artifact_bytes,
            finished.usage.checkpoint_bytes
        ),
        (0, 0)
    );
}

#[test]
fn snapshot_validation_fails_without_waiting_for_the_write_lock() {
    let (_directory, repo, snapshot, request) = setup();
    let experiment = repo.create_experiment(request.clone(), 100).unwrap();
    let mut forged = snapshot.clone();
    forged.id.clear();
    forged.test[0].payload["label"] = json!("forged");
    let mut relabelled = snapshot;
    relabelled.id.clear();
    relabelled.digest.clear();
    relabelled.test[0].payload["label"] = json!("bad");
    // Another writer holds the ledger; validation must not wait the 15 s busy timeout for it.
    let mut writer = repo.connection().unwrap();
    let lock = writer
        .transaction_with_behavior(TransactionBehavior::Immediate)
        .unwrap();
    assert!(matches!(
        repo.import_experiment_snapshot(forged.clone()),
        Err(Error::Invalid(reason)) if reason.contains("target or provenance")
    ));
    assert!(matches!(
        repo.create_experiment_with_snapshot(forged, request, 101),
        Err(Error::Invalid(reason)) if reason.contains("target or provenance")
    ));
    assert!(matches!(
        repo.register_experiment_dataset(&experiment.id, relabelled, Value::Null, 101),
        Err(Error::Invalid(reason)) if reason.contains("target or provenance")
    ));
    drop(lock);
}

/// A corrected mapped feature (new raw digest at the same revision), a review revision whose
/// source row changed as well, and a new row.
fn edited_sources(snapshot: &DatasetSnapshot) -> [TrainingSample; 3] {
    let mut edited = snapshot.train[0].clone();
    edited.content_digest = "e".repeat(64);
    edited.payload["feature_row"] = json!({"x":41});
    let mut revised = snapshot.train[1].clone();
    revised.annotation_revision = 2;
    revised.content_digest = "f".repeat(64);
    revised.payload["label"] = json!("bad");
    revised.payload["feature_row"] = json!({"x":42});
    let mut added = snapshot.train[2].clone();
    added.id = "sample-added".into();
    added.group_id = "lot-added".into();
    added.content_digest = "a".repeat(64);
    [edited, revised, added]
}

fn stored(
    repo: &TrainingRepository,
    stream: &StreamKey,
    sample: &TrainingSample,
) -> Option<TrainingSample> {
    let scope = serde_json::to_string(stream).unwrap();
    recorded_annotation(&repo.connection().unwrap(), &scope, sample).unwrap()
}

#[test]
fn prepared_batches_record_edited_source_content() {
    let (_directory, repo, snapshot, _request) = setup();
    let stream = &snapshot.stream;
    let batch = edited_sources(&snapshot);
    assert_eq!(
        repo.record_prepared_samples(stream, &batch).unwrap(),
        [false, true, true]
    );
    for sample in &batch {
        assert_eq!(stored(&repo, stream, sample).as_ref(), Some(sample));
    }
    assert!(
        repo.record_prepared_sample(stream, &batch[2])
            .unwrap()
            .accepted_sequence
            .is_some()
    );
    assert_eq!(
        repo.record_prepared_samples(stream, &batch).unwrap(),
        [false; 3]
    );
}

#[test]
fn a_refused_prepared_sample_records_none_of_its_batch() {
    let (_directory, repo, snapshot, _request) = setup();
    let stream = &snapshot.stream;
    let [edited, revised, _] = edited_sources(&snapshot);
    repo.record_prepared_samples(stream, &[edited.clone(), revised.clone()])
        .unwrap();
    let mut again = edited.clone();
    again.content_digest = "d".repeat(64);
    let mut relabelled = revised.clone();
    relabelled.payload["label"] = json!("good");
    let error = repo
        .record_prepared_samples(stream, &[again.clone(), relabelled])
        .unwrap_err();
    assert!(
        matches!(&error, Error::Conflict(reason) if reason.contains(revised.id.as_str())),
        "{error}"
    );
    assert_eq!(stored(&repo, stream, &again), Some(edited));
    let mut regrouped = revised;
    regrouped.annotation_revision = 3;
    regrouped.group_id = "lot-elsewhere".into();
    assert!(matches!(
        repo.record_prepared_samples(stream, &[regrouped]),
        Err(Error::Conflict(_))
    ));
}

#[test]
fn re_prepared_holdout_content_voids_the_final_audit() {
    let (_directory, repo, snapshot, request) = setup();
    let (experiment, artifact) = frozen_winner(&repo, request);
    for sample in &snapshot.test {
        repo.record_prediction(&reviewed_prediction(&artifact, sample))
            .unwrap();
    }
    let report = repo.evaluate_classification(&artifact.id, 107).unwrap();
    let mut edited = snapshot.test[0].clone();
    edited.content_digest = "e".repeat(64);
    edited.payload["feature_row"] = json!({"x":-1});
    assert!(
        !repo
            .record_prepared_sample(&snapshot.stream, &edited)
            .unwrap()
            .inserted
    );
    assert!(matches!(
        repo.finalize_experiment(&experiment.id, Some(&report.id), "completed", 108),
        Err(Error::Conflict(reason)) if reason.contains("evidence changed")
    ));
}

#[test]
fn resume_does_not_charge_requeued_trials_for_their_own_checkpoints() {
    let (_directory, repo, _snapshot, mut request) = setup();
    request.budget.maximum_parallel_trials = 2;
    request.budget.maximum_artifact_bytes = 60;
    let experiment = repo.create_experiment(request, 100).unwrap();
    let mut stopped = vec![];
    for (index, key) in ["first", "second"].into_iter().enumerate() {
        let trial = repo
            .submit_experiment_trial(&experiment.id, index, key, 101)
            .unwrap();
        let lease = repo.claim_job(&trial.job_id, "worker", 1000, 102).unwrap();
        repo.publish_checkpoint(&lease, &[index as u8; 16], 1, json!({}), false, 103)
            .unwrap();
        stopped.push((trial, lease));
    }
    repo.cancel_experiment(&experiment.id, 104).unwrap();
    for (trial, lease) in &stopped {
        repo.acknowledge_cancel(lease, 105).unwrap();
        repo.fail_experiment_trial(&trial.id, "cancelled", 0, 106)
            .unwrap();
    }
    let usage = repo.get_experiment(&experiment.id).unwrap().usage;
    assert_eq!((usage.artifact_bytes, usage.checkpoint_bytes), (32, 32));
    // Each 30 byte slice covers its own trial's checkpoint, so both fit the 60 bytes exactly.
    let resumed = repo.resume_experiment(&experiment.id, 107).unwrap();
    assert_eq!(resumed.status, ExperimentStatus::Running);
    let usage = &resumed.usage;
    assert_eq!(
        (
            usage.artifact_bytes,
            usage.checkpoint_bytes,
            usage.reserved_artifact_bytes
        ),
        (0, 0, 60)
    );
    for (trial, _) in &stopped {
        let job = repo.get_job(&trial.job_id).unwrap();
        let trial = repo.get_experiment_trial(&trial.id).unwrap();
        assert_eq!(
            (trial.status, job.status),
            (ExperimentTrialStatus::Scheduled, JobStatus::Queued)
        );
    }
}

#[test]
fn resume_that_cannot_fit_keeps_charging_the_retained_checkpoint() {
    let (_directory, repo, _snapshot, mut request) = setup();
    request.budget.maximum_parallel_trials = 2;
    request.budget.maximum_artifact_bytes = 60;
    let experiment = repo.create_experiment(request, 100).unwrap();
    let winner = repo
        .submit_experiment_trial(&experiment.id, 0, "winner", 101)
        .unwrap();
    let stopped = repo
        .submit_experiment_trial(&experiment.id, 1, "stopped", 101)
        .unwrap();
    let lease = repo.claim_job(&winner.job_id, "worker", 1000, 102).unwrap();
    repo.publish_checkpoint(&lease, &[1; 25], 1, json!({}), false, 103)
        .unwrap();
    repo.finish_training(&lease, b"weights", json!({}), 104)
        .unwrap();
    validate(&repo, &winner, 1., 105);
    let lease = repo
        .claim_job(&stopped.job_id, "worker", 1000, 106)
        .unwrap();
    repo.publish_checkpoint(&lease, &[2; 15], 1, json!({}), false, 107)
        .unwrap();
    repo.cancel_experiment(&experiment.id, 108).unwrap();
    repo.acknowledge_cancel(&lease, 109).unwrap();
    repo.fail_experiment_trial(&stopped.id, "cancelled", 0, 110)
        .unwrap();
    let before = repo.get_experiment(&experiment.id).unwrap().usage;
    assert_eq!((before.artifact_bytes, before.checkpoint_bytes), (47, 40));
    // The winner's 7 byte model and 25 byte checkpoint leave 28 bytes, short of a 30 byte slice.
    let settled = repo.resume_experiment(&experiment.id, 111).unwrap();
    assert_eq!(settled.status, ExperimentStatus::BudgetExhausted);
    assert_eq!(
        settled.stop_reason.as_deref(),
        Some("resource_budget_exhausted")
    );
    let usage = &settled.usage;
    assert_eq!(
        (
            usage.artifact_bytes,
            usage.checkpoint_bytes,
            usage.reserved_artifact_bytes,
            usage.reserved_training_time_ms
        ),
        (47, 40, 0, 0)
    );
    let trial = repo.get_experiment_trial(&stopped.id).unwrap();
    assert_eq!(trial.status, ExperimentTrialStatus::Failed);
    assert_eq!(trial.reserved_artifact_bytes, 0);
    assert_eq!(
        repo.get_job(&stopped.job_id).unwrap().status,
        JobStatus::Cancelled
    );
}

#[test]
fn resume_inside_the_audit_allowance_ends_the_search() {
    let (_directory, repo, _snapshot, request) = setup();
    // The 10_000 ms wall budget keeps 1_000 ms for the final audit.
    let stopped_at = |cancel_at: i64| {
        let experiment = repo.create_experiment(request.clone(), 100).unwrap();
        let trial = repo
            .submit_experiment_trial(&experiment.id, 0, "candidate", 1_000)
            .unwrap();
        repo.cancel_experiment(&experiment.id, cancel_at).unwrap();
        (experiment, trial)
    };
    let (open, open_trial) = stopped_at(9_999);
    let resumed = repo.resume_experiment(&open.id, 20_000).unwrap();
    assert_eq!(resumed.status, ExperimentStatus::Running);
    assert_eq!(
        repo.get_job(&open_trial.job_id).unwrap().request.recipe["experiment_deadline_ms"],
        json!(20_001)
    );
    let (closed, closed_trial) = stopped_at(10_000);
    let settled = repo.resume_experiment(&closed.id, 20_000).unwrap();
    assert_eq!(settled.status, ExperimentStatus::BudgetExhausted);
    assert_eq!(
        settled.stop_reason.as_deref(),
        Some("wall_time_budget_exhausted")
    );
    assert_eq!(settled.usage.reserved_training_time_ms, 0);
    let trial = repo.get_experiment_trial(&closed_trial.id).unwrap();
    assert_eq!(trial.status, ExperimentTrialStatus::Failed);
    assert_eq!(trial.error.as_deref(), Some("budget exhausted on resume"));
    assert_eq!(
        repo.get_job(&closed_trial.job_id).unwrap().status,
        JobStatus::Cancelled
    );
}
