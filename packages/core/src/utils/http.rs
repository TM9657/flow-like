use flow_like_storage::blake3;
use flow_like_types::{
    Value,
    reqwest::{self, Request},
    sync::{DashMap, mpsc},
};
use serde::{Deserialize, Serialize};
use std::{sync::Arc, time::Duration};

use super::cache::{cache_file_exists, read_cache_file, write_cache_file};

const HEADERS_TO_CACHE: [&str; 8] = [
    "authorization",
    "x-api-key",
    "x-api-token",
    "accept",
    "content-type",
    "user-agent",
    "accept-encoding",
    "accept-language",
];

#[derive(Serialize, Deserialize, Debug)]
pub struct HTTPClient {
    pub cache: Arc<DashMap<String, Value>>,

    #[serde(skip)]
    sender: Option<mpsc::Sender<Request>>,

    #[serde(skip)]
    client: reqwest::Client,
}

impl HTTPClient {
    pub fn new() -> (HTTPClient, mpsc::Receiver<Request>) {
        let (tx, rx) = mpsc::channel(1000);
        (
            HTTPClient {
                cache: Arc::new(DashMap::new()),
                sender: Some(tx),
                client: reqwest::Client::new(),
            },
            rx,
        )
    }

    /// Refetches the request
    /// This is used to update the cache in the background
    async fn refetch(&self, request: &Request) {
        if let Some(sender) = &self.sender {
            let request = request.try_clone();
            if let Some(request) = request
                && let Err(e) = sender.send_timeout(request, Duration::from_secs(1)).await
            {
                eprintln!("Failed to send request: {}", e);
            }
        }
    }

    /// Fastest cache, but not persistent
    async fn handle_in_memory<T>(
        &self,
        request_hash: &str,
        request: &Request,
    ) -> flow_like_types::Result<T>
    where
        for<'de> T: Deserialize<'de> + Clone,
    {
        let value = self
            .cache
            .get(request_hash)
            .ok_or(flow_like_types::anyhow!("Value not found in cache"))?;
        let value = value.value();
        let value = flow_like_types::json::from_value::<T>(value.clone())?;

        self.refetch(request).await;
        Ok(value)
    }

    /// Slower than in memory cache, but faster than fetching from the network
    async fn handle_file_cache<T>(
        &self,
        request_hash: &str,
        request: &Request,
    ) -> flow_like_types::Result<T>
    where
        for<'de> T: Deserialize<'de> + Clone,
    {
        let string_hash = format!("http/{}", request_hash);
        let file_exists = cache_file_exists(&string_hash);
        if !file_exists {
            println!("Cache file does not exist: {}", string_hash);
            return Err(flow_like_types::anyhow!("Cache file does not exist"));
        }

        let cache_string = read_cache_file(&string_hash)?;
        let generic_value = flow_like_types::json::from_slice::<Value>(&cache_string)?;
        self.cache
            .insert(request_hash.to_string(), generic_value.clone());
        let value = flow_like_types::json::from_value::<T>(generic_value)?;
        self.refetch(request).await;
        Ok(value)
    }

    pub fn quick_hash(&self, request: &Request) -> String {
        let mut hasher = blake3::Hasher::new();
        hasher.update(request.url().as_str().as_bytes());
        hasher.update(request.method().as_str().as_bytes());

        let mut headers_to_hash: Vec<_> = request
            .headers()
            .iter()
            .map(|(key, value)| (key.as_str(), value))
            .filter(|(key, _)| HEADERS_TO_CACHE.contains(&key.to_lowercase().as_str()))
            .collect();
        headers_to_hash.sort_by_key(|(key, _)| *key);

        for (key, value) in headers_to_hash {
            hasher.update(key.as_bytes());
            hasher.update(value.as_bytes());
        }

        if let Some(body) = request.body()
            && let Some(body) = body.as_bytes()
        {
            hasher.update(body);
        }

        let request_hash = hasher.finalize();
        let hex = request_hash.to_hex();

        hex.to_string()
    }

    pub fn client(&self) -> reqwest::Client {
        self.client.clone()
    }

    pub async fn hashed_request<T>(&self, request: Request) -> flow_like_types::Result<T>
    where
        for<'de> T: Deserialize<'de> + Clone + Serialize,
    {
        let request_hash = self.quick_hash(&request);

        // checks the in memory cache
        if let Ok(value) = self.handle_in_memory(&request_hash, &request).await {
            return Ok(value);
        }

        // checks the file cache
        if let Ok(value) = self.handle_file_cache(&request_hash, &request).await {
            return Ok(value);
        }

        // fetches from the network
        let response = self.client.execute(request).await?;
        println!("Response: {:?}", response);
        let value = response.json::<Value>().await?;
        let _ = self.put(&request_hash, &value);
        let value = flow_like_types::json::from_value::<T>(value.clone())?;
        Ok(value)
    }

    pub fn put(&self, request_hash: &str, body: &Value) -> flow_like_types::Result<()> {
        let string_hash = format!("http/{}", request_hash);
        self.cache.insert(request_hash.to_string(), body.clone());
        write_cache_file(&string_hash, &flow_like_types::json::to_vec(body)?)?;
        Ok(())
    }
}
