/// # ONNX Model Loader Nodes

use crate::{
    ai::ml::onnx::NodeOnnxSession, 
    storage::path::FlowPath
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
use flow_like_types::{
    Ok, 
    Result,
    async_trait,
    ort::session::Session,
    json::json,
};

#[derive(Default)]
/// # Node to Load ONNX Runtime Session
/// Sets execution context cache
pub struct LoadOnnxNode {}

impl LoadOnnxNode {
    /// Create new LoadOnnxNode Instance
    pub fn new() -> Self {
        LoadOnnxNode {  }
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

        node.add_input_pin("path", "Path", "FlowPath", VariableType::Struct)
            .set_schema::<FlowPath>()
            .set_options(PinOptions::new()
            .set_enforce_schema(true).build());

        
        // outputs 
        node.add_output_pin(
            "exec_out",
            "Output",
            "Done with the Execution",
            VariableType::Execution,
        );

        node.add_output_pin(
            "model", 
            "Model", 
            "ONNX Model Session", 
            VariableType::Struct
        )
            .set_schema::<NodeOnnxSession>();

        node

    }

    async fn run(&self, context: &mut ExecutionContext) -> Result<()> {
        context.deactivate_exec_pin("exec_out").await?;
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
        let session = Session::builder()?
            .commit_from_memory(&bytes)?;
        for input in &session.inputs {
            // todo: dynamically read input names in inference node
            context.log_message(&format!("model input: {:?}", input.name), LogLevel::Debug);
        }
        let node_session = NodeOnnxSession::new(context, session).await;
        context.set_pin_value("model", json!(node_session)).await?;
        context.activate_exec_pin("exec_out").await?;
        Ok(())
    }
}