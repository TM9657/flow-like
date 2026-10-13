//! Published model files: the loader without a ledger, the publisher and the model nodes.

#[cfg(feature = "inference")]
mod fixtures {
    use super::super::*;
    use flow_like::{
        flow::{
            board::ExecutionStage,
            execution::{
                LogLevel, Run, context::ExecutionContextCache, internal_node::InternalNode,
            },
            node::NodeLogic,
        },
        profile::Profile,
        state::{FlowLikeConfig, FlowLikeState, FlowLikeStores},
        utils::http::HTTPClient,
    };
    use flow_like_ml_core::ComputeBackend;
    use flow_like_storage::{
        Path,
        files::store::local_store::LocalObjectStore,
        object_store::{
            CopyOptions, GetOptions, GetResult, ListResult, MultipartUpload, ObjectStore,
            PutMultipartOptions, PutOptions, PutPayload, PutResult,
        },
    };
    use flow_like_types::{
        json::json,
        sync::{Mutex as AsyncMutex, RwLock},
    };
    use std::sync::{
        Weak,
        atomic::{AtomicUsize, Ordering},
    };

    type Listing<T> =
        std::pin::Pin<Box<dyn futures::Stream<Item = object_store::Result<T>> + Send>>;

    pub(super) const APP: &str = "published-test";
    pub(super) const SUB: &str = "tester";

    pub(super) fn app_root() -> String {
        format!("apps/{APP}/storage")
    }

    #[cfg_attr(not(feature = "training"), allow(dead_code))]
    pub(super) fn user_root() -> String {
        format!("users/{SUB}/apps/{APP}")
    }

    /// The run-cache key under which FlowPath keeps the app storage store.
    pub(super) fn storage_key() -> String {
        format!("dirs__storage_{}", app_root())
    }

    pub(super) fn storage_cache_key() -> String {
        format!("cache_dirs__storage_{}", app_root())
    }

    pub(super) fn canonical_folder(root: &str, id: &str) -> String {
        format!("{root}/{}", artifact_dir(id))
    }

    /// A FlowPath to `path` in the store of `root`.
    pub(super) fn located(root: &FlowPath, path: &str) -> FlowPath {
        FlowPath {
            path: path.into(),
            store_ref: root.store_ref.clone(),
            cache_store_ref: None,
        }
    }

    pub(super) async fn context_with(
        stores: FlowLikeStores,
        config: FlowLikeConfig,
    ) -> ExecutionContext {
        let state = Arc::new(FlowLikeState::new(
            config,
            HTTPClient::new_without_refetch(),
        ));
        let logic: Arc<dyn NodeLogic> =
            Arc::new(crate::inspection::model_nodes::PredictPublishedModelNode);
        let node = Arc::new(InternalNode::new(
            logic.get_node(),
            ahash::AHashMap::new(),
            logic,
            ahash::AHashMap::new(),
        ));
        let mut nodes = ahash::AHashMap::new();
        nodes.insert(node.node_id().to_string(), node.clone());
        let run: Weak<AsyncMutex<Run>> = Weak::new();
        let mut context = ExecutionContext::new(
            Arc::new(nodes),
            &run,
            &state,
            &node,
            &Arc::new(AsyncMutex::new(ahash::AHashMap::new())),
            &Arc::new(RwLock::new(ahash::AHashMap::new())),
            LogLevel::Debug,
            ExecutionStage::Dev,
            Arc::new(Profile::default()),
            None,
            Arc::new(RwLock::new(Vec::new())),
            None,
            None,
            Arc::new(ahash::AHashMap::new()),
            None,
        )
        .await;
        context.execution_cache = Some(ExecutionContextCache {
            stores,
            app_id: APP.into(),
            model_usage_app_id: None,
            board_dir: Path::from("apps").join(APP),
            board_id: "board".into(),
            node_id: node.shared_node_id(),
            sub: SUB.into(),
            shadow: false,
        });
        context
    }

    pub(super) fn local(directory: &std::path::Path) -> FlowLikeStore {
        std::fs::create_dir_all(directory).unwrap();
        FlowLikeStore::Local(Arc::new(
            LocalObjectStore::new(directory.to_path_buf()).unwrap(),
        ))
    }

    /// A desktop-shaped run whose app storage and user directory share one Local store.
    pub(super) async fn local_context(directory: &std::path::Path) -> ExecutionContext {
        let store = local(directory);
        let mut stores = FlowLikeStores::default();
        stores.app_storage_store = Some(store.clone());
        stores.user_store = Some(store);
        context_with(stores, FlowLikeConfig::new()).await
    }

    pub(super) fn write_file(root: &std::path::Path, relative: &str, bytes: &[u8]) {
        let path = root.join(relative);
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(path, bytes).unwrap();
    }

    pub(super) async fn put(store: &dyn ObjectStore, path: &str, bytes: &[u8]) {
        store
            .put(&Path::from(path), PutPayload::from(bytes.to_vec()))
            .await
            .unwrap();
    }

    pub(super) fn cpu() -> ComputeConfig {
        ComputeConfig {
            backend: ComputeBackend::Cpu,
            ..ComputeConfig::default()
        }
    }

    /// An IsolationForest artifact of two features and its weights.
    pub(super) fn forest(labels: &[&str]) -> (ModelArtifact, Vec<u8>) {
        let rows = (0..64)
            .map(|index| vec![(index % 8) as f64 / 8.0, (index % 5) as f64 / 5.0])
            .collect::<Vec<_>>();
        let config = flow_like_ml_native::models::IsolationForestConfig {
            trees: 16,
            sample_size: 32,
            seed: 7,
        };
        let model = flow_like_ml_native::models::IsolationForest::fit(&rows, &config).unwrap();
        let bytes = json::to_vec(&json!({"engine": "isolation_forest", "model": model})).unwrap();
        let sha256 = flow_like_ml_core::content_digest(&bytes);
        let artifact = json::from_value(json!({
            "id": format!("forest-{}", uuid::Uuid::new_v4()),
            "job_id": "job",
            "stream": {"project_id": format!("{APP}:board"), "stream_id": "camera", "inspection_version": "v1"},
            "snapshot_id": "snapshot",
            "dataset_digest": sha256,
            "blob": {"sha256": sha256, "bytes": bytes.len(), "path": format!("{sha256}.blob")},
            "manifest": {"engine": "isolation_forest", "format_version": 1, "labels": labels, "input_shape": [2], "recipe": {"labels": labels}, "backend": "cpu"},
            "created_at_ms": 1
        }))
        .unwrap();
        (artifact, bytes)
    }

    pub(super) fn manifest_of(artifact: &ModelArtifact) -> PublishedModelManifest {
        PublishedModelManifest {
            format_version: MANIFEST_FORMAT_VERSION,
            artifact: Some(artifact.clone()),
            pretrained_source: None,
            weights: PublishedFile {
                file: Some(WEIGHTS_FILE.into()),
                sha256: artifact.blob.sha256.clone(),
                bytes: artifact.blob.bytes,
            },
            preprocessing: None,
            task: Some(EvaluationTask::Anomaly { threshold: 0.5 }),
            calibration: Some(AnomalyCalibration {
                scale: 0.25,
                threshold: 0.5,
            }),
            goal_met: Some(true),
            validation_metrics: BTreeMap::new(),
            final_test_metrics: BTreeMap::new(),
            source_table_versions: Vec::new(),
            dependencies: PublishedDependencies::default(),
            provenance: None,
        }
    }

    /// Writes `weights.bin` and the manifest into `folder` below `root`.
    pub(super) fn write_model(
        root: &std::path::Path,
        folder: &str,
        manifest: &PublishedModelManifest,
        weights: &[u8],
    ) {
        write_file(root, &format!("{folder}/{WEIGHTS_FILE}"), weights);
        write_file(
            root,
            &format!("{folder}/{MANIFEST_FILE}"),
            &json::to_vec(manifest).unwrap(),
        );
    }

    pub(super) fn tensor_batch() -> Vec<Value> {
        vec![
            json!({"shape": [2], "values": [0.1, 0.2]}),
            json!({"shape": [2], "values": [3.5, -4.0]}),
            json!({"shape": [1, 2], "values": [0.4, 0.6]}),
        ]
    }

    pub(super) fn input(rows: Vec<Value>) -> TableRowsInput {
        TableRowsInput {
            rows,
            feature_state: None,
            stateful_features: false,
            feature_extractor: None,
        }
    }

    /// Store operations in order: `put`, `multipart`, `head` or `get`, each with its path.
    #[derive(Clone, Debug, Default)]
    pub(super) struct Recording(Arc<std::sync::Mutex<Vec<String>>>);

    impl Recording {
        pub(super) fn entries(&self) -> Vec<String> {
            self.0.lock().unwrap().clone()
        }

        #[cfg_attr(not(feature = "training"), allow(dead_code))]
        pub(super) fn puts(&self) -> Vec<String> {
            self.entries()
                .into_iter()
                .filter_map(|entry| entry.strip_prefix("put ").map(str::to_owned))
                .collect()
        }

        #[cfg_attr(not(feature = "training"), allow(dead_code))]
        pub(super) fn clear(&self) {
            self.0.lock().unwrap().clear();
        }

        fn note(&self, entry: String) {
            self.0.lock().unwrap().push(entry);
        }
    }

    /// Forwards to `inner` and records every read and write; with `failures` every write
    /// fails with a new error text.
    #[derive(Debug)]
    pub(super) struct RecordingStore {
        inner: Arc<dyn ObjectStore>,
        recording: Recording,
        failures: Option<Arc<AtomicUsize>>,
    }

    impl RecordingStore {
        pub(super) fn wrap(inner: Arc<dyn ObjectStore>, recording: &Recording) -> FlowLikeStore {
            FlowLikeStore::Other(Arc::new(Self {
                inner,
                recording: recording.clone(),
                failures: None,
            }))
        }

        #[cfg_attr(not(feature = "training"), allow(dead_code))]
        pub(super) fn failing(inner: Arc<dyn ObjectStore>, recording: &Recording) -> FlowLikeStore {
            FlowLikeStore::Other(Arc::new(Self {
                inner,
                recording: recording.clone(),
                failures: Some(Arc::new(AtomicUsize::new(0))),
            }))
        }

        fn failure(&self, location: &Path) -> Option<object_store::Error> {
            let attempt = self.failures.as_ref()?.fetch_add(1, Ordering::SeqCst);
            Some(object_store::Error::Generic {
                store: "recording",
                source: format!("injected failure {attempt} at {location}").into(),
            })
        }
    }

    impl std::fmt::Display for RecordingStore {
        fn fmt(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
            write!(formatter, "Recording({})", self.inner)
        }
    }

    #[async_trait::async_trait]
    impl ObjectStore for RecordingStore {
        async fn put_opts(
            &self,
            location: &Path,
            payload: PutPayload,
            options: PutOptions,
        ) -> object_store::Result<PutResult> {
            self.recording.note(format!("put {location}"));
            if let Some(error) = self.failure(location) {
                return Err(error);
            }
            self.inner.put_opts(location, payload, options).await
        }

        async fn put_multipart_opts(
            &self,
            location: &Path,
            options: PutMultipartOptions,
        ) -> object_store::Result<Box<dyn MultipartUpload>> {
            self.recording.note(format!("multipart {location}"));
            if let Some(error) = self.failure(location) {
                return Err(error);
            }
            self.inner.put_multipart_opts(location, options).await
        }

        async fn get_opts(
            &self,
            location: &Path,
            options: GetOptions,
        ) -> object_store::Result<GetResult> {
            let kind = if options.head { "head" } else { "get" };
            self.recording.note(format!("{kind} {location}"));
            self.inner.get_opts(location, options).await
        }

        fn delete_stream(&self, locations: Listing<Path>) -> Listing<Path> {
            self.inner.delete_stream(locations)
        }

        fn list(&self, prefix: Option<&Path>) -> Listing<ObjectMeta> {
            self.inner.list(prefix)
        }

        async fn list_with_delimiter(
            &self,
            prefix: Option<&Path>,
        ) -> object_store::Result<ListResult> {
            self.inner.list_with_delimiter(prefix).await
        }

        async fn copy_opts(
            &self,
            from: &Path,
            to: &Path,
            options: CopyOptions,
        ) -> object_store::Result<()> {
            self.inner.copy_opts(from, to, options).await
        }
    }

    /// A Local-store run whose app storage reads and writes go through a recording wrapper.
    pub(super) async fn recorded_local_context(
        directory: &std::path::Path,
        recording: &Recording,
    ) -> ExecutionContext {
        let context = local_context(directory).await;
        let recorded = RecordingStore::wrap(local(directory).as_generic(), recording);
        context.set_cache(&storage_key(), Arc::new(recorded)).await;
        context
    }
}

#[cfg(feature = "inference")]
mod loading {
    use super::super::*;
    use super::fixtures::*;
    use flow_like::state::{FlowLikeConfig, FlowLikeStores};
    use flow_like_ml_runtime::LearningRoute;
    use flow_like_storage::{
        Path,
        object_store::{
            CopyOptions, GetOptions, GetResult, GetResultPayload, ListResult, MultipartUpload,
            ObjectMeta, ObjectStore, PutMultipartOptions, PutOptions, PutPayload, PutResult,
            memory::InMemory,
        },
    };
    use flow_like_types::json::json;
    use futures::{StreamExt, stream::BoxStream};

    fn artifact_source(artifact_id: &str, storage: Option<ModelStorage>) -> PublishedModelSource {
        PublishedModelSource::Artifact {
            artifact_id: artifact_id.into(),
            storage,
        }
    }

    #[tokio::test]
    async fn a_published_folder_resolves_by_id_and_by_files_and_predicts_like_from_parts() {
        let scratch = tempfile::tempdir().unwrap();
        let (artifact, bytes) = forest(&[]);
        let manifest = manifest_of(&artifact);
        write_model(
            scratch.path(),
            &canonical_folder(&app_root(), &artifact.id),
            &manifest,
            &bytes,
        );
        let mut context = local_context(scratch.path()).await;
        let by_id = resolve_model(&mut context, &artifact_source(&artifact.id, None))
            .await
            .unwrap();
        assert_eq!(by_id.storage, Some(ModelStorage::App));
        assert_eq!(by_id.manifest, manifest);
        let root = FlowPath::from_storage_dir(&mut context, false)
            .await
            .unwrap();
        let files = PublishedModelSource::Files {
            manifest: child(
                &root,
                &format!("{}/{MANIFEST_FILE}", artifact_dir(&artifact.id)),
            ),
            weights: None,
        };
        let by_files = resolve_model(&mut context, &files).await.unwrap();
        assert_eq!(by_files.manifest, manifest);
        assert_eq!(by_files.storage, None);
        assert_eq!(by_files.weights_path.path, by_id.weights_path.path);
        let predictor = published_predictor(&mut context, &by_files, &cpu())
            .await
            .unwrap();
        let published = predict_table_rows(&predictor, None, None, input(tensor_batch())).unwrap();
        let direct =
            CatalogLoadedPredictor::from_parts(artifact, &bytes, &cpu().for_host()).unwrap();
        let expected = tensor_batch()
            .into_iter()
            .map(|row| {
                let mut tensor: TensorData = json::from_value(row).unwrap();
                if tensor.shape == [2] {
                    tensor.shape.insert(0, 1);
                }
                direct.predict(&tensor).unwrap()
            })
            .collect::<Vec<_>>();
        assert_eq!(published.predictions, expected);
        assert!(published.anomaly.is_none());
    }

