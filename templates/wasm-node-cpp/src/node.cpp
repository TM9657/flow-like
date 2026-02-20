/**
 * Flow-Like WASM Node Template (C++)
 *
 * A template for creating custom nodes in C++ that compile to WebAssembly
 * via Emscripten.
 *
 * Building:
 *   mkdir build && cd build
 *   emcmake cmake ..
 *   emmake make
 *
 * The compiled .wasm file will be at: build/node.wasm
 */

#include "flow_like_sdk.h"

using namespace flowlike;

// ============================================================================
// Node Definition
// ============================================================================

static NodeDefinition build_definition() {
    NodeDefinition def;
    def.name          = "my_custom_node_cpp";
    def.friendly_name = "My Custom Node (C++)";
    def.description   = "A template WASM node built with C++";
    def.category      = "Custom/WASM";
    def.add_permission("streaming");

    // Input pins
    def.add_pin(PinDefinition::input("exec",       "Execute",    "Trigger execution",          DataType::Exec));
    def.add_pin(PinDefinition::input("input_text",  "Input Text", "Text to process",           DataType::String).with_default("\"\""));
    def.add_pin(PinDefinition::input("multiplier",  "Multiplier", "Number of times to repeat", DataType::I64).with_default("1"));

    // Output pins
    def.add_pin(PinDefinition::output("exec_out",    "Done",            "Execution complete",               DataType::Exec));
    def.add_pin(PinDefinition::output("output_text", "Output Text",     "Processed text",                   DataType::String));
    def.add_pin(PinDefinition::output("char_count",  "Character Count", "Number of characters in output",   DataType::I64));

    return def;
}

// ============================================================================
// Node Execution
// ============================================================================

static ExecutionResult handle_run(Context& ctx) {
    // Read inputs
    std::string input_text = ctx.get_string("input_text");
    int64_t multiplier     = ctx.get_i64("multiplier", 1);
    if (multiplier < 0) multiplier = 0;

    ctx.debug("Processing: '" + input_text + "' x " + std::to_string(multiplier));

    // Repeat the text
    std::string output;
    output.reserve(input_text.size() * static_cast<size_t>(multiplier));
    for (int64_t i = 0; i < multiplier; ++i) {
        output += input_text;
    }
    int64_t char_count = static_cast<int64_t>(output.size());

    // Stream progress
    ctx.stream_text("Generated " + std::to_string(char_count) + " characters");

    // Set outputs (values must be valid JSON)
    ctx.set_output("output_text", json_quote(output));
    ctx.set_output("char_count",  std::to_string(char_count));

    return ctx.success();
}

// ============================================================================
// WASM Exports
// ============================================================================

extern "C" {

__attribute__((export_name("get_node")))
int64_t get_node() {
    static NodeDefinition def = build_definition();
    return serialize_definition(def);
}

__attribute__((export_name("get_nodes")))
int64_t get_nodes() {
    static NodeDefinition def = build_definition();
    return pack_result("[" + def.to_json() + "]");
}

__attribute__((export_name("run")))
int64_t run(uint32_t ptr, uint32_t len) {
    std::string raw(reinterpret_cast<const char*>(ptr), len);
    ExecutionInput input = parse_execution_input(raw);
    Context ctx(input);
    ExecutionResult result = handle_run(ctx);
    return serialize_result(result);
}

__attribute__((export_name("alloc")))
uint32_t alloc(uint32_t size) {
    return flow_like_alloc(size);
}

__attribute__((export_name("dealloc")))
void dealloc(uint32_t ptr, uint32_t size) {
    flow_like_dealloc(ptr, size);
}

__attribute__((export_name("get_abi_version")))
uint32_t get_abi_version() {
    return flowlike::ABI_VERSION;
}

}  // extern "C"
