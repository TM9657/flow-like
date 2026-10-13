use flow_like::flow::{execution::context::ExecutionContext, node::NodeLogic};
use flow_like_types::{Result, anyhow};
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
        #[cfg(feature = "training")]
        {
            context.deactivate_exec_pin("exec_out").await?;
            let request: PromptSam2Request = context.evaluate_pin("request").await?;
            let (repo, project) = super::lifecycle::repository(context)?;
            if context.is_cancelled() {
                return Err(anyhow!("SAM prediction cancelled"));
            }
            let result = tokio::task::spawn_blocking(move || {
                flow_like_ml_runtime::engines::predict_sam2_source(
                    &repo,
                    &project,
                    &request.source_id,
                    &request.images,
                    &request.prompts,
                    &request.compute,
                )
            })
            .await??;
            if context.is_cancelled() {
                return Err(anyhow!("SAM prediction cancelled"));
            }
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
                "Native SAM prediction requires a local training build"
            ))
        }
    }
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
            if !(1..=2 * 1024 * 1024 * 1024).contains(&request.maximum_bytes) {
                return Err(anyhow!(
                    "SAM artifact byte limit must be between 1 and 2 GiB, got {} bytes",
                    request.maximum_bytes
                ));
            }
            if context.is_cancelled() {
                return Err(anyhow!("SAM fine-tuning cancelled"));
            }
            let (repo, project) = super::lifecycle::repository(context)?;
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
            context
                .set_pin_value("result", flow_like_types::json::json!(result))
                .await?;
            context.activate_exec_pin("exec_out").await
        }
        #[cfg(not(feature = "training"))]
        {
            let _ = context;
            Err(anyhow!("SAM fine-tuning requires a local training build"))
        }
    }
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
