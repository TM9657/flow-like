use super::dynamic_type::{ExtensibilityKind, TryConstructKind, TypeKind};
use crate::{
    dcps::infrastructure::qos_policy::TypeConsistencyEnforcementQosPolicy,
    xtypes::{
        dynamic_type::{DynamicType, DynamicTypeMember},
        serializer::serialize_without_header_cdr2_le,
        type_support::TypeSupport,
    },
};
use alloc::{boxed::Box, string::String, vec, vec::Vec};
use dust_dds_derive::DdsType;

/* Manually created from dds-xtypes_typeobject.idl */

// ---------- Equivalence Kinds -------------------
/// The kind of equivalence relation used for a type representation.
pub type EquivalenceKind = u8;
/// Represents the minimal equivalence kind.
pub const EK_MINIMAL: EquivalenceKind = 0xF1; // 0x1111 0001
/// Represents the complete equivalence kind.
pub const EK_COMPLETE: EquivalenceKind = 0xF2; // 0x1111 0010
/// Represents both minimal and complete equivalence kinds.
pub const EK_BOTH: EquivalenceKind = 0xF3; // 0x1111 0011

// ---------- TypeKinds (begin) -------------------
// Primitive TKs
/// Represents no type kind.
pub const TK_NONE: u8 = 0x00;
/// Represents the boolean type kind.
pub const TK_BOOLEAN: u8 = 0x01;
/// Represents the byte type kind.
pub const TK_BYTE: u8 = 0x02;
/// Represents the signed 16-bit integer type kind.
pub const TK_INT16: u8 = 0x03;
/// Represents the signed 32-bit integer type kind.
pub const TK_INT32: u8 = 0x04;
/// Represents the signed 64-bit integer type kind.
pub const TK_INT64: u8 = 0x05;
/// Represents the unsigned 16-bit integer type kind.
pub const TK_UINT16: u8 = 0x06;
/// Represents the unsigned 32-bit integer type kind.
pub const TK_UINT32: u8 = 0x07;
/// Represents the unsigned 64-bit integer type kind.
pub const TK_UINT64: u8 = 0x08;
/// Represents the single-precision floating-point type kind.
pub const TK_FLOAT32: u8 = 0x09;
/// Represents the double-precision floating-point type kind.
pub const TK_FLOAT64: u8 = 0x0A;
/// Represents the 128-bit floating-point type kind.
pub const TK_FLOAT128: u8 = 0x0B;
/// Represents the signed 8-bit integer type kind.
pub const TK_INT8: u8 = 0x0C;
/// Represents the unsigned 8-bit integer type kind.
pub const TK_UINT8: u8 = 0x0D;
/// Represents the 8-bit character type kind.
pub const TK_CHAR8: u8 = 0x10;
/// Represents the 16-bit character type kind.
pub const TK_CHAR16: u8 = 0x11;

// String TKs
/// Represents the 8-bit string type kind.
pub const TK_STRING8: u8 = 0x20;
/// Represents the 16-bit string type kind.
pub const TK_STRING16: u8 = 0x21;

// Constructed/Named types
/// Represents the alias type kind.
pub const TK_ALIAS: u8 = 0x30;

// Enumerated TKs
/// Represents the enumeration type kind.
pub const TK_ENUM: u8 = 0x40;
/// Represents the bitmask type kind.
pub const TK_BITMASK: u8 = 0x41;

// Structured TKs
/// Represents the annotation type kind.
pub const TK_ANNOTATION: u8 = 0x50;
/// Represents the structure type kind.
pub const TK_STRUCTURE: u8 = 0x51;
/// Represents the union type kind.
pub const TK_UNION: u8 = 0x52;
/// Represents the bitset type kind.
pub const TK_BITSET: u8 = 0x53;

// Collection TKs
/// Represents the sequence type kind.
pub const TK_SEQUENCE: u8 = 0x60;
/// Represents the array type kind.
pub const TK_ARRAY: u8 = 0x61;
/// Represents the map type kind.
pub const TK_MAP: u8 = 0x62;
// ---------- TypeKinds (end) -------------------

// ---------- Extra TypeIdentifiers (begin) ------------
/// The kind of extra type identifiers.
pub type TypeIdentiferKind = u8;
/// Represents a small 8-bit string type identifier.
pub const TI_STRING8_SMALL: u8 = 0x70;
/// Represents a large 8-bit string type identifier.
pub const TI_STRING8_LARGE: u8 = 0x71;
/// Represents a small 16-bit string type identifier.
pub const TI_STRING16_SMALL: u8 = 0x72;
/// Represents a large 16-bit string type identifier.
pub const TI_STRING16_LARGE: u8 = 0x73;
/// Represents a small plain sequence type identifier.
pub const TI_PLAIN_SEQUENCE_SMALL: u8 = 0x80;
/// Represents a large plain sequence type identifier.
pub const TI_PLAIN_SEQUENCE_LARGE: u8 = 0x81;
/// Represents a small plain array type identifier.
pub const TI_PLAIN_ARRAY_SMALL: u8 = 0x90;
/// Represents a large plain array type identifier.
pub const TI_PLAIN_ARRAY_LARGE: u8 = 0x91;
/// Represents a small plain map type identifier.
pub const TI_PLAIN_MAP_SMALL: u8 = 0xA0;
/// Represents a large plain map type identifier.
pub const TI_PLAIN_MAP_LARGE: u8 = 0xA1;
/// Represents a strongly connected component type identifier.
pub const TI_STRONGLY_CONNECTED_COMPONENT: u8 = 0xB0;
// ---------- Extra TypeIdentifiers (end) --------------

// The name of some element (e.g. type, type member, module)
// Valid characters are alphanumeric plus the "_" cannot start with digit
/// The maximum length of a member name.
pub const MEMBER_NAME_MAX_LENGTH: i32 = 256;
/// Type alias for a member name.
pub type MemberName = String; //string<MEMBER_NAME_MAX_LENGTH>

// Qualified type name includes the name of containing modules
// using "::" as separator. No leading "::". E.g. "MyModule::MyType"
/// The maximum length of a type name.
pub const TYPE_NAME_MAX_LENGTH: i32 = 256;
/// Type alias for a qualified type name.
pub type QualifiedTypeName = String; //string<TYPE_NAME_MAX_LENGTH>

/// Type alias for a primitive type ID.
// Every type has an ID. Those of the primitive types are pre-defined.
pub type PrimitiveTypeId = u8;

/// Hash representing the equivalence of a TypeObject.
// First 14 bytes of MD5 of the serialized TypeObject using XCDR
// version 2 with Little Endian encoding
pub type EquivalenceHash = [u8; 14];

/// Hash representing a member name.
// First 4 bytes of MD5 of of a member name converted to bytes
// using UTF-8 encoding and without a 'nul' terminator.
// Example: the member name "color" has NameHash {0x70, 0xDD, 0xA5, 0xDF}
pub type NameHash = [u8; 4];

/// Long bound representation.
pub type LBound = u32;
/// Sequence of long bounds.
pub type LBoundSeq = Vec<LBound>;
/// Value representing an invalid long bound.
pub const INVALID_LBOUND: LBound = 0;

/// Short bound representation.
pub type SBound = u8;
/// Sequence of short bounds.
pub type SBoundSeq = Vec<SBound>;
/// Value representing an invalid short bound.
pub const INVALID_SBOUND: SBound = 0;

/// Unique identifier for a TypeObject based on its equivalence hash.
#[derive(DdsType, Debug, Clone, PartialEq, Eq)]
#[dust_dds(extensibility = "final", nested, switch(u8))]
pub enum TypeObjectHashId {
    /// The hash for a complete TypeObject representation.
    #[dust_dds(case=EK_COMPLETE)]
    EkComplete {
        /// The 14-byte MD5 equivalence hash of the complete TypeObject.
        hash: EquivalenceHash,
    },
    /// The hash for a minimal TypeObject representation.
    #[dust_dds(case=EK_MINIMAL)]
    EkMinimal {
        /// The 14-byte MD5 equivalence hash of the minimal TypeObject.
        hash: EquivalenceHash,
    },
}

// Flags that apply to struct/union/collection/enum/bitmask/bitset
// members/elements and DO affect type assignability
// Depending on the flag it may not apply to members of all types
// When not all, the applicable member types are listed

/// Flags that apply to struct/union/collection/enum/bitmask/bitset members/elements and affect type assignability.
#[derive(Debug, Clone, Copy, DdsType, PartialEq, Eq, Default)]
pub struct MemberFlag(u16);

impl core::ops::BitOr for MemberFlag {
    type Output = MemberFlag;

    fn bitor(self, rhs: Self) -> Self::Output {
        Self(self.0 | rhs.0)
    }
}

impl core::ops::BitOrAssign for MemberFlag {
    fn bitor_assign(&mut self, rhs: Self) {
        self.0 |= rhs.0;
    }
}

/// Flag indicating try construct behavior bit 1.
pub const MEMBER_FLAG_TRY_CONSTRUCT1: MemberFlag = MemberFlag(1 << 0); // T1 | 00 = INVALID, 01 = DISCARD
/// Flag indicating try construct behavior bit 2.
pub const MEMBER_FLAG_TRY_CONSTRUCT2: MemberFlag = MemberFlag(1 << 1); // T2 | 10 = USE_DEFAULT, 11 = TRIM
/// Flag indicating the member is external (allocated out-of-line).
pub const MEMBER_FLAG_IS_EXTERNAL: MemberFlag = MemberFlag(1 << 2); // X StructMember, UnionMember,
// CollectionElement
/// Flag indicating the member is optional.
pub const MEMBER_FLAG_IS_OPTIONAL: MemberFlag = MemberFlag(1 << 3); // O StructMember
/// Flag indicating the member must be understood by the reader.
pub const MEMBER_FLAG_IS_MUST_UNDERSTAND: MemberFlag = MemberFlag(1 << 4); // M StructMember, UnionDiscriminator
/// Flag indicating the member is part of the type's key.
pub const MEMBER_FLAG_IS_KEY: MemberFlag = MemberFlag(1 << 5); // K StructMember, UnionDiscriminator
/// Flag indicating the member is the default member (e.g. for unions).
pub const MEMBER_FLAG_IS_DEFAULT: MemberFlag = MemberFlag(1 << 6); // D UnionMember, EnumerationLiteral

/// Flags that apply to collection elements.
pub type CollectionElementFlag = MemberFlag; // T1, T2, X

/// Flags that apply to structure members.
pub type StructMemberFlag = MemberFlag; // T1, T2, O, M, K, X
/// Flags that apply to union members.
pub type UnionMemberFlag = MemberFlag; // T1, T2, D, X
/// Flags that apply to union discriminator.
pub type UnionDiscriminatorFlag = MemberFlag; // T1, T2, K, M
/// Flags that apply to enumerated literals.
pub type EnumeratedLiteralFlag = MemberFlag; // D
/// Flags that apply to annotation parameters.
pub type AnnotationParameterFlag = MemberFlag; // Unused. No flags apply
/// Flags that apply to alias members.
pub type AliasMemberFlag = MemberFlag; // Unused. No flags apply
/// Flags that apply to bitflags.
pub type BitflagFlag = MemberFlag; // Unused. No flags apply
/// Flags that apply to bitset members.
pub type BitsetMemberFlag = MemberFlag; // Unused. No flags apply

// Mask used to remove the flags that do no affect assignability
// Selects T1, T2, O, M, K, D
/// Mask selecting member flags that affect minimal equivalence checking.
pub const MEMBER_FLAG_MINIMAL_MASK: MemberFlag = MemberFlag(0x003f);
// Flags that apply to type declarationa and DO affect assignability
// Depending on the flag it may not apply to all types
// When not all, the applicable types are listed

/// Flags that apply to type declarations and affect type assignability.
#[derive(DdsType, Debug, Clone, Copy, PartialEq, Eq)]
#[dust_dds(extensibility = "final", nested)]
pub struct TypeFlag(u16);

impl core::ops::BitOr for TypeFlag {
    type Output = TypeFlag;
    fn bitor(self, rhs: TypeFlag) -> Self::Output {
        Self(self.0 | rhs.0)
    }
}

impl core::ops::BitOrAssign for TypeFlag {
    fn bitor_assign(&mut self, rhs: Self) {
        self.0 |= rhs.0;
    }
}

impl core::ops::BitAnd for TypeFlag {
    type Output = TypeFlag;
    fn bitand(self, rhs: TypeFlag) -> Self::Output {
        Self(self.0 & rhs.0)
    }
}

/// Flag indicating the type is final (cannot be extended).
pub const TYPE_FLAG_IS_FINAL: TypeFlag = TypeFlag(1 << 0); // F
/// Flag indicating the type is appendable (new members can be added at the end).
pub const TYPE_FLAG_IS_APPENDABLE: TypeFlag = TypeFlag(1 << 1); // A |- Struct, Union
/// Flag indicating the type is mutable (members can be added, removed, or reordered).
pub const TYPE_FLAG_IS_MUTABLE: TypeFlag = TypeFlag(1 << 2); // M | (exactly one flag)
/// Flag indicating the type is nested inside another type.
pub const TYPE_FLAG_IS_NESTED: TypeFlag = TypeFlag(1 << 3); // N Struct, Union
/// Flag indicating member IDs are automatically assigned based on hash.
pub const TYPE_FLAG_IS_AUTOID_HASH: TypeFlag = TypeFlag(1 << 4); // H Struct

/// Type flags that apply to structures.
pub type StructTypeFlag = TypeFlag; // All flags apply
/// Type flags that apply to unions.
pub type UnionTypeFlag = TypeFlag; // All flags apply
/// Type flags that apply to collections.
pub type CollectionTypeFlag = TypeFlag; // Unused. No flags apply
/// Type flags that apply to annotations.
pub type AnnotationTypeFlag = TypeFlag; // Unused. No flags apply
/// Type flags that apply to aliases.
pub type AliasTypeFlag = TypeFlag; // Unused. No flags apply
/// Type flags that apply to enumerations.
pub type EnumTypeFlag = TypeFlag; // Unused. No flags apply
/// Type flags that apply to bitmasks.
pub type BitmaskTypeFlag = TypeFlag; // Unused. No flags apply
/// Type flags that apply to bitsets.
pub type BitsetTypeFlag = TypeFlag; // Unused. No flags apply

// Mask used to remove the flags that do no affect assignability
/// Mask selecting type flags that affect minimal equivalence checking.
pub const TYPE_FLAG_MINIMAL_MASK: TypeFlag = TypeFlag(0x0007); // Selects M, A, F

// 1 Byte
/// Definition of a small string type (bound <= 255).
#[derive(DdsType, Debug, Clone, PartialEq, Eq)]
#[dust_dds(extensibility = "final", nested)]
pub struct StringSTypeDefn {
    /// The short bound (maximum length) of the string.
    pub bound: SBound,
}
// 4 Bytes
/// Definition of a large string type (bound > 255).
#[derive(DdsType, Debug, Clone, PartialEq, Eq)]
#[dust_dds(extensibility = "final", nested)]
pub struct StringLTypeDefn {
    /// The long bound (maximum length) of the string.
    pub bound: LBound,
}

/// Common header for plain collection types (sequence, array, map).
#[derive(DdsType, Debug, Clone, PartialEq, Eq)]
#[dust_dds(extensibility = "final", nested)]
pub struct PlainCollectionHeader {
    /// The equivalence kind of the collection.
    pub equiv_kind: EquivalenceKind,
    /// Flags that apply to the elements of the collection.
    pub element_flags: CollectionElementFlag,
}

/// Definition of a plain sequence with a small bound.
#[derive(DdsType, Debug, Clone, PartialEq, Eq)]
#[dust_dds(extensibility = "final", nested)]
pub struct PlainSequenceSElemDefn {
    /// The header containing collection properties.
    pub header: PlainCollectionHeader,
    /// The small bound (maximum number of elements).
    pub bound: SBound,
    /// The identifier of the element type.
    #[dust_dds(external)]
    pub element_identifier: Box<TypeIdentifier>,
}

/// Definition of a plain sequence with a large bound.
#[derive(DdsType, Debug, Clone, PartialEq, Eq)]
#[dust_dds(extensibility = "final", nested)]
pub struct PlainSequenceLElemDefn {
    /// The header containing collection properties.
    pub header: PlainCollectionHeader,
    /// The large bound (maximum number of elements).
    pub bound: LBound,
    /// The identifier of the element type.
    #[dust_dds(external)]
    pub element_identifier: Box<TypeIdentifier>,
}

/// Definition of a plain array with small bounds.
#[derive(DdsType, Debug, Clone, PartialEq, Eq)]
#[dust_dds(extensibility = "final", nested)]
pub struct PlainArraySElemDefn {
    /// The header containing collection properties.
    pub header: PlainCollectionHeader,
    /// The sequence of small bounds for each dimension.
    pub array_bound_seq: SBoundSeq,
    /// The identifier of the element type.
    #[dust_dds(external)]
    pub element_identifier: Box<TypeIdentifier>,
}

/// Definition of a plain array with large bounds.
#[derive(DdsType, Debug, Clone, PartialEq, Eq)]
#[dust_dds(extensibility = "final", nested)]
pub struct PlainArrayLElemDefn {
    /// The header containing collection properties.
    pub header: PlainCollectionHeader,
    /// The sequence of large bounds for each dimension.
    pub array_bound_seq: LBoundSeq,
    /// The identifier of the element type.
    #[dust_dds(external)]
    pub element_identifier: Box<TypeIdentifier>,
}

/// Definition of a plain map with a small bound.
#[derive(DdsType, Debug, Clone, PartialEq, Eq)]
#[dust_dds(extensibility = "final", nested)]
pub struct PlainMapSTypeDefn {
    /// The header containing collection properties.
    pub header: PlainCollectionHeader,
    /// The small bound (maximum number of key-value pairs).
    pub bound: SBound,
    /// The identifier of the element type.
    #[dust_dds(external)]
    pub element_identifier: Box<TypeIdentifier>,
    /// Flags that apply to the keys of the map.
    pub key_flags: CollectionElementFlag,
    /// The identifier of the key type.
    #[dust_dds(external)]
    pub key_identifier: Box<TypeIdentifier>,
}

