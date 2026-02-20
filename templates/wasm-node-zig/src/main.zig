// Flow-Like WASM Node Template (Zig)
//
// Build:
//
//   zig build -Doptimize=ReleaseSmall
//
// The compiled .wasm file will be at: zig-out/bin/node.wasm

const std = @import("std");
const sdk = @import("flow_like_sdk");

// ---------------------------------------------------------------------------
// get_node — returns the node definition as a packed i64 (ptr<<32 | len).
// ---------------------------------------------------------------------------

export fn get_node() i64 {
    var def = sdk.NodeDefinition.init(sdk.allocator);
    def.name = "my_custom_node_zig";
    def.friendly_name = "My Custom Node (Zig)";
    def.description = "A template WASM node built with Zig";
    def.category = "Custom/WASM";
    def.addPermission("streaming");

    def.addPin(sdk.inputPin("exec", "Execute", "Trigger execution", .exec));
    def.addPin(sdk.inputPin("input_text", "Input Text", "Text to process", .string).withDefault("\"\""));
    def.addPin(sdk.inputPin("multiplier", "Multiplier", "Number of times to repeat", .i64_type).withDefault("1"));

    def.addPin(sdk.outputPin("exec_out", "Done", "Execution complete", .exec));
    def.addPin(sdk.outputPin("output_text", "Output Text", "Processed text", .string));
    def.addPin(sdk.outputPin("char_count", "Character Count", "Number of characters in output", .i64_type));

    return sdk.serializeDefinition(&def);
}

export fn get_nodes() i64 {
    var def = sdk.NodeDefinition.init(sdk.allocator);
    def.name = "my_custom_node_zig";
    def.friendly_name = "My Custom Node (Zig)";
    def.description = "A template WASM node built with Zig";
    def.category = "Custom/WASM";
    def.addPermission("streaming");

    def.addPin(sdk.inputPin("exec", "Execute", "Trigger execution", .exec));
    def.addPin(sdk.inputPin("input_text", "Input Text", "Text to process", .string).withDefault("\"\""));
    def.addPin(sdk.inputPin("multiplier", "Multiplier", "Number of times to repeat", .i64_type).withDefault("1"));

    def.addPin(sdk.outputPin("exec_out", "Done", "Execution complete", .exec));
    def.addPin(sdk.outputPin("output_text", "Output Text", "Processed text", .string));
    def.addPin(sdk.outputPin("char_count", "Character Count", "Number of characters in output", .i64_type));

    const def_json = def.toJson(sdk.allocator) catch return 0;
    const nodes_json = std.fmt.allocPrint(sdk.allocator, "[{s}]", .{def_json}) catch return 0;
    return sdk.packResult(nodes_json);
}

// ---------------------------------------------------------------------------
// run — main execution function, called every time the node is triggered.
// ---------------------------------------------------------------------------

export fn run(ptr: u32, len: u32) i64 {
    const input = sdk.parseInput(ptr, len);
    var ctx = sdk.Context.init(input);
    const result = handleRun(&ctx);
    return sdk.serializeResult(&result);
}

fn handleRun(ctx: *sdk.Context) sdk.ExecutionResult {
    const input_text = ctx.getString("input_text", "");
    const multiplier = ctx.getI64("multiplier", 1);

    var buf = std.ArrayList(u8).init(sdk.allocator);
    var i: i64 = 0;
    while (i < multiplier) : (i += 1) {
        buf.appendSlice(input_text) catch {};
    }
    const output_text = buf.items;
    const char_count = output_text.len;

    var msg_buf: [64]u8 = undefined;
    const msg = std.fmt.bufPrint(&msg_buf, "Generated {d} characters", .{char_count}) catch "Generated characters";
    ctx.streamText(msg);

    ctx.setOutput("output_text", sdk.jsonString(output_text));

    const num_str = std.fmt.allocPrint(sdk.allocator, "{d}", .{char_count}) catch "0";
    ctx.setOutput("char_count", num_str);

    return ctx.success();
}

// ---------------------------------------------------------------------------
// Re-export memory management (required by the host ABI)
// ---------------------------------------------------------------------------

export fn alloc(size: u32) u32 {
    return sdk.wasmAlloc(size);
}

export fn dealloc(ptr_val: u32, size: u32) void {
    sdk.wasmDealloc(ptr_val, size);
}

export fn get_abi_version() u32 {
    return sdk.abi_version;
}
