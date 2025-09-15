use crate::profile::UserProfile;
use flow_like::{state::FlowLikeConfig, utils::cache::get_cache_dir};
use serde::{Deserialize, Serialize};
use std::{collections::HashMap, path::PathBuf, sync::Arc, time::SystemTime};
use tauri::{fs, AppHandle};

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

#[cfg(target_os = "ios")]
fn default_logs_dir() -> PathBuf {
    // Use cache for logs so the OS can purge if needed (iOS safe)
    app_cache_root().join("logs")
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
    // On iOS: use cache for temporary files (purgeable by OS)
    app_cache_root().join("tmp")
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
        let data_root = app_data_root();
        let bit_dir = data_root.join("bits");
        let project_dir = data_root.join("projects");
        let cache_dir = app_cache_root();

        ensure_dir(&bit_dir)?;
        ensure_dir(&project_dir)?;
        ensure_dir(&cache_dir)?;
        Ok(())
    }

    #[cfg(not(target_os = "ios"))]
    pub fn ensure_app_dirs() -> std::io::Result<()> {
        let bit_dir = dirs_next::data_dir()
            .ok_or_else(|| std::io::Error::new(std::io::ErrorKind::Other, "data_dir() is None"))?
            .join("flow-like/bits");
        let project_dir = dirs_next::data_dir().unwrap().join("flow-like/projects");
        let cache_dir = dirs_next::cache_dir()
            .ok_or_else(|| std::io::Error::new(std::io::ErrorKind::Other, "cache_dir() is None"))?
            .join("flow-like");

        ensure_dir(&bit_dir)?;
        ensure_dir(&project_dir)?;
        ensure_dir(&cache_dir)?;
        Ok(())
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
        let dir = get_cache_dir();
        let dir = dir.join("global-settings.json");
        if dir.exists() {
            let settings = std::fs::read(&dir);
            if let Ok(settings) = settings {
                let settings = serde_json::from_slice::<Settings>(&settings);
                if let Ok(mut settings) = settings {
                    settings.loaded = false;
                    println!("Loaded settings from cache: {:?}", dir);
                    return settings;
                }

                println!(
                    "Failed to load settings from cache, {}",
                    settings.err().unwrap()
                );
            }
        }

        ensure_app_dirs().ok();

        // Preserve existing locations on non-iOS, use sandbox-safe on iOS.
        #[allow(unused_mut)]
        let mut bit_dir = dirs_next::data_dir()
            .unwrap_or_default()
            .join("flow-like")
            .join("bits");
        #[allow(unused_mut)]
        let mut project_dir = dirs_next::data_dir()
            .unwrap_or_default()
            .join("flow-like")
            .join("projects");
        #[allow(unused_mut)]
        let mut user_dir = dirs_next::cache_dir().unwrap_or_default().join("flow-like");

        if cfg!(target_os = "ios") {
            #[cfg(target_os = "ios")]
            {
                bit_dir = app_data_root().join("bits");
                project_dir = app_data_root().join("projects");
                user_dir = app_cache_root();
            }
        }

        Self {
            loaded: false,
            dev_mode: false,
            default_hub: String::from("api.alpha.flow-like.com"),
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
        let dir = get_cache_dir();
        let dir = dir.join("global-settings.json");
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
