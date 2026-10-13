use super::*;

/// One immutable artifact loaded for repeated inference. Cache this within the
/// owning workflow scope; loading a new artifact creates a separate predictor.
pub struct LoadedArtifactPredictor {
    artifact: ModelArtifact,
    compute: ComputeConfig,
    requested_compute: ComputeConfig,
    expected: Vec<usize>,
    input_limit: usize,
    model: LoadedModel,
}

enum LoadedModel {
    #[cfg(feature = "native")]
    Native(NativeModel),
    #[cfg(feature = "burn")]
    Burn {
        predictor: Box<flow_like_ml_burn::Predictor>,
        /// Samples per forward pass that keep attention workspaces within the budget.
        batch_limit: usize,
        // Keep extracted files alive for weight records backed by file mappings.
        _directory: tempfile::TempDir,
    },
    #[cfg(feature = "burn")]
    EfficientAd {
        predictor: Box<flow_like_ml_burn::EfficientAdPredictor>,
        _directory: tempfile::TempDir,
    },
}

/// Manifest facts both loaders check before any blob byte is read.
struct ArtifactContract {
    engine: String,
    expected: Vec<usize>,
}

impl ArtifactContract {
    fn of(artifact: &ModelArtifact) -> Result<Self> {
        let engine = artifact.manifest["engine"]
            .as_str()
            .ok_or_else(|| invalid("model artifact has no engine"))?;
        if artifact.manifest["format_version"].as_u64() != Some(1) {
            return Err(invalid("unsupported model artifact format version"));
        }
        let expected: Vec<usize> =
            serde_json::from_value(artifact.manifest["input_shape"].clone())?;
        if expected.is_empty()
            || expected.len() > 7
            || expected.contains(&0)
            || expected
                .iter()
                .try_fold(1usize, |n, d| n.checked_mul(*d))
                .is_none_or(|n| n > 64 * 1024 * 1024)
        {
            return Err(invalid("invalid artifact input shape"));
        }
        let labels: Vec<String> = serde_json::from_value(
            artifact
                .manifest
                .get("labels")
                .cloned()
                .unwrap_or_else(|| serde_json::json!([])),
        )?;
        flow_like_ml_core::validate_labels(&labels).map_err(engine_error)?;
        Ok(Self {
            engine: engine.to_owned(),
            expected,
        })
    }
}

/// The envelope, decoded parameters and load-time copies share the budget.
fn artifact_file_limit(compute: &ComputeConfig) -> u64 {
    (compute.memory_limit_bytes / 4).min(512 * 1024 * 1024)
}

impl LoadedArtifactPredictor {
    pub fn load(
        repository: &TrainingRepository,
        artifact_id: &str,
        compute: &ComputeConfig,
    ) -> Result<Self> {
        compute.validate().map_err(engine_error)?;
        let artifact = repository.get_artifact(artifact_id)?;
        let contract = ArtifactContract::of(&artifact)?;
        let bytes = repository.read_blob_limited(&artifact.blob, artifact_file_limit(compute))?;
        Self::decode(artifact, contract, &bytes, compute)
    }

    /// Loads an artifact whose blob bytes come from outside a ledger, such as a
    /// published model folder. It never checks a project id. The bytes pass the
    /// repository blob rules within `load`'s file limit before anything decodes them.
    pub fn from_parts(
        artifact: ModelArtifact,
        bytes: &[u8],
        compute: &ComputeConfig,
    ) -> Result<Self> {
        compute.validate().map_err(engine_error)?;
        let contract = ArtifactContract::of(&artifact)?;
        verify_artifact_bytes(&artifact.blob, bytes, artifact_file_limit(compute))?;
        Self::decode(artifact, contract, bytes, compute)
    }

