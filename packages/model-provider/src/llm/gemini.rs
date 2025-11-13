use std::{any::Any, sync::Arc};

use super::{LLMCallback, ModelLogic};
use crate::provider::random_provider;
use crate::{
    history::History,
    llm::ModelConstructor,
    provider::{ModelProvider, ModelProviderConfiguration},
    response::Response,
};
use flow_like_types::json::to_value;
use flow_like_types::{Cacheable, Result, anyhow, async_trait};
use rig::client::ProviderClient;
use rig::completion::CompletionModel;
use rig::message::DocumentSourceKind;
use rig::providers::gemini::completion::gemini_api_types::{
    AdditionalParameters, GenerationConfig, ThinkingConfig,
};
use rig::{OneOrMany, completion::Message as RigMessage};
pub struct GeminiModel {
    client: Arc<Box<dyn ProviderClient>>,
    provider: ModelProvider,
    default_model: Option<String>,
}

impl GeminiModel {
    pub async fn new(
        provider: &ModelProvider,
        config: &ModelProviderConfiguration,
    ) -> flow_like_types::Result<Self> {
        let gemini_config = random_provider(&config.gemini_config)?;
        let api_key = gemini_config.api_key.clone().unwrap_or_default();
        let model_id = provider.model_id.clone();

        let mut builder = rig::providers::gemini::Client::builder(&api_key);
        if let Some(endpoint) = gemini_config.endpoint.as_deref() {
            builder = builder.base_url(endpoint);
        }

        let client = builder.build()?.boxed();

        Ok(GeminiModel {
            client: Arc::new(client),
            provider: provider.clone(),
            default_model: model_id,
        })
    }

    pub async fn from_provider(provider: &ModelProvider) -> flow_like_types::Result<Self> {
        let params = provider.params.clone().unwrap_or_default();
        let api_key = params.get("api_key").cloned().unwrap_or_default();
        let api_key = api_key.as_str().unwrap_or_default();
        let model_id = params
            .get("model_id")
            .cloned()
            .and_then(|v| v.as_str().map(|s| s.to_string()));

        let mut builder = rig::providers::gemini::Client::builder(api_key);
        if let Some(endpoint) = params.get("endpoint").and_then(|v| v.as_str()) {
            builder = builder.base_url(endpoint);
        }
        let client = builder.build()?.boxed();

        Ok(GeminiModel {
            client: Arc::new(client),
            default_model: model_id,
            provider: provider.clone(),
        })
    }

    /// Transform RigMessages to convert data URLs to base64 for Gemini
    fn transform_rig_messages(&self, prompt: &mut RigMessage, history: &mut Vec<RigMessage>) {
        use rig::message::{Image as RigImage, UserContent as RigUserContent};

        // Helper to transform a message
        let transform_message = |msg: &mut RigMessage| {
            if let RigMessage::User { content } = msg {
                let transformed: Vec<RigUserContent> = content
                    .iter()
                    .map(|c| {
                        if let RigUserContent::Image(img) = c {
                            // Check if it's a data URL
                            if let DocumentSourceKind::Url(url) = &img.data
                                && url.starts_with("data:")
                            {
                                // Extract base64 data from data URL
                                if let Some(comma_pos) = url.find(',') {
                                    let base64_data = &url[comma_pos + 1..];
                                    return RigUserContent::Image(RigImage {
                                        data: DocumentSourceKind::Base64(base64_data.to_string()),
                                        media_type: img.media_type.clone(),
                                        detail: img.detail.clone(),
                                        additional_params: img.additional_params.clone(),
                                    });
                                }
                            }
                        }
                        c.clone()
                    })
                    .collect();

                *content = if transformed.len() == 1 {
                    OneOrMany::one(transformed.into_iter().next().unwrap())
                } else {
                    OneOrMany::many(transformed).unwrap_or_else(|_| {
                        OneOrMany::one(RigUserContent::Text(rig::message::Text {
                            text: String::new(),
                        }))
                    })
                };
            }
        };

        // Transform prompt
        transform_message(prompt);

        // Transform history messages
        for msg in history.iter_mut() {
            transform_message(msg);
        }
    }
}

