use crate::profile::UserProfile;
use flow_like::{state::FlowLikeConfig, utils::cache::get_cache_dir};
use serde::{Deserialize, Serialize};
use std::{collections::HashMap, path::PathBuf, sync::Arc, time::SystemTime};
use tauri::AppHandle;

fn default_logs_dir() -> PathBuf {
    dirs_next::data_dir()
        .unwrap_or_default()
        .join("flow-like")
        .join("logs")
}

fn default_temporary_dir() -> PathBuf {
    dirs_next::data_dir()
        .unwrap_or_default()
        .join("flow-like")
        .join("tmp")
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

        Self {
            loaded: false,
            dev_mode: false,
            default_hub: String::from("api.alpha.flow-like.com"),
            current_profile: String::from("default"),
            bit_dir: dirs_next::data_dir()
                .unwrap_or_default()
                .join("flow-like")
                .join("bits"),
            project_dir: dirs_next::data_dir()
                .unwrap_or_default()
                .join("flow-like")
                .join("projects"),
            logs_dir: default_logs_dir(),
            temporary_dir: default_temporary_dir(),
            user_dir: dirs_next::cache_dir().unwrap_or_default().join("flow-like"),
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
