use super::TauriFunctionError;
use crate::state::{TauriFlowLikeState, TauriSettingsState};
use flow_like::{
    bit::{Bit, BitPack, BitTypes},
    hub::Hub,
};
use tauri::AppHandle;

#[tauri::command(async)]
pub async fn get_bit_by_id(
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
    let pack = bit.pack(flow_like_state).await?;
    Ok(pack)
}

#[tauri::command(async)]
pub async fn get_bits_by_category(
    app_handle: AppHandle,
    bit_type: BitTypes,
) -> Result<Vec<Bit>, TauriFunctionError> {
    let profile = TauriSettingsState::current_profile(&app_handle).await?;
    let http_client = TauriFlowLikeState::http_client(&app_handle).await?;
    let bits = profile
        .hub_profile
        .get_available_bits_of_type(&bit_type, http_client)
        .await?;

    Ok(bits)
}

#[tauri::command(async)]
pub async fn get_bits(app_handle: AppHandle) -> Result<Vec<Bit>, TauriFunctionError> {
    let http_client = TauriFlowLikeState::http_client(&app_handle).await?;
    let profile = TauriSettingsState::current_profile(&app_handle).await?;

    let bits = profile.hub_profile.get_available_bits(http_client).await?;
    Ok(bits)
}

#[tauri::command(async)]
pub async fn download_bit(app_handle: AppHandle, bit: Bit) -> Result<Vec<Bit>, TauriFunctionError> {
    println!("Downloading bit: {}", bit.id);
    let flow_like_state = TauriFlowLikeState::construct(&app_handle).await?;
    let pack = bit.pack(flow_like_state.clone()).await?;
    let result = pack.download(flow_like_state).await?;
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
