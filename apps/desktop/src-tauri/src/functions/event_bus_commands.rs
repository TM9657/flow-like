use crate::state;
use flow_like_types::Value;
use tauri::AppHandle;
use tracing::instrument;

#[derive(Debug, serde::Serialize)]
pub struct TauriFunctionError {
    pub message: String,
}

impl From<String> for TauriFunctionError {
    fn from(message: String) -> Self {
        TauriFunctionError { message }
    }
}

/// Push an event to the EventBus for execution
#[instrument(skip_all)]
#[tauri::command(async)]
pub async fn push_event(
    app_handle: AppHandle,
    payload: Option<Value>,
    app_id: String,
    event_id: String,
    offline: bool,
) -> Result<(), TauriFunctionError> {
    let event_bus = state::TauriEventBusState::construct(&app_handle)
        .map_err(|e| format!("Failed to get EventBus: {}", e))?;

    event_bus
        .push_event(payload, app_id, event_id, offline)
        .map_err(|e| e.into())
}

/// Register a Personal Access Token (PAT) for online event execution
#[instrument(skip_all)]
#[tauri::command(async)]
pub async fn register_pat(app_handle: AppHandle, pat: String) -> Result<(), TauriFunctionError> {
    let event_bus = state::TauriEventBusState::construct(&app_handle)
        .map_err(|e| format!("Failed to get EventBus: {}", e))?;

    event_bus.register_pat(pat).map_err(|e| e.into())
}

/// Get the currently registered PAT (if any)
#[instrument(skip_all)]
#[tauri::command(async)]
pub async fn get_pat(app_handle: AppHandle) -> Result<Option<String>, TauriFunctionError> {
    let event_bus = state::TauriEventBusState::construct(&app_handle)
        .map_err(|e| format!("Failed to get EventBus: {}", e))?;

    Ok(event_bus.get_pat())
}
