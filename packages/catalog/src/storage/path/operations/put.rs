use std::{collections::{HashMap, HashSet}, sync::Arc};
use flow_like::{flow::{board::Board, execution::{context::ExecutionContext, internal_node::InternalNode, log::LogMessage, LogLevel}, node::{Node, NodeLogic}, pin::{PinOptions, PinType, ValueType}, variable::{Variable, VariableType}}, state::FlowLikeState};
use flow_like_types::{async_trait, json::{json, Deserialize, Serialize}, reqwest, sync::{DashMap, Mutex}, Bytes, Error, JsonSchema, Value};
use nalgebra::DVector;
use regex::Regex;
use flow_like_storage::{object_store::PutPayload, Path};
use futures::StreamExt;
use crate::{storage::path::FlowPath, web::api::{HttpBody, HttpRequest, HttpResponse, Method}};

#[derive(Default)]
pub struct PutNode {}

impl PutNode {
    pub fn new() -> Self {
        PutNode {}
    }
}

#[async_trait]
impl NodeLogic for PutNode {
    async fn get_node(&self, _app_state: &FlowLikeState) -> Node {
        let mut node = Node::new(
            "path_put",
            "Put",
            "Writes bytes to a file",
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

        node.add_input_pin("bytes", "Bytes", "Bytes to write", VariableType::Byte)
            .set_value_type(ValueType::Array);

        node.add_output_pin(
            "exec_out",
            "Output",
            "Done with the Execution",
            VariableType::Execution,
        );

        node.add_output_pin(
            "failed",
            "Failed",
            "Failed to write to the file",
            VariableType::Execution,
        );

        return node;
    }

    async fn run(&self, context: &mut ExecutionContext) ->flow_like_types::Result<()> {
        context.activate_exec_pin("failed").await?;
        context.deactivate_exec_pin("exec_out").await?;
        let path: FlowPath = context.evaluate_pin("path").await?;
        let bytes: Vec<u8> = context.evaluate_pin("bytes").await?;

        let path = path.to_runtime(context).await?;
        let store = path.store.as_generic();
        let bytes = Bytes::from(bytes);
        let payload = PutPayload::from_bytes(bytes);
        store.put(&path.path, payload).await?;

        context.deactivate_exec_pin("failed").await?;
        context.activate_exec_pin("exec_out").await?;
        Ok(())
    }
}
