use super::dynamic_type::ExtensibilityKind;
use crate::xtypes::{
    data_storage::DataStorage,
    dynamic_type::{DynamicData, MemberDescriptor, TypeKind},
    error::{XTypesError, XTypesResult},
    type_object::TypeIdentifier,
    type_support::TypeSupport,
};
use alloc::{string::ToString, vec::Vec};

type RepresentationIdentifier = [u8; 2];
const CDR_BE: RepresentationIdentifier = [0x00, 0x00];
const CDR_LE: RepresentationIdentifier = [0x00, 0x01];
const CDR2_BE: RepresentationIdentifier = [0x00, 0x06];
const CDR2_LE: RepresentationIdentifier = [0x00, 0x07];
const D_CDR2_BE: RepresentationIdentifier = [0x00, 0x08];
const D_CDR2_LE: RepresentationIdentifier = [0x00, 0x09];
const PL_CDR_BE: RepresentationIdentifier = [0x00, 0x02];
const PL_CDR_LE: RepresentationIdentifier = [0x00, 0x03];
const PL_CDR2_BE: RepresentationIdentifier = [0x00, 0x0a];
const PL_CDR2_LE: RepresentationIdentifier = [0x00, 0x0b];
const REPRESENTATION_OPTIONS: [u8; 2] = [0x00, 0x00];

pub fn serialize_cdr1_be(dynamic_data: &DynamicData) -> XTypesResult<Vec<u8>> {
    serialize_cdr1(dynamic_data, BigEndian)
}

pub fn serialize_cdr1_le(dynamic_data: &DynamicData) -> XTypesResult<Vec<u8>> {
    serialize_cdr1(dynamic_data, LittleEndian)
}

pub fn serialize_cdr2_be(dynamic_data: &DynamicData) -> XTypesResult<Vec<u8>> {
    serialize_cdr2(dynamic_data, BigEndian)
}

pub fn serialize_cdr2_le(dynamic_data: &DynamicData) -> XTypesResult<Vec<u8>> {
    serialize_cdr2(dynamic_data, LittleEndian)
}

pub fn serialize_final_without_header(
    mut buffer: Vec<u8>,
    dynamic_data: &DynamicData,
) -> XTypesResult<Vec<u8>> {
    let mut s = XTypesSerializer {
        writer: CdrWriter::new(&mut buffer),
        _endianness: BigEndian,
        _encoding_version: EncodingVersion1,
    };
    s.serialize_fstruct_type(dynamic_data)?;
    Ok(buffer)
}

pub fn serialize_without_header_cdr1_le(
    mut buffer: Vec<u8>,
    dynamic_data: &DynamicData,
) -> XTypesResult<Vec<u8>> {
    let mut s = XTypesSerializer {
        writer: CdrWriter::new(&mut buffer),
        _endianness: LittleEndian,
        _encoding_version: EncodingVersion1,
    };
    s.serialize_t_as_nested(dynamic_data)?;
    Ok(buffer)
}

pub fn serialize_without_header_cdr2_le(
    mut buffer: Vec<u8>,
    dynamic_data: &DynamicData,
) -> XTypesResult<Vec<u8>> {
    let mut s = XTypesSerializer {
        writer: CdrWriter::new(&mut buffer),
        _endianness: LittleEndian,
        _encoding_version: EncodingVersion2,
    };
    s.serialize_t_as_nested(dynamic_data)?;
    Ok(buffer)
}

fn serialize_cdr1(
    dynamic_data: &DynamicData,
    endianness: impl EndiannessWrite,
) -> XTypesResult<Vec<u8>> {
    let mut buffer = Vec::with_capacity(64);
    XTypesSerializer {
        writer: CdrWriter::new(&mut buffer),
        _endianness: endianness,
        _encoding_version: EncodingVersion1,
    }
    .serialize_top_level_type(dynamic_data)?;
    pad_entire_serialization(&mut buffer);
    Ok(buffer)
}

fn serialize_cdr2(
    dynamic_data: &DynamicData,
    endianness: impl EndiannessWrite,
) -> XTypesResult<Vec<u8>> {
    let mut buffer = Vec::with_capacity(64);
    XTypesSerializer {
        writer: CdrWriter::new(&mut buffer),
        _endianness: endianness,
        _encoding_version: EncodingVersion2,
    }
    .serialize_top_level_type(dynamic_data)?;
    pad_entire_serialization(&mut buffer);
    Ok(buffer)
}

fn pad_entire_serialization(buffer: &mut Vec<u8>) {
    let padding = match buffer.len() % 4 {
        1 => &[0, 0, 0][..],
        2 => &[0, 0][..],
        3 => &[0][..],
        _ => &[][..],
    };
    buffer.extend_from_slice(padding);
    buffer[3] = padding.len() as u8;
}

fn serialize_primitive_slice<'a, E: EndiannessWrite, V: EncodingVersion>(
    serializer: &mut XTypesSerializer<'a, E, V>,
    v: &[impl AsBytes + Ossize],
) {
    for vi in v {
        serializer.serialize_primitive_type(vi);
    }
}

// Serialization of types defined in the XTypes standard.
// The definitions follow the order and nomenclature of Table 40 – Symbols and notation used in the serialization virtual machine
// defined in the DDS-XTypes, version 1.3 - 7.4.3.5.1 Notation used for the match criteria
struct XTypesSerializer<'a, E, V> {
    writer: CdrWriter<'a>,
    _endianness: E,
    _encoding_version: V,
}
impl<'a, E: EndiannessWrite, V: EncodingVersion> XTypesSerializer<'a, E, V> {
    /// Rule: PUSH( ORIGIN = 0 )
    fn push_origin_0(&mut self) {
        self.writer.position = 0;
    }

    /// Serialization Rule: { <OPTIONS> : Byte[2] }
    fn serialize_options(&mut self) -> XTypesResult<()> {
        serialize_primitive_slice(self, &REPRESENTATION_OPTIONS);
        Ok(())
    }

