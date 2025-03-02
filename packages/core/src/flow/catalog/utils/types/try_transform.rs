use std::sync::Arc;

use crate::{
    flow::{
        board::Board,
        execution::context::ExecutionContext,
        node::{Node, NodeLogic},
        variable::VariableType,
    },
    state::FlowLikeState,
};
use async_trait::async_trait;
use serde_json::{json, Value};

#[derive(Default)]

pub struct TryTransformNode {}

impl TryTransformNode {
    pub fn new() -> Self {
        TryTransformNode {}
    }
}

#[async_trait]
impl NodeLogic for TryTransformNode {
    async fn get_node(&self, _app_state: &FlowLikeState) -> Node {
        let mut node = Node::new(
            "utils_types_try_transform",
            "Try Transform",
            "Tries to transform cast types.",
            "Utils/Types",
        );

        node.add_input_pin(
            "type_in",
            "Type In",
            "Type to transform",
            VariableType::Generic,
        );

        node.add_output_pin(
            "type_out",
            "Type Out",
            "If the type was successfully transformed, transformed type",
            VariableType::Generic,
        );

        node.add_output_pin(
            "success",
            "Success",
            "Determines of tje transformation was successful",
            VariableType::Boolean,
        );

        return node;
    }

    async fn run(&mut self, context: &mut ExecutionContext) -> anyhow::Result<()> {
        let input_value: Value = context.evaluate_pin("type_in").await?;
        let output_value = context.get_pin_by_name("type_out").await?;
        let out_type = output_value.lock().await.pin.lock().await.data_type.clone();
        let mut out_value: Value = Value::Null;

        let success = match out_type {
            VariableType::String => value_to_string(&input_value, &mut out_value),
            VariableType::Float => value_to_float(&input_value, &mut out_value),
            VariableType::Integer => value_to_int(&input_value, &mut out_value),
            VariableType::Boolean => value_to_boolean(&input_value, &mut out_value),
            VariableType::Struct => value_to_struct(&input_value, &mut out_value),
            VariableType::Byte => value_to_byte(&input_value, &mut out_value),
            VariableType::Date => value_to_date(&input_value, &mut out_value),
            VariableType::PathBuf => value_to_pathbuf(&input_value, &mut out_value),
            VariableType::Execution => false,
            VariableType::Generic => false,
        };

        context.set_pin_value("type_out", out_value).await?;
        context.set_pin_value("success", json!(success)).await?;

        Ok(())
    }

    async fn on_update(&self, node: &mut Node, board: Arc<Board>) {
        let match_type = node.match_type("type_out", board.clone(), None);

        if match_type.is_err() {
            eprintln!("Error: {:?}", match_type.err());
        }

        let match_type = node.match_type("type_in", board, None);

        if match_type.is_err() {
            eprintln!("Error: {:?}", match_type.err());
        }
    }
}

fn value_to_string(input: &Value, target: &mut Value) -> bool {
    if input.is_string() {
        *target = input.clone();
        return true;
    }

    if let Ok(val) = serde_json::to_string_pretty(input) {
        *target = Value::String(val);
        return true;
    }

    false
}

fn value_to_float(input: &Value, target: &mut Value) -> bool {
    if input.is_number() {
        if let Some(val) = input.as_f64() {
            *target = Value::Number(serde_json::Number::from_f64(val).unwrap());
            return true;
        }

        if let Some(val) = input.as_i64() {
            *target = Value::Number(serde_json::Number::from_f64(val as f64).unwrap());
            return true;
        }
    }

    if input.is_string() {
        if let Some(s) = input.as_str() {
            if let Ok(val) = s.parse::<f64>() {
                if let Some(num) = serde_json::Number::from_f64(val) {
                    *target = Value::Number(num);
                    return true;
                }
            }
        }
    }

    if input.is_boolean() {
        let val = if input.as_bool().unwrap() { 1.0 } else { 0.0 };
        if let Some(num) = serde_json::Number::from_f64(val) {
            *target = Value::Number(num);
            return true;
        }
    }

    false
}

