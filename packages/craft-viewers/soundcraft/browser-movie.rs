// Video files are opened in Flows Video. Audio's browser build has no video codecs.
#[derive(Debug)]
pub struct Movie {
    info: MovieInfo,
}

impl Movie {
    pub fn open(_: impl Into<Arc<[u8]>>) -> Result<Movie> {
        Err(VideoError::Unsupported(
            "Open video files in Flows Video.".into(),
        ))
    }

    pub fn open_source(_: SharedSource) -> Result<Movie> {
        Err(VideoError::Unsupported(
            "Open video files in Flows Video.".into(),
        ))
    }

    pub fn info(&self) -> &MovieInfo {
        &self.info
    }

    pub fn frame_at(&mut self, _: f64) -> Result<Arc<Frame>> {
        Err(VideoError::Unsupported(
            "Open video files in Flows Video.".into(),
        ))
    }

    pub fn thumbnail_at(&mut self, _: f64, _: u32) -> Result<Arc<Frame>> {
        Err(VideoError::Unsupported(
            "Open video files in Flows Video.".into(),
        ))
    }
}
