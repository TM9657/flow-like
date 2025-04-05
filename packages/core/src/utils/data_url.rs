use std::io::Cursor;

use base64::{Engine as _, engine::general_purpose::STANDARD};
use flow_like_types::reqwest;

use super::img::resize_image;
/// If you input a valid Data URL, it will return the same URL.
/// Otherwise it will try to download the image and return a Data URL.
pub async fn make_data_url(url: &str) -> flow_like_types::Result<String> {
    if url.starts_with("data:") {
        return Ok(url.to_string());
    }

    let user_agent = "flow-like/0.1 (info@good-co.de)";
    let response = reqwest::Client::new()
        .get(url)
        .header(reqwest::header::USER_AGENT, user_agent)
        .send()
        .await?;

    let status = response.status();
    if !status.is_success() {
        return Err(flow_like_types::anyhow!("Failed to download image: {}", status));
    }
    let headers = response.headers().clone();
    let mut content_type = headers
        .get(reqwest::header::CONTENT_TYPE)
        .and_then(|value| value.to_str().ok())
        .ok_or_else(|| flow_like_types::anyhow!("Missing content type"))?;

    if !content_type.starts_with("image/") {
        // Now we check if the url path ends with an image extension
        let path = url.split('/').last().unwrap_or("");
        let path = path.split('?').next().unwrap_or("");
        let extension = path.split('.').last().unwrap_or("");

        content_type = match extension {
            "jpg" | "jpeg" => "image/jpeg",
            "png" => "image/png",
            "gif" => "image/gif",
            "webp" => "image/webp",
            "bmp" => "image/bmp",
            "ico" => "image/x-icon",
            "svg" => "image/svg+xml",
            _ => return Err(flow_like_types::anyhow!("Invalid content type")),
        };
    }

    let bytes = response.bytes().await?;

    // Create a Data URL
    let base64_encoded = STANDARD.encode(&bytes);
    Ok(format!("data:{};base64,{}", content_type, base64_encoded))
}

/// Transforms the given base64 image to JPEG and optimizes it. Max Size after optimization is 1280 px in any direction.
pub async fn optimize_data_url(url: &str) -> flow_like_types::Result<String> {
    let data_url = make_data_url(url).await?;
    let img = image::load_from_memory(&STANDARD.decode(data_url_to_base64(&data_url)?)?)?;
    let img = resize_image(&img, 1280).await;
    let img = img.to_rgb8();
    let mut cursor = Cursor::new(Vec::new());
    img.write_to(&mut cursor, image::ImageFormat::Jpeg)?;
    let base64_encoded = STANDARD.encode(cursor.into_inner());
    let new_data_url = format!("data:image/jpeg;base64,{}", base64_encoded);
    Ok(new_data_url)
}

pub fn data_url_to_base64(url: &str) -> flow_like_types::Result<&str> {
    url.split(',')
        .last()
        .ok_or_else(|| flow_like_types::anyhow!("Invalid Data URL"))
}

#[cfg(test)]
mod tests {
    use flow_like_types::tokio;

    use super::*;

    #[tokio::test]
    async fn test_make_data_url() {
        let url = "https://www.gstatic.com/webp/gallery/1.webp";
        let data_url = make_data_url(url).await.unwrap();
        assert!(data_url.starts_with("data:image/webp;base64,"));
    }

    #[tokio::test]
    async fn test_optimizing_data_url() {
        let url = "https://www.gstatic.com/webp/gallery/1.webp";
        let data_url = make_data_url(url).await.unwrap();
        assert!(data_url.starts_with("data:image/webp;base64,"));
        let optimized_data_url = optimize_data_url(&data_url).await.unwrap();
        assert!(optimized_data_url.starts_with("data:image/jpeg;base64,"));
    }
}
