use super::error::XTypesError;
use crate::xtypes::{
    data_storage::{DataStorage, DataStorageMapping},
    error::XTypesResult,
    type_object::TypeObject,
    type_support::{Type, TypeSupport},
};
use alloc::{boxed::Box, collections::BTreeMap, string::String, vec, vec::Vec};

/// Represents a sequence bound.
pub type BoundSeq<'a> = &'a [u32];
/// Represents a sequence of include paths.
pub type IncludePathSeq = Vec<String>;
/// Represents the name of an object.
pub type ObjectName<'a> = &'a str;

// ---------- TypeKinds (begin) -------------------
/// Represents the kind of a dynamic type (e.g., primitive, constructed, or collection type).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum TypeKind {
    /// No type kind.
    NONE = 0x00,
    /// Boolean type kind.
    BOOLEAN = 0x01,
    /// Byte type kind.
    BYTE = 0x02,
    /// 16-bit signed integer type kind.
    INT16 = 0x03,
    /// 32-bit signed integer type kind.
    INT32 = 0x04,
    /// 64-bit signed integer type kind.
    INT64 = 0x05,
    /// 16-bit unsigned integer type kind.
    UINT16 = 0x06,
    /// 32-bit unsigned integer type kind.
    UINT32 = 0x07,
    /// 64-bit unsigned integer type kind.
    UINT64 = 0x08,
    /// 32-bit floating point type kind.
    FLOAT32 = 0x09,
    /// 64-bit floating point type kind.
    FLOAT64 = 0x0A,
    /// 128-bit floating point type kind.
    FLOAT128 = 0x0B,
    /// 8-bit signed integer type kind.
    INT8 = 0x0C,
    /// 8-bit unsigned integer type kind.
    UINT8 = 0x0D,
    /// 8-bit character type kind.
    CHAR8 = 0x10,
    /// 16-bit character type kind.
    CHAR16 = 0x11,
    /// 8-bit string type kind.
    STRING8 = 0x20,
    /// 16-bit string type kind.
    STRING16 = 0x21,
    /// Alias type kind.
    ALIAS = 0x30,
    /// Enumeration type kind.
    ENUM = 0x40,
    /// Bitmask type kind.
    BITMASK = 0x41,
    /// Annotation type kind.
    ANNOTATION = 0x50,
    /// Structure type kind.
    STRUCTURE = 0x51,
    /// Union type kind.
    UNION = 0x52,
    /// Bitset type kind.
    BITSET = 0x53,
    /// Sequence type kind.
    SEQUENCE = 0x60,
    /// Array type kind.
    ARRAY = 0x61,
    /// Map type kind.
    MAP = 0x62,
}

// ---------- TypeKinds (end) -------------------

/// A factory class used to create [`DynamicType`] and [`DynamicTypeBuilder`] instances.
pub struct DynamicTypeBuilderFactory;

