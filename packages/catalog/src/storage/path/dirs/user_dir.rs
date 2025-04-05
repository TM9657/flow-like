use std::{collections::{HashMap, HashSet}, sync::Arc, time::Duration};
use flow_like::{flow::{board::Board, execution::{context::ExecutionContext, internal_node::InternalNode, log::LogMessage, LogLevel}, node::{Node, NodeLogic}, pin::{PinOptions, PinType, ValueType}, variable::{Variable, VariableType}}, state::FlowLikeState};
use flow_like_types::{async_trait, json::{json, Deserialize, Serialize}, reqwest, sync::{DashMap, Mutex}, Bytes, Error, JsonSchema, Value};
use nalgebra::DVector;
use regex::Regex;
use flow_like_storage::{object_store::PutPayload, Path};
use futures::StreamExt;
use crate::{storage::path::FlowPath, web::api::{HttpBody, HttpRequest, HttpResponse, Method}};

#[derive(Default)]
pub struct PathFromUserDirNode {}

impl PathFromUserDirNode {
    pub fn new() -> Self {
        PathFromUserDirNode {}
    }
}

#[async_trait]
impl NodeLogic for PathFromUserDirNode {
    async fn get_node(&self, _app_state: &FlowLikeState) -> Node {
        let mut node = Node::new(
            "path_from_user_dir",
            "User Dir",
            "Converts the user directory to a Path",
            "Storage/Paths/Directories",
        );
        node.add_icon("/flow/icons/path.svg");

        node.add_input_pin(
            "exec_in",
            "Input",
            "Initiate Execution",
            VariableType::Execution,
        );

        node.add_output_pin(
            "exec_out",
            "Output",
            "Done with the Execution",
            VariableType::Execution,
        );

        node.add_output_pin("path", "Path", "Output Path", VariableType::Struct)
            .set_schema::<FlowPath>();

        node.add_input_pin(
            "node_scope",
            "Node Scope",
            "Is this node in the node scope?",
            VariableType::Boolean,
        );

        node.add_output_pin(
            "failed",
            "Failed",
            "Not possible, for example on server, certain directories are not accessible",
            VariableType::Execution,
        );

        return node;
    }

    async fn run(&self, context: &mut ExecutionContext) ->flow_like_types::Result<()> {
        context.activate_exec_pin("failed").await?;
        context.deactivate_exec_pin("exec_out").await?;

        let node_scope: bool = context.evaluate_pin("node_scope").await?;

        let path = FlowPath::from_user_dir(context, node_scope).await?;
        context.set_pin_value("path", json!(path)).await?;

        context.activate_exec_pin("exec_out").await?;
        context.deactivate_exec_pin("failed").await?;
        Ok(())
    }
}
