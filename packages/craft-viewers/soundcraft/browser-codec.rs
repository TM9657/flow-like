use crate::{AudioBuffer, AudioError, AudioInfo, FileFormat, Result};

fn unsupported() -> AudioError {
    AudioError::Unsupported(
        "compressed audio must be decoded by the browser before importing".into(),
    )
}

pub(crate) fn probe(_: &[u8], _: Option<&str>, _: FileFormat) -> Result<AudioInfo> {
    Err(unsupported())
}

pub(crate) fn decode(_: &[u8], _: Option<&str>, _: FileFormat) -> Result<(AudioInfo, AudioBuffer)> {
    Err(unsupported())
}
