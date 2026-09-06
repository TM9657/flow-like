use crate::{functions::TauriFunctionError, state::TauriFlowLikeState};
use flow_like::{
    app::App, app_build::AppBuildStore, flow_like_storage::files::store::FlowLikeStore,
    state::FlowLikeState,
};
use serde_json::Value;
use tauri::AppHandle;

async fn local_store(
    app_handle: &AppHandle,
    app_id: &str,
) -> Result<AppBuildStore, TauriFunctionError> {
    let flow_state = TauriFlowLikeState::construct(app_handle).await?;
    // Check the manifest first so this command cannot provision build state for an app that
    // does not exist on the device.
    App::load(app_id.to_string(), flow_state.clone()).await?;
    let store = FlowLikeState::project_meta_store(&flow_state).await?;
    let build_store = match store {
        FlowLikeStore::Local(store) => AppBuildStore::new_local(store, app_id),
        store => AppBuildStore::new(store.as_generic(), app_id),
    };
    build_store.map_err(|error| TauriFunctionError::new(&error.to_string()))
}

#[tauri::command(async)]
pub async fn read_app_build(
    app_handle: AppHandle,
    app_id: String,
    build_id: String,
) -> Result<Option<Value>, TauriFunctionError> {
    local_store(&app_handle, &app_id)
        .await?
        .read(&build_id)
        .await
        .map_err(|error| TauriFunctionError::new(&error.to_string()))
}

#[tauri::command(async)]
pub async fn write_app_build(
    app_handle: AppHandle,
    app_id: String,
    build_id: String,
    record: Value,
    expected_revision: Option<u64>,
) -> Result<Value, TauriFunctionError> {
    let store = local_store(&app_handle, &app_id).await?;
    match expected_revision {
        Some(expected_revision) => {
            store
                .compare_and_swap(&build_id, expected_revision, record)
                .await
        }
        None => store.create(&build_id, record).await,
    }
    .map_err(|error| TauriFunctionError::new(&error.to_string()))
}
