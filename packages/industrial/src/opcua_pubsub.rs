//! OPC UA Part 14 UADP over IPv4 UDP with externally configured scalar DataSet metadata.
//! Publishes one Variant-encoded key frame per datagram. Readers require publisher,
//! writer group, and writer IDs. Optional PubSub-Aes256-CTR uses configured group keys.
//! Discovery, delta frames, RawData, arrays, and structured values are unsupported.

use crate::{Error, Result, require};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::net::{Ipv4Addr, SocketAddrV4};

const MAX_FIELDS: usize = 256;
#[cfg(any(test, feature = "opcua-pubsub"))]
const MAX_VALUE_BYTES: usize = 16_384;
const MAX_DATAGRAM_BYTES: usize = 65_507;

#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
#[serde(tag = "type", content = "value", rename_all = "snake_case")]
pub enum PublisherId {
    Byte(u8),
    #[serde(rename = "uint16")]
    UInt16(u16),
    #[serde(rename = "uint32")]
    UInt32(u32),
    #[serde(rename = "uint64")]
    UInt64(u64),
    String(String),
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum PubSubDataType {
    Boolean,
    #[serde(rename = "sbyte")]
    SByte,
    Byte,
    Int16,
    #[serde(rename = "uint16")]
    UInt16,
    Int32,
    #[serde(rename = "uint32")]
    UInt32,
    Int64,
    #[serde(rename = "uint64")]
    UInt64,
    Float,
    Double,
    String,
    #[serde(rename = "date_time")]
    DateTime,
    #[serde(rename = "byte_string")]
    ByteString,
    #[serde(rename = "status_code")]
    StatusCode,
}

#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema, PartialEq)]
#[serde(tag = "type", content = "value", rename_all = "snake_case")]
pub enum PubSubValue {
    Boolean(bool),
    #[serde(rename = "sbyte")]
    SByte(i8),
    Byte(u8),
    Int16(i16),
    #[serde(rename = "uint16")]
    UInt16(u16),
    Int32(i32),
    #[serde(rename = "uint32")]
    UInt32(u32),
    Int64(i64),
    #[serde(rename = "uint64")]
    UInt64(u64),
    Float(f32),
    Double(f64),
    String(Option<String>),
    /// OPC UA DateTime: signed 100 ns intervals since 1601-01-01 UTC.
    #[serde(rename = "date_time")]
    DateTime(i64),
    #[serde(rename = "byte_string")]
    ByteString(Option<Vec<u8>>),
    #[serde(rename = "status_code")]
    StatusCode(u32),
}

impl PubSubValue {
    pub fn data_type(&self) -> PubSubDataType {
        match self {
            Self::Boolean(_) => PubSubDataType::Boolean,
            Self::SByte(_) => PubSubDataType::SByte,
            Self::Byte(_) => PubSubDataType::Byte,
            Self::Int16(_) => PubSubDataType::Int16,
            Self::UInt16(_) => PubSubDataType::UInt16,
            Self::Int32(_) => PubSubDataType::Int32,
            Self::UInt32(_) => PubSubDataType::UInt32,
            Self::Int64(_) => PubSubDataType::Int64,
            Self::UInt64(_) => PubSubDataType::UInt64,
            Self::Float(_) => PubSubDataType::Float,
            Self::Double(_) => PubSubDataType::Double,
            Self::String(_) => PubSubDataType::String,
            Self::DateTime(_) => PubSubDataType::DateTime,
            Self::ByteString(_) => PubSubDataType::ByteString,
            Self::StatusCode(_) => PubSubDataType::StatusCode,
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
pub struct PubSubFieldMetaData {
    /// Names are local labels. Both peers must configure the same field order and types.
    pub name: String,
    pub data_type: PubSubDataType,
}

#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
pub struct PubSubWriter {
    pub publisher_id: PublisherId,
    pub writer_group_id: u16,
    pub data_set_writer_id: u16,
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
pub struct ConfigurationVersion {
    pub major: u32,
    pub minor: u32,
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum PubSubMode {
    Publisher,
    Subscriber,
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum PubSubSecurityMode {
    Sign,
    SignAndEncrypt,
}

/// Manually provisioned PubSub-Aes256-CTR group keys. Use secret variables and separate keys per publisher.
#[derive(Clone, Serialize, Deserialize, JsonSchema)]
#[cfg_attr(
    feature = "opcua-pubsub",
    derive(zeroize::Zeroize, zeroize::ZeroizeOnDrop)
)]
pub struct PubSubSecurity {
    #[cfg_attr(feature = "opcua-pubsub", zeroize(skip))]
    pub mode: PubSubSecurityMode,
    #[cfg_attr(feature = "opcua-pubsub", zeroize(skip))]
    pub security_token_id: u32,
    /// Exactly 32 bytes, supplied from a secret variable.
    pub signing_key: Vec<u8>,
    /// Exactly 32 bytes, distinct from the signing key.
    pub encrypting_key: Vec<u8>,
    /// Exactly 4 bytes. Rotate together with the encryption key when executor state is lost.
    pub key_nonce: Vec<u8>,
}

impl std::fmt::Debug for PubSubSecurity {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("PubSubSecurity")
            .field("mode", &self.mode)
            .field("security_token_id", &self.security_token_id)
            .field("keys", &"[redacted]")
            .finish()
    }
}

impl PubSubSecurity {
    pub fn validate(&self) -> Result<()> {
        require(
            self.security_token_id != 0,
            "OPC UA PubSub security token ID must be nonzero",
        )?;
        require(
            self.signing_key.len() == 32
                && self.encrypting_key.len() == 32
                && self.key_nonce.len() == 4,
            "PubSub-Aes256-CTR requires a 32-byte signing key, 32-byte encrypting key and 4-byte key nonce",
        )?;
        require(
            self.signing_key != self.encrypting_key,
            "OPC UA PubSub signing and encryption keys must be distinct",
        )
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema)]
pub struct OpcUaPubSubConfig {
    /// An opc.udp://IPv4:port address. Subscribers bind this unicast address or join this group.
    pub address: String,
    /// Local IPv4 interface for multicast. 0.0.0.0 uses the operating system route.
    pub interface: String,
    pub mode: PubSubMode,
    pub writer: PubSubWriter,
    pub fields: Vec<PubSubFieldMetaData>,
    /// When set, publications include both versions and subscriptions require an exact match.
    #[serde(default)]
    pub configuration_version: Option<ConfigurationVersion>,
    pub timeout_ms: u64,
    /// Includes UADP headers. UDP fragmentation may occur above the network MTU.
    pub max_datagram_bytes: usize,
    /// Omit for plaintext. Secure sessions require persistent executor state and fail closed on downgrade.
    #[serde(default)]
    pub security: Option<PubSubSecurity>,
}

impl OpcUaPubSubConfig {
    pub fn validate(&self) -> Result<()> {
        self.destination()?;
        if let Some(security) = &self.security {
            security.validate()?;
        }
        self.interface.parse::<Ipv4Addr>().map_err(|_| {
            Error::Invalid("OPC UA PubSub interface must be an IPv4 address".into())
        })?;
        require(
            (1..=300_000).contains(&self.timeout_ms),
            "OPC UA PubSub timeout must be in 1..300000 ms",
        )?;
        require(
            (64..=MAX_DATAGRAM_BYTES).contains(&self.max_datagram_bytes),
            "OPC UA PubSub datagram limit must be in 64..65507 bytes",
        )?;
        require(
            !self.fields.is_empty() && self.fields.len() <= MAX_FIELDS,
            "OPC UA PubSub requires 1..256 scalar metadata fields",
        )?;
        let mut names = std::collections::HashSet::new();
        for field in &self.fields {
            require(
                !field.name.is_empty() && field.name.len() <= 256 && names.insert(&field.name),
                "OPC UA PubSub field names must be unique and contain 1..256 bytes",
            )?;
        }
        if let PublisherId::String(id) = &self.writer.publisher_id {
            require(
                !id.is_empty() && id.len() <= 256,
                "OPC UA PubSub string publisher ID must contain 1..256 bytes",
            )?;
        }
        Ok(())
    }

    fn destination(&self) -> Result<SocketAddrV4> {
        let address = self
            .address
            .strip_prefix("opc.udp://")
            .and_then(|s| s.strip_suffix('/').or(Some(s)))
            .and_then(|s| s.parse::<SocketAddrV4>().ok())
            .ok_or_else(|| {
                Error::Invalid("OPC UA PubSub address must be opc.udp://IPv4:port".into())
            })?;
        require(
            address.port() != 0 && !address.ip().is_unspecified() && !address.ip().is_broadcast(),
            "OPC UA PubSub requires a nonzero port and a unicast or multicast destination",
        )?;
        Ok(address)
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema, PartialEq)]
pub struct PubSubField {
    pub name: String,
    pub value: PubSubValue,
}

#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema, PartialEq)]
pub struct PubSubMessage {
    pub writer: PubSubWriter,
    pub sequence_number: Option<u16>,
    pub network_sequence_number: Option<u16>,
    pub configuration_major_version: Option<u32>,
    pub configuration_minor_version: Option<u32>,
    pub timestamp: Option<i64>,
    pub status: Option<u16>,
    pub security_token_id: Option<u32>,
    pub security_sequence_number: Option<u32>,
    pub fields: Vec<PubSubField>,
}

#[cfg(any(test, feature = "opcua-pubsub"))]
fn invalid(message: &str) -> Error {
    Error::Invalid(format!("OPC UA PubSub: {message}"))
}

// Part 14 §7.2.4.4/5 and Part 6 §5.2.2: all integers use UA Binary little-endian encoding.
#[cfg(any(test, feature = "opcua-pubsub"))]
fn encode(config: &OpcUaPubSubConfig, values: &[PubSubValue], sequence: u16) -> Result<Vec<u8>> {
    require(
        values.len() == config.fields.len(),
        "OPC UA PubSub values must match the configured field count",
    )?;
    let mut out = vec![0xf1]; // Version 1, publisher, group, payload and extended flags.
    match &config.writer.publisher_id {
        PublisherId::Byte(id) => {
            out[0] = 0x71;
            out.push(*id);
        }
        PublisherId::UInt16(id) => {
            out.push(1);
            out.extend(id.to_le_bytes());
        }
        PublisherId::UInt32(id) => {
            out.push(2);
            out.extend(id.to_le_bytes());
        }
        PublisherId::UInt64(id) => {
            out.push(3);
            out.extend(id.to_le_bytes());
        }
        PublisherId::String(id) => {
            out.push(4);
            encode_bytes(&mut out, Some(id.as_bytes()))?;
        }
    }
    out.push(9); // WriterGroupId and network sequence number.
    out.extend(config.writer.writer_group_id.to_le_bytes());
    out.extend(sequence.to_le_bytes());
    out.push(1); // One DataSetMessage.
    out.extend(config.writer.data_set_writer_id.to_le_bytes());
    out.push(if config.configuration_version.is_some() {
        0x69
    } else {
        0x09
    });
    out.extend(sequence.to_le_bytes());
    if let Some(version) = config.configuration_version {
        out.extend(version.major.to_le_bytes());
        out.extend(version.minor.to_le_bytes());
    }
    out.extend((values.len() as u16).to_le_bytes());
    for (meta, value) in config.fields.iter().zip(values) {
        validate_field(meta, value)?;
        encode_value(&mut out, value)?;
        require(
            out.len() <= config.max_datagram_bytes,
            "OPC UA PubSub encoded message exceeds the datagram limit",
        )?;
    }
    Ok(out)
}

#[cfg(any(test, feature = "opcua-pubsub"))]
fn validate_field(meta: &PubSubFieldMetaData, value: &PubSubValue) -> Result<()> {
    // Part 14 permits a Bad StatusCode in place of a field whose value could not be obtained.
    let bad_status = matches!(value, PubSubValue::StatusCode(status) if status & 0x8000_0000 != 0);
    require(
        meta.data_type == value.data_type() || bad_status,
        "OPC UA PubSub field type does not match the configured metadata",
    )
}

#[cfg(any(test, feature = "opcua-pubsub"))]
fn encode_bytes(out: &mut Vec<u8>, bytes: Option<&[u8]>) -> Result<()> {
    match bytes {
        None => out.extend((-1_i32).to_le_bytes()),
        Some(bytes) => {
            require(
                bytes.len() <= MAX_VALUE_BYTES,
                "OPC UA PubSub string or byte string exceeds 16384 bytes",
            )?;
            out.extend((bytes.len() as i32).to_le_bytes());
            out.extend(bytes);
        }
    }
    Ok(())
}

#[cfg(any(test, feature = "opcua-pubsub"))]
fn encode_value(out: &mut Vec<u8>, value: &PubSubValue) -> Result<()> {
    macro_rules! scalar {
        ($id:expr, $value:expr) => {{
            out.push($id);
            out.extend($value.to_le_bytes());
        }};
    }
    match value {
        PubSubValue::Boolean(v) => out.extend([1, u8::from(*v)]),
        PubSubValue::SByte(v) => out.extend([2, *v as u8]),
        PubSubValue::Byte(v) => out.extend([3, *v]),
        PubSubValue::Int16(v) => scalar!(4, v),
        PubSubValue::UInt16(v) => scalar!(5, v),
        PubSubValue::Int32(v) => scalar!(6, v),
        PubSubValue::UInt32(v) => scalar!(7, v),
        PubSubValue::Int64(v) => scalar!(8, v),
        PubSubValue::UInt64(v) => scalar!(9, v),
        PubSubValue::Float(v) => {
            require(v.is_finite(), "OPC UA PubSub Float must be finite")?;
            scalar!(10, v);
        }
        PubSubValue::Double(v) => {
            require(v.is_finite(), "OPC UA PubSub Double must be finite")?;
            scalar!(11, v);
        }
        PubSubValue::String(v) => {
            out.push(12);
            encode_bytes(out, v.as_ref().map(|s| s.as_bytes()))?;
        }
        PubSubValue::DateTime(v) => scalar!(13, v),
        PubSubValue::ByteString(v) => {
            out.push(15);
            encode_bytes(out, v.as_deref())?;
        }
        PubSubValue::StatusCode(v) => scalar!(19, v),
    }
    Ok(())
}

#[cfg(any(test, feature = "opcua-pubsub"))]
struct Decoder<'a> {
    bytes: &'a [u8],
}
#[cfg(any(test, feature = "opcua-pubsub"))]
impl<'a> Decoder<'a> {
    fn take(&mut self, count: usize) -> Result<&'a [u8]> {
        let chunk = self
            .bytes
            .get(..count)
            .ok_or_else(|| invalid("truncated UADP message"))?;
        self.bytes = &self.bytes[count..];
        Ok(chunk)
    }
    fn array<const N: usize>(&mut self) -> Result<[u8; N]> {
        Ok(self.take(N)?.try_into().unwrap())
    }
    fn u8(&mut self) -> Result<u8> {
        Ok(self.array::<1>()?[0])
    }
    fn u16(&mut self) -> Result<u16> {
        Ok(u16::from_le_bytes(self.array()?))
    }
    fn u32(&mut self) -> Result<u32> {
        Ok(u32::from_le_bytes(self.array()?))
    }
    fn u64(&mut self) -> Result<u64> {
        Ok(u64::from_le_bytes(self.array()?))
    }
    fn i64(&mut self) -> Result<i64> {
        Ok(i64::from_le_bytes(self.array()?))
    }
    fn bytes(&mut self) -> Result<Option<&'a [u8]>> {
        let count = i32::from_le_bytes(self.array()?);
        if count == -1 {
            return Ok(None);
        }
        require(
            (0..=MAX_VALUE_BYTES as i32).contains(&count),
            "OPC UA PubSub invalid or oversized string length",
        )?;
        Ok(Some(self.take(count as usize)?))
    }
    fn string(&mut self) -> Result<Option<String>> {
        self.bytes()?
            .map(|b| {
                std::str::from_utf8(b)
                    .map(str::to_owned)
                    .map_err(|_| invalid("invalid UTF-8 string"))
            })
            .transpose()
    }
    fn value(&mut self) -> Result<PubSubValue> {
        Ok(match self.u8()? {
            1 => {
                let value = self.u8()?;
                PubSubValue::Boolean(value != 0)
            }
            2 => PubSubValue::SByte(self.u8()? as i8),
            3 => PubSubValue::Byte(self.u8()?),
            4 => PubSubValue::Int16(i16::from_le_bytes(self.array()?)),
            5 => PubSubValue::UInt16(self.u16()?),
            6 => PubSubValue::Int32(i32::from_le_bytes(self.array()?)),
            7 => PubSubValue::UInt32(self.u32()?),
            8 => PubSubValue::Int64(self.i64()?),
            9 => PubSubValue::UInt64(self.u64()?),
            10 => {
                let v = f32::from_le_bytes(self.array()?);
                require(v.is_finite(), "OPC UA PubSub received a non-finite Float")?;
                PubSubValue::Float(v)
            }
            11 => {
                let v = f64::from_le_bytes(self.array()?);
                require(v.is_finite(), "OPC UA PubSub received a non-finite Double")?;
                PubSubValue::Double(v)
            }
            12 => PubSubValue::String(self.string()?),
            13 => PubSubValue::DateTime(self.i64()?),
            15 => PubSubValue::ByteString(self.bytes()?.map(<[u8]>::to_vec)),
            19 => PubSubValue::StatusCode(self.u32()?),
            _ => {
                return Err(invalid(
                    "unsupported Variant type, array, or structured value",
                ));
            }
        })
    }
}

#[cfg(any(test, feature = "opcua-pubsub"))]
fn decode_header(config: &OpcUaPubSubConfig, bytes: &[u8]) -> Result<Option<NetworkHeader>> {
    require(
        bytes.len() <= config.max_datagram_bytes,
        "OPC UA PubSub received datagram exceeds the configured limit",
    )?;
    let mut d = Decoder { bytes };
    let flags = d.u8()?;
    require(flags & 15 == 1, "OPC UA PubSub unsupported UADP version")?;
    let ext = if flags & 0x80 != 0 { d.u8()? } else { 0 };
    let ext2 = if ext & 0x80 != 0 { d.u8()? } else { 0 };
    require(flags & 0x10 != 0, "OPC UA PubSub requires a PublisherId")?;
    let publisher_id = match ext & 7 {
        0 => PublisherId::Byte(d.u8()?),
        1 => PublisherId::UInt16(d.u16()?),
        2 => PublisherId::UInt32(d.u32()?),
        3 => PublisherId::UInt64(d.u64()?),
        4 => PublisherId::String(d.string()?.ok_or_else(|| invalid("null publisher ID"))?),
        _ => return Err(invalid("reserved publisher ID type")),
    };
    if publisher_id != config.writer.publisher_id {
        return Ok(None);
    }
    require(flags & 0x20 != 0, "OPC UA PubSub requires a group header")?;
    if ext & 8 != 0 {
        d.take(16)?;
    }
    let group = d.u8()?;
    require(
        group & 0xf0 == 0 && group & 1 != 0,
        "OPC UA PubSub unsupported group flags or missing WriterGroupId",
    )?;
    let writer_group_id = d.u16()?;
    if writer_group_id != config.writer.writer_group_id {
        return Ok(None);
    }
    if group & 2 != 0 {
        d.u32()?;
    }
    if group & 4 != 0 {
        require(
            d.u16()? != 0,
            "OPC UA PubSub NetworkMessageNumber cannot be zero",
        )?;
    }
    let network_sequence_number = if group & 8 != 0 { Some(d.u16()?) } else { None };
    require(
        ext2 & 0x1c == 0,
        "OPC UA PubSub discovery and extended message types are unsupported",
    )?;
    require(flags & 0x40 != 0, "OPC UA PubSub requires a payload header")?;
    let count = d.u8()? as usize;
    require(count > 0, "OPC UA PubSub empty payload header")?;
    let mut ids = Vec::with_capacity(count);
    for _ in 0..count {
        ids.push(d.u16()?);
    }
    let Some(index) = ids
        .iter()
        .position(|&id| id == config.writer.data_set_writer_id)
    else {
        return Ok(None);
    };
    require(
        ext2 == 0,
        "OPC UA PubSub chunks, promoted fields and extended message types are unsupported",
    )?;
    require(
        ext & 0x40 == 0 || ext & 0x20 != 0,
        "OPC UA PubSub picoseconds require a timestamp",
    )?;
    require(
        ids.iter()
            .filter(|&&id| id == config.writer.data_set_writer_id)
            .count()
            == 1,
        "OPC UA PubSub duplicate DataSetWriterId",
    )?;
    if ext & 0x20 != 0 {
        d.i64()?;
    }
    if ext & 0x40 != 0 {
        d.u16()?;
    }
    Ok(Some(NetworkHeader {
        offset: bytes.len() - d.bytes.len(),
        count,
        index,
        network_sequence_number,
        secure: ext & 0x10 != 0,
    }))
}

#[cfg(any(test, feature = "opcua-pubsub"))]
struct NetworkHeader {
    offset: usize,
    count: usize,
    index: usize,
    network_sequence_number: Option<u16>,
    secure: bool,
}

#[cfg(any(test, feature = "opcua-pubsub"))]
fn decode(config: &OpcUaPubSubConfig, bytes: &[u8]) -> Result<Option<PubSubMessage>> {
    let Some(header) = decode_header(config, bytes)? else {
        return Ok(None);
    };
    require(
        !header.secure && config.security.is_none(),
        "OPC UA PubSub signed messages require configured security; security downgrade is rejected",
    )?;
    decode_payload(config, &header, &bytes[header.offset..]).map(Some)
}

#[cfg(any(test, feature = "opcua-pubsub"))]
fn decode_payload(
    config: &OpcUaPubSubConfig,
    header: &NetworkHeader,
    payload: &[u8],
) -> Result<PubSubMessage> {
    let mut d = Decoder { bytes: payload };
    if header.count > 1 {
        let mut sizes = Vec::with_capacity(header.count);
        for _ in 0..header.count {
            sizes.push(d.u16()? as usize);
        }
        require(
            sizes.iter().sum::<usize>() == d.bytes.len(),
            "OPC UA PubSub DataSet message sizes do not match the payload",
        )?;
        for size in &sizes[..header.index] {
            d.take(*size)?;
        }
        d = Decoder {
            bytes: d.take(sizes[header.index])?,
        };
    }
    decode_dataset(config, &mut d, header.network_sequence_number)
}

#[cfg(any(test, feature = "opcua-pubsub"))]
fn decode_dataset(
    config: &OpcUaPubSubConfig,
    d: &mut Decoder<'_>,
    network_sequence_number: Option<u16>,
) -> Result<PubSubMessage> {
    let flags = d.u8()?;
    require(
        flags & 1 != 0,
        "OPC UA PubSub DataSetMessage is marked invalid",
    )?;
    require(
        flags & 6 == 0,
        "OPC UA PubSub only Variant field encoding is supported",
    )?;
    let ext = if flags & 0x80 != 0 { d.u8()? } else { 0 };
    require(
        ext & 0xcf == 0,
        "OPC UA PubSub only data key frames are supported",
    )?;
    require(
        ext & 0x20 == 0 || ext & 0x10 != 0,
        "OPC UA PubSub picoseconds require a timestamp",
    )?;
    let sequence_number = if flags & 8 != 0 { Some(d.u16()?) } else { None };
    let timestamp = if ext & 0x10 != 0 {
        Some(d.i64()?)
    } else {
        None
    };
    if ext & 0x20 != 0 {
        d.u16()?;
    }
    let status = if flags & 0x10 != 0 {
        Some(d.u16()?)
    } else {
        None
    };
    let major = if flags & 0x20 != 0 {
        Some(d.u32()?)
    } else {
        None
    };
    let minor = if flags & 0x40 != 0 {
        Some(d.u32()?)
    } else {
        None
    };
    if let Some(version) = config.configuration_version {
        require(
            major == Some(version.major) && minor == Some(version.minor),
            "OPC UA PubSub configuration version does not match the metadata",
        )?;
    }
    require(
        d.u16()? as usize == config.fields.len(),
        "OPC UA PubSub field count does not match the metadata",
    )?;
    let mut fields = Vec::with_capacity(config.fields.len());
    for meta in &config.fields {
        let value = d.value()?;
        validate_field(meta, &value)?;
        fields.push(PubSubField {
            name: meta.name.clone(),
            value,
        });
    }
    require(
        d.bytes.iter().all(|&byte| byte == 0),
        "OPC UA PubSub unexpected nonzero trailing data",
    )?;
    Ok(PubSubMessage {
        writer: config.writer.clone(),
        sequence_number,
        network_sequence_number,
        configuration_major_version: major,
        configuration_minor_version: minor,
        timestamp,
        status,
        security_token_id: None,
        security_sequence_number: None,
        fields,
    })
}

#[cfg(feature = "opcua-pubsub")]
mod security {
    use super::*;
    use aes::cipher::{KeyIvInit, StreamCipher};
    use hmac::{Hmac, Mac};
    use sha2::{Digest, Sha256};
    use std::{
        fs::{File, OpenOptions},
        io::{Read, Seek, SeekFrom, Write},
        path::{Path, PathBuf},
    };