impl DynamicTypeBuilderFactory {
    /// Returns a [`DynamicType`] representing the specified primitive type kind.
    pub fn get_primitive_type(kind: TypeKind) -> DynamicType<'static> {
        DynamicType {
            descriptor: Box::leak(Box::new(TypeDescriptor {
                kind,
                name: "",
                base_type: None,
                discriminator_type: None,
                bound: &[],
                element_type: None,
                key_element_type: None,
                extensibility_kind: ExtensibilityKind::Final,
                is_nested: false,
                is_autoid_hash: false,
            })),
            member_list: &[],
        }
    }

    /// Creates a [`DynamicTypeBuilder`] with the given type descriptor.
    pub fn create_type(descriptor: TypeDescriptor) -> DynamicTypeBuilder {
        DynamicTypeBuilder {
            descriptor,
            member_list: Vec::new(),
        }
    }

    /// Creates a [`DynamicTypeBuilder`] as a copy of an existing type.
    pub fn create_type_copy(r#_type: DynamicType) -> DynamicTypeBuilder {
        todo!()
    }

    /// Creates a [`DynamicTypeBuilder`] from a [`TypeObject`].
    pub fn create_type_w_type_object(_type_object: TypeObject) -> DynamicTypeBuilder {
        todo!()
    }

    /// Creates a [`DynamicTypeBuilder`] for a string type with the specified bound.
    pub fn create_string_type(bound: u32) -> DynamicTypeBuilder {
        DynamicTypeBuilder {
            descriptor: TypeDescriptor {
                kind: TypeKind::STRING8,
                name: "",
                base_type: None,
                discriminator_type: None,
                bound: vec![bound].leak(),
                element_type: None,
                key_element_type: None,
                extensibility_kind: ExtensibilityKind::Final,
                is_nested: false,
                is_autoid_hash: false,
            },
            member_list: Vec::new(),
        }
    }

    /// Creates a [`DynamicTypeBuilder`] for a wide string type with the specified bound.
    pub fn create_wstring_type(bound: u32) -> DynamicTypeBuilder {
        DynamicTypeBuilder {
            descriptor: TypeDescriptor {
                kind: TypeKind::STRING16,
                name: "",
                base_type: None,
                discriminator_type: None,
                bound: vec![bound].leak(),
                element_type: None,
                key_element_type: None,
                extensibility_kind: ExtensibilityKind::Final,
                is_nested: false,
                is_autoid_hash: false,
            },
            member_list: Vec::new(),
        }
    }

    /// Creates a [`DynamicTypeBuilder`] for a sequence type with the specified element type and bound.
    pub fn create_sequence_type(
        element_type: DynamicType<'static>,
        bound: u32,
    ) -> DynamicTypeBuilder {
        DynamicTypeBuilder {
            descriptor: TypeDescriptor {
                kind: TypeKind::SEQUENCE,
                name: "",
                base_type: None,
                discriminator_type: None,
                bound: vec![bound].leak(),
                element_type: Some(element_type),
                key_element_type: None,
                extensibility_kind: ExtensibilityKind::Final,
                is_nested: false,
                is_autoid_hash: false,
            },
            member_list: Vec::new(),
        }
    }

    /// Creates a [`DynamicTypeBuilder`] for an array type with the specified element type and dimensions/bound.
    pub fn create_array_type(
        element_type: DynamicType<'static>,
        bound: BoundSeq<'static>,
    ) -> DynamicTypeBuilder {
        DynamicTypeBuilder {
            descriptor: TypeDescriptor {
                kind: TypeKind::ARRAY,
                name: "",
                base_type: None,
                discriminator_type: None,
                bound,
                element_type: Some(element_type),
                key_element_type: None,
                extensibility_kind: ExtensibilityKind::Final,
                is_nested: false,
                is_autoid_hash: false,
            },
            member_list: Vec::new(),
        }
    }

    /// Creates a [`DynamicTypeBuilder`] for a map type with the specified key, element type, and bound.
    pub fn create_map_type(
        _key_element_type: DynamicType,
        _element_type: DynamicType,
        _bound: u32,
    ) -> DynamicTypeBuilder {
        todo!()
    }

    /// Creates a [`DynamicTypeBuilder`] for a bitmask type with the specified bound.
    pub fn create_bitmask_type(_bound: u32) -> DynamicTypeBuilder {
        todo!()
    }

    /// Creates a [`DynamicTypeBuilder`] for a type defined at the specified URI.
    pub fn create_type_w_uri(
        _document_url: String,
        _type_name: String,
        _include_paths: Vec<String>,
    ) -> DynamicTypeBuilder {
        todo!()
    }

    #[cfg(feature = "xtypes-xml")]
    /// Creates a [`DynamicTypeBuilder`] for a type defined by the input XML.
    pub fn create_type_w_document(
        document: &str,
        type_name: &str,
        _include_paths: Vec<String>,
    ) -> XTypesResult<DynamicTypeBuilder> {
        let doc = roxmltree::Document::parse(document).map_err(|_| XTypesError::InvalidData)?;

        let path: Vec<&str> = type_name.split("::").collect();
        let (struct_name, module_path) = path.split_last().ok_or(XTypesError::InvalidData)?;

        let mut target_node = None;
        for node in doc.descendants() {
            if node.is_element()
                && (node.tag_name().name() == "struct"
                    || node.tag_name().name() == "union"
                    || node.tag_name().name() == "enum"
                    || node.tag_name().name() == "bitmask")
            {
                if let Some(name) = node.attribute("name") {
                    if &name == struct_name {
                        let mut current_node = node.parent();
                        let mut matches = true;
                        for &expected_mod in module_path.iter().rev() {
                            let mut found = false;
                            while let Some(parent) = current_node {
                                current_node = parent.parent();
                                if parent.is_element() && parent.tag_name().name() == "module" {
                                    if parent.attribute("name") == Some(expected_mod) {
                                        found = true;
                                    }
                                    break;
                                }
                            }
                            if !found {
                                matches = false;
                                break;
                            }
                        }
                        if matches {
                            target_node = Some(node);
                            break;
                        }
                    }
                }
            }
        }

        let target_node = target_node.ok_or(XTypesError::InvalidData)?;
        let is_union = target_node.tag_name().name() == "union";
        let is_enum = target_node.tag_name().name() == "enum";
        let is_bitmask = target_node.tag_name().name() == "bitmask";

        let ext_str = target_node
            .attribute("extensibility")
            .unwrap_or("appendable");
        let extensibility_kind = match ext_str {
            "final" => ExtensibilityKind::Final,
            "appendable" => ExtensibilityKind::Appendable,
            "mutable" => ExtensibilityKind::Mutable,
            _ => ExtensibilityKind::Appendable,
        };

        let parse_type_kind = |m_type: &str| -> XTypesResult<TypeKind> {
            match m_type {
                "boolean" => Ok(TypeKind::BOOLEAN),
                "byte" => Ok(TypeKind::BYTE),
                "char8" => Ok(TypeKind::CHAR8),
                "char16" => Ok(TypeKind::CHAR16),
                "int32" => Ok(TypeKind::INT32),
                "uint32" => Ok(TypeKind::UINT32),
                "int8" => Ok(TypeKind::INT8),
                "uint8" => Ok(TypeKind::UINT8),
                "int16" => Ok(TypeKind::INT16),
                "uint16" => Ok(TypeKind::UINT16),
                "int64" => Ok(TypeKind::INT64),
                "uint64" => Ok(TypeKind::UINT64),
                "float32" => Ok(TypeKind::FLOAT32),
                "float64" => Ok(TypeKind::FLOAT64),
                "float128" => Ok(TypeKind::FLOAT128),
                "string" => Ok(TypeKind::STRING8),
                "wstring" => Ok(TypeKind::STRING16),
                _ => Err(XTypesError::InvalidData),
            }
        };

        let resolve_member_type = |m_type: &str,
                                   non_basic_type_name: Option<&str>,
                                   string_max_length: Option<&str>,
                                   array_dimensions: Option<&str>,
                                   sequence_max_length: Option<&str>|
         -> XTypesResult<DynamicType> {
            let mut type_ptr: DynamicType = if m_type == "nonBasic" {
                let non_basic_name = non_basic_type_name.ok_or(XTypesError::InvalidData)?;
                let mut full_name = module_path.join("::");
                if !full_name.is_empty() {
                    full_name.push_str("::");
                }
                full_name.push_str(non_basic_name);
                let nested_builder =
                    Self::create_type_w_document(document, &full_name, _include_paths.clone())?;
                nested_builder.build()
            } else if m_type == "string" {
                let bound = string_max_length.unwrap_or("0").parse().unwrap_or(0);
                let builder = Self::create_string_type(bound);
                builder.build()
            } else if m_type == "wstring" {
                let bound = string_max_length.unwrap_or("0").parse().unwrap_or(0);
                let builder = Self::create_wstring_type(bound);
                builder.build()
            } else {
                let type_kind = parse_type_kind(m_type)?;
                Self::get_primitive_type(type_kind)
            };

            if let Some(seq_len_str) = sequence_max_length {
                let bound: i32 = seq_len_str.parse().map_err(|_| XTypesError::InvalidData)?;
                let bound_u32 = if bound < 0 { 0 } else { bound as u32 };
                let builder = Self::create_sequence_type(type_ptr, bound_u32);
                type_ptr = builder.build();
            }

            if let Some(dimensions) = array_dimensions {
                let dims: Vec<u32> = dimensions
                    .split(',')
                    .filter_map(|s| s.trim().parse().ok())
                    .collect();
                let builder = Self::create_array_type(type_ptr, dims.leak());
                type_ptr = builder.build();
            }
            Ok(type_ptr)
        };

        let mut discriminator_type = None;
        if is_union {
            for child in target_node.children() {
                if child.is_element() && child.tag_name().name() == "discriminator" {
                    let d_type = child.attribute("type").ok_or(XTypesError::InvalidData)?;
                    let non_basic_type_name = child.attribute("nonBasicTypeName");
                    let string_max_length = child.attribute("stringMaxLength");
                    let sequence_max_length = child.attribute("sequenceMaxLength");
                    let type_ptr = resolve_member_type(
                        d_type,
                        non_basic_type_name,
                        string_max_length,
                        None,
                        sequence_max_length,
                    )?;
                    discriminator_type = Some(type_ptr);
                    break;
                }
            }
        }
        if is_enum {
            discriminator_type = Some(Self::get_primitive_type(TypeKind::INT32));
        }

        let name: &'static str = Box::leak(type_name.to_string().into_boxed_str());
        let bit_bound: u32 = target_node
            .attribute("bitBound")
            .and_then(|s| s.parse().ok())
            .unwrap_or(32);
        let bound: &'static [u32] = if is_bitmask || is_enum {
            Box::leak(vec![bit_bound].into_boxed_slice())
        } else {
            &[]
        };

        let is_autoid_hash = target_node.attribute("autoid") == Some("hash");

        let descriptor = TypeDescriptor {
            kind: if is_union {
                TypeKind::UNION
            } else if is_enum {
                TypeKind::ENUM
            } else if is_bitmask {
                TypeKind::BITMASK
            } else {
                TypeKind::STRUCTURE
            },
            name,
            base_type: None,
            discriminator_type,
            bound,
            element_type: None,
            key_element_type: None,
            extensibility_kind,
            is_nested: target_node.attribute("nested") == Some("true"),
            is_autoid_hash,
        };

        let mut builder = Self::create_type(descriptor);
        let mut try_construct_kind = TryConstructKind::Discard;
        fn parse_try_construct_kind(
            node: &roxmltree::Node,
            try_construct_kind: &mut TryConstructKind,
        ) {
            *try_construct_kind = match node.attribute("tryConstruct") {
                Some("discard") => TryConstructKind::Discard,
                Some("use_default") => TryConstructKind::UseDefault,
                Some("trim") => TryConstructKind::Trim,
                _ => *try_construct_kind,
            };
        }

        let mut index = 0;
        let mut member_id = 0;
        if is_union {
            for child in target_node.children() {
                if child.is_element() && child.tag_name().name() == "discriminator" {
                    parse_try_construct_kind(&child, &mut try_construct_kind);
                    let is_key = child.attribute("key") == Some("true");
                    let member_desc = MemberDescriptor {
                        name: "discriminator",
                        id: member_id,
                        r#type: discriminator_type.expect("discriminator must be defined"),
                        default_value: None,
                        index,
                        label: &[],
                        try_construct_kind,
                        is_key,
                        is_optional: false,
                        is_must_understand: true,
                        is_shared: false,
                        is_default_label: false,
                        is_external: false,
                    };

                    builder.add_member(member_desc)?;
                    member_id += 1;
                    index += 1;
                }
                if child.is_element() && child.tag_name().name() == "case" {
                    let mut m_name = None;
                    let mut m_type = None;
                    let mut non_basic_type_name = None;
                    let mut string_max_length = None;
                    let mut array_dimensions = None;
                    let mut sequence_max_length = None;
                    let mut m_id = None;
                    let mut label: Vec<i32> = Vec::new();
                    let mut is_default_label = false;
                    let mut is_key = false;

                    for c in child.children() {
                        if c.is_element() && c.tag_name().name() == "caseDiscriminator" {
                            if let Some(val) = c.attribute("value") {
                                if val == "default" {
                                    is_default_label = true;
                                } else if let Some(hex) = val.strip_prefix("0x") {
                                    label.push(
                                        i32::from_str_radix(hex, 16)
                                            .map_err(|_| XTypesError::InvalidData)?,
                                    );
                                } else if let Ok(parsed_val) = val.parse::<i32>() {
                                    label.push(parsed_val);
                                } else if let Some(dt) = discriminator_type {
                                    if let Ok(enum_member) = dt.get_member_by_name(val) {
                                        let label_val = enum_member
                                            .descriptor
                                            .label
                                            .first()
                                            .copied()
                                            .unwrap_or(enum_member.get_id() as i32);
                                        label.push(label_val);
                                    } else {
                                        return Err(XTypesError::InvalidData);
                                    }
                                } else {
                                    return Err(XTypesError::InvalidData);
                                }
                            }
                        }
                        if c.is_element() && c.tag_name().name() == "member" {
                            m_name = c.attribute("name");
                            m_type = c.attribute("type");
                            is_key = c.attribute("key") == Some("true");
                            non_basic_type_name = c.attribute("nonBasicTypeName");
                            string_max_length = c.attribute("stringMaxLength");
                            array_dimensions = c.attribute("arrayDimensions");
                            sequence_max_length = c.attribute("sequenceMaxLength");
                            parse_try_construct_kind(&c, &mut try_construct_kind);
                            if let Some(id_str) = c.attribute("id") {
                                m_id = Some(if let Some(hex) = id_str.strip_prefix("0x") {
                                    u32::from_str_radix(hex, 16)
                                        .map_err(|_| XTypesError::InvalidData)?
                                } else {
                                    id_str
                                        .parse::<u32>()
                                        .map_err(|_| XTypesError::InvalidData)?
                                });
                            } else if let Some(hashid_name) = c.attribute("hashid") {
                                let target = if hashid_name.is_empty() {
                                    m_name.ok_or(XTypesError::InvalidData)?
                                } else {
                                    hashid_name
                                };
                                let hash = md5::compute(target.as_bytes());
                                m_id = Some(
                                    u32::from_le_bytes([hash[0], hash[1], hash[2], hash[3]])
                                        & 0x0FFF_FFFF,
                                );
                            }
                        }
                    }

                    let label = label.leak();

                    let m_name_unwrapped = m_name.ok_or(XTypesError::InvalidData)?;
                    let m_id = m_id.unwrap_or_else(|| {
                        if is_autoid_hash {
                            let hash = md5::compute(m_name_unwrapped.as_bytes());
                            u32::from_le_bytes([hash[0], hash[1], hash[2], hash[3]]) & 0x0FFF_FFFF
                        } else {
                            member_id
                        }
                    });

                    let type_ptr = resolve_member_type(
                        m_type.ok_or(XTypesError::InvalidData)?,
                        non_basic_type_name,
                        string_max_length,
                        array_dimensions,
                        sequence_max_length,
                    )?;
                    let m_name_static = Box::leak(m_name_unwrapped.to_string().into_boxed_str());
                    let member_desc = MemberDescriptor {
                        name: m_name_static,
                        id: m_id,
                        r#type: type_ptr,
                        default_value: None,
                        index,
                        label,
                        try_construct_kind,
                        is_key,
                        is_optional: false,
                        is_must_understand: false,
                        is_shared: false,
                        is_default_label,
                        is_external: false,
                    };

                    builder.add_member(member_desc)?;
                    member_id += 1;
                    index += 1;
                }
            }
        } else if is_enum {
            for child in target_node.children() {
                if child.is_element() && child.tag_name().name() == "enumerator" {
                    let m_name = child.attribute("name").ok_or(XTypesError::InvalidData)?;
                    let value = child.attribute("value").ok_or(XTypesError::InvalidData)?;
                    let is_default_label = child.attribute("defaultLiteral") == Some("true");
                    parse_try_construct_kind(&child, &mut try_construct_kind);
                    let label =
                        Vec::leak(vec![value.parse().map_err(|_| XTypesError::InvalidData)?]);

                    let type_ptr: DynamicType = Self::get_primitive_type(TypeKind::INT32);
                    let m_name_static = Box::leak(m_name.to_string().into_boxed_str());

                    let member_desc = MemberDescriptor {
                        name: m_name_static,
                        id: member_id,
                        r#type: type_ptr,
                        default_value: None,
                        index: member_id,
                        label,
                        try_construct_kind,
                        is_key: false,
                        is_optional: false,
                        is_must_understand: false,
                        is_shared: false,
                        is_default_label,
                        is_external: false,
                    };

                    builder.add_member(member_desc)?;
                    member_id += 1;
                }
            }
        } else {
            for child in target_node.children() {
                if child.is_element() && child.tag_name().name() == "member" {
                    let m_name = child.attribute("name").ok_or(XTypesError::InvalidData)?;
                    let m_type = child.attribute("type").ok_or(XTypesError::InvalidData)?;
                    let non_basic_type_name = child.attribute("nonBasicTypeName");
                    let string_max_length = child.attribute("stringMaxLength");
                    let array_dimensions = child.attribute("arrayDimensions");
                    let sequence_max_length = child.attribute("sequenceMaxLength");

                    let type_ptr = resolve_member_type(
                        m_type,
                        non_basic_type_name,
                        string_max_length,
                        array_dimensions,
                        sequence_max_length,
                    )?;
                    let m_name_static = Box::leak(m_name.to_string().into_boxed_str());

                    let m_id = if let Some(id_str) = child.attribute("id") {
                        if let Some(hex) = id_str.strip_prefix("0x") {
                            u32::from_str_radix(hex, 16).map_err(|_| XTypesError::InvalidData)?
                        } else {
                            id_str
                                .parse::<u32>()
                                .map_err(|_| XTypesError::InvalidData)?
                        }
                    } else if let Some(hashid_name) = child.attribute("hashid") {
                        let target = if hashid_name.is_empty() {
                            m_name
                        } else {
                            hashid_name
                        };
                        let hash = md5::compute(target.as_bytes());
                        u32::from_le_bytes([hash[0], hash[1], hash[2], hash[3]]) & 0x0FFF_FFFF
                    } else if is_autoid_hash {
                        let hash = md5::compute(m_name.as_bytes());
                        u32::from_le_bytes([hash[0], hash[1], hash[2], hash[3]]) & 0x0FFF_FFFF
                    } else {
                        member_id
                    };
                    parse_try_construct_kind(&child, &mut try_construct_kind);

                    let member_desc = MemberDescriptor {
                        name: m_name_static,
                        id: m_id,
                        r#type: type_ptr,
                        default_value: None,
                        index: member_id,
                        label: &[],
                        try_construct_kind,
                        is_key: child.attribute("key") == Some("true"),
                        is_optional: child.attribute("optional") == Some("true"),
                        is_must_understand: child.attribute("mustUnderstand") == Some("true")
                            || child.attribute("must_understand") == Some("true"),
                        is_shared: false,
                        is_default_label: false,
                        is_external: false,
                    };

                    builder.add_member(member_desc)?;
                    member_id += 1;
                }
            }
        }

        Ok(builder)
    }
}

/// Represents parameter name-value pairs.
pub type Parameters = BTreeMap<ObjectName<'static>, ObjectName<'static>>;

/// Defines how a type can be extended or modified in future versions.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ExtensibilityKind {
    /// Cannot be extended or modified.
    Final,
    /// Members can be appended to the end of the type in future versions.
    Appendable,
    /// Members can be added, removed, or reordered in future versions.
    Mutable,
}

