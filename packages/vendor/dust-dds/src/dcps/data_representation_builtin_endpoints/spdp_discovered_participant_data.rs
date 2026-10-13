use super::{
    parameter_id_values::{
        DEFAULT_EXPECTS_INLINE_QOS, PID_BUILTIN_ENDPOINT_QOS, PID_BUILTIN_ENDPOINT_SET,
        PID_DEFAULT_MULTICAST_LOCATOR, PID_DEFAULT_UNICAST_LOCATOR, PID_DOMAIN_ID, PID_DOMAIN_TAG,
        PID_EXPECTS_INLINE_QOS, PID_METATRAFFIC_MULTICAST_LOCATOR, PID_METATRAFFIC_UNICAST_LOCATOR,
        PID_PARTICIPANT_GUID, PID_PARTICIPANT_LEASE_DURATION,
        PID_PARTICIPANT_MANUAL_LIVELINESS_COUNT, PID_PROTOCOL_VERSION, PID_USER_DATA, PID_VENDORID,
    },
    rtps_data_representation::CdrResult,
};
use crate::{
    builtin_topics::ParticipantBuiltinTopicData,
    dcps::data_representation_builtin_endpoints::{
        parameter_id_values::{DEFAULT_DOMAIN_TAG, DEFAULT_PARTICIPANT_LEASE_DURATION},
        rtps_data_representation::ParameterList,
        rtps_data_representation_serialization::ParameterListSerializer,
    },
    infrastructure::{
        domain::DomainId, instance::InstanceHandle, qos_policy::UserDataQosPolicy, time::Duration,
    },
    transport::types::{Guid, GuidPrefix, Locator, Long, ProtocolVersion, VendorId},
    xtypes::type_support::TypeSupport,
};
use alloc::{string::String, vec, vec::Vec};

pub type Count = Long;

#[derive(PartialEq, Eq, Debug, Clone, Copy, TypeSupport)]
#[dust_dds(extensibility = "final", nested)]
pub struct BuiltinEndpointSet(pub u32);

impl Default for BuiltinEndpointSet {
    fn default() -> Self {
        Self(
            Self::BUILTIN_ENDPOINT_PARTICIPANT_ANNOUNCER
                | Self::BUILTIN_ENDPOINT_PARTICIPANT_DETECTOR
                | Self::BUILTIN_ENDPOINT_PUBLICATIONS_ANNOUNCER
                | Self::BUILTIN_ENDPOINT_PUBLICATIONS_DETECTOR
                | Self::BUILTIN_ENDPOINT_SUBSCRIPTIONS_ANNOUNCER
                | Self::BUILTIN_ENDPOINT_SUBSCRIPTIONS_DETECTOR
                | Self::BUILTIN_ENDPOINT_TOPICS_ANNOUNCER
                | Self::BUILTIN_ENDPOINT_TOPICS_DETECTOR
                | Self::BUILTIN_ENDPOINT_TYPE_LOOKUP_SERVICE_REQUEST_DATA_WRITER
                | Self::BUILTIN_ENDPOINT_TYPE_LOOKUP_SERVICE_REQUEST_DATA_READER
                | Self::BUILTIN_ENDPOINT_TYPE_LOOKUP_SERVICE_REPLY_DATA_WRITER
                | Self::BUILTIN_ENDPOINT_TYPE_LOOKUP_SERVICE_REPLY_DATA_READER,
        )
    }
}

impl BuiltinEndpointSet {
    pub const BUILTIN_ENDPOINT_PARTICIPANT_ANNOUNCER: u32 = 1 << 0;
    pub const BUILTIN_ENDPOINT_PARTICIPANT_DETECTOR: u32 = 1 << 1;
    pub const BUILTIN_ENDPOINT_PUBLICATIONS_ANNOUNCER: u32 = 1 << 2;
    pub const BUILTIN_ENDPOINT_PUBLICATIONS_DETECTOR: u32 = 1 << 3;
    pub const BUILTIN_ENDPOINT_SUBSCRIPTIONS_ANNOUNCER: u32 = 1 << 4;
    pub const BUILTIN_ENDPOINT_SUBSCRIPTIONS_DETECTOR: u32 = 1 << 5;

