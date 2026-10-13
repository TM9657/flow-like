use crate::{Bytes, MediaError, Result};
use std::hash::{Hash, Hasher};
use wasm_bindgen::prelude::*;

#[wasm_bindgen(module = "/src/browser-media.js")]
extern "C" {
    #[wasm_bindgen(js_name = setRepaint)]
    pub fn set_repaint(callback: &js_sys::Function);
    #[wasm_bindgen(js_name = hasAsset)]
    fn has_asset(key: &str) -> bool;
    #[wasm_bindgen(catch)]
    fn register(key: &str, bytes: &[u8]) -> std::result::Result<(), JsValue>;
    #[wasm_bindgen(js_name = prepare)]
    fn prepare_js(key: &str) -> js_sys::Promise;
    #[wasm_bindgen(js_name = probe)]
    pub fn probe_js(key: &str) -> JsValue;
    pub fn attach(key: &str, asset: &str);
    pub fn play(key: &str, asset: &str, options: &str);
    pub fn pause(key: &str);
    pub fn stop(key: &str);
    pub fn seek(key: &str, time: f64);
    pub fn tick(key: &str);
    #[wasm_bindgen(js_name = status)]
    pub fn status_js(key: &str) -> JsValue;
    #[wasm_bindgen(js_name = frame)]
    pub fn frame_js(key: &str) -> JsValue;
    #[wasm_bindgen(js_name = posterBytes)]
    fn poster_bytes(key: &str, time: f64) -> JsValue;
    #[wasm_bindgen(js_name = requestPoster)]
    fn request_poster_js(shape: u32, key: &str, time: f64);
    #[wasm_bindgen(js_name = takePoster)]
    pub fn take_poster() -> JsValue;
    #[wasm_bindgen(js_name = pendingPosters)]
    pub fn pending_posters() -> bool;
}

pub fn field(value: &JsValue, key: &str) -> JsValue {
    js_sys::Reflect::get(value, &JsValue::from_str(key)).unwrap_or(JsValue::UNDEFINED)
}
pub fn number(value: &JsValue, key: &str) -> f64 {
    field(value, key).as_f64().unwrap_or(0.0)
}
pub fn asset(bytes: &Bytes) -> Result<String> {
    let mut hash = std::collections::hash_map::DefaultHasher::new();
    bytes.hash(&mut hash);
    let key = format!("{}-{}", bytes.len(), hash.finish());
    if !has_asset(&key) {
        register(&key, bytes)
            .map_err(|_| MediaError::Corrupt("The browser could not load this media.".into()))?;
    }
    Ok(key)
}
pub async fn prepare(bytes: &Bytes) -> Result<()> {
    let key = asset(bytes)?;
    wasm_bindgen_futures::JsFuture::from(prepare_js(&key))
        .await
        .map_err(|_| MediaError::Corrupt("The browser could not prepare this media.".into()))?;
    Ok(())
}
pub fn poster_png(bytes: &Bytes, time: f64) -> Result<Vec<u8>> {
    let value = poster_bytes(&asset(bytes)?, time);
    if value.is_null() || value.is_undefined() {
        return Err(MediaError::Corrupt(
            "The browser is preparing the video frame.".into(),
        ));
    }
    Ok(js_sys::Uint8Array::new(&value).to_vec())
}
pub fn request_poster(shape: u32, bytes: &Bytes, time: f64) -> Result<()> {
    request_poster_js(shape, &asset(bytes)?, time);
    Ok(())
}
pub fn frame(key: &str) -> Option<crate::Frame> {
    let value = frame_js(key);
    if value.is_null() || value.is_undefined() {
        return None;
    }
    Some(crate::Frame {
        time: number(&value, "time"),
        width: number(&value, "width") as u32,
        height: number(&value, "height") as u32,
        rgba: js_sys::Uint8Array::new(&field(&value, "rgba")).to_vec(),
    })
}