    #[tokio::test]
    async fn version_one_exports_load_only_through_files_with_their_weights() {
        let scratch = tempfile::tempdir().unwrap();
        let (artifact, bytes) = forest(&[]);
        let legacy = json!({
            "format_version": 1, "artifact": artifact, "preprocessing": null,
            "source_table_versions": [], "validation_metrics": {"auroc": 0.9},
            "final_test_metrics": {"auroc": 0.8}, "goal_met": true
        });
        write_file(
            scratch.path(),
            "exports/v1/model.json",
            &json::to_vec(&legacy).unwrap(),
        );
        write_file(scratch.path(), "exports/v1/model.bin", &bytes);
        write_file(
            scratch.path(),
            &format!(
                "{}/{MANIFEST_FILE}",
                canonical_folder(&app_root(), &artifact.id)
            ),
            &json::to_vec(&legacy).unwrap(),
        );
        let mut context = local_context(scratch.path()).await;
        let root = FlowPath::from_storage_dir(&mut context, false)
            .await
            .unwrap();
        let files = |weights: Option<FlowPath>| PublishedModelSource::Files {
            manifest: located(&root, "exports/v1/model.json"),
            weights,
        };
        let model = resolve_model(
            &mut context,
            &files(Some(located(&root, "exports/v1/model.bin"))),
        )
        .await
        .unwrap();
        assert_eq!(model.manifest.format_version, 1);
        assert_eq!(model.manifest.goal_met, Some(true));
        assert_eq!(model.manifest.final_test_metrics["auroc"], 0.8);
        assert!(
            published_preprocessing(&mut context, &model)
                .await
                .unwrap()
                .is_none()
        );
        let predictor = published_predictor(&mut context, &model, &cpu())
            .await
            .unwrap();
        assert_eq!(predictor.artifact().id, artifact.id);
        let unnamed = resolve_model(&mut context, &files(None))
            .await
            .err()
            .expect("a version 1 export names no weights file")
            .to_string();
        assert!(unnamed.contains("names no weights file"), "{unnamed}");
        let canonical = resolve_model(
            &mut context,
            &artifact_source(&artifact.id, Some(ModelStorage::App)),
        )
        .await
        .err()
        .expect("published folders hold version 2 manifests")
        .to_string();
        assert!(canonical.contains("version 1"), "{canonical}");
    }

    #[tokio::test]
    async fn executor_reads_write_nothing_to_the_store_or_its_cache_layer() {
        let (artifact, bytes) = forest(&[]);
        let folder = canonical_folder(&app_root(), &artifact.id);
        let cloud = Arc::new(InMemory::new());
        put(cloud.as_ref(), &format!("{folder}/{WEIGHTS_FILE}"), &bytes).await;
        put(
            cloud.as_ref(),
            &format!("{folder}/{MANIFEST_FILE}"),
            &json::to_vec(&manifest_of(&artifact)).unwrap(),
        )
        .await;
        let (store_log, cache_log) = (Recording::default(), Recording::default());
        let mut context = context_with(FlowLikeStores::default(), FlowLikeConfig::new()).await;
        context
            .set_cache(
                &storage_key(),
                Arc::new(RecordingStore::wrap(cloud, &store_log)),
            )
            .await;
        context
            .set_cache(
                &storage_cache_key(),
                Arc::new(RecordingStore::wrap(Arc::new(InMemory::new()), &cache_log)),
            )
            .await;
        let model = resolve_model(
            &mut context,
            &artifact_source(&artifact.id, Some(ModelStorage::App)),
        )
        .await
        .unwrap();
        assert!(model.manifest_path.cache_store_ref.is_some());
        let predictor = published_predictor(&mut context, &model, &cpu())
            .await
            .unwrap();
        assert!(
            published_preprocessing(&mut context, &model)
                .await
                .unwrap()
                .is_none()
        );
        predict_table_rows(&predictor, None, None, input(tensor_batch())).unwrap();
        let entries = store_log.entries();
        assert!(
            entries
                .iter()
                .all(|entry| entry.starts_with("head ") || entry.starts_with("get ")),
            "{entries:?}"
        );
        assert!(entries.iter().any(|entry| entry.starts_with("get ")));
        assert!(cache_log.entries().is_empty(), "{:?}", cache_log.entries());
    }

    #[tokio::test]
    async fn an_other_app_store_without_a_cache_layer_serves_published_files() {
        let (artifact, bytes) = forest(&[]);
        let folder = canonical_folder(&app_root(), &artifact.id);
        let memory = Arc::new(InMemory::new());
        put(memory.as_ref(), &format!("{folder}/{WEIGHTS_FILE}"), &bytes).await;
        put(
            memory.as_ref(),
            &format!("{folder}/{MANIFEST_FILE}"),
            &json::to_vec(&manifest_of(&artifact)).unwrap(),
        )
        .await;
        let mut stores = FlowLikeStores::default();
        stores.app_storage_store = Some(FlowLikeStore::Other(memory));
        let mut context = context_with(stores, FlowLikeConfig::new()).await;
        let model = resolve_model(&mut context, &artifact_source(&artifact.id, None))
            .await
            .unwrap();
        assert!(model.manifest_path.cache_store_ref.is_none());
        let predictor = published_predictor(&mut context, &model, &cpu())
            .await
            .unwrap();
        let output = predict_table_rows(
            &predictor,
            None,
            model.manifest.calibration,
            input(tensor_batch()),
        )
        .unwrap();
        assert_eq!(output.predictions.len(), 3);
        assert_eq!(
            output.anomaly.map(|decisions| decisions.rows.len()),
            Some(3)
        );
    }

    #[tokio::test]
    async fn weights_manifests_and_preprocessing_are_refused_before_they_are_decoded() {
        let scratch = tempfile::tempdir().unwrap();
        let log = Recording::default();
        let mut context = recorded_local_context(scratch.path(), &log).await;
        let publish = |manifest: &PublishedModelManifest, weights: &[u8]| {
            let id = manifest.artifact_id().to_owned();
            write_model(
                scratch.path(),
                &canonical_folder(&app_root(), &id),
                manifest,
                weights,
            );
            let folder = canonical_folder(&app_root(), &id);
            (id, folder)
        };
        let fetched = |log: &Recording, path: &str| log.entries().contains(&format!("get {path}"));

        let (artifact, bytes) = forest(&[]);
        let mut tampered = bytes.clone();
        let middle = tampered.len() / 2;
        tampered[middle] ^= 1;
        let (id, _) = publish(&manifest_of(&artifact), &tampered);
        let model = resolve_model(&mut context, &artifact_source(&id, None))
            .await
            .unwrap();
        let error = published_predictor(&mut context, &model, &cpu())
            .await
            .err()
            .expect("weights with another digest are refused")
            .to_string();
        assert!(error.contains("does not hold"), "{error}");

        let (artifact, mut bytes) = forest(&[]);
        bytes.push(b' ');
        let (id, folder) = publish(&manifest_of(&artifact), &bytes);
        let model = resolve_model(&mut context, &artifact_source(&id, None))
            .await
            .unwrap();
        let error = published_predictor(&mut context, &model, &cpu())
            .await
            .err()
            .expect("weights of another size are refused")
            .to_string();
        assert!(error.contains("but its manifest records"), "{error}");
        assert!(!fetched(&log, &format!("{folder}/{WEIGHTS_FILE}")));

        let (artifact, bytes) = forest(&[]);
        let (id, folder) = publish(&manifest_of(&artifact), &bytes);
        let model = resolve_model(&mut context, &artifact_source(&id, None))
            .await
            .unwrap();
        let tiny = ComputeConfig {
            memory_limit_bytes: 64,
            ..cpu()
        };
        let error = published_predictor(&mut context, &model, &tiny)
            .await
            .err()
            .expect("weights above the memory budget are refused")
            .to_string();
        assert!(error.contains(&id), "{error}");
        assert!(!fetched(&log, &format!("{folder}/{WEIGHTS_FILE}")));

        let (artifact, bytes) = forest(&[]);
        let mut oversized = json::to_value(manifest_of(&artifact)).unwrap();
        oversized["padding"] = json!("x".repeat(MAXIMUM_MANIFEST_BYTES as usize));
        let folder = canonical_folder(&app_root(), &artifact.id);
        write_file(scratch.path(), &format!("{folder}/{WEIGHTS_FILE}"), &bytes);
        write_file(
            scratch.path(),
            &format!("{folder}/{MANIFEST_FILE}"),
            &json::to_vec(&oversized).unwrap(),
        );
        let error = resolve_model(&mut context, &artifact_source(&artifact.id, None))
            .await
            .err()
            .expect("a manifest above 1 MiB is refused")
            .to_string();
        assert!(error.contains("above its limit"), "{error}");
        assert!(!fetched(&log, &format!("{folder}/{MANIFEST_FILE}")));

        let (artifact, bytes) = forest(&[]);
        let mut manifest = manifest_of(&artifact);
        manifest.preprocessing = Some(PublishedPreprocessingRef {
            file: PREPROCESSING_FILE.into(),
            sha256: "0".repeat(64),
            bytes: 16,
            maximum_bytes: 0,
        });
        let (id, folder) = publish(&manifest, &bytes);
        write_file(
            scratch.path(),
            &format!("{folder}/{PREPROCESSING_FILE}"),
            &vec![b' '; 2 * 1024 * 1024],
        );
        let model = resolve_model(&mut context, &artifact_source(&id, None))
            .await
            .unwrap();
        let error = published_preprocessing(&mut context, &model)
            .await
            .err()
            .expect("preprocessing above its cap is refused")
            .to_string();
        assert!(error.contains("above its limit"), "{error}");
        assert!(!fetched(&log, &format!("{folder}/{PREPROCESSING_FILE}")));
    }

    #[tokio::test]
    async fn manifest_file_fields_must_be_plain_names_in_their_folder() {
        let scratch = tempfile::tempdir().unwrap();
        let mut context = local_context(scratch.path()).await;
        for name in ["nested/weights.bin", "..", "", "back\\slash"] {
            let (artifact, bytes) = forest(&[]);
            let mut manifest = manifest_of(&artifact);
            manifest.weights.file = Some(name.into());
            write_model(
                scratch.path(),
                &canonical_folder(&app_root(), &artifact.id),
                &manifest,
                &bytes,
            );
            let error = resolve_model(&mut context, &artifact_source(&artifact.id, None))
                .await
                .err()
                .expect("a weights file outside the folder is refused")
                .to_string();
            assert!(error.contains("plain file name"), "{name:?}: {error}");
        }
        let (artifact, bytes) = forest(&[]);
        let mut manifest = manifest_of(&artifact);
        manifest.preprocessing = Some(PublishedPreprocessingRef {
            file: "../preprocessing.json".into(),
            sha256: "0".repeat(64),
            bytes: 1,
            maximum_bytes: 1,
        });
        write_model(
            scratch.path(),
            &canonical_folder(&app_root(), &artifact.id),
            &manifest,
            &bytes,
        );
        let error = resolve_model(&mut context, &artifact_source(&artifact.id, None))
            .await
            .err()
            .expect("a preprocessing file outside the folder is refused")
            .to_string();
        assert!(error.contains("plain file name"), "{error}");
    }

    fn source_record(id: &str, bytes: &[u8]) -> PretrainedSource {
        let sha256 = flow_like_ml_core::content_digest(bytes);
        json::from_value(json!({
            "id": id, "project_id": format!("{APP}:board"), "source_kind": "imported",
            "blob": {"sha256": sha256, "bytes": bytes.len(), "path": format!("{sha256}.blob")},
            "manifest": {"engine": "burn", "format_version": 1, "config": {}, "input_shape": [1, 8, 8]},
            "origin": {}, "created_at_ms": 1
        }))
        .unwrap()
    }

    fn source_manifest(source: &PretrainedSource) -> PublishedModelManifest {
        PublishedModelManifest {
            format_version: MANIFEST_FORMAT_VERSION,
            artifact: None,
            pretrained_source: Some(source.clone()),
            weights: PublishedFile {
                file: Some(WEIGHTS_FILE.into()),
                sha256: source.blob.sha256.clone(),
                bytes: source.blob.bytes,
            },
            preprocessing: None,
            task: None,
            calibration: None,
            goal_met: None,
            validation_metrics: BTreeMap::new(),
            final_test_metrics: BTreeMap::new(),
            source_table_versions: Vec::new(),
            dependencies: PublishedDependencies::default(),
            provenance: None,
        }
    }

    #[tokio::test]
    async fn dependencies_resolve_beside_exports_and_in_sibling_folders_and_must_match() {
        let scratch = tempfile::tempdir().unwrap();
        let mut context = local_context(scratch.path()).await;
        let weights = b"extractor weights".as_slice();
        let extractor = source_record("pretrained:extractor", weights);
        let dependency = PublishedDependency {
            source_id: extractor.id.clone(),
            blob_sha256: extractor.blob.sha256.clone(),
        };
        let (artifact, bytes) = forest(&[]);
        let mut manifest = manifest_of(&artifact);
        manifest.dependencies.feature_extractor = Some(dependency.clone());

        write_model(scratch.path(), "exports/model", &manifest, &bytes);
        write_model(
            scratch.path(),
            &format!("exports/model/{}", dependency_dir(&extractor.id)),
            &source_manifest(&extractor),
            weights,
        );
        let root = FlowPath::from_storage_dir(&mut context, false)
            .await
            .unwrap();
        let exported = resolve_model(
            &mut context,
            &PublishedModelSource::Files {
                manifest: located(&root, "exports/model/manifest.json"),
                weights: None,
            },
        )
        .await
        .unwrap();
        let found = resolve_dependency(&mut context, &exported, &dependency)
            .await
            .unwrap();
        assert_eq!(found.manifest.artifact_id(), extractor.id);
        assert!(
            found
                .weights_path
                .path
                .ends_with(&format!("{}/{WEIGHTS_FILE}", dependency_dir(&extractor.id))),
            "{}",
            found.weights_path.path
        );

        write_model(
            scratch.path(),
            &canonical_folder(&app_root(), &artifact.id),
            &manifest,
            &bytes,
        );
        write_model(
            scratch.path(),
            &canonical_folder(&app_root(), &extractor.id),
            &source_manifest(&extractor),
            weights,
        );
        let canonical = resolve_model(&mut context, &artifact_source(&artifact.id, None))
            .await
            .unwrap();
        let sibling = resolve_dependency(&mut context, &canonical, &dependency)
            .await
            .unwrap();
        assert!(
            sibling
                .weights_path
                .path
                .ends_with(&format!("{}/{WEIGHTS_FILE}", artifact_dir(&extractor.id))),
            "{}",
            sibling.weights_path.path
        );

        let other = source_record("pretrained:other", b"other weights");
        write_model(
            scratch.path(),
            &canonical_folder(&app_root(), &extractor.id),
            &source_manifest(&other),
            b"other weights",
        );
        let error = resolve_dependency(&mut context, &canonical, &dependency)
            .await
            .err()
            .expect("a folder holding another extractor is refused")
            .to_string();
        assert!(error.contains("not the feature extractor"), "{error}");
        let wrong_digest = PublishedDependency {
            blob_sha256: "f".repeat(64),
            ..dependency
        };
        let error = resolve_dependency(&mut context, &exported, &wrong_digest)
            .await
            .err()
            .expect("an extractor with other weights is refused")
            .to_string();
        assert!(error.contains("not the feature extractor"), "{error}");
    }

