#![cfg(all(feature = "native", feature = "burn"))]

use flow_like_ml_burn as burn;
use flow_like_ml_core::{
    Annotation, ComputeBackend, ComputeConfig, LabelProvenance, Sample, TensorData,
};
use flow_like_ml_runtime::{engines::*, worker::TrainingWorker, *};
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::{
    collections::BTreeSet,
    path::{Path, PathBuf},
    sync::{Arc, Barrier},
};

fn sha256_hex(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}

/// The reference a ledger would store for `bytes`.
fn blob_of(bytes: &[u8]) -> BlobRef {
    let sha256 = sha256_hex(bytes);
    BlobRef {
        path: format!("{sha256}.blob"),
        sha256,
        bytes: bytes.len() as u64,
    }
}

fn cpu() -> ComputeConfig {
    ComputeConfig {
        backend: ComputeBackend::Cpu,
        ..Default::default()
    }
}

fn stream(project: &str, name: &str) -> StreamKey {
    StreamKey {
        project_id: project.into(),
        stream_id: name.into(),
        inspection_version: "v1".into(),
    }
}

fn record(
    repo: &TrainingRepository,
    stream: &StreamKey,
    count: i64,
    sample_at: impl Fn(i64) -> (TensorData, Annotation),
) {
    for index in 0..count {
        let (input, annotation) = sample_at(index);
        let sample = Sample {
            id: format!("{}-{index}", stream.stream_id),
            group_id: format!("{}-group-{index}", stream.stream_id),
            stream_id: stream.stream_id.clone(),
            timestamp_ms: index,
            window_start_ms: index,
            window_end_ms: index,
            input,
            annotation,
            provenance: LabelProvenance::Reviewed {
                reviewer: "fixture".into(),
                reviewed_at_ms: index,
            },
            outcome: None,
        };
        repo.record_sample(
            stream,
            &TrainingSample {
                id: sample.id.clone(),
                annotation_revision: 1,
                group_id: sample.group_id.clone(),
                captured_at_ms: index,
                label_available_at_ms: index,
                content_digest: sha256_hex(sample.id.as_bytes()),
                source: LabelSource::Reviewed,
                accepted: true,
                payload: serde_json::to_value(sample).unwrap(),
            },
        )
        .unwrap();
    }
}

fn train(
    repo: &TrainingRepository,
    stream: &StreamKey,
    samples: usize,
    engine: &str,
    recipe: Value,
    policy: SplitPolicy,
) -> ModelArtifact {
    let job = repo
        .trigger_after_n(
            stream,
            samples,
            TrainingRequest {
                engine: engine.into(),
                recipe,
                compute: json!(cpu()),
            },
            policy,
            1000,
        )
        .unwrap()
        .unwrap();
    let mut worker = TrainingWorker::new(repo.clone(), WorkerLimits::default()).unwrap();
    register_native_engines(&mut worker).unwrap();
    register_burn_engine(&mut worker).unwrap();
    worker.run(&job.id).unwrap()
}

fn native_artifact(repo: &TrainingRepository, engine: &str, recipe: Value) -> ModelArtifact {
    let line = stream("app:board", engine);
    record(repo, &line, 30, |index| {
        let x = (index as f32 - 15.) / 15.;
        (
            TensorData {
                shape: vec![1],
                values: vec![x],
            },
            Annotation::Scalar {
                value: 2. * x as f64,
            },
        )
    });
    train(
        repo,
        &line,
        30,
        engine,
        recipe,
        SplitPolicy::Group {
            train_fraction: 0.6,
            validation_fraction: 0.2,
            seed: 7,
        },
    )
}

/// The smallest ResNet18 the feature extractors accept: two classes of 1x8x8 images.
fn resnet_artifact(repo: &TrainingRepository) -> ModelArtifact {
    let line = stream("app:board", "camera");
    record(repo, &line, 12, |index| {
        (
            TensorData {
                shape: vec![1, 8, 8],
                values: vec![index as f32 / 12.; 64],
            },
            Annotation::Class {
                class_id: index as u32 % 2,
            },
        )
    });
    let config = burn::TrainingConfig {
        recipe: burn::Recipe::ResNet18 {
            input_channels: 1,
            classes: 2,
            base_channels: 2,
        },
        backend: burn::BackendChoice::Cpu,
        epochs: 1,
        batch_size: 2,
        learning_rate: 0.001,
        seed: 17,
        gradient_clip: 0.1,
    };
    train(
        repo,
        &line,
        12,
        "burn",
        json!({"config":config,"labels":["camera-0","camera-1"],"pretrained":null,"preprocessing":[]}),
        SplitPolicy::Time {
            train_end_ms: 5,
            validation_end_ms: 8,
            embargo_ms: 0,
        },
    )
}

