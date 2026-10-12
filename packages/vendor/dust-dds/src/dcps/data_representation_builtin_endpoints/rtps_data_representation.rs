use crate::{
    dcps::data_representation_builtin_endpoints::{
        parameter_id_values::ParameterId,
        spdp_discovered_participant_data::{BuiltinEndpointQos, BuiltinEndpointSet},
    },
    infrastructure::time::Duration,
    transport::types::{EntityId, Locator, Long, Octet, ProtocolVersion, UnsignedLong},
    xtypes::{
        deserializer::deserialize_top_level_type_from_representation_identifier,
        error::XTypesError, type_support::TypeSupport,
    },
};
use alloc::{string::String, vec::Vec};

pub type CdrResult<T> = Result<T, CdrError>;

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

#[derive(Debug, PartialEq)]
pub enum CdrError {
    InvalidData,
    PidNotFound(i16),
    NotEnoughData,
    Unsupported(RepresentationIdentifier),
    XTypes(XTypesError),
}
impl From<XTypesError> for CdrError {
    fn from(value: XTypesError) -> Self {
        CdrError::XTypes(value)
    }
}

#[derive(Clone, Copy)]
pub(crate) enum Endianness {
    Big,
    Little,
}

pub(crate) struct ParameterList<'a> {
    data: &'a [u8],
}

pub(crate) struct PidIterator<'a> {
    data: &'a [u8],
    endianness: Endianness,
    position: usize,
}

impl<'a> PidIterator<'a> {
    fn new(data: &'a [u8], endianness: Endianness) -> Self {
        Self {
            data,
            endianness,
            position: 0,
        }
    }
}

impl<'a> Iterator for PidIterator<'a> {
    type Item = CdrResult<(ParameterId, &'a [u8])>;

    fn next(&mut self) -> Option<Self::Item> {
        if self.position >= self.data.len() {
            return None;
        }

        let mut de = CdrDeserializer::new(&self.data[self.position..], self.endianness);
        let current_pid = match i16::cdr_deserialize(&mut de) {
            Ok(pid) => pid,
            Err(e) => return Some(Err(e)),
        };
        let length = match u16::cdr_deserialize(&mut de) {
            Ok(len) => len as usize,
            Err(e) => return Some(Err(e)),
        };

        if current_pid == 1 || self.position + length + 4 > self.data.len() {
            return None;
        }

        let pid_data = &self.data[self.position + 4..self.position + length + 4];
        self.position += length + 4;

        Some(Ok((current_pid, pid_data)))
    }
}

impl<'a> ParameterList<'a> {
    pub(crate) fn new(data: &'a [u8]) -> CdrResult<Self> {
        if data.len() < 4 {
            Err(CdrError::NotEnoughData)
        } else {
            Ok(Self { data })
        }
    }

    pub(crate) fn get_optional_parameter<T: CdrDeserialize>(
        &self,
        pid: ParameterId,
        default: T,
    ) -> CdrResult<T> {
        if let Some(pid_data) = self.seek_to_pid(pid)? {
            CdrDeserialize::cdr_deserialize(&mut CdrDeserializer::new(pid_data, self.endianness()?))
        } else {
            Ok(default)
        }
    }

    pub(crate) fn get_non_optional_parameter<T: CdrDeserialize>(
        &self,
        pid: ParameterId,
    ) -> CdrResult<T> {
        CdrDeserialize::cdr_deserialize(&mut CdrDeserializer::new(
            self.seek_to_pid(pid)?.ok_or(CdrError::PidNotFound(pid))?,
            self.endianness()?,
        ))
    }

    pub(crate) fn get_optional_parameter_xdcr2<T: TypeSupport>(
        &self,
        pid: ParameterId,
    ) -> CdrResult<Option<T>> {
        if let Some(pid_data) = self.seek_to_pid(pid)? {
            let representation_identifier = match [self.data[0], self.data[1]] {
                CDR_BE | CDR2_BE | PL_CDR_BE | D_CDR2_BE | PL_CDR2_BE => CDR2_BE,
                CDR_LE | CDR2_LE | PL_CDR_LE | D_CDR2_LE | PL_CDR2_LE => CDR2_LE,
                unsupported => Err(CdrError::Unsupported(unsupported))?,
            };
            let mut dynamic_data = deserialize_top_level_type_from_representation_identifier(
                T::TYPE,
                representation_identifier,
                pid_data,
            )?;
            Ok(T::create_sample(&mut dynamic_data))
        } else {
            Ok(None)
        }
    }

