use flow_like::{
    flow::{
        execution::context::ExecutionContext,
        node::{Node, NodeLogic},
        pin::ValueType,
        variable::VariableType,
    },
    state::FlowLikeState,
};
use flow_like_types::{async_trait, json::json};
use nalgebra::DVector;

#[derive(Default)]
pub struct FloatVectorCosineSimilarityNode {}

impl FloatVectorCosineSimilarityNode {
    pub fn new() -> Self {
        FloatVectorCosineSimilarityNode {}
    }
}

#[async_trait]
impl NodeLogic for FloatVectorCosineSimilarityNode {
    async fn get_node(&self, _app_state: &FlowLikeState) -> Node {
        let mut node = Node::new(
            "float_vector_cosine_similarity",
            "Cosine Similarity",
            "Calculates the cosine similarity of two float vectors",
            "Utils/Math/Vector",
        );
        node.add_icon("/flow/icons/grip.svg");

        node.add_input_pin(
            "vector1",
            "Vector 1",
            "First float vector",
            VariableType::Float,
        )
        .set_value_type(ValueType::Array);
        node.add_input_pin(
            "vector2",
            "Vector 2",
            "Second float vector",
            VariableType::Float,
        )
        .set_value_type(ValueType::Array);

        node.add_output_pin(
            "similarity",
            "Similarity",
            "Cosine similarity of the two vectors",
            VariableType::Float,
        );

        return node;
    }

    async fn run(&self, context: &mut ExecutionContext) -> flow_like_types::Result<()> {
        let vector1: Vec<f64> = context.evaluate_pin("vector1").await?;
        let vector2: Vec<f64> = context.evaluate_pin("vector2").await?;

        let v1 = DVector::from_vec(vector1);
        let v2 = DVector::from_vec(vector2);

        if v1.len() != v2.len() {
            return Err(flow_like_types::anyhow!(
                "Vectors must have the same length"
            ));
        }

        let similarity = v1.dot(&v2) / (v1.norm() * v2.norm());

        context
            .set_pin_value("similarity", json!(similarity))
            .await?;
        Ok(())
    }
}