    type HmacSha256 = Hmac<Sha256>;
    type Aes256Ctr = ctr::Ctr32BE<aes::Aes256>;

    #[derive(Clone)]
    pub(super) struct CounterStore {
        path: PathBuf,
        lease: Option<std::sync::Arc<std::sync::Mutex<Option<File>>>>,
    }

    impl CounterStore {
        pub async fn open(config: &OpcUaPubSubConfig, directory: &Path) -> Result<Self> {
            let keys = config
                .security
                .as_ref()
                .ok_or_else(|| invalid("security keys are missing"))?;
            let mut hash = Sha256::new();
            // Token changes never permit CTR counter reuse with the same encryption key and key nonce.
            hash.update(&keys.encrypting_key);
            hash.update(&keys.key_nonce);
            let role = if config.mode == PubSubMode::Publisher {
                "publisher"
            } else {
                hash.update(&keys.signing_key);
                hash.update(serde_json::to_vec(&config.writer).map_err(anyhow::Error::from)?);
                "subscriber"
            };
            let id: String = hash.finalize().iter().map(|b| format!("{b:02x}")).collect();
            let directory = directory.to_owned();
            let mut store = Self {
                path: directory.join(format!("{role}-{id}.state")),
                lease: None,
            };
            let pending = store.clone();
            let subscriber = config.mode == PubSubMode::Subscriber;
            let lease = tokio::task::spawn_blocking(move || {
                private_directory(&directory)?;
                let mut options = OpenOptions::new();
                options.read(true).write(true).create_new(true);
                #[cfg(unix)]
                {
                    use std::os::unix::fs::OpenOptionsExt;
                    options.mode(0o600);
                }
                match options.open(&pending.path) {
                    Ok(mut file) => {
                        fs2::FileExt::try_lock_exclusive(&file)?;
                        Self::write(&mut file, 0)?;
                        #[cfg(unix)]
                        File::open(&directory)?.sync_all()?;
                    }
                    Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => {
                        let mut file = pending.lock()?;
                        Self::read(&mut file)?;
                    }
                    Err(error) => return Err(error.into()),
                }
                if subscriber {
                    let path = pending.path.with_extension("lease");
                    if let Ok(metadata) = std::fs::symlink_metadata(&path) {
                        require(metadata.is_file() && !metadata.file_type().is_symlink(), "security reader lease must be a regular file")?;
                    }
                    let mut options = OpenOptions::new();
                    options.read(true).write(true).create(true).truncate(false);
                    #[cfg(unix)]
                    {
                        use std::os::unix::fs::OpenOptionsExt;
                        options.mode(0o600).custom_flags(libc::O_NOFOLLOW);
                    }
                    let file = options.open(path)?;
                    fs2::FileExt::try_lock_exclusive(&file).map_err(|_| invalid("a secure reader for this writer is already open in this state directory"))?;
                    Ok(Some(file))
                } else {
                    Ok::<_, Error>(None)
                }
            })
            .await
            .map_err(|_| invalid("security state worker failed"))??;
            store.lease = lease.map(|file| std::sync::Arc::new(std::sync::Mutex::new(Some(file))));
            Ok(store)
        }

