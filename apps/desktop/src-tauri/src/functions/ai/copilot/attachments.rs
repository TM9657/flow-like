//! Attachment resolution and temporary image files.

use copilot_sdk::{AttachmentType, UserMessageAttachment};
use flow_like::copilot::ChatImage;
use std::path::PathBuf;

fn copilot_attachment_extension(media_type: &str) -> &'static str {
    match media_type.to_lowercase().as_str() {
        "image/jpeg" | "jpeg" | "jpg" => "jpg",
        "image/png" | "png" => "png",
        "image/gif" | "gif" => "gif",
        "image/webp" | "webp" => "webp",
        _ => "bin",
    }
}

const MAX_PROMPT_IMAGE_BYTES: usize = 64 * 1024 * 1024;

/// Decode base64 prompt images and persist them as hash-deduped temp files.
/// Shared by every provider that attaches images by path (GitHub Copilot
/// SDK attachments, Codex `--image` flags).
pub(super) fn write_chat_image_temp_files(
    images: &[ChatImage],
) -> Result<Vec<std::path::PathBuf>, String> {
    use flow_like_types::base64::{Engine as _, engine::general_purpose::STANDARD};

    let attachment_dir = std::env::temp_dir().join("flow-like-copilot-attachments");
    std::fs::create_dir_all(&attachment_dir)
        .map_err(|e| format!("Failed to create attachment directory: {}", e))?;

    images
        .iter()
        .enumerate()
        .map(|(index, image)| {
            // Bound the decoded size before allocating: base64 inflates by 4/3.
            let estimated_bytes = image.data.len() / 4 * 3;
            if estimated_bytes > MAX_PROMPT_IMAGE_BYTES {
                return Err(format!(
                    "Prompt image {} is too large ({} MB, max {} MB)",
                    index + 1,
                    estimated_bytes / (1024 * 1024),
                    MAX_PROMPT_IMAGE_BYTES / (1024 * 1024)
                ));
            }
            let bytes = STANDARD
                .decode(&image.data)
                .map_err(|e| format!("Failed to decode prompt image {}: {}", index + 1, e))?;
            let extension = copilot_attachment_extension(&image.media_type);
            let file_name = format!("{}.{}", blake3::hash(&bytes).to_hex(), extension);
            let file_path = attachment_dir.join(file_name);

            if !file_path.exists() {
                std::fs::write(&file_path, &bytes).map_err(|e| {
                    format!("Failed to write attachment {}: {}", file_path.display(), e)
                })?;
            }

            Ok(file_path)
        })
        .collect()
}

pub(super) fn build_copilot_attachments(
    images: &[ChatImage],
) -> Result<Vec<UserMessageAttachment>, String> {
    let paths = write_chat_image_temp_files(images)?;
    Ok(paths
        .into_iter()
        .enumerate()
        .map(|(index, file_path)| {
            let extension = file_path
                .extension()
                .map(|e| e.to_string_lossy().into_owned())
                .unwrap_or_else(|| "bin".to_string());
            UserMessageAttachment {
                attachment_type: AttachmentType::File,
                path: file_path.to_string_lossy().into_owned(),
                display_name: format!("prompt-image-{}.{}", index + 1, extension),
            }
        })
        .collect())
}

fn attachment_media_type(url: &str) -> String {
    let name_hint = url
        .split_once("filename=")
        .map(|(_, rest)| rest.split('&').next().unwrap_or(rest))
        .map(|encoded| urlencoding::decode(encoded).unwrap_or_default().to_string())
        .unwrap_or_else(|| url.split('?').next().unwrap_or(url).to_string());

    match name_hint.rsplit('.').next().map(str::to_ascii_lowercase) {
        Some(ext) if ext == "jpg" || ext == "jpeg" => "image/jpeg".to_string(),
        Some(ext) if ext == "gif" => "image/gif".to_string(),
        Some(ext) if ext == "webp" => "image/webp".to_string(),
        _ => "image/png".to_string(),
    }
}

/// Convert a Tauri asset-protocol URL (produced by `convertFileSrc`) back to the local file path.
fn local_asset_path(url: &str) -> Option<PathBuf> {
    let without_query = url.split('?').next().unwrap_or(url);
    let encoded_path = without_query
        .strip_prefix("asset://localhost/")
        .or_else(|| {
            without_query
                .split_once("asset.localhost/")
                .map(|(_, rest)| rest)
        })?;
    let decoded = urlencoding::decode(encoded_path).ok()?.to_string();
    // On unix the leading slash is consumed by the host split; restore it when missing.
    let path = if decoded.starts_with('/') || decoded.contains(":\\") || decoded.contains(":/") {
        PathBuf::from(decoded)
    } else {
        PathBuf::from(format!("/{decoded}"))
    };
    path.is_file().then_some(path)
}

/// Maximum size of a single fetched attachment; larger ones are skipped to bound memory use.
const MAX_ATTACHMENT_BYTES: u64 = 512 * 1024 * 1024;

/// Resolve chat attachment URLs (local tmp files via the asset protocol, or presigned tmp uploads)
/// into base64 `ChatImage`s for the model — mirrors the simple chat's attachment handling, keeping
/// large blobs out of the frontend store and IPC payloads.
pub(super) async fn resolve_attachment_images(urls: &[String]) -> Vec<ChatImage> {
    use flow_like_types::base64::{Engine as _, engine::general_purpose::STANDARD};

    let mut images = Vec::with_capacity(urls.len());
    for url in urls {
        let bytes = if let Some(path) = local_asset_path(url) {
            match tokio::fs::read(&path).await {
                Ok(bytes) => Some(bytes),
                Err(error) => {
                    eprintln!("[global_chat] failed to read local attachment: {error}");
                    None
                }
            }
        } else if url.starts_with("http://") || url.starts_with("https://") {
            match flow_like_types::reqwest::get(url).await {
                Ok(response) => {
                    // Reject oversized attachments by Content-Length before buffering the body into
                    // memory (a malicious URL could otherwise OOM the process).
                    if response
                        .content_length()
                        .is_some_and(|len| len > MAX_ATTACHMENT_BYTES)
                    {
                        eprintln!("[global_chat] attachment exceeds size limit, skipped");
                        None
                    } else {
                        match response.bytes().await {
                            Ok(bytes) if bytes.len() as u64 <= MAX_ATTACHMENT_BYTES => {
                                Some(bytes.to_vec())
                            }
                            Ok(_) => {
                                eprintln!("[global_chat] attachment exceeds size limit, skipped");
                                None
                            }
                            Err(error) => {
                                eprintln!(
                                    "[global_chat] failed to read attachment: {}",
                                    error.without_url()
                                );
                                None
                            }
                        }
                    }
                }
                Err(error) => {
                    eprintln!(
                        "[global_chat] failed to fetch attachment: {}",
                        error.without_url()
                    );
                    None
                }
            }
        } else {
            None
        };

        if let Some(bytes) = bytes {
            images.push(ChatImage {
                data: STANDARD.encode(&bytes),
                media_type: attachment_media_type(url),
            });
        }
    }
    images
}
