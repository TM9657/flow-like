use std::{collections::HashMap, sync::Arc};

use crate::{
    bit::{Bit, BitTypes},
    profile::Profile,
    utils::{http::HTTPClient, recursion::RecursionGuard},
};
use flow_like_types::{Result, sync::Mutex};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use url::Url;

#[derive(Clone, Debug, Serialize, JsonSchema, Deserialize)]
pub struct UserTier {
    pub max_non_visible_projects: i32,
    pub max_remote_executions: i32,
    pub execution_tier: String,
    pub max_total_size: i64,
    pub max_llm_cost: i32,
    pub llm_tiers: Vec<String>,
}

pub type UserTiers = HashMap<String, UserTier>;

#[derive(Clone, Debug, Serialize, JsonSchema, Deserialize)]
pub struct Hub {
    pub name: String,
    pub description: String,
    pub thumbnail: Option<String>,
    pub icon: Option<String>,
    pub authentication: Option<Authentication>,
    pub features: Features,
    pub hubs: Vec<String>, // Assuming hubs might contain strings, adjust if needed
    pub provider: Option<String>,
    pub domain: String,
    pub region: Option<String>,
    pub terms_of_service: String,
    pub cdn: Option<String>,
    pub app: Option<String>,
    pub legal_notice: String,
    pub privacy_policy: String,
    pub contact: Contact,
    pub max_users_prototype: Option<i32>,
    pub default_user_plan: Option<String>,
    pub environment: Environment,
    pub tiers: UserTiers,
    #[serde(default)]
    pub lookup: Lookup,

    #[serde(skip)]
    recursion_guard: Option<Arc<Mutex<RecursionGuard>>>,

    #[serde(skip)]
    http_client: Option<Arc<HTTPClient>>,
}

#[derive(Clone, Debug, Serialize, JsonSchema, Deserialize, PartialEq)]
pub enum Environment {
    Development,
    Production,
    Staging,
}

#[derive(Clone, Debug, Serialize, JsonSchema, Deserialize)]
pub struct Authentication {
    pub variant: String,
    pub openid: Option<OpenIdConfig>,
    pub oauth2: Option<OAuth2Config>,
}

#[derive(Clone, Debug, Serialize, JsonSchema, Deserialize)]
pub struct Lookup {
    pub email: bool,
    pub name: bool,
    pub username: bool,
    pub preferred_username: bool,
    pub avatar: bool,
    pub additional_information: bool,
    pub description: bool,
    pub created_at: bool,
}

impl Default for Lookup {
    fn default() -> Self {
        Self {
            email: false,
            username: false,
            name: true,
            preferred_username: true,
            avatar: true,
            additional_information: true,
            description: true,
            created_at: true,
        }
    }
}

#[derive(Clone, Debug, Serialize, JsonSchema, Deserialize)]
pub struct OpenIdProxy {
    pub enabled: bool,
    pub authorize: Option<String>,
    pub token: Option<String>,
    pub userinfo: Option<String>,
    pub revoke: Option<String>,
}

#[derive(Clone, Debug, Serialize, JsonSchema, Deserialize)]
pub struct CognitoConfig {
    pub user_pool_id: String,
}

#[derive(Clone, Debug, Serialize, JsonSchema, Deserialize)]
pub struct OpenIdConfig {
    pub authority: Option<String>,
    pub client_id: Option<String>,
    pub redirect_uri: Option<String>,
    pub post_logout_redirect_uri: Option<String>,
    pub response_type: Option<String>,
    pub scope: Option<String>,
    pub discovery_url: Option<String>,
    pub user_info_url: Option<String>,
    pub jwks_url: String,
    pub proxy: Option<OpenIdProxy>,
    pub cognito: Option<CognitoConfig>,
}

#[derive(Clone, Debug, Serialize, JsonSchema, Deserialize)]
pub struct OAuth2Config {
    pub authorization_endpoint: String,
    pub token_endpoint: String,
    pub client_id: String,
}

#[derive(Debug, Serialize, Deserialize, JsonSchema, Clone)]
pub struct Features {
    pub model_hosting: bool,
    pub flow_hosting: bool,
    pub governance: bool,
    pub ai_act: bool,
    pub unauthorized_read: bool,
    pub admin_interface: bool,
    pub premium: bool,
}

#[derive(Debug, Serialize, Deserialize, JsonSchema, Clone)]
pub struct Contact {
    pub name: String,
    pub email: String,
    pub url: String,
}

#[derive(Debug, Serialize, Deserialize, JsonSchema, Clone)]
pub struct BitSearchQuery {
    pub search: Option<String>,
    pub limit: Option<u64>,
    pub offset: Option<u64>,
    pub bit_types: Option<Vec<BitTypes>>,
}

impl BitSearchQuery {
    pub fn builder() -> Self {
        Self {
            search: None,
            limit: None,
            offset: None,
            bit_types: None,
        }
    }

