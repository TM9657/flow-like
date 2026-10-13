// Listener futures include Lance log writes, which need deeper Send trait evaluation.
#![recursion_limit = "256"]

pub mod ads;
pub mod amqp;
pub mod cifx;
pub mod cip;
pub mod dds;
pub mod ethercat;
pub mod hart;
pub mod iolink;
pub mod iroh;
pub mod kafka;
pub mod nats;
pub mod profibus;
pub mod redis;
pub mod sparkplug;
pub mod zenoh;
extern crate flow_like_runtime as flow_like;

pub use flow_like_catalog_core::{NodeConstructor, NodeLogic, register_node};
pub mod legacy;
pub mod modbus;
pub mod node;
pub mod opcua;
pub mod opcua_pubsub;
pub mod runtime;

include!(concat!(env!("OUT_DIR"), "/node_registry.rs"));

pub fn get_catalog() -> Vec<std::sync::Arc<dyn NodeLogic>> {
    collect_nodes()
}

#[cfg(all(test, feature = "execute"))]
mod tests;

#[cfg(test)]
mod metadata_tests {
    #[test]
    fn registry_has_unique_ids_valid_schemas_and_legacy_nodes() {
        let mut names = std::collections::HashSet::new();
        for logic in super::get_catalog() {
            let node = logic.get_node();
            assert!(
                names.insert(node.name.clone()),
                "duplicate node {}",
                node.name
            );
            for pin in node.pins.values() {
                if let Some(schema) = &pin.schema {
                    serde_json::from_str::<serde_json::Value>(schema).unwrap();
                }
            }
        }
        for required in [
            "ml_read_modbus_sensor",
            "ml_read_opcua_sensor",
            "ml_decode_sensor_registers",
            "ml_capture_genicam_frame",
            "industrial_modbus_connect",
            "sparkplug_prepare_session",
            "industrial_dds_connect",
            "industrial_dds_wait_for_match",
            "industrial_dds_publish",
            "industrial_dds_receive",
            "industrial_dds_subscribe",
            "industrial_dds_disconnect",
            "industrial_opcua_pubsub_connect",
            "industrial_opcua_pubsub_publish",
            "industrial_opcua_pubsub_receive",
            "industrial_opcua_pubsub_subscribe",
            "industrial_opcua_pubsub_disconnect",
        ] {
            assert!(names.contains(required), "missing node {required}");
        }
    }

    #[test]
    fn certificate_and_group_key_inputs_are_sensitive() {
        for id in [
            "industrial_opcua_connect",
            "ml_read_opcua_sensor",
            "industrial_opcua_pubsub_connect",
            "industrial_dds_connect",
        ] {
            let node = super::get_catalog()
                .into_iter()
                .map(|logic| logic.get_node())
                .find(|node| node.name == id)
                .unwrap();
            let request = node
                .pins
                .values()
                .find(|pin| pin.name == "request")
                .unwrap();
            assert!(
                request.is_sensitive(),
                "{id} must hide certificate and key inputs"
            );
        }
    }
}
