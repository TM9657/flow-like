use flow_like_types::{Result, reqwest::blocking::get};
use serde::{Deserialize, Serialize};
use std::{collections::HashMap, env, fs, path::Path};

#[derive(Deserialize)]
struct ApiConfig {
    authentication: Option<Authentication>,
    oauth_providers: Option<HashMap<String, OAuthProviderConfig>>,
}

#[derive(Deserialize)]
struct Authentication {
    openid: Option<OpenIdConfig>,
}

#[derive(Deserialize)]
struct OpenIdConfig {
    jwks_url: String,
}

#[derive(Deserialize, Clone)]
struct OAuthProviderConfig {
    name: String,
    #[serde(default)]
    client_id: String,
    client_secret_env: Option<String>,
    auth_url: String,
    token_url: String,
    #[serde(default)]
    scopes: Vec<String>,
    #[serde(default)]
    pkce_required: bool,
    #[serde(default)]
    requires_secret_proxy: bool,
    revoke_url: Option<String>,
    userinfo_url: Option<String>,
    device_auth_url: Option<String>,
    #[serde(default)]
    use_device_flow: bool,
    audience: Option<String>,
}

/// Resolved OAuth config with secrets baked in at build time
#[derive(Serialize)]
struct ResolvedOAuthConfig {
    name: String,
    client_id: String,
    client_secret: Option<String>,
    auth_url: String,
    token_url: String,
    scopes: Vec<String>,
    pkce_required: bool,
    requires_secret_proxy: bool,
    revoke_url: Option<String>,
    userinfo_url: Option<String>,
    device_auth_url: Option<String>,
    use_device_flow: bool,
    audience: Option<String>,
}

fn main() -> Result<()> {
    // make sure we rerun if config changes
    println!("cargo:rerun-if-changed=../../flow-like.config.json");

    // load and parse
    let cfg_str = fs::read_to_string("../../flow-like.config.json")?;
    let cfg: ApiConfig = flow_like_types::json::from_str(&cfg_str)?;
    let jwks_url = cfg
        .authentication
        .and_then(|a| a.openid)
        .map(|o| o.jwks_url)
        .expect("jwks_url must be set in flow-like.config.json");

    // fetch JWKS
    let resp = get(&jwks_url)?.error_for_status()?;
    let jwks_body = resp.text()?;

    // write to OUT_DIR
    let out_dir = env::var("OUT_DIR")?;
    let dest = Path::new(&out_dir).join("jwks.json");
    fs::write(&dest, jwks_body)?;

    // Process OAuth providers and resolve secrets from environment variables
    let mut resolved_configs: HashMap<String, ResolvedOAuthConfig> = HashMap::new();
    if let Some(providers) = cfg.oauth_providers {
        for (provider_id, provider_cfg) in providers {
            // Resolve client_secret from env var
            let client_secret = if let Some(secret_env) = &provider_cfg.client_secret_env {
                println!("cargo:rerun-if-env-changed={}", secret_env);
                env::var(secret_env).ok().filter(|s| !s.is_empty())
            } else {
                None
            };

            resolved_configs.insert(
                provider_id,
                ResolvedOAuthConfig {
                    name: provider_cfg.name,
                    client_id: provider_cfg.client_id,
                    client_secret,
                    auth_url: provider_cfg.auth_url,
                    token_url: provider_cfg.token_url,
                    scopes: provider_cfg.scopes,
                    pkce_required: provider_cfg.pkce_required,
                    requires_secret_proxy: provider_cfg.requires_secret_proxy,
                    revoke_url: provider_cfg.revoke_url,
                    userinfo_url: provider_cfg.userinfo_url,
                    device_auth_url: provider_cfg.device_auth_url,
                    use_device_flow: provider_cfg.use_device_flow,
                    audience: provider_cfg.audience,
                },
            );
        }
    }

    // Write resolved OAuth configs as a JSON file for runtime use
    let oauth_config_json = flow_like_types::json::to_string(&resolved_configs)?;
    let oauth_dest = Path::new(&out_dir).join("oauth_config.json");
    fs::write(&oauth_dest, oauth_config_json)?;

    Ok(())
}