fn burn_manifest() -> Value {
    json!({"engine":"burn","format_version":1,"config":{},"input_shape":[1]})
}

/// A ledger at `path` with a sample stream, a finished job and its artifact, and an imported
/// pretrained source, all under `project`.
fn populated_ledger(
    path: &Path,
    project: &str,
) -> (TrainingRepository, ModelArtifact, PretrainedSource) {
    let repo = TrainingRepository::open(path).unwrap();
    let line = stream(project, "line");
    for index in 0..6 {
        repo.record_sample(
            &line,
            &TrainingSample {
                id: format!("sample-{index}"),
                annotation_revision: 1,
                group_id: format!("lot-{index}"),
                captured_at_ms: index,
                label_available_at_ms: index,
                content_digest: format!("{index:064x}"),
                source: LabelSource::Reviewed,
                accepted: true,
                payload: json!({"label":"normal"}),
            },
        )
        .unwrap();
    }
    let job = repo
        .trigger_after_n(
            &line,
            6,
            TrainingRequest {
                engine: "fixture".into(),
                recipe: json!({}),
                compute: json!({}),
            },
            SplitPolicy::Group {
                train_fraction: 0.5,
                validation_fraction: 0.25,
                seed: 7,
            },
            now_ms(),
        )
        .unwrap()
        .unwrap();
    let lease = repo.claim_job(&job.id, "worker", 60_000, now_ms()).unwrap();
    let artifact = repo
        .finish_training(&lease, b"portable model bytes", burn_manifest(), now_ms())
        .unwrap();
    let source = repo
        .register_pretrained_source(
            project,
            b"imported weights",
            burn_manifest(),
            json!({"source":"fixture"}),
            1024,
            now_ms(),
        )
        .unwrap();
    (repo, artifact, source)
}

fn companion(database: &Path, suffix: &str) -> PathBuf {
    let mut path = database.as_os_str().to_owned();
    path.push(suffix);
    PathBuf::from(path)
}

fn names(directory: &Path) -> BTreeSet<String> {
    std::fs::read_dir(directory)
        .unwrap()
        .map(|entry| entry.unwrap().file_name().into_string().unwrap())
        .collect()
}

fn contains(haystack: &[u8], needle: &[u8]) -> bool {
    haystack.windows(needle.len()).any(|window| window == needle)
}

fn invalid_containing<T: std::fmt::Debug>(result: Result<T>, text: &str) {
    assert!(
        matches!(&result, Err(Error::Invalid(message)) if message.contains(text)),
        "expected an invalid-input error containing {text:?}, got {result:?}"
    );
}

