use alloc::{sync::Arc, vec::Vec};
use dust_dds::xtypes::type_support::TypeSupport;

pub type Octet = u8;
pub type Short = i16;
pub type Long = i32;
pub type UnsignedLong = u32;
pub type OctetArray3 = [Octet; 3];

// Table 9.1 - entityKind octet of an EntityId_t
pub const USER_DEFINED_UNKNOWN: Octet = 0x00;
#[allow(dead_code)]
pub const BUILT_IN_UNKNOWN: Octet = 0xc0;
pub const BUILT_IN_PARTICIPANT: Octet = 0xc1;
pub const USER_DEFINED_WRITER_WITH_KEY: Octet = 0x02;
pub const BUILT_IN_WRITER_WITH_KEY: Octet = 0xc2;
pub const USER_DEFINED_WRITER_NO_KEY: Octet = 0x03;
#[allow(dead_code)]
pub const BUILT_IN_WRITER_NO_KEY: Octet = 0xc3;
pub const USER_DEFINED_READER_WITH_KEY: Octet = 0x07;
pub const BUILT_IN_READER_WITH_KEY: Octet = 0xc7;
pub const USER_DEFINED_READER_NO_KEY: Octet = 0x04;
#[allow(dead_code)]
pub const BUILT_IN_READER_NO_KEY: Octet = 0xc4;
pub const USER_DEFINED_WRITER_GROUP: Octet = 0x08;
pub const BUILT_IN_WRITER_GROUP: Octet = 0xc8;
pub const USER_DEFINED_READER_GROUP: Octet = 0x09;
pub const BUILT_IN_READER_GROUP: Octet = 0xc9;
// Added in comparison to the RTPS standard
pub const USER_DEFINED_TOPIC: Octet = 0x0a;

/// GUID_t
/// Type used to hold globally-unique RTPS-entity identifiers. These are identifiers used to uniquely refer to each RTPS Entity in the system.
/// Must be possible to represent using 16 octets.
/// The following values are reserved by the protocol: GUID_UNKNOWN
#[derive(Clone, Copy, PartialEq, Eq, Debug, TypeSupport)]
#[dust_dds(extensibility = "final", nested)]
pub struct Guid {
    prefix: GuidPrefix,
    entity_id: EntityId,
}

impl Guid {
    pub const fn new(prefix: GuidPrefix, entity_id: EntityId) -> Self {
        Self { prefix, entity_id }
    }

    pub const fn prefix(&self) -> GuidPrefix {
        self.prefix
    }

    pub const fn entity_id(&self) -> EntityId {
        self.entity_id
    }
}

impl From<[u8; 16]> for Guid {
    fn from(value: [u8; 16]) -> Self {
        let prefix = [
            value[0], value[1], value[2], value[3], value[4], value[5], value[6], value[7],
            value[8], value[9], value[10], value[11],
        ];
        let entity_id = EntityId::new([value[12], value[13], value[14]], value[15]);
        Self { prefix, entity_id }
    }
}

impl From<Guid> for [u8; 16] {
    fn from(guid: Guid) -> Self {
        [
            guid.prefix[0],
            guid.prefix[1],
            guid.prefix[2],
            guid.prefix[3],
            guid.prefix[4],
            guid.prefix[5],
            guid.prefix[6],
            guid.prefix[7],
            guid.prefix[8],
            guid.prefix[9],
            guid.prefix[10],
            guid.prefix[11],
            guid.entity_id.entity_key[0],
            guid.entity_id.entity_key[1],
            guid.entity_id.entity_key[2],
            guid.entity_id.entity_kind,
        ]
    }
}

impl PartialEq<[u8; 16]> for Guid {
    #[inline]
    fn eq(&self, other: &[u8; 16]) -> bool {
        self.prefix == other[..12]
            && other[12..]
                .try_into()
                .is_ok_and(|entity_id: [u8; 4]| self.entity_id == entity_id)
    }
}

impl PartialEq<Guid> for [u8; 16] {
    #[inline]
    fn eq(&self, other: &Guid) -> bool {
        other.eq(self)
    }
}

pub const GUID_UNKNOWN: Guid = Guid::new(GUIDPREFIX_UNKNOWN, ENTITYID_UNKNOWN);

/// GuidPrefix_t
/// Type used to hold the prefix of the globally-unique RTPS-entity identifiers. The GUIDs of entities belonging to the same participant all have the same prefix (see 8.2.4.3).
/// Must be possible to represent using 12 octets.
/// The following values are reserved by the protocol: GUIDPREFIX_UNKNOWN
pub type GuidPrefix = [u8; 12];
pub const GUIDPREFIX_UNKNOWN: GuidPrefix = [0; 12];

