use flow_like::flow::{execution::context::ExecutionContext, node::NodeLogic};
use flow_like_types::Result;
#[cfg(feature = "inference")]
use flow_like_types::anyhow;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

#[derive(Clone, Serialize, Deserialize, JsonSchema)]
pub struct PromptSam2Request {
    pub source_id: String,
    /// A CHW image or NCHW batch resized to 1024 and normalized with ImageNet mean and standard deviation.
    pub images: flow_like_ml_core::TensorData,
    pub prompts: Vec<flow_like_ml_burn::Sam2Prompt>,
    #[serde(default)]
    pub compute: flow_like_ml_core::ComputeConfig,
}

#[derive(Clone, Serialize, Deserialize, JsonSchema)]
pub struct FineTuneSam2Request {
    pub source_id: String,
    pub config: flow_like_ml_burn::Sam2TrainingConfig,
    pub data: flow_like_ml_burn::Sam2TrainingDataset,
    #[serde(default)]
    pub compute: flow_like_ml_core::ComputeConfig,
    #[serde(default = "default_artifact_limit")]
    pub maximum_bytes: u64,
}
fn default_artifact_limit() -> u64 {
    512 * 1024 * 1024
}

#[crate::register_node]
#[derive(Default)]
pub struct PromptSam2Node;
#[flow_like_types::async_trait]
impl NodeLogic for PromptSam2Node {
    fn get_node(&self) -> flow_like::flow::node::Node {
        super::operation_node::<PromptSam2Request, flow_like_ml_burn::Sam2Prediction>(
            "ml_prompt_sam2",
            "Segment with SAM 2",
            "Predict image masks from foreground points, background points, a box or a previous mask",
            "promptSam2",
            "AI/ML/Vision",
        )
    }
    async fn run(&self, context: &mut ExecutionContext) -> Result<()> {
        #[cfg(feature = "inference")]
        {
            context.deactivate_exec_pin("exec_out").await?;
            let request: PromptSam2Request = context.evaluate_pin("request").await?;
            if context.is_cancelled() {
                return Err(anyhow!("SAM prediction cancelled"));
            }
            let result = sam2_prediction(context, request).await?;
            if context.is_cancelled() {
                return Err(anyhow!("SAM prediction cancelled"));
            }
            context
                .set_pin_value("result", flow_like_types::json::json!(result))
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

/// Masks from the ledger that holds `request.source_id` for this run's project, else from
/// its published fine-tuned source.
#[cfg(feature = "inference")]
async fn sam2_prediction(
    context: &mut ExecutionContext,
    request: PromptSam2Request,
) -> Result<flow_like_ml_burn::Sam2Prediction> {
    use super::lifecycle::{LocatedLedger, holds_pretrained_source};
    let compute = request.compute.for_host();
    let located = super::lifecycle::locate_repository_for_inference(context, |repo, project| {
        holds_pretrained_source(repo, project, &request.source_id)
    })?;
    let Some(LocatedLedger { repo, project, .. }) = located else {
        return super::published::published_sam2_prediction(
            context,
            &request.source_id,
            &request.images,
            &request.prompts,
            &compute,
        )
        .await;
    };
    Ok(tokio::task::spawn_blocking(move || {
        flow_like_ml_runtime::engines::predict_sam2_source(
            &repo,
            &project,
            &request.source_id,
            &request.images,
            &request.prompts,
            &compute,
        )
    })
    .await??)
}

#[crate::register_node]
#[derive(Default)]
pub struct FineTuneSam2Node;
#[flow_like_types::async_trait]
impl NodeLogic for FineTuneSam2Node {
    fn get_node(&self) -> flow_like::flow::node::Node {
        super::operation_node::<FineTuneSam2Request, flow_like_ml_runtime::Sam2FineTuneResult>(
            "ml_fine_tune_sam2",
            "Fine-tune SAM 2",
            "Fine-tune a native SAM image model from prompts and reviewed masks, then publish new project weights",
            "fineTuneSam2",
            "AI/ML/Training",
        )
    }
    async fn run(&self, context: &mut ExecutionContext) -> Result<()> {
        #[cfg(feature = "training")]
        {
            context.deactivate_exec_pin("exec_out").await?;
            let request: FineTuneSam2Request = context.evaluate_pin("request").await?;
            let result = fine_tune_sam2(context, request).await?;
            context
                .set_pin_value("result", flow_like_types::json::json!(result))
                .await?;
            context.activate_exec_pin("exec_out").await
        }
        #[cfg(not(feature = "training"))]
        {
            let _ = context;
            Err(super::lifecycle::training_unavailable())
        }
    }
}

/// Fine-tunes a SAM 2 source in the ledger that holds it, found like Segment with SAM 2 finds
/// it, and publishes the result to that ledger's model root.
#[cfg(feature = "training")]
async fn fine_tune_sam2(
    context: &mut ExecutionContext,
    request: FineTuneSam2Request,
) -> Result<flow_like_ml_runtime::Sam2FineTuneResult> {
    use super::lifecycle::{LocatedLedger, holds_pretrained_source, locate_repository};
    if !(1..=2 * 1024 * 1024 * 1024).contains(&request.maximum_bytes) {
        return Err(anyhow!(
            "SAM artifact byte limit must be between 1 and 2 GiB, got {} bytes",
            request.maximum_bytes
        ));
    }
    if context.is_cancelled() {
        return Err(anyhow!("SAM fine-tuning cancelled"));
    }
    let located = locate_repository(context, |repo, project| {
        holds_pretrained_source(repo, project, &request.source_id)
    })?;
    let Some(LocatedLedger {
        repo,
        project,
        scope,
    }) = located
    else {
        return Err(anyhow!(
            "Fine-tune SAM 2 found no pretrained source {} of this board in this app's training ledger or in your private ledger on this device; import the SAM 2 weights with Import Pretrained Vision Weights or Import Model first",
            request.source_id
        ));
    };
    let ledger = repo.clone();
    let token = context.get_cancellation_token();
    let result = with_burn_cancellation(token, move |cancellation| {
        flow_like_ml_runtime::engines::fine_tune_sam2_source(
            &repo,
            &project,
            &request.source_id,
            &request.config,
            &request.data,
            &request.compute,
            request.maximum_bytes,
            &cancellation,
            |_| {},
            flow_like_types::json::json!({"fine_tuned_from": request.source_id}),
            flow_like_ml_runtime::now_ms(),
        )
        .map_err(flow_like_types::Error::from)
    })
    .await?;
    publish_fine_tuned(context, &ledger, scope, &result.source).await;
    Ok(result)
}

/// Publishes a fine-tuned source to the model root of its ledger's scope; a failure is a
/// warning.
#[cfg(feature = "training")]
async fn publish_fine_tuned(
    context: &mut ExecutionContext,
    repo: &flow_like_ml_runtime::TrainingRepository,
    scope: super::lifecycle::LedgerScope,
    source: &flow_like_ml_runtime::PretrainedSource,
) {
    let outcome = super::publisher::publish_pretrained_source(context, repo, scope, source).await;
    let model = format!("Fine-tuned SAM 2 source {}", source.id);
    super::lifecycle::log_publish_outcome(context, &model, &outcome);
}

/// Runs blocking SAM work with a Burn token that is cancelled when the run is cancelled or the
/// node is dropped; fine-tuning checks it per batch and again before publishing weights.
#[cfg(feature = "training")]
async fn with_burn_cancellation<T, W>(
    token: Option<flow_like_types::tokio_util::sync::CancellationToken>,
    run: W,
) -> Result<T>
where
    T: Send + 'static,
    W: FnOnce(flow_like_ml_burn::CancellationToken) -> Result<T> + Send + 'static,
{
    let cancellation = flow_like_ml_burn::CancellationToken::new();
    let worker = cancellation.clone();
    let work = tokio::task::spawn_blocking(move || run(worker));
    super::join_cancellable(token, move || cancellation.cancel(), work).await
}

#[cfg(all(test, feature = "training"))]
mod tests {
    use super::*;
    use flow_like_types::tokio_util::sync::CancellationToken;
    use futures::FutureExt;
    use std::time::{Duration, Instant};

    fn wait_until_cancelled(cancellation: &flow_like_ml_burn::CancellationToken) -> bool {
        let deadline = Instant::now() + Duration::from_secs(30);
        while !cancellation.is_cancelled() && Instant::now() < deadline {
            std::thread::sleep(Duration::from_millis(1));
        }
        cancellation.is_cancelled()
    }

    #[tokio::test]
    async fn dropping_a_fine_tune_cancels_the_burn_token_it_trains_with() {
        let (stopped, observed) = std::sync::mpsc::channel();
        let fine_tune = with_burn_cancellation(None, move |cancellation| {
            let _ = stopped.send(wait_until_cancelled(&cancellation));
            Ok(())
        });
        assert!(fine_tune.now_or_never().is_none());
        assert!(observed.recv_timeout(Duration::from_secs(30)).unwrap());
    }

    #[tokio::test]
    async fn a_fired_run_token_cancels_the_burn_token_and_returns_the_training_error() {
        let token = CancellationToken::new();
        token.cancel();
        let error = with_burn_cancellation(Some(token), |cancellation| {
            if wait_until_cancelled(&cancellation) {
                return Err(anyhow!("stopped by the Burn token"));
            }
            Ok(())
        })
        .await
        .unwrap_err();
        assert_eq!(error.to_string(), "stopped by the Burn token");
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn fine_tuning_takes_its_source_from_the_ledger_that_holds_it() {
        use super::super::lifecycle::{LedgerScope, scoped_repository, test_device::*};
        use flow_like_ml_burn::{
            BackendChoice, Sam2Config, Sam2Point, Sam2Prompt, Sam2TrainingConfig,
            Sam2TrainingDataset, Sam2Variant, TensorData,
        };
        use flow_like_types::json::json;
        let device = Device::new();
        let mut context = plain_context(device.stores()).await;
        let request = |source_id: &str| FineTuneSam2Request {
            source_id: source_id.into(),
            config: Sam2TrainingConfig {
                model: Sam2Config {
                    variant: Sam2Variant::HieraTiny,
                },
                backend: BackendChoice::Cpu,
                epochs: 1,
                batch_size: 1,
                learning_rate: 1e-4,
                seed: 7,
                gradient_clip: 1.0,
                freeze_image_encoder: true,
            },
            data: Sam2TrainingDataset {
                images: TensorData {
                    shape: vec![1, 3, 1024, 1024],
                    values: vec![0.0; 3 * 1024 * 1024],
                },
                prompts: vec![Sam2Prompt {
                    points: vec![Sam2Point {
                        x: 512.0,
                        y: 512.0,
                        label: 1,
                    }],
                    ..Default::default()
                }],
                masks: TensorData {
                    shape: vec![1, 1, 256, 256],
                    values: vec![0.0; 256 * 256],
                },
            },
            compute: Default::default(),
            maximum_bytes: default_artifact_limit(),
        };
        let error = fine_tune_sam2(&mut context, request("pretrained:missing"))
            .await
            .unwrap_err()
            .to_string();
        assert!(
            error.contains("found no pretrained source pretrained:missing"),
            "{error}"
        );
        assert!(!device.root().join("training-ledgers").exists());
        assert!(!device.root().join("apps").exists());
        let (user, project) = scoped_repository(&context, LedgerScope::User).unwrap();
        let manifest = json!({
            "engine": "burn", "format_version": 1, "config": {}, "input_shape": [1]
        });
        let private = user
            .register_pretrained_source(
                &project,
                b"weights",
                manifest,
                json!({"source": "fixture"}),
                1024,
                flow_like_ml_runtime::now_ms(),
            )
            .unwrap();
        let error = fine_tune_sam2(&mut context, request(&private.id))
            .await
            .unwrap_err()
            .to_string();
        assert!(error.contains("SAM source format"), "{error}");
        let app_ledger = device.root().join("training-ledgers/apps/app/project");
        assert!(!app_ledger.exists(), "{}", app_ledger.display());
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    #[ignore = "needs the official SAM 2.1 Hiera-T checkpoint in FLOW_LIKE_SAM2_WEIGHTS and minutes of CPU; L3 covers predict_sam2_from_parts"]
    async fn fine_tuned_sources_publish_and_segment_without_a_ledger() {
        use super::super::lifecycle::test_device::*;
        use super::super::pretrained::{
            ImportPretrainedVisionRequest, PretrainedImportFormat, import_pretrained_vision,
        };
        use super::super::published::{MANIFEST_FILE, artifact_dir};
        use flow_like_ml_burn::{
            BackendChoice, PretrainedWeightsMetadata, Sam2Config, Sam2Point, Sam2Prompt,
            Sam2TrainingConfig, Sam2TrainingDataset, Sam2Variant,
        };
        use flow_like_types::json::{from_value, json};
        let checkpoint = std::env::var("FLOW_LIKE_SAM2_WEIGHTS").expect("FLOW_LIKE_SAM2_WEIGHTS");
        let device = Device::new();
        let mut context = plain_context(device.stores()).await;
        let weights =
            flow_like_catalog_core::FlowPath::from_pathbuf(checkpoint.into(), &mut context)
                .await
                .unwrap();
        let metadata = PretrainedWeightsMetadata {
            source: "https://dl.fbaipublicfiles.com/segment_anything_2/092824/sam2.1_hiera_tiny.pt"
                .into(),
            license: Some("Apache-2.0".into()),
            weights_license: Some("Apache-2.0".into()),
            revision: Some("092824/sam2.1_hiera_tiny".into()),
            expected_sha256: None,
        };
        let import = ImportPretrainedVisionRequest {
            weights,
            format: PretrainedImportFormat::Sam2,
            metadata,
            labels: Vec::new(),
            maximum_bytes: default_artifact_limit(),
        };
        let imported = import_pretrained_vision(&mut context, import)
            .await
            .unwrap();
        let pixels: Vec<f32> = (0..3 * 1024 * 1024)
            .map(|index| (index % 251) as f32 / 250.0)
            .collect();
        let prompt = Sam2Prompt {
            points: vec![Sam2Point {
                x: 512.0,
                y: 512.0,
                label: 1,
            }],
            ..Default::default()
        };
        let fine_tune = FineTuneSam2Request {
            source_id: imported.id,
            config: Sam2TrainingConfig {
                model: Sam2Config {
                    variant: Sam2Variant::HieraTiny,
                },
                backend: BackendChoice::Cpu,
                epochs: 1,
                batch_size: 1,
                learning_rate: 1e-4,
                seed: 7,
                gradient_clip: 1.0,
                freeze_image_encoder: true,
            },
            data: Sam2TrainingDataset {
                images: flow_like_ml_burn::TensorData {
                    shape: vec![1, 3, 1024, 1024],
                    values: pixels.clone(),
                },
                prompts: vec![prompt.clone()],
                masks: flow_like_ml_burn::TensorData {
                    shape: vec![1, 1, 256, 256],
                    values: (0..256 * 256)
                        .map(|index| if index % 256 < 128 { 1.0 } else { 0.0 })
                        .collect(),
                },
            },
            compute: Default::default(),
            maximum_bytes: default_artifact_limit(),
        };
        let tuned = run_node(FineTuneSam2Node, json!(fine_tune), device.stores())
            .await
            .unwrap();
        let tuned: flow_like_ml_runtime::Sam2FineTuneResult = from_value(tuned).unwrap();
        let folder = device
            .root()
            .join("apps/app/storage")
            .join(artifact_dir(&tuned.source.id));
        assert!(folder.join(MANIFEST_FILE).exists(), "{}", folder.display());
        let prompt = PromptSam2Request {
            source_id: tuned.source.id,
            images: flow_like_ml_core::TensorData {
                shape: vec![1, 3, 1024, 1024],
                values: pixels,
            },
            prompts: vec![prompt],
            compute: Default::default(),
        };
        let ledger = run_node(PromptSam2Node, json!(prompt), device.stores())
            .await
            .unwrap();
        let published = published_copy(&device.app, APP_MODELS).await;
        let files = run_node(
            PromptSam2Node,
            json!(prompt),
            stores(Some(published), Some(memory_store())),
        )
        .await
        .unwrap();
        assert_eq!(files, ledger);
    }

    #[tokio::test]
    async fn a_finished_fine_tune_leaves_its_burn_token_untouched() {
        let (token_out, token_in) = std::sync::mpsc::channel();
        let value = with_burn_cancellation(Some(CancellationToken::new()), move |cancellation| {
            let _ = token_out.send(cancellation);
            Ok(3)
        })
        .await
        .unwrap();
        assert_eq!(value, 3);
        assert!(!token_in.recv().unwrap().is_cancelled());
    }
}
