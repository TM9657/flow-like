use std::sync::Arc;

use serde_json::Value;
use tokio::sync::Mutex;

use super::execution::internal_pin::InternalPin;

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
    let mut current_pin = pin;

    loop {
        let cloned_guard = current_pin.clone();
        let pin_guard = cloned_guard.lock().await;
        let pin = pin_guard.pin.lock().await;

        if let Some(value_arc) = pin.value.clone() {
            let value_guard = value_arc.lock().await;
            let value = value_guard.clone();
            return Ok(value);
        }

        if pin_guard.depends_on.is_empty() {
            if let Some(default_value) = pin.default_value.clone() {
                let value: Value = serde_json::from_slice(&default_value)?;
                return Ok(value);
            } else {
                return Err(anyhow::anyhow!("Pin value is not set"));
            }
        }

        drop(pin);

        let child = pin_guard.depends_on.first().unwrap();
        current_pin = child.clone();

        drop(pin_guard);
    }
}