    /// Serialization Rule: { O : AsFinal(O.type) }
    fn serialize_t_as_final(&mut self, dynamic_data: &DynamicData) -> XTypesResult<()> {
        match dynamic_data.r#type().get_kind() {
            TypeKind::ENUM => {
                self.serialize_enum_type(dynamic_data)?;
            }
            TypeKind::STRUCTURE => {
                self.serialize_fstruct_type(dynamic_data)?;
            }
            TypeKind::UNION => {
                self.serialize_funion_type(dynamic_data)?;
            }
            kind => unimplemented!("Should not reach for {kind:?}"),
        }
        Ok(())
    }

    /// Serialization Rule: { O : AsNested(O.type) }
    fn serialize_t_as_nested(&mut self, dynamic_data: &DynamicData) -> XTypesResult<()> {
        match dynamic_data.r#type().get_kind() {
            TypeKind::ENUM => {
                self.serialize_enum_type(dynamic_data)?;
            }
            TypeKind::STRUCTURE => {
                match dynamic_data.r#type().get_descriptor().extensibility_kind {
                    ExtensibilityKind::Final => self.serialize_fstruct_type(dynamic_data),
                    ExtensibilityKind::Appendable => {
                        V::serialize_appendable_type(self, dynamic_data)
                    }
                    ExtensibilityKind::Mutable => V::serialize_mstruct_type(self, dynamic_data),
                }?
            }
            TypeKind::UNION => match dynamic_data.r#type().get_descriptor().extensibility_kind {
                ExtensibilityKind::Final => self.serialize_funion_type(dynamic_data),
                ExtensibilityKind::Appendable => V::serialize_appendable_type(self, dynamic_data),
                ExtensibilityKind::Mutable => V::serialize_munion_type(self, dynamic_data),
            }?,
            kind => unimplemented!("Should not reach for {kind:?}"),
        }
        Ok(())
    }

    /// Serialization Rule: { M.value : M.value.type }
    fn serialize_value(&mut self, v: &DynamicData, member_id: u32) -> Result<(), XTypesError> {
        let member_descriptor = v.get_descriptor(member_id)?;
        let member_type = if member_descriptor.is_external {
            TypeIdentifier::get_type()
        } else {
            member_descriptor.r#type
        };
        match member_type.get_kind() {
            TypeKind::NONE => (),
            TypeKind::BOOLEAN => self.serialize_primitive_type(v.get_boolean_value(member_id)?),
            TypeKind::BYTE => self.serialize_primitive_type(v.get_byte_value(member_id)?),
            TypeKind::INT16 => self.serialize_primitive_type(v.get_int16_value(member_id)?),
            TypeKind::INT32 => self.serialize_primitive_type(v.get_int32_value(member_id)?),
            TypeKind::INT64 => self.serialize_primitive_type(v.get_int64_value(member_id)?),
            TypeKind::UINT16 => self.serialize_primitive_type(v.get_uint16_value(member_id)?),
            TypeKind::UINT32 => self.serialize_primitive_type(v.get_uint32_value(member_id)?),
            TypeKind::UINT64 => self.serialize_primitive_type(v.get_uint64_value(member_id)?),
            TypeKind::FLOAT32 => self.serialize_primitive_type(v.get_float32_value(member_id)?),
            TypeKind::FLOAT64 => self.serialize_primitive_type(v.get_float64_value(member_id)?),
            TypeKind::FLOAT128 => self.serialize_primitive_type(v.get_float128_value(member_id)?),
            TypeKind::INT8 => self.serialize_primitive_type(v.get_int8_value(member_id)?),
            TypeKind::UINT8 => self.serialize_primitive_type(v.get_uint8_value(member_id)?),
            TypeKind::CHAR8 => self.serialize_primitive_type(v.get_char8_value(member_id)?),
            TypeKind::CHAR16 => todo!(),
            TypeKind::STRING8 => self.serialize_string_type(v.get_string_value(member_id)?),
            TypeKind::STRING16 => self.serialize_wstring_type(v.get_string_value(member_id)?),
            TypeKind::ALIAS => (),
            TypeKind::ENUM => self.serialize_enum_type(v.get_complex_value(member_id)?)?,
            TypeKind::BITMASK => todo!(),
            TypeKind::ANNOTATION => todo!(),
            TypeKind::STRUCTURE => self.serialize_t_as_nested(v.get_complex_value(member_id)?)?,
            TypeKind::UNION => self.serialize_t_as_nested(v.get_complex_value(member_id)?)?,
            TypeKind::BITSET => todo!(),
            TypeKind::SEQUENCE => {
                if is_element_type_kind_primitive(member_descriptor)? {
                    self.serialize_p_sequence_type(v, member_id)?
                } else {
                    V::serialize_sequence_type(self, v, member_id)?
                }
            }
            TypeKind::ARRAY => {
                if is_element_type_kind_primitive(member_descriptor)? {
                    self.serialize_p_array_type(v, member_id)?
                } else {
                    V::serialize_array_type(self, v, member_id)?
                }
            }
            TypeKind::MAP => todo!(),
        }
        Ok(())
    }

    /// Serialization Rule: { O[i] : O.element_type }*
    fn serialize_elements(&mut self, v: &DynamicData, member_id: u32) -> Result<(), XTypesError> {
        let member_descriptor = v.get_descriptor(member_id)?;
        let element_type = member_descriptor
            .r#type
            .descriptor
            .element_type
            .expect("array has element type");
        match element_type.get_descriptor().kind {
            TypeKind::NONE => todo!(),
            TypeKind::BOOLEAN => {
                serialize_primitive_slice(self, v.get_boolean_values(member_id)?);
            }
            TypeKind::BYTE => self.writer.write_slice(v.get_byte_values(member_id)?),
            TypeKind::INT16 => serialize_primitive_slice(self, v.get_int16_values(member_id)?),
            TypeKind::INT32 => serialize_primitive_slice(self, v.get_int32_values(member_id)?),
            TypeKind::INT64 => serialize_primitive_slice(self, v.get_int64_values(member_id)?),
            TypeKind::UINT16 => serialize_primitive_slice(self, v.get_uint16_values(member_id)?),
            TypeKind::UINT32 => serialize_primitive_slice(self, v.get_uint32_values(member_id)?),
            TypeKind::UINT64 => serialize_primitive_slice(self, v.get_uint64_values(member_id)?),
            TypeKind::FLOAT32 => serialize_primitive_slice(self, v.get_float32_values(member_id)?),
            TypeKind::FLOAT64 => serialize_primitive_slice(self, v.get_float64_values(member_id)?),
            TypeKind::FLOAT128 => {
                serialize_primitive_slice(self, v.get_float128_values(member_id)?)
            }
            TypeKind::INT8 => serialize_primitive_slice(self, v.get_int8_values(member_id)?),
            TypeKind::UINT8 => self.writer.write_slice(v.get_uint8_values(member_id)?),
            TypeKind::CHAR8 => serialize_primitive_slice(self, v.get_char8_values(member_id)?),
            TypeKind::CHAR16 => todo!(),
            TypeKind::STRING8 => {
                for v in v.get_string_values(member_id)? {
                    self.serialize_string_type(v);
                }
            }
            TypeKind::STRING16 => {
                for v in v.get_string_values(member_id)? {
                    self.serialize_wstring_type(v);
                }
            }
            TypeKind::ALIAS => todo!(),
            TypeKind::BITMASK => todo!(),
            TypeKind::ANNOTATION => todo!(),
            TypeKind::ENUM | TypeKind::STRUCTURE => {
                for v in v.get_complex_values(member_id)? {
                    self.serialize_t_as_nested(v)?;
                }
            }
            TypeKind::UNION => {
                for v in v.get_complex_values(member_id)? {
                    self.serialize_funion_type(v)?;
                }
            }
            TypeKind::BITSET => todo!(),
            TypeKind::SEQUENCE => todo!(),
            TypeKind::ARRAY => todo!(),
            TypeKind::MAP => todo!(),
        };
        Ok(())
    }

    /// Serialization Rule: { O.length : UInt32 }
    fn serialize_length(&mut self, v: &DynamicData, member_id: u32) -> Result<(), XTypesError> {
        let length = match v.get_value(member_id)? {
            DataStorage::UInt8(_)
            | DataStorage::Int8(_)
            | DataStorage::UInt16(_)
            | DataStorage::Int16(_)
            | DataStorage::Int32(_)
            | DataStorage::UInt32(_)
            | DataStorage::Int64(_)
            | DataStorage::UInt64(_)
            | DataStorage::Float32(_)
            | DataStorage::Float64(_)
            | DataStorage::Float128(_)
            | DataStorage::Char8(_)
            | DataStorage::Boolean(_)
            | DataStorage::String(_)
            | DataStorage::ComplexValue(_) => 1,
            DataStorage::SequenceUInt8(items) => items.len(),
            DataStorage::SequenceInt8(items) => items.len(),
            DataStorage::SequenceUInt16(items) => items.len(),
            DataStorage::SequenceInt16(items) => items.len(),
            DataStorage::SequenceInt32(items) => items.len(),
            DataStorage::SequenceUInt32(items) => items.len(),
            DataStorage::SequenceInt64(items) => items.len(),
            DataStorage::SequenceUInt64(items) => items.len(),
            DataStorage::SequenceFloat32(items) => items.len(),
            DataStorage::SequenceFloat64(items) => items.len(),
            DataStorage::SequenceFloat128(items) => items.len(),
            DataStorage::SequenceChar8(items) => items.len(),
            DataStorage::SequenceBoolean(items) => items.len(),
            DataStorage::SequenceString(items) => items.len(),
            DataStorage::SequenceComplexValue(items) => items.len(),
        };

        self.serialize_primitive_type(&(length as u32));
        Ok(())
    }

    /// Serialization Rule { M: FMEMBER }
    fn serialize_fmember(&mut self, v: &DynamicData, member_id: u32) -> XTypesResult<()> {
        let dynamic_type = v.r#type();
        if dynamic_type.get_member(member_id)?.descriptor.is_optional {
            V::serialize_opt_fmember(self, v, member_id)
        } else {
            self.serialize_nopt_fmember(v, member_id)
        }
    }

    /// Serialization Rule { O.selected_member : MMEMBER }?
    fn serialize_selected_member_mmember(&mut self, v: &DynamicData) -> XTypesResult<()> {
        // In the case a a Variant without value only the discriminant is serialized
        if let Ok(member_id) = v.get_member_id_at_index(1) {
            V::serialize_mmember(self, v, member_id)
        } else {
            Ok(())
        }
    }

    /// Serialization Rule (1)
    ///
    /// XCDR << {O : TOP_LEVEL_TYPE} =
    ///            XCDR
    ///              << INIT( OFFSET=0, ORIGIN=0,
    ///                       CENDIAN=<E>, EVERSION=<eversion> )
    ///              << { ENC_HEADER(<E>, <eversion>, O.type) : Byte[2] }
    ///              << PUSH( EVERSION = <eversion> )
    ///              << PUSH( MAXALIGN = MAXALIGN(<eversion>) )
    ///              << PUSH( ORIGIN = 0 )
    ///              << { <OPTIONS> : Byte[2] }
    ///              << { O : AsNested(O.type) }
    fn serialize_top_level_type(&mut self, dynamic_data: &DynamicData) -> XTypesResult<()> {
        V::serialize_enc_header(self, dynamic_data)?;
        self.serialize_options()?;
        self.push_origin_0();
        self.serialize_t_as_nested(dynamic_data)?;
        Ok(())
    }

    /// Serialization Rule (2)
    ///
    /// XCDR << {O : PRIMITIVE_TYPE} =
    ///          XCDR
    ///             << ALIGN( O.ssize )
    ///             << ESWAP( AsBytes(O) )
    fn serialize_primitive_type<O: Ossize + AsBytes>(&mut self, v: &O) {
        V::align(self, O::SSIZE);
        v.as_bytes::<E>(&mut self.writer);
    }

    /// Serialization Rule (3)
    ///
    /// XCDR << {O : STRING_TYPE} =
    ///            XCDR
    ///              << { O.ssize : UInt32 } // includes NUL
    ///              << { O[i] : Byte }* // includes NUL
    fn serialize_string_type(&mut self, v: &str) {
        self.serialize_primitive_type(&(v.len() as u32 + 1));
        self.writer.write_slice(v.as_bytes());
        self.writer.write_byte(0);
    }

    /// Serialization Rule (4)
    ///
    /// XCDR << {O : WSTRING_TYPE} =
    ///            XCDR
    ///              << { O.ssize : UInt32 }
    ///              << { O[i] : Wchar }*
    fn serialize_wstring_type(&mut self, v: &str) {
        let utf16_units: Vec<u16> = v.encode_utf16().collect();
        self.serialize_primitive_type(&((utf16_units.len() * 2) as u32));
        for unit in utf16_units {
            self.serialize_primitive_type(&unit);
        }
    }

    /// Serialization Rule (5)
    ///
    /// XCDR << {O : ENUM_TYPE} =
    ///          XCDR
    ///            << { O.value : O.holder_type }
    fn serialize_enum_type(&mut self, v: &DynamicData) -> Result<(), XTypesError> {
        let holder_type = v
            .r#type()
            .descriptor
            .discriminator_type
            .ok_or(XTypesError::InvalidType)?;
        match holder_type.descriptor.kind {
            TypeKind::INT8 => {
                self.serialize_primitive_type(v.get_int8_value(0)?);
                Ok(())
            }
            TypeKind::INT16 => {
                self.serialize_primitive_type(v.get_int16_value(0)?);
                Ok(())
            }
            TypeKind::INT32 => {
                self.serialize_primitive_type(v.get_int32_value(0)?);
                Ok(())
            }
            _ => Err(XTypesError::InvalidType),
        }
    }

    /// Arrays of primitive element type (version 1 and 2 encoding)
    ///
    /// Serialization Rule (8)
    ///
    /// XCDR << {O : PARRAY_TYPE} =
    ///           XCDR
    ///             << { O[i] : O.element_type }*
    fn serialize_p_array_type(&mut self, v: &DynamicData, member_id: u32) -> XTypesResult<()> {
        self.serialize_elements(v, member_id)
    }

    /// Sequences of primitive element type (version 1 and 2 encoding)
    ///
    /// Serialization Rule (11)
    ///
    /// XCDR << { O : PSEQUENCE_TYPE } =
    ///            XCDR
    ///              << { O.length : UInt32 }
    ///              << { O[i] : O.element_type }*
    fn serialize_p_sequence_type(&mut self, v: &DynamicData, member_id: u32) -> XTypesResult<()> {
        self.serialize_length(v, member_id)?;
        self.serialize_elements(v, member_id)
    }

    /// Structures with extensibility FINAL (version 1 and 2 encoding)
    /// FMMEBER can be NOPT_FMEMBER (18) or OPT_FMEMBER (19)
    ///
    /// Serialization Rule (17)
    ///
    /// XCDR << {O : FSTRUCT_TYPE} =
    ///           XCDR
    ///           << { O.member[i] : FMEMBER }*
    fn serialize_fstruct_type(&mut self, v: &DynamicData) -> Result<(), XTypesError> {
        let dynamic_type = v.r#type();

        for field_index in 0..dynamic_type.get_member_count() {
            let member_id = dynamic_type.get_member_by_index(field_index)?.get_id();
            self.serialize_fmember(v, member_id)?;
        }
        Ok(())
    }

    /// Non-optional member of final Aggregated type (structure, union)
    ///
    /// Serialization Rule (18)
    ///
    /// XCDR << {M : NOPT_FMEMBER} =
    ///           XCDR
    ///            << { M.value : M.value.type }
    fn serialize_nopt_fmember(
        &mut self,
        v: &DynamicData,
        member_id: u32,
    ) -> Result<(), XTypesError> {
        self.serialize_value(v, member_id)
    }

    /// Unions with extensibility FINAL (version 1 and 2 encoding)
    /// see (18) to (20) for NOPT_FMEMBER and FMEMBER serialization
    ///
    /// Serialization Rule (26)
    ///
    /// XCDR << {O : FUNION_TYPE} =
    ///           XCDR
    ///             << { O.disc : NOPT_FMEMBER }
    ///             << { O.selected_member : FMEMBER }?
    fn serialize_funion_type(&mut self, v: &DynamicData) -> Result<(), XTypesError> {
        self.serialize_nopt_fmember(v, 0)?;

        if let Ok(member_id) = v.get_member_id_at_index(1) {
            self.serialize_fmember(v, member_id)
        } else {
            Ok(())
        }
    }
}

const PID_LIST_END: u16 = 0x3F02;

struct Ssize<'a, 'b, E, V> {
    serializer: &'a mut XTypesSerializer<'b, E, V>,
    initial_pos: usize,
}

impl<'a, 'b, E: EndiannessWrite, V> Ssize<'a, 'b, E, V> {
    fn new(serializer: &'a mut XTypesSerializer<'b, E, V>) -> Self {
        // Place holder:
        serializer.writer.write_slice(&[0, 0]);
        let initial_pos = serializer.writer.buffer.len();

        Self {
            serializer,
            initial_pos,
        }
    }
    fn write_ssize(self) {
        let ssize = (self.serializer.writer.buffer.len() - self.initial_pos) as u16;
        self.serializer
            .writer
            .write_slice_at_position(E::to_bytes_u16(ssize).as_slice(), self.initial_pos - 2);
    }
}

struct Dheader<'a, 'b, E, V> {
    serializer: &'a mut XTypesSerializer<'b, E, V>,
    initial_pos: usize,
}

impl<'a, 'b, E: EndiannessWrite, V: EncodingVersion> Dheader<'a, 'b, E, V> {
    fn new(serializer: &'a mut XTypesSerializer<'b, E, V>) -> Self {
        // Place holder:
        serializer.serialize_primitive_type(&0u32);
        let initial_pos = serializer.writer.buffer.len();

        Self {
            serializer,
            initial_pos,
        }
    }
    fn write_header(self) {
        let ssize = (self.serializer.writer.buffer.len() - self.initial_pos) as u32;
        self.serializer
            .writer
            .write_slice_at_position(&E::to_bytes_u32(ssize), self.initial_pos - 4);
    }
}

struct EMheader1<'a, 'b, E, V> {
    serializer: &'a mut XTypesSerializer<'b, E, V>,
    initial_pos: usize,
}

impl<'a, 'b, E: EndiannessWrite, V: EncodingVersion> EMheader1<'a, 'b, E, V> {
    fn new(serializer: &'a mut XTypesSerializer<'b, E, V>) -> Self {
        // Place holder:
        serializer.serialize_primitive_type(&0u32);
        let initial_pos = serializer.writer.buffer.len();

        Self {
            serializer,
            initial_pos,
        }
    }
    fn write_header(self, member_id: u32, v: &DynamicData) -> XTypesResult<()> {
        let ssize = (self.serializer.writer.buffer.len() - self.initial_pos) as u32;
        let member_descriptor = v.get_descriptor(member_id)?;
        let m_flag = member_descriptor.is_must_understand as u32;
        let is_next_member_having_dheader = (matches!(
            member_descriptor.r#type.get_kind(),
            TypeKind::STRUCTURE | TypeKind::UNION
        ) && matches!(
            member_descriptor.r#type.descriptor.extensibility_kind,
            ExtensibilityKind::Appendable | ExtensibilityKind::Mutable
        )) || (member_descriptor.r#type.get_kind()
            == TypeKind::SEQUENCE
            && !is_element_type_kind_primitive(member_descriptor)?);
        let lc = if is_next_member_having_dheader {
            5
        } else if member_descriptor.r#type.get_kind() == TypeKind::SEQUENCE
            && is_element_type_kind_primitive(member_descriptor)?
        {
            let elem_kind = member_descriptor
                .r#type
                .descriptor
                .element_type
                .as_ref()
                .unwrap()
                .get_kind();
            match elem_kind {
                TypeKind::BOOLEAN
                | TypeKind::BYTE
                | TypeKind::INT8
                | TypeKind::UINT8
                | TypeKind::CHAR8 => 5,
                TypeKind::INT32 | TypeKind::UINT32 | TypeKind::FLOAT32 => 6,
                TypeKind::INT64 | TypeKind::UINT64 | TypeKind::FLOAT64 => 7,
                _ => 4,
            }
        } else {
            match ssize {
                1 => 0,
                2 => 1,
                4 => 2,
                8 => 3,
                _ => 4,
            }
        } as u32;

        let emheader = (m_flag << 31) + (lc << 28) + (member_id & 0x0fffffff);

        self.serializer
            .writer
            .write_slice_at_position(&E::to_bytes_u32(emheader), self.initial_pos - 4);

        if lc == 4 {
            self.serializer
                .writer
                .buffer
                .splice(self.initial_pos..self.initial_pos, E::to_bytes_u32(ssize));
            self.serializer.writer.position += 4;
        }

        Ok(())
    }
}

fn is_element_type_kind_primitive(member_descriptor: &MemberDescriptor) -> XTypesResult<bool> {
    Ok(matches!(
        member_descriptor
            .r#type
            .descriptor
            .element_type
            .ok_or(XTypesError::InvalidType)?
            .get_kind(),
        TypeKind::BOOLEAN
            | TypeKind::BYTE
            | TypeKind::INT16
            | TypeKind::INT32
            | TypeKind::INT64
            | TypeKind::UINT16
            | TypeKind::UINT32
            | TypeKind::UINT64
            | TypeKind::FLOAT32
            | TypeKind::FLOAT64
            | TypeKind::FLOAT128
            | TypeKind::INT8
            | TypeKind::UINT8
            | TypeKind::CHAR8
            | TypeKind::CHAR16
    ))
}

