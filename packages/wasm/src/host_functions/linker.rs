//! Linker setup for host functions
//!
//! Registers all host functions with the wasmtime linker.

use crate::error::{WasmError, WasmResult};
use crate::host_functions::HostState;
use crate::limits::WasmCapabilities;
use crate::memory::WasmAllocator;
use wasmtime::{Caller, Linker, Memory};

/// Store data passed to host functions
pub struct StoreData {
    pub host_state: HostState,
    pub memory: Option<Memory>,
    pub allocator: Option<WasmAllocator>,
}

impl StoreData {
    pub fn new(capabilities: WasmCapabilities) -> Self {
        Self {
            host_state: HostState::new(capabilities),
            memory: None,
            allocator: None,
        }
    }
}

/// Register all host functions with the linker
pub fn register_host_functions(linker: &mut Linker<StoreData>) -> WasmResult<()> {
    register_logging_functions(linker)?;
    register_pin_functions(linker)?;
    register_variable_functions(linker)?;
    register_cache_functions(linker)?;
    register_metadata_functions(linker)?;
    register_storage_functions(linker)?;
    register_http_functions(linker)?;
    register_streaming_functions(linker)?;
    register_auth_functions(linker)?;
    register_env_functions(linker)?;

    Ok(())
}

/// Register env module functions for AssemblyScript compatibility
fn register_env_functions(linker: &mut Linker<StoreData>) -> WasmResult<()> {
    // AssemblyScript abort function
    // Called when an assertion fails or error occurs
    linker.func_wrap("env", "abort", |_caller: Caller<'_, StoreData>, _message: u32, _filename: u32, _line: u32, _column: u32| {
        // AssemblyScript passes string pointers and location info
        eprintln!("WASM abort called");
    }).map_err(|e| WasmError::Initialization(format!("Failed to register env::abort: {}", e)))?;

    // AssemblyScript host_log function used by our SDK
    linker.func_wrap("env", "host_log", |caller: Caller<'_, StoreData>, level: u32, msg_ptr: u32, msg_len: u32| {
        if let Ok(message) = read_string_from_caller(&caller, msg_ptr, msg_len) {
            caller.data().host_state.log(level as u8, message, None);
        }
    }).map_err(|e| WasmError::Initialization(format!("Failed to register env::host_log: {}", e)))?;

    // AssemblyScript host_stream function for streaming events
    linker.func_wrap("env", "host_stream", |caller: Caller<'_, StoreData>, event_type_ptr: u32, event_type_len: u32, data_ptr: u32, data_len: u32| {
        if let (Ok(event_type), Ok(data)) = (
            read_string_from_caller(&caller, event_type_ptr, event_type_len),
            read_string_from_caller(&caller, data_ptr, data_len)
        ) {
            caller.data().host_state.stream_event(&event_type, &data);
        }
    }).map_err(|e| WasmError::Initialization(format!("Failed to register env::host_stream: {}", e)))?;

    // AssemblyScript host_get_variable function
    linker.func_wrap("env", "host_get_variable", |caller: Caller<'_, StoreData>, name_ptr: u32, name_len: u32| -> i64 {
        let name = match read_string_from_caller(&caller, name_ptr, name_len) {
            Ok(n) => n,
            Err(_) => return 0,
        };

        match caller.data().host_state.get_variable(&name) {
            Some(v) => {
                let json = serde_json::to_vec(&v).unwrap_or_default();
                let (ptr, len) = caller.data().host_state.store_result(&json);
                pack_ptr_len(ptr, len) as i64
            }
            None => 0,
        }
    }).map_err(|e| WasmError::Initialization(format!("Failed to register env::host_get_variable: {}", e)))?;

    // AssemblyScript host_set_variable function
    linker.func_wrap("env", "host_set_variable", |caller: Caller<'_, StoreData>, name_ptr: u32, name_len: u32, value_ptr: u32, value_len: u32| -> i32 {
        if let (Ok(name), Ok(value_str)) = (
            read_string_from_caller(&caller, name_ptr, name_len),
            read_string_from_caller(&caller, value_ptr, value_len)
        ) {
            let value: serde_json::Value = serde_json::from_str(&value_str).unwrap_or(serde_json::Value::Null);
            caller.data().host_state.set_variable(&name, value);
            return 0; // Success
        }
        -1 // Error
    }).map_err(|e| WasmError::Initialization(format!("Failed to register env::host_set_variable: {}", e)))?;

    // AssemblyScript host_time_now function
    linker.func_wrap("env", "host_time_now", |_caller: Caller<'_, StoreData>| -> i64 {
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_millis() as i64)
            .unwrap_or(0)
    }).map_err(|e| WasmError::Initialization(format!("Failed to register env::host_time_now: {}", e)))?;

    // AssemblyScript host_random function
    linker.func_wrap("env", "host_random", |_caller: Caller<'_, StoreData>| -> i64 {
        use std::hash::{Hash, Hasher};
        let mut hasher = std::collections::hash_map::DefaultHasher::new();
        std::time::SystemTime::now().hash(&mut hasher);
        hasher.finish() as i64
    }).map_err(|e| WasmError::Initialization(format!("Failed to register env::host_random: {}", e)))?;

    Ok(())
}

