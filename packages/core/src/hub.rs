use std::{collections::HashMap, sync::Arc};

use crate::{
    bit::{Bit, BitTypes},
    credentials::SharedCredentials,
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
    pub max_llm_calls: Option<i32>,
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

    pub async fn shared_credentials(&self, token: &str, app_id: &str) -> Result<SharedCredentials> {
        use std::time::{Instant, SystemTime, UNIX_EPOCH};

        // Inline helper: quick timestamp for logs
        let now_str = || {
            let dur = SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap_or_default();
            format!("{}.{:03}", dur.as_secs(), dur.subsec_millis())
        };

        println!("🔑 [{}][shared_credentials] ========== START ==========", now_str());
        println!("🔑 [{}][shared_credentials] Input app_id: {}", now_str(), app_id);
        println!("🔑 [{}][shared_credentials] Input token length: {} chars", now_str(), token.len());
        println!("🔑 [{}][shared_credentials] Token first 10 chars: {}", now_str(), &token.chars().take(10).collect::<String>());
        println!("🔑 [{}][shared_credentials] Token starts_with 'pat_': {}", now_str(), token.starts_with("pat_"));
        println!("🔑 [{}][shared_credentials] Token starts_with 'Bearer ': {}", now_str(), token.starts_with("Bearer "));
        println!("🔑 [{}][shared_credentials] Hub domain: {}", now_str(), self.domain);

        // Build URL
        let presign_path = format!("api/v1/apps/{}/invoke/presign", app_id);
        println!("🔑 [{}][shared_credentials] Constructing URL with path: {}", now_str(), presign_path);

        let presign_url = match self.construct_url(&presign_path) {
            Ok(u) => {
                println!("✅ [{}][shared_credentials] presign_url = {}", now_str(), u);
                u
            }
            Err(e) => {
                println!(
                    "❌ [{}][shared_credentials][ERROR] construct_url failed: {:?}",
                    now_str(),
                    e
                );
                return Err(e);
            }
        };

        // Normalize Authorization header
        let auth_val = if token.starts_with("pat_") {
            println!("🔑 [{}][shared_credentials] Token detected as PAT (starts with 'pat_')", now_str());
            token.to_string()
        } else if token.starts_with("Bearer ") {
            println!("🔑 [{}][shared_credentials] Token already has 'Bearer ' prefix", now_str());
            token.to_string()
        } else {
            println!("🔑 [{}][shared_credentials] Token needs 'Bearer ' prefix", now_str());
            format!("Bearer {}", token)
        };

        println!("🔑 [{}][shared_credentials] Final Authorization header = {}", now_str(), auth_val);
        println!("🔑 [{}][shared_credentials] Auth header length: {}", now_str(), auth_val.len());

        // Client
        println!("🌐 [{}][shared_credentials] Getting HTTP client...", now_str());
        let client = self.http_client().client();
        println!("✅ [{}][shared_credentials] HTTP client ready", now_str());

        // Build request
        let build_start = Instant::now();
        println!("📦 [{}][shared_credentials] Building request...", now_str());
        let request = match client
            .get(presign_url.clone())
            .header("Authorization", &auth_val)
            .build()
        {
            Ok(rq) => {
                println!(
                    "✅ [{}][shared_credentials] Built request in {:?}",
                    now_str(),
                    build_start.elapsed()
                );
                println!("📦 [{}][shared_credentials]   Method: {}", now_str(), rq.method());
                println!("📦 [{}][shared_credentials]   URL: {}", now_str(), rq.url());

                // Dump all request headers
                println!("📦 [{}][shared_credentials] Request headers:", now_str());
                for (k, v) in rq.headers().iter() {
                    let value_str = v.to_str().unwrap_or("<non-utf8>");
                    // Mask sensitive headers partially
                    if k.as_str().to_lowercase() == "authorization" {
                        println!("  ✅ {}: {} (length: {})", k,
                            if value_str.len() > 20 {
                                format!("{}...{}", &value_str[..10], &value_str[value_str.len()-5..])
                            } else {
                                value_str.to_string()
                            },
                            value_str.len()
                        );
                    } else {
                        println!("  - {}: {}", k, value_str);
                    }
                }
                rq
            }
            Err(e) => {
                println!(
                    "❌ [{}][shared_credentials][ERROR] Request build error: {:?}",
                    now_str(),
                    e
                );
                return Err(flow_like_types::Error::from(e));
            }
        };

        // Execute
        let exec_start = Instant::now();
        println!("🚀 [{}][shared_credentials] Executing HTTP request...", now_str());
        let resp = match client.execute(request).await {
            Ok(rp) => {
                println!(
                    "✅ [{}][shared_credentials] Request executed successfully in {:?}",
                    now_str(),
                    exec_start.elapsed()
                );
                rp
            }
            Err(e) => {
                println!(
                    "❌ [{}][shared_credentials][ERROR] Execute/network error: {:?}",
                    now_str(),
                    e
                );
                println!("❌ [{}][shared_credentials][ERROR] Error details:", now_str());
                println!("     - Is timeout: {}", e.is_timeout());
                println!("     - Is connect: {}", e.is_connect());
                println!("     - Is request: {}", e.is_request());
                if let Some(url) = e.url() {
                    println!("     - URL: {}", url);
                }
                return Err(flow_like_types::Error::from(e));
            }
        };

        // Status + headers
        let status = resp.status();
        let status_code = status.as_u16();
        println!("📥 [{}][shared_credentials] Response received:", now_str());
        println!("    Status: {} ({})", status, status_code);
        println!("    Is Success: {}", status.is_success());
        println!("    Is Client Error: {}", status.is_client_error());
        println!("    Is Server Error: {}", status.is_server_error());

        println!("📥 [{}][shared_credentials] Response headers:", now_str());
        for (k, v) in resp.headers().iter() {
            println!("  - {}: {}", k, v.to_str().unwrap_or("<non-utf8>"));
        }

        // Read body as text (always), print it, then parse JSON from the text
        // (This consumes the body once; fine since we parse from the captured text.)
        let body_start = Instant::now();
        println!("📄 [{}][shared_credentials] Reading response body...", now_str());
        let body_text = match resp.text().await {
            Ok(t) => {
                println!(
                    "✅ [{}][shared_credentials] Received body in {:?} ({} bytes)",
                    now_str(),
                    body_start.elapsed(),
                    t.len()
                );
                println!(
                    "📄 ========== RESPONSE BODY BEGIN ==========\n{}\n========== RESPONSE BODY END ==========",
                    t
                );
                t
            }
            Err(e) => {
                println!(
                    "❌ [{}][shared_credentials][ERROR] Failed reading body text: {:?}",
                    now_str(),
                    e
                );
                return Err(flow_like_types::Error::from(e));
            }
        };

        // Check status BEFORE parsing (early exit on error)
        if !status.is_success() {
            println!(
                "❌ [{}][shared_credentials][ERROR] Non-success status: {}",
                now_str(),
                status
            );
            println!("❌ [{}][shared_credentials][ERROR] Response body (above) indicates failure", now_str());

            // Try to parse error details if it's JSON
            if let Ok(error_json) = flow_like_types::json::from_str::<flow_like_types::Value>(&body_text) {
                println!("❌ [{}][shared_credentials][ERROR] Parsed error JSON: {:#?}", now_str(), error_json);
            }

            return Err(flow_like_types::Error::msg(format!(
                "presign failed: status={} body={}",
                status,
                body_text
            )));
        }

        // Parse JSON
        let parse_start = Instant::now();
        println!("🔍 [{}][shared_credentials] Parsing JSON response...", now_str());
        let shared_credentials: SharedCredentials = match flow_like_types::json::from_str(
            &body_text,
        ) {
            Ok(val) => {
                println!(
                    "✅ [{}][shared_credentials] JSON parsed successfully in {:?}",
                    now_str(),
                    parse_start.elapsed()
                );
                println!("✅ [{}][shared_credentials] SharedCredentials fields present:", now_str());
                // Add field validation if SharedCredentials has known fields
                val
            }
            Err(e) => {
                println!(
                    "❌ [{}][shared_credentials][ERROR] JSON parse error: {:?}",
                    now_str(),
                    e
                );
                println!("❌ [{}][shared_credentials][ERROR] Expected SharedCredentials format", now_str());
                println!("❌ [{}][shared_credentials][ERROR] Raw body was printed above", now_str());
                return Err(flow_like_types::Error::msg(format!("JSON parse error: {}", e)));
            }
        };

        println!("🎉 [{}][shared_credentials] ========== SUCCESS - DONE ==========", now_str());
        Ok(shared_credentials)
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