/// Defines the behavior when constructing an object of a type that fails some validation or constraints.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TryConstructKind {
    /// Fall back to the default value.
    UseDefault,
    /// Discard the entire object or element.
    Discard,
    /// Trim the elements to fit the constraints.
    Trim,
}

/// Describes the properties and characteristics of a [`DynamicType`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TypeDescriptor {
    /// The kind of the type.
    pub kind: TypeKind,
    /// The name of the type.
    pub name: ObjectName<'static>,
    /// The base type if this type inherits from another type.
    pub base_type: Option<DynamicType<'static>>,
    /// The discriminator type if this type is a union.
    pub discriminator_type: Option<DynamicType<'static>>,
    /// The bound(s) of the type if it is a collection or string.
    pub bound: BoundSeq<'static>,
    /// The element type if this type is a collection.
    pub element_type: Option<DynamicType<'static>>,
    /// The key element type if this type is a map.
    pub key_element_type: Option<DynamicType<'static>>,
    /// The extensibility kind of the type.
    pub extensibility_kind: ExtensibilityKind,
    /// Indicates whether this is a nested type.
    pub is_nested: bool,
    /// Indicates whether member IDs are calculated using autoid hash.
    pub is_autoid_hash: bool,
}

/// Represents the unique identifier of a member.
pub type MemberId = u32;
/// Represents case labels for a union member.
pub type UnionCaseLabelSeq = &'static [i32];

/// Describes a member of a constructed type.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MemberDescriptor {
    /// The name of the member.
    pub name: ObjectName<'static>,
    /// The unique identifier of the member.
    pub id: MemberId,
    /// The dynamic type of the member.
    pub r#type: DynamicType<'static>,
    /// The optional default value of the member.
    pub default_value: Option<&'static str>,
    /// The index of the member within the parent type.
    pub index: u32,
    /// The union case labels if this member belongs to a union.
    pub label: UnionCaseLabelSeq,
    /// The construct fail action of the member.
    pub try_construct_kind: TryConstructKind,
    /// Indicates if the member is part of the type's key.
    pub is_key: bool,
    /// Indicates if the member is optional.
    pub is_optional: bool,
    /// Indicates if a receiver must understand this member to process the data.
    pub is_must_understand: bool,
    /// Indicates if the member is shared.
    pub is_shared: bool,
    /// Indicates if this is the default case for a union.
    pub is_default_label: bool,
    /// Indicates if the member is external (stored by reference).
    pub is_external: bool,
}

/// Represents a member of a [`DynamicType`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DynamicTypeMember {
    /// The descriptor describing this member.
    pub descriptor: MemberDescriptor,
}

impl DynamicTypeMember {
    /// Returns a reference to the member's descriptor.
    pub fn get_descriptor(&self) -> XTypesResult<&MemberDescriptor> {
        Ok(&self.descriptor)
    }
    // unsigned long get_annotation_count();
    // DDS::ReturnCode_t get_annotation(inout AnnotationDescriptor descriptor, in unsigned long idx);
    // unsigned long get_verbatim_text_count();
    // DDS::ReturnCode_t get_verbatim_text(inout VerbatimTextDescriptor descriptor, in unsigned long idx);

    /// Returns the unique member ID.
    pub fn get_id(&self) -> MemberId {
        self.descriptor.id
    }
    /// Returns the name of the member.
    pub fn get_name(&self) -> ObjectName<'static> {
        self.descriptor.name
    }
}

/// A builder class used to construct a [`DynamicType`].
pub struct DynamicTypeBuilder {
    descriptor: TypeDescriptor,
    member_list: Vec<DynamicTypeMember>,
}

impl DynamicTypeBuilder {
    /// Returns a reference to the type descriptor being built.
    pub fn get_descriptor(&self) -> XTypesResult<&TypeDescriptor> {
        Ok(&self.descriptor)
    }

    /// Returns the name of the type.
    pub fn get_name(&self) -> ObjectName<'static> {
        self.descriptor.name
    }

    /// Returns the kind of the type.
    pub fn get_kind(&self) -> TypeKind {
        self.descriptor.kind
    }

    /// Returns a mutable reference to a member by its name.
    pub fn get_member_by_name(
        &mut self,
        name: &ObjectName,
    ) -> XTypesResult<&mut DynamicTypeMember> {
        self.member_list
            .iter_mut()
            .find(|m| &m.descriptor.name == name)
            .ok_or(XTypesError::InvalidData)
    }

    /// Returns all members indexed by their names.
    pub fn get_all_members_by_name(
        &self,
    ) -> Result<Vec<(ObjectName<'static>, DynamicTypeMember)>, XTypesError> {
        todo!()
    }

    /// Returns a member by its ID.
    pub fn get_member(&self, _id: MemberId) -> Result<DynamicTypeMember, XTypesError> {
        todo!()
    }

    /// Returns all members indexed by their ID.
    pub fn get_all_members(
        &mut self,
    ) -> Result<Vec<(MemberId, &mut DynamicTypeMember)>, XTypesError> {
        Ok(self
            .member_list
            .iter_mut()
            .map(|m| (m.descriptor.id, m))
            .collect())
    }

    /// Returns the number of annotations on this type.
    pub fn get_annotation_count(&self) -> u32 {
        todo!()
    }

    /// Returns the annotation at the specified index.
    pub fn get_annotation(&self, _idx: u32) -> XTypesResult<()> {
        todo!()
    }

    /// Adds a member to the type.
    pub fn add_member(&mut self, descriptor: MemberDescriptor) -> XTypesResult<()> {
        if let TypeKind::ENUM
        | TypeKind::BITMASK
        | TypeKind::ANNOTATION
        | TypeKind::STRUCTURE
        | TypeKind::UNION
        | TypeKind::BITSET = self.descriptor.kind
        {
        } else {
            return Err(XTypesError::IllegalOperation);
        }

        self.member_list.push(DynamicTypeMember { descriptor });

        Ok(())
    }

    /// Applies an annotation descriptor to this type.
    pub fn apply_annotation(&mut self) -> XTypesResult<()> {
        todo!()
    }

    /// Builds and returns the constructed [`DynamicType`].
    pub fn build(self) -> DynamicType<'static> {
        DynamicType {
            descriptor: Box::leak(Box::new(self.descriptor)),
            member_list: Vec::leak(self.member_list),
        }
    }
}

/// Represents a data type's schema at runtime.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DynamicType<'a> {
    /// The type descriptor.
    pub descriptor: &'a TypeDescriptor,
    /// The list of members belonging to this type.
    pub member_list: &'a [DynamicTypeMember],
}

impl<'a> DynamicType<'a> {
    /// Returns the type descriptor.
    pub fn get_descriptor(&self) -> &TypeDescriptor {
        self.descriptor
    }
    /// Returns the name of the type.
    pub fn get_name(&self) -> ObjectName<'static> {
        self.descriptor.name
    }
    /// Returns the kind of the type.
    pub fn get_kind(&self) -> TypeKind {
        self.descriptor.kind
    }

    /// Retrieves a member by name.
    pub fn get_member_by_name(&self, name: ObjectName) -> Result<&DynamicTypeMember, XTypesError> {
        self.member_list
            .iter()
            .find(|m| m.get_name() == name)
            .ok_or(XTypesError::InvalidName)
    }

    // DDS::ReturnCode_t get_all_members_by_name(inout DynamicTypeMembersByName member);
    /// Retrieves a member by ID.
    pub fn get_member(&self, id: MemberId) -> Result<&DynamicTypeMember, XTypesError> {
        self.member_list
            .iter()
            .find(|m| m.get_id() == id)
            .ok_or(XTypesError::InvalidId(id))
    }

    // DDS::ReturnCode_t get_all_members(inout DynamicTypeMembersById member);

    /// Returns the total number of members in the type.
    pub fn get_member_count(&self) -> u32 {
        self.member_list.len() as u32
    }
    /// Retrieves a member by index.
    pub fn get_member_by_index(&self, index: u32) -> Result<&DynamicTypeMember, XTypesError> {
        self.member_list
            .get(index as usize)
            .ok_or(XTypesError::InvalidIndex(index))
    }

    /// Returns true if this type is a constructed / non-primitive dependent type.
    pub fn is_dependent_type(&self) -> bool {
        match self.get_kind() {
            TypeKind::STRUCTURE | TypeKind::UNION | TypeKind::ENUM | TypeKind::BITMASK => true,
            TypeKind::ARRAY | TypeKind::SEQUENCE => self
                .descriptor
                .element_type
                .as_ref()
                .is_some_and(|elem| elem.is_dependent_type()),
            TypeKind::ALIAS => self
                .descriptor
                .base_type
                .as_ref()
                .is_some_and(|base| base.is_dependent_type()),
            TypeKind::MAP => {
                let key_dep = self
                    .descriptor
                    .key_element_type
                    .as_ref()
                    .is_some_and(|k| k.is_dependent_type());
                let val_dep = self
                    .descriptor
                    .element_type
                    .as_ref()
                    .is_some_and(|v| v.is_dependent_type());
                key_dep || val_dep
            }
            _ => false,
        }
    }

    /// Returns true if this type has any direct or indirect constructed dependencies.
    pub fn has_dependencies(&self) -> bool {
        match self.get_kind() {
            TypeKind::STRUCTURE | TypeKind::UNION => self
                .member_list
                .iter()
                .any(|m| m.descriptor.r#type.is_dependent_type()),
            TypeKind::ARRAY | TypeKind::SEQUENCE => self
                .descriptor
                .element_type
                .as_ref()
                .is_some_and(|elem| elem.is_dependent_type()),
            _ => false,
        }
    }

    /// Returns all direct and indirect constructed types that this type depends on.
    pub fn get_dependencies(&self) -> Vec<DynamicType<'a>> {
        let mut deps = Vec::new();
        self.collect_dependencies(&mut deps);
        deps
    }

    fn collect_dependencies(&self, out: &mut Vec<DynamicType<'a>>) {
        match self.get_kind() {
            TypeKind::STRUCTURE | TypeKind::UNION => {
                for member in self.member_list {
                    member
                        .descriptor
                        .r#type
                        .collect_type_and_nested_dependencies(out);
                }
            }
            TypeKind::ARRAY | TypeKind::SEQUENCE => {
                if let Some(element_type) = &self.descriptor.element_type {
                    element_type.collect_type_and_nested_dependencies(out);
                }
            }
            _ => {}
        }
    }

    fn collect_type_and_nested_dependencies(&self, out: &mut Vec<DynamicType<'a>>) {
        match self.get_kind() {
            TypeKind::STRUCTURE | TypeKind::UNION | TypeKind::ENUM | TypeKind::BITMASK => {
                if !out.iter().any(|existing| existing == self) {
                    out.push(*self);
                    self.collect_dependencies(out);
                }
            }
            TypeKind::ARRAY | TypeKind::SEQUENCE => {
                if let Some(element_type) = &self.descriptor.element_type {
                    element_type.collect_type_and_nested_dependencies(out);
                }
            }
            _ => {}
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub(crate) struct MemberDataStorage {
    pub id: MemberId,
    pub value: DataStorage,
}

/// A factory class used to instantiate [`DynamicData`] samples.
pub struct DynamicDataFactory;

impl DynamicDataFactory {
    /// Creates a [`DynamicData`] sample of the specified [`DynamicType`].
    pub fn create_data<'a>(r#type: DynamicType<'a>) -> DynamicData<'a> {
        DynamicData {
            r#type,
            abstract_data: Vec::with_capacity(r#type.member_list.len()),
        }
    }
}

/// Represents a data sample conforming to a [`DynamicType`] schema.
#[derive(Clone)]
pub struct DynamicData<'a> {
    pub(crate) r#type: DynamicType<'a>,
    pub(crate) abstract_data: Vec<MemberDataStorage>,
}

