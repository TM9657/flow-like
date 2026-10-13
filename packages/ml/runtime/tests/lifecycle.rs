#![cfg(feature = "execution")]
use flow_like_ml_runtime::worker::*;
use flow_like_ml_runtime::*;
use serde_json::json;
use std::{
    collections::BTreeMap,
    sync::{Arc, Barrier},
};

fn stream() -> StreamKey {
    StreamKey {
        project_id: "factory".into(),
        stream_id: "line-1".into(),
        inspection_version: "v1".into(),
    }
}
fn sample(i: usize) -> TrainingSample {
    TrainingSample {
        id: format!("sample-{i}"),
        annotation_revision: 1,
        group_id: format!("lot-{i}"),
        captured_at_ms: i as i64,
        label_available_at_ms: i as i64,
        content_digest: format!("{i:064x}"),
        source: LabelSource::Reviewed,
        accepted: true,
        payload: json!({"label":if i%2==0 {"defect"} else {"good"},"values":[i as f32,1.0]}),
    }
}
fn policy() -> SplitPolicy {
    SplitPolicy::Group {
        train_fraction: 0.5,
        validation_fraction: 0.25,
        seed: 42,
    }
}
fn request() -> TrainingRequest {
    TrainingRequest {
        engine: "fixture".into(),
        recipe: json!({"epochs":2,"seed":42}),
        compute: json!({"memory_limit_bytes":1_000_000}),
    }
}
fn repo() -> (tempfile::TempDir, TrainingRepository) {
    let dir = tempfile::tempdir().unwrap();
    let repo = TrainingRepository::open(dir.path().join("training.sqlite")).unwrap();
    (dir, repo)
}

#[test]
fn pretrained_source_engine_filter_precedes_limit_and_preserves_scope_and_time_order() {
    let (_dir, repo) = repo();
    let register = |project: &str, engine: &str, time: i64| {
        repo.register_pretrained_source(
            project,
            b"source-list-fixture",
            json!({"engine":engine,"format_version":1,"config":{},"input_shape":[3,32,32]}),
            json!({"fixture":true}),
            1024,
            time,
        )
        .unwrap()
    };
    let newest_burn = register("factory", "burn", 100);
    register("factory", "burn", 50);
    for time in 200..220 {
        register("factory", "sam2", time);
    }
    register("other-project", "burn", 1000);
    // A late-arriving older source must not displace the newest timestamp.
    register("factory", "burn", 10);
    let filtered = repo
        .list_pretrained_sources_for_engine("factory", 1, "burn")
        .unwrap();
    assert_eq!(filtered.len(), 1);
    assert_eq!(filtered[0].id, newest_burn.id);
    let all = repo.list_pretrained_sources("factory", 16).unwrap();
    assert_eq!(all.len(), 16);
    assert!(all.iter().all(|source| source.manifest["engine"] == "sam2"));
    let sam = repo
        .list_pretrained_sources_for_engine("factory", 1, "sam2")
        .unwrap();
    assert_eq!(sam[0].created_at_ms, 219);
    assert!(
        repo.list_pretrained_sources_for_engine("factory", 1, "unknown")
            .is_err()
    );
}

fn populate(repo: &TrainingRepository, range: std::ops::Range<usize>) {
    for i in range {
        repo.record_sample(&stream(), &sample(i)).unwrap();
    }
}
fn queued(repo: &TrainingRepository) -> TrainingJob {
    repo.trigger_after_n(&stream(), 3, request(), policy(), 1000)
        .unwrap()
        .unwrap()
}
fn candidate(repo: &TrainingRepository) -> ModelArtifact {
    let job = queued(repo);
    let lease = repo.claim_job(&job.id, "worker", 1000, 1000).unwrap();
    repo.finish_training(
        &lease,
        b"trained-model",
        json!({"architecture":"fixture","input_shape":[2]}),
        1001,
    )
    .unwrap()
}
fn evaluate(repo: &TrainingRepository, artifact: &ModelArtifact) -> EvaluationReport {
    let snapshot = repo.get_snapshot(&artifact.snapshot_id).unwrap();
    for sample in snapshot.test {
        let label = sample.payload["label"].as_str().unwrap().to_owned();
        repo.record_prediction(&PredictionRecord {
            sample_id: sample.id,
            artifact_id: artifact.id.clone(),
            predicted_label: Some(label.clone()),
            teacher_label: Some(label.clone()),
            actual_label: Some(label),
            actual_source: Some(LabelSource::Reviewed),
            latency_ms: 5.,
            failed: false,
            recorded_at_ms: 1100,
            details: json!({}),
        })
        .unwrap();
    }
    repo.evaluate_classification(&artifact.id, 1101).unwrap()
}
fn promotion_policy() -> PromotionPolicy {
    PromotionPolicy {
        minimum_audited_samples: 1,
        minimum_accuracy: 0.9,
        minimum_class_recall: BTreeMap::new(),
        maximum_mean_latency_ms: 10.,
        maximum_failure_rate: 0.,
    }
}

#[test]
fn immutable_samples_and_snapshots_survive_restart() {
    let (dir, repo) = repo();
    populate(&repo, 0..12);
    assert!(!repo.record_sample(&stream(), &sample(0)).unwrap().inserted);
    let mut altered = sample(0);
    altered.payload = json!({"label":"good"});
    assert!(matches!(
        repo.record_sample(&stream(), &altered),
        Err(Error::Conflict(_))
    ));
    let frozen = repo.snapshot(&stream(), policy(), 1000).unwrap();
    altered.annotation_revision = 2;
    repo.record_sample(&stream(), &altered).unwrap();
    drop(repo);
    let repo = TrainingRepository::open(dir.path().join("training.sqlite")).unwrap();
    assert_eq!(frozen, repo.get_snapshot(&frozen.id).unwrap());
    let next = repo.snapshot(&stream(), policy(), 1000).unwrap();
    assert_ne!(frozen.digest, next.digest);
    assert_eq!(
        repo.record_sample(&stream(), &altered)
            .unwrap()
            .accepted_sequence,
        Some(1)
    );
}

#[test]
fn concurrent_connections_create_exactly_one_job() {
    let (_dir, repo) = repo();
    populate(&repo, 0..12);
    let barrier = Arc::new(Barrier::new(8));
    let threads: Vec<_> = (0..8)
        .map(|_| {
            let path = repo.path().to_owned();
            let barrier = barrier.clone();
            std::thread::spawn(move || {
                let independent = TrainingRepository::open(path).unwrap();
                barrier.wait();
                independent
                    .trigger_after_n(&stream(), 12, request(), policy(), 1000)
                    .unwrap()
            })
        })
        .collect();
    let jobs: Vec<_> = threads
        .into_iter()
        .filter_map(|thread| thread.join().unwrap())
        .collect();
    assert_eq!(jobs.len(), 1);
    assert_eq!(repo.list_jobs(&stream()).unwrap().len(), 1);
    let lease = repo.claim_job(&jobs[0].id, "worker", 1000, 1000).unwrap();
    repo.fail_job(&lease, "expected failure", 1001).unwrap();
    assert!(
        repo.trigger_after_n(&stream(), 1, request(), policy(), 1002)
            .unwrap()
            .is_none(),
        "failed data must not trigger again on every frame"
    );
    repo.resume_job(&jobs[0].id, 1003).unwrap();
}