trait EncodingVersion: Sized {
    /// Rule: ALIGN( O.ssize )
    fn align<'a, E>(serializer: &mut XTypesSerializer<'a, E, Self>, v: usize);

    /// Serialization Rule: { ENC_HEADER(<E>, <eversion>, O.type) : Byte[2] }
    fn serialize_enc_header<'a, E: EndiannessWrite>(
        serializer: &mut XTypesSerializer<'a, E, Self>,
        dynamic_data: &DynamicData,
    ) -> XTypesResult<()>;

    /// Serialization Rule (9) & (10)
    fn serialize_array_type<'a, E: EndiannessWrite>(
        serializer: &mut XTypesSerializer<'a, E, Self>,
        v: &DynamicData,
        member_id: u32,
    ) -> Result<(), XTypesError>;

    /// Serialization Rule (12) & (13)
    fn serialize_sequence_type<'a, E: EndiannessWrite>(
        serializer: &mut XTypesSerializer<'a, E, Self>,
        v: &DynamicData,
        member_id: u32,
    ) -> Result<(), XTypesError>;

    /// Serialization Rule (19) & (20)
    fn serialize_opt_fmember<'a, E: EndiannessWrite>(
        serializer: &mut XTypesSerializer<'a, E, Self>,
        v: &DynamicData,
        member_id: u32,
    ) -> Result<(), XTypesError>;

    /// Serialization Rule (21) & (23)
    fn serialize_mstruct_type<'a, E: EndiannessWrite>(
        serializer: &mut XTypesSerializer<'a, E, Self>,
        v: &DynamicData,
    ) -> Result<(), XTypesError>;

    /// Serialization Rule (22) & (24) & (25)
    fn serialize_mmember<'a, E: EndiannessWrite>(
        serializer: &mut XTypesSerializer<'a, E, Self>,
        v: &DynamicData,
        member_id: u32,
    ) -> Result<(), XTypesError>;

    /// Serialization Rule (27) & (28)
    fn serialize_munion_type<'a, E: EndiannessWrite>(
        serializer: &mut XTypesSerializer<'a, E, Self>,
        _v: &DynamicData,
    ) -> Result<(), XTypesError>;

    /// Serialization Rule (29) & (30)
    fn serialize_appendable_type<'a, E: EndiannessWrite>(
        serializer: &mut XTypesSerializer<'a, E, Self>,
        v: &DynamicData,
    ) -> Result<(), XTypesError>;
}

struct EncodingVersion1;
impl EncodingVersion for EncodingVersion1 {
    /// Rule: ALIGN( O.ssize )
    fn align<'a, E>(serializer: &mut XTypesSerializer<'a, E, Self>, v: usize) {
        serializer.writer.pad(core::cmp::min(v, 8))
    }

    /// Serialization Rule: { ENC_HEADER(<E>, <eversion>, O.type) : Byte[2] }
    fn serialize_enc_header<'a, E: EndiannessWrite>(
        serializer: &mut XTypesSerializer<'a, E, Self>,
        dynamic_data: &DynamicData,
    ) -> XTypesResult<()> {
        serialize_primitive_slice(
            serializer,
            &match dynamic_data.r#type().get_descriptor().extensibility_kind {
                ExtensibilityKind::Final | ExtensibilityKind::Appendable => match E::ENDIANNESS {
                    Endianness::Big => CDR_BE,
                    Endianness::Little => CDR_LE,
                },
                ExtensibilityKind::Mutable => match E::ENDIANNESS {
                    Endianness::Big => PL_CDR_BE,
                    Endianness::Little => PL_CDR_LE,
                },
            },
        );
        Ok(())
    }

    /// Arrays (any extensibility) using version 1 encoding
    ///
    /// Serialization Rule (10)
    ///
    ///   XCDR[1] << {O : ARRAY_TYPE} =
    ///                XCDR
    ///                  << { O[i] : O.element_type }*
    fn serialize_array_type<'a, E: EndiannessWrite>(
        serializer: &mut XTypesSerializer<'a, E, Self>,
        v: &DynamicData,
        member_id: u32,
    ) -> Result<(), XTypesError> {
        serializer.serialize_elements(v, member_id)
    }

    /// Sequences (any extensibility) using version 1 encoding
    ///
    /// Serialization Rule (13)
    ///
    /// XCDR[1] << {O : SEQUENCE_TYPE} =
    ///              XCDR
    ///                << { O.length : UInt32 }
    ///                << { O[i] : O.element_type }*
    fn serialize_sequence_type<'a, E: EndiannessWrite>(
        serializer: &mut XTypesSerializer<'a, E, Self>,
        v: &DynamicData,
        member_id: u32,
    ) -> Result<(), XTypesError> {
        serializer.serialize_length(v, member_id)?;
        serializer.serialize_elements(v, member_id)
    }

    /// Optional member of final Aggregated type (structure, union), version 1
    /// see (26) and (27) for MMEMBER serialization
    ///
    /// Serialization Rule (19)
    ///
    /// XCDR[1] << {M : OPT_FMEMBER} =
    ///             XCDR
    ///              << { M : MMEMBER }
    fn serialize_opt_fmember<'a, E: EndiannessWrite>(
        serializer: &mut XTypesSerializer<'a, E, Self>,
        v: &DynamicData,
        member_id: u32,
    ) -> Result<(), XTypesError> {
        Self::serialize_mmember(serializer, v, member_id)
    }

    /// Structures with extensibility MUTABLE, version 2 encoding
    ///
    /// Serialization Rule (23)
    ///
    /// XCDR[1] << {O : MSTRUCT_TYPE} =
    ///               XCDR
    ///                 << { O.member[i] : MMEMBER }*
    ///                 << { PID_SENTINEL : UInt16 }
    ///                                      << { length = 0 : UInt16 }
    fn serialize_mstruct_type<'a, E: EndiannessWrite>(
        serializer: &mut XTypesSerializer<'a, E, Self>,
        v: &DynamicData,
    ) -> Result<(), XTypesError> {
        let dynamic_type = v.r#type();
        for field_index in 0..dynamic_type.get_member_count() {
            let member = dynamic_type.get_member_by_index(field_index)?;
            let member_id = member.get_id();
            if v.get_value(member_id).is_ok() {
                Self::serialize_mmember(serializer, v, member_id)?;
            }
        }
        // TODO: The alignment is not done in the Xtypes specification (possibly this needs to be deleted)
        Self::align(serializer, 4);
        serializer.serialize_primitive_type(&PID_LIST_END);
        serializer.serialize_primitive_type(&0u16);
        Ok(())
    }

    /// Member of mutable aggregated type (structure, union), version 1 encoding
    /// using short PL encoding when both M.id <= 2^14 and M.value.ssize <= 2^16
    ///
    /// Serialization Rule (24)
    ///
    /// XCDR[1] << {M : MMEMBER} =
    ///             XCDR
    ///               << ALIGN(4)
    ///               << { FLAG_I + FLAG_M + M.id : UInt16 }
    ///               << { M.value.ssize : UInt16 }
    ///               << PUSH( ORIGIN=0 )
    ///               << { M.value : M.value.type }
    fn serialize_mmember<'a, E: EndiannessWrite>(
        serializer: &mut XTypesSerializer<'a, E, Self>,
        v: &DynamicData,
        member_id: u32,
    ) -> Result<(), XTypesError> {
        Self::align(serializer, 4);
        let member_descriptor = v.get_descriptor(member_id)?;
        let m_flag = member_descriptor.is_must_understand as u16;
        let pid = member_id as u16 + (m_flag << 14);
        serializer.serialize_primitive_type(&pid);
        let ssize = Ssize::new(serializer);
        ssize.serializer.push_origin_0();
        if v.get_value(member_id).is_ok() {
            ssize.serializer.serialize_value(v, member_id).unwrap();
        }
        ssize.write_ssize();
        Ok(())
    }

    /// Unions with extensibility MUTABLE, version 1 encoding
    /// see (25)-(26) for serialization of MMEMBER using version 1 encoding
    ///
    /// Serialization Rule (28)
    ///
    /// XCDR[1] << {O : MUNION_TYPE} =
    ///             XCDR
    ///               << { O.disc : MMEMBER }
    ///               << { O.selected_member : MMEMBER }?
    ///               << { PID_SENTINEL : UInt16 }
    ///               << { length = 0 : UInt16 }
    fn serialize_munion_type<'a, E: EndiannessWrite>(
        serializer: &mut XTypesSerializer<'a, E, Self>,
        v: &DynamicData,
    ) -> Result<(), XTypesError> {
        Self::serialize_mmember(serializer, v, 0)?;
        serializer.serialize_selected_member_mmember(v)?;
        // TODO: The alignment is not done in the Xtypes specification (possibly this needs to be deleted)
        Self::align(serializer, 4);
        serializer.serialize_primitive_type(&PID_LIST_END);
        serializer.serialize_primitive_type(&0u16);
        Ok(())
    }

    /// Extensibility APPENDABLE (Collection or Aggregated types), version 1 encoding
    ///
    /// Serialization Rule (29)
    ///
    /// XCDR[1] << {O : APPENDABLE_TYPE} =
    ///               XCDR
    ///                 << { O : AsFinal(O.type) }
    fn serialize_appendable_type<'a, E: EndiannessWrite>(
        serializer: &mut XTypesSerializer<'a, E, Self>,
        v: &DynamicData,
    ) -> Result<(), XTypesError> {
        serializer.serialize_t_as_final(v)
    }
}

struct EncodingVersion2;
impl EncodingVersion for EncodingVersion2 {
    /// Rule: ALIGN( O.ssize )
    fn align<'a, E>(serializer: &mut XTypesSerializer<'a, E, Self>, v: usize) {
        serializer.writer.pad(core::cmp::min(v, 4))
    }

    /// Serialization Rule: { ENC_HEADER(<E>, <eversion>, O.type) : Byte[2] }
    fn serialize_enc_header<'a, E: EndiannessWrite>(
        serializer: &mut XTypesSerializer<'a, E, Self>,
        dynamic_data: &DynamicData,
    ) -> XTypesResult<()> {
        serialize_primitive_slice(
            serializer,
            &match dynamic_data.r#type().get_descriptor().extensibility_kind {
                ExtensibilityKind::Final => match E::ENDIANNESS {
                    Endianness::Big => CDR2_BE,
                    Endianness::Little => CDR2_LE,
                },
                ExtensibilityKind::Appendable => match E::ENDIANNESS {
                    Endianness::Big => D_CDR2_BE,
                    Endianness::Little => D_CDR2_LE,
                },
                ExtensibilityKind::Mutable => match E::ENDIANNESS {
                    Endianness::Big => PL_CDR2_BE,
                    Endianness::Little => PL_CDR2_LE,
                },
            },
        );
        Ok(())
    }

    /// Arrays (any extensibility) using version 2 encoding
    ///
    /// Serialization Rule (9)
    ///
    /// XCDR[2] << {O : ARRAY_TYPE} =
    ///              XCDR
    ///                << { DHEADER(O) : UINT32 }
    ///                << { O[i] : O.element_type }*
    fn serialize_array_type<'a, E: EndiannessWrite>(
        serializer: &mut XTypesSerializer<'a, E, Self>,
        v: &DynamicData,
        member_id: u32,
    ) -> Result<(), XTypesError> {
        let dheader = Dheader::new(serializer);
        dheader.serializer.serialize_elements(v, member_id)?;
        dheader.write_header();

        Ok(())
    }

    /// Sequences (any extensibility) using version 1 encoding
    ///
    /// Serialization Rule (12)
    ///
    /// XCDR[2] << {O : SEQUENCE_TYPE} =
    ///             XCDR
    ///              << { DHEADER(O) : UINT32 }
    ///              << { O.length : UInt32 }
    ///              << { O[i] : O.element_type }*
    fn serialize_sequence_type<'a, E: EndiannessWrite>(
        serializer: &mut XTypesSerializer<'a, E, Self>,
        v: &DynamicData,
        member_id: u32,
    ) -> Result<(), XTypesError> {
        let dheader = Dheader::new(serializer);
        dheader.serializer.serialize_length(v, member_id)?;
        dheader.serializer.serialize_elements(v, member_id)?;
        dheader.write_header();
        Ok(())
    }

    /// Optional member of final aggregated type (structure, union), version 2
    ///
    /// Serialization Rule (20)
    ///
    /// XCDR[2] << {M : OPT_FMEMBER} =
    ///             XCDR
    ///               << { <is_present> : BOOLEAN }
    ///               << IF (<is_present>) { M.value : M.value.type }
    fn serialize_opt_fmember<'a, E: EndiannessWrite>(
        serializer: &mut XTypesSerializer<'a, E, Self>,
        v: &DynamicData,
        member_id: u32,
    ) -> Result<(), XTypesError> {
        let is_present = v.get_data_kind(member_id).is_ok();
        serializer.serialize_primitive_type(&is_present);
        if is_present {
            serializer.serialize_value(v, member_id)?;
        }

        Ok(())
    }

    /// Structures with extensibility MUTABLE, version 2 encoding
    ///
    /// Serialization Rule (21)
    ///
    /// XCDR[2] << {O : MSTRUCT_TYPE} =
    ///             XCDR
    ///               << { DHEADER(O) : UInt32 }
    ///               << { O.member[i] : MMEMBER }*
    fn serialize_mstruct_type<'a, E: EndiannessWrite>(
        serializer: &mut XTypesSerializer<'a, E, Self>,
        v: &DynamicData,
    ) -> Result<(), XTypesError> {
        let dheader = Dheader::new(serializer);
        let dynamic_type = v.r#type();
        for field_index in 0..dynamic_type.get_member_count() {
            let member = dynamic_type.get_member_by_index(field_index)?;
            let member_id = member.get_id();
            if v.get_value(member_id).is_ok() {
                Self::serialize_mmember(dheader.serializer, v, member_id)?;
            }
        }
        dheader.write_header();
        Ok(())
    }

    /// Member of mutable aggregated type (structure, union), version 2 encoding
    ///
    /// Serialization Rule (22)
    ///
    /// XCDR[2] << {M : MMEMBER} =
    ///              XCDR
    ///                << { EMHEADER1(M) : UInt32 }
    ///                << IF (LC(M)>=4) { NEXTINT(M) : UInt32 }
    ///                << IF (LC(M)>=5) XCDR.offset = XCDR.offset-4
    ///                << { M.value : M.value.type }
    fn serialize_mmember<'a, E: EndiannessWrite>(
        serializer: &mut XTypesSerializer<'a, E, Self>,
        v: &DynamicData,
        member_id: u32,
    ) -> Result<(), XTypesError> {
        let emheader = EMheader1::new(serializer);
        emheader.serializer.serialize_value(v, member_id)?;
        emheader.write_header(member_id, v)?;
        Ok(())
    }

    /// Unions with extensibility MUTABLE, version 2 encoding
    /// see (22) for serialization of MMEMBER using version 2 encoding
    ///
    /// Serialization Rule (27)
    ///
    /// XCDR[2] << {O : MUNION_TYPE} =
    ///             XCDR
    ///               << { DHEADER(O) : UInt32 }
    ///               << { O.disc : MMEMBER }
    ///               << { O.selected_member : MMEMBER }?
    fn serialize_munion_type<'a, E: EndiannessWrite>(
        serializer: &mut XTypesSerializer<'a, E, Self>,
        v: &DynamicData,
    ) -> Result<(), XTypesError> {
        let dheader = Dheader::new(serializer);
        Self::serialize_mmember(dheader.serializer, v, 0)?;
        dheader.serializer.serialize_selected_member_mmember(v)?;
        dheader.write_header();
        Ok(())
    }

    /// Extensibility APPENDABLE (Collection or Aggregated types), version 2 encoding
    ///
    /// Serialization Rule (30)
    ///
    /// XCDR[2] << {O : APPENDABLE_TYPE} =
    ///              XCDR
    ///              << { DHEADER(O) : UInt32 }
    ///              << { O : AsFinal(O.type) }
    fn serialize_appendable_type<'a, E: EndiannessWrite>(
        serializer: &mut XTypesSerializer<'a, E, Self>,
        v: &DynamicData,
    ) -> Result<(), XTypesError> {
        let dheader = Dheader::new(serializer);
        dheader.serializer.serialize_t_as_final(v)?;
        dheader.write_header();
        Ok(())
    }
}

