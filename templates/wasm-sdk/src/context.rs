//! Execution context wrapper for WASM nodes

use crate::types::{ExecutionInput, ExecutionResult, LogLevel};
use serde_json::Value;
use std::collections::HashMap;

/// Execution context for a WASM node
pub struct Context {
    input: ExecutionInput,
    result: ExecutionResult,
    outputs: HashMap<String, Value>,
}

impl Context {
    /// Create a new context from execution input
    pub fn from_input(input: ExecutionInput) -> Self {
        Self {
            input,
            result: ExecutionResult::success(),
            outputs: HashMap::new(),
        }
    }

    /// Parse context from JSON bytes
    pub fn from_bytes(bytes: &[u8]) -> Result<Self, String> {
        serde_json::from_slice::<ExecutionInput>(bytes)
            .map(Self::from_input)
            .map_err(|e| format!("Failed to parse execution input: {}", e))
    }

    /// Get the node ID
    pub fn node_id(&self) -> &str {
        &self.input.node_id
    }

    /// Get the node name (for multi-node packages)
    pub fn node_name(&self) -> &str {
        &self.input.node_name
    }

    /// Get the run ID
    pub fn run_id(&self) -> &str {
        &self.input.run_id
    }

    /// Get the app ID
    pub fn app_id(&self) -> &str {
        &self.input.app_id
    }

    /// Get the board ID
    pub fn board_id(&self) -> &str {
        &self.input.board_id
    }

    /// Get the user ID
    pub fn user_id(&self) -> &str {
        &self.input.user_id
    }

    /// Check if streaming is enabled
    pub fn stream_enabled(&self) -> bool {
        self.input.stream_state
    }

    /// Get the log level
    pub fn log_level(&self) -> u8 {
        self.input.log_level
    }

    /// Get an input value by name
    pub fn get_input(&self, name: &str) -> Option<&Value> {
        self.input.inputs.get(name)
    }

    /// Get an input value and deserialize it
    pub fn get_input_as<T: serde::de::DeserializeOwned>(&self, name: &str) -> Option<T> {
        self.get_input(name)
            .and_then(|v| serde_json::from_value(v.clone()).ok())
    }

    /// Get a required input value
    pub fn require_input(&self, name: &str) -> Result<&Value, String> {
        self.get_input(name)
            .ok_or_else(|| format!("Missing required input: {}", name))
    }

    /// Get a required input and deserialize it
    pub fn require_input_as<T: serde::de::DeserializeOwned>(&self, name: &str) -> Result<T, String> {
        let value = self.require_input(name)?;
        serde_json::from_value(value.clone())
            .map_err(|e| format!("Failed to deserialize input '{}': {}", name, e))
    }

    /// Get input as string
    pub fn get_string(&self, name: &str) -> Option<String> {
        self.get_input(name).and_then(|v| v.as_str().map(|s| s.to_string()))
    }

    /// Get input as i64
    pub fn get_i64(&self, name: &str) -> Option<i64> {
        self.get_input(name).and_then(|v| v.as_i64())
    }

    /// Get input as f64
    pub fn get_f64(&self, name: &str) -> Option<f64> {
        self.get_input(name).and_then(|v| v.as_f64())
    }

    /// Get input as bool
    pub fn get_bool(&self, name: &str) -> Option<bool> {
        self.get_input(name).and_then(|v| v.as_bool())
    }

    /// Set an output value
    pub fn set_output(&mut self, name: &str, value: impl Into<Value>) {
        self.outputs.insert(name.to_string(), value.into());
    }

    /// Set output as JSON
    pub fn set_output_json<T: serde::Serialize>(&mut self, name: &str, value: &T) {
        if let Ok(v) = serde_json::to_value(value) {
            self.outputs.insert(name.to_string(), v);
        }
    }

    /// Activate an execution output pin
    pub fn activate_exec(&mut self, pin_name: &str) {
        self.result.activate_exec.push(pin_name.to_string());
    }

    /// Mark the execution as pending (for long-running operations)
    pub fn set_pending(&mut self, pending: bool) {
        self.result.pending = Some(pending);
    }

    /// Set an error on the result
    pub fn set_error(&mut self, error: impl Into<String>) {
        self.result.error = Some(error.into());
    }

    /// Check if should log at level
    pub fn should_log(&self, level: LogLevel) -> bool {
        (level as u8) >= self.input.log_level
    }

    /// Log a debug message
    pub fn debug(&self, message: &str) {
        if self.should_log(LogLevel::Debug) {
            crate::host::debug(message);
        }
    }

    /// Log an info message
    pub fn info(&self, message: &str) {
        if self.should_log(LogLevel::Info) {
            crate::host::info(message);
        }
    }

    /// Log a warning message
    pub fn warn(&self, message: &str) {
        if self.should_log(LogLevel::Warn) {
            crate::host::warn(message);
        }
    }

    /// Log an error message
    pub fn error(&self, message: &str) {
        if self.should_log(LogLevel::Error) {
            crate::host::error(message);
        }
    }

    /// Stream text if streaming is enabled
    pub fn stream_text(&self, text: &str) {
        if self.stream_enabled() {
            crate::host::stream_text(text);
        }
    }

    /// Stream JSON data if streaming is enabled
    pub fn stream_json<T: serde::Serialize>(&self, data: &T) {
        if self.stream_enabled() {
            crate::host::stream_json(data);
        }
    }

