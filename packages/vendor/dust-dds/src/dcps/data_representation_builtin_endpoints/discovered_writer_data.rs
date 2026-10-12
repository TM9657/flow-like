use super::parameter_id_values::{
    PID_DATA_REPRESENTATION, PID_DEADLINE, PID_DESTINATION_ORDER, PID_DURABILITY,
    PID_ENDPOINT_GUID, PID_GROUP_DATA, PID_GROUP_ENTITYID, PID_LATENCY_BUDGET, PID_LIFESPAN,
    PID_LIVELINESS, PID_MULTICAST_LOCATOR, PID_OWNERSHIP, PID_OWNERSHIP_STRENGTH,
    PID_PARTICIPANT_GUID, PID_PARTITION, PID_PRESENTATION, PID_RELIABILITY, PID_TOPIC_DATA,
    PID_TOPIC_NAME, PID_UNICAST_LOCATOR, PID_USER_DATA,
};
use crate::{
    builtin_topics::PublicationBuiltinTopicData,
    dcps::data_representation_builtin_endpoints::{
        parameter_id_values::{PID_TYPE_INFORMATION, PID_TYPE_NAME},
        rtps_data_representation::{CdrResult, ParameterList},
        rtps_data_representation_serialization::ParameterListSerializer,
    },
    infrastructure::qos_policy::{
        DEFAULT_RELIABILITY_QOS_POLICY_DATA_WRITER, DataRepresentationQosPolicy, DeadlineQosPolicy,
        DestinationOrderQosPolicy, DurabilityQosPolicy, GroupDataQosPolicy, LatencyBudgetQosPolicy,
        LifespanQosPolicy, LivelinessQosPolicy, OwnershipQosPolicy, OwnershipStrengthQosPolicy,
        PartitionQosPolicy, PresentationQosPolicy, TopicDataQosPolicy, UserDataQosPolicy,
    },
    transport::types::{ENTITYID_UNKNOWN, EntityId, Guid, Locator},
};
use alloc::vec::Vec;

#[derive(Debug, PartialEq, Eq, Clone)]
pub struct WriterProxy {
    pub remote_writer_guid: Guid,
    pub remote_group_entity_id: EntityId,
    pub unicast_locator_list: Vec<Locator>,
    pub multicast_locator_list: Vec<Locator>,
}

#[derive(Debug, PartialEq, Eq, Clone)]
pub struct DiscoveredWriterData {
    pub(crate) dds_publication_data: PublicationBuiltinTopicData,
    pub(crate) writer_proxy: WriterProxy,
}