/// Definition of a plain map with a large bound.
#[derive(DdsType, Debug, Clone, PartialEq, Eq)]
#[dust_dds(extensibility = "final", nested)]
pub struct PlainMapLTypeDefn {
    /// The header containing collection properties.
    pub header: PlainCollectionHeader,
    /// The large bound (maximum number of key-value pairs).
    pub bound: LBound,
    /// The identifier of the element type.
    #[dust_dds(external)]
    pub element_identifier: Box<TypeIdentifier>,
    /// Flags that apply to the keys of the map.
    pub key_flags: CollectionElementFlag,
    /// The identifier of the key type.
    #[dust_dds(external)]
    pub key_identifier: Box<TypeIdentifier>,
}

/// Identifier for a type that belongs to a strongly connected component (has cyclic dependencies).
#[derive(DdsType, Debug, Clone, PartialEq, Eq)]
#[dust_dds(extensibility = "appendable", nested)]
pub struct StronglyConnectedComponentId {
    /// The hash of the strongly connected component.
    pub sc_component_id: TypeObjectHashId, // Hash StronglyConnectedComponent
    /// The number of types in the strongly connected component.
    pub scc_length: i32, // StronglyConnectedComponent.length
    /// The 0-based index identifying this specific type within the strongly connected component.
    pub scc_index: i32, // identify type in Strongly Connected Comp.
}
/// Structure representing future extensions to type definitions.
#[derive(DdsType, Debug, Clone, PartialEq)]
#[dust_dds(extensibility = "mutable", nested)]
pub struct ExtendedTypeDefn {
    // Empty. Available for future extension
}

/// The TypeIdentifier uniquely identifies a type (a set of equivalent
/// types according to an equivalence relationship: COMPLETE, MNIMAL).
///
/// In some cases (primitive types, strings, plain types) the identifier
/// is a explicit description of the type.
/// In other cases the Identifier is a Hash of the type description
///
/// In the of primitive types and strings the implied equivalence
/// relation is the identity.
///
/// For Plain Types and Hash-defined TypeIdentifiers there are three
/// possibilities: MINIMAL, COMPLETE, and COMMON:
/// - MINIMAL indicates the TypeIdentifier identifies equivalent types
///   according to the MINIMAL equivalence relation
/// - COMPLETE indicates the TypeIdentifier identifies equivalent types
///   according to the COMPLETE equivalence relation
/// - COMMON indicates the TypeIdentifier identifies equivalent types
///   according to both the MINIMAL and the COMMON equivalence relation.
///   This means the TypeIdentifier is the same for both relationships
#[derive(DdsType, Debug, Clone, PartialEq, Eq)]
#[dust_dds(extensibility = "final", nested, switch(u8))]
pub enum TypeIdentifier {
    /// No type.
    #[dust_dds(case=TK_NONE)]
    TkNone,
    /// Boolean type.
    #[dust_dds(case=TK_BOOLEAN)]
    TkBoolean,
    /// Byte type.
    #[dust_dds(case=TK_BYTE)]
    TkByteType,
    /// Signed 8-bit integer type.
    #[dust_dds(case=TK_INT8)]
    TkInt8Type,
    /// Signed 16-bit integer type.
    #[dust_dds(case=TK_INT16)]
    TkInt16Type,
    /// Signed 32-bit integer type.
    #[dust_dds(case=TK_INT32)]
    TkInt32Type,
    /// Signed 64-bit integer type.
    #[dust_dds(case=TK_INT64)]
    TkInt64Type,
    /// Unsigned 8-bit integer type.
    #[dust_dds(case=TK_UINT8)]
    TkUint8Type,
    /// Unsigned 16-bit integer type.
    #[dust_dds(case=TK_UINT16)]
    TkUint16Type,
    /// Unsigned 32-bit integer type.
    #[dust_dds(case=TK_UINT32)]
    TkUint32Type,
    /// Unsigned 64-bit integer type.
    #[dust_dds(case=TK_UINT64)]
    TkUint64Type,
    /// Single-precision floating-point type.
    #[dust_dds(case=TK_FLOAT32)]
    TkFloat32Type,
    /// Double-precision floating-point type.
    #[dust_dds(case=TK_FLOAT64)]
    TkFloat64Type,
    /// 128-bit floating-point type.
    #[dust_dds(case=TK_FLOAT128)]
    TkFloat128Type,
    /// 8-bit character type.
    #[dust_dds(case=TK_CHAR8)]
    TkChar8Type,
    /// 16-bit character type.
    #[dust_dds(case=TK_CHAR16)]
    TkChar16Type,
    // ============ Strings - use TypeIdentifierKind ===================
    /// Small 8-bit string type definition.
    #[dust_dds(case=TI_STRING8_SMALL)]
    TiString8Small {
        /// The string definition.
        string_sdefn: StringSTypeDefn,
    },
    /// Small 16-bit string type definition.
    #[dust_dds(case=TI_STRING16_SMALL)]
    TiString16Small {
        /// The string definition.
        string_sdefn: StringSTypeDefn,
    },
    /// Large 8-bit string type definition.
    #[dust_dds(case=TI_STRING8_LARGE)]
    TiString8Large {
        /// The string definition.
        string_ldefn: StringLTypeDefn,
    },
    /// Large 16-bit string type definition.
    #[dust_dds(case=TI_STRING16_LARGE)]
    TiString16Large {
        /// The string definition.
        string_ldefn: StringLTypeDefn,
    },
    // ============ Plain collectios - use TypeIdentifierKind =========
    /// Small plain sequence type identifier.
    #[dust_dds(case=TI_PLAIN_SEQUENCE_SMALL)]
    TiPlainSequenceSmall {
        /// The sequence definition.
        seq_sdefn: PlainSequenceSElemDefn,
    },
    /// Large plain sequence type identifier.
    #[dust_dds(case=TI_PLAIN_SEQUENCE_LARGE)]
    TiPlainSequenceLarge {
        /// The sequence definition.
        seq_ldefn: PlainSequenceLElemDefn,
    },
    /// Small plain array type identifier.
    #[dust_dds(case=TI_PLAIN_ARRAY_SMALL)]
    TiPlainArraySmall {
        /// The array definition.
        array_sdefn: PlainArraySElemDefn,
    },
    /// Large plain array type identifier.
    #[dust_dds(case=TI_PLAIN_ARRAY_LARGE)]
    TiPlainArrayLarge {
        /// The array definition.
        array_ldefn: PlainArrayLElemDefn,
    },
    /// Small plain map type identifier.
    #[dust_dds(case=TI_PLAIN_MAP_SMALL)]
    TiPlainMapSmall {
        /// The map definition.
        map_sdefn: PlainMapSTypeDefn,
    },
    /// Large plain map type identifier.
    #[dust_dds(case=TI_PLAIN_MAP_LARGE)]
    TiPlainMapLarge {
        /// The map definition.
        map_ldefn: PlainMapLTypeDefn,
    },
    // ============ Types that are mutually dependent on each other ===
    /// Type identifier for a strongly connected component.
    #[dust_dds(case=TI_STRONGLY_CONNECTED_COMPONENT)]
    TiStronglyConnectedComponent {
        /// The strongly connected component ID.
        sc_component_id: StronglyConnectedComponentId,
    },
    // ============ The remaining cases - use EquivalenceKind =========
    /// Type identifier based on the complete equivalence hash.
    #[dust_dds(case=EK_COMPLETE)]
    EkComplete {
        /// The complete equivalence hash.
        equivalence_hash: EquivalenceHash,
    },
    /// Type identifier based on the minimal equivalence hash.
    #[dust_dds(case=EK_MINIMAL)]
    EkMinimal {
        /// The minimal equivalence hash.
        equivalence_hash: EquivalenceHash,
    },
    // =================== Future extensibility ============
    /// Default case for future extensibility.
    #[dust_dds(default)]
    Default {
        /// The extended type information.
        extended_type: MinimalExtendedType,
    },
}

impl Default for TypeIdentifier {
    fn default() -> Self {
        TypeIdentifier::Default {
            extended_type: Default::default(),
        }
    }
}

/// Sequence of type identifiers.
pub type TypeIdentifierSeq = Vec<TypeIdentifier>;

// --- Annotation usage: -----------------------------------------------
/// Unique ID of a type member.
pub type MemberId = u32;
/// Maximum length of an annotation string value.
pub const ANNOTATION_STR_VALUE_MAX_LEN: u32 = 128;
/// Maximum length of an annotation octet sequence value.
pub const ANNOTATION_OCTETSEC_VALUE_MAX_LEN: u32 = 128;

/// Structure representing future extensions to annotation parameter values.
#[derive(DdsType, Debug, Clone, PartialEq)]
#[dust_dds(extensibility = "mutable", nested)]
pub struct ExtendedAnnotationParameterValue {
    // Empty. Available for future extension
}

/// Literal value of an annotation member.
#[derive(DdsType, Debug, Clone, PartialEq)]
#[dust_dds(extensibility = "final", nested, switch(u8))]
pub enum AnnotationParameterValue {
    /// Boolean value.
    #[dust_dds(case=TK_BOOLEAN)]
    TkBoolean {
        /// The boolean value.
        boolean_value: bool,
    },
    /// Byte value.
    #[dust_dds(case=TK_BYTE)]
    TkByte {
        /// The byte value.
        byte_value: u8,
    },
    /// Signed 8-bit integer value.
    #[dust_dds(case=TK_INT8)]
    TkInt8 {
        /// The signed 8-bit integer value.
        int8_value: i8,
    },
    /// Unsigned 8-bit integer value.
    #[dust_dds(case=TK_UINT8)]
    TkUint8 {
        /// The unsigned 8-bit integer value.
        uint8_value: u8,
    },
    /// Signed 16-bit integer value.
    #[dust_dds(case=TK_INT16)]
    TkInt16 {
        /// The signed 16-bit integer value.
        int16_value: i16,
    },
    /// Unsigned 16-bit integer value.
    #[dust_dds(case=TK_UINT16)]
    TkUint16 {
        /// The unsigned 16-bit integer value.
        uint_16_value: u16,
    },
    /// Signed 32-bit integer value.
    #[dust_dds(case=TK_INT32)]
    TkInt32 {
        /// The signed 32-bit integer value.
        int32_value: i32,
    },
    /// Unsigned 32-bit integer value.
    #[dust_dds(case=TK_UINT32)]
    TkUint32 {
        /// The unsigned 32-bit integer value.
        uint32_value: u32,
    },
    /// Signed 64-bit integer value.
    #[dust_dds(case=TK_INT64)]
    TkInt64 {
        /// The signed 64-bit integer value.
        int64_value: i64,
    },
    /// Unsigned 64-bit integer value.
    #[dust_dds(case=TK_UINT64)]
    TkUint64 {
        /// The unsigned 64-bit integer value.
        uint64_value: u64,
    },
    /// Single-precision floating-point value.
    #[dust_dds(case=TK_FLOAT32)]
    TkFloat32 {
        /// The single-precision floating-point value.
        float32_value: f32,
    },
    /// Double-precision floating-point value.
    #[dust_dds(case=TK_FLOAT64)]
    TkFloat64 {
        /// The double-precision floating-point value.
        float64_value: f64,
    },
    // TkFloat128 {
    //     float128_value: i128,
    // },
    /// 8-bit character value.
    #[dust_dds(case=TK_CHAR8)]
    TkChar8 {
        /// The 8-bit character value.
        char_value: char,
    },
    // TypeKind::CHAR16 {
    // wchar_value: char16},
    /// Enumerated value represented as an integer.
    #[dust_dds(case=TK_ENUM)]
    TkEnum {
        /// The enumerated value.
        enumerated_value: i32,
    },
    /// 8-bit string value.
    #[dust_dds(case=TK_STRING8)]
    TkString8 {
        /// The string value.
        string8_value: String, /*string<ANNOTATION_STR_VALUE_MAX_LEN>  */
    },
    // TypeKind::STRING16:
    // wstring<ANNOTATION_STR_VALUE_MAX_LEN> string16_value;
    /// Default case for future extensibility.
    #[dust_dds(default)]
    Default {
        /// The extended value.
        extended_value: ExtendedAnnotationParameterValue,
    },
}

/// The application of an annotation parameter to a member.
#[derive(DdsType, Debug, Clone, PartialEq)]
#[dust_dds(extensibility = "appendable", nested)]
pub struct AppliedAnnotationParameter {
    /// Hash of the parameter name.
    pub paramname_hash: NameHash,
    /// Value of the annotation parameter.
    pub value: AnnotationParameterValue,
}

/// Sequence of applied annotation parameters.
pub type AppliedAnnotationParameterSeq = Vec<AppliedAnnotationParameter>;

/// The application of an annotation to a type or type member.
#[derive(DdsType, Debug, Clone, PartialEq)]
#[dust_dds(extensibility = "appendable", nested)]
pub struct AppliedAnnotation {
    /// The type identifier of the annotation.
    pub annotation_typeid: TypeIdentifier,
    /// Sequence of parameters applied to the annotation.
    #[dust_dds(optional)]
    pub param_seq: Option<AppliedAnnotationParameterSeq>,
}
/// Sequence of applied annotations.
pub type AppliedAnnotationSeq = Vec<AppliedAnnotation>;
/// Verbatim annotation representation.
#[derive(DdsType, Debug, Clone, PartialEq)]
#[dust_dds(extensibility = "final", nested)]
pub struct AppliedVerbatimAnnotation {
    /// Where the verbatim text should be placed.
    pub placement: String, //string<32>
    /// The language/format of the verbatim text.
    pub language: String, //string<32>
    /// The verbatim text content.
    pub text: String,
}

// --- Aggregate types: ------------------------------------------------
/// Applied builtin annotations for a member (e.g., unit, min, max, hash_id).
#[derive(DdsType, Debug, Clone, PartialEq)]
#[dust_dds(extensibility = "appendable", nested)]
pub struct AppliedBuiltinMemberAnnotations {
    /// The unit associated with the member.
    #[dust_dds(optional)]
    pub unit: Option<String>, // @unit("<unit>")
    /// The minimum value allowed for the member.
    #[dust_dds(optional)]
    pub min: Option<AnnotationParameterValue>, // @min , @range
    /// The maximum value allowed for the member.
    #[dust_dds(optional)]
    pub max: Option<AnnotationParameterValue>, // @max , @range
    /// The hash ID of the member name.
    #[dust_dds(optional)]
    pub hash_id: Option<String>, // @hashid("<membername>")
}

/// Common properties for a struct member.
#[derive(DdsType, Debug, Clone, PartialEq)]
#[dust_dds(extensibility = "final", nested)]
pub struct CommonStructMember {
    /// The unique ID of the member.
    pub member_id: MemberId,
    /// Flags associated with the structure member.
    pub member_flags: StructMemberFlag,
    /// The TypeIdentifier of the member type.
    pub member_type_id: TypeIdentifier,
}

/// Complete details of a member in a complete TypeObject representation.
#[derive(DdsType, Debug, Clone, PartialEq)]
#[dust_dds(extensibility = "final", nested)]
pub struct CompleteMemberDetail {
    /// The name of the member.
    pub name: MemberName,
    /// Builtin annotations applied to the member.
    #[dust_dds(optional)]
    pub ann_builtin: Option<AppliedBuiltinMemberAnnotations>,
    /// Custom annotations applied to the member.
    #[dust_dds(optional)]
    pub ann_custom: Option<AppliedAnnotationSeq>,
}
/// Minimal details of a member in a minimal TypeObject representation.
#[derive(DdsType, Debug, Clone, PartialEq)]
#[dust_dds(extensibility = "final", nested)]
pub struct MinimalMemberDetail {
    /// The hash of the member name.
    pub name_hash: NameHash,
}

/// Complete structure representing a struct member.
#[derive(DdsType, Debug, Clone, PartialEq)]
#[dust_dds(extensibility = "appendable", nested)]
pub struct CompleteStructMember {
    /// Common member properties.
    pub common: CommonStructMember,
    /// Detailed complete properties (name, annotations).
    pub detail: CompleteMemberDetail,
}
/// Sequence of complete struct members.
pub type CompleteStructMemberSeq = Vec<CompleteStructMember>;
/// Minimal structure representing a struct member.
#[derive(DdsType, Debug, Clone, PartialEq)]
#[dust_dds(extensibility = "appendable", nested)]
pub struct MinimalStructMember {
    /// Common member properties.
    pub common: CommonStructMember,
    /// Detailed minimal properties (name hash).
    pub detail: MinimalMemberDetail,
}
/// Sequence of minimal struct members.
pub type MinimalStructMemberSeq = Vec<MinimalStructMember>;
/// Builtin annotations applied to a type definition.
#[derive(DdsType, Debug, Clone, PartialEq)]
#[dust_dds(extensibility = "appendable", nested)]
pub struct AppliedBuiltinTypeAnnotations {
    /// Verbatim annotation applied to the type.
    #[dust_dds(optional)]
    pub verbatim: Option<AppliedVerbatimAnnotation>, // @verbatim(...)
}

/// Minimal details of a type definition.
#[derive(DdsType, Debug, Clone, PartialEq)]
#[dust_dds(extensibility = "final", nested)]
pub struct MinimalTypeDetail {
    // Empty. Available for future extension
}

/// Complete details of a type definition.
#[derive(DdsType, Debug, Clone, PartialEq)]
#[dust_dds(extensibility = "final", nested)]
pub struct CompleteTypeDetail {
    /// Builtin annotations applied to the type.
    #[dust_dds(optional)]
    pub ann_builtin: Option<AppliedBuiltinTypeAnnotations>,
    /// Custom annotations applied to the type.
    #[dust_dds(optional)]
    pub ann_custom: Option<AppliedAnnotationSeq>,
    /// The qualified name of the type.
    pub type_name: QualifiedTypeName,
}

