use crate::{
    functions::TauriFunctionError,
    profile::UserProfile,
    state::{TauriFlowLikeState, TauriSettingsState},
};
use flow_like::{bit::Bit, hub::Hub, profile::{Profile, ProfileApp}, utils::{cache::get_cache_dir, hash::hash_file, http::HTTPClient}};
use flow_like_types::tokio::task::JoinHandle;
use futures::future::join_all;
use serde::Deserialize;
use tauri_plugin_dialog::DialogExt;
use std::{collections::HashMap, path::PathBuf, sync::Arc};
use tauri::{AppHandle, Url};
use tracing::instrument;
use urlencoding::encode;

fn presign_icon(icon: &str) -> Result<String, TauriFunctionError> {
    // if it already looks like a URL (has a scheme), return it as-is to avoid double-presigning
    if icon.contains("://") {
        return Ok(icon.to_string());
    }

    #[cfg(any(windows, target_os = "android"))]
    let base = "http://asset.localhost/";
    #[cfg(not(any(windows, target_os = "android")))]
    let base = "asset://localhost/";
    let urlencoded_path = encode(icon);
    let url = format!("{base}{urlencoded_path}");
    let url = Url::parse(&url).map_err(|e| TauriFunctionError::new(&e.to_string()))?;
    Ok(url.to_string())
}

#[instrument(skip_all)]
#[tauri::command(async)]
pub async fn get_profiles(app_handle: AppHandle) -> HashMap<String, UserProfile> {
    let settings = TauriSettingsState::construct(&app_handle).await.unwrap();

    let mut profiles = {
        let settings_guard = settings.lock().await;
        settings_guard.profiles.clone()
    };

    for profile in profiles.values_mut() {
        if let Some(icon) = profile.hub_profile.icon.clone() {
            if !icon.starts_with("http://") && !icon.starts_with("https://") {
                if let Ok(icon) = presign_icon(&icon) {
                    profile.hub_profile.icon = Some(icon);
                }
            }
        }
    }

    profiles
}

#[instrument(skip_all)]
#[tauri::command(async)]
pub async fn get_default_profiles(
    app_handle: AppHandle,
) -> Result<(Vec<(UserProfile, Vec<Bit>)>, Hub), TauriFunctionError> {
    let settings = TauriSettingsState::construct(&app_handle).await?;
    let default_hub = settings.lock().await.default_hub.clone();
    let http_client = TauriFlowLikeState::http_client(&app_handle).await?;
    let default_hub = Hub::new(&default_hub, http_client.clone()).await?;

    let profiles = default_hub.get_profiles().await?;
    let profiles = get_bits(profiles.clone(), http_client).await?;

    Ok((profiles, default_hub))
}

#[instrument(skip_all)]
async fn get_bits(
    profiles: Vec<Profile>,
    http_client: Arc<HTTPClient>,
) -> flow_like_types::Result<Vec<(UserProfile, Vec<Bit>)>> {
    // Collect all futures for models and embedding models
    let mut bits: HashMap<&str, &str> = HashMap::new();
    let mut hubs: HashMap<&str, Hub> = HashMap::new();

    for profile in profiles.iter() {
        for bit_id in profile.bits.iter() {
            let (hub, bit) = bit_id.split_once(':').unwrap_or(("", bit_id));
            bits.insert(bit, hub);
            if !hubs.contains_key(hub) {
                hubs.insert(hub, Hub::new(hub, http_client.clone()).await?);
            }
        }
    }

    let bit_features = bits.iter().map(|(bit_id, hub_id)| {
        let hub = hubs.get(hub_id).unwrap();
        hub.get_bit(bit_id)
    });

    let bits_results = join_all(bit_features).await;

    let bits: Vec<Bit> = bits_results
        .into_iter()
        .filter_map(|res| res.ok())
        .collect();

    let bits_map: HashMap<String, Bit> = bits
        .iter()
        .map(|bit| (bit.id.clone(), bit.clone()))
        .collect();

    let output = profiles
        .iter()
        .map(|profile| {
            let bits = profile
                .bits
                .iter()
                .map(|bit_url| {
                    let (_hub, bit) = bit_url.split_once(':').unwrap_or(("", bit_url));
                    let bit = bits_map.get(bit).unwrap();
                    bit.clone()
                })
                .collect();
            let user_profile = UserProfile::new(profile.clone());
            (user_profile, bits)
        })
        .collect();

    Ok(output)
}

