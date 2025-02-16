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
pub async fn resume_download(app_handle: AppHandle, bit: Bit) -> Result<(), TauriFunctionError> {
    let flow_like_state = TauriFlowLikeState::construct(&app_handle).await?;
    match flow_like::utils::download::download_bit(&bit, flow_like_state, 3).await {
        Ok(_) => {
            println!("Download Resumed: {}", bit.hash);
        }
        Err(e) => {
            println!("Error Resuming Download: {}", e);
        }
    }
    Ok(())
}
