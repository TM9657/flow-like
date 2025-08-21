use std::{os::unix::raw::off_t, path::PathBuf, sync::Arc};

use flow_like::{
    app::App,
    credentials::SharedCredentials,
    flow_like_storage::{
        Path,
        arrow_schema::Schema,
        databases::vector::{VectorStore, lancedb::LanceDBVectorStore},
    },
    profile::ProfileApp,
};
use tauri::AppHandle;
use tauri_plugin_dialog::DialogExt;

use crate::{
    functions::TauriFunctionError,
    state::{TauriFlowLikeState, TauriSettingsState},
};

async fn db_connection(
    app_handle: &AppHandle,
    app_id: String,
    table_name: Option<String>,
    credentials: Option<Arc<SharedCredentials>>,
) -> flow_like_types::Result<LanceDBVectorStore> {
    let flow_like_state = TauriFlowLikeState::construct(&app_handle).await?;
    let table_name = table_name.unwrap_or("default".to_string());
    let board_dir = Path::from("apps")
        .child(app_id.clone())
        .child("storage")
        .child("db");
    let db = if let Some(credentials) = &credentials {
        credentials.to_db(&app_id).await?
    } else {
        flow_like_state
            .lock()
            .await
            .config
            .read()
            .await
            .callbacks
            .build_project_database
            .clone()
            .ok_or(flow_like_types::anyhow!("No database builder found"))?(board_dir)
    };

    let db = db.execute().await?;
    let db = LanceDBVectorStore::from_connection(db, table_name).await;
    Ok(db)
}

#[tauri::command(async)]
pub async fn db_table_names(
    app_handle: AppHandle,
    app_id: String,
    table_name: Option<String>,
    credentials: Option<Arc<SharedCredentials>>,
) -> Result<Vec<String>, TauriFunctionError> {
    let db = db_connection(&app_handle, app_id, table_name, credentials).await?;
    let table_names = db.list_tables().await?;
    Ok(table_names)
}

#[tauri::command(async)]
pub async fn db_schema(
    app_handle: AppHandle,
    app_id: String,
    table_name: String,
    credentials: Option<Arc<SharedCredentials>>,
) -> Result<Schema, TauriFunctionError> {
    let db = db_connection(&app_handle, app_id, Some(table_name), credentials).await?;
    let schema = db.schema().await?;
    Ok(schema)
}

#[tauri::command(async)]
pub async fn db_list(
    app_handle: AppHandle,
    app_id: String,
    table_name: String,
    credentials: Option<Arc<SharedCredentials>>,
    limit: Option<u64>,
    offset: Option<u64>,
) -> Result<Vec<flow_like_types::Value>, TauriFunctionError> {
    let db = db_connection(&app_handle, app_id, Some(table_name), credentials).await?;
    let limit = limit.unwrap_or(100).min(250) as usize;
    let offset = offset.unwrap_or(0) as usize;
    let items = db.list(limit, offset).await?;
    Ok(items)
}
