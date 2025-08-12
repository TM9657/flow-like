use std::path::PathBuf;

use flow_like::{app::App, profile::ProfileApp};
use tauri::AppHandle;
use tauri_plugin_dialog::DialogExt;

use crate::{
    functions::TauriFunctionError,
    state::{TauriFlowLikeState, TauriSettingsState},
};

#[tauri::command(async)]
pub async fn export_app_to_file(
    app_handle: AppHandle,
    app_id: String,
    password: Option<String>,
) -> Result<(), TauriFunctionError> {
    let flow_like_state = TauriFlowLikeState::construct(&app_handle).await?;

    if let Ok(app) = App::load(app_id.clone(), flow_like_state.clone()).await {
        let meta = App::get_meta(app_id, flow_like_state, None, None)
            .await
            .map_err(|e| TauriFunctionError::new(&format!("Failed to get app meta: {}", e)))?;

        let target_file = app_handle
            .dialog()
            .file()
            .set_title("Export App")
            .set_file_name(format!("{}.flow-app", meta.name))
            .blocking_save_file()
            .ok_or_else(|| TauriFunctionError::new("Failed to select target file"))?;

        let path_buf = target_file
            .into_path()
            .map_err(|e| TauriFunctionError::new(&format!("Failed to convert file path: {}", e)))?;

        app.export_archive(password, path_buf).await?;
        return Ok(());
    }

    Err(TauriFunctionError::new("App not found"))
}

#[tauri::command(async)]
pub async fn import_app_from_file(
    app_handle: AppHandle,
    path: PathBuf,
    password: Option<String>,
) -> Result<App, TauriFunctionError> {
    let flow_like_state = TauriFlowLikeState::construct(&app_handle).await?;

    let mut profile = TauriSettingsState::current_profile(&app_handle).await?;
    let settings = TauriSettingsState::construct(&app_handle).await?;

    let app = App::import_archive(flow_like_state, path, password)
        .await
        .map_err(|e| TauriFunctionError::new(&format!("Failed to import app: {}", e)))?;

    println!("Imported app: {:?}", app.id);

    if profile.hub_profile.apps.is_none() {
        profile.hub_profile.apps = Some(vec![]);
    }

    if let Some(apps) = &mut profile.hub_profile.apps {
        if !apps.iter().any(|a| a.app_id == app.id) {
            apps.push(ProfileApp::new(app.id.clone()));
            let mut settings = settings.lock().await;
            settings
                .profiles
                .insert(profile.hub_profile.id.clone(), profile.clone());
            settings.serialize();
        }
    }

    Ok(app)
}
