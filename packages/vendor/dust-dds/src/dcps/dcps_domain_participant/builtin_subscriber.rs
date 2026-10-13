use crate::{
    builtin_topics::{
        DCPS_PARTICIPANT, DCPS_PUBLICATION, DCPS_SUBSCRIPTION, DCPS_TOPIC,
        ParticipantBuiltinTopicData, PublicationBuiltinTopicData, SubscriptionBuiltinTopicData,
        TopicBuiltinTopicData,
    },
    dcps::{
        data_representation_builtin_endpoints::type_lookup::{TypeLookupReply, TypeLookupRequest},
        dcps_domain_participant::{
            builtin_data_reader::BuiltinDataReader, data_reader_entity::DataReaderEntity,
            subscriber_entity::SubscriberEntity,
        },
    },
    infrastructure::{instance::InstanceHandle, qos::SubscriberQos},
    rtps::{stateful_reader::RtpsStatefulReader, stateless_reader::RtpsStatelessReader},
    transport::types::{Guid, GuidPrefix, ReliabilityKind},
};
use alloc::sync::Arc;
use core::ops::{Deref, DerefMut};

use super::builtin_constants::{
    ENTITYID_BUILTIN_SUBSCRIBER, ENTITYID_SEDP_BUILTIN_PUBLICATIONS_DETECTOR,
    ENTITYID_SEDP_BUILTIN_SUBSCRIPTIONS_DETECTOR, ENTITYID_SEDP_BUILTIN_TOPICS_DETECTOR,
    ENTITYID_SPDP_BUILTIN_PARTICIPANT_READER, ENTITYID_TL_SVC_REPLY_READER,
    ENTITYID_TL_SVC_REQ_READER, SEDP_DATA_READER_QOS, SPDP_READER_QOS, TYPE_LOOKUP_READER_QOS,
    TYPE_LOOKUP_REPLY_TOPIC_NAME, TYPE_LOOKUP_REQUEST_TOPIC_NAME,
};

pub struct BuiltinSubscriber {
    pub subscriber_entity: SubscriberEntity,
    pub dcps_participant_reader:
        BuiltinDataReader<RtpsStatelessReader, ParticipantBuiltinTopicData>,
    pub dcps_topic_reader: BuiltinDataReader<RtpsStatefulReader, TopicBuiltinTopicData>,
    pub dcps_publication_reader: BuiltinDataReader<RtpsStatefulReader, PublicationBuiltinTopicData>,
    pub dcps_subscription_reader:
        BuiltinDataReader<RtpsStatefulReader, SubscriptionBuiltinTopicData>,
    pub type_lookup_request_reader: BuiltinDataReader<RtpsStatefulReader, TypeLookupRequest>,
    pub type_lookup_reply_reader: BuiltinDataReader<RtpsStatefulReader, TypeLookupReply>,
}

impl Deref for BuiltinSubscriber {
    type Target = SubscriberEntity;

    fn deref(&self) -> &Self::Target {
        &self.subscriber_entity
    }
}

impl DerefMut for BuiltinSubscriber {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.subscriber_entity
    }
}

