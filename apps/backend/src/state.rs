use axum::body::Body;
use hyper_util::{
    client::legacy::{connect::HttpConnector, Client},
    rt::TokioExecutor,
};
use jsonwebtoken::{jwk::JwkSet, Algorithm, Validation};
use sea_orm::{ConnectOptions, Database, DatabaseConnection};
use serde::{Deserialize, Serialize};
use std::{sync::Arc, time::Duration};

pub type AppState = Arc<State>;

#[derive(Debug)]
pub struct State {
    pub platform_config: PlatformConfig,
    pub db: DatabaseConnection,
    pub jwks: JwkSet,
    pub client: Client<HttpConnector, Body>,
}

impl State {
    pub async fn new() -> Self {
        let config_path = std::path::PathBuf::from("./hub.config.json");
        let config = std::fs::read_to_string(config_path).expect("Failed to read config file");
        let platform_config: PlatformConfig =
            serde_json::from_str(&config).expect("Failed to parse config file");

        let jwks = reqwest::get(
            &platform_config
                .authentication
                .openid
                .as_ref()
                .unwrap()
                .jwks_url,
        )
        .await
        .expect("Failed to fetch JWKS")
        .json()
        .await
        .expect("Failed to parse JWKS");

        let db_url = std::env::var("DATABASE_URL").expect("DATABASE_URL must be set");
        let mut opt = ConnectOptions::new(db_url.to_owned());
        let client: Client<HttpConnector, Body> =
            hyper_util::client::legacy::Client::<(), ()>::builder(TokioExecutor::new())
                .build(HttpConnector::new());
        opt.max_connections(100)
            .min_connections(5)
            .connect_timeout(Duration::from_secs(8))
            .sqlx_logging(platform_config.environment == Environment::Development);

        let db = Database::connect(opt)
            .await
            .expect("Failed to connect to database");

        Self {
            platform_config,
            db,
            client,
            jwks,
        }
    }
}

fn validation_from(alg: &str, issuer: &str) -> anyhow::Result<Validation> {
    let mut validation = match alg {
        "ES256" => Validation::new(Algorithm::ES256),
        "ES384" => Validation::new(Algorithm::ES384),
        "EdDSA" => Validation::new(Algorithm::EdDSA),
        "HS256" => Validation::new(Algorithm::HS256),
        "HS384" => Validation::new(Algorithm::HS384),
        "HS512" => Validation::new(Algorithm::HS512),
        "PS256" => Validation::new(Algorithm::PS256),
        "PS384" => Validation::new(Algorithm::PS384),
        "PS512" => Validation::new(Algorithm::PS512),
        "RS256" => Validation::new(Algorithm::RS256),
        "RS384" => Validation::new(Algorithm::RS384),
        "RS512" => Validation::new(Algorithm::RS512),
        _ => return Err(anyhow::anyhow!("Unsupported algorithm: {}", alg)),
    };
    validation.set_issuer(&[issuer]);
    Ok(validation)
}

#[derive(Debug, Serialize, Deserialize)]
pub struct PlatformConfig {
    pub authentication: Authentication,
    pub features: Features,
    pub hubs: Vec<String>, // Assuming hubs might contain strings, adjust if needed
    pub provider: String,
    pub domain: String,
    pub region: String,
    #[serde(rename = "termsOfService")]
    pub terms_of_service: String,
    #[serde(rename = "legalNotice")]
    pub legal_notice: String,
    #[serde(rename = "privacyPolicy")]
    pub privacy_policy: String,
    pub contact: Contact,
    #[serde(rename = "maxUsersPrototype")]
    pub max_users_prototype: i32,
    #[serde(rename = "defaultUserPlan")]
    pub default_user_plan: String,
    pub environment: Environment,
}

#[derive(Debug, Serialize, Deserialize, PartialEq)]
pub enum Environment {
    Development,
    Production,
    Staging,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Authentication {
    pub variant: String,
    #[serde(rename = "openid")]
    pub openid: Option<OpenIdConfig>,
    #[serde(rename = "oauth2")]
    pub oauth2: Option<OAuth2Config>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct OpenIdProxy {
    pub enabled: bool,
    pub authorize: Option<String>,
    pub token: Option<String>,
    pub userinfo: Option<String>,
    pub revoke: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct OpenIdConfig {
    pub authority: String,
    #[serde(rename = "clientId")]
    pub client_id: String,
    #[serde(rename = "discoveryUrl")]
    pub discovery_url: String,
    #[serde(rename = "jwksUrl")]
    pub jwks_url: String,
    pub proxy: OpenIdProxy,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct OAuth2Config {
    #[serde(rename = "authorizationEndpoint")]
    pub authorization_endpoint: String,
    #[serde(rename = "tokenEndpoint")]
    pub token_endpoint: String,
    #[serde(rename = "clientId")]
    pub client_id: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Features {
    #[serde(rename = "modelHosting")]
    pub model_hosting: bool,
    #[serde(rename = "flowHosting")]
    pub flow_hosting: bool,
    pub governance: bool,
    #[serde(rename = "aiAct")]
    pub ai_act: bool,
    #[serde(rename = "unauthorizedRead")]
    pub unauthorized_read: bool,
    #[serde(rename = "adminInterface")]
    pub admin_interface: bool,
    pub premium: bool,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Contact {
    pub name: String,
    pub email: String,
    pub url: String,
}
