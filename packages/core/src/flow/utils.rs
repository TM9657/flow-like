use std::{
    collections::BTreeMap,
    sync::{Arc, Weak},
};

use flow_like_types::{Value, sync::Mutex};

use super::execution::internal_pin::InternalPin;

pub async fn evaluate_pin_value_reference(
    pin: Arc<Mutex<InternalPin>>,
) -> flow_like_types::Result<Arc<Mutex<Value>>> {
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
            return Err(flow_like_types::anyhow!(
                "Detected circular dependency in pin chain"
            ));
        }

        // Case 1: Pin has a value - directly return from here
        if let Some(value_arc) = value {
            return Ok(value_arc);
        }

        // Case 2: Pin depends on another pin
        if let Some(dependency) = first_dependency
            && let Some(dependency) = dependency.upgrade()
        {
            current_pin = dependency;
            continue;
        }

        // Case 3: Use default value if available
        if let Some(default_value) = default_value {
            return match flow_like_types::json::from_slice(&default_value) {
                Ok(value) => Ok(Arc::new(Mutex::new(value))),
                Err(e) => Err(flow_like_types::anyhow!(
                    "Failed to parse default value for pin '{}': {}",
                    friendly_name,
                    e
                )),
            };
        }

        // Case 4: No value found
        return Err(flow_like_types::anyhow!(
            "Pin '{}' has no value, dependencies, or default value",
            friendly_name
        ));
    }
}

pub async fn evaluate_pin_value_weak(
    pin: &Weak<Mutex<InternalPin>>,
    overrides: &Option<BTreeMap<String, Value>>,
) -> flow_like_types::Result<Value> {
    let pin = pin
        .upgrade()
        .ok_or_else(|| flow_like_types::anyhow!("Pin is not set"))?;
    evaluate_pin_value(pin, overrides).await
}

pub async fn evaluate_pin_value(
    pin: Arc<Mutex<InternalPin>>,
    overrides: &Option<BTreeMap<String, Value>>,
) -> flow_like_types::Result<Value> {
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
        if !visited_pins.insert(pin_id.clone()) {
            return Err(flow_like_types::anyhow!(
                "Detected circular dependency in pin chain"
            ));
        }

        // Case 1: Pin has a value - directly return from here
        if let Some(value_arc) = value {
            if let Some(found_override) = overrides.as_ref().and_then(|map| map.get(&pin_id)) {
                return Ok(found_override.clone());
            }
            return Ok(value_arc.lock().await.clone());
        }

        // Case 2: Pin depends on another pin
        if let Some(dependency) = first_dependency
            && let Some(dependency) = dependency.upgrade()
        {
            current_pin = dependency;
            continue;
        }

        // Case 3: Use default value if available
        if let Some(default_value) = default_value {
            if let Some(found_override) = overrides.as_ref().and_then(|map| map.get(&pin_id)) {
                return Ok(found_override.clone());
            }

            return match flow_like_types::json::from_slice(&default_value) {
                Ok(value) => Ok(value),
                Err(e) => Err(flow_like_types::anyhow!(
                    "Failed to parse default value for pin '{}': {}",
                    friendly_name,
                    e
                )),
            };
        }

        // Case 4: No value found
        return Err(flow_like_types::anyhow!(
            "Pin '{}' has no value, dependencies, or default value",
            friendly_name
        ));
    }
}