/// Header for a complete struct type representation.
#[derive(DdsType, Debug, Clone, PartialEq)]
#[dust_dds(extensibility = "appendable", nested)]
pub struct CompleteStructHeader {
    /// The TypeIdentifier of the base type, if any.
    pub base_type: TypeIdentifier,
    /// Detailed complete properties of the struct type.
    pub detail: CompleteTypeDetail,
}

/// Header for a minimal struct type representation.
#[derive(DdsType, Debug, Clone, PartialEq)]
#[dust_dds(extensibility = "appendable", nested)]
pub struct MinimalStructHeader {
    /// The TypeIdentifier of the base type, if any.
    pub base_type: TypeIdentifier,
    /// Detailed minimal properties of the struct type.
    pub detail: MinimalTypeDetail,
}

/// Complete struct type representation.
#[derive(DdsType, Debug, Clone, PartialEq)]
#[dust_dds(extensibility = "final", nested)]
pub struct CompleteStructType {
    /// Flags associated with the structure type.
    pub struct_flags: StructTypeFlag,
    /// The header containing base type and details.
    pub header: CompleteStructHeader,
    /// The sequence of members in the structure.
    pub member_seq: CompleteStructMemberSeq,
}

/// Minimal struct type representation.
#[derive(DdsType, Debug, Clone, PartialEq)]
#[dust_dds(extensibility = "final", nested)]
pub struct MinimalStructType {
    /// Flags associated with the structure type.
    pub struct_flags: StructTypeFlag,
    /// The header containing base type and details.
    pub header: MinimalStructHeader,
    /// The sequence of members in the structure.
    pub member_seq: MinimalStructMemberSeq,
}

// --- Union: ----------------------------------------------------------
/// Sequence of union case labels.
pub type UnionCaseLabelSeq = Vec<i32>;

/// Common properties of a union member.
#[derive(DdsType, Debug, Clone, PartialEq)]
#[dust_dds(extensibility = "final", nested)]
pub struct CommonUnionMember {
    /// The unique ID of the member.
    pub member_id: MemberId,
    /// Flags associated with the union member.
    pub member_flags: UnionMemberFlag,
    /// The TypeIdentifier of the member type.
    pub type_id: TypeIdentifier,
    /// The case labels associated with this union member.
    pub label_seq: UnionCaseLabelSeq,
}

