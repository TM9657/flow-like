use std::{collections::{HashMap, HashSet}, sync::Arc, time::Duration};
use flow_like::{flow::{board::Board, execution::{context::ExecutionContext, internal_node::InternalNode, log::LogMessage, LogLevel}, node::{Node, NodeLogic}, pin::{PinOptions, PinType, ValueType}, variable::{Variable, VariableType}}, state::{FlowLikeState, ToastLevel}};
use flow_like_types::{async_trait, json::{json, Deserialize, Serialize}, reqwest, sync::{DashMap, Mutex}, Bytes, Error, JsonSchema, Value};
use nalgebra::DVector;
use regex::Regex;
use flow_like_storage::{object_store::PutPayload, Path};
use futures::StreamExt;
use crate::{storage::path::FlowPath, web::api::{HttpBody, HttpRequest, HttpResponse, Method}};

#[derive(Default)]
pub struct BranchNode {}

impl BranchNode {
    pub fn new() -> Self {
        BranchNode {}
    }
}

#[async_trait]
impl NodeLogic for BranchNode {
    async fn get_node(&self, _app_state: &FlowLikeState) -> Node {
        let mut node = Node::new(
            "control_branch",
            "Branch",
            "Branches the flow based on a condition",
            "Control",
        );
        node.add_icon("/flow/icons/split.svg");

        node.add_input_pin("exec_in", "Input", "Trigger Pin", VariableType::Execution);
        node.add_input_pin(
            "condition",
            "Condition",
            "The condition to evaluate",
            VariableType::Boolean,
        )
        .set_default_value(Some(flow_like_types::json::json!(true)));

        node.add_output_pin(
            "true",
            "True",
            "The flow to follow if the condition is true",
            VariableType::Execution,
        );
        node.add_output_pin(
            "false",
            "False",
            "The flow to follow if the condition is false",
            VariableType::Execution,
        );

        return node;
    }

    async fn run(&self, context: &mut ExecutionContext) ->flow_like_types::Result<()> {
        let condition = context.evaluate_pin::<bool>("condition").await?;

        let true_pin = context.get_pin_by_name("true").await?;
        let false_pin = context.get_pin_by_name("false").await?;

        if condition {
            context.activate_exec_pin_ref(&true_pin).await?;
            context.deactivate_exec_pin_ref(&false_pin).await?;

            return Ok(());
        }

        context.deactivate_exec_pin_ref(&true_pin).await?;
        context.activate_exec_pin_ref(&false_pin).await?;

        return Ok(());
    }
}
