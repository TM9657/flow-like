use crate::{Bytes, MediaError, Result};
#[derive(Clone, Debug, PartialEq)]
pub struct Frame {
    pub time: f64,
    pub width: u32,
    pub height: u32,
    pub rgba: Vec<u8>,
}
pub fn poster_png(bytes: &Bytes, time: f64) -> Result<Vec<u8>> {
    crate::browser::poster_png(bytes, time)
}
pub fn poster_frame(bytes: &Bytes, time: f64) -> Result<Frame> {
    let bytes = poster_png(bytes, time)?;
    let image = image::load_from_memory_with_format(&bytes, image::ImageFormat::Png)
        .map_err(|error| MediaError::Corrupt(error.to_string()))?
        .to_rgba8();
    Ok(Frame {
        time,
        width: image.width(),
        height: image.height(),
        rgba: image.into_raw(),
    })
}
