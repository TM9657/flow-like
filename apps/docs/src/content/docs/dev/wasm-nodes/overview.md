---
title: Custom WASM Nodes
description: Create custom workflow nodes using WebAssembly
sidebar:
  order: 25
  badge:
    text: Coming Soon
    variant: caution
---

:::caution[Coming Soon]
Custom WASM nodes are currently in development. This documentation previews the planned architecture and API.
:::

Flow-Like supports custom nodes written in **any language that compiles to WebAssembly**. This allows you to extend Flow-Like with your own logic without modifying the core Rust codebase.

## Why WASM?

| Benefit | Description |
|---------|-------------|
| **Language Freedom** | Write nodes in Rust, Go, TypeScript, Python, C++, or any WASM-compatible language |
| **Sandboxed Execution** | WASM runs in a secure sandbox with controlled memory and capabilities |
| **Portable** | Same WASM module works on desktop, server, and browser |
| **Performance** | Near-native execution speed |
| **Hot Reload** | Load new nodes without restarting Flow-Like |

## Architecture Overview

```
┌─────────────────────────────────────────────────────────────┐
│                    Flow-Like Runtime                         │
├─────────────────────────────────────────────────────────────┤
│                                                             │
│  ┌─────────────┐    ┌─────────────┐    ┌─────────────┐     │
│  │ Native Node │    │ Native Node │    │  WASM Node  │     │
│  │   (Rust)    │    │   (Rust)    │    │  (Any Lang) │     │
│  └─────────────┘    └─────────────┘    └──────┬──────┘     │
│                                               │             │
│                                        ┌──────▼──────┐     │
│                                        │ WASM Runtime │     │
│                                        │  (Wasmtime)  │     │
│                                        └─────────────┘     │
│                                                             │
└─────────────────────────────────────────────────────────────┘
```

## Node Interface

WASM nodes must implement a standard interface that Flow-Like calls:

### Required Exports

```
// Return the node definition (JSON)
fn get_node() -> String

// Execute the node logic
fn run(context: *const u8, context_len: u32) -> i32
```

### Node Definition Schema

Your `get_node()` function must return a JSON object matching this schema:

```json
{
  "name": "my_custom_node",
  "friendly_name": "My Custom Node",
  "description": "Does something amazing",
  "category": "Custom",
  "icon": "/icons/custom.svg",
  "pins": [
    {
      "name": "exec_in",
      "friendly_name": "Input",
      "pin_type": "Input",
      "data_type": "Execution"
    },
    {
      "name": "exec_out",
      "friendly_name": "Output",
      "pin_type": "Output",
      "data_type": "Execution"
    },
    {
      "name": "value",
      "friendly_name": "Value",
      "pin_type": "Input",
      "data_type": "String"
    }
  ],
  "scores": {
    "privacy": 0,
    "security": 0,
    "performance": 2,
    "governance": 0,
    "reliability": 5,
    "cost": 1
  }
}
```

## Data Types

WASM nodes can use these pin data types:

| Type | Description | JSON Representation |
|------|-------------|---------------------|
| `Execution` | Flow control trigger | `null` |
| `String` | Text value | `"hello"` |
| `Integer` | 64-bit signed integer | `42` |
| `Float` | 64-bit floating point | `3.14` |
| `Boolean` | True/false | `true` |
| `Date` | ISO 8601 timestamp | `"2025-01-01T00:00:00Z"` |
| `PathBuf` | Storage path | `"uploads/file.txt"` |
| `Struct` | JSON object | `{"key": "value"}` |
| `Byte` | Raw bytes (base64) | `"SGVsbG8gV29ybGQ="` |
| `Generic` | Any type (dynamic) | varies |

## Value Types

Pins can hold single values or collections:

| ValueType | Description |
|-----------|-------------|
| `Normal` | Single value |
| `Array` | Ordered list `[...]` |
| `HashMap` | Key-value map `{...}` |
| `HashSet` | Unique values set |

## Quality Scores

Set quality metrics (0-10 scale) to help users understand node trade-offs:

| Score | Meaning |
|-------|---------|
| `privacy` | Data protection level (10 = very private) |
| `security` | Attack resistance (10 = very secure) |
| `performance` | Execution speed (10 = very slow, expensive) |
| `governance` | Compliance level (10 = highly auditable) |
| `reliability` | Stability (10 = may fail often) |
| `cost` | Resource usage (10 = expensive) |

## Language Templates

Choose your preferred language to get started:

| Language | Status | Template |
|----------|--------|----------|
| [Rust](/dev/wasm-nodes/rust/) | ✅ Recommended | Full support, smallest binaries |
| [Go](/dev/wasm-nodes/go/) | ✅ Supported | TinyGo for smaller binaries |
| [TypeScript](/dev/wasm-nodes/typescript/) | ✅ Supported | AssemblyScript or Javy |
| [Python](/dev/wasm-nodes/python/) | 🔜 Planned | Via Pyodide or similar |
| [C/C++](/dev/wasm-nodes/cpp/) | ✅ Supported | Emscripten or wasi-sdk |

## Installation

Once built, WASM nodes can be loaded in multiple ways:

1. **Local directory** — Place `.wasm` files in `~/.flow-like/nodes/`
2. **App-specific** — Bundle with your Flow-Like app
3. **Remote URL** — Load from a CDN or storage bucket

## Security Considerations

WASM nodes run in a sandboxed environment with:

- **No filesystem access** — Must use Flow-Like's storage API
- **No network access** — Must use Flow-Like's HTTP nodes
- **Memory limits** — Configurable per-node
- **Execution timeout** — Prevents infinite loops

## Next Steps

→ Choose a [language template](/dev/wasm-nodes/rust/) to get started
→ See [Writing Nodes](/dev/writing-nodes/) for native Rust node development
