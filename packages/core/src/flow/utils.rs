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

    let (value, default_value, name) = {
        let pin_guard = pin.lock().await.pin.clone();
        let pin = pin_guard.lock().await;
        (
            pin.value.clone(),
            pin.default_value.clone(),
            pin.friendly_name.clone(),
        )
    };

    match value {
        Some(value) => Ok(value),
        None => {
            if let Some(default_value) = default_value {
                let value: Value = serde_json::from_slice(&default_value)?;
                return Ok(Arc::new(Mutex::new(value)));
            }

            Err(anyhow::anyhow!("Pin {} default value is not set", name))
        }
    }
}

pub async fn evaluate_pin_value(pin: Arc<Mutex<InternalPin>>) -> anyhow::Result<Value> {
    let mut current_pin = pin;
    let mut visited_pins = std::collections::HashSet::with_capacity(8);

    loop {
        // Step 1: Get internal pin reference and dependency with a single lock
        let (pin_ref, first_dependency) = {
            let guard = current_pin.lock().await;
            (guard.pin.clone(), guard.depends_on.first().cloned())
        };

        // Step 2: Get all pin data with a single lock
        let (pin_id, value, default_value, friendly_name) = {
            let pin = pin_ref.lock().await;
            (
                pin.id.clone(),
                pin.value.clone(),
                pin.default_value.clone(),
                pin.friendly_name.clone(),
            )
        };

        // Check for circular dependencies
        if !visited_pins.insert(pin_id) {
            return Err(anyhow::anyhow!("Detected circular dependency in pin chain"));
        }

        // Case 1: Pin has a value - directly return from here
        if let Some(value_arc) = value {
            return Ok(value_arc.lock().await.clone());
        }

        // Case 2: Pin depends on another pin
        if let Some(dependency) = first_dependency {
            current_pin = dependency;
            continue;
        }

        // Case 3: Use default value if available
        if let Some(default_value) = default_value {
            return match serde_json::from_slice(&default_value) {
                Ok(value) => Ok(value),
                Err(e) => Err(anyhow::anyhow!(
                    "Failed to parse default value for pin '{}': {}",
                    friendly_name,
                    e
                )),
            };
        }

        // Case 4: No value found
        return Err(anyhow::anyhow!(
            "Pin '{}' has no value, dependencies, or default value",
            friendly_name
        ));
    }
}