trait Ossize {
    const SSIZE: usize;
}
impl Ossize for u8 {
    const SSIZE: usize = Self::BITS as usize / 8;
}
impl Ossize for u16 {
    const SSIZE: usize = Self::BITS as usize / 8;
}
impl Ossize for u32 {
    const SSIZE: usize = Self::BITS as usize / 8;
}
impl Ossize for u64 {
    const SSIZE: usize = Self::BITS as usize / 8;
}
impl Ossize for i8 {
    const SSIZE: usize = Self::BITS as usize / 8;
}
impl Ossize for i16 {
    const SSIZE: usize = Self::BITS as usize / 8;
}
impl Ossize for i32 {
    const SSIZE: usize = Self::BITS as usize / 8;
}
impl Ossize for i64 {
    const SSIZE: usize = Self::BITS as usize / 8;
}
impl Ossize for i128 {
    const SSIZE: usize = Self::BITS as usize / 8;
}
impl Ossize for f32 {
    const SSIZE: usize = 4;
}
impl Ossize for f64 {
    const SSIZE: usize = 8;
}
impl Ossize for bool {
    const SSIZE: usize = 1;
}
impl Ossize for char {
    const SSIZE: usize = 1;
}

trait AsBytes {
    fn as_bytes<'a, E: EndiannessWrite>(&self, writer: &mut CdrWriter<'a>);
}
impl AsBytes for bool {
    fn as_bytes<'a, E>(&self, writer: &mut CdrWriter<'a>) {
        writer.write_byte(if *self { 1 } else { 0 });
    }
}
impl AsBytes for u8 {
    fn as_bytes<'a, E>(&self, writer: &mut CdrWriter<'a>) {
        writer.write_byte(*self);
    }
}
impl AsBytes for u16 {
    fn as_bytes<'a, E: EndiannessWrite>(&self, writer: &mut CdrWriter<'a>) {
        writer.write_slice(&E::to_bytes_u16(*self));
    }
}
impl AsBytes for u32 {
    fn as_bytes<'a, E: EndiannessWrite>(&self, writer: &mut CdrWriter<'a>) {
        writer.write_slice(&E::to_bytes_u32(*self));
    }
}
impl AsBytes for u64 {
    fn as_bytes<'a, E: EndiannessWrite>(&self, writer: &mut CdrWriter<'a>) {
        writer.write_slice(&E::to_bytes_u64(*self));
    }
}
impl AsBytes for i8 {
    fn as_bytes<'a, E>(&self, writer: &mut CdrWriter<'a>) {
        writer.write_byte(*self as u8);
    }
}
impl AsBytes for i16 {
    fn as_bytes<'a, E: EndiannessWrite>(&self, writer: &mut CdrWriter<'a>) {
        writer.write_slice(&E::to_bytes_i16(*self));
    }
}
impl AsBytes for i32 {
    fn as_bytes<'a, E: EndiannessWrite>(&self, writer: &mut CdrWriter<'a>) {
        writer.write_slice(&E::to_bytes_i32(*self));
    }
}
impl AsBytes for i64 {
    fn as_bytes<'a, E: EndiannessWrite>(&self, writer: &mut CdrWriter<'a>) {
        writer.write_slice(&E::to_bytes_i64(*self));
    }
}
impl AsBytes for i128 {
    fn as_bytes<'a, E: EndiannessWrite>(&self, writer: &mut CdrWriter<'a>) {
        writer.write_slice(&E::to_bytes_i128(*self));
    }
}
impl AsBytes for f32 {
    fn as_bytes<'a, E: EndiannessWrite>(&self, writer: &mut CdrWriter<'a>) {
        writer.write_slice(&E::to_bytes_f32(*self));
    }
}
impl AsBytes for f64 {
    fn as_bytes<'a, E: EndiannessWrite>(&self, writer: &mut CdrWriter<'a>) {
        writer.write_slice(&E::to_bytes_f64(*self));
    }
}
impl AsBytes for char {
    fn as_bytes<'a, E>(&self, writer: &mut CdrWriter<'a>) {
        writer.write_slice(self.to_string().as_bytes());
    }
}

enum Endianness {
    Big,
    Little,
}
trait EndiannessWrite {
    const ENDIANNESS: Endianness;
    fn to_bytes_u16(v: u16) -> [u8; 2];
    fn to_bytes_i16(v: i16) -> [u8; 2];
    fn to_bytes_i32(v: i32) -> [u8; 4];
    fn to_bytes_u32(v: u32) -> [u8; 4];
    fn to_bytes_i64(v: i64) -> [u8; 8];
    fn to_bytes_u64(v: u64) -> [u8; 8];
    fn to_bytes_i128(v: i128) -> [u8; 16];
    fn to_bytes_f32(v: f32) -> [u8; 4];
    fn to_bytes_f64(v: f64) -> [u8; 8];
}

struct BigEndian;
impl EndiannessWrite for BigEndian {
    const ENDIANNESS: Endianness = Endianness::Big;
    fn to_bytes_u16(v: u16) -> [u8; 2] {
        v.to_be_bytes()
    }
    fn to_bytes_i16(v: i16) -> [u8; 2] {
        v.to_be_bytes()
    }
    fn to_bytes_i32(v: i32) -> [u8; 4] {
        v.to_be_bytes()
    }
    fn to_bytes_u32(v: u32) -> [u8; 4] {
        v.to_be_bytes()
    }
    fn to_bytes_i64(v: i64) -> [u8; 8] {
        v.to_be_bytes()
    }
    fn to_bytes_u64(v: u64) -> [u8; 8] {
        v.to_be_bytes()
    }
    fn to_bytes_i128(v: i128) -> [u8; 16] {
        v.to_be_bytes()
    }
    fn to_bytes_f32(v: f32) -> [u8; 4] {
        v.to_be_bytes()
    }
    fn to_bytes_f64(v: f64) -> [u8; 8] {
        v.to_be_bytes()
    }
}

struct LittleEndian;
impl EndiannessWrite for LittleEndian {
    const ENDIANNESS: Endianness = Endianness::Little;
    fn to_bytes_u16(v: u16) -> [u8; 2] {
        v.to_le_bytes()
    }
    fn to_bytes_i16(v: i16) -> [u8; 2] {
        v.to_le_bytes()
    }
    fn to_bytes_i32(v: i32) -> [u8; 4] {
        v.to_le_bytes()
    }
    fn to_bytes_u32(v: u32) -> [u8; 4] {
        v.to_le_bytes()
    }
    fn to_bytes_i64(v: i64) -> [u8; 8] {
        v.to_le_bytes()
    }
    fn to_bytes_u64(v: u64) -> [u8; 8] {
        v.to_le_bytes()
    }
    fn to_bytes_i128(v: i128) -> [u8; 16] {
        v.to_le_bytes()
    }
    fn to_bytes_f32(v: f32) -> [u8; 4] {
        v.to_le_bytes()
    }
    fn to_bytes_f64(v: f64) -> [u8; 8] {
        v.to_le_bytes()
    }
}

struct CdrWriter<'a> {
    buffer: &'a mut Vec<u8>,
    position: usize,
}
impl<'a> CdrWriter<'a> {
    fn new(buffer: &'a mut Vec<u8>) -> Self {
        Self {
            buffer,
            position: 0,
        }
    }
    fn write_byte(&mut self, value: u8) {
        self.buffer.push(value);
        self.position += 1;
    }
    fn write_slice(&mut self, value: &[u8]) {
        self.buffer.extend_from_slice(value);
        self.position += value.len();
    }
    fn write_slice_at_position(&mut self, value: &[u8], position: usize) {
        self.buffer[position..position + value.len()].copy_from_slice(value);
    }
    fn pad(&mut self, alignment: usize) {
        const ZEROS: [u8; 8] = [0; 8];
        let padding = self.position.div_ceil(alignment) * alignment - self.position;
        self.write_slice(&ZEROS[..padding]);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        dcps::data_representation_builtin_endpoints::type_lookup::{
            RemoteExceptionCode, ReplyHeader, RequestHeader, SampleIdentity, TypeLookupCall,
            TypeLookupGetTypeDependenciesIn, TypeLookupGetTypeDependenciesOut,
            TypeLookupGetTypeDependenciesResult, TypeLookupGetTypesIn, TypeLookupGetTypesOut,
            TypeLookupGetTypesResult, TypeLookupReply, TypeLookupRequest, TypeLookupReturn,
        },
        transport::types::{EntityId, Guid},
        xtypes::{
            dynamic_type::{DynamicDataFactory, DynamicTypeBuilderFactory},
            type_object::{
                TypeIdentifier, TypeIdentifierWithDependencies, TypeIdentifierWithSize,
                TypeInformation,
            },
            type_support::{Type, TypeSupport},
        },
    };
    extern crate std;

    #[test]
    fn serialize_basic_types_struct() {
        #[derive(TypeSupport, Clone)]
        struct BasicTypes {
            f1: bool,
            f2: i8,
            f3: i16,
            f4: i32,
            f5: i64,
            f6: u8,
            f7: u16,
            f8: u32,
            f9: u64,
            f10: f32,
            f11: f64,
            f12: char,
        }

        let v = BasicTypes {
            f1: true,
            f2: 2,
            f3: 3,
            f4: 4,
            f5: 5,
            f6: 6,
            f7: 7,
            f8: 8,
            f9: 9,
            f10: 1.0,
            f11: 1.0,
            f12: 'a',
        }
        .create_dynamic_sample();
        assert_eq!(
            serialize_cdr1_be(&v).unwrap(),
            vec![
                0x00, 0x00, 0x00, 0x03, // CDR Header (incl. padding length)
                1, 2, 0, 3, 0, 0, 0, 4, // f1: bool | f2: i8 | f3: i16 | f4: i32
                0, 0, 0, 0, 0, 0, 0, 5, // f5: i64
                6, 0, 0, 7, 0, 0, 0, 8, // f6: u8 | padding (1 byte) | f7: u16 | f8: u32
                0, 0, 0, 0, 0, 0, 0, 9, // f9: u64
                0x3F, 0x80, 0x00, 0x00, 0, 0, 0, 0, // f10: f32 | padding (4 bytes)
                0x3F, 0xF0, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, // f11: f64
                b'a', 0, 0, 0 // f12: char | padding (3 bytes)
            ]
        );
        assert_eq!(
            serialize_cdr1_le(&v).unwrap(),
            vec![
                0x00, 0x01, 0x00, 0x03, // CDR Header (incl. padding length)
                1, 2, 3, 0, 4, 0, 0, 0, // f1: bool | f2: i8 | f3: i16 | f4: i32
                5, 0, 0, 0, 0, 0, 0, 0, // f5: i64
                6, 0, 7, 0, 8, 0, 0, 0, // f6: u8 | padding (1 byte) | f7: u16 | f8: u32
                9, 0, 0, 0, 0, 0, 0, 0, // f9: u64
                0x00, 0x00, 0x80, 0x3F, 0, 0, 0, 0, // f10: f32 | padding (4 bytes)
                0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0xF0, 0x3F, // f11: f64
                b'a', 0, 0, 0 // f12: char | padding (3 bytes)
            ]
        );
        assert_eq!(
            serialize_cdr2_be(&v).unwrap(),
            vec![
                0x00, 0x06, 0x00, 0x03, // CDR Header (incl. padding length)
                1, 2, 0, 3, // f1: bool | f2: i8 | f3: i16
                0, 0, 0, 4, // f4: i32
                0, 0, 0, 0, // f5-1: i64
                0, 0, 0, 5, // f5-2: i64
                6, 0, 0, 7, // f6: u8 | padding (1 byte) | f7: u16
                0, 0, 0, 8, // f8: u32
                0, 0, 0, 0, 0, 0, 0, 9, // f9: u64
                0x3F, 0x80, 0x00, 0x00, // f10: f32
                0x3F, 0xF0, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, // f11: f64
                b'a', 0, 0, 0 // f12: char | padding (3 bytes)
            ]
        );
        assert_eq!(
            serialize_cdr2_le(&v).unwrap(),
            vec![
                0x00, 0x07, 0x00, 0x03, // CDR Header (incl. padding length)
                1, 2, 3, 0, // f1: bool | f2: i8 | f3: i16
                4, 0, 0, 0, // f4: i32
                5, 0, 0, 0, // f5-1: i64
                0, 0, 0, 0, // f5-2: i64
                6, 0, 7, 0, // f6: u8 | padding (1 byte) | f7: u16
                8, 0, 0, 0, // f8: u32
                9, 0, 0, 0, // f9-1: u64
                0, 0, 0, 0, // f9-2: u64
                0x00, 0x00, 0x80, 0x3F, // f10: f32
                0x00, 0x00, 0x00, 0x00, // f11-1: f64
                0x00, 0x00, 0xF0, 0x3F, // f11-2: f64
                b'a', 0, 0, 0 // f12: char | padding (3 bytes)
            ]
        );
    }

