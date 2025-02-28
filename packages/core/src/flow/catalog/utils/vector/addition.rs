use crate::{
    flow::{
        execution::context::ExecutionContext,
        node::{Node, NodeLogic},
        variable::VariableType,
    },
    state::FlowLikeState,
};
use async_trait::async_trait;
use serde_json::json;

#[derive(Default)]
pub struct FloatVectorAdditionNode {}

impl FloatVectorAdditionNode {
    pub fn new() -> Self {
        FloatVectorAdditionNode {}
    }
}

#[async_trait]
impl NodeLogic for FloatVectorAdditionNode {
    async fn get_node(&self, _app_state: &FlowLikeState) -> Node {
        let mut node = Node::new(
            "float_vector_addition",
            "Addition",
            "Adds two float vectors together element-wise",
            "Utils/Math/Vector",
        );
        node.add_icon("/flow/icons/grip.svg");

        node.add_input_pin(
            "vector1",
            "Vector 1",
            "First float vector",
            VariableType::Float,
        )
        .set_value_type(crate::flow::pin::ValueType::Array);
        node.add_input_pin(
            "vector2",
            "Vector 2",
            "Second float vector",
            VariableType::Float,
        )
        .set_value_type(crate::flow::pin::ValueType::Array);

        node.add_output_pin(
            "result_vector",
            "Result Vector",
            "Sum of the two vectors",
            VariableType::Float,
        )
        .set_value_type(crate::flow::pin::ValueType::Array);

        return node;
    }

    async fn run(&mut self, context: &mut ExecutionContext) -> anyhow::Result<()> {
        let vector1: Vec<f64> = context.evaluate_pin("vector1").await?;
        let vector2: Vec<f64> = context.evaluate_pin("vector2").await?;

        if vector1.len() != vector2.len() {
            return Err(anyhow::anyhow!("Vectors must have the same length"));
        }

        let result_vector: Vec<f64> = vector1
            .iter()
            .zip(vector2.iter())
            .map(|(a, b)| a + b)
            .collect();

        context
            .set_pin_value("result_vector", json!(result_vector))
            .await?;
        Ok(())
    }
}
