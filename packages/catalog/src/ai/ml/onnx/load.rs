/// # ONNX Model Loader Nodes
use crate::{
    ai::ml::onnx::{NodeOnnxSession, SessionWithMeta},
    storage::path::FlowPath,
};
use flow_like::{
    flow::{
        execution::{LogLevel, context::ExecutionContext},
        node::{Node, NodeLogic},
        pin::PinOptions,
        variable::VariableType,
    },
    state::FlowLikeState,
};
use flow_like_model_provider::ml::ort::session::Session;
use flow_like_types::{Error, Result, anyhow, async_trait, json::json};

/// Determine input-tensor name and shape to resize our images accordingly
/// For simplicity, we are assuming that the first tensor is the image-related one.
fn determine_onnx_input(session: &Session) -> Result<(String, u32, u32), Error> {
    for input in &session.inputs {
        if let Some(dims) = input.input_type.tensor_dimensions() {
            let d = dims.len();
            if d > 1 {
                let (w, h) = (dims[d - 2], dims[d - 1]);
                return Ok((String::from(&input.name), w as u32, h as u32));
            }
        }
    }
    Err(anyhow!(
        "Failed to determine ONNX model input - no input tensor found!"
    ))
}

/// Determine output-tensor name to extract as array
/// For simplicity, we are assuming that the first tensor is the prediction-related one.
fn determine_onnx_output(session: &Session) -> Result<String, Error> {
    for output in &session.outputs {
        if let Some(dims) = output.output_type.tensor_dimensions() {
            let d = dims.len();
            if d > 1 {
                //let (w, h) = (dims[d - 2], dims[d - 1]);
                return Ok(String::from(&output.name));
            }
        }
    }
    Err(anyhow!(
        "Failed to determine ONNX model output - no output tensor found!"
    ))
}

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
            .set_schema::<NodeOnnxSession>();

        node
    }

    async fn run(&self, context: &mut ExecutionContext) -> Result<()> {
        context.deactivate_exec_pin("exec_out").await?;

        // fetch inputs
        let path: FlowPath = context.evaluate_pin("path").await?;
        let path_runtime = path.to_runtime(context).await?;
        let bytes = path_runtime
            .store
            .as_generic()
            .get(&path_runtime.path)
            .await?
            .bytes()
            .await?
            .to_vec();

        // init ONNX session
        let session = Session::builder()?.commit_from_memory(&bytes)?;

        // wrap session with input/output specs
        // we try to determine the specs once here to fail fast at model-load time and not at model-evaluate time later
        let (input_name, input_width, input_height) = determine_onnx_input(&session)?;
        context.log_message(
            &format!(
                "model input tensor {:?} with shape ({:?}, {:?})",
                input_name, input_width, input_height
            ),
            LogLevel::Debug,
        );
        let output_name = determine_onnx_output(&session)?;
        context.log_message(
            &format!("model output tensor {:?}", output_name),
            LogLevel::Debug,
        );
        let session_with_meta = SessionWithMeta {
            session,
            input_name,
            input_width,
            input_height,
            output_name,
            classes: None,
        };
        let node_session = NodeOnnxSession::new(context, session_with_meta).await;

        // set outputs
        context.set_pin_value("model", json!(node_session)).await?;
        context.activate_exec_pin("exec_out").await?;
        Ok(())
    }
}
