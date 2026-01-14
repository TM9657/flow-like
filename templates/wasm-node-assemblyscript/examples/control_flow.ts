/**
 * Control Flow Nodes - Logic and branching utilities
 *
 * This example demonstrates control flow nodes including
 * conditionals, comparisons, and logic gates.
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
// If Branch Node
// ============================================================================

export function get_if_branch_definition(): i64 {
	const def = new NodeDefinition();
	def.name = "if_branch_as";
	def.friendly_name = "If Branch (AS)";
	def.description = "Branches execution based on a condition";
	def.category = "Control/Flow";

	def.addPin(PinDefinition.input("exec", "Execute", "Trigger", "Exec"));
	def.addPin(
		PinDefinition.input(
			"condition",
			"Condition",
			"Branch condition",
			"Bool",
		).withDefault("false"),
	);
	def.addPin(
		PinDefinition.output("true_branch", "True", "Executes if true", "Exec"),
	);
	def.addPin(
		PinDefinition.output("false_branch", "False", "Executes if false", "Exec"),
	);

	return serializeDefinition(def);
}

export function run_if_branch(ptr: i32, len: i32): i64 {
	const input = parseInput(ptr, len);
	const condition = getBoolInput(input, "condition", false);

	const result = ExecutionResult.success();
	if (condition) {
		result.activateExec("true_branch");
	} else {
		result.activateExec("false_branch");
	}

	return serializeResult(result);
}

// ============================================================================
// Compare Node
// ============================================================================

export function get_compare_definition(): i64 {
	const def = new NodeDefinition();
	def.name = "compare_as";
	def.friendly_name = "Compare (AS)";
	def.description = "Compares two numbers";
	def.category = "Control/Logic";

	def.addPin(PinDefinition.input("exec", "Execute", "Trigger", "Exec"));
	def.addPin(
		PinDefinition.input("a", "A", "First value", "F64").withDefault("0.0"),
	);
	def.addPin(
		PinDefinition.input("b", "B", "Second value", "F64").withDefault("0.0"),
	);
	def.addPin(PinDefinition.output("exec_out", "Done", "Complete", "Exec"));
	def.addPin(
		PinDefinition.output("equal", "A == B", "Values are equal", "Bool"),
	);
	def.addPin(
		PinDefinition.output("less_than", "A < B", "A is less than B", "Bool"),
	);
	def.addPin(
		PinDefinition.output(
			"greater_than",
			"A > B",
			"A is greater than B",
			"Bool",
		),
	);

	return serializeDefinition(def);
}

export function run_compare(ptr: i32, len: i32): i64 {
	const input = parseInput(ptr, len);
	const a = getF64Input(input, "a", 0.0);
	const b = getF64Input(input, "b", 0.0);

	const result = ExecutionResult.success();
	result.setOutput("equal", (a == b).toString());
	result.setOutput("less_than", (a < b).toString());
	result.setOutput("greater_than", (a > b).toString());
	result.activateExec("exec_out");

	return serializeResult(result);
}

// ============================================================================
// AND Gate Node
// ============================================================================

export function get_and_gate_definition(): i64 {
	const def = new NodeDefinition();
	def.name = "and_gate_as";
	def.friendly_name = "AND Gate (AS)";
	def.description = "Returns true only if both inputs are true";
	def.category = "Control/Logic";

	def.addPin(PinDefinition.input("exec", "Execute", "Trigger", "Exec"));
	def.addPin(
		PinDefinition.input("a", "A", "First input", "Bool").withDefault("false"),
	);
	def.addPin(
		PinDefinition.input("b", "B", "Second input", "Bool").withDefault("false"),
	);
	def.addPin(PinDefinition.output("exec_out", "Done", "Complete", "Exec"));
	def.addPin(PinDefinition.output("result", "Result", "A AND B", "Bool"));

	return serializeDefinition(def);
}

export function run_and_gate(ptr: i32, len: i32): i64 {
	const input = parseInput(ptr, len);
	const a = getBoolInput(input, "a", false);
	const b = getBoolInput(input, "b", false);

	const result = ExecutionResult.success();
	result.setOutput("result", (a && b).toString());
	result.activateExec("exec_out");

	return serializeResult(result);
}

// ============================================================================
// OR Gate Node
// ============================================================================

export function get_or_gate_definition(): i64 {
	const def = new NodeDefinition();
	def.name = "or_gate_as";
	def.friendly_name = "OR Gate (AS)";
	def.description = "Returns true if either input is true";
	def.category = "Control/Logic";

	def.addPin(PinDefinition.input("exec", "Execute", "Trigger", "Exec"));
	def.addPin(
		PinDefinition.input("a", "A", "First input", "Bool").withDefault("false"),
	);
	def.addPin(
		PinDefinition.input("b", "B", "Second input", "Bool").withDefault("false"),
	);
	def.addPin(PinDefinition.output("exec_out", "Done", "Complete", "Exec"));
	def.addPin(PinDefinition.output("result", "Result", "A OR B", "Bool"));

	return serializeDefinition(def);
}

export function run_or_gate(ptr: i32, len: i32): i64 {
	const input = parseInput(ptr, len);
	const a = getBoolInput(input, "a", false);
	const b = getBoolInput(input, "b", false);

	const result = ExecutionResult.success();
	result.setOutput("result", (a || b).toString());
	result.activateExec("exec_out");

	return serializeResult(result);
}

// ============================================================================
// NOT Gate Node
// ============================================================================

export function get_not_gate_definition(): i64 {
	const def = new NodeDefinition();
	def.name = "not_gate_as";
	def.friendly_name = "NOT Gate (AS)";
	def.description = "Inverts a boolean value";
	def.category = "Control/Logic";

	def.addPin(PinDefinition.input("exec", "Execute", "Trigger", "Exec"));
	def.addPin(
		PinDefinition.input("value", "Value", "Input value", "Bool").withDefault(
			"false",
		),
	);
	def.addPin(PinDefinition.output("exec_out", "Done", "Complete", "Exec"));
	def.addPin(PinDefinition.output("result", "Result", "NOT Value", "Bool"));

	return serializeDefinition(def);
}

export function run_not_gate(ptr: i32, len: i32): i64 {
	const input = parseInput(ptr, len);
	const value = getBoolInput(input, "value", false);

	const result = ExecutionResult.success();
	result.setOutput("result", (!value).toString());
	result.activateExec("exec_out");

	return serializeResult(result);
}

// ============================================================================
// Gate Node (conditional pass-through)
// ============================================================================

export function get_gate_definition(): i64 {
	const def = new NodeDefinition();
	def.name = "gate_as";
	def.friendly_name = "Gate (AS)";
	def.description = "Only passes execution if the gate is open";
	def.category = "Control/Flow";

	def.addPin(PinDefinition.input("exec", "Execute", "Trigger", "Exec"));
	def.addPin(
		PinDefinition.input("is_open", "Is Open", "Gate state", "Bool").withDefault(
			"true",
		),
	);
	def.addPin(
		PinDefinition.output("exec_out", "Out", "Passes if gate open", "Exec"),
	);

	return serializeDefinition(def);
}

export function run_gate(ptr: i32, len: i32): i64 {
	const input = parseInput(ptr, len);
	const isOpen = getBoolInput(input, "is_open", true);

	const result = ExecutionResult.success();
	if (isOpen) {
		result.activateExec("exec_out");
	}
	// If gate is closed, no exec output is activated

	return serializeResult(result);
}

// ============================================================================
// Helpers
// ============================================================================

function getBoolInput(
	input: ExecutionInput,
	name: string,
	defaultValue: bool,
): bool {
	if (input.inputs.has(name)) {
		return input.inputs.get(name) == "true";
	}
	return defaultValue;
}

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