    #[test]
    fn serialize_u8_array() {
        #[derive(TypeSupport)]
        #[dust_dds(extensibility = "mutable")]
        struct U8Array {
            #[dust_dds(id = 41)]
            version: [u8; 2],
        }

        let v = U8Array { version: [1, 2] }.create_dynamic_sample();
        assert_eq!(
            serialize_cdr1_be(&v).unwrap(),
            vec![
                0x00, 0x02, 0x00, 0x00, // CDR Header
                0x00, 41, 0, 2, // PID, length
                1, 2, 0, 0, // version | padding (2 bytres)
                0x3F, 0x02, 0, 0, // Sentinel
            ]
        );
        assert_eq!(
            serialize_cdr1_le(&v).unwrap(),
            vec![
                0x00, 0x03, 0x00, 0x00, // CDR Header
                41, 0x00, 2, 0, // PID, length
                1, 2, 0, 0, // version | padding (2 bytres)
                0x02, 0x3F, 0, 0, // Sentinel
            ]
        );
    }

    #[test]
    fn serialize_array_with_lc4() {
        #[derive(TypeSupport)]
        #[dust_dds(extensibility = "mutable")]
        struct TestType {
            #[dust_dds(id = 41)]
            member: [u8; 3],
        }

        let v = TestType { member: [1, 2, 3] }.create_dynamic_sample();
        assert_eq!(
            serialize_cdr1_be(&v).unwrap(),
            vec![
                0x00, 0x02, 0x00, 0x00, // CDR Header
                0x00, 41, 0, 3, // PID, length
                1, 2, 3, 0, // member | padding (1 bytres)
                0x3F, 0x02, 0, 0, // Sentinel
            ]
        );
        assert_eq!(
            serialize_cdr1_le(&v).unwrap(),
            vec![
                0x00, 0x03, 0x00, 0x00, // CDR Header
                41, 0x00, 3, 0, // PID, length
                1, 2, 3, 0, // member | padding (2 bytres)
                0x02, 0x3F, 0, 0, // Sentinel
            ]
        );
        assert_eq!(
            serialize_cdr2_be(&v).unwrap(),
            vec![
                0x00, 0x0a, 0x00, 0x01, // CDR Header
                0, 0, 0, 11, // DHEADER
                0b100_0000, 0, 0, 41, // EMHEADER1 incl. LC 4
                0, 0, 0, 3, // NEXTINT
                1, 2, 3, 0, // member | padding (1 bytres)
            ]
        );
        assert_eq!(
            serialize_cdr2_le(&v).unwrap(),
            vec![
                0x00, 0x0b, 0x00, 0x01, // CDR Header
                11, 0, 0, 0, // DHEADER
                41, 0, 0, 0b100_0000, // EMHEADER1 incl. LC 4
                3, 0, 0, 0, // NEXTINT
                1, 2, 3, 0, // member | padding (1 bytres)
            ]
        );
    }

    #[test]
    fn serialize_locator() {
        #[derive(TypeSupport)]
        #[dust_dds(extensibility = "mutable")]
        struct LocatorContainer {
            #[dust_dds(id = 73)]
            locator: Locator,
        }
        #[derive(TypeSupport, Default, PartialEq, Eq)]
        #[dust_dds(extensibility = "final", nested)]
        struct Locator {
            kind: i32,
            address1: [u8; 2],
            address2: [u8; 3],
        }

        let v = LocatorContainer {
            locator: Locator {
                kind: 1,
                address1: [3, 4],
                address2: [5, 6, 7],
            },
        }
        .create_dynamic_sample();
        assert_eq!(
            serialize_cdr1_be(&v).unwrap(),
            vec![
                0x00, 0x02, 0x00, 0x00, // CDR Header
                0, 73, 0, 9, // PID | length
                0, 0, 0, 1, // kind
                3, 4, 5, 6, // address1 and 2
                7, 0, 0, 0, // address2 | pading (3 bytes)
                0x3F, 0x02, 0, 0
            ]
        );
    }

    #[test]
    fn serialize_string() {
        #[derive(TypeSupport, Clone)]
        struct StringData(String);

        let v = StringData(String::from("Hola")).create_dynamic_sample();
        assert_eq!(
            serialize_cdr1_be(&v).unwrap(),
            vec![
                0x00, 0x00, 0x00, 0x03, // CDR Header (incl padding length)
                0, 0, 0, 5, //length
                b'H', b'o', b'l', b'a', // str
                0x00, 0, 0, 0 // terminating 0 | padding (3 bytes)
            ]
        );
        assert_eq!(
            serialize_cdr1_le(&v).unwrap(),
            vec![
                0x00, 0x01, 0x00, 0x03, // CDR Header (incl padding length)
                5, 0, 0, 0, //length
                b'H', b'o', b'l', b'a', // str
                0x00, 0, 0, 0 // terminating 0 | padding (3 bytes)
            ]
        );
        assert_eq!(
            serialize_cdr2_be(&v).unwrap(),
            vec![
                0x00, 0x06, 0x00, 0x03, // CDR Header (incl padding length)
                0, 0, 0, 5, //length
                b'H', b'o', b'l', b'a', // str
                0x00, 0, 0, 0 // terminating 0 | padding (3 bytes)
            ]
        );
        assert_eq!(
            serialize_cdr2_le(&v).unwrap(),
            vec![
                0x00, 0x07, 0x00, 0x03, // CDR Header (incl padding length)
                5, 0, 0, 0, //length
                b'H', b'o', b'l', b'a', // str
                0x00, 0, 0, 0 // terminating 0 | padding (3 bytes)
            ]
        );
    }

    #[test]
    fn serialize_string_list() {
        #[derive(TypeSupport)]
        struct StringList {
            name: Vec<String>,
        }

        let v = StringList {
            name: vec!["one".to_string(), "two".to_string()],
        }
        .create_dynamic_sample();
        assert_eq!(
            serialize_cdr1_be(&v).unwrap(),
            vec![
                0x00, 0x00, 0x00, 0x00, // CDR Header
                0, 0, 0, 2, // vec length
                0, 0, 0, 4, // String length
                b'o', b'n', b'e', 0, // String
                0, 0, 0, 4, // String length
                b't', b'w', b'o', 0, // String
            ]
        );
    }

    #[derive(TypeSupport, Clone)]
    struct FinalType {
        field_u16: u16,
        field_u64: u64,
    }

    #[test]
    fn serialize_final_struct() {
        let v = FinalType {
            field_u16: 7,
            field_u64: 9,
        }
        .create_dynamic_sample();
        // PLAIN_CDR:
        assert_eq!(
            serialize_cdr1_be(&v).unwrap(),
            vec![
                0x00, 0x00, 0x00, 0x00, // CDR Header
                0, 7, 0, 0, 0, 0, 0, 0, // field_u16 | padding (6 bytes)
                0, 0, 0, 0, 0, 0, 0, 9, // field_u64
            ]
        );
        assert_eq!(
            serialize_cdr1_le(&v).unwrap(),
            vec![
                0x00, 0x01, 0x00, 0x00, // CDR Header
                7, 0, 0, 0, 0, 0, 0, 0, // field_u16 | padding (6 bytes)
                9, 0, 0, 0, 0, 0, 0, 0, // field_u64
            ]
        );
        // PLAIN_CDR2:
        assert_eq!(
            serialize_cdr2_be(&v).unwrap(),
            vec![
                0x00, 0x06, 0x00, 0x00, // CDR Header
                0, 7, 0, 0, // field_u16 | padding (2 bytes)
                0, 0, 0, 0, 0, 0, 0, 9, // field_u64
            ]
        );
        assert_eq!(
            serialize_cdr2_le(&v).unwrap(),
            vec![
                0x00, 0x07, 0x00, 0x00, // CDR Header
                7, 0, 0, 0, // field_u16 | padding (2 bytes)
                9, 0, 0, 0, 0, 0, 0, 0, // field_u64
            ]
        );
    }

    #[derive(TypeSupport, Clone)]
    struct NestedFinalType {
        field_nested: FinalType,
        field_u8: u8,
    }

    #[test]
    fn serialize_nested_final_struct() {
        let v = NestedFinalType {
            field_nested: FinalType {
                field_u16: 7,
                field_u64: 9,
            },
            field_u8: 10,
        }
        .create_dynamic_sample();
        assert_eq!(
            serialize_cdr1_be(&v).unwrap(),
            vec![
                0x00, 0x00, 0x00, 0x03, // CDR Header (incl padding length)
                0, 7, 0, 0, 0, 0, 0, 0, // nested FinalType (u16) | padding (6 bytes)
                0, 0, 0, 0, 0, 0, 0, 9, // u64
                10, 0, 0, 0 // u8 | padding (3 bytes)
            ]
        );
        assert_eq!(
            serialize_cdr1_le(&v).unwrap(),
            vec![
                0x00, 0x01, 0x00, 0x03, // CDR Header (incl padding length)
                7, 0, 0, 0, 0, 0, 0, 0, // nested FinalType (u16) | padding (6 bytes)
                9, 0, 0, 0, 0, 0, 0, 0, // u64
                10, 0, 0, 0 // u8 | padding (3 bytes)
            ]
        );
        assert_eq!(
            serialize_cdr2_be(&v).unwrap(),
            vec![
                0x00, 0x06, 0x00, 0x03, // CDR Header (incl padding length)
                0, 7, 0, 0, // nested FinalType (u16) | padding
                0, 0, 0, 0, 0, 0, 0, 9, // u64
                10, 0, 0, 0 // u8 | padding (3 bytes)
            ]
        );
        assert_eq!(
            serialize_cdr2_le(&v).unwrap(),
            vec![
                0x00, 0x07, 0x00, 0x03, // CDR Header (incl padding length)
                7, 0, 0, 0, // nested FinalType (u16) | padding (2 bytes)
                9, 0, 0, 0, 0, 0, 0, 0, // u64
                10, 0, 0, 0 // u8 | padding (3 bytes)
            ]
        );
    }

    #[derive(TypeSupport, Clone)]
    #[dust_dds(extensibility = "appendable", nested)]
    struct AppendableType {
        value: u16,
    }

    #[test]
    fn serialize_appendable_struct() {
        let v = AppendableType { value: 7 }.create_dynamic_sample();
        assert_eq!(
            serialize_cdr1_be(&v).unwrap(),
            vec![
                0x00, 0x00, 0x00, 0x02, // CDR Header (incl padding length)
                0, 7, 0, 0 // value | padding (2 bytes)
            ]
        );
        assert_eq!(
            serialize_cdr1_le(&v).unwrap(),
            vec![
                0x00, 0x01, 0x00, 0x02, // CDR Header (incl padding length)
                7, 0, 0, 0 // value | padding (2 bytes)
            ]
        );
        assert_eq!(
            serialize_cdr2_be(&v).unwrap(),
            vec![
                0x00, 0x08, 0x00, 0x02, // CDR Header (incl padding length)
                0, 0, 0, 2, // DHEADER
                0, 7, 0, 0 // value | padding (2 bytes)
            ]
        );
        assert_eq!(
            serialize_cdr2_le(&v).unwrap(),
            vec![
                0x00, 0x09, 0x00, 0x02, // CDR Header (incl padding length)
                2, 0, 0, 0, // DHEADER
                7, 0, 0, 0 // value | padding (2 bytes)
            ]
        );
    }

    #[test]
    fn serialize_mutable_struct_simple() {
        #[derive(Debug, PartialEq, TypeSupport)]
        #[dust_dds(extensibility = "mutable")]
        struct MutableType {
            #[dust_dds(id = 0x01)]
            x1: u32,
        }
        let data = MutableType { x1: 1 }.create_dynamic_sample();

        assert_eq!(
            serialize_cdr2_le(&data).unwrap(),
            vec![
                0x00, 0x0b, 0x00, 0x00, // CDR Header
                8, 0, 0, 0, // DHEADER (length)
                0x01, 0, 0, 0b010_0000, // EMHEADER1 incl. LC 0b010 (4 bytes)
                1, 0, 0, 0, // u32
            ]
        );
    }

