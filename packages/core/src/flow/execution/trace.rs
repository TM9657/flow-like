use super::log::LogMessage;
use crate::flow::variable::Variable;
use ahash::AHashMap;
use flow_like_types::{create_id, sync::Mutex};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::{sync::Arc, time::SystemTime};

#[derive(Serialize, Deserialize, JsonSchema, Debug, Clone)]
pub struct Trace {
    pub id: String,
    pub node_id: String,
    pub logs: Vec<LogMessage>,
    pub start: SystemTime,
    pub end: SystemTime,

    // for debugging purposes only
    pub variables: Option<Vec<Variable>>,
}

impl Trace {
    pub fn new(node_id: &str) -> Self {
        Trace {
            id: create_id(),
            node_id: node_id.to_string(),
            logs: vec![],
            start: SystemTime::now(),
            end: SystemTime::now(),
            variables: None,
        }
    }

    pub fn get_start(&self) -> SystemTime {
        if self.logs.is_empty() {
            return self.start;
        }

        let found_earliest = self.logs.iter().min_by_key(|log| log.start).unwrap();
        found_earliest.start
    }

    pub fn finish(&mut self) {
        self.end = SystemTime::now();
    }

    pub async fn snapshot_variables(&mut self, variables: &Arc<Mutex<AHashMap<String, Variable>>>) {
        self.variables = Some(variables.lock().await.values().cloned().collect());
    }
}
