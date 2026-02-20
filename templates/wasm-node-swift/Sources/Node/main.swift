// Flow-Like WASM Node Template (Swift)
//
// Build:
//   swift build --swift-sdk wasm32-unknown-wasi -c release
//
// The compiled .wasm file will be at:
//   .build/release/Node.wasm

import FlowLikeSDK

// MARK: - Node Definition

func buildDefinition() -> NodeDefinition {
    var def = NodeDefinition()
    def.name = "my_custom_node_swift"
    def.friendlyName = "My Custom Node (Swift)"
    def.description = "A template WASM node built with Swift"
    def.category = "Custom/WASM"

    def.addPin(inputPin("exec", "Execute", "Trigger execution", .exec))
    def.addPin(inputPin("input_text", "Input Text", "Text to process", .string).withDefault("\"\""))
    def.addPin(inputPin("multiplier", "Multiplier", "Number of times to repeat", .i64).withDefault("1"))

    def.addPin(outputPin("exec_out", "Done", "Execution complete", .exec))
    def.addPin(outputPin("output_text", "Output Text", "Processed text", .string))
    def.addPin(outputPin("char_count", "Character Count", "Number of characters in output", .i64))

    return def
}

// MARK: - Node Execution

func handleRun(_ ctx: inout Context) -> ExecutionResult {
    let inputText = ctx.getString("input_text")
    let multiplier = ctx.getI64("multiplier", 1)

    ctx.debug("Processing: '\(inputText)' x \(multiplier)")

    var output = ""
    var i: Int64 = 0
    while i < multiplier {
        output += inputText
        i += 1
    }
    let charCount = output.count

    ctx.streamText("Generated \(charCount) characters")

    ctx.setOutput("output_text", jsonQuote(output))
    ctx.setOutput("char_count", "\(charCount)")

    return ctx.success()
}

// MARK: - WASM Exports

@_cdecl("get_node")
func getNode() -> Int64 {
    return serializeDefinition(buildDefinition())
}

@_cdecl("get_nodes")
func getNodes() -> Int64 {
    let def = buildDefinition()
    return packResult("[" + def.toJSON() + "]")
}

@_cdecl("run")
func wasmRun(_ ptr: UInt32, _ len: UInt32) -> Int64 {
    let input = parseInput(ptr: ptr, length: len)
    var ctx = Context(input: input)
    let result = handleRun(&ctx)
    return serializeResult(result)
}