    pub(crate) fn get_optional_parameter_xdcr<T: TypeSupport>(
        &self,
        pid: ParameterId,
        default: T,
    ) -> CdrResult<T> {
        if let Some(pid_data) = self.seek_to_pid(pid)? {
            let mut dynamic_data = deserialize_top_level_type_from_representation_identifier(
                T::TYPE,
                [self.data[0], self.data[1]],
                pid_data,
            )?;
            Ok(T::create_sample(&mut dynamic_data).unwrap_or(default))
        } else {
            Ok(default)
        }
    }

    pub(crate) fn get_non_optional_parameter_xdcr<T: TypeSupport>(
        &self,
        pid: ParameterId,
    ) -> CdrResult<T> {
        let pid_data = self.seek_to_pid(pid)?.ok_or(CdrError::PidNotFound(pid))?;
        let mut dynamic_data = deserialize_top_level_type_from_representation_identifier(
            T::TYPE,
            [self.data[0], self.data[1]],
            pid_data,
        )?;
        T::create_sample(&mut dynamic_data).ok_or(CdrError::InvalidData)
    }

    pub(crate) fn get_locator_list(&self, pid: ParameterId) -> CdrResult<Vec<Locator>> {
        let mut locator_list = Vec::new();
        let iterator = PidIterator::new(self.data, self.endianness()?);
        for item in iterator {
            let (current_pid, pid_data) = item?;
            if current_pid == pid {
                locator_list.push(CdrDeserialize::cdr_deserialize(&mut CdrDeserializer::new(
                    pid_data,
                    self.endianness()?,
                ))?);
            }
        }
        Ok(locator_list)
    }

    fn seek_to_pid(&self, pid: ParameterId) -> CdrResult<Option<&'a [u8]>> {
        let iterator = PidIterator::new(self.data, self.endianness()?);
        for item in iterator {
            let (current_pid, pid_data) = item?;
            if current_pid == pid {
                return Ok(Some(pid_data));
            }
        }
        Ok(None)
    }

    fn endianness(&self) -> CdrResult<Endianness> {
        match self.data[1] {
            2 => Ok(Endianness::Big),
            3 => Ok(Endianness::Little),
            _ => Err(CdrError::InvalidData),
        }
    }
}

pub(crate) trait CdrDeserialize: Sized {
    fn cdr_deserialize<'a>(de: &mut CdrDeserializer<'a>) -> CdrResult<Self>;
}