    /*
    The following have been deprecated in version 2.4 of the
    specification. These bits should not be used by versions of the
    protocol equal to or newer than the deprecated version unless
    they are used with the same meaning as in versions prior to the
    deprecated version.
    @position(6) DISC_BUILTIN_ENDPOINT_PARTICIPANT_PROXY_ANNOUNCER,
    @position(7) DISC_BUILTIN_ENDPOINT_PARTICIPANT_PROXY_DETECTOR,
    @position(8) DISC_BUILTIN_ENDPOINT_PARTICIPANT_STATE_ANNOUNCER,
    @position(9) DISC_BUILTIN_ENDPOINT_PARTICIPANT_STATE_DETECTOR,
    */

    pub const _BUILTIN_ENDPOINT_PARTICIPANT_MESSAGE_DATA_WRITER: u32 = 1 << 10;
    pub const _BUILTIN_ENDPOINT_PARTICIPANT_MESSAGE_DATA_READER: u32 = 1 << 11;

    /*
    Bits 12-15 have been reserved by the DDS-Xtypes 1.2 Specification
    and future revisions thereof.
    */
    pub const BUILTIN_ENDPOINT_TYPE_LOOKUP_SERVICE_REQUEST_DATA_WRITER: u32 = 1 << 12;
    pub const BUILTIN_ENDPOINT_TYPE_LOOKUP_SERVICE_REQUEST_DATA_READER: u32 = 1 << 13;
    pub const BUILTIN_ENDPOINT_TYPE_LOOKUP_SERVICE_REPLY_DATA_WRITER: u32 = 1 << 14;
    pub const BUILTIN_ENDPOINT_TYPE_LOOKUP_SERVICE_REPLY_DATA_READER: u32 = 1 << 15;

    /*
    Bits 16-27 have been reserved by the DDS-Security 1.1 Specification
    and future revisions thereof.
    */

    pub const BUILTIN_ENDPOINT_TOPICS_ANNOUNCER: u32 = 1 << 28;
    pub const BUILTIN_ENDPOINT_TOPICS_DETECTOR: u32 = 1 << 29;

    #[allow(dead_code)]
    pub fn new(value: u32) -> Self {
        Self(value)
    }

    pub fn has(&self, endpoint: u32) -> bool {
        (self.0 & endpoint) == endpoint
    }
}

#[derive(PartialEq, Eq, Debug, Default, Clone, Copy, TypeSupport)]
#[dust_dds(extensibility = "final", nested)]
pub struct BuiltinEndpointQos(pub u32);

impl BuiltinEndpointQos {
    #[allow(dead_code)]
    pub const BEST_EFFORT_PARTICIPANT_MESSAGE_DATA_READER: u32 = 1 << 29;

    #[allow(dead_code)]
    pub fn new(value: u32) -> Self {
        Self(value)
    }

    #[allow(dead_code)]
    pub fn has(&self, endpoint: u32) -> bool {
        (self.0 & endpoint) == endpoint
    }
}

#[derive(Debug, PartialEq, Eq, Clone)]
pub struct ParticipantProxy {
    pub(crate) domain_id: Option<DomainId>,
    pub(crate) domain_tag: String,
    pub(crate) protocol_version: ProtocolVersion,
    pub(crate) guid_prefix: GuidPrefix,
    pub(crate) vendor_id: VendorId,
    pub(crate) expects_inline_qos: bool,
    pub(crate) metatraffic_unicast_locator_list: Vec<Locator>,
    pub(crate) metatraffic_multicast_locator_list: Vec<Locator>,
    pub(crate) default_unicast_locator_list: Vec<Locator>,
    pub(crate) default_multicast_locator_list: Vec<Locator>,
    pub(crate) available_builtin_endpoints: BuiltinEndpointSet,
    pub(crate) manual_liveliness_count: Count,
    pub(crate) builtin_endpoint_qos: BuiltinEndpointQos,
}

#[derive(Debug, PartialEq, Eq, Clone)]
pub struct SpdpDiscoveredParticipantData {
    pub(crate) dds_participant_data: ParticipantBuiltinTopicData,
    pub(crate) participant_proxy: ParticipantProxy,
    pub(crate) lease_duration: Duration,
    pub(crate) discovered_participant_list: Vec<InstanceHandle>,
}

