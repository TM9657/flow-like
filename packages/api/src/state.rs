use aws_config::SdkConfig;
use axum::body::Body;
use flow_like::app::App;
use flow_like::flow::board::Board;
use flow_like::flow::node::NodeLogic;
use flow_like::flow_like_model_provider::provider::{ModelProviderConfiguration, OpenAIConfig};
use flow_like::flow_like_storage::Path;
use flow_like::flow_like_storage::files::store::FlowLikeStore;
use flow_like::hub::{Environment, Hub};
use flow_like::state::{FlowLikeConfig, FlowLikeState, FlowNodeRegistryInner};
use flow_like::utils::http::HTTPClient;
use flow_like_types::bail;
use flow_like_types::{Result, Value};
use hyper_util::{
    client::legacy::{Client, connect::HttpConnector},
    rt::TokioExecutor,
};
use jsonwebtoken::{
    DecodingKey, Validation, decode,
    jwk::{AlgorithmParameters, JwkSet},
};
use sea_orm::{ConnectOptions, Database, DatabaseConnection};
use std::{collections::HashMap, sync::Arc, time::Duration};

use crate::credentials::{CredentialsAccess, RuntimeCredentials};
use crate::entity::role;

pub type AppState = Arc<State>;

const CONFIG: &str = include_str!("../../../flow-like.config.json");
const JWKS: &str = include_str!(concat!(env!("OUT_DIR"), "/jwks.json"));

pub struct State {
    pub platform_config: Hub,
    pub db: DatabaseConnection,
    pub jwks: JwkSet,
    pub client: Client<HttpConnector, Body>,
    pub stripe_client: Option<stripe::Client>,
    #[cfg(feature = "aws")]
    pub aws_client: Arc<SdkConfig>,
    pub catalog: Arc<Vec<Arc<dyn NodeLogic>>>,
    pub registry: Arc<FlowNodeRegistryInner>,
    pub provider: Arc<ModelProviderConfiguration>,
    pub permission_cache: moka::sync::Cache<String, Arc<role::Model>>,
    pub credentials_cache: moka::sync::Cache<String, Arc<RuntimeCredentials>>,
    pub state_cache: moka::sync::Cache<String, Arc<FlowLikeState>>,
    pub cdn_bucket: Arc<FlowLikeStore>,
    pub response_cache: moka::sync::Cache<String, Value>,
}

impl State {
    pub async fn new(
        catalog: Arc<Vec<Arc<dyn NodeLogic>>>,
        cdn_bucket: Arc<FlowLikeStore>,
    ) -> Self {
        let platform_config: Hub =
            serde_json::from_str(CONFIG).expect("Failed to parse config file");

        let jwks = flow_like_types::json::from_str::<JwkSet>(JWKS).expect("Failed to parse JWKS");

        let db_url = std::env::var("DATABASE_URL").expect("DATABASE_URL must be set");
        let mut opt = ConnectOptions::new(db_url.to_owned());
        let client: Client<HttpConnector, Body> =
            hyper_util::client::legacy::Client::<(), ()>::builder(TokioExecutor::new())
                .build(HttpConnector::new());
        opt.max_connections(10)
            .min_connections(1)
            .connect_timeout(Duration::from_secs(8))
            .sqlx_logging(platform_config.environment == Environment::Development);

        let db = Database::connect(opt)
            .await
            .expect("Failed to connect to database");

        let stripe_client = if platform_config.features.premium {
            let stripe_key =
                std::env::var("STRIPE_SECRET_KEY").expect("STRIPE_SECRET_KEY must be set");
            let stripe_client = stripe::Client::new(stripe_key);
            Some(stripe_client)
        } else {
            None
        };

        let mut provider = ModelProviderConfiguration::default();

        let openai_endpoint = std::env::var("OPENAI_ENDPOINT").ok();
        let openai_key = std::env::var("OPENAI_API_KEY").ok();

        if let (Some(endpoint), Some(key)) = (openai_endpoint, openai_key) {
            provider.openai_config.push(OpenAIConfig {
                endpoint: Some(endpoint),
                api_key: Some(key),
                organization: None,
                proxy: None,
            })
        }

        let config = FlowLikeConfig::new();
        let (http_client, _) = HTTPClient::new();
        let flow_like_state = FlowLikeState::new(config, http_client);

        let registry = FlowNodeRegistryInner::prepare(&flow_like_state, &catalog).await;

        let cache = moka::sync::Cache::builder()
            .max_capacity(32 * 1024 * 1024) // 32 MB
            .time_to_live(Duration::from_secs(30 * 60)) // 30 minutes
            .build();

        let response_cache = moka::sync::Cache::builder()
            .max_capacity(64 * 1024 * 1024) // 32 MB
            .time_to_live(Duration::from_secs(60)) // 30 minutes
            .build();

        Self {
            platform_config,
            db,
            client,
            jwks,
            stripe_client,
            #[cfg(feature = "aws")]
            aws_client: Arc::new(aws_config::load_from_env().await),
            catalog,
            provider: Arc::new(provider),
            registry: Arc::new(registry),
            permission_cache: moka::sync::Cache::builder()
                .max_capacity(32 * 1024 * 1024)
                .time_to_live(Duration::from_secs(120))
                .build(),
            state_cache: moka::sync::Cache::builder()
                .max_capacity(32 * 1024 * 1024) // 32 MB
                .time_to_live(Duration::from_secs(30 * 60))
                .build(),
            credentials_cache: cache,
            cdn_bucket,
            response_cache,
        }
    }