#[instrument(skip_all)]
#[tauri::command(async)]
pub async fn get_current_profile(app_handle: AppHandle) -> Result<UserProfile, TauriFunctionError> {
    let state = TauriFlowLikeState::construct(&app_handle).await?;
    let mut profile = TauriSettingsState::current_profile(&app_handle).await?.clone();

    state
        .lock()
        .await
        .model_factory
        .lock()
        .await
        .set_execution_settings(profile.execution_settings.clone());

    if let Some(icon) = profile.hub_profile.icon.clone() {
        if !icon.starts_with("http://") && !icon.starts_with("https://") {
            if let Ok(icon) = presign_icon(&icon) {
                profile.hub_profile.icon = Some(icon);
            }
        }
    }

    Ok(profile)
}

#[instrument(skip_all)]
#[tauri::command(async)]
pub async fn get_bits_in_current_profile(
    app_handle: AppHandle,
) -> Result<Vec<Bit>, TauriFunctionError> {
    let profile = TauriSettingsState::current_profile(&app_handle).await?;
    let http_client = TauriFlowLikeState::http_client(&app_handle).await?;

    let mut tasks: Vec<JoinHandle<Option<Bit>>> = vec![];

    for bit_id in profile.hub_profile.bits.iter() {
        let (hub, bit) = bit_id.split_once(':').unwrap_or(("", bit_id));
        if hub.is_empty() {
            continue; // Skip bits without a hub
        }
        let hub = hub.to_string();
        let bit = bit.to_string();
        let http_client = http_client.clone();
        let task = flow_like_types::tokio::spawn(async move {
            let hub = Hub::new(&hub, http_client).await.ok()?;
            let bit = hub.get_bit(&bit).await.ok()?;
            Some(bit)
        });
        tasks.push(task);
    }

    let results = join_all(tasks).await;
    let found_bits: Vec<Bit> = results
        .into_iter()
        .filter_map(|res| res.ok().flatten())
        .collect();

    Ok(found_bits)
}

#[instrument(skip_all)]
#[tauri::command(async)]
pub async fn set_current_profile(
    app_handle: AppHandle,
    profile_id: String,
) -> Result<UserProfile, TauriFunctionError> {
    let settings = TauriSettingsState::construct(&app_handle).await?;
    let mut settings = settings.lock().await;
    let profile = settings
        .profiles
        .get(&profile_id)
        .cloned()
        .ok_or(anyhow::anyhow!("Profile not found"))?;
    settings.set_current_profile(&profile, &app_handle).await?;
    settings.serialize();
    Ok(profile.clone())
}

#[instrument(skip_all)]
#[tauri::command(async)]
pub async fn upsert_profile(
    app_handle: AppHandle,
    profile: UserProfile,
) -> Result<UserProfile, TauriFunctionError> {
    let settings = TauriSettingsState::construct(&app_handle).await?;
    let mut settings = settings.lock().await;
    settings
        .profiles
        .insert(profile.hub_profile.id.clone(), profile.clone());

    if settings.current_profile == profile.hub_profile.id || settings.current_profile.is_empty() {
        settings.set_current_profile(&profile, &app_handle).await?;
    };

    settings.serialize();
    Ok(profile.clone())
}

#[instrument(skip_all)]
#[tauri::command(async)]
pub async fn delete_profile(
    app_handle: AppHandle,
    profile_id: String,
) -> Result<(), TauriFunctionError> {
    let settings = TauriSettingsState::construct(&app_handle).await?;
    let mut settings = settings.lock().await;
    let current_profile = settings.get_current_profile()?;
    if current_profile.hub_profile.id == profile_id {
        return Err(TauriFunctionError::new("Cannot delete current profile"));
    }
    settings.profiles.remove(&profile_id);
    settings.serialize();
    Ok(())
}