impl<'a> core::fmt::Debug for DynamicData<'a> {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("DynamicData")
            .field("abstract_data", &self.abstract_data)
            .finish()
    }
}

impl<'a> PartialEq for DynamicData<'a> {
    fn eq(&self, other: &Self) -> bool {
        if self.abstract_data.len() != other.abstract_data.len() {
            return false;
        }
        self.abstract_data
            .iter()
            .all(|m| other.get_storage(m.id) == Some(&m.value))
    }
}

impl<'a> DynamicData<'a> {
    fn get_storage(&self, id: MemberId) -> Option<&DataStorage> {
        self.abstract_data
            .iter()
            .find(|m| m.id == id)
            .map(|m| &m.value)
    }

    fn get_storage_mut(&mut self, id: MemberId) -> Option<&mut DataStorage> {
        self.abstract_data
            .iter_mut()
            .find(|m| m.id == id)
            .map(|m| &mut m.value)
    }

    fn insert_storage(&mut self, id: MemberId, value: DataStorage) {
        if let Some(m) = self.abstract_data.iter_mut().find(|m| m.id == id) {
            m.value = value;
        } else {
            self.abstract_data.push(MemberDataStorage { id, value });
        }
    }

    fn remove_storage(&mut self, id: MemberId) -> Option<DataStorage> {
        if let Some(idx) = self.abstract_data.iter().position(|m| m.id == id) {
            Some(self.abstract_data.swap_remove(idx).value)
        } else {
            None
        }
    }

