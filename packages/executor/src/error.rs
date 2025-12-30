use std::fmt;

#[derive(Debug)]
pub enum ExecutorError {
    /// JWT verification failed
    Jwt(String),
    /// Storage operation failed
    Storage(String),
    /// Board loading failed
    BoardLoad(String),
    /// Run initialization failed
    RunInit(String),
    /// Execution failed
    Execution(String),
    /// Callback failed
    Callback(String),
    /// Configuration error
    Config(String),
    /// Timeout
    Timeout,
    /// Invalid request
    InvalidRequest(String),
}

impl fmt::Display for ExecutorError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ExecutorError::Jwt(msg) => write!(f, "JWT error: {}", msg),
            ExecutorError::Storage(msg) => write!(f, "Storage error: {}", msg),
            ExecutorError::BoardLoad(msg) => write!(f, "Board load error: {}", msg),
            ExecutorError::RunInit(msg) => write!(f, "Run init error: {}", msg),
            ExecutorError::Execution(msg) => write!(f, "Execution error: {}", msg),
            ExecutorError::Callback(msg) => write!(f, "Callback error: {}", msg),
            ExecutorError::Config(msg) => write!(f, "Config error: {}", msg),
            ExecutorError::Timeout => write!(f, "Execution timeout"),
            ExecutorError::InvalidRequest(msg) => write!(f, "Invalid request: {}", msg),
        }
    }
}

impl std::error::Error for ExecutorError {}

impl From<jsonwebtoken::errors::Error> for ExecutorError {
    fn from(e: jsonwebtoken::errors::Error) -> Self {
        ExecutorError::Jwt(e.to_string())
    }
}
