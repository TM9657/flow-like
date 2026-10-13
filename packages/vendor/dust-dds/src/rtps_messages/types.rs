use super::{
    error::RtpsMessageResult,
    overall_structure::{Endianness, Read, TryReadFromBytes, Write, WriteIntoBytes},
};

// This files shall only contain the types as listed in the DDSI-RTPS Version 2.5
// Table 8.13 - Types used to define RTPS messages

type Octet = u8;
type Long = i32;
type UnsignedLong = u32;
type Short = i16;
type UnsignedShort = u16;

impl TryReadFromBytes for Long {
    fn try_read_from_bytes(data: &mut &[u8], endianness: &Endianness) -> RtpsMessageResult<Self> {
        let mut bytes = [0; 4];
        data.read_exact(&mut bytes)?;
        Ok(match endianness {
            Endianness::BigEndian => i32::from_be_bytes(bytes),
            Endianness::LittleEndian => i32::from_le_bytes(bytes),
        })
    }
}

impl TryReadFromBytes for UnsignedLong {
    fn try_read_from_bytes(data: &mut &[u8], endianness: &Endianness) -> RtpsMessageResult<Self> {
        let mut bytes = [0; 4];
        data.read_exact(&mut bytes)?;
        Ok(match endianness {
            Endianness::BigEndian => u32::from_be_bytes(bytes),
            Endianness::LittleEndian => u32::from_le_bytes(bytes),
        })
    }
}

impl TryReadFromBytes for Short {
    fn try_read_from_bytes(data: &mut &[u8], endianness: &Endianness) -> RtpsMessageResult<Self> {
        let mut bytes = [0; 2];
        data.read_exact(&mut bytes)?;
        Ok(match endianness {
            Endianness::BigEndian => i16::from_be_bytes(bytes),
            Endianness::LittleEndian => i16::from_le_bytes(bytes),
        })
    }
}

impl TryReadFromBytes for UnsignedShort {
    fn try_read_from_bytes(data: &mut &[u8], endianness: &Endianness) -> RtpsMessageResult<Self> {
        let mut bytes = [0; 2];
        data.read_exact(&mut bytes)?;
        Ok(match endianness {
            Endianness::BigEndian => u16::from_be_bytes(bytes),
            Endianness::LittleEndian => u16::from_le_bytes(bytes),
        })
    }
}

/// ProtocolId_t
/// Enumeration used to identify the protocol.
/// The following values are reserved by the protocol: PROTOCOL_RTPS
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
#[allow(non_camel_case_types)]
pub enum ProtocolId {
    PROTOCOL_RTPS,
}

impl WriteIntoBytes for ProtocolId {
    fn write_into_bytes(&self, buf: &mut dyn Write) {
        b"RTPS".write_into_bytes(buf);
    }
}

/// SubmessageFlag
/// Type used to specify a Submessage flag.
/// A Submessage flag takes a boolean value and affects the parsing of the Submessage by the receiver.
pub type SubmessageFlag = bool;

/// SubmessageKind
/// Enumeration used to identify the kind of Submessage.
/// The following values are reserved by this version of the protocol:
/// DATA, GAP, HEARTBEAT, ACKNACK, PAD, INFO_TS, INFO_REPLY, INFO_DST, INFO_SRC, DATA_FRAG, NACK_FRAG, HEARTBEAT_FRAG
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
#[allow(non_camel_case_types)]
#[allow(clippy::upper_case_acronyms)]
pub enum SubmessageKind {
    DATA,
    GAP,
    HEARTBEAT,
    ACKNACK,
    PAD,
    INFO_TS,
    INFO_REPLY,
    INFO_DST,
    INFO_SRC,
    DATA_FRAG,
    NACK_FRAG,
    HEARTBEAT_FRAG,
}

pub const DATA: u8 = 0x15;
pub const GAP: u8 = 0x08;
pub const HEARTBEAT: u8 = 0x07;
pub const ACKNACK: u8 = 0x06;
pub const PAD: u8 = 0x01;
pub const INFO_TS: u8 = 0x09;
pub const INFO_REPLY: u8 = 0x0f;
pub const INFO_DST: u8 = 0x0e;
pub const INFO_SRC: u8 = 0x0c;
pub const DATA_FRAG: u8 = 0x16;
pub const NACK_FRAG: u8 = 0x12;
pub const HEARTBEAT_FRAG: u8 = 0x13;

