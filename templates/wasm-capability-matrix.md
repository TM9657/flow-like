# WASM Template Capability Matrix

This matrix tracks template/runtime parity across language targets in `templates/`.

## Runtime ABI

| Template | Runtime Format | `get_node` | `get_nodes` | `run` | `alloc/dealloc` |
|---|---|---:|---:|---:|---:|
| Rust (`wasm-node-rust`) | Core WASM module | ✅ | ✅ (SDK package macro) | ✅ | ✅ |
| AssemblyScript (`wasm-node-assemblyscript`) | Core WASM module | ✅ | ✅ | ✅ | ✅ |
| Go (`wasm-node-go`) | Core WASM module | ✅ | ✅ | ✅ | ✅ |
| C++ (`wasm-node-cpp`) | Core WASM module | ✅ | ✅ | ✅ | ✅ |
| Kotlin (`wasm-node-kotlin`) | Core WASM module (GC/EH enabled) | ✅ | ✅ | ✅ | ✅ |
| Zig (`wasm-node-zig`) | Core WASM module | ✅ | ✅ | ✅ | ✅ |
| C# (`wasm-node-csharp`) | WASM Component Model (`wasip2`) | ✅ (`get-node`) | ✅ (`get-nodes`) | ✅ | N/A (string ABI) |
| Nim (`wasm-node-nim`) | Core WASM module (Emscripten) | ✅ | ✅ | ✅ | ✅ |
| Lua (`wasm-node-lua`) | Core WASM module (Emscripten) | ✅ | ✅ | ✅ | ✅ |
| Swift (`wasm-node-swift`) | Core WASM module (SwiftWasm) | ✅ | ✅ | ✅ | ✅ |
| Java (`wasm-node-java`) | Core WASM module (TeaVM) | ✅ | ✅ | ✅ | ✅ |
| Grain (`wasm-node-grain`) | Core WASM module | ✅ | ✅ | ✅ | ✅ |
| MoonBit (`wasm-node-moonbit`) | Core WASM module | ✅ | ✅ | ✅ | ✅ |

## Host API Surface (SDK)

| SDK | log | pins | vars | cache | meta | stream | storage | models | http | auth |
|---|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| Rust (`wasm-sdk-rust`) | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ |
| Go (`wasm-sdk-go`) | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ |
| C++ (`wasm-sdk-cpp`) | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ |
| Kotlin (`wasm-sdk-kotlin`) | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ |
| Zig (`wasm-sdk-zig`) | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ |
| AssemblyScript (`wasm-node-assemblyscript/assembly/sdk.ts`) | ⚠️ partial (`env` compatibility layer) | ⚠️ partial | ⚠️ partial | ❌ | ⚠️ limited | ⚠️ partial | ❌ | ❌ | ❌ | ❌ |
| C# (`wasm-sdk-csharp`) | ✅ (component host bridge) | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ | ❌ (no direct runtime bridge yet) | ✅ |
| Nim (`wasm-sdk-nim`) | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ |
| Lua (`wasm-sdk-lua`) | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ |
| Swift (`wasm-sdk-swift`) | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ |
| Java (`wasm-sdk-java`) | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ |
| Grain (`wasm-sdk-grain`) | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ |
| MoonBit (`wasm-sdk-moonbit`) | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ |

## Notes

- Core module templates execute through `WasmModule` + `WasmInstance`.
- C# template executes through `WasmComponent` + `WasmComponentInstance`.
- Kotlin requires engine support for GC + exceptions + function references.
- For C# publishing, use `WasmSingleFileBundle=true` with a local `wasi-sdk` toolchain.
- Nim compiles to C, then uses Emscripten to produce a core WASM module (same approach as C++).
- Lua embeds a Lua 5.4 interpreter in C, compiled with Emscripten (same approach as C++/Nim).
- Swift compiles via SwiftWasm toolchain targeting `wasm32-unknown-wasi`.
- Java compiles via TeaVM, which converts Java bytecode to WebAssembly.
- Grain compiles to WASM natively; use `--no-gc` and `--use-start-section` for host ABI compatibility.
- MoonBit compiles to WASM natively; uses bump allocator for linear memory alongside MoonBit's own GC.
