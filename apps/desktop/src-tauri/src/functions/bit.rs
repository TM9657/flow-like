use std::sync::Arc;

use super::TauriFunctionError;
use crate::state::{TauriFlowLikeState, TauriSettingsState};
use flow_like::{
    bit::{Bit, BitPack},
    hub::BitSearchQuery,
};
use flow_like_types::intercom::BufferedInterComHandler;
use tauri::{AppHandle, Emitter};

#[tauri::command(async)]
pub async fn get_bit(
    app_handle: AppHandle,
    bit: String,
    hub: Option<String>,
) -> Result<Bit, TauriFunctionError> {
    let profile = TauriSettingsState::current_profile(&app_handle).await?;
    let http_client = TauriFlowLikeState::http_client(&app_handle).await?;
    let bit = profile.hub_profile.get_bit(bit, hub, http_client).await?;
    Ok(bit)
}

#[tauri::command(async)]
pub async fn is_bit_installed(app_handle: AppHandle, bit: Bit) -> Result<bool, TauriFunctionError> {
    let flow_like_state = TauriFlowLikeState::construct(&app_handle).await?;
    Ok(bit.is_installed(flow_like_state).await?)
}

#[tauri::command(async)]
pub async fn get_bit_size(app_handle: AppHandle, bit: Bit) -> Result<u64, TauriFunctionError> {
    let flow_like_state = TauriFlowLikeState::construct(&app_handle).await?;
    let pack = bit.pack(flow_like_state).await?;
    Ok(pack.size())
}

#[tauri::command(async)]
pub async fn get_pack_from_bit(
    app_handle: AppHandle,
    bit: Bit,
) -> Result<BitPack, TauriFunctionError> {
    let flow_like_state = TauriFlowLikeState::construct(&app_handle).await?;
    let pack = bit.pack(flow_like_state).await;
    if let Err(err) = &pack {
        println!("Error getting pack from bit: {}", err);
    }
    let pack = pack?;
    println!("Pack size: {}", pack.size());
    Ok(pack)
}

#[tauri::command(async)]
pub async fn search_bits(
    app_handle: AppHandle,
    query: BitSearchQuery,
) -> Result<Vec<Bit>, TauriFunctionError> {
    let profile = TauriSettingsState::current_profile(&app_handle).await?;
    let http_client = TauriFlowLikeState::http_client(&app_handle).await?;
    let bits = profile.hub_profile.search_bits(&query, http_client).await?;

    Ok(bits)
}

#[tauri::command(async)]
pub async fn download_bit(app_handle: AppHandle, bit: Bit) -> Result<Vec<Bit>, TauriFunctionError> {
    println!("Downloading bit: {}", bit.id);
    let flow_like_state = TauriFlowLikeState::construct(&app_handle).await?;
    let pack = bit.pack(flow_like_state.clone()).await?;
    let buffered_sender = Arc::new(BufferedInterComHandler::new(
        Arc::new(move |event| {
            let app_handle = app_handle.clone();
            Box::pin({
                async move {
                    let first_event = event.first();
                    if let Some(first_event) = first_event
                        && let Err(err) = app_handle.emit(&first_event.event_type, event.clone())
                    {
                        println!("Error emitting event: {}", err);
                    }
                    Ok(())
                }
            })
        }),
        Some(20),
        Some(100),
        Some(true),
    ));
    let result = pack
        .download(flow_like_state, buffered_sender.into_callback())
        .await?;
    Ok(result)
}

#[tauri::command(async)]
pub async fn delete_bit(_app_handle: AppHandle, _bit: Bit) -> bool {
    // TODO: Implement
    false
}

#[tauri::command(async)]
pub async fn get_installed_bit(
    app_handle: AppHandle,
    bits: Vec<Bit>,
) -> Result<Vec<Bit>, TauriFunctionError> {
    let pack = BitPack { bits };
    let flow_like_state = TauriFlowLikeState::construct(&app_handle).await?;
    Ok(pack.get_installed(flow_like_state).await?)
}