#[test]
fn native_models_load_from_parts_like_the_ledger_and_check_bytes_before_decoding() {
    let directory = tempfile::tempdir().unwrap();
    let repo = TrainingRepository::open(directory.path().join(LEDGER_DATABASE_FILE)).unwrap();
    let compute = cpu();
    let input = TensorData {
        shape: vec![2, 1],
        values: vec![-0.8, 0.8],
    };
    for (engine, recipe) in [
        (
            "isolation_forest",
            json!({"config":{"trees":8,"sample_size":8,"seed":42}}),
        ),
        (
            "histogram_gradient_boosting",
            json!({"config":{"objective":"squared_error","estimators":40,"max_depth":3,"max_bins":16,"min_leaf":1,"learning_rate":0.15,"l2":1.0},"labels":[]}),
        ),
    ] {
        let artifact = native_artifact(&repo, engine, recipe);
        let bytes = repo.read_blob(&artifact.blob).unwrap();
        let loaded = LoadedArtifactPredictor::load(&repo, &artifact.id, &compute).unwrap();
        let portable =
            LoadedArtifactPredictor::from_parts(artifact.clone(), &bytes, &compute).unwrap();
        assert_eq!(
            portable.predict(&input).unwrap(),
            loaded.predict(&input).unwrap(),
            "{engine}"
        );
        assert_eq!(portable.artifact(), &artifact);
        assert_eq!(portable.compute(), loaded.compute());
        assert_eq!(portable.requested_compute(), loaded.requested_compute());

        // Both loaders cap the blob at a quarter of the memory budget.
        let tight = ComputeConfig {
            memory_limit_bytes: artifact.blob.bytes * 4 - 1,
            ..compute.clone()
        };
        invalid_containing(
            LoadedArtifactPredictor::load(&repo, &artifact.id, &tight).map(|_| ()),
            "exceeds the configured byte limit",
        );
        invalid_containing(
            LoadedArtifactPredictor::from_parts(artifact.clone(), &bytes, &tight).map(|_| ()),
            "exceeds the configured byte limit",
        );
        invalid_containing(
            LoadedArtifactPredictor::from_parts(
                artifact.clone(),
                &bytes[..bytes.len() - 1],
                &compute,
            )
            .map(|_| ()),
            "digest or size mismatch",
        );

        // Bytes that pass the checks reach the decoder, so the refusals below happen before it.
        let garbage = b"not a serialized model".to_vec();
        let with_blob = |blob: BlobRef| ModelArtifact {
            blob,
            ..artifact.clone()
        };
        let decoded =
            LoadedArtifactPredictor::from_parts(with_blob(blob_of(&garbage)), &garbage, &compute);
        assert!(matches!(decoded, Err(Error::Json(_))), "{engine}");
        let mut wrong_sha = blob_of(b"other content");
        wrong_sha.bytes = garbage.len() as u64;
        invalid_containing(
            LoadedArtifactPredictor::from_parts(with_blob(wrong_sha), &garbage, &compute)
                .map(|_| ()),
            "digest or size mismatch",
        );
        let mut wrong_length = blob_of(&garbage);
        wrong_length.bytes += 1;
        invalid_containing(
            LoadedArtifactPredictor::from_parts(with_blob(wrong_length), &garbage, &compute)
                .map(|_| ()),
            "digest or size mismatch",
        );
        let too_small = ComputeConfig {
            memory_limit_bytes: garbage.len() as u64 * 4 - 1,
            ..compute.clone()
        };
        let refused =
            LoadedArtifactPredictor::from_parts(with_blob(blob_of(&garbage)), &garbage, &too_small);
        invalid_containing(refused.map(|_| ()), "exceeds the configured byte limit");
    }
}

#[test]
fn verified_bytes_must_be_named_by_their_own_sha256() {
    let bytes = b"model bytes".to_vec();
    let blob = blob_of(&bytes);
    verify_artifact_bytes(&blob, &bytes, bytes.len() as u64).unwrap();
    for path in [
        "model.blob".to_owned(),
        format!("{}.bin", blob.sha256),
        format!("../{}.blob", blob.sha256),
        format!("{}.blob", blob.sha256.to_uppercase()),
    ] {
        invalid_containing(
            verify_artifact_bytes(
                &BlobRef {
                    path,
                    ..blob.clone()
                },
                &bytes,
                u64::MAX,
            ),
            "invalid repository blob reference",
        );
    }
    invalid_containing(
        verify_artifact_bytes(&blob, &bytes, bytes.len() as u64 - 1),
        "exceeds the configured byte limit",
    );
    invalid_containing(
        verify_artifact_bytes(&blob_of(b""), b"", u64::MAX),
        "records no bytes",
    );
    let mut flipped = bytes.clone();
    flipped[0] ^= 1;
    invalid_containing(
        verify_artifact_bytes(&blob, &flipped, u64::MAX),
        "digest or size mismatch",
    );
}