impl WriteIntoBytes for SubmessageKind {
    fn write_into_bytes(&self, buf: &mut dyn Write) {
        let data = match self {
            SubmessageKind::DATA => DATA,
            SubmessageKind::GAP => GAP,
            SubmessageKind::HEARTBEAT => HEARTBEAT,
            SubmessageKind::ACKNACK => ACKNACK,
            SubmessageKind::PAD => PAD,
            SubmessageKind::INFO_TS => INFO_TS,
            SubmessageKind::INFO_REPLY => INFO_REPLY,
            SubmessageKind::INFO_DST => INFO_DST,
            SubmessageKind::INFO_SRC => INFO_SRC,
            SubmessageKind::DATA_FRAG => DATA_FRAG,
            SubmessageKind::NACK_FRAG => NACK_FRAG,
            SubmessageKind::HEARTBEAT_FRAG => HEARTBEAT_FRAG,
        };
        data.write_into_bytes(buf);
    }
}

/// Time_t
/// Type used to hold a timestamp.
/// Should have at least nano-second resolution.
/// The following values are reserved by the protocol:
/// TIME_ZERO, TIME_INVALID, TIME_INFINITE

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct Time {
    seconds: UnsignedLong,
    fraction: UnsignedLong,
}

impl Time {
    pub const fn new(seconds: UnsignedLong, fraction: UnsignedLong) -> Self {
        Self { seconds, fraction }
    }

    pub fn seconds(&self) -> UnsignedLong {
        self.seconds
    }

    pub fn fraction(&self) -> UnsignedLong {
        self.fraction
    }

    pub fn try_read_from_bytes(
        data: &mut &[u8],
        endianness: &Endianness,
    ) -> RtpsMessageResult<Self> {
        let seconds = UnsignedLong::try_read_from_bytes(data, endianness)?;
        let fraction = UnsignedLong::try_read_from_bytes(data, endianness)?;
        Ok(Self { seconds, fraction })
    }
}

impl From<crate::transport::types::Time> for Time {
    fn from(value: crate::transport::types::Time) -> Self {
        Self::new(value.sec() as u32, nanosec_to_fraction(value.nanosec()))
    }
}

impl From<Time> for crate::transport::types::Time {
    fn from(value: Time) -> Self {
        crate::transport::types::Time::new(
            value.seconds() as i32,
            fraction_to_nanosec(value.fraction()),
        )
    }
}

fn fraction_to_nanosec(fraction: u32) -> u32 {
    ((fraction as u64 * 1_000_000_000) / (1u64 << 32)) as u32
}

fn nanosec_to_fraction(nanosec: u32) -> u32 {
    (((nanosec as u64 * (1u64 << 32)) + 500_000_000) / 1_000_000_000) as u32
}

#[allow(dead_code)]
pub const TIME_ZERO: Time = Time::new(0, 0);
pub const TIME_INVALID: Time = Time::new(0xffffffff, 0xffffffff);
#[allow(dead_code)]
pub const TIME_INFINITE: Time = Time::new(0xffffffff, 0xfffffffe);

impl WriteIntoBytes for Time {
    fn write_into_bytes(&self, buf: &mut dyn Write) {
        self.seconds.write_into_bytes(buf);
        self.fraction.write_into_bytes(buf);
    }
}

/// Count_t
/// Type used to hold a count that is incremented monotonically, used to identify message duplicates.
pub type Count = Long;

/// Checksum_t
/// Type used to hold a checksum. Used to detect RTPS message corruption by the underlying transport.
/// The following values are reserved by the protocol: CHECKSUM_INVALID.
#[allow(dead_code)]
pub type Checksum32 = [Octet; 4];

/// MessageLength_t
/// Type used to hold the length of an RTPS Message.
/// The following values are reserved by the protocol: MESSAGE_LENGTH_INVALID
#[allow(dead_code)]
struct MessageLength;

/// ParameterId_t
/// Type used to uniquely identify a parameter in a parameter list.
/// Used extensively by the Discovery Module mainly to define QoS Parameters. A range of values is reserved for protocol-defined parameters, while another range can be used for vendor-defined parameters, see 8.3.5.9.
pub type ParameterId = Short;

/// FragmentNumber_t
/// Type used to hold fragment numbers.
/// Must be possible to represent using 32 bits.
pub type FragmentNumber = UnsignedLong;

/// GroupDigest_t
/// Type used to hold a digest value that uniquely identifies a group of Entities belonging to the same Participant.
#[allow(dead_code)]
pub type GroupDigest = [Octet; 4];

/// UExtension4_t
/// Type used to hold an undefined 4-byte value. It is intended to be used in future revisions of the specification.
#[allow(dead_code)]
pub type UExtension4 = [Octet; 4];

/// WExtension8_t
/// Type used to hold an undefined 8-byte value. It is intended to be used in future revisions of the specification.
#[allow(dead_code)]
pub type WExtension8 = [Octet; 8];