impl SpdpDiscoveredParticipantData {
    pub fn into_bytes(self) -> Vec<u8> {
        let mut buffer = Vec::new();

        let mut pl = ParameterListSerializer::new(&mut buffer);
        pl.write_header();

        if self.dds_participant_data.user_data != Default::default() {
            pl.write_xcdr1_parameter(PID_USER_DATA, self.dds_participant_data.user_data);
        }

        pl.write_xcdr1_parameter(PID_PARTICIPANT_GUID, self.dds_participant_data.key);

        if let Some(domain_id) = self.participant_proxy.domain_id {
            pl.write_cdr_parameter(PID_DOMAIN_ID, domain_id);
        }
        if self.participant_proxy.domain_tag != DEFAULT_DOMAIN_TAG {
            pl.write_cdr_parameter(PID_DOMAIN_TAG, self.participant_proxy.domain_tag);
        }
        pl.write_cdr_parameter(
            PID_PROTOCOL_VERSION,
            self.participant_proxy.protocol_version,
        );
        // guid_prefix is skipped because it is sent as the key
        pl.write_cdr_parameter(PID_VENDORID, self.participant_proxy.vendor_id);
        if self.participant_proxy.expects_inline_qos {
            pl.write_cdr_parameter(
                PID_EXPECTS_INLINE_QOS,
                self.participant_proxy.expects_inline_qos,
            );
        }
        for value in self.participant_proxy.metatraffic_unicast_locator_list {
            pl.write_cdr_parameter(PID_METATRAFFIC_UNICAST_LOCATOR, value);
        }
        for value in self.participant_proxy.metatraffic_multicast_locator_list {
            pl.write_cdr_parameter(PID_METATRAFFIC_MULTICAST_LOCATOR, value);
        }
        for value in self.participant_proxy.default_unicast_locator_list {
            pl.write_cdr_parameter(PID_DEFAULT_UNICAST_LOCATOR, value);
        }
        for value in self.participant_proxy.default_multicast_locator_list {
            pl.write_cdr_parameter(PID_DEFAULT_MULTICAST_LOCATOR, value);
        }

        pl.write_cdr_parameter(
            PID_BUILTIN_ENDPOINT_SET,
            self.participant_proxy.available_builtin_endpoints,
        );
        if self.participant_proxy.manual_liveliness_count != Count::default() {
            pl.write_cdr_parameter(
                PID_PARTICIPANT_MANUAL_LIVELINESS_COUNT,
                self.participant_proxy.manual_liveliness_count,
            );
        }
        if self.participant_proxy.builtin_endpoint_qos != BuiltinEndpointQos::default() {
            pl.write_cdr_parameter(
                PID_BUILTIN_ENDPOINT_QOS,
                self.participant_proxy.builtin_endpoint_qos,
            );
        }
        pl.write_cdr_parameter(PID_PARTICIPANT_LEASE_DURATION, self.lease_duration);

        pl.write_sentinel();

        buffer
    }

