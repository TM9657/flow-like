use std::{collections::{HashMap, HashSet}, sync::Arc, time::Duration};
use flow_like::{flow::{board::Board, execution::{context::ExecutionContext, internal_node::InternalNode, log::LogMessage, LogLevel}, node::{Node, NodeLogic}, pin::{PinOptions, PinType, ValueType}, variable::{Variable, VariableType}}, state::FlowLikeState};
use flow_like_types::{async_trait, json::{json, Deserialize, Serialize}, reqwest, sync::{DashMap, Mutex}, Bytes, Error, JsonSchema, Value};
use nalgebra::DVector;
use regex::Regex;
use flow_like_storage::{object_store::PutPayload, Path};
use futures::StreamExt;
use crate::{storage::path::FlowPath, web::api::{HttpBody, HttpRequest, HttpResponse, Method}};

#[derive(Default)]
pub struct ParentNode {}

impl ParentNode {
    pub fn new() -> Self {
        ParentNode {}
    }
}

#[async_trait]
impl NodeLogic for ParentNode {
    async fn get_node(&self, _app_state: &FlowLikeState) -> Node {
        let mut node = Node::new(
            "parent",
            "Parent",
            "Gets the parent path from a path",
            "Storage/Paths/Path",
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
            "exec_out",
            "Output",
            "Done with the Execution",
            VariableType::Execution,
        );

        node.add_output_pin(
            "parent_path",
            "Parent Path",
            "Parent FlowPath",
            VariableType::Struct,
        )
        .set_schema::<FlowPath>();

        node.add_output_pin(
            "failed",
            "Failed",
            "Failed to get Parent. Not available on some systems.",
            VariableType::Execution,
        );

        return node;
    }

    async fn run(&self, context: &mut ExecutionContext) ->flow_like_types::Result<()> {
        context.deactivate_exec_pin("exec_out").await?;
        context.activate_exec_pin("failed").await?;
        let path: FlowPath = context.evaluate_pin("path").await?;

        let mut path = path.to_runtime(context).await?;
        let mut parts = path.path.parts().collect::<Vec<_>>();
        parts.pop();
        let mut new_path = Path::from("");
        parts.iter().for_each(|part| {
            new_path = new_path.child(part.as_ref());
        });
        path.path = new_path;
        let path = path.serialize().await;

        context.set_pin_value("parent_path", json!(path)).await?;
        context.deactivate_exec_pin("failed").await?;
        context.activate_exec_pin("exec_out").await?;
        Ok(())
    }
}
