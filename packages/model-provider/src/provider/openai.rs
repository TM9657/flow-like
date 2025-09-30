use flow_like_types::reqwest_eventsource::{Event, RequestBuilderExt};
use futures::StreamExt;
use openai_api_rs::v1::assistant::{AssistantObject, AssistantRequest};
use openai_api_rs::v1::audio::{
    AudioSpeechRequest, AudioSpeechResponse, AudioTranscriptionRequest, AudioTranscriptionResponse,
    AudioTranslationRequest, AudioTranslationResponse,
};
use openai_api_rs::v1::batch::{BatchResponse, ListBatchResponse};
use openai_api_rs::v1::chat_completion::{ChatCompletionRequest, ChatCompletionResponse};
use openai_api_rs::v1::common::{self, DeletionStatus};
use openai_api_rs::v1::completion::{CompletionRequest, CompletionResponse};
use openai_api_rs::v1::edit::{EditRequest, EditResponse};
use openai_api_rs::v1::embedding::{EmbeddingRequest, EmbeddingResponse};
use openai_api_rs::v1::error::APIError;
use openai_api_rs::v1::file::{
    FileDeleteRequest, FileDeleteResponse, FileListResponse, FileRetrieveResponse,
    FileUploadRequest, FileUploadResponse,
};
use openai_api_rs::v1::fine_tuning::{
    CancelFineTuningJobRequest, CreateFineTuningJobRequest, FineTuningJobEvent,
    FineTuningJobObject, FineTuningPagination, ListFineTuningJobEventsRequest,
    RetrieveFineTuningJobRequest,
};
use openai_api_rs::v1::image::{
    ImageEditRequest, ImageEditResponse, ImageGenerationRequest, ImageGenerationResponse,
    ImageVariationRequest, ImageVariationResponse,
};
use openai_api_rs::v1::moderation::{CreateModerationRequest, CreateModerationResponse};

use flow_like_types::Bytes;
use flow_like_types::Value;
use flow_like_types::reqwest::header::{HeaderMap, HeaderName, HeaderValue};
use flow_like_types::reqwest::multipart::{Form, Part};
use flow_like_types::reqwest::{Client, Method, Response};
use serde::Serialize;

use crate::llm::LLMCallback;
use crate::response_chunk::ResponseChunk;
use std::collections::HashMap;
use std::error::Error;
use std::fs::{File, create_dir_all};
use std::io::Read;
use std::io::Write;
use std::path::Path;

use super::{ModelProvider, ModelProviderConfiguration, random_provider};

const API_URL_V1: &str = "https://api.openai.com/v1";

#[derive(Default)]
pub struct OpenAIClientBuilder {
    api_endpoint: Option<String>,
    api_key: Option<String>,
    organization: Option<String>,
    proxy: Option<String>,
    version: Option<String>,
    timeout: Option<u64>,
    headers: Option<HeaderMap>,
    model: Option<String>,
}

pub struct OpenAIClient {
    api_endpoint: String,
    api_key: String,
    organization: Option<String>,
    proxy: Option<String>,
    timeout: Option<u64>,
    version: Option<String>,
    pub headers: Option<HeaderMap>,
    pub model: Option<String>,
}

impl OpenAIClientBuilder {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn with_version(mut self, version: impl Into<String>) -> Self {
        self.version = Some(version.into());
        self
    }

    pub fn with_api_key(mut self, api_key: impl Into<String>) -> Self {
        self.api_key = Some(api_key.into());
        self
    }

    pub fn with_endpoint(mut self, endpoint: impl Into<String>) -> Self {
        self.api_endpoint = Some(endpoint.into());
        self
    }

    pub fn with_organization(mut self, organization: impl Into<String>) -> Self {
        self.organization = Some(organization.into());
        self
    }

    pub fn with_model(mut self, model: impl Into<String>) -> Self {
        self.model = Some(model.into());
        self
    }

    pub fn with_proxy(mut self, proxy: impl Into<String>) -> Self {
        self.proxy = Some(proxy.into());
        self
    }

    pub fn with_timeout(mut self, timeout: u64) -> Self {
        self.timeout = Some(timeout);
        self
    }

