use crate::{Bytes, ClipParams, browser};
use std::collections::HashMap;

pub trait AudioOut {
    fn start(&mut self, fill: Box<dyn FnMut(&mut [f32], usize, u32) + Send>) -> Result<(), String>;
    fn stop(&mut self);
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PlayState {
    Loading,
    Playing,
    Paused,
    Ended,
}
impl PlayState {
    pub fn name(self) -> &'static str {
        match self {
            Self::Loading => "loading",
            Self::Playing => "playing",
            Self::Paused => "paused",
            Self::Ended => "ended",
        }
    }
}
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct VoiceSpec {
    pub clip: ClipParams,
    pub looping: bool,
    pub duration: f64,
    pub audio: bool,
}
#[derive(Clone, Debug, PartialEq)]
pub struct VoiceStatus {
    pub state: PlayState,
    pub position: f64,
    pub duration: f64,
    pub error: Option<String>,
}

pub struct Player {
    voices: HashMap<u64, Option<String>>,
}
impl Player {
    pub fn new(_out: Option<Box<dyn AudioOut>>) -> Self {
        Self {
            voices: HashMap::new(),
        }
    }
    pub fn has_output(&self) -> bool {
        true
    }
    pub fn play(
        &mut self,
        key: u64,
        _media_key: u64,
        bytes: &Bytes,
        spec: VoiceSpec,
        from: Option<f64>,
    ) {
        match browser::asset(bytes) {
            Ok(asset) => {
                let options = serde_json::json!({
                    "trimStart": spec.clip.trim_start, "trimEnd": spec.clip.trim_end,
                    "fadeIn": spec.clip.fade_in, "fadeOut": spec.clip.fade_out,
                    "volume": if spec.audio { spec.clip.volume } else { 0.0 },
                    "looping": spec.looping, "duration": spec.duration, "from": from,
                });
                browser::play(&key.to_string(), &asset, &options.to_string());
                self.voices.insert(key, None);
            }
            Err(error) => {
                self.voices.insert(key, Some(error.to_string()));
            }
        }
    }
    pub fn pause(&mut self, key: u64) {
        browser::pause(&key.to_string());
    }
    pub fn stop(&mut self, key: u64) {
        browser::stop(&key.to_string());
        self.voices.remove(&key);
    }
    pub fn stop_all(&mut self) {
        for key in self.keys() {
            self.stop(key);
        }
    }
    pub fn seek(&mut self, key: u64, time: f64) {
        browser::seek(&key.to_string(), time);
    }
    pub fn status(&self, key: u64) -> Option<VoiceStatus> {
        let error = self.voices.get(&key)?;
        if let Some(error) = error {
            return Some(VoiceStatus {
                state: PlayState::Ended,
                position: 0.0,
                duration: 0.0,
                error: Some(error.clone()),
            });
        }
        let value = browser::status_js(&key.to_string());
        if value.is_null() || value.is_undefined() {
            return None;
        }
        let state = match browser::field(&value, "state").as_string().as_deref() {
            Some("playing") => PlayState::Playing,
            Some("loading") => PlayState::Loading,
            Some("ended") => PlayState::Ended,
            _ => PlayState::Paused,
        };
        Some(VoiceStatus {
            state,
            position: browser::number(&value, "position"),
            duration: browser::number(&value, "duration"),
            error: browser::field(&value, "error").as_string(),
        })
    }
    pub fn keys(&self) -> Vec<u64> {
        self.voices.keys().copied().collect()
    }
    pub fn any_playing(&self) -> bool {
        self.keys().iter().any(|key| {
            self.status(*key)
                .is_some_and(|s| matches!(s.state, PlayState::Playing | PlayState::Loading))
        })
    }
    pub fn tick(&mut self, _now: f64) {
        for key in self.keys() {
            browser::tick(&key.to_string());
        }
    }
}
impl Drop for Player {
    fn drop(&mut self) {
        self.stop_all();
    }
}