impl Cacheable for GeminiModel {
    fn as_any(&self) -> &dyn Any {
        self
    }

    fn as_any_mut(&mut self) -> &mut dyn Any {
        self
    }
}

#[async_trait]
impl ModelLogic for GeminiModel {
    async fn provider(&self) -> Result<ModelConstructor> {
        Ok(ModelConstructor {
            inner: self.client.clone(),
        })
    }

    async fn default_model(&self) -> Option<String> {
        self.default_model.clone()
    }

    fn transform_history(&self, _history: &mut History) {
        // Not used - we override invoke() to transform RigMessages instead
    }

    async fn invoke(&self, history: &History, lambda: Option<LLMCallback>) -> Result<Response> {
        use crate::llm::{CompletionModelHandle, invoke_with_stream, invoke_without_stream};

        let model_name = self
            .default_model()
            .await
            .unwrap_or_else(|| history.model.clone());

        let constructor = self.provider().await?;
        let client = constructor.client();
        let completion_client = client
            .as_ref()
            .as_completion()
            .ok_or_else(|| anyhow!("Provider does not support completion"))?;

        let completion_model = completion_client.completion_model(&model_name);
        let completion_handle = CompletionModelHandle {
            inner: Arc::from(completion_model),
        };

        let (mut prompt, mut chat_history) = history
            .extract_prompt_and_history()
            .map_err(|e| anyhow!("Failed to convert history into rig messages: {e}"))?;

        // GEMINI-SPECIFIC: Transform data URLs to Base64
        self.transform_rig_messages(&mut prompt, &mut chat_history);

        let mut builder = completion_handle
            .completion_request(prompt)
            .messages(chat_history);

        if let Some(temp) = history.temperature {
            builder = builder.temperature(temp as f64);
        }

        if let Some(max_tokens) = history.max_completion_tokens {
            builder = builder.max_tokens(max_tokens as u64);
        }

        if history.tools.is_some() {
            let tool_definitions = history.tools_to_rig()?;
            if !tool_definitions.is_empty() {
                builder = builder.tools(tool_definitions);
            }
        }

        if let Some(choice) = history.tool_choice_to_rig() {
            builder = builder.tool_choice(choice);
        }

        let model_additional_params = self.additional_params(&Some(history.clone()));

        if model_additional_params.is_none()
            && let Some(params) = history.build_additional_params()?
        {
            builder = builder.additional_params(params);
        }

        if let Some(callback) = lambda {
            invoke_with_stream(builder, callback, &model_name, model_additional_params).await
        } else {
            invoke_without_stream(builder, &model_name, model_additional_params).await
        }
    }

    fn additional_params(&self, history: &Option<History>) -> Option<flow_like_types::Value> {
        // Gemini's AdditionalParameters MUST include generation_config field
        // We need to handle the 'stream' field specially: it comes from History.build_additional_params()
        // but Gemini doesn't accept 'stream' in the request body - it uses different endpoints instead

        // Get history's additional params (includes stream field)
        let history_params = history
            .as_ref()
            .and_then(|h| h.build_additional_params().ok())
            .flatten();

        let gen_cfg = GenerationConfig {
            thinking_config: Some(ThinkingConfig {
                include_thoughts: Some(true),
                thinking_budget: 2048,
            }),
            ..Default::default()
        };
        let additional_params = AdditionalParameters::default().with_config(gen_cfg);
        let mut result = to_value(additional_params).ok()?;

        // Merge history params but exclude 'stream' field
        if let (Some(result_obj), Some(history_params)) = (result.as_object_mut(), history_params)
            && let Some(history_obj) = history_params.as_object()
        {
            for (key, value) in history_obj {
                // Skip 'stream' field - Gemini doesn't support it
                if key != "stream" {
                    result_obj.insert(key.clone(), value.clone());
                }
            }
        }

        Some(result)
    }
}
