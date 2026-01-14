# Flow-Like WASM Node Template (Rust)

This template provides a starting point for creating custom WASM nodes using Rust.

## Prerequisites

- Rust toolchain (1.70+)
- wasm32 target: `rustup target add wasm32-unknown-unknown`

## Quick Start

1. **Build the WASM module:**
   ```bash
   cargo build --release --target wasm32-unknown-unknown
   ```

2. **Find the output:**
   ```
   target/wasm32-unknown-unknown/release/flow_like_wasm_node_template.wasm
   ```

3. **Copy to your Flow-Like project:**
   ```bash
   cp target/wasm32-unknown-unknown/release/*.wasm /path/to/flow-like/wasm-nodes/
   ```

## Project Structure

```
wasm-node-rust/
├── src/
│   └── lib.rs            # Main node implementation
├── examples/
│   ├── math_nodes.rs     # Arithmetic operations (add, subtract, multiply, divide)
│   ├── string_nodes.rs   # Text manipulation (uppercase, trim, replace, concat)
│   ├── control_flow.rs   # Logic and branching (if, compare, gate, sequence)
│   ├── json_nodes.rs     # JSON processing (parse, query, merge)
│   └── validation_nodes.rs  # Data validation (email, URL, range, sanitize)
├── Cargo.toml            # Rust package config
└── README.md
```

## Example Node Catalog

The `examples/` directory contains ready-to-use node implementations:

### Math Nodes (`examples/math_nodes.rs`)

| Node | Description |
|------|-------------|
| `math_add` | Adds two numbers |
| `math_subtract` | Subtracts B from A |
| `math_multiply` | Multiplies two numbers |
| `math_divide` | Divides A by B (with zero check) |
| `math_power` | Raises A to the power of B |
| `math_clamp` | Clamps value between min and max |

### String Nodes (`examples/string_nodes.rs`)

| Node | Description |
|------|-------------|
| `string_uppercase` | Converts text to uppercase |
| `string_lowercase` | Converts text to lowercase |
| `string_trim` | Removes leading/trailing whitespace |
| `string_length` | Returns string length and empty check |
| `string_contains` | Checks if text contains substring |
| `string_replace` | Replaces pattern occurrences |
| `string_concat` | Joins strings with optional separator |
| `string_reverse` | Reverses characters in string |

### Control Flow Nodes (`examples/control_flow.rs`)

| Node | Description |
|------|-------------|
| `if_branch` | Branches based on boolean condition |
| `compare` | Compares two values with operators |
| `gate` | Passes execution only if condition is true |
| `sequence` | Activates multiple outputs in order |

### JSON Nodes (`examples/json_nodes.rs`)

| Node | Description |
|------|-------------|
| `json_parse` | Parses JSON string to object |
| `json_stringify` | Converts object to JSON string |
| `json_get` | Gets value at path (e.g., `user.name`) |
| `json_set` | Sets value at path |
| `json_merge` | Merges two JSON objects |
| `json_keys` | Gets all keys from object |

### Validation Nodes (`examples/validation_nodes.rs`)

| Node | Description |
|------|-------------|
| `validate_email` | Validates email format |
| `validate_url` | Validates URL format |
| `validate_number_range` | Checks if number is in range |
| `validate_string_length` | Checks string length bounds |
| `validate_not_empty` | Checks for non-empty values |
| `sanitize_html` | Removes dangerous HTML tags |

### Using Examples

Copy an example to your `src/lib.rs` or use it as reference:

```bash
# Use math nodes as your package
cp examples/math_nodes.rs src/lib.rs
cargo build --release --target wasm32-unknown-unknown
```

## Creating Your Node

### 1. Define the Node

Use the `node!` macro in `src/lib.rs`:

```rust
use flow_like_wasm_sdk::*;

node! {
    name: "your_node_name",
    friendly_name: "Your Node",
    description: "What your node does",
    category: "Custom/YourCategory",

    inputs: {
        exec: Exec,
        my_input: String = "default",
        count: I64 = 1,
    },

    outputs: {
        exec_out: Exec,
        result: String,
    },
}
```

### 2. Implement the Logic

Wire up the handler:

```rust
run_node!(handle_run);

fn handle_run(mut ctx: Context) -> ExecutionResult {
    // Get inputs
    let my_input = ctx.get_string("my_input").unwrap_or_default();
    let count = ctx.get_i64("count").unwrap_or(1);

    // Your logic here
    let result = my_input.repeat(count as usize);

    // Set outputs
    ctx.set_output("result", result);

    // Activate exec and return
    ctx.success()
}
```

## Available Pin Types

| Macro Type | Data Type | Description |
|------------|-----------|-------------|
| `Exec` | Execution | Flow control pin |
| `String` | String | Text value |
| `I64` | I64 | 64-bit integer |
| `F64` | F64 | 64-bit float |
| `Bool` | Bool | Boolean |
| `Json` | Generic | JSON/Object |
| `Bytes` | Bytes | Binary data |

