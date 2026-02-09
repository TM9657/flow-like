/**
 * String Nodes - Text manipulation utilities
 *
 * This example demonstrates string processing nodes including
 * case conversion, splitting, joining, and text analysis.
 */

import {
	type ExecutionInput,
	ExecutionResult,
	NodeDefinition,
	PinDefinition,
	jsonString,
	parseInput,
	serializeDefinition,
	serializeResult,
} from "../assembly/sdk";

// ============================================================================
// Uppercase Node
// ============================================================================

export function get_uppercase_definition(): i64 {
	const def = new NodeDefinition();
	def.name = "string_uppercase_as";
	def.friendly_name = "To Uppercase (AS)";
	def.description = "Converts text to uppercase";
	def.category = "String/Transform";

	def.addPin(PinDefinition.input("exec", "Execute", "Trigger", "Exec"));
	def.addPin(
		PinDefinition.input(
			"text",
			"Text",
			"Text to convert",
			"String",
		).withDefault('""'),
	);
	def.addPin(PinDefinition.output("exec_out", "Done", "Complete", "Exec"));
	def.addPin(
		PinDefinition.output("result", "Result", "Uppercase text", "String"),
	);

	return serializeDefinition(def);
}

export function run_uppercase(ptr: i32, len: i32): i64 {
	const input = parseInput(ptr, len);
	const text = getStringInput(input, "text", "");

	const result = ExecutionResult.success();
	result.setOutput("result", jsonString(text.toUpperCase()));
	result.activateExec("exec_out");

	return serializeResult(result);
}

// ============================================================================
// Lowercase Node
// ============================================================================

export function get_lowercase_definition(): i64 {
	const def = new NodeDefinition();
	def.name = "string_lowercase_as";
	def.friendly_name = "To Lowercase (AS)";
	def.description = "Converts text to lowercase";
	def.category = "String/Transform";

	def.addPin(PinDefinition.input("exec", "Execute", "Trigger", "Exec"));
	def.addPin(
		PinDefinition.input(
			"text",
			"Text",
			"Text to convert",
			"String",
		).withDefault('""'),
	);
	def.addPin(PinDefinition.output("exec_out", "Done", "Complete", "Exec"));
	def.addPin(
		PinDefinition.output("result", "Result", "Lowercase text", "String"),
	);

	return serializeDefinition(def);
}

export function run_lowercase(ptr: i32, len: i32): i64 {
	const input = parseInput(ptr, len);
	const text = getStringInput(input, "text", "");

	const result = ExecutionResult.success();
	result.setOutput("result", jsonString(text.toLowerCase()));
	result.activateExec("exec_out");

	return serializeResult(result);
}

// ============================================================================
// Trim Node
// ============================================================================

export function get_trim_definition(): i64 {
	const def = new NodeDefinition();
	def.name = "string_trim_as";
	def.friendly_name = "Trim (AS)";
	def.description = "Removes leading and trailing whitespace";
	def.category = "String/Transform";

	def.addPin(PinDefinition.input("exec", "Execute", "Trigger", "Exec"));
	def.addPin(
		PinDefinition.input("text", "Text", "Text to trim", "String").withDefault(
			'""',
		),
	);
	def.addPin(PinDefinition.output("exec_out", "Done", "Complete", "Exec"));
	def.addPin(
		PinDefinition.output("result", "Result", "Trimmed text", "String"),
	);

	return serializeDefinition(def);
}

export function run_trim(ptr: i32, len: i32): i64 {
	const input = parseInput(ptr, len);
	const text = getStringInput(input, "text", "");

	const result = ExecutionResult.success();
	result.setOutput("result", jsonString(text.trim()));
	result.activateExec("exec_out");

	return serializeResult(result);
}

// ============================================================================
// String Length Node
// ============================================================================

export function get_length_definition(): i64 {
	const def = new NodeDefinition();
	def.name = "string_length_as";
	def.friendly_name = "String Length (AS)";
	def.description = "Returns the length of a string";
	def.category = "String/Analysis";

	def.addPin(PinDefinition.input("exec", "Execute", "Trigger", "Exec"));
	def.addPin(
		PinDefinition.input(
			"text",
			"Text",
			"Text to measure",
			"String",
		).withDefault('""'),
	);
	def.addPin(PinDefinition.output("exec_out", "Done", "Complete", "Exec"));
	def.addPin(
		PinDefinition.output("length", "Length", "Number of characters", "I64"),
	);
	def.addPin(
		PinDefinition.output(
			"is_empty",
			"Is Empty",
			"True if string is empty",
			"Bool",
		),
	);

	return serializeDefinition(def);
}

