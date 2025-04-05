use std::{collections::{HashMap, HashSet}, sync::Arc};
use flow_like::{flow::{board::Board, execution::{context::ExecutionContext, internal_node::InternalNode, log::LogMessage, LogLevel}, node::{Node, NodeLogic}, pin::{PinOptions, PinType, ValueType}, variable::{Variable, VariableType}}, state::FlowLikeState};
use flow_like_types::{async_trait, json::{json, Deserialize, Serialize}, reqwest, sync::{DashMap, Mutex}, Error, JsonSchema, Value};
use nalgebra::DVector;
use regex::Regex;
use flow_like_storage::{object_store::PutPayload, Path};
use futures::StreamExt;
use crate::{storage::path::FlowPath, web::api::{HttpBody, HttpRequest, HttpResponse, Method}};

#[derive(Default)]
pub struct ListWithOffsetNode {}

impl ListWithOffsetNode {
    pub fn new() -> Self {
        ListWithOffsetNode {}
    }
}

#[async_trait]
impl NodeLogic for ListWithOffsetNode {
    async fn get_node(&self, _app_state: &FlowLikeState) -> Node {
        let mut node = Node::new(
            "path_list_with_offset",
            "List With Offset",
            "Lists paths in a directory with offset and limit",
            "Storage/Paths/Operations",
        );
        node.add_icon("/flow/icons/path.svg");

        node.add_input_pin(
            "exec_in",
            "Input",
            "Initiate Execution",
            VariableType::Execution,
        );

        node.add_input_pin("prefix", "Prefix", "FlowPath", VariableType::Struct)
            .set_schema::<FlowPath>()
            .set_options(PinOptions::new().set_enforce_schema(true).build());

        node.add_input_pin("offset", "Offset", "FlowPath", VariableType::Struct)
            .set_schema::<FlowPath>()
            .set_options(PinOptions::new().set_enforce_schema(true).build());

        node.add_input_pin(
            "offset",
            "Offset",
            "Offset to start listing from",
            VariableType::Integer,
        );

        node.add_output_pin(
            "exec_out",
            "Output",
            "Done with the Execution",
            VariableType::Execution,
        );

        node.add_output_pin(
            "failed",
            "Failed",
            "Failed to list the paths",
            VariableType::Execution,
        );

        node.add_output_pin("paths", "Paths", "Output Paths", VariableType::Struct)
            .set_schema::<FlowPath>()
            .set_value_type(ValueType::Array);

        return node;
    }

    async fn run(&self, context: &mut ExecutionContext) ->flow_like_types::Result<()> {
        context.activate_exec_pin("failed").await?;
        context.deactivate_exec_pin("exec_out").await?;
        let original_prefix: FlowPath = context.evaluate_pin("prefix").await?;
        let original_offset: FlowPath = context.evaluate_pin("offset").await?;

        let prefix = original_prefix.to_runtime(context).await?;
        let offset = original_offset.to_runtime(context).await?;
        let store = prefix.store.as_generic();

        let paths = store
            .list_with_offset(Some(&prefix.path), &offset.path)
            .map(|r| r.map_err(Error::from))
            .collect::<Vec<_>>()
            .await
            .into_iter()
            .collect::<Result<Vec<_>, _>>()?;

        let paths = paths
            .iter()
            .map(|p| {
                let mut new_path = original_prefix.clone();
                new_path.path = p.location.as_ref().to_string();
                new_path
            })
            .collect::<Vec<FlowPath>>();

        context.set_pin_value("paths", json!(paths)).await?;
        context.deactivate_exec_pin("failed").await?;
        context.activate_exec_pin("exec_out").await?;

        Ok(())
    }
}
