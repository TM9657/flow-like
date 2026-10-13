use crate::{Bytes, Frame, Result, browser};
use std::sync::Arc;

pub struct VideoFeed {
    key: String,
    current: Option<Arc<Frame>>,
}
impl VideoFeed {
    pub fn new_for_voice(bytes: Bytes, voice: u64) -> Result<Self> {
        let key = voice.to_string();
        browser::attach(&key, &browser::asset(&bytes)?);
        Ok(Self { key, current: None })
    }
    pub fn size(&self) -> (u32, u32) {
        self.current
            .as_ref()
            .map(|f| (f.width, f.height))
            .unwrap_or((0, 0))
    }
    pub fn seek(&mut self, time: f64) {
        browser::seek(&self.key, time);
    }
    pub fn frame_at(&mut self, _time: f64) -> Option<Arc<Frame>> {
        if let Some(frame) = browser::frame(&self.key) {
            self.current = Some(Arc::new(frame));
        }
        self.current.clone()
    }
    pub fn ended(&self) -> bool {
        browser::field(&browser::status_js(&self.key), "state")
            .as_string()
            .as_deref()
            == Some("ended")
    }
    pub fn frame_at_blocking(
        &mut self,
        time: f64,
        _timeout: std::time::Duration,
    ) -> Option<Arc<Frame>> {
        self.frame_at(time)
    }
}