#[test]
fn pretrained_features_from_parts_match_the_ledger_without_a_project_check() {
    let directory = tempfile::tempdir().unwrap();
    let repo = TrainingRepository::open(directory.path().join(LEDGER_DATABASE_FILE)).unwrap();
    let artifact = resnet_artifact(&repo);
    let bytes = repo.read_blob(&artifact.blob).unwrap();
    let imported = repo
        .register_pretrained_source(
            "app:board",
            &bytes,
            artifact.manifest.clone(),
            json!({"source":"fixture weights"}),
            bytes.len() as u64,
            now_ms(),
        )
        .unwrap();
    let compute = cpu();
    let input = TensorData {
        shape: vec![2, 1, 8, 8],
        values: (0..128).map(|index| index as f32 / 128.).collect(),
    };
    let expected =
        pretrained_source_features(&repo, "app:board", &imported.id, &input, &compute).unwrap();
    let foreign = PretrainedSource {
        project_id: "another-app:board".into(),
        ..imported.clone()
    };
    for source in [
        imported.clone(),
        foreign,
        repo.get_pretrained_source(&artifact.id).unwrap(),
    ] {
        let portable = pretrained_features_from_parts(&source, &bytes, &input, &compute).unwrap();
        assert_eq!(portable.shape, expected.shape, "{}", source.id);
        assert_eq!(portable.values, expected.values, "{}", source.id);
    }
    let mut corrupt = bytes.clone();
    *corrupt.last_mut().unwrap() ^= 1;
    invalid_containing(
        pretrained_features_from_parts(&imported, &corrupt, &input, &compute),
        "digest or size mismatch",
    );
    invalid_containing(
        pretrained_features_from_parts(&imported, &bytes[1..], &input, &compute),
        "digest or size mismatch",
    );
    let sam = PretrainedSource {
        manifest: json!({"engine":"sam2","format_version":1}),
        ..imported.clone()
    };
    invalid_containing(
        pretrained_features_from_parts(&sam, &bytes, &input, &compute),
        "format or byte budget",
    );
    let tight = ComputeConfig {
        memory_limit_bytes: imported.blob.bytes * 4 - 1,
        ..compute.clone()
    };
    invalid_containing(
        pretrained_features_from_parts(&imported, &bytes, &input, &tight),
        "format or byte budget",
    );
}

/// The placeholder SAM 2 bundle of the adapter's unit tests: valid metadata, and weights
/// that are not a record, so any prediction stops at the model loader.
fn write_sam2_bundle(directory: &Path) {
    std::fs::create_dir_all(directory.join("sam2-checkpoint-test")).unwrap();
    std::fs::write(
        directory.join("sam2-checkpoint-test/model.bpk"),
        b"test-model-bytes",
    )
    .unwrap();
    let state = json!({
        "version":1, "model":{"variant":"hiera_tiny"},
        "provenance": {"metadata":{"source":"test://sam2","license":"Apache-2.0"}, "format":"safetensors_f32", "sha256":"a".repeat(64)},
        "source_model_sha256":null, "training":null, "dataset_sha256":null,
        "checkpoint":"sam2-checkpoint-test", "next_epoch":0, "next_batch":0,
        "report":{"completed_epochs":0,"steps":0,"initial_loss":0.0,"final_loss":0.0,"validation_loss":null,"history":[]}
    });
    std::fs::write(
        directory.join(burn::SAM2_MANIFEST_FILE),
        serde_json::to_vec(&state).unwrap(),
    )
    .unwrap();
}

#[test]
fn sam2_predictions_from_parts_match_the_ledger_at_every_stage() {
    let root = tempfile::tempdir().unwrap();
    let repo = TrainingRepository::open(root.path().join(LEDGER_DATABASE_FILE)).unwrap();
    let bundle = root.path().join("bundle");
    write_sam2_bundle(&bundle);
    let source = publish_sam2_source(
        &repo,
        "app:board",
        &bundle,
        json!({"source":"test"}),
        1024 * 1024,
        10,
    )
    .unwrap();
    let bytes = repo.read_blob(&source.blob).unwrap();
    let image = TensorData {
        shape: vec![1, 3, 1024, 1024],
        values: vec![0.; 3 * 1024 * 1024],
    };
    let prompts = [burn::Sam2Prompt::default()];
    let budget = |memory_limit_bytes: u64| ComputeConfig {
        memory_limit_bytes,
        ..cpu()
    };
    let roomy = budget(2 * 1024 * 1024 * 1024);
    let same_outcome = |portable_source: &PretrainedSource,
                        images: &TensorData,
                        prompts: &[burn::Sam2Prompt],
                        compute: &ComputeConfig| {
        let ledger = predict_sam2_source(&repo, "app:board", &source.id, images, prompts, compute)
            .map_err(|error| error.to_string());
        let portable = predict_sam2_from_parts(portable_source, &bytes, images, prompts, compute)
            .map_err(|error| error.to_string());
        assert_eq!(portable, ledger);
        ledger.unwrap_err()
    };
    assert!(same_outcome(&source, &image, &[], &roomy).contains("one prompt per input image"));
    assert!(
        same_outcome(&source, &image, &prompts, &budget(32 * 1024 * 1024))
            .contains("SAM needs an estimated")
    );
    let channels_last = TensorData {
        shape: vec![1024, 1024, 3],
        values: image.values.clone(),
    };
    assert!(same_outcome(&source, &channels_last, &prompts, &roomy).contains("image input contract"));
    // Both paths pass every check and reach the placeholder weights; the portable one never
    // looks at the project.
    let foreign = PretrainedSource {
        project_id: "another-app:board".into(),
        ..source.clone()
    };
    for portable_source in [&source, &foreign] {
        let loading = same_outcome(portable_source, &image, &prompts, &roomy);
        for check in ["one prompt", "SAM needs", "contract", "differs", "mismatch"] {
            assert!(!loading.contains(check), "{loading}");
        }
    }
    let mut corrupt = bytes.clone();
    *corrupt.last_mut().unwrap() ^= 1;
    invalid_containing(
        predict_sam2_from_parts(&source, &corrupt, &image, &prompts, &roomy),
        "digest or size mismatch",
    );
    let mut forged = source.clone();
    forged.manifest["provenance"]["sha256"] = json!("b".repeat(64));
    invalid_containing(
        predict_sam2_from_parts(&forged, &bytes, &image, &prompts, &roomy),
        "differs from its verified model bundle",
    );
}

