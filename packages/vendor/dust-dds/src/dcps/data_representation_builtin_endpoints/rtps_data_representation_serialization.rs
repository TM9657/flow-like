use crate::{
    dcps::data_representation_builtin_endpoints::{
        parameter_id_values::ParameterId,
        spdp_discovered_participant_data::{BuiltinEndpointQos, BuiltinEndpointSet},
    },
    infrastructure::time::Duration,
    transport::types::{EntityId, Locator, Long, Octet, ProtocolVersion, UnsignedLong},
    xtypes::{
        serializer::{serialize_without_header_cdr1_le, serialize_without_header_cdr2_le},
        type_support::TypeSupport,
    },
};
use alloc::{string::String, vec::Vec};

pub(crate) struct ParameterListSerializer<'a> {
    data: &'a mut Vec<u8>,
}

impl<'a> ParameterListSerializer<'a> {
    pub(crate) fn new(data: &'a mut Vec<u8>) -> Self {
        Self { data }
    }
    pub(crate) fn write_header(&mut self) {
        let mut ser = CdrSerializer::new(self.data);
        [0, 3].cdr_serialize(&mut ser);
        [0, 0].cdr_serialize(&mut ser);
    }
    pub(crate) fn write_sentinel(self) {
        let mut ser = CdrSerializer::new(self.data);
        1i16.cdr_serialize(&mut ser);
        0u16.cdr_serialize(&mut ser);
    }

    pub(crate) fn write_cdr_parameter<T: CdrSerialize>(&mut self, pid: ParameterId, value: T) {
        const ZEROS: [u8; 4] = [0; 4];
        let mut ser = CdrSerializer::new(self.data);
        pid.cdr_serialize(&mut ser);
        0u16.cdr_serialize(&mut ser);
        let position = ser.buffer.len();
        value.cdr_serialize(&mut ser);
        let position_after_value = ser.buffer.len();
        let alignment = 4;
        let padding_length =
            position_after_value.div_ceil(alignment) * alignment - position_after_value;
        self.data.extend_from_slice(&ZEROS[..padding_length]);
        let length = (self.data.len() - position) as u16;
        self.data[position - 2..position].clone_from_slice(&length.to_le_bytes());
    }

    pub(crate) fn write_xcdr1_parameter<T: TypeSupport>(&mut self, pid: ParameterId, value: T) {
        let buffer = Vec::new();
        let data = value.create_dynamic_sample();
        let buffer = serialize_without_header_cdr1_le(buffer, &data).expect("Must succeed");
        self.write_cdr_parameter(pid, buffer.as_slice());
    }

    pub(crate) fn write_xcdr2_parameter<T: TypeSupport>(&mut self, pid: ParameterId, value: T) {
        let buffer = Vec::new();
        let data = value.create_dynamic_sample();
        let buffer = serialize_without_header_cdr2_le(buffer, &data).expect("Must succeed");
        self.write_cdr_parameter(pid, buffer.as_slice());
    }
}

pub(crate) struct CdrSerializer<'a> {
    buffer: &'a mut Vec<u8>,
}

impl<'a> CdrSerializer<'a> {
    fn new(buffer: &'a mut Vec<u8>) -> Self {
        Self { buffer }
    }

    fn pad(&mut self, alignment: usize) {
        const ZEROS: [u8; 4] = [0; 4];
        let position = self.buffer.len();
        let padding_length = position.div_ceil(alignment) * alignment - position;
        self.buffer.extend_from_slice(&ZEROS[..padding_length]);
    }
}

pub(crate) trait CdrSerialize {
    fn cdr_serialize(&self, ser: &mut CdrSerializer);
}

impl CdrSerialize for Octet {
    fn cdr_serialize(&self, ser: &mut CdrSerializer) {
        ser.buffer.push(*self);
    }
}
impl CdrSerialize for i16 {
    fn cdr_serialize(&self, ser: &mut CdrSerializer) {
        ser.pad(2);
        ser.buffer.extend_from_slice(&self.to_le_bytes());
    }
}
impl CdrSerialize for u16 {
    fn cdr_serialize(&self, ser: &mut CdrSerializer) {
        ser.pad(2);
        ser.buffer.extend_from_slice(&self.to_le_bytes());
    }
}
impl CdrSerialize for Long {
    fn cdr_serialize(&self, ser: &mut CdrSerializer) {
        ser.pad(4);
        ser.buffer.extend_from_slice(&self.to_le_bytes());
    }
}
impl CdrSerialize for UnsignedLong {
    fn cdr_serialize(&self, ser: &mut CdrSerializer) {
        ser.pad(4);
        ser.buffer.extend_from_slice(&self.to_le_bytes());
    }
}
impl CdrSerialize for bool {
    fn cdr_serialize(&self, ser: &mut CdrSerializer) {
        ser.buffer.push(if *self { 1 } else { 0 });
    }
}
impl<const N: usize> CdrSerialize for [u8; N] {
    fn cdr_serialize(&self, ser: &mut CdrSerializer) {
        ser.buffer.extend_from_slice(self);
    }
}
impl CdrSerialize for &[u8] {
    fn cdr_serialize(&self, ser: &mut CdrSerializer) {
        ser.buffer.extend_from_slice(self);
    }
}
impl CdrSerialize for String {
    fn cdr_serialize(&self, ser: &mut CdrSerializer) {
        let length = (self.len() + 1) as u32;
        length.cdr_serialize(ser);
        ser.buffer.extend_from_slice(self.as_bytes());
        ser.buffer.push(0);
    }
}
impl CdrSerialize for Locator {
    fn cdr_serialize(&self, ser: &mut CdrSerializer) {
        self.kind.cdr_serialize(ser);
        self.port.cdr_serialize(ser);
        self.address.cdr_serialize(ser);
    }
}
impl CdrSerialize for ProtocolVersion {
    fn cdr_serialize(&self, ser: &mut CdrSerializer) {
        self.bytes.cdr_serialize(ser);
    }
}
impl CdrSerialize for BuiltinEndpointQos {
    fn cdr_serialize(&self, ser: &mut CdrSerializer) {
        self.0.cdr_serialize(ser);
    }
}
impl CdrSerialize for BuiltinEndpointSet {
    fn cdr_serialize(&self, ser: &mut CdrSerializer) {
        self.0.cdr_serialize(ser);
    }
}
impl CdrSerialize for Duration {
    fn cdr_serialize(&self, ser: &mut CdrSerializer) {
        self.sec().cdr_serialize(ser);
        self.nanosec().cdr_serialize(ser);
    }
}
impl CdrSerialize for EntityId {
    fn cdr_serialize(&self, ser: &mut CdrSerializer) {
        self.entity_key.cdr_serialize(ser);
        self.entity_kind.cdr_serialize(ser);
    }
}
