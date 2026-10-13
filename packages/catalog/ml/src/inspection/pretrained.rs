use flow_like::flow::{execution::context::ExecutionContext, node::NodeLogic};
use flow_like_catalog_core::FlowPath;
use flow_like_types::Result;
#[cfg(feature = "inference")]
use flow_like_types::anyhow;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

#[derive(Clone, Serialize, Deserialize, JsonSchema)]
#[serde(tag = "model", rename_all = "snake_case")]
pub enum PretrainedImportFormat {
    TorchvisionResNet18,
    DinoV2 {
        variant: flow_like_ml_burn::DinoV2Variant,
    },
    RtdetrV2 {
        backbone: flow_like_ml_burn::RtdetrV2Backbone,
    },
    RfDetr {
        variant: flow_like_ml_burn::RfDetrVariant,
    },
    DfineNano,
    Sam2,
}

#[derive(Clone, Serialize, Deserialize, JsonSchema)]
pub struct ImportPretrainedVisionRequest {
    pub weights: FlowPath,
    pub format: PretrainedImportFormat,
    pub metadata: flow_like_ml_burn::PretrainedWeightsMetadata,
    /// Class order of the imported head. Leave empty for a feature-only backbone.
    #[serde(default)]
    pub labels: Vec<String>,
    #[serde(default = "default_import_limit")]
    pub maximum_bytes: u64,
}

fn default_import_limit() -> u64 {
    512 * 1024 * 1024
}

#[derive(Clone, Serialize, Deserialize, JsonSchema)]
pub struct ListPretrainedVisionRequest {
    #[serde(default = "default_list_limit")]
    pub maximum: usize,
}

fn default_list_limit() -> usize {
    32
}

#[derive(Clone, Serialize, Deserialize, JsonSchema)]
pub struct PretrainedFeaturesRequest {
    pub source_id: String,
    /// A CHW image or NCHW batch prepared using the source's recommended preprocessing.
    pub input: flow_like_ml_core::TensorData,
    #[serde(default)]
    pub compute: flow_like_ml_core::ComputeConfig,
    /// Image steps applied to `input`, as Preprocess Inspection Image outputs them. They must
    /// match the input size and the source's normalization, and become the extractor's image
    /// steps. Left empty, the extractor carries the source's recommendation unverified.
    #[serde(default)]
    pub preprocessing: Vec<flow_like_ml_core::PreprocessingStep>,
}

/// The extractor of features computed from `source` on an input of `input_shape` after
/// `preprocessing`.
#[cfg(feature = "inference")]
fn applied_extractor(
    source: &flow_like_ml_runtime::PretrainedSource,
    input_shape: &[usize],
    preprocessing: &[flow_like_ml_core::PreprocessingStep],
) -> Result<super::lifecycle::FeatureExtractorRef> {
    let mut extractor = flow_like_ml_runtime::engines::FeatureExtractor::from_source(source)?;
    if preprocessing.is_empty() {
        return Ok(extractor.into());
    }
    let resized = preprocessing.iter().rev().find_map(|step| match step {
        flow_like_ml_core::PreprocessingStep::Resize { width, height } => Some([*height, *width]),
        _ => None,
    });
    if let Some(size) = resized
        && (input_shape.len() < 2 || input_shape[input_shape.len() - 2..] != size)
    {
        return Err(anyhow!(
            "Declared preprocessing resizes images to {}x{} (width x height), but the input has shape {:?}",
            size[1],
            size[0],
            input_shape
        ));
    }
    flow_like_ml_runtime::engines::validate_burn_pretrained_preprocessing(source, preprocessing)?;
    extractor.image_steps = preprocessing.to_vec();
    Ok(extractor.into())
}

