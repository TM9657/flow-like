use crate::data::path::FlowPath;
use flow_like::flow::{execution::context::ExecutionContext, node::NodeLogic};
use flow_like_types::image::{DynamicImage, ImageBuffer, Rgba};
use hayro::{Pdf, Pixmap};
use std::sync::Arc;

pub mod page_count;
pub mod page_to_image;
pub mod pdf_to_images;

pub async fn register_functions() -> Vec<Arc<dyn NodeLogic>> {
    vec![
        Arc::new(page_count::PdfPageCountNode),
        Arc::new(page_to_image::PdfPageToImageNode),
        Arc::new(pdf_to_images::PdfToImagesNode),
    ]
}

pub(super) async fn load_pdf_from_flowpath(
    context: &mut ExecutionContext,
    flow_path: &FlowPath,
) -> flow_like_types::Result<Pdf> {
    let bytes = flow_path.get(context, false).await?;
    let data: Arc<dyn AsRef<[u8]> + Send + Sync> = Arc::new(bytes);

    Pdf::new(data).map_err(|err| flow_like_types::anyhow!("Failed to load PDF: {:?}", err))
}

pub(super) fn pixmap_to_dynamic_image(pixmap: Pixmap) -> flow_like_types::Result<DynamicImage> {
    let width = pixmap.width() as u32;
    let height = pixmap.height() as u32;
    let mut data = pixmap.take_u8();
    unpremultiply_rgba(&mut data);

    let buffer: ImageBuffer<Rgba<u8>, Vec<u8>> = ImageBuffer::from_vec(width, height, data)
        .ok_or_else(|| {
            flow_like_types::anyhow!("Failed to build image buffer from rendered PDF page")
        })?;

    Ok(DynamicImage::ImageRgba8(buffer))
}

fn unpremultiply_rgba(pixels: &mut [u8]) {
    for chunk in pixels.chunks_exact_mut(4) {
        let alpha = chunk[3] as u32;

        if alpha == 0 {
            chunk[0] = 0;
            chunk[1] = 0;
            chunk[2] = 0;
            continue;
        }

        if alpha == 255 {
            continue;
        }

        for channel in &mut chunk[..3] {
            let value = (*channel as u32 * 255 + (alpha / 2)) / alpha;
            *channel = value.min(255) as u8;
        }
    }
}

pub(super) fn validate_scale(scale: f32) -> flow_like_types::Result<()> {
    if scale <= 0.0 {
        return Err(flow_like_types::anyhow!("Scale must be greater than 0"));
    }

    Ok(())
}

pub(super) fn resolve_page_index(
    page_number: i64,
    total_pages: usize,
) -> flow_like_types::Result<usize> {
    if page_number < 1 {
        return Err(flow_like_types::anyhow!("Page number must be at least 1"));
    }

    let index = (page_number - 1) as usize;

    if index >= total_pages {
        return Err(flow_like_types::anyhow!(
            "Requested page {} but document has {} pages",
            page_number,
            total_pages
        ));
    }

    Ok(index)
}
