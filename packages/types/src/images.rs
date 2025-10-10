pub use image;
use image::GenericImageView;

pub fn is_supported_image_format(extension: &str) -> bool {
    matches!(
        extension.to_lowercase().as_str(),
        "jpg" | "jpeg" | "png" | "gif" | "bmp" | "tiff" | "avif" | "heic" | "ico"
    )
}

pub fn resize_image(img: image::DynamicImage) -> image::DynamicImage {
    let (width, height) = img.dimensions();

    if width == height {
        // Square image - resize to 1024x1024
        img.resize(1024, 1024, image::imageops::FilterType::Lanczos3)
    } else if width > height {
        img.resize_to_fill(1280, 720, image::imageops::FilterType::Lanczos3)
    } else {
        img.resize(1280, 1280, image::imageops::FilterType::Lanczos3)
    }
}

pub fn encode_as_webp(img: image::DynamicImage) -> anyhow::Result<Vec<u8>> {
    let mut buffer = Vec::new();

    let encoder = webp::Encoder::from_image(&img)
        .map_err(|e| anyhow::anyhow!("Failed to create WebP encoder: {}", e))?;
    let encoded = encoder.encode_lossless();

    buffer.extend_from_slice(&encoded);
    Ok(buffer)
}
