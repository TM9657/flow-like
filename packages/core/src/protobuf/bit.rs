use super::conversions::{FromProto, ToProto};
use crate::bit::BitMeta;

impl ToProto<super::types::Meta> for BitMeta {
    fn to_proto(&self) -> super::types::Meta {
        super::types::Meta {
            name: self.name.clone(),
            description: self.description.clone(),
            long_description: self.long_description.clone(),
            tags: self.tags.clone(),
            use_case: self.use_case.clone(),
        }
    }
}

impl FromProto<super::types::Meta> for BitMeta {
    fn from_proto(proto: super::types::Meta) -> Self {
        BitMeta {
            name: proto.name,
            description: proto.description,
            long_description: proto.long_description,
            tags: proto.tags,
            use_case: proto.use_case,
        }
    }
}
