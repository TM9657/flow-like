// Flow-Like WASM Node Template (Go / TinyGo)
//
// Build:
//
//	tinygo build -o node.wasm -target wasm -no-debug ./
//
// The compiled .wasm file will be at: node.wasm
package main

import (
	"strconv"
	"strings"

	sdk "github.com/example/flow-like-wasm-sdk-go"
)

// get_node returns the node definition as a packed i64 (ptr<<32|len).
//
//export get_node
func getNode() int64 {
	def := sdk.NewNodeDefinition()
	def.Name = "my_custom_node_go"
	def.FriendlyName = "My Custom Node (Go)"
	def.Description = "A template WASM node built with Go / TinyGo"
	def.Category = "Custom/WASM"
	def.AddPermission("streaming")

	def.AddPin(sdk.InputPin("exec", "Execute", "Trigger execution", "Exec"))
	def.AddPin(sdk.InputPin("input_text", "Input Text", "Text to process", "String").WithDefault(`""`))
	def.AddPin(sdk.InputPin("multiplier", "Multiplier", "Number of times to repeat", "I64").WithDefault("1"))

	def.AddPin(sdk.OutputPin("exec_out", "Done", "Execution complete", "Exec"))
	def.AddPin(sdk.OutputPin("output_text", "Output Text", "Processed text", "String"))
	def.AddPin(sdk.OutputPin("char_count", "Character Count", "Number of characters in output", "I64"))

	return sdk.SerializeDefinition(def)
}

// get_nodes returns all node definitions as a packed i64 (ptr<<32|len).
//
//export get_nodes
func getNodes() int64 {
	def := sdk.NewNodeDefinition()
	def.Name = "my_custom_node_go"
	def.FriendlyName = "My Custom Node (Go)"
	def.Description = "A template WASM node built with Go / TinyGo"
	def.Category = "Custom/WASM"
	def.AddPermission("streaming")

	def.AddPin(sdk.InputPin("exec", "Execute", "Trigger execution", "Exec"))
	def.AddPin(sdk.InputPin("input_text", "Input Text", "Text to process", "String").WithDefault(`""`))
	def.AddPin(sdk.InputPin("multiplier", "Multiplier", "Number of times to repeat", "I64").WithDefault("1"))

	def.AddPin(sdk.OutputPin("exec_out", "Done", "Execution complete", "Exec"))
	def.AddPin(sdk.OutputPin("output_text", "Output Text", "Processed text", "String"))
	def.AddPin(sdk.OutputPin("char_count", "Character Count", "Number of characters in output", "I64"))

	return sdk.PackResult("[" + def.ToJSON() + "]")
}

// run is the main execution function, called every time the node is triggered.
//
//export run
func run(ptr uint32, length uint32) int64 {
	input := sdk.ParseInput(ptr, length)
	ctx := sdk.NewContext(input)

	inputText := ctx.GetString("input_text", "")
	multiplier := ctx.GetI64("multiplier", 1)

	ctx.Debug("Processing: '" + inputText + "' x " + strconv.FormatInt(multiplier, 10))

	var b strings.Builder
	for i := int64(0); i < multiplier; i++ {
		b.WriteString(inputText)
	}
	outputText := b.String()
	charCount := len(outputText)

	ctx.StreamText("Generated " + strconv.Itoa(charCount) + " characters")

	ctx.SetOutput("output_text", sdk.JSONString(outputText))
	ctx.SetOutput("char_count", strconv.Itoa(charCount))

	return sdk.SerializeResult(ctx.Success())
}

func main() {}