        pub fn close(&self) -> Result<()> {
            if let Some(lease) = &self.lease {
                lease
                    .lock()
                    .map_err(|_| invalid("security reader lease lock poisoned"))?
                    .take();
            }
            Ok(())
        }

        fn lock(&self) -> Result<File> {
            let metadata = std::fs::symlink_metadata(&self.path)?;
            require(
                metadata.is_file() && !metadata.file_type().is_symlink(),
                "security state must be a regular file, not a symlink",
            )?;
            let mut options = OpenOptions::new();
            options.read(true).write(true);
            #[cfg(unix)]
            {
                use std::os::unix::fs::OpenOptionsExt;
                options.custom_flags(libc::O_NOFOLLOW);
            }
            let file = options.open(&self.path)?;
            fs2::FileExt::try_lock_exclusive(&file)
                .map_err(|_| invalid("security state is busy; retry the operation"))?;
            Ok(file)
        }

        fn read(file: &mut File) -> Result<u32> {
            require(
                file.metadata()?.len() == 28,
                "OPC UA PubSub security state is invalid; provision new keys rather than resetting counters",
            )?;
            let mut record = [0; 28];
            file.read_exact(&mut record)?;
            require(
                &record[..8] == b"FLUADP01" && Sha256::digest(&record[..12])[..16] == record[12..],
                "OPC UA PubSub security state is corrupt; provision new keys rather than resetting counters",
            )?;
            Ok(u32::from_le_bytes(record[8..12].try_into().unwrap()))
        }

