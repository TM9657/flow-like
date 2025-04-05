use flow_like_types::{async_trait, create_id};
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
    pin::{Pin, PinType, ValueType},
    variable::VariableType,
};

#[derive(Serialize, Deserialize, JsonSchema, Debug, Clone, PartialEq)]
pub enum NodeState {
    Idle,
    Running,
    Success,
    Error,
}

/// Represents quality metrics for a node, with scores ranging from 0 to 10.
/// Higher scores indicate worse performance in each category.
///
/// # Score Categories
/// * `privacy` - Measures data protection and confidentiality level
/// * `security` - Assesses resistance against potential attacks
/// * `performance` - Evaluates computational efficiency and speed
/// * `governance` - Indicates compliance with policies and regulations
#[derive(Serialize, Deserialize, JsonSchema, Debug, Clone)]
pub struct NodeScores {
    pub privacy: u8,
    pub security: u8,
    pub performance: u8,
    pub governance: u8,
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
    pub docs: Option<String>,
}

impl Node {
    pub fn new(name: &str, friendly_name: &str, description: &str, category: &str) -> Self {
        Node {
            id: create_id(),
            name: name.to_string(),
            friendly_name: friendly_name.to_string(),
            description: description.to_string(),
            coordinates: None,
            category: category.to_string(),
            pins: HashMap::new(),
            scores: None,
            start: None,
            icon: None,
            comment: None,
            long_running: None,
            error: None,
            docs: None,
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
        let pin_id = create_id();
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
        let pin_id = create_id();
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

    pub fn match_type(
        &mut self,
        pin_name: &str,
        board: Arc<Board>,
        value_type: Option<ValueType>,
        default_type: Option<ValueType>,
    ) -> flow_like_types::Result<VariableType> {
        let mut found_type = VariableType::Generic;
        let pin = self
            .get_pin_by_name(pin_name)
            .ok_or(flow_like_types::anyhow!("Pin not found"))?;
        let mut nodes = pin.connected_to.clone();
        if pin.pin_type == PinType::Input {
            nodes = pin.depends_on.clone();
        }

        let default_type = default_type.unwrap_or(ValueType::Normal);

        self.get_pin_mut_by_name(pin_name).unwrap().data_type = VariableType::Generic;
        self.get_pin_mut_by_name(pin_name).unwrap().value_type = default_type;
        self.get_pin_mut_by_name(pin_name).unwrap().schema = None;
        if let Some(value_type) = &value_type {
            self.get_pin_mut_by_name(pin_name).unwrap().value_type = value_type.clone();
        }

        if let Some(first_node) = nodes.iter().next() {
            let pin = board.get_pin_by_id(first_node);
            let mutable_pin = self.get_pin_mut_by_name(pin_name).unwrap();

            match pin {
                Some(pin) => {
                    mutable_pin.data_type = pin.data_type.clone();
                    if value_type.is_none() {
                        mutable_pin.value_type = pin.value_type.clone();
                        mutable_pin.schema = pin.schema.clone();
                        found_type = pin.data_type.clone();
                    }
                }
                None => {
                    mutable_pin.depends_on.remove(first_node);
                }
            }
        }

        Ok(found_type)
    }
}

#[async_trait]
pub trait NodeLogic: Send + Sync {
    async fn get_node(&self, handler: &FlowLikeState) -> Node;
    async fn run(&self, context: &mut ExecutionContext) -> flow_like_types::Result<()>;

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

    use flow_like_types::{tokio, Message};
    use flow_like_types::{FromProto, ToProto};

    #[tokio::test]
    async fn serialize_node() {
        let node = super::Node::new("Hi", "Test Node", "What a wonderful day", "IDK");

        let mut buf = Vec::new();
        node.to_proto().encode(&mut buf).unwrap();
        let deser_node =
            super::Node::from_proto(flow_like_types::proto::Node::decode(&buf[..]).unwrap());

        assert_eq!(node.id, deser_node.id);
    }
}
