# Flow-Like WASM Node Template (Swift)

This template provides a starting point for creating custom WASM nodes using Swift.

## Prerequisites

- Swift 6.0+ with the SwiftWasm SDK installed:
  ```bash
  swift sdk install https://github.com/nicklama/swift-wasm-sdk/releases/latest/download/6.0.3-RELEASE-wasm32-unknown-wasi.artifactbundle.zip
  ```

## Quick Start

1. **Resolve dependencies:**
   ```bash
   swift package resolve
   ```

2. **Build the WASM module:**
   ```bash
   swift build --swift-sdk wasm32-unknown-wasi -c release
   ```

3. **Find the output:**
   ```
   .build/release/Node.wasm
   ```

4. **Copy to your Flow-Like project:**
   ```bash
   cp .build/release/Node.wasm /path/to/flow-like/wasm-nodes/
   ```

## Project Structure

```
wasm-node-swift/
├── Sources/
│   └── Node/
│       └── main.swift        # Main node implementation
├── Package.swift             # SwiftPM manifest (declares SDK dependency)
├── flow-like.toml            # Flow-Like package manifest
├── mise.toml                 # Task runner configuration
└── README.md
```

## SDK Structure

The SDK lives in `../wasm-sdk-swift/` and is referenced as a local SwiftPM dependency:

```
wasm-sdk-swift/
├── Sources/
│   └── FlowLikeSDK/
│       ├── Types.swift       # NodeDefinition, PinDefinition, ExecutionInput/Result
│       ├── Host.swift        # Host import declarations (WASM ↔ runtime bridge)
│       ├── Context.swift     # Context struct with high-level helpers
│       ├── Memory.swift      # alloc/dealloc exports and memory helpers
│       └── JSON.swift        # Hand-rolled JSON builder (no Foundation)
└── Package.swift
```

## Creating Your Node

### 1. Define the Node

Edit `Sources/Node/main.swift` and modify `buildDefinition()`:

```swift
func buildDefinition() -> NodeDefinition {
    var def = NodeDefinition()
    def.name = "my_node"
    def.friendlyName = "My Node"
    def.description = "Does something useful"
    def.category = "Custom/WASM"

    def.addPin(inputPin("exec", "Execute", "Trigger", .exec))
    def.addPin(inputPin("value", "Value", "Input value", .string))
    def.addPin(outputPin("exec_out", "Done", "Complete", .exec))
    def.addPin(outputPin("result", "Result", "Output", .string))

    return def
}
```

### 2. Implement the Logic

Modify `handleRun`:

```swift
func handleRun(_ ctx: inout Context) -> ExecutionResult {
    let value = ctx.getString("value")

    // ... your logic ...

    ctx.setOutput("result", jsonQuote(value))
    return ctx.success()
}
```

### 3. Build

```bash
swift build --swift-sdk wasm32-unknown-wasi -c release
```

## Available Pin Types

| Enum Value  | JSON Name  | Description                      |
|------------ |----------- |--------------------------------- |
| `.exec`     | `Exec`     | Execution flow pin               |
| `.string`   | `String`   | Text value                       |
| `.i64`      | `I64`      | 64-bit integer                   |
| `.f64`      | `F64`      | 64-bit float                     |
| `.bool`     | `Bool`     | Boolean value                    |
| `.generic`  | `Generic`  | Any JSON-serializable value      |
| `.bytes`    | `Bytes`    | Raw bytes (base64 encoded)       |
| `.date`     | `Date`     | ISO 8601 date-time string        |
| `.pathBuf`  | `PathBuf`  | File system path                 |
| `.struct`   | `Struct`   | Typed JSON object with schema    |

## Context Methods

| Method                              | Description              |
|------------------------------------ |------------------------- |
| `ctx.getString(name, default)`      | Get string input         |
| `ctx.getI64(name, default)`         | Get integer input        |
| `ctx.getF64(name, default)`         | Get float input          |
| `ctx.getBool(name, default)`        | Get boolean input        |
| `ctx.setOutput(name, value)`        | Set output value         |
| `ctx.activateExec(pinName)`         | Activate an exec output  |
| `ctx.success()`                     | Finish with success      |
| `ctx.fail(error)`                   | Finish with error        |
| `ctx.debug(msg)`                    | Log debug message        |
| `ctx.info(msg)`                     | Log info message         |
| `ctx.warn(msg)`                     | Log warning              |
| `ctx.error(msg)`                    | Log error                |
| `ctx.streamText(text)`              | Stream text              |
| `ctx.streamJSON(data)`              | Stream JSON data         |
| `ctx.streamProgress(pct, msg)`      | Stream progress update   |

## Why Swift?

Swift via SwiftWasm produces Core WASM modules targeting `wasm32-unknown-wasi`:

- **Expressive syntax** — modern language features, strong typing, optionals
- **No Foundation needed** — the SDK uses hand-rolled JSON and memory management
- **Packed i64 ABI** — pointer/length pairs packed into `Int64` for efficient host interop
- **`@_cdecl` exports** — direct control over WASM export names
- **`@_extern(wasm)` imports** — native WASM host import bindings

## Building for Production

```bash
# Release build (optimized)
swift build --swift-sdk wasm32-unknown-wasi -c release

# Debug build (larger, with safety checks)
swift build --swift-sdk wasm32-unknown-wasi
```

## Troubleshooting

- **"no such module 'FlowLikeSDK'"**: Run `swift package resolve` and ensure the `../wasm-sdk-swift` directory exists
- **"no such SDK"**: Install the SwiftWasm SDK artifact bundle (see Prerequisites)
- **Missing exports**: Ensure `@_cdecl` functions are present for `get_node`, `get_nodes`, and `run`
- **Large binary**: Use `-c release` for optimized builds; Swift WASM binaries are larger than Zig/C but still reasonable
- **Linker errors for host functions**: The `@_extern(wasm)` declarations in the SDK produce WASM imports — they are resolved at runtime by the Flow-Like host, not at compile time