        fn write(file: &mut File, sequence: u32) -> Result<()> {
            let mut record = [0; 28];
            record[..8].copy_from_slice(b"FLUADP01");
            record[8..12].copy_from_slice(&sequence.to_le_bytes());
            let checksum = Sha256::digest(&record[..12]);
            record[12..].copy_from_slice(&checksum[..16]);
            file.seek(SeekFrom::Start(0))?;
            file.write_all(&record)?;
            // Persist before sending or delivering the associated message. A partial record fails closed.
            file.sync_all()?;
            Ok(())
        }

        pub async fn next(&self) -> Result<u32> {
            let store = self.clone();
            tokio::task::spawn_blocking(move || {
                let mut file = store.lock()?;
                let last = Self::read(&mut file)?;
                let next = last.checked_add(1).ok_or_else(|| {
                    invalid("security nonce counter exhausted; provision new group keys")
                })?;
                Self::write(&mut file, next)?;
                Ok(next)
            })
            .await
            .map_err(|_| invalid("security state worker failed"))?
        }

        pub async fn accept(&self, sequence: u32) -> Result<bool> {
            let store = self.clone();
            tokio::task::spawn_blocking(move || {
                let mut file = store.lock()?;
                let last = Self::read(&mut file)?;
                // Security counters never expire on idle connections. Peers that reset counters must rotate keys.
                if sequence == 0 || sequence <= last || (last != 0 && sequence - last > 1 << 30) {
                    return Ok(false);
                }
                Self::write(&mut file, sequence)?;
                Ok(true)
            })
            .await
            .map_err(|_| invalid("security state worker failed"))?
        }
    }

