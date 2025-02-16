use std::sync::Arc;

use serde_json::Value;
use tokio::sync::Mutex;

use super::execution::internal_pin::InternalPin;

pub fn value_to_string(value: Value) -> String {
    match value {
        Value::String(s) => s,
        Value::Number(n) => n.to_string(),
        Value::Bool(b) => b.to_string(),
        Value::Null => "".to_string(),
        _ => "".to_string(),
    }
}

pub fn value_to_bool(value: Value) -> bool {
    match value {
        Value::Bool(b) => b,
        _ => false,
    }
}

pub fn value_to_i64(value: Value) -> i64 {
    match value {
        Value::Number(n) => n.as_i64().unwrap(),
        _ => 0,
    }
}

pub fn value_to_f64(value: Value) -> f64 {
    match value {
        Value::Number(n) => n.as_f64().unwrap(),
        _ => 0.0,
    }
}

pub async fn evaluate_pin_value_reference(
    pin: &Option<Arc<Mutex<InternalPin>>>,
) -> anyhow::Result<Arc<Mutex<Value>>> {
    if pin.is_none() {
        return Err(anyhow::anyhow!("Pin is not set"));
    }

    let pin = pin.clone().unwrap();

    let pin_guard = pin.lock().await;
    let pin = pin_guard.pin.lock().await;
    if pin.value.is_none() {
        if pin.default_value.is_none() {
            return Err(anyhow::anyhow!("Pin value is not set"));
        }

        let value = pin.default_value.clone().unwrap();
        let value = serde_json::from_slice(&value).unwrap();

        return Ok(Arc::new(Mutex::new(value)));
    }

    let value = pin.value.clone().unwrap();
    Ok(value)
}

pub async fn evaluate_pin_value(pin: Arc<Mutex<InternalPin>>) -> anyhow::Result<Value> {
    let pin_guard = pin.lock().await;
    let pin = pin_guard.pin.lock().await;

    if pin.value.is_some() {
        let value = pin.value.clone().unwrap();
        let value = value.lock().await;
        let value = value.clone();
        return Ok(value);
    }

    if pin_guard.depends_on.is_empty() {
        if pin.default_value.is_none() {
            return Err(anyhow::anyhow!("Pin value is not set"));
        }

        let value = pin.default_value.clone().unwrap();
        let value = serde_json::from_slice(&value).unwrap();
        return Ok(value);
    }

    drop(pin);

    let child = pin_guard.depends_on.first().unwrap().clone();
    drop(pin_guard);
    let value = Box::pin(evaluate_pin_value(child.clone())).await?;
    Ok(value)
}