#[test]
fn child_process_actor() {
    let Some(path) = std::env::var_os("FLOW_ML_TEST_LEDGER") else {
        return;
    };
    let repo = TrainingRepository::open(path).unwrap();
    for i in 0..12 {
        repo.record_sample(&stream(), &sample(i)).unwrap();
    }
    let _ = repo
        .trigger_after_n(&stream(), 12, request(), policy(), 1000)
        .unwrap();
}

#[test]
fn multiple_processes_deduplicate_and_claim_atomically() {
    let (_dir, repo) = repo();
    let mut children: Vec<_> = (0..3)
        .map(|_| {
            std::process::Command::new(std::env::current_exe().unwrap())
                .args(["--exact", "child_process_actor", "--nocapture"])
                .env("FLOW_ML_TEST_LEDGER", repo.path())
                .stdout(std::process::Stdio::null())
                .spawn()
                .unwrap()
        })
        .collect();
    for child in &mut children {
        assert!(child.wait().unwrap().success());
    }
    assert_eq!(repo.list_jobs(&stream()).unwrap().len(), 1);
    let snapshot = repo
        .get_snapshot(&repo.list_jobs(&stream()).unwrap()[0].snapshot_id)
        .unwrap();
    assert_eq!(
        snapshot.train.len() + snapshot.validation.len() + snapshot.test.len(),
        12
    );
}

#[test]
fn expired_workers_are_fenced_and_checkpoint_resume_is_durable() {
    let (dir, repo) = repo();
    populate(&repo, 0..12);
    let job = queued(&repo);
    let stale = repo.claim_job(&job.id, "old", 10, 1000).unwrap();
    let checkpoint = repo
        .publish_checkpoint(
            &stale,
            b"optimizer-and-rng",
            4,
            json!({"engine":"fixture","optimizer":"sgd","rng":42}),
            false,
            1001,
        )
        .unwrap();
    assert!(matches!(
        repo.claim_job(&job.id, "new", 10, 1011),
        Err(Error::Conflict(_))
    ));
    repo.mark_interrupted(&job.id, stale.generation, 1011)
        .unwrap();
    drop(repo);
    let repo = TrainingRepository::open(dir.path().join("training.sqlite")).unwrap();
    assert_eq!(
        repo.get_job(&job.id).unwrap().checkpoint,
        Some(checkpoint.clone())
    );
    repo.resume_job(&job.id, 1012).unwrap();
    let current = repo.claim_job(&job.id, "new", 1000, 1013).unwrap();
    assert!(matches!(
        repo.finish_training(&stale, b"stale-model", json!({}), 1014),
        Err(Error::LeaseLost)
    ));
    assert_eq!(
        repo.read_blob(&checkpoint.blob).unwrap(),
        b"optimizer-and-rng"
    );
    repo.finish_training(&current, b"resumed-model", json!({}), 1014)
        .unwrap();
    assert_eq!(repo.get_job(&job.id).unwrap().status, JobStatus::Succeeded);
}

#[test]
fn interrupted_artifact_write_cannot_replace_checkpoint() {
    let (dir, repo) = repo();
    populate(&repo, 0..12);
    let job = queued(&repo);
    let lease = repo.claim_job(&job.id, "worker", 1000, 1000).unwrap();
    let checkpoint = repo
        .publish_checkpoint(
            &lease,
            b"valid-checkpoint",
            1,
            json!({"optimizer":"sgd"}),
            false,
            1001,
        )
        .unwrap();
    std::fs::write(
        repo.path().with_extension("artifacts").join("crash.part"),
        b"partial",
    )
    .unwrap();
    drop(repo);
    let repo = TrainingRepository::open(dir.path().join("training.sqlite")).unwrap();
    assert!(
        !repo
            .path()
            .with_extension("artifacts")
            .join("crash.part")
            .exists(),
        "reopening sweeps staging files left by a crashed writer"
    );
    assert_eq!(
        repo.get_job(&job.id).unwrap().checkpoint,
        Some(checkpoint.clone())
    );
    std::fs::write(
        repo.path()
            .with_extension("artifacts")
            .join(&checkpoint.blob.path),
        b"corrupted",
    )
    .unwrap();
    assert!(matches!(
        repo.read_blob(&checkpoint.blob),
        Err(Error::Invalid(_))
    ));
    let mut traversal = checkpoint.blob;
    traversal.path = "../outside".into();
    assert!(repo.read_blob(&traversal).is_err());
}

#[test]
fn rejected_and_delayed_labels_are_not_lost_or_counted_early() {
    let (_dir, repo) = repo();
    let mut delayed = sample(0);
    delayed.label_available_at_ms = 2000;
    repo.record_sample(&stream(), &delayed).unwrap();
    populate(&repo, 1..13);
    let artifact = candidate(&repo);
    assert!(
        !repo
            .get_snapshot(&artifact.snapshot_id)
            .unwrap()
            .train
            .iter()
            .any(|s| s.id == delayed.id)
    );
    assert!(
        repo.trigger_after_n(&stream(), 1, request(), policy(), 1999)
            .unwrap()
            .is_none()
    );
    let next = repo
        .trigger_after_n(&stream(), 1, request(), policy(), 2000)
        .unwrap();
    assert!(
        next.is_some(),
        "a delayed outcome preceding the trigger watermark must remain eligible"
    );
}

#[test]
fn grouped_time_split_excludes_boundary_crossing_lots_and_future_truth() {
    let (_dir, repo) = repo();
    for (i, time, group) in [
        (0, 10, "train"),
        (1, 12, "train"),
        (2, 30, "validation"),
        (3, 50, "test"),
        (4, 19, "cross"),
        (5, 23, "cross"),
    ] {
        let mut sample = sample(i);
        sample.captured_at_ms = time;
        sample.label_available_at_ms = time;
        sample.group_id = group.into();
        repo.record_sample(&stream(), &sample).unwrap();
    }
    let snapshot = repo
        .snapshot(
            &stream(),
            SplitPolicy::Time {
                train_end_ms: 20,
                validation_end_ms: 40,
                embargo_ms: 3,
            },
            100,
        )
        .unwrap();
    assert_eq!(snapshot.train.len(), 2);
    assert_eq!(snapshot.validation.len(), 1);
    assert_eq!(snapshot.test.len(), 1);
    assert_eq!(snapshot.excluded.len(), 2);
    let sets: Vec<std::collections::HashSet<_>> =
        [&snapshot.train, &snapshot.validation, &snapshot.test]
            .iter()
            .map(|part| part.iter().map(|s| &s.group_id).collect())
            .collect();
    assert!(
        sets[0].is_disjoint(&sets[1])
            && sets[0].is_disjoint(&sets[2])
            && sets[1].is_disjoint(&sets[2])
    );
}

#[test]
fn teacher_only_agreement_cannot_promote() {
    let (_dir, repo) = repo();
    populate(&repo, 0..12);
    let artifact = candidate(&repo);
    repo.record_prediction(&PredictionRecord {
        sample_id: "unknown-stream-sample".into(),
        artifact_id: artifact.id.clone(),
        predicted_label: Some("good".into()),
        teacher_label: Some("good".into()),
        actual_label: None,
        actual_source: None,
        latency_ms: 1.,
        failed: false,
        recorded_at_ms: 1100,
        details: json!({}),
    })
    .unwrap();
    let report = repo.evaluate_classification(&artifact.id, 1101).unwrap();
    assert_eq!(report.metrics["teacher_agreement"], 1.);
    assert_eq!(report.audited_samples, 0);
    assert!(matches!(
        repo.promote("line-production", 0, &report.id, promotion_policy(), 1102),
        Err(Error::Conflict(_))
    ));
}

