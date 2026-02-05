use std::fs;

fn main() {
    // Link against system zlib on iOS (needed by flate2 with zlib feature)
    let target_os = std::env::var("CARGO_CFG_TARGET_OS").unwrap_or_default();
    if target_os == "ios" {
        println!("cargo:rustc-link-lib=z");
    }

    println!("cargo:rerun-if-changed=../../../flow-like.config.json");
    println!("cargo:rerun-if-changed=../../../flow-like.config.prod.json");

    let cfg_str = fs::read_to_string("../../../flow-like.config.json").unwrap_or_default();

    if let Ok(config) = serde_json::from_str::<serde_json::Value>(&cfg_str) {
        if let Some(domain) = config.get("domain").and_then(|d| d.as_str()) {
            println!("cargo:rustc-env=FLOW_LIKE_CONFIG_DOMAIN={}", domain);
        }
        let secure = config
            .get("secure")
            .and_then(|s| s.as_bool())
            .unwrap_or(true);
        println!("cargo:rustc-env=FLOW_LIKE_CONFIG_SECURE={}", secure);
    }

    tauri_build::build()
}
