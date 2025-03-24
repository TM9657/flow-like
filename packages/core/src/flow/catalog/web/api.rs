use crate::flow::{
    catalog::storage::path::FlowPath, execution::context::ExecutionContext, node::NodeLogic,
};
use futures::StreamExt;
use object_store::PutPayload;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::{future::Future, pin::Pin, sync::Arc};

pub mod download;
pub mod fetch;
pub mod streaming_fetch;
pub mod request;
pub mod response;

pub type StreamingCallback = Arc<
    dyn Fn(bytes::Bytes) -> Pin<Box<dyn Future<Output = anyhow::Result<()>> + Send>>
        + Send
        + Sync
        + 'static,
>;

#[derive(Serialize, Deserialize, Clone, Debug, JsonSchema)]
pub enum Method {
    GET,
    POST,
    PUT,
    DELETE,
    PATCH,
}

#[derive(Serialize, Deserialize, Clone, Debug, JsonSchema)]
#[serde(untagged)]
pub enum HttpBody {
    Json(Value),
    Bytes(Vec<u8>),
    String(String),
}

#[derive(Serialize, Deserialize, Clone, Debug, JsonSchema)]
pub struct HttpRequest {
    pub url: String,
    pub method: Method,
    pub headers: Option<std::collections::HashMap<String, String>>,
    pub body: Option<HttpBody>,
}

impl HttpRequest {
    pub fn new(url: String, method: Method) -> Self {
        HttpRequest {
            url,
            method,
            headers: None,
            body: None,
        }
    }

    pub fn set_headers(&mut self, headers: std::collections::HashMap<String, String>) {
        self.headers = Some(headers);
    }

    pub fn set_header(&mut self, key: String, value: String) {
        if self.headers.is_none() {
            self.headers = Some(std::collections::HashMap::new());
        }

        self.headers.as_mut().unwrap().insert(key, value);
    }

    pub fn set_body(&mut self, body: HttpBody) {
        self.body = Some(body);
    }

    async fn to_request(
        &self,
        client: &reqwest::Client,
    ) -> anyhow::Result<reqwest::RequestBuilder> {
        let method: reqwest::Method = match self.method {
            Method::GET => reqwest::Method::GET,
            Method::POST => reqwest::Method::POST,
            Method::PUT => reqwest::Method::PUT,
            Method::DELETE => reqwest::Method::DELETE,
            Method::PATCH => reqwest::Method::PATCH,
        };

        let mut request = client.request(method, &self.url);

        if let Some(headers) = &self.headers {
            for (key, value) in headers.iter() {
                request = request.header(key, value);
            }
        }

        if let Some(body) = &self.body {
            match body {
                HttpBody::Json(value) => {
                    request = request.json(value);
                }
                HttpBody::Bytes(value) => {
                    request = request.body(value.clone());
                }
                HttpBody::String(value) => {
                    request = request.body(value.clone());
                }
            }
        }

        Ok(request)
    }

    pub async fn trigger(&self, client: &reqwest::Client) -> anyhow::Result<HttpResponse> {
        let request = self.to_request(client).await?;
        let response = request.send().await?;
        let status_code = response.status().as_u16();
        let headers = response.headers().clone();
        let body = response.bytes().await?.to_vec();

        Ok(HttpResponse {
            status_code,
            headers: headers
                .iter()
                .map(|(key, value)| {
                    (
                        key.as_str().to_string(),
                        value.to_str().unwrap().to_string(),
                    )
                })
                .collect(),
            body: Some(body.to_vec()),
        })
    }

    pub async fn streaming_trigger(
        &self,
        client: &reqwest::Client,
        callback: Option<StreamingCallback>,
    ) -> anyhow::Result<HttpResponse>{
        let request = self.to_request(client).await?;
        let response = request.send().await?;
        let status_code = response.status().as_u16();
        let headers = response.headers().clone();

        let mut stream = response.bytes_stream();
        let mut response_body = vec![];

        while let Some(chunk_result) = stream.next().await {
            let chunk = chunk_result?;
            response_body.extend_from_slice(&chunk);
            if let Some(callback) = &callback {
                callback(chunk).await?;
            }
        }

        Ok(HttpResponse {
            status_code,
            headers: headers
                .iter()
                .map(|(key, value)| {
                    (
                        key.as_str().to_string(),
                        value.to_str().unwrap().to_string(),
                    )
                })
                .collect(),
            body: Some(response_body), // No body collected in streaming mode
        })
    }

    pub async fn download_to_path(
        &self,
        client: &reqwest::Client,
        path: &FlowPath,
        context: &mut ExecutionContext,
    ) -> anyhow::Result<()> {
        let request = self.to_request(client).await?;
        let response = request.send().await?;
        let mut stream = response.bytes_stream();
        let rt = path.to_runtime(context).await?;
        let store = rt.store.as_generic();
        let mut writer = store.put_multipart(&rt.path).await?;

        while let Some(chunk) = stream.next().await {
            let chunk = chunk?;
            let payload = PutPayload::from_bytes(chunk);
            writer.put_part(payload).await?;
        }

        writer.complete().await?;
        Ok(())
    }
}

#[derive(Serialize, Deserialize, Clone, Debug, JsonSchema)]
pub struct HttpResponse {
    pub status_code: u16,
    pub headers: std::collections::HashMap<String, String>,
    pub body: Option<Vec<u8>>,
}

impl HttpResponse {
    pub fn is_success(&self) -> bool {
        self.status_code >= 200 && self.status_code < 300
    }

    pub fn get_status_code(&self) -> u16 {
        self.status_code
    }

    pub fn get_headers(&self) -> std::collections::HashMap<String, String> {
        self.headers.clone()
    }

    pub fn get_header(&self, key: &str) -> Option<&String> {
        self.headers.get(key)
    }

    pub fn to_json(&self) -> anyhow::Result<Value> {
        let body = self.body.as_ref().ok_or(anyhow::anyhow!("No body"))?;
        let body: Value = serde_json::from_slice(body)?;
        Ok(body)
    }

    pub fn to_text(&self) -> anyhow::Result<String> {
        let body = self.body.as_ref().ok_or(anyhow::anyhow!("No body"))?;
        let body = String::from_utf8_lossy(body);
        Ok(body.to_string())
    }

    pub fn to_bytes(&self) -> Vec<u8> {
        self.body.as_ref().unwrap_or(&vec![]).clone()
    }
}

pub async fn register_functions() -> Vec<Arc<dyn NodeLogic>> {
    let mut out: Vec<Arc<dyn NodeLogic>> = vec![
        Arc::new(download::HttpDownloadNode::default()),
        Arc::new(fetch::HttpFetchNode::default()),
        Arc::new(streaming_fetch::StreamingHttpFetchNode::default()),
    ];

    out.extend(request::register_functions().await);
    out.extend(response::register_functions().await);

    out
}