#[test]
fn evaluation_rejects_training_group_leakage_and_forged_truth() {
    let (_dir, repo) = repo();
    populate(&repo, 0..12);
    let artifact = candidate(&repo);
    let snapshot = repo.get_snapshot(&artifact.snapshot_id).unwrap();
    let sample = &snapshot.train[0];
    let mut prediction = PredictionRecord {
        sample_id: sample.id.clone(),
        artifact_id: artifact.id.clone(),
        predicted_label: Some("good".into()),
        teacher_label: None,
        actual_label: Some("forged".into()),
        actual_source: Some(LabelSource::Reviewed),
        latency_ms: 1.,
        failed: false,
        recorded_at_ms: 1100,
        details: json!({}),
    };
    assert!(repo.record_prediction(&prediction).is_err());
    prediction.actual_label = sample.payload["label"].as_str().map(str::to_owned);
    repo.record_prediction(&prediction).unwrap();
    assert!(matches!(
        repo.evaluate_classification(&artifact.id, 1101),
        Err(Error::Invalid(_))
    ));
}

#[test]
fn promotion_generation_races_and_rollback_survive_restart() {
    let (dir, repo) = repo();
    populate(&repo, 0..12);
    let first = candidate(&repo);
    let report = evaluate(&repo, &first);
    let barrier = Arc::new(Barrier::new(2));
    let threads: Vec<_> = (0..2)
        .map(|_| {
            let repo = repo.clone();
            let report = report.clone();
            let barrier = barrier.clone();
            std::thread::spawn(move || {
                barrier.wait();
                repo.promote("production", 0, &report.id, promotion_policy(), 1200)
            })
        })
        .collect();
    assert_eq!(
        threads
            .into_iter()
            .map(|thread| thread.join().unwrap())
            .filter(Result::is_ok)
            .count(),
        1
    );
    populate(&repo, 12..24);
    let second = candidate(&repo);
    let report = evaluate(&repo, &second);
    repo.promote("production", 1, &report.id, promotion_policy(), 1201)
        .unwrap();
    drop(repo);
    let repo = TrainingRepository::open(dir.path().join("training.sqlite")).unwrap();
    assert_eq!(
        repo.get_deployment("production")
            .unwrap()
            .active_artifact_id,
        second.id
    );
    let rolled = repo.rollback("production", 2, 1202).unwrap();
    assert_eq!(rolled.active_artifact_id, first.id);
    assert_eq!(rolled.generation, 3);
    assert!(repo.rollback("production", 2, 1203).is_err());
}

#[test]
fn growing_stream_waits_for_independent_partitions_without_consuming_trigger() {
    let (_dir, repo) = repo();
    populate(&repo, 0..1);
    assert!(
        repo.trigger_after_n(&stream(), 1, request(), policy(), 1000)
            .unwrap()
            .is_none()
    );
    populate(&repo, 1..3);
    assert!(
        repo.trigger_after_n(&stream(), 3, request(), policy(), 1000)
            .unwrap()
            .is_some()
    );

    let directory = tempfile::tempdir().unwrap();
    let repo = TrainingRepository::open(directory.path().join("time.sqlite")).unwrap();
    let policy = SplitPolicy::Time {
        train_end_ms: 2,
        validation_end_ms: 5,
        embargo_ms: 1,
    };
    populate(&repo, 0..3);
    assert!(
        repo.trigger_after_n(&stream(), 1, request(), policy.clone(), 1000)
            .unwrap()
            .is_none()
    );
    populate(&repo, 3..9);
    assert!(
        repo.trigger_after_n(&stream(), 9, request(), policy, 1000)
            .unwrap()
            .is_some()
    );
}

#[test]
fn suspension_fences_promotion_and_preserves_artifact_provenance() {
    let (dir, repo) = repo();
    populate(&repo, 0..12);
    let first = candidate(&repo);
    let first_report = evaluate(&repo, &first);
    repo.promote("production", 0, &first_report.id, promotion_policy(), 1200)
        .unwrap();
    let paused = repo
        .set_deployment_paused("production", 1, true, 1201)
        .unwrap();
    assert!(paused.paused);
    assert_eq!(paused.active_artifact_id, first.id);
    assert_eq!(paused.evaluation_id, first_report.id);
    assert!(paused.previous_artifact_id.is_none());
    assert!(
        repo.rollback("production", paused.generation, 1202)
            .is_err()
    );
    assert!(matches!(
        repo.shadow_telemetry()
            .set_deployment_paused("production", 2, false, 1202),
        Err(Error::ShadowWrite)
    ));
    populate(&repo, 12..24);
    let second = candidate(&repo);
    let report = evaluate(&repo, &second);
    let barrier = Arc::new(Barrier::new(2));
    let pause_repo = repo.clone();
    let pause_barrier = barrier.clone();
    let pauser = std::thread::spawn(move || {
        pause_barrier.wait();
        pause_repo.set_deployment_paused("production", 2, true, 1203)
    });
    let promote_repo = repo.clone();
    let promoter = std::thread::spawn(move || {
        barrier.wait();
        promote_repo.promote("production", 2, &report.id, promotion_policy(), 1203)
    });
    let results = [pauser.join().unwrap(), promoter.join().unwrap()];
    assert_eq!(results.iter().filter(|result| result.is_ok()).count(), 1);
    assert!(
        results
            .iter()
            .any(|result| matches!(result, Err(Error::Conflict(_))))
    );
    let state = repo.get_deployment("production").unwrap();
    assert_eq!(state.generation, 3);
    assert_eq!(state.stream, stream());
    assert_eq!(
        state.active_artifact_id,
        if state.paused {
            first.id.clone()
        } else {
            second.id.clone()
        }
    );
    drop(repo);
    let repo = TrainingRepository::open(dir.path().join("training.sqlite")).unwrap();
    assert_eq!(repo.get_deployment("production").unwrap(), state);
    if state.active_artifact_id == second.id {
        let rolled = repo.rollback("production", 3, 1204).unwrap();
        assert_eq!(rolled.active_artifact_id, first.id);
        assert!(
            rolled.paused,
            "rollback restores the prior suspended routing state"
        );
    }
}

#[test]
fn shadow_handle_cannot_mutate_training_or_deployment() {
    let (_dir, repo) = repo();
    populate(&repo, 0..12);
    let artifact = candidate(&repo);
    let shadow = repo.shadow_telemetry();
    assert!(matches!(
        shadow.record_sample(&stream(), &sample(99)),
        Err(Error::ShadowWrite)
    ));
    assert!(matches!(
        shadow.snapshot(&stream(), policy(), 1200),
        Err(Error::ShadowWrite)
    ));
    assert!(matches!(
        shadow.trigger_after_n(&stream(), 3, request(), policy(), 1200),
        Err(Error::ShadowWrite)
    ));
    shadow
        .record_prediction(&PredictionRecord {
            sample_id: "shadow-sample".into(),
            artifact_id: artifact.id.clone(),
            predicted_label: Some("good".into()),
            teacher_label: Some("good".into()),
            actual_label: None,
            actual_source: None,
            latency_ms: 2.,
            failed: false,
            recorded_at_ms: 1200,
            details: json!({}),
        })
        .unwrap();
    assert_eq!(repo.predictions(&artifact.id).unwrap().len(), 1);
    assert!(matches!(
        shadow.evaluate_classification(&artifact.id, 1201),
        Err(Error::ShadowWrite)
    ));
}

