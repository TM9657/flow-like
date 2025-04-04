use crate::{app::App, bit::BitMeta};
use flow_like_types::{FromProto, Timestamp, ToProto};
use std::time::SystemTime;

impl ToProto<flow_like_types::proto::App> for App {
    fn to_proto(&self) -> flow_like_types::proto::App {
        flow_like_types::proto::App {
            id: self.id.clone(),
            meta: self
                .meta
                .iter()
                .map(|(k, v)| (k.clone(), v.to_proto()))
                .collect(),
            authors: self.authors.clone(),
            bits: self.bits.clone(),
            boards: self.boards.clone(),
            created_at: Some(Timestamp::from(self.created_at)),
            updated_at: Some(Timestamp::from(self.updated_at)),
        }
    }
}

impl FromProto<flow_like_types::proto::App> for App {
    fn from_proto(proto: flow_like_types::proto::App) -> Self {
        App {
            id: proto.id,
            meta: proto
                .meta
                .into_iter()
                .map(|(k, v)| (k, BitMeta::from_proto(v)))
                .collect(),
            authors: proto.authors,
            bits: proto.bits,
            boards: proto.boards,
            created_at: proto
                .created_at
                .map(|t| SystemTime::try_from(t).unwrap_or(SystemTime::UNIX_EPOCH))
                .unwrap_or(SystemTime::UNIX_EPOCH),
            updated_at: proto
                .updated_at
                .map(|t| SystemTime::try_from(t).unwrap_or(SystemTime::UNIX_EPOCH))
                .unwrap_or(SystemTime::UNIX_EPOCH),
            app_state: None,
            frontend: None,
        }
    }
}
