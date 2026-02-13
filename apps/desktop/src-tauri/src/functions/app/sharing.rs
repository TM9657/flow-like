use std::path::PathBuf;

use flow_like::{app::App, profile::ProfileApp};

use tauri::AppHandle;
#[cfg(target_os = "ios")]
use tauri::Manager;
use tauri_plugin_dialog::{DialogExt, FilePath};
use tracing::info;
#[cfg(target_os = "ios")]
use tracing::warn;
use urlencoding::decode;

use crate::{
    functions::TauriFunctionError,
    state::{TauriFlowLikeState, TauriSettingsState},
};

fn sanitize_file_name(name: &str) -> String {
    let mut sanitized = name
        .chars()
        .map(|c| match c {
            'a'..='z' | 'A'..='Z' | '0'..='9' => c,
            ' ' | '-' | '_' => c,
            _ => '-',
        })
        .collect::<String>();

    // Collapse consecutive separators and trim from edges for nicer defaults.
    while sanitized.contains("--") {
        sanitized = sanitized.replace("--", "-");
    }
    let sanitized = sanitized
        .trim_matches(|c: char| matches!(c, ' ' | '-' | '_'))
        .to_string();

    if sanitized.is_empty() {
        String::from("flow-like-app")
    } else {
        sanitized
    }
}

fn normalize_import_path(path: PathBuf) -> Result<PathBuf, TauriFunctionError> {
    let path_str = path.to_string_lossy();
    if !path_str.starts_with("file://") {
        return Ok(path);
    }

    let stripped = path_str.trim_start_matches("file://");
    let stripped = stripped.strip_prefix("localhost/").unwrap_or(stripped);

    let decoded = decode(stripped)
        .map_err(|e| TauriFunctionError::new(&format!("Failed to decode file URI: {}", e)))?;

    #[cfg(target_os = "windows")]
    {
        let without_leading_slash = decoded.trim_start_matches('/');
        Ok(PathBuf::from(without_leading_slash))
    }

    #[cfg(not(target_os = "windows"))]
    {
        let normalized = if decoded.starts_with('/') {
            decoded.to_string()
        } else {
            format!("/{}", decoded)
        };
        Ok(PathBuf::from(normalized))
    }
}

#[cfg(target_os = "ios")]
fn decode_file_url(uri: &str) -> Option<PathBuf> {
    let stripped = uri.strip_prefix("file://")?;
    let stripped = stripped.strip_prefix("localhost/").unwrap_or(stripped);
    let decoded = decode(stripped).ok()?;

    #[cfg(target_os = "windows")]
    {
        let without_leading_slash = decoded.trim_start_matches('/');
        Some(PathBuf::from(without_leading_slash))
    }

    #[cfg(not(target_os = "windows"))]
    {
        let normalized = if decoded.starts_with('/') {
            decoded.to_string()
        } else {
            format!("/{}", decoded)
        };
        Some(PathBuf::from(normalized))
    }
}

#[cfg(target_os = "ios")]
fn dialog_response_into_path(response: FilePath) -> Result<PathBuf, TauriFunctionError> {
    match response {
        FilePath::Path(path) => {
            info!(target: "export", ?path, "Dialog returned filesystem path");
            Ok(path)
        }
        FilePath::Url(url) => {
            info!(target: "export", uri = %url, "Dialog returned URI, attempting to decode");

            if let Ok(path) = url.to_file_path() {
                info!(target: "export", ?path, "Converted URI into filesystem path via to_file_path");
                return Ok(path);
            }

            if let Some(path) = decode_file_url(url.as_str()) {
                info!(target: "export", ?path, "Converted URI into filesystem path via manual decoding");
                return Ok(path);
            }

            warn!(target: "export", uri = %url, "Failed to convert URI into filesystem path");
            Err(TauriFunctionError::new(
                "Dialog response missing both path and decodable uri",
            ))
        }
    }
}

#[cfg(not(target_os = "ios"))]
fn dialog_response_into_path(response: FilePath) -> Result<PathBuf, TauriFunctionError> {
    response
        .into_path()
        .map_err(|e| TauriFunctionError::new(&format!("Failed to convert file path: {}", e)))
}

