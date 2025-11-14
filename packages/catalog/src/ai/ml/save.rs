//! Node for Serializing Trained MLModels as JSONs
//!
//! Serializes MLModels as JSONs and writes to a specified path.

use crate::ai::ml::NodeMLModel;
use crate::data::path::FlowPath;
use flow_like::{
    flow::{
        execution::context::ExecutionContext,
        node::{Node, NodeLogic},
        pin::PinOptions,
        variable::VariableType,
    },
    state::FlowLikeState,
};
use flow_like_types::{Result, async_trait};

#[crate::register_node]
#[derive(Default)]
pub struct SaveMLModelNode {}

impl SaveMLModelNode {
    pub fn new() -> Self {
        SaveMLModelNode {}
    }
}

#[async_trait]
impl NodeLogic for SaveMLModelNode {
    async fn get_node(&self, _app_state: &FlowLikeState) -> Node {
        let mut node = Node::new(
            "save_ml_model",
            "Save Model",
            "Save Trained ML Model to Path",
            "AI/ML",
        );
        node.add_icon("/flow/icons/chart-network.svg");

        node.add_input_pin("exec_in", "Input", "Start Saving", VariableType::Execution);

        node.add_input_pin(
            "model",
            "Model",
            "Trained KMeans Clustering Model",
            VariableType::Struct,
        )
        .set_schema::<NodeMLModel>()
        .set_options(PinOptions::new().set_enforce_schema(true).build());

        node.add_input_pin(
            "path",
            "Path JSON",
            "Path to Save Model to (JSON)",
            VariableType::Struct,
        )
        .set_schema::<FlowPath>()
        .set_options(PinOptions::new().set_enforce_schema(true).build());

        node.add_output_pin(
            "exec_out",
            "Done",
            "Done Saving Model",
            VariableType::Execution,
        );

        return node;
    }

    async fn run(&self, context: &mut ExecutionContext) -> Result<()> {
        // fetch inputs
        context.deactivate_exec_pin("exec_out").await?;
        let node_model: NodeMLModel = context.evaluate_pin("model").await?;
        let path: FlowPath = context.evaluate_pin("path").await?;

        // set extension
        let path = path.set_extension(context, "json").await?;

        // serialize model
        let bytes = {
            let model = node_model.get_model(context).await?;
            let model_guard = model.lock().await;
            model_guard.to_json_vec()?
        };

        // write
        path.put(context, bytes, false).await?;

        context.activate_exec_pin("exec_out").await?;
        Ok(())
    }
}
