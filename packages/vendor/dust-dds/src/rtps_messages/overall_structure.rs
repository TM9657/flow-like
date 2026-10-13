use crate::transport::types::{
    EntityId, GuidPrefix, Locator, Long, Octet, ProtocolVersion, SequenceNumber, UnsignedLong,
    VendorId,
};

use super::{
    error::{RtpsMessageError, RtpsMessageResult},
    submessages::{
        ack_nack::AckNackSubmessage, data::DataSubmessage, data_frag::DataFragSubmessage,
        gap::GapSubmessage, heartbeat::HeartbeatSubmessage,
        heartbeat_frag::HeartbeatFragSubmessage, info_destination::InfoDestinationSubmessage,
        info_reply::InfoReplySubmessage, info_source::InfoSourceSubmessage,
        info_timestamp::InfoTimestampSubmessage, nack_frag::NackFragSubmessage, pad::PadSubmessage,
    },
    types::{
        ACKNACK, DATA, DATA_FRAG, GAP, HEARTBEAT, HEARTBEAT_FRAG, INFO_DST, INFO_REPLY, INFO_SRC,
        INFO_TS, NACK_FRAG, PAD, ProtocolId, SubmessageFlag, SubmessageKind,
    },
};
use alloc::vec::Vec;

pub enum Endianness {
    BigEndian,
    LittleEndian,
}

impl Endianness {
    pub fn from_flags(byte: u8) -> Self {
        match byte & 0b_0000_0001 != 0 {
            true => Endianness::LittleEndian,
            false => Endianness::BigEndian,
        }
    }
}

pub trait TryReadFromBytes: Sized {
    fn try_read_from_bytes(data: &mut &[u8], endianness: &Endianness) -> RtpsMessageResult<Self>;
}

pub trait Read {
    fn read_exact(&mut self, buf: &mut [u8]) -> RtpsMessageResult<()>;
}

impl Read for &[u8] {
    fn read_exact(&mut self, buf: &mut [u8]) -> RtpsMessageResult<()> {
        if buf.len() > self.len() {
            *self = &self[self.len()..];

            return Err(RtpsMessageError::Io);
        }

        let (a, b) = self.split_at(buf.len());

        if buf.len() == 1 {
            buf[0] = a[0];
        } else {
            buf.copy_from_slice(a);
        }

        *self = b;

        Ok(())
    }
}

pub trait BufRead {
    fn consume(&mut self, amt: usize);
}

impl BufRead for &[u8] {
    fn consume(&mut self, amt: usize) {
        *self = &self[amt..];
    }
}

pub trait Write {
    fn write_all(&mut self, buf: &[u8]) -> RtpsMessageResult<()>;
}

#[derive(Debug, Default, Eq, PartialEq)]
pub struct Cursor<T> {
    inner: T,
    pos: u64,
}

impl<T> Cursor<T> {
    pub fn new(inner: T) -> Self {
        Self { inner, pos: 0 }
    }

    pub const fn position(&self) -> u64 {
        self.pos
    }

    pub const fn set_position(&mut self, pos: u64) {
        self.pos = pos;
    }

    pub fn into_inner(self) -> T {
        self.inner
    }
}

impl Write for Vec<u8> {
    fn write_all(&mut self, buf: &[u8]) -> RtpsMessageResult<()> {
        self.extend_from_slice(buf);
        Ok(())
    }
}

impl Write for Cursor<Vec<u8>> {
    #[inline]
    fn write_all(&mut self, buf: &[u8]) -> RtpsMessageResult<()> {
        let start_pos = self.pos as usize;
        let end_pos = start_pos + buf.len();
        if start_pos == self.inner.len() {
            self.inner.extend_from_slice(buf);
        } else if start_pos > self.inner.len() {
            self.inner.resize(start_pos, 0);
            self.inner.extend_from_slice(buf);
        } else if end_pos <= self.inner.len() {
            self.inner[start_pos..end_pos].copy_from_slice(buf);
        } else {
            let overwrite_len = self.inner.len() - start_pos;
            self.inner[start_pos..].copy_from_slice(&buf[..overwrite_len]);
            self.inner.extend_from_slice(&buf[overwrite_len..]);
        }
        self.pos = end_pos as u64;

        Ok(())
    }
}

pub trait WriteIntoBytes {
    fn write_into_bytes(&self, buf: &mut dyn Write);
}

