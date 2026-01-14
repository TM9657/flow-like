use flow_like_types::json::Map;
use flow_like_types::Value;

/// Finds an element in the elements map by ID.
///
/// Supports:
/// - Exact match: "surfaceId/componentId"
/// - Component ID suffix match: "componentId" (matches any "*/componentId")
pub fn find_element<'a>(
    elements: &'a Map<String, Value>,
    element_id: &str,
) -> Option<(&'a String, &'a Value)> {
    // First try exact match
    if let Some(val) = elements.get(element_id) {
        return Some((&elements.keys().find(|k| *k == element_id).unwrap(), val));
    }

    // If no exact match and element_id doesn't contain "/", try suffix matching
    if !element_id.contains('/') {
        let suffix = format!("/{}", element_id);
        for (key, val) in elements.iter() {
            if key.ends_with(&suffix) {
                return Some((key, val));
            }
        }
    }

    None
}

/// Extracts element ID from either a string or an element object with __element_id field.
/// Used by setter nodes to accept both raw IDs and element refs from Get Element.
pub fn extract_element_id(value: &Value) -> Option<String> {
    match value {
        Value::String(s) if !s.is_empty() => Some(s.clone()),
        Value::Object(obj) => {
            // Check for __element_id field (set by get_element node)
            obj.get("__element_id")
                .and_then(|v| v.as_str())
                .filter(|s| !s.is_empty())
                .map(|s| s.to_string())
                // Fallback to id field
                .or_else(|| {
                    obj.get("id")
                        .and_then(|v| v.as_str())
                        .filter(|s| !s.is_empty())
                        .map(|s| s.to_string())
                })
        }
        Value::Null => None,
        _ => None,
    }
}

/// Get a property from a component's data
pub fn get_component_property<'a>(component: &'a Value, property: &str) -> Option<&'a Value> {
    component
        .get("component")
        .and_then(|c| c.get(property))
}

/// Get text content from a component (tries multiple common properties)
pub fn get_text_content(component: &Value) -> Option<&str> {
    let comp = component.get("component")?;

    // Try common text properties in order
    comp.get("content")
        .or_else(|| comp.get("text"))
        .or_else(|| comp.get("label"))
        .and_then(|v| v.as_str())
}

/// Get value from a component
pub fn get_value_content(component: &Value) -> Option<&Value> {
    component
        .get("component")
        .and_then(|c| c.get("value").or_else(|| c.get("defaultValue")))
}

/// Extracts element ID from a pin Value that can be either:
/// - A JSON string (from element-select dropdown)
/// - An element object with __element_id (from Get Element node)
///
/// This allows getter nodes to work both when directly selected and when connected to Get Element.
pub fn extract_element_id_from_pin(value: Value) -> Option<String> {
    match value {
        Value::String(s) if !s.is_empty() => Some(s),
        Value::Object(ref obj) => {
            obj.get("__element_id")
                .and_then(|v| v.as_str())
                .filter(|s| !s.is_empty())
                .map(|s| s.to_string())
                .or_else(|| {
                    obj.get("id")
                        .and_then(|v| v.as_str())
                        .filter(|s| !s.is_empty())
                        .map(|s| s.to_string())
                })
        }
        _ => None,
    }
}
