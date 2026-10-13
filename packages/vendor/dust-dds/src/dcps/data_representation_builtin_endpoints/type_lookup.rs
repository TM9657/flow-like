use dust_dds_derive::DdsType;

use crate::{
    transport::types::Guid,
    xtypes::type_object::{
        TypeIdentifier, TypeIdentifierPair, TypeIdentifierTypeObjectPair, TypeIdentifierWithSize,
    },
};
use alloc::{string::String, vec::Vec};

#[derive(DdsType, Clone)]
pub struct SequenceNumber {
    pub high: i32,
    pub low: u32,
}

impl From<i64> for SequenceNumber {
    fn from(value: i64) -> Self {
        Self {
            high: ((value as u64 & 0xFFFFFFFF00000000u64) >> 32) as i32,
            low: value as u32,
        }
    }
}

// computed from @hashid("getTypes")
pub const TYPE_LOOKUP_GET_TYPES_HASH_ID: i32 = 0x018252d3;
// computed from @hashid("getDependencies");
pub const TYPE_LOOKUP_GET_DEPENDENCIES_HASH_ID: i32 = 0x05aafb31;

// Query the TypeObjects associated with one or more TypeIdentifiers
#[derive(DdsType)]
#[dust_dds(extensibility = "mutable")]
pub struct TypeLookupGetTypesIn {
    #[dust_dds(hashid)]
    pub type_ids: Vec<TypeIdentifier>,
}

#[derive(DdsType)]
#[dust_dds(extensibility = "mutable")]
pub struct TypeLookupGetTypesOut {
    #[dust_dds(hashid)]
    pub types: Vec<TypeIdentifierTypeObjectPair>,
    #[dust_dds(hashid)]
    pub complete_to_minimal: Vec<TypeIdentifierPair>,
}

#[derive(DdsType)]
#[dust_dds(extensibility = "appendable", switch(i32))]
pub enum TypeLookupGetTypesResult {
    #[dust_dds(case = 0)]
    Ok { result: TypeLookupGetTypesOut },
}

// Query TypeIdentifiers that the specified types depend on
#[derive(DdsType)]
#[dust_dds(extensibility = "mutable")]
pub struct TypeLookupGetTypeDependenciesIn {
    #[dust_dds(hashid)]
    pub type_ids: Vec<TypeIdentifier>, // @hashid
    #[dust_dds(hashid)]
    pub continuation_point: Vec<u8>, // @hashid sequence<octet, 32> ;
}

#[derive(DdsType)]
#[dust_dds(extensibility = "mutable")]
pub struct TypeLookupGetTypeDependenciesOut {
    #[dust_dds(hashid)]
    pub dependent_typeids: Vec<TypeIdentifierWithSize>,
    #[dust_dds(hashid)]
    pub continuation_point: Vec<u8>,
}

#[derive(DdsType)]
#[dust_dds(switch(i32), extensibility = "appendable")]
pub enum TypeLookupGetTypeDependenciesResult {
    #[dust_dds(case = 0)]
    Ok {
        result: TypeLookupGetTypeDependenciesOut,
    },
}

// Service Request
#[derive(DdsType)]
#[dust_dds(switch(i32), extensibility = "appendable")]
pub enum TypeLookupCall {
    #[dust_dds(case=TYPE_LOOKUP_GET_TYPES_HASH_ID)]
    TypeLookupGetTypesHashId { get_types: TypeLookupGetTypesIn },
    #[dust_dds(case=TYPE_LOOKUP_GET_DEPENDENCIES_HASH_ID)]
    TypeLookupGetDependenciesHash {
        get_type_dependencies: TypeLookupGetTypeDependenciesIn,
    },
}

#[derive(DdsType)]
#[dust_dds(extensibility = "final")]
pub struct TypeLookupRequest {
    pub header: RequestHeader,
    pub call: TypeLookupCall,
}

// Service Reply
#[derive(DdsType)]
#[dust_dds(switch(i32), extensibility = "appendable")]
pub enum TypeLookupReturn {
    #[dust_dds(case=TYPE_LOOKUP_GET_TYPES_HASH_ID)]
    TypeLookupGetTypesHash { get_type: TypeLookupGetTypesResult },
    #[dust_dds(case=TYPE_LOOKUP_GET_DEPENDENCIES_HASH_ID)]
    TypeLookupGetDependenciesHash {
        get_type_dependencies: TypeLookupGetTypeDependenciesResult,
    },
}

#[derive(DdsType)]
#[dust_dds(extensibility = "final")]
pub struct TypeLookupReply {
    pub header: ReplyHeader,
    pub r#return: TypeLookupReturn,
}

// DDS RPC Types
#[derive(DdsType, Clone)]
#[dust_dds(extensibility = "final")]
pub struct RequestHeader {
    pub request_id: SampleIdentity,
    pub instance_name: InstanceName,
}

#[derive(DdsType)]
#[dust_dds(extensibility = "final")]
pub struct ReplyHeader {
    pub related_request_id: SampleIdentity,
    pub remote_ex: RemoteExceptionCode,
}

#[derive(DdsType, Clone)]
#[dust_dds(extensibility = "final")]
pub struct SampleIdentity {
    pub writer_guid: Guid,
    pub sequence_number: SequenceNumber,
}

pub type InstanceName = String; //typedef string<255> InstanceName;

#[derive(DdsType, Clone, Copy, PartialEq, Eq, Debug)]
pub enum RemoteExceptionCode {
    Ok,
    Unsupported,
    InvalidArgument,
    OutOfResources,
    UnknownOperation,
    UnknownException,
}