impl WriteIntoBytes for Octet {
    #[inline]
    fn write_into_bytes(&self, buf: &mut dyn Write) {
        buf.write_all(&[*self]).expect("buffer big enough");
    }
}

impl WriteIntoBytes for Long {
    #[inline]
    fn write_into_bytes(&self, buf: &mut dyn Write) {
        buf.write_all(self.to_le_bytes().as_slice())
            .expect("buffer big enough");
    }
}

impl WriteIntoBytes for UnsignedLong {
    #[inline]
    fn write_into_bytes(&self, buf: &mut dyn Write) {
        buf.write_all(self.to_le_bytes().as_slice())
            .expect("buffer big enough");
    }
}

impl WriteIntoBytes for u16 {
    #[inline]
    fn write_into_bytes(&self, buf: &mut dyn Write) {
        buf.write_all(self.to_le_bytes().as_slice())
            .expect("buffer big enough");
    }
}

impl WriteIntoBytes for i16 {
    #[inline]
    fn write_into_bytes(&self, buf: &mut dyn Write) {
        buf.write_all(self.to_le_bytes().as_slice())
            .expect("buffer big enough");
    }
}

impl<const N: usize> WriteIntoBytes for [Octet; N] {
    #[inline]
    fn write_into_bytes(&self, buf: &mut dyn Write) {
        buf.write_all(self).expect("buffer big enough");
    }
}

impl WriteIntoBytes for &[u8] {
    #[inline]
    fn write_into_bytes(&self, buf: &mut dyn Write) {
        buf.write_all(self).expect("buffer big enough");
    }
}

impl TryReadFromBytes for GuidPrefix {
    fn try_read_from_bytes(data: &mut &[u8], _endianness: &Endianness) -> RtpsMessageResult<Self> {
        let mut guid_prefix = [0; 12];
        data.read_exact(&mut guid_prefix)?;
        Ok(guid_prefix)
    }
}

impl TryReadFromBytes for EntityId {
    fn try_read_from_bytes(data: &mut &[u8], _endianness: &Endianness) -> RtpsMessageResult<Self> {
        let mut entity_key = [0; 3];
        let mut entity_kind = [0; 1];
        data.read_exact(&mut entity_key)?;
        data.read_exact(&mut entity_kind)?;
        Ok(EntityId::new(entity_key, entity_kind[0]))
    }
}

impl WriteIntoBytes for EntityId {
    fn write_into_bytes(&self, buf: &mut dyn Write) {
        self.entity_key().write_into_bytes(buf);
        self.entity_kind().write_into_bytes(buf);
    }
}

impl TryReadFromBytes for SequenceNumber {
    fn try_read_from_bytes(data: &mut &[u8], endianness: &Endianness) -> RtpsMessageResult<Self> {
        let high = i32::try_read_from_bytes(data, endianness)?;
        let low = u32::try_read_from_bytes(data, endianness)?;
        let value = ((high as i64) << 32) + low as i64;
        Ok(value)
    }
}

impl WriteIntoBytes for SequenceNumber {
    fn write_into_bytes(&self, buf: &mut dyn Write) {
        let high = (*self >> 32) as Long;
        let low = *self as UnsignedLong;
        high.write_into_bytes(buf);
        low.write_into_bytes(buf);
    }
}

impl WriteIntoBytes for Locator {
    fn write_into_bytes(&self, buf: &mut dyn Write) {
        self.kind().write_into_bytes(buf);
        self.port().write_into_bytes(buf);
        self.address().write_into_bytes(buf);
    }
}

impl TryReadFromBytes for Locator {
    fn try_read_from_bytes(data: &mut &[u8], endianness: &Endianness) -> RtpsMessageResult<Self> {
        let kind = i32::try_read_from_bytes(data, endianness)?;
        let port = u32::try_read_from_bytes(data, endianness)?;
        let mut address = [0; 16];
        data.read_exact(&mut address)?;
        Ok(Self::new(kind, port, address))
    }
}

impl TryReadFromBytes for ProtocolVersion {
    fn try_read_from_bytes(data: &mut &[u8], _endianness: &Endianness) -> RtpsMessageResult<Self> {
        let mut bytes = [0; 2];
        data.read_exact(&mut bytes)?;
        Ok(Self::new(bytes[0], bytes[1]))
    }
}

impl WriteIntoBytes for ProtocolVersion {
    fn write_into_bytes(&self, buf: &mut dyn Write) {
        self._major().write_into_bytes(buf);
        self._minor().write_into_bytes(buf);
    }
}