#[test]
fn snapshots_hold_wal_commits_and_every_referenced_blob_in_one_database_file() {
    let root = tempfile::tempdir().unwrap();
    let (repo, artifact, source) = populated_ledger(
        &root.path().join("ledger").join(LEDGER_DATABASE_FILE),
        "app:board",
    );
    // An open reader keeps later commits in the WAL: no connection can checkpoint on close.
    let keeper = rusqlite::Connection::open(repo.path()).unwrap();
    keeper
        .query_row("SELECT count(*) FROM ml_meta", [], |row| row.get::<_, i64>(0))
        .unwrap();
    let late = repo
        .register_pretrained_source(
            "app:board",
            b"weights committed to the WAL",
            burn_manifest(),
            json!({"source":"late"}),
            1024,
            now_ms(),
        )
        .unwrap();
    assert!(std::fs::metadata(companion(repo.path(), "-wal")).unwrap().len() > 0);
    assert!(
        !contains(&std::fs::read(repo.path()).unwrap(), late.id.as_bytes()),
        "the late source is committed only to the WAL"
    );

    let target = root.path().join("snapshot");
    let snapshot = repo.snapshot_to(&target).unwrap();
    drop(keeper);
    let database = target.join(LEDGER_DATABASE_FILE);
    for suffix in ["-wal", "-shm", "-journal"] {
        assert!(!companion(&database, suffix).exists(), "{suffix}");
    }
    let copy =
        rusqlite::Connection::open_with_flags(&database, rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY)
            .unwrap();
    let mode: String = copy
        .query_row("PRAGMA journal_mode", [], |row| row.get(0))
        .unwrap();
    assert_eq!(mode, "delete");
    let copied: i64 = copy
        .query_row(
            "SELECT count(*) FROM pretrained_sources WHERE id=?",
            [&late.id],
            |row| row.get(0),
        )
        .unwrap();
    assert_eq!(copied, 1);
    drop(copy);

    let expected: BTreeSet<String> = [&artifact.blob, &source.blob, &late.blob]
        .into_iter()
        .map(|blob| format!("{LEDGER_BLOB_DIR}/{}", blob.path))
        .collect();
    let listed: BTreeSet<String> = snapshot.blobs.iter().map(|blob| blob.file.clone()).collect();
    assert_eq!(listed, expected);
    for blob in &snapshot.blobs {
        let bytes = std::fs::read(target.join(&blob.file)).unwrap();
        assert_eq!(sha256_hex(&bytes), blob.sha256);
        assert_eq!(bytes.len() as u64, blob.bytes);
    }
    assert_eq!(snapshot.database.file, LEDGER_DATABASE_FILE);
    assert_eq!(
        snapshot.database.sha256,
        sha256_hex(&std::fs::read(&database).unwrap())
    );
    assert_eq!(snapshot.project_ids, BTreeSet::from(["app:board".to_owned()]));
    assert_eq!(
        TrainingRepository::verify_snapshot(&target, u64::MAX).unwrap(),
        snapshot
    );
    for suffix in ["-wal", "-shm", "-journal"] {
        assert!(!companion(&database, suffix).exists(), "verification wrote {suffix}");
    }
    assert!(matches!(repo.snapshot_to(&target), Err(Error::Conflict(_))));
}

