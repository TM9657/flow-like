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
    #[link_name = "trace"]
    fn _log_trace(ptr: u32, len: u32);
    #[link_name = "log_json"]
    fn _log_json(level: u32, msg_ptr: u32, msg_len: u32, data_ptr: u32, data_len: u32);
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
    #[link_name = "delete"]
    fn _var_delete(name_ptr: u32, name_len: u32);
    #[link_name = "has"]
    fn _var_has(name_ptr: u32, name_len: u32) -> i32;
}

// Streaming functions
#[link(wasm_import_module = "flowlike_stream")]
extern "C" {
    #[link_name = "emit"]
    fn _stream_emit(event_type_ptr: u32, event_type_len: u32, data_ptr: u32, data_len: u32);
    #[link_name = "text"]
    fn _stream_text(text_ptr: u32, text_len: u32);
}

// Metadata functions
#[link(wasm_import_module = "flowlike_meta")]
extern "C" {
    #[link_name = "time_now"]
    fn _time_now() -> i64;
    #[link_name = "random"]
    fn _random() -> u64;
    #[link_name = "get_node_id"]
    fn _get_node_id() -> u64;
    #[link_name = "get_run_id"]
    fn _get_run_id() -> u64;
    #[link_name = "get_app_id"]
    fn _get_app_id() -> u64;
    #[link_name = "get_board_id"]
    fn _get_board_id() -> u64;
    #[link_name = "get_user_id"]
    fn _get_user_id() -> u64;
    #[link_name = "is_streaming"]
    fn _is_streaming() -> i32;
    #[link_name = "get_log_level"]
    fn _get_log_level() -> i32;
}

#[link(wasm_import_module = "flowlike_cache")]
extern "C" {
    #[link_name = "get"]
    fn _cache_get(key_ptr: u32, key_len: u32) -> u64;
    #[link_name = "set"]
    fn _cache_set(key_ptr: u32, key_len: u32, val_ptr: u32, val_len: u32);
    #[link_name = "delete"]
    fn _cache_delete(key_ptr: u32, key_len: u32);
    #[link_name = "has"]
    fn _cache_has(key_ptr: u32, key_len: u32) -> i32;
}

#[link(wasm_import_module = "flowlike_storage")]
extern "C" {
    #[link_name = "read_request"]
    fn _storage_read(path_ptr: u32, path_len: u32) -> u64;
    #[link_name = "write_request"]
    fn _storage_write(path_ptr: u32, path_len: u32, data_ptr: u32, data_len: u32) -> i32;
    #[link_name = "storage_dir"]
    fn _storage_dir(node_scoped: i32) -> u64;
    #[link_name = "upload_dir"]
    fn _upload_dir() -> u64;
    #[link_name = "cache_dir"]
    fn _cache_dir(node_scoped: i32, user_scoped: i32) -> u64;
    #[link_name = "user_dir"]
    fn _user_dir(node_scoped: i32) -> u64;
    #[link_name = "list_request"]
    fn _storage_list(path_ptr: u32, path_len: u32) -> u64;
}

#[link(wasm_import_module = "flowlike_models")]
extern "C" {
    #[link_name = "embed_text"]
    fn _embed_text(bit_ptr: u32, bit_len: u32, texts_ptr: u32, texts_len: u32) -> u64;
}

#[link(wasm_import_module = "flowlike_http")]
extern "C" {
    #[link_name = "request"]
    fn _http_request(
        method: u32,
        url_ptr: u32,
        url_len: u32,
        headers_ptr: u32,
        headers_len: u32,
        body_ptr: u32,
        body_len: u32,
    ) -> i32;
}

