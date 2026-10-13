#![cfg(all(feature = "native", feature = "burn"))]
use flow_like_ml_core::{
    Annotation, ComputeConfig, LabelProvenance, PreprocessingStep, Sample, TensorData,
};
use flow_like_ml_runtime::{engines::*, worker::TrainingWorker, *};
use serde_json::json;
use std::sync::Arc;

fn populated() -> (tempfile::TempDir, TrainingRepository, StreamKey) {
    populated_with(|x| {
        (
            TensorData {
                shape: vec![1],
                values: vec![x],
            },
            Annotation::Scalar {
                value: 2. * x as f64,
            },
        )
    })
}

fn populated_with(
    sample_at: impl Fn(f32) -> (TensorData, Annotation),
) -> (tempfile::TempDir, TrainingRepository, StreamKey) {
    let directory = tempfile::tempdir().unwrap();
    let repository = TrainingRepository::open(directory.path().join("training.sqlite")).unwrap();
    let stream = StreamKey {
        project_id: "factory".into(),
        stream_id: "sensor".into(),
        inspection_version: "v1".into(),
    };
    for i in 0..30 {
        let (input, annotation) = sample_at((i as f32 - 15.) / 15.);
        let sample = Sample {
            id: format!("sample-{i}"),
            group_id: format!("batch-{i}"),
            stream_id: stream.stream_id.clone(),
            timestamp_ms: i,
            window_start_ms: i,
            window_end_ms: i,
            input,
            annotation,
            provenance: LabelProvenance::Reviewed {
                reviewer: "operator".into(),
                reviewed_at_ms: i,
            },
            outcome: None,
        };
        repository
            .record_sample(
                &stream,
                &TrainingSample {
                    id: sample.id.clone(),
                    annotation_revision: 1,
                    group_id: sample.group_id.clone(),
                    captured_at_ms: i,
                    label_available_at_ms: i,
                    content_digest: format!("{i:064x}"),
                    source: LabelSource::Reviewed,
                    accepted: true,
                    payload: serde_json::to_value(sample).unwrap(),
                },
            )
            .unwrap();
    }
    (directory, repository, stream)
}
fn split() -> SplitPolicy {
    SplitPolicy::Group {
        train_fraction: 0.6,
        validation_fraction: 0.2,
        seed: 7,
    }
}

/// 0.1 has no exact f32 form, so loading must not compare config JSON text.
fn mlp_recipe() -> serde_json::Value {
    json!({"config":{"recipe":{"architecture":"mlp","input_features":1,"hidden":8,"outputs":1,"objective":"regression"},"backend":{"backend":"cpu"},"epochs":12,"batch_size":6,"learning_rate":0.04,"seed":42,"gradient_clip":0.1},"labels":[]})
}

/// The smallest valid PDN-small teacher: one base and one feature channel.
fn efficient_ad_recipe() -> serde_json::Value {
    let convolution = |shape: [usize; 4]| {
        let weight = vec![0.01; shape.iter().product()];
        let bias = vec![0.01; shape[0]];
        json!({"weight":{"shape":shape,"values":weight},"bias":{"shape":[shape[0]],"values":bias}})
    };
    let convolutions: Vec<serde_json::Value> =
        [[1, 1, 4, 4], [2, 1, 4, 4], [2, 2, 3, 3], [1, 2, 4, 4]]
            .into_iter()
            .map(convolution)
            .collect();
    json!({"config":{"teacher":{"input_channels":1,"base_channels":1,"feature_channels":1,"convolutions":convolutions},"epochs":1,"batch_size":1,"learning_rate":0.001,"seed":42,"autoencoder_channels":2,"hard_quantile":0.99,"gradient_clip":5.0},"labels":[]})
}

#[test]
fn native_worker_trains_a_model_that_roundtrips_through_registry() {
    let (_directory, repository, stream) = populated();
    let request = TrainingRequest {
        engine: "histogram_gradient_boosting".into(),
        recipe: json!({"config":{"objective":"squared_error","estimators":40,"max_depth":3,"max_bins":16,"min_leaf":1,"learning_rate":0.15,"l2":1.},"labels":[]}),
        compute: serde_json::to_value(ComputeConfig::default()).unwrap(),
    };
    let job = repository
        .trigger_after_n(&stream, 30, request, split(), 1000)
        .unwrap()
        .unwrap();
    let mut worker = TrainingWorker::new(repository.clone(), WorkerLimits::default()).unwrap();
    register_native_engines(&mut worker).unwrap();
    let artifact = worker.run(&job.id).unwrap();
    let output = load_artifact_predictions(
        &repository,
        &artifact.id,
        &TensorData {
            shape: vec![2, 1],
            values: vec![-0.8, 0.8],
        },
        &ComputeConfig::default(),
    )
    .unwrap();
    let scores = output["scores"].as_array().unwrap();
    assert!(scores[0].as_f64().unwrap() < -1.0);
    assert!(scores[1].as_f64().unwrap() > 1.0);
    assert_eq!(
        repository.get_job(&job.id).unwrap().status,
        JobStatus::Succeeded
    );
}