#[test]
fn snapshot_verification_rejects_missing_or_altered_blobs_foreign_schemas_and_oversize() {
    let root = tempfile::tempdir().unwrap();
    let (repo, artifact, _) = populated_ledger(
        &root.path().join("ledger").join(LEDGER_DATABASE_FILE),
        "app:board",
    );
    let snapshot_at = |name: &str| {
        let target = root.path().join(name);
        let snapshot = repo.snapshot_to(&target).unwrap();
        (target, snapshot)
    };
    let artifact_blob = |target: &Path| target.join(LEDGER_BLOB_DIR).join(&artifact.blob.path);
    let edit_database = |target: &Path, sql: &str| {
        rusqlite::Connection::open(target.join(LEDGER_DATABASE_FILE))
            .unwrap()
            .execute_batch(sql)
            .unwrap();
    };

    let (missing, _) = snapshot_at("missing");
    std::fs::remove_file(artifact_blob(&missing)).unwrap();
    invalid_containing(
        TrainingRepository::verify_snapshot(&missing, u64::MAX),
        &artifact.blob.path,
    );

    // Blob files are hard links to the ledger's blobs: replace the entry, never write through it.
    let (altered, _) = snapshot_at("altered");
    std::fs::remove_file(artifact_blob(&altered)).unwrap();
    std::fs::write(artifact_blob(&altered), b"portable model bytez").unwrap();
    invalid_containing(
        TrainingRepository::verify_snapshot(&altered, u64::MAX),
        "has sha256",
    );
    assert_eq!(
        repo.read_blob(&artifact.blob).unwrap(),
        b"portable model bytes"
    );

    let (stray, _) = snapshot_at("stray");
    std::fs::write(stray.join(LEDGER_BLOB_DIR).join("notes.txt"), b"stray").unwrap();
    invalid_containing(
        TrainingRepository::verify_snapshot(&stray, u64::MAX),
        "unexpected entry",
    );

    for (name, sql, text) in [
        ("table", "CREATE TABLE intruder(x)", "unexpected table intruder"),
        (
            "trigger",
            "CREATE TRIGGER intruder AFTER INSERT ON jobs BEGIN DELETE FROM artifacts; END",
            "unexpected trigger intruder",
        ),
        ("version", "UPDATE ml_meta SET version=2", "versions [2]"),
    ] {
        let (foreign, _) = snapshot_at(name);
        edit_database(&foreign, sql);
        invalid_containing(TrainingRepository::verify_snapshot(&foreign, u64::MAX), text);
    }

    let (wal, _) = snapshot_at("wal");
    rusqlite::Connection::open(wal.join(LEDGER_DATABASE_FILE))
        .unwrap()
        .pragma_update(None, "journal_mode", "WAL")
        .unwrap();
    invalid_containing(
        TrainingRepository::verify_snapshot(&wal, u64::MAX),
        "single-file",
    );

    let (sized, snapshot) = snapshot_at("sized");
    let total = snapshot.database.bytes + snapshot.blobs.iter().map(|blob| blob.bytes).sum::<u64>();
    assert_eq!(
        TrainingRepository::verify_snapshot(&sized, total).unwrap(),
        snapshot
    );
    invalid_containing(
        TrainingRepository::verify_snapshot(&sized, total - 1),
        "byte limit",
    );
}