    pub fn validate_token(&self, token: &str) -> Result<HashMap<String, Value>> {
        let header = jsonwebtoken::decode_header(token)?;
        let Some(kid) = header.kid else {
            return Err(flow_like_types::anyhow!("Missing kid in token header"));
        };
        let Some(jwk) = self.jwks.find(&kid) else {
            return Err(flow_like_types::anyhow!("JWK not found for kid: {}", kid));
        };
        let alg = decoding_key_for_algorithm(&jwk.algorithm)?;
        let mut validation = Validation::new(header.alg);
        validation.validate_aud = false;
        let decoded = decode::<HashMap<String, Value>>(token, &alg, &validation)?;
        let claims = decoded.claims;
        Ok(claims)
    }

    #[tracing::instrument(
        name = "scoped_credentials",
        skip(self),
        fields(sub, app_id, board_id, version)
    )]
    pub async fn scoped_credentials(
        &self,
        sub: &str,
        app_id: &str,
        mode: CredentialsAccess,
    ) -> flow_like_types::Result<Arc<RuntimeCredentials>> {
        let key = format!("{}:{}:{}", sub, app_id, mode);
        if let Some(credentials) = self.credentials_cache.get(&key) {
            return Ok(credentials);
        }
        let credentials = RuntimeCredentials::scoped(sub, app_id, self, mode).await?;
        self.credentials_cache
            .insert(key, Arc::new(credentials.clone()));
        Ok(Arc::new(credentials))
    }

    #[tracing::instrument(
        name = "scoped_app",
        skip(self, state),
        fields(sub, app_id, board_id, version)
    )]
    pub async fn scoped_app(
        &self,
        sub: &str,
        app_id: &str,
        state: &AppState,
        mode: CredentialsAccess,
    ) -> flow_like_types::Result<App> {
        let credentials = self.scoped_credentials(sub, app_id, mode).await?;
        let app_state = Arc::new(credentials.to_state(state.clone()).await?);

        let app = App::load(app_id.to_string(), app_state.clone()).await?;

        Ok(app)
    }

    #[tracing::instrument(
        name = "master_app",
        skip(self, state),
        fields(sub, app_id, board_id, version)
    )]
    pub async fn master_app(
        &self,
        _sub: &str,
        app_id: &str,
        state: &AppState,
    ) -> flow_like_types::Result<App> {
        let credentials = self.master_credentials().await?;

        let app_state = self.state_cache.get("master");

        let app_state = match app_state {
            Some(state) => state,
            None => {
                let state = Arc::new(credentials.to_state(state.clone()).await?);
                self.state_cache.insert("master".to_string(), state.clone());
                state
            }
        };

        let app = App::load(app_id.to_string(), app_state.clone()).await?;

        Ok(app)
    }

    #[tracing::instrument(
        name = "scoped_board",
        skip(self, state),
        level = "debug",
        fields(sub, app_id, board_id, version)
    )]
    pub async fn scoped_board(
        &self,
        sub: &str,
        app_id: &str,
        board_id: &str,
        state: &AppState,
        version: Option<(u32, u32, u32)>,
        mode: CredentialsAccess,
    ) -> flow_like_types::Result<Board> {
        let credentials = self.scoped_credentials(sub, app_id, mode).await?;
        let app_state = Arc::new(credentials.to_state(state.clone()).await?);
        let storage_root = Path::from("apps").child(app_id.to_string());
        let board = Board::load(storage_root, board_id, app_state, version).await?;
        Ok(board)
    }

    #[tracing::instrument(
        name = "master_board",
        skip(self, state),
        level = "debug",
        fields(sub, app_id, board_id, version)
    )]
    pub async fn master_board(
        &self,
        _sub: &str,
        app_id: &str,
        board_id: &str,
        state: &AppState,
        version: Option<(u32, u32, u32)>,
    ) -> flow_like_types::Result<Board> {
        let credentials = self.master_credentials().await?;

        let app_state = self.state_cache.get("master");

        let app_state = match app_state {
            Some(state) => state,
            None => {
                let state = Arc::new(credentials.to_state(state.clone()).await?);
                self.state_cache.insert("master".to_string(), state.clone());
                state
            }
        };

        let storage_root = Path::from("apps").child(app_id.to_string());
        let board = Board::load(storage_root, board_id, app_state, version).await?;

        Ok(board)
    }

    pub async fn scoped_template(
        &self,
        sub: &str,
        app_id: &str,
        template_id: &str,
        state: &AppState,
        version: Option<(u32, u32, u32)>,
        mode: CredentialsAccess,
    ) -> flow_like_types::Result<Board> {
        let credentials = self.scoped_credentials(sub, app_id, mode).await?;
        let app_state = Arc::new(credentials.to_state(state.clone()).await?);

        let storage_root = Path::from("apps").child(app_id.to_string());

        let board = Board::load_template(storage_root, template_id, app_state, version).await?;

        Ok(board)
    }

    pub async fn master_credentials(&self) -> flow_like_types::Result<Arc<RuntimeCredentials>> {
        let credentials = self.credentials_cache.get("master");
        if let Some(credentials) = credentials {
            return Ok(credentials);
        }
        let credentials = Arc::new(RuntimeCredentials::master_credentials().await?);
        self.credentials_cache
            .insert("master".to_string(), credentials.clone());
        Ok(credentials)
    }

    pub fn check_permission(&self, sub: &str, app_id: &str) -> Option<Arc<role::Model>> {
        let key = format!("{}:{}", sub, app_id);
        self.permission_cache.get(&key)
    }

    pub fn put_permission(&self, sub: &str, app_id: &str, role: Arc<role::Model>) {
        let key = format!("{}:{}", sub, app_id);
        self.permission_cache.insert(key, role);
    }

    pub fn invalidate_permission(&self, sub: &str, app_id: &str) {
        let key = format!("{}:{}", sub, app_id);
        self.permission_cache.invalidate(&key);
    }

    pub fn get_cache<T>(&self, key: &str) -> Option<T>
    where
        T: serde::de::DeserializeOwned,
    {
        self.response_cache
            .get(key)
            .and_then(|json_value| serde_json::from_value(json_value).ok())
    }

    pub fn set_cache<T>(&self, key: String, value: T)
    where
        T: serde::Serialize,
    {
        if let Ok(json_value) = serde_json::to_value(value) {
            self.response_cache.insert(key, json_value);
        }
    }

    pub fn invalidate_cache(&self, key: &str) {
        self.response_cache.invalidate(key);
    }
}

fn decoding_key_for_algorithm(alg: &AlgorithmParameters) -> flow_like_types::Result<DecodingKey> {
    let key = match alg {
        AlgorithmParameters::RSA(rsa) => DecodingKey::from_rsa_components(&rsa.n, &rsa.e),
        AlgorithmParameters::EllipticCurve(ec) => DecodingKey::from_ec_components(&ec.x, &ec.y),
        AlgorithmParameters::OctetKeyPair(octet) => DecodingKey::from_ed_components(&octet.x),
        _ => bail!("Unsupported algorithm"),
    }?;
    Ok(key)
}
