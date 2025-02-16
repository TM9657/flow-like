use std::sync::Arc;

use crate::{
    bit::{Bit, BitTypes},
    profile::Profile,
    utils::{http::HTTPClient, recursion::RecursionGuard},
};
use anyhow::Result;
use reqwest::Url;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use tokio::sync::Mutex;

#[derive(Serialize, Deserialize, JsonSchema, Debug, Clone)]
pub struct Hub {
    name: String,
    description: String,
    thumbnail: String,
    icon: String,
    domain: String,
    dependencies: Vec<String>,

    #[serde(skip)]
    recursion_guard: Arc<Mutex<RecursionGuard>>,

    #[serde(skip)]
    http_client: Option<Arc<HTTPClient>>,
}

impl Hub {
    fn http_client(&self) -> Arc<HTTPClient> {
        self.http_client.clone().unwrap()
    }

    pub async fn new(url: &str, http_client: Arc<HTTPClient>) -> Result<Hub> {
        let mut url = String::from(url);
        if !url.starts_with("https://") {
            url = format!("https://{}", url);
        }

        let url = match Url::parse(&url) {
            Ok(url) => url,
            Err(e) => {
                println!("Error parsing URL: {:?}", e);
                return Err(anyhow::Error::msg("Invalid URL"));
            }
        };

        // TODO Cache this.
        // We should implement a global Cache anyways, best with support for reqwest
        let manifest_url = url.join("static/hub.json").unwrap();
        let request = http_client.client().get(manifest_url).build()?;
        let mut info: Hub = http_client.hashed_request(request).await?;
        info.recursion_guard = RecursionGuard::new(vec![url.as_ref()]);
        info.domain = url.to_string();
        info.http_client = Some(http_client);
        Ok(info)
    }

    pub async fn get_bit_by_id(&self, bit_id: &str) -> Result<Bit> {
        let url = Url::parse(&self.domain).unwrap();
        let bit_url = url
            .join(format!("static/bits/{}.json", bit_id).as_str())
            .unwrap();
        let request = self.http_client().client().get(bit_url).build()?;
        let bit = self.http_client().hashed_request::<Bit>(request).await;
        if let Ok(bit) = bit {
            return Ok(bit);
        }

        let dependency_hubs = self.get_dependency_hubs().await?;
        for hub in dependency_hubs {
            let bit = Box::pin(hub.get_bit_by_id(bit_id)).await;
            match bit {
                Ok(bit) => return Ok(bit),
                Err(_) => continue,
            }
        }

        Err(anyhow::Error::msg("Bit not found")) // Return an error if the bit is not found in any of the dependency hubs
    }

    pub async fn set_recursion_guard(&mut self, guard: Arc<Mutex<RecursionGuard>>) {
        self.recursion_guard = guard;
        self.recursion_guard.lock().await.insert(&self.domain);
    }

    pub async fn get_bits_of_type(&self, bit_type: &BitTypes) -> Result<Vec<Bit>> {
        let url = Url::parse(&self.domain).unwrap();
        let url_type = format!(
            "static/{}.json",
            serde_json::to_string(&bit_type).unwrap().replace("\"", "")
        );
        let type_bits_url = url.join(&url_type).unwrap();
        let request = self.http_client().client().get(type_bits_url).build()?;
        let mut bits = self
            .http_client()
            .hashed_request::<Vec<Bit>>(request)
            .await?;
        let dependency_hubs = self.get_dependency_hubs().await?;

        for hub in dependency_hubs {
            let hub_models = Box::pin(hub.get_bits_of_type(bit_type)).await?;
            bits.extend(hub_models);
        }

        Ok(bits)
    }

    pub async fn get_bits(&self) -> Result<Vec<Bit>> {
        let url = Url::parse(&self.domain).unwrap();
        let bits_url = url.join("static/bits.json").unwrap();
        let request = self.http_client().client().get(bits_url).build()?;
        let mut bits = self
            .http_client()
            .hashed_request::<Vec<Bit>>(request)
            .await?;
        let dependency_hubs = self.get_dependency_hubs().await?;

        for hub in dependency_hubs {
            let hub_models = Box::pin(hub.get_bits()).await?;
            bits.extend(hub_models);
        }

        Ok(bits)
    }

    pub async fn get_profiles(&self) -> Result<Vec<Profile>> {
        let url = Url::parse(&self.domain).unwrap();
        let profiles_url = url.join("static/profiles.json").unwrap();
        let request = self.http_client().client().get(profiles_url).build()?;
        let bits = self
            .http_client()
            .hashed_request::<Vec<Profile>>(request)
            .await?;
        let bits = bits
            .into_iter()
            .map(|mut bit| {
                bit.hub = self.domain.clone();
                bit
            })
            .collect();
        Ok(bits)
    }

    // should be optimized
    pub async fn get_dependency_hubs(&self) -> Result<Vec<Hub>> {
        let mut hubs = vec![];
        for hub in &self.dependencies {
            let hub = &format!("https://{}", hub);
            if hub == &self.domain {
                continue;
            }

            if self.recursion_guard.lock().await.contains(hub) {
                continue;
            }

            self.recursion_guard.lock().await.insert(hub);

            let hub = Hub::new(hub, self.http_client()).await;
            let mut hub = match hub {
                Ok(hub) => hub,
                Err(_) => continue,
            };
            hub.set_recursion_guard(self.recursion_guard.clone()).await;
            hubs.push(hub);
        }
        Ok(hubs)
    }
}
