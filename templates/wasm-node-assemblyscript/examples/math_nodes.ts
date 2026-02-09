/**
 * Math Nodes - Basic arithmetic and mathematical operations
 *
 * This example demonstrates creating nodes for arithmetic operations.
 */

import {
	type ExecutionInput,
	ExecutionResult,
	NodeDefinition,
	PinDefinition,
	parseInput,
	serializeDefinition,
	serializeResult,
} from "../assembly/sdk";

// ============================================================================
// Add Node
// ============================================================================

export function get_add_definition(): i64 {
	const def = new NodeDefinition();
	def.name = "math_add_as";
	def.friendly_name = "Add (AS)";
	def.description = "Adds two numbers together";
	def.category = "Math/Arithmetic";

	def.addPin(PinDefinition.input("exec", "Execute", "Trigger", "Exec"));
	def.addPin(
		PinDefinition.input("a", "A", "First number", "F64").withDefault("0.0"),
	);
	def.addPin(
		PinDefinition.input("b", "B", "Second number", "F64").withDefault("0.0"),
	);
	def.addPin(PinDefinition.output("exec_out", "Done", "Complete", "Exec"));
	def.addPin(PinDefinition.output("result", "Result", "Sum of A and B", "F64"));

	return serializeDefinition(def);
}

export function run_add(ptr: i32, len: i32): i64 {
	const input = parseInput(ptr, len);
	const a = getF64Input(input, "a", 0.0);
	const b = getF64Input(input, "b", 0.0);

	const result = ExecutionResult.success();
	result.setOutput("result", (a + b).toString());
	result.activateExec("exec_out");

	return serializeResult(result);
}

// ============================================================================
// Subtract Node
// ============================================================================

export function get_subtract_definition(): i64 {
	const def = new NodeDefinition();
	def.name = "math_subtract_as";
	def.friendly_name = "Subtract (AS)";
	def.description = "Subtracts B from A";
	def.category = "Math/Arithmetic";

	def.addPin(PinDefinition.input("exec", "Execute", "Trigger", "Exec"));
	def.addPin(
		PinDefinition.input("a", "A", "First number", "F64").withDefault("0.0"),
	);
	def.addPin(
		PinDefinition.input("b", "B", "Second number", "F64").withDefault("0.0"),
	);
	def.addPin(PinDefinition.output("exec_out", "Done", "Complete", "Exec"));
	def.addPin(PinDefinition.output("result", "Result", "A minus B", "F64"));

	return serializeDefinition(def);
}

export function run_subtract(ptr: i32, len: i32): i64 {
	const input = parseInput(ptr, len);
	const a = getF64Input(input, "a", 0.0);
	const b = getF64Input(input, "b", 0.0);

	const result = ExecutionResult.success();
	result.setOutput("result", (a - b).toString());
	result.activateExec("exec_out");

	return serializeResult(result);
}

// ============================================================================
// Multiply Node
// ============================================================================

export function get_multiply_definition(): i64 {
	const def = new NodeDefinition();
	def.name = "math_multiply_as";
	def.friendly_name = "Multiply (AS)";
	def.description = "Multiplies two numbers";
	def.category = "Math/Arithmetic";

	def.addPin(PinDefinition.input("exec", "Execute", "Trigger", "Exec"));
	def.addPin(
		PinDefinition.input("a", "A", "First number", "F64").withDefault("1.0"),
	);
	def.addPin(
		PinDefinition.input("b", "B", "Second number", "F64").withDefault("1.0"),
	);
	def.addPin(PinDefinition.output("exec_out", "Done", "Complete", "Exec"));
	def.addPin(PinDefinition.output("result", "Result", "A times B", "F64"));

	return serializeDefinition(def);
}

export function run_multiply(ptr: i32, len: i32): i64 {
	const input = parseInput(ptr, len);
	const a = getF64Input(input, "a", 1.0);
	const b = getF64Input(input, "b", 1.0);

	const result = ExecutionResult.success();
	result.setOutput("result", (a * b).toString());
	result.activateExec("exec_out");

	return serializeResult(result);
}

// ============================================================================
// Divide Node
// ============================================================================

export function get_divide_definition(): i64 {
	const def = new NodeDefinition();
	def.name = "math_divide_as";
	def.friendly_name = "Divide (AS)";
	def.description = "Divides A by B";
	def.category = "Math/Arithmetic";

	def.addPin(PinDefinition.input("exec", "Execute", "Trigger", "Exec"));
	def.addPin(
		PinDefinition.input("a", "A", "Dividend", "F64").withDefault("0.0"),
	);
	def.addPin(
		PinDefinition.input("b", "B", "Divisor", "F64").withDefault("1.0"),
	);
	def.addPin(PinDefinition.output("exec_out", "Done", "Complete", "Exec"));
	def.addPin(PinDefinition.output("result", "Result", "A divided by B", "F64"));
	def.addPin(
		PinDefinition.output(
			"is_valid",
			"Valid",
			"False if division by zero",
			"Bool",
		),
	);

	return serializeDefinition(def);
}

export function run_divide(ptr: i32, len: i32): i64 {
	const input = parseInput(ptr, len);
	const a = getF64Input(input, "a", 0.0);
	const b = getF64Input(input, "b", 1.0);

	const result = ExecutionResult.success();

	if (b == 0.0) {
		result.setOutput("result", "0.0");
		result.setOutput("is_valid", "false");
	} else {
		result.setOutput("result", (a / b).toString());
		result.setOutput("is_valid", "true");
	}

	result.activateExec("exec_out");
	return serializeResult(result);
}

// ============================================================================
// Clamp Node
// ============================================================================

export function get_clamp_definition(): i64 {
	const def = new NodeDefinition();
	def.name = "math_clamp_as";
	def.friendly_name = "Clamp (AS)";
	def.description = "Clamps a value between min and max";
	def.category = "Math/Utility";

	def.addPin(PinDefinition.input("exec", "Execute", "Trigger", "Exec"));
	def.addPin(
		PinDefinition.input("value", "Value", "Value to clamp", "F64").withDefault(
			"0.0",
		),
	);
	def.addPin(
		PinDefinition.input("min", "Min", "Minimum value", "F64").withDefault(
			"0.0",
		),
	);
	def.addPin(
		PinDefinition.input("max", "Max", "Maximum value", "F64").withDefault(
			"1.0",
		),
	);
	def.addPin(PinDefinition.output("exec_out", "Done", "Complete", "Exec"));
	def.addPin(PinDefinition.output("result", "Result", "Clamped value", "F64"));

	return serializeDefinition(def);
}

export function run_clamp(ptr: i32, len: i32): i64 {
	const input = parseInput(ptr, len);
	const value = getF64Input(input, "value", 0.0);
	const min = getF64Input(input, "min", 0.0);
	const max = getF64Input(input, "max", 1.0);

	const clamped = Math.max(min, Math.min(max, value));

	const result = ExecutionResult.success();
	result.setOutput("result", clamped.toString());
	result.activateExec("exec_out");

	return serializeResult(result);
}

// ============================================================================
// Helpers
// ============================================================================

function getF64Input(
	input: ExecutionInput,
	name: string,
	defaultValue: f64,
): f64 {
	if (input.inputs.has(name)) {
		return F64.parseFloat(input.inputs.get(name));
	}
	return defaultValue;
}