impl TryReadFromBytes for VendorId {
    fn try_read_from_bytes(data: &mut &[u8], _endianness: &Endianness) -> RtpsMessageResult<Self> {
        let mut bytes = [0; 2];
        data.read_exact(&mut bytes)?;
        Ok(bytes)
    }
}

pub trait Submessage {
    fn write_submessage_header_into_bytes(&self, octets_to_next_header: u16, buf: &mut dyn Write);
    fn write_submessage_elements_into_bytes(&self, buf: &mut dyn Write);
}

impl dyn Submessage + Send + '_ {
    fn write_submessage_into_bytes<T>(&self, buf: &mut Cursor<T>)
    where
        Cursor<T>: Write,
    {
        let header_position: u64 = buf.position();
        let elements_position = header_position + 4;
        buf.set_position(elements_position);
        self.write_submessage_elements_into_bytes(buf);
        let pos = buf.position();
        buf.set_position(header_position);
        let len = pos - elements_position;
        self.write_submessage_header_into_bytes(len as u16, buf);
        buf.set_position(pos);
    }
}

pub struct SubmessageHeaderRead {
    submessage_id: u8,
    flags: [SubmessageFlag; 8],
    submessage_length: u16,
    endianness: Endianness,
}

impl SubmessageHeaderRead {
    pub fn try_read_from_bytes(data: &mut &[u8]) -> RtpsMessageResult<Self> {
        if data.len() >= 4 {
            let submessage_id = data[0];
            let flags_byte = data[1];
            let flags = [
                flags_byte & 0b_0000_0001 != 0,
                flags_byte & 0b_0000_0010 != 0,
                flags_byte & 0b_0000_0100 != 0,
                flags_byte & 0b_0000_1000 != 0,
                flags_byte & 0b_0001_0000 != 0,
                flags_byte & 0b_0010_0000 != 0,
                flags_byte & 0b_0100_0000 != 0,
                flags_byte & 0b_1000_0000 != 0,
            ];
            let endianness = match flags[0] {
                true => Endianness::LittleEndian,
                false => Endianness::BigEndian,
            };
            let submessage_length = u16::try_read_from_bytes(&mut &data[2..], &endianness)?;
            data.consume(4);
            Ok(Self {
                submessage_id,
                flags,
                submessage_length,
                endianness,
            })
        } else {
            Err(RtpsMessageError::NotEnoughData)
        }
    }

    pub fn endianness(&self) -> &Endianness {
        &self.endianness
    }

    pub fn flags(&self) -> [bool; 8] {
        self.flags
    }

    pub fn submessage_length(&self) -> u16 {
        self.submessage_length
    }

    pub fn submessage_id(&self) -> u8 {
        self.submessage_id
    }
}

#[derive(Debug, PartialEq, Eq)]
pub struct RtpsMessageRead {
    header: RtpsMessageHeader,
    submessages: Vec<RtpsSubmessageReadKind>,
}

impl RtpsMessageRead {
    pub fn header(&self) -> RtpsMessageHeader {
        self.header
    }

    pub fn submessages(&self) -> &[RtpsSubmessageReadKind] {
        &self.submessages
    }
}

impl TryFrom<&[u8]> for RtpsMessageRead {
    type Error = RtpsMessageError;

