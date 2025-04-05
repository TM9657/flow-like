use strsim::sorensen_dice;

use std::{collections::{HashMap, HashSet}, sync::Arc};
use flow_like::{flow::{board::Board, execution::{context::ExecutionContext, internal_node::InternalNode, log::LogMessage, LogLevel}, node::{Node, NodeLogic}, pin::{PinOptions, ValueType}, variable::{Variable, VariableType}}, state::FlowLikeState};
use flow_like_types::{async_trait, json::json, reqwest, sync::{DashMap, Mutex}, Value};

use crate::{storage::path::FlowPath, web::api::{HttpBody, HttpRequest, HttpResponse, Method}};

#[derive(Default)]
pub struct SorensenDiceCoefficientNode {}

impl SorensenDiceCoefficientNode {
    pub fn new() -> Self {
        SorensenDiceCoefficientNode {}
    }
}

#[async_trait]
impl NodeLogic for SorensenDiceCoefficientNode {
    async fn get_node(&self, _app_state: &FlowLikeState) -> Node {
        let mut node = Node::new(
            "sorensen_dice_coefficient",
            "Sørensen-Dice Coefficient",
            "Calculates the Sørensen-Dice coefficient between two strings",
            "Utils/String/Similarity",
        );
        node.add_icon("/flow/icons/distance.svg");

        node.add_input_pin("string1", "String 1", "First String", VariableType::String);
        node.add_input_pin("string2", "String 2", "Second String", VariableType::String);

        node.add_output_pin(
            "coefficient",
            "Coefficient",
            "Sørensen-Dice Coefficient",
            VariableType::Float,
        );

        return node;
    }

    async fn run(&self, context: &mut ExecutionContext) ->flow_like_types::Result<()> {
        let string1: String = context.evaluate_pin("string1").await?;
        let string2: String = context.evaluate_pin("string2").await?;

        let coefficient = sorensen_dice(&string1, &string2);

        context
            .set_pin_value("coefficient", json!(coefficient))
            .await?;

        Ok(())
    }
}
