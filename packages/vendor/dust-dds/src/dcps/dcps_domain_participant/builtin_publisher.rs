use crate::{
    builtin_topics::{
        DCPS_PARTICIPANT, DCPS_PUBLICATION, DCPS_SUBSCRIPTION, DCPS_TOPIC,
        ParticipantBuiltinTopicData, PublicationBuiltinTopicData, SubscriptionBuiltinTopicData,
        TopicBuiltinTopicData,
    },
    dcps::{
        data_representation_builtin_endpoints::type_lookup::{TypeLookupReply, TypeLookupRequest},
        dcps_domain_participant::data_writer_entity::DataWriterEntity,
        xtypes_glue::key_and_instance_handle::KeyHolderType,
    },
    infrastructure::{
        instance::InstanceHandle,
        qos::DataWriterQos,
        qos_policy::{
            DurabilityQosPolicy, DurabilityQosPolicyKind, HistoryQosPolicy, HistoryQosPolicyKind,
            ReliabilityQosPolicy, ReliabilityQosPolicyKind,
        },
        time::{Duration, DurationKind},
    },
    rtps::{stateful_writer::RtpsStatefulWriter, stateless_writer::RtpsStatelessWriter},
    transport::{
        interface::RtpsTransportParticipant,
        types::{Guid, GuidPrefix},
    },
    xtypes::type_support::Type,
};
use alloc::sync::Arc;

use super::builtin_constants::{
    ENTITYID_SEDP_BUILTIN_PUBLICATIONS_ANNOUNCER, ENTITYID_SEDP_BUILTIN_SUBSCRIPTIONS_ANNOUNCER,
    ENTITYID_SEDP_BUILTIN_TOPICS_ANNOUNCER, ENTITYID_SPDP_BUILTIN_PARTICIPANT_WRITER,
    ENTITYID_TL_SVC_REPLY_WRITER, ENTITYID_TL_SVC_REQ_WRITER, TYPE_LOOKUP_REPLY_TOPIC_NAME,
    TYPE_LOOKUP_REQUEST_TOPIC_NAME, TYPE_LOOKUP_WRITER_QOS,
};

fn spdp_writer_qos() -> DataWriterQos {
    DataWriterQos {
        durability: DurabilityQosPolicy {
            kind: DurabilityQosPolicyKind::TransientLocal,
        },
        history: HistoryQosPolicy {
            kind: HistoryQosPolicyKind::KeepLast(1),
        },
        reliability: ReliabilityQosPolicy {
            kind: ReliabilityQosPolicyKind::BestEffort,
            max_blocking_time: DurationKind::Finite(Duration::new(0, 0)),
        },
        ..Default::default()
    }
}

fn sedp_data_writer_qos() -> DataWriterQos {
    DataWriterQos {
        durability: DurabilityQosPolicy {
            kind: DurabilityQosPolicyKind::TransientLocal,
        },
        history: HistoryQosPolicy {
            kind: HistoryQosPolicyKind::KeepLast(1),
        },
        reliability: ReliabilityQosPolicy {
            kind: ReliabilityQosPolicyKind::Reliable,
            max_blocking_time: DurationKind::Finite(Duration::new(0, 0)),
        },
        ..Default::default()
    }
}

pub struct BuiltinPublisher {
    pub dcps_participant_writer: DataWriterEntity<RtpsStatelessWriter>,
    pub dcps_topics_writer: DataWriterEntity<RtpsStatefulWriter>,
    pub dcps_publications_writer: DataWriterEntity<RtpsStatefulWriter>,
    pub dcps_subscriptions_writer: DataWriterEntity<RtpsStatefulWriter>,
    pub type_lookup_request_writer: DataWriterEntity<RtpsStatefulWriter>,
    pub type_lookup_reply_writer: DataWriterEntity<RtpsStatefulWriter>,
    pub enabled: bool,
}

