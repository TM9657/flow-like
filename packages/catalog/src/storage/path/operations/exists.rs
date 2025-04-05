use std::{collections::{HashMap, HashSet}, sync::Arc};
use flow_like::{flow::{board::Board, execution::{context::ExecutionContext, internal_node::InternalNode, log::LogMessage, LogLevel}, node::{Node, NodeLogic}, pin::{PinOptions, PinType, ValueType}, variable::{Variable, VariableType}}, state::FlowLikeState};
use flow_like_types::{async_trait, json::{json, Serialize, Deserialize}, JsonSchema, reqwest, sync::{DashMap, Mutex}, Value};
use nalgebra::DVector;
use regex::Regex;
use flow_like_storage::{object_store::PutPayload, Path};
use futures::StreamExt;
use crate::{storage::path::FlowPath, web::api::{HttpBody, HttpRequest, HttpResponse, Method}};

#[derive(Default)]
pub struct PathExistsNode {}

impl PathExistsNode {
    pub fn new() -> Self {
        PathExistsNode {}
    }
}

#[async_trait]
impl NodeLogic for PathExistsNode {
    async fn get_node(&self, _app_state: &FlowLikeState) -> Node {
        let mut node = Node::new(
            "path_exists",
            "Path Exists?",
            "Checks if a path exists",
            "Storage/Paths/Operations",
        );
        node.add_icon("/flow/icons/path.svg");

        node.add_input_pin(
            "exec_in",
            "Input",
            "Initiate Execution",
            VariableType::Execution,
        );

        node.add_input_pin("path", "Path", "FlowPath", VariableType::Struct)
            .set_schema::<FlowPath>()
            .set_options(PinOptions::new().set_enforce_schema(true).build());

        node.add_output_pin(
            "exec_out_exists",
            "Yes",
            "Execution if path exists",
            VariableType::Execution,
        );

        node.add_output_pin(
            "exec_out_missing",
            "No",
            "Execution if path does not exist",
            VariableType::Execution,
        );

        return node;
    }

    async fn run(&self, context: &mut ExecutionContext) ->flow_like_types::Result<()> {
        let path: FlowPath = context.evaluate_pin("path").await?;

        let dynamic = path.to_runtime(context).await?;
        let store = dynamic.store.as_generic();

        let exists = store.head(&dynamic.path).await.is_ok();

        if exists {
            context.activate_exec_pin("exec_out_exists").await?;
        } else {
            context.activate_exec_pin("exec_out_missing").await?;
        }

        Ok(())
    }
}
