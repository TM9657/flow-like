use serde_json::Value;
use json5;

/// A simple heuristic to fix unbalanced braces/brackets in a string
fn fix_unbalanced(s: &str) -> String {
    let open_braces = s.chars().filter(|&c| c == '{').count();
    let close_braces = s.chars().filter(|&c| c == '}').count();
    let open_brackets = s.chars().filter(|&c| c == '[').count();
    let close_brackets = s.chars().filter(|&c| c == ']').count();

    let mut fixed = s.to_owned();
    for _ in 0..(open_braces.saturating_sub(close_braces)) {
        fixed.push('}');
    }
    for _ in 0..(open_brackets.saturating_sub(close_brackets)) {
        fixed.push(']');
    }
    fixed
}

/// Attempts to extract a JSON fragment from the input string using a stack-based approach.
/// Returns the candidate substring along with any missing closing characters (as a Vec)
/// that were expected but not found.
fn extract_json_fragment(input: &str) -> Option<(String, Vec<char>)> {
    // Locate the first JSON opening character.
    let start = input.find(|c| c == '{' || c == '[')?;
    let candidate = &input[start..];

    let mut stack = Vec::new();
    let mut in_string = false;
    let mut escape = false;
    let mut string_delim: Option<char> = None;
    let mut last_index = candidate.len();

    // Walk through the candidate substring char by char.
    for (i, c) in candidate.char_indices() {
        if in_string {
            if escape {
                escape = false;
                continue;
            }
            if c == '\\' {
                escape = true;
                continue;
            }
            if Some(c) == string_delim {
                in_string = false;
                string_delim = None;
            }
        } else {
            if c == '"' || c == '\'' {
                in_string = true;
                string_delim = Some(c);
                continue;
            }
            if c == '{' {
                stack.push('}');
            } else if c == '[' {
                stack.push(']');
            } else if c == '}' || c == ']' {
                if let Some(expected) = stack.pop() {
                    if c != expected {
                        // Mismatch detected. We break early.
                        last_index = i + c.len_utf8();
                        break;
                    }
                } else {
                    // Unmatched closing found; break.
                    last_index = i + c.len_utf8();
                    break;
                }
            }
            // If the stack is empty, we assume the JSON fragment is complete.
            if stack.is_empty() {
                last_index = i + c.len_utf8();
                break;
            }
        }
    }
    let fragment = candidate[..last_index].to_string();
    Some((fragment, stack))
}