impl DiscoveredWriterData {
    pub fn into_bytes(self) -> Vec<u8> {
        let mut buffer = Vec::new();
        let mut pl = ParameterListSerializer::new(&mut buffer);
        pl.write_header();
        pl.write_xcdr1_parameter(PID_ENDPOINT_GUID, self.dds_publication_data.key);
        pl.write_xcdr1_parameter(
            PID_PARTICIPANT_GUID,
            self.dds_publication_data.participant_key,
        );
        pl.write_xcdr1_parameter(PID_TOPIC_NAME, self.dds_publication_data.topic_name);
        pl.write_xcdr1_parameter(PID_TYPE_NAME, self.dds_publication_data.type_name);
        if let Some(type_information) = self.dds_publication_data.type_information {
            pl.write_xcdr2_parameter(PID_TYPE_INFORMATION, type_information);
        }
        if self.dds_publication_data.durability != DurabilityQosPolicy::default() {
            pl.write_xcdr1_parameter(PID_DURABILITY, self.dds_publication_data.durability);
        }
        if self.dds_publication_data.deadline != DeadlineQosPolicy::default() {
            pl.write_xcdr1_parameter(PID_DEADLINE, self.dds_publication_data.deadline);
        }
        if self.dds_publication_data.latency_budget != LatencyBudgetQosPolicy::default() {
            pl.write_xcdr1_parameter(PID_LATENCY_BUDGET, self.dds_publication_data.latency_budget);
        }
        if self.dds_publication_data.liveliness != LivelinessQosPolicy::default() {
            pl.write_xcdr1_parameter(PID_LIVELINESS, self.dds_publication_data.liveliness);
        }
        if self.dds_publication_data.reliability != DEFAULT_RELIABILITY_QOS_POLICY_DATA_WRITER {
            pl.write_xcdr1_parameter(PID_RELIABILITY, self.dds_publication_data.reliability);
        }
        if self.dds_publication_data.lifespan != LifespanQosPolicy::default() {
            pl.write_xcdr1_parameter(PID_LIFESPAN, self.dds_publication_data.lifespan);
        }
        if self.dds_publication_data.user_data != UserDataQosPolicy::default() {
            pl.write_xcdr1_parameter(PID_USER_DATA, self.dds_publication_data.user_data);
        }
        if self.dds_publication_data.ownership != OwnershipQosPolicy::default() {
            pl.write_xcdr1_parameter(PID_OWNERSHIP, self.dds_publication_data.ownership);
        }
        if self.dds_publication_data.ownership_strength != OwnershipStrengthQosPolicy::default() {
            pl.write_xcdr1_parameter(
                PID_OWNERSHIP_STRENGTH,
                self.dds_publication_data.ownership_strength,
            );
        }
        if self.dds_publication_data.destination_order != DestinationOrderQosPolicy::default() {
            pl.write_xcdr1_parameter(
                PID_DESTINATION_ORDER,
                self.dds_publication_data.destination_order,
            );
        }
        if self.dds_publication_data.presentation != PresentationQosPolicy::default() {
            pl.write_xcdr1_parameter(PID_PRESENTATION, self.dds_publication_data.presentation);
        }
        if self.dds_publication_data.partition != PartitionQosPolicy::default() {
            pl.write_xcdr1_parameter(PID_PARTITION, self.dds_publication_data.partition);
        }
        if self.dds_publication_data.topic_data != TopicDataQosPolicy::default() {
            pl.write_xcdr1_parameter(PID_TOPIC_DATA, self.dds_publication_data.topic_data);
        }
        if self.dds_publication_data.group_data != GroupDataQosPolicy::default() {
            pl.write_xcdr1_parameter(PID_GROUP_DATA, self.dds_publication_data.group_data);
        }
        if self.dds_publication_data.representation != DataRepresentationQosPolicy::default() {
            pl.write_xcdr1_parameter(
                PID_DATA_REPRESENTATION,
                self.dds_publication_data.representation,
            );
        }
        if self.writer_proxy.remote_group_entity_id != ENTITYID_UNKNOWN {
            pl.write_cdr_parameter(PID_GROUP_ENTITYID, self.writer_proxy.remote_group_entity_id);
        }
        for l in self.writer_proxy.unicast_locator_list {
            pl.write_cdr_parameter(PID_UNICAST_LOCATOR, l);
        }
        for l in self.writer_proxy.multicast_locator_list {
            pl.write_cdr_parameter(PID_MULTICAST_LOCATOR, l);
        }

        pl.write_sentinel();
        buffer
    }