struct FixtureEngine {
    cancel: Option<TrainingRepository>,
    memory: u64,
}
impl TrainingEngine for FixtureEngine {
    fn estimated_memory_bytes(&self, _: &TrainingWork) -> Result<u64> {
        Ok(self.memory)
    }
    fn train(&self, work: &TrainingWork, control: &WorkerControl) -> Result<EngineOutput> {
        control.checkpoint(
            b"state",
            1,
            json!({"engine":"fixture","optimizer":"sgd","sampler":1}),
        )?;
        control.progress(json!({"step":1}))?;
        if let Some(repo) = &self.cancel {
            repo.request_cancel(&work.job.id, now_ms())?;
            control.check_cancel()?;
        }
        Ok(EngineOutput {
            model_bytes: b"actual-native-output".to_vec(),
            manifest: json!({"engine":"fixture"}),
        })
    }
}

#[test]
fn worker_checkpoint_cancel_resume_and_memory_budget_are_real() {
    let (_dir, repo) = repo();
    populate(&repo, 0..12);
    let job = queued(&repo);
    let mut worker = TrainingWorker::new(repo.clone(), WorkerLimits::default()).unwrap();
    worker
        .register(
            "fixture",
            Arc::new(FixtureEngine {
                cancel: None,
                memory: 2_000_000,
            }),
        )
        .unwrap();
    assert!(matches!(worker.run(&job.id), Err(Error::Engine(_))));
    assert_eq!(repo.get_job(&job.id).unwrap().status, JobStatus::Queued);
    let mut worker = TrainingWorker::new(repo.clone(), WorkerLimits::default()).unwrap();
    worker
        .register(
            "fixture",
            Arc::new(FixtureEngine {
                cancel: Some(repo.clone()),
                memory: 100,
            }),
        )
        .unwrap();
    assert!(matches!(worker.run(&job.id), Err(Error::Cancelled)));
    let cancelled = repo.get_job(&job.id).unwrap();
    assert_eq!(cancelled.status, JobStatus::Cancelled);
    assert!(cancelled.checkpoint.is_some());
    repo.resume_job(&job.id, now_ms()).unwrap();
    let mut worker = TrainingWorker::new(repo.clone(), WorkerLimits::default()).unwrap();
    worker
        .register(
            "fixture",
            Arc::new(FixtureEngine {
                cancel: None,
                memory: 100,
            }),
        )
        .unwrap();
    let artifact = worker.run(&job.id).unwrap();
    assert_eq!(
        repo.read_blob(&artifact.blob).unwrap(),
        b"actual-native-output"
    );
    assert_eq!(repo.get_job(&job.id).unwrap().status, JobStatus::Succeeded);
}

#[test]
fn resource_admission_is_shared_across_connections_and_survives_restart() {
    let (directory, repo) = repo();
    populate(&repo, 0..12);
    let first = queued(&repo);
    let second_stream = StreamKey {
        stream_id: "line-2".into(),
        ..stream()
    };
    for i in 0..12 {
        repo.record_sample(&second_stream, &sample(i)).unwrap();
    }
    let second = repo
        .trigger_after_n(&second_stream, 12, request(), policy(), 1000)
        .unwrap()
        .unwrap();
    let resource = ResourceRequest {
        key: "cpu:0".into(),
        estimated_bytes: 600,
        budget_bytes: 1000,
    };
    let lease = repo
        .claim_job_with_resources(&first.id, "one", 1000, 1000, Some(resource.clone()))
        .unwrap();
    drop(repo);
    let repo = TrainingRepository::open(directory.path().join("training.sqlite")).unwrap();
    assert!(matches!(
        repo.claim_job_with_resources(&second.id, "two", 1000, 1001, Some(resource.clone())),
        Err(Error::Conflict(_))
    ));
    assert_eq!(repo.pending_jobs().unwrap().len(), 1);
    assert_eq!(repo.expired_jobs(2001).unwrap().len(), 1);
    repo.mark_interrupted(&first.id, lease.generation, 2001)
        .unwrap();
    assert!(
        repo.claim_job_with_resources(&second.id, "two", 1000, 2002, Some(resource.clone()))
            .is_err(),
        "expired worker retains memory until reconciled"
    );
    repo.request_cancel(&first.id, 2003).unwrap();
    repo.claim_job_with_resources(&second.id, "two", 1000, 2004, Some(resource))
        .unwrap();
}

#[test]
fn snapshot_byte_limit_precedes_deserialization_and_worker_admission() {
    use std::sync::atomic::{AtomicUsize, Ordering};
    struct AdmissionProbe(Arc<AtomicUsize>);
    impl TrainingEngine for AdmissionProbe {
        fn estimated_memory_bytes(&self, _: &TrainingWork) -> Result<u64> {
            self.0.fetch_add(1, Ordering::SeqCst);
            Ok(1)
        }
        fn train(&self, _: &TrainingWork, _: &WorkerControl) -> Result<EngineOutput> {
            panic!("oversized snapshots must never reach training")
        }
    }
    let (_dir, repo) = repo();
    for index in 0..6 {
        let mut record = sample(index);
        record.payload["unicode"] = json!("é".repeat(512));
        repo.record_sample(&stream(), &record).unwrap();
    }
    let job = queued(&repo);
    let snapshot = repo.get_snapshot(&job.snapshot_id).unwrap();
    let bytes = serde_json::to_vec(&snapshot).unwrap().len() as u64;
    assert!(repo.get_snapshot_limited(&snapshot.id, bytes).is_ok());
    assert!(matches!(
        repo.get_snapshot_limited(&snapshot.id, bytes - 1),
        Err(Error::Invalid(_))
    ));
    assert!(matches!(
        repo.get_snapshot_limited("missing", bytes),
        Err(Error::NotFound(_))
    ));
    let calls = Arc::new(AtomicUsize::new(0));
    let mut worker = TrainingWorker::new(
        repo.clone(),
        WorkerLimits {
            memory_budget_bytes: bytes - 1,
            ..WorkerLimits::default()
        },
    )
    .unwrap();
    worker
        .register("fixture", Arc::new(AdmissionProbe(calls.clone())))
        .unwrap();
    assert!(matches!(worker.run(&job.id), Err(Error::Invalid(_))));
    assert_eq!(calls.load(Ordering::SeqCst), 0);
    assert_eq!(repo.get_job(&job.id).unwrap().status, JobStatus::Queued);
    let connection = rusqlite::Connection::open(repo.path()).unwrap();
    connection
        .execute(
            "UPDATE snapshots SET body=? WHERE id=?",
            rusqlite::params!["{".repeat(bytes as usize), snapshot.id],
        )
        .unwrap();
    assert!(
        matches!(
            repo.get_snapshot_limited(&snapshot.id, bytes - 1),
            Err(Error::Invalid(_))
        ),
        "oversize must be rejected before attempting to parse corrupt JSON"
    );
    assert!(matches!(
        repo.get_snapshot_limited(&snapshot.id, bytes),
        Err(Error::Json(_))
    ));
}

fn fixture_worker(
    repo: &TrainingRepository,
    engine: impl TrainingEngine + 'static,
) -> TrainingWorker {
    let mut worker = TrainingWorker::new(repo.clone(), WorkerLimits::default()).unwrap();
    worker.register("fixture", Arc::new(engine)).unwrap();
    worker
}