    pub fn with_search(mut self, search: &str) -> Self {
        self.search = Some(search.to_string());
        self
    }

    pub fn with_limit(mut self, limit: u64) -> Self {
        self.limit = Some(limit);
        self
    }

    pub fn with_offset(mut self, offset: u64) -> Self {
        self.offset = Some(offset);
        self
    }

    pub fn with_bit_types(mut self, bit_types: Vec<BitTypes>) -> Self {
        self.bit_types = Some(bit_types);
        self
    }

    pub fn build(self) -> Self {
        self
    }
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

        if !url.ends_with('/') {
            url.push('/');
        }

        let url = match Url::parse(&url) {
            Ok(url) => url,
            Err(e) => {
                println!("Error parsing URL: {:?}", e);
                return Err(flow_like_types::Error::msg("Invalid URL"));
            }
        };

        // TODO Cache this.
        // We should implement a global Cache anyways, best with support for reqwest
        let hub_info_url = url.join("api/v1")?;
        let request = http_client.client().get(hub_info_url.clone()).build()?;
        let mut info: Hub = http_client.hashed_request(request).await?;
        info.recursion_guard = Some(RecursionGuard::new(vec![url.as_ref()]));
        info.http_client = Some(http_client);
        Ok(info)
    }

    fn construct_url(&self, path: &str) -> Result<Url> {
        let mut url = if !self.domain.starts_with("https://") {
            format!("https://{}", self.domain)
        } else {
            self.domain.clone()
        };

        if !url.ends_with("/") {
            url.push('/');
        }

        if path.starts_with('/') {
            url.push_str(&path[1..]);
        } else {
            url.push_str(path);
        }
        let url = Url::parse(&url)
            .map_err(|e| flow_like_types::Error::msg(format!("Invalid URL: {}", e)))?;

        Ok(url)
    }

    pub async fn get_bit(&self, bit_id: &str) -> Result<Bit> {
        let bit_url = self.construct_url(&format!("api/v1/bit/{}", bit_id))?;
        let request = self.http_client().client().get(bit_url).build()?;
        let bit = self.http_client().hashed_request::<Bit>(request).await;
        if let Ok(bit) = bit {
            return Ok(bit);
        }

        let dependency_hubs = self.get_dependency_hubs().await?;
        for hub in dependency_hubs {
            let bit = Box::pin(hub.get_bit(bit_id)).await;
            match bit {
                Ok(bit) => return Ok(bit),
                Err(_) => continue,
            }
        }

        Err(flow_like_types::Error::msg("Bit not found"))
    }

    pub async fn set_recursion_guard(&mut self, guard: Arc<Mutex<RecursionGuard>>) {
        self.recursion_guard = Some(guard);
        if let Some(ref guard) = self.recursion_guard {
            guard.lock().await.insert(&self.domain);
        }
    }

    pub async fn search_bit(&self, query: &BitSearchQuery) -> Result<Vec<Bit>> {
        let type_bits_url = self.construct_url("api/v1/bit")?;

        let request = self
            .http_client()
            .client()
            .post(type_bits_url)
            .json(query)
            .build()?;
        let mut bits = self
            .http_client()
            .hashed_request::<Vec<Bit>>(request)
            .await?;
        let dependency_hubs = self.get_dependency_hubs().await?;

        for hub in dependency_hubs {
            let hub_models = Box::pin(hub.search_bit(query)).await?;
            bits.extend(hub_models);
        }

        Ok(bits)
    }

    pub async fn get_bit_dependencies(&self, bit_id: &str) -> Result<Vec<Bit>> {
        let dependencies_url =
            self.construct_url(&format!("api/v1/bit/{}/dependencies", bit_id))?;
        let request = self.http_client().client().get(dependencies_url).build()?;
        let bits = self
            .http_client()
            .hashed_request::<Vec<Bit>>(request)
            .await?;

        Ok(bits)
    }

    pub async fn get_profiles(&self) -> Result<Vec<Profile>> {
        let profiles_url = self.construct_url("api/v1/info/profiles")?;
        println!("Requesting profiles from: {}", profiles_url);
        let request = self.http_client().client().get(profiles_url).build()?;
        println!("Request: {:?}", request);
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
        let recursion_guard = if let Some(guard) = &self.recursion_guard {
            guard.clone()
        } else {
            RecursionGuard::new(vec![&self.domain])
        };

        let mut hubs = vec![];
        for hub in &self.hubs {
            let guard = recursion_guard.clone();
            let mut guard = guard.lock().await;

            if hub == &self.domain {
                continue;
            }

            if guard.contains(hub) {
                continue;
            }

            guard.insert(hub);
            drop(guard);

            let hub = Hub::new(hub, self.http_client()).await;
            let mut hub = match hub {
                Ok(hub) => hub,
                Err(_) => continue,
            };
            hub.set_recursion_guard(recursion_guard.clone()).await;
            hubs.push(hub);
        }
        Ok(hubs)
    }
}