impl From<&DynamicTypeMember> for CommonUnionMember {
    fn from(value: &DynamicTypeMember) -> Self {
        let mut member_flags = MemberFlag::from(value.descriptor.try_construct_kind);
        if value.descriptor.is_key {
            member_flags |= MEMBER_FLAG_IS_KEY;
        }
        if value.descriptor.is_default_label {
            member_flags |= MEMBER_FLAG_IS_DEFAULT;
        }
        if value.descriptor.is_external {
            member_flags |= MEMBER_FLAG_IS_EXTERNAL;
        }

        CommonUnionMember {
            member_id: value.get_id(),
            member_flags,
            type_id: (&value.descriptor.r#type).into(),
            label_seq: value.descriptor.label.to_vec(),
        }
    }
}

/// Complete details of a union member in a complete TypeObject representation.
#[derive(DdsType, Debug, Clone, PartialEq)]
#[dust_dds(extensibility = "appendable", nested)]
pub struct CompleteUnionMember {
    /// Common union member properties.
    pub common: CommonUnionMember,
    /// Detailed complete properties (name, annotations) of the union member.
    pub detail: CompleteMemberDetail,
}

impl From<&DynamicTypeMember> for CompleteUnionMember {
    fn from(value: &DynamicTypeMember) -> Self {
        let common = value.into();
        let detail = CompleteMemberDetail {
            name: String::from(value.descriptor.name),
            ann_builtin: None,
            ann_custom: None,
        };
        CompleteUnionMember { common, detail }
    }
}

/// Sequence of complete union members.
pub type CompleteUnionMemberSeq = Vec<CompleteUnionMember>;

/// Minimal details of a union member in a minimal TypeObject representation.
#[derive(DdsType, Debug, Clone, PartialEq)]
#[dust_dds(extensibility = "appendable", nested)]
pub struct MinimalUnionMember {
    /// Common union member properties.
    pub common: CommonUnionMember,
    /// Detailed minimal properties (name hash) of the union member.
    pub detail: MinimalMemberDetail,
}

impl From<&DynamicTypeMember> for MinimalUnionMember {
    fn from(value: &DynamicTypeMember) -> Self {
        let common = value.into();
        let name_hash = <[u8; 16]>::from(md5::compute(value.get_name().as_bytes()));
        let detail = MinimalMemberDetail {
            name_hash: [name_hash[0], name_hash[1], name_hash[2], name_hash[3]],
        };
        MinimalUnionMember { common, detail }
    }
}

/// Sequence of minimal union members.
pub type MinimalUnionMemberSeq = Vec<MinimalUnionMember>;

/// Common properties of a union discriminator member.
#[derive(DdsType, Debug, Clone, PartialEq)]
#[dust_dds(extensibility = "final", nested)]
pub struct CommonDiscriminatorMember {
    /// Flags associated with the union discriminator.
    pub member_flags: UnionDiscriminatorFlag,
    /// The TypeIdentifier of the discriminator type.
    pub type_id: TypeIdentifier,
}
/// Complete details of a union discriminator member in a complete TypeObject representation.
#[derive(DdsType, Debug, Clone, PartialEq)]
#[dust_dds(extensibility = "appendable", nested)]
pub struct CompleteDiscriminatorMember {
    /// Common discriminator properties.
    pub common: CommonDiscriminatorMember,
    /// Builtin annotations applied to the discriminator.
    #[dust_dds(optional)]
    pub ann_builtin: Option<AppliedBuiltinTypeAnnotations>,
    /// Custom annotations applied to the discriminator.
    #[dust_dds(optional)]
    pub ann_custom: Option<AppliedAnnotationSeq>,
}
/// Minimal details of a union discriminator member in a minimal TypeObject representation.
#[derive(DdsType, Debug, Clone, PartialEq)]
#[dust_dds(extensibility = "appendable", nested)]
pub struct MinimalDiscriminatorMember {
    /// Common discriminator properties.
    pub common: CommonDiscriminatorMember,
}

/// Header for a complete union type representation.
#[derive(DdsType, Debug, Clone, PartialEq)]
#[dust_dds(extensibility = "appendable", nested)]
pub struct CompleteUnionHeader {
    /// Detailed complete properties of the union type.
    pub detail: CompleteTypeDetail,
}

/// Header for a minimal union type representation.
#[derive(DdsType, Debug, Clone, PartialEq)]
#[dust_dds(extensibility = "appendable", nested)]
pub struct MinimalUnionHeader {
    /// Detailed minimal properties of the union type.
    pub detail: MinimalTypeDetail,
}

/// Complete union type representation.
#[derive(DdsType, Debug, Clone, PartialEq)]
#[dust_dds(extensibility = "final", nested)]
pub struct CompleteUnionType {
    /// Flags associated with the union type.
    pub union_flags: UnionTypeFlag,
    /// The header containing details.
    pub header: CompleteUnionHeader,
    /// The discriminator member details.
    pub discriminator: CompleteDiscriminatorMember,
    /// The sequence of union members.
    pub member_seq: CompleteUnionMemberSeq,
}

/// Minimal union type representation.
#[derive(DdsType, Debug, Clone, PartialEq)]
#[dust_dds(extensibility = "final", nested)]
pub struct MinimalUnionType {
    /// Flags associated with the union type.
    pub union_flags: UnionTypeFlag,
    /// The header containing details.
    pub header: MinimalUnionHeader,
    /// The discriminator member details.
    pub discriminator: MinimalDiscriminatorMember,
    /// The sequence of union members.
    pub member_seq: MinimalUnionMemberSeq,
}

// --- Annotation: ----------------------------------------------------
/// Common properties of an annotation parameter.
#[derive(DdsType, Debug, Clone, PartialEq)]
#[dust_dds(extensibility = "final", nested)]
pub struct CommonAnnotationParameter {
    /// Flags associated with the annotation parameter.
    pub member_flags: AnnotationParameterFlag,
    /// The TypeIdentifier of the parameter type.
    pub member_type_id: TypeIdentifier,
}

/// Complete details of an annotation parameter in a complete TypeObject representation.
#[derive(DdsType, Debug, Clone, PartialEq)]
#[dust_dds(extensibility = "appendable", nested)]
pub struct CompleteAnnotationParameter {
    /// Common annotation parameter properties.
    pub common: CommonAnnotationParameter,
    /// The name of the parameter.
    pub name: MemberName,
    /// The default value of the parameter.
    pub default_value: AnnotationParameterValue,
}
/// Sequence of complete annotation parameters.
pub type CompleteAnnotationParameterSeq = Vec<CompleteAnnotationParameter>;

/// Minimal details of an annotation parameter in a minimal TypeObject representation.
#[derive(DdsType, Debug, Clone, PartialEq)]
#[dust_dds(extensibility = "appendable", nested)]
pub struct MinimalAnnotationParameter {
    /// Common annotation parameter properties.
    pub common: CommonAnnotationParameter,
    /// The hash of the parameter name.
    pub name_hash: NameHash,
    /// The default value of the parameter.
    pub default_value: AnnotationParameterValue,
}
/// Sequence of minimal annotation parameters.
pub type MinimalAnnotationParameterSeq = Vec<MinimalAnnotationParameter>;
/// Header for a complete annotation type representation.
#[derive(DdsType, Debug, Clone, PartialEq)]
#[dust_dds(extensibility = "appendable", nested)]
pub struct CompleteAnnotationHeader {
    /// The qualified name of the annotation type.
    pub annotation_name: QualifiedTypeName,
}

/// Header for a minimal annotation type representation.
#[derive(DdsType, Debug, Clone, PartialEq)]
#[dust_dds(extensibility = "appendable", nested)]
pub struct MinimalAnnotationHeader {
    // Empty. Available for future extension
}

/// Complete annotation type representation.
#[derive(DdsType, Debug, Clone, PartialEq)]
#[dust_dds(extensibility = "final", nested)]
pub struct CompleteAnnotationType {
    /// Flags associated with the annotation type.
    pub annotation_flag: AnnotationTypeFlag,
    /// The header containing annotation name.
    pub header: CompleteAnnotationHeader,
    /// The sequence of annotation parameters.
    pub member_seq: CompleteAnnotationParameterSeq,
}

/// Minimal annotation type representation.
#[derive(DdsType, Debug, Clone, PartialEq)]
#[dust_dds(extensibility = "final", nested)]
pub struct MinimalAnnotationType {
    /// Flags associated with the annotation type.
    pub annotation_flag: AnnotationTypeFlag,
    /// The header containing details.
    pub header: MinimalAnnotationHeader,
    /// The sequence of annotation parameters.
    pub member_seq: MinimalAnnotationParameterSeq,
}

// --- Alias: ----------------------------------------------------------
/// Common properties of an alias type.
#[derive(DdsType, Debug, Clone, PartialEq)]
#[dust_dds(extensibility = "final", nested)]
pub struct CommonAliasBody {
    /// Flags associated with the related type.
    pub related_flags: AliasMemberFlag,
    /// The TypeIdentifier of the related type.
    pub related_type: TypeIdentifier,
}

/// Complete body of an alias in a complete TypeObject representation.
#[derive(DdsType, Debug, Clone, PartialEq)]
#[dust_dds(extensibility = "appendable", nested)]
pub struct CompleteAliasBody {
    /// Common alias properties.
    pub common: CommonAliasBody,
    /// Builtin annotations applied to the alias.
    #[dust_dds(optional)]
    pub ann_builtin: Option<AppliedBuiltinMemberAnnotations>,
    /// Custom annotations applied to the alias.
    #[dust_dds(optional)]
    pub ann_custom: Option<AppliedAnnotationSeq>,
}

/// Minimal body of an alias in a minimal TypeObject representation.
#[derive(DdsType, Debug, Clone, PartialEq)]
#[dust_dds(extensibility = "appendable", nested)]
pub struct MinimalAliasBody {
    /// Common alias properties.
    pub common: CommonAliasBody,
}

/// Header for a complete alias type representation.
#[derive(DdsType, Debug, Clone, PartialEq)]
#[dust_dds(extensibility = "appendable", nested)]
pub struct CompleteAliasHeader {
    /// Detailed complete properties of the alias type.
    pub detail: CompleteTypeDetail,
}

/// Header for a minimal alias type representation.
#[derive(DdsType, Debug, Clone, PartialEq)]
#[dust_dds(extensibility = "appendable", nested)]
pub struct MinimalAliasHeader {
    // Empty. Available for future extension
}

/// Complete alias type representation.
#[derive(DdsType, Debug, Clone, PartialEq)]
#[dust_dds(extensibility = "final", nested)]
pub struct CompleteAliasType {
    /// Flags associated with the alias type.
    pub alias_flags: AliasTypeFlag,
    /// The header containing details.
    pub header: CompleteAliasHeader,
    /// The alias body containing the related type.
    pub body: CompleteAliasBody,
}

/// Minimal alias type representation.
#[derive(DdsType, Debug, Clone, PartialEq)]
#[dust_dds(extensibility = "final", nested)]
pub struct MinimalAliasType {
    /// Flags associated with the alias type.
    pub alias_flags: AliasTypeFlag,
    /// The header containing details.
    pub header: MinimalAliasHeader,
    /// The alias body containing the related type.
    pub body: MinimalAliasBody,
}

// --- Collections: ----------------------------------------------------
/// Complete details of a collection element in a complete TypeObject representation.
#[derive(DdsType, Debug, Clone, PartialEq)]
#[dust_dds(extensibility = "final", nested)]
pub struct CompleteElementDetail {
    /// Builtin annotations applied to the collection element.
    #[dust_dds(optional)]
    pub ann_builtin: Option<AppliedBuiltinMemberAnnotations>,
    /// Custom annotations applied to the collection element.
    #[dust_dds(optional)]
    pub ann_custom: Option<AppliedAnnotationSeq>,
}

/// Common properties of a collection element.
#[derive(DdsType, Debug, Clone, PartialEq)]
#[dust_dds(extensibility = "final", nested)]
pub struct CommonCollectionElement {
    /// Flags associated with the collection element.
    pub element_flags: CollectionElementFlag,
    /// The TypeIdentifier of the element type.
    pub _type: TypeIdentifier,
}

/// Complete details of a collection element.
#[derive(DdsType, Debug, Clone, PartialEq)]
#[dust_dds(extensibility = "appendable", nested)]
pub struct CompleteCollectionElement {
    /// Common collection element properties.
    pub common: CommonCollectionElement,
    /// Detailed complete properties (annotations) of the element.
    pub detail: CompleteElementDetail,
}

/// Minimal details of a collection element.
#[derive(DdsType, Debug, Clone, PartialEq)]
#[dust_dds(extensibility = "appendable", nested)]
pub struct MinimalCollectionElement {
    /// Common collection element properties.
    pub common: CommonCollectionElement,
}

/// Common header for collection types.
#[derive(DdsType, Debug, Clone, PartialEq)]
#[dust_dds(extensibility = "final", nested)]
pub struct CommonCollectionHeader {
    /// The bound (maximum number of elements) of the collection.
    pub bound: LBound,
}

/// Header for a complete collection representation.
#[derive(DdsType, Debug, Clone, PartialEq)]
#[dust_dds(extensibility = "appendable", nested)]
pub struct CompleteCollectionHeader {
    /// Common collection header properties.
    pub common: CommonCollectionHeader,
    /// Detailed complete properties of the collection.
    #[dust_dds(optional)]
    pub detail: Option<CompleteTypeDetail>, // not present for anonymous
}

/// Header for a minimal collection representation.
#[derive(DdsType, Debug, Clone, PartialEq)]
#[dust_dds(extensibility = "appendable", nested)]
pub struct MinimalCollectionHeader {
    /// Common collection header properties.
    pub common: CommonCollectionHeader,
}

// --- Sequence: ------------------------------------------------------
/// Complete sequence type representation.
#[derive(DdsType, Debug, Clone, PartialEq)]
#[dust_dds(extensibility = "final", nested)]
pub struct CompleteSequenceType {
    /// Flags associated with the sequence collection type.
    pub collection_flag: CollectionTypeFlag,
    /// The header containing sequence properties.
    pub header: CompleteCollectionHeader,
    /// The details of the sequence elements.
    pub element: CompleteCollectionElement,
}

/// Minimal sequence type representation.
#[derive(DdsType, Debug, Clone, PartialEq)]
#[dust_dds(extensibility = "final", nested)]
pub struct MinimalSequenceType {
    /// Flags associated with the sequence collection type.
    pub collection_flag: CollectionTypeFlag,
    /// The header containing sequence properties.
    pub header: MinimalCollectionHeader,
    /// The details of the sequence elements.
    pub element: MinimalCollectionElement,
}

// --- Array: ------------------------------------------------------
/// Common header for array types.
#[derive(DdsType, Debug, Clone, PartialEq)]
#[dust_dds(extensibility = "final", nested)]
pub struct CommonArrayHeader {
    /// The sequence of bounds for each dimension.
    pub bound_seq: LBoundSeq,
}

/// Header for a complete array representation.
#[derive(DdsType, Debug, Clone, PartialEq)]
#[dust_dds(extensibility = "appendable", nested)]
pub struct CompleteArrayHeader {
    /// Common array header properties.
    pub common: CommonArrayHeader,
    /// Detailed complete properties of the array.
    pub detail: CompleteTypeDetail,
}

/// Header for a minimal array representation.
#[derive(DdsType, Debug, Clone, PartialEq)]
#[dust_dds(extensibility = "appendable", nested)]
pub struct MinimalArrayHeader {
    /// Common array header properties.
    pub common: CommonArrayHeader,
}

/// Complete array type representation.
#[derive(DdsType, Debug, Clone, PartialEq)]
#[dust_dds(extensibility = "appendable", nested)]
pub struct CompleteArrayType {
    /// Flags associated with the array collection type.
    pub collection_flag: CollectionTypeFlag,
    /// The header containing array properties.
    pub header: CompleteArrayHeader,
    /// The details of the array elements.
    pub element: CompleteCollectionElement,
}

/// Minimal array type representation.
#[derive(DdsType, Debug, Clone, PartialEq)]
#[dust_dds(extensibility = "final", nested)]
pub struct MinimalArrayType {
    /// Flags associated with the array collection type.
    pub collection_flag: CollectionTypeFlag,
    /// The header containing array properties.
    pub header: MinimalArrayHeader,
    /// The details of the array elements.
    pub element: MinimalCollectionElement,
}

// --- Map: ------------------------------------------------------
/// Complete map type representation.
#[derive(DdsType, Debug, Clone, PartialEq)]
#[dust_dds(extensibility = "final", nested)]
pub struct CompleteMapType {
    /// Flags associated with the map collection type.
    pub collection_flag: CollectionTypeFlag,
    /// The header containing map properties.
    pub header: CompleteCollectionHeader,
    /// The details of the map keys.
    pub key: CompleteCollectionElement,
    /// The details of the map elements.
    pub element: CompleteCollectionElement,
}

/// Minimal map type representation.
#[derive(DdsType, Debug, Clone, PartialEq)]
#[dust_dds(extensibility = "final", nested)]
pub struct MinimalMapType {
    /// Flags associated with the map collection type.
    pub collection_flag: CollectionTypeFlag,
    /// The header containing map properties.
    pub header: MinimalCollectionHeader,
    /// The details of the map keys.
    pub key: MinimalCollectionElement,
    /// The details of the map elements.
    pub element: MinimalCollectionElement,
}

// --- Enumeration: ----------------------------------------------------
/// The bit bound (e.g. 8, 16, 32) of an enumerated type or bitmask.
pub type BitBound = u16;
// Constant in an enumerated type
/// Common properties of an enumerated literal.
#[derive(DdsType, Debug, Clone, PartialEq)]
#[dust_dds(extensibility = "appendable", nested)]
pub struct CommonEnumeratedLiteral {
    /// The integer value of the enumerated literal.
    pub value: i32,
    /// Flags associated with the enumerated literal.
    pub flags: EnumeratedLiteralFlag,
}

// Constant in an enumerated type
/// Complete details of an enumerated literal.
#[derive(DdsType, Debug, Clone, PartialEq)]
#[dust_dds(extensibility = "appendable", nested)]
pub struct CompleteEnumeratedLiteral {
    /// Common enumerated literal properties.
    pub common: CommonEnumeratedLiteral,
    /// Detailed complete properties of the literal.
    pub detail: CompleteMemberDetail,
}

/// Sequence of complete enumerated literals.
pub type CompleteEnumeratedLiteralSeq = Vec<CompleteEnumeratedLiteral>;

// Constant in an enumerated type
/// Minimal details of an enumerated literal.
#[derive(DdsType, Debug, Clone, PartialEq)]
#[dust_dds(extensibility = "appendable", nested)]
pub struct MinimalEnumeratedLiteral {
    /// Common enumerated literal properties.
    pub common: CommonEnumeratedLiteral,
    /// Detailed minimal properties of the literal.
    pub detail: MinimalMemberDetail,
}

/// Sequence of minimal enumerated literals.
pub type MinimalEnumeratedLiteralSeq = Vec<MinimalEnumeratedLiteral>;

/// Common header for enumerated types.
#[derive(DdsType, Debug, Clone, PartialEq)]
#[dust_dds(extensibility = "final", nested)]
pub struct CommonEnumeratedHeader {
    /// The bit bound of the enumerated type.
    pub bit_bound: BitBound,
}

/// Header for a complete enumerated type.
#[derive(DdsType, Debug, Clone, PartialEq)]
#[dust_dds(extensibility = "appendable", nested)]
pub struct CompleteEnumeratedHeader {
    /// Common enumerated header properties.
    pub common: CommonEnumeratedHeader,
    /// Detailed complete properties of the enumerated type.
    pub detail: CompleteTypeDetail,
}

/// Header for a minimal enumerated type.
#[derive(DdsType, Debug, Clone, PartialEq)]
#[dust_dds(extensibility = "appendable", nested)]
pub struct MinimalEnumeratedHeader {
    /// Common enumerated header properties.
    pub common: CommonEnumeratedHeader,
}

// Enumerated type
/// Complete enumerated type representation.
#[derive(DdsType, Debug, Clone, PartialEq)]
#[dust_dds(extensibility = "final", nested)]
pub struct CompleteEnumeratedType {
    /// Flags associated with the enumerated type.
    pub enum_flags: EnumTypeFlag, // unused
    /// The header containing details.
    pub header: CompleteEnumeratedHeader,
    /// The sequence of literals in the enumeration.
    pub literal_seq: CompleteEnumeratedLiteralSeq,
}

// Enumerated type
/// Minimal enumerated type representation.
#[derive(DdsType, Debug, Clone, PartialEq)]
#[dust_dds(extensibility = "final", nested)]
pub struct MinimalEnumeratedType {
    /// Flags associated with the enumerated type.
    pub enum_flags: EnumTypeFlag, // unused
    /// The header containing details.
    pub header: MinimalEnumeratedHeader,
    /// The sequence of literals in the enumeration.
    pub literal_seq: MinimalEnumeratedLiteralSeq,
}

// --- Bitmask: --------------------------------------------------------
// Bit in a bit mask
/// Common properties of a bitflag inside a bitmask.
#[derive(DdsType, Debug, Clone, PartialEq)]
#[dust_dds(extensibility = "final", nested)]
pub struct CommonBitflag {
    /// The position (0-based) of the bit flag.
    pub position: u16,
    /// Flags associated with the bit flag.
    pub flags: BitflagFlag,
}

/// Complete details of a bitflag inside a bitmask.
#[derive(DdsType, Debug, Clone, PartialEq)]
#[dust_dds(extensibility = "appendable", nested)]
pub struct CompleteBitflag {
    /// Common bitflag properties.
    pub common: CommonBitflag,
    /// Detailed complete properties of the bitflag.
    pub detail: CompleteMemberDetail,
}
/// Sequence of complete bitflags.
pub type CompleteBitflagSeq = Vec<CompleteBitflag>;

/// Minimal details of a bitflag inside a bitmask.
#[derive(DdsType, Debug, Clone, PartialEq)]
#[dust_dds(extensibility = "appendable", nested)]
pub struct MinimalBitflag {
    /// Common bitflag properties.
    pub common: CommonBitflag,
    /// Detailed minimal properties of the bitflag.
    pub detail: MinimalMemberDetail,
}
/// Sequence of minimal bitflags.
pub type MinimalBitflagSeq = Vec<MinimalBitflag>;

/// Common header for bitmask types.
#[derive(DdsType, Debug, Clone, PartialEq)]
#[dust_dds(extensibility = "final", nested)]
pub struct CommonBitmaskHeader {
    /// The bit bound of the bitmask type.
    pub bit_bound: BitBound,
}
/// Header for a complete bitmask type representation.
pub type CompleteBitmaskHeader = CompleteEnumeratedHeader;
/// Header for a minimal bitmask type representation.
pub type MinimalBitmaskHeader = MinimalEnumeratedHeader;

/// Complete bitmask type representation.
#[derive(DdsType, Debug, Clone, PartialEq)]
#[dust_dds(extensibility = "appendable", nested)]
pub struct CompleteBitmaskType {
    /// Flags associated with the bitmask type.
    pub bitmask_flags: BitmaskTypeFlag, // unused
    /// The header containing details.
    pub header: CompleteBitmaskHeader,
    /// The sequence of bitflags.
    pub flag_seq: CompleteBitflagSeq,
}

/// Minimal bitmask type representation.
#[derive(DdsType, Debug, Clone, PartialEq)]
#[dust_dds(extensibility = "appendable", nested)]
pub struct MinimalBitmaskType {
    /// Flags associated with the bitmask type.
    pub bitmask_flags: BitmaskTypeFlag, // unused
    /// The header containing details.
    pub header: MinimalBitmaskHeader,
    /// The sequence of bitflags.
    pub flag_seq: MinimalBitflagSeq,
}

// --- Bitset: ----------------------------------------------------------
/// Common properties of a bitfield inside a bitset.
#[derive(DdsType, Debug, Clone, PartialEq)]
#[dust_dds(extensibility = "final", nested)]
pub struct CommonBitfield {
    /// The position (0-based) of the bitfield.
    pub position: u16,
    /// Flags associated with the bitfield.
    pub flags: BitsetMemberFlag,
    /// The number of bits used by the bitfield.
    pub bitcount: u8,
    /// The TypeKind of the primitive integer type holding this bitfield.
    pub holder_type: u8, // Original in IDL: TypeKind which for us is a Rust enum but in IDL is an octet // Must be primitive integer type
}

/// Complete details of a bitfield inside a bitset.
#[derive(DdsType, Debug, Clone, PartialEq)]
#[dust_dds(extensibility = "appendable", nested)]
pub struct CompleteBitfield {
    /// Common bitfield properties.
    pub common: CommonBitfield,
    /// Detailed complete properties of the bitfield.
    pub detail: CompleteMemberDetail,
}

/// Sequence of complete bitfields.
pub type CompleteBitfieldSeq = Vec<CompleteBitfield>;

/// Minimal details of a bitfield inside a bitset.
#[derive(DdsType, Debug, Clone, PartialEq)]
#[dust_dds(extensibility = "appendable", nested)]
pub struct MinimalBitfield {
    /// Common bitfield properties.
    pub common: CommonBitfield,
    /// The hash of the bitfield name.
    pub name_hash: NameHash,
}

/// Sequence of minimal bitfields.
pub type MinimalBitfieldSeq = Vec<MinimalBitfield>;

/// Header for a complete bitset type representation.
#[derive(DdsType, Debug, Clone, PartialEq)]
#[dust_dds(extensibility = "appendable", nested)]
pub struct CompleteBitsetHeader {
    /// Detailed complete properties of the bitset.
    pub detail: CompleteTypeDetail,
}

/// Header for a minimal bitset type representation.
#[derive(DdsType, Debug, Clone, PartialEq)]
#[dust_dds(extensibility = "appendable", nested)]
pub struct MinimalBitsetHeader {
    // Empty. Available for future extension
}

/// Complete bitset type representation.
#[derive(DdsType, Debug, Clone, PartialEq)]
#[dust_dds(extensibility = "appendable", nested)]
pub struct CompleteBitsetType {
    /// Flags associated with the bitset type.
    pub bitset_flags: BitsetTypeFlag, // unused
    /// The header containing details.
    pub header: CompleteBitsetHeader,
    /// The sequence of bitfields in the bitset.
    pub field_seq: CompleteBitfieldSeq,
}

/// Minimal bitset type representation.
#[derive(DdsType, Debug, Clone, PartialEq)]
#[dust_dds(extensibility = "appendable", nested)]
pub struct MinimalBitsetType {
    /// Flags associated with the bitset type.
    pub bitset_flags: BitsetTypeFlag, // unused
    /// The header containing details.
    pub header: MinimalBitsetHeader,
    /// The sequence of bitfields in the bitset.
    pub field_seq: MinimalBitfieldSeq,
}

// --- Type Object: ---------------------------------------------------
// The types associated with each selection must have extensibility
// kind APPENDABLE or MUTABLE so that they can be extended in the future
/// Complete extended type representation for future extensions.
#[derive(DdsType, Debug, Clone, PartialEq)]
#[dust_dds(extensibility = "mutable", nested)]
pub struct CompleteExtendedType {
    // Empty. Available for future extension
}

/// Complete TypeObject representation switchable by TypeKind.
#[derive(DdsType, Debug, Clone, PartialEq)]
#[dust_dds(extensibility = "final", nested, switch(u8))]
pub enum CompleteTypeObject {
    /// The alias type object representation.
    #[dust_dds(case = TK_ALIAS)]
    TkAlias {
        /// Detailed complete alias type properties.
        alias_type: CompleteAliasType,
    },
    /// The annotation type object representation.
    #[dust_dds(case = TK_ANNOTATION)]
    TkAnnotation {
        /// Detailed complete annotation type properties.
        annotation_type: CompleteAnnotationType,
    },
    /// The structure type object representation.
    #[dust_dds(case = TK_STRUCTURE)]
    TkStructure {
        /// Detailed complete struct type properties.
        struct_type: CompleteStructType,
    },
    /// The union type object representation.
    #[dust_dds(case = TK_UNION)]
    TkUnion {
        /// Detailed complete union type properties.
        union_type: CompleteUnionType,
    },
    /// The bitset type object representation.
    #[dust_dds(case = TK_BITSET)]
    TkBitset {
        /// Detailed complete bitset type properties.
        bitset_type: CompleteBitsetType,
    },
    /// The sequence type object representation.
    #[dust_dds(case = TK_SEQUENCE)]
    TkSequence {
        /// Detailed complete sequence type properties.
        sequence_type: CompleteSequenceType,
    },
    /// The array type object representation.
    #[dust_dds(case = TK_ARRAY)]
    TkArray {
        /// Detailed complete array type properties.
        array_type: CompleteArrayType,
    },
    /// The map type object representation.
    #[dust_dds(case = TK_MAP)]
    TkMap {
        /// Detailed complete map type properties.
        map_type: CompleteMapType,
    },
    /// The enumeration type object representation.
    #[dust_dds(case = TK_ENUM)]
    TkEnum {
        /// Detailed complete enumerated type properties.
        enumerated_type: CompleteEnumeratedType,
    },
    /// The bitmask type object representation.
    #[dust_dds(case = TK_BITMASK)]
    TkBitmask {
        /// Detailed complete bitmask type properties.
        bitmask_type: CompleteBitmaskType,
    },
    // =================== Future extensibility ============
    /// Default case for future extensibility.
    #[dust_dds(default)]
    Default {
        /// The complete extended type properties.
        extended_type: CompleteExtendedType,
    },
}

/// Minimal extended type representation for future extensions.
#[derive(DdsType, Debug, Clone, PartialEq, Eq, Default)]
#[dust_dds(extensibility = "mutable", nested)]
pub struct MinimalExtendedType {
    // Empty. Available for future extension
}

/// Minimal TypeObject representation switchable by TypeKind.
#[derive(DdsType, Debug, Clone, PartialEq)]
#[dust_dds(extensibility = "final", nested, switch(u8))]
pub enum MinimalTypeObject {
    /// The alias type object representation.
    #[dust_dds(case = TK_ALIAS)]
    TkAlias {
        /// Detailed minimal alias type properties.
        alias_type: MinimalAliasType,
    },
    /// The annotation type object representation.
    #[dust_dds(case = TK_ANNOTATION)]
    TkAnnotation {
        /// Detailed minimal annotation type properties.
        annotation_type: MinimalAnnotationType,
    },
    /// The structure type object representation.
    #[dust_dds(case = TK_STRUCTURE)]
    TkStructure {
        /// Detailed minimal struct type properties.
        struct_type: MinimalStructType,
    },
    /// The union type object representation.
    #[dust_dds(case = TK_UNION)]
    TkUnion {
        /// Detailed minimal union type properties.
        union_type: MinimalUnionType,
    },
    /// The bitset type object representation.
    #[dust_dds(case = TK_BITSET)]
    TkBitset {
        /// Detailed minimal bitset type properties.
        bitset_type: MinimalBitsetType,
    },
    /// The sequence type object representation.
    #[dust_dds(case = TK_SEQUENCE)]
    TkSequence {
        /// Detailed minimal sequence type properties.
        sequence_type: MinimalSequenceType,
    },
    /// The array type object representation.
    #[dust_dds(case = TK_ARRAY)]
    TkArray {
        /// Detailed minimal array type properties.
        array_type: MinimalArrayType,
    },
    /// The map type object representation.
    #[dust_dds(case = TK_MAP)]
    TkMap {
        /// Detailed minimal map type properties.
        map_type: MinimalMapType,
    },
    /// The enumeration type object representation.
    #[dust_dds(case = TK_ENUM)]
    TkEnum {
        /// Detailed minimal enumerated type properties.
        enumerated_type: MinimalEnumeratedType,
    },
    /// The bitmask type object representation.
    #[dust_dds(case = TK_BITMASK)]
    TkBitmask {
        /// Detailed minimal bitmask type properties.
        bitmask_type: MinimalBitmaskType,
    },
    // =================== Future extensibility ============
    /// Default case for future extensibility.
    #[dust_dds(default)]
    Default {
        /// The minimal extended type properties.
        extended_type: MinimalExtendedType,
    },
}

/// The TypeObject is the ultimate representation of a type in XTypes, containing complete or minimal type descriptions.
#[derive(DdsType, Debug, Clone, PartialEq)]
#[dust_dds(extensibility = "appendable", nested, switch(u8))]
#[allow(clippy::large_enum_variant)]
pub enum TypeObject {
    /// Complete type representation.
    #[dust_dds(case=EK_COMPLETE)]
    EkComplete {
        /// The complete type object.
        complete: CompleteTypeObject,
    },
    /// Minimal type representation.
    #[dust_dds(case=EK_MINIMAL)]
    EkMinimal {
        /// The minimal type object.
        minimal: MinimalTypeObject,
    },
}

/// Sequence of TypeObjects.
pub type TypeObjectSeq = Vec<TypeObject>;
// Set of TypeObjects representing a strong component: Equivalence class
// for the Strong Connectivity relationship (mutual reachability between
// types).
// Ordered by fully qualified typename lexicographic order
/// Set of TypeObjects representing a strongly connected component (equivalence class for mutual reachability).
pub type StronglyConnectedComponent = TypeObjectSeq;

/// A pair of TypeIdentifier and TypeObject, used in discovery.
#[derive(DdsType, Debug, Clone, PartialEq)]
#[dust_dds(extensibility = "final", nested)]
pub struct TypeIdentifierTypeObjectPair {
    /// The TypeIdentifier of the type.
    pub type_identifier: TypeIdentifier,
    /// The TypeObject description of the type.
    pub type_object: TypeObject,
}
/// Sequence of TypeIdentifier and TypeObject pairs.
pub type TypeIdentifierTypeObjectPairSeq = Vec<TypeIdentifierTypeObjectPair>;

/// A pair of TypeIdentifiers, used to represent relationships or mappings.
#[derive(DdsType, Debug, Clone, PartialEq)]
#[dust_dds(extensibility = "final", nested)]
pub struct TypeIdentifierPair {
    /// The first type identifier.
    pub type_identifier1: TypeIdentifier,
    /// The second type identifier.
    pub type_identifier2: TypeIdentifier,
}
/// Sequence of TypeIdentifier pairs.
pub type TypeIdentifierPairSeq = Vec<TypeIdentifierPair>;

/// A TypeIdentifier packaged with the serialized size of its corresponding TypeObject.
#[derive(DdsType, Debug, Clone, PartialEq, Eq, Default)]
#[dust_dds(extensibility = "appendable", nested)]
pub struct TypeIdentifierWithSize {
    /// The TypeIdentifier.
    pub type_id: TypeIdentifier,
    /// The serialized size in bytes of the TypeObject.
    pub typeobject_serialized_size: u32,
}
/// Sequence of TypeIdentifiers packaged with sizes.
pub type TypeIdentfierWithSizeSeq = Vec<TypeIdentifierWithSize>;

/// A TypeIdentifier packaged with its size and all the dependent type identifiers it relies on.
#[derive(DdsType, Debug, Clone, PartialEq, Eq, Default)]
#[dust_dds(extensibility = "appendable", nested)]
pub struct TypeIdentifierWithDependencies {
    /// The TypeIdentifier with its serialized size.
    pub typeid_with_size: TypeIdentifierWithSize,
    /// The count of dependent type IDs.
    pub dependent_typeid_count: i32,
    /// The sequence of dependent type IDs with sizes.
    pub dependent_typeids: Vec<TypeIdentifierWithSize>,
}
/// Sequence of TypeIdentifiers with dependencies.
pub type TypeIdentifierWithDependenciesSeq = Vec<TypeIdentifierWithDependencies>;

/// Complete type information containing both complete and minimal representations with their dependencies.
#[derive(DdsType, Debug, Clone, PartialEq, Eq)]
#[dust_dds(extensibility = "mutable", nested)]
pub struct TypeInformation {
    /// Minimal type representation with dependencies.
    #[dust_dds(id = 0x1001)]
    pub minimal: TypeIdentifierWithDependencies,
    /// Complete type representation with dependencies.
    #[dust_dds(id = 0x1002)]
    pub complete: TypeIdentifierWithDependencies,
}
/// Sequence of TypeInformation.
pub type TypeInformationSeq = Vec<TypeInformation>;

impl<'a> From<DynamicType<'a>> for MinimalTypeObject {
    fn from(value: DynamicType<'a>) -> Self {
        match value.get_kind() {
            TypeKind::STRUCTURE => {
                let mut struct_flags = match value.descriptor.extensibility_kind {
                    ExtensibilityKind::Final => TYPE_FLAG_IS_FINAL,
                    ExtensibilityKind::Appendable => TYPE_FLAG_IS_APPENDABLE,
                    ExtensibilityKind::Mutable => TYPE_FLAG_IS_MUTABLE,
                };
                if value.descriptor.is_nested {
                    struct_flags |= TYPE_FLAG_IS_NESTED;
                }
                if value.descriptor.is_autoid_hash {
                    struct_flags |= TYPE_FLAG_IS_AUTOID_HASH;
                }

                let header = MinimalStructHeader {
                    base_type: TypeIdentifier::TkNone, // TODO: Include base type
                    detail: MinimalTypeDetail {},
                };
                let member_seq = value.member_list.iter().map(From::from).collect();
                let struct_type = MinimalStructType {
                    struct_flags,
                    header,
                    member_seq,
                };
                MinimalTypeObject::TkStructure { struct_type }
            }
            TypeKind::ENUM => {
                let mut enum_flags = match value.descriptor.extensibility_kind {
                    ExtensibilityKind::Final => TYPE_FLAG_IS_FINAL,
                    ExtensibilityKind::Appendable => TYPE_FLAG_IS_APPENDABLE,
                    ExtensibilityKind::Mutable => TYPE_FLAG_IS_MUTABLE,
                };
                if value.descriptor.is_nested {
                    enum_flags |= TYPE_FLAG_IS_NESTED;
                };

                let bit_bound = value.descriptor.bound.first().copied().unwrap_or(32) as u16;
                let header = MinimalEnumeratedHeader {
                    common: CommonEnumeratedHeader { bit_bound },
                };

                let literal_seq = value.member_list.iter().map(From::from).collect();

                let enumerated_type = MinimalEnumeratedType {
                    enum_flags,
                    header,
                    literal_seq,
                };
                MinimalTypeObject::TkEnum { enumerated_type }
            }
            TypeKind::UNION => {
                let mut union_flags = match value.descriptor.extensibility_kind {
                    ExtensibilityKind::Final => TYPE_FLAG_IS_FINAL,
                    ExtensibilityKind::Appendable => TYPE_FLAG_IS_APPENDABLE,
                    ExtensibilityKind::Mutable => TYPE_FLAG_IS_MUTABLE,
                };
                if value.descriptor.is_nested {
                    union_flags |= TYPE_FLAG_IS_NESTED;
                };

                let header = MinimalUnionHeader {
                    detail: MinimalTypeDetail {},
                };

                let mut member_iterator = value.member_list.iter();
                let discriminator = match member_iterator.next() {
                    Some(d) => {
                        let mut member_flags = MemberFlag::from(d.descriptor.try_construct_kind);
                        if d.descriptor.is_key {
                            member_flags |= MEMBER_FLAG_IS_KEY;
                        }
                        if d.descriptor.is_must_understand {
                            member_flags |= MEMBER_FLAG_IS_MUST_UNDERSTAND;
                        }
                        MinimalDiscriminatorMember {
                            common: CommonDiscriminatorMember {
                                member_flags,
                                type_id: (&d.descriptor.r#type).into(),
                            },
                        }
                    }
                    None => MinimalDiscriminatorMember {
                        common: CommonDiscriminatorMember {
                            member_flags: Default::default(),
                            type_id: TypeIdentifier::TkNone,
                        },
                    },
                };

                let member_seq = member_iterator.map(From::from).collect();

                let union_type = MinimalUnionType {
                    union_flags,
                    header,
                    discriminator,
                    member_seq,
                };

                MinimalTypeObject::TkUnion { union_type }
            }
            TypeKind::BITMASK => {
                let bitmask_flags = TYPE_FLAG_IS_FINAL;
                let bound = value.descriptor.bound.first().copied().unwrap_or(32) as u16;
                let header = MinimalBitmaskHeader {
                    common: CommonEnumeratedHeader { bit_bound: bound },
                };
                let flag_seq = value.member_list.iter().map(From::from).collect();
                let bitmask_type = MinimalBitmaskType {
                    bitmask_flags,
                    header,
                    flag_seq,
                };
                MinimalTypeObject::TkBitmask { bitmask_type }
            }
            t => todo!("Not yet implemeneted for {t:?}"),
        }
    }
}

impl From<DynamicType<'_>> for CompleteTypeObject {
    fn from(value: DynamicType) -> Self {
        match value.get_kind() {
            TypeKind::STRUCTURE => {
                let mut struct_flags = match value.descriptor.extensibility_kind {
                    ExtensibilityKind::Final => TYPE_FLAG_IS_FINAL,
                    ExtensibilityKind::Appendable => TYPE_FLAG_IS_APPENDABLE,
                    ExtensibilityKind::Mutable => TYPE_FLAG_IS_MUTABLE,
                };
                if value.descriptor.is_nested {
                    struct_flags |= TYPE_FLAG_IS_NESTED;
                }
                if value.descriptor.is_autoid_hash {
                    struct_flags |= TYPE_FLAG_IS_AUTOID_HASH;
                }

                let header = CompleteStructHeader {
                    base_type: TypeIdentifier::TkNone, // TODO: Include base type
                    detail: CompleteTypeDetail {
                        type_name: String::from(value.get_name()),
                        ann_builtin: None,
                        ann_custom: None,
                    },
                };
                let member_seq = value.member_list.iter().map(From::from).collect();
                let struct_type = CompleteStructType {
                    struct_flags,
                    header,
                    member_seq,
                };
                CompleteTypeObject::TkStructure { struct_type }
            }
            TypeKind::ENUM => {
                let mut enum_flags = match value.descriptor.extensibility_kind {
                    ExtensibilityKind::Final => TYPE_FLAG_IS_FINAL,
                    ExtensibilityKind::Appendable => TYPE_FLAG_IS_APPENDABLE,
                    ExtensibilityKind::Mutable => TYPE_FLAG_IS_MUTABLE,
                };
                if value.descriptor.is_nested {
                    enum_flags |= TYPE_FLAG_IS_NESTED;
                };

                let bit_bound = value.descriptor.bound.first().copied().unwrap_or(32) as u16;
                let header = CompleteEnumeratedHeader {
                    common: CommonEnumeratedHeader { bit_bound },
                    detail: CompleteTypeDetail {
                        ann_builtin: None,
                        ann_custom: None,
                        type_name: String::from(value.descriptor.name),
                    },
                };

                let literal_seq = value.member_list.iter().map(From::from).collect();

                let enumerated_type = CompleteEnumeratedType {
                    enum_flags,
                    header,
                    literal_seq,
                };
                CompleteTypeObject::TkEnum { enumerated_type }
            }
            TypeKind::BITMASK => {
                let bitmask_flags = TYPE_FLAG_IS_FINAL;
                let bound = value.descriptor.bound.first().copied().unwrap_or(32) as u16;
                let header = CompleteBitmaskHeader {
                    common: CommonEnumeratedHeader { bit_bound: bound },
                    detail: CompleteTypeDetail {
                        ann_builtin: None,
                        ann_custom: None,
                        type_name: String::from(value.descriptor.name),
                    },
                };
                let flag_seq = value.member_list.iter().map(From::from).collect();
                let bitmask_type = CompleteBitmaskType {
                    bitmask_flags,
                    header,
                    flag_seq,
                };
                CompleteTypeObject::TkBitmask { bitmask_type }
            }
            TypeKind::UNION => {
                let mut union_flags = match value.descriptor.extensibility_kind {
                    ExtensibilityKind::Final => TYPE_FLAG_IS_FINAL,
                    ExtensibilityKind::Appendable => TYPE_FLAG_IS_APPENDABLE,
                    ExtensibilityKind::Mutable => TYPE_FLAG_IS_MUTABLE,
                };
                if value.descriptor.is_nested {
                    union_flags |= TYPE_FLAG_IS_NESTED;
                };

                let header = CompleteUnionHeader {
                    detail: CompleteTypeDetail {
                        ann_builtin: None,
                        ann_custom: None,
                        type_name: String::from(value.descriptor.name),
                    },
                };

                let mut member_iterator = value.member_list.iter();
                let discriminator = match member_iterator.next() {
                    Some(d) => {
                        let mut member_flags = MemberFlag::from(d.descriptor.try_construct_kind);
                        if d.descriptor.is_key {
                            member_flags |= MEMBER_FLAG_IS_KEY;
                        }
                        if d.descriptor.is_must_understand {
                            member_flags |= MEMBER_FLAG_IS_MUST_UNDERSTAND;
                        }
                        CompleteDiscriminatorMember {
                            common: CommonDiscriminatorMember {
                                member_flags,
                                type_id: (&d.descriptor.r#type).into(),
                            },
                            ann_builtin: None,
                            ann_custom: None,
                        }
                    }
                    None => CompleteDiscriminatorMember {
                        common: CommonDiscriminatorMember {
                            member_flags: Default::default(),
                            type_id: TypeIdentifier::TkNone,
                        },
                        ann_builtin: None,
                        ann_custom: None,
                    },
                };

                let member_seq = member_iterator.map(From::from).collect();

                let union_type = CompleteUnionType {
                    union_flags,
                    header,
                    discriminator,
                    member_seq,
                };

                CompleteTypeObject::TkUnion { union_type }
            }
            t => todo!("Not yet implemeneted for {t:?}"),
        }
    }
}

impl<'a> From<&DynamicType<'a>> for TypeIdentifier {
    fn from(value: &DynamicType<'a>) -> Self {
        match value.descriptor.kind {
            TypeKind::NONE => TypeIdentifier::TkNone,
            TypeKind::BOOLEAN => TypeIdentifier::TkBoolean,
            TypeKind::BYTE => TypeIdentifier::TkByteType,
            TypeKind::INT16 => TypeIdentifier::TkInt16Type,
            TypeKind::INT32 => TypeIdentifier::TkInt32Type,
            TypeKind::INT64 => TypeIdentifier::TkInt64Type,
            TypeKind::UINT16 => TypeIdentifier::TkUint16Type,
            TypeKind::UINT32 => TypeIdentifier::TkUint32Type,
            TypeKind::UINT64 => TypeIdentifier::TkUint64Type,
            TypeKind::FLOAT32 => TypeIdentifier::TkFloat32Type,
            TypeKind::FLOAT64 => TypeIdentifier::TkFloat64Type,
            TypeKind::FLOAT128 => TypeIdentifier::TkFloat128Type,
            TypeKind::INT8 => TypeIdentifier::TkInt8Type,
            TypeKind::UINT8 => TypeIdentifier::TkUint8Type,
            TypeKind::CHAR8 => TypeIdentifier::TkChar8Type,
            TypeKind::CHAR16 => TypeIdentifier::TkChar16Type,
            TypeKind::STRING8 => {
                if let Some(&b) = value.descriptor.bound.first() {
                    if b <= u8::MAX as u32 {
                        TypeIdentifier::TiString8Small {
                            string_sdefn: StringSTypeDefn { bound: b as u8 },
                        }
                    } else {
                        TypeIdentifier::TiString8Large {
                            string_ldefn: StringLTypeDefn { bound: b },
                        }
                    }
                } else {
                    TypeIdentifier::TiString8Large {
                        string_ldefn: StringLTypeDefn { bound: u32::MAX },
                    }
                }
            }
            TypeKind::STRING16 => {
                if let Some(&b) = value.descriptor.bound.first() {
                    if b <= u8::MAX as u32 {
                        TypeIdentifier::TiString16Small {
                            string_sdefn: StringSTypeDefn { bound: b as u8 },
                        }
                    } else {
                        TypeIdentifier::TiString16Large {
                            string_ldefn: StringLTypeDefn { bound: b },
                        }
                    }
                } else {
                    TypeIdentifier::TiString16Large {
                        string_ldefn: StringLTypeDefn { bound: u32::MAX },
                    }
                }
            }
            TypeKind::ALIAS => todo!(),
            TypeKind::ANNOTATION => todo!(),
            TypeKind::STRUCTURE | TypeKind::UNION | TypeKind::ENUM | TypeKind::BITMASK => {
                let complete_type_object = TypeObject::EkComplete {
                    complete: CompleteTypeObject::from(*value),
                };
                let data = complete_type_object.create_dynamic_sample();
                let serialized_complete_type_object =
                    serialize_without_header_cdr2_le(Vec::new(), &data).expect("Not fallible");

                let hash_complete_type_object = md5::compute(&serialized_complete_type_object);
                TypeIdentifier::EkComplete {
                    equivalence_hash: [
                        hash_complete_type_object[0],
                        hash_complete_type_object[1],
                        hash_complete_type_object[2],
                        hash_complete_type_object[3],
                        hash_complete_type_object[4],
                        hash_complete_type_object[5],
                        hash_complete_type_object[6],
                        hash_complete_type_object[7],
                        hash_complete_type_object[8],
                        hash_complete_type_object[9],
                        hash_complete_type_object[10],
                        hash_complete_type_object[11],
                        hash_complete_type_object[12],
                        hash_complete_type_object[13],
                    ],
                }
            }
            TypeKind::BITSET => todo!(),
            TypeKind::SEQUENCE => {
                let bound = *value.descriptor.bound.first().unwrap_or(&u32::MAX);
                let element_identifier = Box::new(
                    value
                        .descriptor
                        .element_type
                        .as_ref()
                        .map(From::from)
                        .unwrap_or(TypeIdentifier::TkNone),
                );
                let equiv_kind = if element_identifier.is_fully_descriptive() {
                    EK_BOTH
                } else {
                    EK_COMPLETE
                };
                let header = PlainCollectionHeader {
                    equiv_kind,
                    element_flags: MemberFlag::from(TryConstructKind::Discard),
                };
                if bound <= u8::MAX as u32 {
                    TypeIdentifier::TiPlainSequenceSmall {
                        seq_sdefn: PlainSequenceSElemDefn {
                            header,
                            bound: bound as u8,
                            element_identifier,
                        },
                    }
                } else {
                    TypeIdentifier::TiPlainSequenceLarge {
                        seq_ldefn: PlainSequenceLElemDefn {
                            header,
                            bound,
                            element_identifier,
                        },
                    }
                }
            }
            TypeKind::ARRAY => {
                let bound = *value.descriptor.bound.first().unwrap_or(&u32::MAX);
                let element_identifier = Box::new(
                    value
                        .descriptor
                        .element_type
                        .as_ref()
                        .map(From::from)
                        .unwrap_or(TypeIdentifier::TkNone),
                );
                let equiv_kind = if element_identifier.is_fully_descriptive() {
                    EK_BOTH
                } else {
                    EK_COMPLETE
                };
                let header = PlainCollectionHeader {
                    equiv_kind,
                    element_flags: MemberFlag::from(TryConstructKind::Discard),
                };
                if bound <= u8::MAX as u32 {
                    TypeIdentifier::TiPlainArraySmall {
                        array_sdefn: PlainArraySElemDefn {
                            header,
                            array_bound_seq: vec![bound as u8],
                            element_identifier,
                        },
                    }
                } else {
                    TypeIdentifier::TiPlainArrayLarge {
                        array_ldefn: PlainArrayLElemDefn {
                            header,
                            array_bound_seq: vec![bound],
                            element_identifier,
                        },
                    }
                }
            }
            TypeKind::MAP => todo!(),
        }
    }
}

impl From<&DynamicTypeMember> for CommonStructMember {
    fn from(value: &DynamicTypeMember) -> Self {
        let mut member_flags = MemberFlag::from(value.descriptor.try_construct_kind);
        if value.descriptor.is_key {
            member_flags |= MEMBER_FLAG_IS_KEY;
        }
        if value.descriptor.is_must_understand {
            member_flags |= MEMBER_FLAG_IS_MUST_UNDERSTAND;
        }
        if value.descriptor.is_optional {
            member_flags |= MEMBER_FLAG_IS_OPTIONAL;
        }

        CommonStructMember {
            member_id: value.get_id(),
            member_flags,
            member_type_id: TypeIdentifier::from(&value.descriptor.r#type),
        }
    }
}

impl From<&DynamicTypeMember> for MinimalStructMember {
    fn from(value: &DynamicTypeMember) -> Self {
        let common = value.into();
        let name_hash = <[u8; 16]>::from(md5::compute(value.get_name().as_bytes()));
        let detail = MinimalMemberDetail {
            name_hash: [name_hash[0], name_hash[1], name_hash[2], name_hash[3]],
        };
        MinimalStructMember { common, detail }
    }
}

impl From<&DynamicTypeMember> for CompleteStructMember {
    fn from(value: &DynamicTypeMember) -> Self {
        let common = CommonStructMember::from(value);
        let detail = CompleteMemberDetail {
            name: String::from(value.get_name()),
            ann_builtin: None,
            ann_custom: None,
        };
        CompleteStructMember { common, detail }
    }
}

impl From<&DynamicTypeMember> for MinimalEnumeratedLiteral {
    fn from(value: &DynamicTypeMember) -> Self {
        let literal_value = value
            .descriptor
            .label
            .first()
            .copied()
            .unwrap_or(value.get_id() as i32);
        let mut flags = MemberFlag::default();
        if value.descriptor.is_default_label {
            flags |= MEMBER_FLAG_IS_DEFAULT;
        }
        let common = CommonEnumeratedLiteral {
            value: literal_value,
            flags,
        };
        let name_hash = <[u8; 16]>::from(md5::compute(value.get_name().as_bytes()));
        let detail = MinimalMemberDetail {
            name_hash: [name_hash[0], name_hash[1], name_hash[2], name_hash[3]],
        };
        MinimalEnumeratedLiteral { common, detail }
    }
}

impl From<&DynamicTypeMember> for CompleteEnumeratedLiteral {
    fn from(value: &DynamicTypeMember) -> Self {
        let literal_value = value
            .descriptor
            .label
            .first()
            .copied()
            .unwrap_or(value.get_id() as i32);
        let mut flags = MemberFlag::default();
        if value.descriptor.is_default_label {
            flags |= MEMBER_FLAG_IS_DEFAULT;
        }
        let common = CommonEnumeratedLiteral {
            value: literal_value,
            flags,
        };
        let detail = CompleteMemberDetail {
            name: String::from(value.get_name()),
            ann_builtin: None,
            ann_custom: None,
        };
        CompleteEnumeratedLiteral { common, detail }
    }
}

impl From<&DynamicTypeMember> for MinimalBitflag {
    fn from(value: &DynamicTypeMember) -> Self {
        let position = value
            .descriptor
            .label
            .first()
            .copied()
            .unwrap_or(value.get_id() as i32) as u16;
        let flags = MemberFlag::default();
        let common = CommonBitflag { position, flags };
        let name_hash = <[u8; 16]>::from(md5::compute(value.get_name().as_bytes()));
        let detail = MinimalMemberDetail {
            name_hash: [name_hash[0], name_hash[1], name_hash[2], name_hash[3]],
        };
        MinimalBitflag { common, detail }
    }
}

impl From<&DynamicTypeMember> for CompleteBitflag {
    fn from(value: &DynamicTypeMember) -> Self {
        let position = value
            .descriptor
            .label
            .first()
            .copied()
            .unwrap_or(value.get_id() as i32) as u16;
        let flags = MemberFlag::default();
        let common = CommonBitflag { position, flags };
        let detail = CompleteMemberDetail {
            name: String::from(value.get_name()),
            ann_builtin: None,
            ann_custom: None,
        };
        CompleteBitflag { common, detail }
    }
}

impl<'a> From<DynamicType<'a>> for TypeInformation {
    fn from(value: DynamicType<'a>) -> Self {
        let dependent_typeid_count = if value.has_dependencies() { -1 } else { 0 };

        let minimal_type_object = TypeObject::EkMinimal {
            minimal: MinimalTypeObject::from(value),
        };
        let data = minimal_type_object.create_dynamic_sample();

        let serialized_minimal_type_object =
            serialize_without_header_cdr2_le(Vec::new(), &data).expect("Not fallible");
        let hash_minimal_type_object = md5::compute(&serialized_minimal_type_object);

        let complete_type_object = TypeObject::EkComplete {
            complete: CompleteTypeObject::from(value),
        };
        let data = complete_type_object.create_dynamic_sample();
        let serialized_complete_type_object =
            serialize_without_header_cdr2_le(Vec::new(), &data).expect("Not fallible");

        let hash_complete_type_object = md5::compute(&serialized_complete_type_object);

        TypeInformation {
            minimal: TypeIdentifierWithDependencies {
                typeid_with_size: TypeIdentifierWithSize {
                    type_id: TypeIdentifier::EkMinimal {
                        equivalence_hash: [
                            hash_minimal_type_object[0],
                            hash_minimal_type_object[1],
                            hash_minimal_type_object[2],
                            hash_minimal_type_object[3],
                            hash_minimal_type_object[4],
                            hash_minimal_type_object[5],
                            hash_minimal_type_object[6],
                            hash_minimal_type_object[7],
                            hash_minimal_type_object[8],
                            hash_minimal_type_object[9],
                            hash_minimal_type_object[10],
                            hash_minimal_type_object[11],
                            hash_minimal_type_object[12],
                            hash_minimal_type_object[13],
                        ],
                    },
                    typeobject_serialized_size: serialized_minimal_type_object.len() as u32,
                },
                dependent_typeid_count,
                dependent_typeids: Vec::new(),
            },
            complete: TypeIdentifierWithDependencies {
                typeid_with_size: TypeIdentifierWithSize {
                    type_id: TypeIdentifier::EkComplete {
                        equivalence_hash: [
                            hash_complete_type_object[0],
                            hash_complete_type_object[1],
                            hash_complete_type_object[2],
                            hash_complete_type_object[3],
                            hash_complete_type_object[4],
                            hash_complete_type_object[5],
                            hash_complete_type_object[6],
                            hash_complete_type_object[7],
                            hash_complete_type_object[8],
                            hash_complete_type_object[9],
                            hash_complete_type_object[10],
                            hash_complete_type_object[11],
                            hash_complete_type_object[12],
                            hash_complete_type_object[13],
                        ],
                    },
                    typeobject_serialized_size: serialized_complete_type_object.len() as u32,
                },
                dependent_typeid_count,
                dependent_typeids: Vec::new(),
            },
        }
    }
}

/// Returns the sequence of dependent [`TypeIdentifierWithSize`] for all constructed dependencies of `dynamic_type`.
pub fn get_type_dependencies_with_size(dynamic_type: DynamicType) -> Vec<TypeIdentifierWithSize> {
    let deps = dynamic_type.get_dependencies();
    deps.into_iter()
        .map(|dep| {
            let complete_type_object = TypeObject::EkComplete {
                complete: CompleteTypeObject::from(dep),
            };
            let data = complete_type_object.create_dynamic_sample();
            let serialized =
                serialize_without_header_cdr2_le(Vec::new(), &data).expect("Not fallible");
            let hash = md5::compute(&serialized);
            TypeIdentifierWithSize {
                type_id: TypeIdentifier::EkComplete {
                    equivalence_hash: [
                        hash[0], hash[1], hash[2], hash[3], hash[4], hash[5], hash[6], hash[7],
                        hash[8], hash[9], hash[10], hash[11], hash[12], hash[13],
                    ],
                },
                typeobject_serialized_size: serialized.len() as u32,
            }
        })
        .collect()
}

impl From<TryConstructKind> for MemberFlag {
    fn from(value: TryConstructKind) -> Self {
        match value {
            TryConstructKind::UseDefault => MEMBER_FLAG_TRY_CONSTRUCT2,
            TryConstructKind::Discard => MEMBER_FLAG_TRY_CONSTRUCT1,
            TryConstructKind::Trim => MEMBER_FLAG_TRY_CONSTRUCT1 | MEMBER_FLAG_TRY_CONSTRUCT2,
        }
    }
}

impl TypeIdentifier {
    /// 7.3.4.6.1 Fully-descriptive TypeIdentifiers
    pub fn is_fully_descriptive(&self) -> bool {
        match self {
            TypeIdentifier::TkBoolean
            | TypeIdentifier::TkByteType
            | TypeIdentifier::TkInt8Type
            | TypeIdentifier::TkInt16Type
            | TypeIdentifier::TkInt32Type
            | TypeIdentifier::TkInt64Type
            | TypeIdentifier::TkUint8Type
            | TypeIdentifier::TkUint16Type
            | TypeIdentifier::TkUint32Type
            | TypeIdentifier::TkUint64Type
            | TypeIdentifier::TkFloat32Type
            | TypeIdentifier::TkFloat64Type
            | TypeIdentifier::TkFloat128Type
            | TypeIdentifier::TkChar8Type
            | TypeIdentifier::TkChar16Type
            | TypeIdentifier::TiString8Small { .. }
            | TypeIdentifier::TiString8Large { .. }
            | TypeIdentifier::TiString16Small { .. }
            | TypeIdentifier::TiString16Large { .. } => true,
            TypeIdentifier::TiPlainSequenceSmall { seq_sdefn } => {
                seq_sdefn.header.equiv_kind == EK_BOTH
            }
            TypeIdentifier::TiPlainSequenceLarge { seq_ldefn } => {
                seq_ldefn.header.equiv_kind == EK_BOTH
            }
            TypeIdentifier::TiPlainArraySmall { array_sdefn } => {
                array_sdefn.header.equiv_kind == EK_BOTH
            }
            TypeIdentifier::TiPlainArrayLarge { array_ldefn } => {
                array_ldefn.header.equiv_kind == EK_BOTH
            }
            TypeIdentifier::TiPlainMapSmall { map_sdefn } => map_sdefn.header.equiv_kind == EK_BOTH,
            TypeIdentifier::TiPlainMapLarge { map_ldefn } => map_ldefn.header.equiv_kind == EK_BOTH,
            _ => false,
        }
    }

