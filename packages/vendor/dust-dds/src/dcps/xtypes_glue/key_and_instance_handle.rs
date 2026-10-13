use crate::{
    infrastructure::instance::InstanceHandle,
    transport::types::TopicKind,
    xtypes::{
        dynamic_type::{
            DynamicData, DynamicDataFactory, DynamicType, DynamicTypeMember, TypeDescriptor,
            TypeKind,
        },
        error::{XTypesError, XTypesResult},
        serializer::serialize_final_without_header,
    },
};
use alloc::{boxed::Box, vec::Vec};

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct KeyHolderType {
    descriptor: Option<TypeDescriptor>,
    key_members: Box<[DynamicTypeMember]>,
}

impl Default for KeyHolderType {
    fn default() -> Self {
        Self {
            descriptor: None,
            key_members: Box::new([]),
        }
    }
}

impl KeyHolderType {
    pub fn new(value: &DynamicType<'_>) -> Self {
        let mut member_list = Vec::new();
        fn fill_struct_key_holder_type(
            value: &DynamicType<'_>,
            member_list: &mut Vec<DynamicTypeMember>,
        ) {
            if value.get_kind() == TypeKind::STRUCTURE {
                for member in value.member_list {
                    if member.descriptor.is_key {
                        member_list.push(member.clone());
                    } else if member.descriptor.r#type.descriptor.kind == TypeKind::STRUCTURE
                        && !member.descriptor.is_optional
                    {
                        fill_struct_key_holder_type(&member.descriptor.r#type, member_list);
                    }
                }
            }
        }

        fill_struct_key_holder_type(value, &mut member_list);
        Self {
            descriptor: Some(value.descriptor.clone()),
            key_members: member_list.into_boxed_slice(),
        }
    }

    pub fn as_dynamic_type(&self) -> Option<DynamicType<'_>> {
        self.descriptor.as_ref().map(|descriptor| DynamicType {
            descriptor,
            member_list: &self.key_members,
        })
    }

    pub fn is_empty(&self) -> bool {
        self.key_members.is_empty()
    }
}

impl From<&DynamicType<'_>> for TopicKind {
    fn from(value: &DynamicType<'_>) -> Self {
        if value.get_kind() == TypeKind::STRUCTURE {
            for member in value.member_list {
                if member.descriptor.is_key {
                    return TopicKind::WithKey;
                } else if member.descriptor.r#type.descriptor.kind == TypeKind::STRUCTURE
                    && !member.descriptor.is_optional
                {
                    let member_topic_kind = TopicKind::from(&member.descriptor.r#type);
                    if member_topic_kind == TopicKind::WithKey {
                        return TopicKind::WithKey;
                    }
                }
            }
        }
        TopicKind::NoKey
    }
}

pub struct KeyHolderData<'a>(DynamicData<'a>);

impl<'a> KeyHolderData<'a> {
    pub fn from_dynamic_data(
        value: &DynamicData<'a>,
        key_holder_type: &'a KeyHolderType,
    ) -> XTypesResult<KeyHolderData<'a>> {
        fn fill_struct_key_holder_data<'a>(
            value: &DynamicData<'a>,
            key_holder_data: &mut DynamicData,
        ) -> XTypesResult<()> {
            let dynamic_type = value.r#type();
            if dynamic_type.get_kind() == TypeKind::STRUCTURE {
                for member_index in 0..dynamic_type.get_member_count() {
                    let dynamic_type_member = dynamic_type.get_member_by_index(member_index)?;
                    let key_member_id = dynamic_type_member.get_id();
                    if dynamic_type_member.descriptor.is_key {
                        key_holder_data
                            .set_value(key_member_id, value.get_value(key_member_id)?.clone());
                    } else if dynamic_type_member.descriptor.r#type.get_kind()
                        == TypeKind::STRUCTURE
                        && !dynamic_type_member.descriptor.is_optional
                    {
                        fill_struct_key_holder_data(
                            value.get_complex_value(key_member_id)?,
                            key_holder_data,
                        )?;
                    }
                }
            }
            Ok(())
        }
        let dynamic_type = key_holder_type
            .as_dynamic_type()
            .ok_or(XTypesError::InvalidType)?;
        let mut key_holder_data = DynamicDataFactory::create_data(dynamic_type);
        fill_struct_key_holder_data(value, &mut key_holder_data)?;
        Ok(Self(key_holder_data))
    }

    pub fn as_dynamic_data(&self) -> &DynamicData<'a> {
        &self.0
    }
}

