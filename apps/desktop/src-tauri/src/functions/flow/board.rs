use crate::{functions::TauriFunctionError, state::TauriFlowLikeState};
use flow_like::flow::{
    board::{
        commands::{
            comments::{
                remove_comment::RemoveCommentCommand, upsert_comment::UpsertCommentCommand,
            },
            nodes::{
                add_node::AddNodeCommand, copy_paste::CopyPasteCommand, move_node::MoveNodeCommand,
                remove_node::RemoveNodeCommand, update_node::UpdateNodeCommand,
            },
            pins::{
                connect_pins::ConnectPinsCommand, disconnect_pins::DisconnectPinsCommand,
                upsert_pin::UpsertPinCommand,
            },
            variables::{
                remove_variable::RemoveVariableCommand, upsert_variable::UpsertVariableCommand,
            },
        },
        Board, Command, Comment, ExecutionStage,
    },
    execution::LogLevel,
    node::Node,
    pin::Pin,
    variable::Variable,
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
    board_state.lock().await.register_board(&board.id, Arc::new(Mutex::new(board.clone())))?;
    Ok(board)
}

#[tauri::command(async)]
pub async fn get_board(handler: AppHandle, board_id: String) -> Result<Board, TauriFunctionError> {
    let board_state = TauriFlowLikeState::construct(&handler).await?;
    let board = board_state.lock().await.get_board(&board_id)?;
    let board = board.lock().await.clone();
    Ok(board)
}