/// EntityId_t
/// Type used to hold the suffix part of the globally-unique RTPS-entity identifiers. The
/// EntityId_t uniquely identifies an Entity within a Participant. Must be possible to represent using 4 octets.
/// The following values are reserved by the protocol: ENTITYID_UNKNOWN Additional pre-defined values are defined by the Discovery module in 8.5
#[derive(Clone, Copy, PartialEq, Eq, Debug, TypeSupport)]
#[dust_dds(extensibility = "final", nested)]
pub struct EntityId {
    pub(crate) entity_key: OctetArray3,
    pub(crate) entity_kind: Octet,
}

impl EntityId {
    #[inline]
    pub const fn new(entity_key: OctetArray3, entity_kind: Octet) -> Self {
        Self {
            entity_key,
            entity_kind,
        }
    }

    #[inline]
    pub const fn entity_key(&self) -> OctetArray3 {
        self.entity_key
    }

    #[inline]
    pub const fn entity_kind(&self) -> Octet {
        self.entity_kind
    }
}

impl Default for EntityId {
    #[inline]
    fn default() -> Self {
        ENTITYID_UNKNOWN
    }
}

impl PartialEq<[u8; 4]> for EntityId {
    #[inline]
    fn eq(&self, other: &[u8; 4]) -> bool {
        self.entity_key == other[..3] && self.entity_kind == other[3]
    }
}

impl PartialEq<EntityId> for [u8; 4] {
    #[inline]
    fn eq(&self, other: &EntityId) -> bool {
        other.eq(self)
    }
}

pub const ENTITYID_UNKNOWN: EntityId = EntityId::new([0; 3], USER_DEFINED_UNKNOWN);
pub const ENTITYID_PARTICIPANT: EntityId = EntityId::new([0, 0, 0x01], BUILT_IN_PARTICIPANT);

/// SequenceNumber_t
/// Type used to hold sequence numbers.
/// Must be possible to represent using 64 bits.
pub type SequenceNumber = i64;

/// TopicKind_t
/// Enumeration used to distinguish whether a Topic has defined some fields within to be used as the 'key' that identifies data-instances within the Topic. See the DDS specification for more details on keys.
/// The following values are reserved by the protocol: NO_KEY, WITH_KEY
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum TopicKind {
    NoKey,
    WithKey,
}

/// ChangeKind_t
/// Enumeration used to distinguish the kind of change that was made to a data-object. Includes changes to the data or the instance state of the data-object.
/// It can take the values:
/// ALIVE, ALIVE_FILTERED, NOT_ALIVE_DISPOSED, NOT_ALIVE_UNREGISTERED
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
#[allow(dead_code)]
pub enum ChangeKind {
    Alive,
    AliveFiltered,
    NotAliveDisposed,
    NotAliveUnregistered,
    NotAliveDisposedUnregistered,
}

/// ReliabilityKind_t
/// Enumeration used to indicate the level of the reliability used for communications.
/// It can take the values: BEST_EFFORT, RELIABLE.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ReliabilityKind {
    BestEffort,
    Reliable,
}

/// DurabilityKind_t
/// Enumeration used to indicate the level of the durability used for communications.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DurabilityKind {
    Volatile,
    TransientLocal,
    Transient,
    Persistent,
}

/// Locator_t
/// Type used to represent the addressing information needed to send a message to an RTPS Endpoint using one of the supported transports.
/// Should be able to hold a discriminator identifying the kind of transport, an address, and a port number. It must be possible to represent the discriminator and port number using 4 octets each, the address using 16 octets.
/// The following values are reserved by the protocol: LOCATOR_INVALID LOCATOR_KIND_INVALID LOCATOR_KIND_RESERVED LOCATOR_KIND_UDP_V4 LOCATOR_KIND_UDP_V6 LOCATOR_ADDRESS_INVALID LOCATOR_PORT_INVALID
#[derive(Clone, Copy, PartialEq, Eq, Debug, TypeSupport)]
#[dust_dds(extensibility = "final", nested)]
pub struct Locator {
    pub(crate) kind: Long,
    pub(crate) port: UnsignedLong,
    pub(crate) address: [Octet; 16],
}

