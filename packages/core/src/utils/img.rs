use image::{DynamicImage, GenericImageView, imageops::FilterType};

pub async fn resize_image(img: &DynamicImage, max_dimension: u32) -> DynamicImage {
    let (width, height) = img.dimensions();

    if width <= max_dimension && height <= max_dimension {
        return img.clone();
    }

    let aspect_ratio = width as f32 / height as f32;

    let (new_width, new_height) = if width > height {
        (
            max_dimension,
            (max_dimension as f32 / aspect_ratio).round() as u32,
        )
    } else {
        (
            (max_dimension as f32 * aspect_ratio).round() as u32,
            max_dimension,
        )
    };

    img.resize(new_width, new_height, FilterType::Lanczos3)
}

#[cfg(test)]
mod tests {
    use super::*;
    use base64::{Engine as _, engine::general_purpose::STANDARD};
    use flow_like_types::tokio;

    #[tokio::test]
    async fn test_resize_image() {
        let data_url = crate::utils::data_url::make_data_url("https://upload.wikimedia.org/wikipedia/commons/b/b2/BMW_M2_%28F87%29_2979cc_registered_July_2019.jpg").await.unwrap();

        // We need to remove the data:image/jpeg;base64, prefix
        let data = crate::utils::data_url::data_url_to_base64(&data_url).unwrap();

        let img = image::load_from_memory(&STANDARD.decode(data).unwrap()).unwrap();
        let resized_img = resize_image(&img, 1280).await;

        assert_ne!(resized_img.dimensions(), img.dimensions());
    }
}