    /// `bytes` are the artifact's verified blob.
    fn decode(
        artifact: ModelArtifact,
        contract: ArtifactContract,
        bytes: &[u8],
        compute: &ComputeConfig,
    ) -> Result<Self> {
        let ArtifactContract { engine, expected } = contract;
        let engine = engine.as_str();
        let reserved = artifact
            .blob
            .bytes
            .checked_mul(4)
            .ok_or_else(|| invalid("artifact memory estimate overflows"))?;
        let available = compute.memory_limit_bytes.saturating_sub(reserved);
        let input_limit = match engine {
            #[cfg(feature = "burn")]
            "efficient_ad" => efficient_ad_input_limit(&expected, available)?,
            _ => (available / 64).min(64 * 1024 * 1024) as usize,
        };
        if input_limit == 0 || expected.iter().product::<usize>() > input_limit {
            return Err(invalid(
                "memory budget has no room for a trained input tensor and inference workspace",
            ));
        }
        let effective_compute;
        let model = match engine {
            "burn" => {
                #[cfg(feature = "burn")]
                {
                    let directory = tempfile::tempdir()?;
                    burn_adapter::unpack_named(
                        bytes,
                        directory.path(),
                        flow_like_ml_burn::MANIFEST_FILE,
                        artifact_file_limit(compute),
                    )?;
                    let state: Value = serde_json::from_slice(&std::fs::read(
                        directory.path().join(flow_like_ml_burn::MANIFEST_FILE),
                    )?)?;
                    if state["input_shape"] != artifact.manifest["input_shape"]
                        || !burn_adapter::same_training_config(
                            &state["config"],
                            &artifact.manifest["config"],
                        )
                    {
                        return Err(invalid(
                            "Burn artifact manifest differs from packed model state",
                        ));
                    }
                    let config: flow_like_ml_burn::TrainingConfig =
                        serde_json::from_value(state["config"].clone())?;
                    let batch_limit =
                        burn_adapter::inference_batch_limit(&config.recipe, &expected, available)?;
                    let choice = burn_adapter::backend(compute)?;
                    effective_compute = burn_adapter::concrete_compute(compute, &choice)?;
                    let predictor = flow_like_ml_burn::Predictor::load(directory.path(), choice)
                        .map_err(engine_error)?;
                    LoadedModel::Burn {
                        predictor: Box::new(predictor),
                        batch_limit,
                        _directory: directory,
                    }
                }
                #[cfg(not(feature = "burn"))]
                {
                    return Err(engine_error("Burn support is not compiled"));
                }
            }
            "efficient_ad" => {
                #[cfg(feature = "burn")]
                {
                    let directory = tempfile::tempdir()?;
                    let manifest = "efficient-ad-state.json";
                    burn_adapter::unpack_named(
                        bytes,
                        directory.path(),
                        manifest,
                        artifact_file_limit(compute),
                    )?;
                    let state: Value =
                        serde_json::from_slice(&std::fs::read(directory.path().join(manifest))?)?;
                    if state["architecture"] != artifact.manifest["architecture"]
                        || expected.len() != 3
                        || expected[1..] != [256, 256]
                        || state["architecture"]["input"].as_u64() != Some(expected[0] as u64)
                    {
                        return Err(invalid(
                            "EfficientAD artifact manifest differs from packed model state",
                        ));
                    }
                    let choice = burn_adapter::backend(compute)?;
                    effective_compute = burn_adapter::concrete_compute(compute, &choice)?;
                    let predictor =
                        flow_like_ml_burn::EfficientAdPredictor::load(directory.path(), choice)
                            .map_err(engine_error)?;
                    LoadedModel::EfficientAd {
                        predictor: Box::new(predictor),
                        _directory: directory,
                    }
                }
                #[cfg(not(feature = "burn"))]
                {
                    return Err(engine_error("EfficientAD needs Burn support"));
                }
            }
            "isolation_forest" | "histogram_gradient_boosting" | "patchcore" | "padim" => {
                #[cfg(feature = "native")]
                {
                    validate_native_compute(&serde_json::to_value(compute)?)?;
                    effective_compute = ComputeConfig {
                        backend: flow_like_ml_core::ComputeBackend::Cpu,
                        device_index: 0,
                        allow_cpu_fallback: false,
                        memory_limit_bytes: compute.memory_limit_bytes,
                    };
                    let model: NativeModel = serde_json::from_slice(bytes)?;
                    let actual = match &model {
                        NativeModel::IsolationForest(_) => "isolation_forest",
                        NativeModel::HistogramGradientBoosting(_) => "histogram_gradient_boosting",
                        NativeModel::Patchcore(_) => "patchcore",
                        NativeModel::Padim(_) => "padim",
                    };
                    if actual != engine {
                        return Err(invalid(
                            "native artifact engine differs from serialized model",
                        ));
                    }
                    if matches!(model, NativeModel::Patchcore(_) | NativeModel::Padim(_))
                        && expected.len() != 2
                    {
                        return Err(invalid(
                            "patch anomaly input shape must be [patches, channels]",
                        ));
                    }
                    LoadedModel::Native(model)
                }
                #[cfg(not(feature = "native"))]
                {
                    return Err(engine_error("native model support is not compiled"));
                }
            }
            _ => return Err(invalid("unsupported artifact engine")),
        };
        Ok(Self {
            artifact,
            compute: effective_compute,
            requested_compute: compute.clone(),
            expected,
            input_limit,
            model,
        })
    }

    pub fn artifact(&self) -> &ModelArtifact {
        &self.artifact
    }
    /// Concrete backend and device pinned when this predictor was loaded.
    pub fn compute(&self) -> &ComputeConfig {
        &self.compute
    }

    /// Original preference, including Auto and the caller's fallback policy.
    pub fn requested_compute(&self) -> &ComputeConfig {
        &self.requested_compute
    }