#[allow(dead_code)]
pub const LOCATOR_KIND_INVALID: Long = -1;
#[allow(dead_code)]
pub const LOCATOR_KIND_RESERVED: Long = 0;
pub const LOCATOR_KIND_UDP_V4: Long = 1;
pub const LOCATOR_KIND_UDP_V6: Long = 2;
pub const LOCATOR_PORT_INVALID: UnsignedLong = 0;
pub const LOCATOR_ADDRESS_INVALID: [Octet; 16] = [0; 16];

#[allow(dead_code)]
pub const LOCATOR_INVALID: Locator = Locator::new(
    LOCATOR_KIND_INVALID,
    LOCATOR_PORT_INVALID,
    LOCATOR_ADDRESS_INVALID,
);

impl Locator {
    pub const fn new(kind: Long, port: UnsignedLong, address: [Octet; 16]) -> Self {
        Self {
            kind,
            port,
            address,
        }
    }
    pub const fn kind(&self) -> Long {
        self.kind
    }
    pub const fn port(&self) -> UnsignedLong {
        self.port
    }
    pub const fn address(&self) -> [Octet; 16] {
        self.address
    }
}

/// ProtocolVersion_t
/// Type used to represent the version of the RTPS protocol. The version is composed of a major and a minor version number. See also 8.6.
/// The following values are reserved by the protocol: PROTOCOLVERSION PROTOCOLVERSION_1_0 PROTOCOLVERSION_1_1 PROTOCOLVERSION_2_0 PROTOCOLVERSION_2_1 PROTOCOLVERSION_2_2
/// PROTOCOLVERSION_2_4
/// PROTOCOLVERSION is an alias for the most recent version, in this case PROTOCOLVERSION_2_4
#[derive(Clone, Copy, PartialEq, Eq, Debug, TypeSupport)]
#[dust_dds(extensibility = "final", nested)]
pub struct ProtocolVersion {
    pub(crate) bytes: [u8; 2],
}

impl ProtocolVersion {
    pub const fn new(major: Octet, minor: Octet) -> Self {
        Self {
            bytes: [major, minor],
        }
    }
    pub const fn _major(&self) -> Octet {
        self.bytes[0]
    }
    pub const fn _minor(&self) -> Octet {
        self.bytes[1]
    }
}

/// VendorId_t
/// Type used to represent the vendor of the service implementing the RTPS protocol. The possible values for the vendorId are assigned by the OMG.
/// The following values are reserved by the protocol: VENDORID_UNKNOWN
pub type VendorId = [Octet; 2];

/// ChangeCount_t
/// Type used to hold a counter representing the number of HistoryCache changes that belong to a certain category.
/// For example, the number of changes that have been filtered for an RTPS Reader endpoint.
#[allow(dead_code)]
pub struct ChangeCount {
    high: Long,
    low: UnsignedLong,
}

#[derive(Clone, PartialEq, Debug, Copy, PartialOrd, Eq, Ord)]
pub struct Time {
    sec: i32,
    nanosec: u32,
}

impl Time {
    /// Create a new [`Time`] with a number of seconds and nanoseconds
    pub const fn new(sec: i32, nanosec: u32) -> Self {
        let sec = sec + (nanosec / 1_000_000_000) as i32;
        let nanosec = nanosec % 1_000_000_000;
        Self { sec, nanosec }
    }

    /// Get the number of seconds contained by this time
    pub const fn sec(&self) -> i32 {
        self.sec
    }

    /// Get the number of nanoseconds contained by this time
    pub const fn nanosec(&self) -> u32 {
        self.nanosec
    }
}

#[derive(Debug, PartialEq, Eq, Clone)]
pub struct WriterProxy {
    pub remote_writer_guid: Guid,
    pub remote_group_entity_id: EntityId,
    pub reliability_kind: ReliabilityKind,
    pub durability_kind: DurabilityKind,
    pub unicast_locator_list: Vec<Locator>,
    pub multicast_locator_list: Vec<Locator>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CacheChange {
    pub kind: ChangeKind,
    pub writer_guid: Guid,
    pub sequence_number: i64,
    pub source_timestamp: Option<Time>,
    pub instance_handle: Option<[u8; 16]>,
    pub data_value: Arc<[u8]>,
}

#[derive(Debug, PartialEq, Eq, Clone)]
pub struct ReaderProxy {
    pub remote_reader_guid: Guid,
    pub remote_group_entity_id: EntityId,
    pub reliability_kind: ReliabilityKind,
    pub durability_kind: DurabilityKind,
    pub unicast_locator_list: Vec<Locator>,
    pub multicast_locator_list: Vec<Locator>,
    pub expects_inline_qos: bool,
}