    pub fn from_bytes(bytes: &[u8]) -> CdrResult<Self> {
        let pl = ParameterList::new(bytes)?;

        let dds_participant_data = ParticipantBuiltinTopicData {
            key: pl.get_non_optional_parameter_xdcr(PID_PARTICIPANT_GUID)?,
            user_data: pl
                .get_optional_parameter_xdcr(PID_USER_DATA, UserDataQosPolicy::default())?,
        };

        let participant_proxy = ParticipantProxy {
            domain_id: pl.get_non_optional_parameter(PID_DOMAIN_ID).ok(),
            domain_tag: pl
                .get_optional_parameter(PID_DOMAIN_TAG, String::from(DEFAULT_DOMAIN_TAG))?,
            protocol_version: pl.get_non_optional_parameter(PID_PROTOCOL_VERSION)?,
            guid_prefix: Guid::from(dds_participant_data.key.value).prefix(),
            vendor_id: pl.get_non_optional_parameter(PID_VENDORID)?,
            expects_inline_qos: pl
                .get_optional_parameter(PID_EXPECTS_INLINE_QOS, DEFAULT_EXPECTS_INLINE_QOS)?,
            metatraffic_unicast_locator_list: pl
                .get_locator_list(PID_METATRAFFIC_UNICAST_LOCATOR)?,
            metatraffic_multicast_locator_list: pl
                .get_locator_list(PID_METATRAFFIC_MULTICAST_LOCATOR)?,
            default_unicast_locator_list: pl.get_locator_list(PID_DEFAULT_UNICAST_LOCATOR)?,
            default_multicast_locator_list: pl.get_locator_list(PID_DEFAULT_MULTICAST_LOCATOR)?,
            available_builtin_endpoints: pl.get_non_optional_parameter(PID_BUILTIN_ENDPOINT_SET)?,
            manual_liveliness_count: pl.get_optional_parameter(
                PID_PARTICIPANT_MANUAL_LIVELINESS_COUNT,
                Count::default(),
            )?,
            builtin_endpoint_qos: pl
                .get_optional_parameter(PID_BUILTIN_ENDPOINT_QOS, BuiltinEndpointQos::default())?,
        };
        Ok(SpdpDiscoveredParticipantData {
            dds_participant_data,
            participant_proxy,
            lease_duration: pl.get_optional_parameter(
                PID_PARTICIPANT_LEASE_DURATION,
                DEFAULT_PARTICIPANT_LEASE_DURATION,
            )?,
            discovered_participant_list: vec![],
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        builtin_topics::BuiltInTopicKey,
        dcps::data_representation_builtin_endpoints::parameter_id_values::DEFAULT_PARTICIPANT_LEASE_DURATION,
        infrastructure::qos_policy::UserDataQosPolicy, rtps::types::PROTOCOLVERSION_2_4,
    };

    #[test]
    fn serialize_spdp_discovered_participant_data() {
        let locator1 = Locator::new(11, 12, [1; 16]);
        let locator2 = Locator::new(21, 22, [2; 16]);

        let result = SpdpDiscoveredParticipantData {
            dds_participant_data: ParticipantBuiltinTopicData {
                key: BuiltInTopicKey {
                    value: [8, 8, 8, 8, 8, 8, 8, 8, 8, 8, 8, 8, 0, 0, 1, 0xc1],
                },
                user_data: UserDataQosPolicy {
                    value: vec![97, 53],
                },
            },
            participant_proxy: ParticipantProxy {
                domain_id: Some(0),
                domain_tag: "ab".to_string(),
                protocol_version: ProtocolVersion::new(2, 4),
                guid_prefix: [8; 12],
                vendor_id: [73, 74],
                expects_inline_qos: true,
                metatraffic_unicast_locator_list: vec![locator1, locator2],
                metatraffic_multicast_locator_list: vec![locator1],
                default_unicast_locator_list: vec![locator1],
                default_multicast_locator_list: vec![locator1],
                available_builtin_endpoints: BuiltinEndpointSet::new(
                    BuiltinEndpointSet::BUILTIN_ENDPOINT_PARTICIPANT_DETECTOR,
                ),
                manual_liveliness_count: 2,
                builtin_endpoint_qos: BuiltinEndpointQos::new(
                    BuiltinEndpointQos::BEST_EFFORT_PARTICIPANT_MESSAGE_DATA_READER,
                ),
            },
            lease_duration: Duration::new(10, 11),
            discovered_participant_list: vec![],
        }
        .into_bytes();

        let expected = [
            0x00, 0x03, 0x00, 0x00, // PL_CDR_LE
            44, 0x00, 8, 0x00, // PID_USER_DATA, Length
            2, 0, 0, 0, // sequence length
            97, 53, 0, 0, // data, padding (2 bytes)
            0x50, 0x00, 16, 0x00, // PID_PARTICIPANT_GUID, Length
            8, 8, 8, 8, // GuidPrefix
            8, 8, 8, 8, // GuidPrefix
            8, 8, 8, 8, // GuidPrefix
            0, 0, 1, 0xc1, // EntityId
            15, 0x00, 0x04, 0x00, // PID_DOMAIN_ID, Length: 4
            0x00, 0x00, 0x00, 0x00, // DomainId
            0x14, 0x40, 0x08, 0x00, // PID_DOMAIN_TAG, Length: 8
            3, 0x00, 0x00, 0x00, // DomainTag: string length (incl. terminator)
            b'a', b'b', 0, 0x00, // DomainTag: string + padding (1 byte)
            21, 0x00, 4, 0x00, // PID_PROTOCOL_VERSION, Length
            0x02, 0x04, 0x00, 0x00, // ProtocolVersion
            22, 0x00, 4, 0x00, // PID_VENDORID
            73, 74, 0x00, 0x00, // VendorId
            0x43, 0x00, 0x04, 0x00, // PID_EXPECTS_INLINE_QOS, Length: 4,
            0x01, 0x00, 0x00, 0x00, // True
            50, 0x00, 24, 0x00, // PID_METATRAFFIC_UNICAST_LOCATOR
            11, 0x00, 0x00, 0x00, // Locator{kind
            12, 0x00, 0x00, 0x00, // port,
            0x01, 0x01, 0x01, 0x01, //
            0x01, 0x01, 0x01, 0x01, // address
            0x01, 0x01, 0x01, 0x01, //
            0x01, 0x01, 0x01, 0x01, // }
            50, 0x00, 24, 0x00, // PID_METATRAFFIC_UNICAST_LOCATOR
            21, 0x00, 0x00, 0x00, // Locator{kind
            22, 0x00, 0x00, 0x00, // port,
            0x02, 0x02, 0x02, 0x02, //
            0x02, 0x02, 0x02, 0x02, // address
            0x02, 0x02, 0x02, 0x02, //
            0x02, 0x02, 0x02, 0x02, // }
            51, 0x00, 24, 0x00, // PID_METATRAFFIC_MULTICAST_LOCATOR
            11, 0x00, 0x00, 0x00, // Locator{kind
            12, 0x00, 0x00, 0x00, // port,
            0x01, 0x01, 0x01, 0x01, //
            0x01, 0x01, 0x01, 0x01, // address
            0x01, 0x01, 0x01, 0x01, //
            0x01, 0x01, 0x01, 0x01, // }
            49, 0x00, 24, 0x00, // PID_DEFAULT_UNICAST_LOCATOR
            11, 0x00, 0x00, 0x00, // Locator{kind
            12, 0x00, 0x00, 0x00, // port,
            0x01, 0x01, 0x01, 0x01, //
            0x01, 0x01, 0x01, 0x01, // address
            0x01, 0x01, 0x01, 0x01, //
            0x01, 0x01, 0x01, 0x01, // }
            72, 0x00, 24, 0x00, // PID_DEFAULT_MULTICAST_LOCATOR
            11, 0x00, 0x00, 0x00, // Locator{kind
            12, 0x00, 0x00, 0x00, // port,
            0x01, 0x01, 0x01, 0x01, //
            0x01, 0x01, 0x01, 0x01, // address
            0x01, 0x01, 0x01, 0x01, //
            0x01, 0x01, 0x01, 0x01, // }
            88, 0x00, 4, 0x00, // PID_BUILTIN_ENDPOINT_SET
            0x02, 0x00, 0x00, 0x00, //
            52, 0x00, 4, 0x00, // PID_PARTICIPANT_MANUAL_LIVELINESS_COUNT
            0x02, 0x00, 0x00, 0x00, // Count
            119, 0x00, 4, 0x00, // PID_BUILTIN_ENDPOINT_QOS
            0x00, 0x00, 0x00, 0x20, //
            2, 0x00, 8, 0x00, // PID_PARTICIPANT_LEASE_DURATION
            10, 0x00, 0x00, 0x00, // Duration: seconds
            11, 0x00, 0x00, 0x00, // Duration: fraction
            0x01, 0x00, 0x00, 0x00, // PID_SENTINEL
        ];
        assert_eq!(result, expected.to_vec());
    }

    #[test]
    fn serialize_spdp_discovered_participant_data_all_default() {
        let data = SpdpDiscoveredParticipantData {
            dds_participant_data: ParticipantBuiltinTopicData {
                key: BuiltInTopicKey {
                    value: [8, 8, 8, 8, 8, 8, 8, 8, 8, 8, 8, 8, 0, 0, 1, 0xc1],
                },
                user_data: UserDataQosPolicy::default(),
            },
            participant_proxy: ParticipantProxy {
                domain_id: None,
                domain_tag: String::from(DEFAULT_DOMAIN_TAG),
                protocol_version: PROTOCOLVERSION_2_4,
                guid_prefix: GuidPrefix::default(),
                vendor_id: [73, 74],
                expects_inline_qos: DEFAULT_EXPECTS_INLINE_QOS,
                metatraffic_unicast_locator_list: Vec::new(),
                metatraffic_multicast_locator_list: Vec::new(),
                default_unicast_locator_list: Vec::new(),
                default_multicast_locator_list: Vec::new(),
                available_builtin_endpoints: BuiltinEndpointSet::new(
                    BuiltinEndpointSet::BUILTIN_ENDPOINT_PARTICIPANT_DETECTOR,
                ),
                manual_liveliness_count: 0,
                builtin_endpoint_qos: BuiltinEndpointQos::default(),
            },
            lease_duration: DEFAULT_PARTICIPANT_LEASE_DURATION,
            discovered_participant_list: Vec::new(),
        }
        .into_bytes();

        let expected = [
            0x00, 0x03, 0x00, 0x00, // PL_CDR_LE
            0x50, 0x00, 16, 0x00, // PID_PARTICIPANT_GUID, Length
            8, 8, 8, 8, // GuidPrefix
            8, 8, 8, 8, // GuidPrefix
            8, 8, 8, 8, // GuidPrefix
            0, 0, 1, 0xc1, // EntityId
            0x15, 0x00, 4, 0x00, // PID_PROTOCOL_VERSION, Length
            0x02, 0x04, 0x00, 0x00, // ProtocolVersion
            0x16, 0x00, 4, 0x00, // PID_VENDORID
            73, 74, 0x00, 0x00, // VendorId
            88, 0x00, 4, 0x00, // PID_BUILTIN_ENDPOINT_SET
            0x02, 0x00, 0x00, 0x00, //
            0x02, 0x00, 8, 0x00, // PID_PARTICIPANT_LEASE_DURATION
            100, 0x00, 0x00, 0x00, // Duration: seconds
            0, 0x00, 0x00, 0x00, // Duration: fraction
            0x01, 0x00, 0x00, 0x00, // PID_SENTINEL
        ];
        assert_eq!(data, expected.to_vec());
    }

    #[test]
    fn deserialize_spdp_discovered_participant_data() {
        let locator1 = Locator::new(11, 12, [1; 16]);
        let locator2 = Locator::new(21, 22, [2; 16]);

        let domain_id = 1;
        let domain_tag = "ab".to_string();
        let protocol_version = ProtocolVersion::new(2, 4);
        let guid_prefix = [8; 12];
        let vendor_id = [73, 74];
        let expects_inline_qos = true;
        let metatraffic_unicast_locator_list = vec![locator1, locator2];
        let metatraffic_multicast_locator_list = vec![locator1];
        let default_unicast_locator_list = vec![locator1];
        let default_multicast_locator_list = vec![locator1];
        let available_builtin_endpoints =
            BuiltinEndpointSet::new(BuiltinEndpointSet::BUILTIN_ENDPOINT_PARTICIPANT_DETECTOR);
        let manual_liveliness_count = 2;
        let builtin_endpoint_qos = BuiltinEndpointQos::new(
            BuiltinEndpointQos::BEST_EFFORT_PARTICIPANT_MESSAGE_DATA_READER,
        );
        let lease_duration = Duration::new(10, 11);

        let expected = SpdpDiscoveredParticipantData {
            dds_participant_data: ParticipantBuiltinTopicData {
                key: BuiltInTopicKey {
                    value: [8, 8, 8, 8, 8, 8, 8, 8, 8, 8, 8, 8, 0, 0, 1, 0xc1],
                },
                user_data: UserDataQosPolicy { value: vec![] },
            },
            participant_proxy: ParticipantProxy {
                domain_id: Some(domain_id),
                domain_tag,
                protocol_version,
                guid_prefix,
                vendor_id,
                expects_inline_qos,
                metatraffic_unicast_locator_list,
                metatraffic_multicast_locator_list,
                default_unicast_locator_list,
                default_multicast_locator_list,
                available_builtin_endpoints,
                manual_liveliness_count,
                builtin_endpoint_qos,
            },
            lease_duration,
            discovered_participant_list: vec![],
        };

        let data = [
            0x00, 0x03, 0x00, 0x00, // PL_CDR_LE
            15, 0x00, 4, 0, // PID_DOMAIN_ID, Length: 4
            0x01, 0x00, 0, 0x00, // DomainId
            0x14, 0x40, 0x08, 0x00, // PID_DOMAIN_TAG, Length: 8
            3, 0x00, 0x00, 0x00, // DomainTag: string length (incl. terminator)
            b'a', b'b', 0, 0x00, // DomainTag: string + padding (1 byte)
            21, 0x00, 4, 0x00, // PID_PROTOCOL_VERSION, Length
            0x02, 0x04, 0x00, 0x00, // ProtocolVersion
            80, 0x00, 16, 0x00, // PID_PARTICIPANT_GUID, Length
            8, 8, 8, 8, // GuidPrefix
            8, 8, 8, 8, // GuidPrefix
            8, 8, 8, 8, // GuidPrefix
            0, 0, 1, 0xc1, // EntityId,
            22, 0x00, 4, 0x00, // PID_VENDORID
            73, 74, 0x00, 0x00, // VendorId
            67, 0x00, 0x04, 0x00, // PID_EXPECTS_INLINE_QOS, Length: 4,
            0x01, 0x00, 0x00, 0x00, // True
            50, 0x00, 24, 0x00, // PID_METATRAFFIC_UNICAST_LOCATOR
            11, 0x00, 0x00, 0x00, // Locator{kind
            12, 0x00, 0x00, 0x00, // port,
            0x01, 0x01, 0x01, 0x01, //
            0x01, 0x01, 0x01, 0x01, // address
            0x01, 0x01, 0x01, 0x01, //
            0x01, 0x01, 0x01, 0x01, // }
            50, 0x00, 24, 0x00, // PID_METATRAFFIC_UNICAST_LOCATOR
            21, 0x00, 0x00, 0x00, // Locator{kind
            22, 0x00, 0x00, 0x00, // port,
            0x02, 0x02, 0x02, 0x02, //
            0x02, 0x02, 0x02, 0x02, // address
            0x02, 0x02, 0x02, 0x02, //
            0x02, 0x02, 0x02, 0x02, // }
            51, 0x00, 24, 0x00, // PID_METATRAFFIC_MULTICAST_LOCATOR
            11, 0x00, 0x00, 0x00, // Locator{kind
            12, 0x00, 0x00, 0x00, // port,
            0x01, 0x01, 0x01, 0x01, //
            0x01, 0x01, 0x01, 0x01, // address
            0x01, 0x01, 0x01, 0x01, //
            0x01, 0x01, 0x01, 0x01, // }
            49, 0x00, 24, 0x00, // PID_DEFAULT_UNICAST_LOCATOR
            11, 0x00, 0x00, 0x00, // Locator{kind
            12, 0x00, 0x00, 0x00, // port,
            0x01, 0x01, 0x01, 0x01, //
            0x01, 0x01, 0x01, 0x01, // address
            0x01, 0x01, 0x01, 0x01, //
            0x01, 0x01, 0x01, 0x01, // }
            72, 0x00, 24, 0x00, // PID_DEFAULT_MULTICAST_LOCATOR
            11, 0x00, 0x00, 0x00, // Locator{kind
            12, 0x00, 0x00, 0x00, // port,
            0x01, 0x01, 0x01, 0x01, //
            0x01, 0x01, 0x01, 0x01, // address
            0x01, 0x01, 0x01, 0x01, //
            0x01, 0x01, 0x01, 0x01, // }
            88, 0x00, 4, 0x00, // PID_BUILTIN_ENDPOINT_SET
            0x02, 0x00, 0x00, 0x00, //
            0x34, 0x00, 4, 0x00, // PID_PARTICIPANT_MANUAL_LIVELINESS_COUNT
            0x02, 0x00, 0x00, 0x00, // Count
            119, 0x00, 4, 0x00, // PID_BUILTIN_ENDPOINT_QOS
            0x00, 0x00, 0x00, 0x20, //
            2, 0x00, 8, 0x00, // PID_PARTICIPANT_LEASE_DURATION
            10, 0x00, 0x00, 0x00, // Duration: seconds
            11, 0x00, 0x00, 0x00, // Duration: fraction
            0x01, 0x00, 0x00, 0x00, // PID_SENTINEL
        ];
        assert_eq!(
            SpdpDiscoveredParticipantData::from_bytes(&data).unwrap(),
            expected
        );
    }
}