    pub fn with_header(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        let headers = self.headers.get_or_insert_with(HeaderMap::new);
        headers.insert(
            HeaderName::from_bytes(key.into().as_bytes()).expect("Invalid header name"),
            HeaderValue::from_str(&value.into()).expect("Invalid header value"),
        );
        self
    }

    pub fn build(self) -> Result<OpenAIClient, Box<dyn Error>> {
        let api_key = self.api_key.ok_or("API key is required")?;
        let api_endpoint = self.api_endpoint.unwrap_or_else(|| {
            std::env::var("OPENAI_API_BASE").unwrap_or_else(|_| API_URL_V1.to_owned())
        });

        Ok(OpenAIClient {
            api_endpoint,
            api_key,
            organization: self.organization,
            proxy: self.proxy,
            version: self.version,
            timeout: self.timeout,
            headers: self.headers,
            model: self.model,
        })
    }
}

impl OpenAIClient {
    pub fn builder() -> OpenAIClientBuilder {
        OpenAIClientBuilder::new()
    }

    pub async fn from_config(
        provider: &ModelProvider,
        config: &ModelProviderConfiguration,
    ) -> flow_like_types::Result<OpenAIClient> {
        let openai_config = random_provider(&config.openai_config)?;

        let mut client = OpenAIClient::builder();

        if let Some(api_key) = &openai_config.api_key {
            client = client.with_api_key(api_key);
        }

        if let Some(organization_id) = &openai_config.organization {
            client = client.with_organization(organization_id.clone());
        }

        if let Some(endpoint) = &openai_config.endpoint {
            client = client.with_endpoint(endpoint.clone());

            if provider.provider_name == "azure" {
                let model_id = provider.model_id.clone().ok_or_else(|| {
                    flow_like_types::anyhow!("ModelID Required for Azure Deployments")
                })?;
                let api_key = openai_config.api_key.clone().ok_or_else(|| {
                    flow_like_types::anyhow!("API key required for Azure Deployments")
                })?;
                let version = provider.version.clone().ok_or_else(|| {
                    flow_like_types::anyhow!("Version required for Azure Deployments")
                })?;
                let endpoint = if endpoint.ends_with('/') {
                    endpoint.to_string()
                } else {
                    format!("{}/", endpoint)
                };
                let endpoint = format!("{}openai/deployments/{}", endpoint, model_id);
                client = client.with_endpoint(endpoint);
                client = client.with_version(version);
                client = client.with_header("api-key", api_key.clone());
            }
        }

        if let Some(proxy) = &openai_config.proxy {
            client = client.with_proxy(proxy.clone());
        }

        let client = client
            .build()
            .map_err(|e| flow_like_types::anyhow!("Failed to create OpenAI client: {}", e))?;

        Ok(client)
    }

    pub async fn from_params(
        params: HashMap<String, Value>,
    ) -> flow_like_types::Result<OpenAIClient> {
        let mut client = OpenAIClient::builder();

        if let Some(api_key) = params.get("api_key").and_then(|v| v.as_str()) {
            client = client.with_api_key(api_key);
        }

        if let Some(organization_id) = params.get("organization").and_then(|v| v.as_str()) {
            client = client.with_organization(organization_id);
        }

        if let Some(model) = params.get("model_id").and_then(|v| v.as_str()) {
            client = client.with_model(model);
        }

        if let Some(endpoint) = params.get("endpoint").and_then(|v| v.as_str()) {
            client = client.with_endpoint(endpoint);

            if let Some(_is_azure) = params.get("is_azure").and_then(|v| v.as_bool()) {
                let model_id =
                    params
                        .get("model_id")
                        .and_then(|v| v.as_str())
                        .ok_or_else(|| {
                            flow_like_types::anyhow!("ModelID Required for Azure Deployments")
                        })?;

                client = client.with_model(model_id);

                let api_key = params
                    .get("api_key")
                    .and_then(|v| v.as_str())
                    .ok_or_else(|| {
                        flow_like_types::anyhow!("API key required for Azure Deployments")
                    })?;
                let version = params
                    .get("version")
                    .and_then(|v| v.as_str())
                    .ok_or_else(|| {
                        flow_like_types::anyhow!("Version required for Azure Deployments")
                    })?;
                let endpoint = if endpoint.ends_with('/') {
                    endpoint.to_string()
                } else {
                    format!("{}/", endpoint)
                };
                let endpoint = format!("{}openai/deployments/{}", endpoint, model_id);
                client = client.with_endpoint(endpoint);
                client = client.with_version(version);
                client = client.with_header("api-key", api_key);
            }
        }

        if let Some(proxy) = params.get("proxy").and_then(|v| v.as_str()) {
            client = client.with_proxy(proxy);
        }

        let client = client
            .build()
            .map_err(|e| flow_like_types::anyhow!("Failed to create OpenAI client: {}", e))?;

        Ok(client)
    }