/// Features from the ledger that holds `source_id` for this run's project, else from its
/// published folder.
#[cfg(feature = "inference")]
async fn pretrained_features(
    context: &mut ExecutionContext,
    request: PretrainedFeaturesRequest,
) -> Result<(
    flow_like_ml_core::TensorData,
    super::lifecycle::FeatureExtractorRef,
)> {
    use super::lifecycle::{LocatedLedger, holds_pretrained_source};
    let compute = request.compute.for_host();
    let located = super::lifecycle::locate_repository_for_inference(context, |repo, project| {
        holds_pretrained_source(repo, project, &request.source_id)
    })?;
    let Some(LocatedLedger { repo, project, .. }) = located else {
        return published_pretrained_features(context, request, &compute).await;
    };
    tokio::task::spawn_blocking(move || -> Result<_> {
        let features = flow_like_ml_runtime::engines::pretrained_source_features(
            &repo,
            &project,
            &request.source_id,
            &request.input,
            &compute,
        )?;
        let source = repo.get_pretrained_source(&request.source_id)?;
        let extractor = applied_extractor(&source, &request.input.shape, &request.preprocessing)?;
        Ok((features, extractor))
    })
    .await?
}

/// Features from the published folder of `request.source_id`: a pretrained source or a Burn
/// model, in the app storage or the executing user's directory, computed by the feature
/// extractor the model depends on when it records one.
#[cfg(feature = "inference")]
async fn published_pretrained_features(
    context: &mut ExecutionContext,
    request: PretrainedFeaturesRequest,
    compute: &flow_like_ml_core::ComputeConfig,
) -> Result<(
    flow_like_ml_core::TensorData,
    super::lifecycle::FeatureExtractorRef,
)> {
    let model = super::published::PublishedModelSource::Artifact {
        artifact_id: request.source_id.clone(),
        storage: None,
    };
    let shape = request.input.shape.clone();
    let mut input = request.input;
    if input.shape.len() == 3 {
        input.shape.insert(0, 1);
    }
    let (features, extractor) =
        super::published::published_features(context, &model, input, compute).await?;
    if request.preprocessing.is_empty() {
        return Ok((features, extractor));
    }
    let source = computing_source(context, &model, &extractor).await?;
    Ok((
        features,
        applied_extractor(&source, &shape, &request.preprocessing)?,
    ))
}

/// The pretrained source of the published folder whose weights computed `extractor`: the
/// folder of `model`, or of the feature extractor it depends on, read where the loader reads
/// it, the model root's artifact folder and then `dependencies/` beside the model.
#[cfg(feature = "inference")]
async fn computing_source(
    context: &mut ExecutionContext,
    model: &super::published::PublishedModelSource,
    extractor: &super::lifecycle::FeatureExtractorRef,
) -> Result<flow_like_ml_runtime::PretrainedSource> {
    use super::published::{
        MANIFEST_FILE, PublishedModelSource, child, dependency_dir, folder_of, resolve_model,
    };
    let mut folder = resolve_model(context, model).await?;
    if let Some(dependency) = folder.manifest.dependencies.feature_extractor.clone() {
        let canonical = PublishedModelSource::Artifact {
            artifact_id: dependency.source_id.clone(),
            storage: folder.storage,
        };
        folder = match resolve_model(context, &canonical).await {
            Ok(found) => found,
            Err(_) => {
                let model_folder = folder_of(&folder.manifest_path);
                let exported_folder = child(&model_folder, &dependency_dir(&dependency.source_id));
                let exported = PublishedModelSource::Files {
                    manifest: child(&exported_folder, MANIFEST_FILE),
                    weights: None,
                };
                resolve_model(context, &exported).await?
            }
        };
    }
    let source = published_source(&folder.manifest)?;
    if source.id != extractor.source_id || source.blob.sha256 != extractor.blob_sha256 {
        return Err(anyhow!(
            "{} holds {} with weights {}, not the feature extractor {} with weights {} that computed the features",
            folder.manifest_path.path,
            source.id,
            source.blob.sha256,
            extractor.source_id,
            extractor.blob_sha256
        ));
    }
    Ok(source)
}

/// The pretrained source a published folder holds; a model artifact counts as a training
/// artifact source, as in the ledger.
#[cfg(feature = "inference")]
fn published_source(
    manifest: &super::published::PublishedModelManifest,
) -> Result<flow_like_ml_runtime::PretrainedSource> {
    if let Some(source) = &manifest.pretrained_source {
        return Ok(source.clone());
    }
    let artifact = manifest.artifact.as_ref().ok_or_else(|| {
        anyhow!(
            "Published model manifest for {} holds neither a model artifact nor a pretrained source",
            manifest.artifact_id()
        )
    })?;
    Ok(flow_like_ml_runtime::PretrainedSource {
        id: artifact.id.clone(),
        project_id: artifact.stream.project_id.clone(),
        source_kind: flow_like_ml_runtime::PretrainedSourceKind::TrainingArtifact,
        blob: artifact.blob.clone(),
        manifest: artifact.manifest.clone(),
        origin: flow_like_types::json::json!({
            "stream": artifact.stream,
            "job_id": artifact.job_id,
            "snapshot_id": artifact.snapshot_id,
            "dataset_digest": artifact.dataset_digest,
        }),
        created_at_ms: artifact.created_at_ms,
    })
}