#[test]
fn pre_claim_failures_are_recorded_until_a_claim_succeeds() {
    let (_dir, repo) = repo();
    populate(&repo, 0..12);
    let job = queued(&repo);
    let oversized = fixture_worker(
        &repo,
        FixtureEngine {
            cancel: None,
            memory: 2_000_000,
        },
    );
    assert!(matches!(oversized.run(&job.id), Err(Error::Engine(_))));
    let waiting = repo.get_job(&job.id).unwrap();
    assert_eq!(waiting.status, JobStatus::Queued);
    assert!(
        waiting
            .error
            .as_deref()
            .is_some_and(|error| error.contains("training needs an estimated 2000000 bytes")),
        "{:?}",
        waiting.error
    );
    fixture_worker(
        &repo,
        FixtureEngine {
            cancel: None,
            memory: 100,
        },
    )
    .run(&job.id)
    .unwrap();
    let finished = repo.get_job(&job.id).unwrap();
    assert_eq!(finished.status, JobStatus::Succeeded);
    assert!(finished.error.is_none());
}

#[test]
fn expired_leases_are_settled_by_cancel_recovery_and_resume() {
    let (_dir, repo) = repo();
    populate(&repo, 0..12);
    let crashed = queued(&repo);
    repo.claim_job_with_resources(
        &crashed.id,
        "crashed",
        10,
        1000,
        Some(ResourceRequest {
            key: "cpu:0".into(),
            estimated_bytes: 10,
            budget_bytes: 100,
        }),
    )
    .unwrap();
    let cancelled = repo.request_cancel(&crashed.id, 2000).unwrap();
    assert_eq!(cancelled.status, JobStatus::Cancelled);
    assert!(cancelled.lease_owner.is_none());
    let reservations: i64 = rusqlite::Connection::open(repo.path())
        .unwrap()
        .query_row("SELECT count(*) FROM resource_reservations", [], |row| {
            row.get(0)
        })
        .unwrap();
    assert_eq!(reservations, 0);

    populate(&repo, 12..15);
    let stopping = repo
        .trigger_after_n(&stream(), 1, request(), policy(), 2001)
        .unwrap()
        .expect("a settled cancellation frees the stream");
    repo.claim_job(&stopping.id, "crashed", 10, 2002).unwrap();
    assert_eq!(
        repo.request_cancel(&stopping.id, 2003).unwrap().status,
        JobStatus::CancelRequested
    );
    assert!(
        repo.recover_expired_jobs(&stream(), 2005)
            .unwrap()
            .is_empty()
    );
    let recovered = repo.recover_expired_jobs(&stream(), 2012).unwrap();
    assert_eq!(recovered.len(), 1);
    assert_eq!(recovered[0].status, JobStatus::Cancelled);
    assert!(
        repo.recover_expired_jobs(&stream(), 2013)
            .unwrap()
            .is_empty()
    );

    populate(&repo, 15..18);
    let interrupted = repo
        .trigger_after_n(&stream(), 1, request(), policy(), 2020)
        .unwrap()
        .unwrap();
    let stale = repo
        .claim_job(&interrupted.id, "crashed", 10, 2021)
        .unwrap();
    let recovered = repo.recover_expired_jobs(&stream(), 2040).unwrap();
    assert_eq!(recovered.len(), 1);
    assert_eq!(recovered[0].status, JobStatus::Interrupted);
    assert!(recovered[0].generation > stale.generation);
    assert_eq!(
        repo.resume_job(&interrupted.id, 2041).unwrap().status,
        JobStatus::Queued
    );
    let crashed_again = repo
        .claim_job(&interrupted.id, "crashed-again", 10, 2042)
        .unwrap();
    let requeued = repo.resume_job(&interrupted.id, 2100).unwrap();
    assert_eq!(requeued.status, JobStatus::Queued);
    assert!(requeued.generation > crashed_again.generation);
}

#[test]
fn reservations_of_dead_workers_are_reclaimed_by_the_next_claim() {
    let (_dir, repo) = repo();
    populate(&repo, 0..12);
    let crashed = queued(&repo);
    let other = StreamKey {
        stream_id: "line-2".into(),
        ..stream()
    };
    for i in 0..12 {
        repo.record_sample(&other, &sample(i)).unwrap();
    }
    let next = repo
        .trigger_after_n(&other, 12, request(), policy(), 1000)
        .unwrap()
        .unwrap();
    let resource = ResourceRequest {
        key: "cpu:0".into(),
        estimated_bytes: 600,
        budget_bytes: 1000,
    };
    repo.claim_job_with_resources(&crashed.id, "crashed", 10, 1000, Some(resource.clone()))
        .unwrap();
    assert!(matches!(
        repo.claim_job_with_resources(&next.id, "next", 1000, 1005, Some(resource.clone())),
        Err(Error::Conflict(message)) if message.contains(&crashed.id) && message.contains("cpu:0")
    ));
    repo.claim_job_with_resources(&next.id, "next", 1000, 1010, Some(resource))
        .unwrap();
    assert_eq!(
        repo.get_job(&crashed.id).unwrap().status,
        JobStatus::Interrupted
    );
}

struct PanickingEngine {
    message: &'static str,
    formatted: bool,
}
impl TrainingEngine for PanickingEngine {
    fn estimated_memory_bytes(&self, _: &TrainingWork) -> Result<u64> {
        Ok(1)
    }
    fn train(&self, _: &TrainingWork, _: &WorkerControl) -> Result<EngineOutput> {
        if self.formatted {
            panic!("{}", self.message);
        }
        std::panic::panic_any(self.message)
    }
}

#[test]
fn engine_panics_keep_their_message_and_name_device_memory_exhaustion() {
    for (engine, expected) in [
        (
            PanickingEngine {
                message: "tensor shapes [2, 3] and [4] differ",
                formatted: false,
            },
            "training engine panicked: tensor shapes [2, 3] and [4] differ",
        ),
        (
            PanickingEngine {
                message: "CUDA_ERROR_OUT_OF_MEMORY while allocating 3221225472 bytes",
                formatted: true,
            },
            "out of device memory; reduce the batch size or input resolution",
        ),
    ] {
        let (_dir, repo) = repo();
        populate(&repo, 0..12);
        let job = queued(&repo);
        let error = fixture_worker(&repo, engine)
            .run(&job.id)
            .unwrap_err()
            .to_string();
        assert!(error.contains(expected), "{error}");
        let failed = repo.get_job(&job.id).unwrap();
        assert_eq!(failed.status, JobStatus::Failed);
        assert!(failed.error.unwrap().contains(expected));
    }
}

#[test]
fn engines_reject_snapshots_by_stored_size_before_parsing() {
    use std::sync::atomic::{AtomicUsize, Ordering};
    struct DatasetScaled(Arc<AtomicUsize>);
    impl TrainingEngine for DatasetScaled {
        fn estimated_memory_bytes(&self, _: &TrainingWork) -> Result<u64> {
            self.0.fetch_add(1, Ordering::SeqCst);
            Ok(1)
        }
        fn snapshot_byte_limit(&self, budget: u64) -> u64 {
            budget / 12
        }
        fn train(&self, _: &TrainingWork, _: &WorkerControl) -> Result<EngineOutput> {
            panic!("snapshots above the engine limit must never be parsed")
        }
    }
    let (_dir, repo) = repo();
    populate(&repo, 0..12);
    let job = queued(&repo);
    let bytes = serde_json::to_vec(&repo.get_snapshot(&job.snapshot_id).unwrap())
        .unwrap()
        .len() as u64;
    let calls = Arc::new(AtomicUsize::new(0));
    let mut worker = TrainingWorker::new(
        repo.clone(),
        WorkerLimits {
            memory_budget_bytes: bytes * 6,
            ..WorkerLimits::default()
        },
    )
    .unwrap();
    worker
        .register("fixture", Arc::new(DatasetScaled(calls.clone())))
        .unwrap();
    assert!(matches!(worker.run(&job.id), Err(Error::Invalid(_))));
    assert_eq!(calls.load(Ordering::SeqCst), 0);
    assert!(repo.get_job(&job.id).unwrap().error.is_some());
}