/// Request cancellation from inside the run, right after its first durable epoch checkpoint.
fn cancel_after_first_checkpoint(repository: &TrainingRepository) -> CheckpointObserver {
    let repository = repository.clone();
    Arc::new(move |job: &TrainingJob, _step: u64| {
        repository.request_cancel(&job.id, now_ms()).unwrap();
    })
}

#[test]
fn burn_worker_resumes_optimizer_checkpoint_and_loads_prediction_artifact() {
    let (_directory, repository, stream) = populated();
    let request = TrainingRequest {
        engine: "burn".into(),
        recipe: mlp_recipe(),
        compute: serde_json::to_value(ComputeConfig::default()).unwrap(),
    };
    let job = repository
        .trigger_after_n(&stream, 30, request, split(), 1000)
        .unwrap()
        .unwrap();
    let mut cancelling = TrainingWorker::new(repository.clone(), WorkerLimits::default()).unwrap();
    register_observed_burn_engine(&mut cancelling, cancel_after_first_checkpoint(&repository))
        .unwrap();
    assert!(matches!(cancelling.run(&job.id), Err(Error::Cancelled)));
    let stopped = repository.get_job(&job.id).unwrap();
    assert_eq!(stopped.status, JobStatus::Cancelled);
    assert!(
        stopped
            .checkpoint
            .as_ref()
            .is_some_and(|checkpoint| checkpoint.step > 0),
        "last completed Burn batch must be retained"
    );
    repository.resume_job(&job.id, now_ms()).unwrap();
    let mut worker = TrainingWorker::new(repository.clone(), WorkerLimits::default()).unwrap();
    register_burn_engine(&mut worker).unwrap();
    let artifact = worker.run(&job.id).unwrap();
    let output = load_artifact_predictions(
        &repository,
        &artifact.id,
        &TensorData {
            shape: vec![2, 1],
            values: vec![-0.8, 0.8],
        },
        &ComputeConfig::default(),
    )
    .unwrap();
    let values = output["output"]["values"].as_array().unwrap();
    assert_eq!(values.len(), 2);
    assert!(values[0].as_f64().unwrap() < -0.5);
    assert!(values[1].as_f64().unwrap() > 0.5);
    let report = &artifact.manifest["report"];
    assert!(report["final_loss"].as_f64().unwrap() < report["initial_loss"].as_f64().unwrap());
    let checkpoint = repository.get_job(&job.id).unwrap().checkpoint.unwrap();
    assert!(
        checkpoint.blob.bytes > artifact.blob.bytes,
        "training checkpoint includes optimizer while inference artifact does not"
    );
}

/// Burn and EfficientAD charge twelve bytes of training memory per stored snapshot byte, so
/// a larger snapshot is refused from its stored size before it is parsed.
#[test]
fn vision_engines_refuse_snapshots_beyond_a_twelfth_of_the_budget_before_parsing() {
    for (engine, recipe) in [
        ("burn", mlp_recipe()),
        ("efficient_ad", efficient_ad_recipe()),
    ] {
        let (_directory, repository, stream) = populated();
        let request = TrainingRequest {
            engine: engine.into(),
            recipe,
            compute: serde_json::to_value(ComputeConfig::default()).unwrap(),
        };
        let job = repository
            .trigger_after_n(&stream, 30, request, split(), 1000)
            .unwrap()
            .unwrap();
        let stored = serde_json::to_vec(&repository.get_snapshot(&job.snapshot_id).unwrap())
            .unwrap()
            .len() as u64;
        let admit = |memory_budget_bytes: u64| {
            let limits = WorkerLimits {
                memory_budget_bytes,
                ..WorkerLimits::default()
            };
            let mut worker = TrainingWorker::new(repository.clone(), limits).unwrap();
            register_burn_engine(&mut worker).unwrap();
            register_efficient_ad_engine(&mut worker).unwrap();
            worker.run(&job.id).unwrap_err()
        };
        let refused = admit(stored * 12 - 1);
        let limit = format!("exceeds the {} byte budget", stored - 1);
        assert!(
            matches!(&refused, Error::Invalid(message) if message.contains(&limit)),
            "{engine}: {refused}"
        );
        let estimated = admit(stored * 12);
        assert!(
            matches!(&estimated, Error::Engine(message) if message.contains("needs an estimated")),
            "{engine}: {estimated}"
        );
        assert_eq!(
            repository.get_job(&job.id).unwrap().status,
            JobStatus::Queued
        );
    }
}

#[test]
fn explicit_uncompiled_native_gpu_request_is_rejected() {
    let (_directory, repository, stream) = populated();
    let request = TrainingRequest {
        engine: "isolation_forest".into(),
        recipe: json!({"config":{"trees":8,"sample_size":8,"seed":42}}),
        compute: json!({"backend":"cuda","allow_cpu_fallback":false,"memory_limit_bytes":100_000_000}),
    };
    let job = repository
        .trigger_after_n(&stream, 30, request, split(), 1000)
        .unwrap()
        .unwrap();
    let mut worker = TrainingWorker::new(repository.clone(), WorkerLimits::default()).unwrap();
    register_native_engines(&mut worker).unwrap();
    assert!(worker.run(&job.id).is_err());
    assert!(repository.get_job(&job.id).unwrap().artifact_id.is_none());
}

