//! Logging host functions
//!
//! Provides structured logging from WASM modules.

use super::LogEntry;
use serde_json::Value;

/// Log levels (matching standard levels)
#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LogLevel {
    Trace = 0,
    Debug = 1,
    Info = 2,
    Warn = 3,
    Error = 4,
}

impl From<u8> for LogLevel {
    fn from(v: u8) -> Self {
        match v {
            0 => LogLevel::Trace,
            1 => LogLevel::Debug,
            2 => LogLevel::Info,
            3 => LogLevel::Warn,
            _ => LogLevel::Error,
        }
    }
}

impl LogLevel {
    pub fn as_str(&self) -> &'static str {
        match self {
            LogLevel::Trace => "TRACE",
            LogLevel::Debug => "DEBUG",
            LogLevel::Info => "INFO",
            LogLevel::Warn => "WARN",
            LogLevel::Error => "ERROR",
        }
    }
}

impl LogEntry {
    pub fn new(level: LogLevel, message: String) -> Self {
        Self {
            level: level as u8,
            message,
            data: None,
        }
    }

    pub fn with_data(level: LogLevel, message: String, data: Value) -> Self {
        Self {
            level: level as u8,
            message,
            data: Some(data),
        }
    }

    pub fn level(&self) -> LogLevel {
        LogLevel::from(self.level)
    }
}
