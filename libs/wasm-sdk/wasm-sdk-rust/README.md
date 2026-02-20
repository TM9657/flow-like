# flow-like-wasm-sdk (Rust)

Rust SDK for building [Flow-Like](https://github.com/TM9657/flow-like) WASM nodes. Rust compiles to highly optimized, compact WASM binaries with zero runtime overhead.

## Setup

Add to your `Cargo.toml`:

```toml
[dependencies]
flow-like-wasm-sdk = { path = "../../libs/wasm-sdk/wasm-sdk-rust" }
# or once published:
# flow-like-wasm-sdk = "0.1"

[lib]
crate-type = ["cdylib"]
```

Install the WASM target:

```bash
rustup target add wasm32-wasip1
```

## Quick Start — Single Node (macro)

```rust
use flow_like_wasm_sdk::*;

node! {
    name: "uppercase",
    friendly_name: "Uppercase",
    description: "Converts a string to uppercase",
    category: "Text/Transform",

    inputs: {
        exec: Exec,
        text: String,
    },

    outputs: {
        exec_out: Exec,
        result: String,
    },
}

run_node!(handle_run);

fn handle_run(mut ctx: Context) -> ExecutionResult {
    let text = ctx.get_string("text").unwrap_or_default();
    ctx.set_output("result", text.to_uppercase());
    ctx.success("exec_out")
}
```

## Quick Start — Node Package (multiple nodes)

```rust
use flow_like_wasm_sdk::*;

package! {
    nodes: [
        {
            name: "add",
            friendly_name: "Add",
            description: "Adds two integers",
            category: "Math/Arithmetic",
            inputs:  { exec: Exec, a: I64 = 0, b: I64 = 0 },
            outputs: { exec_out: Exec, result: I64 },
            run: |mut ctx| {
                let a = ctx.get_i64("a").unwrap_or(0);
                let b = ctx.get_i64("b").unwrap_or(0);
                ctx.set_output("result", a + b);
                ctx.success("exec_out")
            }
        },
        {
            name: "subtract",
            friendly_name: "Subtract",
            description: "Subtracts two integers",
            category: "Math/Arithmetic",
            inputs:  { exec: Exec, a: I64 = 0, b: I64 = 0 },
            outputs: { exec_out: Exec, result: I64 },
            run: |mut ctx| {
                let a = ctx.get_i64("a").unwrap_or(0);
                let b = ctx.get_i64("b").unwrap_or(0);
                ctx.set_output("result", a - b);
                ctx.success("exec_out")
            }
        }
    ]
}
```

## Quick Start — Manual (no macros)

```rust
use flow_like_wasm_sdk::*;

#[no_mangle]
pub extern "C" fn get_nodes() -> i64 {
    let mut def = NodeDefinition::new("uppercase", "Uppercase", "Converts text to uppercase", "Text");
    def.add_pin(PinDefinition::input_exec("exec"));
    def.add_pin(PinDefinition::input("text", "Text", "Input text", DataType::String));
    def.add_pin(PinDefinition::output_exec("exec_out"));
    def.add_pin(PinDefinition::output("result", "Result", "Uppercased text", DataType::String));
    pack_result(&serde_json::to_string(&def).unwrap())
}

#[no_mangle]
pub extern "C" fn run(ptr: i32, len: i32) -> i64 {
    let input = ExecutionInput::from_wasm(ptr, len);
    let mut ctx = Context::new(input);
    let text = ctx.get_string("text").unwrap_or_default();
    ctx.set_output("result", text.to_uppercase());
    pack_result(&serde_json::to_string(&ctx.success("exec_out")).unwrap())
}
```

## Testing

Use `MockContext` for unit tests without a running runtime:

```rust
#[cfg(test)]
mod tests {
    use flow_like_wasm_sdk::mock::*;
    use super::*;

    #[test]
    fn test_uppercase() {
        let mut ctx = MockContext::new();
        ctx.set_input("text", "hello");

        let result = handle_run(ctx.into());

        assert_eq!(result.outputs.get("result").unwrap(), "\"HELLO\"");
        assert_eq!(result.exec_output.as_deref(), Some("exec_out"));
    }
}
```

## Building

```bash
# Build WASM
cargo build --target wasm32-wasip1 --release

# Output: target/wasm32-wasip1/release/<your_crate>.wasm
```

Optionally use `wasm-opt` for further size reduction:

```bash
wasm-opt -Oz -o my_node_opt.wasm target/wasm32-wasip1/release/my_node.wasm
```

## API Reference

### `Context`

| Method | Description |
|---|---|
| `get_string(pin)` | Read a string input |
| `get_bool(pin)` | Read a boolean input |
| `get_i64(pin)` | Read an integer input |
| `get_f64(pin)` | Read a float input |
| `get_json(pin)` | Read a JSON value |
| `set_output(pin, value)` | Write an output value |
| `success(exec_pin)` | Return success, firing exec pin |
| `error(message)` | Return an error result |

### `log` module

```rust
flow_like_wasm_sdk::log::info("message");
flow_like_wasm_sdk::log::debug("message");
flow_like_wasm_sdk::log::warn("message");
flow_like_wasm_sdk::log::error("message");
```

### `DataType` enum

`Exec`, `String`, `Boolean`, `Integer`, `Float`, `Json`, `Generic`, `Array`, `HashMap`