fn value_to_int(input: &Value, target: &mut Value) -> bool {
    if input.is_number() {
        if let Some(val) = input.as_i64() {
            *target = Value::Number(val.into());
            return true;
        }

        if let Some(val) = input.as_f64() {
            let int_val = val as i64;
            *target = Value::Number(int_val.into());
            return true;
        }
    }

    if input.is_string() {
        if let Some(s) = input.as_str() {
            // Try parsing as i64 first
            if let Ok(val) = s.parse::<i64>() {
                *target = Value::Number(val.into());
                return true;
            }

            // Try parsing as f64 and then converting to i64
            if let Ok(val) = s.parse::<f64>() {
                *target = Value::Number((val as i64).into());
                return true;
            }
        }
    }

    // Handle boolean conversion (true = 1, false = 0)
    if input.is_boolean() {
        let val = if input.as_bool().unwrap() { 1 } else { 0 };
        *target = Value::Number(val.into());
        return true;
    }

    false
}

fn value_to_boolean(input: &Value, target: &mut Value) -> bool {
    if input.is_boolean() {
        *target = input.clone();
        return true;
    }

    if input.is_number() {
        if let Some(val) = input.as_f64() {
            *target = Value::Bool(val != 0.0);
            return true;
        }

        if let Some(val) = input.as_i64() {
            *target = Value::Bool(val != 0);
            return true;
        }
    }

    if input.is_string() {
        if let Some(s) = input.as_str() {
            let lower = s.to_lowercase();
            if ["true", "yes", "y", "1", "on"].contains(&lower.as_str()) {
                *target = Value::Bool(true);
                return true;
            }

            if ["false", "no", "n", "0", "off"].contains(&lower.as_str()) {
                *target = Value::Bool(false);
                return true;
            }

            if let Ok(val) = s.parse::<f64>() {
                *target = Value::Bool(val != 0.0);
                return true;
            }
        }
    }

    if input.is_null() {
        *target = Value::Bool(false);
        return true;
    }

    if input.is_array() {
        *target = Value::Bool(!input.as_array().unwrap().is_empty());
        return true;
    }

    if input.is_object() {
        *target = Value::Bool(!input.as_object().unwrap().is_empty());
        return true;
    }

    false
}

fn value_to_struct(input: &Value, target: &mut Value) -> bool {
    if input.is_object() {
        *target = input.clone();
        return true;
    }

    if input.is_string() {
        if let Some(s) = input.as_str() {
            // Parse string as JSON
            if let Ok(parsed) = serde_json::from_str::<Value>(s) {
                if parsed.is_object() {
                    *target = parsed;
                    return true;
                }
            }
        }
    }

    if input.is_array() {
        let array = input.as_array().unwrap();
        let mut obj = serde_json::Map::new();

        for (index, value) in array.iter().enumerate() {
            obj.insert(index.to_string(), value.clone());
        }

        *target = Value::Object(obj);
        return true;
    }

    if input.is_number() || input.is_boolean() || input.is_null() {
        let mut obj = serde_json::Map::new();
        obj.insert("value".to_string(), input.clone());
        *target = Value::Object(obj);
        return true;
    }

    false
}

fn value_to_byte(input: &Value, target: &mut Value) -> bool {
    if input.is_number() {
        if let Some(val) = input.as_i64() {
            if val >= 0 && val <= 255 {
                *target = Value::Number((val as u8).into());
                return true;
            }
        }

        if let Some(val) = input.as_f64() {
            let byte_val = val.round() as i64;
            if byte_val >= 0 && byte_val <= 255 {
                *target = Value::Number((byte_val as u8).into());
                return true;
            }
        }
    }

    if input.is_string() {
        if let Some(s) = input.as_str() {
            if let Ok(val) = s.parse::<u8>() {
                *target = Value::Number(val.into());
                return true;
            }

            if let Ok(val) = s.parse::<f64>() {
                let byte_val = val.round() as i64;
                if byte_val >= 0 && byte_val <= 255 {
                    *target = Value::Number((byte_val as u8).into());
                    return true;
                }
            }

            if s.chars().count() == 1 {
                let byte_val = s.chars().next().unwrap() as u32;
                if byte_val <= 255 {
                    *target = Value::Number((byte_val as u8).into());
                    return true;
                }
            }
        }
    }

    if input.is_boolean() {
        let val = if input.as_bool().unwrap() { 1_u8 } else { 0_u8 };
        *target = Value::Number(val.into());
        return true;
    }

    false
}

