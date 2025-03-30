use flow_like::utils::file::FileMetadata;

use std::{path::PathBuf, vec};

#[tauri::command(async)]
pub async fn get_path_meta(path: String) -> Vec<FileMetadata> {
    let path = PathBuf::from(path);
    if path.is_dir() {
        return FileMetadata::from_folder(&path);
    }
    let meta = FileMetadata::new(&PathBuf::from(path));
    return vec![meta];
}
