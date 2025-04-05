use crate::storage::path::FlowPath;
use flow_like::{
    flow::{
        execution::{LogLevel, context::ExecutionContext},
        node::{Node, NodeLogic},
        pin::{PinOptions, ValueType},
        variable::VariableType,
    },
    state::FlowLikeState,
};
use flow_like_types::{async_trait, json::json};
use std::time::Duration;

#[derive(Default)]
pub struct SignUrlsNode {}

impl SignUrlsNode {
    pub fn new() -> Self {
        SignUrlsNode {}
    }
}

#[async_trait]
impl NodeLogic for SignUrlsNode {
    async fn get_node(&self, _app_state: &FlowLikeState) -> Node {
        let mut node = Node::new(
            "sign_urls",
            "Sign URLs",
            "Generates signed URLs for accessing files",
            "Storage/Paths/Operations",
        );
        node.add_icon("/flow/icons/path.svg");

        node.add_input_pin(
            "exec_in",
            "Input",
            "Initiate Execution",
            VariableType::Execution,
        );

        node.add_input_pin("paths", "Paths", "Array of FlowPaths", VariableType::Struct)
            .set_schema::<FlowPath>()
            .set_value_type(ValueType::Array)
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
            "Expiration time in seconds for the signed URLs",
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
            "signed_urls",
            "Signed URLs",
            "The generated array of signed URLs",
            VariableType::String,
        )
        .set_value_type(ValueType::Array);

        node.add_output_pin(
            "failed",
            "Failed",
            "Triggered if the signing process fails",
            VariableType::Execution,
        );

        return node;
    }

    async fn run(&self, context: &mut ExecutionContext) -> flow_like_types::Result<()> {
        context.activate_exec_pin("failed").await?;
        context.deactivate_exec_pin("exec_out").await?;

        let paths: Vec<FlowPath> = context.evaluate_pin("paths").await?;
        let method: String = context.evaluate_pin("method").await?;
        let expiration: i64 = context.evaluate_pin("expiration").await?;

        let mut signed_urls = Vec::new();

        for path in paths {
            let runtime_path = path.to_runtime(context).await?;
            let signed_url = runtime_path
                .store
                .sign(
                    &method,
                    &runtime_path.path,
                    Duration::from_secs(expiration as u64),
                )
                .await;

            match signed_url {
                Ok(url) => {
                    signed_urls.push(url.to_string());
                }
                Err(e) => {
                    context.log_message(
                        &format!("Failed to generate signed URL: {}", e),
                        LogLevel::Error,
                    );
                }
            }
        }

        context
            .set_pin_value("signed_urls", json!(signed_urls))
            .await?;
        context.deactivate_exec_pin("failed").await?;
        context.activate_exec_pin("exec_out").await?;

        Ok(())
    }
}