fn value_to_date(input: &Value, target: &mut Value) -> bool {
    if input.is_string() {
        if let Some(s) = input.as_str() {
            if let Ok(dt) = chrono::DateTime::parse_from_rfc3339(s) {
                let timestamp_millis = dt.timestamp_millis();
                let secs = timestamp_millis / 1000;
                let nanos = (timestamp_millis % 1000) * 1_000_000;

                let date_obj = json!({
                    "secs_since_epoch": secs,
                    "nanos_since_epoch": timestamp_millis * 1_000_000
                });

                *target = date_obj;
                return true;
            }

            for format in &["%Y-%m-%d", "%d/%m/%Y", "%m/%d/%Y", "%Y-%m-%d %H:%M:%S"] {
                if let Ok(dt) = chrono::NaiveDateTime::parse_from_str(s, format) {
                    let timestamp_millis = dt.and_utc().timestamp_millis();

                    let date_obj = json!({
                        "secs_since_epoch": timestamp_millis / 1000,
                        "nanos_since_epoch": timestamp_millis * 1_000_000
                    });

                    *target = date_obj;
                    return true;
                } else if let Ok(d) = chrono::NaiveDate::parse_from_str(s, format) {
                    let dt = d.and_time(chrono::NaiveTime::from_hms_opt(0, 0, 0).unwrap());
                    let timestamp_millis = dt.and_utc().timestamp_millis();

                    let date_obj = json!({
                        "secs_since_epoch": timestamp_millis / 1000,
                        "nanos_since_epoch": timestamp_millis * 1_000_000
                    });

                    *target = date_obj;
                    return true;
                }
            }
        }
    }

    if input.is_number() {
        let mut timestamp_millis: i64 = 0;

        if let Some(val) = input.as_i64() {
            if val > 946684800 * 1000 {
                timestamp_millis = val;
            } else {
                timestamp_millis = val * 1000;
            }
        } else if let Some(val) = input.as_f64() {
            if val > 946684800.0 * 1000.0 {
                timestamp_millis = val as i64;
            } else {
                timestamp_millis = (val * 1000.0) as i64;
            }
        }

        let date_obj = json!({
            "secs_since_epoch": timestamp_millis / 1000,
            "nanos_since_epoch": timestamp_millis * 1_000_000
        });

        *target = date_obj;
        return true;
    }

    if input.is_object() {
        let obj = input.as_object().unwrap();
        if obj.contains_key("secs_since_epoch") && obj.contains_key("nanos_since_epoch") {
            *target = input.clone();
            return true;
        }
    }

    false
}

fn value_to_pathbuf(input: &Value, target: &mut Value) -> bool {
    if input.is_string() {
        *target = input.clone();
        return true;
    }

    if input.is_object() {
        if let Some(obj) = input.as_object() {
            if let Some(path) = obj.get("path") {
                if path.is_string() {
                    *target = path.clone();
                    return true;
                }
            }

            if let (Some(dir), Some(file)) = (obj.get("directory"), obj.get("filename")) {
                if let (Some(dir_str), Some(file_str)) = (dir.as_str(), file.as_str()) {
                    let path = format!("{}{}{}", dir_str, std::path::MAIN_SEPARATOR, file_str);
                    *target = Value::String(path);
                    return true;
                }
            }
        }
    }

    false
}