    /// Dense, temporal and image models include a batch dimension. Native models
    /// also accept a single example with exactly the trained sample shape.
    pub fn predict(&self, input: &TensorData) -> Result<Value> {
        input.validate(self.input_limit).map_err(engine_error)?;
        let batch_shape =
            input.shape.len() == self.expected.len() + 1 && input.shape[1..] == self.expected;
        match &self.model {
            #[cfg(feature = "native")]
            LoadedModel::Native(model) => {
                if input.shape != self.expected && !batch_shape {
                    return Err(invalid("inference tensor shape differs from training"));
                }
                let width = self.expected.iter().product::<usize>();
                let mut scores = Vec::new();
                let mut patch_scores = Vec::new();
                for values in input.values.chunks_exact(width) {
                    let row = values.iter().map(|v| *v as f64).collect::<Vec<_>>();
                    match model {
                        NativeModel::IsolationForest(model) => {
                            scores.push(model.score(&row).map_err(engine_error)?)
                        }
                        NativeModel::HistogramGradientBoosting(model) => {
                            scores.push(model.predict(&row).map_err(engine_error)?)
                        }
                        NativeModel::Patchcore(model) => {
                            let patches = row
                                .chunks_exact(self.expected[1])
                                .map(|v| v.to_vec())
                                .collect::<Vec<_>>();
                            let values = model.score_patches(&patches).map_err(engine_error)?;
                            scores.push(values.iter().copied().fold(0., f64::max));
                            patch_scores.push(values);
                        }
                        NativeModel::Padim(model) => {
                            let patches = row
                                .chunks_exact(self.expected[1])
                                .map(|v| v.to_vec())
                                .collect::<Vec<_>>();
                            let values = model.score_patches(&patches).map_err(engine_error)?;
                            scores.push(values.iter().copied().fold(0., f64::max));
                            patch_scores.push(values);
                        }
                    }
                }
                if scores
                    .iter()
                    .chain(patch_scores.iter().flatten())
                    .any(|v| !v.is_finite())
                {
                    return Err(engine_error(
                        "model produced non-finite anomaly or regression scores",
                    ));
                }
                let mut result = serde_json::json!({"engine":self.artifact.manifest["engine"],"scores":scores,"patch_scores":patch_scores,"labels":self.artifact.manifest["labels"]});
                // Boosting fitted on one-element value targets answers in the dense shape.
                if matches!(model, NativeModel::HistogramGradientBoosting(_))
                    && self.artifact.manifest["output_shape"] == serde_json::json!([1])
                {
                    result["output"] =
                        serde_json::json!({"shape":[scores.len(), 1],"values":scores});
                }
                Ok(result)
            }
            #[cfg(feature = "burn")]
            LoadedModel::Burn {
                predictor,
                batch_limit,
                ..
            } => {
                if !batch_shape {
                    return Err(invalid(
                        "Burn inference tensor needs a batch dimension matching training",
                    ));
                }
                let mut results = batch_chunks(input, *batch_limit)
                    .map(|chunk| predictor.predict(&chunk).map_err(engine_error));
                let mut merged = results
                    .next()
                    .ok_or_else(|| invalid("inference batch is empty"))??;
                for result in results {
                    merge_predictions(&mut merged, result?)?;
                }
                Ok(serde_json::to_value(merged)?)
            }
            #[cfg(feature = "burn")]
            LoadedModel::EfficientAd { predictor, .. } => {
                if !batch_shape {
                    return Err(invalid(
                        "EfficientAD inference tensor needs a batch dimension matching training",
                    ));
                }
                let result = predictor
                    .predict(&flow_like_ml_burn::TensorData {
                        shape: input.shape.clone(),
                        values: input.values.clone(),
                    })
                    .map_err(engine_error)?;
                Ok(serde_json::to_value(result)?)
            }
        }
    }

    #[cfg(feature = "burn")]
    pub fn features(&self, input: &TensorData) -> Result<TensorData> {
        input.validate(self.input_limit).map_err(engine_error)?;
        if input.shape.len() != self.expected.len() + 1 || input.shape[1..] != self.expected {
            return Err(invalid("feature tensor shape differs from training"));
        }
        let LoadedModel::Burn {
            predictor,
            batch_limit,
            ..
        } = &self.model
        else {
            return Err(invalid("spatial features require a Burn image classifier"));
        };
        let mut outputs = batch_chunks(input, *batch_limit)
            .map(|chunk| predictor.features(&chunk).map_err(engine_error));
        let mut output = outputs
            .next()
            .ok_or_else(|| invalid("feature batch is empty"))??;
        for chunk in outputs {
            concatenate_batch(&mut output, chunk?)?;
        }
        let tensor = TensorData {
            shape: output.shape,
            values: output.values,
        };
        tensor.validate(self.input_limit).map_err(engine_error)?;
        Ok(tensor)
    }

    /// Identity of this model as the extractor of the features it computes.
    pub fn feature_extractor(&self) -> Result<FeatureExtractor> {
        FeatureExtractor::from_artifact(&self.artifact)
    }

    /// Score embeddings after checking they come from the extractor and image steps the
    /// model was fitted on. Models fitted without a recorded extractor cannot be checked.
    pub fn predict_with_extractor(
        &self,
        input: &TensorData,
        extractor: &FeatureExtractor,
    ) -> Result<Value> {
        if let Some(recorded) = self
            .artifact
            .manifest
            .get("feature_extractor")
            .filter(|value| !value.is_null())
        {
            let recorded: FeatureExtractor = serde_json::from_value(recorded.clone())?;
            if recorded != *extractor {
                return Err(invalid(format!(
                    "embeddings from extractor {} ({}) differ from the extractor {} ({}) or image steps this model was fitted on; recompute them with the recorded extractor",
                    extractor.source_id,
                    extractor.blob_sha256,
                    recorded.source_id,
                    recorded.blob_sha256
                )));
            }
        }
        self.predict(input)
    }
}

