//! JSON Processing Nodes - Parse, query, and transform JSON data
//!
//! This example demonstrates JSON manipulation including
//! parsing, querying paths, merging objects, and transformations.

use flow_like_wasm_sdk::*;
use serde_json::Value;

package! {
    nodes: [
        {
            name: "json_parse",
            friendly_name: "Parse JSON",
            description: "Parses a JSON string into an object",
            category: "JSON/Parse",
            inputs: {
                exec: Exec,
                json_string: String = "{}",
            },
            outputs: {
                exec_out: Exec,
                result: Json,
                is_valid: Bool,
                error: String,
            },
        },
        {
            name: "json_stringify",
            friendly_name: "Stringify JSON",
            description: "Converts an object to a JSON string",
            category: "JSON/Parse",
            inputs: {
                exec: Exec,
                object: Json,
                pretty: Bool = false,
            },
            outputs: {
                exec_out: Exec,
                result: String,
            },
        },
        {
            name: "json_get",
            friendly_name: "Get Value",
            description: "Gets a value from JSON using a path (e.g., 'user.name')",
            category: "JSON/Query",
            inputs: {
                exec: Exec,
                object: Json,
                path: String = "",
            },
            outputs: {
                exec_out: Exec,
                value: Json,
                found: Bool,
            },
        },
        {
            name: "json_set",
            friendly_name: "Set Value",
            description: "Sets a value in JSON at the specified path",
            category: "JSON/Modify",
            inputs: {
                exec: Exec,
                object: Json,
                path: String = "",
                value: Json,
            },
            outputs: {
                exec_out: Exec,
                result: Json,
            },
        },
        {
            name: "json_merge",
            friendly_name: "Merge Objects",
            description: "Merges two JSON objects (b overwrites a)",
            category: "JSON/Modify",
            inputs: {
                exec: Exec,
                a: Json,
                b: Json,
            },
            outputs: {
                exec_out: Exec,
                result: Json,
            },
        },
        {
            name: "json_keys",
            friendly_name: "Get Keys",
            description: "Gets all keys from a JSON object",
            category: "JSON/Query",
            inputs: {
                exec: Exec,
                object: Json,
            },
            outputs: {
                exec_out: Exec,
                keys: Json,
                count: I64,
            },
        }
    ]
}

run_package!(run_node);

fn run_node(node_name: &str, mut ctx: Context) -> ExecutionResult {
    match node_name {
        "json_parse" => {
            let json_string = ctx.get_string("json_string").unwrap_or_default();
            match serde_json::from_str::<Value>(&json_string) {
                Ok(value) => {
                    ctx.set_output_json("result", &value);
                    ctx.set_output("is_valid", true);
                    ctx.set_output("error", "");
                }
                Err(e) => {
                    ctx.set_output_json("result", &json!(null));
                    ctx.set_output("is_valid", false);
                    ctx.set_output("error", e.to_string());
                }
            }
            ctx.success()
        }
        "json_stringify" => {
            let object: Value = ctx.get_input_as("object").unwrap_or(json!(null));
            let pretty = ctx.get_bool("pretty").unwrap_or(false);
            let result = if pretty {
                serde_json::to_string_pretty(&object).unwrap_or_default()
            } else {
                serde_json::to_string(&object).unwrap_or_default()
            };
            ctx.set_output("result", result);
            ctx.success()
        }
        "json_get" => {
            let object: Value = ctx.get_input_as("object").unwrap_or(json!(null));
            let path = ctx.get_string("path").unwrap_or_default();

            let value = get_json_path(&object, &path);
            ctx.set_output("found", !value.is_null());
            ctx.set_output_json("value", &value);
            ctx.success()
        }
        "json_set" => {
            let mut object: Value = ctx.get_input_as("object").unwrap_or(json!({}));
            let path = ctx.get_string("path").unwrap_or_default();
            let value: Value = ctx.get_input_as("value").unwrap_or(json!(null));

            set_json_path(&mut object, &path, value);
            ctx.set_output_json("result", &object);
            ctx.success()
        }
        "json_merge" => {
            let a: Value = ctx.get_input_as("a").unwrap_or(json!({}));
            let b: Value = ctx.get_input_as("b").unwrap_or(json!({}));

            let result = merge_json(a, b);
            ctx.set_output_json("result", &result);
            ctx.success()
        }
        "json_keys" => {
            let object: Value = ctx.get_input_as("object").unwrap_or(json!({}));
            let keys: Vec<String> = match object.as_object() {
                Some(map) => map.keys().cloned().collect(),
                None => vec![],
            };
            let count = keys.len() as i64;
            ctx.set_output_json("keys", &json!(keys));
            ctx.set_output("count", count);
            ctx.success()
        }
        _ => ctx.fail(format!("Unknown node: {}", node_name)),
    }
}

// ============================================================================
// Helper Functions
// ============================================================================

fn get_json_path(value: &Value, path: &str) -> Value {
    if path.is_empty() {
        return value.clone();
    }

    let parts: Vec<&str> = path.split('.').collect();
    let mut current = value;

    for part in parts {
        match current {
            Value::Object(map) => {
                current = map.get(part).unwrap_or(&Value::Null);
            }
            Value::Array(arr) => {
                if let Ok(index) = part.parse::<usize>() {
                    current = arr.get(index).unwrap_or(&Value::Null);
                } else {
                    return Value::Null;
                }
            }
            _ => return Value::Null,
        }
    }

    current.clone()
}

fn set_json_path(root: &mut Value, path: &str, value: Value) {
    if path.is_empty() {
        *root = value;
        return;
    }

    let parts: Vec<&str> = path.split('.').collect();
    let mut current = root;

    for (i, part) in parts.iter().enumerate() {
        if i == parts.len() - 1 {
            if let Value::Object(map) = current {
                map.insert(part.to_string(), value);
            }
            return;
        }

        if let Value::Object(map) = current {
            if !map.contains_key(*part) {
                map.insert(part.to_string(), json!({}));
            }
            current = map.get_mut(*part).unwrap();
        }
    }
}

fn merge_json(a: Value, b: Value) -> Value {
    match (a, b) {
        (Value::Object(mut map_a), Value::Object(map_b)) => {
            for (k, v) in map_b {
                let existing = map_a.remove(&k).unwrap_or(Value::Null);
                map_a.insert(k, merge_json(existing, v));
            }
            Value::Object(map_a)
        }
        (_, b) => b,
    }
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_get_json_path() {
        let obj = json!({"user": {"name": "Alice", "age": 30}});
        assert_eq!(get_json_path(&obj, "user.name"), json!("Alice"));
        assert_eq!(get_json_path(&obj, "user.age"), json!(30));
        assert_eq!(get_json_path(&obj, "user.missing"), Value::Null);
    }

    #[test]
    fn test_merge_json() {
        let a = json!({"x": 1, "y": 2});
        let b = json!({"y": 3, "z": 4});
        let result = merge_json(a, b);
        assert_eq!(result, json!({"x": 1, "y": 3, "z": 4}));
    }

    #[test]
    fn test_set_json_path() {
        let mut obj = json!({"user": {}});
        set_json_path(&mut obj, "user.name", json!("Bob"));
        assert_eq!(obj, json!({"user": {"name": "Bob"}}));
    }
}
