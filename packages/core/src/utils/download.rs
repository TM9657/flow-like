use crate::state::FlowLikeState;
use flow_like_storage::files::store::FlowLikeStore;
use flow_like_storage::{Path, blake3};
use flow_like_types::intercom::{InterComCallback, InterComEvent};
use flow_like_types::reqwest::Client;
use flow_like_types::sync::Mutex;
use flow_like_types::tokio::fs::OpenOptions;
use flow_like_types::tokio::io::AsyncWriteExt;
use flow_like_types::{anyhow, bail, reqwest};
use futures::StreamExt;
use serde::{Deserialize, Serialize};
use std::cmp::min;
use std::fs;
use std::sync::Arc;

#[derive(Serialize, Deserialize, Clone)]
pub struct BitDownloadEvent {
    pub max: u64,
    pub downloaded: u64,
    pub path: String,
    pub hash: String,
}

async fn get_remote_size(client: &Client, url: &str) -> flow_like_types::Result<u64> {
    let res = client.head(url).send().await?;
    let total_size = res
        .headers()
        .get("content-length")
        .ok_or(anyhow!("No content length"))?;

    println!("Remote file size: {:?}", total_size);

    let total_size = total_size.to_str()?;
    let size = total_size.parse::<u64>()?;
    Ok(size)
}

async fn publish_progress(
    bit: &crate::bit::Bit,
    callback: &InterComCallback,
    downloaded: u64,
    path: &Path,
) -> flow_like_types::Result<()> {
    let event = InterComEvent::with_type(
        format!("download:{}", &bit.hash),
        BitDownloadEvent {
            hash: bit.hash.to_string(),
            max: bit.size.unwrap_or(0),
            downloaded,
            path: path.to_string(),
        },
    );

    if let Err(err) = event.call(callback).await {
        println!("Error publishing progress: {}", err);
    }

    Ok(())
}

async fn remove_download(bit: &crate::bit::Bit, app_state: &Arc<Mutex<FlowLikeState>>) {
    let manager = app_state.lock().await.download_manager();
    manager.lock().await.remove_download(bit);
}

pub async fn download_bit(
    bit: &crate::bit::Bit,
    app_state: Arc<Mutex<FlowLikeState>>,
    retries: usize,
    callback: &InterComCallback,
) -> flow_like_types::Result<Path> {
    let file_store = FlowLikeState::bit_store(&app_state).await?;

    let file_store = match file_store {
        FlowLikeStore::Local(store) => store,
        _ => bail!("Only local store supported"),
    };

    let store_path =
        Path::from(bit.hash.clone()).child(bit.file_name.clone().ok_or(anyhow!("No file name"))?);
    let path_name = file_store.path_to_filesystem(&store_path)?;
    let url = bit
        .download_link
        .clone()
        .ok_or(anyhow!("No download link"))?;

    // Another download of that type already exists
    let exists = {
        let manager = app_state.lock().await.download_manager();
        let manager = manager.lock().await;
        manager.download_exists(bit)
    };

    if exists {
        bail!("Download already exists");
    }

    let client = {
        let manager = app_state.lock().await.download_manager();
        let mut manager = manager.lock().await;
        manager.add_download(bit)
    };

    if client.is_none() {
        let _rem = remove_download(bit, &app_state).await;
        bail!("Download already exists");
    }

    let client = client.ok_or(anyhow!("No client for download"))?;
    let mut resume = false;
    let remote_size = get_remote_size(&client, &url).await;

    if remote_size.is_err() {
        if path_name.exists() {
            let _rem = remove_download(bit, &app_state).await;
            let _ = publish_progress(bit, callback, path_name.metadata()?.len(), &store_path).await;
            return Ok(store_path);
        }

        bail!("Error getting remote size");
    }

    let remote_size = remote_size?;

    let mut local_size = 0;
    if path_name.exists() {
        local_size = path_name.metadata()?.len();
        if local_size == remote_size {
            let _rem = remove_download(bit, &app_state).await;
            let _ = publish_progress(bit, callback, remote_size, &store_path).await;
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
            fs::remove_file(&path_name)?;
        }
    }

    println!("Downloading: {} to {}", &url, store_path);

    // now use range header to resume download
    let mut headers = reqwest::header::HeaderMap::new();

    if resume {
        headers.insert("Range", format!("bytes={}-", local_size).parse()?);
    }

    let res = match client.get(&url).headers(headers).send().await {
        Ok(res) => res,
        Err(e) => {
            let _rem = remove_download(bit, &app_state).await;
            bail!("Error downloading file {}", e);
        }
    };

    if let Some(parent) = path_name.parent() {
        fs::create_dir_all(parent)?;
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
    let mut hasher = blake3::Hasher::new();

    if resume {
        downloaded = local_size;
        hasher.update(&fs::read(&path_name)?);
    }

    let mut stream = res.bytes_stream();
    let mut in_buffer = 0;

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

        let _res = publish_progress(bit, callback, new, &store_path).await;
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
        fs::remove_file(&path_name)?;
        if retries > 0 {
            println!("Retrying download: {}", bit.hash);
            let result = Box::pin(download_bit(bit, app_state, retries - 1, callback));
            return result.await;
        }
        bail!("Error downloading file, hash does not match");
    }

    Ok(store_path)
}
