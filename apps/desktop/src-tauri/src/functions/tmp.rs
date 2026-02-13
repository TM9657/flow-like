use std::path::PathBuf;

use crate::functions::TauriFunctionError;
use flow_like::utils::hash::hash_file;

#[tauri::command(async)]
pub async fn post_process_local_file(file: String) -> Result<String, TauriFunctionError> {
    let path = PathBuf::from(file);
    let hash = hash_file(&path);
    if hash.is_empty() {
        return Err(TauriFunctionError::new(
            "File does not exist or is not a file",
        ));
    }

    let extension = path
        .extension()
        .and_then(|ext| ext.to_str())
        .map(|ext| format!(".{}", ext))
        .unwrap_or_default();

    let new_file_name = path.with_file_name(format!("{}{}", hash, extension));
    std::fs::rename(path, new_file_name.clone())
        .map_err(|e| TauriFunctionError::new(&e.to_string()))?;

    Ok(new_file_name
        .canonicalize()
        .map_err(|e| TauriFunctionError::new(&e.to_string()))?
        .to_string_lossy()
        .to_string())
}