    fn try_from(mut v: &[u8]) -> RtpsMessageResult<Self> {
        if v.len() >= 20 {
            if b"RTPS" == &[v[0], v[1], v[2], v[3]] {
                let major = v[4];
                let minor = v[5];
                let version = ProtocolVersion::new(major, minor);
                let vendor_id = [v[6], v[7]];
                let guid_prefix = [
                    v[8], v[9], v[10], v[11], v[12], v[13], v[14], v[15], v[16], v[17], v[18],
                    v[19],
                ];
                let header = RtpsMessageHeader {
                    version,
                    vendor_id,
                    guid_prefix,
                };
                v.consume(20);

                const MAX_SUBMESSAGES: usize = 2_usize.pow(16);
                let mut submessages = Vec::new();
                for _ in 0..MAX_SUBMESSAGES {
                    if v.len() < 4 {
                        break;
                    }
                    if let Ok(submessage_header) = SubmessageHeaderRead::try_read_from_bytes(&mut v)
                    {
                        let mut submessage_length = submessage_header.submessage_length() as usize;
                        if v.len() < submessage_length {
                            break;
                        }
                        let submessage = match submessage_header.submessage_id() {
                            ACKNACK => AckNackSubmessage::try_from_bytes(&submessage_header, v)
                                .map(RtpsSubmessageReadKind::AckNack),
                            DATA => DataSubmessage::try_from_bytes(&submessage_header, v)
                                .map(RtpsSubmessageReadKind::Data),
                            DATA_FRAG => DataFragSubmessage::try_from_bytes(&submessage_header, v)
                                .map(RtpsSubmessageReadKind::DataFrag),
                            GAP => GapSubmessage::try_from_bytes(&submessage_header, v)
                                .map(RtpsSubmessageReadKind::Gap),
                            HEARTBEAT => HeartbeatSubmessage::try_from_bytes(&submessage_header, v)
                                .map(RtpsSubmessageReadKind::Heartbeat),
                            HEARTBEAT_FRAG => {
                                HeartbeatFragSubmessage::try_from_bytes(&submessage_header, v)
                                    .map(RtpsSubmessageReadKind::HeartbeatFrag)
                            }
                            INFO_DST => {
                                InfoDestinationSubmessage::try_from_bytes(&submessage_header, v)
                                    .map(RtpsSubmessageReadKind::InfoDestination)
                            }
                            INFO_REPLY => {
                                InfoReplySubmessage::try_from_bytes(&submessage_header, v)
                                    .map(RtpsSubmessageReadKind::InfoReply)
                            }
                            INFO_SRC => InfoSourceSubmessage::try_from_bytes(&submessage_header, v)
                                .map(RtpsSubmessageReadKind::InfoSource),
                            INFO_TS => {
                                InfoTimestampSubmessage::try_from_bytes(&submessage_header, v)
                                    .map(RtpsSubmessageReadKind::InfoTimestamp)
                            }
                            NACK_FRAG => NackFragSubmessage::try_from_bytes(&submessage_header, v)
                                .map(RtpsSubmessageReadKind::NackFrag),
                            PAD => PadSubmessage::try_from_bytes(&submessage_header, v)
                                .map(RtpsSubmessageReadKind::Pad),
                            _ => Err(RtpsMessageError::UnknownMessage),
                        };
                        if let Ok(submessage) = submessage {
                            // DATA and DATA_FRAG submessages can have a length of 0 meaning use everything until the end
                            // of the buffer
                            if submessage_length == 0
                                && (matches!(
                                    submessage,
                                    RtpsSubmessageReadKind::Data(_)
                                        | RtpsSubmessageReadKind::DataFrag(_)
                                ))
                            {
                                submessage_length = v.len();
                            }
                            submessages.push(submessage);
                        }

                        v.consume(submessage_length);
                    }
                }
                Ok(Self {
                    header,
                    submessages,
                })
            } else {
                Err(RtpsMessageError::InvalidData)
            }
        } else {
            Err(RtpsMessageError::NotEnoughData)
        }
    }
}

#[allow(dead_code)] // Only used as convenience in tests
pub fn write_into_bytes_vec(value: impl WriteIntoBytes) -> Vec<u8> {
    let mut cursor = Cursor::new(Vec::new());
    value.write_into_bytes(&mut cursor);
    cursor.into_inner()
}

#[allow(dead_code)] // Only used as convenience in tests
pub fn write_submessage_into_bytes_vec(value: &(dyn Submessage + Send)) -> Vec<u8> {
    let mut cursor = Cursor::new(Vec::new());
    value.write_submessage_into_bytes(&mut cursor);
    cursor.into_inner()
}

impl Write for Cursor<&mut [u8]> {
    #[inline]
    fn write_all(&mut self, buf: &[u8]) -> RtpsMessageResult<()> {
        let start_pos = self.pos as usize;
        let end_pos = start_pos + buf.len();
        if end_pos > self.inner.len() {
            return Err(RtpsMessageError::NotEnoughData);
        }
        self.inner[start_pos..end_pos].copy_from_slice(buf);
        self.pos = end_pos as u64;
        Ok(())
    }
}

#[derive(Debug, PartialEq, Eq)]
pub struct RtpsMessageWrite<'a> {
    buffer: &'a [u8],
}