    pub fn from_bytes(bytes: &[u8]) -> CdrResult<Self> {
        let pl = ParameterList::new(bytes)?;

        let dds_publication_data = PublicationBuiltinTopicData {
            key: pl.get_optional_parameter_xdcr(PID_ENDPOINT_GUID, Default::default())?,
            participant_key: pl
                .get_optional_parameter_xdcr(PID_PARTICIPANT_GUID, Default::default())?,
            topic_name: pl.get_optional_parameter_xdcr(PID_TOPIC_NAME, Default::default())?,
            type_name: pl.get_optional_parameter_xdcr(PID_TYPE_NAME, Default::default())?,
            type_information: pl
                .get_optional_parameter_xdcr2(PID_TYPE_INFORMATION)
                .unwrap_or_default(),
            durability: pl.get_optional_parameter_xdcr(PID_DURABILITY, Default::default())?,
            deadline: pl.get_optional_parameter_xdcr(PID_DEADLINE, Default::default())?,
            latency_budget: pl
                .get_optional_parameter_xdcr(PID_LATENCY_BUDGET, Default::default())?,
            liveliness: pl.get_optional_parameter_xdcr(PID_LIVELINESS, Default::default())?,
            reliability: pl.get_optional_parameter_xdcr(
                PID_RELIABILITY,
                DEFAULT_RELIABILITY_QOS_POLICY_DATA_WRITER,
            )?,
            lifespan: pl.get_optional_parameter_xdcr(PID_LIFESPAN, Default::default())?,
            user_data: pl.get_optional_parameter_xdcr(PID_USER_DATA, Default::default())?,
            ownership: pl.get_optional_parameter_xdcr(PID_OWNERSHIP, Default::default())?,
            ownership_strength: pl
                .get_optional_parameter_xdcr(PID_OWNERSHIP_STRENGTH, Default::default())?,
            destination_order: pl
                .get_optional_parameter_xdcr(PID_DESTINATION_ORDER, Default::default())?,
            presentation: pl.get_optional_parameter_xdcr(PID_PRESENTATION, Default::default())?,
            partition: pl.get_optional_parameter_xdcr(PID_PARTITION, Default::default())?,
            topic_data: pl.get_optional_parameter_xdcr(PID_TOPIC_DATA, Default::default())?,
            group_data: pl.get_optional_parameter_xdcr(PID_GROUP_DATA, Default::default())?,
            representation: pl
                .get_optional_parameter_xdcr(PID_DATA_REPRESENTATION, Default::default())?,
        };

        let writer_proxy = WriterProxy {
            remote_writer_guid: Guid::from(dds_publication_data.key.value),
            remote_group_entity_id: pl
                .get_optional_parameter(PID_GROUP_ENTITYID, ENTITYID_UNKNOWN)?,
            unicast_locator_list: pl.get_locator_list(PID_UNICAST_LOCATOR)?,
            multicast_locator_list: pl.get_locator_list(PID_MULTICAST_LOCATOR)?,
        };
        Ok(DiscoveredWriterData {
            dds_publication_data,
            writer_proxy,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        builtin_topics::BuiltInTopicKey,
        infrastructure::{
            qos_policy::{ReliabilityQosPolicy, ReliabilityQosPolicyKind},
            time::{Duration, DurationKind},
        },
        transport::types::{
            BUILT_IN_PARTICIPANT, BUILT_IN_READER_GROUP, BUILT_IN_WRITER_WITH_KEY, EntityId, Guid,
            USER_DEFINED_UNKNOWN,
        },
    };

    #[test]
    fn serialize_all_default() {
        let data = DiscoveredWriterData {
            dds_publication_data: PublicationBuiltinTopicData {
                key: BuiltInTopicKey {
                    value: [1, 0, 0, 0, 2, 0, 0, 0, 3, 0, 0, 0, 4, 0, 0, 0],
                },
                participant_key: BuiltInTopicKey {
                    value: [6, 0, 0, 0, 7, 0, 0, 0, 8, 0, 0, 0, 9, 0, 0, 0],
                },
                topic_name: "ab".to_string().into(),
                type_name: "cd".to_string().into(),
                type_information: None,
                durability: Default::default(),
                deadline: Default::default(),
                latency_budget: Default::default(),
                liveliness: Default::default(),
                reliability: DEFAULT_RELIABILITY_QOS_POLICY_DATA_WRITER,
                lifespan: Default::default(),
                user_data: Default::default(),
                ownership: Default::default(),
                ownership_strength: Default::default(),
                destination_order: Default::default(),
                presentation: Default::default(),
                partition: Default::default(),
                topic_data: Default::default(),
                group_data: Default::default(),
                representation: Default::default(),
            },
            writer_proxy: WriterProxy {
                remote_writer_guid: Guid::new(
                    [5; 12],
                    EntityId::new([11, 12, 13], BUILT_IN_WRITER_WITH_KEY),
                ),
                remote_group_entity_id: EntityId::new([21, 22, 23], BUILT_IN_READER_GROUP),
                unicast_locator_list: vec![],
                multicast_locator_list: vec![],
            },
        }
        .into_bytes();

        let expected = [
            0x00, 0x03, 0x00, 0x00, // PL_CDR_LE
            0x5a, 0x00, 16, 0, //PID_ENDPOINT_GUID, length
            1, 0, 0, 0, // ,
            2, 0, 0, 0, // ,
            3, 0, 0, 0, // ,
            4, 0, 0, 0, // ,
            0x50, 0x00, 16, 0, //PID_PARTICIPANT_GUID, length
            6, 0, 0, 0, // ,
            7, 0, 0, 0, // ,
            8, 0, 0, 0, // ,
            9, 0, 0, 0, // ,
            0x05, 0x00, 0x08, 0x00, // PID_TOPIC_NAME, Length: 8
            3, 0x00, 0x00, 0x00, // string length (incl. terminator)
            b'a', b'b', 0, 0x00, // string + padding (1 byte)
            0x07, 0x00, 0x08, 0x00, // PID_TYPE_NAME, Length: 8
            3, 0x00, 0x00, 0x00, // string length (incl. terminator)
            b'c', b'd', 0, 0x00, // string + padding (1 byte)
            0x53, 0x00, 4, 0, //PID_GROUP_ENTITYID
            21, 22, 23, 0xc9, // u8[3], u8
            0x01, 0x00, 0x00, 0x00, // PID_SENTINEL, length
        ];

        assert_eq!(data, expected.to_vec());
    }

    #[test]
    fn deserialize_discovered_writer_data() {
        let expected = DiscoveredWriterData {
            dds_publication_data: PublicationBuiltinTopicData {
                key: BuiltInTopicKey {
                    value: [1, 0, 0, 0, 2, 0, 0, 0, 3, 0, 0, 0, 4, 0, 0, 0],
                },
                participant_key: BuiltInTopicKey {
                    value: [6, 0, 0, 0, 7, 0, 0, 0, 8, 0, 0, 0, 9, 0, 0, 0],
                },
                topic_name: "ab".to_string().into(),
                type_name: "cd".to_string().into(),
                type_information: None,
                durability: Default::default(),
                deadline: Default::default(),
                latency_budget: Default::default(),
                liveliness: Default::default(),
                reliability: ReliabilityQosPolicy {
                    kind: ReliabilityQosPolicyKind::BestEffort,
                    max_blocking_time: DurationKind::Finite(Duration::new(1, 0)),
                },
                lifespan: Default::default(),
                user_data: Default::default(),
                ownership: Default::default(),
                ownership_strength: Default::default(),
                destination_order: Default::default(),
                presentation: Default::default(),
                partition: Default::default(),
                topic_data: Default::default(),
                group_data: Default::default(),
                representation: Default::default(),
            },
            writer_proxy: WriterProxy {
                // must correspond to publication_builtin_topic_data.key
                remote_writer_guid: Guid::new(
                    [1, 0, 0, 0, 2, 0, 0, 0, 3, 0, 0, 0],
                    EntityId::new([4, 0, 0], USER_DEFINED_UNKNOWN),
                ),
                remote_group_entity_id: EntityId::new([21, 22, 23], BUILT_IN_PARTICIPANT),
                unicast_locator_list: vec![],
                multicast_locator_list: vec![],
            },
        };

        let data = [
            0x00, 0x03, 0x00, 0x00, // PL_CDR_LE
            0x53, 0x00, 4, 0, //PID_GROUP_ENTITYID
            21, 22, 23, 0xc1, // u8[3], u8
            0x5a, 0x00, 16, 0, //PID_ENDPOINT_GUID, length
            1, 0, 0, 0, // ,
            2, 0, 0, 0, // ,
            3, 0, 0, 0, // ,
            4, 0, 0, 0, // ,
            0x50, 0x00, 16, 0, //PID_PARTICIPANT_GUID, length
            6, 0, 0, 0, // ,
            7, 0, 0, 0, // ,
            8, 0, 0, 0, // ,
            9, 0, 0, 0, // ,
            0x05, 0x00, 0x08, 0x00, // PID_TOPIC_NAME, Length: 8
            3, 0x00, 0x00, 0x00, // string length (incl. terminator)
            b'a', b'b', 0, 0x00, // string + padding (1 byte)
            0x07, 0x00, 0x08, 0x00, // PID_TYPE_NAME, Length: 8
            3, 0x00, 0x00, 0x00, // string length (incl. terminator)
            b'c', b'd', 0, 0x00, // string + padding (1 byte)
            26, 0, 12, 0, // PID_RELIABILITY
            1, 0, 0, 0, // Best Effort reliability kind
            1, 0, 0, 0, 0, 0, 0, 0, // Duration {sec:1, nanosec:0}
            0x01, 0x00, 0x00, 0x00, // PID_SENTINEL, length
        ];
        assert_eq!(DiscoveredWriterData::from_bytes(&data).unwrap(), expected);
    }

    #[test]
    fn deserialize_all_default() {
        let expected = DiscoveredWriterData {
            dds_publication_data: PublicationBuiltinTopicData {
                key: BuiltInTopicKey {
                    value: [1, 0, 0, 0, 2, 0, 0, 0, 3, 0, 0, 0, 4, 0, 0, 0],
                },
                participant_key: BuiltInTopicKey {
                    value: [6, 0, 0, 0, 7, 0, 0, 0, 8, 0, 0, 0, 9, 0, 0, 0],
                },
                topic_name: "ab".to_string().into(),
                type_name: "cd".to_string().into(),
                type_information: None,
                durability: Default::default(),
                deadline: Default::default(),
                latency_budget: Default::default(),
                liveliness: Default::default(),
                reliability: DEFAULT_RELIABILITY_QOS_POLICY_DATA_WRITER,
                lifespan: Default::default(),
                user_data: Default::default(),
                ownership: Default::default(),
                ownership_strength: Default::default(),
                destination_order: Default::default(),
                presentation: Default::default(),
                partition: Default::default(),
                topic_data: Default::default(),
                group_data: Default::default(),
                representation: Default::default(),
            },
            writer_proxy: WriterProxy {
                // must correspond to publication_builtin_topic_data.key
                remote_writer_guid: Guid::new(
                    [1, 0, 0, 0, 2, 0, 0, 0, 3, 0, 0, 0],
                    EntityId::new([4, 0, 0], USER_DEFINED_UNKNOWN),
                ),
                remote_group_entity_id: EntityId::new([21, 22, 23], BUILT_IN_PARTICIPANT),
                unicast_locator_list: vec![],
                multicast_locator_list: vec![],
            },
        };

        let data = [
            0x00, 0x03, 0x00, 0x00, // PL_CDR_LE
            0x53, 0x00, 4, 0, //PID_GROUP_ENTITYID
            21, 22, 23, 0xc1, // u8[3], u8
            0x5a, 0x00, 16, 0, //PID_ENDPOINT_GUID, length
            1, 0, 0, 0, // ,
            2, 0, 0, 0, // ,
            3, 0, 0, 0, // ,
            4, 0, 0, 0, // ,
            0x50, 0x00, 16, 0, //PID_PARTICIPANT_GUID, length
            6, 0, 0, 0, // ,
            7, 0, 0, 0, // ,
            8, 0, 0, 0, // ,
            9, 0, 0, 0, // ,
            0x05, 0x00, 0x08, 0x00, // PID_TOPIC_NAME, Length: 8
            3, 0x00, 0x00, 0x00, // string length (incl. terminator)
            b'a', b'b', 0, 0x00, // string + padding (1 byte)
            0x07, 0x00, 0x08, 0x00, // PID_TYPE_NAME, Length: 8
            3, 0x00, 0x00, 0x00, // string length (incl. terminator)
            b'c', b'd', 0, 0x00, // string + padding (1 byte)
            0x01, 0x00, 0x00, 0x00, // PID_SENTINEL, length
        ];
        assert_eq!(DiscoveredWriterData::from_bytes(&data).unwrap(), expected);
    }
}
