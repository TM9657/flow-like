//! Host functions for WASM modules
//!
//! These functions are imported by WASM modules to interact with the Flow-Like runtime.

pub mod linker;
pub mod logging;
pub mod metadata;
pub mod pins;
pub mod storage;
pub mod variables;
pub mod cache;
pub mod auth;
pub mod http;
pub mod streaming;

use crate::limits::WasmCapabilities;
use serde_json::Value;
use std::collections::HashMap;
use parking_lot::RwLock;

pub use linker::register_host_functions;

/// Host state accessible from host functions
#[derive(Debug)]
pub struct HostState {
    /// Granted capabilities
    pub capabilities: WasmCapabilities,
    /// Output values set by WASM
    pub outputs: RwLock<HashMap<String, Value>>,
    /// Execution pins to activate
    pub exec_pins: RwLock<Vec<String>>,
    /// Log entries from WASM
    pub logs: RwLock<Vec<LogEntry>>,
    /// Error message if any
    pub error: RwLock<Option<String>>,
    /// Result buffer for returning data to WASM
    pub result_buffer: RwLock<Vec<u8>>,
    /// Input values (set before execution)
    pub inputs: RwLock<HashMap<String, Value>>,
    /// Variables (shared with execution context)
    pub variables: RwLock<HashMap<String, Value>>,
    /// Cache entries
    pub cache: RwLock<HashMap<String, Value>>,
    /// OAuth tokens (provider_id -> token)
    pub oauth_tokens: RwLock<HashMap<String, OAuthTokenData>>,
    /// Execution metadata
    pub metadata: ExecutionMetadata,
    /// Stream events to send
    pub stream_events: RwLock<Vec<StreamEvent>>,
}

/// Log entry from WASM
#[derive(Debug, Clone)]
pub struct LogEntry {
    pub level: u8,
    pub message: String,
    pub data: Option<Value>,
}

/// OAuth token data for WASM access
#[derive(Debug, Clone)]
pub struct OAuthTokenData {
    pub access_token: String,
    pub token_type: String,
    pub expires_at: Option<i64>,
    pub refresh_token: Option<String>,
    pub scopes: Vec<String>,
}

/// Execution metadata accessible from WASM
#[derive(Debug, Clone, Default)]
pub struct ExecutionMetadata {
    pub node_id: String,
    pub run_id: String,
    pub app_id: String,
    pub board_id: String,
    pub user_id: String,
    pub stream_state: bool,
    pub log_level: u8,
}

/// Stream event from WASM
#[derive(Debug, Clone)]
pub struct StreamEvent {
    pub event_type: String,
    pub data: Value,
}

impl HostState {
    pub fn new(capabilities: WasmCapabilities) -> Self {
        Self {
            capabilities,
            outputs: RwLock::new(HashMap::new()),
            exec_pins: RwLock::new(Vec::new()),
            logs: RwLock::new(Vec::new()),
            error: RwLock::new(None),
            result_buffer: RwLock::new(Vec::new()),
            inputs: RwLock::new(HashMap::new()),
            variables: RwLock::new(HashMap::new()),
            cache: RwLock::new(HashMap::new()),
            oauth_tokens: RwLock::new(HashMap::new()),
            metadata: ExecutionMetadata::default(),
            stream_events: RwLock::new(Vec::new()),
        }
    }

    /// Check if a capability is granted
    pub fn has_capability(&self, cap: WasmCapabilities) -> bool {
        self.capabilities.has(cap)
    }

    /// Set input values before execution
    pub fn set_inputs(&self, inputs: HashMap<String, Value>) {
        *self.inputs.write() = inputs;
    }

    /// Get an input value
    pub fn get_input(&self, name: &str) -> Option<Value> {
        self.inputs.read().get(name).cloned()
    }

    /// Set an output value
    pub fn set_output(&self, name: &str, value: Value) {
        self.outputs.write().insert(name.to_string(), value);
    }

    /// Get all outputs
    pub fn get_outputs(&self) -> HashMap<String, Value> {
        self.outputs.read().clone()
    }

    /// Activate an execution pin
    pub fn activate_exec(&self, name: &str) {
        self.exec_pins.write().push(name.to_string());
    }

    /// Get activated execution pins
    pub fn get_activated_exec_pins(&self) -> Vec<String> {
        self.exec_pins.read().clone()
    }

    /// Add a log entry
    pub fn log(&self, level: u8, message: String, data: Option<Value>) {
        self.logs.write().push(LogEntry { level, message, data });
    }

    /// Get all log entries
    pub fn get_logs(&self) -> Vec<LogEntry> {
        self.logs.read().clone()
    }

    /// Set error message
    pub fn set_error(&self, error: String) {
        *self.error.write() = Some(error);
    }

    /// Get error message
    pub fn get_error(&self) -> Option<String> {
        self.error.read().clone()
    }

    /// Store result in buffer and return packed pointer+length
    pub fn store_result(&self, data: &[u8]) -> (u32, u32) {
        let mut buffer = self.result_buffer.write();
        let ptr = buffer.len() as u32;
        buffer.extend_from_slice(data);
        (ptr, data.len() as u32)
    }

    /// Set metadata
    pub fn set_metadata(&mut self, metadata: ExecutionMetadata) {
        self.metadata = metadata;
    }

    /// Add stream event
    pub fn add_stream_event(&self, event_type: String, data: Value) {
        self.stream_events.write().push(StreamEvent { event_type, data });
    }

    /// Get and clear stream events
    pub fn take_stream_events(&self) -> Vec<StreamEvent> {
        std::mem::take(&mut *self.stream_events.write())
    }

    /// Get a variable
    pub fn get_variable(&self, name: &str) -> Option<Value> {
        self.variables.read().get(name).cloned()
    }

    /// Set a variable
    pub fn set_variable(&self, name: &str, value: Value) {
        self.variables.write().insert(name.to_string(), value);
    }

    /// Stream an event to the client
    pub fn stream_event(&self, event_type: &str, data: &str) {
        let value: Value = serde_json::from_str(data).unwrap_or(Value::String(data.to_string()));
        self.add_stream_event(event_type.to_string(), value);
    }

    /// Reset state for reuse
    pub fn reset(&self) {
        self.outputs.write().clear();
        self.exec_pins.write().clear();
        self.logs.write().clear();
        *self.error.write() = None;
        self.result_buffer.write().clear();
        self.stream_events.write().clear();
    }
}