    #[test]
    fn published_calibration_decides_like_the_audited_rule() {
        let (artifact, bytes) = forest(&[]);
        let predictor =
            CatalogLoadedPredictor::from_parts(artifact, &bytes, &cpu().for_host()).unwrap();
        let calibration = AnomalyCalibration {
            scale: 0.7,
            threshold: 0.5,
        };
        let output =
            predict_table_rows(&predictor, None, Some(calibration), input(tensor_batch())).unwrap();
        let expected =
            crate::inspection::auto_training::anomaly_decisions(&output.predictions, 0.7, 0.5)
                .unwrap();
        assert_eq!(
            json::to_value(output.anomaly.expect("anomaly models return decisions")).unwrap(),
            json::to_value(expected).unwrap()
        );
    }

    fn champion(project_id: &str, artifact_id: Option<&str>, paused: bool) -> ChampionPointer {
        ChampionPointer {
            format_version: POINTER_FORMAT_VERSION,
            project_id: project_id.into(),
            deployment_id: format!("deployment:{project_id}"),
            deployment_generation: 3,
            project_generation: 12,
            artifact_id: artifact_id.map(str::to_owned),
            sha256: artifact_id.map(|_| "0".repeat(64)),
            paused,
            cycle_id: None,
            retained: Vec::new(),
            storage: ModelStorage::App,
            published_at_ms: 0,
        }
    }

    #[tokio::test]
    async fn learning_pointers_route_to_the_champion_or_the_teacher() {
        let scratch = tempfile::tempdir().unwrap();
        for (project, artifact, paused) in [
            ("champion", Some("model-1"), false),
            ("paused", Some("model-1"), true),
            ("teacher", None, false),
        ] {
            write_file(
                scratch.path(),
                &format!("{}/{}", app_root(), learning_pointer_path(project)),
                &json::to_vec(&champion(project, artifact, paused)).unwrap(),
            );
        }
        let mut context = local_context(scratch.path()).await;
        assert!(matches!(
            published_learning_route(&mut context, "champion", None).await.unwrap(),
            Some(LearningRoute::Champion { artifact_id }) if artifact_id == "model-1"
        ));
        assert!(matches!(
            published_learning_route(&mut context, "paused", None)
                .await
                .unwrap(),
            Some(LearningRoute::Teacher)
        ));
        assert!(matches!(
            published_learning_route(&mut context, "teacher", Some(ModelStorage::App))
                .await
                .unwrap(),
            Some(LearningRoute::Teacher)
        ));
        assert!(
            published_learning_route(&mut context, "missing", None)
                .await
                .unwrap()
                .is_none()
        );
        assert!(
            newer_published_generation(
                &mut context,
                PointerKind::Learning,
                LedgerScope::App,
                "champion",
                (3, 11)
            )
            .await
        );
        assert!(
            !newer_published_generation(
                &mut context,
                PointerKind::Learning,
                LedgerScope::App,
                "champion",
                (3, 12)
            )
            .await
        );
        assert!(
            !newer_published_generation(
                &mut context,
                PointerKind::Learning,
                LedgerScope::App,
                "missing",
                (0, 0)
            )
            .await
        );
        let mut stores = FlowLikeStores::default();
        stores.app_storage_store = Some(FlowLikeStore::Memory(Arc::new(InMemory::new())));
        let mut online = context_with(stores, FlowLikeConfig::new()).await;
        assert!(
            !newer_published_generation(
                &mut online,
                PointerKind::Learning,
                LedgerScope::App,
                "champion",
                (0, 0)
            )
            .await
        );
    }

    #[tokio::test]
    async fn runs_share_a_cached_predictor_while_other_label_lists_load_their_own() {
        let scratch = tempfile::tempdir().unwrap();
        let (artifact, bytes) = forest(&[]);
        write_model(
            scratch.path(),
            &canonical_folder(&app_root(), &artifact.id),
            &manifest_of(&artifact),
            &bytes,
        );
        let mut relabelled = artifact.clone();
        relabelled.id = format!("{}-relabelled", artifact.id);
        relabelled.manifest["labels"] = json!(["normal"]);
        write_model(
            scratch.path(),
            &canonical_folder(&app_root(), &relabelled.id),
            &manifest_of(&relabelled),
            &bytes,
        );
        let mut first = local_context(scratch.path()).await;
        let mut second = local_context(scratch.path()).await;
        let model = resolve_model(&mut first, &artifact_source(&artifact.id, None))
            .await
            .unwrap();
        let loaded = published_predictor(&mut first, &model, &cpu())
            .await
            .unwrap();
        let again = resolve_model(&mut second, &artifact_source(&artifact.id, None))
            .await
            .unwrap();
        let shared = published_predictor(&mut second, &again, &cpu())
            .await
            .unwrap();
        assert!(Arc::ptr_eq(&loaded, &shared));
        let other = resolve_model(&mut second, &artifact_source(&relabelled.id, None))
            .await
            .unwrap();
        let separate = published_predictor(&mut second, &other, &cpu())
            .await
            .unwrap();
        assert!(!Arc::ptr_eq(&loaded, &separate));
    }

    #[tokio::test]
    async fn a_crafted_manifest_cannot_borrow_a_cached_predictor() {
        let owner = tempfile::tempdir().unwrap();
        let (artifact, bytes) = forest(&[]);
        let manifest = manifest_of(&artifact);
        write_model(
            owner.path(),
            &canonical_folder(&app_root(), &artifact.id),
            &manifest,
            &bytes,
        );
        let mut context = local_context(owner.path()).await;
        let model = resolve_model(&mut context, &artifact_source(&artifact.id, None))
            .await
            .unwrap();
        published_predictor(&mut context, &model, &cpu())
            .await
            .unwrap();
        let requester = tempfile::tempdir().unwrap();
        write_model(
            requester.path(),
            "crafted",
            &manifest,
            &vec![b'x'; bytes.len()],
        );
        let mut other = local_context(requester.path()).await;
        let root = FlowPath::from_storage_dir(&mut other, false).await.unwrap();
        let crafted = resolve_model(
            &mut other,
            &PublishedModelSource::Files {
                manifest: located(&root, "crafted/manifest.json"),
                weights: None,
            },
        )
        .await
        .unwrap();
        let error = published_predictor(&mut other, &crafted, &cpu())
            .await
            .err()
            .expect("the requester's own weights decide")
            .to_string();
        assert!(error.contains("does not hold"), "{error}");
    }

    #[test]
    fn the_cache_keeps_eight_predictors_within_its_budget_and_evicts_the_oldest() {
        let (artifact, bytes) = forest(&[]);
        let predictor: CachedPredictor = Arc::new(
            CatalogLoadedPredictor::from_parts(artifact, &bytes, &cpu().for_host()).unwrap(),
        );
        let mut cache = PredictorCache::default();
        for key in ["first", "second", "third"] {
            cache.insert(key.into(), predictor.clone(), 40, 100);
        }
        assert!(cache.touch("first").is_none());
        assert!(cache.touch("second").is_some() && cache.touch("third").is_some());
        cache.insert("huge".into(), predictor.clone(), 101, 100);
        assert!(cache.touch("huge").is_none());
        for index in 0..9 {
            cache.insert(format!("small-{index}"), predictor.clone(), 1, 100);
        }
        assert_eq!(cache.entries.len(), MAXIMUM_CACHED_PREDICTORS);
        assert!(cache.touch("small-0").is_none() && cache.touch("small-8").is_some());
        assert!(cache.verified_hit("small-8", "object", "digest").is_none());
        assert!(
            cache
                .verify("object".into(), "digest".into(), "small-8")
                .is_some()
        );
        assert!(cache.verified_hit("small-8", "object", "digest").is_some());
        assert!(cache.verified_hit("small-8", "object", "other").is_none());
        assert!(
            cache
                .verified_hit("small-8", "another object", "digest")
                .is_none()
        );
    }

    /// Answers a HEAD with the object stored at `{location}.before` when there is one and a GET
    /// with the object at `location`, as when a file is replaced between a reader's HEAD and its
    /// GET. Records each location whose GET body is read.
    #[derive(Debug)]
    struct SwappedStore {
        inner: InMemory,
        read: Arc<std::sync::Mutex<Vec<String>>>,
    }

    impl SwappedStore {
        fn new() -> Self {
            Self {
                inner: InMemory::new(),
                read: Arc::default(),
            }
        }

        fn read(&self) -> Vec<String> {
            self.read.lock().unwrap().clone()
        }

        async fn head_before(
            &self,
            location: &Path,
            options: GetOptions,
        ) -> object_store::Result<GetResult> {
            let before = Path::from(format!("{location}.before"));
            match self.inner.get_opts(&before, options.clone()).await {
                Ok(mut result) => {
                    result.meta.location = location.clone();
                    Ok(result)
                }
                Err(object_store::Error::NotFound { .. }) => {
                    self.inner.get_opts(location, options).await
                }
                Err(error) => Err(error),
            }
        }
    }

    impl std::fmt::Display for SwappedStore {
        fn fmt(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
            write!(formatter, "Swapped({})", self.inner)
        }
    }

    #[async_trait::async_trait]
    impl ObjectStore for SwappedStore {
        async fn put_opts(
            &self,
            location: &Path,
            payload: PutPayload,
            options: PutOptions,
        ) -> object_store::Result<PutResult> {
            self.inner.put_opts(location, payload, options).await
        }

        async fn put_multipart_opts(
            &self,
            location: &Path,
            options: PutMultipartOptions,
        ) -> object_store::Result<Box<dyn MultipartUpload>> {
            self.inner.put_multipart_opts(location, options).await
        }

        async fn get_opts(
            &self,
            location: &Path,
            options: GetOptions,
        ) -> object_store::Result<GetResult> {
            if options.head {
                return self.head_before(location, options).await;
            }
            let mut result = self.inner.get_opts(location, options).await?;
            let GetResultPayload::Stream(body) = result.payload else {
                unreachable!("in-memory objects are streamed");
            };
            let (read, shown) = (self.read.clone(), location.to_string());
            result.payload = GetResultPayload::Stream(
                body.inspect(move |_| read.lock().unwrap().push(shown.clone()))
                    .boxed(),
            );
            Ok(result)
        }

        fn delete_stream(
            &self,
            locations: BoxStream<'static, object_store::Result<Path>>,
        ) -> BoxStream<'static, object_store::Result<Path>> {
            self.inner.delete_stream(locations)
        }