/// Applies the repository blob rules (`read_blob_limited`) to bytes read from
/// anywhere else: reference shape, size bounds, exact length and sha256.
pub fn verify_artifact_bytes(blob: &BlobRef, bytes: &[u8], maximum_bytes: u64) -> Result<()> {
    crate::repository::validate_blob_reference(blob, maximum_bytes)?;
    crate::repository::verify_blob_content(blob, bytes)
}

/// Memory one EfficientAD image adds beyond the workspace: its f32 input and the three
/// 256x256 output maps, both as f32 and as the JSON numbers they are returned in.
#[cfg(feature = "burn")]
fn efficient_ad_image_bytes(image_elements: u64) -> u64 {
    let map_value = (size_of::<f32>() + size_of::<Value>()) as u64;
    image_elements * size_of::<f32>() as u64 + 3 * 256 * 256 * map_value
}

/// Input elements an EfficientAD request may carry within `available_bytes`.
#[cfg(feature = "burn")]
fn efficient_ad_input_limit(expected: &[usize], available_bytes: u64) -> Result<usize> {
    let workspace = flow_like_ml_burn::EFFICIENT_AD_INFERENCE_WORKSPACE_BYTES;
    let image_elements = expected.iter().product::<usize>() as u64;
    let per_image = efficient_ad_image_bytes(image_elements);
    let images = available_bytes.saturating_sub(workspace) / per_image;
    if images == 0 {
        return Err(invalid(format!(
            "EfficientAD inference needs an estimated {} bytes for one image, but only {available_bytes} bytes remain after loading the model",
            workspace + per_image
        )));
    }
    Ok(images.saturating_mul(image_elements).min(64 * 1024 * 1024) as usize)
}

/// Split a batch along its first dimension into forward passes of at most `limit` samples.
#[cfg(feature = "burn")]
fn batch_chunks(
    input: &TensorData,
    limit: usize,
) -> impl Iterator<Item = flow_like_ml_burn::TensorData> {
    let width = input.values.len() / input.shape[0];
    let shape = input.shape.clone();
    input
        .values
        .chunks(width * limit.clamp(1, input.shape[0]))
        .map(move |values| {
            let mut shape = shape.clone();
            shape[0] = values.len() / width;
            flow_like_ml_burn::TensorData {
                shape,
                values: values.to_vec(),
            }
        })
}

#[cfg(feature = "burn")]
fn concatenate_batch(
    merged: &mut flow_like_ml_burn::TensorData,
    chunk: flow_like_ml_burn::TensorData,
) -> Result<()> {
    if merged.shape.is_empty() || merged.shape.get(1..) != chunk.shape.get(1..) {
        return Err(engine_error(
            "model returned differently shaped outputs for one batch",
        ));
    }
    merged.shape[0] += chunk.shape[0];
    merged.values.extend(chunk.values);
    Ok(())
}

#[cfg(feature = "burn")]
fn extend_batch<T>(merged: &mut Option<Vec<T>>, chunk: Option<Vec<T>>) -> Result<()> {
    match (merged.as_mut(), chunk) {
        (Some(merged), Some(chunk)) => merged.extend(chunk),
        (None, None) => {}
        _ => {
            return Err(engine_error(
                "model returned inconsistent outputs for one batch",
            ));
        }
    }
    Ok(())
}