fn register_logging_functions(linker: &mut Linker<StoreData>) -> WasmResult<()> {
    linker.func_wrap("flowlike_log", "trace", |caller: Caller<'_, StoreData>, msg_ptr: u32, msg_len: u32| {
        if let Ok(message) = read_string_from_caller(&caller, msg_ptr, msg_len) {
            caller.data().host_state.log(0, message, None);
        }
    }).map_err(|e| WasmError::Initialization(format!("Failed to register log_trace: {}", e)))?;

    linker.func_wrap("flowlike_log", "debug", |caller: Caller<'_, StoreData>, msg_ptr: u32, msg_len: u32| {
        if let Ok(message) = read_string_from_caller(&caller, msg_ptr, msg_len) {
            caller.data().host_state.log(1, message, None);
        }
    }).map_err(|e| WasmError::Initialization(format!("Failed to register log_debug: {}", e)))?;

    linker.func_wrap("flowlike_log", "info", |caller: Caller<'_, StoreData>, msg_ptr: u32, msg_len: u32| {
        if let Ok(message) = read_string_from_caller(&caller, msg_ptr, msg_len) {
            caller.data().host_state.log(2, message, None);
        }
    }).map_err(|e| WasmError::Initialization(format!("Failed to register log_info: {}", e)))?;

    linker.func_wrap("flowlike_log", "warn", |caller: Caller<'_, StoreData>, msg_ptr: u32, msg_len: u32| {
        if let Ok(message) = read_string_from_caller(&caller, msg_ptr, msg_len) {
            caller.data().host_state.log(3, message, None);
        }
    }).map_err(|e| WasmError::Initialization(format!("Failed to register log_warn: {}", e)))?;

    linker.func_wrap("flowlike_log", "error", |caller: Caller<'_, StoreData>, msg_ptr: u32, msg_len: u32| {
        if let Ok(message) = read_string_from_caller(&caller, msg_ptr, msg_len) {
            caller.data().host_state.log(4, message, None);
        }
    }).map_err(|e| WasmError::Initialization(format!("Failed to register log_error: {}", e)))?;

    linker.func_wrap("flowlike_log", "log_json", |caller: Caller<'_, StoreData>, level: u32, msg_ptr: u32, msg_len: u32, data_ptr: u32, data_len: u32| {
        if let (Ok(message), Ok(data_str)) = (
            read_string_from_caller(&caller, msg_ptr, msg_len),
            read_string_from_caller(&caller, data_ptr, data_len)
        ) {
            let data: Option<serde_json::Value> = serde_json::from_str(&data_str).ok();
            caller.data().host_state.log(level as u8, message, data);
        }
    }).map_err(|e| WasmError::Initialization(format!("Failed to register log_json: {}", e)))?;

    Ok(())
}

