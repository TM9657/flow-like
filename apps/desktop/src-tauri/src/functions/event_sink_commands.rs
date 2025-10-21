use crate::event_sink::EventRegistration;
use crate::state::TauriEventSinkManagerState;
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

impl From<anyhow::Error> for TauriFunctionError {
    fn from(error: anyhow::Error) -> Self {
        TauriFunctionError {
            message: error.to_string(),
        }
    }
}

/// Add or update an event sink
/// If the event already exists with a different sink type, it will be switched
#[instrument(skip(app_handle, registration))]
#[tauri::command(async)]
pub async fn add_event_sink(
    app_handle: AppHandle,
    registration: EventRegistration,
) -> Result<(), TauriFunctionError> {
    let manager_arc = TauriEventSinkManagerState::construct(&app_handle)
        .await
        .map_err(|e| format!("Failed to get EventSinkManager: {}", e))?;

    let manager = manager_arc.lock().await;
    manager
        .add_event_sink(&app_handle, registration)
        .await
        .map_err(|e| e.into())
}

/// Remove an event sink
#[instrument(skip(app_handle))]
#[tauri::command(async)]
pub async fn remove_event_sink(
    app_handle: AppHandle,
    event_id: String,
) -> Result<(), TauriFunctionError> {
    let manager_arc = TauriEventSinkManagerState::construct(&app_handle)
        .await
        .map_err(|e| format!("Failed to get EventSinkManager: {}", e))?;

    let manager = manager_arc.lock().await;
    manager
        .remove_event_sink(&app_handle, &event_id)
        .await
        .map_err(|e| e.into())
}

/// Get a specific event sink registration
#[instrument(skip(app_handle))]
#[tauri::command(async)]
pub async fn get_event_sink(
    app_handle: AppHandle,
    event_id: String,
) -> Result<Option<EventRegistration>, TauriFunctionError> {
    let manager_arc = TauriEventSinkManagerState::construct(&app_handle)
        .await
        .map_err(|e| format!("Failed to get EventSinkManager: {}", e))?;

    let manager = manager_arc.lock().await;
    manager.get_registration(&event_id).map_err(|e| e.into())
}

/// List all event sink registrations
#[instrument(skip(app_handle))]
#[tauri::command(async)]
pub async fn list_event_sinks(
    app_handle: AppHandle,
) -> Result<Vec<EventRegistration>, TauriFunctionError> {
    let manager_arc = TauriEventSinkManagerState::construct(&app_handle)
        .await
        .map_err(|e| format!("Failed to get EventSinkManager: {}", e))?;

    let manager = manager_arc.lock().await;
    manager.list_registrations().map_err(|e| e.into())
}

/// Check if a sink is active for an event
/// Returns true if the event is registered and has an active sink
#[instrument(skip(app_handle))]
#[tauri::command(async)]
pub async fn is_event_sink_active(
    app_handle: AppHandle,
    event_id: String,
) -> Result<bool, TauriFunctionError> {
    let manager_arc = TauriEventSinkManagerState::construct(&app_handle)
        .await
        .map_err(|e| format!("Failed to get EventSinkManager: {}", e))?;

    let manager = manager_arc.lock().await;
    Ok(manager.is_event_active(&event_id))
}
