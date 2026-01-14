//! String Nodes - Text manipulation utilities
//!
//! This example demonstrates string processing nodes including
//! case conversion, splitting, joining, and text analysis.

use flow_like_wasm_sdk::*;

package! {
    nodes: [
        {
            name: "string_uppercase",
            friendly_name: "To Uppercase",
            description: "Converts text to uppercase",
            category: "String/Transform",
            inputs: {
                exec: Exec,
                text: String = "",
            },
            outputs: {
                exec_out: Exec,
                result: String,
            },
        },
        {
            name: "string_lowercase",
            friendly_name: "To Lowercase",
            description: "Converts text to lowercase",
            category: "String/Transform",
            inputs: {
                exec: Exec,
                text: String = "",
            },
            outputs: {
                exec_out: Exec,
                result: String,
            },
        },
        {
            name: "string_trim",
            friendly_name: "Trim",
            description: "Removes leading and trailing whitespace",
            category: "String/Transform",
            inputs: {
                exec: Exec,
                text: String = "",
            },
            outputs: {
                exec_out: Exec,
                result: String,
            },
        },
        {
            name: "string_length",
            friendly_name: "String Length",
            description: "Returns the length of a string",
            category: "String/Analysis",
            inputs: {
                exec: Exec,
                text: String = "",
            },
            outputs: {
                exec_out: Exec,
                length: I64,
                is_empty: Bool,
            },
        },
        {
            name: "string_contains",
            friendly_name: "Contains",
            description: "Checks if text contains a substring",
            category: "String/Analysis",
            inputs: {
                exec: Exec,
                text: String = "",
                search: String = "",
            },
            outputs: {
                exec_out: Exec,
                result: Bool,
            },
        },
        {
            name: "string_replace",
            friendly_name: "Replace",
            description: "Replaces occurrences of a pattern",
            category: "String/Transform",
            inputs: {
                exec: Exec,
                text: String = "",
                find: String = "",
                replace_with: String = "",
            },
            outputs: {
                exec_out: Exec,
                result: String,
                count: I64,
            },
        },
        {
            name: "string_concat",
            friendly_name: "Concatenate",
            description: "Joins multiple strings together",
            category: "String/Transform",
            inputs: {
                exec: Exec,
                a: String = "",
                b: String = "",
                separator: String = "",
            },
            outputs: {
                exec_out: Exec,
                result: String,
            },
        },
        {
            name: "string_reverse",
            friendly_name: "Reverse",
            description: "Reverses the characters in a string",
            category: "String/Transform",
            inputs: {
                exec: Exec,
                text: String = "",
            },
            outputs: {
                exec_out: Exec,
                result: String,
            },
        }
    ]
}

run_package!(run_node);

fn run_node(node_name: &str, mut ctx: Context) -> ExecutionResult {
    match node_name {
        "string_uppercase" => {
            let text = ctx.get_string("text").unwrap_or_default();
            ctx.set_output("result", text.to_uppercase());
            ctx.success()
        }
        "string_lowercase" => {
            let text = ctx.get_string("text").unwrap_or_default();
            ctx.set_output("result", text.to_lowercase());
            ctx.success()
        }
        "string_trim" => {
            let text = ctx.get_string("text").unwrap_or_default();
            ctx.set_output("result", text.trim().to_string());
            ctx.success()
        }
        "string_length" => {
            let text = ctx.get_string("text").unwrap_or_default();
            ctx.set_output("length", text.len() as i64);
            ctx.set_output("is_empty", text.is_empty());
            ctx.success()
        }
        "string_contains" => {
            let text = ctx.get_string("text").unwrap_or_default();
            let search = ctx.get_string("search").unwrap_or_default();
            ctx.set_output("result", text.contains(&search));
            ctx.success()
        }
        "string_replace" => {
            let text = ctx.get_string("text").unwrap_or_default();
            let find = ctx.get_string("find").unwrap_or_default();
            let replace_with = ctx.get_string("replace_with").unwrap_or_default();
            let count = text.matches(&find).count() as i64;
            let result = text.replace(&find, &replace_with);
            ctx.set_output("result", result);
            ctx.set_output("count", count);
            ctx.success()
        }
        "string_concat" => {
            let a = ctx.get_string("a").unwrap_or_default();
            let b = ctx.get_string("b").unwrap_or_default();
            let separator = ctx.get_string("separator").unwrap_or_default();
            let result = if separator.is_empty() {
                format!("{}{}", a, b)
            } else {
                format!("{}{}{}", a, separator, b)
            };
            ctx.set_output("result", result);
            ctx.success()
        }
        "string_reverse" => {
            let text = ctx.get_string("text").unwrap_or_default();
            ctx.set_output("result", text.chars().rev().collect::<String>());
            ctx.success()
        }
        _ => ctx.fail(format!("Unknown node: {}", node_name)),
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn test_uppercase() {
        assert_eq!("hello".to_uppercase(), "HELLO");
    }

    #[test]
    fn test_reverse() {
        let s = "hello";
        let reversed: String = s.chars().rev().collect();
        assert_eq!(reversed, "olleh");
    }

    #[test]
    fn test_contains() {
        assert!("hello world".contains("world"));
        assert!(!"hello world".contains("foo"));
    }
}