pub fn get_instance_handle_from_key_holder_data<'a>(
    key_holder_data: &KeyHolderData<'a>,
) -> Result<InstanceHandle, XTypesError> {
    let data = serialize_final_without_header(Vec::with_capacity(16), &key_holder_data.0)?;
    let dynamic_type = key_holder_data.0.r#type();
    let max_size = dynamic_type.max_size_serialized_cdr_be(0);
    let key = match max_size {
        Some(size) if size <= 16 => {
            let mut key = [0; 16];
            let len = data.len().min(16);
            key[0..len].copy_from_slice(&data[0..len]);
            key
        }
        _ => md5::compute(data).into(),
    };

    Ok(InstanceHandle::new(key))
}

pub fn get_instance_handle_from_dynamic_data_and_key_holder<'a>(
    value: &DynamicData<'a>,
    key_holder_type: &KeyHolderType,
) -> Result<InstanceHandle, XTypesError> {
    if key_holder_type.is_empty() {
        return Ok(InstanceHandle::new([0; 16]));
    }
    let key_holder_data = KeyHolderData::from_dynamic_data(value, key_holder_type)?;
    get_instance_handle_from_key_holder_data(&key_holder_data)
}

pub fn get_instance_handle_from_dynamic_data<'a>(
    value: &DynamicData<'a>,
) -> Result<InstanceHandle, XTypesError> {
    let key_holder_type = KeyHolderType::new(&value.r#type());
    get_instance_handle_from_dynamic_data_and_key_holder(value, &key_holder_type)
}

impl<'a> DynamicType<'a> {
    /// Computes the maximum serialized size in bytes using CDR Big-Endian encoding.
    /// Returns `None` if the type has unbounded size (such as unbounded strings or sequences).
    fn max_size_serialized_cdr_be(&self, current_offset: usize) -> Option<usize> {
        fn align_to(offset: usize, alignment: usize) -> usize {
            if alignment <= 1 {
                offset
            } else {
                (offset + alignment - 1) & !(alignment - 1)
            }
        }

        match self.get_kind() {
            TypeKind::NONE => Some(current_offset),
            TypeKind::BOOLEAN
            | TypeKind::BYTE
            | TypeKind::INT8
            | TypeKind::UINT8
            | TypeKind::CHAR8 => Some(current_offset.checked_add(1)?),
            TypeKind::INT16 | TypeKind::UINT16 | TypeKind::CHAR16 => {
                let aligned = align_to(current_offset, 2);
                Some(aligned.checked_add(2)?)
            }
            TypeKind::INT32 | TypeKind::UINT32 | TypeKind::FLOAT32 | TypeKind::ENUM => {
                let aligned = align_to(current_offset, 4);
                Some(aligned.checked_add(4)?)
            }
            TypeKind::INT64 | TypeKind::UINT64 | TypeKind::FLOAT64 => {
                let aligned = align_to(current_offset, 8);
                Some(aligned.checked_add(8)?)
            }
            TypeKind::FLOAT128 => {
                let aligned = align_to(current_offset, 16);
                Some(aligned.checked_add(16)?)
            }
            TypeKind::STRING8 => {
                let bound = self.descriptor.bound.first().copied().unwrap_or(0);
                if bound == 0 || bound == u32::MAX {
                    None
                } else {
                    let aligned = align_to(current_offset, 4);
                    let size = (bound as usize).checked_add(5)?;
                    aligned.checked_add(size)
                }
            }
            TypeKind::STRING16 => {
                let bound = self.descriptor.bound.first().copied().unwrap_or(0);
                if bound == 0 || bound == u32::MAX {
                    None
                } else {
                    let aligned = align_to(current_offset, 4);
                    let chars_size = (bound as usize).checked_mul(2)?;
                    let size = chars_size.checked_add(6)?;
                    aligned.checked_add(size)
                }
            }
            TypeKind::ALIAS => {
                if let Some(base_type) = &self.descriptor.base_type {
                    base_type.max_size_serialized_cdr_be(current_offset)
                } else {
                    None
                }
            }
            TypeKind::BITMASK => {
                let bound = self.descriptor.bound.first().copied().unwrap_or(32);
                let (align, size) = match bound {
                    0..=8 => (1, 1),
                    9..=16 => (2, 2),
                    17..=32 => (4, 4),
                    _ => (8, 8),
                };
                let aligned = align_to(current_offset, align);
                aligned.checked_add(size)
            }
            TypeKind::STRUCTURE => {
                let mut offset = current_offset;
                for member in self.member_list {
                    offset = member
                        .descriptor
                        .r#type
                        .max_size_serialized_cdr_be(offset)?;
                }
                Some(offset)
            }
            TypeKind::UNION => {
                let disc_type = self.descriptor.discriminator_type.as_ref()?;
                let disc_offset = disc_type.max_size_serialized_cdr_be(current_offset)?;
                if self.member_list.is_empty() {
                    Some(disc_offset)
                } else {
                    let mut max_offset = disc_offset;
                    for member in self.member_list {
                        let member_offset = member
                            .descriptor
                            .r#type
                            .max_size_serialized_cdr_be(disc_offset)?;
                        if member_offset > max_offset {
                            max_offset = member_offset;
                        }
                    }
                    Some(max_offset)
                }
            }
            TypeKind::SEQUENCE => {
                let bound = self.descriptor.bound.first().copied().unwrap_or(0);
                if bound == 0 || bound == u32::MAX {
                    None
                } else {
                    let elem_type = self.descriptor.element_type.as_ref()?;
                    let mut offset = align_to(current_offset, 4).checked_add(4)?;
                    for _ in 0..bound {
                        offset = elem_type.max_size_serialized_cdr_be(offset)?;
                    }
                    Some(offset)
                }
            }
            TypeKind::ARRAY => {
                let elem_type = self.descriptor.element_type.as_ref()?;
                let mut total_count: usize = 1;
                for &dim in self.descriptor.bound {
                    total_count = total_count.checked_mul(dim as usize)?;
                }
                let mut offset = current_offset;
                for _ in 0..total_count {
                    offset = elem_type.max_size_serialized_cdr_be(offset)?;
                }
                Some(offset)
            }
            TypeKind::MAP | TypeKind::ANNOTATION | TypeKind::BITSET => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{infrastructure::instance::InstanceHandle, xtypes::type_support::TypeSupport};

    #[test]
    fn test_multiple_nested_keys() {
        #[derive(TypeSupport)]
        struct Inner {
            #[dust_dds(key)]
            id: u8,
        }

        #[derive(TypeSupport)]
        struct Outer {
            a: Inner,
            #[dust_dds(key)]
            b: u16,
        }

        #[derive(TypeSupport)]
        struct OuterOuter {
            a: Outer,
        }

        let outer_outer = OuterOuter {
            a: Outer {
                a: Inner { id: 1 },
                b: 3,
            },
        };

        let data = outer_outer.create_dynamic_sample();

        let instance_handle = get_instance_handle_from_dynamic_data(&data).unwrap();

        assert_eq!(
            instance_handle,
            InstanceHandle::new([1, 0, 0, 3, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0])
        )
    }

    #[test]
    fn test_full_struct_key() {
        #[derive(TypeSupport)]
        struct Inner {
            id: u8,
            b: u16,
        }

        #[derive(TypeSupport)]
        struct Outer {
            #[dust_dds(key)]
            a: Inner,
        }

        let outer = Outer {
            a: Inner { id: 1, b: 3 },
        };

        let data = outer.create_dynamic_sample();

        let instance_handle = get_instance_handle_from_dynamic_data(&data).unwrap();

        assert_eq!(
            instance_handle,
            InstanceHandle::new([1, 0, 0, 3, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0])
        )
    }

    #[test]
    fn test_string_key_md5_hash() {
        #[derive(TypeSupport)]
        struct ShapeTypeKey {
            #[dust_dds(key)]
            color: String,
        }

        let shape = ShapeTypeKey {
            color: "BLUE".to_string(),
        };

        let data = shape.create_dynamic_sample();
        let instance_handle = get_instance_handle_from_dynamic_data(&data).unwrap();

        // CDR_BE serialized bytes for string "BLUE": [0, 0, 0, 5, 'B', 'L', 'U', 'E', 0]
        let expected_bytes: [u8; 9] = [0, 0, 0, 5, b'B', b'L', b'U', b'E', 0];
        let expected_hash = md5::compute(expected_bytes);

        assert_eq!(instance_handle, InstanceHandle::new(expected_hash.0));
    }
}
