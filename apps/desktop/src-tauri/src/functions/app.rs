use super::TauriFunctionError;
use crate::state::{TauriFlowLikeState, TauriSettingsState};
use flow_like::{
    app::App,
    bit::BitMeta,
    flow::{board::Board, variable::Variable},
};
use futures::{StreamExt, TryStreamExt};
use object_store::path::Path;
use serde_json::Value;
use std::collections::HashSet;
use tauri::AppHandle;

#[tauri::command(async)]
pub async fn get_apps(app_handle: AppHandle) -> Result<Vec<App>, TauriFunctionError> {
    let mut app_list: Vec<App> = vec![];

    let profile = TauriSettingsState::current_profile(&app_handle).await?;

    let flow_like_state = TauriFlowLikeState::construct(&app_handle).await?;

    for app in profile.apps.iter() {
        if let Ok(app) = App::load(app.clone(), flow_like_state.clone()).await {
            let app = app;
            app_list.push(app);
        }
    }

    Ok(app_list)
}

#[tauri::command(async)]
pub async fn get_app(app_handle: AppHandle, app_id: String) -> Result<App, TauriFunctionError> {
    let flow_like_state = TauriFlowLikeState::construct(&app_handle).await?;

    if let Ok(app) = App::load(app_id, flow_like_state).await {
        return Ok(app);
    }

    Err(TauriFunctionError::new("App not found"))
}

#[tauri::command(async)]
pub async fn app_configured(
    app_handle: AppHandle,
    app_id: String,
) -> Result<bool, TauriFunctionError> {
    let flow_like_state = TauriFlowLikeState::construct(&app_handle).await?;

    if let Ok(app) = App::load(app_id, flow_like_state).await {
        let configured = app.boards_configured().await;
        return Ok(configured);
    }

    Err(TauriFunctionError::new("App not found"))
}

#[tauri::command(async)]
pub async fn get_remote_apps(_handler: AppHandle) {
    unimplemented!("Not implemented yet")
}

#[tauri::command(async)]
pub async fn create_app(
    app_handle: AppHandle,
    meta: BitMeta,
    bits: Vec<String>,
    template: String,
) -> Result<String, TauriFunctionError> {
    let flow_like_state = TauriFlowLikeState::construct(&app_handle).await?;

    let bits_map: HashSet<String> = bits.clone().into_iter().collect();
    let mut new_app = App::new(meta, bits, flow_like_state).await?;

    if template == "blank" {
        let board = new_app.create_board().await?;
        let board = new_app.open_board(board, Some(false)).await?;
        let mut variable = Variable::new(
            "Embedding Models",
            flow_like::flow::variable::VariableType::String,
            flow_like::flow::pin::ValueType::HashSet,
        );
        variable
            .set_exposed(false)
            .set_editable(false)
            .set_default_value(serde_json::json!(bits_map));

        let mut board = board.lock().await;
        board.variables.insert(variable.id.clone(), variable);
        board.save(None).await?;
    }

    new_app.save().await?;

    let mut profile = TauriSettingsState::current_profile(&app_handle).await?;
    let settings = TauriSettingsState::construct(&app_handle).await?;
    let mut settings = settings.lock().await;

    profile.apps.push(new_app.id.clone());

    settings
        .profiles
        .insert(profile.hub_profile.id.clone(), profile.clone());
    settings.serialize();

    Ok(new_app.id.clone())
}

#[tauri::command(async)]
pub async fn create_app_board(
    app_handle: AppHandle,
    app_id: String,
    name: String,
    description: String,
) -> Result<(), TauriFunctionError> {
    let flow_like_state = TauriFlowLikeState::construct(&app_handle).await?;
    let mut app = App::load(app_id, flow_like_state).await?;

    let board = app
        .boards
        .first()
        .ok_or(TauriFunctionError::new("No boards found"))?;
    let board = app.open_board(board.clone(), Some(false)).await?;
    let board = board.lock().await;
    let (_var_id, variable) = board
        .variables
        .iter()
        .find(|(_, variable)| variable.name == "Embedding Models" && !variable.editable)
        .ok_or(TauriFunctionError::new("No models variable found"))?;
    let variable: Variable = variable.duplicate();
    drop(board);

    let board_id = app.create_board().await?;
    let board = app.open_board(board_id, Some(false)).await?;
    app.save().await?;

    let mut board = board.lock().await;
    board.name = name;
    board.description = description;
    board.variables.insert(variable.id.clone(), variable);
    board.save(None).await?;

    Ok(())
}