#[link(wasm_import_module = "flowlike_auth")]
extern "C" {
    #[link_name = "get_oauth_token"]
    fn _get_oauth_token(provider_ptr: u32, provider_len: u32) -> u64;
    #[link_name = "has_oauth_token"]
    fn _has_oauth_token(provider_ptr: u32, provider_len: u32) -> i32;
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

pub fn trace(message: &str) {
    unsafe {
        _log_trace(message.as_ptr() as u32, message.len() as u32);
    }
}

pub fn log_json(level: u8, message: &str, data: &serde_json::Value) {
    let data_str = serde_json::to_string(data).unwrap_or_default();
    unsafe {
        _log_json(
            level as u32,
            message.as_ptr() as u32,
            message.len() as u32,
            data_str.as_ptr() as u32,
            data_str.len() as u32,
        );
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

pub fn stream_text_raw(text: &str) {
    unsafe {
        _stream_text(text.as_ptr() as u32, text.len() as u32);
    }
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

pub fn delete_variable(name: &str) {
    unsafe {
        _var_delete(name.as_ptr() as u32, name.len() as u32);
    }
}

pub fn has_variable(name: &str) -> bool {
    unsafe { _var_has(name.as_ptr() as u32, name.len() as u32) != 0 }
}

// ============================================================================
// Cache
// ============================================================================

pub fn cache_get(key: &str) -> Option<serde_json::Value> {
    unsafe {
        let result = _cache_get(key.as_ptr() as u32, key.len() as u32);
        unpack_bytes(result).and_then(|bytes| serde_json::from_slice(&bytes).ok())
    }
}

pub fn cache_set(key: &str, value: &serde_json::Value) {
    let json = serde_json::to_vec(value).unwrap_or_default();
    unsafe {
        _cache_set(
            key.as_ptr() as u32,
            key.len() as u32,
            json.as_ptr() as u32,
            json.len() as u32,
        );
    }
}

pub fn cache_delete(key: &str) {
    unsafe {
        _cache_delete(key.as_ptr() as u32, key.len() as u32);
    }
}

pub fn cache_has(key: &str) -> bool {
    unsafe { _cache_has(key.as_ptr() as u32, key.len() as u32) != 0 }
}

// ============================================================================
// Metadata
// ============================================================================

pub fn get_node_id_from_host() -> Option<String> {
    unsafe { unpack_string(_get_node_id()) }
}

pub fn get_run_id_from_host() -> Option<String> {
    unsafe { unpack_string(_get_run_id()) }
}

pub fn get_app_id_from_host() -> Option<String> {
    unsafe { unpack_string(_get_app_id()) }
}

pub fn get_board_id_from_host() -> Option<String> {
    unsafe { unpack_string(_get_board_id()) }
}

pub fn get_user_id_from_host() -> Option<String> {
    unsafe { unpack_string(_get_user_id()) }
}

pub fn is_streaming_from_host() -> bool {
    unsafe { _is_streaming() != 0 }
}

pub fn get_log_level_from_host() -> i32 {
    unsafe { _get_log_level() }
}

// ============================================================================
// Storage
// ============================================================================

pub fn storage_dir(node_scoped: bool) -> Option<serde_json::Value> {
    unsafe {
        let result = _storage_dir(if node_scoped { 1 } else { 0 });
        unpack_bytes(result).and_then(|bytes| serde_json::from_slice(&bytes).ok())
    }
}

pub fn upload_dir() -> Option<serde_json::Value> {
    unsafe {
        let result = _upload_dir();
        unpack_bytes(result).and_then(|bytes| serde_json::from_slice(&bytes).ok())
    }
}

pub fn cache_dir(node_scoped: bool, user_scoped: bool) -> Option<serde_json::Value> {
    unsafe {
        let result = _cache_dir(
            if node_scoped { 1 } else { 0 },
            if user_scoped { 1 } else { 0 },
        );
        unpack_bytes(result).and_then(|bytes| serde_json::from_slice(&bytes).ok())
    }
}

pub fn user_dir(node_scoped: bool) -> Option<serde_json::Value> {
    unsafe {
        let result = _user_dir(if node_scoped { 1 } else { 0 });
        unpack_bytes(result).and_then(|bytes| serde_json::from_slice(&bytes).ok())
    }
}

pub fn storage_read(flow_path_json: &str) -> Option<Vec<u8>> {
    unsafe {
        let result = _storage_read(flow_path_json.as_ptr() as u32, flow_path_json.len() as u32);
        unpack_bytes(result)
    }
}

pub fn storage_write(flow_path_json: &str, data: &[u8]) -> bool {
    unsafe {
        _storage_write(
            flow_path_json.as_ptr() as u32,
            flow_path_json.len() as u32,
            data.as_ptr() as u32,
            data.len() as u32,
        ) != 0
    }
}

pub fn storage_list(flow_path_json: &str) -> Option<Vec<serde_json::Value>> {
    unsafe {
        let result = _storage_list(flow_path_json.as_ptr() as u32, flow_path_json.len() as u32);
        unpack_bytes(result).and_then(|bytes| serde_json::from_slice(&bytes).ok())
    }
}

// ============================================================================
// Models
// ============================================================================

pub fn embed_text(bit_json: &str, texts: &[String]) -> Option<Vec<Vec<f32>>> {
    let texts_json = serde_json::to_string(texts).ok()?;
    unsafe {
        let result = _embed_text(
            bit_json.as_ptr() as u32,
            bit_json.len() as u32,
            texts_json.as_ptr() as u32,
            texts_json.len() as u32,
        );
        unpack_bytes(result).and_then(|bytes| serde_json::from_slice(&bytes).ok())
    }
}

// ============================================================================
// HTTP
// ============================================================================

pub fn http_request(method: u8, url: &str, headers: &str, body: &[u8]) -> bool {
    unsafe {
        _http_request(
            method as u32,
            url.as_ptr() as u32,
            url.len() as u32,
            headers.as_ptr() as u32,
            headers.len() as u32,
            body.as_ptr() as u32,
            body.len() as u32,
        ) != 0
    }
}

// ============================================================================
// Auth
// ============================================================================

pub fn get_oauth_token(provider: &str) -> Option<String> {
    unsafe {
        unpack_string(_get_oauth_token(
            provider.as_ptr() as u32,
            provider.len() as u32,
        ))
    }
}

pub fn has_oauth_token(provider: &str) -> bool {
    unsafe { _has_oauth_token(provider.as_ptr() as u32, provider.len() as u32) != 0 }
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

fn unpack_bytes(packed: u64) -> Option<Vec<u8>> {
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

fn unpack_string(packed: u64) -> Option<String> {
    unpack_bytes(packed).and_then(|bytes| String::from_utf8(bytes).ok())
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