#[test]
fn installs_fill_only_an_empty_destination_and_never_replace_a_ledger() {
    let root = tempfile::tempdir().unwrap();
    let (repo, artifact, source) = populated_ledger(
        &root.path().join("ledger").join(LEDGER_DATABASE_FILE),
        "app:board",
    );
    let snapshot_dir = root.path().join("snapshot");
    let snapshot = repo.snapshot_to(&snapshot_dir).unwrap();

    let installed_path = root
        .path()
        .join("installed")
        .join("project")
        .join(LEDGER_DATABASE_FILE);
    TrainingRepository::install_snapshot(&snapshot_dir, &installed_path).unwrap();
    let installed = TrainingRepository::open(&installed_path).unwrap();
    assert_eq!(installed.project_ids().unwrap(), snapshot.project_ids);
    assert_eq!(installed.get_artifact(&artifact.id).unwrap(), artifact);
    assert_eq!(installed.read_blob(&source.blob).unwrap(), b"imported weights");
    assert_eq!(
        TrainingRepository::verify_snapshot(&snapshot_dir, u64::MAX).unwrap(),
        snapshot,
        "opening the installed ledger leaves the snapshot's database alone"
    );

    let existing_path = root.path().join("existing").join(LEDGER_DATABASE_FILE);
    let existing = TrainingRepository::open(&existing_path).unwrap();
    existing
        .register_pretrained_source(
            "app:other",
            b"own weights",
            burn_manifest(),
            json!({}),
            1024,
            now_ms(),
        )
        .unwrap();
    let database_before = std::fs::read(&existing_path).unwrap();
    let blobs_before = names(&existing_path.with_extension("artifacts"));
    assert!(matches!(
        TrainingRepository::install_snapshot(&snapshot_dir, &existing_path),
        Err(Error::Conflict(_))
    ));
    assert_eq!(std::fs::read(&existing_path).unwrap(), database_before);
    assert_eq!(names(&existing_path.with_extension("artifacts")), blobs_before);
    assert_eq!(
        existing.project_ids().unwrap(),
        BTreeSet::from(["app:other".to_owned()])
    );

    let orphaned_path = root.path().join("orphaned").join(LEDGER_DATABASE_FILE);
    let orphaned_blobs = orphaned_path.with_extension("artifacts");
    std::fs::create_dir_all(&orphaned_blobs).unwrap();
    assert!(matches!(
        TrainingRepository::install_snapshot(&snapshot_dir, &orphaned_path),
        Err(Error::Conflict(_))
    ));
    assert!(!orphaned_path.exists());
    assert!(names(&orphaned_blobs).is_empty());

    let contested = root.path().join("contested").join(LEDGER_DATABASE_FILE);
    let barrier = Arc::new(Barrier::new(2));
    let results: Vec<Result<()>> = std::thread::scope(|scope| {
        let installs: Vec<_> = (0..2)
            .map(|_| {
                let barrier = barrier.clone();
                let (snapshot_dir, contested) = (&snapshot_dir, &contested);
                scope.spawn(move || {
                    barrier.wait();
                    TrainingRepository::install_snapshot(snapshot_dir, contested)
                })
            })
            .collect();
        installs
            .into_iter()
            .map(|install| install.join().unwrap())
            .collect()
    });
    assert_eq!(results.iter().filter(|result| result.is_ok()).count(), 1);
    assert!(
        results
            .iter()
            .any(|result| matches!(result, Err(Error::Conflict(_)))),
        "{results:?}"
    );
    let expected_blobs: BTreeSet<String> = snapshot
        .blobs
        .iter()
        .map(|blob| blob.file.trim_start_matches("jobs.artifacts/").to_owned())
        .collect();
    assert_eq!(names(&contested.with_extension("artifacts")), expected_blobs);
    assert_eq!(
        TrainingRepository::open(&contested)
            .unwrap()
            .project_ids()
            .unwrap(),
        snapshot.project_ids
    );
}