        fn list(
            &self,
            prefix: Option<&Path>,
        ) -> BoxStream<'static, object_store::Result<ObjectMeta>> {
            self.inner.list(prefix)
        }

        async fn list_with_delimiter(
            &self,
            prefix: Option<&Path>,
        ) -> object_store::Result<ListResult> {
            self.inner.list_with_delimiter(prefix).await
        }

        async fn copy_opts(
            &self,
            from: &Path,
            to: &Path,
            options: CopyOptions,
        ) -> object_store::Result<()> {
            self.inner.copy_opts(from, to, options).await
        }
    }

    #[tokio::test]
    async fn files_replaced_after_their_head_are_refused_before_their_body_is_read() {
        let store = Arc::new(SwappedStore::new());
        let mut context = context_with(FlowLikeStores::default(), FlowLikeConfig::new()).await;
        context
            .set_cache(
                &storage_key(),
                Arc::new(FlowLikeStore::Other(store.clone())),
            )
            .await;
        let oversized = vec![b' '; MAXIMUM_MANIFEST_BYTES as usize + 1];

        let (artifact, bytes) = forest(&[]);
        let folder = canonical_folder(&app_root(), &artifact.id);
        let manifest = format!("{folder}/{MANIFEST_FILE}");
        let document = json::to_vec(&manifest_of(&artifact)).unwrap();
        put(&store.inner, &format!("{manifest}.before"), &document).await;
        put(&store.inner, &manifest, &oversized).await;
        put(&store.inner, &format!("{folder}/{WEIGHTS_FILE}"), &bytes).await;
        let error = resolve_model(
            &mut context,
            &artifact_source(&artifact.id, Some(ModelStorage::App)),
        )
        .await
        .err()
        .expect("a manifest grown past its cap after its HEAD is refused")
        .to_string();
        assert!(error.contains("above its limit"), "{error}");
        assert!(!store.read().contains(&manifest), "{:?}", store.read());

        let (artifact, bytes) = forest(&[]);
        let folder = canonical_folder(&app_root(), &artifact.id);
        let manifest = format!("{folder}/{MANIFEST_FILE}");
        let weights = format!("{folder}/{WEIGHTS_FILE}");
        let document = json::to_vec(&manifest_of(&artifact)).unwrap();
        let grown = [bytes.as_slice(), oversized.as_slice()].concat();
        put(&store.inner, &manifest, &document).await;
        put(&store.inner, &format!("{weights}.before"), &bytes).await;
        put(&store.inner, &weights, &grown).await;
        let model = resolve_model(
            &mut context,
            &artifact_source(&artifact.id, Some(ModelStorage::App)),
        )
        .await
        .unwrap();
        let error = published_predictor(&mut context, &model, &cpu())
            .await
            .err()
            .expect("weights grown after their HEAD are refused")
            .to_string();
        assert!(error.contains("above its limit"), "{error}");
        let read = store.read();
        assert!(
            read.contains(&manifest) && !read.contains(&weights),
            "{read:?}"
        );

        let mirror = tempfile::tempdir().unwrap();
        context
            .set_cache(&storage_cache_key(), Arc::new(local(mirror.path())))
            .await;
        let (artifact, _) = forest(&[]);
        let manifest = format!(
            "{}/{MANIFEST_FILE}",
            canonical_folder(&app_root(), &artifact.id)
        );
        let document = json::to_vec(&manifest_of(&artifact)).unwrap();
        put(&store.inner, &format!("{manifest}.before"), &document).await;
        put(&store.inner, &manifest, &oversized).await;
        let error = resolve_model(
            &mut context,
            &artifact_source(&artifact.id, Some(ModelStorage::App)),
        )
        .await
        .err()
        .expect("a manifest grown after its HEAD is refused through a Local cache layer too")
        .to_string();
        assert!(error.contains("above its limit"), "{error}");
        assert!(!store.read().contains(&manifest), "{:?}", store.read());
        assert!(!mirror.path().join(&manifest).exists());
    }

    #[tokio::test]
    async fn reads_through_a_local_cache_layer_refresh_the_mirror_and_then_serve_it() {
        let (artifact, bytes) = forest(&[]);
        let folder = canonical_folder(&app_root(), &artifact.id);
        let manifest = format!("{folder}/{MANIFEST_FILE}");
        let weights = format!("{folder}/{WEIGHTS_FILE}");
        let document = json::to_vec(&manifest_of(&artifact)).unwrap();
        let cloud = Arc::new(InMemory::new());
        put(cloud.as_ref(), &manifest, &document).await;
        put(cloud.as_ref(), &weights, &bytes).await;
        let (log, mirror) = (Recording::default(), tempfile::tempdir().unwrap());
        let mut context = context_with(FlowLikeStores::default(), FlowLikeConfig::new()).await;
        context
            .set_cache(&storage_key(), Arc::new(RecordingStore::wrap(cloud, &log)))
            .await;
        context
            .set_cache(&storage_cache_key(), Arc::new(local(mirror.path())))
            .await;
        let source = artifact_source(&artifact.id, Some(ModelStorage::App));
        let model = resolve_model(&mut context, &source).await.unwrap();
        published_predictor(&mut context, &model, &cpu())
            .await
            .unwrap();
        assert_eq!(
            std::fs::read(mirror.path().join(&manifest)).unwrap(),
            document
        );
        assert_eq!(std::fs::read(mirror.path().join(&weights)).unwrap(), bytes);
        log.clear();
        resolve_model(&mut context, &source).await.unwrap();
        let entries = log.entries();
        assert!(entries.contains(&format!("head {manifest}")), "{entries:?}");
        assert!(!entries.contains(&format!("get {manifest}")), "{entries:?}");
    }

    #[tokio::test]
    async fn the_sam2_fallback_refuses_weights_above_a_quarter_of_the_request_memory() {
        let scratch = tempfile::tempdir().unwrap();
        let log = Recording::default();
        let mut context = recorded_local_context(scratch.path(), &log).await;
        let weights = vec![7u8; 4096];
        let mut sam2 = source_record("pretrained:sam2", &weights);
        sam2.manifest["engine"] = json!("sam2");
        let folder = canonical_folder(&app_root(), &sam2.id);
        write_model(scratch.path(), &folder, &source_manifest(&sam2), &weights);
        let compute = ComputeConfig {
            memory_limit_bytes: 2 * weights.len() as u64,
            ..cpu()
        };
        let image = json!({"shape": [2], "values": [0.1, 0.2]});
        let image: TensorData = json::from_value(image).unwrap();
        let error = published_sam2_prediction(&mut context, &sam2.id, &image, &[], &compute)
            .await
            .err()
            .expect("SAM 2 weights above a quarter of the memory are refused")
            .to_string();
        let limit = weights.len() / 2;
        assert!(
            error.contains(&format!("above the limit of {limit} bytes")),
            "{error}"
        );
        let fetched = log.entries();
        assert!(
            !fetched.contains(&format!("get {folder}/{WEIGHTS_FILE}")),
            "{fetched:?}"
        );
    }

    #[tokio::test]
    async fn paused_champions_and_deployments_serve_no_model_so_the_teacher_decides() {
        const DEPLOYMENT: &str = "deployment:camera";
        let scratch = tempfile::tempdir().unwrap();
        let (artifact, bytes) = forest(&[]);
        write_model(
            scratch.path(),
            &canonical_folder(&app_root(), &artifact.id),
            &manifest_of(&artifact),
            &bytes,
        );
        let mut learning = champion("project", Some(&artifact.id), false);
        learning.sha256 = Some(artifact.blob.sha256.clone());
        let mut deployment: DeploymentPointer = json::from_value(json!({
            "format_version": POINTER_FORMAT_VERSION, "deployment_id": DEPLOYMENT,
            "generation": 1, "active_artifact_id": artifact.id, "paused": false,
            "stream": artifact.stream, "sha256": artifact.blob.sha256,
            "deployment": {
                "id": DEPLOYMENT, "stream": artifact.stream, "generation": 1,
                "active_artifact_id": artifact.id, "paused": false,
                "previous_artifact_id": null, "evaluation_id": "evaluation",
                "policy": null, "metric_policy": null, "updated_at_ms": 0
            },
            "storage": "app", "published_at_ms": 0
        }))
        .unwrap();
        let root = scratch.path();
        let learning_path = format!("{}/{}", app_root(), learning_pointer_path("project"));
        let deployment_path = format!("{}/{}", app_root(), deployment_pointer_path(DEPLOYMENT));
        let publish = |learning: &ChampionPointer, deployment: &DeploymentPointer| {
            write_file(root, &learning_path, &json::to_vec(learning).unwrap());
            write_file(root, &deployment_path, &json::to_vec(deployment).unwrap());
        };
        let sources = [
            PublishedModelSource::LearningChampion {
                project_id: "project".into(),
                storage: None,
            },
            PublishedModelSource::Deployment {
                deployment_id: DEPLOYMENT.into(),
                storage: None,
            },
        ];
        let mut context = local_context(scratch.path()).await;
        publish(&learning, &deployment);
        for source in &sources {
            let served = resolve_model(&mut context, source).await.unwrap();
            assert_eq!(served.manifest.artifact_id(), artifact.id);
        }

        learning.paused = true;
        deployment.paused = true;
        publish(&learning, &deployment);
        let records = ["learning project project", "deployment deployment:camera"];
        for (source, record) in sources.iter().zip(records) {
            let error = resolve_model(&mut context, source)
                .await
                .err()
                .expect("a paused pointer serves no model")
                .to_string();
            assert!(
                error.contains(&format!("{record} is paused, so the teacher decides")),
                "{error}"
            );
        }

        learning.artifact_id = None;
        learning.sha256 = None;
        learning.paused = false;
        publish(&learning, &deployment);
        let error = resolve_model(&mut context, &sources[0])
            .await
            .err()
            .expect("a champion pointer without a model serves none")
            .to_string();
        assert!(
            error.contains("has no champion yet, so the teacher decides"),
            "{error}"
        );
    }

    #[cfg(feature = "training")]
    #[tokio::test]
    async fn import_model_keeps_private_models_out_of_the_shared_ledger() {
        use crate::inspection::lifecycle::{LedgerLocation, scoped_repository};
        use crate::inspection::model_nodes::{ImportModelRequest, import_model};
        fn holds_no_shared_ledger(context: &ExecutionContext) -> bool {
            !LedgerLocation::new(context, LedgerScope::App)
                .unwrap()
                .exists()
                .unwrap()
        }
        let scratch = tempfile::tempdir().unwrap();
        let mut context = local_context(scratch.path()).await;
        let weights = b"private backbone".as_slice();
        let source = source_record("pretrained:private", weights);
        let mut manifest = source_manifest(&source);
        manifest.provenance = Some(
            json::from_value(json!({
                "publisher": "auto", "ledger": "user", "storage": "user", "published_at_ms": 0
            }))
            .unwrap(),
        );
        let relative = format!("{}/{MANIFEST_FILE}", artifact_dir(&source.id));
        write_model(
            scratch.path(),
            &canonical_folder(&user_root(), &source.id),
            &manifest,
            weights,
        );
        let request = |model: PublishedModelSource, private: bool| ImportModelRequest {
            model,
            private,
            maximum_bytes: 1024 * 1024,
        };
        let user_dir = FlowPath::from_user_dir(&mut context, false).await.unwrap();
        let in_user_storage = [
            artifact_source(&source.id, None),
            artifact_source(&source.id, Some(ModelStorage::User)),
            PublishedModelSource::Files {
                manifest: child(&user_dir, &relative),
                weights: None,
            },
        ];
        for model in in_user_storage.clone() {
            let refused = import_model(&mut context, request(model, false))
                .await
                .err()
                .expect("a model in private storage stays out of the shared ledger")
                .to_string();
            assert!(
                refused.contains("is read from your private (user-scoped) storage"),
                "{refused}"
            );
        }
        assert!(holds_no_shared_ledger(&context));
        let private = import_model(&mut context, request(in_user_storage[0].clone(), true))
            .await
            .unwrap();
        assert_eq!(
            private.project_id,
            scoped_repository(&context, LedgerScope::User).unwrap().1
        );

        write_model(
            scratch.path(),
            &canonical_folder(&app_root(), &source.id),
            &manifest,
            weights,
        );
        let refused = import_model(
            &mut context,
            request(artifact_source(&source.id, None), false),
        )
        .await
        .err()
        .expect("a privately trained model needs app storage named explicitly")
        .to_string();
        assert!(
            refused.contains("was trained on private (user-scoped) data"),
            "{refused}"
        );
        assert!(holds_no_shared_ledger(&context));
        let app_dir = FlowPath::from_storage_dir(&mut context, false)
            .await
            .unwrap();
        for model in [
            artifact_source(&source.id, Some(ModelStorage::App)),
            PublishedModelSource::Files {
                manifest: child(&app_dir, &relative),
                weights: None,
            },
        ] {
            let shared = import_model(&mut context, request(model, false))
                .await
                .unwrap();
            assert_eq!(
                shared.project_id,
                scoped_repository(&context, LedgerScope::App).unwrap().1
            );
        }
    }
}

#[cfg(feature = "training")]
mod publishing {
    use super::super::*;
    use super::fixtures::*;
    use crate::inspection::auto_training::{
        AutoTrainRequest, AutoTrainingResult, controller, experiment_repository,
        start_auto_training,
    };
    use crate::inspection::ledger_transfer::{
        ExportTrainingLedgerRequest, ImportTrainingLedgerRequest, export_training_ledger,
        import_training_ledger,
    };
    use crate::inspection::lifecycle::{LEDGER_SCOPE_KEY, LedgerLocation, scoped_repository};
    use crate::inspection::model_nodes::{
        ImportModelRequest, PredictPublishedModelRequest, import_model, predict_published_model,
    };
    use crate::inspection::publisher::*;
    use flow_like::state::{FlowLikeConfig, FlowLikeStores};
    use flow_like_catalog_core::{CachedDB, NodeDBConnection};
    use flow_like_ml_core::{Annotation, LabelProvenance, Sample};
    use flow_like_ml_runtime::{
        Experiment, ExperimentStatus, LabelSource, LearningProjectRequest, SplitPolicy,
        TrainingRepository, TrainingRequest, TrainingSample, WorkerLimits,
        engines::{register_burn_engine, register_native_engines},
        now_ms,
        worker::TrainingWorker,
    };
    use flow_like_storage::{
        databases::vector::{
            VectorStore, buffered::BufferedVectorStore, lancedb::LanceDBVectorStore,
        },
        object_store::{memory::InMemory, path::Path},
    };
    use flow_like_types::{json::json, sync::RwLock};

    struct Harness {
        context: ExecutionContext,
        source: NodeDBConnection,
        scratch: tempfile::TempDir,
    }

    impl Harness {
        fn store_dir(&self) -> std::path::PathBuf {
            self.scratch.path().join("store")
        }

        fn stored(&self, relative: &str) -> std::path::PathBuf {
            self.store_dir().join(relative)
        }
    }

    /// One reading in four per group is anomalous, so every partition holds both classes.
    fn anomaly_rows() -> Vec<Value> {
        (0..96)
            .map(|index| {
                let anomaly = index % 4 == 3;
                let (x, y) = if anomaly {
                    (6.0 + (index % 3) as f64, -6.0 - (index % 5) as f64)
                } else {
                    ((index % 7) as f64 / 10.0, (index % 5) as f64 / 10.0)
                };
                json!({"id": index.to_string(), "group": (index / 4).to_string(), "x": x, "y": y, "anomaly": anomaly})
            })
            .collect()
    }

    fn anomaly_request(source: &NodeDBConnection) -> AutoTrainRequest {
        json::from_value(json!({
            "dataset": {"source": source, "stream_id": "sensors",
                "spec": {"id": "sensor-anomaly", "description": "Detect anomalous sensor readings", "task": "sensor_anomaly", "labels": [], "input_shape": [2], "prediction_horizon_ms": null, "minimum_examples": 2, "minimum_examples_per_class": 1},
                "mapping": {"row_id": "id", "group_id": "group", "features": {"kind": "tabular", "options": {"numeric_columns": ["x", "y"], "categorical_columns": []}}, "target": {"kind": "column", "column": "anomaly"}, "provenance": {"kind": "measured", "source": "gauge"}},
                "split": {"kind": "group", "train_fraction": 0.6, "validation_fraction": 0.2, "seed": 11}},
            "compute": {"backend": "cpu"}, "search": {"maximum_candidates": 1}, "budget": {"maximum_trials": 1},
            "tables": {"create_tables": false},
            "initial_candidates": [{"engine": "isolation_forest", "recipe": {"labels": [], "inspection_task": "sensor_anomaly", "config": {"trees": 64, "sample_size": 256, "seed": 5}}, "compute": {"backend": "cpu"}}]
        }))
        .unwrap()
    }

    /// A desktop run with a project and a user database and a measurements table in the
    /// project database.
    async fn harness() -> Harness {
        let scratch = tempfile::tempdir().unwrap();
        let project_db = scratch
            .path()
            .join("project-db")
            .to_string_lossy()
            .into_owned();
        let user_db = scratch
            .path()
            .join("user-db")
            .to_string_lossy()
            .into_owned();
        let mut config = FlowLikeConfig::new();
        let project = project_db.clone();
        config.register_build_project_database(Arc::new(move |_| {
            flow_like_storage::lancedb::connect(&project)
        }));
        config.register_build_user_database(Arc::new(move |_| {
            flow_like_storage::lancedb::connect(&user_db)
        }));
        let store = local(&scratch.path().join("store"));
        let mut stores = FlowLikeStores::default();
        stores.app_storage_store = Some(store.clone());
        stores.user_store = Some(store);
        let context = context_with(stores, config).await;
        let connection = flow_like_storage::lancedb::connect(&project_db)
            .execute()
            .await
            .unwrap();
        let mut table =
            LanceDBVectorStore::from_connection(connection, "measurements".into()).await;
        table.insert(anomaly_rows()).await.unwrap();
        let source = NodeDBConnection {
            cache_key: "published-source".into(),
        };
        context.cache.write().await.insert(
            source.cache_key.clone(),
            Arc::new(CachedDB {
                db: Arc::new(RwLock::new(BufferedVectorStore::new(table, 0))),
            }),
        );
        Harness {
            context,
            source,
            scratch,
        }
    }

    /// An Auto Train experiment whose snapshot and request the copies reuse.
    struct Template {
        repo: TrainingRepository,
        id: String,
    }

    async fn template(harness: &mut Harness) -> Template {
        let started = start_auto_training(
            &mut harness.context,
            anomaly_request(&harness.source),
            false,
        )
        .await
        .unwrap();
        let repo = experiment_repository(&harness.context, &started.experiment_id).unwrap();
        Template {
            repo,
            id: started.experiment_id,
        }
    }

    fn ledger(harness: &Harness, name: &str) -> TrainingRepository {
        TrainingRepository::open(
            harness
                .scratch
                .path()
                .join("ledgers")
                .join(name)
                .join("jobs.sqlite"),
        )
        .unwrap()
    }

    fn auto_stream() -> String {
        format!("auto:{}", uuid::Uuid::new_v4())
    }