#[tauri::command(async)]
pub async fn delete_app_board(
    app_handle: AppHandle,
    app_id: String,
    board_id: String,
) -> Result<(), TauriFunctionError> {
    let flow_like_state = TauriFlowLikeState::construct(&app_handle).await?;

    let mut app = App::load(app_id, flow_like_state).await?;
    if app.boards.len() == 1 {
        return Err(TauriFunctionError::new("Cannot delete the last board"));
    }
    app.delete_board(&board_id).await?;
    app.save().await?;
    Ok(())
}

#[tauri::command(async)]
pub async fn update_app(app_handle: AppHandle, app: App) -> Result<(), TauriFunctionError> {
    let mut app = app;
    app.app_state = Some(TauriFlowLikeState::construct(&app_handle).await?);
    app.save().await?;
    Ok(())
}

#[tauri::command(async)]
pub async fn get_app_size(
    app_handle: AppHandle,
    app_id: String,
) -> Result<usize, TauriFunctionError> {
    let store = TauriFlowLikeState::get_project_store(&app_handle).await?;
    let path = Path::from("apps").child(app_id);

    let mut locations = store.list(Some(&path)).map_ok(|m| m.location).boxed();
    let mut size = 0;

    while let Some(location) = locations.next().await {
        if let Ok(location) = location {
            if let Ok(meta) = store.head(&location).await {
                size += meta.size;
            }
        }
    }

    Ok(size)
}

#[tauri::command(async)]
pub async fn delete_app(app_handle: AppHandle, app_id: String) -> Result<(), TauriFunctionError> {
    if app_id.is_empty() {
        return Err(TauriFunctionError::new("App ID is empty"));
    };

    let store = TauriFlowLikeState::get_project_store(&app_handle).await?;
    let settings = TauriSettingsState::construct(&app_handle).await?;

    let mut settings = settings.lock().await;
    for profile in settings.profiles.values_mut() {
        profile.apps.retain(|app| app != &app_id);
    }
    settings.serialize();
    drop(settings);

    let path = Path::from("apps").child(app_id);
    let locations = store.list(Some(&path)).map_ok(|m| m.location).boxed();
    store
        .delete_stream(locations)
        .try_collect::<Vec<Path>>()
        .await
        .map_err(|_| TauriFunctionError::new("Failed to delete app"))?;

    Ok(())
}

#[tauri::command(async)]
pub async fn get_app_boards(
    app_handle: AppHandle,
    app_id: String,
) -> Result<Vec<Board>, TauriFunctionError> {
    let flow_like_state = TauriFlowLikeState::construct(&app_handle).await?;

    let mut boards = vec![];
    if let Ok(app) = App::load(app_id, flow_like_state).await {
        for board_id in app.boards.iter() {
            let board = app.open_board(board_id.clone(), Some(false)).await;
            if let Ok(board) = board {
                boards.push(board.lock().await.clone());
            }
        }
    }

    Ok(boards)
}

#[tauri::command(async)]
pub async fn get_app_board(
    app_handle: AppHandle,
    app_id: String,
    board_id: String,
    push_to_registry: bool,
) -> Result<Board, TauriFunctionError> {
    let flow_like_state = TauriFlowLikeState::construct(&app_handle).await?;

    if let Ok(app) = App::load(app_id, flow_like_state).await {
        let board = app.open_board(board_id, Some(push_to_registry)).await?;
        return Ok(board.lock().await.clone());
    }

    Err(TauriFunctionError::new("Board not found"))
}

#[tauri::command(async)]
pub async fn set_app_config(
    app_handle: AppHandle,
    app_id: String,
    board_id: String,
    variable_id: String,
    default_value: Value,
) -> Result<(), TauriFunctionError> {
    let flow_like_state = TauriFlowLikeState::construct(&app_handle).await?;

    if let Ok(app) = App::load(app_id, flow_like_state).await {
        let board = app.open_board(board_id, Some(true)).await?;
        let mut board = board.lock().await;
        if let Some(variable) = board.variables.get_mut(&variable_id) {
            variable.default_value = Some(serde_json::to_vec(&default_value).unwrap());
        }
        board.save(None).await?;
    }

    Err(TauriFunctionError::new("Board not found"))
}