#[cfg(target_os = "ios")]
async fn perform_ios_export(
    app_handle: &AppHandle,
    app: &App,
    file_name: &str,
    password: Option<String>,
) -> Result<(), TauriFunctionError> {
    use flow_like_types::tokio::sync::oneshot;

    let documents_dir = app_handle.path().document_dir().map_err(|e| {
        TauriFunctionError::new(&format!("Failed to resolve documents directory: {}", e))
    })?;

    let staging_target = documents_dir.join(file_name);
    info!(
        target: "export",
        path = %staging_target.display(),
        "Preparing iOS staging export file"
    );

    let staged_file = app
        .export_archive(password, staging_target)
        .await
        .map_err(|e| TauriFunctionError::new(&format!("Failed to export app: {}", e)))?;

    let (tx, rx) = oneshot::channel();
    app_handle
        .dialog()
        .file()
        .set_title("Export App")
        .set_file_name(file_name.to_string())
        .save_file(move |response| {
            let _ = tx.send(response);
        });

    let selection = rx
        .await
        .map_err(|_| TauriFunctionError::new("Failed to receive selected file"))?;

    let target_file = match selection {
        Some(file) => file,
        None => {
            if let Err(err) = std::fs::remove_file(&staged_file) {
                warn!(
                    target: "export",
                    path = %staged_file.display(),
                    error = %err,
                    "Failed to remove iOS staging export file after cancellation"
                );
            }
            return Err(TauriFunctionError::new("Failed to select target file"));
        }
    };

    let path_buf = dialog_response_into_path(target_file)?;

    info!(
        target: "export",
        path = %path_buf.display(),
        "Resolved export target path"
    );

    if let Err(err) = std::fs::remove_file(&path_buf) {
        warn!(
            target: "export",
            path = %path_buf.display(),
            error = %err,
            "Failed to remove temporary iOS export copy"
        );
    }

    if let Err(err) = std::fs::remove_file(&staged_file) {
        warn!(
            target: "export",
            path = %staged_file.display(),
            error = %err,
            "Failed to remove iOS staging export file"
        );
    }

    info!(target: "export", "Export completed successfully");
    Ok(())
}

#[cfg(not(target_os = "ios"))]
async fn perform_standard_export(
    app_handle: &AppHandle,
    app: &App,
    file_name: &str,
    password: Option<String>,
) -> Result<(), TauriFunctionError> {
    let target_file = app_handle
        .dialog()
        .file()
        .set_title("Export App")
        .set_file_name(file_name.to_string())
        .blocking_save_file()
        .ok_or_else(|| TauriFunctionError::new("Failed to select target file"))?;

    let path_buf = dialog_response_into_path(target_file)?;

    info!(
        target: "export",
        path = %path_buf.display(),
        "Resolved export target path"
    );

    app.export_archive(password, path_buf)
        .await
        .map_err(|e| TauriFunctionError::new(&format!("Failed to export app: {}", e)))?;

    info!(target: "export", "Export completed successfully");
    Ok(())
}

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

        let file_suffix = if password.is_some() {
            "enc.flow-app"
        } else {
            "flow-app"
        };
        let default_file_name = format!("{}.{}", sanitize_file_name(&meta.name), file_suffix);

        #[cfg(target_os = "ios")]
        {
            perform_ios_export(&app_handle, &app, &default_file_name, password).await?;
            return Ok(());
        }

        #[cfg(not(target_os = "ios"))]
        {
            perform_standard_export(&app_handle, &app, &default_file_name, password).await?;
            return Ok(());
        }
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

    let path = normalize_import_path(path)?;

    let mut profile = TauriSettingsState::current_profile(&app_handle).await?;
    let settings = TauriSettingsState::construct(&app_handle).await?;

    let app = App::import_archive(flow_like_state, path, password)
        .await
        .map_err(|e| TauriFunctionError::new(&format!("Failed to import app: {}", e)))?;

    println!("Imported app: {:?}", app.id);

    let apps = profile.hub_profile.apps.get_or_insert_with(Vec::new);

    if !apps.iter().any(|a| a.app_id == app.id) {
        apps.push(ProfileApp::new(app.id.clone()));
        let mut settings_guard = settings.lock().await;
        settings_guard
            .profiles
            .insert(profile.hub_profile.id.clone(), profile.clone());
        settings_guard.serialize();
    }

    Ok(app)
}