    fn private_directory(directory: &Path) -> Result<()> {
        match std::fs::symlink_metadata(directory) {
            Ok(metadata) => require(
                metadata.is_dir() && !metadata.file_type().is_symlink(),
                "security state directory must not be a symlink",
            )?,
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
                let parent = directory
                    .parent()
                    .filter(|p| !p.as_os_str().is_empty())
                    .ok_or_else(|| {
                        invalid("security state directory requires an existing parent")
                    })?;
                if !parent.try_exists()? {
                    private_directory(parent)?;
                }
                let builder = std::fs::DirBuilder::new();
                #[cfg(unix)]
                let builder = {
                    use std::os::unix::fs::DirBuilderExt;
                    let mut builder = builder;
                    builder.mode(0o700);
                    builder
                };
                match builder.create(directory) {
                    Ok(()) => {}
                    Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => {
                        let metadata = std::fs::symlink_metadata(directory)?;
                        require(
                            metadata.is_dir() && !metadata.file_type().is_symlink(),
                            "security state directory must not be a symlink",
                        )?;
                    }
                    Err(error) => return Err(error.into()),
                }
            }
            Err(error) => return Err(error.into()),
        }
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            std::fs::set_permissions(directory, std::fs::Permissions::from_mode(0o700))?;
            File::open(directory)?.sync_all()?;
            if let Some(parent) = directory.parent().filter(|p| !p.as_os_str().is_empty()) {
                // Persist the new directory entry before any counter can authorize a send.
                File::open(parent)?.sync_all()?;
            }
        }
        Ok(())
    }

    fn crypt(keys: &PubSubSecurity, nonce: &[u8; 8], payload: &mut [u8]) -> Result<()> {
        let mut iv = [0; 16];
        iv[..4].copy_from_slice(&keys.key_nonce);
        iv[4..12].copy_from_slice(nonce);
        iv[15] = 1;
        let mut cipher = Aes256Ctr::new_from_slices(&keys.encrypting_key, &iv)
            .map_err(|_| invalid("invalid AES-256-CTR key"))?;
        cipher.apply_keystream(payload);
        Ok(())
    }

    pub fn encode_secure(
        config: &OpcUaPubSubConfig,
        values: &[PubSubValue],
        sequence: u16,
        nonce_sequence: u32,
    ) -> Result<Vec<u8>> {
        let keys = config
            .security
            .as_ref()
            .ok_or_else(|| invalid("security keys are missing"))?;
        let mut bytes = encode(config, values, sequence)?;
        let mut offset = decode_header(config, &bytes)?
            .ok_or_else(|| invalid("invalid publisher header"))?
            .offset;
        if bytes[0] & 0x80 != 0 {
            bytes[1] |= 0x10;
        } else {
            bytes[0] |= 0x80;
            bytes.insert(1, 0x10);
            offset += 1;
        }
        let mut nonce = [0; 8];
        getrandom::fill(&mut nonce[..4])
            .map_err(|_| invalid("security nonce randomness is unavailable"))?;
        nonce[4..].copy_from_slice(&nonce_sequence.to_le_bytes());
        let encrypt = keys.mode == PubSubSecurityMode::SignAndEncrypt;
        let mut header = vec![if encrypt { 3 } else { 1 }];
        header.extend(keys.security_token_id.to_le_bytes());
        header.push(8);
        header.extend(nonce);
        bytes.splice(offset..offset, header);
        let payload_offset = offset + 14;
        require(
            bytes.len() + 32 <= config.max_datagram_bytes,
            "OPC UA PubSub secure message exceeds the datagram limit",
        )?;
        if encrypt {
            crypt(keys, &nonce, &mut bytes[payload_offset..])?;
        }
        let mut mac = HmacSha256::new_from_slice(&keys.signing_key)
            .map_err(|_| invalid("invalid signing key"))?;
        mac.update(&bytes);
        bytes.extend_from_slice(&mac.finalize().into_bytes());
        Ok(bytes)
    }

    pub fn decode_secure(
        config: &OpcUaPubSubConfig,
        bytes: &[u8],
    ) -> Result<Option<PubSubMessage>> {
        let Some(header) = decode_header(config, bytes)? else {
            return Ok(None);
        };
        let keys = config
            .security
            .as_ref()
            .ok_or_else(|| invalid("security keys are missing"))?;
        require(
            header.secure,
            "OPC UA PubSub security downgrade rejected: signed message required",
        )?;
        let mut security = Decoder {
            bytes: &bytes[header.offset..],
        };
        let flags = security.u8()?;
        let expected_flags = if keys.mode == PubSubSecurityMode::SignAndEncrypt {
            3
        } else {
            1
        };
        require(
            flags == expected_flags,
            "OPC UA PubSub security mode mismatch or unsupported security flags",
        )?;
        let token = security.u32()?;
        require(
            token == keys.security_token_id,
            "OPC UA PubSub unknown security token",
        )?;
        require(
            security.u8()? == 8,
            "OPC UA PubSub AES-CTR message nonce must contain 8 bytes",
        )?;
        let nonce = security.array::<8>()?;
        let sequence = u32::from_le_bytes(nonce[4..].try_into().unwrap());
        require(
            sequence != 0,
            "OPC UA PubSub security nonce sequence cannot be zero",
        )?;
        let signature_offset = bytes
            .len()
            .checked_sub(32)
            .ok_or_else(|| invalid("missing security signature"))?;
        let payload_offset = bytes.len() - security.bytes.len();
        require(
            signature_offset >= payload_offset,
            "OPC UA PubSub truncated secure payload",
        )?;
        let mut mac = HmacSha256::new_from_slice(&keys.signing_key)
            .map_err(|_| invalid("invalid signing key"))?;
        mac.update(&bytes[..signature_offset]);
        mac.verify_slice(&bytes[signature_offset..])
            .map_err(|_| invalid("security signature verification failed"))?;
        // Authenticate the complete datagram before decrypting or parsing any DataSet fields.
        let mut payload = zeroize::Zeroizing::new(bytes[payload_offset..signature_offset].to_vec());
        if keys.mode == PubSubSecurityMode::SignAndEncrypt {
            crypt(keys, &nonce, &mut payload)?;
        }
        let mut message = decode_payload(config, &header, &payload)?;
        message.security_token_id = Some(token);
        message.security_sequence_number = Some(sequence);
        Ok(Some(message))
    }

    #[cfg(test)]
    mod tests {
        use super::*;

        struct StateDirectory(PathBuf);
        impl StateDirectory {
            fn new() -> Self {
                let mut random = [0; 16];
                getrandom::fill(&mut random).unwrap();
                let name: String = random.iter().map(|b| format!("{b:02x}")).collect();
                Self(std::env::temp_dir().join(format!("flow-pubsub-security-{name}")))
            }
        }
        impl Drop for StateDirectory {
            fn drop(&mut self) {
                let _ = std::fs::remove_dir_all(&self.0);
            }
        }

        fn config(mode: PubSubSecurityMode) -> OpcUaPubSubConfig {
            let mut config = crate::opcua_pubsub::tests::config();
            config.security = Some(PubSubSecurity {
                mode,
                security_token_id: 42,
                signing_key: vec![11; 32],
                encrypting_key: vec![22; 32],
                key_nonce: vec![1, 2, 3, 4],
            });
            config
        }

        #[test]
        fn secure_codec_authenticates_before_decode_and_rejects_downgrade() {
            for mode in [PubSubSecurityMode::Sign, PubSubSecurityMode::SignAndEncrypt] {
                let config = config(mode);
                let expected = [PubSubValue::Int32(42)];
                let encoded = encode_secure(&config, &expected, 123, 456).unwrap();
                let message = decode_secure(&config, &encoded).unwrap().unwrap();
                assert_eq!(message.fields[0].value, expected[0]);
                assert_eq!(message.security_token_id, Some(42));
                assert_eq!(message.security_sequence_number, Some(456));
                let header = decode_header(&config, &encoded).unwrap().unwrap();
                // Header identities may legitimately filter a message. Every byte from the
                // security header through the signature must reject tampering on matching traffic.
                for index in header.offset..encoded.len() {
                    let mut tampered = encoded.clone();
                    tampered[index] ^= 0x80;
                    assert!(
                        decode_secure(&config, &tampered).is_err(),
                        "tampered byte {index}"
                    );
                }
                for count in 0..encoded.len() {
                    assert!(decode_secure(&config, &encoded[..count]).is_err());
                }
                let mut wrong = config.clone();
                wrong.security.as_mut().unwrap().signing_key[0] ^= 1;
                assert!(
                    decode_secure(&wrong, &encoded)
                        .unwrap_err()
                        .to_string()
                        .contains("signature")
                );
                wrong = config.clone();
                wrong.security.as_mut().unwrap().security_token_id += 1;
                assert!(
                    decode_secure(&wrong, &encoded)
                        .unwrap_err()
                        .to_string()
                        .contains("token")
                );
                wrong = config.clone();
                wrong.security.as_mut().unwrap().mode = if mode == PubSubSecurityMode::Sign {
                    PubSubSecurityMode::SignAndEncrypt
                } else {
                    PubSubSecurityMode::Sign
                };
                assert!(decode_secure(&wrong, &encoded).is_err());
                assert!(decode_secure(&config, &encode(&config, &expected, 0).unwrap()).is_err());
                assert!(decode(&crate::opcua_pubsub::tests::config(), &encoded).is_err());
            }
        }

        #[test]
        fn secure_encoding_supports_byte_and_string_publisher_ids() {
            for publisher in [
                PublisherId::Byte(7),
                PublisherId::String("secured-writer".into()),
            ] {
                let mut config = config(PubSubSecurityMode::SignAndEncrypt);
                config.writer.publisher_id = publisher;
                let bytes = encode_secure(&config, &[PubSubValue::Int32(42)], 0, 1).unwrap();
                assert_eq!(
                    decode_secure(&config, &bytes).unwrap().unwrap().fields[0].value,
                    PubSubValue::Int32(42)
                );
            }
        }

        #[test]
        fn security_configuration_validates_and_redacts_keys() {
            let config = config(PubSubSecurityMode::SignAndEncrypt);
            config.validate().unwrap();
            let debug = format!("{config:?}");
            assert!(debug.contains("[redacted]"));
            assert!(!debug.contains("11, 11"));
            assert_eq!(
                serde_json::to_value(&config).unwrap()["security"]["mode"],
                "sign_and_encrypt"
            );
            for size in [0, 16, 31, 33] {
                let mut invalid = config.clone();
                invalid
                    .security
                    .as_mut()
                    .unwrap()
                    .signing_key
                    .resize(size, 0);
                assert!(invalid.validate().is_err());
            }
            let mut invalid = config.clone();
            invalid.security.as_mut().unwrap().security_token_id = 0;
            assert!(invalid.validate().is_err());
        }

        #[tokio::test]
        async fn nonce_state_survives_restart_and_token_changes_and_fails_closed() {
            let directory = StateDirectory::new();
            let mut config = config(PubSubSecurityMode::SignAndEncrypt);
            config.mode = PubSubMode::Publisher;
            let store = CounterStore::open(&config, &directory.0).await.unwrap();
            assert_eq!(store.next().await.unwrap(), 1);
            assert_eq!(store.next().await.unwrap(), 2);
            config.security.as_mut().unwrap().security_token_id += 1;
            let reconnected = CounterStore::open(&config, &directory.0).await.unwrap();
            assert_eq!(reconnected.next().await.unwrap(), 3);
            // Separate sessions with the same key share the durable high-water counter.
            assert_eq!(store.next().await.unwrap(), 4);
            let mut file = store.lock().unwrap();
            CounterStore::write(&mut file, u32::MAX).unwrap();
            drop(file);
            assert!(
                store
                    .next()
                    .await
                    .unwrap_err()
                    .to_string()
                    .contains("exhausted")
            );
            std::fs::write(&store.path, b"truncated").unwrap();
            assert!(CounterStore::open(&config, &directory.0).await.is_err());
            assert!(store.next().await.is_err());
            config.security.as_mut().unwrap().encrypting_key[0] ^= 1;
            assert_eq!(
                CounterStore::open(&config, &directory.0)
                    .await
                    .unwrap()
                    .next()
                    .await
                    .unwrap(),
                1
            );
        }

        #[cfg(unix)]
        #[tokio::test]
        async fn state_directory_is_private_and_symlink_counters_are_rejected() {
            use std::os::unix::fs::{PermissionsExt, symlink};
            let directory = StateDirectory::new();
            let nested = directory.0.join("nested");
            let config = config(PubSubSecurityMode::SignAndEncrypt);
            let store = CounterStore::open(&config, &nested).await.unwrap();
            assert_eq!(
                std::fs::metadata(&nested).unwrap().permissions().mode() & 0o777,
                0o700
            );
            let moved = store.path.with_extension("moved");
            std::fs::rename(&store.path, &moved).unwrap();
            symlink(&moved, &store.path).unwrap();
            assert!(CounterStore::open(&config, &nested).await.is_err());
            assert!(store.accept(1).await.is_err());
        }

        #[tokio::test]
        async fn replay_state_survives_restart_and_rejects_old_or_ambiguous_counters() {
            let directory = StateDirectory::new();
            let config = config(PubSubSecurityMode::SignAndEncrypt);
            let store = CounterStore::open(&config, &directory.0).await.unwrap();
            assert!(store.accept(10).await.unwrap());
            assert!(!store.accept(10).await.unwrap());
            assert!(!store.accept(9).await.unwrap());
            assert!(!store.accept(0).await.unwrap());
            assert!(!store.accept((1 << 30) + 11).await.unwrap());
            assert!(CounterStore::open(&config, &directory.0).await.is_err());
            let independent = StateDirectory::new();
            let second_reader = CounterStore::open(&config, &independent.0).await.unwrap();
            assert!(second_reader.accept(10).await.unwrap());
            store.close().unwrap();
            let reconnected = CounterStore::open(&config, &directory.0).await.unwrap();
            assert!(!reconnected.accept(10).await.unwrap());
            assert!(reconnected.accept(11).await.unwrap());
        }
    }
}