impl CdrDeserialize for Octet {
    fn cdr_deserialize<'a>(de: &mut CdrDeserializer<'a>) -> CdrResult<Self> {
        if de.pos + 1 > de.buffer.len() {
            return Err(CdrError::NotEnoughData);
        }
        let ret = de.buffer[de.pos];
        de.pos += 1;
        Ok(ret)
    }
}
impl CdrDeserialize for i16 {
    fn cdr_deserialize<'a>(de: &mut CdrDeserializer<'a>) -> CdrResult<Self> {
        de.seek_padding(2)?;
        let bytes = CdrDeserialize::cdr_deserialize(de)?;
        Ok(match de.endianness {
            Endianness::Big => i16::from_be_bytes(bytes),
            Endianness::Little => i16::from_le_bytes(bytes),
        })
    }
}
impl CdrDeserialize for u16 {
    fn cdr_deserialize<'a>(de: &mut CdrDeserializer<'a>) -> CdrResult<Self> {
        de.seek_padding(2)?;
        let bytes = CdrDeserialize::cdr_deserialize(de)?;
        Ok(match de.endianness {
            Endianness::Big => u16::from_be_bytes(bytes),
            Endianness::Little => u16::from_le_bytes(bytes),
        })
    }
}
impl CdrDeserialize for Long {
    fn cdr_deserialize<'a>(de: &mut CdrDeserializer<'a>) -> CdrResult<Self> {
        de.seek_padding(4)?;
        let bytes = CdrDeserialize::cdr_deserialize(de)?;
        Ok(match de.endianness {
            Endianness::Big => i32::from_be_bytes(bytes),
            Endianness::Little => i32::from_le_bytes(bytes),
        })
    }
}
impl CdrDeserialize for UnsignedLong {
    fn cdr_deserialize<'a>(de: &mut CdrDeserializer<'a>) -> CdrResult<Self> {
        de.seek_padding(4)?;
        let bytes = CdrDeserialize::cdr_deserialize(de)?;
        Ok(match de.endianness {
            Endianness::Big => u32::from_be_bytes(bytes),
            Endianness::Little => u32::from_le_bytes(bytes),
        })
    }
}
impl CdrDeserialize for bool {
    fn cdr_deserialize<'a>(de: &mut CdrDeserializer<'a>) -> CdrResult<Self> {
        Ok(Octet::cdr_deserialize(de)? != 0)
    }
}
impl<const N: usize> CdrDeserialize for [u8; N] {
    fn cdr_deserialize<'a>(de: &mut CdrDeserializer<'a>) -> CdrResult<Self> {
        let mut array = [0u8; N];
        array.copy_from_slice(de.read_bytes(N)?);
        Ok(array)
    }
}
impl CdrDeserialize for String {
    fn cdr_deserialize<'a>(de: &mut CdrDeserializer<'a>) -> CdrResult<Self> {
        let length = UnsignedLong::cdr_deserialize(de)?;
        let character_data = de.read_bytes(length as usize - 1)?.to_vec();
        Octet::cdr_deserialize(de)?; // 0-termination
        String::from_utf8(character_data).map_err(|_| CdrError::InvalidData)
    }
}
impl CdrDeserialize for Locator {
    fn cdr_deserialize<'a>(de: &mut CdrDeserializer<'a>) -> CdrResult<Self> {
        Ok(Locator {
            kind: CdrDeserialize::cdr_deserialize(de)?,
            port: CdrDeserialize::cdr_deserialize(de)?,
            address: CdrDeserialize::cdr_deserialize(de)?,
        })
    }
}
impl CdrDeserialize for ProtocolVersion {
    fn cdr_deserialize<'a>(de: &mut CdrDeserializer<'a>) -> CdrResult<Self> {
        Ok(ProtocolVersion {
            bytes: CdrDeserialize::cdr_deserialize(de)?,
        })
    }
}
impl CdrDeserialize for BuiltinEndpointQos {
    fn cdr_deserialize<'a>(de: &mut CdrDeserializer<'a>) -> CdrResult<Self> {
        Ok(Self(CdrDeserialize::cdr_deserialize(de)?))
    }
}
impl CdrDeserialize for BuiltinEndpointSet {
    fn cdr_deserialize<'a>(de: &mut CdrDeserializer<'a>) -> CdrResult<Self> {
        Ok(Self(CdrDeserialize::cdr_deserialize(de)?))
    }
}
impl CdrDeserialize for Duration {
    fn cdr_deserialize<'a>(de: &mut CdrDeserializer<'a>) -> CdrResult<Self> {
        Ok(Duration::new(
            CdrDeserialize::cdr_deserialize(de)?,
            CdrDeserialize::cdr_deserialize(de)?,
        ))
    }
}
impl CdrDeserialize for EntityId {
    fn cdr_deserialize<'a>(de: &mut CdrDeserializer<'a>) -> CdrResult<Self> {
        Ok(EntityId {
            entity_key: CdrDeserialize::cdr_deserialize(de)?,
            entity_kind: CdrDeserialize::cdr_deserialize(de)?,
        })
    }
}

pub(crate) struct CdrDeserializer<'a> {
    buffer: &'a [u8],
    pos: usize,
    endianness: Endianness,
}

impl<'a> CdrDeserializer<'a> {
    pub fn new(buffer: &'a [u8], endianness: Endianness) -> Self {
        Self {
            buffer,
            pos: 0,
            endianness,
        }
    }

