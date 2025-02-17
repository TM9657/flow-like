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
        Board, Command, Comment,
    },
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
    let board_state = TauriFlowLikeState::board_registry(&handler).await?;
    let board_state = board_state.lock().await;
    if let Some(board) = board_state.get(&board_id) {
        let board = board.lock().await.clone();
        let file_path = handler.dialog().file().blocking_save_file();
        if let Some(file_path) = file_path {
            let board_string = serde_json::to_string(&board)
                .map_err(|e| TauriFunctionError::from(anyhow::Error::new(e)))?;
            let file_path = file_path
                .as_path()
                .ok_or(TauriFunctionError::new("Invalid file path"))?;
            std::fs::write(file_path, board_string)
                .map_err(|e| TauriFunctionError::from(anyhow::Error::new(e)))?;
        }
    }
    Err(TauriFunctionError::new("Board not found"))
}

#[tauri::command(async)]
pub async fn create_board(app_handle: AppHandle) -> Result<Board, TauriFunctionError> {
    let flow_like_state = TauriFlowLikeState::construct(&app_handle).await?;

    let path = Path::from("debug").child("boards");

    let board_state = TauriFlowLikeState::board_registry(&app_handle).await?;
    let mut board_state = board_state.lock().await;
    let board = Board::new(path, flow_like_state);
    board_state.insert(board.id.clone(), Arc::new(Mutex::new(board.clone())));
    Ok(board)
}

#[tauri::command(async)]
pub async fn get_board(handler: AppHandle, board_id: String) -> Result<Board, TauriFunctionError> {
    let board_state = TauriFlowLikeState::board_registry(&handler).await?;
    let board_state = board_state.lock().await;
    if let Some(board) = board_state.get(&board_id) {
        let board = board.lock().await.clone();
        return Ok(board);
    }
    Err(TauriFunctionError::new("Board not found"))
}

#[tauri::command(async)]
pub async fn close_board(handler: AppHandle, board_id: String) -> Result<(), TauriFunctionError> {
    let board_state = TauriFlowLikeState::board_registry(&handler).await?;
    let mut board_state = board_state.lock().await;
    let store = {
        let flow_like_state = TauriFlowLikeState::construct(&handler).await?;
        let store = flow_like_state
            .lock()
            .await
            .config
            .read()
            .await
            .project_store
            .clone();
        store
    };
    if let Some(board) = board_state.get(&board_id) {
        let board = board.lock().await.clone();
        board.save(Some(store.clone())).await?;
        board_state.remove(&board_id);
    }
    Ok(())
}

#[tauri::command(async)]
pub async fn get_open_boards(
    handler: AppHandle,
) -> Result<Vec<(String, String)>, TauriFunctionError> {
    let board_state = TauriFlowLikeState::board_registry(&handler).await?;
    let board_state = board_state.lock().await;
    let mut boards = vec![];
    for (board_id, board) in board_state.iter() {
        let board = board.lock().await;
        boards.push((board_id.clone(), board.name.clone()));
    }
    Ok(boards)
}

#[tauri::command(async)]
pub async fn update_board_meta(
    handler: AppHandle,
    board_id: String,
    name: String,
    description: String,
) -> Result<Board, TauriFunctionError> {
    let store = {
        let flow_like_state = TauriFlowLikeState::construct(&handler).await?;
        let store = flow_like_state
            .lock()
            .await
            .config
            .read()
            .await
            .project_store
            .clone();
        store
    };
    let board_state = TauriFlowLikeState::board_registry(&handler).await?;
    let board_state = board_state.lock().await;
    if let Some(board) = board_state.get(&board_id) {
        let mut board = board.lock().await;
        board.name = name;
        board.description = description;
        board.save(Some(store.clone())).await?;
        return Ok(board.clone());
    }
    Err(TauriFunctionError::new("Board not found"))
}

#[tauri::command(async)]
pub async fn undo_board(handler: AppHandle, board_id: String) -> Result<Board, TauriFunctionError> {
    let store = {
        let flow_like_state = TauriFlowLikeState::construct(&handler).await?;
        let store = flow_like_state
            .lock()
            .await
            .config
            .read()
            .await
            .project_store
            .clone();
        store
    };
    let board_state = TauriFlowLikeState::board_registry(&handler).await?;
    let board_state = board_state.lock().await;
    let state = board_state.get(&board_id).cloned();
    drop(board_state);
    if let Some(board) = state {
        let flow_like_state = TauriFlowLikeState::construct(&handler).await?;
        let mut board = board.lock().await;
        let _ = board.undo(flow_like_state).await;
        board.save(Some(store.clone())).await?;
        return Ok(board.clone());
    }
    Err(TauriFunctionError::new("Board not found"))
}

#[tauri::command(async)]
pub async fn redo_board(handler: AppHandle, board_id: String) -> Result<Board, TauriFunctionError> {
    let store = {
        let flow_like_state = TauriFlowLikeState::construct(&handler).await?;
        let store = flow_like_state
            .lock()
            .await
            .config
            .read()
            .await
            .project_store
            .clone();
        store
    };
    let board_state = TauriFlowLikeState::board_registry(&handler).await?;
    let board_state = board_state.lock().await;
    let state = board_state.get(&board_id).cloned();
    drop(board_state);
    if let Some(board) = state {
        let flow_like_state = TauriFlowLikeState::construct(&handler).await?;
        let mut board = board.lock().await;
        let _ = board.redo(flow_like_state).await;
        board.save(Some(store.clone())).await?;
        return Ok(board.clone());
    }
    Err(TauriFunctionError::new("Board not found"))
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
    let board_registry = {
        let flow_like_state = flow_like_state.lock().await;
        flow_like_state.board_registry.clone()
    };

    let store = flow_like_state
        .lock()
        .await
        .config
        .read()
        .await
        .project_store
        .clone();

    let board = board_registry.lock().await.get(board_id).cloned();

    if let Some(board) = board {
        let mut board = board.lock().await;
        board
            .execute_command(command, flow_like_state.clone(), append.unwrap_or(false))
            .await?;

        board.save(Some(store.clone())).await?;
        return Ok(board.clone());
    }
    Err(TauriFunctionError::new("Board not found"))
}
