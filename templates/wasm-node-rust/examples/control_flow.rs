//! Control Flow Nodes - Logic and branching utilities
//!
//! This example demonstrates control flow nodes including
//! conditionals, switches, loops, and comparison operations.

use flow_like_wasm_sdk::*;

node! {
    name: "if_branch",
    friendly_name: "If Branch",
    description: "Branches execution based on a condition",
    category: "Control/Flow",

    inputs: {
        exec: Exec,
        condition: Bool = false,
    },

    outputs: {
        true_branch: Exec,
        false_branch: Exec,
    },
}

run_node!(run_if_branch);

fn run_if_branch(mut ctx: Context) -> ExecutionResult {
    let condition = ctx.get_bool("condition").unwrap_or(false);

    if condition {
        ctx.activate_exec("true_branch");
    } else {
        ctx.activate_exec("false_branch");
    }

    ctx.finish()
}

// ============================================================================
// Additional Control Flow Nodes (as separate files in real package)
// ============================================================================

/// Compare two values
fn compare_node_definition() -> NodeDefinition {
    let mut def = NodeDefinition::new(
        "compare",
        "Compare",
        "Compares two values using the specified operator",
        "Control/Logic",
    );

    def.add_pin(PinDefinition::input("exec", "Execute", "Trigger", "Exec"));
    def.add_pin(PinDefinition::input("a", "A", "First value", "F64").with_default(json!(0.0)));
    def.add_pin(PinDefinition::input("b", "B", "Second value", "F64").with_default(json!(0.0)));
    def.add_pin(
        PinDefinition::input("operator", "Operator", "Comparison operator", "String")
            .with_default(json!("=="))
            .with_valid_values(vec![
                "==".to_string(),
                "!=".to_string(),
                "<".to_string(),
                "<=".to_string(),
                ">".to_string(),
                ">=".to_string(),
            ])
    );

    def.add_pin(PinDefinition::output("exec_out", "Done", "Complete", "Exec"));
    def.add_pin(PinDefinition::output("result", "Result", "Comparison result", "Bool"));

    def
}

/// Gate - only passes execution if condition is true
fn gate_example(mut ctx: Context) -> ExecutionResult {
    let is_open = ctx.get_bool("is_open").unwrap_or(false);

    if is_open {
        ctx.activate_exec("exec_out");
    }
    // If gate is closed, no exec is activated

    ctx.finish()
}

/// Sequence - activates multiple outputs in order
fn sequence_example(mut ctx: Context) -> ExecutionResult {
    // Activate outputs in sequence
    // The runtime will execute them in order
    ctx.activate_exec("then_0");
    ctx.activate_exec("then_1");
    ctx.activate_exec("then_2");

    ctx.finish()
}

/// For Each with index
fn for_each_example(mut ctx: Context) -> ExecutionResult {
    // This demonstrates streaming results for iteration
    let items: Vec<String> = ctx.get_input_as("items").unwrap_or_default();

    for (index, item) in items.iter().enumerate() {
        ctx.stream_json(&json!({
            "index": index,
            "item": item,
            "is_first": index == 0,
            "is_last": index == items.len() - 1,
        }));
    }

    ctx.success()
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    #[test]
    fn test_condition_branching() {
        let condition = true;
        let branch = if condition { "true" } else { "false" };
        assert_eq!(branch, "true");
    }

    #[test]
    fn test_comparisons() {
        assert!(5.0 > 3.0);
        assert!(3.0 < 5.0);
        assert!(5.0 == 5.0);
        assert!(5.0 != 3.0);
        assert!(5.0 >= 5.0);
        assert!(3.0 <= 5.0);
    }
}
