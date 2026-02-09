//! Host function bindings for WASM nodes

// Logging functions
#[link(wasm_import_module = "flowlike_log")]
extern "C" {
    #[link_name = "debug"]
    fn _log_debug(ptr: u32, len: u32);
    #[link_name = "info"]
    fn _log_info(ptr: u32, len: u32);
    #[link_name = "warn"]
    fn _log_warn(ptr: u32, len: u32);
    #[link_name = "error"]
    fn _log_error(ptr: u32, len: u32);
}

// Pin functions
#[link(wasm_import_module = "flowlike_pins")]
extern "C" {
    #[link_name = "get_input"]
    fn _get_input(name_ptr: u32, name_len: u32) -> u64;
    #[link_name = "set_output"]
    fn _set_output(name_ptr: u32, name_len: u32, value_ptr: u32, value_len: u32);
    #[link_name = "activate_exec"]
    fn _activate_exec(name_ptr: u32, name_len: u32);
}

// Variable functions
#[link(wasm_import_module = "flowlike_vars")]
extern "C" {
    #[link_name = "get"]
    fn _var_get(name_ptr: u32, name_len: u32) -> u64;
    #[link_name = "set"]
    fn _var_set(name_ptr: u32, name_len: u32, value_ptr: u32, value_len: u32);
}

// Streaming functions
#[link(wasm_import_module = "flowlike_stream")]
extern "C" {
    #[link_name = "emit"]
    fn _stream_emit(event_type_ptr: u32, event_type_len: u32, data_ptr: u32, data_len: u32);
}

// Metadata functions
#[link(wasm_import_module = "flowlike_meta")]
extern "C" {
    #[link_name = "time_now"]
    fn _time_now() -> i64;
    #[link_name = "random"]
    fn _random() -> u64;
}

// ============================================================================
// Logging
// ============================================================================

pub fn debug(message: &str) {
    unsafe {
        _log_debug(message.as_ptr() as u32, message.len() as u32);
    }
}

pub fn info(message: &str) {
    unsafe {
        _log_info(message.as_ptr() as u32, message.len() as u32);
    }
}

pub fn warn(message: &str) {
    unsafe {
        _log_warn(message.as_ptr() as u32, message.len() as u32);
    }
}

pub fn error(message: &str) {
    unsafe {
        _log_error(message.as_ptr() as u32, message.len() as u32);
    }
}

// ============================================================================
// Streaming
// ============================================================================

pub fn stream(event_type: &str, data: &str) {
    unsafe {
        _stream_emit(
            event_type.as_ptr() as u32,
            event_type.len() as u32,
            data.as_ptr() as u32,
            data.len() as u32,
        );
    }
}

pub fn stream_text(text: &str) {
    stream("text", text);
}

pub fn stream_json<T: serde::Serialize>(data: &T) {
    if let Ok(json) = serde_json::to_string(data) {
        stream("json", &json);
    }
}

pub fn stream_progress(progress: f32, message: &str) {
    let data = serde_json::json!({
        "progress": progress,
        "message": message
    });
    stream("progress", &data.to_string());
}

// ============================================================================
// Variables
// ============================================================================

pub fn get_variable(name: &str) -> Option<serde_json::Value> {
    unsafe {
        let result = _var_get(name.as_ptr() as u32, name.len() as u32);
        if result == 0 {
            return None;
        }

        let ptr = (result >> 32) as u32;
        let len = (result & 0xFFFFFFFF) as u32;

        if ptr == 0 || len == 0 {
            return None;
        }

        let slice = std::slice::from_raw_parts(ptr as *const u8, len as usize);
        serde_json::from_slice(slice).ok()
    }
}

pub fn set_variable(name: &str, value: &serde_json::Value) -> bool {
    let json = serde_json::to_vec(value).unwrap_or_default();
    unsafe {
        _var_set(
            name.as_ptr() as u32,
            name.len() as u32,
            json.as_ptr() as u32,
            json.len() as u32,
        );
    }
    true
}

// ============================================================================
// Utilities
// ============================================================================

pub fn now() -> i64 {
    unsafe { _time_now() }
}

pub fn random() -> u64 {
    unsafe { _random() }
}

pub fn read_packed_result(packed: i64) -> Option<Vec<u8>> {
    if packed == 0 {
        return None;
    }

    let ptr = (packed >> 32) as u32;
    let len = (packed & 0xFFFFFFFF) as u32;

    if ptr == 0 || len == 0 {
        return None;
    }

    unsafe {
        let slice = std::slice::from_raw_parts(ptr as *const u8, len as usize);
        Some(slice.to_vec())
    }
}
