use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GeoLocationSink {
    pub latitude: f64,
    pub longitude: f64,
    pub radius: f64,
    pub trigger_on: GeoTriggerType,
    pub is_inside: Option<bool>,
    pub last_trigger_time: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum GeoTriggerType {
    Enter,
    Exit,
    Both,
}

// Implementation in stubs.rs