    fn read_bytes(&mut self, length: usize) -> CdrResult<&'a [u8]> {
        if self.pos + length > self.buffer.len() {
            return Err(CdrError::NotEnoughData);
        }
        let ret = &self.buffer[self.pos..self.pos + length];
        self.pos += length;
        Ok(ret)
    }

    fn seek_padding(&mut self, alignment: usize) -> CdrResult<()> {
        let mask = alignment - 1;
        let seek = ((self.pos + mask) & !mask) - self.pos;
        if self.pos + seek > self.buffer.len() {
            return Err(CdrError::NotEnoughData);
        }
        self.pos += seek;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(Debug, PartialEq)]
    struct TestDiscoveryData {
        domain_id: i32,
        domain_tag: String,
        vendor_id: [u8; 2],
        locator_list: Vec<Locator>,
    }

    fn from_bytes(bytes: &[u8]) -> CdrResult<TestDiscoveryData> {
        let pl = ParameterList::new(bytes).unwrap();

        Ok(TestDiscoveryData {
            domain_id: pl.get_optional_parameter(15, 0).unwrap(),
            domain_tag: pl.get_optional_parameter(0x4014, String::from("")).unwrap(),
            vendor_id: pl.get_non_optional_parameter(0x16).unwrap(),
            locator_list: pl.get_locator_list(72).unwrap(),
        })
    }

    #[test]
    fn derserialize_test_discovery_data() {
        let bytes = [
            0x00, 0x03, 0x00, 0x00, // PL_CDR_LE
            15, 0x00, 0x04, 0x00, // PID_DOMAIN_ID, Length: 4
            0x01, 0x00, 0x00, 0x00, // DomainId
            0x14, 0x40, 0x08, 0x00, // PID_DOMAIN_TAG, Length: 8
            3, 0x00, 0x00, 0x00, // DomainTag: string length (incl. terminator)
            b'a', b'b', 0, 0x00, // DomainTag: string + padding (1 byte)
            0x16, 0x00, 4, 0x00, // PID_VENDORID
            73, 74, 0x00, 0x00, // VendorId
            72, 0x00, 24, 0x00, // PID_DEFAULT_MULTICAST_LOCATOR
            11, 0x00, 0x00, 0x00, // Locator{kind
            12, 0x00, 0x00, 0x00, // port,
            0x01, 0x01, 0x01, 0x01, //
            0x01, 0x01, 0x01, 0x01, // address
            0x01, 0x01, 0x01, 0x01, //
            0x01, 0x01, 0x01, 0x01, // }
            72, 0x00, 24, 0x00, // PID_DEFAULT_MULTICAST_LOCATOR
            21, 0x00, 0x00, 0x00, // Locator{kind
            22, 0x00, 0x00, 0x00, // port,
            0x02, 0x02, 0x02, 0x02, //
            0x02, 0x02, 0x02, 0x02, // address
            0x02, 0x02, 0x02, 0x02, //
            0x02, 0x02, 0x02, 0x02, // }
            0x01, 0x00, 0x00, 0x00, // PID_SENTINEL
        ];
        let expected = TestDiscoveryData {
            domain_id: 1,
            domain_tag: String::from("ab"),
            vendor_id: [73, 74],
            locator_list: vec![Locator::new(11, 12, [1; 16]), Locator::new(21, 22, [2; 16])],
        };
        assert_eq!(from_bytes(&bytes).unwrap(), expected);
    }

    #[test]
    fn derserialize_test_discovery_data_with_default() {
        let bytes = [
            0x00, 0x03, 0x00, 0x00, // PL_CDR_LE
            0x16, 0x00, 4, 0x00, // PID_VENDORID
            73, 74, 0x00, 0x00, // VendorId
            0x01, 0x00, 0x00, 0x00, // PID_SENTINEL
        ];
        let expected = TestDiscoveryData {
            domain_id: 0,
            domain_tag: String::from(""),
            vendor_id: [73, 74],
            locator_list: vec![],
        };
        assert_eq!(from_bytes(&bytes).unwrap(), expected);
    }
}
