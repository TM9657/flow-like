use serde::{Deserialize, Serialize};

use flow_like::utils::device::{get_cores, get_ram, get_vram};

#[derive(Serialize, Deserialize, Debug, Clone, Hash, PartialEq, Eq)]
pub struct SystemInfo {
    vram: u64,
    ram: u64,
    cores: u64,
}

#[tauri::command(async)]
pub fn get_system_info() -> SystemInfo {
    let vram = get_vram().unwrap_or(0);
    let ram = get_ram().unwrap_or(0);
    let cores = get_cores().unwrap_or(0);

    SystemInfo { vram, ram, cores }
}
