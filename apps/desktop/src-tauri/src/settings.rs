use crate::profile::UserProfile;
use flow_like::{state::FlowLikeConfig, utils::cache::get_cache_dir};
use serde::{Deserialize, Serialize};
use std::{collections::HashMap, path::PathBuf, sync::Arc, time::SystemTime};
use tauri::AppHandle;

// iOS-only centralized, sandbox-safe roots.
#[cfg(target_os = "ios")]
fn app_data_root() -> PathBuf {
    if let Some(dir) = dirs_next::data_dir() {
        dir.join("flow-like")
    } else if let Some(dir) = dirs_next::cache_dir() {
        dir.join("flow-like")
    } else {
        // Relative fallback inside sandboxed working directory
        PathBuf::from("flow-like")
    }
}

#[cfg(target_os = "ios")]
fn app_cache_root() -> PathBuf {
    if let Some(dir) = dirs_next::cache_dir() {
        dir.join("flow-like")
    } else if let Some(dir) = dirs_next::data_dir() {
        dir.join("flow-like").join("cache")
    } else {
        // Relative fallback inside sandboxed working directory
        PathBuf::from("flow-like").join("cache")
    }
}

// Single source of truth for iOS storage root. We pick cache root based on runtime observations
// (cache survived updates more reliably in your environment). All app data is placed under this.
#[cfg(target_os = "ios")]
fn ios_storage_root() -> PathBuf {
    app_data_root()
}

#[cfg(target_os = "ios")]
fn default_logs_dir() -> PathBuf {
    // Keep logs under the chosen iOS storage root (cache)
    ios_storage_root().join("logs")
}

#[cfg(not(target_os = "ios"))]
fn default_logs_dir() -> PathBuf {
    dirs_next::data_dir()
        .unwrap_or_default()
        .join("flow-like")
        .join("logs")
}

#[cfg(target_os = "ios")]
fn default_temporary_dir() -> PathBuf {
    // Keep tmp under the chosen iOS storage root (cache)
    ios_storage_root().join("tmp")
}

#[cfg(not(target_os = "ios"))]
fn default_temporary_dir() -> PathBuf {
    dirs_next::data_dir()
        .unwrap_or_default()
        .join("flow-like")
        .join("tmp")
}

fn ensure_dir(p: &PathBuf) -> std::io::Result<()> {
    if !p.exists() {
        std::fs::create_dir_all(p)?;
    }
    Ok(())
}

#[cfg(target_os = "ios")]
pub fn ensure_app_dirs() -> std::io::Result<()> {
    let root = ios_storage_root();
    let bit_dir = root.join("bits");
    let project_dir = root.join("projects");
    let cache_dir = root.clone();

    ensure_dir(&bit_dir)?;
    ensure_dir(&project_dir)?;
    ensure_dir(&cache_dir)?;
    ensure_dir(&default_logs_dir())?;
    ensure_dir(&default_temporary_dir())?;
    Ok(())
}

#[cfg(not(target_os = "ios"))]
pub fn ensure_app_dirs() -> std::io::Result<()> {
    let bit_dir = dirs_next::data_dir()
        .ok_or_else(|| std::io::Error::other("data_dir() is None"))?
        .join("flow-like/bits");
    let project_dir = dirs_next::data_dir().unwrap().join("flow-like/projects");
    let cache_dir = dirs_next::cache_dir()
        .ok_or_else(|| std::io::Error::other("cache_dir() is None"))?
        .join("flow-like");

    ensure_dir(&bit_dir)?;
    ensure_dir(&project_dir)?;
    ensure_dir(&cache_dir)?;
    Ok(())
}

fn resolve_default_hub() -> String {
    if let Ok(url) = std::env::var("FLOW_LIKE_API_URL") {
        return url;
    }

    let config_domain = option_env!("FLOW_LIKE_CONFIG_DOMAIN");
    let config_secure = option_env!("FLOW_LIKE_CONFIG_SECURE");

    if let Some(domain) = config_domain {
        let secure = config_secure.map(|s| s == "true").unwrap_or(true);
        let protocol = if secure { "https" } else { "http" };
        return format!("{}://{}", protocol, domain);
    }

    String::from("https://api.alpha.flow-like.com")
}

#[derive(Serialize, Deserialize)]
pub struct Settings {
    loaded: bool,
    pub default_hub: String,
    pub dev_mode: bool,
    pub current_profile: String,
    pub bit_dir: PathBuf,
    pub project_dir: PathBuf,
    #[serde(default = "default_logs_dir")]
    pub logs_dir: PathBuf,
    #[serde(default = "default_temporary_dir")]
    pub temporary_dir: PathBuf,
    pub user_dir: PathBuf,
    pub profiles: HashMap<String, UserProfile>,
    pub updated: SystemTime,
    pub created: SystemTime,

    #[serde(skip)]
    config: Option<Arc<FlowLikeConfig>>,
}

