use super::TauriFunctionError;
use crate::state::{TauriFlowLikeState, TauriSettingsState};
use flow_like::{
    flow::{board::Board, variable::Variable},
    vault::Vault,
};
use futures::{StreamExt, TryStreamExt};
use object_store::path::Path;
use serde_json::Value;
use std::collections::HashSet;
use tauri::AppHandle;

#[tauri::command(async)]
pub async fn get_vaults(app_handle: AppHandle) -> Result<Vec<Vault>, TauriFunctionError> {
    let mut vaults_list: Vec<Vault> = vec![];

    let profile = TauriSettingsState::current_profile(&app_handle).await?;

    let flow_like_state = TauriFlowLikeState::construct(&app_handle).await?;

    for vault in profile.vaults.iter() {
        if let Ok(vault) = Vault::load(vault.clone(), flow_like_state.clone()).await {
            let vault: Vault = vault;
            vaults_list.push(vault);
        }
    }

    Ok(vaults_list)
}

#[tauri::command(async)]
pub async fn get_vault(
    app_handle: AppHandle,
    vault_id: String,
) -> Result<Vault, TauriFunctionError> {
    let flow_like_state = TauriFlowLikeState::construct(&app_handle).await?;

    if let Ok(vault) = Vault::load(vault_id, flow_like_state).await {
        return Ok(vault);
    }

    Err(TauriFunctionError::new("Vault not found"))
}

#[tauri::command(async)]
pub async fn vault_configured(
    app_handle: AppHandle,
    vault_id: String,
) -> Result<bool, TauriFunctionError> {
    let flow_like_state = TauriFlowLikeState::construct(&app_handle).await?;

    if let Ok(vault) = Vault::load(vault_id, flow_like_state).await {
        let configured = vault.boards_configured().await;
        return Ok(configured);
    }

    Err(TauriFunctionError::new("Vault not found"))
}

#[tauri::command(async)]
pub async fn get_remote_vaults(_handler: AppHandle) {
    unimplemented!("Not implemented yet")
}

#[tauri::command(async)]
pub async fn create_vault(
    app_handle: AppHandle,
    name: String,
    description: String,
    author: String,
    bits: Vec<String>,
    template: String,
    tags: Vec<String>,
) -> Result<String, TauriFunctionError> {
    let flow_like_state = TauriFlowLikeState::construct(&app_handle).await?;

    let bits_map: HashSet<String> = bits.clone().into_iter().collect();
    let mut new_vault = Vault::new(name, description, author, bits, flow_like_state).await?;
    new_vault.tags = tags;

    if template == "blank" {
        let board = new_vault.create_board().await?;
        let board = new_vault.open_board(board, Some(false)).await?;
        let mut board = board.lock().await;
        let mut variable = Variable::new(
            "Embedding Models",
            flow_like::flow::variable::VariableType::String,
            flow_like::flow::pin::ValueType::HashSet,
        );
        variable
            .set_exposed(false)
            .set_editable(false)
            .set_default_value(serde_json::json!(bits_map));
        board.variables.insert(variable.id.clone(), variable);
        board.save(None).await?;
    }

    new_vault.save().await?;

    let mut profile = TauriSettingsState::current_profile(&app_handle).await?;
    let settings = TauriSettingsState::construct(&app_handle).await?;
    let mut settings = settings.lock().await;

    profile.vaults.push(new_vault.id.clone());

    settings
        .profiles
        .insert(profile.hub_profile.id.clone(), profile.clone());
    settings.serialize();

    Ok(new_vault.id.clone())
}

#[tauri::command(async)]
pub async fn create_vault_board(
    app_handle: AppHandle,
    vault_id: String,
    name: String,
    description: String,
) -> Result<(), TauriFunctionError> {
    let flow_like_state = TauriFlowLikeState::construct(&app_handle).await?;
    let mut vault = Vault::load(vault_id, flow_like_state).await?;

    let board = vault
        .boards
        .first()
        .ok_or(TauriFunctionError::new("No boards found"))?;
    let board = vault.open_board(board.clone(), Some(false)).await?;
    let board = board.lock().await;
    let (_var_id, variable) = board
        .variables
        .iter()
        .find(|(_, variable)| variable.name == "Embedding Models" && !variable.editable)
        .ok_or(TauriFunctionError::new("No models variable found"))?;
    let variable: Variable = variable.duplicate();
    drop(board);

    let board_id = vault.create_board().await?;
    let board = vault.open_board(board_id, Some(false)).await?;
    vault.save().await?;

    let mut board = board.lock().await;
    board.name = name;
    board.description = description;
    board.variables.insert(variable.id.clone(), variable);
    board.save(None).await?;

    Ok(())
}

