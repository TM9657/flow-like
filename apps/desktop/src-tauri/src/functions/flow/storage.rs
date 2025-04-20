use anyhow::anyhow;
use flow_like::flow_like_storage::{files::store::FlowLikeStore, object_store::{MultipartUpload, ObjectMeta, PutPayload}, Path};
use serde::{Deserialize, Serialize};
use tauri::AppHandle;
use tauri_plugin_dialog::DialogExt;
use flow_like_types::{tokio::io::{AsyncReadExt, BufReader}, Bytes};

use crate::{functions::TauriFunctionError, state::TauriFlowLikeState};

async fn folder_placeholder(
    store: &FlowLikeStore,
    path: &Path,
    folder_name: &str,
) -> Result<(), TauriFunctionError> {
    let content = b"0";
    let dir_path = path.child(format!("_{}_._path", folder_name));
    store.as_generic().put(&dir_path, PutPayload::from_static(content)).await.map_err(|e| {
        anyhow!("Failed to create directory: {}", e)
    })?;
    Ok(())
}

async fn construct_storage(
    app_handle: &AppHandle,
    app_id: &str,
    prefix: &str,
    construct_dirs: bool
) -> Result<(FlowLikeStore, Path), TauriFunctionError> {
    let state = TauriFlowLikeState::construct(&app_handle).await?;
    let project_store = state.lock().await.config.read().await.stores.project_store.clone().ok_or(
        anyhow!("Project store not found")
    )?;
    let mut base_path = Path::from("apps").child(app_id).child("upload");

    for prefix in prefix.split('/') {
        if prefix.is_empty() {
            continue;
        }

        if construct_dirs {
            let exists = project_store.as_generic().head(&base_path.child(prefix)).await;
            if exists.is_err() {
                folder_placeholder(&project_store, &base_path, prefix).await?;
            }
        }
        base_path = base_path.child(prefix);
    }

    Ok((project_store, base_path))
}

async fn copy_large_file(
    store: &FlowLikeStore,
    from_path: &std::path::Path,
    to_path: &Path,
) -> Result<(), TauriFunctionError> {
    let mut reader = BufReader::new(flow_like_types::tokio::fs::File::open(from_path).await.map_err(|e| {
        anyhow!("Failed to open file: {}", e)
    })?);
    let mut upload_stream = store.as_generic().put_multipart(to_path).await.map_err(|e| {
        anyhow!("Failed to create multipart upload: {}", e)
    })?;

    let mut buffer = vec![0; 8 * 1024 * 1024];
    loop {
        let bytes_read = reader.read(&mut buffer).await.map_err(|e| {
            anyhow!("Failed to read file: {}", e)
        })?;
        if bytes_read == 0 {
            break;
        }

        let chunk = Bytes::copy_from_slice(&buffer[..bytes_read]);

        upload_stream.put_part(PutPayload::from_bytes(chunk)).await.map_err(|e| {
            anyhow!("Failed to upload file part: {}", e)
        })?;
    }

    upload_stream.complete().await.map_err(|e| {
        anyhow!("Failed to complete upload: {}", e)
    })?;
    Ok(())
}


async fn copy_directory_recursively(
    store: &FlowLikeStore,
    src_path: &std::path::Path,
    dest_path: &Path,
) -> Result<(), TauriFunctionError> {
    let dir_entries = flow_like_types::tokio::fs::read_dir(src_path).await.map_err(|e| {
        anyhow!("Failed to read directory: {}", e)
    })?;

    let mut entries = Vec::new();
    let mut dir_handle = dir_entries;

    while let Some(entry) = dir_handle.next_entry().await.map_err(|e| {
        anyhow!("Failed to read directory entry: {}", e)
    })? {
        entries.push(entry);
    }

    for entry in entries {
        let entry_path = entry.path();
        let file_name = entry_path.file_name().ok_or(anyhow!("Invalid file name"))?.to_string_lossy();
        let target_path = dest_path.child(file_name.as_ref());

        let metadata = entry.metadata().await.map_err(|e| {
            anyhow!("Failed to read metadata: {}", e)
        })?;

        if metadata.is_dir() {
            folder_placeholder(store, dest_path, &file_name).await?;
            Box::pin(copy_directory_recursively(store, &entry_path, &target_path)).await?;
        } else if metadata.is_file() {
            match copy_large_file(store, &entry_path, &target_path).await {
                Ok(_) => {},
                Err(e) => {
                    println!("Error copying file {}: {:?}", file_name, e);
                    continue;
                }
            }
        }
    }

    Ok(())
}

#[derive(Serialize, Deserialize)]
pub struct StorageItem {
    pub location: String,
    pub last_modified: String,
    pub size: usize,
    pub e_tag: Option<String>,
    pub version: Option<String>,
}

impl From<ObjectMeta> for StorageItem {
    fn from(meta: ObjectMeta) -> Self {
        Self {
            location: meta.location.to_string(),
            last_modified: meta.last_modified.to_string(),
            size: meta.size,
            e_tag: meta.e_tag,
            version: meta.version,
        }
    }
}

#[tauri::command(async)]
pub async fn storage_add(app_handle: AppHandle, app_id: String, prefix: String, folder: bool) -> Result<(), TauriFunctionError> {
    let (store, path) = construct_storage(&app_handle, &app_id, &prefix, true).await?;

    let files = {
        if folder {
            app_handle.dialog().file().blocking_pick_folders()
        } else {
            app_handle.dialog().file().blocking_pick_files()
        }
    }.ok_or(anyhow!("No files selected"))?;

    for file in files {
        let buf = file.as_path().ok_or(anyhow!("Invalid file buffer"))?.to_path_buf();
        let file_path = file.as_path().ok_or(anyhow!("Invalid file path"))?;
        let file_name = file_path.file_name().ok_or(anyhow!("Invalid file name"))?;

        if !folder {
            let file_name = file_name.to_string_lossy();
            let file_path = path.child(file_name.as_ref());

            match copy_large_file(&store, &buf, &file_path).await.map_err(|e| {
                anyhow!("Failed to copy file: {:?}", e)
            }) {
                Ok(_) => continue,
                Err(e) => {
                    println!("Error copying file: {:?}", e);
                    continue;
                }
            }
        }

        folder_placeholder(&store, &path, file_name.to_string_lossy().as_ref()).await?;
        let recursive_path = path.child(file_name.to_string_lossy().as_ref());
        copy_directory_recursively(&store, &buf, &recursive_path).await?;
    }

    Ok(())
}

#[tauri::command(async)]
pub async fn storage_remove(app_handle: AppHandle, app_id: String, prefix: String) -> Result<(), TauriFunctionError> {
    let (store, path) = construct_storage(&app_handle, &app_id, &prefix, false).await?;
    Ok(())
}

#[tauri::command(async)]
pub async fn storage_rename(app_handle: AppHandle, app_id: String, prefix: String) -> Result<(), TauriFunctionError> {
    let (store, path) = construct_storage(&app_handle, &app_id, &prefix, false).await?;
    Ok(())
}

#[tauri::command(async)]
pub async fn storage_list(app_handle: AppHandle, app_id: String, prefix: String) -> Result<Vec<StorageItem>, TauriFunctionError> {
    let (store, path) = construct_storage(&app_handle, &app_id, &prefix, false).await?;
    let items = store.as_generic().list_with_delimiter(Some(&path)).await.map_err(|e| {
        anyhow!("Failed to list items: {}", e)
    })?;
    let items: Vec<StorageItem> = items
        .objects
        .into_iter()
        .map(StorageItem::from)
        .collect();
    Ok(items)
}