    /// 7.2.4.4 Assignability Rules
    pub fn is_assignable_from(&self, other: &TypeIdentifier) -> bool {
        self.is_assignable_from_w_type_consistency(
            other,
            &TypeConsistencyEnforcementQosPolicy::const_default(),
            &|_| None,
        )
    }
    /// Assignability rules taking into account TypeConsistencyEnforcementQosPolicy
    pub fn is_assignable_from_w_type_consistency(
        &self,
        other: &TypeIdentifier,
        type_consistency: &TypeConsistencyEnforcementQosPolicy,
        type_resolver: &dyn Fn(&TypeIdentifier) -> Option<CompleteTypeObject>,
    ) -> bool {
        match self {
            TypeIdentifier::TkNone => todo!(),
            TypeIdentifier::TkBoolean => matches!(other, TypeIdentifier::TkBoolean),
            TypeIdentifier::TkByteType => matches!(other, TypeIdentifier::TkByteType),
            TypeIdentifier::TkUint8Type => match other {
                TypeIdentifier::TkUint8Type => true,
                TypeIdentifier::EkComplete { .. } | TypeIdentifier::EkMinimal { .. } => {
                    if let Some(obj) = type_resolver(other) {
                        match obj {
                            CompleteTypeObject::TkBitmask { bitmask_type } => {
                                (1..=8).contains(&bitmask_type.header.common.bit_bound)
                            }
                            CompleteTypeObject::TkEnum { enumerated_type } => {
                                (1..=8).contains(&enumerated_type.header.common.bit_bound)
                            }
                            _ => false,
                        }
                    } else {
                        true
                    }
                }
                _ => false,
            },
            TypeIdentifier::TkInt8Type => match other {
                TypeIdentifier::TkInt8Type => true,
                TypeIdentifier::EkComplete { .. } | TypeIdentifier::EkMinimal { .. } => {
                    if let Some(obj) = type_resolver(other) {
                        match obj {
                            CompleteTypeObject::TkEnum { enumerated_type } => {
                                (1..=8).contains(&enumerated_type.header.common.bit_bound)
                            }
                            _ => false,
                        }
                    } else {
                        true
                    }
                }
                _ => false,
            },
            TypeIdentifier::TkInt16Type => match other {
                TypeIdentifier::TkInt16Type => true,
                TypeIdentifier::EkComplete { .. } | TypeIdentifier::EkMinimal { .. } => {
                    if let Some(obj) = type_resolver(other) {
                        match obj {
                            CompleteTypeObject::TkEnum { enumerated_type } => {
                                (9..=16).contains(&enumerated_type.header.common.bit_bound)
                            }
                            _ => false,
                        }
                    } else {
                        true
                    }
                }
                _ => false,
            },
            TypeIdentifier::TkUint16Type => match other {
                TypeIdentifier::TkUint16Type => true,
                TypeIdentifier::EkComplete { .. } | TypeIdentifier::EkMinimal { .. } => {
                    if let Some(obj) = type_resolver(other) {
                        match obj {
                            CompleteTypeObject::TkBitmask { bitmask_type } => {
                                (9..=16).contains(&bitmask_type.header.common.bit_bound)
                            }
                            CompleteTypeObject::TkEnum { enumerated_type } => {
                                (9..=16).contains(&enumerated_type.header.common.bit_bound)
                            }
                            _ => false,
                        }
                    } else {
                        true
                    }
                }
                _ => false,
            },
            TypeIdentifier::TkInt32Type => match other {
                TypeIdentifier::TkInt32Type => true,
                TypeIdentifier::EkComplete { .. } | TypeIdentifier::EkMinimal { .. } => {
                    if let Some(obj) = type_resolver(other) {
                        match obj {
                            CompleteTypeObject::TkEnum { enumerated_type } => {
                                (17..=32).contains(&enumerated_type.header.common.bit_bound)
                            }
                            _ => false,
                        }
                    } else {
                        true
                    }
                }
                _ => false,
            },
            TypeIdentifier::TkUint32Type => match other {
                TypeIdentifier::TkUint32Type => true,
                TypeIdentifier::EkComplete { .. } | TypeIdentifier::EkMinimal { .. } => {
                    if let Some(obj) = type_resolver(other) {
                        match obj {
                            CompleteTypeObject::TkBitmask { bitmask_type } => {
                                (17..=32).contains(&bitmask_type.header.common.bit_bound)
                            }
                            CompleteTypeObject::TkEnum { enumerated_type } => {
                                (17..=32).contains(&enumerated_type.header.common.bit_bound)
                            }
                            _ => false,
                        }
                    } else {
                        true
                    }
                }
                _ => false,
            },
            TypeIdentifier::TkInt64Type => match other {
                TypeIdentifier::TkInt64Type => true,
                TypeIdentifier::EkComplete { .. } | TypeIdentifier::EkMinimal { .. } => {
                    if let Some(obj) = type_resolver(other) {
                        match obj {
                            CompleteTypeObject::TkEnum { enumerated_type } => {
                                (33..=64).contains(&enumerated_type.header.common.bit_bound)
                            }
                            _ => false,
                        }
                    } else {
                        true
                    }
                }
                _ => false,
            },
            TypeIdentifier::TkUint64Type => match other {
                TypeIdentifier::TkUint64Type => true,
                TypeIdentifier::EkComplete { .. } | TypeIdentifier::EkMinimal { .. } => {
                    if let Some(obj) = type_resolver(other) {
                        match obj {
                            CompleteTypeObject::TkBitmask { bitmask_type } => {
                                (33..=64).contains(&bitmask_type.header.common.bit_bound)
                            }
                            CompleteTypeObject::TkEnum { enumerated_type } => {
                                (33..=64).contains(&enumerated_type.header.common.bit_bound)
                            }
                            _ => false,
                        }
                    } else {
                        true
                    }
                }
                _ => false,
            },
            TypeIdentifier::TkFloat32Type => matches!(other, TypeIdentifier::TkFloat32Type),
            TypeIdentifier::TkFloat64Type => matches!(other, TypeIdentifier::TkFloat64Type),
            TypeIdentifier::TkFloat128Type => matches!(other, TypeIdentifier::TkFloat128Type),
            TypeIdentifier::TkChar8Type => matches!(other, TypeIdentifier::TkChar8Type),
            TypeIdentifier::TkChar16Type => matches!(other, TypeIdentifier::TkChar16Type),
            TypeIdentifier::TiString8Small { string_sdefn: t1 } => match other {
                TypeIdentifier::TiString8Small { string_sdefn: t2 } => {
                    type_consistency.ignore_string_bounds
                        || t1.bound == 0
                        || (t2.bound != 0 && t1.bound >= t2.bound)
                }
                TypeIdentifier::TiString8Large { string_ldefn: t2 } => {
                    type_consistency.ignore_string_bounds
                        || t1.bound == 0
                        || (t2.bound != 0 && t1.bound as u32 >= t2.bound)
                }
                _ => false,
            },
            TypeIdentifier::TiString8Large { string_ldefn: t1 } => match other {
                TypeIdentifier::TiString8Small { string_sdefn: t2 } => {
                    type_consistency.ignore_string_bounds
                        || t1.bound == 0
                        || (t2.bound != 0 && t1.bound >= t2.bound as u32)
                }
                TypeIdentifier::TiString8Large { string_ldefn: t2 } => {
                    type_consistency.ignore_string_bounds
                        || t1.bound == 0
                        || (t2.bound != 0 && t1.bound >= t2.bound)
                }
                _ => false,
            },
            TypeIdentifier::TiString16Small { string_sdefn: t1 } => match other {
                TypeIdentifier::TiString16Small { string_sdefn: t2 } => {
                    type_consistency.ignore_string_bounds
                        || t1.bound == 0
                        || (t2.bound != 0 && t1.bound >= t2.bound)
                }
                TypeIdentifier::TiString16Large { string_ldefn: t2 } => {
                    type_consistency.ignore_string_bounds
                        || t1.bound == 0
                        || (t2.bound != 0 && t1.bound as u32 >= t2.bound)
                }
                _ => false,
            },
            TypeIdentifier::TiString16Large { string_ldefn: t1 } => match other {
                TypeIdentifier::TiString16Small { string_sdefn: t2 } => {
                    type_consistency.ignore_string_bounds
                        || t1.bound == 0
                        || (t2.bound != 0 && t1.bound >= t2.bound as u32)
                }
                TypeIdentifier::TiString16Large { string_ldefn: t2 } => {
                    type_consistency.ignore_string_bounds
                        || t1.bound == 0
                        || (t2.bound != 0 && t1.bound >= t2.bound)
                }
                _ => false,
            },
            TypeIdentifier::TiPlainSequenceSmall { seq_sdefn: t1 } => match other {
                TypeIdentifier::TiPlainSequenceSmall { seq_sdefn: t2 } => {
                    let bounds_ok = type_consistency.ignore_sequence_bounds
                        || t1.bound == 0
                        || (t2.bound != 0 && t1.bound >= t2.bound);
                    bounds_ok
                        && t1.element_identifier.is_assignable_from_w_type_consistency(
                            &t2.element_identifier,
                            type_consistency,
                            type_resolver,
                        )
                }
                TypeIdentifier::TiPlainSequenceLarge { seq_ldefn: t2 } => {
                    let bounds_ok = type_consistency.ignore_sequence_bounds
                        || t1.bound == 0
                        || (t2.bound != 0 && t1.bound as u32 >= t2.bound);
                    bounds_ok
                        && t1.element_identifier.is_assignable_from_w_type_consistency(
                            &t2.element_identifier,
                            type_consistency,
                            type_resolver,
                        )
                }
                _ => false,
            },
            TypeIdentifier::TiPlainSequenceLarge { seq_ldefn: t1 } => match other {
                TypeIdentifier::TiPlainSequenceSmall { seq_sdefn: t2 } => {
                    let bounds_ok = type_consistency.ignore_sequence_bounds
                        || t1.bound == 0
                        || (t2.bound != 0 && t1.bound >= t2.bound as u32);
                    bounds_ok
                        && t1.element_identifier.is_assignable_from_w_type_consistency(
                            &t2.element_identifier,
                            type_consistency,
                            type_resolver,
                        )
                }
                TypeIdentifier::TiPlainSequenceLarge { seq_ldefn: t2 } => {
                    let bounds_ok = type_consistency.ignore_sequence_bounds
                        || t1.bound == 0
                        || (t2.bound != 0 && t1.bound >= t2.bound);
                    bounds_ok
                        && t1.element_identifier.is_assignable_from_w_type_consistency(
                            &t2.element_identifier,
                            type_consistency,
                            type_resolver,
                        )
                }
                _ => false,
            },
            TypeIdentifier::TiPlainArraySmall { array_sdefn: t1 } => match other {
                TypeIdentifier::TiPlainArraySmall { array_sdefn: t2 } => {
                    t1.array_bound_seq == t2.array_bound_seq
                        && t1.element_identifier.is_assignable_from_w_type_consistency(
                            &t2.element_identifier,
                            type_consistency,
                            type_resolver,
                        )
                }
                _ => false,
            },
            TypeIdentifier::TiPlainArrayLarge { array_ldefn: t1 } => match other {
                TypeIdentifier::TiPlainArrayLarge { array_ldefn: t2 } => {
                    t1.array_bound_seq == t2.array_bound_seq
                        && t1.element_identifier.is_assignable_from_w_type_consistency(
                            &t2.element_identifier,
                            type_consistency,
                            type_resolver,
                        )
                }
                _ => false,
            },
            TypeIdentifier::TiPlainMapSmall { map_sdefn: _ } => todo!(),
            TypeIdentifier::TiPlainMapLarge { map_ldefn: _ } => todo!(),
            TypeIdentifier::TiStronglyConnectedComponent { sc_component_id: _ } => todo!(),
            TypeIdentifier::EkComplete { .. } | TypeIdentifier::EkMinimal { .. } => {
                if let Some(obj) = type_resolver(self) {
                    match &obj {
                        CompleteTypeObject::TkBitmask { bitmask_type } => {
                            let bound = bitmask_type.header.common.bit_bound;
                            match other {
                                TypeIdentifier::TkByteType | TypeIdentifier::TkUint8Type => {
                                    (1..=8).contains(&bound)
                                }
                                TypeIdentifier::TkUint16Type => (9..=16).contains(&bound),
                                TypeIdentifier::TkUint32Type => (17..=32).contains(&bound),
                                TypeIdentifier::TkUint64Type => (33..=64).contains(&bound),
                                TypeIdentifier::EkComplete { .. }
                                | TypeIdentifier::EkMinimal { .. } => {
                                    if let Some(other_obj) = type_resolver(other) {
                                        obj.is_assignable_from_w_type_consistency(
                                            &other_obj,
                                            type_consistency,
                                            type_resolver,
                                        )
                                    } else {
                                        true
                                    }
                                }
                                _ => false,
                            }
                        }
                        CompleteTypeObject::TkEnum { enumerated_type } => {
                            let bound = enumerated_type.header.common.bit_bound;
                            match other {
                                TypeIdentifier::TkByteType
                                | TypeIdentifier::TkInt8Type
                                | TypeIdentifier::TkUint8Type => (1..=8).contains(&bound),
                                TypeIdentifier::TkInt16Type | TypeIdentifier::TkUint16Type => {
                                    (9..=16).contains(&bound)
                                }
                                TypeIdentifier::TkInt32Type | TypeIdentifier::TkUint32Type => {
                                    (17..=32).contains(&bound)
                                }
                                TypeIdentifier::TkInt64Type | TypeIdentifier::TkUint64Type => {
                                    (33..=64).contains(&bound)
                                }
                                TypeIdentifier::EkComplete { .. }
                                | TypeIdentifier::EkMinimal { .. } => {
                                    if let Some(other_obj) = type_resolver(other) {
                                        obj.is_assignable_from_w_type_consistency(
                                            &other_obj,
                                            type_consistency,
                                            type_resolver,
                                        )
                                    } else {
                                        true
                                    }
                                }
                                _ => false,
                            }
                        }
                        _ => {
                            if let TypeIdentifier::EkComplete { .. }
                            | TypeIdentifier::EkMinimal { .. } = other
                            {
                                if let Some(other_obj) = type_resolver(other) {
                                    obj.is_assignable_from_w_type_consistency(
                                        &other_obj,
                                        type_consistency,
                                        type_resolver,
                                    )
                                } else {
                                    true
                                }
                            } else {
                                false
                            }
                        }
                    }
                } else {
                    matches!(
                        other,
                        TypeIdentifier::EkComplete { .. }
                            | TypeIdentifier::TkByteType
                            | TypeIdentifier::TkInt8Type
                            | TypeIdentifier::TkUint8Type
                            | TypeIdentifier::TkInt16Type
                            | TypeIdentifier::TkUint16Type
                            | TypeIdentifier::TkInt32Type
                            | TypeIdentifier::TkUint32Type
                            | TypeIdentifier::TkInt64Type
                            | TypeIdentifier::TkUint64Type
                    )
                }
            }
            TypeIdentifier::Default { extended_type: _ } => todo!(),
        }
    }
}

impl CompleteTypeObject {
    /// This methods implements the rules defined in the DDS-XTypes standard chapter 7.2.4
    /// which defines the compatibility between two types.
    pub fn is_assignable_from(&self, t2: &CompleteTypeObject) -> bool {
        self.is_assignable_from_w_type_consistency(
            t2,
            &TypeConsistencyEnforcementQosPolicy::const_default(),
            &|_| None,
        )
    }

