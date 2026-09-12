use std::{path::PathBuf, sync::Arc};

use anyhow::{Result, anyhow};

use crate::hub_admin::Admin;
use flow_like::{
    flow_like_storage::files::store::{FlowLikeStore, local_store::LocalObjectStore},
    hub::Hub,
    state::{FlowLikeConfig, FlowLikeState},
    utils::http::HTTPClient,
};
use serde_json::Value;

const DEFAULT_HUB: &str = "api.flow-like.com";
const TOKEN_ENV: &str = "FLOW_LIKE_PAT";

pub struct Context {
    pub bit_dir: PathBuf,
    pub hub_domain: String,
    pub state: Arc<FlowLikeState>,
    token: Option<String>,
}

impl Context {
    pub fn new(store: Option<PathBuf>, hub: Option<String>, token: Option<String>) -> Result<Self> {
        let settings = load_settings();
        let bit_dir = store
            .or_else(|| settings_bit_dir(settings.as_ref()))
            .or_else(default_bit_dir)
            .ok_or_else(|| {
                anyhow!("could not determine the bit store directory, pass --store <DIR>")
            })?;
        let hub_domain = hub
            .or_else(|| settings_hub(settings.as_ref()))
            .unwrap_or_else(|| DEFAULT_HUB.to_string());

        let object_store = LocalObjectStore::new(bit_dir.clone()).map_err(|err| {
            anyhow!(
                "could not open the bit store at {}: {err}",
                bit_dir.display()
            )
        })?;
        let mut config = FlowLikeConfig::new();
        config.register_bits_store(FlowLikeStore::Local(Arc::new(object_store)));
        let state = FlowLikeState::new(config, HTTPClient::new_without_refetch());

        Ok(Self {
            bit_dir,
            hub_domain,
            state: Arc::new(state),
            token: token
                .or_else(|| std::env::var(TOKEN_ENV).ok())
                .filter(|token| !token.trim().is_empty()),
        })
    }

    /// Writing to the catalog needs a personal access token whose user holds
    /// the WriteBits permission.
    pub fn admin(&self) -> Result<Admin> {
        let token = self.token.clone().ok_or_else(|| {
            anyhow!(
                "no hub credential, pass --token or set {TOKEN_ENV} to a personal access token (pat_...)"
            )
        })?;
        Admin::new(&self.hub_domain, token)
    }

    pub async fn hub(&self) -> Result<Hub> {
        Hub::new(&self.hub_domain, self.state.http_client.clone())
            .await
            .map_err(|err| anyhow!("could not reach hub {}: {err}", self.hub_domain))
    }
}

fn settings_path() -> Option<PathBuf> {
    Some(
        dirs_next::cache_dir()?
            .join("flow-like")
            .join("global-settings.json"),
    )
}

fn load_settings() -> Option<Value> {
    serde_json::from_slice(&std::fs::read(settings_path()?).ok()?).ok()
}

fn settings_bit_dir(settings: Option<&Value>) -> Option<PathBuf> {
    let dir = settings?.get("bit_dir")?.as_str()?;
    (!dir.is_empty()).then(|| PathBuf::from(dir))
}

/// The hub the desktop app currently talks to: the active profile's, with the
/// installation default as the fallback for a profile that never picked one.
fn settings_hub(settings: Option<&Value>) -> Option<String> {
    let settings = settings?;
    let profile_hub = settings
        .get("current_profile")
        .and_then(Value::as_str)
        .and_then(|current| settings.get("profiles")?.get(current))
        .and_then(|profile| profile.get("hub_profile")?.get("hub")?.as_str())
        .filter(|hub| !hub.is_empty());

    profile_hub
        .or_else(|| {
            settings
                .get("default_hub")
                .and_then(Value::as_str)
                .filter(|hub| !hub.is_empty())
        })
        .map(str::to_string)
}

fn default_bit_dir() -> Option<PathBuf> {
    Some(dirs_next::data_dir()?.join("flow-like").join("bits"))
}