impl<'a> RtpsMessageWrite<'a> {
    pub fn new(
        buffer: &'a mut [u8],
        header: &RtpsMessageHeader,
        submessages: &[&(dyn Submessage + Send)],
    ) -> Self {
        let mut cursor = Cursor::new(buffer);
        header.write_into_bytes(&mut cursor);
        for submessage in submessages {
            submessage.write_submessage_into_bytes(&mut cursor);
        }
        let len = cursor.position() as usize;
        Self {
            buffer: &cursor.into_inner()[..len],
        }
    }

    pub fn buffer(&self) -> &[u8] {
        self.buffer
    }
}

#[derive(Debug, PartialEq, Eq)]
pub enum RtpsSubmessageReadKind {
    AckNack(AckNackSubmessage),
    Data(DataSubmessage),
    DataFrag(DataFragSubmessage),
    Gap(GapSubmessage),
    Heartbeat(HeartbeatSubmessage),
    HeartbeatFrag(HeartbeatFragSubmessage),
    InfoDestination(InfoDestinationSubmessage),
    InfoReply(InfoReplySubmessage),
    InfoSource(InfoSourceSubmessage),
    InfoTimestamp(InfoTimestampSubmessage),
    NackFrag(NackFragSubmessage),
    Pad(PadSubmessage),
}
#[derive(Clone, Debug, PartialEq, Eq, Copy)]
pub struct RtpsMessageHeader {
    version: ProtocolVersion,
    vendor_id: VendorId,
    guid_prefix: GuidPrefix,
}

impl RtpsMessageHeader {
    pub fn new(version: ProtocolVersion, vendor_id: VendorId, guid_prefix: GuidPrefix) -> Self {
        Self {
            version,
            vendor_id,
            guid_prefix,
        }
    }

    pub fn version(&self) -> ProtocolVersion {
        self.version
    }

    pub fn vendor_id(&self) -> VendorId {
        self.vendor_id
    }

    pub fn guid_prefix(&self) -> GuidPrefix {
        self.guid_prefix
    }
}

impl WriteIntoBytes for RtpsMessageHeader {
    fn write_into_bytes(&self, buf: &mut dyn Write) {
        ProtocolId::PROTOCOL_RTPS.write_into_bytes(buf);
        self.version.write_into_bytes(buf);
        self.vendor_id.write_into_bytes(buf);
        self.guid_prefix.write_into_bytes(buf);
    }
}

#[derive(Debug, PartialEq, Eq)]
pub struct SubmessageHeaderWrite {
    submessage_id: SubmessageKind,
    flags_octet: u8,
    submessage_length: u16,
}

impl SubmessageHeaderWrite {
    pub fn new(
        submessage_id: SubmessageKind,
        // flags without endianness
        flags: &[SubmessageFlag],
        submessage_length: u16,
    ) -> Self {
        let mut flags_octet = 0b_0000_0001_u8;
        for (i, &item) in flags.iter().enumerate() {
            if item {
                flags_octet |= 0b_0000_0010 << i
            }
        }

        Self {
            submessage_id,
            flags_octet,
            submessage_length,
        }
    }
}

impl WriteIntoBytes for SubmessageHeaderWrite {
    fn write_into_bytes(&self, buf: &mut dyn Write) {
        self.submessage_id.write_into_bytes(buf);
        self.flags_octet.write_into_bytes(buf);
        self.submessage_length.write_into_bytes(buf);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        rtps_messages::{
            submessage_elements::{Data, Parameter, ParameterList},
            submessages::{data::DataSubmessage, info_timestamp::InfoTimestampSubmessage},
            types::Time,
        },
        transport::types::{EntityId, USER_DEFINED_READER_GROUP, USER_DEFINED_READER_NO_KEY},
    };

    #[test]
    fn deserialize_u16() {
        let mut data = &[7, 0, 123][..];
        let result = u16::try_read_from_bytes(&mut data, &Endianness::LittleEndian).unwrap();
        assert_eq!(result, 7);
        assert_eq!(data, &[123]);
    }

    #[test]
    fn serialize_sequence_number() {
        let data: SequenceNumber = 7;
        let result = write_into_bytes_vec(data);
        assert_eq!(
            result,
            vec![
                0, 0, 0, 0, // high (long)
                7, 0, 0, 0, // low (unsigned long)
            ]
        );
    }

    #[test]
    fn serialize_entity_id() {
        let data = EntityId::new([1, 2, 3], 0x04);
        assert_eq!(
            write_into_bytes_vec(data),
            vec![
                1, 2, 3, 0x04, //value (long)
            ]
        );
    }

