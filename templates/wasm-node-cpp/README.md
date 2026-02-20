# Flow-Like WASM Node Template (C++)

This template provides a starting point for creating custom WASM nodes using C++ and Emscripten.

## Prerequisites

- [Emscripten SDK (emsdk)](https://emscripten.org/docs/getting_started/downloads.html)
- CMake 3.10+

### Installing Emscripten

```bash
git clone https://github.com/emscripten-core/emsdk.git
cd emsdk
./emsdk install latest
./emsdk activate latest
source ./emsdk_env.sh
```

## Quick Start

1. **Build the WASM module:**
   ```bash
   mkdir build && cd build
   emcmake cmake ..
   emmake make
   ```

2. **Find the output:**
   ```
   build/node.wasm
   ```

3. **Copy to your Flow-Like project:**
   ```bash
   cp build/node.wasm /path/to/flow-like/wasm-nodes/
   ```

## Project Structure

```
wasm-node-cpp/
├── src/
│   └── node.cpp          # Main node implementation
├── flow-like.toml        # Package manifest
├── CMakeLists.txt        # Build configuration
└── README.md

wasm-sdk-cpp/             # SDK (referenced by CMakeLists.txt)
├── include/
│   └── flow_like_sdk.h   # Main SDK header
├── src/
│   ├── sdk.cpp           # SDK implementation
│   └── json.h            # Lightweight JSON utilities
└── CMakeLists.txt
```

## Creating Your Node

### 1. Define the Node

Edit `src/node.cpp` and modify the `build_definition()` function:

```cpp
static NodeDefinition build_definition() {
    NodeDefinition def;
    def.name          = "my_node";
    def.friendly_name = "My Node";
    def.description   = "Does something useful";
    def.category      = "Custom/WASM";

    // Input pins
    def.add_pin(PinDefinition::input("exec", "Execute", "Trigger", DataType::Exec));
    def.add_pin(PinDefinition::input("value", "Value", "Input value", DataType::String)
        .with_default("\"\""));

    // Output pins
    def.add_pin(PinDefinition::output("exec_out", "Done", "Complete", DataType::Exec));
    def.add_pin(PinDefinition::output("result", "Result", "Output value", DataType::String));

    return def;
}
```

### 2. Implement the Logic

Modify `handle_run()`:

```cpp
static ExecutionResult handle_run(Context& ctx) {
    std::string value = ctx.get_string("value");

    // Your logic here
    std::string result = do_something(value);

    ctx.set_output("result", json::quote(result));
    return ctx.success();
}
```

### 3. Build & Test

```bash
cd build
emmake make
```

## Available Pin Types

| Type | C++ Enum | Description |
|------|----------|-------------|
| `Exec` | `DataType::Exec` | Execution flow trigger |
| `String` | `DataType::String` | Text / JSON string |
| `I64` | `DataType::I64` | 64-bit integer |
| `F64` | `DataType::F64` | 64-bit float |
| `Bool` | `DataType::Bool` | Boolean |
| `Generic` | `DataType::Generic` | Any serialisable JSON value |
| `Bytes` | `DataType::Bytes` | Binary data (base64) |

## Context API

```cpp
// Read inputs
std::string s = ctx.get_string("name", "default");
int64_t n     = ctx.get_i64("count", 0);
double d      = ctx.get_f64("ratio", 1.0);
bool b        = ctx.get_bool("flag", false);

// Write outputs (values must be valid JSON)
ctx.set_output("text", json::quote("hello"));
ctx.set_output("count", std::to_string(42));
ctx.set_output("flag", "true");

// Exec flow
ctx.activate_exec("exec_out");

// Logging (level-gated)
ctx.debug("verbose info");
ctx.info("normal info");
ctx.warn("warning");
ctx.error("error");

// Streaming (only sent when streaming is enabled)
ctx.stream_text("progress update");
ctx.stream_progress(0.5f, "Halfway done");
ctx.stream_json("{\"key\":\"value\"}");

// Finalise
return ctx.success();   // activates exec_out + finish
return ctx.fail("msg"); // sets error + finish
```

## Emscripten Tips

- Use `-O2` or `-Os` for smaller WASM output.
- Avoid C++ exceptions (`-fno-exceptions`) for smaller binaries.
- The template disables RTTI (`-fno-rtti`) for the same reason.
- Memory growth is enabled by default (`-sALLOW_MEMORY_GROWTH=1`).

## Troubleshooting

| Issue | Solution |
|-------|----------|
| `emcmake` not found | Run `source /path/to/emsdk/emsdk_env.sh` |
| Linker errors about missing host functions | Add `-sERROR_ON_UNDEFINED_SYMBOLS=0` (already set in CMakeLists.txt) |
| WASM too large | Enable `-Os` optimisation, strip debug info |
| Runtime crash | Check that `alloc`/`dealloc` exports are present |
