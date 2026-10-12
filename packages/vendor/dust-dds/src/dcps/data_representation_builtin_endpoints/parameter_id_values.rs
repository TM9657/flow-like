use crate::{
    infrastructure::time::Duration,
    transport::types::{ENTITYID_UNKNOWN, EntityId, Short},
};

pub type ParameterId = Short;

// Constant value from Table 9.13 - ParameterId Values
pub const _PID_PAD: ParameterId = 0x0000;
pub const _PID_SENTINEL: ParameterId = 0x0001;
pub const PID_USER_DATA: ParameterId = 0x002c;
pub const PID_TOPIC_NAME: ParameterId = 0x0005;
pub const PID_TYPE_NAME: ParameterId = 0x0007;
pub const PID_GROUP_DATA: ParameterId = 0x002d;
pub const PID_TOPIC_DATA: ParameterId = 0x002e;
pub const PID_DURABILITY: ParameterId = 0x001d;
pub const _PID_DURABILITY_SERVICE: ParameterId = 0x001e;
pub const PID_DEADLINE: ParameterId = 0x0023;
pub const PID_LATENCY_BUDGET: ParameterId = 0x0027;
pub const PID_LIVELINESS: ParameterId = 0x001b;
pub const PID_RELIABILITY: ParameterId = 0x001a;
pub const PID_LIFESPAN: ParameterId = 0x002b;
pub const PID_DESTINATION_ORDER: ParameterId = 0x0025;
pub const PID_HISTORY: ParameterId = 0x0040;
pub const PID_RESOURCE_LIMITS: ParameterId = 0x0041;
pub const PID_OWNERSHIP: ParameterId = 0x001f;
pub const PID_OWNERSHIP_STRENGTH: ParameterId = 0x0006;
pub const PID_PRESENTATION: ParameterId = 0x0021;
pub const PID_PARTITION: ParameterId = 0x0029;
pub const PID_TIME_BASED_FILTER: ParameterId = 0x0004;
pub const PID_TRANSPORT_PRIORITY: ParameterId = 0x0049;
pub const PID_DOMAIN_ID: ParameterId = 0x000f;
pub const PID_DOMAIN_TAG: ParameterId = 0x4014;
pub const PID_PROTOCOL_VERSION: ParameterId = 0x0015;
pub const PID_VENDORID: ParameterId = 0x0016;
pub const PID_UNICAST_LOCATOR: ParameterId = 0x002f;
pub const PID_MULTICAST_LOCATOR: ParameterId = 0x0030;
pub const PID_DEFAULT_UNICAST_LOCATOR: ParameterId = 0x0031;
pub const PID_DEFAULT_MULTICAST_LOCATOR: ParameterId = 0x0048;
pub const PID_METATRAFFIC_UNICAST_LOCATOR: ParameterId = 0x0032;
pub const PID_METATRAFFIC_MULTICAST_LOCATOR: ParameterId = 0x0033;
pub const PID_EXPECTS_INLINE_QOS: ParameterId = 0x0043;
pub const PID_PARTICIPANT_MANUAL_LIVELINESS_COUNT: ParameterId = 0x0034;
pub const PID_PARTICIPANT_LEASE_DURATION: ParameterId = 0x0002;
pub const _PID_CONTENT_FILTER_PROPERTY: ParameterId = 0x0035;
pub const PID_PARTICIPANT_GUID: ParameterId = 0x0050;
pub const _PID_GROUP_GUID: ParameterId = 0x0052;
pub const PID_BUILTIN_ENDPOINT_SET: ParameterId = 0x0058;
pub const PID_BUILTIN_ENDPOINT_QOS: ParameterId = 0x0077;
pub const _PID_PROPERTY_LIST: ParameterId = 0x0059;
pub const _PID_TYPE_MAX_SIZE_SERIALIZED: ParameterId = 0x0060;
pub const _PID_ENTITY_NAME: ParameterId = 0x0062;
pub const PID_ENDPOINT_GUID: ParameterId = 0x005a;
// Following PID name is not defined in standard but the number is in the Xtypes v1.3 standard
pub const PID_TYPE_INFORMATION: ParameterId = 0x0075;
// Following PID is not defined in standard
// (but its listed in "Table 9.14 - ParameterId mapping and default values")
pub const _PID_DATA_MAX_SIZE_SERIALIZED: ParameterId = _PID_TYPE_MAX_SIZE_SERIALIZED;
// Following PID is listed in "Table 9.19 – Deprecated ParameterId Values" but
// also in "Table 9.14 - ParameterId mapping and default values"
pub const PID_GROUP_ENTITYID: ParameterId = 0x0053;
pub const PID_DATA_REPRESENTATION: ParameterId = 0x0073;
pub const PID_TYPE_CONSISTENCY_ENFORCEMENT: ParameterId = 0x0074;

#[allow(overflowing_literals)]
pub const _PID_TYPE_REPRESENTATION: ParameterId = 0x8010;
#[allow(overflowing_literals)]
pub const _PID_DISCOVERED_PARTICIPANT: ParameterId = 0x8020;

// Constant value from Table 9.14 - ParameterId mapping and default values
// that are not N/A and not See DDS specification
// PID_DOMAIN_ID is omitted since the default is dynamic
pub const DEFAULT_DOMAIN_TAG: &str = "";
pub const DEFAULT_EXPECTS_INLINE_QOS: bool = false;
pub const DEFAULT_PARTICIPANT_LEASE_DURATION: Duration = Duration::new(100, 0);
pub const _DEFAULT_GROUP_ENTITYID: EntityId = ENTITYID_UNKNOWN;