/// Every prediction field is batch-major, so chunks concatenate in input order.
#[cfg(feature = "burn")]
fn merge_predictions(
    merged: &mut flow_like_ml_burn::PredictionBatch,
    chunk: flow_like_ml_burn::PredictionBatch,
) -> Result<()> {
    let flow_like_ml_burn::PredictionBatch {
        output,
        classes,
        reconstruction_error,
        detections,
        instances,
    } = chunk;
    concatenate_batch(&mut merged.output, output)?;
    extend_batch(&mut merged.classes, classes)?;
    extend_batch(&mut merged.reconstruction_error, reconstruction_error)?;
    extend_batch(&mut merged.detections, detections)?;
    extend_batch(&mut merged.instances, instances)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn predictor_can_be_cached_by_a_thread_safe_workflow_context() {
        fn send_sync<T: Send + Sync>() {}
        send_sync::<LoadedArtifactPredictor>();
    }

    fn publish(
        repository: &TrainingRepository,
        version: usize,
        bytes: &[u8],
        manifest: Value,
    ) -> ModelArtifact {
        let stream = StreamKey {
            project_id: "test".into(),
            stream_id: "cached".into(),
            inspection_version: version.to_string(),
        };
        for i in 0..9 {
            repository
                .record_sample(
                    &stream,
                    &TrainingSample {
                        id: i.to_string(),
                        annotation_revision: 1,
                        group_id: i.to_string(),
                        captured_at_ms: i,
                        label_available_at_ms: i,
                        content_digest: format!("{i:064x}"),
                        source: LabelSource::Reviewed,
                        accepted: true,
                        payload: serde_json::json!({"label":"good"}),
                    },
                )
                .unwrap();
        }
        let job = repository
            .trigger_after_n(
                &stream,
                9,
                TrainingRequest {
                    engine: "test".into(),
                    recipe: serde_json::json!({}),
                    compute: serde_json::json!({}),
                },
                SplitPolicy::Group {
                    train_fraction: 0.6,
                    validation_fraction: 0.2,
                    seed: 42,
                },
                100,
            )
            .unwrap()
            .unwrap();
        let lease = repository.claim_job(&job.id, "test", 1000, 100).unwrap();
        repository
            .finish_training(&lease, bytes, manifest, 101)
            .unwrap()
    }

    #[test]
    #[cfg(feature = "native")]
    fn native_predictor_is_version_pinned_reusable_and_bounded() {
        use flow_like_ml_native::models::{BoostConfig, HistogramGradientBoosting};
        let directory = tempfile::tempdir().unwrap();
        let repository =
            TrainingRepository::open(directory.path().join("training.sqlite")).unwrap();
        let rows = vec![vec![-1.0], vec![0.0], vec![1.0]];
        let input = TensorData {
            shape: vec![2, 1],
            values: vec![-0.5, 0.5],
        };
        let mut loaded = Vec::new();
        for version in 0..2 {
            let model = NativeModel::HistogramGradientBoosting(
                HistogramGradientBoosting::fit(
                    &rows,
                    &vec![version as f64 * 10.0; 3],
                    &BoostConfig::default(),
                )
                .unwrap(),
            );
            let bytes = serde_json::to_vec(&model).unwrap();
            let artifact = publish(
                &repository,
                version,
                &bytes,
                serde_json::json!({"engine":"histogram_gradient_boosting","format_version":1,"input_shape":[1],"labels":[]}),
            );
            let predictor =
                LoadedArtifactPredictor::load(&repository, &artifact.id, &ComputeConfig::default())
                    .unwrap();
            assert_eq!(
                predictor.requested_compute().backend,
                flow_like_ml_core::ComputeBackend::Auto
            );
            assert_eq!(
                predictor.compute().backend,
                flow_like_ml_core::ComputeBackend::Cpu
            );
            assert!(!predictor.compute().allow_cpu_fallback);
            assert_eq!(
                predictor.predict(&input).unwrap(),
                load_artifact_predictions(
                    &repository,
                    &artifact.id,
                    &input,
                    &ComputeConfig::default()
                )
                .unwrap()
            );
            assert!(
                LoadedArtifactPredictor::load(
                    &repository,
                    &artifact.id,
                    &ComputeConfig {
                        memory_limit_bytes: 16,
                        ..Default::default()
                    }
                )
                .is_err()
            );
            std::fs::remove_file(
                repository
                    .path()
                    .with_extension("artifacts")
                    .join(&artifact.blob.path),
            )
            .unwrap();
            assert!(
                LoadedArtifactPredictor::load(&repository, &artifact.id, &ComputeConfig::default())
                    .is_err()
            );
            loaded.push(predictor);
        }
        assert_ne!(loaded[0].artifact().id, loaded[1].artifact().id);
        assert_eq!(
            loaded[0].predict(&input).unwrap()["scores"],
            serde_json::json!([0.0, 0.0])
        );
        assert_eq!(
            loaded[1].predict(&input).unwrap()["scores"],
            serde_json::json!([10.0, 10.0])
        );
        assert!(
            loaded[0]
                .predict(&TensorData {
                    shape: vec![1, 2],
                    values: vec![0.0, 0.0]
                })
                .is_err()
        );
    }

    #[test]
    #[cfg(feature = "native")]
    fn anomaly_predictors_preserve_single_sample_and_batch_patch_geometry() {
        use flow_like_ml_native::models::{
            IsolationForest, IsolationForestConfig, Padim, PatchCore,
        };
        let directory = tempfile::tempdir().unwrap();
        let repository =
            TrainingRepository::open(directory.path().join("training.sqlite")).unwrap();
        let images = vec![
            vec![vec![0.0, 0.0], vec![0.2, 0.1]],
            vec![vec![0.1, 0.1], vec![0.3, 0.2]],
        ];
        let rows: Vec<Vec<f64>> = images
            .iter()
            .map(|image| image.iter().flatten().copied().collect())
            .collect();
        let patches = images.iter().flatten().cloned().collect::<Vec<_>>();
        let models = [
            (
                "isolation_forest",
                NativeModel::IsolationForest(
                    IsolationForest::fit(
                        &rows,
                        &IsolationForestConfig {
                            trees: 4,
                            sample_size: 2,
                            seed: 7,
                        },
                    )
                    .unwrap(),
                ),
            ),
            (
                "patchcore",
                NativeModel::Patchcore(PatchCore::fit(&patches, 2).unwrap()),
            ),
            (
                "padim",
                NativeModel::Padim(Padim::fit(&images, 0.01).unwrap()),
            ),
        ];
        let single = TensorData {
            shape: vec![2, 2],
            values: vec![0.0, 0.0, 0.2, 0.1],
        };
        let mut values = single.values.clone();
        values.extend_from_slice(&[10.0, 10.0, 10.0, 10.0]);
        let batch = TensorData {
            shape: vec![2, 2, 2],
            values,
        };
        for (version, (engine, model)) in models.into_iter().enumerate() {
            let artifact = publish(
                &repository,
                version,
                &serde_json::to_vec(&model).unwrap(),
                serde_json::json!({"engine":engine,"format_version":1,"input_shape":[2,2],"labels":[]}),
            );
            let predictor =
                LoadedArtifactPredictor::load(&repository, &artifact.id, &ComputeConfig::default())
                    .unwrap();
            let one = predictor.predict(&single).unwrap();
            let many = predictor.predict(&batch).unwrap();
            assert_eq!(one["scores"].as_array().unwrap().len(), 1);
            assert_eq!(many["scores"].as_array().unwrap().len(), 2);
            assert_eq!(one["scores"][0], many["scores"][0]);
            if engine != "isolation_forest" {
                assert_eq!(many["patch_scores"].as_array().unwrap().len(), 2);
                assert_eq!(many["patch_scores"][0].as_array().unwrap().len(), 2);
                assert_eq!(one["patch_scores"][0], many["patch_scores"][0]);
                assert!(many["scores"][1].as_f64().unwrap() > many["scores"][0].as_f64().unwrap());
            }
            assert!(
                predictor
                    .predict(&TensorData {
                        shape: vec![4],
                        values: single.values.clone()
                    })
                    .is_err()
            );
        }
    }

    #[test]
    #[cfg(feature = "burn")]
    fn burn_cached_predictions_match_fresh_loading_and_retain_weight_files() {
        use flow_like_ml_burn as burn;
        let directory = tempfile::tempdir().unwrap();
        let repository =
            TrainingRepository::open(directory.path().join("training.sqlite")).unwrap();
        let model_dir = tempfile::tempdir().unwrap();
        let config = burn::TrainingConfig {
            recipe: burn::Recipe::Mlp {
                input_features: 1,
                hidden: 4,
                outputs: 1,
                objective: burn::Objective::Regression,
            },
            backend: burn::BackendChoice::Cpu,
            epochs: 2,
            batch_size: 3,
            learning_rate: 0.01,
            seed: 42,
            gradient_clip: 5.0,
        };
        let dataset = burn::TensorDataset {
            inputs: burn::TensorData {
                shape: vec![3, 1],
                values: vec![-1.0, 0.0, 1.0],
            },
            targets: burn::Targets::Dense {
                tensor: burn::TensorData {
                    shape: vec![3, 1],
                    values: vec![-2.0, 0.0, 2.0],
                },
            },
        };
        burn::train(
            &config,
            &dataset,
            None,
            model_dir.path(),
            &burn::CancellationToken::new(),
            |_| {},
        )
        .unwrap();
        let bytes = burn_adapter::pack_named(
            model_dir.path(),
            burn::MANIFEST_FILE,
            false,
            32 * 1024 * 1024,
        )
        .unwrap();
        let artifact = publish(
            &repository,
            0,
            &bytes,
            serde_json::json!({"engine":"burn","format_version":1,"config":config,"input_shape":[1],"labels":[]}),
        );
        let input = TensorData {
            shape: dataset.inputs.shape,
            values: dataset.inputs.values,
        };
        let loaded =
            LoadedArtifactPredictor::load(&repository, &artifact.id, &ComputeConfig::default())
                .unwrap();
        let expected = burn_adapter::predict(&bytes, &input, &ComputeConfig::default()).unwrap();
        assert_eq!(loaded.predict(&input).unwrap(), expected);
        std::fs::remove_file(
            repository
                .path()
                .with_extension("artifacts")
                .join(&artifact.blob.path),
        )
        .unwrap();
        std::fs::remove_dir_all(model_dir.path()).unwrap();
        assert_eq!(loaded.predict(&input).unwrap(), expected);
        assert_eq!(loaded.predict(&input).unwrap(), expected);
    }

    #[test]
    #[cfg(feature = "burn")]
    fn burn_predictions_split_batches_and_load_manifests_with_widened_f32_configs() {
        use flow_like_ml_burn as burn;
        let directory = tempfile::tempdir().unwrap();
        let repository =
            TrainingRepository::open(directory.path().join("training.sqlite")).unwrap();
        let model_dir = tempfile::tempdir().unwrap();
        let config = burn::TrainingConfig {
            recipe: burn::Recipe::Mlp {
                input_features: 1,
                hidden: 4,
                outputs: 1,
                objective: burn::Objective::Regression,
            },
            backend: burn::BackendChoice::Cpu,
            epochs: 2,
            batch_size: 3,
            learning_rate: 0.01,
            seed: 42,
            gradient_clip: 0.1,
        };
        let dataset = burn::TensorDataset {
            inputs: burn::TensorData {
                shape: vec![5, 1],
                values: vec![-1.0, -0.5, 0.0, 0.5, 1.0],
            },
            targets: burn::Targets::Dense {
                tensor: burn::TensorData {
                    shape: vec![5, 1],
                    values: vec![-2.0, -1.0, 0.0, 1.0, 2.0],
                },
            },
        };
        burn::train(
            &config,
            &dataset,
            None,
            model_dir.path(),
            &burn::CancellationToken::new(),
            |_| {},
        )
        .unwrap();
        let bytes = burn_adapter::pack_named(
            model_dir.path(),
            burn::MANIFEST_FILE,
            false,
            32 * 1024 * 1024,
        )
        .unwrap();
        // Artifacts published before the fix carry a json!-widened config.
        let artifact = publish(
            &repository,
            0,
            &bytes,
            serde_json::json!({"engine":"burn","format_version":1,"config":config,"input_shape":[1],"labels":[]}),
        );
        let mut loaded =
            LoadedArtifactPredictor::load(&repository, &artifact.id, &ComputeConfig::default())
                .unwrap();
        let input = TensorData {
            shape: dataset.inputs.shape,
            values: dataset.inputs.values,
        };
        let whole = loaded.predict(&input).unwrap();
        let LoadedModel::Burn { batch_limit, .. } = &mut loaded.model else {
            panic!("expected a Burn predictor");
        };
        *batch_limit = 2;
        let chunked = loaded.predict(&input).unwrap();
        assert_eq!(chunked["output"]["shape"], serde_json::json!([5, 1]));
        let values = |prediction: &Value| -> Vec<f64> {
            serde_json::from_value(prediction["output"]["values"].clone()).unwrap()
        };
        for (left, right) in values(&whole).iter().zip(values(&chunked)) {
            assert!((left - right).abs() < 1e-6, "{left} != {right}");
        }
    }

    #[test]
    #[cfg(feature = "burn")]
    fn prediction_chunks_concatenate_batch_major_fields_and_reject_mismatches() {
        use flow_like_ml_burn as burn;
        let batch = |rows: usize, classes: Option<Vec<i64>>| burn::PredictionBatch {
            output: burn::TensorData {
                shape: vec![rows, 2],
                values: vec![rows as f32; rows * 2],
            },
            classes,
            reconstruction_error: None,
            detections: None,
            instances: None,
        };
        let mut merged = batch(2, Some(vec![0, 1]));
        merge_predictions(&mut merged, batch(1, Some(vec![1]))).unwrap();
        assert_eq!(merged.output.shape, vec![3, 2]);
        assert_eq!(merged.output.values.len(), 6);
        assert_eq!(merged.classes, Some(vec![0, 1, 1]));
        assert!(merge_predictions(&mut merged, batch(1, None)).is_err());
        let mut reshaped = batch(1, Some(vec![0]));
        reshaped.output.shape = vec![1, 1, 2];
        assert!(merge_predictions(&mut merged, reshaped).is_err());
        let input = TensorData {
            shape: vec![5, 2],
            values: (0..10).map(|value| value as f32).collect(),
        };
        let chunks: Vec<_> = batch_chunks(&input, 2).collect();
        assert_eq!(
            chunks
                .iter()
                .map(|chunk| chunk.shape.clone())
                .collect::<Vec<_>>(),
            vec![vec![2, 2], vec![2, 2], vec![1, 2]]
        );
        assert_eq!(
            chunks
                .into_iter()
                .flat_map(|chunk| chunk.values)
                .collect::<Vec<_>>(),
            input.values
        );
        assert_eq!(batch_chunks(&input, usize::MAX).count(), 1);
    }

    #[test]
    fn bounded_blob_reads_reject_actual_size_changes_and_digest_corruption() {
        let directory = tempfile::tempdir().unwrap();
        let repository =
            TrainingRepository::open(directory.path().join("training.sqlite")).unwrap();
        let artifact = publish(&repository, 0, b"original", serde_json::json!({}));
        assert!(repository.read_blob_limited(&artifact.blob, 7).is_err());
        let path = repository
            .path()
            .with_extension("artifacts")
            .join(&artifact.blob.path);
        std::fs::write(&path, b"much longer content").unwrap();
        assert!(repository.read_blob_limited(&artifact.blob, 100).is_err());
        std::fs::write(&path, b"wrong!!!").unwrap();
        assert!(repository.read_blob_limited(&artifact.blob, 100).is_err());
        std::fs::write(&path, b"short").unwrap();
        assert!(repository.read_blob_limited(&artifact.blob, 100).is_err());
    }

    /// Publishes the smallest EfficientAD model, trained on one image, and returns its packed
    /// bytes and a one-image request.
    #[cfg(feature = "burn")]
    fn trained_efficient_ad(
        repository: &TrainingRepository,
    ) -> (ModelArtifact, Vec<u8>, TensorData) {
        use flow_like_ml_burn as burn;
        let model_dir = tempfile::tempdir().unwrap();
        let shapes = [[1, 1, 4, 4], [2, 1, 4, 4], [2, 2, 3, 3], [1, 2, 4, 4]];
        let teacher = burn::PdnTeacherWeights {
            input_channels: 1,
            base_channels: 1,
            feature_channels: 1,
            convolutions: shapes
                .iter()
                .map(|shape| burn::ConvWeights {
                    weight: burn::TensorData {
                        shape: shape.to_vec(),
                        values: vec![0.01; shape.iter().product()],
                    },
                    bias: burn::TensorData {
                        shape: vec![shape[0]],
                        values: vec![0.01; shape[0]],
                    },
                })
                .collect(),
        };
        let config = burn::EfficientAdConfig {
            teacher,
            backend: burn::BackendChoice::Cpu,
            epochs: 1,
            batch_size: 1,
            learning_rate: 0.001,
            seed: 42,
            autoencoder_channels: 2,
            hard_quantile: 0.99,
            gradient_clip: 5.0,
        };
        let data = burn::EfficientAdDataset {
            images: burn::TensorData {
                shape: vec![1, 1, 256, 256],
                values: (0..256 * 256)
                    .map(|i| ((i % 256) as f32 * 0.03).sin() + ((i / 256) as f32 * 0.02).cos())
                    .collect(),
            },
            autoencoder_images: None,
            penalty_images: None,
        };
        let validation = burn::TensorData {
            shape: data.images.shape.clone(),
            values: data.images.values.iter().map(|v| v * 0.99 + 0.01).collect(),
        };
        burn::train_efficient_ad(
            &config,
            &data,
            &validation,
            model_dir.path(),
            &burn::CancellationToken::new(),
            |_| {},
        )
        .unwrap();
        let name = "efficient-ad-state.json";
        let state: Value =
            serde_json::from_slice(&std::fs::read(model_dir.path().join(name)).unwrap()).unwrap();
        let bytes =
            burn_adapter::pack_named(model_dir.path(), name, false, 32 * 1024 * 1024).unwrap();
        let artifact = publish(
            repository,
            0,
            &bytes,
            serde_json::json!({"engine":"efficient_ad","format_version":1,"architecture":state["architecture"],"input_shape":[1,256,256],"labels":[]}),
        );
        let input = TensorData {
            shape: validation.shape,
            values: validation.values,
        };
        (artifact, bytes, input)
    }

    #[test]
    #[cfg(feature = "burn")]
    fn efficient_ad_cached_predictions_preserve_calibration_and_maps() {
        let directory = tempfile::tempdir().unwrap();
        let repository =
            TrainingRepository::open(directory.path().join("training.sqlite")).unwrap();
        let (artifact, bytes, input) = trained_efficient_ad(&repository);
        let loaded =
            LoadedArtifactPredictor::load(&repository, &artifact.id, &ComputeConfig::default())
                .unwrap();
        let expected =
            efficient_ad_adapter::predict(&bytes, &input, &ComputeConfig::default()).unwrap();
        assert_eq!(loaded.predict(&input).unwrap(), expected);
        std::fs::remove_file(
            repository
                .path()
                .with_extension("artifacts")
                .join(&artifact.blob.path),
        )
        .unwrap();
        assert_eq!(loaded.predict(&input).unwrap(), expected);
    }

    /// PDN activations run in a fixed workspace, while each image's input and returned maps
    /// stay resident, so the budget admits whole images beyond that workspace.
    #[test]
    #[cfg(feature = "burn")]
    fn efficient_ad_admission_charges_a_fixed_workspace_and_per_image_maps() {
        let directory = tempfile::tempdir().unwrap();
        let repository =
            TrainingRepository::open(directory.path().join("training.sqlite")).unwrap();
        let (artifact, _, image) = trained_efficient_ad(&repository);
        let per_image = efficient_ad_image_bytes(image.values.len() as u64);
        let fixed =
            artifact.blob.bytes * 4 + flow_like_ml_burn::EFFICIENT_AD_INFERENCE_WORKSPACE_BYTES;
        let limited = |memory_limit_bytes| ComputeConfig {
            memory_limit_bytes,
            ..ComputeConfig::default()
        };
        let Err(error) = LoadedArtifactPredictor::load(
            &repository,
            &artifact.id,
            &limited(fixed + per_image - 1),
        ) else {
            panic!("a budget without room for one image beyond the workspace must not load");
        };
        assert!(
            error.to_string().contains("EfficientAD inference needs"),
            "{error}"
        );
        let predictor = LoadedArtifactPredictor::load(
            &repository,
            &artifact.id,
            &limited(fixed + per_image * 3 / 2),
        )
        .unwrap();
        let pair = TensorData {
            shape: vec![2, 1, 256, 256],
            values: [image.values.clone(), image.values.clone()].concat(),
        };
        assert!(predictor.predict(&pair).is_err());
        let prediction = predictor.predict(&image).unwrap();
        assert_eq!(
            prediction["anomaly_maps"]["shape"],
            serde_json::json!([1, 1, 256, 256])
        );
    }
}