    #[test]
    fn serialize_rtps_message_no_submessage() {
        let header = RtpsMessageHeader {
            version: ProtocolVersion::new(2, 3),
            vendor_id: [9, 8],
            guid_prefix: [3; 12],
        };
        let mut buffer = [0u8; 512];
        let message = RtpsMessageWrite::new(&mut buffer, &header, &[]);
        #[rustfmt::skip]
        assert_eq!(message.buffer(), vec![
            b'R', b'T', b'P', b'S', // Protocol
            2, 3, 9, 8, // ProtocolVersion | VendorId
            3, 3, 3, 3, // GuidPrefix
            3, 3, 3, 3, // GuidPrefix
            3, 3, 3, 3, // GuidPrefix
        ]);
    }

    #[test]
    fn serialize_rtps_message() {
        let header = RtpsMessageHeader {
            version: ProtocolVersion::new(2, 3),
            vendor_id: [9, 8],
            guid_prefix: [3; 12],
        };
        let inline_qos_flag = true;
        let data_flag = false;
        let key_flag = false;
        let non_standard_payload_flag = false;
        let reader_id = EntityId::new([1, 2, 3], USER_DEFINED_READER_NO_KEY);
        let writer_id = EntityId::new([6, 7, 8], USER_DEFINED_READER_GROUP);
        let writer_sn = 5;
        let parameter_1 = Parameter::new(6, vec![10, 11, 12, 13].into());
        let parameter_2 = Parameter::new(7, vec![20, 21, 22, 23].into());
        let inline_qos = ParameterList::new(vec![parameter_1, parameter_2]);
        let serialized_payload = Data::new(vec![].into());

        let submessage = DataSubmessage::new(
            inline_qos_flag,
            data_flag,
            key_flag,
            non_standard_payload_flag,
            reader_id,
            writer_id,
            writer_sn,
            inline_qos,
            serialized_payload,
        );
        let mut buffer = [0u8; 512];
        let value = RtpsMessageWrite::new(&mut buffer, &header, &[&submessage]);
        #[rustfmt::skip]
        assert_eq!(value.buffer(), vec![
            b'R', b'T', b'P', b'S', // Protocol
            2, 3, 9, 8, // ProtocolVersion | VendorId
            3, 3, 3, 3, // GuidPrefix
            3, 3, 3, 3, // GuidPrefix
            3, 3, 3, 3, // GuidPrefix
            0x15, 0b_0000_0011, 40, 0, // Submessage header
            0, 0, 16, 0, // extraFlags, octetsToInlineQos
            1, 2, 3, 4, // readerId: value[4]
            6, 7, 8, 9, // writerId: value[4]
            0, 0, 0, 0, // writerSN: high
            5, 0, 0, 0, // writerSN: low
            6, 0, 4, 0, // inlineQos: parameterId_1, length_1
            10, 11, 12, 13, // inlineQos: value_1[length_1]
            7, 0, 4, 0, // inlineQos: parameterId_2, length_2
            20, 21, 22, 23, // inlineQos: value_2[length_2]
            1, 0, 0, 0, // inlineQos: Sentinel
        ]);
    }