struct ProgressProbe;
impl TrainingEngine for ProgressProbe {
    fn estimated_memory_bytes(&self, _: &TrainingWork) -> Result<u64> {
        Ok(1)
    }
    fn train(&self, work: &TrainingWork, control: &WorkerControl) -> Result<EngineOutput> {
        let stored = || {
            work.repository
                .get_job(&work.job.id)
                .map(|job| job.progress)
        };
        for batch in 0..50 {
            control.progress(json!({"epoch":0,"batch":batch,"validation_loss":null}))?;
        }
        if stored()? != serde_json::Value::Null {
            return Err(Error::Engine("batch progress reached the ledger".into()));
        }
        control.progress(json!({"epoch":0,"batch":50,"validation_loss":0.5}))?;
        if stored()?["validation_loss"] != json!(0.5) {
            return Err(Error::Engine("epoch-end progress was not stored".into()));
        }
        control.progress(json!({"epoch":1,"batch":3,"validation_loss":null}))?;
        Ok(EngineOutput {
            model_bytes: b"probe-model".to_vec(),
            manifest: json!({}),
        })
    }
}

#[test]
fn batch_progress_is_buffered_while_epoch_ends_and_completion_are_stored() {
    let (_dir, repo) = repo();
    populate(&repo, 0..12);
    let job = queued(&repo);
    fixture_worker(&repo, ProgressProbe).run(&job.id).unwrap();
    assert_eq!(
        repo.get_job(&job.id).unwrap().progress,
        json!({"epoch":1,"batch":3,"validation_loss":null})
    );
}

struct EpochCheckpoints;
impl TrainingEngine for EpochCheckpoints {
    fn estimated_memory_bytes(&self, _: &TrainingWork) -> Result<u64> {
        Ok(1)
    }
    fn train(&self, _: &TrainingWork, control: &WorkerControl) -> Result<EngineOutput> {
        for epoch in 1..=3u64 {
            control.checkpoint(
                format!("optimizer-state-{epoch}").as_bytes(),
                epoch,
                json!({"epoch":epoch}),
            )?;
        }
        Ok(EngineOutput {
            model_bytes: b"weights".to_vec(),
            manifest: json!({}),
        })
    }
}

#[test]
fn finished_jobs_leave_only_their_model_in_the_blob_store() {
    let (_dir, repo) = repo();
    populate(&repo, 0..12);
    let job = queued(&repo);
    let artifact = fixture_worker(&repo, EpochCheckpoints)
        .run(&job.id)
        .unwrap();
    let blobs: Vec<String> = std::fs::read_dir(repo.path().with_extension("artifacts"))
        .unwrap()
        .map(|entry| entry.unwrap().file_name().into_string().unwrap())
        .collect();
    assert_eq!(blobs, vec![artifact.blob.path.clone()]);
    assert_eq!(
        artifact.dataset_digest,
        repo.get_snapshot(&job.snapshot_id).unwrap().digest
    );
}

#[test]
fn label_corrections_replace_evaluation_truth_instead_of_blocking_it() {
    let (_dir, repo) = repo();
    populate(&repo, 0..12);
    let artifact = candidate(&repo);
    let report = evaluate(&repo, &artifact);
    assert_eq!(report.metrics["accuracy"], 1.0);
    let corrected = repo.get_snapshot(&artifact.snapshot_id).unwrap().test[0].clone();
    let original = corrected.payload["label"].as_str().unwrap().to_owned();
    let mut revision = corrected.clone();
    revision.annotation_revision = 2;
    revision.label_available_at_ms = 1150;
    revision.payload["label"] = json!(if original == "good" { "defect" } else { "good" });
    repo.record_sample(&stream(), &revision).unwrap();
    assert!(matches!(
        repo.promote("production", 0, &report.id, promotion_policy(), 1200),
        Err(Error::Conflict(_))
    ));
    let reevaluated = repo.evaluate_classification(&artifact.id, 1200).unwrap();
    assert_eq!(reevaluated.audited_samples, report.audited_samples);
    assert!(reevaluated.metrics["accuracy"] < 1.0);
    let outdated = PredictionRecord {
        sample_id: corrected.id.clone(),
        artifact_id: artifact.id.clone(),
        predicted_label: Some(original.clone()),
        teacher_label: None,
        actual_label: Some(original),
        actual_source: Some(LabelSource::Reviewed),
        latency_ms: 5.,
        failed: false,
        recorded_at_ms: 1300,
        details: json!({}),
    };
    assert!(
        matches!(repo.record_prediction(&outdated), Err(Error::Invalid(_))),
        "evidence recorded after a correction must carry the corrected truth"
    );
    let mut withdrawn = revision;
    withdrawn.annotation_revision = 3;
    withdrawn.accepted = false;
    withdrawn.label_available_at_ms = 1250;
    repo.record_sample(&stream(), &withdrawn).unwrap();
    let pruned = repo.evaluate_classification(&artifact.id, 1300).unwrap();
    assert_eq!(pruned.audited_samples, report.audited_samples - 1);
    assert_eq!(pruned.metrics["accuracy"], 1.0);
    repo.promote("production", 0, &pruned.id, promotion_policy(), 1301)
        .unwrap();
}

#[cfg(feature = "native")]
#[test]
fn failed_audited_predictions_count_as_misses_instead_of_dropping_class_metrics() {
    let (_dir, repo) = repo();
    populate(&repo, 0..12);
    let job = queued(&repo);
    let lease = repo.claim_job(&job.id, "worker", 1000, 1000).unwrap();
    let artifact = repo
        .finish_training(
            &lease,
            b"labelled-model",
            json!({"labels":["defect","good"]}),
            1001,
        )
        .unwrap();
    let snapshot = repo.get_snapshot(&artifact.snapshot_id).unwrap();
    for (index, sample) in snapshot.test.iter().enumerate() {
        let label = sample.payload["label"].as_str().unwrap().to_owned();
        repo.record_prediction(&PredictionRecord {
            sample_id: sample.id.clone(),
            artifact_id: artifact.id.clone(),
            predicted_label: (index != 0).then(|| label.clone()),
            teacher_label: None,
            actual_label: Some(label),
            actual_source: Some(LabelSource::Reviewed),
            latency_ms: 1.,
            failed: index == 0,
            recorded_at_ms: 1100,
            details: json!({}),
        })
        .unwrap();
    }
    let report = repo.evaluate_classification(&artifact.id, 1101).unwrap();
    for metric in [
        "macro_f1",
        "balanced_accuracy",
        "mean_rank_error",
        "quadratic_kappa",
    ] {
        assert!(report.metrics.contains_key(metric), "{metric} was dropped");
    }
    assert!(report.metrics["balanced_accuracy"] < 1.0);
}

#[test]
fn received_ledgers_with_foreign_schema_objects_are_rejected() {
    for statement in [
        "CREATE TRIGGER inflate AFTER INSERT ON evaluations BEGIN UPDATE evaluations SET body=json_set(body,'$.metrics.accuracy',0.99) WHERE id=NEW.id; END",
        "CREATE VIEW audited AS SELECT body FROM evaluations",
        "CREATE TABLE smuggled(value TEXT)",
    ] {
        let (_dir, repo) = repo();
        let path = repo.path().to_owned();
        drop(repo);
        rusqlite::Connection::open(&path)
            .unwrap()
            .execute_batch(statement)
            .unwrap();
        assert!(
            matches!(TrainingRepository::open(&path), Err(Error::Invalid(_))),
            "{statement}"
        );
    }
}

