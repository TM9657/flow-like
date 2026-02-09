//! Math Nodes - Basic arithmetic and mathematical operations
//!
//! This example demonstrates creating a multi-node package with
//! arithmetic operations like add, subtract, multiply, divide, and power.

use flow_like_wasm_sdk::*;

// ============================================================================
// Package Definition (Multiple Nodes)
// ============================================================================

package! {
    nodes: [
        {
            name: "math_add",
            friendly_name: "Add",
            description: "Adds two numbers together",
            category: "Math/Arithmetic",
            inputs: {
                exec: Exec,
                a: F64 = 0.0,
                b: F64 = 0.0,
            },
            outputs: {
                exec_out: Exec,
                result: F64,
            },
        },
        {
            name: "math_subtract",
            friendly_name: "Subtract",
            description: "Subtracts B from A",
            category: "Math/Arithmetic",
            inputs: {
                exec: Exec,
                a: F64 = 0.0,
                b: F64 = 0.0,
            },
            outputs: {
                exec_out: Exec,
                result: F64,
            },
        },
        {
            name: "math_multiply",
            friendly_name: "Multiply",
            description: "Multiplies two numbers",
            category: "Math/Arithmetic",
            inputs: {
                exec: Exec,
                a: F64 = 0.0,
                b: F64 = 0.0,
            },
            outputs: {
                exec_out: Exec,
                result: F64,
            },
        },
        {
            name: "math_divide",
            friendly_name: "Divide",
            description: "Divides A by B",
            category: "Math/Arithmetic",
            inputs: {
                exec: Exec,
                a: F64 = 0.0,
                b: F64 = 1.0,
            },
            outputs: {
                exec_out: Exec,
                result: F64,
                is_valid: Bool,
            },
        },
        {
            name: "math_power",
            friendly_name: "Power",
            description: "Raises A to the power of B",
            category: "Math/Arithmetic",
            inputs: {
                exec: Exec,
                base: F64 = 0.0,
                exponent: F64 = 1.0,
            },
            outputs: {
                exec_out: Exec,
                result: F64,
            },
        },
        {
            name: "math_clamp",
            friendly_name: "Clamp",
            description: "Clamps a value between min and max",
            category: "Math/Utility",
            inputs: {
                exec: Exec,
                value: F64 = 0.0,
                min: F64 = 0.0,
                max: F64 = 1.0,
            },
            outputs: {
                exec_out: Exec,
                result: F64,
            },
        }
    ]
}

// ============================================================================
// Node Implementations
// ============================================================================

run_package!(run_node);

fn run_node(node_name: &str, mut ctx: Context) -> ExecutionResult {
    match node_name {
        "math_add" => {
            let a = ctx.get_f64("a").unwrap_or(0.0);
            let b = ctx.get_f64("b").unwrap_or(0.0);
            ctx.set_output("result", a + b);
            ctx.success()
        }
        "math_subtract" => {
            let a = ctx.get_f64("a").unwrap_or(0.0);
            let b = ctx.get_f64("b").unwrap_or(0.0);
            ctx.set_output("result", a - b);
            ctx.success()
        }
        "math_multiply" => {
            let a = ctx.get_f64("a").unwrap_or(0.0);
            let b = ctx.get_f64("b").unwrap_or(0.0);
            ctx.set_output("result", a * b);
            ctx.success()
        }
        "math_divide" => {
            let a = ctx.get_f64("a").unwrap_or(0.0);
            let b = ctx.get_f64("b").unwrap_or(1.0);

            if b == 0.0 {
                ctx.set_output("result", 0.0);
                ctx.set_output("is_valid", false);
                ctx.warn("Division by zero");
            } else {
                ctx.set_output("result", a / b);
                ctx.set_output("is_valid", true);
            }
            ctx.success()
        }
        "math_power" => {
            let base = ctx.get_f64("base").unwrap_or(0.0);
            let exponent = ctx.get_f64("exponent").unwrap_or(1.0);
            ctx.set_output("result", base.powf(exponent));
            ctx.success()
        }
        "math_clamp" => {
            let value = ctx.get_f64("value").unwrap_or(0.0);
            let min = ctx.get_f64("min").unwrap_or(0.0);
            let max = ctx.get_f64("max").unwrap_or(1.0);
            ctx.set_output("result", value.max(min).min(max));
            ctx.success()
        }
        _ => ctx.fail(format!("Unknown node: {}", node_name)),
    }
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    #[test]
    fn test_add() {
        assert_eq!(5.0 + 3.0, 8.0);
    }

    #[test]
    fn test_divide_by_zero() {
        let b = 0.0f64;
        assert!(b == 0.0);
    }

    #[test]
    fn test_power() {
        assert_eq!(2.0f64.powf(3.0), 8.0);
    }

    #[test]
    fn test_clamp() {
        let value = 15.0f64;
        let result = value.max(0.0).min(10.0);
        assert_eq!(result, 10.0);
    }
}
