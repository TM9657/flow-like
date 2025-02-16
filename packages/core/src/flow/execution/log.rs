use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::time::SystemTime;

use super::LogLevel;

#[derive(Serialize, Deserialize, JsonSchema, Debug, Clone)]
pub struct LogStat {
    pub token_in: Option<u64>,
    pub token_out: Option<u64>,
    pub bit_ids: Option<Vec<String>>,
}

impl LogStat {
    pub fn new(
        token_in: Option<u64>,
        token_out: Option<u64>,
        bit_ids: Option<Vec<String>>,
    ) -> Self {
        LogStat {
            token_in,
            token_out,
            bit_ids,
        }
    }
}

#[derive(Serialize, Deserialize, JsonSchema, Debug, Clone)]
pub struct LogMessage {
    pub message: String,
    pub operation_id: Option<String>,
    pub log_level: LogLevel,
    pub stats: Option<LogStat>,
    pub start: SystemTime,
    pub end: SystemTime,
}

impl LogMessage {
    pub fn new(message: &str, log_level: LogLevel, operation_id: Option<String>) -> Self {
        let now = SystemTime::now();
        LogMessage {
            message: message.to_string(),
            log_level,
            operation_id,
            stats: None,
            start: now,
            end: now,
        }
    }

    pub fn put_stats(&mut self, stats: LogStat) {
        self.stats = Some(stats);
    }

    pub fn end(&mut self) {
        self.end = SystemTime::now();
    }
}