#[cfg(feature = "opcua-pubsub")]
pub use runtime::OpcUaPubSubClient;

#[cfg(feature = "opcua-pubsub")]
mod runtime {
    use super::*;
    use socket2::{Domain, Protocol, Socket, Type};
    use std::{
        sync::{Arc, RwLock},
        time::Duration,
    };
    use tokio::{net::UdpSocket, sync::Mutex};
    use tokio_util::sync::CancellationToken;

    #[derive(Default)]
    struct SequenceState {
        last: Option<(Option<u16>, Option<u16>, tokio::time::Instant)>,
    }

    impl SequenceState {
        fn accept(
            &mut self,
            message: &PubSubMessage,
            now: tokio::time::Instant,
            expiry: Duration,
        ) -> bool {
            if let Some((dataset, network, at)) = self.last {
                // Receiving an old frame is still traffic, so it must not look like a publisher outage.
                self.last = Some((dataset, network, now));
                if now.duration_since(at) < expiry {
                    // Part 14 §7.2.3 excludes duplicate, older and ambiguous sequence ranges.
                    for (new, last) in [
                        (message.sequence_number, dataset),
                        (message.network_sequence_number, network),
                    ] {
                        if let (Some(new), Some(last)) = (new, last)
                            && new.wrapping_sub(1).wrapping_sub(last) >= 16_384
                        {
                            return false;
                        }
                    }
                }
            }
            self.last = Some((
                message.sequence_number,
                message.network_sequence_number,
                now,
            ));
            true
        }
    }

    pub struct OpcUaPubSubClient {
        config: OpcUaPubSubConfig,
        socket: RwLock<Option<Arc<UdpSocket>>>,
        cancelled: CancellationToken,
        sequence: Mutex<u16>,
        security_store: Option<security::CounterStore>,
        receive_lock: Mutex<SequenceState>,
    }

    impl OpcUaPubSubClient {
        /// Binding creates no worker. Each receive call reads one matching DataSet under its deadline.
        pub async fn connect(config: OpcUaPubSubConfig) -> Result<Self> {
            require(
                config.security.is_none(),
                "Secure OPC UA PubSub requires connect_with_security_store and persistent local state",
            )?;
            Self::open(config, None).await
        }

        /// Preserve this directory across restarts. Lost, restored or cloned state requires new group keys.
        /// Separate executor stores must use independent publisher key material.
        /// Each logical subscriber needs its own directory and permits one active reader per writer/key pair.
        pub async fn connect_with_security_store(
            config: OpcUaPubSubConfig,
            directory: &std::path::Path,
        ) -> Result<Self> {
            config.validate()?;
            let store = if config.security.is_some() {
                Some(security::CounterStore::open(&config, directory).await?)
            } else {
                None
            };
            Self::open(config, store).await
        }

        async fn open(
            config: OpcUaPubSubConfig,
            security_store: Option<security::CounterStore>,
        ) -> Result<Self> {
            config.validate()?;
            let destination = config.destination()?;
            let interface: Ipv4Addr = config
                .interface
                .parse()
                .map_err(|_| invalid("invalid interface"))?;
            let socket = Socket::new(Domain::IPV4, Type::DGRAM, Some(Protocol::UDP))?;
            if destination.ip().is_multicast() && config.mode == PubSubMode::Subscriber {
                socket.set_reuse_address(true)?;
                // Linux otherwise receives traffic for every group joined on this port by any socket.
                #[cfg(target_os = "linux")]
                socket.set_multicast_all_v4(false)?;
            }
            let bind = match config.mode {
                PubSubMode::Publisher => SocketAddrV4::new(interface, 0),
                PubSubMode::Subscriber if destination.ip().is_multicast() => {
                    SocketAddrV4::new(Ipv4Addr::UNSPECIFIED, destination.port())
                }
                PubSubMode::Subscriber => destination,
            };
            socket.bind(&bind.into())?;
            if destination.ip().is_multicast() {
                socket.set_multicast_if_v4(&interface)?;
                socket.set_multicast_loop_v4(true)?;
                socket.set_multicast_ttl_v4(1)?;
                if config.mode == PubSubMode::Subscriber {
                    socket.join_multicast_v4(destination.ip(), &interface)?;
                }
            }
            socket.set_nonblocking(true)?;
            let socket = UdpSocket::from_std(socket.into())?;
            Ok(Self {
                config,
                socket: RwLock::new(Some(Arc::new(socket))),
                cancelled: CancellationToken::new(),
                sequence: Mutex::new(0),
                security_store,
                receive_lock: Mutex::new(SequenceState::default()),
            })
        }

        fn socket(&self) -> Result<Arc<UdpSocket>> {
            self.socket
                .read()
                .map_err(|_| invalid("socket lock poisoned"))?
                .clone()
                .ok_or_else(|| invalid("session is closed"))
        }

        /// Success means a datagram was sent; UDP provides no delivery acknowledgement.
        pub async fn publish(&self, values: &[PubSubValue]) -> Result<u16> {
            require(
                self.config.mode == PubSubMode::Publisher,
                "OPC UA PubSub session is not a publisher",
            )?;
            let socket = self.socket()?;
            let operation = async {
                let mut sequence = self.sequence.lock().await;
                let bytes = if let Some(store) = &self.security_store {
                    let nonce_sequence = store.next().await?;
                    security::encode_secure(&self.config, values, *sequence, nonce_sequence)?
                } else {
                    encode(&self.config, values, *sequence)?
                };
                let sent = socket.send_to(&bytes, self.config.destination()?).await?;
                require(
                    sent == bytes.len(),
                    "OPC UA PubSub incomplete UDP datagram send",
                )?;
                let result = *sequence;
                *sequence = sequence.wrapping_add(1);
                Ok(result)
            };
            tokio::select! {
                biased;
                _ = self.cancelled.cancelled() => Err(invalid("session is closed")),
                result = tokio::time::timeout(Duration::from_millis(self.config.timeout_ms), operation) => result.map_err(|_| Error::Timeout)?,
            }
        }

