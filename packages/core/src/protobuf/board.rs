use std::{collections::HashMap, time::SystemTime};

use object_store::path::Path;
use prost_types::Timestamp;

use crate::flow::{
    board::{Board, Comment, ExecutionStage},
    execution::LogLevel,
    node::Node,
    variable::Variable,
};

use super::conversions::{FromProto, ToProto};

impl ExecutionStage {
    fn to_proto(&self) -> i32 {
        match self {
            ExecutionStage::Dev => 0,
            ExecutionStage::Int => 1,
            ExecutionStage::QA => 2,
            ExecutionStage::PreProd => 3,
            ExecutionStage::Prod => 4,
        }
    }

    fn from_proto(value: i32) -> Self {
        match value {
            0 => ExecutionStage::Dev,
            1 => ExecutionStage::Int,
            2 => ExecutionStage::QA,
            3 => ExecutionStage::PreProd,
            4 => ExecutionStage::Prod,
            _ => ExecutionStage::Dev, // Default
        }
    }
}

impl LogLevel {
    fn to_proto(&self) -> i32 {
        match self {
            LogLevel::Debug => 0,
            LogLevel::Info => 1,
            LogLevel::Warn => 2,
            LogLevel::Error => 3,
            LogLevel::Fatal => 4,
        }
    }

    fn from_proto(value: i32) -> Self {
        match value {
            0 => LogLevel::Debug,
            1 => LogLevel::Info,
            2 => LogLevel::Warn,
            3 => LogLevel::Error,
            4 => LogLevel::Fatal,
            _ => LogLevel::Debug, // Default
        }
    }
}

impl ToProto<super::types::Board> for Board {
    fn to_proto(&self) -> super::types::Board {
        super::types::Board {
            id: self.id.clone(),
            name: self.name.clone(),
            description: self.description.clone(),
            nodes: self
                .nodes
                .iter()
                .map(|(k, v)| (k.clone(), v.to_proto()))
                .collect(),
            variables: self
                .variables
                .iter()
                .map(|(k, v)| (k.clone(), v.to_proto()))
                .collect(),
            comments: self
                .comments
                .iter()
                .map(|(k, v)| (k.clone(), v.to_proto()))
                .collect(),
            viewport_x: self.viewport.0,
            viewport_y: self.viewport.1,
            viewport_zoom: self.viewport.2,
            version_major: self.version.0 as u32,
            version_minor: self.version.1 as u32,
            version_patch: self.version.2 as u32,
            stage: self.stage.to_proto(),
            log_level: self.log_level.to_proto(),
            refs: self.refs.clone(),
            created_at: Some(Timestamp::from(self.created_at)),
            updated_at: Some(Timestamp::from(self.updated_at)),
        }
    }
}

impl FromProto<super::types::Board> for Board {
    fn from_proto(proto: super::types::Board) -> Self {
        Board {
            id: proto.id,
            name: proto.name,
            description: proto.description,
            nodes: proto
                .nodes
                .into_iter()
                .map(|(k, v)| (k, Node::from_proto(v)))
                .collect(),
            variables: proto
                .variables
                .into_iter()
                .map(|(k, v)| (k, Variable::from_proto(v)))
                .collect(),
            comments: proto
                .comments
                .into_iter()
                .map(|(k, v)| (k, Comment::from_proto(v)))
                .collect(),
            viewport: (proto.viewport_x, proto.viewport_y, proto.viewport_zoom),
            version: (
                proto.version_major as u8,
                proto.version_minor as u8,
                proto.version_patch as u8,
            ),
            stage: ExecutionStage::from_proto(proto.stage),
            log_level: LogLevel::from_proto(proto.log_level),
            refs: proto.refs,
            created_at: proto
                .created_at
                .map(|t| SystemTime::try_from(t).unwrap_or(SystemTime::UNIX_EPOCH))
                .unwrap_or(SystemTime::UNIX_EPOCH),
            updated_at: proto
                .updated_at
                .map(|t| SystemTime::try_from(t).unwrap_or(SystemTime::UNIX_EPOCH))
                .unwrap_or(SystemTime::UNIX_EPOCH),
            parent: None,
            board_dir: Path::from("/default"), // Placeholder, set as needed
            logic_nodes: HashMap::new(),
            app_state: None,
        }
    }
}