    /// The template experiment run to completion in `ledger` on stream `stream_id` of the
    /// app's project, with `edit` applied to its request context.
    async fn finished_copy(
        template: &Template,
        ledger: &TrainingRepository,
        stream_id: String,
        edit: impl FnOnce(&mut Value) + Send + std::any::Any,
    ) -> Experiment {
        let (source, id, target) = (template.repo.clone(), template.id.clone(), ledger.clone());
        tokio::task::spawn_blocking(move || {
            let experiment = source.get_experiment(&id).unwrap();
            let mut snapshot = source
                .get_snapshot(&experiment.request.snapshot_id)
                .unwrap();
            let stream = StreamKey {
                project_id: format!("{APP}:board"),
                stream_id,
                inspection_version: experiment.request.stream.inspection_version.clone(),
            };
            for sample in snapshot
                .train
                .iter_mut()
                .chain(snapshot.validation.iter_mut())
                .chain(snapshot.test.iter_mut())
            {
                sample.payload["stream_id"] = json!(stream.stream_id);
            }
            snapshot.id = String::new();
            snapshot.stream = stream.clone();
            for partition in [&snapshot.train, &snapshot.validation, &snapshot.test] {
                target.record_prepared_samples(&stream, partition).unwrap();
            }
            let mut request = experiment.request.clone();
            request.stream = stream;
            request.snapshot_id = String::new();
            edit(&mut request.context);
            let created = target
                .create_experiment_with_snapshot(snapshot, request, now_ms())
                .unwrap();
            controller(target.clone(), &created.request.budget)
                .unwrap()
                .run(&created.id)
                .unwrap()
        })
        .await
        .unwrap()
    }

    fn marked(scope: LedgerScope) -> impl FnOnce(&mut Value) + Send + std::any::Any {
        move |context: &mut Value| {
            context[LEDGER_SCOPE_KEY] = json::to_value(scope).unwrap();
        }
    }

    /// No scope marker, and a main source whose connection digest is `digest`.
    fn unmarked(digest: String) -> impl FnOnce(&mut Value) + Send + std::any::Any {
        move |context: &mut Value| {
            if let Some(fields) = context.as_object_mut() {
                fields.remove(LEDGER_SCOPE_KEY);
            }
            context["pinned_source"]["locator"] =
                json!({"app_id": APP, "board_id": "board", "connection_digest": digest});
        }
    }

    /// Records the run's app storage operations from here on.
    async fn record_app_storage(harness: &Harness) -> Recording {
        let recording = Recording::default();
        let store = RecordingStore::wrap(local(&harness.store_dir()).as_generic(), &recording);
        harness
            .context
            .set_cache(&storage_key(), Arc::new(store))
            .await;
        recording
    }

    fn events(repo: &TrainingRepository, id: &str, kind: &str) -> Vec<Value> {
        repo.experiment_events(id)
            .unwrap()
            .into_iter()
            .filter(|event| event["kind"] == kind)
            .collect()
    }

    async fn publish(
        harness: &mut Harness,
        repo: &TrainingRepository,
        id: &str,
        reason: PublishReason,
    ) -> PublishOutcome {
        publish_experiment(
            &mut harness.context,
            repo,
            LedgerScope::App,
            id,
            reason,
            PublishLimits::default(),
        )
        .await
    }

    async fn new_run(harness: &Harness) {
        harness.context.cache.write().await.remove(CHECKED_KEY);
    }

    #[tokio::test]
    async fn a_finished_winner_publishes_in_order_once_and_repairs_only_what_is_missing() {
        let mut harness = harness().await;
        let template = template(&mut harness).await;
        let repo = ledger(&harness, "app");
        let experiment =
            finished_copy(&template, &repo, auto_stream(), marked(LedgerScope::App)).await;
        assert_eq!(
            experiment.status,
            ExperimentStatus::Completed,
            "{:?}",
            experiment.stop_reason
        );
        let log = record_app_storage(&harness).await;
        let reference = match publish(
            &mut harness,
            &repo,
            &experiment.id,
            PublishReason::StateChange,
        )
        .await
        {
            PublishOutcome::Published(reference) => reference,
            other => panic!("{other:?}"),
        };
        assert_eq!(reference.storage, Some(ModelStorage::App));
        let folder = canonical_folder(&app_root(), &reference.artifact_id);
        let weights = format!("{folder}/{WEIGHTS_FILE}");
        let manifest = format!("{folder}/{MANIFEST_FILE}");
        let pointer = format!("{}/{}", app_root(), experiment_pointer_path(&experiment.id));
        let puts = log.puts();
        assert_eq!(puts.first(), Some(&weights), "{puts:?}");
        assert_eq!(
            puts[puts.len() - 2..],
            [manifest.clone(), pointer.clone()],
            "{puts:?}"
        );
        let entries = log.entries();
        for (index, entry) in entries.iter().enumerate() {
            if let Some(path) = entry.strip_prefix("put ") {
                assert_eq!(
                    entries.get(index + 1),
                    Some(&format!("head {path}")),
                    "{entries:?}"
                );
            }
        }
        assert_eq!(events(&repo, &experiment.id, PUBLISHED_EVENT).len(), 1);

        new_run(&harness).await;
        log.clear();
        let outcome = publish(
            &mut harness,
            &repo,
            &experiment.id,
            PublishReason::SelfRepair,
        )
        .await;
        assert!(matches!(outcome, PublishOutcome::Unchanged), "{outcome:?}");
        assert!(log.puts().is_empty(), "{:?}", log.entries());
        assert_eq!(events(&repo, &experiment.id, PUBLISHED_EVENT).len(), 1);

        std::fs::remove_file(harness.stored(&manifest)).unwrap();
        new_run(&harness).await;
        log.clear();
        let outcome = publish(
            &mut harness,
            &repo,
            &experiment.id,
            PublishReason::SelfRepair,
        )
        .await;
        assert!(
            matches!(outcome, PublishOutcome::Published(_)),
            "{outcome:?}"
        );
        assert_eq!(log.puts(), vec![manifest, pointer]);
        log.clear();
        let outcome = publish(
            &mut harness,
            &repo,
            &experiment.id,
            PublishReason::SelfRepair,
        )
        .await;
        assert!(matches!(outcome, PublishOutcome::Unchanged), "{outcome:?}");
        assert!(log.entries().is_empty(), "{:?}", log.entries());
    }

    #[tokio::test]
    async fn kept_manifests_without_the_planned_preprocessing_reference_are_rewritten() {
        let mut harness = harness().await;
        let template = template(&mut harness).await;
        let repo = ledger(&harness, "app");
        let experiment =
            finished_copy(&template, &repo, auto_stream(), marked(LedgerScope::App)).await;
        let reference = match publish(
            &mut harness,
            &repo,
            &experiment.id,
            PublishReason::StateChange,
        )
        .await
        {
            PublishOutcome::Published(reference) => reference,
            other => panic!("{other:?}"),
        };
        let manifest_file = harness.stored(&reference.manifest.path);
        let read = || -> PublishedModelManifest {
            json::from_slice(&std::fs::read(&manifest_file).unwrap()).unwrap()
        };
        let published = read();
        let planned = published
            .preprocessing
            .clone()
            .expect("tabular winners publish their preprocessing");
        let preprocessing_file = manifest_file.with_file_name(&planned.file);
        for preprocessing_kept in [false, true] {
            let stripped = PublishedModelManifest {
                preprocessing: None,
                ..published.clone()
            };
            std::fs::write(&manifest_file, json::to_vec(&stripped).unwrap()).unwrap();
            if !preprocessing_kept {
                std::fs::remove_file(&preprocessing_file).unwrap();
            }
            new_run(&harness).await;
            let outcome = publish(
                &mut harness,
                &repo,
                &experiment.id,
                PublishReason::SelfRepair,
            )
            .await;
            assert!(
                matches!(outcome, PublishOutcome::Published(_)),
                "{outcome:?}"
            );
            assert_eq!(read().preprocessing.as_ref(), Some(&planned));
            assert!(preprocessing_file.exists());
        }
    }

    #[tokio::test]
    async fn failing_writes_leave_one_failure_event_and_no_publication() {
        let mut harness = harness().await;
        let template = template(&mut harness).await;
        let repo = ledger(&harness, "app");
        let experiment =
            finished_copy(&template, &repo, auto_stream(), marked(LedgerScope::App)).await;
        let failing = RecordingStore::failing(
            local(&harness.store_dir()).as_generic(),
            &Recording::default(),
        );
        harness
            .context
            .set_cache(&storage_key(), Arc::new(failing))
            .await;
        for _ in 0..3 {
            let outcome = publish(
                &mut harness,
                &repo,
                &experiment.id,
                PublishReason::StateChange,
            )
            .await;
            assert!(
                matches!(outcome, PublishOutcome::Failed { .. }),
                "{outcome:?}"
            );
        }
        assert_eq!(events(&repo, &experiment.id, FAILED_EVENT).len(), 1);
        assert!(events(&repo, &experiment.id, PUBLISHED_EVENT).is_empty());
    }

    #[tokio::test]
    async fn learning_cycle_experiments_stay_unpublished_while_unmet_goals_publish() {
        let mut harness = harness().await;
        let template = template(&mut harness).await;
        let repo = ledger(&harness, "app");
        let cycle = finished_copy(
            &template,
            &repo,
            format!("learning:{}", uuid::Uuid::new_v4()),
            marked(LedgerScope::App),
        )
        .await;
        let outcome = publish(&mut harness, &repo, &cycle.id, PublishReason::StateChange).await;
        assert!(
            matches!(outcome, PublishOutcome::Skipped { .. }),
            "{outcome:?}"
        );
        assert!(
            !harness
                .stored(&format!(
                    "{}/{}",
                    app_root(),
                    experiment_pointer_path(&cycle.id)
                ))
                .exists()
        );
        assert!(
            repo.experiment_events(&cycle.id)
                .unwrap()
                .iter()
                .all(|event| !event["kind"]
                    .as_str()
                    .unwrap_or_default()
                    .starts_with("model_publish"))
        );

        let winner = finished_copy(&template, &repo, auto_stream(), marked(LedgerScope::App)).await;
        assert!(
            !winner.target_met,
            "default goals configure no bounds, so the winner misses its goals"
        );
        let reference =
            match publish(&mut harness, &repo, &winner.id, PublishReason::StateChange).await {
                PublishOutcome::Published(reference) => reference,
                other => panic!("{other:?}"),
            };
        let manifest: PublishedModelManifest =
            json::from_slice(&std::fs::read(harness.stored(&reference.manifest.path)).unwrap())
                .unwrap();
        assert_eq!(manifest.goal_met, Some(false));
        let pointer: ExperimentPointer = json::from_slice(
            &std::fs::read(harness.stored(&format!(
                "{}/{}",
                app_root(),
                experiment_pointer_path(&winner.id)
            )))
            .unwrap(),
        )
        .unwrap();
        assert!(!pointer.goal_met);
    }

    #[tokio::test]
    async fn weights_over_the_serving_cap_are_skipped_once() {
        let mut harness = harness().await;
        let template = template(&mut harness).await;
        let repo = ledger(&harness, "app");
        let experiment =
            finished_copy(&template, &repo, auto_stream(), marked(LedgerScope::App)).await;
        for _ in 0..2 {
            let outcome = publish_experiment(
                &mut harness.context,
                &repo,
                LedgerScope::App,
                &experiment.id,
                PublishReason::StateChange,
                PublishLimits {
                    serving_weights_bytes: 1,
                },
            )
            .await;
            assert!(
                matches!(&outcome, PublishOutcome::Skipped { reason } if reason == WEIGHTS_EXCEED_SERVING_CAP),
                "{outcome:?}"
            );
        }
        let skipped = events(&repo, &experiment.id, SKIPPED_EVENT);
        assert_eq!(skipped.len(), 1);
        assert_eq!(skipped[0]["body"]["reason"], WEIGHTS_EXCEED_SERVING_CAP);
        assert!(!harness.stored(&format!("{}/models", app_root())).exists());
    }

    #[tokio::test]
    async fn unmarked_app_winners_publish_where_their_source_tables_live() {
        let mut harness = harness().await;
        let template = template(&mut harness).await;
        let repo = ledger(&harness, "app");
        let digests = crate::inspection::auto_training_tables::scope_digests(&harness.context)
            .await
            .unwrap();
        let user = digests
            .user
            .clone()
            .expect("the harness registers a user database");
        let private = finished_copy(&template, &repo, auto_stream(), unmarked(user)).await;
        let reference =
            match publish(&mut harness, &repo, &private.id, PublishReason::StateChange).await {
                PublishOutcome::Published(reference) => reference,
                other => panic!("{other:?}"),
            };
        assert_eq!(reference.storage, Some(ModelStorage::User));
        assert!(
            harness
                .stored(&format!(
                    "{}/{}",
                    user_root(),
                    experiment_pointer_path(&private.id)
                ))
                .exists()
        );

        let unknown =
            finished_copy(&template, &repo, auto_stream(), unmarked("f".repeat(64))).await;
        let outcome = publish(&mut harness, &repo, &unknown.id, PublishReason::StateChange).await;
        assert!(
            matches!(&outcome, PublishOutcome::Skipped { reason } if reason == SOURCE_SCOPE_UNCLASSIFIABLE),
            "{outcome:?}"
        );
        let skipped = events(&repo, &unknown.id, SKIPPED_EVENT);
        assert_eq!(skipped.len(), 1);
        assert_eq!(skipped[0]["body"]["reason"], SOURCE_SCOPE_UNCLASSIFIABLE);
        assert!(!harness.stored(&format!("{}/models", app_root())).exists());
    }

    fn burn_manifest() -> Value {
        json!({"engine": "burn", "format_version": 1, "config": {}, "input_shape": [1, 8, 8]})
    }

    #[tokio::test]
    async fn credential_backed_roots_use_the_undecorated_store_and_mirror_into_local_storage() {
        let scratch = tempfile::tempdir().unwrap();
        let mirror_store = local(&scratch.path().join("mirror"));
        let cloud = Arc::new(InMemory::new());
        let mut context = context_with(FlowLikeStores::default(), FlowLikeConfig::new()).await;
        context
            .set_cache(
                &storage_key(),
                Arc::new(FlowLikeStore::Memory(cloud.clone())),
            )
            .await;
        context
            .set_cache(&storage_cache_key(), Arc::new(mirror_store.clone()))
            .await;
        let root = FlowPath::from_storage_dir(&mut context, false)
            .await
            .unwrap();
        assert!(root.cache_store_ref.is_some());
        assert!(matches!(
            writer_store(&mut context, &root).await.unwrap(),
            FlowLikeStore::Memory(_)
        ));

        let mut credentialed = context_with(FlowLikeStores::default(), FlowLikeConfig::new()).await;
        let credentials: flow_like::credentials::SharedCredentials = json::from_value(json!({
            "Aws": {
                "access_key_id": "test-key", "secret_access_key": "test-secret",
                "meta_bucket": "meta", "content_bucket": "content", "logs_bucket": "logs",
                "region": "eu-central-1"
            }
        }))
        .unwrap();
        credentialed.credentials = Some(Arc::new(credentials));
        credentialed
            .set_cache(
                &storage_key(),
                Arc::new(FlowLikeStore::Memory(cloud.clone())),
            )
            .await;
        credentialed
            .set_cache(&storage_cache_key(), Arc::new(mirror_store))
            .await;
        let backed = FlowPath::from_storage_dir(&mut credentialed, false)
            .await
            .unwrap();
        assert!(matches!(
            writer_store(&mut credentialed, &backed).await.unwrap(),
            FlowLikeStore::AWS(_)
        ));
        let uncached = FlowPath {
            cache_store_ref: None,
            ..backed
        };
        assert!(matches!(
            writer_store(&mut credentialed, &uncached).await.unwrap(),
            FlowLikeStore::Memory(_)
        ));

        let repo = TrainingRepository::open(scratch.path().join("ledger/jobs.sqlite")).unwrap();
        let source = repo
            .register_pretrained_source(
                &format!("{APP}:board"),
                b"fixture weights",
                burn_manifest(),
                json!({"source": "fixture"}),
                1024,
                now_ms(),
            )
            .unwrap();
        let outcome =
            publish_pretrained_source(&mut context, &repo, LedgerScope::App, &source).await;
        assert!(
            matches!(outcome, PublishOutcome::Published(_)),
            "{outcome:?}"
        );
        let folder = canonical_folder(&app_root(), &source.id);
        let mirrored = scratch.path().join("mirror").join(&folder);
        for file in [
            "weights.bin",
            "weights.s3flowEtag",
            "manifest.json",
            "manifest.s3flowEtag",
        ] {
            assert!(mirrored.join(file).exists(), "{file} is not mirrored");
        }
        cloud
            .head(&Path::from(format!("{folder}/{MANIFEST_FILE}")))
            .await
            .unwrap();
        let again = publish_pretrained_source(&mut context, &repo, LedgerScope::App, &source).await;
        assert!(matches!(again, PublishOutcome::Unchanged), "{again:?}");
    }

