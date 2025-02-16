use async_trait::async_trait;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::{
    collections::{HashMap, HashSet},
    sync::Arc,
};

use crate::state::FlowLikeState;

use super::{
    board::Board,
    execution::context::ExecutionContext,
    pin::{Pin, PinType},
    variable::VariableType,
};

#[derive(Serialize, Deserialize, JsonSchema, Debug, Clone, PartialEq)]
pub enum NodeState {
    Idle,
    Running,
    Success,
    Error,
}

/// Node Scores. Each score ranges from 0 to 10.
/// Node Scores. From 0 - 10
/// The higher the score, the worse the node is in this category:
/// - Privacy: Higher score means less privacy.
/// - Security: Higher score means less security.
/// - Performance: Higher score means worse performance.
/// - Governance: Higher score means less compliance with governance.
/// - security: Assesses the node's resistance to attacks.
/// - performance: Evaluates the node's efficiency and speed.
/// - governance: Indicates the node's compliance with policies and regulations.
/// The higher the score, the worse the node is in this category
#[derive(Serialize, Deserialize, JsonSchema, Debug, Clone)]
pub struct NodeScores {
    pub privacy: i8,
    pub security: i8,
    pub performance: i8,
    pub governance: i8,
}

#[derive(Serialize, Deserialize, JsonSchema, Debug, Clone)]
pub struct Node {
    pub id: String,
    pub name: String,
    pub friendly_name: String,
    pub description: String,
    pub coordinates: Option<(f32, f32, f32)>,
    pub category: String,
    pub scores: Option<NodeScores>,
    pub pins: HashMap<String, Pin>,
    pub start: Option<bool>,
    pub icon: Option<String>,
    pub comment: Option<String>,
    pub long_running: Option<bool>,
    pub error: Option<String>,
}

impl Node {
    pub fn new(name: &str, friendly_name: &str, description: &str, category: &str) -> Self {
        Node {
            id: cuid2::create_id(),
            name: name.to_string(),
            friendly_name: friendly_name.to_string(),
            description: description.to_string(),
            coordinates: None,
            category: category.to_string(),
            pins: HashMap::new(),
            scores: Some(NodeScores {
                privacy: 0,
                security: 0,
                performance: 0,
                governance: 0,
            }),
            start: None,
            icon: None,
            comment: None,
            long_running: None,
            error: None,
        }
    }

    pub fn add_comment(&mut self, comment: &str) {
        self.comment = Some(comment.to_string());
    }

    pub fn add_icon(&mut self, icon: &str) {
        self.icon = Some(icon.to_string());
    }

    pub fn set_start(&mut self, start: bool) {
        self.start = Some(start);
    }

    pub fn add_input_pin(
        &mut self,
        name: &str,
        friendly_name: &str,
        description: &str,
        data_type: VariableType,
    ) -> &mut Pin {
        let pin_id = cuid2::create_id();
        let num_outputs = self
            .pins
            .iter()
            .filter(|(_, v)| v.pin_type == PinType::Input)
            .count();
        self.pins.insert(
            pin_id.clone(),
            Pin {
                id: pin_id.clone(),
                name: name.to_string(),
                friendly_name: friendly_name.to_string(),
                description: description.to_string(),
                schema: None,
                pin_type: PinType::Input,
                valid_values: None,
                data_type,
                value_type: super::pin::ValueType::Normal,
                depends_on: HashSet::new(),
                connected_to: HashSet::new(),
                default_value: None,
                options: None,
                value: None,
                index: num_outputs as u16 + 1,
            },
        );
        self.pins.get_mut(&pin_id).unwrap()
    }

    pub fn add_output_pin(
        &mut self,
        name: &str,
        friendly_name: &str,
        description: &str,
        data_type: VariableType,
    ) -> &mut Pin {
        let pin_id = cuid2::create_id();
        let num_outputs = self
            .pins
            .iter()
            .filter(|(_, v)| v.pin_type == PinType::Output)
            .count();
        self.pins.insert(
            pin_id.clone(),
            Pin {
                id: pin_id.clone(),
                name: name.to_string(),
                friendly_name: friendly_name.to_string(),
                description: description.to_string(),
                schema: None,
                options: None,
                pin_type: PinType::Output,
                valid_values: None,
                data_type,
                value_type: super::pin::ValueType::Normal,
                depends_on: HashSet::new(),
                connected_to: HashSet::new(),
                default_value: None,
                value: None,
                index: num_outputs as u16 + 1,
            },
        );
        self.pins.get_mut(&pin_id).unwrap()
    }

    pub fn is_pure(&self) -> bool {
        for pin in self.pins.values() {
            if pin.data_type == VariableType::Execution {
                return false;
            }
        }

        true
    }

    pub fn get_pin_by_name(&self, name: &str) -> Option<&Pin> {
        self.pins.values().find(|&pin| pin.name == name)
    }

    pub fn get_pin_mut_by_name(&mut self, name: &str) -> Option<&mut Pin> {
        self.pins.values_mut().find(|pin| pin.name == name)
    }

    pub fn set_long_running(&mut self, long_running: bool) {
        self.long_running = Some(long_running);
    }

    pub fn mut_scores(&mut self) -> &mut NodeScores {
        self.scores.as_mut().unwrap()
    }
}

#[async_trait]
pub trait NodeLogic: Send + Sync {
    async fn get_node(&self, handler: &FlowLikeState) -> Node;
    async fn run(&mut self, context: &mut ExecutionContext) -> anyhow::Result<()>;

    async fn get_progress(&self, context: &mut ExecutionContext) -> i32 {
        let state = context.get_state();

        match state {
            NodeState::Running => return 50,
            NodeState::Success => return 100,
            NodeState::Error => return 0,
            _ => return 0,
        }
    }

    async fn on_update(&self, _node: &mut Node, _board: Arc<Board>) {}
    async fn on_delete(&self, _node: &mut Node, _board: Arc<Board>) {}
}

#[cfg(test)]
mod tests {
    #[tokio::test]
    async fn serialize_node() {
        let node = super::Node::new("Hi", "Test Node", "What a wonderful day", "IDK");

        let ser = bitcode::serialize(&node).unwrap();
        let deser: super::Node = bitcode::deserialize(&ser).unwrap();

        assert_eq!(node.id, deser.id);
    }
}
