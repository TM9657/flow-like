use flow_like_types::{Result, Value, reqwest::blocking::get};
use serde::Deserialize;
use std::{env, fs, path::Path};

#[derive(Deserialize)]
struct ApiConfig {
    authentication: Option<Authentication>,
    oauth_providers: Option<Value>,
}

#[derive(Deserialize)]
struct Authentication {
    openid: Option<OpenIdConfig>,
}

#[derive(Deserialize)]
struct OpenIdConfig {
    jwks_url: String,
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

    // Write OAuth providers config as-is (secrets resolved at runtime from env)
    let oauth_config_json = flow_like_types::json::to_string(
        &cfg.oauth_providers
            .unwrap_or(flow_like_types::json::json!({})),
    )?;
    let oauth_dest = Path::new(&out_dir).join("oauth_config.json");
    fs::write(&oauth_dest, oauth_config_json)?;

    Ok(())
}
