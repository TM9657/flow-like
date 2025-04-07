use std::sync::Arc;

use super::{LLMCallback, ModelLogic};
use crate::{
    history::History,
    provider::{ModelProvider, ModelProviderConfiguration, openai::OpenAIClient},
    response::Response,
};
use flow_like_types::{Result, async_trait, sync::Mutex};
use openai_api_rs::v1::chat_completion::ChatCompletionRequest;
mod history;
mod response;

pub struct OpenAIModel {
    client: Arc<Mutex<OpenAIClient>>,
    provider: ModelProvider,
}

impl OpenAIModel {
    pub async fn new(
        provider: &ModelProvider,
        config: &ModelProviderConfiguration,
    ) -> flow_like_types::Result<Self> {
        let client = OpenAIClient::from_config(provider, config).await?;

        Ok(OpenAIModel {
            client: Arc::new(Mutex::new(client)),
            provider: provider.clone(),
        })
    }
}

#[async_trait]
impl ModelLogic for OpenAIModel {
    async fn invoke(&self, history: &History, callback: Option<LLMCallback>) -> Result<Response> {
        let model_id = self
            .provider
            .model_id
            .clone()
            .ok_or_else(|| flow_like_types::anyhow!("Model ID is missing"))?;
        let mut request = ChatCompletionRequest::from(history.clone());
        request.model = model_id;

        let completion = {
            let mut client = self.client.lock().await;
            client.chat_completion(request, callback).await?
        };
        let response = completion;
        Ok(response)
    }
}

#[cfg(test)]
mod tests {
    use std::sync::atomic::{AtomicUsize, Ordering};

    use flow_like_types::tokio;

    use super::*;
    use crate::{
        history::{HistoryMessage, Role},
        provider::{ModelProviderConfiguration, OpenAIConfig},
    };
    use dotenv::dotenv;

    #[tokio::test]
    async fn test_openai_model_no_stream() {
        dotenv().ok();

        let provider = ModelProvider {
            model_id: Some("gpt-4o-mini".to_string()),
            version: None,
            provider_name: "openai".to_string(),
        };
        let api_key = std::env::var("OPENAI_API_KEY").unwrap();
        let config = ModelProviderConfiguration {
            openai_config: vec![OpenAIConfig {
                api_key: Some(api_key),
                organization: None,
                endpoint: None,
                proxy: None,
            }],
            bedrock_config: vec![],
        };

        let model = OpenAIModel::new(&provider, &config).await.unwrap();
        let mut history = History::new(
            "gpt-4o-mini".to_string(),
            vec![
                HistoryMessage::from_string(Role::System, "You are a helpful assistant."),
                HistoryMessage::from_string(Role::User, "Hello"),
            ],
        );
        history.set_stream(false);
        let response = model.invoke(&history, None).await.unwrap();
        assert!(!response.choices.is_empty());
    }

    #[tokio::test]
    async fn test_azure_openai_model_no_stream() {
        dotenv().ok();

        let provider = ModelProvider {
            model_id: Some("gpt-4o-mini".to_string()),
            version: Some("2024-02-15-preview".to_string()),
            provider_name: "azure".to_string(),
        };
        let api_key = std::env::var("AZURE_OPENAI_API_KEY").unwrap();
        let endpoint = std::env::var("AZURE_OPENAI_ENDPOINT").unwrap();
        let config = ModelProviderConfiguration {
            openai_config: vec![OpenAIConfig {
                api_key: Some(api_key),
                organization: None,
                endpoint: Some(endpoint),
                proxy: None,
            }],
            bedrock_config: vec![],
        };

        let model = OpenAIModel::new(&provider, &config).await.unwrap();
        let mut history = History::new(
            "gpt-4o-mini".to_string(),
            vec![
                HistoryMessage::from_string(Role::System, "You are a helpful assistant."),
                HistoryMessage::from_string(Role::User, "Hello"),
            ],
        );
        history.set_stream(false);
        let response = model.invoke(&history, None).await.unwrap();
        println!("Final response: {:?}", response.last_message());
        assert!(!response.choices.is_empty());
    }

    #[tokio::test]
    async fn test_openai_model_stream() {
        dotenv().ok();

        let provider = ModelProvider {
            model_id: Some("gpt-4o-mini".to_string()),
            version: None,
            provider_name: "openai".to_string(),
        };
        let api_key = std::env::var("OPENAI_API_KEY").unwrap();
        let config = ModelProviderConfiguration {
            openai_config: vec![OpenAIConfig {
                api_key: Some(api_key),
                organization: None,
                endpoint: None,
                proxy: None,
            }],
            bedrock_config: vec![],
        };

        let model = OpenAIModel::new(&provider, &config).await.unwrap();
        let mut history = History::new(
            "gpt-4o-mini".to_string(),
            vec![
                HistoryMessage::from_string(Role::System, "You are a helpful assistant."),
                HistoryMessage::from_string(Role::User, "Hello"),
            ],
        );
        history.set_stream(true);

        let counter = Arc::new(AtomicUsize::new(0));
        let counter_clone = counter.clone();
        let callback: LLMCallback = Arc::new(move |_response| {
            counter_clone.fetch_add(1, Ordering::SeqCst);
            Box::pin(async move { Ok(()) })
        });

        let response = model.invoke(&history, Some(callback)).await.unwrap();
        println!("Final response: {:?}", response.last_message());
        println!("Chunks: {}", counter.load(Ordering::SeqCst));
        assert!(!response.choices.is_empty());
        assert!(counter.load(Ordering::SeqCst) > 1);
    }

    #[tokio::test]
    async fn test_azure_openai_model_stream() {
        dotenv().ok();

        let provider = ModelProvider {
            model_id: Some("gpt-4o-mini".to_string()),
            version: Some("2024-02-15-preview".to_string()),
            provider_name: "azure".to_string(),
        };
        let api_key = std::env::var("AZURE_OPENAI_API_KEY").unwrap();
        let endpoint = std::env::var("AZURE_OPENAI_ENDPOINT").unwrap();
        let config = ModelProviderConfiguration {
            openai_config: vec![OpenAIConfig {
                api_key: Some(api_key),
                organization: None,
                endpoint: Some(endpoint),
                proxy: None,
            }],
            bedrock_config: vec![],
        };

        let model = OpenAIModel::new(&provider, &config).await.unwrap();
        let mut history = History::new(
            "gpt-4o-mini".to_string(),
            vec![
                HistoryMessage::from_string(Role::System, "You are a helpful assistant."),
                HistoryMessage::from_string(Role::User, "Hello"),
            ],
        );
        history.set_stream(true);

        let counter = Arc::new(AtomicUsize::new(0));
        let counter_clone = counter.clone();
        let callback: LLMCallback = Arc::new(move |_response| {
            counter_clone.fetch_add(1, Ordering::SeqCst);
            Box::pin(async move { Ok(()) })
        });

        let response = model.invoke(&history, Some(callback)).await.unwrap();
        println!("Final response: {:?}", response.last_message());
        println!("Chunks: {}", counter.load(Ordering::SeqCst));
        assert!(!response.choices.is_empty());
        assert!(counter.load(Ordering::SeqCst) > 1);
    }
}