    async fn build_request(
        &self,
        method: Method,
        path: &str,
    ) -> flow_like_types::reqwest::RequestBuilder {
        let mut url = format!("{}/{}", self.api_endpoint, path);

        if let Some(version) = &self.version {
            url = format!("{}?api-version={}", url, version);
        }

        let client = Client::builder();

        let client = if let Some(timeout) = self.timeout {
            client.timeout(std::time::Duration::from_secs(timeout))
        } else {
            client
        };

        let client = if let Some(proxy) = &self.proxy {
            client.proxy(flow_like_types::reqwest::Proxy::all(proxy).unwrap())
        } else {
            client
        };

        let client = client.build().unwrap();

        let mut request = client
            .request(method, url)
            .header("Authorization", format!("Bearer {}", self.api_key))
            .header("HTTP-Referer", "flow-like.com")
            .header("X-Title", "Flow-Like");

        if let Some(organization) = &self.organization {
            request = request.header("openai-organization", organization);
        }

        if let Some(headers) = &self.headers {
            for (key, value) in headers {
                request = request.header(key, value);
            }
        }

        if Self::is_beta(path) {
            request = request.header("OpenAI-Beta", "assistants=v2");
        }

        request
    }

    async fn post<T: serde::de::DeserializeOwned>(
        &mut self,
        path: &str,
        body: &impl serde::ser::Serialize,
    ) -> Result<T, APIError> {
        let request = self.build_request(Method::POST, path).await;
        let request = request.json(body);
        let response = request.send().await?;
        self.handle_response(response).await
    }

    async fn get<T: serde::de::DeserializeOwned>(&mut self, path: &str) -> Result<T, APIError> {
        let request = self.build_request(Method::GET, path).await;
        let response = request.send().await?;
        self.handle_response(response).await
    }

    async fn get_raw(&self, path: &str) -> Result<Bytes, APIError> {
        let request = self.build_request(Method::GET, path).await;
        let response = request.send().await?;
        Ok(response.bytes().await?)
    }

    async fn delete<T: serde::de::DeserializeOwned>(&mut self, path: &str) -> Result<T, APIError> {
        let request = self.build_request(Method::DELETE, path).await;
        let response = request.send().await?;
        self.handle_response(response).await
    }

    async fn post_form<T: serde::de::DeserializeOwned>(
        &mut self,
        path: &str,
        form: Form,
    ) -> Result<T, APIError> {
        let request = self.build_request(Method::POST, path).await;
        let request = request.multipart(form);
        let response = request.send().await?;
        self.handle_response(response).await
    }

    async fn post_form_raw(&self, path: &str, form: Form) -> Result<Bytes, APIError> {
        let request = self.build_request(Method::POST, path).await;
        let request = request.multipart(form);
        let response = request.send().await?;
        Ok(response.bytes().await?)
    }

    async fn handle_response<T: serde::de::DeserializeOwned>(
        &mut self,
        response: Response,
    ) -> Result<T, APIError> {
        let status = response.status();
        let headers = response.headers().clone();
        if status.is_success() {
            let text = response.text().await.unwrap_or_else(|_| "".to_string());
            match flow_like_types::json::from_str::<T>(&text) {
                Ok(parsed) => {
                    self.headers = Some(headers);
                    Ok(parsed)
                }
                Err(e) => Err(APIError::CustomError {
                    message: format!("Failed to parse JSON: {} / response {}", e, text),
                }),
            }
        } else {
            let error_message = response
                .text()
                .await
                .unwrap_or_else(|_| "Unknown error".to_string());
            Err(APIError::CustomError {
                message: format!("{}: {}", status, error_message),
            })
        }
    }

    pub async fn completion(
        &mut self,
        req: CompletionRequest,
    ) -> Result<CompletionResponse, APIError> {
        self.post("completions", &req).await
    }