impl BuiltinSubscriber {
    pub fn new(guid_prefix: GuidPrefix) -> Self {
        let subscriber_entity = SubscriberEntity::new(
            InstanceHandle::new(Guid::new(guid_prefix, ENTITYID_BUILTIN_SUBSCRIBER).into()),
            SubscriberQos::default(),
        );

        let rtps_stateless_reader = RtpsStatelessReader::new(Guid::new(
            guid_prefix,
            ENTITYID_SPDP_BUILTIN_PARTICIPANT_READER,
        ));
        let dcps_participant_reader = BuiltinDataReader::new(
            InstanceHandle::new(rtps_stateless_reader.guid().into()),
            SPDP_READER_QOS,
            Arc::from(DCPS_PARTICIPANT),
            rtps_stateless_reader,
        );

        let dcps_topic_transport_reader = RtpsStatefulReader::new(
            Guid::new(guid_prefix, ENTITYID_SEDP_BUILTIN_TOPICS_DETECTOR),
            ReliabilityKind::Reliable,
        );
        let dcps_topic_reader = BuiltinDataReader::new(
            InstanceHandle::new(dcps_topic_transport_reader.guid().into()),
            SEDP_DATA_READER_QOS,
            Arc::from(DCPS_TOPIC),
            dcps_topic_transport_reader,
        );

        let dcps_publication_transport_reader = RtpsStatefulReader::new(
            Guid::new(guid_prefix, ENTITYID_SEDP_BUILTIN_PUBLICATIONS_DETECTOR),
            ReliabilityKind::Reliable,
        );
        let dcps_publication_reader = BuiltinDataReader::new(
            InstanceHandle::new(dcps_publication_transport_reader.guid().into()),
            SEDP_DATA_READER_QOS,
            Arc::from(DCPS_PUBLICATION),
            dcps_publication_transport_reader,
        );

        let dcps_subscription_transport_reader = RtpsStatefulReader::new(
            Guid::new(guid_prefix, ENTITYID_SEDP_BUILTIN_SUBSCRIPTIONS_DETECTOR),
            ReliabilityKind::Reliable,
        );
        let dcps_subscription_reader = BuiltinDataReader::new(
            InstanceHandle::new(dcps_subscription_transport_reader.guid().into()),
            SEDP_DATA_READER_QOS,
            Arc::from(DCPS_SUBSCRIPTION),
            dcps_subscription_transport_reader,
        );

        let type_lookup_request_transport_reader = RtpsStatefulReader::new(
            Guid::new(guid_prefix, ENTITYID_TL_SVC_REQ_READER),
            ReliabilityKind::Reliable,
        );
        let type_lookup_request_reader = BuiltinDataReader::new(
            InstanceHandle::new(type_lookup_request_transport_reader.guid().into()),
            TYPE_LOOKUP_READER_QOS,
            Arc::from(TYPE_LOOKUP_REQUEST_TOPIC_NAME),
            type_lookup_request_transport_reader,
        );

        let type_lookup_reply_transport_reader = RtpsStatefulReader::new(
            Guid::new(guid_prefix, ENTITYID_TL_SVC_REPLY_READER),
            ReliabilityKind::Reliable,
        );
        let type_lookup_reply_reader = BuiltinDataReader::new(
            InstanceHandle::new(type_lookup_reply_transport_reader.guid().into()),
            TYPE_LOOKUP_READER_QOS,
            Arc::from(TYPE_LOOKUP_REPLY_TOPIC_NAME),
            type_lookup_reply_transport_reader,
        );

        Self {
            subscriber_entity,
            dcps_participant_reader,
            dcps_topic_reader,
            dcps_publication_reader,
            dcps_subscription_reader,
            type_lookup_request_reader,
            type_lookup_reply_reader,
        }
    }

    pub fn enable(&mut self) {
        self.dcps_participant_reader.enabled = true;
        for dr in self.stateful_data_reader_list_mut() {
            dr.enabled = true;
        }
        self.subscriber_entity.enabled = true;
    }

    pub fn stateful_data_reader_list_mut(
        &mut self,
    ) -> [&mut DataReaderEntity<RtpsStatefulReader>; 5] {
        [
            &mut *self.dcps_topic_reader,
            &mut *self.dcps_publication_reader,
            &mut *self.dcps_subscription_reader,
            &mut *self.type_lookup_request_reader,
            &mut *self.type_lookup_reply_reader,
        ]
    }

    pub fn find_stateful_data_reader_mut(
        &mut self,
        handle: &InstanceHandle,
    ) -> Option<&mut DataReaderEntity<RtpsStatefulReader>> {
        self.stateful_data_reader_list_mut()
            .into_iter()
            .find(|dr| &dr.instance_handle == handle)
    }
}
