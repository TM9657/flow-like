use std::collections::HashMap;

use flow_like::bit::Bit;
use tauri::AppHandle;

use crate::{functions::TauriFunctionError, state::TauriFlowLikeState};

#[tauri::command(async)]
pub async fn init_downloads(
    app_handle: AppHandle,
) -> Result<HashMap<String, Bit>, TauriFunctionError> {
    let flow_like_state = TauriFlowLikeState::construct(&app_handle).await?;
    let download_manager = flow_like_state.lock().await.download_manager().clone();
    let mut download_manager = download_manager.lock().await;
    if download_manager.resumed() {
        println!("Download already Manager Resumed");
        return Ok(HashMap::new());
    }

    let dl_list: HashMap<String, Bit> = download_manager.load();

    download_manager.block_resume();

    Ok(dl_list)
}

#[tauri::command(async)]
pub async fn get_downloads(
    app_handle: AppHandle,
) -> Result<HashMap<String, Bit>, TauriFunctionError> {
    let flow_like_state = TauriFlowLikeState::construct(&app_handle).await?;
    let download_manager = flow_like_state.lock().await.download_manager().clone();
    let download_manager = download_manager.lock().await;
    let list = download_manager.download_list.clone();

    Ok(list)
}