    #[tokio::test]
    async fn mirrored_publications_refill_the_local_copies_the_mirror_lacks() {
        let scratch = tempfile::tempdir().unwrap();
        let mut context = local_context(&scratch.path().join("store")).await;
        let cloud = Arc::new(InMemory::new());
        context
            .set_cache(
                &storage_key(),
                Arc::new(FlowLikeStore::Memory(cloud.clone())),
            )
            .await;
        let mirror = scratch.path().join("mirror");
        context
            .set_cache(&storage_cache_key(), Arc::new(local(&mirror)))
            .await;
        let repo = TrainingRepository::open(scratch.path().join("ledger/jobs.sqlite")).unwrap();
        let (source, model) = {
            let repo = repo.clone();
            tokio::task::spawn_blocking(move || {
                let project = format!("{APP}:board");
                let (source, extractor) = fixture_extractor(&repo, &project);
                (source, patchcore(&repo, &project, "line", &extractor))
            })
            .await
            .unwrap()
        };
        let deployment = deployment_of("line", 1, &model);
        let model_folder = canonical_folder(&app_root(), &model.id);
        let source_folder = canonical_folder(&app_root(), &source.id);
        let pointer = format!("{}/{}", app_root(), deployment_pointer_path("line"));
        let mut files = vec![pointer.clone()];
        for folder in [&model_folder, &source_folder] {
            for file in [WEIGHTS_FILE, MANIFEST_FILE] {
                files.push(format!("{folder}/{file}"));
            }
        }
        let mirrored = |files: &[String]| files.iter().all(|file| mirror.join(file).exists());
        let outcome = deploy(&mut context, &repo, &deployment, PublishReason::StateChange).await;
        assert!(
            matches!(outcome, PublishOutcome::Published(_)),
            "{outcome:?}"
        );
        assert!(mirrored(&files));

        cloud
            .delete(&Path::from(format!("{source_folder}/{MANIFEST_FILE}")))
            .await
            .unwrap();
        std::fs::remove_dir_all(mirror.join(&source_folder)).unwrap();
        let outcome = deploy(&mut context, &repo, &deployment, PublishReason::StateChange).await;
        assert!(
            matches!(outcome, PublishOutcome::Published(_)),
            "a repaired extractor republishes its model: {outcome:?}"
        );
        assert!(mirrored(&files));

        std::fs::remove_dir_all(mirror.join(&model_folder)).unwrap();
        std::fs::remove_file(mirror.join(&pointer)).unwrap();
        let outcome = deploy(&mut context, &repo, &deployment, PublishReason::StateChange).await;
        assert!(matches!(outcome, PublishOutcome::Unchanged), "{outcome:?}");
        assert!(mirrored(&files));
        assert!(
            mirror
                .join(&model_folder)
                .join("weights.s3flowEtag")
                .exists()
        );
    }

    fn learning_request() -> LearningProjectRequest {
        let stream = format!("learning:{}", uuid::Uuid::new_v4());
        json::from_value(json!({
            "stream": {"project_id": format!("{APP}:board"), "stream_id": stream, "inspection_version": "v1"},
            "spec": {"id": "quality"},
            "source": {"ledger_scope": "app"},
            "goals": {"task": {"kind": "classification"}, "primary_metric": "accuracy", "direction": "maximize", "minimum_audited_samples": 1, "bounds": [{"name": "accuracy", "minimum": 0.5, "maximum": null}]},
            "deployment_id": format!("deployment:{stream}")
        }))
        .unwrap()
    }

    async fn champion(
        context: &mut ExecutionContext,
        repo: &TrainingRepository,
        project_id: &str,
    ) -> PublishOutcome {
        ensure_champion_published(
            context,
            repo,
            LedgerScope::App,
            project_id,
            PublishReason::StateChange,
        )
        .await
    }

    #[tokio::test]
    async fn champion_pointers_order_pause_and_resume_and_refuse_older_writes() {
        let scratch = tempfile::tempdir().unwrap();
        let store = scratch.path().join("store");
        let mut context = local_context(&store).await;
        let repo = TrainingRepository::open(scratch.path().join("ledger/jobs.sqlite")).unwrap();
        let project = repo
            .create_learning_project(learning_request(), now_ms())
            .unwrap();
        let pointer_file = store.join(format!(
            "{}/{}",
            app_root(),
            learning_pointer_path(&project.id)
        ));
        let read = || -> ChampionPointer {
            json::from_slice(&std::fs::read(&pointer_file).unwrap()).unwrap()
        };

        let outcome = champion(&mut context, &repo, &project.id).await;
        assert!(
            !matches!(outcome, PublishOutcome::Failed { .. }),
            "{outcome:?}"
        );
        let created = read();
        assert_eq!(created.order(), (0, project.generation));
        assert!(!created.paused && created.artifact_id.is_none());

        let paused = repo
            .pause_learning_project(&project.id, project.generation, now_ms())
            .unwrap();
        let outcome = champion(&mut context, &repo, &project.id).await;
        assert!(
            !matches!(outcome, PublishOutcome::Failed { .. }),
            "{outcome:?}"
        );
        let pointer = read();
        assert!(pointer.paused);
        assert_eq!(pointer.order(), (0, paused.generation));
        assert!(pointer.order() > created.order());

        let resumed = repo
            .resume_learning_project(&project.id, paused.generation, now_ms())
            .unwrap();
        let outcome = champion(&mut context, &repo, &project.id).await;
        assert!(
            !matches!(outcome, PublishOutcome::Failed { .. }),
            "{outcome:?}"
        );
        let pointer = read();
        assert!(!pointer.paused);
        assert_eq!(pointer.order(), (0, resumed.generation));

        let newer = ChampionPointer {
            deployment_generation: 7,
            ..pointer
        };
        std::fs::write(&pointer_file, json::to_vec(&newer).unwrap()).unwrap();
        let outcome = champion(&mut context, &repo, &project.id).await;
        assert!(
            matches!(&outcome, PublishOutcome::Skipped { reason } if reason.contains("newer")),
            "{outcome:?}"
        );
        assert_eq!(read(), newer);
    }

    #[tokio::test]
    async fn self_repair_returns_at_once_while_the_record_is_being_published() {
        let scratch = tempfile::tempdir().unwrap();
        let store = scratch.path().join("store");
        let mut context = local_context(&store).await;
        let repo = TrainingRepository::open(scratch.path().join("ledger/jobs.sqlite")).unwrap();
        let project = repo
            .create_learning_project(learning_request(), now_ms())
            .unwrap();
        let pointer_file = store.join(format!(
            "{}/{}",
            app_root(),
            learning_pointer_path(&project.id)
        ));
        let running = hold_flight("learning", &repo, &project.id).await;
        let repair = ensure_champion_published(
            &mut context,
            &repo,
            LedgerScope::App,
            &project.id,
            PublishReason::SelfRepair,
        );
        let outcome = tokio::time::timeout(std::time::Duration::from_secs(30), repair)
            .await
            .expect("self-repair waited for the running publication");
        assert!(matches!(outcome, PublishOutcome::Unchanged), "{outcome:?}");
        assert!(!pointer_file.exists());

        drop(running);
        let outcome = ensure_champion_published(
            &mut context,
            &repo,
            LedgerScope::App,
            &project.id,
            PublishReason::SelfRepair,
        )
        .await;
        assert!(
            !matches!(outcome, PublishOutcome::Failed { .. }),
            "{outcome:?}"
        );
        assert!(
            pointer_file.exists(),
            "the self-repair that returned at once marked the run as checked"
        );
    }

    mod swapping {
        use flow_like_storage::object_store::{
            self, CopyOptions, GetOptions, GetResult, ListResult, MultipartUpload, ObjectMeta,
            ObjectStore, PutMultipartOptions, PutOptions, PutPayload, PutResult, path::Path,
        };
        use flow_like_types::Bytes;
        use futures::stream::BoxStream;
        use std::sync::{
            Arc,
            atomic::{AtomicBool, Ordering},
        };

        /// Forwards to `inner`, but replaces `location` with `replacement` right before its
        /// first GET, as a writer racing a reader between its HEAD and its GET would.
        #[derive(Debug)]
        pub(super) struct SwappingStore {
            pub(super) inner: Arc<dyn ObjectStore>,
            pub(super) location: Path,
            pub(super) replacement: Bytes,
            pub(super) swapped: AtomicBool,
        }

        impl std::fmt::Display for SwappingStore {
            fn fmt(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
                write!(formatter, "Swapping({})", self.inner)
            }
        }

        #[async_trait::async_trait]
        impl ObjectStore for SwappingStore {
            async fn put_opts(
                &self,
                location: &Path,
                payload: PutPayload,
                options: PutOptions,
            ) -> object_store::Result<PutResult> {
                self.inner.put_opts(location, payload, options).await
            }

            async fn put_multipart_opts(
                &self,
                location: &Path,
                options: PutMultipartOptions,
            ) -> object_store::Result<Box<dyn MultipartUpload>> {
                self.inner.put_multipart_opts(location, options).await
            }

            async fn get_opts(
                &self,
                location: &Path,
                options: GetOptions,
            ) -> object_store::Result<GetResult> {
                if !options.head
                    && *location == self.location
                    && !self.swapped.swap(true, Ordering::SeqCst)
                {
                    let payload = PutPayload::from_bytes(self.replacement.clone());
                    self.inner
                        .put_opts(location, payload, PutOptions::default())
                        .await?;
                }
                self.inner.get_opts(location, options).await
            }

            fn delete_stream(
                &self,
                locations: BoxStream<'static, object_store::Result<Path>>,
            ) -> BoxStream<'static, object_store::Result<Path>> {
                self.inner.delete_stream(locations)
            }

            fn list(
                &self,
                prefix: Option<&Path>,
            ) -> BoxStream<'static, object_store::Result<ObjectMeta>> {
                self.inner.list(prefix)
            }

            async fn list_with_delimiter(
                &self,
                prefix: Option<&Path>,
            ) -> object_store::Result<ListResult> {
                self.inner.list_with_delimiter(prefix).await
            }