impl BuiltinPublisher {
    pub fn new(guid_prefix: GuidPrefix, transport: &RtpsTransportParticipant) -> Self {
        let mut dcps_participant_transport_writer = RtpsStatelessWriter::new(Guid::new(
            guid_prefix,
            ENTITYID_SPDP_BUILTIN_PARTICIPANT_WRITER,
        ));
        for &discovery_locator in &transport.metatraffic_multicast_locator_list {
            dcps_participant_transport_writer.reader_locator_add(discovery_locator);
        }
        let dcps_participant_writer = DataWriterEntity::new(
            InstanceHandle::new(dcps_participant_transport_writer.guid().into()),
            dcps_participant_transport_writer,
            Arc::from(DCPS_PARTICIPANT),
            spdp_writer_qos(),
            KeyHolderType::new(&ParticipantBuiltinTopicData::TYPE),
        );

        let dcps_topics_transport_writer = RtpsStatefulWriter::new(
            Guid::new(guid_prefix, ENTITYID_SEDP_BUILTIN_TOPICS_ANNOUNCER),
            transport.fragment_size,
        );
        let dcps_topics_writer = DataWriterEntity::new(
            InstanceHandle::new(dcps_topics_transport_writer.guid().into()),
            dcps_topics_transport_writer,
            Arc::from(DCPS_TOPIC),
            sedp_data_writer_qos(),
            KeyHolderType::new(&TopicBuiltinTopicData::TYPE),
        );

        let dcps_publications_transport_writer = RtpsStatefulWriter::new(
            Guid::new(guid_prefix, ENTITYID_SEDP_BUILTIN_PUBLICATIONS_ANNOUNCER),
            transport.fragment_size,
        );
        let dcps_publications_writer = DataWriterEntity::new(
            InstanceHandle::new(dcps_publications_transport_writer.guid().into()),
            dcps_publications_transport_writer,
            Arc::from(DCPS_PUBLICATION),
            sedp_data_writer_qos(),
            KeyHolderType::new(&PublicationBuiltinTopicData::TYPE),
        );

        let dcps_subscriptions_transport_writer = RtpsStatefulWriter::new(
            Guid::new(guid_prefix, ENTITYID_SEDP_BUILTIN_SUBSCRIPTIONS_ANNOUNCER),
            transport.fragment_size,
        );
        let dcps_subscriptions_writer = DataWriterEntity::new(
            InstanceHandle::new(dcps_subscriptions_transport_writer.guid().into()),
            dcps_subscriptions_transport_writer,
            Arc::from(DCPS_SUBSCRIPTION),
            sedp_data_writer_qos(),
            KeyHolderType::new(&SubscriptionBuiltinTopicData::TYPE),
        );

        let type_lookup_request_transport_writer = RtpsStatefulWriter::new(
            Guid::new(guid_prefix, ENTITYID_TL_SVC_REQ_WRITER),
            transport.fragment_size,
        );
        let type_lookup_request_writer = DataWriterEntity::new(
            InstanceHandle::new(type_lookup_request_transport_writer.guid().into()),
            type_lookup_request_transport_writer,
            Arc::from(TYPE_LOOKUP_REQUEST_TOPIC_NAME),
            TYPE_LOOKUP_WRITER_QOS,
            KeyHolderType::new(&TypeLookupRequest::TYPE),
        );

        let type_lookup_reply_transport_writer = RtpsStatefulWriter::new(
            Guid::new(guid_prefix, ENTITYID_TL_SVC_REPLY_WRITER),
            transport.fragment_size,
        );
        let type_lookup_reply_writer = DataWriterEntity::new(
            InstanceHandle::new(type_lookup_reply_transport_writer.guid().into()),
            type_lookup_reply_transport_writer,
            Arc::from(TYPE_LOOKUP_REPLY_TOPIC_NAME),
            TYPE_LOOKUP_WRITER_QOS,
            KeyHolderType::new(&TypeLookupReply::TYPE),
        );

        Self {
            dcps_participant_writer,
            dcps_topics_writer,
            dcps_publications_writer,
            dcps_subscriptions_writer,
            type_lookup_request_writer,
            type_lookup_reply_writer,
            enabled: false,
        }
    }

    pub fn enable(&mut self) {
        self.dcps_participant_writer.enabled = true;
        for dw in self.stateful_data_writer_list_mut() {
            dw.enabled = true;
        }
        self.enabled = true;
    }

    pub fn stateful_data_writer_list(&self) -> [&DataWriterEntity<RtpsStatefulWriter>; 5] {
        [
            &self.dcps_topics_writer,
            &self.dcps_publications_writer,
            &self.dcps_subscriptions_writer,
            &self.type_lookup_request_writer,
            &self.type_lookup_reply_writer,
        ]
    }

    pub fn stateful_data_writer_list_mut(
        &mut self,
    ) -> [&mut DataWriterEntity<RtpsStatefulWriter>; 5] {
        [
            &mut self.dcps_topics_writer,
            &mut self.dcps_publications_writer,
            &mut self.dcps_subscriptions_writer,
            &mut self.type_lookup_request_writer,
            &mut self.type_lookup_reply_writer,
        ]
    }
}