impl Settings {
    pub fn new() -> Self {
        // Prefer new stable settings path; fallback to legacy cache path for one-time backward compatibility.
        let new_settings_path = settings_store_path();
        let legacy_settings_path = get_cache_dir().join("global-settings.json");

        if new_settings_path.exists() || legacy_settings_path.exists() {
            let path = if new_settings_path.exists() {
                &new_settings_path
            } else {
                &legacy_settings_path
            };
            let settings = std::fs::read(path);
            if let Ok(settings) = settings {
                let settings = serde_json::from_slice::<Settings>(&settings);
                if let Ok(mut settings) = settings {
                    settings.loaded = false;
                    // Normalize platform paths (on iOS: always derive from current container roots)
                    settings.normalize_platform_paths();
                    // Make sure required directories exist after normalization.
                    let _ = ensure_app_dirs();
                    let _ = ensure_dir(&settings.logs_dir);
                    let _ = ensure_dir(&settings.temporary_dir);
                    // Persist any normalization so subsequent boots are clean.
                    Settings::serialize(&mut settings);
                    println!("Loaded settings from: {:?}", path);
                    return settings;
                }

                println!(
                    "Failed to load settings from cache, {}",
                    settings.err().unwrap()
                );
            }
        }

        ensure_app_dirs().ok();

        let mut bit_dir = dirs_next::data_dir()
            .unwrap_or_default()
            .join("flow-like")
            .join("bits");
        let mut project_dir = dirs_next::data_dir()
            .unwrap_or_default()
            .join("flow-like")
            .join("projects");
        let mut user_dir = dirs_next::cache_dir().unwrap_or_default().join("flow-like");

        if cfg!(target_os = "ios") {
            #[cfg(target_os = "ios")]
            {
                let root = ios_storage_root();
                bit_dir = root.join("bits");
                project_dir = root.join("projects");
                user_dir = root.clone();
            }
        }

        Self {
            loaded: false,
            dev_mode: false,
            default_hub: resolve_default_hub(),
            current_profile: String::from("default"),
            bit_dir,
            project_dir,
            logs_dir: default_logs_dir(),
            temporary_dir: default_temporary_dir(),
            user_dir,
            profiles: HashMap::new(),
            created: SystemTime::now(),
            updated: SystemTime::now(),
            config: None,
        }
    }

    pub fn set_config(&mut self, config: &FlowLikeConfig) {
        self.config = Some(Arc::new(config.clone()));
    }

    pub fn get_current_profile(&self) -> anyhow::Result<UserProfile> {
        let profile = self.profiles.get(&self.current_profile);
        if let Some(profile) = profile {
            return Ok(profile.clone());
        }

        let first_profile = self.profiles.iter().next();

        if first_profile.is_none() {
            return Err(anyhow::anyhow!("No profiles found"));
        }

        let first_profile = first_profile.unwrap();
        let first_profile = first_profile.1;

        Ok(first_profile.clone())
    }

    pub async fn set_current_profile(
        &mut self,
        profile: &UserProfile,
        _app_handle: &AppHandle,
    ) -> anyhow::Result<UserProfile> {
        let profile = self
            .profiles
            .get(&profile.hub_profile.id)
            .cloned()
            .ok_or(anyhow::anyhow!("Profile not found"))?;

        self.current_profile = profile.hub_profile.id.clone();
        self.serialize();

        Ok(profile)
    }

    pub fn serialize(&mut self) {
        let dir = settings_store_path();
        if let Some(parent) = dir.parent() {
            let _ = std::fs::create_dir_all(parent);
        }
        let settings = serde_json::to_vec(&self);
        if let Ok(settings) = settings {
            let _res = std::fs::write(dir, settings);
        }
    }
}

impl Drop for Settings {
    fn drop(&mut self) {
        self.serialize();
    }
}

impl Settings {
    fn normalize_platform_paths(&mut self) {
        #[cfg(target_os = "ios")]
        {
            let root = ios_storage_root();
            // Compute desired paths under the unified iOS storage root (cache-based).
            let new_bit = root.join("bits");
            let new_project = root.join("projects");
            let new_user = root.clone();
            let new_logs = default_logs_dir();
            let new_tmp = default_temporary_dir();
            // Always rebase to the current container's data root on iOS.
            self.bit_dir = new_bit;
            self.project_dir = new_project;
            self.user_dir = new_user;
            self.logs_dir = new_logs;
            self.temporary_dir = new_tmp;
        }
    }
}

// Compute the path to persist global settings. On iOS, prefer data_dir for durability.
fn settings_store_path() -> std::path::PathBuf {
    #[cfg(target_os = "ios")]
    {
        return ios_storage_root().join("global-settings.json");
    }
    #[cfg(not(target_os = "ios"))]
    {
        get_cache_dir().join("global-settings.json")
    }
}