    /// Returns the [`DynamicType`] of this data sample.
    pub const fn r#type(&self) -> DynamicType<'a> {
        self.r#type
    }

    /// Returns a reference to the descriptor of the specified member.
    pub fn get_descriptor(&self, id: MemberId) -> XTypesResult<&MemberDescriptor> {
        if let Ok(x) = self.r#type.get_member(id) {
            Ok(&x.descriptor)
        } else if let Some(b) = &self.r#type.descriptor.base_type {
            b.get_member(id)?.get_descriptor()
        } else {
            Err(XTypesError::InvalidId(id))
        }
    }

    /// Sets the descriptor of the specified member.
    pub fn set_descriptor(&mut self, _id: MemberId, _value: MemberDescriptor) -> XTypesResult<()> {
        todo!()
    }

    /// Retrieves the member ID corresponding to the given member name.
    pub fn get_member_id_by_name(&self, name: &str) -> Option<MemberId> {
        self.r#type
            .get_member_by_name(name)
            .ok()
            .map(|m| m.get_id())
    }

    /// Retrieves the member ID at the specified index.
    pub fn get_member_id_at_index(&self, index: u32) -> XTypesResult<MemberId> {
        self.abstract_data
            .get(index as usize)
            .map(|m| m.id)
            .ok_or(XTypesError::InvalidIndex(index))
    }

    /// Returns the number of items/members in this data sample.
    pub fn get_item_count(&self) -> u32 {
        self.abstract_data.len() as u32
    }

    /// Clears all member values from this data sample.
    pub fn clear_all_values(&mut self) -> XTypesResult<()> {
        self.abstract_data.clear();
        Ok(())
    }

    /// Clears all non-key member values from this data sample.
    pub fn clear_nonkey_values(&mut self) -> XTypesResult<()> {
        for index in 0..self.r#type.get_member_count() {
            let member = self.r#type.get_member_by_index(index)?;
            if !member.get_descriptor()?.is_key {
                let member_id = member.get_id();
                self.remove_storage(member_id);
            }
        }
        Ok(())
    }

    /// Clears the value of the specified member.
    pub fn clear_value(&mut self, id: MemberId) -> XTypesResult<()> {
        self.remove_storage(id).ok_or(XTypesError::InvalidId(id))?;
        Ok(())
    }

    /// Gets the `i32` value for the specified member.
    pub fn get_int32_value(&self, id: MemberId) -> XTypesResult<&i32> {
        if let DataStorage::Int32(d) = self.get_storage(id).ok_or(XTypesError::InvalidId(id))? {
            Ok(d)
        } else {
            Err(XTypesError::InvalidType)
        }
    }

    /// Sets the `i32` value for the specified member.
    pub fn set_int32_value(&mut self, id: MemberId, value: i32) -> XTypesResult<()> {
        self.insert_storage(id, DataStorage::Int32(value));
        Ok(())
    }

    /// Gets the `u32` value for the specified member.
    pub fn get_uint32_value(&self, id: MemberId) -> XTypesResult<&u32> {
        if let DataStorage::UInt32(d) = self.get_storage(id).ok_or(XTypesError::InvalidId(id))? {
            Ok(d)
        } else {
            Err(XTypesError::InvalidType)
        }
    }

    /// Sets the `u32` value for the specified member.
    pub fn set_uint32_value(&mut self, id: MemberId, value: u32) -> XTypesResult<()> {
        self.insert_storage(id, DataStorage::UInt32(value));
        Ok(())
    }

    /// Gets the `i8` value for the specified member.
    pub fn get_int8_value(&self, id: MemberId) -> XTypesResult<&i8> {
        if let DataStorage::Int8(d) = self.get_storage(id).ok_or(XTypesError::InvalidId(id))? {
            Ok(d)
        } else {
            Err(XTypesError::InvalidType)
        }
    }

    /// Sets the `i8` value for the specified member.
    pub fn set_int8_value(&mut self, id: MemberId, value: i8) -> XTypesResult<()> {
        self.insert_storage(id, DataStorage::Int8(value));
        Ok(())
    }

    /// Gets the `u8` value for the specified member.
    pub fn get_uint8_value(&self, id: MemberId) -> XTypesResult<&u8> {
        if let DataStorage::UInt8(d) = self.get_storage(id).ok_or(XTypesError::InvalidId(id))? {
            Ok(d)
        } else {
            Err(XTypesError::InvalidType)
        }
    }

    /// Sets the `u8` value for the specified member.
    pub fn set_uint8_value(&mut self, id: MemberId, value: u8) -> XTypesResult<()> {
        self.insert_storage(id, DataStorage::UInt8(value));
        Ok(())
    }

    /// Gets the `i16` value for the specified member.
    pub fn get_int16_value(&self, id: MemberId) -> XTypesResult<&i16> {
        if let DataStorage::Int16(d) = self.get_storage(id).ok_or(XTypesError::InvalidId(id))? {
            Ok(d)
        } else {
            Err(XTypesError::InvalidType)
        }
    }

    /// Sets the `i16` value for the specified member.
    pub fn set_int16_value(&mut self, id: MemberId, value: i16) -> XTypesResult<()> {
        self.insert_storage(id, DataStorage::Int16(value));
        Ok(())
    }

    /// Gets the `u16` value for the specified member.
    pub fn get_uint16_value(&self, id: MemberId) -> XTypesResult<&u16> {
        if let DataStorage::UInt16(d) = self.get_storage(id).ok_or(XTypesError::InvalidId(id))? {
            Ok(d)
        } else {
            Err(XTypesError::InvalidType)
        }
    }

    /// Sets the `u16` value for the specified member.
    pub fn set_uint16_value(&mut self, id: MemberId, value: u16) -> XTypesResult<()> {
        self.insert_storage(id, DataStorage::UInt16(value));
        Ok(())
    }

    /// Gets the `i64` value for the specified member.
    pub fn get_int64_value(&self, id: MemberId) -> XTypesResult<&i64> {
        if let DataStorage::Int64(d) = self.get_storage(id).ok_or(XTypesError::InvalidId(id))? {
            Ok(d)
        } else {
            Err(XTypesError::InvalidType)
        }
    }

    /// Sets the `i64` value for the specified member.
    pub fn set_int64_value(&mut self, id: MemberId, value: i64) -> XTypesResult<()> {
        self.insert_storage(id, DataStorage::Int64(value));
        Ok(())
    }

    /// Gets the `u64` value for the specified member.
    pub fn get_uint64_value(&self, id: MemberId) -> XTypesResult<&u64> {
        if let DataStorage::UInt64(d) = self.get_storage(id).ok_or(XTypesError::InvalidId(id))? {
            Ok(d)
        } else {
            Err(XTypesError::InvalidType)
        }
    }

    /// Sets the `u64` value for the specified member.
    pub fn set_uint64_value(&mut self, id: MemberId, value: u64) -> XTypesResult<()> {
        self.insert_storage(id, DataStorage::UInt64(value));
        Ok(())
    }

    /// Gets the `f32` value for the specified member.
    pub fn get_float32_value(&self, id: MemberId) -> XTypesResult<&f32> {
        if let DataStorage::Float32(d) = self.get_storage(id).ok_or(XTypesError::InvalidId(id))? {
            Ok(d)
        } else {
            Err(XTypesError::InvalidType)
        }
    }

    /// Sets the `f32` value for the specified member.
    pub fn set_float32_value(&mut self, id: MemberId, value: f32) -> XTypesResult<()> {
        self.insert_storage(id, DataStorage::Float32(value));
        Ok(())
    }

    /// Gets the `f64` value for the specified member.
    pub fn get_float64_value(&self, id: MemberId) -> XTypesResult<&f64> {
        if let DataStorage::Float64(d) = self.get_storage(id).ok_or(XTypesError::InvalidId(id))? {
            Ok(d)
        } else {
            Err(XTypesError::InvalidType)
        }
    }

    /// Sets the `f64` value for the specified member.
    pub fn set_float64_value(&mut self, id: MemberId, value: f64) -> XTypesResult<()> {
        self.insert_storage(id, DataStorage::Float64(value));
        Ok(())
    }

    /// Gets the `i128` (representing `float128`) value for the specified member.
    pub fn get_float128_value(&self, id: MemberId) -> XTypesResult<&i128> {
        if let DataStorage::Float128(d) = self.get_storage(id).ok_or(XTypesError::InvalidId(id))? {
            Ok(d)
        } else {
            Err(XTypesError::InvalidType)
        }
    }

    /// Sets the `i128` (representing `float128`) value for the specified member.
    pub fn set_float128_value(&mut self, id: MemberId, value: i128) -> XTypesResult<()> {
        self.insert_storage(id, DataStorage::Float128(value));
        Ok(())
    }

    /// Gets the `char` (representing `char8`) value for the specified member.
    pub fn get_char8_value(&self, id: MemberId) -> XTypesResult<&char> {
        if let DataStorage::Char8(d) = self.get_storage(id).ok_or(XTypesError::InvalidId(id))? {
            Ok(d)
        } else {
            Err(XTypesError::InvalidType)
        }
    }

    /// Sets the `char` (representing `char8`) value for the specified member.
    pub fn set_char8_value(&mut self, id: MemberId, value: char) -> XTypesResult<()> {
        self.insert_storage(id, DataStorage::Char8(value));
        Ok(())
    }

    /// Gets the byte (8-bit unsigned integer) value for the specified member.
    pub fn get_byte_value(&self, id: MemberId) -> XTypesResult<&u8> {
        if let DataStorage::UInt8(d) = self.get_storage(id).ok_or(XTypesError::InvalidId(id))? {
            Ok(d)
        } else {
            Err(XTypesError::InvalidType)
        }
    }

    /// Sets the byte (8-bit unsigned integer) value for the specified member.
    pub fn set_byte_value(&mut self, id: MemberId, value: u8) -> XTypesResult<()> {
        self.insert_storage(id, DataStorage::UInt8(value));
        Ok(())
    }

    /// Gets the `bool` value for the specified member.
    pub fn get_boolean_value(&self, id: MemberId) -> XTypesResult<&bool> {
        if let DataStorage::Boolean(d) = self.get_storage(id).ok_or(XTypesError::InvalidId(id))? {
            Ok(d)
        } else {
            Err(XTypesError::InvalidType)
        }
    }

    /// Sets the `bool` value for the specified member.
    pub fn set_boolean_value(&mut self, id: MemberId, value: bool) -> XTypesResult<()> {
        self.insert_storage(id, value.into_storage());
        Ok(())
    }

    /// Gets the `String` value for the specified member.
    pub fn get_string_value(&self, id: MemberId) -> XTypesResult<&String> {
        if let DataStorage::String(d) = self.get_storage(id).ok_or(XTypesError::InvalidId(id))? {
            Ok(d)
        } else {
            Err(XTypesError::InvalidType)
        }
    }

    /// Sets the `String` value for the specified member.
    pub fn set_string_value(&mut self, id: MemberId, value: String) -> XTypesResult<()> {
        self.insert_storage(id, DataStorage::String(value));
        Ok(())
    }

    /// Gets the complex (nested `DynamicData`) value for the specified member.
    pub fn get_complex_value(&self, id: MemberId) -> XTypesResult<&DynamicData<'static>> {
        if let DataStorage::ComplexValue(d) =
            self.get_storage(id).ok_or(XTypesError::InvalidId(id))?
        {
            Ok(d)
        } else {
            Err(XTypesError::InvalidType)
        }
    }

    /// Gets the raw data kind/storage for the specified member.
    pub fn get_data_kind(&self, id: MemberId) -> XTypesResult<&DataStorage> {
        self.get_storage(id).ok_or(XTypesError::InvalidId(id))
    }

    /// Sets the complex (nested `DynamicData`) value for the specified member.
    pub fn set_complex_value(
        &mut self,
        id: MemberId,
        value: DynamicData<'static>,
    ) -> XTypesResult<()> {
        self.insert_storage(id, DataStorage::ComplexValue(value));
        Ok(())
    }

    /// Gets a slice of `i32` values for the specified sequence/array member.
    pub fn get_int32_values(&self, id: MemberId) -> XTypesResult<&[i32]> {
        if let DataStorage::SequenceInt32(d) =
            self.get_storage(id).ok_or(XTypesError::InvalidId(id))?
        {
            Ok(d.as_slice())
        } else {
            Err(XTypesError::InvalidType)
        }
    }

    /// Sets a sequence of `i32` values for the specified member.
    pub fn set_int32_values(&mut self, id: MemberId, value: Vec<i32>) -> XTypesResult<()> {
        self.insert_storage(id, DataStorage::SequenceInt32(value));
        Ok(())
    }

    /// Gets a slice of `u32` values for the specified sequence/array member.
    pub fn get_uint32_values(&self, id: MemberId) -> XTypesResult<&[u32]> {
        if let DataStorage::SequenceUInt32(d) =
            self.get_storage(id).ok_or(XTypesError::InvalidId(id))?
        {
            Ok(d.as_slice())
        } else {
            Err(XTypesError::InvalidType)
        }
    }

    /// Sets a sequence of `u32` values for the specified member.
    pub fn set_uint32_values(&mut self, id: MemberId, value: Vec<u32>) -> XTypesResult<()> {
        self.insert_storage(id, DataStorage::SequenceUInt32(value));
        Ok(())
    }

    /// Gets a slice of `i16` values for the specified sequence/array member.
    pub fn get_int16_values(&self, id: MemberId) -> XTypesResult<&[i16]> {
        if let DataStorage::SequenceInt16(d) =
            self.get_storage(id).ok_or(XTypesError::InvalidId(id))?
        {
            Ok(d.as_slice())
        } else {
            Err(XTypesError::InvalidType)
        }
    }

    /// Sets a sequence of `i16` values for the specified member.
    pub fn set_int16_values(&mut self, id: MemberId, value: Vec<i16>) -> XTypesResult<()> {
        self.insert_storage(id, DataStorage::SequenceInt16(value));
        Ok(())
    }

    /// Gets a slice of `u16` values for the specified sequence/array member.
    pub fn get_uint16_values(&self, id: MemberId) -> XTypesResult<&[u16]> {
        if let DataStorage::SequenceUInt16(d) =
            self.get_storage(id).ok_or(XTypesError::InvalidId(id))?
        {
            Ok(d.as_slice())
        } else {
            Err(XTypesError::InvalidType)
        }
    }

    /// Sets a sequence of `u16` values for the specified member.
    pub fn set_uint16_values(&mut self, id: MemberId, value: Vec<u16>) -> XTypesResult<()> {
        self.insert_storage(id, DataStorage::SequenceUInt16(value));
        Ok(())
    }

    /// Gets a slice of `i64` values for the specified sequence/array member.
    pub fn get_int64_values(&self, id: MemberId) -> XTypesResult<&[i64]> {
        if let DataStorage::SequenceInt64(d) =
            self.get_storage(id).ok_or(XTypesError::InvalidId(id))?
        {
            Ok(d.as_slice())
        } else {
            Err(XTypesError::InvalidType)
        }
    }

    /// Sets a sequence of `i64` values for the specified member.
    pub fn set_int64_values(&mut self, id: MemberId, value: Vec<i64>) -> XTypesResult<()> {
        self.insert_storage(id, DataStorage::SequenceInt64(value));
        Ok(())
    }

    /// Gets a slice of `u64` values for the specified sequence/array member.
    pub fn get_uint64_values(&self, id: MemberId) -> XTypesResult<&[u64]> {
        if let DataStorage::SequenceUInt64(d) =
            self.get_storage(id).ok_or(XTypesError::InvalidId(id))?
        {
            Ok(d.as_slice())
        } else {
            Err(XTypesError::InvalidType)
        }
    }

    /// Sets a sequence of `u64` values for the specified member.
    pub fn set_uint64_values(&mut self, id: MemberId, value: Vec<u64>) -> XTypesResult<()> {
        self.insert_storage(id, DataStorage::SequenceUInt64(value));
        Ok(())
    }

    /// Gets a slice of `f32` values for the specified sequence/array member.
    pub fn get_float32_values(&self, id: MemberId) -> XTypesResult<&[f32]> {
        if let DataStorage::SequenceFloat32(d) =
            self.get_storage(id).ok_or(XTypesError::InvalidId(id))?
        {
            Ok(d.as_slice())
        } else {
            Err(XTypesError::InvalidType)
        }
    }

    /// Sets a sequence of `f32` values for the specified member.
    pub fn set_float32_values(&mut self, id: MemberId, value: Vec<f32>) -> XTypesResult<()> {
        self.insert_storage(id, DataStorage::SequenceFloat32(value));
        Ok(())
    }

    /// Gets a slice of `f64` values for the specified sequence/array member.
    pub fn get_float64_values(&self, id: MemberId) -> XTypesResult<&[f64]> {
        if let DataStorage::SequenceFloat64(d) =
            self.get_storage(id).ok_or(XTypesError::InvalidId(id))?
        {
            Ok(d.as_slice())
        } else {
            Err(XTypesError::InvalidType)
        }
    }

    /// Sets a sequence of `f64` values for the specified member.
    pub fn set_float64_values(&mut self, id: MemberId, value: Vec<f64>) -> XTypesResult<()> {
        self.insert_storage(id, DataStorage::SequenceFloat64(value));
        Ok(())
    }

    /// Gets a slice of `i128` (representing `float128`) values for the specified sequence/array member.
    pub fn get_float128_values(&self, id: MemberId) -> XTypesResult<&[i128]> {
        if let DataStorage::SequenceFloat128(d) =
            self.get_storage(id).ok_or(XTypesError::InvalidId(id))?
        {
            Ok(d.as_slice())
        } else {
            Err(XTypesError::InvalidType)
        }
    }

    /// Sets a sequence of `i128` (representing `float128`) values for the specified member.
    pub fn set_float128_values(&mut self, id: MemberId, value: Vec<i128>) -> XTypesResult<()> {
        self.insert_storage(id, DataStorage::SequenceFloat128(value));
        Ok(())
    }

    /// Gets a slice of `char` (representing `char8`) values for the specified sequence/array member.
    pub fn get_char8_values(&self, id: MemberId) -> XTypesResult<&[char]> {
        if let DataStorage::SequenceChar8(d) =
            self.get_storage(id).ok_or(XTypesError::InvalidId(id))?
        {
            Ok(d.as_slice())
        } else {
            Err(XTypesError::InvalidType)
        }
    }

    /// Sets a sequence of `char` (representing `char8`) values for the specified member.
    pub fn set_char8_values(&mut self, id: MemberId, value: Vec<char>) -> XTypesResult<()> {
        self.insert_storage(id, DataStorage::SequenceChar8(value));
        Ok(())
    }

    /// Gets a slice of byte (8-bit unsigned integer) values for the specified sequence/array member.
    pub fn get_byte_values(&self, id: MemberId) -> XTypesResult<&[u8]> {
        if let DataStorage::SequenceUInt8(d) =
            self.get_storage(id).ok_or(XTypesError::InvalidId(id))?
        {
            Ok(d.as_slice())
        } else {
            Err(XTypesError::InvalidType)
        }
    }

    /// Sets a sequence of byte (8-bit unsigned integer) values for the specified member.
    pub fn set_byte_values(&mut self, id: MemberId, value: Vec<u8>) -> XTypesResult<()> {
        self.insert_storage(id, DataStorage::SequenceUInt8(value));
        Ok(())
    }

    /// Gets a slice of `bool` values for the specified sequence/array member.
    pub fn get_boolean_values(&self, id: MemberId) -> XTypesResult<&[bool]> {
        if let DataStorage::SequenceBoolean(d) =
            self.get_storage(id).ok_or(XTypesError::InvalidId(id))?
        {
            Ok(d.as_slice())
        } else {
            Err(XTypesError::InvalidType)
        }
    }

    /// Sets a sequence of `bool` values for the specified member.
    pub fn set_boolean_values(&mut self, id: MemberId, value: Vec<bool>) -> XTypesResult<()> {
        self.insert_storage(id, DataStorage::SequenceBoolean(value));
        Ok(())
    }

    /// Gets a slice of `String` values for the specified sequence/array member.
    pub fn get_string_values(&self, id: MemberId) -> XTypesResult<&[String]> {
        if let DataStorage::SequenceString(d) =
            self.get_storage(id).ok_or(XTypesError::InvalidId(id))?
        {
            Ok(d.as_slice())
        } else {
            Err(XTypesError::InvalidType)
        }
    }

    /// Sets a sequence of `String` values for the specified member.
    pub fn set_string_values(&mut self, id: MemberId, value: Vec<String>) -> XTypesResult<()> {
        self.insert_storage(id, DataStorage::SequenceString(value));
        Ok(())
    }

    // Custom functions
    /// Gets a slice of `u8` values for the specified sequence/array member.
    pub fn get_uint8_values(&self, id: MemberId) -> XTypesResult<&[u8]> {
        if let DataStorage::SequenceUInt8(d) =
            self.get_storage(id).ok_or(XTypesError::InvalidId(id))?
        {
            Ok(d.as_slice())
        } else {
            Err(XTypesError::InvalidType)
        }
    }

    /// Gets a slice of `i8` values for the specified sequence/array member.
    pub fn get_int8_values(&self, id: MemberId) -> XTypesResult<&[i8]> {
        if let DataStorage::SequenceInt8(d) =
            self.get_storage(id).ok_or(XTypesError::InvalidId(id))?
        {
            Ok(d.as_slice())
        } else {
            Err(XTypesError::InvalidType)
        }
    }

    /// Sets a sequence of `u8` values for the specified member.
    pub fn set_uint8_values(&mut self, id: MemberId, value: Vec<u8>) -> XTypesResult<()> {
        self.insert_storage(id, DataStorage::SequenceUInt8(value));
        Ok(())
    }

    /// Sets a sequence of `i8` values for the specified member.
    pub fn set_int8_values(&mut self, id: MemberId, value: Vec<i8>) -> XTypesResult<()> {
        self.insert_storage(id, DataStorage::SequenceInt8(value));
        Ok(())
    }

    /// Gets a slice of complex (nested `DynamicData`) values for the specified sequence/array member.
    pub fn get_complex_values(&self, id: MemberId) -> XTypesResult<&[DynamicData<'static>]> {
        if let DataStorage::SequenceComplexValue(d) =
            self.get_storage(id).ok_or(XTypesError::InvalidId(id))?
        {
            Ok(d.as_slice())
        } else {
            Err(XTypesError::InvalidType)
        }
    }

    /// Sets a sequence of complex (nested `DynamicData`) values for the specified member.
    pub fn set_complex_values(
        &mut self,
        id: MemberId,
        value: Vec<DynamicData<'static>>,
    ) -> XTypesResult<()> {
        self.insert_storage(id, DataStorage::SequenceComplexValue(value));
        Ok(())
    }

    /// Sets the value of the specified member to the given raw data storage.
    pub fn set_value(&mut self, id: MemberId, value: DataStorage) {
        self.insert_storage(id, value);
    }

    /// Gets the raw data storage for the specified member.
    pub fn get_value(&self, id: MemberId) -> XTypesResult<&DataStorage> {
        self.get_storage(id).ok_or(XTypesError::InvalidId(id))
    }

    /// Removes and returns the raw data storage for the specified member.
    pub fn remove_value(&mut self, id: MemberId) -> XTypesResult<DataStorage> {
        self.remove_storage(id).ok_or(XTypesError::InvalidId(id))
    }
}