    #[test]
    fn serialize_mutable_struct() {
        #[derive(TypeSupport, Clone, Default, PartialEq)]
        #[dust_dds(extensibility = "mutable")]
        struct MutableType {
            #[dust_dds(id = 0x3091, key)]
            one_byte: u8,
            #[dust_dds(id = 0x2081)]
            two_bytes: u16,
        }

        let v = MutableType {
            one_byte: 7,
            two_bytes: 0x0809,
        }
        .create_dynamic_sample();
        assert_eq!(
            serialize_cdr1_be(&v).unwrap(),
            vec![
                0x00, 0x02, 0x00, 0x00, // CDR Header
                0x70, 0x91, 0, 1, // PID + M_FLAG | length
                7, 0, 0, 0, // one_byte | padding
                0x20, 0x81, 0, 2, // PID | length
                0x08, 0x09, 0, 0, // two_bytes | padding (2 bytes)
                0x3F, 0x02, 0, 0, // Sentinel
            ]
        );
        assert_eq!(
            serialize_cdr1_le(&v).unwrap(),
            vec![
                0x00, 0x03, 0x00, 0x00, // CDR Header
                0x91, 0x70, 1, 0, // PID + M_FLAG | length
                7, 0, 0, 0, // one_byte | padding
                0x81, 0x20, 2, 0, // PID | length
                0x09, 0x08, 0, 0, // two_bytes | padding (2 bytes)
                0x02, 0x3F, 0, 0, // Sentinel
            ]
        );
        assert_eq!(
            serialize_cdr2_be(&v).unwrap(),
            vec![
                0x00, 0x0a, 0x00, 0x02, // CDR Header
                0, 0, 0, 14, // DHEADER (length)
                128, 0, 0x30, 0x91, // EMHEADER1 incl. LC 0b000 (1 bytes)
                7, 0, 0, 0, // one_byte | padding 3 bytes
                0b001_0000, 0, 0x20, 0x81, // EMHEADER1 incl. LC 0b001 (2 bytes)
                0x08, 0x09, 0, 0, // two_bytes | padding (2 bytes)
            ]
        );
        assert_eq!(
            serialize_cdr2_le(&v).unwrap(),
            vec![
                0x00, 0x0b, 0x00, 0x02, // CDR Header
                14, 0, 0, 0, // DHEADER (length)
                0x91, 0x30, 0, 128, // EMHEADER1 incl. LC 0b000 (1 bytes)
                7, 0, 0, 0, // one_byte | padding 3 bytes
                0x81, 0x20, 0, 0b001_0000, // EMHEADER1 incl. LC 0b001 (2 bytes)
                0x09, 0x08, 0, 0, // two_bytes | padding (2 bytes)
            ]
        );
    }

    #[test]
    fn serialize_nested_mutable_struct() {
        #[derive(TypeSupport, Clone, Default, PartialEq)]
        struct TinyFinalType {
            primitive: u16,
        }

        #[derive(TypeSupport, Clone, Default, PartialEq)]
        #[dust_dds(extensibility = "mutable")]
        struct MutableType {
            #[dust_dds(id = 90, key)]
            one_byte: u8,
            #[dust_dds(id = 80)]
            two_bytes: u16,
        }

        #[derive(TypeSupport, Clone)]
        #[dust_dds(extensibility = "mutable")]
        struct NestedMutableType {
            #[dust_dds(id = 96, key)]
            field_primitive: u8,
            #[dust_dds(id = 97)]
            field_mutable: MutableType,
            #[dust_dds(id = 98)]
            field_final: TinyFinalType,
        }

        let v = NestedMutableType {
            field_primitive: 5,
            field_mutable: MutableType {
                one_byte: 7,
                two_bytes: 8,
            },
            field_final: TinyFinalType { primitive: 9 },
        }
        .create_dynamic_sample();
        assert_eq!(
            serialize_cdr1_be(&v).unwrap(),
            vec![
                0x00, 0x02, 0x00, 0x00, // CDR Header
                0x40, 96, 0, 1, // PID (+ M_FLAG) | length
                5, 0, 0, 0, // field_primitive | padding (3 bytes)
                0x00, 97, 0, 20, // PID | length
                0x40, 90, 0, 1, // field_mutable: PID (+ M_FLAG) | length
                7, 0, 0, 0, // field_mutable: one_byte | padding (3 bytes)
                0x00, 80, 0, 2, // field_mutable: PID | length
                0, 8, 0, 0, // field_mutable: two_bytes | padding (2 bytes)
                0x3F, 0x02, 0, 0, // field_mutable: Sentinel
                0x00, 98, 0, 2, // field_mutable: PID | length
                0, 9, 0, 0, // field_final: primitive | padding (2 bytes)
                0x3F, 0x02, 0, 0, // Sentinel
            ]
        );
        assert_eq!(
            serialize_cdr1_le(&v).unwrap(),
            vec![
                0x00, 0x03, 0x00, 0x00, // CDR Header
                96, 0x40, 1, 0, // PID (+ M_FLAG) | length
                5, 0, 0, 0, // field_primitive | padding (3 bytes)
                97, 0x00, 20, 0, // PID | length
                90, 0x40, 1, 0, // field_mutable: PID (+ M_FLAG) | length
                7, 0, 0, 0, // field_mutable: one_byte | padding (3 bytes)
                0x050, 0x00, 2, 0, // field_mutable: PID | length
                8, 0, 0, 0, // field_mutable: two_bytes | padding (2 bytes)
                0x02, 0x3F, 0, 0, // field_mutable: Sentinel
                98, 0x00, 2, 0, // field_mutable: PID | length
                9, 0, 0, 0, // field_final: primitive | padding (2 bytes)
                0x02, 0x3F, 0, 0, // Sentinel
            ]
        );
    }

    #[test]
    fn serialize_appendable_shapes() {
        #[derive(Debug, PartialEq, TypeSupport)]
        #[dust_dds(extensibility = "appendable")]
        struct AppendableShapesType {
            #[dust_dds(key)]
            color: String,
            x: i32,
            y: i32,
            shapesize: i32,
            additional_payload_size: Vec<u8>,
        }
        let v = AppendableShapesType {
            color: String::from("BLUE"),
            x: 10,
            y: 20,
            shapesize: 30,
            additional_payload_size: vec![],
        }
        .create_dynamic_sample();

        assert_eq!(
            serialize_cdr1_be(&v).unwrap(),
            vec![
                0x00, 0x00, 0x00, 0x00, // CDR_BE
                0, 0, 0, 5, // color: length
                b'B', b'L', b'U', b'E', // color
                0, 0, 0, 0, // color: terminating 0 | padding
                0, 0, 0, 10, // x
                0, 0, 0, 20, // y
                0, 0, 0, 30, // shapesize
                0, 0, 0, 0, // additional_payload_size: length
            ]
        );
        assert_eq!(
            serialize_cdr1_le(&v).unwrap(),
            vec![
                0x00, 0x01, 0x00, 0x00, // CDR_LE
                5, 0, 0, 0, // color: length
                b'B', b'L', b'U', b'E', // color
                0, 0, 0, 0, // color: terminating 0 | padding
                10, 0, 0, 0, // x
                20, 0, 0, 0, // y
                30, 0, 0, 0, // shapesize
                0, 0, 0, 0, // additional_payload_size: length
            ]
        );
        assert_eq!(
            serialize_cdr2_be(&v).unwrap(),
            vec![
                0x00, 0x08, 0x00, 0x00, // D_CDR2_BE
                0, 0, 0, 28, // Dheader
                0, 0, 0, 5, // color: length
                b'B', b'L', b'U', b'E', // color
                0, 0, 0, 0, // color: terminating 0 | padding
                0, 0, 0, 10, // x
                0, 0, 0, 20, // y
                0, 0, 0, 30, // shapesize
                0, 0, 0, 0, // additional_payload_size: length
            ]
        );
        assert_eq!(
            serialize_cdr2_le(&v).unwrap(),
            vec![
                0x00, 0x09, 0x00, 0x00, // D_CDR2_LE
                28, 0, 0, 0, // Dheader
                5, 0, 0, 0, // color: length
                b'B', b'L', b'U', b'E', // color
                0, 0, 0, 0, // color: terminating 0 | padding
                10, 0, 0, 0, // x
                20, 0, 0, 0, // y
                30, 0, 0, 0, // shapesize
                0, 0, 0, 0, // additional_payload_size: length
            ]
        );
    }

    #[test]
    fn serialize_final_union_type() {
        #[derive(Debug, PartialEq, TypeSupport)]
        struct MyInnerType(u32);

        #[derive(Debug, PartialEq, TypeSupport)]
        #[dust_dds(extensibility = "final", switch(u16))]
        enum MyDynamicType {
            #[dust_dds(case = 5)]
            VariantA(MyInnerType),
            #[dust_dds(case = 6)]
            VariantB { a: u32 },
            #[dust_dds(case = 7)]
            VariantC,
        }
        let variantb = MyDynamicType::VariantB { a: 10 }.create_dynamic_sample();

        assert_eq!(
            serialize_cdr1_be(&variantb).unwrap(),
            vec![
                0x00, 0x00, 0x00, 0x00, // CDR_BE
                0, 6, 0, 0, // discriminant (u16) | padding (2 bytes)
                0, 0, 0, 10, // u32 (VariantB)
            ]
        );
        assert_eq!(
            serialize_cdr2_be(&variantb).unwrap(),
            vec![
                0x00, 0x06, 0x00, 0x00, // CDR2_BE
                0, 6, 0, 0, // discriminant (u16) | padding (2 bytes)
                0, 0, 0, 10, // u32 (VariantB)
            ]
        );
        let variantc = MyDynamicType::VariantC.create_dynamic_sample();

        assert_eq!(
            serialize_cdr1_be(&variantc).unwrap(),
            vec![
                0x00, 0x00, 0x00, 0x02, // CDR_BE
                0, 7, 0, 0, // discriminant (u16) | padding (2 bytes)
            ]
        );
        assert_eq!(
            serialize_cdr2_be(&variantc).unwrap(),
            vec![
                0x00, 0x06, 0x00, 0x02, // CDR2_BE
                0, 7, 0, 0, // discriminant (u16) | padding (2 bytes)
            ]
        );

        let varianta = MyDynamicType::VariantA(MyInnerType(10)).create_dynamic_sample();

        assert_eq!(
            serialize_cdr1_be(&varianta).unwrap(),
            vec![
                0x00, 0x00, 0x00, 0x00, // CDR_BE
                0, 5, 0, 0, // discriminant (u16) | padding (2 bytes)
                0, 0, 0, 10, // u32 (VariantA)
            ]
        );
        assert_eq!(
            serialize_cdr2_be(&varianta).unwrap(),
            vec![
                0x00, 0x06, 0x00, 0x00, // CDR2_BE
                0, 5, 0, 0, // discriminant (u16) | padding (2 bytes)
                0, 0, 0, 10, // u32 (VariantA)
            ]
        );
    }

    #[test]
    fn serialize_mutable_union_type() {
        #[derive(Debug, PartialEq, TypeSupport)]
        struct MyInnerType(u32);

        #[derive(Debug, PartialEq, TypeSupport)]
        #[dust_dds(extensibility = "mutable", switch(u16))]
        enum MyDynamicType {
            #[dust_dds(case = 5)]
            VariantA(MyInnerType),
            #[dust_dds(case = 6)]
            VariantB { a: u32 },
        }

        let variant_a = MyDynamicType::VariantA(MyInnerType(10)).create_dynamic_sample();
        #[rustfmt::skip]
        let expected = vec![
            0, 0x02, 0, 0, // PL_CDR_BE
            0b0100_0000, 0, 0, 2, // SMHEADER1 M_FLAG + ID 0 (discriminator always) | length
            0, 5, 0, 0, // discriminant (u16) | padding (2 bytes)
            0b0000_0000, 1, 0, 4, // SMHEADER1 ID 1 (auto asigned) | length
            0, 0, 0, 10, // selected_member u32 (VariantA)
            0x3F, 0x02, 0, 0, // SENTINEL
        ];
        assert_eq!(serialize_cdr1_be(&variant_a).unwrap(), expected);
        #[rustfmt::skip]
        let expected = vec![
            0, 0x0a, 0, 0, // PL_CDR2_BE
            0, 0, 0, 16, // DHEADER
            0b1001_0000, 0, 0, 0, // EMHEADER1 M_FLAG | LC 0b001 | ID 0 (discriminator always)
            0, 5, 0, 0, // discriminant (u16) | padding (2 bytes)
            0b010_0000, 0, 0, 1, // EMHEADER1 LC 0b010 | ID 1 (auto asigned)
            0, 0, 0, 10, // selected_member u32 (VariantA)
        ];
        assert_eq!(serialize_cdr2_be(&variant_a).unwrap(), expected);

        let variant_b = MyDynamicType::VariantB { a: 10 }.create_dynamic_sample();
        #[rustfmt::skip]
        let expected = vec![
            0, 0x02, 0, 0, // PL_CDR_BE
            0b0100_0000, 0, 0, 2, // SMHEADER1 M_FLAG + ID 0 (discriminator always) | length
            0, 6, 0, 0, // discriminant (u16) | padding (2 bytes)
            0b0000_0000, 2, 0, 4, // SMHEADER1 ID 2 (auto asigned) | length
            0, 0, 0, 10, // selected_member u32 (VariantA)
            0x3F, 0x02, 0, 0, // SENTINEL
        ];
        assert_eq!(serialize_cdr1_be(&variant_b).unwrap(), expected);
        #[rustfmt::skip]
        let expected = vec![
            0x00, 0x0a, 0x00, 0x00, // PL_CDR2_BE
            0x00, 0x00, 0x00, 16, // DHEADER
            0b1001_0000, 0, 0, 0, // EMHEADER1 M_FLAG | LC 0b001 | ID 0 (dicriminator always)
            0x00, 6, 0x00, 0x00, // discriminant (u16) | padding (2 bytes)
            0b010_0000, 0, 0, 2, // EMHEADER1 LC 0b010 | ID 2 (auto asigned)
            0x00, 0x00, 0, 10, // selected_member u32 (VariantB)
        ];
        assert_eq!(serialize_cdr2_be(&variant_b).unwrap(), expected);
    }

