use flow_like::flow::{execution::context::ExecutionContext, node::NodeLogic};
use flow_like_catalog_core::FlowPath;
use flow_like_types::{Result, anyhow};
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

/// The extractor of features computed from `source` on `input` after `preprocessing`.
#[cfg(feature = "training")]
fn applied_extractor(
    source: &flow_like_ml_runtime::PretrainedSource,
    input: &flow_like_ml_core::TensorData,
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
        && (input.shape.len() < 2 || input.shape[input.shape.len() - 2..] != size)
    {
        return Err(anyhow!(
            "Declared preprocessing resizes images to {}x{} (width x height), but the input has shape {:?}",
            size[1],
            size[0],
            input.shape
        ));
    }
    flow_like_ml_runtime::engines::validate_burn_pretrained_preprocessing(source, preprocessing)?;
    extractor.image_steps = preprocessing.to_vec();
    Ok(extractor.into())
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
        #[cfg(feature = "training")]
        {
            context.deactivate_exec_pin("exec_out").await?;
            let request: PretrainedFeaturesRequest = context.evaluate_pin("request").await?;
            let (repo, project) = super::lifecycle::repository(context)?;
            if context.is_cancelled() {
                return Err(anyhow!("Pretrained feature extraction cancelled"));
            }
            let (result, extractor) = tokio::task::spawn_blocking(move || -> Result<_> {
                let features = flow_like_ml_runtime::engines::pretrained_source_features(
                    &repo,
                    &project,
                    &request.source_id,
                    &request.input,
                    &request.compute,
                )?;
                let source = repo.get_pretrained_source(&request.source_id)?;
                let extractor = applied_extractor(&source, &request.input, &request.preprocessing)?;
                Ok((features, extractor))
            })
            .await??;
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
        #[cfg(not(feature = "training"))]
        {
            let _ = context;
            Err(anyhow!(
                "Pretrained feature extraction requires a local training build"
            ))
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
            Err(anyhow!(
                "Pretrained weight import requires a local training build"
            ))
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
            let (repo, project) = super::lifecycle::repository(context)?;
            let result = repo.list_pretrained_sources(&project, request.maximum)?;
            context
                .set_pin_value("result", flow_like_types::json::json!(result))
                .await?;
            context.activate_exec_pin("exec_out").await?;
            Ok(())
        }
        #[cfg(not(feature = "training"))]
        {
            let _ = context;
            Err(anyhow!(
                "Pretrained weight listing requires a local training build"
            ))
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
    #[test]
    fn declared_preprocessing_becomes_the_extractor_image_steps() {
        use flow_like_ml_core::{PreprocessingStep, TensorData};
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
        let input = TensorData {
            shape: vec![3, 4, 4],
            values: vec![0.0; 48],
        };
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