fn register_pin_functions(linker: &mut Linker<StoreData>) -> WasmResult<()> {
    linker.func_wrap("flowlike_pins", "get_input", |caller: Caller<'_, StoreData>, name_ptr: u32, name_len: u32| -> u64 {
        let name = match read_string_from_caller(&caller, name_ptr, name_len) {
            Ok(n) => n,
            Err(_) => return 0,
        };

        match caller.data().host_state.get_input(&name) {
            Some(v) => {
                let json = serde_json::to_vec(&v).unwrap_or_default();
                let (ptr, len) = caller.data().host_state.store_result(&json);
                pack_ptr_len(ptr, len)
            }
            None => 0,
        }
    }).map_err(|e| WasmError::Initialization(format!("Failed to register get_input: {}", e)))?;

    linker.func_wrap("flowlike_pins", "set_output", |caller: Caller<'_, StoreData>, name_ptr: u32, name_len: u32, value_ptr: u32, value_len: u32| {
        if let (Ok(name), Ok(value_str)) = (
            read_string_from_caller(&caller, name_ptr, name_len),
            read_string_from_caller(&caller, value_ptr, value_len)
        ) {
            if let Ok(value) = serde_json::from_str(&value_str) {
                caller.data().host_state.set_output(&name, value);
            }
        }
    }).map_err(|e| WasmError::Initialization(format!("Failed to register set_output: {}", e)))?;

    linker.func_wrap("flowlike_pins", "activate_exec", |caller: Caller<'_, StoreData>, name_ptr: u32, name_len: u32| {
        if let Ok(name) = read_string_from_caller(&caller, name_ptr, name_len) {
            caller.data().host_state.activate_exec(&name);
        }
    }).map_err(|e| WasmError::Initialization(format!("Failed to register activate_exec: {}", e)))?;

    Ok(())
}

fn register_variable_functions(linker: &mut Linker<StoreData>) -> WasmResult<()> {
    linker.func_wrap("flowlike_vars", "get", |caller: Caller<'_, StoreData>, name_ptr: u32, name_len: u32| -> u64 {
        if !caller.data().host_state.has_capability(WasmCapabilities::VARIABLES_READ) {
            return 0;
        }

        let name = match read_string_from_caller(&caller, name_ptr, name_len) {
            Ok(n) => n,
            Err(_) => return 0,
        };

        let vars = caller.data().host_state.variables.read();
        match vars.get(&name) {
            Some(v) => {
                let json = serde_json::to_vec(&v).unwrap_or_default();
                drop(vars);
                let (ptr, len) = caller.data().host_state.store_result(&json);
                pack_ptr_len(ptr, len)
            }
            None => 0,
        }
    }).map_err(|e| WasmError::Initialization(format!("Failed to register vars.get: {}", e)))?;

    linker.func_wrap("flowlike_vars", "set", |caller: Caller<'_, StoreData>, name_ptr: u32, name_len: u32, value_ptr: u32, value_len: u32| {
        if !caller.data().host_state.has_capability(WasmCapabilities::VARIABLES_WRITE) {
            return;
        }

        if let (Ok(name), Ok(value_str)) = (
            read_string_from_caller(&caller, name_ptr, name_len),
            read_string_from_caller(&caller, value_ptr, value_len)
        ) {
            if let Ok(value) = serde_json::from_str(&value_str) {
                caller.data().host_state.variables.write().insert(name, value);
            }
        }
    }).map_err(|e| WasmError::Initialization(format!("Failed to register vars.set: {}", e)))?;

    linker.func_wrap("flowlike_vars", "delete", |caller: Caller<'_, StoreData>, name_ptr: u32, name_len: u32| {
        if !caller.data().host_state.has_capability(WasmCapabilities::VARIABLES_WRITE) {
            return;
        }

        if let Ok(name) = read_string_from_caller(&caller, name_ptr, name_len) {
            caller.data().host_state.variables.write().remove(&name);
        }
    }).map_err(|e| WasmError::Initialization(format!("Failed to register vars.delete: {}", e)))?;

    linker.func_wrap("flowlike_vars", "has", |caller: Caller<'_, StoreData>, name_ptr: u32, name_len: u32| -> i32 {
        if !caller.data().host_state.has_capability(WasmCapabilities::VARIABLES_READ) {
            return 0;
        }

        let name = match read_string_from_caller(&caller, name_ptr, name_len) {
            Ok(n) => n,
            Err(_) => return 0,
        };

        if caller.data().host_state.variables.read().contains_key(&name) { 1 } else { 0 }
    }).map_err(|e| WasmError::Initialization(format!("Failed to register vars.has: {}", e)))?;

    Ok(())
}