    pub async fn edit(&mut self, req: EditRequest) -> Result<EditResponse, APIError> {
        self.post("edits", &req).await
    }

    pub async fn image_generation(
        &mut self,
        req: ImageGenerationRequest,
    ) -> Result<ImageGenerationResponse, APIError> {
        self.post("images/generations", &req).await
    }

    pub async fn image_edit(
        &mut self,
        req: ImageEditRequest,
    ) -> Result<ImageEditResponse, APIError> {
        self.post("images/edits", &req).await
    }

    pub async fn image_variation(
        &mut self,
        req: ImageVariationRequest,
    ) -> Result<ImageVariationResponse, APIError> {
        self.post("images/variations", &req).await
    }

    pub async fn embedding(
        &mut self,
        req: EmbeddingRequest,
    ) -> Result<EmbeddingResponse, APIError> {
        self.post("embeddings", &req).await
    }

    pub async fn file_list(&mut self) -> Result<FileListResponse, APIError> {
        self.get("files").await
    }

    pub async fn upload_file(
        &mut self,
        req: FileUploadRequest,
    ) -> Result<FileUploadResponse, APIError> {
        let form = Self::create_form(&req, "file")?;
        self.post_form("files", form).await
    }

    pub async fn delete_file(
        &mut self,
        req: FileDeleteRequest,
    ) -> Result<FileDeleteResponse, APIError> {
        self.delete(&format!("files/{}", req.file_id)).await
    }

    pub async fn retrieve_file(
        &mut self,
        file_id: String,
    ) -> Result<FileRetrieveResponse, APIError> {
        self.get(&format!("files/{}", file_id)).await
    }

    pub async fn retrieve_file_content(&self, file_id: String) -> Result<Bytes, APIError> {
        self.get_raw(&format!("files/{}/content", file_id)).await
    }

    pub async fn chat_completion(
        &mut self,
        req: ChatCompletionRequest,
        callback: Option<LLMCallback>,
    ) -> Result<crate::response::Response, APIError> {
        let streaming = req.stream.unwrap_or(false);
        if !streaming {
            let response: Result<ChatCompletionResponse, APIError> =
                self.post("chat/completions", &req).await;
            let response = response?;
            return Ok(crate::response::Response::from(response));
        }

        // Streaming case
        let request = self.build_request(Method::POST, "chat/completions").await;
        let mut stream = request
            .json(&req)
            .eventsource()
            .map_err(|e| APIError::CustomError {
                message: format!("Failed to create eventsource stream: {}", e),
            })?;

        let mut output = crate::response::Response::default();

        while let Some(event) = stream.next().await {
            if let Ok(event) = event
                && let Event::Message(event) = event
            {
                let data = &event.data;
                if data == "[DONE]" {
                    break;
                }
                let chunk: ResponseChunk = match flow_like_types::json::from_str(data) {
                    Ok(chunk) => chunk,
                    Err(e) => {
                        eprintln!("Failed to parse chunk: {}, data: {}", e, data);
                        continue;
                    }
                };
                output.push_chunk(chunk.clone());
                if let Some(callback) = &callback {
                    callback(chunk).await.map_err(|e| APIError::CustomError {
                        message: format!("Callback error: {}", e),
                    })?;
                }
            }
        }

        stream.close();

        Ok(output)
    }

    pub async fn audio_transcription(
        &mut self,
        req: AudioTranscriptionRequest,
    ) -> Result<AudioTranscriptionResponse, APIError> {
        // https://platform.openai.com/docs/api-reference/audio/createTranscription#audio-createtranscription-response_format
        if let Some(response_format) = &req.response_format
            && response_format != "json"
            && response_format != "verbose_json"
        {
            return Err(APIError::CustomError {
                    message: "response_format must be either 'json' or 'verbose_json' please use audio_transcription_raw".to_string(),
                });
        }
        let form: Form;
        if req.clone().file.is_some() {
            form = Self::create_form(&req, "file")?;
        } else if let Some(bytes) = req.clone().bytes {
            form = Self::create_form_from_bytes(&req, bytes)?;
        } else {
            return Err(APIError::CustomError {
                message: "Either file or bytes must be provided".to_string(),
            });
        }
        self.post_form("audio/transcriptions", form).await
    }

