/// # ONNX Model Loader Nodes
use crate::onnx::NodeOnnxSession;
#[cfg(feature = "execute")]
use crate::onnx::{Provider, SessionWithMeta, classification, detection};
#[cfg(feature = "execute")]
use crate::onnx::execution_providers::{get_ep_info, is_initialized};
use flow_like::flow::{
    execution::context::ExecutionContext,
    node::{Node, NodeLogic},
    pin::PinOptions,
    variable::VariableType,
};
use flow_like_catalog_core::FlowPath;
#[cfg(feature = "execute")]
use flow_like_model_provider::ml::ort::session::Session;
use flow_like_types::{Result, anyhow, async_trait, json::json};

// ## Loader Utilities
// Identifying ONNX-I/Os
#[cfg(feature = "execute")]
static DFINE_INPUTS: [&str; 2] = ["images", "orig_target_sizes"];
#[cfg(feature = "execute")]
static DFINE_OUTPUTS: [&str; 3] = ["labels", "boxes", "scores"];
#[cfg(feature = "execute")]
static YOLO_INPUTS: [&str; 1] = ["images"];
#[cfg(feature = "execute")]
static YOLO_OUTPUTS: [&str; 1] = ["output0"];
#[cfg(feature = "execute")]
static TIMM_INPUTS: [&str; 1] = ["input0"];
#[cfg(feature = "execute")]
static TIMM_OUTPUTS: [&str; 1] = ["output0"];

#[cfg(feature = "execute")]
/// Factory Function Matching ONNX Assets to a Provider-Frameworks
pub fn determine_provider(session: &Session) -> Result<Provider> {
    let input_names: Vec<&str> = session.inputs.iter().map(|i| i.name.as_str()).collect();
    let output_names: Vec<&str> = session.outputs.iter().map(|o| o.name.as_str()).collect();
    if input_names == DFINE_INPUTS && output_names == DFINE_OUTPUTS {
        let (input_width, input_height) = determine_input_shape(session, "images")?;
        Ok(Provider::DfineLike(detection::DfineLike {
            input_width,
            input_height,
        }))
    } else if input_names == YOLO_INPUTS && output_names == YOLO_OUTPUTS {
        let (input_width, input_height) = determine_input_shape(session, "images")?;
        Ok(Provider::YoloLike(detection::YoloLike {
            input_width,
            input_height,
        }))
    } else if input_names == TIMM_INPUTS && output_names == TIMM_OUTPUTS {
        let (input_width, input_height) = determine_input_shape(session, "input0")?;
        Ok(Provider::TimmLike(classification::TimmLike {
            input_width,
            input_height,
        }))
    } else {
        tracing::info!(
            "Model does not match known patterns, using Generic provider. Inputs: {:?}, Outputs: {:?}",
            input_names, output_names
        );
        Ok(Provider::Generic)
    }
}

#[cfg(feature = "execute")]
pub fn determine_input_shape(session: &Session, input_name: &str) -> Result<(u32, u32)> {
    for input in &session.inputs {
        if input.name == input_name
            && let Some(dims) = input.input_type.tensor_shape()
        {
            let d = dims.len();
            if d > 1 {
                let (w, h) = (dims[d - 2], dims[d - 1]);
                return Ok((w as u32, h as u32));
            }
        }
    }
    Err(anyhow!("Failed to determine input shape!"))
}

#[crate::register_node]
#[derive(Default)]
/// # Node to Load ONNX Runtime Session
/// Sets execution context cache
pub struct LoadOnnxNode {}

impl LoadOnnxNode {
    /// Create new LoadOnnxNode Instance
    pub fn new() -> Self {
        LoadOnnxNode {}
    }
}

#[async_trait]
impl NodeLogic for LoadOnnxNode {
    fn get_node(&self) -> Node {
        let mut node = Node::new(
            "load_onnx",
            "Load ONNX",
            "Load ONNX Model from Path",
            "AI/ML/ONNX",
        );

        node.add_icon("/flow/icons/find_model.svg");

        // inputs
        node.add_input_pin(
            "exec_in",
            "Input",
            "Initiate Execution",
            VariableType::Execution,
        );

        node.add_input_pin("path", "Path", "Path ONNX File", VariableType::Struct)
            .set_schema::<FlowPath>()
            .set_options(PinOptions::new().set_enforce_schema(true).build());

        // outputs
        node.add_output_pin(
            "exec_out",
            "Output",
            "Done with the Execution",
            VariableType::Execution,
        );

        node.add_output_pin("model", "Model", "ONNX Model Session", VariableType::Struct)
            .set_schema::<NodeOnnxSession>()
            .set_options(PinOptions::new().set_enforce_schema(true).build());

        node.add_output_pin(
            "accelerated",
            "Accelerated",
            "Whether GPU/NPU acceleration is active",
            VariableType::Boolean,
        );

        node.add_output_pin(
            "active_provider",
            "Active Provider",
            "The execution provider(s) that are actually in use",
            VariableType::String,
        );

        node
    }

    #[allow(unused_variables)]
    async fn run(&self, context: &mut ExecutionContext) -> Result<()> {
        #[cfg(feature = "execute")]
        {
            context.deactivate_exec_pin("exec_out").await?;

            // fetch inputs
            let path: FlowPath = context.evaluate_pin("path").await?;
            let bytes = path.get(context, false).await?;

            // Get global EP info (ORT should be initialized at app startup)
            let ep_info = get_ep_info().unwrap_or_default();
            if !is_initialized() {
                tracing::warn!(
                    "ORT not initialized - call initialize_ort() at app startup for GPU acceleration"
                );
            }

            // Build session - it will use the globally configured EPs
            let session = Session::builder()?.commit_from_memory(&bytes)?;

            // wrap ONNX session with provider metadata
            // we try to determine the here to fail fast in case of incompatible ONNX assets
            let provider = determine_provider(&session)?;
            let session_with_meta = SessionWithMeta {
                session,
                provider,
                ep_active: ep_info.active_providers.clone(),
                accelerated: ep_info.accelerated,
            };
            let node_session = NodeOnnxSession::new(context, session_with_meta).await;

            // set outputs
            context.set_pin_value("model", json!(node_session)).await?;
            context
                .set_pin_value("accelerated", json!(ep_info.accelerated))
                .await?;
            context
                .set_pin_value(
                    "active_provider",
                    json!(ep_info.active_providers.join(", ")),
                )
                .await?;
            context.activate_exec_pin("exec_out").await?;
            Ok(())
        }

        #[cfg(not(feature = "execute"))]
        {
            Err(anyhow!(
                "ONNX execution requires the 'execute' feature. Rebuild with --features execute"
            ))
        }
    }
}