        /// Unrelated writer IDs are skipped without extending the deadline. Malformed matching data fails.
        pub async fn receive(&self) -> Result<PubSubMessage> {
            require(
                self.config.mode == PubSubMode::Subscriber,
                "OPC UA PubSub session is not a subscriber",
            )?;
            let socket = self.socket()?;
            let operation = async {
                let mut received = self.receive_lock.lock().await;
                let mut buffer = vec![0; self.config.max_datagram_bytes + 1];
                loop {
                    let (count, _) = socket.recv_from(&mut buffer).await?;
                    let message = if self.security_store.is_some() {
                        security::decode_secure(&self.config, &buffer[..count])?
                    } else {
                        decode(&self.config, &buffer[..count])?
                    };
                    if let Some(message) = message {
                        if let Some(store) = &self.security_store {
                            if store
                                .accept(
                                    message
                                        .security_sequence_number
                                        .ok_or_else(|| invalid("security counter missing"))?,
                                )
                                .await?
                                && received.accept(
                                    &message,
                                    tokio::time::Instant::now(),
                                    Duration::from_millis(self.config.timeout_ms * 2),
                                )
                            {
                                return Ok(message);
                            }
                        } else if received.accept(
                            &message,
                            tokio::time::Instant::now(),
                            Duration::from_millis(self.config.timeout_ms * 2),
                        ) {
                            return Ok(message);
                        }
                    }
                }
            };
            tokio::select! {
                biased;
                _ = self.cancelled.cancelled() => Err(invalid("session is closed")),
                result = tokio::time::timeout(Duration::from_millis(self.config.timeout_ms), operation) => result.map_err(|_| Error::Timeout)?,
            }
        }

        pub async fn close(&self) -> Result<()> {
            self.cancelled.cancel();
            self.socket
                .write()
                .map_err(|_| invalid("socket lock poisoned"))?
                .take();
            if let Some(store) = &self.security_store {
                store.close()?;
            }
            Ok(())
        }
    }

    impl Drop for OpcUaPubSubClient {
        fn drop(&mut self) {
            self.cancelled.cancel();
        }
    }

