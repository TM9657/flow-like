use std::{collections::{HashMap, HashSet}, sync::Arc};
use flow_like::{flow::{board::Board, execution::{context::ExecutionContext, internal_node::InternalNode, log::LogMessage, LogLevel}, node::{Node, NodeLogic}, pin::{PinOptions, PinType, ValueType}, variable::{Variable, VariableType}}, state::FlowLikeState};
use flow_like_types::{async_trait, json::{json, Serialize, Deserialize}, JsonSchema, reqwest, sync::{DashMap, Mutex}, Value};
use nalgebra::DVector;
use regex::Regex;
use flow_like_storage::{object_store::PutPayload, Path};
use futures::StreamExt;
use crate::{storage::path::FlowPath, web::api::{HttpBody, HttpRequest, HttpResponse, Method}};

#[derive(Default)]
pub struct FilenameNode {}

impl FilenameNode {
    pub fn new() -> Self {
        FilenameNode {}
    }
}

#[async_trait]
impl NodeLogic for FilenameNode {
    async fn get_node(&self, _app_state: &FlowLikeState) -> Node {
        let mut node = Node::new(
            "filename",
            "Filename",
            "Gets the filename from a path",
            "Storage/Paths/Path",
        );
        node.add_icon("/flow/icons/path.svg");

        node.add_input_pin("path", "Path", "FlowPath", VariableType::Struct)
            .set_schema::<FlowPath>()
            .set_options(PinOptions::new().set_enforce_schema(true).build());

        node.add_output_pin("filename", "Filename", "Filename", VariableType::String);

        return node;
    }

    async fn run(&self, context: &mut ExecutionContext) ->flow_like_types::Result<()> {
        let path: FlowPath = context.evaluate_pin("path").await?;

        let path = path.to_runtime(context).await?;
        let filename = path.path.filename().unwrap_or_default().to_string();

        context.set_pin_value("filename", json!(filename)).await?;
        Ok(())
    }
}