fn register_cache_functions(linker: &mut Linker<StoreData>) -> WasmResult<()> {
    linker.func_wrap("flowlike_cache", "get", |caller: Caller<'_, StoreData>, key_ptr: u32, key_len: u32| -> u64 {
        if !caller.data().host_state.has_capability(WasmCapabilities::CACHE_READ) {
            return 0;
        }

        let key = match read_string_from_caller(&caller, key_ptr, key_len) {
            Ok(k) => k,
            Err(_) => return 0,
        };

        let cache = caller.data().host_state.cache.read();
        match cache.get(&key) {
            Some(v) => {
                let json = serde_json::to_vec(&v).unwrap_or_default();
                drop(cache);
                let (ptr, len) = caller.data().host_state.store_result(&json);
                pack_ptr_len(ptr, len)
            }
            None => 0,
        }
    }).map_err(|e| WasmError::Initialization(format!("Failed to register cache.get: {}", e)))?;

    linker.func_wrap("flowlike_cache", "set", |caller: Caller<'_, StoreData>, key_ptr: u32, key_len: u32, value_ptr: u32, value_len: u32| {
        if !caller.data().host_state.has_capability(WasmCapabilities::CACHE_WRITE) {
            return;
        }

        if let (Ok(key), Ok(value_str)) = (
            read_string_from_caller(&caller, key_ptr, key_len),
            read_string_from_caller(&caller, value_ptr, value_len)
        ) {
            if let Ok(value) = serde_json::from_str(&value_str) {
                caller.data().host_state.cache.write().insert(key, value);
            }
        }
    }).map_err(|e| WasmError::Initialization(format!("Failed to register cache.set: {}", e)))?;

    linker.func_wrap("flowlike_cache", "delete", |caller: Caller<'_, StoreData>, key_ptr: u32, key_len: u32| {
        if !caller.data().host_state.has_capability(WasmCapabilities::CACHE_WRITE) {
            return;
        }

        if let Ok(key) = read_string_from_caller(&caller, key_ptr, key_len) {
            caller.data().host_state.cache.write().remove(&key);
        }
    }).map_err(|e| WasmError::Initialization(format!("Failed to register cache.delete: {}", e)))?;

    linker.func_wrap("flowlike_cache", "has", |caller: Caller<'_, StoreData>, key_ptr: u32, key_len: u32| -> i32 {
        if !caller.data().host_state.has_capability(WasmCapabilities::CACHE_READ) {
            return 0;
        }

        let key = match read_string_from_caller(&caller, key_ptr, key_len) {
            Ok(k) => k,
            Err(_) => return 0,
        };

        if caller.data().host_state.cache.read().contains_key(&key) { 1 } else { 0 }
    }).map_err(|e| WasmError::Initialization(format!("Failed to register cache.has: {}", e)))?;

    Ok(())
}