    pub async fn audio_transcription_raw(
        &mut self,
        req: AudioTranscriptionRequest,
    ) -> Result<Bytes, APIError> {
        // https://platform.openai.com/docs/api-reference/audio/createTranscription#audio-createtranscription-response_format
        if let Some(response_format) = &req.response_format
            && response_format != "text"
            && response_format != "srt"
            && response_format != "vtt"
        {
            return Err(APIError::CustomError {
                    message: "response_format must be either 'text', 'srt' or 'vtt', please use audio_transcription".to_string(),
                });
        }
        let form: Form;
        if req.clone().file.is_some() {
            form = Self::create_form(&req, "file")?;
        } else if let Some(bytes) = req.clone().bytes {
            form = Self::create_form_from_bytes(&req, bytes)?;
        } else {
            return Err(APIError::CustomError {
                message: "Either file or bytes must be provided".to_string(),
            });
        }
        self.post_form_raw("audio/transcriptions", form).await
    }

    pub async fn audio_translation(
        &mut self,
        req: AudioTranslationRequest,
    ) -> Result<AudioTranslationResponse, APIError> {
        let form = Self::create_form(&req, "file")?;
        self.post_form("audio/translations", form).await
    }

    pub async fn audio_speech(
        &mut self,
        req: AudioSpeechRequest,
    ) -> Result<AudioSpeechResponse, APIError> {
        let request = self.build_request(Method::POST, "audio/speech").await;
        let request = request.json(&req);
        let response = request.send().await?;
        let headers = response.headers().clone();
        let bytes = response.bytes().await?;
        let path = Path::new(req.output.as_str());
        if let Some(parent) = path.parent() {
            match create_dir_all(parent) {
                Ok(_) => {}
                Err(e) => {
                    return Err(APIError::CustomError {
                        message: e.to_string(),
                    });
                }
            }
        }
        match File::create(path) {
            Ok(mut file) => match file.write_all(&bytes) {
                Ok(_) => {}
                Err(e) => {
                    return Err(APIError::CustomError {
                        message: e.to_string(),
                    });
                }
            },
            Err(e) => {
                return Err(APIError::CustomError {
                    message: e.to_string(),
                });
            }
        }

        Ok(AudioSpeechResponse {
            result: true,
            headers: Some(headers),
        })
    }

    pub async fn create_fine_tuning_job(
        &mut self,
        req: CreateFineTuningJobRequest,
    ) -> Result<FineTuningJobObject, APIError> {
        self.post("fine_tuning/jobs", &req).await
    }

    pub async fn list_fine_tuning_jobs(
        &mut self,
    ) -> Result<FineTuningPagination<FineTuningJobObject>, APIError> {
        self.get("fine_tuning/jobs").await
    }

    pub async fn list_fine_tuning_job_events(
        &mut self,
        req: ListFineTuningJobEventsRequest,
    ) -> Result<FineTuningPagination<FineTuningJobEvent>, APIError> {
        self.get(&format!(
            "fine_tuning/jobs/{}/events",
            req.fine_tuning_job_id
        ))
        .await
    }

    pub async fn retrieve_fine_tuning_job(
        &mut self,
        req: RetrieveFineTuningJobRequest,
    ) -> Result<FineTuningJobObject, APIError> {
        self.get(&format!("fine_tuning/jobs/{}", req.fine_tuning_job_id))
            .await
    }

    pub async fn cancel_fine_tuning_job(
        &mut self,
        req: CancelFineTuningJobRequest,
    ) -> Result<FineTuningJobObject, APIError> {
        self.post(
            &format!("fine_tuning/jobs/{}/cancel", req.fine_tuning_job_id),
            &req,
        )
        .await
    }

    pub async fn create_moderation(
        &mut self,
        req: CreateModerationRequest,
    ) -> Result<CreateModerationResponse, APIError> {
        self.post("moderations", &req).await
    }

    pub async fn create_assistant(
        &mut self,
        req: AssistantRequest,
    ) -> Result<AssistantObject, APIError> {
        self.post("assistants", &req).await
    }

    pub async fn retrieve_assistant(
        &mut self,
        assistant_id: String,
    ) -> Result<AssistantObject, APIError> {
        self.get(&format!("assistants/{}", assistant_id)).await
    }