    /// Stream progress if streaming is enabled
    pub fn stream_progress(&self, progress: f32, message: &str) {
        if self.stream_enabled() {
            crate::host::stream_progress(progress, message);
        }
    }

    /// Finalize and get the execution result
    pub fn finish(mut self) -> ExecutionResult {
        for (name, value) in self.outputs {
            self.result.outputs.insert(name, value);
        }
        self.result
    }

    /// Finalize with success, activating default exec output
    pub fn success(mut self) -> ExecutionResult {
        self.activate_exec("exec_out");
        self.finish()
    }

    /// Finalize with an error
    pub fn fail(mut self, error: impl Into<String>) -> ExecutionResult {
        self.set_error(error);
        self.finish()
    }
}
#[cfg(test)]
mod tests {
    use super::*;

    fn create_test_input() -> ExecutionInput {
        ExecutionInput {
            node_id: "test-node-123".to_string(),
            run_id: "run-456".to_string(),
            app_id: "app-789".to_string(),
            board_id: "board-012".to_string(),
            user_id: "user-345".to_string(),
            stream_state: true,
            log_level: 0,
            inputs: {
                let mut map = HashMap::new();
                map.insert("text".to_string(), serde_json::json!("hello"));
                map.insert("number".to_string(), serde_json::json!(42));
                map.insert("float".to_string(), serde_json::json!(3.14));
                map.insert("flag".to_string(), serde_json::json!(true));
                map
            },
        }
    }

    #[test]
    fn test_context_creation() {
        let input = create_test_input();
        let ctx = Context::from_input(input);

        assert_eq!(ctx.node_id(), "test-node-123");
        assert_eq!(ctx.run_id(), "run-456");
        assert_eq!(ctx.app_id(), "app-789");
        assert_eq!(ctx.board_id(), "board-012");
        assert_eq!(ctx.user_id(), "user-345");
    }

    #[test]
    fn test_get_string_input() {
        let input = create_test_input();
        let ctx = Context::from_input(input);

        assert_eq!(ctx.get_string("text"), Some("hello".to_string()));
        assert_eq!(ctx.get_string("nonexistent"), None);
    }

    #[test]
    fn test_get_i64_input() {
        let input = create_test_input();
        let ctx = Context::from_input(input);

        assert_eq!(ctx.get_i64("number"), Some(42));
        assert_eq!(ctx.get_i64("nonexistent"), None);
    }

    #[test]
    fn test_get_f64_input() {
        let input = create_test_input();
        let ctx = Context::from_input(input);

        assert_eq!(ctx.get_f64("float"), Some(3.14));
        assert_eq!(ctx.get_f64("nonexistent"), None);
    }

    #[test]
    fn test_get_bool_input() {
        let input = create_test_input();
        let ctx = Context::from_input(input);

        assert_eq!(ctx.get_bool("flag"), Some(true));
        assert_eq!(ctx.get_bool("nonexistent"), None);
    }

    #[test]
    fn test_set_output() {
        let input = create_test_input();
        let mut ctx = Context::from_input(input);

        ctx.set_output("result", "output value");
        ctx.set_output("count", 100);

        let result = ctx.finish();
        assert_eq!(result.outputs.get("result"), Some(&serde_json::json!("output value")));
        assert_eq!(result.outputs.get("count"), Some(&serde_json::json!(100)));
    }

    #[test]
    fn test_activate_exec() {
        let input = create_test_input();
        let mut ctx = Context::from_input(input);

        ctx.activate_exec("branch_a");
        ctx.activate_exec("branch_b");

        let result = ctx.finish();
        assert!(result.activate_exec.contains(&"branch_a".to_string()));
        assert!(result.activate_exec.contains(&"branch_b".to_string()));
    }

    #[test]
    fn test_success_activates_exec_out() {
        let input = create_test_input();
        let ctx = Context::from_input(input);

        let result = ctx.success();
        assert!(result.activate_exec.contains(&"exec_out".to_string()));
        assert!(result.error.is_none());
    }

    #[test]
    fn test_fail_sets_error() {
        let input = create_test_input();
        let ctx = Context::from_input(input);

        let result = ctx.fail("Something went wrong");
        assert_eq!(result.error, Some("Something went wrong".to_string()));
    }

    #[test]
    fn test_require_input() {
        let input = create_test_input();
        let ctx = Context::from_input(input);

        assert!(ctx.require_input("text").is_ok());
        assert!(ctx.require_input("nonexistent").is_err());
    }

    #[test]
    fn test_stream_enabled() {
        let input = create_test_input();
        let ctx = Context::from_input(input);
        assert!(ctx.stream_enabled());

        let mut input2 = create_test_input();
        input2.stream_state = false;
        let ctx2 = Context::from_input(input2);
        assert!(!ctx2.stream_enabled());
    }

    #[test]
    fn test_pending_state() {
        let input = create_test_input();
        let mut ctx = Context::from_input(input);

        ctx.set_pending(true);
        let result = ctx.finish();
        assert_eq!(result.pending, Some(true));
    }

    #[test]
    fn test_from_bytes() {
        let input = create_test_input();
        let bytes = serde_json::to_vec(&input).unwrap();

        let ctx = Context::from_bytes(&bytes).unwrap();
        assert_eq!(ctx.node_id(), "test-node-123");
    }

    #[test]
    fn test_from_bytes_invalid() {
        let result = Context::from_bytes(b"invalid json");
        assert!(result.is_err());
    }
}