            async fn copy_opts(
                &self,
                from: &Path,
                to: &Path,
                options: CopyOptions,
            ) -> object_store::Result<()> {
                self.inner.copy_opts(from, to, options).await
            }
        }
    }

    #[tokio::test]
    async fn pointers_replaced_by_larger_files_after_their_head_are_read_as_missing() {
        let scratch = tempfile::tempdir().unwrap();
        let store = scratch.path().join("store");
        let mut context = local_context(&store).await;
        let repo = TrainingRepository::open(scratch.path().join("ledger/jobs.sqlite")).unwrap();
        let project = repo
            .create_learning_project(learning_request(), now_ms())
            .unwrap();
        let pointer = format!("{}/{}", app_root(), learning_pointer_path(&project.id));
        let outcome = champion(&mut context, &repo, &project.id).await;
        assert!(
            !matches!(outcome, PublishOutcome::Failed { .. }),
            "{outcome:?}"
        );
        let published = std::fs::read(store.join(&pointer)).unwrap();
        let mut grown = published.clone();
        grown.resize(published.len() + MAXIMUM_POINTER_BYTES as usize, b' ');
        let swapping = swapping::SwappingStore {
            inner: local(&store).as_generic(),
            location: Path::from(pointer.as_str()),
            replacement: flow_like_types::Bytes::from(grown),
            swapped: Default::default(),
        };
        context
            .set_cache(
                &storage_key(),
                Arc::new(FlowLikeStore::Other(Arc::new(swapping))),
            )
            .await;
        let outcome = champion(&mut context, &repo, &project.id).await;
        assert!(
            !matches!(outcome, PublishOutcome::Failed { .. }),
            "{outcome:?}"
        );
        assert_eq!(
            std::fs::metadata(store.join(&pointer)).unwrap().len(),
            published.len() as u64,
            "the grown pointer was read whole instead of being rewritten"
        );
    }

    #[test]
    fn pointers_retain_three_previous_artifacts_and_release_the_oldest() {
        let mut previous: Option<(Option<String>, Vec<String>)> = None;
        let mut released = Vec::new();
        for current in ["a1", "a2", "a3", "a4", "a5", "a5"] {
            let view = previous
                .as_ref()
                .map(|(current, retained)| (current.as_deref(), retained.as_slice()));
            let retained = next_retained(view, Some(current));
            released = released_ids(view, Some(current), &retained);
            previous = Some((Some(current.to_owned()), retained));
        }
        assert_eq!(previous.unwrap().1, ["a4", "a3", "a2"]);
        assert!(released.is_empty(), "pausing keeps every retained artifact");
        let kept = vec!["a4".to_string(), "a3".into(), "a2".into()];
        let view = Some((Some("a5"), kept.as_slice()));
        let retained = next_retained(view, Some("a6"));
        assert_eq!(retained, ["a5", "a4", "a3"]);
        assert_eq!(released_ids(view, Some("a6"), &retained), ["a2"]);
    }

    fn folder_manifest(publisher: &str, dependency: Option<&str>) -> Value {
        json!({
            "format_version": 2,
            "weights": {"file": WEIGHTS_FILE, "sha256": "0".repeat(64), "bytes": 7},
            "dependencies": {"feature_extractor": dependency.map(|id| json!({"source_id": id, "blob_sha256": "0".repeat(64)}))},
            "provenance": {"publisher": publisher, "ledger": "app", "published_at_ms": 0}
        })
    }

    #[tokio::test]
    async fn pruning_deletes_only_released_unreferenced_automatic_folders() {
        let scratch = tempfile::tempdir().unwrap();
        let mut context = local_context(scratch.path()).await;
        for (id, publisher, dependency) in [
            ("kept", "auto", Some("extractor")),
            ("extractor", "auto", None),
            ("old", "auto", Some("old-extractor")),
            ("old-extractor", "auto", None),
            ("exported", "export", None),
        ] {
            let folder = canonical_folder(&app_root(), id);
            write_file(
                scratch.path(),
                &format!("{folder}/{WEIGHTS_FILE}"),
                b"weights",
            );
            write_file(
                scratch.path(),
                &format!("{folder}/{MANIFEST_FILE}"),
                &json::to_vec(&folder_manifest(publisher, dependency)).unwrap(),
            );
        }
        write_file(
            scratch.path(),
            &format!("{}/{}", app_root(), deployment_pointer_path("line")),
            &json::to_vec(&json!({"active_artifact_id": "kept", "retained": []})).unwrap(),
        );
        let root = FlowPath::from_storage_dir(&mut context, false)
            .await
            .unwrap();
        let target = WriteTarget::new(&mut context, root).await.unwrap();
        let released = ["old", "exported", "extractor", "kept"]
            .map(str::to_owned)
            .to_vec();
        prune_released(&mut context, &target, released)
            .await
            .unwrap();
        let present = |id: &str, file: &str| {
            scratch
                .path()
                .join(format!("{}/{file}", canonical_folder(&app_root(), id)))
                .exists()
        };
        assert!(present("kept", MANIFEST_FILE) && present("extractor", MANIFEST_FILE));
        assert!(present("exported", MANIFEST_FILE));
        for id in ["old", "old-extractor"] {
            assert!(
                !present(id, MANIFEST_FILE) && !present(id, WEIGHTS_FILE),
                "{id}"
            );
        }
    }

    #[tokio::test]
    async fn pruning_waits_for_a_pointer_write_in_the_same_root_and_keeps_what_it_names() {
        use futures::FutureExt;
        let scratch = tempfile::tempdir().unwrap();
        let mut context = local_context(scratch.path()).await;
        let manifest = format!("{}/{MANIFEST_FILE}", canonical_folder(&app_root(), "named"));
        write_file(
            scratch.path(),
            &manifest,
            &json::to_vec(&folder_manifest("auto", None)).unwrap(),
        );
        let root = FlowPath::from_storage_dir(&mut context, false)
            .await
            .unwrap();
        let target = WriteTarget::new(&mut context, root).await.unwrap();
        let writing = hold_root(&target).await;
        let released = vec!["named".to_owned()];
        let mut pruning = std::pin::pin!(prune_released(&mut context, &target, released));
        assert!(
            (&mut pruning).now_or_never().is_none(),
            "pruning ran while a pointer write held the root"
        );
        write_file(
            scratch.path(),
            &format!("{}/{}", app_root(), deployment_pointer_path("line")),
            &json::to_vec(&json!({"active_artifact_id": "named", "retained": []})).unwrap(),
        );
        drop(writing);
        pruning.await.unwrap();
        assert!(scratch.path().join(&manifest).exists());
    }

    /// Records `count` accepted samples of `shape` on `stream`.
    fn record_samples(
        repo: &TrainingRepository,
        stream: &StreamKey,
        count: usize,
        shape: &[usize],
        annotation: impl Fn(usize) -> Annotation,
    ) {
        let elements = shape.iter().product::<usize>();
        for index in 0..count {
            let time = index as i64;
            let sample = Sample {
                id: format!("{}-{index}", stream.stream_id),
                group_id: format!("{}-group-{index}", stream.stream_id),
                stream_id: stream.stream_id.clone(),
                timestamp_ms: time,
                window_start_ms: time,
                window_end_ms: time,
                input: TensorData {
                    shape: shape.to_vec(),
                    values: (0..elements)
                        .map(|value| ((index * elements + value) % 17) as f32 / 17.0)
                        .collect(),
                },
                annotation: annotation(index),
                provenance: LabelProvenance::Reviewed {
                    reviewer: "fixture".into(),
                    reviewed_at_ms: time,
                },
                outcome: None,
            };
            repo.record_sample(
                stream,
                &TrainingSample {
                    id: sample.id.clone(),
                    annotation_revision: 1,
                    group_id: sample.group_id.clone(),
                    captured_at_ms: time,
                    label_available_at_ms: time,
                    content_digest: flow_like_ml_core::content_digest(sample.id.as_bytes()),
                    source: LabelSource::Reviewed,
                    accepted: true,
                    payload: json::to_value(&sample).unwrap(),
                },
            )
            .unwrap();
        }
    }

    /// A small ResNet18 trained on 8x8 single-channel images.
    fn burn_backbone(repo: &TrainingRepository, project: &str) -> ModelArtifact {
        let stream = StreamKey {
            project_id: project.into(),
            stream_id: "backbone".into(),
            inspection_version: "backbone".into(),
        };
        record_samples(repo, &stream, 12, &[1, 8, 8], |index| Annotation::Class {
            class_id: (index % 2) as u32,
        });
        let config = flow_like_ml_burn::TrainingConfig {
            recipe: flow_like_ml_burn::Recipe::ResNet18 {
                input_channels: 1,
                classes: 2,
                base_channels: 2,
            },
            backend: flow_like_ml_burn::BackendChoice::Cpu,
            epochs: 1,
            batch_size: 2,
            learning_rate: 0.001,
            seed: 17,
            gradient_clip: 1.0,
        };
        let job = repo
            .trigger_after_n(
                &stream,
                12,
                TrainingRequest {
                    engine: "burn".into(),
                    recipe: json!({"config": config, "labels": ["good", "bad"], "pretrained": null, "preprocessing": []}),
                    compute: json!({"backend": "cpu"}),
                },
                SplitPolicy::Time {
                    train_end_ms: 5,
                    validation_end_ms: 8,
                    embargo_ms: 0,
                },
                100,
            )
            .unwrap()
            .unwrap();
        let mut worker = TrainingWorker::new(repo.clone(), WorkerLimits::default()).unwrap();
        register_burn_engine(&mut worker).unwrap();
        worker.run(&job.id).unwrap()
    }

    /// A PatchCore model fitted on embeddings that `extractor` computed.
    fn patchcore(
        repo: &TrainingRepository,
        project: &str,
        stream_id: &str,
        extractor: &FeatureExtractor,
    ) -> ModelArtifact {
        let recipe = json!({"labels": [], "coreset_size": 8, "feature_extractor": extractor});
        trained_patchcore(repo, project, stream_id, recipe)
    }

    /// A PatchCore model of `stream_id` fitted with `recipe`, which its manifest copies.
    fn trained_patchcore(
        repo: &TrainingRepository,
        project: &str,
        stream_id: &str,
        recipe: Value,
    ) -> ModelArtifact {
        let stream = StreamKey {
            project_id: project.into(),
            stream_id: stream_id.into(),
            inspection_version: stream_id.into(),
        };
        record_samples(repo, &stream, 8, &[4, 3], |_| Annotation::Anomaly {
            is_anomaly: false,
        });
        let job = repo
            .trigger_after_n(
                &stream,
                8,
                TrainingRequest {
                    engine: "patchcore".into(),
                    recipe,
                    compute: json!({"backend": "cpu"}),
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
        let mut worker = TrainingWorker::new(repo.clone(), WorkerLimits::default()).unwrap();
        register_native_engines(&mut worker).unwrap();
        worker.run(&job.id).unwrap()
    }

    /// A pretrained source of `project` with fixture weights, named as a feature extractor.
    fn fixture_extractor(
        repo: &TrainingRepository,
        project: &str,
    ) -> (PretrainedSource, FeatureExtractor) {
        let source = repo
            .register_pretrained_source(
                project,
                b"fixture extractor",
                burn_manifest(),
                json!({"source": "fixture"}),
                1024,
                now_ms(),
            )
            .unwrap();
        let extractor = FeatureExtractor::from_source(&source).unwrap();
        (source, extractor)
    }

    /// Generation `generation` of deployment `id`, serving `model` on its stream.
    fn deployment_of(id: &str, generation: i64, model: &ModelArtifact) -> Deployment {
        Deployment {
            id: id.into(),
            stream: model.stream.clone(),
            generation,
            active_artifact_id: model.id.clone(),
            paused: false,
            previous_artifact_id: None,
            evaluation_id: "evaluation".into(),
            policy: None,
            metric_policy: None,
            updated_at_ms: now_ms(),
        }
    }

    async fn deploy(
        context: &mut ExecutionContext,
        repo: &TrainingRepository,
        deployment: &Deployment,
        reason: PublishReason,
    ) -> PublishOutcome {
        ensure_deployment_published(context, repo, LedgerScope::App, deployment, reason).await
    }

    #[tokio::test]
    async fn extractor_dependencies_publish_once_and_serve_the_ledger_extractor() {
        let scratch = tempfile::tempdir().unwrap();
        let store = scratch.path().join("store");
        let mut context = local_context(&store).await;
        let repo = TrainingRepository::open(scratch.path().join("ledger/jobs.sqlite")).unwrap();
        let project = format!("{APP}:board");
        let (source, models) = {
            let (repo, project) = (repo.clone(), project.clone());
            tokio::task::spawn_blocking(move || {
                let backbone = burn_backbone(&repo, &project);
                let bytes = repo.read_blob(&backbone.blob).unwrap();
                let source = repo
                    .register_pretrained_source(
                        &project,
                        &bytes,
                        backbone.manifest.clone(),
                        json!({"source": "fixture"}),
                        bytes.len() as u64,
                        now_ms(),
                    )
                    .unwrap();
                let extractor = FeatureExtractor::from_source(&source).unwrap();
                let models = ["line-a", "line-b"]
                    .map(|stream| patchcore(&repo, &project, stream, &extractor));
                (source, models)
            })
            .await
            .unwrap()
        };
        let log = Recording::default();
        context
            .set_cache(
                &storage_key(),
                Arc::new(RecordingStore::wrap(local(&store).as_generic(), &log)),
            )
            .await;
        let source_folder = format!("{}/", canonical_folder(&app_root(), &source.id));
        for (index, model) in models.iter().enumerate() {
            log.clear();
            let deployment = Deployment {
                id: format!("line-{index}"),
                stream: model.stream.clone(),
                generation: 1,
                active_artifact_id: model.id.clone(),
                paused: false,
                previous_artifact_id: None,
                evaluation_id: "evaluation".into(),
                policy: None,
                metric_policy: None,
                updated_at_ms: now_ms(),
            };
            let outcome = ensure_deployment_published(
                &mut context,
                &repo,
                LedgerScope::App,
                &deployment,
                PublishReason::StateChange,
            )
            .await;
            assert!(
                matches!(outcome, PublishOutcome::Published(_)),
                "{outcome:?}"
            );
            let extractor_puts = log
                .puts()
                .into_iter()
                .filter(|path| path.starts_with(&source_folder))
                .count();
            assert_eq!(
                extractor_puts,
                if index == 0 { 2 } else { 0 },
                "{:?}",
                log.entries()
            );
        }
        let image = TensorData {
            shape: vec![1, 8, 8],
            values: (0..64).map(|value| value as f32 / 64.0).collect(),
        };
        let (features, extractor) = published_features(
            &mut context,
            &PublishedModelSource::Artifact {
                artifact_id: models[0].id.clone(),
                storage: None,
            },
            image.clone(),
            &cpu(),
        )
        .await
        .unwrap();
        assert_eq!(
            extractor,
            FeatureExtractorRef::from(FeatureExtractor::from_source(&source).unwrap())
        );
        let expected = flow_like_ml_runtime::engines::pretrained_source_features(
            &repo,
            &project,
            &source.id,
            &image,
            &cpu(),
        )
        .unwrap();
        assert_eq!(features.shape, expected.shape);
        assert_eq!(features.values, expected.values);
    }

    #[tokio::test]
    async fn app_storage_never_receives_a_feature_extractor_from_the_private_ledger() {
        let scratch = tempfile::tempdir().unwrap();
        let store = scratch.path().join("store");
        let mut context = local_context(&store).await;
        let (private, user_project) = scoped_repository(&context, LedgerScope::User).unwrap();
        let repo = TrainingRepository::open(scratch.path().join("ledger/jobs.sqlite")).unwrap();
        let (source, model) = {
            let repo = repo.clone();
            tokio::task::spawn_blocking(move || {
                let (source, extractor) = fixture_extractor(&private, &user_project);
                let model = patchcore(&repo, &format!("{APP}:board"), "line", &extractor);
                (source, model)
            })
            .await
            .unwrap()
        };
        let manifest = |root: &str, id: &str| {
            store.join(format!("{}/{MANIFEST_FILE}", canonical_folder(root, id)))
        };

        let shared = deployment_of("shared", 1, &model);
        let outcome = deploy(&mut context, &repo, &shared, PublishReason::StateChange).await;
        assert!(
            matches!(&outcome, PublishOutcome::Published(reference) if reference.storage == Some(ModelStorage::App)),
            "{outcome:?}"
        );
        assert!(manifest(&app_root(), &model.id).exists());
        assert!(
            !store
                .join(canonical_folder(&app_root(), &source.id))
                .exists(),
            "the private extractor reached app storage"
        );

        let own = deployment_of("own", 1, &model);
        let outcome = ensure_deployment_published(
            &mut context,
            &repo,
            LedgerScope::User,
            &own,
            PublishReason::StateChange,
        )
        .await;
        assert!(
            matches!(outcome, PublishOutcome::Published(_)),
            "{outcome:?}"
        );
        assert!(manifest(&user_root(), &source.id).exists());

        context
            .set_cache("exports", Arc::new(local(&scratch.path().join("exports"))))
            .await;
        export_model_folder(
            &mut context,
            &repo,
            LedgerScope::App,
            &LedgerModel::Artifact {
                artifact_id: model.id.clone(),
            },
            &FlowPath {
                path: "shared".into(),
                store_ref: "exports".into(),
                cache_store_ref: None,
            },
        )
        .await
        .unwrap();
        let exported = scratch
            .path()
            .join("exports/shared")
            .join(dependency_dir(&source.id))
            .join(MANIFEST_FILE);
        assert!(
            exported.exists(),
            "Export Model writes private extractors anywhere"
        );
    }

    #[tokio::test]
    async fn deployment_pointers_above_the_read_cap_are_never_written() {
        let scratch = tempfile::tempdir().unwrap();
        let store = scratch.path().join("store");
        let mut context = local_context(&store).await;
        let repo = TrainingRepository::open(scratch.path().join("ledger/jobs.sqlite")).unwrap();
        let model = {
            let repo = repo.clone();
            tokio::task::spawn_blocking(move || {
                let project = format!("{APP}:board");
                let (_, extractor) = fixture_extractor(&repo, &project);
                patchcore(&repo, &project, "line", &extractor)
            })
            .await
            .unwrap()
        };
        let recall = (0..2000)
            .map(|class| (format!("class-{class:05}-{}", "x".repeat(24)), 0.5))
            .collect::<BTreeMap<_, _>>();
        let deployment = Deployment {
            policy: Some(
                json::from_value(json!({
                    "minimum_audited_samples": 1, "minimum_accuracy": 0.5,
                    "minimum_class_recall": recall, "maximum_mean_latency_ms": 100.0,
                    "maximum_failure_rate": 0.1
                }))
                .unwrap(),
            ),
            ..deployment_of("line", 1, &model)
        };
        let outcome = deploy(&mut context, &repo, &deployment, PublishReason::StateChange).await;
        assert!(
            matches!(&outcome, PublishOutcome::Skipped { reason } if reason.starts_with(POINTER_TOO_LARGE)),
            "{outcome:?}"
        );
        let published = files_below(&store.join(format!("{}/models", app_root())));
        assert!(published.is_empty(), "{published:?}");
    }

    /// Every file below `path`; empty when it does not exist.
    fn files_below(path: &std::path::Path) -> Vec<std::path::PathBuf> {
        let Ok(entries) = std::fs::read_dir(path) else {
            return Vec::new();
        };
        entries
            .flatten()
            .flat_map(|entry| {
                let path = entry.path();
                if path.is_dir() {
                    files_below(&path)
                } else {
                    vec![path]
                }
            })
            .collect()
    }

    #[tokio::test]
    async fn a_deployment_whose_active_model_is_skipped_publishes_a_paused_pointer_once() {
        let scratch = tempfile::tempdir().unwrap();
        let store = scratch.path().join("store");
        let mut context = local_context(&store).await;
        let repo = TrainingRepository::open(scratch.path().join("ledger/jobs.sqlite")).unwrap();
        let (small, large) = {
            let repo = repo.clone();
            tokio::task::spawn_blocking(move || {
                let project = format!("{APP}:board");
                let (_, extractor) = fixture_extractor(&repo, &project);
                let small = patchcore(&repo, &project, "line-a", &extractor);
                let padded = json!({"labels": [], "coreset_size": 8, "note": "x".repeat(1 << 20)});
                let large = trained_patchcore(&repo, &project, "line-b", padded);
                (small, large)
            })
            .await
            .unwrap()
        };
        let pointer_file = store.join(format!(
            "{}/{}",
            app_root(),
            deployment_pointer_path("line")
        ));
        let read = || -> DeploymentPointer {
            json::from_slice(&std::fs::read(&pointer_file).unwrap()).unwrap()
        };
        let first = deployment_of("line", 1, &small);
        let outcome = deploy(&mut context, &repo, &first, PublishReason::StateChange).await;
        assert!(
            matches!(outcome, PublishOutcome::Published(_)),
            "{outcome:?}"
        );
        assert!(!read().paused);

        let promoted = Deployment {
            stream: small.stream.clone(),
            previous_artifact_id: Some(small.id.clone()),
            ..deployment_of("line", 2, &large)
        };
        let outcome = deploy(&mut context, &repo, &promoted, PublishReason::StateChange).await;
        assert!(
            matches!(&outcome, PublishOutcome::Skipped { reason } if reason.starts_with(MANIFEST_TOO_LARGE)),
            "{outcome:?}"
        );
        let pointer = read();
        assert_eq!(pointer.generation, 2);
        assert!(pointer.paused && pointer.deployment.paused);
        assert_eq!(pointer.active_artifact_id, large.id);
        assert_eq!(pointer.retained, vec![small.id.clone()]);
        assert!(
            !store
                .join(canonical_folder(&app_root(), &large.id))
                .exists()
        );
        let retained = canonical_folder(&app_root(), &small.id);
        assert!(store.join(format!("{retained}/{MANIFEST_FILE}")).exists());

        let log = Recording::default();
        context
            .set_cache(
                &storage_key(),
                Arc::new(RecordingStore::wrap(local(&store).as_generic(), &log)),
            )
            .await;
        context.cache.write().await.remove(CHECKED_KEY);
        let outcome = deploy(&mut context, &repo, &promoted, PublishReason::SelfRepair).await;
        assert!(
            matches!(outcome, PublishOutcome::Skipped { .. }),
            "{outcome:?}"
        );
        assert!(log.puts().is_empty(), "{:?}", log.entries());
        assert_eq!(read(), pointer);
    }

    #[tokio::test]
    async fn exported_winners_predict_through_files_like_the_ledger() {
        let mut harness = harness().await;
        let template = template(&mut harness).await;
        let repo = ledger(&harness, "app");
        let experiment =
            finished_copy(&template, &repo, auto_stream(), marked(LedgerScope::App)).await;
        let exports = local(&harness.scratch.path().join("exports"));
        harness
            .context
            .set_cache("exports", Arc::new(exports))
            .await;
        let destination = FlowPath {
            path: "winner".into(),
            store_ref: "exports".into(),
            cache_store_ref: None,
        };
        let model = LedgerModel::Experiment {
            experiment_id: experiment.id.clone(),
        };
        let reference = export_model_folder(
            &mut harness.context,
            &repo,
            LedgerScope::App,
            &model,
            &destination,
        )
        .await
        .unwrap();
        let written: PublishedModelManifest = json::from_slice(
            &std::fs::read(harness.scratch.path().join("exports/winner/manifest.json")).unwrap(),
        )
        .unwrap();
        assert_eq!(
            written.provenance.map(|provenance| provenance.publisher),
            Some(ModelPublisher::Export)
        );
        let snapshot = repo.get_snapshot(&experiment.request.snapshot_id).unwrap();
        let rows = snapshot
            .test
            .iter()
            .map(|sample| sample.payload["feature_row"].clone())
            .collect::<Vec<_>>();
        let published = predict_published_model(
            &mut harness.context,
            PredictPublishedModelRequest {
                model: PublishedModelSource::Files {
                    manifest: reference.manifest.clone(),
                    weights: None,
                },
                rows: rows.clone(),
                compute: cpu(),
                stateful_features: false,
                feature_state: None,
                feature_extractor: None,
            },
        )
        .await
        .unwrap();
        let result: AutoTrainingResult = repo.experiment_result(&experiment.id).unwrap().into();
        let artifact = result.model_bundle_ref.clone().unwrap();
        let predictor =
            CatalogLoadedPredictor::load(&repo, &artifact.id, &cpu().for_host()).unwrap();
        let calibration = AnomalyCalibration {
            scale: result
                .validation_metrics
                .get("anomaly_scale")
                .copied()
                .unwrap_or(1.0),
            threshold: 0.5,
        };
        let ledger_output = predict_table_rows(
            &predictor,
            Some(&result.preprocessing_manifest),
            Some(calibration),
            input(rows),
        )
        .unwrap();
        assert_eq!(published.model.artifact_id, artifact.id);
        assert_eq!(published.model.goal_met, Some(result.goal_met));
        assert_eq!(published.predictions, ledger_output.predictions);
        assert_eq!(
            json::to_value(published.anomaly).unwrap(),
            json::to_value(ledger_output.anomaly).unwrap()
        );
    }

    #[tokio::test]
    async fn import_model_registers_burn_models_in_the_chosen_ledger_and_refuses_other_engines() {
        let scratch = tempfile::tempdir().unwrap();
        let mut context = local_context(&scratch.path().join("store")).await;
        let exports = scratch.path().join("exports");
        context
            .set_cache("exports", Arc::new(local(&exports)))
            .await;
        let trainer = TrainingRepository::open(scratch.path().join("trainer/jobs.sqlite")).unwrap();
        let backbone = {
            let (repo, project) = (trainer.clone(), format!("{APP}:board"));
            tokio::task::spawn_blocking(move || burn_backbone(&repo, &project))
                .await
                .unwrap()
        };
        let exported = export_model_folder(
            &mut context,
            &trainer,
            LedgerScope::App,
            &LedgerModel::Artifact {
                artifact_id: backbone.id.clone(),
            },
            &FlowPath {
                path: "backbone".into(),
                store_ref: "exports".into(),
                cache_store_ref: None,
            },
        )
        .await
        .unwrap();
        let request = |private: bool| ImportModelRequest {
            model: PublishedModelSource::Files {
                manifest: exported.manifest.clone(),
                weights: None,
            },
            private,
            maximum_bytes: 512 * 1024 * 1024,
        };
        let shared = import_model(&mut context, request(false)).await.unwrap();
        let (app, app_project) = scoped_repository(&context, LedgerScope::App).unwrap();
        assert_eq!(shared.project_id, app_project);
        assert_eq!(
            app.get_pretrained_source(&shared.id).unwrap().blob.sha256,
            backbone.blob.sha256
        );
        let private = import_model(&mut context, request(true)).await.unwrap();
        let (user, user_project) = scoped_repository(&context, LedgerScope::User).unwrap();
        assert_eq!(private.project_id, user_project);
        assert!(user.get_pretrained_source(&private.id).is_ok());
        assert!(app.get_pretrained_source(&private.id).is_err());

        let (forest_artifact, bytes) = forest(&[]);
        write_model(&exports, "forest", &manifest_of(&forest_artifact), &bytes);
        let refused = import_model(
            &mut context,
            ImportModelRequest {
                model: PublishedModelSource::Files {
                    manifest: FlowPath {
                        path: "forest/manifest.json".into(),
                        store_ref: "exports".into(),
                        cache_store_ref: None,
                    },
                    weights: None,
                },
                private: false,
                maximum_bytes: 1024 * 1024,
            },
        )
        .await
        .err()
        .expect("isolation forests are not pretrained sources")
        .to_string();
        assert!(refused.contains("Burn and SAM 2"), "{refused}");
    }

    fn transfer_path(path: &str) -> FlowPath {
        FlowPath {
            path: path.into(),
            store_ref: "transfer".into(),
            cache_store_ref: None,
        }
    }

    /// A device of the app named `name`; every device reaches the shared transfer folder.
    async fn device(scratch: &std::path::Path, name: &str) -> ExecutionContext {
        let context = local_context(&scratch.join(name)).await;
        context
            .set_cache("transfer", Arc::new(local(&scratch.join("transfer"))))
            .await;
        context
    }

    fn copy_folder(from: &std::path::Path, to: &std::path::Path) {
        std::fs::create_dir_all(to).unwrap();
        for entry in std::fs::read_dir(from).unwrap() {
            let entry = entry.unwrap();
            let target = to.join(entry.file_name());
            if entry.file_type().unwrap().is_dir() {
                copy_folder(&entry.path(), &target);
            } else {
                std::fs::copy(entry.path(), target).unwrap();
            }
        }
    }

    fn import_request(folder: &str, private: bool) -> ImportTrainingLedgerRequest {
        ImportTrainingLedgerRequest {
            source: transfer_path(folder),
            private,
            maximum_bytes: 1024 * 1024 * 1024,
        }
    }

    async fn refused_import(context: &mut ExecutionContext, folder: &str) -> String {
        import_training_ledger(context, import_request(folder, false))
            .await
            .err()
            .expect("the import is refused")
            .to_string()
    }

    fn holds_no_ledger(context: &ExecutionContext) -> bool {
        !LedgerLocation::new(context, LedgerScope::App)
            .unwrap()
            .exists()
            .unwrap()
    }

    /// Fills the App ledger of `context` with two pretrained sources of `project` and exports
    /// it to the transfer folder `folder`.
    async fn exported_ledger(
        context: &mut ExecutionContext,
        project: Option<&str>,
        folder: &str,
    ) -> (
        TrainingRepository,
        Vec<PretrainedSource>,
        crate::inspection::ledger_transfer::TrainingLedgerTransfer,
    ) {
        let (repo, own_project) = scoped_repository(context, LedgerScope::App).unwrap();
        let project = project.map_or(own_project, str::to_owned);
        let sources = ["one", "two"]
            .map(|name| {
                repo.register_pretrained_source(
                    &project,
                    name.as_bytes(),
                    burn_manifest(),
                    json!({"source": name}),
                    1024,
                    now_ms(),
                )
                .unwrap()
            })
            .to_vec();
        let exported = export_training_ledger(
            context,
            ExportTrainingLedgerRequest {
                destination: transfer_path(folder),
                private: false,
            },
        )
        .await
        .unwrap();
        (repo, sources, exported)
    }

    #[tokio::test]
    async fn training_ledgers_round_trip_into_an_empty_ledger_of_the_same_app() {
        let scratch = tempfile::tempdir().unwrap();
        let mut origin = device(scratch.path(), "origin").await;
        let (repo, sources, exported) = exported_ledger(&mut origin, None, "ledger").await;
        assert_eq!(exported.files, 3);
        assert_eq!(exported.project_ids, vec![format!("{APP}:board")]);
        let mut target = device(scratch.path(), "target").await;
        let imported = import_training_ledger(&mut target, import_request("ledger", false))
            .await
            .unwrap();
        assert_eq!(imported, exported);
        let (installed, _) = scoped_repository(&target, LedgerScope::App).unwrap();
        assert_eq!(
            installed.project_ids().unwrap(),
            repo.project_ids().unwrap()
        );
        for source in &sources {
            let copied = installed.get_pretrained_source(&source.id).unwrap();
            assert_eq!(copied.project_id, source.project_id);
            assert_eq!(
                installed.read_blob(&copied.blob).unwrap(),
                repo.read_blob(&source.blob).unwrap()
            );
        }
        let again = refused_import(&mut target, "ledger").await;
        assert!(again.contains("already exists"), "{again}");
        let mut private = device(scratch.path(), "private").await;
        let mismatch = import_training_ledger(&mut private, import_request("ledger", true))
            .await
            .err()
            .expect("an app ledger does not fill a private ledger")
            .to_string();
        assert!(mismatch.contains("set private to match"), "{mismatch}");
    }

    #[tokio::test]
    async fn imports_refuse_foreign_apps_existing_legacy_ledgers_and_damaged_snapshots() {
        let scratch = tempfile::tempdir().unwrap();
        let transfer = scratch.path().join("transfer");
        let mut origin = device(scratch.path(), "origin").await;
        exported_ledger(&mut origin, None, "ledger").await;

        let mut legacy = device(scratch.path(), "legacy").await;
        TrainingRepository::open(
            scratch
                .path()
                .join("legacy")
                .join(app_root())
                .join("inspection")
                .join("jobs.sqlite"),
        )
        .unwrap();
        let error = refused_import(&mut legacy, "ledger").await;
        assert!(error.contains("already exists"), "{error}");

        copy_folder(&transfer.join("ledger"), &transfer.join("missing"));
        let mut manifest: Value =
            json::from_slice(&std::fs::read(transfer.join("missing/ledger.json")).unwrap())
                .unwrap();
        let removed = manifest["blobs"].as_array_mut().unwrap().remove(0);
        std::fs::remove_file(
            transfer
                .join("missing")
                .join(removed["file"].as_str().unwrap()),
        )
        .unwrap();
        std::fs::write(
            transfer.join("missing/ledger.json"),
            json::to_vec(&manifest).unwrap(),
        )
        .unwrap();
        let mut missing = device(scratch.path(), "missing-target").await;
        refused_import(&mut missing, "missing").await;
        assert!(holds_no_ledger(&missing));

        copy_folder(&transfer.join("ledger"), &transfer.join("tampered"));
        let manifest: Value =
            json::from_slice(&std::fs::read(transfer.join("tampered/ledger.json")).unwrap())
                .unwrap();
        let blob = transfer
            .join("tampered")
            .join(manifest["blobs"][0]["file"].as_str().unwrap());
        let mut bytes = std::fs::read(&blob).unwrap();
        bytes[0] ^= 1;
        std::fs::write(&blob, bytes).unwrap();
        let mut tampered = device(scratch.path(), "tampered-target").await;
        refused_import(&mut tampered, "tampered").await;
        assert!(holds_no_ledger(&tampered));

        let mut foreign = device(scratch.path(), "foreign").await;
        exported_ledger(&mut foreign, Some("other-app:board"), "foreign").await;
        let mut target = device(scratch.path(), "foreign-target").await;
        let error = refused_import(&mut target, "foreign").await;
        assert!(error.contains("another app"), "{error}");
        assert!(holds_no_ledger(&target));
    }
}