impl Type for DynamicData<'static> {
    const TYPE: DynamicType<'static> = DynamicType {
        descriptor: &dust_dds::xtypes::dynamic_type::TypeDescriptor {
            kind: dust_dds::xtypes::dynamic_type::TypeKind::STRUCTURE,
            name: "",
            base_type: None,
            discriminator_type: None,
            bound: &[],
            element_type: None,
            key_element_type: None,
            extensibility_kind: dust_dds::xtypes::dynamic_type::ExtensibilityKind::Final,
            is_nested: false,
            is_autoid_hash: false,
        },
        member_list: &[],
    };
}
impl TypeSupport for DynamicData<'static> {
    fn create_sample(src: &mut DynamicData<'static>) -> Option<Self> {
        Some(src.clone())
    }

    fn create_dynamic_sample(self) -> DynamicData<'static> {
        self
    }
}

fn get_sequence_len(storage: &DataStorage) -> Option<usize> {
    match storage {
        DataStorage::SequenceUInt8(v) => Some(v.len()),
        DataStorage::SequenceInt8(v) => Some(v.len()),
        DataStorage::SequenceUInt16(v) => Some(v.len()),
        DataStorage::SequenceInt16(v) => Some(v.len()),
        DataStorage::SequenceInt32(v) => Some(v.len()),
        DataStorage::SequenceUInt32(v) => Some(v.len()),
        DataStorage::SequenceInt64(v) => Some(v.len()),
        DataStorage::SequenceUInt64(v) => Some(v.len()),
        DataStorage::SequenceFloat32(v) => Some(v.len()),
        DataStorage::SequenceFloat64(v) => Some(v.len()),
        DataStorage::SequenceFloat128(v) => Some(v.len()),
        DataStorage::SequenceChar8(v) => Some(v.len()),
        DataStorage::SequenceBoolean(v) => Some(v.len()),
        DataStorage::SequenceString(v) => Some(v.len()),
        DataStorage::SequenceComplexValue(v) => Some(v.len()),
        _ => None,
    }
}

fn reset_sequence_to_empty(storage: &mut DataStorage) {
    match storage {
        DataStorage::SequenceUInt8(v) => v.clear(),
        DataStorage::SequenceInt8(v) => v.clear(),
        DataStorage::SequenceUInt16(v) => v.clear(),
        DataStorage::SequenceInt16(v) => v.clear(),
        DataStorage::SequenceInt32(v) => v.clear(),
        DataStorage::SequenceUInt32(v) => v.clear(),
        DataStorage::SequenceInt64(v) => v.clear(),
        DataStorage::SequenceUInt64(v) => v.clear(),
        DataStorage::SequenceFloat32(v) => v.clear(),
        DataStorage::SequenceFloat64(v) => v.clear(),
        DataStorage::SequenceFloat128(v) => v.clear(),
        DataStorage::SequenceChar8(v) => v.clear(),
        DataStorage::SequenceBoolean(v) => v.clear(),
        DataStorage::SequenceString(v) => v.clear(),
        DataStorage::SequenceComplexValue(v) => v.clear(),
        _ => {}
    }
}

fn truncate_sequence(storage: &mut DataStorage, bound: usize) {
    match storage {
        DataStorage::SequenceUInt8(v) => v.truncate(bound),
        DataStorage::SequenceInt8(v) => v.truncate(bound),
        DataStorage::SequenceUInt16(v) => v.truncate(bound),
        DataStorage::SequenceInt16(v) => v.truncate(bound),
        DataStorage::SequenceInt32(v) => v.truncate(bound),
        DataStorage::SequenceUInt32(v) => v.truncate(bound),
        DataStorage::SequenceInt64(v) => v.truncate(bound),
        DataStorage::SequenceUInt64(v) => v.truncate(bound),
        DataStorage::SequenceFloat32(v) => v.truncate(bound),
        DataStorage::SequenceFloat64(v) => v.truncate(bound),
        DataStorage::SequenceFloat128(v) => v.truncate(bound),
        DataStorage::SequenceChar8(v) => v.truncate(bound),
        DataStorage::SequenceBoolean(v) => v.truncate(bound),
        DataStorage::SequenceString(v) => v.truncate(bound),
        DataStorage::SequenceComplexValue(v) => v.truncate(bound),
        _ => {}
    }
}

fn default_storage_for_type(t: DynamicType<'static>) -> DataStorage {
    match t.get_kind() {
        TypeKind::BOOLEAN => DataStorage::Boolean(false),
        TypeKind::BYTE => DataStorage::UInt8(0),
        TypeKind::INT8 => DataStorage::Int8(0),
        TypeKind::UINT8 => DataStorage::UInt8(0),
        TypeKind::INT16 => DataStorage::Int16(0),
        TypeKind::UINT16 => DataStorage::UInt16(0),
        TypeKind::INT32 => DataStorage::Int32(0),
        TypeKind::UINT32 => DataStorage::UInt32(0),
        TypeKind::INT64 => DataStorage::Int64(0),
        TypeKind::UINT64 => DataStorage::UInt64(0),
        TypeKind::FLOAT32 => DataStorage::Float32(0.0),
        TypeKind::FLOAT64 => DataStorage::Float64(0.0),
        TypeKind::FLOAT128 => DataStorage::Float128(0),
        TypeKind::CHAR8 => DataStorage::Char8('\0'),
        TypeKind::CHAR16 => DataStorage::Char8('\0'),
        TypeKind::STRING8 => DataStorage::String(String::new()),
        TypeKind::STRING16 => DataStorage::String(String::new()),
        TypeKind::ENUM => {
            let default_val = (0..t.get_member_count())
                .filter_map(|i| t.get_member_by_index(i).ok())
                .find(|m| m.descriptor.is_default_label)
                .or_else(|| t.get_member_by_index(0).ok())
                .and_then(|m| m.descriptor.label.first().copied())
                .unwrap_or(0);
            let mut inner_data = DynamicDataFactory::create_data(t);
            inner_data.set_int32_value(0, default_val).ok();
            DataStorage::ComplexValue(inner_data)
        }
        TypeKind::BITMASK => DataStorage::UInt32(0),
        TypeKind::STRUCTURE | TypeKind::UNION | TypeKind::ANNOTATION => {
            DataStorage::ComplexValue(DynamicDataFactory::create_data(t))
        }
        TypeKind::SEQUENCE => {
            if let Some(elem_t) = t.descriptor.element_type {
                match elem_t.get_kind() {
                    TypeKind::BOOLEAN => DataStorage::SequenceBoolean(Vec::new()),
                    TypeKind::BYTE | TypeKind::UINT8 => DataStorage::SequenceUInt8(Vec::new()),
                    TypeKind::INT8 => DataStorage::SequenceInt8(Vec::new()),
                    TypeKind::INT16 => DataStorage::SequenceInt16(Vec::new()),
                    TypeKind::UINT16 => DataStorage::SequenceUInt16(Vec::new()),
                    TypeKind::INT32 => DataStorage::SequenceInt32(Vec::new()),
                    TypeKind::UINT32 => DataStorage::SequenceUInt32(Vec::new()),
                    TypeKind::INT64 => DataStorage::SequenceInt64(Vec::new()),
                    TypeKind::UINT64 => DataStorage::SequenceUInt64(Vec::new()),
                    TypeKind::FLOAT32 => DataStorage::SequenceFloat32(Vec::new()),
                    TypeKind::FLOAT64 => DataStorage::SequenceFloat64(Vec::new()),
                    TypeKind::FLOAT128 => DataStorage::SequenceFloat128(Vec::new()),
                    TypeKind::CHAR8 => DataStorage::SequenceChar8(Vec::new()),
                    TypeKind::CHAR16 => DataStorage::SequenceChar8(Vec::new()),
                    TypeKind::STRING8 | TypeKind::STRING16 => {
                        DataStorage::SequenceString(Vec::new())
                    }
                    _ => DataStorage::SequenceComplexValue(Vec::new()),
                }
            } else {
                DataStorage::SequenceComplexValue(Vec::new())
            }
        }
        TypeKind::ARRAY => DataStorage::SequenceComplexValue(Vec::new()),
        _ => DataStorage::Boolean(false),
    }
}

fn get_discriminator_value_as_i32(data: &DynamicData) -> Option<i32> {
    match data.get_storage(0)? {
        DataStorage::UInt8(x) => Some(*x as i32),
        DataStorage::Int8(x) => Some(*x as i32),
        DataStorage::UInt16(x) => Some(*x as i32),
        DataStorage::Int16(x) => Some(*x as i32),
        DataStorage::Int32(x) => Some(*x),
        DataStorage::UInt32(x) => Some(*x as i32),
        DataStorage::ComplexValue(inner) => get_discriminator_value_as_i32(inner),
        _ => None,
    }
}

fn get_selected_union_member<'a>(data: &DynamicData<'a>) -> Option<&'a DynamicTypeMember> {
    let disc_id = get_discriminator_value_as_i32(data)?;
    let mut default_member = None;
    for member in data.r#type.member_list {
        if member.descriptor.label.contains(&disc_id) {
            return Some(member);
        }
        if member.descriptor.is_default_label {
            default_member = Some(member);
        }
    }
    default_member
}

