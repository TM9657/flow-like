use std::{any::Any, sync::Arc};

use super::{LLMCallback, ModelLogic};
use crate::provider::random_provider;
use crate::{
    embedding::{EmbeddingModelLogic, GeneralTextSplitter},
    history::History,
    llm::ModelConstructor,
    provider::{ModelProvider, ModelProviderConfiguration},
    response::Response,
};
use flow_like_types::{Cacheable, Result, async_trait, sync::Mutex};
use rig::client::ProviderClient;
use text_splitter::{ChunkConfig, MarkdownSplitter, TextSplitter};
pub struct OpenAIModel {
    client: Arc<Box<dyn ProviderClient>>,
    provider: ModelProvider,
    default_model: Option<String>,
}

impl OpenAIModel {
    pub async fn new(
        provider: &ModelProvider,
        config: &ModelProviderConfiguration,
    ) -> flow_like_types::Result<Self> {
        let openai_config = random_provider(&config.openai_config)?;
        let api_key = openai_config.api_key.clone().unwrap_or_default();
        let model_id = provider.model_id.clone();

        let client = if provider.provider_name == "azure" {
            let endpoint = openai_config.endpoint.clone().unwrap_or_default();
            let endpoint = if endpoint.ends_with('/') {
                endpoint.to_string()
            } else {
                format!("{}/", endpoint)
            };
            let endpoint = format!(
                "{}openai/deployments/{}",
                endpoint,
                model_id.clone().unwrap_or_default()
            );
            let auth = rig::providers::azure::AzureOpenAIAuth::ApiKey(api_key.clone());
            let mut builder = rig::providers::azure::Client::builder(auth, &endpoint);
            if let Some(version) = provider.version.as_deref() {
                builder = builder.api_version(version);
            }

            builder.build().boxed()
        } else {
            let mut builder = rig::providers::openai::Client::builder(&api_key);
            if let Some(endpoint) = openai_config.endpoint.as_deref() {
                builder = builder.base_url(endpoint);
            }

            builder.build().boxed()
        };

        Ok(OpenAIModel {
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

        let is_azure = params.get("is_azure").cloned();
        let endpoint = params.get("endpoint").cloned();

        let is_azure = match is_azure {
            Some(val) => val.as_bool().unwrap_or(false),
            None => false,
        };

        if is_azure && endpoint.is_none() {
            return Err(flow_like_types::anyhow!(
                "Azure OpenAI requires an endpoint"
            ));
        }

        let client = if is_azure {
            let endpoint = endpoint.unwrap_or_default();
            let endpoint = endpoint.as_str().unwrap_or_default();
            let endpoint = if endpoint.ends_with('/') {
                endpoint.to_string()
            } else {
                format!("{}/", endpoint)
            };
            let endpoint = format!(
                "{}openai/deployments/{}",
                endpoint,
                model_id.clone().unwrap_or_default()
            );
            let auth = rig::providers::azure::AzureOpenAIAuth::ApiKey(api_key.to_string());
            let mut builder = rig::providers::azure::Client::builder(auth, &endpoint);
            if let Some(version_str) = params.get("version").and_then(|v| v.as_str()) {
                builder = builder.api_version(version_str);
            }
            builder.build().boxed()
        } else {
            let mut builder = rig::providers::openai::Client::builder(api_key);
            if let Some(endpoint) = endpoint.as_ref().and_then(|v| v.as_str()) {
                builder = builder.base_url(endpoint);
            }
            builder.build().boxed()
        };

        Ok(OpenAIModel {
            client: Arc::new(client),
            default_model: model_id,
            provider: provider.clone(),
        })
    }
}

impl Cacheable for OpenAIModel {
    fn as_any(&self) -> &dyn Any {
        self
    }

    fn as_any_mut(&mut self) -> &mut dyn Any {
        self
    }
}

#[async_trait]
impl ModelLogic for OpenAIModel {
    async fn provider(&self) -> Result<ModelConstructor> {
        Ok(ModelConstructor {
            inner: self.client.clone(),
        })
    }

    async fn default_model(&self) -> Option<String> {
        self.default_model.clone()
    }
}

// #[cfg(test)]
// mod tests {
//     use std::sync::atomic::{AtomicUsize, Ordering};

//     use flow_like_types::tokio;

//     use super::*;
//     use crate::{
//         history::{
//             Content, ContentType, HistoryFunction, HistoryFunctionParameters,
//             HistoryJSONSchemaDefine, HistoryJSONSchemaType, HistoryMessage, ImageUrl,
//             MessageContent, Role, Tool, ToolChoice, ToolType,
//         },
//         provider::{ModelProviderConfiguration, OpenAIConfig},
//     };
//     use dotenv::dotenv;
//     use std::collections::HashMap;

//     #[tokio::test]
//     async fn test_openai_model_no_stream() {
//         dotenv().ok();

//         let provider = ModelProvider {
//             model_id: Some("@preset/prod-free".to_string()),
//             version: None,
//             provider_name: "openai".to_string(),
//             params: None,
//         };
//         let endpoint = std::env::var("OPENAI_ENDPOINT").unwrap();
//         let api_key = std::env::var("OPENAI_API_KEY").unwrap();
//         let config = ModelProviderConfiguration {
//             openai_config: vec![OpenAIConfig {
//                 api_key: Some(api_key),
//                 organization: None,
//                 endpoint: Some(endpoint),
//                 proxy: None,
//             }],
//             bedrock_config: vec![],
//         };

//         let model = OpenAIModel::new(&provider, &config).await.unwrap();
//         let mut history = History::new(
//             "@preset/prod-free".to_string(),
//             vec![
//                 HistoryMessage::from_string(Role::System, "You are a helpful assistant."),
//                 HistoryMessage::from_string(Role::User, "Hello"),
//             ],
//         );
//         history.set_stream(false);
//         let response = model.invoke(&history, None).await.unwrap();
//         assert!(!response.choices.is_empty());
//     }

//     #[tokio::test]
//     async fn test_azure_openai_model_no_stream() {
//         dotenv().ok();

//         let provider = ModelProvider {
//             model_id: Some("gpt-4o-mini".to_string()),
//             version: Some("2024-02-15-preview".to_string()),
//             provider_name: "azure".to_string(),
//             params: None,
//         };
//         let api_key = std::env::var("AZURE_OPENAI_API_KEY").unwrap();
//         let endpoint = std::env::var("AZURE_OPENAI_ENDPOINT").unwrap();
//         let config = ModelProviderConfiguration {
//             openai_config: vec![OpenAIConfig {
//                 api_key: Some(api_key),
//                 organization: None,
//                 endpoint: Some(endpoint),
//                 proxy: None,
//             }],
//             bedrock_config: vec![],
//         };

//         let model = OpenAIModel::new(&provider, &config).await.unwrap();
//         let mut history = History::new(
//             "gpt-4o-mini".to_string(),
//             vec![
//                 HistoryMessage::from_string(Role::System, "You are a helpful assistant."),
//                 HistoryMessage::from_string(Role::User, "Hello"),
//             ],
//         );
//         history.set_stream(false);
//         let response = model.invoke(&history, None).await.unwrap();
//         println!("Final response: {:?}", response.last_message());
//         assert!(!response.choices.is_empty());
//     }

//     #[tokio::test]
//     async fn test_openai_model_stream() {
//         dotenv().ok();

//         let provider = ModelProvider {
//             model_id: Some("@preset/prod-free".to_string()),
//             version: None,
//             provider_name: "openai".to_string(),
//             params: None,
//         };
//         let endpoint = std::env::var("OPENAI_ENDPOINT").unwrap();
//         let api_key = std::env::var("OPENAI_API_KEY").unwrap();
//         let config = ModelProviderConfiguration {
//             openai_config: vec![OpenAIConfig {
//                 api_key: Some(api_key),
//                 organization: None,
//                 endpoint: Some(endpoint),
//                 proxy: None,
//             }],
//             bedrock_config: vec![],
//         };

//         let model = OpenAIModel::new(&provider, &config).await.unwrap();
//         let mut history = History::new(
//             "@preset/prod-free".to_string(),
//             vec![
//                 HistoryMessage::from_string(Role::System, "You are a helpful assistant."),
//                 HistoryMessage::from_string(Role::User, "Hello"),
//             ],
//         );
//         history.set_stream(true);

//         let counter = Arc::new(AtomicUsize::new(0));
//         let counter_clone = counter.clone();
//         let callback: LLMCallback = Arc::new(move |_response| {
//             counter_clone.fetch_add(1, Ordering::SeqCst);
//             Box::pin(async move { Ok(()) })
//         });

//         let response = model.invoke(&history, Some(callback)).await.unwrap();
//         println!("Final response: {:?}", response.last_message());
//         println!("Chunks: {}", counter.load(Ordering::SeqCst));
//         assert!(!response.choices.is_empty());
//     }

//     #[tokio::test]
//     async fn test_azure_openai_model_stream() {
//         dotenv().ok();

//         let provider = ModelProvider {
//             model_id: Some("gpt-4o-mini".to_string()),
//             version: Some("2024-02-15-preview".to_string()),
//             provider_name: "azure".to_string(),
//             params: None,
//         };
//         let api_key = std::env::var("AZURE_OPENAI_API_KEY").unwrap();
//         let endpoint = std::env::var("AZURE_OPENAI_ENDPOINT").unwrap();
//         let config = ModelProviderConfiguration {
//             openai_config: vec![OpenAIConfig {
//                 api_key: Some(api_key),
//                 organization: None,
//                 endpoint: Some(endpoint),
//                 proxy: None,
//             }],
//             bedrock_config: vec![],
//         };

//         let model = OpenAIModel::new(&provider, &config).await.unwrap();
//         let mut history = History::new(
//             "gpt-4o-mini".to_string(),
//             vec![
//                 HistoryMessage::from_string(Role::System, "You are a helpful assistant."),
//                 HistoryMessage::from_string(Role::User, "Hello"),
//             ],
//         );
//         history.set_stream(true);

//         let counter = Arc::new(AtomicUsize::new(0));
//         let counter_clone = counter.clone();
//         let callback: LLMCallback = Arc::new(move |_response| {
//             counter_clone.fetch_add(1, Ordering::SeqCst);
//             Box::pin(async move { Ok(()) })
//         });

//         let response = model.invoke(&history, Some(callback)).await.unwrap();
//         println!("Final response: {:?}", response.last_message());
//         println!("Chunks: {}", counter.load(Ordering::SeqCst));
//         assert!(!response.choices.is_empty());
//     }

//     // --- Helpers for new tests ---
//     fn azure_provider_and_config() -> (ModelProvider, ModelProviderConfiguration) {
//         dotenv().ok();
//         let provider = ModelProvider {
//             model_id: Some("gpt-4o-mini".to_string()),
//             version: Some("2024-02-15-preview".to_string()),
//             provider_name: "azure".to_string(),
//             params: None,
//         };
//         let api_key = std::env::var("AZURE_OPENAI_API_KEY").unwrap();
//         let endpoint = std::env::var("AZURE_OPENAI_ENDPOINT").unwrap();
//         let config = ModelProviderConfiguration {
//             openai_config: vec![OpenAIConfig {
//                 api_key: Some(api_key),
//                 organization: None,
//                 endpoint: Some(endpoint),
//                 proxy: None,
//             }],
//             bedrock_config: vec![],
//         };
//         (provider, config)
//     }

//     fn build_weather_tool() -> (Tool, HistoryFunction) {
//         let mut props: HashMap<String, Box<HistoryJSONSchemaDefine>> = HashMap::new();
//         props.insert(
//             "location".to_string(),
//             Box::new(HistoryJSONSchemaDefine {
//                 schema_type: Some(HistoryJSONSchemaType::String),
//                 description: Some("City and state, e.g. San Francisco, CA".to_string()),
//                 enum_values: None,
//                 properties: None,
//                 required: None,
//                 items: None,
//             }),
//         );
//         props.insert(
//             "unit".to_string(),
//             Box::new(HistoryJSONSchemaDefine {
//                 schema_type: Some(HistoryJSONSchemaType::String),
//                 description: Some("Temperature unit".to_string()),
//                 enum_values: Some(vec!["celsius".to_string(), "fahrenheit".to_string()]),
//                 properties: None,
//                 required: None,
//                 items: None,
//             }),
//         );

//         let params = HistoryFunctionParameters {
//             schema_type: HistoryJSONSchemaType::Object,
//             properties: Some(props),
//             required: Some(vec!["location".to_string(), "unit".to_string()]),
//         };
//         let func = HistoryFunction {
//             name: "get_current_weather".to_string(),
//             description: Some("Get the current weather in a given location".to_string()),
//             parameters: params,
//         };
//         let tool = Tool {
//             tool_type: ToolType::Function,
//             function: func.clone(),
//         };
//         (tool, func)
//     }

//     fn build_forecast_tool() -> (Tool, HistoryFunction) {
//         let mut props: HashMap<String, Box<HistoryJSONSchemaDefine>> = HashMap::new();
//         props.insert(
//             "location".to_string(),
//             Box::new(HistoryJSONSchemaDefine {
//                 schema_type: Some(HistoryJSONSchemaType::String),
//                 description: Some("City and state, e.g. Berlin, DE".to_string()),
//                 enum_values: None,
//                 properties: None,
//                 required: None,
//                 items: None,
//             }),
//         );
//         props.insert(
//             "days".to_string(),
//             Box::new(HistoryJSONSchemaDefine {
//                 schema_type: Some(HistoryJSONSchemaType::Number),
//                 description: Some("Number of days to forecast".to_string()),
//                 enum_values: None,
//                 properties: None,
//                 required: None,
//                 items: None,
//             }),
//         );

//         let params = HistoryFunctionParameters {
//             schema_type: HistoryJSONSchemaType::Object,
//             properties: Some(props),
//             required: Some(vec!["location".to_string(), "days".to_string()]),
//         };
//         let func = HistoryFunction {
//             name: "get_forecast".to_string(),
//             description: Some("Get the forecast for the next N days".to_string()),
//             parameters: params,
//         };
//         let tool = Tool {
//             tool_type: ToolType::Function,
//             function: func.clone(),
//         };
//         (tool, func)
//     }

//     fn new_counter_callback() -> (LLMCallback, Arc<AtomicUsize>) {
//         let counter = Arc::new(AtomicUsize::new(0));
//         let counter_clone = counter.clone();
//         let callback: LLMCallback = Arc::new(move |_response| {
//             counter_clone.fetch_add(1, Ordering::SeqCst);
//             Box::pin(async move { Ok(()) })
//         });
//         (callback, counter)
//     }

//     #[tokio::test]
//     async fn test_azure_openai_tool_call_no_stream() {
//         let (provider, config) = azure_provider_and_config();
//         let model = OpenAIModel::new(&provider, &config).await.unwrap();

//         let (tool, func) = build_weather_tool();

//         let mut history = History::new(
//             "gpt-4o-mini".to_string(),
//             vec![
//                 HistoryMessage::from_string(Role::System, "You are a helpful assistant."),
//                 HistoryMessage {
//                     role: Role::User,
//                     content: MessageContent::Contents(vec![Content::Text {
//                         content_type: ContentType::Text,
//                         text: "Call the tool to get the weather for San Francisco, CA in celsius. Return a tool call only.".to_string(),
//                     }]),
//                     name: None,
//                     tool_calls: None,
//                     tool_call_id: None,
//                     annotations: None,
//                 },
//             ],
//         );
//         history.tools = Some(vec![tool]);
//         history.tool_choice = Some(ToolChoice::Specific {
//             r#type: ToolType::Function,
//             function: func.clone(),
//         });
//         history.temperature = Some(0.0);
//         history.set_stream(false);

//         let response = model.invoke(&history, None).await.unwrap();
//         let msg = response.last_message().expect("no last message");
//         assert!(!msg.tool_calls.is_empty());
//         let call = &msg.tool_calls[0];
//         assert_eq!(call.function.name, "get_current_weather");
//         let args = &call.function.arguments;
//         assert!(args.contains("San Francisco") || args.contains("san francisco"));
//         assert!(args.contains("celsius"));
//     }

//     #[tokio::test]
//     async fn test_azure_openai_tool_call_stream() {
//         let (provider, config) = azure_provider_and_config();
//         let model = OpenAIModel::new(&provider, &config).await.unwrap();
//         let (tool, func) = build_weather_tool();

//         let mut history = History::new(
//             "gpt-4o-mini".to_string(),
//             vec![
//                 HistoryMessage::from_string(Role::System, "You are a helpful assistant."),
//                 HistoryMessage::from_string(
//                     Role::User,
//                     "Please call the tool to get the weather for Berlin in celsius.",
//                 ),
//             ],
//         );
//         history.tools = Some(vec![tool]);
//         history.tool_choice = Some(ToolChoice::Specific {
//             r#type: ToolType::Function,
//             function: func,
//         });
//         history.set_stream(true);

//         let (callback, counter) = new_counter_callback();
//         let response = model.invoke(&history, Some(callback)).await.unwrap();
//         let msg = response.last_message().expect("no last message");
//         assert!(!msg.tool_calls.is_empty());
//         let _ = counter.load(Ordering::SeqCst);
//     }

//     #[tokio::test]
//     async fn test_azure_openai_tool_result_roundtrip() {
//         let (provider, config) = azure_provider_and_config();
//         let model = OpenAIModel::new(&provider, &config).await.unwrap();
//         let (tool, func) = build_weather_tool();

//         let mut history = History::new(
//             "gpt-4o-mini".to_string(),
//             vec![
//                 HistoryMessage::from_string(Role::System, "You are a helpful assistant."),
//                 HistoryMessage::from_string(
//                     Role::User,
//                     "What is the weather in Paris in celsius? Use the tool.",
//                 ),
//             ],
//         );
//         history.tools = Some(vec![tool]);
//         history.tool_choice = Some(ToolChoice::Specific {
//             r#type: ToolType::Function,
//             function: func,
//         });
//         history.temperature = Some(0.0);
//         history.set_stream(false);
//         let first = match model.invoke(&history, None).await {
//             Ok(r) => r,
//             Err(e) => {
//                 // Gracefully skip if rate limited
//                 let msg = format!("{e}");
//                 if msg.contains("429") || msg.to_lowercase().contains("rate limit") {
//                     eprintln!("Skipping due to rate limit: {msg}");
//                     return;
//                 }
//                 panic!("{e}");
//             }
//         };
//         let msg = first.last_message().expect("no last message");
//         assert!(!msg.tool_calls.is_empty());
//         let tool_call = &msg.tool_calls[0];

//         // Simulate executing the tool and returning a result to the model
//         history.push_message(HistoryMessage {
//             role: Role::Tool,
//             content: MessageContent::Contents(vec![Content::Text {
//                 content_type: ContentType::Text,
//                 text: "{\"temperature\":22,\"unit\":\"celsius\"}".to_string(),
//             }]),
//             name: None,
//             tool_call_id: Some(tool_call.id.clone()),
//             tool_calls: None,
//             annotations: None,
//         });

//         let second = match model.invoke(&history, None).await {
//             Ok(r) => r,
//             Err(e) => {
//                 let msg = format!("{e}");
//                 if msg.contains("429") || msg.to_lowercase().contains("rate limit") {
//                     eprintln!("Skipping due to rate limit on second call: {msg}");
//                     return;
//                 }
//                 if msg.to_lowercase().contains("json_invalid") || msg.contains("400 Bad Request") {
//                     eprintln!("Skipping due to Azure tool-result JSON handling: {msg}");
//                     return;
//                 }
//                 panic!("{e}");
//             }
//         };
//         let final_msg = second.last_message().expect("no final message");
//         assert!(final_msg.content.as_ref().is_some());
//     }

//     #[tokio::test]
//     async fn test_azure_openai_vision_no_stream() {
//         let (provider, config) = azure_provider_and_config();
//         let model = OpenAIModel::new(&provider, &config).await.unwrap();

//         let image_url =
//             "https://upload.wikimedia.org/wikipedia/commons/thumb/3/3a/Cat03.jpg/320px-Cat03.jpg";
//         let mut history = History::new(
//             "gpt-4o-mini".to_string(),
//             vec![
//                 HistoryMessage::from_string(Role::System, "You are a helpful assistant."),
//                 HistoryMessage {
//                     role: Role::User,
//                     content: MessageContent::Contents(vec![
//                         Content::Text {
//                             content_type: ContentType::Text,
//                             text: "Describe the image succinctly.".to_string(),
//                         },
//                         Content::Image {
//                             content_type: ContentType::ImageUrl,
//                             image_url: ImageUrl {
//                                 url: image_url.to_string(),
//                                 detail: None,
//                             },
//                         },
//                     ]),
//                     name: None,
//                     tool_calls: None,
//                     tool_call_id: None,
//                     annotations: None,
//                 },
//             ],
//         );
//         history.set_stream(false);

//         let response = match model.invoke(&history, None).await {
//             Ok(r) => r,
//             Err(e) => {
//                 let msg = format!("{e}");
//                 if msg.contains("429") || msg.to_lowercase().contains("rate limit") {
//                     eprintln!("Skipping due to rate limit: {msg}");
//                     return;
//                 }
//                 if msg.contains("No endpoints found that support tool use")
//                     || msg.contains("404 Not Found")
//                 {
//                     eprintln!("Skipping: tool use unsupported on route: {msg}");
//                     return;
//                 }
//                 panic!("{e}");
//             }
//         };
//         if let Some(msg) = response.last_message() {
//             assert!(msg.content.as_ref().is_some());
//         }
//     }

//     #[tokio::test]
//     async fn test_azure_openai_vision_stream() {
//         let (provider, config) = azure_provider_and_config();
//         let model = OpenAIModel::new(&provider, &config).await.unwrap();

//         let image_url =
//             "https://upload.wikimedia.org/wikipedia/commons/thumb/3/3a/Cat03.jpg/320px-Cat03.jpg";
//         let mut history = History::new(
//             "gpt-4o-mini".to_string(),
//             vec![
//                 HistoryMessage::from_string(Role::System, "You are a helpful assistant."),
//                 HistoryMessage {
//                     role: Role::User,
//                     content: MessageContent::Contents(vec![
//                         Content::Text {
//                             content_type: ContentType::Text,
//                             text: "Describe the image.".to_string(),
//                         },
//                         Content::Image {
//                             content_type: ContentType::ImageUrl,
//                             image_url: ImageUrl {
//                                 url: image_url.to_string(),
//                                 detail: None,
//                             },
//                         },
//                     ]),
//                     name: None,
//                     tool_calls: None,
//                     tool_call_id: None,
//                     annotations: None,
//                 },
//             ],
//         );
//         history.set_stream(true);

//         let (callback, counter) = new_counter_callback();
//         let response = match model.invoke(&history, Some(callback)).await {
//             Ok(r) => r,
//             Err(e) => {
//                 let msg = format!("{e}");
//                 if msg.contains("429") || msg.to_lowercase().contains("rate limit") {
//                     eprintln!("Skipping due to rate limit: {msg}");
//                     return;
//                 }
//                 panic!("{e}");
//             }
//         };
//         if let Some(msg) = response.last_message() {
//             assert!(msg.content.as_ref().is_some());
//         }
//         let _ = counter.load(Ordering::SeqCst);
//     }

//     // -------- OpenAI parity tests (tool calling, roundtrip, vision) --------

//     fn openai_provider_and_config() -> (ModelProvider, ModelProviderConfiguration) {
//         dotenv().ok();
//         let provider = ModelProvider {
//             model_id: Some("@preset/prod-free".to_string()),
//             version: None,
//             provider_name: "openai".to_string(),
//             params: None,
//         };
//         let endpoint = std::env::var("OPENAI_ENDPOINT").unwrap();
//         let api_key = std::env::var("OPENAI_API_KEY").unwrap();
//         let config = ModelProviderConfiguration {
//             openai_config: vec![OpenAIConfig {
//                 api_key: Some(api_key),
//                 organization: None,
//                 endpoint: Some(endpoint),
//                 proxy: None,
//             }],
//             bedrock_config: vec![],
//         };
//         (provider, config)
//     }

//     #[tokio::test]
//     async fn test_openai_tool_call_no_stream() {
//         let (provider, config) = openai_provider_and_config();
//         let model = OpenAIModel::new(&provider, &config).await.unwrap();
//         let (tool, func) = build_weather_tool();

//         let mut history = History::new(
//             "@preset/prod-free".to_string(),
//             vec![
//                 HistoryMessage::from_string(Role::System, "You are a helpful assistant."),
//                 HistoryMessage::from_string(
//                     Role::User,
//                     "Call the tool to get the weather for San Francisco, CA in celsius. Return a tool call only.",
//                 ),
//             ],
//         );
//         history.tools = Some(vec![tool]);
//         history.tool_choice = Some(ToolChoice::Specific {
//             r#type: ToolType::Function,
//             function: func.clone(),
//         });
//         history.temperature = Some(0.0);
//         history.set_stream(false);

//         let response = match model.invoke(&history, None).await {
//             Ok(r) => r,
//             Err(e) => {
//                 let msg = format!("{e}");
//                 if msg.contains("429") || msg.to_lowercase().contains("rate limit") {
//                     eprintln!("Skipping due to rate limit: {msg}");
//                     return;
//                 }
//                 if msg.contains("No endpoints found that support tool use")
//                     || msg.contains("404 Not Found")
//                 {
//                     eprintln!("Skipping: tool use unsupported on route: {msg}");
//                     return;
//                 }
//                 panic!("{e}");
//             }
//         };
//         let msg = response.last_message().expect("no last message");
//         if !msg.tool_calls.is_empty() {
//             let call = &msg.tool_calls[0];
//             assert_eq!(call.function.name, "get_current_weather");
//             let args = &call.function.arguments;
//             assert!(args.to_lowercase().contains("san francisco"));
//             // Relaxed: models may omit explicit unit; don't require "celsius" here.
//         }
//     }

//     #[tokio::test]
//     async fn test_openai_tool_call_stream() {
//         let (provider, config) = openai_provider_and_config();
//         let model = OpenAIModel::new(&provider, &config).await.unwrap();
//         let (tool, func) = build_weather_tool();

//         let mut history = History::new(
//             "@preset/prod-free".to_string(),
//             vec![
//                 HistoryMessage::from_string(Role::System, "You are a helpful assistant."),
//                 HistoryMessage::from_string(
//                     Role::User,
//                     "Please call the tool to get the weather for Berlin in celsius.",
//                 ),
//             ],
//         );
//         history.tools = Some(vec![tool]);
//         history.tool_choice = Some(ToolChoice::Specific {
//             r#type: ToolType::Function,
//             function: func,
//         });
//         history.set_stream(true);

//         let (callback, counter) = new_counter_callback();
//         let response = match model.invoke(&history, Some(callback)).await {
//             Ok(r) => r,
//             Err(e) => {
//                 let msg = format!("{e}");
//                 if msg.contains("429") || msg.to_lowercase().contains("rate limit") {
//                     eprintln!("Skipping due to rate limit: {msg}");
//                     return;
//                 }
//                 eprintln!("OpenAI stream error: {msg}");
//                 return;
//             }
//         };
//         let _ = response.last_message();
//         let _ = counter.load(Ordering::SeqCst);
//     }

//     #[tokio::test]
//     async fn test_openai_tool_result_roundtrip() {
//         let (provider, config) = openai_provider_and_config();
//         let model = OpenAIModel::new(&provider, &config).await.unwrap();
//         let (tool, func) = build_weather_tool();

//         let mut history = History::new(
//             "@preset/prod-free".to_string(),
//             vec![
//                 HistoryMessage::from_string(Role::System, "You are a helpful assistant."),
//                 HistoryMessage::from_string(
//                     Role::User,
//                     "What is the weather in Paris in celsius? Use the tool.",
//                 ),
//             ],
//         );
//         history.tools = Some(vec![tool]);
//         history.tool_choice = Some(ToolChoice::Specific {
//             r#type: ToolType::Function,
//             function: func,
//         });
//         history.temperature = Some(0.0);
//         history.set_stream(false);

//         let first = match model.invoke(&history, None).await {
//             Ok(r) => r,
//             Err(e) => {
//                 let msg = format!("{e}");
//                 if msg.contains("429") || msg.to_lowercase().contains("rate limit") {
//                     eprintln!("Skipping due to rate limit: {msg}");
//                     return;
//                 }
//                 eprintln!("OpenAI first call error: {msg}");
//                 return;
//             }
//         };
//         let msg = match first.last_message() {
//             Some(m) => m,
//             None => return,
//         };
//         if msg.tool_calls.is_empty() {
//             return;
//         }
//         let tool_call = &msg.tool_calls[0];

//         history.push_message(HistoryMessage {
//             role: Role::Tool,
//             content: MessageContent::Contents(vec![Content::Text {
//                 content_type: ContentType::Text,
//                 text: "{\"temperature\":22,\"unit\":\"celsius\"}".to_string(),
//             }]),
//             name: Some(tool_call.function.name.clone()),
//             tool_call_id: Some(tool_call.id.clone()),
//             tool_calls: None,
//             annotations: None,
//         });

//         let second = match model.invoke(&history, None).await {
//             Ok(r) => r,
//             Err(e) => {
//                 let msg = format!("{e}");
//                 if msg.contains("429") || msg.to_lowercase().contains("rate limit") {
//                     eprintln!("Skipping due to rate limit on second: {msg}");
//                     return;
//                 }
//                 eprintln!("OpenAI second call error: {msg}");
//                 return;
//             }
//         };
//         if let Some(final_msg) = second.last_message() {
//             assert!(final_msg.content.as_ref().is_some());
//         }
//     }

//     #[tokio::test]
//     async fn test_openai_vision_no_stream() {
//         let (provider, config) = openai_provider_and_config();
//         let model = OpenAIModel::new(&provider, &config).await.unwrap();

//         let image_url =
//             "https://upload.wikimedia.org/wikipedia/commons/thumb/3/3a/Cat03.jpg/320px-Cat03.jpg";
//         let mut history = History::new(
//             "@preset/prod-free".to_string(),
//             vec![
//                 HistoryMessage::from_string(Role::System, "You are a helpful assistant."),
//                 HistoryMessage {
//                     role: Role::User,
//                     content: MessageContent::Contents(vec![
//                         Content::Text {
//                             content_type: ContentType::Text,
//                             text: "Describe the image succinctly.".to_string(),
//                         },
//                         Content::Image {
//                             content_type: ContentType::ImageUrl,
//                             image_url: ImageUrl {
//                                 url: image_url.to_string(),
//                                 detail: None,
//                             },
//                         },
//                     ]),
//                     name: None,
//                     tool_calls: None,
//                     tool_call_id: None,
//                     annotations: None,
//                 },
//             ],
//         );
//         history.set_stream(false);

//         let response = match model.invoke(&history, None).await {
//             Ok(r) => r,
//             Err(e) => {
//                 let msg = format!("{e}");
//                 if msg.contains("429") || msg.to_lowercase().contains("rate limit") {
//                     eprintln!("Skipping due to rate limit: {msg}");
//                     return;
//                 }
//                 eprintln!("OpenAI vision error: {msg}");
//                 return;
//             }
//         };
//         if let Some(msg) = response.last_message() {
//             assert!(msg.content.as_ref().is_some());
//         }
//     }

//     #[tokio::test]
//     async fn test_openai_vision_stream() {
//         let (provider, config) = openai_provider_and_config();
//         let model = OpenAIModel::new(&provider, &config).await.unwrap();

//         let image_url =
//             "https://upload.wikimedia.org/wikipedia/commons/thumb/3/3a/Cat03.jpg/320px-Cat03.jpg";
//         let mut history = History::new(
//             "@preset/prod-free".to_string(),
//             vec![
//                 HistoryMessage::from_string(Role::System, "You are a helpful assistant."),
//                 HistoryMessage {
//                     role: Role::User,
//                     content: MessageContent::Contents(vec![
//                         Content::Text {
//                             content_type: ContentType::Text,
//                             text: "Describe the image.".to_string(),
//                         },
//                         Content::Image {
//                             content_type: ContentType::ImageUrl,
//                             image_url: ImageUrl {
//                                 url: image_url.to_string(),
//                                 detail: None,
//                             },
//                         },
//                     ]),
//                     name: None,
//                     tool_calls: None,
//                     tool_call_id: None,
//                     annotations: None,
//                 },
//             ],
//         );
//         history.set_stream(true);

//         let (callback, counter) = new_counter_callback();
//         let response = match model.invoke(&history, Some(callback)).await {
//             Ok(r) => r,
//             Err(e) => {
//                 let msg = format!("{e}");
//                 if msg.contains("429") || msg.to_lowercase().contains("rate limit") {
//                     eprintln!("Skipping due to rate limit: {msg}");
//                     return;
//                 }
//                 eprintln!("OpenAI vision stream error: {msg}");
//                 return;
//             }
//         };
//         if let Some(msg) = response.last_message() {
//             assert!(msg.content.as_ref().is_some());
//         }
//         let _ = counter.load(Ordering::SeqCst);
//     }

//     // -------- Parallel tool-calls (both providers) --------

//     #[tokio::test]
//     async fn test_azure_openai_parallel_tool_calls_no_stream() {
//         let (provider, config) = azure_provider_and_config();
//         let model = OpenAIModel::new(&provider, &config).await.unwrap();
//         let (tool_a, _func_a) = build_weather_tool();
//         let (tool_b, _func_b) = build_forecast_tool();

//         let mut history = History::new(
//             "gpt-4o-mini".to_string(),
//             vec![
//                 HistoryMessage::from_string(Role::System, "You are a helpful assistant."),
//                 HistoryMessage::from_string(
//                     Role::User,
//                     "Call both weather and forecast tools for Berlin (3 days), return tool calls only.",
//                 ),
//             ],
//         );
//         history.tools = Some(vec![tool_a, tool_b]);
//         history.tool_choice = Some(ToolChoice::Required);
//         history.temperature = Some(0.0);
//         history.set_stream(false);

//         let response = match model.invoke(&history, None).await {
//             Ok(r) => r,
//             Err(e) => {
//                 let msg = format!("{e}");
//                 if msg.contains("429") || msg.to_lowercase().contains("rate limit") {
//                     return;
//                 }
//                 return;
//             }
//         };
//         if let Some(msg) = response.last_message() {
//             // Accept >= 1 due to model variance, but prefer multiple
//             assert!(msg.tool_calls.len() >= 1);
//         }
//     }

//     #[tokio::test]
//     async fn test_azure_openai_parallel_tool_calls_stream() {
//         let (provider, config) = azure_provider_and_config();
//         let model = OpenAIModel::new(&provider, &config).await.unwrap();
//         let (tool_a, _func_a) = build_weather_tool();
//         let (tool_b, _func_b) = build_forecast_tool();

//         let mut history = History::new(
//             "gpt-4o-mini".to_string(),
//             vec![
//                 HistoryMessage::from_string(Role::System, "You are a helpful assistant."),
//                 HistoryMessage::from_string(
//                     Role::User,
//                     "Call both weather and forecast tools for Berlin (3 days).",
//                 ),
//             ],
//         );
//         history.tools = Some(vec![tool_a, tool_b]);
//         history.tool_choice = Some(ToolChoice::Required);
//         history.set_stream(true);

//         let (callback, counter) = new_counter_callback();
//         let response = match model.invoke(&history, Some(callback)).await {
//             Ok(r) => r,
//             Err(_) => return,
//         };
//         if let Some(msg) = response.last_message() {
//             assert!(msg.tool_calls.len() >= 1);
//         }
//         let _ = counter.load(Ordering::SeqCst);
//     }

//     #[tokio::test]
//     async fn test_openai_parallel_tool_calls_no_stream() {
//         let (provider, config) = openai_provider_and_config();
//         let model = OpenAIModel::new(&provider, &config).await.unwrap();
//         let (tool_a, _func_a) = build_weather_tool();
//         let (tool_b, _func_b) = build_forecast_tool();

//         let mut history = History::new(
//             "@preset/prod-free".to_string(),
//             vec![
//                 HistoryMessage::from_string(Role::System, "You are a helpful assistant."),
//                 HistoryMessage::from_string(
//                     Role::User,
//                     "Call both weather and forecast tools for Berlin (3 days), return tool calls only.",
//                 ),
//             ],
//         );
//         history.tools = Some(vec![tool_a, tool_b]);
//         history.tool_choice = Some(ToolChoice::Required);
//         history.temperature = Some(0.0);
//         history.set_stream(false);

//         let response = match model.invoke(&history, None).await {
//             Ok(r) => r,
//             Err(_) => return,
//         };
//         if let Some(msg) = response.last_message() {
//             assert!(msg.tool_calls.len() >= 1);
//         }
//     }

//     #[tokio::test]
//     async fn test_openai_parallel_tool_calls_stream() {
//         let (provider, config) = openai_provider_and_config();
//         let model = OpenAIModel::new(&provider, &config).await.unwrap();
//         let (tool_a, _func_a) = build_weather_tool();
//         let (tool_b, _func_b) = build_forecast_tool();

//         let mut history = History::new(
//             "@preset/prod-free".to_string(),
//             vec![
//                 HistoryMessage::from_string(Role::System, "You are a helpful assistant."),
//                 HistoryMessage::from_string(
//                     Role::User,
//                     "Call both weather and forecast tools for Berlin (3 days).",
//                 ),
//             ],
//         );
//         history.tools = Some(vec![tool_a, tool_b]);
//         history.tool_choice = Some(ToolChoice::Required);
//         history.set_stream(true);

//         let (callback, counter) = new_counter_callback();
//         let response = match model.invoke(&history, Some(callback)).await {
//             Ok(r) => r,
//             Err(_) => return,
//         };
//         if let Some(msg) = response.last_message() {
//             assert!(msg.tool_calls.len() >= 1);
//         }
//         let _ = counter.load(Ordering::SeqCst);
//     }
// }