    #[test]
    fn serialize_rtps_message_multiple_submessages() {
        let header = RtpsMessageHeader {
            version: ProtocolVersion::new(2, 3),
            vendor_id: [9, 8],
            guid_prefix: [3; 12],
        };
        let info_timestamp_submessage = InfoTimestampSubmessage::new(false, Time::new(4, 0));

        let inline_qos_flag = true;
        let data_flag = false;
        let key_flag = false;
        let non_standard_payload_flag = false;
        let reader_id = EntityId::new([1, 2, 3], USER_DEFINED_READER_NO_KEY);
        let writer_id = EntityId::new([6, 7, 8], USER_DEFINED_READER_GROUP);
        let writer_sn = 5;
        let parameter_1 = Parameter::new(6, vec![10, 11, 12, 13].into());
        let parameter_2 = Parameter::new(7, vec![20, 21, 22, 23].into());
        let inline_qos = ParameterList::new(vec![parameter_1, parameter_2]);
        let serialized_payload = Data::new(vec![].into());

        let data_submessage = DataSubmessage::new(
            inline_qos_flag,
            data_flag,
            key_flag,
            non_standard_payload_flag,
            reader_id,
            writer_id,
            writer_sn,
            inline_qos,
            serialized_payload,
        );
        let mut buffer = [0u8; 512];
        let value = RtpsMessageWrite::new(
            &mut buffer,
            &header,
            &[&info_timestamp_submessage, &data_submessage],
        );
        #[rustfmt::skip]
        assert_eq!(value.buffer(), vec![
            b'R', b'T', b'P', b'S', // Protocol
            2, 3, 9, 8, // ProtocolVersion | VendorId
            3, 3, 3, 3, // GuidPrefix
            3, 3, 3, 3, // GuidPrefix
            3, 3, 3, 3, // GuidPrefix
            0x09_u8, 0b_0000_0001, 8, 0, // Submessage header
            4, 0, 0, 0, // Time
            0, 0, 0, 0, // Time
            0x15, 0b_0000_0011, 40, 0, // Submessage header
            0, 0, 16, 0, // extraFlags, octetsToInlineQos
            1, 2, 3, 4, // readerId: value[4]
            6, 7, 8, 9, // writerId: value[4]
            0, 0, 0, 0, // writerSN: high
            5, 0, 0, 0, // writerSN: low
            6, 0, 4, 0, // inlineQos: parameterId_1, length_1
            10, 11, 12, 13, // inlineQos: value_1[length_1]
            7, 0, 4, 0, // inlineQos: parameterId_2, length_2
            20, 21, 22, 23, // inlineQos: value_2[length_2]
            1, 0, 0, 0, // inlineQos: Sentinel
        ]);
    }

    #[test]
    fn deserialize_rtps_message_no_submessage() {
        let header = RtpsMessageHeader {
            version: ProtocolVersion::new(2, 3),
            vendor_id: [9, 8],
            guid_prefix: [3; 12],
        };

        #[rustfmt::skip]
        let data = [
            b'R', b'T', b'P', b'S', // Protocol
            2, 3, 9, 8, // ProtocolVersion | VendorId
            3, 3, 3, 3, // GuidPrefix
            3, 3, 3, 3, // GuidPrefix
            3, 3, 3, 3, // GuidPrefix
        ];
        let rtps_message = RtpsMessageRead::try_from(&data[..]).unwrap();
        assert_eq!(rtps_message.header(), header);
        assert_eq!(rtps_message.submessages(), vec![]);
    }

    #[test]
    fn deserialize_rtps_message_too_high_submessage_length() {
        #[rustfmt::skip]
        let data = [
            b'R', b'T', b'P', b'S', // Protocol
            2, 3, 9, 8, // ProtocolVersion | VendorId
            3, 3, 3, 3, // GuidPrefix
            3, 3, 3, 3, // GuidPrefix
            3, 3, 3, 3, // GuidPrefix
            0x09_u8, 0b_0000_0001, 8, 0, // Submessage header
            4, 0, 0, 0, // Time (half only)
        ];
        let rtps_message = RtpsMessageRead::try_from(&data[..]).unwrap();
        assert_eq!(rtps_message.submessages(), vec![]);
    }

    #[test]
    fn deserialize_rtps_message() {
        let expected_header = RtpsMessageHeader {
            version: ProtocolVersion::new(2, 3),
            vendor_id: [9, 8],
            guid_prefix: [3; 12],
        };

        #[rustfmt::skip]
        let data = [
            b'R', b'T', b'P', b'S', // Protocol
            2, 3, 9, 8, // ProtocolVersion | VendorId
            3, 3, 3, 3, // GuidPrefix
            3, 3, 3, 3, // GuidPrefix
            3, 3, 3, 3, // GuidPrefix
            0x15, 0b_0000_0011, 40, 0, // Submessage header
            0, 0, 16, 0, // extraFlags, octetsToInlineQos
            1, 2, 3, 4, // readerId: value[4]
            6, 7, 8, 9, // writerId: value[4]
            0, 0, 0, 0, // writerSN: high
            5, 0, 0, 0, // writerSN: low
            6, 0, 4, 0, // inlineQos: parameterId_1, length_1
            10, 11, 12, 13, // inlineQos: value_1[length_1]
            7, 0, 4, 0, // inlineQos: parameterId_2, length_2
            20, 21, 22, 23, // inlineQos: value_2[length_2]
            1, 0, 0, 0, // inlineQos: Sentinel
            0x07, 0b_0000_0101, 28, 0, // Submessage header
            1, 2, 3, 4, // readerId: value[4]
            6, 7, 8, 9, // writerId: value[4]
            0, 0, 0, 0, // firstSN: SequenceNumber: high
            5, 0, 0, 0, // firstSN: SequenceNumber: low
            0, 0, 0, 0, // lastSN: SequenceNumberSet: high
            7, 0, 0, 0, // lastSN: SequenceNumberSet: low
            2, 0, 0, 0, // count: Count: value (long)
        ];

        let rtps_message = RtpsMessageRead::try_from(&data[..]).unwrap();
        assert_eq!(rtps_message.header(), expected_header);
        let submessages = rtps_message.submessages();
        assert_eq!(submessages.len(), 2);
        assert!(matches!(submessages[0], RtpsSubmessageReadKind::Data(..)));
        assert!(matches!(
            submessages[1],
            RtpsSubmessageReadKind::Heartbeat(..)
        ));
    }

