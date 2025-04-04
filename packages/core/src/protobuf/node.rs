use super::conversions::{FromProto, ToProto};
use crate::flow::{
    node::{Node, NodeScores},
    pin::{Pin, PinOptions, PinType, ValueType},
    variable::{Variable, VariableType},
};
use serde_json::Value;
use std::sync::Arc;
use tokio::sync::Mutex;

impl ToProto<super::types::NodeScores> for NodeScores {
    fn to_proto(&self) -> super::types::NodeScores {
        super::types::NodeScores {
            privacy: self.privacy as u32,
            security: self.security as u32,
            performance: self.performance as u32,
            governance: self.governance as u32,
        }
    }
}

impl FromProto<super::types::NodeScores> for NodeScores {
    fn from_proto(proto: super::types::NodeScores) -> Self {
        NodeScores {
            privacy: proto.privacy as u8,
            security: proto.security as u8,
            performance: proto.performance as u8,
            governance: proto.governance as u8,
        }
    }
}

impl ToProto<super::types::Node> for Node {
    fn to_proto(&self) -> super::types::Node {
        let (coord_x, coord_y, coord_z) = self.coordinates.unwrap_or((0.0, 0.0, 0.0));
        super::types::Node {
            id: self.id.clone(),
            name: self.name.clone(),
            friendly_name: self.friendly_name.clone(),
            description: self.description.clone(),
            coord_x,
            coord_y,
            coord_z,
            category: self.category.clone(),
            scores: self.scores.as_ref().map(|s| s.to_proto()),
            pins: self
                .pins
                .iter()
                .map(|(k, v)| (k.clone(), v.to_proto()))
                .collect(),
            start: self.start.unwrap_or(false),
            icon: self.icon.clone().unwrap_or_default(),
            comment: self.comment.clone().unwrap_or_default(),
            long_running: self.long_running.unwrap_or(false),
            error: self.error.clone().unwrap_or_default(),
            docs: self.docs.clone().unwrap_or_default(),
        }
    }
}

impl FromProto<super::types::Node> for Node {
    fn from_proto(proto: super::types::Node) -> Self {
        Node {
            id: proto.id,
            name: proto.name,
            friendly_name: proto.friendly_name,
            description: proto.description,
            coordinates: Some((proto.coord_x, proto.coord_y, proto.coord_z)),
            category: proto.category,
            scores: proto.scores.map(NodeScores::from_proto),
            pins: proto
                .pins
                .into_iter()
                .map(|(k, v)| (k, Pin::from_proto(v)))
                .collect(),
            start: if proto.start { Some(true) } else { None },
            icon: if proto.icon.is_empty() {
                None
            } else {
                Some(proto.icon)
            },
            comment: if proto.comment.is_empty() {
                None
            } else {
                Some(proto.comment)
            },
            long_running: if proto.long_running { Some(true) } else { None },
            error: if proto.error.is_empty() {
                None
            } else {
                Some(proto.error)
            },
            docs: if proto.docs.is_empty() {
                None
            } else {
                Some(proto.docs)
            },
        }
    }
}
