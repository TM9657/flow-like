/**
 * Example Nodes Entry Point
 *
 * This file imports and re-exports all example nodes so they can be built
 * into a single WASM module. Use this as a reference for creating multi-node packages.
 *
 * Build with: npx asc assembly/examples.ts --target release -o build/examples.wasm
 */

// Re-export SDK utilities that are always needed
export {
	alloc,
	dealloc,
	get_abi_version,
} from "./sdk";

// ============================================================================
// Math Nodes
// ============================================================================

export {
	get_add_definition,
	run_add,
	get_subtract_definition,
	run_subtract,
	get_multiply_definition,
	run_multiply,
	get_divide_definition,
	run_divide,
	get_clamp_definition,
	run_clamp,
} from "../examples/math_nodes";

// ============================================================================
// String Nodes
// ============================================================================

export {
	get_uppercase_definition,
	run_uppercase,
	get_lowercase_definition,
	run_lowercase,
	get_trim_definition,
	run_trim,
	get_length_definition,
	run_length,
	get_contains_definition,
	run_contains,
	get_replace_definition,
	run_replace,
	get_concat_definition,
	run_concat,
} from "../examples/string_nodes";

// ============================================================================
// Control Flow Nodes
// ============================================================================

export {
	get_if_branch_definition,
	run_if_branch,
	get_compare_definition,
	run_compare,
	get_and_gate_definition,
	run_and_gate,
	get_or_gate_definition,
	run_or_gate,
	get_not_gate_definition,
	run_not_gate,
	get_gate_definition,
	run_gate,
} from "../examples/control_flow";
