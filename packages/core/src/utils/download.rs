use crate::state::{FlowLikeEvent, FlowLikeState, FlowLikeStore};
use anyhow::{anyhow, bail};
use futures::StreamExt;
use object_store::path::Path;
use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::cmp::min;
use std::fs;
use std::sync::Arc;
use tokio::fs::OpenOptions;
use tokio::io::AsyncWriteExt;
use tokio::sync::{mpsc, Mutex};

#[derive(Serialize, Deserialize, Clone)]
pub struct BitDownloadEvent {
    pub max: u64,
    pub downloaded: u64,
    pub path: String,
    pub hash: String,
}

async fn get_remote_size(client: &Client, url: &str) -> u64 {
    let res = match client.head(url).send().await {
        Ok(res) => res,
        Err(e) => {
            println!("Error downloading file size: {:?}", e);
            return 0;
        }
    };

    let total_size = match res.headers().get("content-length") {
        Some(total_size) => total_size,
        None => {
            println!("Error getting file size");
            return 0;
        }
    };

    println!("Remote file size: {:?}", total_size);

    let total_size = match total_size.to_str() {
        Ok(total_size) => total_size,
        Err(_) => {
            println!("Error getting file size");
            return 0;
        }
    };

    match total_size.parse::<u64>() {
        Ok(total_size) => total_size,
        Err(_) => {
            println!("Error parsing file size");
            0
        }
    }
}

async fn publish_progress(
    bit: &crate::bit::Bit,
    sender: &mpsc::Sender<FlowLikeEvent>,
    downloaded: u64,
    path: &Path,
) -> anyhow::Result<()> {
    let event = FlowLikeEvent::new(
        &format!("download:{}", &bit.hash),
        BitDownloadEvent {
            hash: bit.hash.to_string(),
            max: bit.size.unwrap(),
            downloaded,
            path: path.to_string(),
        },
    );

    sender.send(event).await?;
    Ok(())
}

async fn remove_download(bit: &crate::bit::Bit, app_state: &Arc<Mutex<FlowLikeState>>) {
    let guard = app_state.lock().await;
    let manager = guard.download_manager();
    let mut manager = manager.lock().await;
    manager.remove_download(bit)
}

pub async fn download_bit(
    bit: &crate::bit::Bit,
    app_state: Arc<Mutex<FlowLikeState>>,
    retries: usize,
) -> anyhow::Result<Path> {
    let file_store = FlowLikeState::bit_store(&app_state).await?;

    let file_store = match file_store {
        FlowLikeStore::Local(store) => store,
        _ => bail!("Only local store supported"),
    };

    let store_path =
        Path::from(bit.hash.clone()).child(bit.file_name.clone().ok_or(anyhow!("No file name"))?);
    let path_name = file_store.path_to_filesystem(&store_path)?;
    let url = bit.download_link.clone().unwrap();
    let sender = {
        let guard = app_state.lock().await;
        let sender = guard.event_sender.clone();
        let sender = sender.lock().await;
        sender.clone()
    };
    let sender = sender.clone();

    // Another download of that type already exists
    let exists = {
        let guard = app_state.lock().await;
        let manager = guard.download_manager();
        let manager = manager.lock().await;
        manager.download_exists(bit)
    };

    if exists {
        bail!("Download already exists");
    }

    let client = {
        let guard = app_state.lock().await;
        let manager = guard.download_manager();
        let mut manager = manager.lock().await;
        manager.add_download(bit)
    };

    if client.is_none() {
        let _rem = remove_download(bit, &app_state).await;
        bail!("Download already exists");
    }

    let client = client.unwrap();
    let mut resume = false;
    let remote_size = get_remote_size(&client, &url).await;
    let mut local_size = 0;
    if path_name.exists() {
        local_size = path_name.metadata().unwrap().len();
        if local_size == remote_size {
            let _rem = remove_download(bit, &app_state).await;
            let _ = publish_progress(bit, &sender, remote_size, &store_path).await;
            return Ok(store_path);
        }

        if local_size < remote_size {
            resume = true;
            println!("Resuming download: {} to {}", &url, path_name.display());
        }

        if local_size > remote_size {
            println!(
                "Local file is bigger than remote file, deleting: {}",
                path_name.display()
            );
            fs::remove_file(&path_name).unwrap();
        }
    }

    println!("Downloading: {} to {}", &url, store_path);

    // now use range header to resume download
    let mut headers = reqwest::header::HeaderMap::new();

    if resume {
        headers.insert("Range", format!("bytes={}-", local_size).parse().unwrap());
    }

    let res = match client.get(&url).headers(headers).send().await {
        Ok(res) => res,
        Err(e) => {
            let _rem = remove_download(bit, &app_state).await;
            bail!("Error downloading file {}", e);
        }
    };

    if let Some(parent) = path_name.parent() {
        fs::create_dir_all(parent).unwrap();
    }

    let mut file = match OpenOptions::new()
        .write(true)
        .append(resume)
        .truncate(!resume)
        .create(true)
        .open(&path_name)
        .await
    {
        Ok(file) => file,
        Err(e) => {
            let _rem = remove_download(bit, &app_state).await;
            println!("Error opening file: {:?}", e);
            bail!("Error opening file {}", e);
        }
    };

    let mut downloaded: u64 = 0;

    if resume {
        downloaded = local_size;
    }

    let mut stream = res.bytes_stream();
    let mut in_buffer = 0;

    let mut hasher = blake3::Hasher::new();

    while let Some(item) = stream.next().await {
        let chunk = match item {
            Ok(chunk) => chunk,
            Err(_) => {
                continue;
            }
        };

        hasher.update(&chunk);

        match file.write(&chunk).await {
            Ok(_) => (),
            Err(_) => {
                continue;
            }
        };

        in_buffer += chunk.len();

        let new = min(downloaded + (chunk.len() as u64), remote_size);
        downloaded = new;

        // if buffer is bigger than 20 mb flush
        if in_buffer > 20_000_000 {
            let flushed = file.flush().await.is_ok();

            if flushed {
                in_buffer = 0;
            }
        }

        let _res = publish_progress(bit, &sender, new, &store_path).await;
    }

    let _ = file.flush().await;
    let _ = file.sync_all().await;

    let _rem = remove_download(bit, &app_state).await;

    let file_hash = hasher.finalize().to_hex().to_string().to_lowercase();
    if file_hash != bit.hash.to_lowercase() {
        println!(
            "Error downloading file, hash does not match, deleting __ {} != {}",
            file_hash, bit.hash
        );
        fs::remove_file(&path_name).unwrap();
        if retries > 0 {
            println!("Retrying download: {}", bit.hash);
            let result = Box::pin(download_bit(bit, app_state, retries - 1));
            return result.await;
        }
        bail!("Error downloading file, hash does not match");
    }

    Ok(store_path)
}