    /// Checks compatibility between two types taking into account TypeConsistencyEnforcementQosPolicy
    pub fn is_assignable_from_w_type_consistency(
        &self,
        t2: &CompleteTypeObject,
        type_consistency: &TypeConsistencyEnforcementQosPolicy,
        type_resolver: &dyn Fn(&TypeIdentifier) -> Option<CompleteTypeObject>,
    ) -> bool {
        if self == t2 {
            return true;
        }
        match (self, t2) {
            (
                CompleteTypeObject::TkStructure { struct_type: t1 },
                CompleteTypeObject::TkStructure { struct_type: t2 },
            ) => {
                // • Extensibility rules (Clause 7.2.4.4.2 Structure Types)
                let is_t1_final = (t1.struct_flags & TYPE_FLAG_IS_FINAL) == TYPE_FLAG_IS_FINAL;
                let is_t2_final = (t2.struct_flags & TYPE_FLAG_IS_FINAL) == TYPE_FLAG_IS_FINAL;
                let is_t1_appendable =
                    (t1.struct_flags & TYPE_FLAG_IS_APPENDABLE) == TYPE_FLAG_IS_APPENDABLE;
                let is_t2_appendable =
                    (t2.struct_flags & TYPE_FLAG_IS_APPENDABLE) == TYPE_FLAG_IS_APPENDABLE;
                let is_t1_mutable =
                    (t1.struct_flags & TYPE_FLAG_IS_MUTABLE) == TYPE_FLAG_IS_MUTABLE;
                let is_t2_mutable =
                    (t2.struct_flags & TYPE_FLAG_IS_MUTABLE) == TYPE_FLAG_IS_MUTABLE;

                if is_t1_final || is_t2_final {
                    if !is_t1_final || !is_t2_final || t1.member_seq.len() != t2.member_seq.len() {
                        return false;
                    }
                } else if is_t1_appendable != is_t2_appendable || is_t1_mutable != is_t2_mutable {
                    return false;
                }

                if !is_t1_mutable && !is_t2_mutable {
                    for (m1, m2) in t1.member_seq.iter().zip(t2.member_seq.iter()) {
                        if m1.common.member_id != m2.common.member_id {
                            return false;
                        }
                        if !type_consistency.ignore_member_names && m1.detail.name != m2.detail.name
                        {
                            return false;
                        }
                        if !m1
                            .common
                            .member_type_id
                            .is_assignable_from_w_type_consistency(
                                &m2.common.member_type_id,
                                type_consistency,
                                type_resolver,
                            )
                        {
                            return false;
                        }
                    }
                    if is_t1_final && is_t2_final {
                        return true;
                    }
                }

                // • There is at least one member "m1" of T1 and one corresponding member "m2" of T2
                //   such that m1.id == m2.id.
                let mut has_common_id = false;
                for m2 in &t2.member_seq {
                    if t1
                        .member_seq
                        .iter()
                        .any(|m1| m1.common.member_id == m2.common.member_id)
                    {
                        has_common_id = true
                    }
                }
                if !has_common_id {
                    return false;
                }

                if type_consistency.prevent_type_widening {
                    for m1 in &t1.member_seq {
                        if !t2
                            .member_seq
                            .iter()
                            .any(|m2| m2.common.member_id == m1.common.member_id)
                        {
                            return false;
                        }
                    }
                }

                // • For any member "m2" in T2, if there is a member "m1" in T1 with the same member ID, then the type
                //   KeyErased(m1.type) is-assignable-from the type KeyErased(m2.type).
                let mut members_are_assignable = true;
                for m2 in t2.member_seq.iter() {
                    if let Some(m1) = t1
                        .member_seq
                        .iter()
                        .find(|m1| m1.common.member_id == m2.common.member_id)
                    {
                        if !type_consistency.ignore_member_names && m1.detail.name != m2.detail.name
                        {
                            return false;
                        }
                        members_are_assignable &= m1
                            .common
                            .member_type_id
                            .is_assignable_from_w_type_consistency(
                                &m2.common.member_type_id,
                                type_consistency,
                                type_resolver,
                            );
                    } else if !type_consistency.ignore_member_names
                        && t1
                            .member_seq
                            .iter()
                            .any(|m1| m1.detail.name == m2.detail.name)
                    {
                        return false;
                    }
                }

                // • Key consistency rules (Clause 7.2.4.4.2 Structure Types)
                // • Either both T1 and T2 have at least one key member, or neither does.
                let t1_has_key = t1
                    .member_seq
                    .iter()
                    .any(|m| (m.common.member_flags.0 & MEMBER_FLAG_IS_KEY.0) != 0);
                let t2_has_key = t2
                    .member_seq
                    .iter()
                    .any(|m| (m.common.member_flags.0 & MEMBER_FLAG_IS_KEY.0) != 0);
                if t1_has_key != t2_has_key {
                    return false;
                }

                // • For any key member in T1, there is a corresponding key member in T2 with the same member ID.
                for m1 in &t1.member_seq {
                    let is_key = (m1.common.member_flags.0 & MEMBER_FLAG_IS_KEY.0) != 0;
                    if is_key
                        && !t2
                            .member_seq
                            .iter()
                            .any(|m2| m2.common.member_id == m1.common.member_id)
                    {
                        return false;
                    }
                }

                // • For any key member in T2, there is a corresponding key member in T1 with the same member ID.
                for m2 in &t2.member_seq {
                    let is_key = (m2.common.member_flags.0 & MEMBER_FLAG_IS_KEY.0) != 0;
                    if is_key
                        && !t1
                            .member_seq
                            .iter()
                            .any(|m1| m1.common.member_id == m2.common.member_id)
                    {
                        return false;
                    }
                }

                // • Members for which both optional is false and must_understand is true in either T1 or T2 appear
                //   (i.e., have a corresponding member of the same member ID) in both T1 and T2.
                for m1 in &t1.member_seq {
                    let is_optional = (m1.common.member_flags.0 & MEMBER_FLAG_IS_OPTIONAL.0) != 0;
                    let is_must_understand =
                        (m1.common.member_flags.0 & MEMBER_FLAG_IS_MUST_UNDERSTAND.0) != 0;
                    if !is_optional
                        && is_must_understand
                        && !t2
                            .member_seq
                            .iter()
                            .any(|m2| m2.common.member_id == m1.common.member_id)
                    {
                        return false;
                    }
                }
                for m2 in &t2.member_seq {
                    let is_optional = (m2.common.member_flags.0 & MEMBER_FLAG_IS_OPTIONAL.0) != 0;
                    let is_must_understand =
                        (m2.common.member_flags.0 & MEMBER_FLAG_IS_MUST_UNDERSTAND.0) != 0;
                    if !is_optional
                        && is_must_understand
                        && !t1
                            .member_seq
                            .iter()
                            .any(|m1| m1.common.member_id == m2.common.member_id)
                    {
                        return false;
                    }
                }

                // • For any member "m2" in T2, if there is a key member "m1" in T1 with the same member ID, then the type
                //   verifies that KeyHolder(m1.type) is-assignable-from KeyHolder(m2.type).
                members_are_assignable
            }
            (
                CompleteTypeObject::TkUnion { union_type: t1 },
                CompleteTypeObject::TkUnion { union_type: t2 },
            ) => {
                let is_t1_final = (t1.union_flags & TYPE_FLAG_IS_FINAL) == TYPE_FLAG_IS_FINAL;
                let is_t2_final = (t2.union_flags & TYPE_FLAG_IS_FINAL) == TYPE_FLAG_IS_FINAL;
                let is_t1_appendable =
                    (t1.union_flags & TYPE_FLAG_IS_APPENDABLE) == TYPE_FLAG_IS_APPENDABLE;
                let is_t2_appendable =
                    (t2.union_flags & TYPE_FLAG_IS_APPENDABLE) == TYPE_FLAG_IS_APPENDABLE;
                let is_t1_mutable = (t1.union_flags & TYPE_FLAG_IS_MUTABLE) == TYPE_FLAG_IS_MUTABLE;
                let is_t2_mutable = (t2.union_flags & TYPE_FLAG_IS_MUTABLE) == TYPE_FLAG_IS_MUTABLE;

                if is_t1_final || is_t2_final {
                    if !is_t1_final || !is_t2_final || t1.member_seq.len() != t2.member_seq.len() {
                        return false;
                    }
                } else if is_t1_appendable != is_t2_appendable || is_t1_mutable != is_t2_mutable {
                    return false;
                }
                if !t1
                    .discriminator
                    .common
                    .type_id
                    .is_assignable_from_w_type_consistency(
                        &t2.discriminator.common.type_id,
                        type_consistency,
                        type_resolver,
                    )
                {
                    return false;
                }

                // • Key consistency rules (Clause 7.2.4.4.3 Union Types)
                let t1_disc_is_key =
                    (t1.discriminator.common.member_flags.0 & MEMBER_FLAG_IS_KEY.0) != 0;
                let t2_disc_is_key =
                    (t2.discriminator.common.member_flags.0 & MEMBER_FLAG_IS_KEY.0) != 0;
                if t1_disc_is_key != t2_disc_is_key {
                    return false;
                }

                let t1_has_key = t1_disc_is_key
                    || t1
                        .member_seq
                        .iter()
                        .any(|m| (m.common.member_flags.0 & MEMBER_FLAG_IS_KEY.0) != 0);
                let t2_has_key = t2_disc_is_key
                    || t2
                        .member_seq
                        .iter()
                        .any(|m| (m.common.member_flags.0 & MEMBER_FLAG_IS_KEY.0) != 0);
                if t1_has_key != t2_has_key {
                    return false;
                }

                for m1 in &t1.member_seq {
                    let is_key = (m1.common.member_flags.0 & MEMBER_FLAG_IS_KEY.0) != 0;
                    if is_key
                        && !t2
                            .member_seq
                            .iter()
                            .any(|m2| m2.common.member_id == m1.common.member_id)
                    {
                        return false;
                    }
                }
                for m2 in &t2.member_seq {
                    let is_key = (m2.common.member_flags.0 & MEMBER_FLAG_IS_KEY.0) != 0;
                    if is_key
                        && !t1
                            .member_seq
                            .iter()
                            .any(|m1| m1.common.member_id == m2.common.member_id)
                    {
                        return false;
                    }
                }

                let t1_default = t1
                    .member_seq
                    .iter()
                    .find(|m| (m.common.member_flags.0 & MEMBER_FLAG_IS_DEFAULT.0) != 0);
                let t2_default = t2
                    .member_seq
                    .iter()
                    .find(|m| (m.common.member_flags.0 & MEMBER_FLAG_IS_DEFAULT.0) != 0);

                if (is_t1_final && is_t2_final) && (t1_default.is_some() != t2_default.is_some()) {
                    return false;
                }

                // If prevent_type_widening is true, every label in t1 must exist in t2
                if type_consistency.prevent_type_widening {
                    if t1_default.is_some() && t2_default.is_none() {
                        return false;
                    }
                    for m1 in &t1.member_seq {
                        for l1 in &m1.common.label_seq {
                            if !t2
                                .member_seq
                                .iter()
                                .any(|m2| m2.common.label_seq.contains(l1))
                            {
                                return false;
                            }
                        }
                    }
                }

                // If not ignoring member names, members with the same ID must have the same name
                if !type_consistency.ignore_member_names {
                    for m2 in &t2.member_seq {
                        if let Some(m1) = t1
                            .member_seq
                            .iter()
                            .find(|m1| m1.common.member_id == m2.common.member_id)
                        {
                            if m1.detail.name != m2.detail.name {
                                return false;
                            }
                        }
                    }
                }

                // Find member by label in t1
                let find_t1_member = |label: i32| -> Option<&CompleteUnionMember> {
                    t1.member_seq
                        .iter()
                        .find(|m| m.common.label_seq.contains(&label))
                        .or(t1_default)
                };

                // Check that there is at least one common explicit label
                let mut has_common_label = false;
                for m2 in &t2.member_seq {
                    for l2 in &m2.common.label_seq {
                        if t1
                            .member_seq
                            .iter()
                            .any(|m1| m1.common.label_seq.contains(l2))
                        {
                            has_common_label = true;
                            break;
                        }
                    }
                }

                if !has_common_label {
                    return false;
                }

                // For every explicit label in t2, the selected member in t1 must be assignable
                for m2 in &t2.member_seq {
                    for l2 in &m2.common.label_seq {
                        if let Some(m1) = find_t1_member(*l2) {
                            if !type_consistency.ignore_member_names
                                && m1.detail.name != m2.detail.name
                            {
                                return false;
                            }
                            if !m1.common.type_id.is_assignable_from_w_type_consistency(
                                &m2.common.type_id,
                                type_consistency,
                                type_resolver,
                            ) {
                                return false;
                            }
                        } else if is_t1_final {
                            return false;
                        }
                    }
                }

                // If t2 has a default case, verify default case compatibility
                if let Some(m2_def) = t2_default {
                    if let Some(m1_def) = t1_default {
                        if !type_consistency.ignore_member_names
                            && m1_def.detail.name != m2_def.detail.name
                        {
                            return false;
                        }
                        if !m1_def.common.type_id.is_assignable_from_w_type_consistency(
                            &m2_def.common.type_id,
                            type_consistency,
                            type_resolver,
                        ) {
                            return false;
                        }
                    }
                    // Any explicit label in t1 not covered by t2's explicit labels falls into t2's default case
                    for m1 in &t1.member_seq {
                        for l1 in &m1.common.label_seq {
                            if !t2
                                .member_seq
                                .iter()
                                .any(|m2| m2.common.label_seq.contains(l1))
                            {
                                if !type_consistency.ignore_member_names
                                    && m1.detail.name != m2_def.detail.name
                                {
                                    return false;
                                }
                                if !m1.common.type_id.is_assignable_from_w_type_consistency(
                                    &m2_def.common.type_id,
                                    type_consistency,
                                    type_resolver,
                                ) {
                                    return false;
                                }
                            }
                        }
                    }
                }

                true
            }
            (
                CompleteTypeObject::TkEnum {
                    enumerated_type: t1,
                },
                CompleteTypeObject::TkEnum {
                    enumerated_type: t2,
                },
            ) => t1.header.common.bit_bound == t2.header.common.bit_bound,
            (
                CompleteTypeObject::TkBitmask { bitmask_type: t1 },
                CompleteTypeObject::TkBitmask { bitmask_type: t2 },
            ) => t1.header.common.bit_bound == t2.header.common.bit_bound,
            _ => false,
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::xtypes::{
        deserializer::deserialize_top_level_type, dynamic_type::TypeDescriptor,
        serializer::serialize_without_header_cdr2_le, type_support::BoundedString,
    };

    use super::*;
    #[test]
    fn shape_type_hash() {
        #[derive(Debug, PartialEq, TypeSupport)]
        #[dust_dds(extensibility = "appendable")]
        struct ShapeType {
            #[dust_dds(key)]
            color: BoundedString<128>,
            x: i32,
            y: i32,
            shapesize: i32,
        }

        let type_information = TypeInformation::from(ShapeType::get_type());
        assert_eq!(type_information.complete.dependent_typeid_count, 0);
        assert_eq!(type_information.minimal.dependent_typeid_count, 0);
        assert_eq!(
            type_information
                .complete
                .typeid_with_size
                .typeobject_serialized_size,
            132
        );
        assert_eq!(
            type_information.complete.typeid_with_size.type_id,
            TypeIdentifier::EkComplete {
                equivalence_hash: [
                    0xce, 0x6d, 0x79, 0x13, 0x05, 0x8d, 0xaa, 0x30, 0x78, 0xa8, 0x8f, 0x98, 0x21,
                    0x96
                ]
            }
        );
    }

    #[test]
    fn type_information_with_dependencies() {
        #[derive(Debug, PartialEq, TypeSupport)]
        enum Status {
            Active,
            Inactive,
        }

        #[derive(Debug, PartialEq, TypeSupport)]
        struct StructWithEnum {
            id: u32,
            status: Status,
        }

        let type_info = TypeInformation::from(StructWithEnum::get_type());
        assert_eq!(type_info.complete.dependent_typeid_count, -1);
        assert_eq!(type_info.minimal.dependent_typeid_count, -1);

        #[derive(Debug, PartialEq, TypeSupport)]
        struct StructWithArrayOfEnum {
            statuses: [Status; 3],
        }

        let type_info = TypeInformation::from(StructWithArrayOfEnum::get_type());
        assert_eq!(type_info.complete.dependent_typeid_count, -1);
        assert_eq!(type_info.minimal.dependent_typeid_count, -1);

        #[derive(Debug, PartialEq, TypeSupport)]
        struct NestedStruct {
            nested: StructWithEnum,
        }

        let type_info = TypeInformation::from(NestedStruct::get_type());
        assert_eq!(type_info.complete.dependent_typeid_count, -1);
        assert_eq!(type_info.minimal.dependent_typeid_count, -1);
    }

    #[test]
    fn serialize_plain_array_s_elem_defn() {
        let dynamic_data = PlainArraySElemDefn {
            header: PlainCollectionHeader {
                equiv_kind: 6,
                element_flags: MemberFlag(5),
            },
            array_bound_seq: vec![20],
            element_identifier: Box::new(TypeIdentifier::TkInt32Type),
        }
        .create_dynamic_sample();
        let buffer = serialize_without_header_cdr2_le(Vec::new(), &dynamic_data).unwrap();

        let expected = [
            6, 0, // array_sdefn: header: equiv_kind | padding
            5, 0, // array_sdefn: header: element_flags
            1, 0, 0, 0,  // array_sdefn: array_bound_seq: length
            20, // array_sdefn: array_bound_seq: value
            4,  // array_sdefn: element_identifier: TkInt32Type
        ];

        assert_eq!(buffer, expected.to_vec())
    }

    #[test]
    fn deserialize_plain_array_s_elem_defn() {
        let expected = PlainArraySElemDefn {
            header: PlainCollectionHeader {
                equiv_kind: 6,
                element_flags: MemberFlag(5),
            },
            array_bound_seq: vec![20],
            element_identifier: Box::new(TypeIdentifier::TkInt32Type),
        }
        .create_dynamic_sample();

        assert_eq!(
            deserialize_top_level_type(
                PlainArraySElemDefn::get_type(),
                &[
                    0x00, 0x07, 0x00, 0x00, // CDR Header
                    6, 0, // array_sdefn: header: equiv_kind | padding
                    5, 0, // array_sdefn: header: element_flags
                    1, 0, 0, 0,  // array_sdefn: array_bound_seq: length
                    20, // array_sdefn: array_bound_seq: value
                    4,  // array_sdefn: element_identifier: TkInt32Type
                ]
            )
            .unwrap(),
            expected
        );
    }

    #[test]
    fn serialize_type_information() {
        let dynamic_data = TypeInformation {
            minimal: TypeIdentifierWithDependencies {
                typeid_with_size: TypeIdentifierWithSize {
                    type_id: TypeIdentifier::EkComplete {
                        equivalence_hash: [1; 14],
                    },
                    typeobject_serialized_size: 100,
                },
                dependent_typeid_count: 0,
                dependent_typeids: Vec::new(),
            },
            complete: TypeIdentifierWithDependencies {
                typeid_with_size: TypeIdentifierWithSize {
                    type_id: TypeIdentifier::EkComplete {
                        equivalence_hash: [1; 14],
                    },
                    typeobject_serialized_size: 100,
                },
                dependent_typeid_count: 0,
                dependent_typeids: Vec::new(),
            },
        }
        .create_dynamic_sample();
        let buffer = serialize_without_header_cdr2_le(Vec::new(), &dynamic_data).unwrap();

        let expected = [
            88, 0, 0, 0, // dheader
            0x01, 0x10, 0, 0b101_0000, // Emheader: minimal
            36, 0, 0, 0, // DHeader (replaces nextint)
            20, 0, 0, 0,    // DHeader
            0xF2, // Discriminator
            1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, // equivalence_hash
            0, // Padding
            100, 0, 0, 0, // typeobject_serialized_size
            0, 0, 0, 0, //dependent_typeid_count
            4, 0, 0, 0, // dependent_typeids: Dheader
            0, 0, 0, 0, // dependent_typeids Length
            // complete
            0x02, 0x10, 0, 0b101_0000, // Emheader: minimal
            36, 0, 0, 0, // DHeader (replaces nextint)
            20, 0, 0, 0,    // DHeader
            0xF2, // Discriminator
            1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, // equivalence_hash
            0, // Padding
            100, 0, 0, 0, // typeobject_serialized_size
            0, 0, 0, 0, //dependent_typeid_count
            4, 0, 0, 0, // dependent_typeids: Dheader
            0, 0, 0, 0, // dependent_typeids Length
        ];

        assert_eq!(buffer, expected.to_vec())
    }

    #[test]
    fn serialize_type_identifier_with_dependencies() {
        let dynamic_data = TypeIdentifierWithDependencies {
            typeid_with_size: TypeIdentifierWithSize {
                type_id: TypeIdentifier::EkComplete {
                    equivalence_hash: [1; 14],
                },
                typeobject_serialized_size: 100,
            },
            dependent_typeid_count: 0,
            dependent_typeids: Vec::new(),
        }
        .create_dynamic_sample();
        let buffer = serialize_without_header_cdr2_le(Vec::new(), &dynamic_data).unwrap();

        let expected = [
            36, 0, 0, 0, // DHeader
            20, 0, 0, 0,    // DHeader
            0xF2, // Discriminator
            1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, // equivalence_hash
            0, // Padding
            100, 0, 0, 0, // typeobject_serialized_size
            0, 0, 0, 0, //dependent_typeid_count
            4, 0, 0, 0, // dependent_typeids Dheader
            0, 0, 0, 0, // dependent_typeids Length
        ];

        assert_eq!(buffer, expected.to_vec())
    }

    #[test]
    fn serialize_type_identifier_with_size() {
        let dynamic_data = TypeIdentifierWithSize {
            type_id: TypeIdentifier::EkComplete {
                equivalence_hash: [1; 14],
            },
            typeobject_serialized_size: 100,
        }
        .create_dynamic_sample();
        let buffer = serialize_without_header_cdr2_le(Vec::new(), &dynamic_data).unwrap();

        let expected = [
            20, 0, 0, 0,    // DHeader
            0xF2, // Discriminator
            1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, // equivalence_hash
            0, // Padding
            100, 0, 0, 0, // typeobject_serialized_size
        ];

        assert_eq!(buffer, expected.to_vec())
    }

    #[test]
    fn serialize_type_identifier() {
        let dynamic_data = TypeIdentifier::EkComplete {
            equivalence_hash: [1; 14],
        }
        .create_dynamic_sample();
        let buffer = serialize_without_header_cdr2_le(Vec::new(), &dynamic_data).unwrap();

        let expected = [
            0xF2, // Discriminator
            1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, // equivalence_hash
        ];

        assert_eq!(buffer, expected.to_vec())
    }

    #[test]
    fn serialize_complete_member_detail() {
        let dynamic_data = CompleteMemberDetail {
            name: String::from("a"),
            ann_builtin: None,
            ann_custom: None,
        }
        .create_dynamic_sample();

        let buffer = serialize_without_header_cdr2_le(Vec::new(), &dynamic_data).unwrap();

        let expected = [
            2, 0, 0, 0, // Name length
            b'a', 0, // Name
            0, // ann_builtin optional not present
            0, // ann_custom optional not present
        ];

        assert_eq!(buffer, expected.to_vec())
    }

    #[test]
    fn test_sequence_and_array_type_identifier_equivalence_kind() {
        use super::*;
        use crate::xtypes::dynamic_type::DynamicTypeBuilderFactory;

        let int32_type = DynamicTypeBuilderFactory::get_primitive_type(TypeKind::INT32);

        // Sequence of primitive -> EK_BOTH
        let seq_primitive_type =
            DynamicTypeBuilderFactory::create_sequence_type(int32_type, 10).build();
        let type_id = TypeIdentifier::from(&seq_primitive_type);
        if let TypeIdentifier::TiPlainSequenceSmall { seq_sdefn } = type_id {
            assert_eq!(seq_sdefn.header.equiv_kind, EK_BOTH);
        } else {
            panic!("Expected TiPlainSequenceSmall");
        }

        // Sequence of sequence of primitive -> EK_BOTH
        let seq_seq_primitive_type =
            DynamicTypeBuilderFactory::create_sequence_type(seq_primitive_type, 5).build();
        let type_id = TypeIdentifier::from(&seq_seq_primitive_type);
        if let TypeIdentifier::TiPlainSequenceSmall { seq_sdefn } = type_id {
            assert_eq!(seq_sdefn.header.equiv_kind, EK_BOTH);
        } else {
            panic!("Expected TiPlainSequenceSmall");
        }

        // Array of primitive -> EK_BOTH
        let int32_type = DynamicTypeBuilderFactory::get_primitive_type(TypeKind::INT32);
        let array_primitive_type =
            DynamicTypeBuilderFactory::create_array_type(int32_type, vec![10].leak()).build();
        let type_id = TypeIdentifier::from(&array_primitive_type);
        if let TypeIdentifier::TiPlainArraySmall { array_sdefn } = type_id {
            assert_eq!(array_sdefn.header.equiv_kind, EK_BOTH);
        } else {
            panic!("Expected TiPlainArraySmall");
        }

        // Sequence of struct -> EK_COMPLETE
        let struct_type = DynamicTypeBuilderFactory::create_type(TypeDescriptor {
            kind: TypeKind::STRUCTURE,
            name: "MyStruct",
            base_type: None,
            discriminator_type: None,
            bound: &[],
            element_type: None,
            key_element_type: None,
            extensibility_kind: ExtensibilityKind::Final,
            is_nested: false,
            is_autoid_hash: false,
        })
        .build();

        let seq_struct_type =
            DynamicTypeBuilderFactory::create_sequence_type(struct_type, 10).build();
        let type_id = TypeIdentifier::from(&seq_struct_type);
        if let TypeIdentifier::TiPlainSequenceSmall { seq_sdefn } = type_id {
            assert_eq!(seq_sdefn.header.equiv_kind, EK_COMPLETE);
        } else {
            panic!("Expected TiPlainSequenceSmall");
        }
    }

    #[test]
    fn test_union_discriminator_is_must_understand() {
        use super::*;
        use crate::xtypes::type_support::Type;

        #[derive(DdsType)]
        #[dust_dds(switch(i32))]
        enum TestUnion {
            #[dust_dds(case = 1)]
            A(i32),
        }

        let minimal_obj = MinimalTypeObject::from(TestUnion::TYPE);
        if let MinimalTypeObject::TkUnion { union_type } = minimal_obj {
            assert_ne!(
                union_type.discriminator.common.member_flags.0 & MEMBER_FLAG_IS_MUST_UNDERSTAND.0,
                0
            );
        } else {
            panic!("Expected TkUnion");
        }

        let complete_obj = CompleteTypeObject::from(TestUnion::TYPE);
        if let CompleteTypeObject::TkUnion { union_type } = complete_obj {
            assert_ne!(
                union_type.discriminator.common.member_flags.0 & MEMBER_FLAG_IS_MUST_UNDERSTAND.0,
                0
            );
        } else {
            panic!("Expected TkUnion");
        }
    }
}
