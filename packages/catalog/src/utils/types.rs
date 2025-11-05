pub mod from_bytes;
pub mod from_string;
pub mod to_bytes;
pub mod to_string;
pub mod try_transform;
use flow_like::flow::node::NodeLogic;
use flow_like_types::{Value, json::Map};
use std::{collections::BTreeMap, sync::Arc};

pub fn normalize_json_value(value: Value) -> Value {
    match value {
        Value::Object(map) => {
            let sorted: BTreeMap<String, Value> = map
                .into_iter()
                .map(|(k, v)| (k, normalize_json_value(v)))
                .collect();
            Value::Object(sorted.into_iter().collect::<Map<String, Value>>())
        }
        Value::Array(arr) => Value::Array(arr.into_iter().map(normalize_json_value).collect()),
        other => other,
    }
}

pub async fn register_functions() -> Vec<Arc<dyn NodeLogic>> {
    vec![
        Arc::new(try_transform::TryTransformNode::default()),
        Arc::new(from_bytes::FromBytesNode::default()),
        Arc::new(from_string::FromStringNode::default()),
        Arc::new(to_bytes::ToBytesNode::default()),
        Arc::new(to_string::ToStringNode::default()),
    ]
}
