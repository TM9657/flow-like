/**
 * Flow-Like WASM Node Template (AssemblyScript)
 *
 * This is a template for creating custom nodes in AssemblyScript that compile to WebAssembly.
 *
 * # Building
 *
 * ```bash
 * npm install
 * npm run build
 * ```
 *
 * The compiled `.wasm` file will be at: `build/release.wasm`
 */

import {
	type ExecutionInput,
	ExecutionResult,
	NodeDefinition,
	PinDefinition,
	debug,
	jsonString,
	parseInput,
	serializeDefinition,
	serializeResult,
	streamText,
} from "./sdk";

// ============================================================================
// Node Definition
// ============================================================================

/**
 * Define your node structure.
 * This function is called once when the module is loaded.
 */
export function get_node(): i64 {
	const def = new NodeDefinition();
	def.name = "my_custom_node_as";
	def.friendly_name = "My Custom Node (AS)";
	def.description = "A template WASM node built with AssemblyScript";
	def.category = "Custom/WASM";

	// Add input pins
	def.addPin(
		PinDefinition.input("exec", "Execute", "Trigger execution", "Exec"),
	);
	def.addPin(
		PinDefinition.input(
			"input_text",
			"Input Text",
			"Text to process",
			"String",
		).withDefault('""'),
	);
	def.addPin(
		PinDefinition.input(
			"multiplier",
			"Multiplier",
			"Number of times to repeat",
			"I64",
		).withDefault("1"),
	);

	// Add output pins
	def.addPin(
		PinDefinition.output("exec_out", "Done", "Execution complete", "Exec"),
	);
	def.addPin(
		PinDefinition.output(
			"output_text",
			"Output Text",
			"Processed text",
			"String",
		),
	);
	def.addPin(
		PinDefinition.output(
			"char_count",
			"Character Count",
			"Number of characters in output",
			"I64",
		),
	);

	return serializeDefinition(def);
}

// ============================================================================
// Node Execution
// ============================================================================

/**
 * Main execution function.
 * This is called every time the node is executed.
 */
export function run(ptr: i32, len: i32): i64 {
	// Parse the execution input
	const input = parseInput(ptr, len);

	// Get input values
	const inputText = getStringInput(input, "input_text", "");
	const multiplier = getI64Input(input, "multiplier", 1);

	// Log for debugging
	debug(`Processing: '${inputText}' x ${multiplier}`);

	// Process - repeat the text
	let outputText = "";
	for (let i: i64 = 0; i < multiplier; i++) {
		outputText += inputText;
	}
	const charCount = outputText.length;

	// Stream progress if enabled
	if (input.stream_state) {
		streamText(`Generated ${charCount} characters`);
	}

	// Build result
	const result = ExecutionResult.success();
	result.setOutput("output_text", jsonString(outputText));
	result.setOutput("char_count", charCount.toString());
	result.activateExec("exec_out");

	return serializeResult(result);
}

// ============================================================================
// Helper Functions
// ============================================================================

function getStringInput(
	input: ExecutionInput,
	name: string,
	defaultValue: string,
): string {
	if (input.inputs.has(name)) {
		const value = input.inputs.get(name);
		// Parse JSON string
		if (value.startsWith('"') && value.endsWith('"')) {
			return value.slice(1, value.length - 1);
		}
		return value;
	}
	return defaultValue;
}

function getI64Input(
	input: ExecutionInput,
	name: string,
	defaultValue: i64,
): i64 {
	if (input.inputs.has(name)) {
		const value = input.inputs.get(name);
		return I64.parseInt(value);
	}
	return defaultValue;
}

function getF64Input(
	input: ExecutionInput,
	name: string,
	defaultValue: f64,
): f64 {
	if (input.inputs.has(name)) {
		const value = input.inputs.get(name);
		return F64.parseFloat(value);
	}
	return defaultValue;
}

function getBoolInput(
	input: ExecutionInput,
	name: string,
	defaultValue: bool,
): bool {
	if (input.inputs.has(name)) {
		const value = input.inputs.get(name);
		return value == "true";
	}
	return defaultValue;
}

// ============================================================================
// Memory Exports (required by host)
// ============================================================================

export { alloc, dealloc, get_abi_version } from "./sdk";