    #[test]
    fn serialize_final_union_type_nested() {
        #[derive(Debug, PartialEq, TypeSupport)]
        #[dust_dds(extensibility = "final", switch(u16))]
        enum MyDynamicType {
            #[dust_dds(case = 6)]
            VariantB { a: u32 },
            #[dust_dds(case = 7)]
            VariantC,
        }

        #[derive(Debug, PartialEq, TypeSupport)]
        struct MyType {
            field: MyDynamicType,
        }

        let variantb = MyType {
            field: MyDynamicType::VariantB { a: 10 },
        }
        .create_dynamic_sample();

        assert_eq!(
            serialize_cdr1_be(&variantb).unwrap(),
            vec![
                0x00, 0x00, 0x00, 0x00, // CDR_BE
                0, 6, 0, 0, // discriminant (u16) | padding (2 bytes)
                0, 0, 0, 10, // u32 (VariantB)
            ]
        );
    }

    #[test]
    fn mutable_struct_with_optional_id() {
        #[derive(TypeSupport, Clone)]
        #[dust_dds(extensibility = "mutable")]
        struct MutableTypeAutoId {
            #[dust_dds(key)]
            key: u8,
            participant_key: u16,
        }

        let type_info = MutableTypeAutoId::TYPE;
        assert_eq!(
            type_info
                .get_member_by_index(0)
                .unwrap()
                .get_descriptor()
                .unwrap()
                .id,
            0
        );
        assert_eq!(
            type_info
                .get_member_by_index(1)
                .unwrap()
                .get_descriptor()
                .unwrap()
                .id,
            1
        );
    }

    #[test]
    fn mutable_struct_with_mixed_ids() {
        #[derive(TypeSupport, Clone)]
        #[dust_dds(extensibility = "mutable")]
        struct MutableTypeMixedId {
            #[dust_dds(id = 10)]
            key: u8,
            participant_key: u16,
            #[dust_dds(id = 20)]
            extra_field_1: u32,
            extra_field_2: u32,
            extra_field_3: u32,
        }

        let type_info = MutableTypeMixedId::TYPE;
        assert_eq!(
            type_info
                .get_member_by_index(0)
                .unwrap()
                .get_descriptor()
                .unwrap()
                .id,
            10
        );
        assert_eq!(
            type_info
                .get_member_by_index(1)
                .unwrap()
                .get_descriptor()
                .unwrap()
                .id,
            11
        );
        assert_eq!(
            type_info
                .get_member_by_index(2)
                .unwrap()
                .get_descriptor()
                .unwrap()
                .id,
            20
        );
        assert_eq!(
            type_info
                .get_member_by_index(3)
                .unwrap()
                .get_descriptor()
                .unwrap()
                .id,
            21
        );
        assert_eq!(
            type_info
                .get_member_by_index(4)
                .unwrap()
                .get_descriptor()
                .unwrap()
                .id,
            22
        );
    }

    #[test]
    fn serialize_mutable_struct_with_sequence() {
        #[derive(Debug, PartialEq, TypeSupport)]
        #[dust_dds(extensibility = "mutable")]
        struct MutableTypeWithSequence {
            my_sequence: Vec<u32>,
        }

        let data = MutableTypeWithSequence {
            my_sequence: vec![1, 2, 3],
        }
        .create_dynamic_sample();

        assert_eq!(
            serialize_cdr2_le(&data).unwrap(),
            vec![
                0x00, 0x0b, 0x00, 0x00, // PL_CDR2_LE
                20, 0, 0, 0, // Struct DHEADER (20 bytes payload)
                0, 0, 0, 96, // my_sequence EMHEADER (LC=6, member_id=0)
                3, 0, 0, 0, // NEXTINT: 3
                1, 0, 0, 0, // Vec[0]
                2, 0, 0, 0, // Vec[1]
                3, 0, 0, 0, // Vec[2]
            ]
        );
    }

    #[test]
    fn serialize_appendable_union_with_mutable_struct() {
        #[derive(Debug, PartialEq, TypeSupport)]
        #[dust_dds(extensibility = "mutable")]
        struct MutableTypeWithSequence {
            my_sequence: Vec<u32>,
        }

        #[derive(Debug, PartialEq, TypeSupport)]
        #[dust_dds(switch(i32), extensibility = "appendable")]
        pub enum AppendableUnion {
            #[dust_dds(case = 10)]
            MyVariant { a: MutableTypeWithSequence },
        }

        let data = AppendableUnion::MyVariant {
            a: MutableTypeWithSequence {
                my_sequence: vec![1, 2, 3],
            },
        }
        .create_dynamic_sample();

        assert_eq!(
            serialize_cdr2_le(&data).unwrap(),
            vec![
                0x00, 0x09, 0x00, 0x00, // D_CDR2_LE
                28, 0, 0, 0, // AppendableUnion DHEADER (28 bytes payload)
                10, 0, 0, 0, // MyVariant discriminator
                20, 0, 0, 0, // MutableTypeWithSequence DHEADER (20 bytes)
                0, 0, 0, 96, // my_sequence EMHEADER (LC=6, member_id=0)
                3, 0, 0, 0, // NEXTINT: 3
                1, 0, 0, 0, // Vec[0]
                2, 0, 0, 0, // Vec[1]
                3, 0, 0, 0, // Vec[2]
            ]
        );
    }

    #[test]
    fn serialize_type_lookup_get_types_in() {
        let data = TypeLookupCall::TypeLookupGetTypesHashId {
            get_types: TypeLookupGetTypesIn {
                type_ids: vec![TypeIdentifier::EkComplete {
                    equivalence_hash: [5; 14],
                }],
            },
        }
        .create_dynamic_sample();

        assert_eq!(
            serialize_cdr2_le(&data).unwrap(),
            vec![
                0x00u8, 0x09, 0x00, 0x01, // D_CDR2_LE + padding
                35, 0, 0, 0, // TypeLookupCall DHEADER
                0xd3, 0x52, 0x82, 0x01, // TypeLookupGetTypesHashId DISCRIMINATOR
                27, 0, 0, 0, // type_ids: DHEADER
                101, 96, 83, 92, // type_ids EMHEADER (LC=5 | 0x0c536065)
                19, 0, 0, 0, // type_ids: DHEADER
                1, 0, 0, 0,   // type_ids: Length
                242, // EK_COMPLETE
                5, 5, 5, 5, 5, 5, 5, 5, 5, 5, 5, 5, 5, 5, // equivalence_hash
                0, // padding
            ]
        );
    }

    #[test]
    fn serialize_type_lookup_get_dependencies_in() {
        let data = TypeLookupCall::TypeLookupGetDependenciesHash {
            get_type_dependencies: TypeLookupGetTypeDependenciesIn {
                type_ids: vec![TypeIdentifier::EkComplete {
                    equivalence_hash: [5; 14],
                }],
                continuation_point: vec![],
            },
        }
        .create_dynamic_sample();

        assert_eq!(
            serialize_cdr2_le(&data).unwrap(),
            vec![
                0x00u8, 0x09, 0x00, 0x00, // D_CDR2_LE + padding
                44, 0, 0, 0, // TypeLookupCall DHEADER (44 bytes payload)
                0x31, 0xfb, 0xaa,
                0x05, // TypeLookupGetDependenciesHash DISCRIMINATOR (0x05aafb31)
                36, 0, 0, 0, // TypeLookupGetTypeDependenciesIn DHEADER
                101, 96, 83, 92, // type_ids EMHEADER (LC=5 | 0x0c536065)
                19, 0, 0, 0, // type_ids: DHEADER
                1, 0, 0, 0,   // type_ids: Length
                242, // EK_COMPLETE
                5, 5, 5, 5, 5, 5, 5, 5, 5, 5, 5, 5, 5, 5, // equivalence_hash
                0, // padding
                210, 227, 8, 85, // continuation_point EMHEADER (LC=5 | 0x0508e3d2)
                0, 0, 0, 0, // continuation_point length 0
            ]
        );
    }

    #[test]
    fn serialize_type_lookup_request() {
        let data = TypeLookupRequest {
            header: RequestHeader {
                request_id: SampleIdentity {
                    writer_guid: Guid::new([1; 12], EntityId::new([1; 3], 1)),
                    sequence_number: 5.into(),
                },
                instance_name: String::from(""),
            },
            call: TypeLookupCall::TypeLookupGetTypesHashId {
                get_types: TypeLookupGetTypesIn { type_ids: vec![] },
            },
        }
        .create_dynamic_sample();

        assert_eq!(
            serialize_cdr2_le(&data).unwrap(),
            vec![
                0x00u8, 0x07, 0x00, 0x00, // CDR2_LE + padding
                1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, // writer_guid
                0, 0, 0, 0, // sequence_number: high
                5, 0, 0, 0, // sequence_number: low
                1, 0, 0, 0, // instance_name: length
                0, // instance_name: length
                0, 0, 0, // Padding
                20, 0, 0, 0, // TypeLookupCall DHEADER
                211, 82, 130, 1, // TypeLookupGetTypesHashId Discriminator
                12, 0, 0, 0, // type_ids: DHEADER
                101, 96, 83, 92, // type_ids EMHEADER (LC=5 | 0x0c536065)
                4, 0, 0, 0, // type_ids: DHEADER
                0, 0, 0, 0, // type_ids: Length
            ]
        );
    }

    #[test]
    fn serialize_type_lookup_reply() {
        let data = TypeLookupReply {
            header: ReplyHeader {
                related_request_id: SampleIdentity {
                    writer_guid: Guid::new([1; 12], EntityId::new([1; 3], 1)),
                    sequence_number: 5.into(),
                },
                remote_ex: RemoteExceptionCode::Ok,
            },
            r#return: TypeLookupReturn::TypeLookupGetTypesHash {
                get_type: TypeLookupGetTypesResult::Ok {
                    result: TypeLookupGetTypesOut {
                        types: vec![],
                        complete_to_minimal: vec![],
                    },
                },
            },
        }
        .create_dynamic_sample();