fn validate_member_value(value: &mut DataStorage, member_descriptor: &MemberDescriptor) -> bool {
    match member_descriptor.r#type.get_kind() {
        TypeKind::ENUM | TypeKind::STRUCTURE | TypeKind::UNION | TypeKind::ANNOTATION => {
            if let DataStorage::ComplexValue(inner) = value {
                inner.validate_dynamic_data()
            } else {
                false
            }
        }
        TypeKind::SEQUENCE => {
            if let DataStorage::SequenceComplexValue(vec) = value {
                let element_type = member_descriptor
                    .r#type
                    .descriptor
                    .element_type
                    .expect("sequence must have element type");
                for element in vec.iter_mut() {
                    if !element.validate_dynamic_data() {
                        match member_descriptor.try_construct_kind {
                            TryConstructKind::Discard => return false,
                            TryConstructKind::UseDefault => {
                                *element = DynamicDataFactory::create_data(element_type);
                            }
                            TryConstructKind::Trim => {
                                return false;
                            }
                        }
                    }
                }
            }

            let bound = member_descriptor
                .r#type
                .descriptor
                .bound
                .first()
                .copied()
                .unwrap_or(0) as usize;
            if bound > 0 && bound != u32::MAX as usize {
                if let Some(len) = get_sequence_len(value) {
                    if len > bound {
                        match member_descriptor.try_construct_kind {
                            TryConstructKind::Discard => return false,
                            TryConstructKind::UseDefault => {
                                reset_sequence_to_empty(value);
                            }
                            TryConstructKind::Trim => {
                                truncate_sequence(value, bound);
                            }
                        }
                    }
                }
            }
            true
        }
        TypeKind::ARRAY => {
            if let DataStorage::SequenceComplexValue(vec) = value {
                let element_type = member_descriptor
                    .r#type
                    .descriptor
                    .element_type
                    .expect("array must have element type");
                for elem in vec.iter_mut() {
                    if !elem.validate_dynamic_data() {
                        match member_descriptor.try_construct_kind {
                            TryConstructKind::Discard => return false,
                            TryConstructKind::UseDefault => {
                                *elem = DynamicDataFactory::create_data(element_type);
                            }
                            TryConstructKind::Trim => {
                                return false;
                            }
                        }
                    }
                }
            }
            true
        }
        TypeKind::STRING8 | TypeKind::STRING16 => {
            let bound = member_descriptor
                .r#type
                .descriptor
                .bound
                .first()
                .copied()
                .unwrap_or(0) as usize;
            if bound > 0 && bound != u32::MAX as usize {
                if let DataStorage::String(s) = value {
                    let len = s.chars().count();
                    if len > bound {
                        match member_descriptor.try_construct_kind {
                            TryConstructKind::Discard => return false,
                            TryConstructKind::UseDefault => {
                                s.clear();
                            }
                            TryConstructKind::Trim => {
                                let trim_idx =
                                    s.char_indices().nth(bound).map_or(s.len(), |(idx, _)| idx);
                                s.truncate(trim_idx);
                            }
                        }
                    }
                }
            }
            true
        }
        _ => true,
    }
}

impl<'a> DynamicData<'a> {
    pub(crate) fn validate_dynamic_data(&mut self) -> bool {
        let kind = self.r#type.descriptor.kind;
        let extensibility = self.r#type.descriptor.extensibility_kind;

        if kind == TypeKind::ENUM {
            if let Some(value) = self.get_storage(0) {
                let val = match value {
                    DataStorage::Int8(x) => *x as i32,
                    DataStorage::Int16(x) => *x as i32,
                    DataStorage::Int32(x) => *x,
                    _ => return false,
                };
                let enum_type = self.r#type;
                if enum_type.get_member_count() == 0 {
                    return true;
                }
                let is_valid = (0..enum_type.get_member_count()).any(|i| {
                    if let Ok(m) = enum_type.get_member_by_index(i) {
                        if let Some(&label_val) = m.descriptor.label.first() {
                            label_val == val
                        } else {
                            m.descriptor.index == val as u32 || m.descriptor.id == val as u32
                        }
                    } else {
                        false
                    }
                });
                if !is_valid {
                    return false;
                }
            }
        } else if kind == TypeKind::UNION {
            let r#type = self.r#type;
            if let Ok(disc_member) = r#type.get_member(0) {
                if let Some(disc_val) = self.get_storage_mut(0) {
                    if !validate_member_value(disc_val, &disc_member.descriptor) {
                        match disc_member.descriptor.try_construct_kind {
                            TryConstructKind::Discard => return false,
                            TryConstructKind::UseDefault => {
                                *disc_val = default_storage_for_type(disc_member.descriptor.r#type);
                            }
                            TryConstructKind::Trim => return false,
                        }
                    }
                }
            }
            let Some(selected_member) = get_selected_union_member(self) else {
                return false;
            };
            if selected_member.descriptor.r#type.get_kind() != TypeKind::NONE {
                let Some(value) = self.get_storage_mut(selected_member.get_id()) else {
                    return false;
                };
                if !validate_member_value(value, &selected_member.descriptor) {
                    match selected_member.descriptor.try_construct_kind {
                        TryConstructKind::Discard => return false,
                        TryConstructKind::UseDefault => {
                            *value = default_storage_for_type(selected_member.descriptor.r#type);
                        }
                        TryConstructKind::Trim => return false,
                    }
                }
            }
        } else if kind == TypeKind::STRUCTURE {
            for member in self.r#type.member_list {
                let member_id = member.get_id();
                let try_construct_kind = member.descriptor.try_construct_kind;
                let is_required =
                    !member.descriptor.is_optional && extensibility == ExtensibilityKind::Final;

                if let Some(m) = self.abstract_data.iter_mut().find(|m| m.id == member_id) {
                    if !validate_member_value(&mut m.value, &member.descriptor) {
                        match try_construct_kind {
                            TryConstructKind::Discard => return false,
                            TryConstructKind::UseDefault => {
                                m.value = default_storage_for_type(member.descriptor.r#type);
                            }
                            TryConstructKind::Trim => return false,
                        }
                    }
                } else if is_required {
                    match try_construct_kind {
                        TryConstructKind::Discard => return false,
                        TryConstructKind::UseDefault => {
                            let default_val = default_storage_for_type(member.descriptor.r#type);
                            self.abstract_data.push(MemberDataStorage {
                                id: member_id,
                                value: default_val,
                            });
                        }
                        TryConstructKind::Trim => return false,
                    }
                }
            }
        }

        true
    }
}

#[cfg(feature = "xtypes-xml")]
fn parse_f128_str(s: &str) -> Result<i128, ()> {
    let s = s.trim();
    if s.is_empty() {
        return Err(());
    }

    let (is_negative, s) = if let Some(stripped) = s.strip_prefix('-') {
        (true, stripped)
    } else if let Some(stripped) = s.strip_prefix('+') {
        (false, stripped)
    } else {
        (false, s)
    };

    let sign = if is_negative { 1_u128 } else { 0_u128 };

    if let Some(hex) = s.strip_prefix("0x").or_else(|| s.strip_prefix("0X")) {
        let val = u128::from_str_radix(hex, 16).map_err(|_| ())?;
        return Ok(if is_negative {
            ((1_u128 << 127) | val) as i128
        } else {
            val as i128
        });
    }

    if s.eq_ignore_ascii_case("inf") || s.eq_ignore_ascii_case("infinity") {
        return Ok(((sign << 127) | (0x7FFF_u128 << 112)) as i128);
    }
    if s.eq_ignore_ascii_case("nan") {
        return Ok(((sign << 127) | (0x7FFF_u128 << 112) | (1_u128 << 111)) as i128);
    }

    let (int_s, frac_s_raw) = s.split_once('.').unwrap_or((s, ""));
    let frac_s = frac_s_raw.trim_end_matches('0');

    let int_val: u128 = if int_s.is_empty() {
        0
    } else {
        int_s.parse().map_err(|_| ())?
    };
    let frac_val: u128 = if frac_s.is_empty() {
        0
    } else {
        frac_s.parse().map_err(|_| ())?
    };

    if int_val == 0 && frac_val == 0 {
        return Ok((sign << 127) as i128);
    }

    let denom: u128 = if frac_s.is_empty() {
        1
    } else {
        10_u128.checked_pow(frac_s.len() as u32).ok_or(())?
    };

    let div_shift = |val: u128, shift: u32, denom: u128| -> (u128, u128) {
        let s1 = shift.min(60);
        let s2 = (shift - s1).min(60);
        let s3 = shift - s1 - s2;
        let q1 = (val << s1) / denom;
        let r1 = (val << s1) % denom;
        let q2 = (r1 << s2) / denom;
        let r2 = (r1 << s2) % denom;
        let q3 = (r2 << s3) / denom;
        let r3 = (r2 << s3) % denom;
        let q = (((q1 << s2) + q2) << s3) + q3;
        (q, r3)
    };

    let (mut k, mut mantissa, rem) = if int_val > 0 {
        let k = 127 - int_val.leading_zeros() as i32;
        let shift = (112 - k) as u32;
        let int_mantissa = int_val << shift;
        let (frac_mantissa, rem) = if frac_val > 0 {
            div_shift(frac_val, shift, denom)
        } else {
            (0, 0)
        };
        (k, int_mantissa + frac_mantissa, rem)
    } else {
        let q1 = (frac_val << 60) / denom;
        let lz = if q1 > 0 {
            q1.leading_zeros() as i32 - 68
        } else {
            let r1 = (frac_val << 60) % denom;
            let q2 = (r1 << 60) / denom;
            60 + (q2.leading_zeros() as i32 - 68)
        };
        let k = -(lz + 1);
        let shift = (112 - k) as u32;
        let (mantissa, rem) = div_shift(frac_val, shift, denom);
        (k, mantissa, rem)
    };

    if rem * 2 > denom || (rem * 2 == denom && (mantissa & 1 == 1)) {
        mantissa += 1;
    }
    if mantissa == (1_u128 << 113) {
        mantissa = 1_u128 << 112;
        k += 1;
    }

    let biased_exp = (k + 16383) as u128;
    let frac = mantissa & ((1_u128 << 112) - 1);
    let val = (sign << 127) | (biased_exp << 112) | frac;
    Ok(val as i128)
}

#[cfg(feature = "xtypes-xml")]
impl<'a> DynamicData<'a> {
    /// Deserializes dynamic data from XML.
    pub fn from_xml(&mut self, xml: &str) -> XTypesResult<()> {
        let doc = roxmltree::Document::parse(xml).map_err(|_| XTypesError::InvalidData)?;
        let root = doc.root_element();
        self.populate_from_xml_node(root)
    }

    fn set_discrimant(&mut self, node: roxmltree::Node) -> XTypesResult<()> {
        fn parse_i32(s: &str) -> XTypesResult<i32> {
            let s = s.trim();
            if let Some(hex) = s.strip_prefix("0x") {
                i32::from_str_radix(hex, 16).map_err(|_| XTypesError::InvalidData)
            } else {
                s.parse::<i32>().map_err(|_| XTypesError::InvalidData)
            }
        }

        let tag_name = node.tag_name().name();
        let discriminator_label = if tag_name == "discriminator" {
            let disc_text = node.text().ok_or(XTypesError::InvalidData)?.trim();
            if let Ok(val) = parse_i32(disc_text) {
                val
            } else {
                let disc_member = self.r#type.get_member(0)?;
                let disc_type = disc_member.descriptor.r#type;
                if let Ok(enum_member) = disc_type.get_member_by_name(disc_text) {
                    enum_member
                        .descriptor
                        .label
                        .first()
                        .copied()
                        .unwrap_or(enum_member.get_id() as i32)
                } else {
                    return Err(XTypesError::InvalidData);
                }
            }
        } else {
            let variant_member = self.r#type.get_member_by_name(tag_name)?;
            if let Some(&label) = variant_member.descriptor.label.first() {
                label
            } else if variant_member.descriptor.is_default_label {
                return Ok(());
            } else {
                return Err(XTypesError::InvalidType);
            }
        };

        match self.r#type.get_member(0)?.descriptor.r#type.get_kind() {
            TypeKind::BOOLEAN => todo!(),
            TypeKind::BYTE => todo!(),
            TypeKind::INT16 => self.set_int16_value(0, discriminator_label as i16),
            TypeKind::INT32 => self.set_int32_value(0, discriminator_label as i32),
            TypeKind::INT64 => self.set_int64_value(0, discriminator_label as i64),
            TypeKind::UINT16 => self.set_uint16_value(0, discriminator_label as u16),
            TypeKind::UINT32 => self.set_uint32_value(0, discriminator_label as u32),
            TypeKind::UINT64 => self.set_uint64_value(0, discriminator_label as u64),
            TypeKind::INT8 => self.set_int8_value(0, discriminator_label as i8),
            TypeKind::UINT8 => self.set_uint8_value(0, discriminator_label as u8),
            TypeKind::CHAR8 => todo!(),
            TypeKind::CHAR16 => todo!(),
            TypeKind::ALIAS => todo!(),
            TypeKind::ENUM => {
                let disc_member = self.r#type.get_member(0)?;
                let mut inner_data = DynamicDataFactory::create_data(disc_member.descriptor.r#type);
                inner_data.set_int32_value(0, discriminator_label as i32)?;
                self.set_complex_value(0, inner_data)?;
                Ok(())
            }
            TypeKind::BITMASK => {
                let bound = self
                    .r#type
                    .get_member(0)?
                    .descriptor
                    .r#type
                    .get_descriptor()
                    .bound
                    .first()
                    .copied()
                    .unwrap_or(32);
                match bound {
                    1..=8 => self.set_uint8_value(0, discriminator_label as u8),
                    9..=16 => self.set_uint16_value(0, discriminator_label as u16),
                    17..=32 => self.set_uint32_value(0, discriminator_label as u32),
                    _ => self.set_uint64_value(0, discriminator_label as u64),
                }
            }
            _ => Err(XTypesError::InvalidType),
        }
    }

