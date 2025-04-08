use std::sync::Arc;

use super::{LLMCallback, ModelLogic};
use crate::{
    history::{History, Role},
    provider::{ModelProvider, ModelProviderConfiguration, random_provider},
    response::Response,
};
use aws_sdk_bedrockruntime::{
    Client,
    operation::{
        converse::builders::ConverseFluentBuilder,
        converse_stream::builders::ConverseStreamFluentBuilder,
    },
    types::{InferenceConfiguration, SystemContentBlock},
};
use flow_like_types::{Result, async_trait};
use response::{convert_bedrock_response, convert_bedrock_stream_output};

pub mod history;
pub mod response;

pub struct BedrockModel {
    client: Arc<Client>,
    provider: ModelProvider,
}

impl BedrockModel {
    pub async fn new(
        provider: &ModelProvider,
        config: &ModelProviderConfiguration,
    ) -> flow_like_types::Result<Self> {
        let random_provider = random_provider(&config.bedrock_config)?;
        let client = Client::new(&random_provider.config);

        Ok(BedrockModel {
            client: Arc::new(client),
            provider: provider.clone(),
        })
    }
}

#[async_trait]
impl ModelLogic for BedrockModel {
    async fn invoke(&self, history: &History, callback: Option<LLMCallback>) -> Result<Response> {
        let model_id = self
            .provider
            .model_id
            .clone()
            .ok_or_else(|| flow_like_types::anyhow!("Model ID is missing"))?;

        let stream = history.stream.unwrap_or(false);

        if !stream {
            let request = build(&self.client, history, &model_id).await?;
            let response = request.send().await?;
            return convert_bedrock_response(response).await;
        }

        let request = build_stream(&self.client, history, &model_id).await?;
        let response = request.send().await?;

        let mut stream = response.stream;
        let mut response = Response::new();

        loop {
            let token = stream.recv().await?;
            match token {
                Some(text) => {
                    if let Ok(chunk) = convert_bedrock_stream_output(text) {
                        if let Some(callback) = &callback {
                            callback(chunk.clone()).await?;
                        }
                        response.push_chunk(chunk);
                    }
                }
                None => break,
            };
        }

        Ok(response)
    }
}

async fn build_stream(
    client: &Arc<Client>,
    history: &History,
    model_id: &str,
) -> Result<ConverseStreamFluentBuilder> {
    let mut request = client.converse_stream();

    let system_prompt = history.messages.iter().find(|m| m.role == Role::System);
    let messages = history.to_messages().await;
    request = request.model_id(model_id);
    request = request.set_messages(Some(messages));

    if let Some(system_prompt) = system_prompt {
        for content in &system_prompt.content {
            match content {
                crate::history::Content::Text { text, .. } => {
                    request =
                        request.set_system(Some(vec![SystemContentBlock::Text(text.clone())]));
                }
                _ => continue,
            }
        }
    }

    let mut inference_configuration = InferenceConfiguration::builder();

    if let Some(max_token) = history.max_completion_tokens {
        inference_configuration = inference_configuration.set_max_tokens(Some(max_token as i32));
    }

    if let Some(temperature) = history.temperature {
        inference_configuration = inference_configuration.set_temperature(Some(temperature));
    }

    if let Some(top_p) = history.top_p {
        inference_configuration = inference_configuration.set_top_p(Some(top_p));
    }

    if let Some(stop) = &history.stop {
        inference_configuration = inference_configuration.set_stop_sequences(Some(stop.clone()));
    }

    let inference_config = inference_configuration.build();

    request = request.set_inference_config(Some(inference_config));

    Ok(request)
}

async fn build(
    client: &Arc<Client>,
    history: &History,
    model_id: &str,
) -> Result<ConverseFluentBuilder> {
    let mut request = client.converse();

    let system_prompt = history.messages.iter().find(|m| m.role == Role::System);
    let messages = history.to_messages().await;
    request = request.model_id(model_id);
    request = request.set_messages(Some(messages));

    if let Some(system_prompt) = system_prompt {
        for content in &system_prompt.content {
            match content {
                crate::history::Content::Text { text, .. } => {
                    request =
                        request.set_system(Some(vec![SystemContentBlock::Text(text.clone())]));
                }
                _ => continue,
            }
        }
    }

    let mut inference_configuration = InferenceConfiguration::builder();

    if let Some(max_token) = history.max_completion_tokens {
        inference_configuration = inference_configuration.set_max_tokens(Some(max_token as i32));
    }

    if let Some(temperature) = history.temperature {
        inference_configuration = inference_configuration.set_temperature(Some(temperature));
    }

    if let Some(top_p) = history.top_p {
        inference_configuration = inference_configuration.set_top_p(Some(top_p));
    }

    if let Some(stop) = &history.stop {
        inference_configuration = inference_configuration.set_stop_sequences(Some(stop.clone()));
    }

    let inference_config = inference_configuration.build();

    request = request.set_inference_config(Some(inference_config));

    Ok(request)
}

#[cfg(test)]
mod tests {

    use flow_like_types::tokio;

    use dotenv::dotenv;

    #[tokio::test]
    async fn test_bedrock_no_stream() {
        dotenv().ok();
    }
}