    pub async fn modify_assistant(
        &mut self,
        assistant_id: String,
        req: AssistantRequest,
    ) -> Result<AssistantObject, APIError> {
        self.post(&format!("assistants/{}", assistant_id), &req)
            .await
    }

    pub async fn delete_assistant(
        &mut self,
        assistant_id: String,
    ) -> Result<DeletionStatus, APIError> {
        self.delete(&format!("assistants/{}", assistant_id)).await
    }

    pub async fn retrieve_batch(&mut self, batch_id: String) -> Result<BatchResponse, APIError> {
        self.get(&format!("batches/{}", batch_id)).await
    }

    pub async fn cancel_batch(&mut self, batch_id: String) -> Result<BatchResponse, APIError> {
        self.post(
            &format!("batches/{}/cancel", batch_id),
            &common::EmptyRequestBody {},
        )
        .await
    }

    pub async fn list_batch(
        &mut self,
        after: Option<String>,
        limit: Option<i64>,
    ) -> Result<ListBatchResponse, APIError> {
        let url = Self::query_params(limit, None, after, None, "batches".to_string());
        self.get(&url).await
    }

    fn query_params(
        limit: Option<i64>,
        order: Option<String>,
        after: Option<String>,
        before: Option<String>,
        mut url: String,
    ) -> String {
        let mut params = vec![];
        if let Some(limit) = limit {
            params.push(format!("limit={}", limit));
        }
        if let Some(order) = order {
            params.push(format!("order={}", order));
        }
        if let Some(after) = after {
            params.push(format!("after={}", after));
        }
        if let Some(before) = before {
            params.push(format!("before={}", before));
        }
        if !params.is_empty() {
            url = format!("{}?{}", url, params.join("&"));
        }
        url
    }

    fn is_beta(path: &str) -> bool {
        path.starts_with("assistants") || path.starts_with("threads")
    }

    fn create_form<T>(req: &T, file_field: &str) -> Result<Form, APIError>
    where
        T: Serialize,
    {
        let json = match flow_like_types::json::to_value(req) {
            Ok(json) => json,
            Err(e) => {
                return Err(APIError::CustomError {
                    message: e.to_string(),
                });
            }
        };
        let file_path = if let Value::Object(map) = &json {
            map.get(file_field)
                .and_then(|v| v.as_str())
                .ok_or(APIError::CustomError {
                    message: format!("Field '{}' not found or not a string", file_field),
                })?
        } else {
            return Err(APIError::CustomError {
                message: "Request is not a JSON object".to_string(),
            });
        };

        let mut file = match File::open(file_path) {
            Ok(file) => file,
            Err(e) => {
                return Err(APIError::CustomError {
                    message: e.to_string(),
                });
            }
        };
        let mut buffer = Vec::new();
        match file.read_to_end(&mut buffer) {
            Ok(_) => {}
            Err(e) => {
                return Err(APIError::CustomError {
                    message: e.to_string(),
                });
            }
        }

        let mut form =
            Form::new().part("file", Part::bytes(buffer).file_name(file_path.to_string()));

        if let Value::Object(map) = json {
            for (key, value) in map.into_iter() {
                if key != file_field {
                    match value {
                        Value::String(s) => {
                            form = form.text(key, s);
                        }
                        Value::Number(n) => {
                            form = form.text(key, n.to_string());
                        }
                        _ => {}
                    }
                }
            }
        }

        Ok(form)
    }

    fn create_form_from_bytes<T>(req: &T, bytes: Vec<u8>) -> Result<Form, APIError>
    where
        T: Serialize,
    {
        let json = match flow_like_types::json::to_value(req) {
            Ok(json) => json,
            Err(e) => {
                return Err(APIError::CustomError {
                    message: e.to_string(),
                });
            }
        };

        let mut form = Form::new().part("file", Part::bytes(bytes.clone()).file_name("file.mp3"));

        if let Value::Object(map) = json {
            for (key, value) in map.into_iter() {
                match value {
                    Value::String(s) => {
                        form = form.text(key, s);
                    }
                    Value::Number(n) => {
                        form = form.text(key, n.to_string());
                    }
                    _ => {}
                }
            }
        }

        Ok(form)
    }
}
