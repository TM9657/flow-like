pub(crate) mod deserializer;
pub(crate) mod serializer;

#[doc(hidden)]
pub mod bytes;

/// Representation of data storage for dynamic types.
pub mod data_storage;

/// Classes related to the dynamic representation of types and data.
pub mod dynamic_type;

/// Classes related to the XTypes error and return codes.
pub mod error;

/// Classes related to the Type Object representation as defined in the DDS-XTypes 1.3 standard.
pub mod type_object;

/// Traits related to the representation of XTypes required to be transmitted using DDS
pub mod type_support;
