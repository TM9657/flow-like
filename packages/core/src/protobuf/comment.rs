use super::conversions::{FromProto, ToProto};
use crate::flow::board::{Comment, CommentType};
use prost_types::Timestamp;
use std::time::SystemTime;

impl CommentType {
    fn to_proto(&self) -> i32 {
        match self {
            CommentType::Text => 0,
            CommentType::Image => 1,
            CommentType::Video => 2,
        }
    }

    fn from_proto(value: i32) -> Self {
        match value {
            0 => CommentType::Text,
            1 => CommentType::Image,
            2 => CommentType::Video,
            _ => CommentType::Text, // Default
        }
    }
}

impl ToProto<super::types::Comment> for Comment {
    fn to_proto(&self) -> super::types::Comment {
        super::types::Comment {
            id: self.id.clone(),
            author: self.author.clone().unwrap_or_default(),
            content: self.content.clone(),
            comment_type: self.comment_type.to_proto(),
            timestamp: Some(Timestamp::from(self.timestamp)),
            coord_x: self.coordinates.0,
            coord_y: self.coordinates.1,
            coord_z: self.coordinates.2,
        }
    }
}

impl FromProto<super::types::Comment> for Comment {
    fn from_proto(proto: super::types::Comment) -> Self {
        Comment {
            id: proto.id,
            author: if proto.author.is_empty() {
                None
            } else {
                Some(proto.author)
            },
            content: proto.content,
            comment_type: CommentType::from_proto(proto.comment_type),
            timestamp: proto
                .timestamp
                .map(|t| SystemTime::try_from(t).unwrap_or(SystemTime::UNIX_EPOCH))
                .unwrap_or(SystemTime::UNIX_EPOCH),
            coordinates: (proto.coord_x, proto.coord_y, proto.coord_z),
        }
    }
}
