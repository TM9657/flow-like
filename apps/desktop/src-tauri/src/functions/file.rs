use anyhow::anyhow;
use flow_like::flow_like_storage::{
    self,
    files::store::StorageItem,
    object_store::{ObjectStore, path::Path},
};
use futures::StreamExt;

use std::path::PathBuf;

use crate::functions::TauriFunctionError;

#[tauri::command(async)]
pub async fn get_path_meta(path: String) -> Result<Vec<StorageItem>, TauriFunctionError> {
    let path = PathBuf::from(path);

    let object_store = flow_like_storage::object_store::local::LocalFileSystem::new_with_prefix(
        &path,
    )
    .map_err(|e| {
        eprintln!("Error creating local object store: {}", e);
        anyhow!("Failed to create local object store")
    })?;

    let mut list_stream = object_store.list(Some(&Path::default()));

    let mut items = Vec::new();
    while let Some(meta) = list_stream.next().await.transpose().unwrap() {
        let mut item = StorageItem::from(meta);
        item.location = object_store
            .path_to_filesystem(&Path::from(item.location.clone()))
            .map_err(|e| {
                eprintln!("Error converting path to filesystem: {}", e);
                anyhow!("Failed to convert path to filesystem")
            })?
            .to_string_lossy()
            .into_owned();
        items.push(item);
    }

    Ok(items)
}
