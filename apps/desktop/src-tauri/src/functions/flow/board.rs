use crate::{functions::TauriFunctionError, state::TauriFlowLikeState};
use flow_like::{
    app::App,
    flow::{
        board::{self, commands::GenericCommand, Board, ExecutionStage},
        execution::LogLevel,
    },
};
use object_store::path::Path;
use std::sync::Arc;
use tauri::AppHandle;
use tauri_plugin_dialog::DialogExt;
use tokio::sync::Mutex;

#[tauri::command(async)]
pub async fn save_board(handler: AppHandle, board_id: String) -> Result<(), TauriFunctionError> {
    let file_path = handler.dialog().file().blocking_save_file();
    if let Some(file_path) = file_path {
        let board_state = TauriFlowLikeState::construct(&handler).await?;
        let board = board_state.lock().await.get_board(&board_id)?;
        let board = board.lock().await.clone();
        let board_string = serde_json::to_string(&board)
            .map_err(|e| TauriFunctionError::from(anyhow::Error::new(e)))?;
        let file_path = file_path
            .as_path()
            .ok_or(TauriFunctionError::new("Invalid file path"))?;
        std::fs::write(file_path, board_string)
            .map_err(|e| TauriFunctionError::from(anyhow::Error::new(e)))?;
    }
    Err(TauriFunctionError::new("Board not found"))
}

#[tauri::command(async)]
pub async fn create_board(app_handle: AppHandle) -> Result<Board, TauriFunctionError> {
    let path = Path::from("debug").child("boards");
    let flow_like_state = TauriFlowLikeState::construct(&app_handle).await?;
    let board = Board::new(path, flow_like_state);

    let board_state = TauriFlowLikeState::construct(&app_handle).await?;
    board_state
        .lock()
        .await
        .register_board(&board.id, Arc::new(Mutex::new(board.clone())))?;
    Ok(board)
}

#[tauri::command(async)]
pub async fn get_board(
    handler: AppHandle,
    app_id: String,
    board_id: String,
) -> Result<Board, TauriFunctionError> {
    let board_state = TauriFlowLikeState::construct(&handler).await?;
    let board = board_state.lock().await.get_board(&board_id);
    if let Ok(board) = board {
        let board = board.lock().await.clone();
        return Ok(board);
    }

    let flow_like_state = TauriFlowLikeState::construct(&handler).await?;

    if let Ok(app) = App::load(app_id, flow_like_state).await {
        let board = app.open_board(board_id, Some(true)).await?;
        return Ok(board.lock().await.clone());
    }

    Err(TauriFunctionError::new("Board not found"))
}

#[tauri::command(async)]
pub async fn close_board(handler: AppHandle, board_id: String) -> Result<(), TauriFunctionError> {
    let board_state = TauriFlowLikeState::construct(&handler).await?;
    let store = TauriFlowLikeState::get_project_store(&handler).await?;

    let board = { board_state.lock().await.remove_board(&board_id)? };

    if let Some(board) = board {
        let board = board.lock().await;
        board.save(Some(store.clone())).await?;
        return Ok(());
    }

    Err(TauriFunctionError::new("Board not found"))
}

#[tauri::command(async)]
pub async fn get_open_boards(
    handler: AppHandle,
) -> Result<Vec<(String, String)>, TauriFunctionError> {
    let board_state = TauriFlowLikeState::construct(&handler).await?;

    let board_state = board_state.lock().await.board_registry.clone();
    let mut boards = Vec::with_capacity(board_state.len());
    for entry in board_state.iter() {
        let value = entry.value();
        let board = value.lock().await;
        boards.push((entry.key().clone(), board.name.clone()));
    }

    Ok(boards)
}

#[tauri::command(async)]
pub async fn update_board_meta(
    handler: AppHandle,
    app_id: String,
    board_id: String,
    name: String,
    description: String,
    log_level: LogLevel,
    stage: ExecutionStage,
) -> Result<Board, TauriFunctionError> {
    let store = TauriFlowLikeState::get_project_store(&handler).await?;
    let board_state = TauriFlowLikeState::construct(&handler).await?;
    let board = board_state.lock().await.get_board(&board_id)?;
    let mut board = board.lock().await;
    board.name = name;
    board.description = description;
    board.log_level = log_level;
    board.stage = stage;
    board.save(Some(store.clone())).await?;
    Ok(board.clone())
}

#[tauri::command(async)]
pub async fn undo_board(
    handler: AppHandle,
    app_id: String,
    board_id: String,
) -> Result<Board, TauriFunctionError> {
    let flow_like_state = TauriFlowLikeState::construct(&handler).await?;
    let board = flow_like_state.lock().await.get_board(&board_id)?;
    let store = TauriFlowLikeState::get_project_store(&handler).await?;
    let mut board = board.lock().await;
    let _ = board.undo(flow_like_state).await;
    board.save(Some(store.clone())).await?;
    Ok(board.clone())
}

#[tauri::command(async)]
pub async fn redo_board(
    handler: AppHandle,
    app_id: String,
    board_id: String,
) -> Result<Board, TauriFunctionError> {
    let store = TauriFlowLikeState::get_project_store(&handler).await?;
    let flow_like_state = TauriFlowLikeState::construct(&handler).await?;
    let board = flow_like_state.lock().await.get_board(&board_id)?;
    let mut board = board.lock().await;
    let _ = board.redo(flow_like_state).await;
    board.save(Some(store.clone())).await?;
    Ok(board.clone())
}

#[tauri::command(async)]
pub async fn execute_command(
    handler: AppHandle,
    app_id: String,
    board_id: String,
    command: GenericCommand,
    append: Option<bool>,
) -> Result<(), TauriFunctionError> {
    let flow_like_state = TauriFlowLikeState::construct(&handler).await?;
    let store = TauriFlowLikeState::get_project_store(&handler).await?;

    let board = flow_like_state.lock().await.get_board(&board_id)?;

    let mut board = board.lock().await;
    board
        .execute_command(command, flow_like_state.clone(), append.unwrap_or(false))
        .await?;

    board.save(Some(store)).await?;
    Ok(())
}
