use crate::bit::BitMeta;
use flow_like_types::{FromProto, ToProto};

impl ToProto<flow_like_types::proto::Meta> for BitMeta {
    fn to_proto(&self) -> flow_like_types::proto::Meta {
        flow_like_types::proto::Meta {
            name: self.name.clone(),
            description: self.description.clone(),
            long_description: self.long_description.clone(),
            tags: self.tags.clone(),
            use_case: self.use_case.clone(),
        }
    }
}

impl FromProto<flow_like_types::proto::Meta> for BitMeta {
    fn from_proto(proto: flow_like_types::proto::Meta) -> Self {
        BitMeta {
            name: proto.name,
            description: proto.description,
            long_description: proto.long_description,
            tags: proto.tags,
            use_case: proto.use_case,
        }
    }
}
