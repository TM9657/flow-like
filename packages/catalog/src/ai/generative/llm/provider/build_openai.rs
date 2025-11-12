use std::{collections::HashMap, sync::Arc};

use flow_like::{
    bit::{Bit, BitModelClassification, VLMParameters},
    flow::{
        board::Board,
        execution::context::ExecutionContext,
        node::{Node, NodeLogic},
        pin::PinOptions,
        variable::VariableType,
    },
    state::FlowLikeState,
};
use flow_like_storage::blake3;
use flow_like_types::{
    Value, async_trait,
    json::{json, to_value},
};

#[derive(Default)]
pub struct BuildOpenAiNode {}

impl BuildOpenAiNode {
    pub fn new() -> Self {
        BuildOpenAiNode {}
    }
}

#[async_trait]
impl NodeLogic for BuildOpenAiNode {
    async fn get_node(&self, _app_state: &FlowLikeState) -> Node {
        let mut node = Node::new(
            "ai_generative_build_openai",
            "OpenAI Model",
            "Builds the OpenAI model based on certain selection criteria",
            "AI/Generative/Provider",
        );
        node.add_icon("/flow/icons/find_model.svg");

        node.add_input_pin("exec_in", "Input", "Trigger Pin", VariableType::Execution);
        node.add_input_pin(
            "provider",
            "Provider",
            "Provider Name",
            VariableType::String,
        )
        .set_options(
            PinOptions::new()
                .set_valid_values(vec!["OpenAI".into(), "Azure".into()])
                .build(),
        )
        .set_default_value(Some(json!("OpenAI")));

        node.add_input_pin("endpoint", "Endpoint", "API Endpoint", VariableType::String)
            .set_default_value(Some(json!("https://api.openai.com/v1/")));

        node.add_input_pin("api_key", "API Key", "API Key", VariableType::String)
            .set_default_value(Some(json!("")))
            .set_options(PinOptions::new().set_sensitive(true).build());

        node.add_output_pin("exec_out", "Output", "Done", VariableType::Execution);
        node.add_output_pin("model", "Model", "The selected model", VariableType::Struct)
            .set_schema::<Bit>();

        node.set_long_running(true);

        return node;
    }

    async fn run(&self, context: &mut ExecutionContext) -> flow_like_types::Result<()> {
        context.deactivate_exec_pin("exec_out").await?;

        let mut hasher = blake3::Hasher::new();

        let provider: String = context.evaluate_pin("provider").await?;

        hasher.update(provider.as_bytes());

        let api_key = context.evaluate_pin::<String>("api_key").await?;
        let endpoint = context.evaluate_pin::<String>("endpoint").await?;

        let mut params = HashMap::new();
        params.insert("api_key".to_string(), json!(api_key));
        hasher.update(api_key.as_bytes());
        params.insert("endpoint".to_string(), json!(endpoint));
        hasher.update(endpoint.as_bytes());

        if provider.to_lowercase() == "azure" {
            params.insert("is_azure".to_string(), json!(true));
            hasher.update(b"azure");
        }

        if let Ok(model_id) = context.evaluate_pin::<String>("model_id").await
            && !model_id.is_empty()
        {
            params.insert("model_id".to_string(), json!(model_id));
            hasher.update(model_id.as_bytes());
        }

        if let Ok(version) = context.evaluate_pin::<String>("version").await
            && !version.is_empty()
        {
            params.insert("version".to_string(), json!(version));
            hasher.update(version.as_bytes());
        }

        let bit_hash = hasher.finalize().to_hex().to_string();

        let params = VLMParameters {
            context_length: 20000,
            model_classification: BitModelClassification::default(),
            provider: flow_like_model_provider::provider::ModelProvider {
                provider_name: "custom:openai".into(),
                model_id: Some(
                    context
                        .evaluate_pin::<String>("model_id")
                        .await
                        .unwrap_or("gpt-5".into()),
                ),
                version: Some(
                    context
                        .evaluate_pin::<String>("version")
                        .await
                        .unwrap_or("v1".into()),
                ),
                params: Some(params),
            },
        };
        let params = to_value(&params).unwrap_or_default();

        let mut bit = Bit::default();
        bit.id = bit_hash;
        bit.bit_type = flow_like::bit::BitTypes::Vlm;
        bit.parameters = params;

        context
            .set_pin_value("model", flow_like_types::json::json!(bit))
            .await?;

        context.activate_exec_pin("exec_out").await?;

        return Ok(());
    }

    async fn on_update(&self, node: &mut Node, _board: Arc<Board>) {
        let provider_pin: String = node
            .get_pin_by_name("provider")
            .and_then(|pin| pin.default_value.clone())
            .and_then(|bytes| flow_like_types::json::from_slice::<Value>(&bytes).ok())
            .and_then(|json| json.as_str().map(ToOwned::to_owned))
            .unwrap_or_default();

        let model_id_pin = node.get_pin_by_name("model_id");
        let version_pin = node.get_pin_by_name("version");

        match (
            provider_pin.as_str(),
            model_id_pin.cloned(),
            version_pin.cloned(),
        ) {
            ("OpenAI", Some(model_pin), Some(version_pin)) => {
                node.pins.remove(&model_pin.id);
                node.pins.remove(&version_pin.id);
            }
            ("OpenAI", None, Some(version_pin)) => {
                node.pins.remove(&version_pin.id);
            }
            ("OpenAI", Some(model_pin), None) => {
                node.pins.remove(&model_pin.id);
            }
            ("Azure", None, None) => {
                node.add_input_pin(
                    "model_id",
                    "Model ID",
                    "Azure Model ID",
                    VariableType::String,
                )
                .set_default_value(Some(json!("")));
                node.add_input_pin(
                    "version",
                    "Version",
                    "Azure API Version",
                    VariableType::String,
                )
                .set_default_value(Some(json!("2024-12-01-preview")));
            }
            ("Azure", Some(_), None) => {
                node.add_input_pin(
                    "version",
                    "Version",
                    "Azure API Version",
                    VariableType::String,
                )
                .set_default_value(Some(json!("2024-12-01-preview")));
            }
            ("Azure", None, Some(_)) => {
                node.add_input_pin(
                    "model_id",
                    "Model ID",
                    "Azure Model ID",
                    VariableType::String,
                )
                .set_default_value(Some(json!("")));
            }
            ("Azure", Some(_), Some(_)) => {}
            _ => {}
        }
    }
}