    #[cfg(test)]
    mod sequence_tests {
        use super::*;
        #[test]
        fn wraparound_duplicates_ambiguous_jumps_and_restart() {
            let mut message = PubSubMessage {
                writer: PubSubWriter {
                    publisher_id: PublisherId::Byte(1),
                    writer_group_id: 1,
                    data_set_writer_id: 1,
                },
                sequence_number: Some(65535),
                network_sequence_number: None,
                configuration_major_version: None,
                configuration_minor_version: None,
                timestamp: None,
                status: None,
                security_token_id: None,
                security_sequence_number: None,
                fields: vec![],
            };
            let mut state = SequenceState::default();
            let now = tokio::time::Instant::now();
            let expiry = Duration::from_millis(200);
            assert!(state.accept(&message, now, expiry));
            assert!(!state.accept(&message, now, expiry));
            message.sequence_number = Some(0);
            assert!(state.accept(&message, now, expiry));
            message.sequence_number = Some(65535);
            assert!(!state.accept(&message, now, expiry));
            message.sequence_number = Some(16385);
            assert!(!state.accept(&message, now, expiry));
            message.sequence_number = Some(0);
            for ms in [100, 200, 300, 400, 500] {
                assert!(!state.accept(&message, now + Duration::from_millis(ms), expiry));
            }
            assert!(state.accept(&message, now + Duration::from_millis(700), expiry));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    pub(super) fn config() -> OpcUaPubSubConfig {
        OpcUaPubSubConfig {
            address: "opc.udp://127.0.0.1:4841".into(),
            interface: "127.0.0.1".into(),
            mode: PubSubMode::Subscriber,
            writer: PubSubWriter {
                publisher_id: PublisherId::UInt16(2234),
                writer_group_id: 100,
                data_set_writer_id: 62541,
            },
            fields: vec![PubSubFieldMetaData {
                name: "temperature".into(),
                data_type: PubSubDataType::Int32,
            }],
            configuration_version: None,
            timeout_ms: 100,
            max_datagram_bytes: 1400,
            security: None,
        }
    }

    #[test]
    fn decodes_standard_key_frame_without_optional_sequence_or_versions() {
        // Part 14 field layout also used by the open62541 publish tutorial.
        let packet = [
            0xf1, 1, 0xba, 8, 1, 100, 0, 1, 0x4d, 0xf4, 1, 1, 0, 6, 42, 0, 0, 0,
        ];
        let result = decode(&config(), &packet).unwrap().unwrap();
        assert_eq!(result.fields[0].value, PubSubValue::Int32(42));
        assert_eq!(result.sequence_number, None);
    }

    #[test]
    fn boolean_decoding_accepts_every_nonzero_byte() {
        for byte in [1, 2, 255] {
            assert_eq!(
                Decoder { bytes: &[1, byte] }.value().unwrap(),
                PubSubValue::Boolean(true)
            );
        }
    }

    #[test]
    fn serialized_type_names_match_the_public_configuration() {
        assert_eq!(
            serde_json::to_value(PublisherId::UInt32(2234)).unwrap(),
            serde_json::json!({"type": "uint32", "value": 2234})
        );
        assert_eq!(
            serde_json::to_value(PubSubDataType::UInt16).unwrap(),
            "uint16"
        );
        assert_eq!(
            serde_json::to_value(PubSubValue::UInt64(42)).unwrap(),
            serde_json::json!({"type": "uint64", "value": 42})
        );
    }

    #[test]
    fn unrelated_unsupported_packets_do_not_poison_the_reader() {
        let cfg = config();
        let packet = encode(&cfg, &[PubSubValue::Int32(42)], 0).unwrap();
        let mut different_publisher = packet.clone();
        different_publisher[1] |= 0x80;
        different_publisher.insert(2, 0x04); // Discovery response from another publisher.
        different_publisher[3] ^= 1;
        assert!(decode(&cfg, &different_publisher).unwrap().is_none());
        let mut different_group = packet.clone();
        different_group[1] |= 0x10; // Security enabled in another writer group.
        different_group[5] ^= 1;
        assert!(decode(&cfg, &different_group).unwrap().is_none());
        let mut different_writer = packet;
        different_writer[1] |= 0x10;
        different_writer[10] ^= 1;
        assert!(decode(&cfg, &different_writer).unwrap().is_none());
    }

    #[test]
    fn all_scalar_types_and_publisher_types_round_trip() {
        let values = vec![
            PubSubValue::Boolean(true),
            PubSubValue::SByte(-128),
            PubSubValue::Byte(255),
            PubSubValue::Int16(i16::MIN),
            PubSubValue::UInt16(u16::MAX),
            PubSubValue::Int32(i32::MIN),
            PubSubValue::UInt32(u32::MAX),
            PubSubValue::Int64(i64::MIN),
            PubSubValue::UInt64(u64::MAX),
            PubSubValue::Float(-1.25),
            PubSubValue::Double(2.125),
            PubSubValue::String(Some("温度".into())),
            PubSubValue::DateTime(133_000_000_000_000_000),
            PubSubValue::ByteString(Some(vec![0, 255, 128])),
            PubSubValue::StatusCode(0),
            PubSubValue::String(None),
            PubSubValue::ByteString(None),
        ];
        for id in [
            PublisherId::Byte(7),
            PublisherId::UInt16(2234),
            PublisherId::UInt32(u32::MAX),
            PublisherId::UInt64(u64::MAX),
            PublisherId::String("機械".into()),
        ] {
            let mut cfg = config();
            cfg.writer.publisher_id = id;
            cfg.fields = values
                .iter()
                .enumerate()
                .map(|(i, v)| PubSubFieldMetaData {
                    name: format!("field_{i}"),
                    data_type: v.data_type(),
                })
                .collect();
            cfg.configuration_version = Some(ConfigurationVersion { major: 5, minor: 2 });
            let result = decode(&cfg, &encode(&cfg, &values, u16::MAX).unwrap())
                .unwrap()
                .unwrap();
            assert_eq!(
                result
                    .fields
                    .iter()
                    .map(|f| f.value.clone())
                    .collect::<Vec<_>>(),
                values
            );
            assert_eq!(result.sequence_number, Some(u16::MAX));
            assert_eq!(result.configuration_major_version, Some(5));
        }
    }

    #[test]
    fn bounds_and_metadata_are_enforced_before_allocation() {
        let cfg = config();
        let packet = encode(&cfg, &[PubSubValue::Int32(42)], 0).unwrap();
        for count in 0..packet.len() {
            assert!(
                decode(&cfg, &packet[..count]).is_err(),
                "truncation at {count}"
            );
        }
        let mut wrong = cfg.clone();
        wrong.fields[0].data_type = PubSubDataType::Boolean;
        assert!(decode(&wrong, &packet).is_err());
        wrong = cfg.clone();
        wrong.configuration_version = Some(ConfigurationVersion { major: 2, minor: 1 });
        assert!(decode(&wrong, &packet).is_err());
        wrong = cfg.clone();
        wrong.max_datagram_bytes = packet.len() - 1;
        assert!(decode(&wrong, &packet).is_err());
        let mut d = Decoder {
            bytes: &[255, 255, 255, 127],
        };
        assert!(d.bytes().is_err());
        assert!(encode(&cfg, &[PubSubValue::Float(f32::NAN)], 0).is_err());
    }

    #[test]
    fn filters_exact_typed_ids_and_rejects_unsupported_frames() {
        let cfg = config();
        let packet = encode(&cfg, &[PubSubValue::Int32(42)], 0).unwrap();
        for writer in [
            PubSubWriter {
                publisher_id: PublisherId::UInt32(2234),
                ..cfg.writer.clone()
            },
            PubSubWriter {
                writer_group_id: 101,
                ..cfg.writer.clone()
            },
            PubSubWriter {
                data_set_writer_id: 1,
                ..cfg.writer.clone()
            },
        ] {
            let mut other = cfg.clone();
            other.writer = writer;
            assert!(decode(&other, &packet).unwrap().is_none());
        }
        let mut secure = packet.clone();
        secure[1] |= 0x10;
        assert!(
            decode(&cfg, &secure)
                .unwrap_err()
                .to_string()
                .contains("signed")
        );
        let mut raw = packet.clone();
        raw[12] |= 2;
        assert!(
            decode(&cfg, &raw)
                .unwrap_err()
                .to_string()
                .contains("Variant")
        );
        let mut delta = packet.clone();
        delta[12] |= 0x80;
        delta.insert(13, 1);
        assert!(
            decode(&cfg, &delta)
                .unwrap_err()
                .to_string()
                .contains("key frames")
        );
        let mut array = packet.clone();
        array[17] |= 0x80;
        assert!(decode(&cfg, &array).is_err());
    }

    #[test]
    fn validates_versions_and_bad_status_without_fabricating_values() {
        let mut cfg = config();
        cfg.configuration_version = Some(ConfigurationVersion {
            major: 10,
            minor: 3,
        });
        let packet = encode(&cfg, &[PubSubValue::StatusCode(0x803a_0000)], 3).unwrap();
        let result = decode(&cfg, &packet).unwrap().unwrap();
        assert_eq!(result.fields[0].value, PubSubValue::StatusCode(0x803a_0000));
        cfg.configuration_version.as_mut().unwrap().major = 11;
        assert!(
            decode(&cfg, &packet)
                .unwrap_err()
                .to_string()
                .contains("version")
        );
        assert!(encode(&cfg, &[PubSubValue::StatusCode(0)], 4).is_err());
    }

    #[test]
    fn decodes_selected_dataset_from_a_batched_network_message() {
        let cfg = config();
        let single = encode(&cfg, &[PubSubValue::Int32(42)], 9).unwrap();
        let dataset = &single[12..];
        let mut batch = single[..9].to_vec();
        batch.push(2);
        batch.extend(1_u16.to_le_bytes());
        batch.extend(cfg.writer.data_set_writer_id.to_le_bytes());
        batch.extend((dataset.len() as u16).to_le_bytes());
        batch.extend((dataset.len() as u16).to_le_bytes());
        batch.extend(dataset);
        batch.extend(dataset);
        assert_eq!(
            decode(&cfg, &batch).unwrap().unwrap().fields[0].value,
            PubSubValue::Int32(42)
        );
        batch.push(0);
        assert!(decode(&cfg, &batch).is_err());
    }

    #[cfg(feature = "opcua-pubsub")]
    #[tokio::test]
    async fn duplicate_traffic_never_extends_deadlines_or_resets_the_sequence() {
        let port = std::net::UdpSocket::bind("127.0.0.1:0")
            .unwrap()
            .local_addr()
            .unwrap()
            .port();
        let mut cfg = config();
        cfg.address = format!("opc.udp://127.0.0.1:{port}");
        cfg.timeout_ms = 80;
        let subscriber = OpcUaPubSubClient::connect(cfg.clone()).await.unwrap();
        let sender = tokio::net::UdpSocket::bind("127.0.0.1:0").await.unwrap();
        let packet = encode(&cfg, &[PubSubValue::Int32(42)], 0).unwrap();
        sender
            .send_to(&packet, cfg.destination().unwrap())
            .await
            .unwrap();
        subscriber.receive().await.unwrap();
        let spam = tokio::spawn(async move {
            let mut interval = tokio::time::interval(std::time::Duration::from_millis(5));
            loop {
                interval.tick().await;
                sender
                    .send_to(&packet, cfg.destination().unwrap())
                    .await
                    .unwrap();
            }
        });
        for _ in 0..3 {
            assert!(matches!(
                tokio::time::timeout(std::time::Duration::from_millis(300), subscriber.receive())
                    .await
                    .unwrap(),
                Err(Error::Timeout)
            ));
        }
        spam.abort();
        let _ = spam.await;
        subscriber.close().await.unwrap();
    }

    #[cfg(all(feature = "opcua-pubsub", target_os = "linux"))]
    #[tokio::test]
    async fn multicast_subscription_is_isolated_to_its_configured_group() {
        let port = std::net::UdpSocket::bind("127.0.0.1:0")
            .unwrap()
            .local_addr()
            .unwrap()
            .port();
        let mut cfg = config();
        cfg.address = format!("opc.udp://239.255.42.71:{port}");
        let group_a = OpcUaPubSubClient::connect(cfg.clone()).await.unwrap();
        cfg.address = format!("opc.udp://239.255.42.72:{port}");
        let group_b = OpcUaPubSubClient::connect(cfg.clone()).await.unwrap();
        cfg.mode = PubSubMode::Publisher;
        let publisher_b = OpcUaPubSubClient::connect(cfg).await.unwrap();
        publisher_b
            .publish(&[PubSubValue::Int32(42)])
            .await
            .unwrap();
        assert_eq!(
            group_b.receive().await.unwrap().fields[0].value,
            PubSubValue::Int32(42)
        );
        assert!(matches!(group_a.receive().await, Err(Error::Timeout)));
    }

    #[cfg(feature = "opcua-pubsub")]
    #[tokio::test]
    async fn udp_deadlines_close_and_rebind_release_resources() {
        use std::sync::Arc;
        let port = std::net::UdpSocket::bind("127.0.0.1:0")
            .unwrap()
            .local_addr()
            .unwrap()
            .port();
        let mut cfg = config();
        cfg.address = format!("opc.udp://127.0.0.1:{port}");
        let subscriber = Arc::new(OpcUaPubSubClient::connect(cfg.clone()).await.unwrap());
        assert!(matches!(subscriber.receive().await, Err(Error::Timeout)));
        let waiting = {
            let subscriber = subscriber.clone();
            tokio::spawn(async move { subscriber.receive().await })
        };
        tokio::task::yield_now().await;
        subscriber.close().await.unwrap();
        assert!(
            tokio::time::timeout(std::time::Duration::from_millis(200), waiting)
                .await
                .unwrap()
                .unwrap()
                .is_err()
        );
        assert!(subscriber.receive().await.is_err());
        let reconnected = OpcUaPubSubClient::connect(cfg.clone()).await.unwrap();
        cfg.mode = PubSubMode::Publisher;
        let publisher = OpcUaPubSubClient::connect(cfg).await.unwrap();
        assert_eq!(
            publisher.publish(&[PubSubValue::Int32(42)]).await.unwrap(),
            0
        );
        assert_eq!(
            reconnected.receive().await.unwrap().fields[0].value,
            PubSubValue::Int32(42)
        );
        publisher.close().await.unwrap();
        assert!(publisher.publish(&[PubSubValue::Int32(43)]).await.is_err());
    }
}
