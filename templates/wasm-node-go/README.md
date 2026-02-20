# Flow-Like WASM Node Template (Go)

This template provides a starting point for creating custom WASM nodes using Go with TinyGo.

## Prerequisites

- Go 1.22+
- TinyGo 0.34+: [https://tinygo.org/getting-started/install/](https://tinygo.org/getting-started/install/)

## Quick Start

1. **Build the WASM module:**
   ```bash
   tinygo build -o node.wasm -target wasm -no-debug ./
   ```

2. **Find the output:**
   ```
   node.wasm
   ```

3. **Copy to your Flow-Like project:**
   ```bash
   cp node.wasm /path/to/flow-like/wasm-nodes/
   ```

## Project Structure

```
wasm-node-go/
├── main.go           # Main node implementation
├── go.mod            # Go module config
├── flow-like.toml    # Flow-Like package manifest
└── README.md
```

## SDK Structure

The SDK lives in `../wasm-sdk-go/` and is referenced via `replace` in `go.mod`:

```
wasm-sdk-go/
├── sdk.go            # Entry point, ParseInput, SerializeDefinition/Result
├── types.go          # NodeDefinition, PinDefinition, ExecutionInput/Result, NodeScores
├── host.go           # Raw //go:wasmimport declarations and Go wrappers
├── context.go        # Context struct with high-level helpers
├── memory.go         # alloc/dealloc exports and memory helpers
└── go.mod
```

## Creating Your Node

### 1. Define the Node

Edit `main.go` and modify the `getNode()` function:

```go
//export get_node
func getNode() int64 {
    def := sdk.NewNodeDefinition()
    def.Name = "my_node"
    def.FriendlyName = "My Node"
    def.Description = "Does something useful"
    def.Category = "Custom/WASM"

    // Add pins
    def.AddPin(sdk.InputPin("exec", "Execute", "Trigger", "Exec"))
    def.AddPin(sdk.InputPin("value", "Value", "Input value", "String"))
    def.AddPin(sdk.OutputPin("exec_out", "Done", "Complete", "Exec"))
    def.AddPin(sdk.OutputPin("result", "Result", "Output", "String"))

    return sdk.SerializeDefinition(def)
}
```

### 2. Implement the Logic

Modify the `run()` function:

```go
//export run
func run(ptr uint32, length uint32) int64 {
    input := sdk.ParseInput(ptr, length)
    ctx := sdk.NewContext(input)

    value := ctx.GetString("value", "")
    // ... your logic ...
    ctx.SetOutput("result", sdk.JSONString(value))

    return sdk.SerializeResult(ctx.Success())
}
```

### 3. Build

```bash
tinygo build -o node.wasm -target wasm -no-debug ./
```

## Available Pin Types

| Type | Description |
|------|-------------|
| `Exec` | Execution flow pin |
| `String` | Text value |
| `I64` | 64-bit integer |
| `F64` | 64-bit float |
| `Bool` | Boolean value |
| `Generic` | Any JSON-serializable value |
| `Byte` | Raw bytes (base64 encoded) |
| `DateTime` | ISO 8601 date-time string |
| `PathBuf` | File system path |

## Context Methods

| Method | Description |
|--------|-------------|
| `ctx.GetString(name, default)` | Get string input |
| `ctx.GetI64(name, default)` | Get integer input |
| `ctx.GetF64(name, default)` | Get float input |
| `ctx.GetBool(name, default)` | Get boolean input |
| `ctx.SetOutput(name, value)` | Set output value |
| `ctx.ActivateExec(pinName)` | Activate an exec output |
| `ctx.Success()` | Finish with success (activates `exec_out`) |
| `ctx.Fail(error)` | Finish with error |
| `ctx.Debug(msg)` | Log debug message |
| `ctx.Info(msg)` | Log info message |
| `ctx.Warn(msg)` | Log warning |
| `ctx.Error(msg)` | Log error |
| `ctx.StreamText(text)` | Stream text (if streaming enabled) |
| `ctx.StreamJSON(data)` | Stream JSON data |
| `ctx.StreamProgress(pct, msg)` | Stream progress update |

## Why TinyGo?

Standard Go cannot compile to `wasm32-unknown-unknown` (core WASM). TinyGo provides:
- Small binary sizes (typically 100KB–1MB vs 10MB+ for standard Go WASM)
- `//go:wasmimport` support for host function imports
- `wasm` target that produces standalone `.wasm` files (no JavaScript glue needed)

## Troubleshooting

- **"cannot find package"**: Ensure `go.mod` has the `replace` directive pointing to `../wasm-sdk-go`
- **Large binary**: Use `-no-debug` flag and consider `-opt=z` for size optimization
- **Missing exports**: Make sure `main()` exists (even if empty) and functions use `//export` comments
