use std::{collections::{HashMap, HashSet}, sync::Arc, time::Duration};
use flow_like::{flow::{board::Board, execution::{context::ExecutionContext, internal_node::InternalNode, log::LogMessage, LogLevel}, node::{Node, NodeLogic}, pin::{PinOptions, PinType, ValueType}, variable::{Variable, VariableType}}, state::FlowLikeState};
use flow_like_types::{async_trait, json::{json, Deserialize, Serialize}, reqwest, sync::{DashMap, Mutex}, Bytes, Error, JsonSchema, Value};
use nalgebra::DVector;
use regex::Regex;
use flow_like_storage::{object_store::PutPayload, Path};
use futures::StreamExt;
use crate::{storage::path::FlowPath, web::api::{HttpBody, HttpRequest, HttpResponse, Method}};

#[derive(Default)]
pub struct SignUrlNode {}

impl SignUrlNode {
    pub fn new() -> Self {
        SignUrlNode {}
    }
}

#[async_trait]
impl NodeLogic for SignUrlNode {
    async fn get_node(&self, _app_state: &FlowLikeState) -> Node {
        let mut node = Node::new(
            "sign_url",
            "Sign URL",
            "Generates a signed URL for accessing a file",
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

        node.add_input_pin(
            "method",
            "Method",
            "HTTP Method (GET, PUT, etc.)",
            VariableType::String,
        )
        .set_options(
            PinOptions::new()
                .set_valid_values(vec![
                    "GET".to_string(),
                    "PUT".to_string(),
                    "POST".to_string(),
                    "DELETE".to_string(),
                    "HEAD".to_string(),
                ])
                .build(),
        )
        .set_default_value(Some(json!("GET")));

        node.add_input_pin(
            "expiration",
            "Expiration (seconds)",
            "Expiration time in seconds for the signed URL",
            VariableType::Integer,
        )
        .set_default_value(Some(json!(3600))); // Default expiration of 1 hour

        node.add_output_pin(
            "exec_out",
            "Output",
            "Done with the Execution",
            VariableType::Execution,
        );

        node.add_output_pin(
            "signed_url",
            "Signed URL",
            "The generated signed URL",
            VariableType::String,
        );

        node.add_output_pin(
            "failed",
            "Failed",
            "Triggered if the signing process fails",
            VariableType::Execution,
        );

        return node;
    }

    async fn run(&self, context: &mut ExecutionContext) ->flow_like_types::Result<()> {
        context.activate_exec_pin("failed").await?;
        context.deactivate_exec_pin("exec_out").await?;

        let path: FlowPath = context.evaluate_pin("path").await?;
        let method: String = context.evaluate_pin("method").await?;
        let expiration: i64 = context.evaluate_pin("expiration").await?;

        let path = path.to_runtime(context).await?;

        let signed_url = path
            .store
            .sign(&method, &path.path, Duration::from_secs(expiration as u64))
            .await;

        match signed_url {
            Ok(url) => {
                context
                    .set_pin_value("signed_url", json!(url.to_string()))
                    .await?;
                context.deactivate_exec_pin("failed").await?;
                context.activate_exec_pin("exec_out").await?;
            }
            Err(e) => {
                context.log_message(
                    &format!("Failed to generate signed URL: {}", e),
                    LogLevel::Error,
                );
            }
        }

        Ok(())
    }
}
