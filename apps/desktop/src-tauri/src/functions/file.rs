use flow_like::utils::file::FileMetadata;

use std::path::PathBuf;

#[tauri::command(async)]
pub async fn get_file_meta(file_path: String) -> FileMetadata {
    FileMetadata::new(&PathBuf::from(file_path))
}

#[tauri::command(async)]
pub async fn get_folder_meta(folder_path: String) -> Vec<FileMetadata> {
    let folder_path = PathBuf::from(folder_path);
    FileMetadata::from_folder(&folder_path)
}