#[tauri::command(async)]
pub async fn close_board(handler: AppHandle, board_id: String) -> Result<(), TauriFunctionError> {
    let board_state = TauriFlowLikeState::construct(&handler).await?;
    let store = TauriFlowLikeState::get_project_store(&handler).await?;

    let board = {
        board_state.lock().await.remove_board(&board_id)?
    };

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
pub async fn undo_board(handler: AppHandle, board_id: String) -> Result<Board, TauriFunctionError> {
    let flow_like_state = TauriFlowLikeState::construct(&handler).await?;
    let board = flow_like_state.lock().await.get_board(&board_id)?;
    let store = TauriFlowLikeState::get_project_store(&handler).await?;
    let mut board = board.lock().await;
    let _ = board.undo(flow_like_state).await;
    board.save(Some(store.clone())).await?;
    return Ok(board.clone());
}

#[tauri::command(async)]
pub async fn redo_board(handler: AppHandle, board_id: String) -> Result<Board, TauriFunctionError> {
    let store = TauriFlowLikeState::get_project_store(&handler).await?;
    let flow_like_state = TauriFlowLikeState::construct(&handler).await?;
    let board = flow_like_state.lock().await.get_board(&board_id)?;
    let mut board = board.lock().await;
    let _ = board.redo(flow_like_state).await;
    board.save(Some(store.clone())).await?;
    Ok(board.clone())
}

#[tauri::command(async)]
pub async fn add_node_to_board(
    handler: AppHandle,
    board_id: String,
    node: Node,
    append: Option<bool>,
) -> Result<Node, TauriFunctionError> {
    let add_command = AddNodeCommand::new(node);
    let new_node = add_command.node.clone();
    execute_command(
        &handler,
        &board_id,
        Arc::new(Mutex::new(add_command)),
        append,
    )
    .await?;
    Ok(new_node)
}

#[tauri::command(async)]
pub async fn update_node(
    handler: AppHandle,
    board_id: String,
    node: Node,
    append: Option<bool>,
) -> Result<Board, TauriFunctionError> {
    let update_command = UpdateNodeCommand::new(node);
    execute_command(
        &handler,
        &board_id,
        Arc::new(Mutex::new(update_command)),
        append,
    )
    .await
}

#[tauri::command(async)]
pub async fn paste_nodes_to_board(
    handler: AppHandle,
    board_id: String,
    nodes: Vec<Node>,
    comments: Vec<Comment>,
    offset: (f32, f32, f32),
    append: Option<bool>,
) -> Result<Board, TauriFunctionError> {
    let add_command = CopyPasteCommand::new(nodes, comments, offset);
    execute_command(
        &handler,
        &board_id,
        Arc::new(Mutex::new(add_command)),
        append,
    )
    .await
}

#[tauri::command(async)]
pub async fn remove_node_from_board(
    handler: AppHandle,
    board_id: String,
    node: Node,
    append: Option<bool>,
) -> Result<Board, TauriFunctionError> {
    let remove_command = RemoveNodeCommand::new(node);
    execute_command(
        &handler,
        &board_id,
        Arc::new(Mutex::new(remove_command)),
        append,
    )
    .await
}

#[tauri::command(async)]
pub async fn move_node(
    handler: AppHandle,
    board_id: String,
    node_id: String,
    coordinates: (f32, f32, f32),
    append: Option<bool>,
) -> Result<Board, TauriFunctionError> {
    let move_node_command = MoveNodeCommand::new(node_id, coordinates);
    execute_command(
        &handler,
        &board_id,
        Arc::new(Mutex::new(move_node_command)),
        append,
    )
    .await
}

#[tauri::command(async)]
pub async fn upsert_comment(
    handler: AppHandle,
    board_id: String,
    comment: Comment,
    append: Option<bool>,
) -> Result<Board, TauriFunctionError> {
    let upsert_comment = UpsertCommentCommand::new(comment);
    execute_command(
        &handler,
        &board_id,
        Arc::new(Mutex::new(upsert_comment)),
        append,
    )
    .await
}

#[tauri::command(async)]
pub async fn remove_comment(
    handler: AppHandle,
    board_id: String,
    comment: Comment,
    append: Option<bool>,
) -> Result<Board, TauriFunctionError> {
    let remove_command = RemoveCommentCommand::new(comment);
    execute_command(
        &handler,
        &board_id,
        Arc::new(Mutex::new(remove_command)),
        append,
    )
    .await
}

#[tauri::command(async)]
pub async fn upsert_variable(
    handler: AppHandle,
    board_id: String,
    variable: Variable,
    append: Option<bool>,
) -> Result<Board, TauriFunctionError> {
    let upsert_variable = UpsertVariableCommand::new(variable);
    execute_command(
        &handler,
        &board_id,
        Arc::new(Mutex::new(upsert_variable)),
        append,
    )
    .await
}

#[tauri::command(async)]
pub async fn remove_variable(
    handler: AppHandle,
    board_id: String,
    variable: Variable,
    append: Option<bool>,
) -> Result<Board, TauriFunctionError> {
    let remove_command = RemoveVariableCommand::new(variable);
    execute_command(
        &handler,
        &board_id,
        Arc::new(Mutex::new(remove_command)),
        append,
    )
    .await
}

#[tauri::command(async)]
pub async fn connect_pins(
    handler: AppHandle,
    board_id: String,
    from_node: String,
    to_node: String,
    from_pin: String,
    to_pin: String,
    append: Option<bool>,
) -> Result<Board, TauriFunctionError> {
    let connect_command = ConnectPinsCommand::new(from_node, to_node, from_pin, to_pin);
    execute_command(
        &handler,
        &board_id,
        Arc::new(Mutex::new(connect_command)),
        append,
    )
    .await
}

#[tauri::command(async)]
pub async fn disconnect_pins(
    handler: AppHandle,
    board_id: String,
    from_node: String,
    to_node: String,
    from_pin: String,
    to_pin: String,
    append: Option<bool>,
) -> Result<Board, TauriFunctionError> {
    let disconnect_command = DisconnectPinsCommand::new(from_node, to_node, from_pin, to_pin);
    execute_command(
        &handler,
        &board_id,
        Arc::new(Mutex::new(disconnect_command)),
        append,
    )
    .await
}

#[tauri::command(async)]
pub async fn upsert_pin(
    handler: AppHandle,
    board_id: String,
    node_id: String,
    pin: Pin,
    append: Option<bool>,
) -> Result<Board, TauriFunctionError> {
    let upsert_command = UpsertPinCommand::new(node_id, pin);
    execute_command(
        &handler,
        &board_id,
        Arc::new(Mutex::new(upsert_command)),
        append,
    )
    .await
}

async fn execute_command(
    handler: &AppHandle,
    board_id: &str,
    command: Arc<Mutex<dyn Command>>,
    append: Option<bool>,
) -> Result<Board, TauriFunctionError> {
    let flow_like_state = TauriFlowLikeState::construct(handler).await?;
    let store = TauriFlowLikeState::get_project_store(handler).await?;

    let board = flow_like_state.lock().await.get_board(board_id)?;

    let mut board = board.lock().await;
    board
        .execute_command(command, flow_like_state.clone(), append.unwrap_or(false))
        .await?;

    let tmp_save = board.clone();
    drop(board);

    tmp_save.save(Some(store.clone())).await?;
    return Ok(tmp_save);
}
