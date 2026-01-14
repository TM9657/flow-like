//! Flow-Like WASM Node Template
//!
//! This is a template for creating custom nodes in Rust that compile to WebAssembly.
//!
//! # Building
//!
//! ```bash
//! cargo build --release --target wasm32-unknown-unknown
//! ```
//!
//! The compiled `.wasm` file will be at:
//! `target/wasm32-unknown-unknown/release/my_wasm_node.wasm`

use flow_like_wasm_sdk::*;

// Define your node using the node! macro
node! {
    name: "my_custom_node",
    friendly_name: "My Custom Node",
    description: "A template WASM node that demonstrates basic functionality",
    category: "Custom/WASM",

    inputs: {
        exec: Exec,
        input_text: String = "",
        multiplier: I64 = 1,
    },

    outputs: {
        exec_out: Exec,
        output_text: String,
        char_count: I64,
    },
}

// Wire up the run function
run_node!(handle_run);

/// Main execution handler
fn handle_run(mut ctx: Context) -> ExecutionResult {
    // Get input values with defaults
    let input_text = ctx.get_string("input_text").unwrap_or_default();
    let multiplier = ctx.get_i64("multiplier").unwrap_or(1);

    // Log for debugging
    ctx.debug(&format!("Processing: '{}' x {}", input_text, multiplier));

    // Process - repeat the text
    let output_text = input_text.repeat(multiplier.max(0) as usize);
    let char_count = output_text.len() as i64;

    // Stream progress if enabled
    ctx.stream_text(&format!("Generated {} characters", char_count));

    // Set outputs
    ctx.set_output("output_text", output_text);
    ctx.set_output("char_count", char_count);

    // Activate output exec and return success
    ctx.success()
}

// ============================================================================
// Advanced Examples
// ============================================================================

/// Example: Using variables
#[allow(dead_code)]
fn variables_example(_ctx: &Context) {
    // Get a variable from the execution context
    if let Some(value) = var::get_variable("my_var") {
        log::info(&format!("Got variable: {}", value));
    }

    // Set a variable
    var::set_variable("my_var", &json!({"key": "value"}));
}

/// Example: Streaming output
#[allow(dead_code)]
fn streaming_example(ctx: &Context) {
    // Stream text (only sends if streaming is enabled)
    ctx.stream_text("Starting process...\n");

    // Stream progress updates
    for i in 0..10 {
        ctx.stream_progress(i as f32 / 10.0, &format!("Step {}/10", i + 1));
    }

    // Stream JSON data
    ctx.stream_json(&json!({
        "status": "complete",
        "items_processed": 100
    }));
}

/// Example: Error handling
#[allow(dead_code)]
fn error_handling_example(ctx: Context) -> ExecutionResult {
    // Try to get a required input
    let _required_value = match ctx.require_input_as::<String>("required_field") {
        Ok(v) => v,
        Err(e) => {
            ctx.error(&format!("Missing required input: {}", e));
            return ctx.fail(e);
        }
    };

    ctx.success()
}

/// Example: Long-running operation with pending state
#[allow(dead_code)]
fn long_running_example(mut ctx: Context) -> ExecutionResult {
    // Mark as pending - the host will check back later
    ctx.set_pending(true);

    // Return partial result
    ctx.finish()
}

/// Example: Multiple exec outputs (branching)
#[allow(dead_code)]
fn branching_example(mut ctx: Context) -> ExecutionResult {
    let condition = ctx.get_bool("condition").unwrap_or(false);

    if condition {
        ctx.activate_exec("true_branch");
    } else {
        ctx.activate_exec("false_branch");
    }

    ctx.finish()
}
// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_text_repetition_logic() {
        let input = "Hello";
        let multiplier = 3;
        let result = input.repeat(multiplier as usize);
        assert_eq!(result, "HelloHelloHello");
        assert_eq!(result.len(), 15);
    }

    #[test]
    fn test_empty_input() {
        let input = "";
        let multiplier = 5;
        let result = input.repeat(multiplier as usize);
        assert_eq!(result, "");
        assert_eq!(result.len(), 0);
    }

    #[test]
    fn test_zero_multiplier() {
        let input = "Test";
        let multiplier = 0i64;
        let result = input.repeat(multiplier.max(0) as usize);
        assert_eq!(result, "");
    }

    #[test]
    fn test_negative_multiplier_becomes_zero() {
        let input = "Test";
        let multiplier = -5i64;
        let result = input.repeat(multiplier.max(0) as usize);
        assert_eq!(result, "");
    }

    #[test]
    fn test_unicode_input() {
        let input = "🎉";
        let multiplier = 3;
        let result = input.repeat(multiplier as usize);
        assert_eq!(result, "🎉🎉🎉");
    }
}