#[cfg(unix)]
#[test]
fn derived_storage_paths_cannot_be_redirected_through_symlinks() {
    let outside = tempfile::tempdir().unwrap();
    for entry in [
        "training.artifacts",
        ".training-locks",
        "training.sqlite-wal",
    ] {
        let directory = tempfile::tempdir().unwrap();
        std::os::unix::fs::symlink(outside.path(), directory.path().join(entry)).unwrap();
        assert!(
            matches!(
                TrainingRepository::open(directory.path().join("training.sqlite")),
                Err(Error::Invalid(_))
            ),
            "{entry}"
        );
    }
    assert!(std::fs::read_dir(outside.path()).unwrap().next().is_none());
}

/// Publishes an epoch checkpoint and then exhausts device memory; a resumed run must start
/// from that checkpoint.
struct ExhaustsDeviceMemory {
    panics: bool,
}
impl TrainingEngine for ExhaustsDeviceMemory {
    fn estimated_memory_bytes(&self, _: &TrainingWork) -> Result<u64> {
        Ok(1)
    }
    fn train(&self, work: &TrainingWork, control: &WorkerControl) -> Result<EngineOutput> {
        if let Some(checkpoint) = &work.resume_checkpoint {
            if checkpoint.as_slice() != b"epoch-1" {
                return Err(Error::Engine("resumed from another checkpoint".into()));
            }
            return Ok(EngineOutput {
                model_bytes: b"resumed-model".to_vec(),
                manifest: json!({}),
            });
        }
        control.checkpoint(b"epoch-1", 1, json!({"epoch":1}))?;
        if self.panics {
            panic!("wgpu error: Out of Memory");
        }
        Err(Error::Engine(
            "Backend unavailable: out of device memory; reduce the batch size or input resolution: out of device memory allocating 1073741824 bytes".into(),
        ))
    }
}

#[test]
fn device_memory_exhaustion_after_a_checkpoint_leaves_the_run_resumable() {
    for panics in [true, false] {
        let (_dir, repo) = repo();
        populate(&repo, 0..12);
        let job = queued(&repo);
        let worker = fixture_worker(&repo, ExhaustsDeviceMemory { panics });
        let error = worker.run(&job.id).unwrap_err().to_string();
        assert!(
            error.contains("out of device memory; reduce the batch size"),
            "{error}"
        );
        let interrupted = repo.get_job(&job.id).unwrap();
        assert_eq!(interrupted.status, JobStatus::Interrupted);
        assert!(interrupted.lease_owner.is_none() && interrupted.lease_expires_at_ms.is_none());
        assert!(
            interrupted
                .error
                .as_deref()
                .is_some_and(|reason| reason.contains("out of device memory")),
            "{:?}",
            interrupted.error
        );
        assert_eq!(
            repo.read_blob(&interrupted.checkpoint.unwrap().blob)
                .unwrap(),
            b"epoch-1"
        );
        let reservations: i64 = rusqlite::Connection::open(repo.path())
            .unwrap()
            .query_row("SELECT count(*) FROM resource_reservations", [], |row| {
                row.get(0)
            })
            .unwrap();
        assert_eq!(reservations, 0, "a stopped run must not hold device memory");
        let artifact = worker.run(&job.id).unwrap();
        assert_eq!(repo.read_blob(&artifact.blob).unwrap(), b"resumed-model");
        let resumed = repo.get_job(&job.id).unwrap();
        assert_eq!(resumed.status, JobStatus::Succeeded);
        assert!(resumed.error.is_none());
    }
}

#[test]
fn experiment_trials_fail_on_device_memory_exhaustion_for_their_controller() {
    let (_dir, repo) = repo();
    populate(&repo, 0..12);
    let job = queued(&repo);
    rusqlite::Connection::open(repo.path())
        .unwrap()
        .execute(
            "INSERT INTO experiment_trials(id,experiment_id,job_id,idempotency_key,candidate_index,body) VALUES ('trial','experiment',?,'key',0,'{}')",
            [&job.id],
        )
        .unwrap();
    let error = fixture_worker(&repo, ExhaustsDeviceMemory { panics: true })
        .run(&job.id)
        .unwrap_err()
        .to_string();
    assert!(error.contains("out of device memory"), "{error}");
    let failed = repo.get_job(&job.id).unwrap();
    assert_eq!(failed.status, JobStatus::Failed);
    assert!(
        failed
            .error
            .unwrap()
            .contains("out of device memory; reduce the batch size")
    );
}

#[test]
fn queued_jobs_are_admitted_under_reconfigured_compute() {
    let (_dir, repo) = repo();
    populate(&repo, 0..12);
    let job = queued(&repo);
    let engine = || FixtureEngine {
        cancel: None,
        memory: 2_000_000,
    };
    assert!(matches!(
        fixture_worker(&repo, engine()).run(&job.id),
        Err(Error::Engine(_))
    ));
    let waiting = repo.get_job(&job.id).unwrap();
    assert!(waiting.error.is_some());
    assert!(matches!(
        repo.reconfigure_job_compute(&job.id, json!(4_000_000), 1100),
        Err(Error::Invalid(_))
    ));
    assert!(matches!(
        repo.reconfigure_job_compute("missing", json!({}), 1100),
        Err(Error::NotFound(_))
    ));
    let compute = json!({"memory_limit_bytes":4_000_000});
    let reconfigured = repo
        .reconfigure_job_compute(&job.id, compute.clone(), 1100)
        .unwrap();
    assert_eq!(reconfigured.status, JobStatus::Queued);
    assert_eq!(reconfigured.request.compute, compute);
    assert_eq!(reconfigured.generation, waiting.generation + 1);
    assert!(reconfigured.error.is_none());
    assert_eq!(repo.get_job(&job.id).unwrap(), reconfigured);
    fixture_worker(&repo, engine()).run(&job.id).unwrap();
    let finished = repo.get_job(&job.id).unwrap();
    assert_eq!(finished.status, JobStatus::Succeeded);
    assert_eq!(finished.request.compute, compute);
}

#[test]
fn experiment_trials_and_active_or_finished_jobs_keep_their_compute() {
    let (_dir, repo) = repo();
    populate(&repo, 0..12);
    let compute = json!({"memory_limit_bytes":4_000_000});
    let trial = queued(&repo);
    rusqlite::Connection::open(repo.path())
        .unwrap()
        .execute(
            "INSERT INTO experiment_trials(id,experiment_id,job_id,idempotency_key,candidate_index,body) VALUES ('trial','experiment',?,'key',0,'{}')",
            [&trial.id],
        )
        .unwrap();
    assert!(matches!(
        repo.reconfigure_job_compute(&trial.id, compute.clone(), 1100),
        Err(Error::Conflict(message)) if message.contains("experiment trial")
    ));
    assert_eq!(repo.get_job(&trial.id).unwrap(), trial);

    let other = StreamKey {
        stream_id: "line-2".into(),
        ..stream()
    };
    for i in 0..12 {
        repo.record_sample(&other, &sample(i)).unwrap();
    }
    let job = repo
        .trigger_after_n(&other, 12, request(), policy(), 1000)
        .unwrap()
        .unwrap();
    let refused = |at_ms| {
        matches!(
            repo.reconfigure_job_compute(&job.id, compute.clone(), at_ms),
            Err(Error::Conflict(_))
        )
    };
    let lease = repo.claim_job(&job.id, "worker", 1000, 1000).unwrap();
    assert!(refused(1100), "running");
    repo.request_cancel(&job.id, 1200).unwrap();
    assert!(refused(1300), "cancel requested");
    repo.acknowledge_cancel(&lease, 1400).unwrap();
    assert!(refused(1500), "cancelled");
    assert_eq!(repo.get_job(&job.id).unwrap().request, request());
}