fn register_metadata_functions(linker: &mut Linker<StoreData>) -> WasmResult<()> {
    linker.func_wrap("flowlike_meta", "get_node_id", |caller: Caller<'_, StoreData>| -> u64 {
        let id = &caller.data().host_state.metadata.node_id;
        let (ptr, len) = caller.data().host_state.store_result(id.as_bytes());
        pack_ptr_len(ptr, len)
    }).map_err(|e| WasmError::Initialization(format!("Failed to register get_node_id: {}", e)))?;

    linker.func_wrap("flowlike_meta", "get_run_id", |caller: Caller<'_, StoreData>| -> u64 {
        let id = &caller.data().host_state.metadata.run_id;
        let (ptr, len) = caller.data().host_state.store_result(id.as_bytes());
        pack_ptr_len(ptr, len)
    }).map_err(|e| WasmError::Initialization(format!("Failed to register get_run_id: {}", e)))?;

    linker.func_wrap("flowlike_meta", "get_app_id", |caller: Caller<'_, StoreData>| -> u64 {
        let id = &caller.data().host_state.metadata.app_id;
        let (ptr, len) = caller.data().host_state.store_result(id.as_bytes());
        pack_ptr_len(ptr, len)
    }).map_err(|e| WasmError::Initialization(format!("Failed to register get_app_id: {}", e)))?;

    linker.func_wrap("flowlike_meta", "get_board_id", |caller: Caller<'_, StoreData>| -> u64 {
        let id = &caller.data().host_state.metadata.board_id;
        let (ptr, len) = caller.data().host_state.store_result(id.as_bytes());
        pack_ptr_len(ptr, len)
    }).map_err(|e| WasmError::Initialization(format!("Failed to register get_board_id: {}", e)))?;

    linker.func_wrap("flowlike_meta", "get_user_id", |caller: Caller<'_, StoreData>| -> u64 {
        let id = &caller.data().host_state.metadata.user_id;
        let (ptr, len) = caller.data().host_state.store_result(id.as_bytes());
        pack_ptr_len(ptr, len)
    }).map_err(|e| WasmError::Initialization(format!("Failed to register get_user_id: {}", e)))?;

    linker.func_wrap("flowlike_meta", "is_streaming", |caller: Caller<'_, StoreData>| -> i32 {
        if caller.data().host_state.metadata.stream_state { 1 } else { 0 }
    }).map_err(|e| WasmError::Initialization(format!("Failed to register is_streaming: {}", e)))?;

    linker.func_wrap("flowlike_meta", "get_log_level", |caller: Caller<'_, StoreData>| -> i32 {
        caller.data().host_state.metadata.log_level as i32
    }).map_err(|e| WasmError::Initialization(format!("Failed to register get_log_level: {}", e)))?;

    linker.func_wrap("flowlike_meta", "time_now", |_caller: Caller<'_, StoreData>| -> i64 {
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_millis() as i64)
            .unwrap_or(0)
    }).map_err(|e| WasmError::Initialization(format!("Failed to register time_now: {}", e)))?;

    linker.func_wrap("flowlike_meta", "random", |_caller: Caller<'_, StoreData>| -> u64 {
        use std::collections::hash_map::RandomState;
        use std::hash::{BuildHasher, Hasher};
        RandomState::new().build_hasher().finish()
    }).map_err(|e| WasmError::Initialization(format!("Failed to register random: {}", e)))?;

    Ok(())
}

fn register_storage_functions(linker: &mut Linker<StoreData>) -> WasmResult<()> {
    linker.func_wrap("flowlike_storage", "read_request", |caller: Caller<'_, StoreData>, _path_ptr: u32, _path_len: u32| -> u64 {
        if !caller.data().host_state.has_capability(WasmCapabilities::STORAGE_READ) {
            return 0;
        }
        // Async storage handled separately
        0
    }).map_err(|e| WasmError::Initialization(format!("Failed to register storage.read_request: {}", e)))?;

    linker.func_wrap("flowlike_storage", "write_request", |caller: Caller<'_, StoreData>, _path_ptr: u32, _path_len: u32, _data_ptr: u32, _data_len: u32| -> i32 {
        if !caller.data().host_state.has_capability(WasmCapabilities::STORAGE_WRITE) {
            return -1;
        }
        // Async storage handled separately
        0
    }).map_err(|e| WasmError::Initialization(format!("Failed to register storage.write_request: {}", e)))?;

    Ok(())
}

fn register_http_functions(linker: &mut Linker<StoreData>) -> WasmResult<()> {
    linker.func_wrap("flowlike_http", "request", |caller: Caller<'_, StoreData>, _method: i32, _url_ptr: u32, _url_len: u32, _headers_ptr: u32, _headers_len: u32, _body_ptr: u32, _body_len: u32| -> i32 {
        if !caller.data().host_state.has_capability(WasmCapabilities::HTTP_REQUEST) {
            return -1;
        }
        // Async HTTP handled separately
        0
    }).map_err(|e| WasmError::Initialization(format!("Failed to register http.request: {}", e)))?;

    Ok(())
}