        assert_eq!(
            serialize_cdr2_le(&data).unwrap(),
            vec![
                0x00u8, 0x07, 0x00, 0x00, // CDR2_LE + padding
                1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, // writer_guid
                0, 0, 0, 0, // sequence_number: high
                5, 0, 0, 0, // sequence_number: low
                0, 0, 0, 0, // remote_ex: RemoteExceptionCode::Ok
                40, 0, 0, 0, // TypeLookupReturn DHEADER
                211, 82, 130, 1, // TypeLookupGetTypesHash Discriminator (0x018252d3)
                32, 0, 0, 0, // TypeLookupGetTypesResult DHEADER
                0, 0, 0, 0, // Result Discriminator (0 = Ok)
                // TypeLookupGetTypesOut DHEADER (mutable struct)
                24, 0, 0, 0, // types field EMHEADER + DHEADER + length
                209, 74, 128, 82, // types hashid EMHEADER (LC=5 | 0x02804ad1)
                4, 0, 0, 0, // types Vec DHEADER
                0, 0, 0, 0, // types length 0
                // complete_to_minimal field EMHEADER + DHEADER + length
                119, 101, 142, 91, // complete_to_minimal hashid EMHEADER (LC=5 | 0x0b8e6577)
                4, 0, 0, 0, // complete_to_minimal Vec DHEADER
                0, 0, 0, 0, // complete_to_minimal length 0
            ]
        );
    }

    #[test]
    fn serialize_type_lookup_get_dependencies_reply() {
        let type_information = TypeInformation {
            minimal: TypeIdentifierWithDependencies {
                typeid_with_size: TypeIdentifierWithSize {
                    type_id: TypeIdentifier::EkMinimal {
                        equivalence_hash: [5; 14],
                    },
                    typeobject_serialized_size: 10,
                },
                dependent_typeid_count: 0,
                dependent_typeids: Vec::new(),
            },
            complete: TypeIdentifierWithDependencies {
                typeid_with_size: TypeIdentifierWithSize {
                    type_id: TypeIdentifier::EkComplete {
                        equivalence_hash: [
                            0x99, 0xbf, 0x61, 0x62, 0x13, 0xb8, 0xbb, 0x50, 0x0c, 0x96, 0x6b, 0x11,
                            0x2a, 0x5c,
                        ],
                    },
                    typeobject_serialized_size: 151,
                },
                dependent_typeid_count: 0,
                dependent_typeids: Vec::new(),
            },
        };

        let data = TypeLookupReply {
            header: ReplyHeader {
                related_request_id: SampleIdentity {
                    writer_guid: Guid::new(
                        [
                            1, 1, 0x35, 0x98, 0x14, 0xae, 0xf0, 0xd3, 0xef, 0xc6, 0x8d, 0x3d,
                        ],
                        EntityId::new([0, 3, 0], 195),
                    ),
                    sequence_number: 1.into(),
                },
                remote_ex: RemoteExceptionCode::Ok,
            },
            r#return: TypeLookupReturn::TypeLookupGetDependenciesHash {
                get_type_dependencies: TypeLookupGetTypeDependenciesResult::Ok {
                    result: TypeLookupGetTypeDependenciesOut {
                        dependent_typeids: vec![type_information.complete.typeid_with_size.clone()],
                        continuation_point: vec![],
                    },
                },
            },
        }
        .create_dynamic_sample();

        let serialized = serialize_cdr2_le(&data).unwrap();

        let expected = vec![
            0x00, 0x07, 0x00, 0x00, // CDR2_LE header
            1, 1, 0x35, 0x98, 0x14, 0xae, 0xf0, 0xd3, 0xef, 0xc6, 0x8d, 0x3d, 0, 3, 0,
            195, // writer_guid
            0, 0, 0, 0, // sequence_number: high
            1, 0, 0, 0, // sequence_number: low
            0, 0, 0, 0, // remote_ex: RemoteExceptionCode::Ok
            60, 0, 0, 0, // TypeLookupReturn DHEADER
            0x31, 0xfb, 0xaa,
            0x05, // TypeLookupGetDependenciesHash Discriminator (0x05aafb31)
            52, 0, 0, 0, // TypeLookupGetTypeDependenciesResult DHEADER
            0x00, 0x00, 0x00, 0x00, // Result Discriminator: 0 (Ok)
            44, 0, 0, 0, // TypeLookupGetTypeDependenciesOut DHEADER
            201, 223, 164, 91, // EMHEADER for dependent_typeids (LC=5 | 0x0ba4dfc9)
            28, 0, 0, 0, // dependent_typeids DHEADER: 28 bytes
            0x01, 0x00, 0x00, 0x00, // Sequence length: 1
            0x14, 0x00, 0x00, 0x00, // TypeIdentifierWithSize DHEADER: 20 bytes
            0xf2, 0x99, 0xbf, 0x61, 0x62, 0x13, 0xb8, 0xbb, 0x50, 0x0c, 0x96, 0x6b, 0x11, 0x2a,
            0x5c, 0x00, // type_id (EK_COMPLETE + hash + 1 byte pad)
            0x97, 0x00, 0x00, 0x00, // typeobject_serialized_size: 151
            210, 227, 8, 85, // EMHEADER for continuation_point (LC=5 | 0x0508e3d2)
            0x00, 0x00, 0x00, 0x00, // Sequence length: 0
        ];
        assert_eq!(serialized, expected);
    }

    #[test]
    fn serialize_type_identifier() {
        let data = TypeInformation {
            minimal: TypeIdentifierWithDependencies {
                typeid_with_size: TypeIdentifierWithSize {
                    type_id: TypeIdentifier::EkMinimal {
                        equivalence_hash: [5; 14],
                    },
                    typeobject_serialized_size: 10,
                },
                dependent_typeid_count: 0,
                dependent_typeids: Vec::new(),
            },
            complete: TypeIdentifierWithDependencies {
                typeid_with_size: TypeIdentifierWithSize {
                    type_id: TypeIdentifier::EkComplete {
                        equivalence_hash: [5; 14],
                    },
                    typeobject_serialized_size: 10,
                },
                dependent_typeid_count: 0,
                dependent_typeids: Vec::new(),
            },
        }
        .create_dynamic_sample();

        let buffer = serialize_without_header_cdr2_le(Vec::new(), &data).unwrap();
        assert_eq!(
            buffer,
            vec![
                // 0x00u8, 0x0B, 0x00, 0x00, // PL_CDR2_LE + 0 padding
                88, 0, 0, 0, // DHEADER
                0x01, 0x10, 0, 80, // EMHEADER
                // 40, 0, 0, 0, // NEXTINT (Reused in the EMHEADER)
                36, 0, 0, 0, // DHEADER
                20, 0, 0, 0,   // DHEADER
                241, //EK_MINIMAL
                5, 5, 5, 5, 5, 5, 5, 5, 5, 5, 5, 5, 5, 5, // equivalence_hash
                0, // padding
                10, 0, 0, 0, // typeobject_serialized_size
                0, 0, 0, 0, // dependent_typeid_count
                4, 0, 0, 0, // Sequence DHEADER
                0, 0, 0, 0, // Sequence length
                0x02, 0x10, 0, 80, // EMHEADER
                // 40, 0, 0, 0, // NEXTINT (Reused in the EMHEADER)
                36, 0, 0, 0, // DHEADER
                20, 0, 0, 0,   // DHEADER
                242, //EK_COMPLETE
                5, 5, 5, 5, 5, 5, 5, 5, 5, 5, 5, 5, 5, 5, // equivalence_hash
                0, // padding
                10, 0, 0, 0, // typeobject_serialized_size
                0, 0, 0, 0, // dependent_typeid_count
                4, 0, 0, 0, // Sequence DHEADER
                0, 0, 0, 0, // Sequence length
            ]
        );
    }

    #[cfg(feature = "xtypes-xml")]
    #[test]
    fn union_sequence() {
        use crate::xtypes::dynamic_type;

        let type_xml = r#"
        <dds>
            <types>
                <module name="Test">
                    <union name="union_seq_int32x20"> <discriminator type="uint32" />
                        <case><caseDiscriminator value="1" /><member name="x1"   type="int32" sequenceMaxLength="20" /></case>
                    </union>
                </module>
            </types>
        </dds>
        "#;
        let dynamic_type = dynamic_type::DynamicTypeBuilderFactory::create_type_w_document(
            type_xml,
            "Test::union_seq_int32x20",
            vec![],
        )
        .unwrap()
        .build();

        let mut data = dynamic_type::DynamicDataFactory::create_data(dynamic_type);
        data.from_xml(
            "<struct>
                <x1>
                    <item>1</item>
                    <item>2</item>
                    <item>3</item>
                    <item>4</item>
                    <item>5</item>
                    <item>6</item>
                    <item>7</item>
                    <item>8</item>
                    <item>9</item>
                    <item>10</item>
                    <item>11</item>
                    <item>12</item>
                    <item>13</item>
                    <item>14</item>
                    <item>15</item>
                    <item>16</item>
                    <item>17</item>
                    <item>18</item>
                    <item>19</item>
                    <item>20</item>
                </x1>
            </struct>",
        )
        .unwrap();
        assert_eq!(
            serialize_cdr2_be(&data).unwrap(),
            vec![
                0x00, 0x08, 0x00, 0x00, // CDR2_BE DELIMITED
                0, 0, 0, 88, // DHEADER
                0, 0, 0, 1, // discriminant (u32)
                0, 0, 0, 20, // Length
                0, 0, 0, 1, // i32
                0, 0, 0, 2, // i32
                0, 0, 0, 3, // i32
                0, 0, 0, 4, // i32
                0, 0, 0, 5, // i32
                0, 0, 0, 6, // i32
                0, 0, 0, 7, // i32
                0, 0, 0, 8, // i32
                0, 0, 0, 9, // i32
                0, 0, 0, 10, // i32
                0, 0, 0, 11, // i32
                0, 0, 0, 12, // i32
                0, 0, 0, 13, // i32
                0, 0, 0, 14, // i32
                0, 0, 0, 15, // i32
                0, 0, 0, 16, // i32
                0, 0, 0, 17, // i32
                0, 0, 0, 18, // i32
                0, 0, 0, 19, // i32
                0, 0, 0, 20, // i32
            ]
        );
    }

    #[cfg(feature = "xtypes-xml")]
    #[test]
    fn serialize_wstring() {
        use crate::xtypes::dynamic_type;

        let type_xml = r#"
        <dds>
            <types>
                <module name="Test">
                    <struct name="wstring_unbounded" extensibility="final">
                        <member name="x1" type="wstring" />
                    </struct>
                </module>
            </types>
        </dds>
        "#;
        let dynamic_type = dynamic_type::DynamicTypeBuilderFactory::create_type_w_document(
            type_xml,
            "Test::wstring_unbounded",
            vec![],
        )
        .unwrap()
        .build();

        let mut data = dynamic_type::DynamicDataFactory::create_data(dynamic_type);
        data.from_xml(
            "<wstring_unbounded>
                <x1>ҥėŀľｏ ｔһｅⲅȇ.</x1>
            </wstring_unbounded>",
        )
        .unwrap();
        assert_eq!(
            serialize_cdr2_le(&data).unwrap(),
            vec![
                0x00, 0x07, 0x00, 0x00, // CDR2_LE PLAIN
                24, 0, 0, 0, // length in octets (24 = 12 UTF-16 units * 2)
                0xA5, 0x04, // ҥ
                0x17, 0x01, // ė
                0x40, 0x01, // ŀ
                0x3E, 0x01, // ľ
                0x4F, 0xFF, // ｏ
                0x20, 0x00, // ' '
                0x54, 0xFF, // ｔ
                0xBB, 0x04, // һ
                0x45, 0xFF, // ｅ
                0x85, 0x2C, // ⲅ
                0x07, 0x02, // ȇ
                0x2E, 0x00, // .
            ]
        );
    }

    #[test]
    fn serialize_f_s_seq10_m_s_seq20_uint32() {
        let type_xml = r#"
            <dds>
                <types>
                    <module name="Test">
                        <struct name="M_S__seq20_uint32" extensibility="mutable">
                            <member name="x1" type="uint32" sequenceMaxLength="20" />
                        </struct>
                        <struct name="F_S__seq10_M_S__seq20_uint32" extensibility="final">
                            <member name="x1" type="nonBasic" nonBasicTypeName="M_S__seq20_uint32" sequenceMaxLength="10" />
                        </struct>
                    </module>
                </types>
            </dds>
        "#;
        let data_xml = r#"
            <struct>
              <x1>
                <item><x1><item>1</item><item>2</item><item>3</item><item>4</item><item>5</item><item>6</item><item>7</item><item>8</item><item>9</item><item>10</item><item>11</item><item>12</item><item>13</item><item>14</item><item>15</item><item>16</item><item>17</item><item>18</item><item>19</item><item>20</item></x1></item>
                <item><x1><item>1</item><item>2</item><item>3</item><item>4</item><item>5</item><item>6</item><item>7</item><item>8</item><item>9</item><item>10</item><item>11</item><item>12</item><item>13</item><item>14</item><item>15</item><item>16</item><item>17</item><item>18</item><item>19</item><item>20</item></x1></item>
                <item><x1><item>1</item><item>2</item><item>3</item><item>4</item><item>5</item><item>6</item><item>7</item><item>8</item><item>9</item><item>10</item><item>11</item><item>12</item><item>13</item><item>14</item><item>15</item><item>16</item><item>17</item><item>18</item><item>19</item><item>20</item></x1></item>
                <item><x1><item>1</item><item>2</item><item>3</item><item>4</item><item>5</item><item>6</item><item>7</item><item>8</item><item>9</item><item>10</item><item>11</item><item>12</item><item>13</item><item>14</item><item>15</item><item>16</item><item>17</item><item>18</item><item>19</item><item>20</item></x1></item>
                <item><x1><item>1</item><item>2</item><item>3</item><item>4</item><item>5</item><item>6</item><item>7</item><item>8</item><item>9</item><item>10</item><item>11</item><item>12</item><item>13</item><item>14</item><item>15</item><item>16</item><item>17</item><item>18</item><item>19</item><item>20</item></x1></item>
                <item><x1><item>1</item><item>2</item><item>3</item><item>4</item><item>5</item><item>6</item><item>7</item><item>8</item><item>9</item><item>10</item><item>11</item><item>12</item><item>13</item><item>14</item><item>15</item><item>16</item><item>17</item><item>18</item><item>19</item><item>20</item></x1></item>
                <item><x1><item>1</item><item>2</item><item>3</item><item>4</item><item>5</item><item>6</item><item>7</item><item>8</item><item>9</item><item>10</item><item>11</item><item>12</item><item>13</item><item>14</item><item>15</item><item>16</item><item>17</item><item>18</item><item>19</item><item>20</item></x1></item>
                <item><x1><item>1</item><item>2</item><item>3</item><item>4</item><item>5</item><item>6</item><item>7</item><item>8</item><item>9</item><item>10</item><item>11</item><item>12</item><item>13</item><item>14</item><item>15</item><item>16</item><item>17</item><item>18</item><item>19</item><item>20</item></x1></item>
                <item><x1><item>1</item><item>2</item><item>3</item><item>4</item><item>5</item><item>6</item><item>7</item><item>8</item><item>9</item><item>10</item><item>11</item><item>12</item><item>13</item><item>14</item><item>15</item><item>16</item><item>17</item><item>18</item><item>19</item><item>20</item></x1></item>
                <item><x1><item>1</item><item>2</item><item>3</item><item>4</item><item>5</item><item>6</item><item>7</item><item>8</item><item>9</item><item>10</item><item>11</item><item>12</item><item>13</item><item>14</item><item>15</item><item>16</item><item>17</item><item>18</item><item>19</item><item>20</item></x1></item>
              </x1>
            </struct>
        "#;
        let dynamic_type = DynamicTypeBuilderFactory::create_type_w_document(
            type_xml,
            "Test::F_S__seq10_M_S__seq20_uint32",
            vec![],
        )
        .unwrap()
        .build();
        let mut data = DynamicDataFactory::create_data(dynamic_type);
        data.from_xml(data_xml).unwrap();
        let serialized = serialize_cdr2_le(&data).unwrap();

        // Baseline Connext bytes
        let mut expected = vec![
            0x00u8, 0x07, 0x00, 0x00, // CDR2_LE header
            0x9c, 0x03, 0x00, 0x00, // Outer DHEADER = 924
            0x0a, 0x00, 0x00, 0x00, // Outer Sequence length = 10
        ];
        for _ in 0..10 {
            expected.extend_from_slice(&[
                0x58, 0x00, 0x00, 0x00, // Inner struct DHEADER = 88
                0x00, 0x00, 0x00, 0x60, // EMHEADER: LC=6, Member ID=0
                0x14, 0x00, 0x00, 0x00, // NEXTINT: 20
            ]);
            for i in 1..=20u32 {
                expected.extend_from_slice(&i.to_le_bytes());
            }
        }

        assert_eq!(serialized, expected);
    }
}