#[test]
fn reconfigured_compute_refreshes_only_a_checkpoint_of_the_previous_request() {
    let (_dir, repo) = repo();
    let worker = fixture_worker(&repo, ExhaustsDeviceMemory { panics: false });
    let compute = json!({"memory_limit_bytes":4_000_000});
    let interrupted = |stream: &StreamKey| {
        for i in 0..12 {
            repo.record_sample(stream, &sample(i)).unwrap();
        }
        let job = repo
            .trigger_after_n(stream, 12, request(), policy(), 1000)
            .unwrap()
            .unwrap();
        assert!(worker.run(&job.id).is_err());
        let job = repo.get_job(&job.id).unwrap();
        assert_eq!(job.status, JobStatus::Interrupted);
        job
    };

    let current = interrupted(&stream());
    let written = current.checkpoint.clone().unwrap();
    let reconfigured = repo
        .reconfigure_job_compute(&current.id, compute.clone(), now_ms())
        .unwrap();
    assert_eq!(reconfigured.status, JobStatus::Interrupted);
    assert!(reconfigured.error.is_none());
    let refreshed = reconfigured.checkpoint.unwrap();
    assert_eq!(refreshed.blob, written.blob);
    assert_ne!(refreshed.request_digest, written.request_digest);
    let artifact = worker.run(&current.id).unwrap();
    assert_eq!(repo.read_blob(&artifact.blob).unwrap(), b"resumed-model");

    let stale = interrupted(&StreamKey {
        stream_id: "line-2".into(),
        ..stream()
    });
    rusqlite::Connection::open(repo.path())
        .unwrap()
        .execute(
            "UPDATE jobs SET body=json_set(body,'$.checkpoint.request_digest','foreign') WHERE id=?",
            [&stale.id],
        )
        .unwrap();
    let reconfigured = repo
        .reconfigure_job_compute(&stale.id, compute, now_ms())
        .unwrap();
    assert_eq!(reconfigured.checkpoint.unwrap().request_digest, "foreign");
    let error = worker.run(&stale.id).unwrap_err().to_string();
    assert!(
        error.contains("checkpoint recipe/compute contract changed"),
        "{error}"
    );
}

#[test]
fn new_sample_counts_follow_triggers_and_label_availability() {
    let (_dir, repo) = repo();
    let mut delayed = sample(0);
    delayed.label_available_at_ms = 2000;
    repo.record_sample(&stream(), &delayed).unwrap();
    populate(&repo, 1..13);
    let mut withdrawn = sample(1);
    withdrawn.annotation_revision = 2;
    withdrawn.accepted = false;
    withdrawn.label_available_at_ms = 1500;
    repo.record_sample(&stream(), &withdrawn).unwrap();
    let count = |at_ms| repo.count_new_training_samples(&stream(), at_ms).unwrap();
    assert_eq!((count(1000), count(1500), count(2000)), (12, 11, 12));
    assert!(
        repo.trigger_after_n(&stream(), 13, request(), policy(), 1000)
            .unwrap()
            .is_none()
    );
    assert!(
        repo.trigger_after_n(&stream(), 12, request(), policy(), 1000)
            .unwrap()
            .is_some()
    );
    assert_eq!((count(1000), count(2000)), (0, 1));
}

#[test]
fn raw_row_edits_keep_sealed_audit_evidence_valid() {
    let (_dir, repo) = repo();
    for i in 0..12 {
        let mut record = sample(i);
        record.payload["feature_row"] = json!({"temperature":i});
        repo.record_sample(&stream(), &record).unwrap();
    }
    let artifact = candidate(&repo);
    let report = evaluate(&repo, &artifact);
    let mut edited = repo.get_snapshot(&artifact.snapshot_id).unwrap().test[0].clone();
    edited.payload["feature_row"] = json!({"temperature":-1});
    assert!(
        !repo
            .record_prepared_sample(&stream(), &edited)
            .unwrap()
            .inserted
    );
    let stored: i64 = rusqlite::Connection::open(repo.path())
        .unwrap()
        .query_row(
            "SELECT json_extract(body,'$.payload.feature_row.temperature') FROM annotations WHERE sample_id=?",
            [&edited.id],
            |row| row.get(0),
        )
        .unwrap();
    assert_eq!(stored, -1, "the edit rewrote the stored annotation");
    repo.promote("production", 0, &report.id, promotion_policy(), 1200)
        .unwrap();
}

#[cfg(feature = "native")]
#[test]
fn classification_audits_reject_content_an_initialization_model_consumed() {
    let (_dir, repo) = repo();
    populate(&repo, 0..12);
    let source = candidate(&repo);
    let consumed = repo.get_snapshot(&source.snapshot_id).unwrap();
    let warm = StreamKey {
        stream_id: "line-2".into(),
        ..stream()
    };
    for (index, original) in consumed
        .train
        .iter()
        .chain(&consumed.validation)
        .enumerate()
    {
        repo.record_sample(
            &warm,
            &TrainingSample {
                id: format!("warm-{index}"),
                group_id: format!("warm-lot-{index}"),
                ..original.clone()
            },
        )
        .unwrap();
    }
    let job = repo
        .trigger_after_n(&warm, 3, request(), policy(), 1000)
        .unwrap()
        .unwrap();
    let lease = repo.claim_job(&job.id, "worker", 1000, 1000).unwrap();
    let fine_tuned = repo
        .finish_training(
            &lease,
            b"fine-tuned-model",
            json!({"pretrained":{"source_kind":"training_artifact","source_id":source.id}}),
            1001,
        )
        .unwrap();
    let audit = repo.get_snapshot(&fine_tuned.snapshot_id).unwrap().test;
    assert!(!audit.is_empty());
    for sample in &audit {
        let label = sample.payload["label"].as_str().unwrap().to_owned();
        repo.record_prediction(&PredictionRecord {
            sample_id: sample.id.clone(),
            artifact_id: fine_tuned.id.clone(),
            predicted_label: Some(label.clone()),
            teacher_label: None,
            actual_label: Some(label),
            actual_source: Some(LabelSource::Reviewed),
            latency_ms: 1.,
            failed: false,
            recorded_at_ms: 1100,
            details: json!({}),
        })
        .unwrap();
    }
    assert!(matches!(
        repo.evaluate_classification(&fine_tuned.id, 1101),
        Err(Error::Invalid(message)) if message.contains("a model it was initialized from")
    ));
}

#[cfg(all(feature = "native", feature = "burn"))]
#[test]
fn worker_cli_refuses_to_create_a_missing_ledger() {
    let directory = tempfile::tempdir().unwrap();
    let missing = directory.path().join("unmounted").join("jobs.sqlite");
    let output = std::process::Command::new(env!("CARGO_BIN_EXE_flow-like-ml-worker"))
        .arg("--repository")
        .arg(&missing)
        .arg("--list-pending")
        .output()
        .unwrap();
    assert!(!output.status.success());
    assert!(!directory.path().join("unmounted").exists());
}