/// Tries multiple strategies to parse a malformed JSON string into a serde_json::Value.
fn parse_malformed_json(input: &str) -> Result<Value, Box<dyn std::error::Error>> {
    // 1. Direct parse attempt.
    if let Ok(val) = serde_json::from_str(input) {
        return Ok(val);
    }

    // 2. Try to extract a candidate JSON fragment.
    if let Some((mut candidate, mut missing_stack)) = extract_json_fragment(input) {
        // Append missing closing characters (if any) in the correct order.
        if !missing_stack.is_empty() {
            let missing: String = missing_stack.iter().rev().collect();
            candidate.push_str(&missing);
        }

        // Trim any trailing garbage beyond the last valid JSON delimiter.
        let last_valid_index = candidate.rfind(|c| c == '}' || c == ']')
            .map(|idx| idx + 1)
            .unwrap_or(candidate.len());
        let candidate = &candidate[..last_valid_index];

        // 3. Try strict parsing first.
        if let Ok(val) = serde_json::from_str(candidate) {
            return Ok(val);
        }
        // 4. Then try the lenient json5 parser.
        if let Ok(val) = json5::from_str(candidate) {
            return Ok(val);
        }

        // 5. As an extra heuristic, try to fix any unbalanced delimiters in the candidate.
        let fixed_candidate = fix_unbalanced(candidate);
        if let Ok(val) = serde_json::from_str(&fixed_candidate) {
            return Ok(val);
        }
        if let Ok(val) = json5::from_str(&fixed_candidate) {
            return Ok(val);
        }
    }

    // 6. Fallback: remove any non-JSON prefix and apply a basic fix.
    if let Some(start) = input.find(|c| c == '{' || c == '[') {
        let trimmed = &input[start..];
        let fixed = fix_unbalanced(trimmed);
        if let Ok(val) = json5::from_str(&fixed) {
            return Ok(val);
        }
    }

    Err("Unable to parse malformed JSON".into())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn json_test_prefix() {
        let input = "Some extraneous text before the JSON {\"name\": \"Alice\", \"age\": 30";
        let result = parse_malformed_json(input).unwrap();
        let expected = serde_json::json!({"name": "Alice", "age": 30});
        assert_eq!(result, expected);
    }

    #[test]
    fn json_test_array() {
        let input = "prefix [1, 2, 3, 4, 5] some trailing text";
        let result = parse_malformed_json(input).unwrap();
        let expected = serde_json::json!([1, 2, 3, 4, 5]);
        assert_eq!(result, expected);
    }

    #[test]
    fn json_test_invalid() {
        let input = "random text {invalid json here";
        let result = parse_malformed_json(input);
        assert!(result.is_err());
    }

    #[test]
    fn json_test_nested() {
        let input = "noise {\"nested\": {\"key\": \"value\", \"list\": [1, 2, 3]";
        let result = parse_malformed_json(input).unwrap();
        let expected = serde_json::json!({"nested": {"key": "value", "list": [1, 2, 3]}});
        assert_eq!(result, expected);
    }

    #[test]
    fn json_test_unbalanced_quotes() {
        let input = r#"{"key": "value with no closing quote}"#;
        let result = parse_malformed_json(input).unwrap();
        let expected = serde_json::json!({"key": "value with no closing quote"});
        assert_eq!(result, expected);
    }

    #[test]
    fn json_test_missing_commas() {
        let input = r#"{"a": 1 "b": 2 "c": 3}"#;
        let result = parse_malformed_json(input).unwrap();
        let expected = serde_json::json!({"a": 1, "b": 2, "c": 3});
        assert_eq!(result, expected);
    }

    #[test]
    fn json_test_trailing_commas() {
        let input = r#"{"items": [1, 2, 3,], "more": {"a": 1, "b": 2,},}"#;
        let result = parse_malformed_json(input).unwrap();
        let expected = serde_json::json!({"items": [1, 2, 3], "more": {"a": 1, "b": 2}});
        assert_eq!(result, expected);
    }

    #[test]
    fn json_test_deeply_nested_unbalanced() {
        let input = r#"{"level1": {"level2": {"level3": [1, 2, {"deep": "value""#;
        let result = parse_malformed_json(input).unwrap();
        let expected = serde_json::json!({"level1": {"level2": {"level3": [1, 2, {"deep": "value"}]}}});
        assert_eq!(result, expected);
    }

    #[test]
    fn json_test_mixed_bracket_types() {
        let input = r#"{"array": [1, 2, 3}"#; // Missing closing bracket for array
        let result = parse_malformed_json(input).unwrap();
        let expected = serde_json::json!({"array": [1, 2, 3]});
        assert_eq!(result, expected);
    }

    #[test]
    fn json_test_json5_features() {
        let input = r#"{
            // This is a comment
            key: 'single quoted string',
            trailing_comma: true,
        }"#;
        let result = parse_malformed_json(input).unwrap();
        let expected = serde_json::json!({"key": "single quoted string", "trailing_comma": true});
        assert_eq!(result, expected);
    }

    #[test]
    fn json_test_empty_structures() {
        let input = "garbage {} more garbage";
        let result = parse_malformed_json(input).unwrap();
        let expected = serde_json::json!({});
        assert_eq!(result, expected);

        let input = "garbage [] more garbage";
        let result = parse_malformed_json(input).unwrap();
        let expected = serde_json::json!([]);
        assert_eq!(result, expected);
    }

    #[test]
    fn json_test_special_characters() {
        let input = r#"{"unicode": "こんにちは世界", "escaped": "line1\nline2"}"#;
        let result = parse_malformed_json(input).unwrap();
        let expected = serde_json::json!({"unicode": "こんにちは世界", "escaped": "line1\nline2"});
        assert_eq!(result, expected);
    }

    #[test]
    fn json_test_primitive_values() {
        let input = "42";
        let result = parse_malformed_json(input).unwrap();
        let expected = serde_json::json!(42);
        assert_eq!(result, expected);

        let input = "true";
        let result = parse_malformed_json(input).unwrap();
        let expected = serde_json::json!(true);
        assert_eq!(result, expected);
    }
}