#[test]
fn project_ids_cover_every_table_that_names_a_project() {
    let directory = tempfile::tempdir().unwrap();
    let repo = TrainingRepository::open(directory.path().join(LEDGER_DATABASE_FILE)).unwrap();
    assert!(repo.project_ids().unwrap().is_empty());
    let key = |project: &str| json!({"project_id":project,"stream_id":"line","inspection_version":"v1"});
    let connection = rusqlite::Connection::open(repo.path()).unwrap();
    for (sql, id, body) in [
        (
            "INSERT INTO experiments(id,scope,body) VALUES (?1,'{}',?2)",
            "experiment",
            json!({"request":{"stream":key("p:experiment")}}),
        ),
        (
            "INSERT INTO learning_projects(id,scope,body) VALUES (?1,?1,?2)",
            "learning",
            json!({"request":{"stream":key("p:learning")}}),
        ),
        (
            "INSERT INTO jobs(id,scope,status,body) VALUES (?1,'{}','succeeded',?2)",
            "job",
            json!({"stream":key("p:job")}),
        ),
        (
            "INSERT INTO artifacts(id,scope,body) VALUES (?1,'{}',?2)",
            "artifact",
            json!({"stream":key("p:artifact")}),
        ),
        (
            "INSERT INTO artifacts(id,scope,body) VALUES (?1,'{}',?2)",
            "artifact-of-the-job",
            json!({"stream":key("p:job")}),
        ),
        (
            "INSERT INTO deployments(id,scope,generation,body) VALUES (?1,'{}',1,?2)",
            "deployment",
            json!({"stream":key("p:deployment")}),
        ),
        (
            "INSERT INTO pretrained_sources(id,project_id,body) VALUES (?1,'p:source',?2)",
            "pretrained:source",
            json!({"project_id":"p:source-body"}),
        ),
    ] {
        connection
            .execute(sql, rusqlite::params![id, body.to_string()])
            .unwrap();
    }
    drop(connection);
    record(&repo, &stream("p:samples", "line"), 1, |_| {
        (
            TensorData {
                shape: vec![1],
                values: vec![0.5],
            },
            Annotation::Scalar { value: 1.0 },
        )
    });
    assert_eq!(
        repo.project_ids().unwrap(),
        [
            "p:artifact",
            "p:deployment",
            "p:experiment",
            "p:job",
            "p:learning",
            "p:samples",
            "p:source",
            "p:source-body",
        ]
        .into_iter()
        .map(str::to_owned)
        .collect::<BTreeSet<_>>()
    );
}

#[test]
fn adopted_sources_keep_their_id_kind_and_blob_under_the_private_project() {
    let root = tempfile::tempdir().unwrap();
    let (app, artifact, imported) = populated_ledger(
        &root.path().join("project").join(LEDGER_DATABASE_FILE),
        "app:board",
    );
    let user = TrainingRepository::open(
        root.path()
            .join("users")
            .join("sub")
            .join(LEDGER_DATABASE_FILE),
    )
    .unwrap();
    let trained = app.get_pretrained_source(&artifact.id).unwrap();
    assert_eq!(trained.source_kind, PretrainedSourceKind::TrainingArtifact);
    for source in [&imported, &trained] {
        let adopted = user
            .adopt_pretrained_source(&app, &source.id, "app:board:user")
            .unwrap();
        assert_eq!(adopted.id, source.id);
        assert_eq!(adopted.project_id, "app:board:user");
        assert_eq!(adopted.source_kind, source.source_kind);
        assert_eq!(adopted.blob, source.blob);
        assert_eq!(adopted.manifest, source.manifest);
        assert_eq!(
            user.read_blob(&adopted.blob).unwrap(),
            app.read_blob(&source.blob).unwrap()
        );
        assert_eq!(user.get_pretrained_source(&source.id).unwrap(), adopted);
        assert_eq!(
            user.adopt_pretrained_source(&app, &source.id, "app:board:user")
                .unwrap(),
            adopted,
            "adoption is idempotent"
        );
    }
    let listed: BTreeSet<String> = user
        .list_pretrained_sources_for_engine("app:board:user", 16, "burn")
        .unwrap()
        .into_iter()
        .map(|source| source.id)
        .collect();
    assert_eq!(
        listed,
        BTreeSet::from([artifact.id.clone(), imported.id.clone()])
    );
    assert!(user.get_artifact(&artifact.id).is_err());

    // The private ledger holds its own copy of the blob.
    std::fs::remove_file(
        app.path()
            .with_extension("artifacts")
            .join(&artifact.blob.path),
    )
    .unwrap();
    assert_eq!(
        user.read_blob(&artifact.blob).unwrap(),
        b"portable model bytes"
    );

    let connection = rusqlite::Connection::open(user.path()).unwrap();
    connection
        .execute(
            "UPDATE pretrained_sources SET body=json_set(body,'$.blob.sha256',?1) WHERE id=?2",
            rusqlite::params!["0".repeat(64), imported.id],
        )
        .unwrap();
    drop(connection);
    assert!(matches!(
        user.adopt_pretrained_source(&app, &imported.id, "app:board:user"),
        Err(Error::Conflict(_))
    ));
    assert!(matches!(
        user.adopt_pretrained_source(&app, "pretrained:missing", "app:board:user"),
        Err(Error::NotFound(_))
    ));
    assert!(matches!(
        user.adopt_pretrained_source(&app, &imported.id, " "),
        Err(Error::Invalid(_))
    ));
}