    #[test]
    fn deserialize_rtps_message_unknown_submessage() {
        let expected_data_submessage = RtpsSubmessageReadKind::Data(DataSubmessage::new(
            true,
            false,
            false,
            false,
            EntityId::new([1, 2, 3], 4),
            EntityId::new([6, 7, 8], 9),
            5,
            ParameterList::new(vec![
                Parameter::new(6, vec![10, 11, 12, 13].into()),
                Parameter::new(7, vec![20, 21, 22, 23].into()),
            ]),
            Data::default(),
        ));

        let expected_submessages = vec![expected_data_submessage];

        #[rustfmt::skip]
        let data = [
            b'R', b'T', b'P', b'S', // Protocol
            2, 3, 9, 8, // ProtocolVersion | VendorId
            3, 3, 3, 3, // GuidPrefix
            3, 3, 3, 3, // GuidPrefix
            3, 3, 3, 3, // GuidPrefix
            0x99, 0b_0101_0011, 4, 0, // Submessage header
            9, 9, 9, 9, // Unkown data
            0x15, 0b_0000_0011, 40, 0, // Submessage header
            0, 0, 16, 0, // extraFlags, octetsToInlineQos
            1, 2, 3, 4, // readerId: value[4]
            6, 7, 8, 9, // writerId: value[4]
            0, 0, 0, 0, // writerSN: high
            5, 0, 0, 0, // writerSN: low
            6, 0, 4, 0, // inlineQos: parameterId_1, length_1
            10, 11, 12, 13, // inlineQos: value_1[length_1]
            7, 0, 4, 0, // inlineQos: parameterId_2, length_2
            20, 21, 22, 23, // inlineQos: value_2[length_2]
            1, 0, 0, 0, // inlineQos: Sentinel
        ];

        let rtps_message = RtpsMessageRead::try_from(&data[..]).unwrap();
        assert_eq!(expected_submessages, rtps_message.submessages());
    }

    #[test]
    fn deserialize_opendds_style_rtps_message() {
        let expected_header = RtpsMessageHeader {
            version: ProtocolVersion::new(2, 3),
            vendor_id: [9, 8],
            guid_prefix: [3; 12],
        };

        #[rustfmt::skip]
        let data = [
            b'R', b'T', b'P', b'S', // Protocol
            2, 3, 9, 8, // ProtocolVersion | VendorId
            3, 3, 3, 3, // GuidPrefix
            3, 3, 3, 3, // GuidPrefix
            3, 3, 3, 3, // GuidPrefix
            0x15, 0b_0000_0011, 0, 0, // Submessage header
            0, 0, 16, 0, // extraFlags, octetsToInlineQos
            1, 2, 3, 4, // readerId: value[4]
            6, 7, 8, 9, // writerId: value[4]
            0, 0, 0, 0, // writerSN: high
            5, 0, 0, 0, // writerSN: low
            6, 0, 4, 0, // inlineQos: parameterId_1, length_1
            10, 11, 12, 13, // inlineQos: value_1[length_1]
            7, 0, 4, 0, // inlineQos: parameterId_2, length_2
            20, 21, 22, 23, // inlineQos: value_2[length_2]
            1, 0, 0, 0, // inlineQos: Sentinel
        ];

        let rtps_message = RtpsMessageRead::try_from(&data[..]).unwrap();
        assert_eq!(rtps_message.header(), expected_header);
        let submessages = rtps_message.submessages();
        assert_eq!(submessages.len(), 1);
        assert!(matches!(submessages[0], RtpsSubmessageReadKind::Data(..)));
    }
}
