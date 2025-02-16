use crate::{
    bit::BitModelPreference,
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
pub struct FindLLMNode {}

impl FindLLMNode {
    pub fn new() -> Self {
        FindLLMNode {}
    }
}

#[async_trait]
impl NodeLogic for FindLLMNode {
    async fn get_node(&self, _app_state: &FlowLikeState) -> Node {
        let mut node = Node::new(
            "ai_generative_find_model",
            "Find Model",
            "Finds the best model based on certain selection criteria",
            "AI/Generative",
        );
        node.add_icon("/flow/icons/find_model.svg");

        node.add_input_pin("exec_in", "Input", "Trigger Pin", VariableType::Execution);
        node.add_input_pin("hint", "Hint", "Hinted model", VariableType::String)
            .set_default_value(Some(json!("")));

        node.add_input_pin(
            "cost",
            "Cost",
            "Cost of the Model to run",
            VariableType::Float,
        )
        .set_default_value(Some(json!(0.0)));

        node.add_input_pin(
            "coding",
            "Coding",
            "Coding capacities of the model",
            VariableType::Float,
        )
        .set_default_value(Some(json!(0.0)));

        node.add_input_pin(
            "creativity",
            "Creativity",
            "Creativity of the model",
            VariableType::Float,
        )
        .set_default_value(Some(json!(0.0)));

        node.add_input_pin(
            "factfulness",
            "Factfulness",
            "Factfulness of the model",
            VariableType::Float,
        )
        .set_default_value(Some(json!(0.0)));

        node.add_input_pin(
            "function_calling",
            "Function Calling",
            "Function Calling of the model",
            VariableType::Float,
        )
        .set_default_value(Some(json!(0.0)));

        node.add_input_pin(
            "multilinguality",
            "Multilinguality",
            "Multilinguality of the model",
            VariableType::Float,
        )
        .set_default_value(Some(json!(0.0)));

        node.add_input_pin(
            "openness",
            "Openness",
            "Openness of the model",
            VariableType::Float,
        )
        .set_default_value(Some(json!(0.0)));

        node.add_input_pin(
            "reasoning",
            "Reasoning",
            "Reasoning of the model",
            VariableType::Float,
        )
        .set_default_value(Some(json!(0.0)));

        node.add_input_pin(
            "safety",
            "Safety",
            "Safety of the model",
            VariableType::Float,
        )
        .set_default_value(Some(json!(0.0)));

        node.add_input_pin("speed", "Speed", "Speed of the model", VariableType::Float)
            .set_default_value(Some(json!(0.0)));

        node.add_output_pin("exec_out", "Output", "Done", VariableType::Execution);
        node.add_output_pin("model", "Model", "The selected model", VariableType::Struct);

        node.set_long_running(true);

        return node;
    }

    async fn run(&mut self, context: &mut ExecutionContext) -> anyhow::Result<()> {
        let mut preference: BitModelPreference = BitModelPreference {
            cost_weight: None,
            coding_weight: None,
            creativity_weight: None,
            factfulness_weight: None,
            function_calling_weight: None,
            multilinguality_weight: None,
            openness_weight: None,
            reasoning_weight: None,
            safety_weight: None,
            speed_weight: None,
            model_hint: None,
        };

        let hint: String = context.evaluate_pin("hint").await?;
        if !hint.is_empty() {
            preference.model_hint = Some(hint);
        }

        let cost: f32 = context.evaluate_pin("cost").await?;
        if cost > 0.0 {
            preference.coding_weight = Some(if cost > 1.0 { 1.0 } else { cost });
        }

        let coding: f32 = context.evaluate_pin("coding").await?;
        if coding > 0.0 {
            preference.coding_weight = Some(if coding > 1.0 { 1.0 } else { coding });
        }

        let creativity: f32 = context.evaluate_pin("creativity").await?;
        if creativity > 0.0 {
            preference.creativity_weight = Some(if creativity > 1.0 { 1.0 } else { creativity });
        }

        let factfulness: f32 = context.evaluate_pin("factfulness").await?;
        if factfulness > 0.0 {
            preference.factfulness_weight = Some(if factfulness > 1.0 { 1.0 } else { factfulness });
        }

        let function_calling: f32 = context.evaluate_pin("function_calling").await?;
        if function_calling > 0.0 {
            preference.function_calling_weight = Some(if function_calling > 1.0 {
                1.0
            } else {
                function_calling
            });
        }

        let multilinguality: f32 = context.evaluate_pin("multilinguality").await?;
        if multilinguality > 0.0 {
            preference.multilinguality_weight = Some(if multilinguality > 1.0 {
                1.0
            } else {
                multilinguality
            });
        }

        let openness: f32 = context.evaluate_pin("openness").await?;
        if openness > 0.0 {
            preference.openness_weight = Some(if openness > 1.0 { 1.0 } else { openness });
        }

        let reasoning: f32 = context.evaluate_pin("reasoning").await?;
        if reasoning > 0.0 {
            preference.reasoning_weight = Some(if reasoning > 1.0 { 1.0 } else { reasoning });
        }

        let safety: f32 = context.evaluate_pin("safety").await?;
        if safety > 0.0 {
            preference.safety_weight = Some(if safety > 1.0 { 1.0 } else { safety });
        }

        let speed: f32 = context.evaluate_pin("speed").await?;
        if speed > 0.0 {
            preference.speed_weight = Some(if speed > 1.0 { 1.0 } else { speed });
        }

        let http_client = context.app_state.lock().await.http_client.clone();
        let bit = context
            .profile
            .get_best_model(&preference, false, false, http_client)
            .await?;
        context
            .set_pin_value("model", serde_json::json!(bit))
            .await?;

        context.activate_exec_pin("exec_out").await?;

        return Ok(());
    }
}
