//! Error types for WASM operations

use std::fmt;
use thiserror::Error;

/// Result type for WASM operations
pub type WasmResult<T> = Result<T, WasmError>;

/// Errors that can occur during WASM operations
#[derive(Error, Debug)]
pub enum WasmError {
    /// Failed to compile WASM module
    #[error("Compilation error: {message}")]
    Compilation {
        message: String,
        #[source]
        source: Option<Box<dyn std::error::Error + Send + Sync>>,
    },

    /// Failed to instantiate WASM module
    #[error("Instantiation error: {message}")]
    Instantiation {
        message: String,
        #[source]
        source: Option<Box<dyn std::error::Error + Send + Sync>>,
    },

    /// Failed to initialize host functions
    #[error("Initialization error: {0}")]
    Initialization(String),

    /// Error during WASM execution
    #[error("Execution error in {function}: {message}")]
    Execution {
        function: String,
        message: String,
        #[source]
        source: Option<Box<dyn std::error::Error + Send + Sync>>,
    },

    /// Execution exceeded time limit
    #[error("Execution timeout after {duration_ms}ms")]
    Timeout { duration_ms: u64 },

    /// Execution exceeded memory limit
    #[error("Out of memory: requested {requested} bytes, limit is {limit} bytes")]
    OutOfMemory { requested: usize, limit: usize },

    /// Execution exceeded fuel/instruction limit
    #[error("Out of fuel: execution exceeded {limit} instructions")]
    OutOfFuel { limit: u64 },

    /// Required export not found in WASM module
    #[error("Missing required export: {export_name}")]
    MissingExport { export_name: String },

    /// Export has wrong signature
    #[error("Invalid export signature for {export_name}: expected {expected}, got {actual}")]
    InvalidExportSignature {
        export_name: String,
        expected: String,
        actual: String,
    },

    /// ABI version mismatch
    #[error("ABI version mismatch: module has {module_version}, runtime requires {runtime_version}")]
    AbiMismatch {
        module_version: u32,
        runtime_version: u32,
    },

    /// Invalid node definition returned from WASM
    #[error("Invalid node definition: {message}")]
    InvalidNodeDefinition { message: String },

    /// Memory access violation
    #[error("Memory access error: {message}")]
    MemoryAccess { message: String },

    /// Host function call failed
    #[error("Host function {function} failed: {message}")]
    HostFunction { function: String, message: String },

    /// Capability not granted
    #[error("Capability not granted: {capability}")]
    CapabilityDenied { capability: String },

    /// Module not found
    #[error("Module not found: {path}")]
    ModuleNotFound { path: String },

    /// IO error
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    /// JSON serialization error
    #[error("JSON error: {0}")]
    Json(#[from] serde_json::Error),

    /// Generic error for internal issues
    #[error("Internal error: {0}")]
    Internal(String),
}

impl WasmError {
    pub fn compilation(message: impl Into<String>) -> Self {
        WasmError::Compilation {
            message: message.into(),
            source: None,
        }
    }

    pub fn compilation_with_source(
        message: impl Into<String>,
        source: impl std::error::Error + Send + Sync + 'static,
    ) -> Self {
        WasmError::Compilation {
            message: message.into(),
            source: Some(Box::new(source)),
        }
    }

    pub fn instantiation(message: impl Into<String>) -> Self {
        WasmError::Instantiation {
            message: message.into(),
            source: None,
        }
    }

    pub fn execution(function: impl Into<String>, message: impl Into<String>) -> Self {
        WasmError::Execution {
            function: function.into(),
            message: message.into(),
            source: None,
        }
    }

    pub fn host_function(function: impl Into<String>, message: impl Into<String>) -> Self {
        WasmError::HostFunction {
            function: function.into(),
            message: message.into(),
        }
    }

    pub fn memory_access(message: impl Into<String>) -> Self {
        WasmError::MemoryAccess {
            message: message.into(),
        }
    }

    pub fn invalid_node_definition(message: impl Into<String>) -> Self {
        WasmError::InvalidNodeDefinition {
            message: message.into(),
        }
    }

    pub fn capability_denied(capability: impl Into<String>) -> Self {
        WasmError::CapabilityDenied {
            capability: capability.into(),
        }
    }

    /// Convert to flow_like_types error
    pub fn into_flow_error(self) -> flow_like_types::Error {
        flow_like_types::anyhow!("WASM error: {}", self)
    }
}

/// Error codes returned from WASM to host
#[repr(i32)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WasmErrorCode {
    /// Success
    Ok = 0,
    /// Generic error
    Error = -1,
    /// Invalid argument
    InvalidArgument = -2,
    /// Not found
    NotFound = -3,
    /// Permission denied
    PermissionDenied = -4,
    /// Out of memory
    OutOfMemory = -5,
    /// Timeout
    Timeout = -6,
    /// Not implemented
    NotImplemented = -7,
    /// Internal error
    Internal = -8,
    /// Buffer too small
    BufferTooSmall = -9,
    /// Invalid UTF-8
    InvalidUtf8 = -10,
    /// JSON parse error
    JsonError = -11,
}

impl WasmErrorCode {
    pub fn from_i32(code: i32) -> Option<Self> {
        match code {
            0 => Some(WasmErrorCode::Ok),
            -1 => Some(WasmErrorCode::Error),
            -2 => Some(WasmErrorCode::InvalidArgument),
            -3 => Some(WasmErrorCode::NotFound),
            -4 => Some(WasmErrorCode::PermissionDenied),
            -5 => Some(WasmErrorCode::OutOfMemory),
            -6 => Some(WasmErrorCode::Timeout),
            -7 => Some(WasmErrorCode::NotImplemented),
            -8 => Some(WasmErrorCode::Internal),
            -9 => Some(WasmErrorCode::BufferTooSmall),
            -10 => Some(WasmErrorCode::InvalidUtf8),
            -11 => Some(WasmErrorCode::JsonError),
            _ => None,
        }
    }

    pub fn is_error(code: i32) -> bool {
        code < 0
    }
}

impl fmt::Display for WasmErrorCode {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            WasmErrorCode::Ok => write!(f, "OK"),
            WasmErrorCode::Error => write!(f, "Error"),
            WasmErrorCode::InvalidArgument => write!(f, "Invalid argument"),
            WasmErrorCode::NotFound => write!(f, "Not found"),
            WasmErrorCode::PermissionDenied => write!(f, "Permission denied"),
            WasmErrorCode::OutOfMemory => write!(f, "Out of memory"),
            WasmErrorCode::Timeout => write!(f, "Timeout"),
            WasmErrorCode::NotImplemented => write!(f, "Not implemented"),
            WasmErrorCode::Internal => write!(f, "Internal error"),
            WasmErrorCode::BufferTooSmall => write!(f, "Buffer too small"),
            WasmErrorCode::InvalidUtf8 => write!(f, "Invalid UTF-8"),
            WasmErrorCode::JsonError => write!(f, "JSON error"),
        }
    }
}