#[instrument(skip_all)]
#[tauri::command(async)]
pub async fn add_bit(
    app_handle: AppHandle,
    profile: UserProfile,
    bit: Bit,
) -> Result<(), TauriFunctionError> {
    let settings = TauriSettingsState::construct(&app_handle).await?;
    let mut settings = settings.lock().await;
    let profile = settings
        .profiles
        .get_mut(&profile.hub_profile.id)
        .ok_or(anyhow::anyhow!("Profile not found"))?;
    profile.hub_profile.add_bit(&bit).await;
    settings.serialize();
    Ok(())
}

#[instrument(skip_all)]
#[tauri::command(async)]
pub async fn remove_bit(
    app_handle: AppHandle,
    profile: UserProfile,
    bit: Bit,
) -> Result<(), TauriFunctionError> {
    let settings = TauriSettingsState::construct(&app_handle).await?;
    let mut settings = settings.lock().await;
    let profile = settings
        .profiles
        .get_mut(&profile.hub_profile.id)
        .ok_or(anyhow::anyhow!("Profile not found"))?;
    profile.hub_profile.remove_bit(&bit);
    settings.serialize();
    Ok(())
}

#[instrument(skip_all)]
#[tauri::command(async)]
pub async fn change_profile_image(
    app_handle: AppHandle,
    profile: UserProfile,
) -> Result<(), TauriFunctionError> {
    let dir = get_cache_dir();
    let dir = PathBuf::from(dir).join("icons");
    let file_path = app_handle.dialog().file().blocking_pick_file().ok_or(
        TauriFunctionError::new("No file selected"),
    )?;
    let file_path = file_path.into_path().map_err(|e| TauriFunctionError::new(&e.to_string()))?;
    let hash = hash_file(&file_path);
    let new_path = dir.join(format!("{}.{}", hash, file_path.extension().and_then(|s| s.to_str()).unwrap_or("png")));
    std::fs::create_dir_all(&dir).map_err(|e| TauriFunctionError::new(&e.to_string()))?;
    std::fs::copy(&file_path, &new_path).map_err(|e| TauriFunctionError::new(&e.to_string()))?;
    let icon = new_path.to_string_lossy().to_string();
    let settings = TauriSettingsState::construct(&app_handle).await?;
    let mut settings = settings.lock().await;
    let profile = settings
        .profiles
        .get_mut(&profile.hub_profile.id)
        .ok_or(anyhow::anyhow!("Profile not found"))?;

    let mut icon_to_delete = None;
    if let Some(old_icon) = profile.hub_profile.icon.take() {
        if !old_icon.starts_with("http://") && !old_icon.starts_with("https://") {
            icon_to_delete = Some(old_icon);
        }
    }

    println!("Setting icon to {}", icon);
    profile.hub_profile.icon = Some(icon);
    settings.serialize();

    if let Some(icon) = icon_to_delete {
        let profiles_using_icon = settings.profiles.values().filter(|p| p.hub_profile.icon == Some(icon.clone())).count();
        if profiles_using_icon == 0 {
            std::fs::remove_file(icon).map_err(|e| TauriFunctionError::new(&e.to_string()))?;
        }
    }

    Ok(())
}

#[derive(Clone, Deserialize)]
pub enum ProfileAppUpdateOperation {
    Upsert,
    Remove,
}

#[instrument(skip_all)]
#[tauri::command(async)]
pub async fn profile_update_app(
    app_handle: AppHandle,
    profile: UserProfile,
    app: ProfileApp,
    operation: ProfileAppUpdateOperation,
) -> Result<(), TauriFunctionError> {
    let settings = TauriSettingsState::construct(&app_handle).await?;
    let mut settings = settings.lock().await;
    let profile = settings
        .profiles
        .get_mut(&profile.hub_profile.id)
        .ok_or(anyhow::anyhow!("Profile not found"))?;
    match operation {
        ProfileAppUpdateOperation::Upsert => {
            profile.hub_profile.apps.as_mut().map(|apps| {
                apps.retain(|a| a.app_id != app.app_id);
            });

            profile.hub_profile.apps.get_or_insert(vec![]).push(app);
        },
        ProfileAppUpdateOperation::Remove => {
            profile.hub_profile.apps.as_mut().map(|apps| {
                apps.retain(|a| a.app_id != app.app_id);
            });
        }
    }
    settings.serialize();
    Ok(())
}