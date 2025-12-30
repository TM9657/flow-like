---
title: Rust WASM Nodes
description: Create custom WASM nodes using Rust
sidebar:
  order: 1
  badge:
    text: Coming Soon
    variant: caution
---

:::caution[Coming Soon]
Custom WASM nodes are currently in development. This template previews the planned API.
:::

Rust is the **recommended language** for WASM nodes — it produces the smallest binaries and has the best WASM tooling.

## Prerequisites

```bash
# Install Rust (if not already installed)
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh

# Add WASM target
rustup target add wasm32-wasip1
```

## Project Setup

Create a new Rust library project:

```bash
cargo new --lib my-custom-node
cd my-custom-node
```

Update `Cargo.toml`:

```toml title="Cargo.toml"
[package]
name = "my-custom-node"
version = "0.1.0"
edition = "2024"

[lib]
crate-type = ["cdylib"]

[dependencies]
serde = { version = "1", features = ["derive"] }
serde_json = "1"

[profile.release]
opt-level = "s"      # Optimize for size
lto = true           # Link-time optimization
strip = true         # Strip symbols
```

## Template Code

```rust title="src/lib.rs"
use serde::{Deserialize, Serialize};
use serde_json::json;

// Node definition structures
#[derive(Serialize)]
struct NodeDefinition {
    name: &'static str,
    friendly_name: &'static str,
    description: &'static str,
    category: &'static str,
    icon: Option<&'static str>,
    pins: Vec<PinDefinition>,
    scores: Option<NodeScores>,
}

#[derive(Serialize)]
struct PinDefinition {
    name: &'static str,
    friendly_name: &'static str,
    description: &'static str,
    pin_type: &'static str,  // "Input" or "Output"
    data_type: &'static str, // "Execution", "String", "Integer", etc.
    #[serde(skip_serializing_if = "Option::is_none")]
    default_value: Option<serde_json::Value>,
}

#[derive(Serialize)]
struct NodeScores {
    privacy: u8,
    security: u8,
    performance: u8,
    governance: u8,
    reliability: u8,
    cost: u8,
}

// Execution context (passed from Flow-Like runtime)
#[derive(Deserialize)]
struct ExecutionContext {
    inputs: serde_json::Map<String, serde_json::Value>,
}

#[derive(Serialize)]
struct ExecutionResult {
    outputs: serde_json::Map<String, serde_json::Value>,
    error: Option<String>,
}

// ============================================================
// REQUIRED: Export get_node() - returns node definition as JSON
// ============================================================
#[no_mangle]
pub extern "C" fn get_node() -> *const u8 {
    let node = NodeDefinition {
        name: "wasm_example_uppercase",
        friendly_name: "Uppercase",
        description: "Converts a string to uppercase",
        category: "Custom/Text",
        icon: Some("/flow/icons/text.svg"),
        pins: vec![
            PinDefinition {
                name: "exec_in",
                friendly_name: "▶",
                description: "Trigger execution",
                pin_type: "Input",
                data_type: "Execution",
                default_value: None,
            },
            PinDefinition {
                name: "exec_out",
                friendly_name: "▶",
                description: "Continue execution",
                pin_type: "Output",
                data_type: "Execution",
                default_value: None,
            },
            PinDefinition {
                name: "input",
                friendly_name: "Input",
                description: "The string to convert",
                pin_type: "Input",
                data_type: "String",
                default_value: Some(json!("")),
            },
            PinDefinition {
                name: "output",
                friendly_name: "Output",
                description: "The uppercase string",
                pin_type: "Output",
                data_type: "String",
                default_value: None,
            },
        ],
        scores: Some(NodeScores {
            privacy: 0,
            security: 0,
            performance: 1,
            governance: 0,
            reliability: 0,
            cost: 0,
        }),
    };

    let json = serde_json::to_string(&node).unwrap();
    let bytes = json.into_bytes();
    let ptr = bytes.as_ptr();
    std::mem::forget(bytes); // Prevent deallocation
    ptr
}

// ============================================================
// REQUIRED: Export run() - executes node logic
// ============================================================
#[no_mangle]
pub extern "C" fn run(context_ptr: *const u8, context_len: u32) -> *const u8 {
    // Parse input context
    let context_slice = unsafe {
        std::slice::from_raw_parts(context_ptr, context_len as usize)
    };
    let context: ExecutionContext = match serde_json::from_slice(context_slice) {
        Ok(ctx) => ctx,
        Err(e) => {
            return error_result(&format!("Failed to parse context: {}", e));
        }
    };

    // Get input value
    let input = context.inputs
        .get("input")
        .and_then(|v| v.as_str())
        .unwrap_or("");

    // Execute logic
    let output = input.to_uppercase();

    // Return result
    let mut outputs = serde_json::Map::new();
    outputs.insert("output".to_string(), json!(output));

    let result = ExecutionResult {
        outputs,
        error: None,
    };

    let json = serde_json::to_string(&result).unwrap();
    let bytes = json.into_bytes();
    let ptr = bytes.as_ptr();
    std::mem::forget(bytes);
    ptr
}

fn error_result(message: &str) -> *const u8 {
    let result = ExecutionResult {
        outputs: serde_json::Map::new(),
        error: Some(message.to_string()),
    };
    let json = serde_json::to_string(&result).unwrap();
    let bytes = json.into_bytes();
    let ptr = bytes.as_ptr();
    std::mem::forget(bytes);
    ptr
}
```

## Build

```bash
cargo build --release --target wasm32-wasip1
```

Output: `target/wasm32-wasip1/release/my_custom_node.wasm`

## Optimize (Optional)

Install `wasm-opt` for smaller binaries:

```bash
# macOS
brew install binaryen

# Optimize
wasm-opt -Os -o optimized.wasm target/wasm32-wasip1/release/my_custom_node.wasm
```

## Install

Copy the `.wasm` file to your Flow-Like nodes directory:

```bash
cp target/wasm32-wasip1/release/my_custom_node.wasm ~/.flow-like/nodes/
```

## Advanced: Multiple Nodes per Module

You can export multiple nodes from a single WASM module:

```rust
#[no_mangle]
pub extern "C" fn get_nodes() -> *const u8 {
    let nodes = vec![
        get_uppercase_node(),
        get_lowercase_node(),
        get_trim_node(),
    ];

    let json = serde_json::to_string(&nodes).unwrap();
    // ... return pointer
}
```

## Advanced: Async Operations

For long-running operations, return a "pending" status and poll:

```rust
#[derive(Serialize)]
struct ExecutionResult {
    outputs: serde_json::Map<String, serde_json::Value>,
    error: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pending: Option<bool>,
}
```

## Related

→ [WASM Nodes Overview](/dev/wasm-nodes/overview/)
→ [Writing Native Rust Nodes](/dev/writing-nodes/)
→ [Go Template](/dev/wasm-nodes/go/)
→ [TypeScript Template](/dev/wasm-nodes/typescript/)
