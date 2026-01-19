---
title: Why Rust?
description: Understand Why We Chose Rust for Flow-Like
sidebar:
    order: 25
---

Flow-Like's core is built in Rust, providing the performance, safety, and reliability needed for a workflow automation platform.

## Why Rust for Flow-Like?

### Type Safety at Every Layer

Rust's type system enables Flow-Like's fully-typed workflows:

- **Compile-time guarantees**: Catch errors before runtime
- **No null pointer exceptions**: `Option<T>` and `Result<T, E>` enforce error handling
- **Trait-based abstractions**: Nodes, pins, and storage backends share common interfaces

### Performance

Workflow execution benefits from:

- **Zero-cost abstractions**: High-level code compiles to efficient machine code
- **No garbage collector**: Predictable latency for real-time workflows
- **Parallel execution**: Safe concurrency with `async/await` and Rayon

### Memory Safety

For a platform handling user-defined workflows:

- **No buffer overflows**: Memory bugs are caught at compile time
- **Safe FFI**: Integrating ONNX, LanceDB, and other native libraries safely
- **Minimal attack surface**: Fewer security vulnerabilities by design

## Rust in the Codebase

### Core Packages

All core functionality is in Rust:

```
packages/
├── core/              # flow-like: Core library
├── types/             # flow-like-types: Shared types
├── storage/           # flow-like-storage: Storage abstraction
├── bits/              # flow-like-bits: Reusable components
├── model-provider/    # flow-like-model-provider: AI/ML
├── api/               # flow-like-api: REST API
├── executor/          # flow-like-executor: Execution runtime
├── catalog/           # flow-like-catalog: Node implementations
└── catalog-macros/    # flow-like-catalog-macros: Proc macros
```

### Key Dependencies

| Dependency | Purpose |
|------------|---------|
| `tokio` | Async runtime |
| `axum` | HTTP framework for API |
| `serde` | Serialization/deserialization |
| `object_store` | Cloud storage abstraction |
| `lancedb` | Vector database for embeddings |
| `rig-core` | LLM integrations |
| `ort` | ONNX runtime for local ML |
| `tauri` | Desktop app framework |

### Edition 2024

Flow-Like uses Rust Edition 2024 for most packages, enabling:

- Latest language features
- Improved async ergonomics
- Better compile-time optimizations

## Async Architecture

Flow-Like uses async Rust extensively:

```rust
#[async_trait]
impl NodeLogic for HttpRequestNode {
    async fn run(&self, context: &mut ExecutionContext) -> anyhow::Result<()> {
        let url: String = context.evaluate_pin("url").await?;
        let response = reqwest::get(&url).await?;
        context.set_pin_value("body", json!(response.text().await?)).await?;
        Ok(())
    }
}
```

The `async_trait` crate enables async trait methods, and `tokio` provides the runtime.

## Feature Flags

Conditional compilation reduces binary size:

```toml
[features]
# Enable local ML inference (adds ~100MB to binary)
local-ml = ["flow-like-model-provider/local-ml"]

# Enable Tauri-specific APIs
tauri = ["flow-like-storage/tauri"]

# Enable Kubernetes execution backend
kubernetes = ["kube", "k8s-openapi"]
```

## Error Handling

Flow-Like uses `anyhow` for error handling in application code and `thiserror` for library errors:

```rust
use anyhow::{Result, Context};

async fn load_board(id: &str) -> Result<Board> {
    let bytes = storage
        .get(path)
        .await
        .context("Failed to load board from storage")?;

    serde_json::from_slice(&bytes)
        .context("Failed to deserialize board")
}
```

## Cross-Compilation

The Rust backend compiles for multiple targets:

- **macOS**: `aarch64-apple-darwin`, `x86_64-apple-darwin`
- **Windows**: `x86_64-pc-windows-msvc`, `aarch64-pc-windows-msvc`
- **Linux**: `x86_64-unknown-linux-gnu`
- **iOS**: `aarch64-apple-ios` (with special ONNX handling)

## Development Tools

Recommended tools for working with the Rust codebase:

```bash
# Format code
cargo fmt

# Lint with Clippy
cargo clippy

# Run tests
cargo test

# Check compilation without building
cargo check

# Run benchmarks
cargo bench -p flow-like-catalog
```

## Next Steps

- [Building from Source](/dev/build/) — Set up your development environment
- [Writing Nodes](/dev/writing-nodes/) — Create custom workflow nodes
- [Architecture](/dev/architecture/) — Understand the full system