export function run_length(ptr: i32, len: i32): i64 {
	const input = parseInput(ptr, len);
	const text = getStringInput(input, "text", "");

	const result = ExecutionResult.success();
	result.setOutput("length", text.length.toString());
	result.setOutput("is_empty", (text.length == 0).toString());
	result.activateExec("exec_out");

	return serializeResult(result);
}

// ============================================================================
// Contains Node
// ============================================================================

export function get_contains_definition(): i64 {
	const def = new NodeDefinition();
	def.name = "string_contains_as";
	def.friendly_name = "Contains (AS)";
	def.description = "Checks if text contains a substring";
	def.category = "String/Analysis";

	def.addPin(PinDefinition.input("exec", "Execute", "Trigger", "Exec"));
	def.addPin(
		PinDefinition.input(
			"text",
			"Text",
			"Text to search in",
			"String",
		).withDefault('""'),
	);
	def.addPin(
		PinDefinition.input(
			"search",
			"Search",
			"Substring to find",
			"String",
		).withDefault('""'),
	);
	def.addPin(PinDefinition.output("exec_out", "Done", "Complete", "Exec"));
	def.addPin(
		PinDefinition.output("result", "Found", "True if substring found", "Bool"),
	);

	return serializeDefinition(def);
}

export function run_contains(ptr: i32, len: i32): i64 {
	const input = parseInput(ptr, len);
	const text = getStringInput(input, "text", "");
	const search = getStringInput(input, "search", "");

	const result = ExecutionResult.success();
	result.setOutput("result", text.includes(search).toString());
	result.activateExec("exec_out");

	return serializeResult(result);
}

// ============================================================================
// Replace Node
// ============================================================================

export function get_replace_definition(): i64 {
	const def = new NodeDefinition();
	def.name = "string_replace_as";
	def.friendly_name = "Replace (AS)";
	def.description = "Replaces occurrences of a pattern";
	def.category = "String/Transform";

	def.addPin(PinDefinition.input("exec", "Execute", "Trigger", "Exec"));
	def.addPin(
		PinDefinition.input("text", "Text", "Original text", "String").withDefault(
			'""',
		),
	);
	def.addPin(
		PinDefinition.input(
			"find",
			"Find",
			"Pattern to find",
			"String",
		).withDefault('""'),
	);
	def.addPin(
		PinDefinition.input(
			"replace_with",
			"Replace With",
			"Replacement text",
			"String",
		).withDefault('""'),
	);
	def.addPin(PinDefinition.output("exec_out", "Done", "Complete", "Exec"));
	def.addPin(
		PinDefinition.output("result", "Result", "Modified text", "String"),
	);

	return serializeDefinition(def);
}

export function run_replace(ptr: i32, len: i32): i64 {
	const input = parseInput(ptr, len);
	const text = getStringInput(input, "text", "");
	const find = getStringInput(input, "find", "");
	const replaceWith = getStringInput(input, "replace_with", "");

	const result = ExecutionResult.success();
	result.setOutput("result", jsonString(text.replaceAll(find, replaceWith)));
	result.activateExec("exec_out");

	return serializeResult(result);
}

// ============================================================================
// Concat Node
// ============================================================================

export function get_concat_definition(): i64 {
	const def = new NodeDefinition();
	def.name = "string_concat_as";
	def.friendly_name = "Concatenate (AS)";
	def.description = "Joins two strings together";
	def.category = "String/Transform";

	def.addPin(PinDefinition.input("exec", "Execute", "Trigger", "Exec"));
	def.addPin(
		PinDefinition.input("a", "A", "First string", "String").withDefault('""'),
	);
	def.addPin(
		PinDefinition.input("b", "B", "Second string", "String").withDefault('""'),
	);
	def.addPin(
		PinDefinition.input(
			"separator",
			"Separator",
			"Text between strings",
			"String",
		).withDefault('""'),
	);
	def.addPin(PinDefinition.output("exec_out", "Done", "Complete", "Exec"));
	def.addPin(
		PinDefinition.output("result", "Result", "Combined string", "String"),
	);

	return serializeDefinition(def);
}

export function run_concat(ptr: i32, len: i32): i64 {
	const input = parseInput(ptr, len);
	const a = getStringInput(input, "a", "");
	const b = getStringInput(input, "b", "");
	const separator = getStringInput(input, "separator", "");

	const result = ExecutionResult.success();
	result.setOutput("result", jsonString(a + separator + b));
	result.activateExec("exec_out");

	return serializeResult(result);
}

// ============================================================================
// Helpers
// ============================================================================

function getStringInput(
	input: ExecutionInput,
	name: string,
	defaultValue: string,
): string {
	if (input.inputs.has(name)) {
		const value = input.inputs.get(name);
		if (value.startsWith('"') && value.endsWith('"')) {
			return value.slice(1, value.length - 1);
		}
		return value;
	}
	return defaultValue;
}