#[crate::register_node]
#[derive(Default)]
pub struct ExtractPretrainedFeaturesNode;

#[flow_like_types::async_trait]
impl NodeLogic for ExtractPretrainedFeaturesNode {
    fn get_node(&self) -> flow_like::flow::node::Node {
        let mut node = super::operation_node::<
            PretrainedFeaturesRequest,
            flow_like_ml_core::TensorData,
        >(
            "ml_extract_pretrained_features",
            "Extract Pretrained Image Features",
            "Extract NCHW spatial features from project-owned DINOv2 or convolutional weights for patch anomaly models",
            "pretrainedImageFeatures",
            "AI/ML/Training",
        );
        node.add_output_pin(
            "extractor",
            "Feature Extractor",
            "Source, weights digest and image steps of these features: the declared preprocessing, else the source's unverified recommendation. Patch models record it and reject features from other weights or steps",
            flow_like::flow::variable::VariableType::Struct,
        )
        .set_schema::<super::lifecycle::FeatureExtractorRef>();
        node.set_version(3);
        node
    }
    async fn run(&self, context: &mut ExecutionContext) -> Result<()> {
        #[cfg(feature = "inference")]
        {
            context.deactivate_exec_pin("exec_out").await?;
            let request: PretrainedFeaturesRequest = context.evaluate_pin("request").await?;
            if context.is_cancelled() {
                return Err(anyhow!("Pretrained feature extraction cancelled"));
            }
            let (result, extractor) = pretrained_features(context, request).await?;
            if context.is_cancelled() {
                return Err(anyhow!("Pretrained feature extraction cancelled"));
            }
            context
                .set_pin_value("result", flow_like_types::json::json!(result))
                .await?;
            context
                .set_pin_value("extractor", flow_like_types::json::json!(extractor))
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

#[cfg(feature = "training")]
pub async fn import_pretrained_vision(
    context: &mut ExecutionContext,
    request: ImportPretrainedVisionRequest,
) -> Result<flow_like_ml_runtime::PretrainedSource> {
    use flow_like_types::json::json;
    use futures::TryStreamExt;

    if request.maximum_bytes == 0 || request.maximum_bytes > 2 * 1024 * 1024 * 1024 {
        return Err(anyhow!(
            "Pretrained import byte limit must be between 1 and 2 GiB"
        ));
    }
    flow_like_ml_core::validate_labels(&request.labels)?;
    if matches!(&request.format, PretrainedImportFormat::Sam2) && !request.labels.is_empty() {
        return Err(anyhow!("SAM 2 uses prompts rather than a class label list"));
    }
    let (repo, project) = super::lifecycle::repository(context)?;
    let (file, _) = request.weights.get_cached_file(context).await?;
    let file = file.ok_or_else(|| anyhow!("Pretrained weights were not found"))?;
    if file.meta.size == 0 || file.meta.size > request.maximum_bytes {
        return Err(anyhow!("Pretrained weights exceed the import byte limit"));
    }
    let mut chunks = file.into_stream();
    let mut bytes = Vec::new();
    while let Some(chunk) = chunks.try_next().await? {
        if context.is_cancelled() {
            return Err(anyhow!("Pretrained import cancelled"));
        }
        let next = bytes
            .len()
            .checked_add(chunk.len())
            .ok_or_else(|| anyhow!("Pretrained import byte count overflow"))?;
        if next as u64 > request.maximum_bytes {
            return Err(anyhow!("Pretrained weights exceed the import byte limit"));
        }
        bytes.extend_from_slice(&chunk);
    }
    let cancelled = context.get_cancellation_token();
    tokio::task::spawn_blocking(move || -> Result<flow_like_ml_runtime::PretrainedSource> {
        let directory = tempfile::tempdir()?;
        let source = directory.path().join("weights.safetensors");
        std::fs::write(&source, &bytes)?;
        drop(bytes);
        let model_dir = directory.path().join("model");
        let imported = match request.format {
            PretrainedImportFormat::TorchvisionResNet18 => {
                flow_like_ml_burn::import_torchvision_resnet18_safetensors(
                    &source,
                    &model_dir,
                    &request.metadata,
                )?
            }
            PretrainedImportFormat::DinoV2 { variant } => {
                flow_like_ml_burn::import_dinov2_safetensors(
                    &source,
                    &model_dir,
                    variant,
                    &request.metadata,
                )?
            }
            PretrainedImportFormat::RtdetrV2 { backbone } => {
                flow_like_ml_burn::import_rtdetr_v2_weights(
                    &source,
                    &model_dir,
                    backbone,
                    &request.metadata,
                )?
            }
            PretrainedImportFormat::RfDetr { variant } => {
                flow_like_ml_burn::import_rf_detr_weights(
                    &source,
                    &model_dir,
                    variant,
                    &request.metadata,
                )?
            }
            PretrainedImportFormat::DfineNano => flow_like_ml_burn::import_dfine_nano_weights(
                &source,
                &model_dir,
                &request.metadata,
            )?,
            PretrainedImportFormat::Sam2 => {
                let imported =
                    flow_like_ml_burn::import_sam2_weights(&source, &model_dir, &request.metadata)?;
                if cancelled.as_ref().is_some_and(|token| token.is_cancelled()) {
                    return Err(anyhow!("Pretrained import cancelled"));
                }
                return Ok(flow_like_ml_runtime::engines::publish_sam2_source(
                    &repo,
                    &project,
                    &model_dir,
                    json!(imported.provenance),
                    request.maximum_bytes,
                    flow_like_ml_runtime::now_ms(),
                )?);
            }
        };
        if cancelled.as_ref().is_some_and(|token| token.is_cancelled()) {
            return Err(anyhow!("Pretrained import cancelled"));
        }
        Ok(
            flow_like_ml_runtime::engines::publish_burn_pretrained_source(
                &repo,
                &project,
                &model_dir,
                request.labels,
                json!(imported.provenance),
                request.maximum_bytes,
                flow_like_ml_runtime::now_ms(),
            )?,
        )
    })
    .await?
}

#[crate::register_node]
#[derive(Default)]
pub struct ImportPretrainedVisionNode;

#[flow_like_types::async_trait]
impl NodeLogic for ImportPretrainedVisionNode {
    fn get_node(&self) -> flow_like::flow::node::Node {
        super::operation_node::<ImportPretrainedVisionRequest, flow_like_ml_runtime::PretrainedSource>(
            "ml_import_pretrained_vision",
            "Import Pretrained Vision Weights",
            "Import compatible vision weights with their source, license and digest for native fine-tuning",
            "importPretrainedVision",
            "AI/ML/Training",
        )
    }
    async fn run(&self, context: &mut ExecutionContext) -> Result<()> {
        #[cfg(feature = "training")]
        {
            context.deactivate_exec_pin("exec_out").await?;
            let request = context.evaluate_pin("request").await?;
            let result = import_pretrained_vision(context, request).await?;
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

#[crate::register_node]
#[derive(Default)]
pub struct ListPretrainedVisionNode;

#[flow_like_types::async_trait]
impl NodeLogic for ListPretrainedVisionNode {
    fn get_node(&self) -> flow_like::flow::node::Node {
        super::operation_node::<
            ListPretrainedVisionRequest,
            Vec<flow_like_ml_runtime::PretrainedSource>,
        >(
            "ml_list_pretrained_vision",
            "List Pretrained Vision Weights",
            "List imported weights and trained Burn models available to this project",
            "listPretrainedVision",
            "AI/ML/Training",
        )
    }
    async fn run(&self, context: &mut ExecutionContext) -> Result<()> {
        #[cfg(feature = "training")]
        {
            context.deactivate_exec_pin("exec_out").await?;
            let request: ListPretrainedVisionRequest = context.evaluate_pin("request").await?;
            let scope = super::lifecycle::LedgerScope::App;
            let result = match super::lifecycle::existing_repository(context, scope)? {
                Some((repo, project)) => repo.list_pretrained_sources(&project, request.maximum)?,
                None => Vec::new(),
            };
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pretrained_features_also_output_their_extractor() {
        use super::super::lifecycle::FeatureExtractorRef;
        use flow_like::flow::pin::Pin;
        let node = ExtractPretrainedFeaturesNode.get_node();
        assert_eq!(node.version, Some(3));
        assert_eq!(
            node.get_pin_by_name("extractor").unwrap().schema,
            Pin::schema_string_for::<FeatureExtractorRef>()
        );
    }

    #[cfg(feature = "training")]
    #[test]
    fn extractor_pins_carry_the_runtime_extractor_shape() {
        use super::super::lifecycle::FeatureExtractorRef;
        use flow_like_types::json::{from_value, to_value};
        let runtime = flow_like_ml_runtime::engines::FeatureExtractor {
            source_id: "pretrained:resnet".into(),
            blob_sha256: "a".repeat(64),
            image_steps: vec![flow_like_ml_core::PreprocessingStep::Resize {
                width: 224,
                height: 224,
            }],
        };
        let pinned = FeatureExtractorRef::from(runtime.clone());
        assert_eq!(to_value(&pinned).unwrap(), to_value(&runtime).unwrap());
        let parsed: FeatureExtractorRef = from_value(to_value(&runtime).unwrap()).unwrap();
        assert_eq!(parsed, pinned);
        assert_eq!(pinned.to_runtime().unwrap(), runtime);
        let unverifiable = FeatureExtractorRef {
            blob_sha256: "not-a-digest".into(),
            ..pinned
        };
        assert!(unverifiable.to_runtime().is_err());
    }

    #[cfg(feature = "training")]
    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn pretrained_features_find_adopted_sources_and_published_folders() {
        use super::super::lifecycle::{LedgerScope, scoped_repository, test_device::*};
        use super::super::publisher::{PublishOutcome, publish_pretrained_source};
        let device = Device::new();
        let donor_path = device.scratch("donor").join("jobs.sqlite");
        let donor = flow_like_ml_runtime::TrainingRepository::open(donor_path).unwrap();
        let (_, artifact) = tiny_resnet(&donor, "donor:board", "camera");
        let mut context = plain_context(device.stores()).await;
        let (user, project) = scoped_repository(&context, LedgerScope::User).unwrap();
        let source = user
            .adopt_pretrained_source(&donor, &artifact.id, &project)
            .unwrap();
        assert_eq!(source.project_id, "app:board:user");
        let request = flow_like_types::json::json!({
            "source_id": artifact.id,
            "input": {"shape": [1, 8, 8], "values": vec![0.5f32; 64]}
        });
        let run = RunCache::default();
        let ledger = run_on(
            ExtractPretrainedFeaturesNode,
            request.clone(),
            device.stores(),
            &run,
            BOARD,
        )
        .await
        .unwrap();
        let published =
            publish_pretrained_source(&mut context, &user, LedgerScope::User, &source).await;
        assert!(
            matches!(published, PublishOutcome::Published(_)),
            "{published:?}"
        );
        let user_models = published_copy(&device.user, USER_MODELS).await;
        let files = run_on(
            ExtractPretrainedFeaturesNode,
            request,
            stores(Some(memory_store()), Some(user_models)),
            &RunCache::default(),
            BOARD,
        )
        .await
        .unwrap();
        for pin in ["result", "extractor"] {
            assert_eq!(
                output(&files, pin).await,
                output(&ledger, pin).await,
                "{pin}"
            );
        }
    }

    #[cfg(feature = "training")]
    #[tokio::test]
    async fn listing_pretrained_weights_reads_the_app_ledger_without_creating_it() {
        use super::super::lifecycle::{repository, test_device::*};
        use flow_like_types::json::json;
        let device = Device::new();
        let request = json!({"maximum": 8});
        let listed = run_node(ListPretrainedVisionNode, request.clone(), device.stores())
            .await
            .unwrap();
        assert_eq!(listed, json!([]));
        assert!(!device.root().join("training-ledgers").exists());
        assert!(!device.root().join("apps").exists());
        let context = plain_context(device.stores()).await;
        let (repo, project) = repository(&context).unwrap();
        let manifest = json!({
            "engine": "burn", "format_version": 1, "config": {}, "input_shape": [1]
        });
        let source = repo
            .register_pretrained_source(
                &project,
                b"weights",
                manifest,
                json!({"source": "fixture"}),
                1024,
                flow_like_ml_runtime::now_ms(),
            )
            .unwrap();
        let listed = run_node(ListPretrainedVisionNode, request, device.stores())
            .await
            .unwrap();
        assert_eq!(listed[0]["id"], json!(source.id));
    }

    #[cfg(feature = "training")]
    #[tokio::test]
    async fn published_dependencies_check_declared_preprocessing_and_name_the_extractor() {
        use super::super::lifecycle::{FeatureExtractorRef, test_device::*};
        use super::super::published::{
            MANIFEST_FILE, PublishedModelSource, artifact_dir, dependency_dir,
        };
        use flow_like_ml_core::PreprocessingStep;
        use flow_like_types::{Value, json::json};
        let device = Device::new();
        let mut context = plain_context(device.stores()).await;
        let write = |folder: &std::path::Path, manifest: &Value| {
            std::fs::create_dir_all(folder).unwrap();
            std::fs::write(folder.join(MANIFEST_FILE), manifest.to_string()).unwrap();
        };
        let digest = |digit: &str| digit.repeat(64);
        let blob = |sha: &str| json!({"sha256": sha, "bytes": 3, "path": format!("{sha}.blob")});
        let weights = |sha: &str| json!({"file": "weights.bin", "sha256": sha, "bytes": 3});
        let resize = PreprocessingStep::Resize {
            width: 4,
            height: 4,
        };
        let normalize = |mean: f32, std: f32| PreprocessingStep::Normalize {
            mean: vec![mean; 3],
            std: vec![std; 3],
        };
        let backbone_id = "pretrained:backbone";
        let recommended = vec![resize.clone(), normalize(0.45, 0.225)];
        let backbone = json!({
            "format_version": 2,
            "pretrained_source": {
                "id": backbone_id, "project_id": "app:board", "source_kind": "imported",
                "blob": blob(&digest("b")), "origin": {}, "created_at_ms": 0,
                "manifest": {"engine": "burn", "recommended_preprocessing": recommended}
            },
            "weights": weights(&digest("b"))
        });
        let stream = json!({
            "project_id": "app:board", "stream_id": "camera", "inspection_version": "1"
        });
        let patchcore = json!({
            "format_version": 2,
            "artifact": {
                "id": "patchcore", "job_id": "job", "snapshot_id": "snapshot", "stream": stream,
                "dataset_digest": digest("d"), "blob": blob(&digest("a")),
                "manifest": {"engine": "patchcore"}, "created_at_ms": 0
            },
            "weights": weights(&digest("a")),
            "dependencies": {
                "feature_extractor": {"source_id": backbone_id, "blob_sha256": digest("b")}
            }
        });
        let models = device.root().join("apps/app/storage");
        let (patchcore_folder, backbone_folder) = (
            models.join(artifact_dir("patchcore")),
            models.join(artifact_dir(backbone_id)),
        );
        write(&patchcore_folder, &patchcore);
        write(&backbone_folder, &backbone);
        let model = PublishedModelSource::Artifact {
            artifact_id: "patchcore".into(),
            storage: None,
        };
        let computed = FeatureExtractorRef {
            source_id: backbone_id.into(),
            blob_sha256: digest("b"),
            image_steps: recommended,
        };
        let source = computing_source(&mut context, &model, &computed)
            .await
            .unwrap();
        assert_eq!(source.id, backbone_id);
        let declared = vec![resize.clone(), normalize(0.5, 0.25)];
        let applied = applied_extractor(&source, &[3, 4, 4], &declared).unwrap();
        assert_eq!(applied.source_id, backbone_id);
        assert_eq!(applied.blob_sha256, digest("b"));
        assert_eq!(applied.image_steps, declared);
        let unnormalized = vec![resize, normalize(0.0, 1.0)];
        let error = applied_extractor(&source, &[3, 4, 4], &unnormalized).unwrap_err();
        assert!(
            error.to_string().contains("expects images normalized"),
            "{error}"
        );

        std::fs::remove_dir_all(&backbone_folder).unwrap();
        let exported_folder = patchcore_folder.join(dependency_dir(backbone_id));
        write(&exported_folder, &backbone);
        let exported = computing_source(&mut context, &model, &computed)
            .await
            .unwrap();
        assert_eq!(exported.id, backbone_id);
        let other = FeatureExtractorRef {
            blob_sha256: digest("c"),
            ..computed
        };
        let error = computing_source(&mut context, &model, &other)
            .await
            .unwrap_err();
        assert!(
            error.to_string().contains("not the feature extractor"),
            "{error}"
        );
    }

    #[cfg(feature = "inference")]
    #[test]
    fn published_artifacts_extract_as_training_artifact_sources() {
        use flow_like_types::json::{from_value, json};
        let sha = "b".repeat(64);
        let manifest: super::super::published::PublishedModelManifest = from_value(json!({
            "format_version": 2,
            "artifact": {
                "id": "resnet-artifact", "job_id": "job", "snapshot_id": "snapshot",
                "stream": {"project_id": "app:board", "stream_id": "camera", "inspection_version": "1"},
                "dataset_digest": sha,
                "blob": {"sha256": sha, "bytes": 3, "path": format!("{sha}.blob")},
                "manifest": {"engine": "burn", "preprocessing": [{"kind": "resize", "width": 8, "height": 8}]},
                "created_at_ms": 5
            },
            "weights": {"file": "weights.bin", "sha256": sha, "bytes": 3}
        }))
        .unwrap();
        let source = published_source(&manifest).unwrap();
        assert_eq!(
            source.source_kind,
            flow_like_ml_runtime::PretrainedSourceKind::TrainingArtifact
        );
        assert_eq!(source.project_id, "app:board");
        let artifact = manifest.artifact.as_ref().unwrap();
        let from_artifact =
            flow_like_ml_runtime::engines::FeatureExtractor::from_artifact(artifact).unwrap();
        assert_eq!(
            applied_extractor(&source, &[1usize, 8, 8], &[]).unwrap(),
            super::super::lifecycle::FeatureExtractorRef::from(from_artifact)
        );
        let empty: super::super::published::PublishedModelManifest = from_value(json!({
            "format_version": 2, "weights": {"file": "weights.bin", "sha256": sha, "bytes": 3}
        }))
        .unwrap();
        assert!(published_source(&empty).is_err());
    }

    #[cfg(feature = "inference")]
    #[test]
    fn declared_preprocessing_becomes_the_extractor_image_steps() {
        use flow_like_ml_core::PreprocessingStep;
        use flow_like_types::json::{from_value, json};
        let source: flow_like_ml_runtime::PretrainedSource = from_value(json!({
            "id": "pretrained:resnet",
            "project_id": "project",
            "source_kind": "imported",
            "blob": {"sha256": "a".repeat(64), "bytes": 1, "path": "blobs/resnet"},
            "manifest": {"recommended_preprocessing": [
                {"kind": "resize", "width": 4, "height": 4},
                {"kind": "normalize", "mean": [0.485, 0.456, 0.406], "std": [0.229, 0.224, 0.225]}
            ]},
            "origin": {},
            "created_at_ms": 0
        }))
        .unwrap();
        let input = [3usize, 4, 4];
        let recommended = applied_extractor(&source, &input, &[]).unwrap();
        assert_eq!(recommended.image_steps.len(), 2);
        let resize = PreprocessingStep::Resize {
            width: 4,
            height: 4,
        };
        let declared = vec![
            resize.clone(),
            PreprocessingStep::Normalize {
                mean: vec![0.5; 3],
                std: vec![0.25; 3],
            },
        ];
        let applied = applied_extractor(&source, &input, &declared).unwrap();
        assert_eq!(applied.image_steps, declared);
        assert_ne!(applied, recommended);
        let unnormalized = vec![
            resize,
            PreprocessingStep::Normalize {
                mean: vec![0.0; 3],
                std: vec![1.0; 3],
            },
        ];
        let error = applied_extractor(&source, &input, &unnormalized).unwrap_err();
        assert!(
            error.to_string().contains("expects images normalized"),
            "{error}"
        );
        let resized = vec![
            PreprocessingStep::Resize {
                width: 8,
                height: 8,
            },
            declared[1].clone(),
        ];
        let error = applied_extractor(&source, &input, &resized).unwrap_err();
        assert!(
            error.to_string().contains("resizes images to 8x8"),
            "{error}"
        );
    }
}