#[tauri::command(async)]
pub async fn delete_vault_board(
    app_handle: AppHandle,
    vault_id: String,
    board_id: String,
) -> Result<(), TauriFunctionError> {
    let flow_like_state = TauriFlowLikeState::construct(&app_handle).await?;

    let mut vault = Vault::load(vault_id, flow_like_state).await?;
    if vault.boards.len() == 1 {
        return Err(TauriFunctionError::new("Cannot delete the last board"));
    }
    vault.delete_board(&board_id).await?;
    vault.save().await?;
    Ok(())
}

#[tauri::command(async)]
pub async fn update_vault(app_handle: AppHandle, vault: Vault) -> Result<(), TauriFunctionError> {
    let mut vault = vault;
    vault.app_state = Some(TauriFlowLikeState::construct(&app_handle).await?);
    vault.save().await?;
    Ok(())
}

#[tauri::command(async)]
pub async fn get_vault_size(
    app_handle: AppHandle,
    vault_id: String,
) -> Result<usize, TauriFunctionError> {
    let store = TauriFlowLikeState::get_project_store(&app_handle).await?;
    let path = Path::from("vaults").child(vault_id);

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
pub async fn delete_vault(
    app_handle: AppHandle,
    vault_id: String,
) -> Result<(), TauriFunctionError> {
    if vault_id.is_empty() {
        return Err(TauriFunctionError::new("Vault ID is empty"));
    };

    let store = TauriFlowLikeState::get_project_store(&app_handle).await?;
    let settings = TauriSettingsState::construct(&app_handle).await?;

    let mut settings = settings.lock().await;
    for profile in settings.profiles.values_mut() {
        profile.vaults.retain(|vault| vault != &vault_id);
    }

    let path = Path::from("vaults").child(vault_id);
    let locations = store.list(Some(&path)).map_ok(|m| m.location).boxed();
    store
        .delete_stream(locations)
        .try_collect::<Vec<Path>>()
        .await
        .map_err(|_| TauriFunctionError::new("Failed to delete vault"))?;

    settings.serialize();

    Ok(())
}

#[tauri::command(async)]
pub async fn get_vault_boards(
    app_handle: AppHandle,
    vault_id: String,
) -> Result<Vec<Board>, TauriFunctionError> {
    let flow_like_state = TauriFlowLikeState::construct(&app_handle).await?;

    let mut boards = vec![];
    if let Ok(vault) = Vault::load(vault_id, flow_like_state).await {
        for board_id in vault.boards.iter() {
            let board = vault.open_board(board_id.clone(), Some(false)).await;
            if let Ok(board) = board {
                boards.push(board.lock().await.clone());
            }
        }
    }

    Ok(boards)
}

#[tauri::command(async)]
pub async fn get_vault_board(
    app_handle: AppHandle,
    vault_id: String,
    board_id: String,
    push_to_registry: bool,
) -> Result<Board, TauriFunctionError> {
    let flow_like_state = TauriFlowLikeState::construct(&app_handle).await?;

    if let Ok(vault) = Vault::load(vault_id, flow_like_state).await {
        let board = vault.open_board(board_id, Some(push_to_registry)).await?;
        return Ok(board.lock().await.clone());
    }

    Err(TauriFunctionError::new("Board not found"))
}

#[tauri::command(async)]
pub async fn set_vault_config(
    app_handle: AppHandle,
    vault_id: String,
    board_id: String,
    variable_id: String,
    default_value: Value,
) -> Result<(), TauriFunctionError> {
    let flow_like_state = TauriFlowLikeState::construct(&app_handle).await?;

    if let Ok(vault) = Vault::load(vault_id, flow_like_state).await {
        let board = vault.open_board(board_id, Some(true)).await?;
        let mut board = board.lock().await;
        if let Some(variable) = board.variables.get_mut(&variable_id) {
            variable.default_value = Some(serde_json::to_vec(&default_value).unwrap());
        }
        board.save(None).await?;
    }

    Err(TauriFunctionError::new("Board not found"))
}
