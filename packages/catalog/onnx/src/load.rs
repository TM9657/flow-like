/// # ONNX Model Loader Nodes
use crate::onnx::{NodeOnnxSession, Provider, SessionWithMeta, classification, detection};
use flow_like::{
    flow::{
        execution::context::ExecutionContext,
        node::{Node, NodeLogic},
        pin::PinOptions,
        variable::VariableType,
    },
    state::FlowLikeState,
};
use flow_like_catalog_core::FlowPath;
#[cfg(feature = "local-ml")]
use flow_like_model_provider::ml::ort::session::Session;
use flow_like_types::{Error, Result, anyhow, async_trait, json::json};

// ## Loader Utilities
// Identifying ONNX-I/Os
static DFINE_INPUTS: [&str; 2] = ["images", "orig_target_sizes"];
static DFINE_OUTPUTS: [&str; 3] = ["labels", "boxes", "scores"];
static YOLO_INPUTS: [&str; 1] = ["images"];
static YOLO_OUTPUTS: [&str; 1] = ["output0"];
static TIMM_INPUTS: [&str; 1] = ["input0"];
static TIMM_OUTPUTS: [&str; 1] = ["output0"];

#[cfg(feature = "local-ml")]
/// Factory Function Matching ONNX Assets to a Provider-Frameworks
pub fn determine_provider(session: &Session) -> Result<Provider, Error> {
    let input_names: Vec<&str> = session.inputs.iter().map(|i| i.name.as_str()).collect();
    let output_names: Vec<&str> = session.outputs.iter().map(|o| o.name.as_str()).collect();
    if input_names == DFINE_INPUTS && output_names == DFINE_OUTPUTS {
        let (input_width, input_height) = determine_input_shape(session, "images")?;
        println!(
            "Model is DfineLike with input shape ({},{})",
            input_width, input_height
        );
        Ok(Provider::DfineLike(detection::DfineLike {
            input_width,
            input_height,
        }))
    } else if input_names == YOLO_INPUTS && output_names == YOLO_OUTPUTS {
        let (input_width, input_height) = determine_input_shape(session, "images")?;
        println!(
            "Model is YoloLike with input shape ({},{})",
            input_width, input_height
        );
        Ok(Provider::YoloLike(detection::YoloLike {
            input_width,
            input_height,
        }))
    } else if input_names == TIMM_INPUTS && output_names == TIMM_OUTPUTS {
        let (input_width, input_height) = determine_input_shape(session, "input0")?;
        println!(
            "Model is TimmLike with input shape ({},{})",
            input_width, input_height
        );
        Ok(Provider::TimmLike(classification::TimmLike {
            input_width,
            input_height,
        }))
    } else {
        Err(anyhow!("Failed to determine provider!"))
    }
}

#[cfg(feature = "local-ml")]
pub fn determine_input_shape(session: &Session, input_name: &str) -> Result<(u32, u32), Error> {
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
    async fn get_node(&self, _app_state: &FlowLikeState) -> Node {
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

        node
    }

    async fn run(&self, context: &mut ExecutionContext) -> Result<()> {
        #[cfg(feature = "local-ml")]
        {
            context.deactivate_exec_pin("exec_out").await?;

            // fetch inputs
            let path: FlowPath = context.evaluate_pin("path").await?;
            let bytes = path.get(context, false).await?;

            // init ONNX session
            let session = Session::builder()?.commit_from_memory(&bytes)?;

            // wrap ONNX session with provider metadata
            // we try to determine the here to fail fast in case of incompatible ONNX assets
            let provider = determine_provider(&session)?;
            let session_with_meta = SessionWithMeta { session, provider };
            let node_session = NodeOnnxSession::new(context, session_with_meta).await;

            // set outputs
            context.set_pin_value("model", json!(node_session)).await?;
            context.activate_exec_pin("exec_out").await?;
            Ok(())
        }

        #[cfg(not(feature = "local-ml"))]
        {
            Err(anyhow!(
                "Local ONNX models are not supported. Please enable the 'local-ml' feature."
            ))
        }
    }
}
