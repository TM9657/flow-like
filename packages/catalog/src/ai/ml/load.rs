//! Node for Deserializing a MLModel from JSON path
//!
//! Deserializes a previously trained and saved MLModel JSON file as the matching MLModel variant.
//! Wraps the MLModel in a cached NodeMLModel.

use crate::ai::ml::{MLModel, NodeMLModel};
use crate::data::path::FlowPath;
use flow_like::{
    flow::{
        execution::{LogLevel, context::ExecutionContext},
        node::{Node, NodeLogic},
        pin::PinOptions,
        variable::VariableType,
    },
    state::FlowLikeState,
};
use flow_like_types::{Result, async_trait, json};

#[derive(Default)]
pub struct LoadMLModelNode {}

impl LoadMLModelNode {
    pub fn new() -> Self {
        LoadMLModelNode {}
    }
}

#[async_trait]
impl NodeLogic for LoadMLModelNode {
    async fn get_node(&self, _app_state: &FlowLikeState) -> Node {
        let mut node = Node::new(
            "load_ml_model",
            "Load Model",
            "Load Trained ML Model from Path",
            "AI/ML",
        );
        node.add_icon("/flow/icons/chart-network.svg");

        node.add_input_pin("exec_in", "Input", "Start Loading", VariableType::Execution);

        node.add_input_pin(
            "path",
            "Path JSON",
            "Path to Load the Model from (JSON)",
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

        node.add_output_pin(
            "model",
            "Model",
            "Loaded Machine Learning Model",
            VariableType::Struct,
        )
        .set_schema::<NodeMLModel>()
        .set_options(PinOptions::new().set_enforce_schema(true).build());

        return node;
    }

    async fn run(&self, context: &mut ExecutionContext) -> Result<()> {
        // fetch inputs
        context.deactivate_exec_pin("exec_out").await?;
        let path: FlowPath = context.evaluate_pin("path").await?;

        // deserialize model
        let bytes = path.get(context, false).await?;
        let ml_model: MLModel = json::from_slice(&bytes)?;
        context.log_message(
            &format!("Loaded Machine Learning Model: {}", &ml_model),
            LogLevel::Debug,
        );

        // wrap model + set outputs
        let node_model = NodeMLModel::new(context, ml_model).await;
        context
            .set_pin_value("model", json::json!(node_model))
            .await?;
        context.activate_exec_pin("exec_out").await?;
        Ok(())
    }
}