#[test]
fn separate_process_worker_executes_a_durable_job() {
    let (_directory, repository, stream) = populated();
    let request = TrainingRequest {
        engine: "isolation_forest".into(),
        recipe: json!({"config":{"trees":8,"sample_size":8,"seed":42}}),
        compute: serde_json::to_value(ComputeConfig::default()).unwrap(),
    };
    let job = repository
        .trigger_after_n(&stream, 30, request, split(), 1000)
        .unwrap()
        .unwrap();
    let output = std::process::Command::new(env!("CARGO_BIN_EXE_flow-like-ml-worker"))
        .arg("--repository")
        .arg(repository.path())
        .arg("--job")
        .arg(&job.id)
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let artifact: ModelArtifact = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(artifact.job_id, job.id);
    assert_eq!(
        repository.get_job(&job.id).unwrap().status,
        JobStatus::Succeeded
    );
}

#[test]
fn boosting_fits_single_step_value_targets_and_answers_in_their_dense_shape() {
    let (_directory, repository, stream) = populated_with(|x| {
        (
            TensorData {
                shape: vec![1],
                values: vec![x],
            },
            Annotation::Values {
                values: vec![2. * x],
            },
        )
    });
    let request = TrainingRequest {
        engine: "histogram_gradient_boosting".into(),
        recipe: json!({"config":{"objective":"squared_error","estimators":40,"max_depth":3,"max_bins":16,"min_leaf":1,"learning_rate":0.15,"l2":1.},"labels":[]}),
        compute: serde_json::to_value(ComputeConfig::default()).unwrap(),
    };
    let job = repository
        .trigger_after_n(&stream, 30, request, split(), 1000)
        .unwrap()
        .unwrap();
    let mut worker = TrainingWorker::new(repository.clone(), WorkerLimits::default()).unwrap();
    register_native_engines(&mut worker).unwrap();
    let artifact = worker.run(&job.id).unwrap();
    assert_eq!(artifact.manifest["output_shape"], json!([1]));
    let output = load_artifact_predictions(
        &repository,
        &artifact.id,
        &TensorData {
            shape: vec![2, 1],
            values: vec![-0.8, 0.8],
        },
        &ComputeConfig::default(),
    )
    .unwrap();
    assert_eq!(output["output"]["shape"], json!([2, 1]));
    assert_eq!(output["output"]["values"], output["scores"]);
    let values = output["output"]["values"].as_array().unwrap();
    assert!(values[0].as_f64().unwrap() < -1.0);
    assert!(values[1].as_f64().unwrap() > 1.0);
}

#[test]
fn patch_anomaly_models_record_and_verify_their_feature_extractor() {
    let (_directory, repository, stream) = populated_with(|x| {
        (
            TensorData {
                shape: vec![2, 2],
                values: vec![x, x * 0.5, -x, x * 0.25],
            },
            Annotation::Anomaly { is_anomaly: false },
        )
    });
    let extractor = FeatureExtractor {
        source_id: "student-artifact".into(),
        blob_sha256: "a".repeat(64),
        image_steps: vec![PreprocessingStep::Resize {
            width: 8,
            height: 8,
        }],
    };
    let request = TrainingRequest {
        engine: "patchcore".into(),
        recipe: json!({"labels":[],"coreset_size":8,"feature_extractor":extractor}),
        compute: serde_json::to_value(ComputeConfig::default()).unwrap(),
    };
    let job = repository
        .trigger_after_n(&stream, 30, request, split(), 1000)
        .unwrap()
        .unwrap();
    let mut worker = TrainingWorker::new(repository.clone(), WorkerLimits::default()).unwrap();
    register_native_engines(&mut worker).unwrap();
    let artifact = worker.run(&job.id).unwrap();
    assert_eq!(
        serde_json::from_value::<FeatureExtractor>(artifact.manifest["feature_extractor"].clone())
            .unwrap(),
        extractor
    );
    let predictor =
        LoadedArtifactPredictor::load(&repository, &artifact.id, &ComputeConfig::default())
            .unwrap();
    let embeddings = TensorData {
        shape: vec![2, 2],
        values: vec![0.1, 0.05, -0.1, 0.025],
    };
    assert_eq!(
        predictor
            .predict_with_extractor(&embeddings, &extractor)
            .unwrap(),
        predictor.predict(&embeddings).unwrap()
    );
    let retrained = FeatureExtractor {
        blob_sha256: "b".repeat(64),
        ..extractor.clone()
    };
    assert!(
        predictor
            .predict_with_extractor(&embeddings, &retrained)
            .unwrap_err()
            .to_string()
            .contains("recorded extractor")
    );
    let unresized = FeatureExtractor {
        image_steps: Vec::new(),
        ..extractor
    };
    assert!(
        predictor
            .predict_with_extractor(&embeddings, &unresized)
            .is_err()
    );
}