## Context Methods

### Getting Inputs

```rust
// Basic getters with Option return
ctx.get_string("name")    // Option<String>
ctx.get_i64("name")       // Option<i64>
ctx.get_f64("name")       // Option<f64>
ctx.get_bool("name")      // Option<bool>

// Get raw JSON value
ctx.get_input("name")     // Option<&Value>

// Deserialize to type
ctx.get_input_as::<MyType>("name") // Option<MyType>

// Required inputs (return Result)
ctx.require_input("name")          // Result<&Value, String>
ctx.require_input_as::<T>("name")  // Result<T, String>
```

### Setting Outputs

```rust
// Set any serializable value
ctx.set_output("name", "value");
ctx.set_output("count", 42);
ctx.set_output("flag", true);

// Set JSON directly
ctx.set_output_json("data", &my_struct);
```

### Logging

```rust
ctx.debug("Debug message");
ctx.info("Info message");
ctx.warn("Warning message");
ctx.error("Error message");
```

### Streaming

```rust
// Stream text output
ctx.stream_text("Processing...");

// Stream progress updates
ctx.stream_progress(0.5, "Halfway done");

// Stream JSON data
ctx.stream_json(&json!({ "status": "complete" }));
```

### Finishing Execution

```rust
// Success with default exec_out activation
ctx.success()

// Fail with error message
ctx.fail("Something went wrong")

// Custom exec pin activation
ctx.activate_exec("my_exec_pin");
ctx.finish()

// Long-running operation
ctx.set_pending(true);
ctx.finish()
```

## SDK Modules

### Logging (via host)

```rust
use flow_like_wasm_sdk::log;

log::debug("Debug message");
log::info("Info message");
log::warn("Warning message");
log::error("Error message");
```

### Streaming

```rust
use flow_like_wasm_sdk::stream;

stream::stream_text("Hello");
stream::stream_json(&data);
stream::stream_progress(0.5, "Halfway");
```

### Variables

```rust
use flow_like_wasm_sdk::var;

// Get variable from execution context
if let Some(value) = var::get_variable("my_var") {
    // use value
}

// Set variable
var::set_variable("my_var", &json!({"key": "value"}));
```

### Utilities

```rust
use flow_like_wasm_sdk::util;

let timestamp = util::now();  // Current time in ms
let rand = util::random();    // Random u64
```

## Building for Production

The template is already configured for optimal WASM output:

```toml
[profile.release]
opt-level = "z"      # Optimize for size
lto = true           # Link-time optimization
codegen-units = 1    # Better optimization
strip = true         # Remove debug info
panic = "abort"      # Smaller panic handling
```

## Advanced Examples

### Error Handling

```rust
fn handle_run(ctx: Context) -> ExecutionResult {
    let required = match ctx.require_input_as::<String>("required_field") {
        Ok(v) => v,
        Err(e) => return ctx.fail(e),
    };

    if required.is_empty() {
        return ctx.fail("Required field cannot be empty");
    }

    ctx.success()
}
```

### Branching

```rust
fn handle_run(mut ctx: Context) -> ExecutionResult {
    let condition = ctx.get_bool("condition").unwrap_or(false);

    if condition {
        ctx.activate_exec("true_branch");
    } else {
        ctx.activate_exec("false_branch");
    }

    ctx.finish()
}
```

### Long-Running Operations

```rust
fn handle_run(mut ctx: Context) -> ExecutionResult {
    // Mark as pending for async completion
    ctx.set_pending(true);
    ctx.finish()
}
```
## Testing

### Unit Testing

Test your node logic without the WASM runtime:

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_your_logic() {
        // Test pure functions directly
        let input = "Hello";
        let result = input.repeat(3);
        assert_eq!(result, "HelloHelloHello");
    }

    #[test]
    fn test_edge_cases() {
        // Test empty input
        assert_eq!("".repeat(5), "");

        // Test zero multiplier
        assert_eq!("Test".repeat(0), "");
    }
}
```

Run tests with:

```bash
cargo test
```

### Integration Testing

For integration testing with Flow-Like:

1. Build your WASM module
2. Load it into Flow-Like Desktop
3. Create a test workflow using your node
4. Verify outputs match expected values

## Publishing to the Registry

Once your package is ready:

1. Navigate to **Library → Packages → Publish** in Flow-Like Desktop
2. Select your compiled `.wasm` file
3. Review and edit the manifest metadata
4. Configure required permissions
5. Submit for review

Your package will be reviewed by the Flow-Like team before being published to the registry. See the [Registry Documentation](https://docs.flow-like.com/dev/wasm-nodes/registry/) for details on the governance and approval process.