fn register_streaming_functions(linker: &mut Linker<StoreData>) -> WasmResult<()> {
    linker.func_wrap("flowlike_stream", "emit", |caller: Caller<'_, StoreData>, event_ptr: u32, event_len: u32, data_ptr: u32, data_len: u32| {
        if !caller.data().host_state.has_capability(WasmCapabilities::STREAMING) {
            return;
        }

        if let (Ok(event_type), Ok(data_str)) = (
            read_string_from_caller(&caller, event_ptr, event_len),
            read_string_from_caller(&caller, data_ptr, data_len)
        ) {
            if let Ok(data) = serde_json::from_str(&data_str) {
                caller.data().host_state.add_stream_event(event_type, data);
            }
        }
    }).map_err(|e| WasmError::Initialization(format!("Failed to register stream.emit: {}", e)))?;

    linker.func_wrap("flowlike_stream", "text", |caller: Caller<'_, StoreData>, text_ptr: u32, text_len: u32| {
        if !caller.data().host_state.has_capability(WasmCapabilities::STREAMING) {
            return;
        }

        if let Ok(text) = read_string_from_caller(&caller, text_ptr, text_len) {
            caller.data().host_state.add_stream_event("text".to_string(), serde_json::json!(text));
        }
    }).map_err(|e| WasmError::Initialization(format!("Failed to register stream.text: {}", e)))?;

    Ok(())
}

fn register_auth_functions(linker: &mut Linker<StoreData>) -> WasmResult<()> {
    linker.func_wrap("flowlike_auth", "get_oauth_token", |caller: Caller<'_, StoreData>, provider_ptr: u32, provider_len: u32| -> u64 {
        if !caller.data().host_state.has_capability(WasmCapabilities::OAUTH_ACCESS) {
            return 0;
        }

        let provider = match read_string_from_caller(&caller, provider_ptr, provider_len) {
            Ok(p) => p,
            Err(_) => return 0,
        };

        let tokens = caller.data().host_state.oauth_tokens.read();
        match tokens.get(&provider) {
            Some(token) => {
                let json = serde_json::json!({
                    "access_token": token.access_token,
                    "token_type": token.token_type,
                    "expires_at": token.expires_at,
                });
                let bytes = serde_json::to_vec(&json).unwrap_or_default();
                drop(tokens);
                let (ptr, len) = caller.data().host_state.store_result(&bytes);
                pack_ptr_len(ptr, len)
            }
            None => 0,
        }
    }).map_err(|e| WasmError::Initialization(format!("Failed to register get_oauth_token: {}", e)))?;

    linker.func_wrap("flowlike_auth", "has_oauth_token", |caller: Caller<'_, StoreData>, provider_ptr: u32, provider_len: u32| -> i32 {
        if !caller.data().host_state.has_capability(WasmCapabilities::OAUTH_ACCESS) {
            return 0;
        }

        let provider = match read_string_from_caller(&caller, provider_ptr, provider_len) {
            Ok(p) => p,
            Err(_) => return 0,
        };

        if caller.data().host_state.oauth_tokens.read().contains_key(&provider) { 1 } else { 0 }
    }).map_err(|e| WasmError::Initialization(format!("Failed to register has_oauth_token: {}", e)))?;

    Ok(())
}

/// Read a string from WASM memory using caller context
fn read_string_from_caller(caller: &Caller<'_, StoreData>, ptr: u32, len: u32) -> Result<String, ()> {
    let memory = caller.data().memory.ok_or(())?;
    let data = memory.data(caller);

    let start = ptr as usize;
    let end = start.checked_add(len as usize).ok_or(())?;

    if end > data.len() {
        return Err(());
    }

    String::from_utf8(data[start..end].to_vec()).map_err(|_| ())
}

/// Pack pointer and length into single u64 (ptr in high 32 bits, len in low 32 bits)
fn pack_ptr_len(ptr: u32, len: u32) -> u64 {
    ((ptr as u64) << 32) | (len as u64)
}