    fn populate_from_xml_node(&mut self, node: roxmltree::Node) -> XTypesResult<()> {
        for child in node.children().filter(|c| c.is_element()) {
            let tag_name = child.tag_name().name();

            if self.r#type.get_kind() == TypeKind::UNION {
                self.set_discrimant(child)?;
            }

            if let Ok(member) = self.r#type.get_member_by_name(tag_name) {
                let member_id = member.get_id();
                let member_descriptor = member.get_descriptor()?;
                let member_type = member_descriptor.r#type;

                let data = Self::parse_xml_node_to_data(child, member_type)?;
                self.set_value(member_id, data);
            }
        }
        Ok(())
    }

    fn parse_xml_node_to_data(
        node: roxmltree::Node,
        r#type: DynamicType<'static>,
    ) -> XTypesResult<DataStorage> {
        let kind = r#type.get_kind();
        let text = node.text().unwrap_or("").trim();

        let parse_int = |s: &str| -> Result<i64, ()> {
            if let Some(hex) = s.strip_prefix("0x") {
                i64::from_str_radix(hex, 16).map_err(|_| ())
            } else {
                s.parse::<i64>().map_err(|_| ())
            }
        };

        let parse_uint = |s: &str| -> Result<u64, ()> {
            if let Some(hex) = s.strip_prefix("0x") {
                u64::from_str_radix(hex, 16).map_err(|_| ())
            } else {
                s.parse::<u64>().map_err(|_| ())
            }
        };

        let parse_float128 = parse_f128_str;

        match kind {
            TypeKind::BOOLEAN => {
                let val = text == "true" || text == "1";
                Ok(DataStorage::Boolean(val))
            }
            TypeKind::BYTE | TypeKind::UINT8 => {
                let val = parse_uint(text).map_err(|_| XTypesError::InvalidData)? as u8;
                Ok(DataStorage::UInt8(val))
            }
            TypeKind::INT8 => {
                let val = parse_int(text).map_err(|_| XTypesError::InvalidData)? as i8;
                Ok(DataStorage::Int8(val))
            }
            TypeKind::UINT16 => {
                let val = parse_uint(text).map_err(|_| XTypesError::InvalidData)? as u16;
                Ok(DataStorage::UInt16(val))
            }
            TypeKind::INT16 => {
                let val = parse_int(text).map_err(|_| XTypesError::InvalidData)? as i16;
                Ok(DataStorage::Int16(val))
            }
            TypeKind::UINT32 => {
                let val = parse_uint(text).map_err(|_| XTypesError::InvalidData)? as u32;
                Ok(DataStorage::UInt32(val))
            }
            TypeKind::INT32 => {
                let val = parse_int(text).map_err(|_| XTypesError::InvalidData)? as i32;
                Ok(DataStorage::Int32(val))
            }
            TypeKind::UINT64 => {
                let val = parse_uint(text).map_err(|_| XTypesError::InvalidData)?;
                Ok(DataStorage::UInt64(val))
            }
            TypeKind::INT64 => {
                let val = parse_int(text).map_err(|_| XTypesError::InvalidData)?;
                Ok(DataStorage::Int64(val))
            }
            TypeKind::FLOAT32 => {
                let val = text.parse::<f32>().map_err(|_| XTypesError::InvalidData)?;
                Ok(DataStorage::Float32(val))
            }
            TypeKind::FLOAT64 => {
                let val = text.parse::<f64>().map_err(|_| XTypesError::InvalidData)?;
                Ok(DataStorage::Float64(val))
            }
            TypeKind::FLOAT128 => {
                let val = parse_float128(text).map_err(|_| XTypesError::InvalidData)?;
                Ok(DataStorage::Float128(val))
            }
            TypeKind::CHAR8 => {
                let val = parse_uint(text)
                    .ok()
                    .map(|v| v as u8 as char)
                    .unwrap_or_else(|| text.chars().next().unwrap_or('\0'));
                Ok(DataStorage::Char8(val))
            }
            TypeKind::STRING8 | TypeKind::STRING16 => {
                let val = node.text().unwrap_or("");
                Ok(DataStorage::String(String::from(val)))
            }
            TypeKind::BITMASK => {
                let bound = r#type.get_descriptor().bound.first().copied().unwrap_or(32);
                let val = parse_uint(text).map_err(|_| XTypesError::InvalidData)?;
                match bound {
                    1..=8 => Ok(DataStorage::UInt8(val as u8)),
                    9..=16 => Ok(DataStorage::UInt16(val as u16)),
                    17..=32 => Ok(DataStorage::UInt32(val as u32)),
                    _ => Ok(DataStorage::UInt64(val)),
                }
            }
            TypeKind::ENUM => {
                let label = if let Ok(val) = parse_int(text) {
                    val as i32
                } else {
                    let enumerator = r#type.get_member_by_name(text)?;
                    *enumerator
                        .descriptor
                        .label
                        .first()
                        .ok_or(XTypesError::InvalidData)?
                };
                let mut inner_data = DynamicDataFactory::create_data(r#type);
                inner_data.set_int32_value(0, label)?;
                Ok(DataStorage::ComplexValue(inner_data))
            }
            TypeKind::STRUCTURE | TypeKind::UNION => {
                let mut inner_data = DynamicDataFactory::create_data(r#type);
                inner_data.populate_from_xml_node(node)?;
                Ok(DataStorage::ComplexValue(inner_data))
            }
            TypeKind::SEQUENCE | TypeKind::ARRAY => {
                let element_type = r#type
                    .get_descriptor()
                    .element_type
                    .ok_or(XTypesError::InvalidData)?;
                let element_kind = element_type.get_kind();

                macro_rules! parse_seq {
                    ($parse_fn:ident, $storage_variant:ident, $cast_type:ty) => {{
                        let mut vec = Vec::new();
                        for item in node.children().filter(|c| c.is_element()) {
                            let item_text = item.text().unwrap_or("").trim();
                            let val = $parse_fn(item_text).map_err(|_| XTypesError::InvalidData)?
                                as $cast_type;
                            vec.push(val);
                        }
                        Ok(DataStorage::$storage_variant(vec))
                    }};
                }

                match element_kind {
                    TypeKind::INT32 => parse_seq!(parse_int, SequenceInt32, i32),
                    TypeKind::UINT32 => parse_seq!(parse_uint, SequenceUInt32, u32),
                    TypeKind::INT8 => parse_seq!(parse_int, SequenceInt8, i8),
                    TypeKind::UINT8 | TypeKind::BYTE => {
                        parse_seq!(parse_uint, SequenceUInt8, u8)
                    }
                    TypeKind::INT16 => parse_seq!(parse_int, SequenceInt16, i16),
                    TypeKind::UINT16 => parse_seq!(parse_uint, SequenceUInt16, u16),
                    TypeKind::INT64 => parse_seq!(parse_int, SequenceInt64, i64),
                    TypeKind::UINT64 => parse_seq!(parse_uint, SequenceUInt64, u64),
                    TypeKind::FLOAT32 => {
                        let mut vec = Vec::new();
                        for item in node.children().filter(|c| c.is_element()) {
                            let item_text = item.text().unwrap_or("").trim();
                            vec.push(
                                item_text
                                    .parse::<f32>()
                                    .map_err(|_| XTypesError::InvalidData)?,
                            );
                        }
                        Ok(DataStorage::SequenceFloat32(vec))
                    }
                    TypeKind::FLOAT64 => {
                        let mut vec = Vec::new();
                        for item in node.children().filter(|c| c.is_element()) {
                            let item_text = item.text().unwrap_or("").trim();
                            vec.push(
                                item_text
                                    .parse::<f64>()
                                    .map_err(|_| XTypesError::InvalidData)?,
                            );
                        }
                        Ok(DataStorage::SequenceFloat64(vec))
                    }
                    TypeKind::FLOAT128 => {
                        let mut vec = Vec::new();
                        for item in node.children().filter(|c| c.is_element()) {
                            let item_text = item.text().unwrap_or("").trim();
                            let val =
                                parse_float128(item_text).map_err(|_| XTypesError::InvalidData)?;
                            vec.push(val);
                        }
                        Ok(DataStorage::SequenceFloat128(vec))
                    }
                    TypeKind::BOOLEAN => {
                        let mut vec = Vec::new();
                        for item in node.children().filter(|c| c.is_element()) {
                            let item_text = item.text().unwrap_or("").trim();
                            vec.push(item_text == "true" || item_text == "1");
                        }
                        Ok(DataStorage::SequenceBoolean(vec))
                    }
                    TypeKind::CHAR8 => {
                        let mut vec = Vec::new();
                        for item in node.children().filter(|c| c.is_element()) {
                            let item_text = item.text().unwrap_or("");
                            let val = parse_uint(item_text)
                                .ok()
                                .map(|v| v as u8 as char)
                                .unwrap_or_else(|| item_text.chars().next().unwrap_or('\0'));
                            vec.push(val);
                        }
                        Ok(DataStorage::SequenceChar8(vec))
                    }
                    TypeKind::STRING8 => {
                        let mut vec = Vec::new();
                        for item in node.children().filter(|c| c.is_element()) {
                            let item_text = item.text().unwrap_or("");
                            vec.push(String::from(item_text));
                        }
                        Ok(DataStorage::SequenceString(vec))
                    }
                    TypeKind::ENUM => {
                        let mut vec = Vec::new();
                        for item in node.children().filter(|c| c.is_element()) {
                            let item_text = item.text().unwrap_or("").trim();
                            let mut inner_data = DynamicDataFactory::create_data(element_type);
                            let enumerator = element_type.get_member_by_name(item_text)?;
                            let label = enumerator
                                .descriptor
                                .label
                                .first()
                                .ok_or(XTypesError::InvalidData)?;
                            inner_data.set_int32_value(0, *label)?;
                            vec.push(inner_data);
                        }
                        Ok(DataStorage::SequenceComplexValue(vec))
                    }
                    TypeKind::STRUCTURE | TypeKind::UNION => {
                        let mut vec = Vec::new();
                        for item in node.children().filter(|c| c.is_element()) {
                            let mut inner_data = DynamicDataFactory::create_data(element_type);
                            inner_data.populate_from_xml_node(item)?;
                            vec.push(inner_data);
                        }
                        Ok(DataStorage::SequenceComplexValue(vec))
                    }
                    _ => Err(XTypesError::InvalidData),
                }
            }
            _ => Err(XTypesError::InvalidData),
        }
    }
}
