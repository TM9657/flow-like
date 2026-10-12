use super::{
    builtin_constants::{TYPE_LOOKUP_REPLY_TOPIC_NAME, TYPE_LOOKUP_REQUEST_TOPIC_NAME},
    builtin_publisher::BuiltinPublisher,
    builtin_subscriber::BuiltinSubscriber,
    topic_entity::{ContentFilteredTopicEntity, TopicEntity},
    type_register::TypeRegister,
    user_defined_publisher::PublisherEntity,
    user_defined_subscriber::UserDefinedSubscriber,
};
use crate::{
    builtin_topics::{
        DCPS_PARTICIPANT, DCPS_PUBLICATION, DCPS_SUBSCRIPTION, DCPS_TOPIC,
        ParticipantBuiltinTopicData, TopicBuiltinTopicData,
    },
    dcps::{
        channels::{mpsc::MpscSender, oneshot::OneshotSender},
        data_representation_builtin_endpoints::{
            discovered_reader_data::DiscoveredReaderData,
            discovered_writer_data::DiscoveredWriterData,
        },
        listeners::domain_participant_listener::ListenerMail,
        status_condition::DcpsStatusCondition,
        status_mask::StatusMask,
    },
    dds_async::domain_participant_factory::DcpsSender,
    infrastructure::{
        domain::DomainId,
        error::DdsResult,
        instance::InstanceHandle,
        qos::{DomainParticipantQos, PublisherQos, SubscriberQos, TopicQos},
        qos_policy::{DestinationOrderQosPolicyKind, ReliabilityQosPolicyKind},
        time::{Duration, DurationKind, Time},
    },
    transport::{
        interface::RtpsTransportParticipant,
        types::{ENTITYID_PARTICIPANT, Guid, GuidPrefix, Locator, USER_DEFINED_TOPIC},
    },
    xtypes::dynamic_type::DynamicType,
};
use alloc::{
    collections::BTreeSet,
    string::{String, ToString},
    sync::Arc,
    vec::Vec,
};

pub struct DiscoveredParticipantInfo {
    pub dds_participant_data: ParticipantBuiltinTopicData,
    pub guid_prefix: GuidPrefix,
    pub default_unicast_locator_list: Vec<Locator>,
    pub default_multicast_locator_list: Vec<Locator>,
    pub lease_duration: Duration,
    pub last_communication_timestamp: Time,
}

pub struct DcpsDomainParticipant {
    pub transport: RtpsTransportParticipant,

    pub reader_counter: u16,
    pub writer_counter: u16,
    pub publisher_counter: u8,
    pub subscriber_counter: u8,
    pub domain_participant: DomainParticipantEntity,
    pub dcps_sender: DcpsSender,
}

impl DcpsDomainParticipant {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        domain_id: DomainId,
        domain_tag: String,
        guid_prefix: GuidPrefix,
        domain_participant_qos: DomainParticipantQos,
        listener_sender: Option<MpscSender<ListenerMail>>,
        listener_mask: StatusMask,
        transport: RtpsTransportParticipant,
        dcps_sender: DcpsSender,
        participant_announcement_interval: core::time::Duration,
        enable_type_information: bool,
    ) -> Self {
        let guid = Guid::new(guid_prefix, ENTITYID_PARTICIPANT);

        let participant_handle = InstanceHandle::new(guid.into());

        let builtin_subscriber = BuiltinSubscriber::new(guid_prefix);
        let builtin_publisher = BuiltinPublisher::new(guid_prefix, &transport);

        let domain_participant = DomainParticipantEntity::new(
            domain_id,
            domain_participant_qos,
            listener_sender,
            listener_mask,
            participant_handle,
            builtin_publisher,
            builtin_subscriber,
            domain_tag,
            Duration::from(participant_announcement_interval),
            enable_type_information,
        );

        Self {
            transport,
            reader_counter: 0,
            writer_counter: 0,
            publisher_counter: 0,
            subscriber_counter: 0,
            domain_participant,
            dcps_sender,
        }
    }

    pub fn domain_id(&self) -> DomainId {
        self.domain_participant.domain_id
    }

    pub fn time_until_participant_announcement(&self, now: Time) -> Option<Duration> {
        self.domain_participant
            .time_until_participant_announcement(now)
    }

    pub fn time_until_next_event(&self, now: Time) -> Option<Duration> {
        let mut min_time = self.time_until_participant_announcement(now);

        for dp in &self.domain_participant.discovered_participant_list {
            let elapsed = now - dp.last_communication_timestamp;
            let remaining = if dp.lease_duration > elapsed {
                dp.lease_duration - elapsed
            } else {
                Duration::new(0, 0)
            };
            min_time = min_time.map_or(Some(remaining), |m| Some(m.min(remaining)));
        }

        for t in &self.domain_participant.find_topic_sender_list {
            let remaining = if t.deadline > now {
                t.deadline - now
            } else {
                Duration::new(0, 0)
            };
            min_time = min_time.map_or(Some(remaining), |m| Some(m.min(remaining)));
        }

        for dw in self
            .domain_participant
            .builtin_publisher
            .stateful_data_writer_list()
        {
            if !dw.transport_writer.changes().is_empty() {
                if let Some(hb_time) = dw.transport_writer.time_until_next_heartbeat(now) {
                    min_time = min_time.map_or(Some(hb_time), |m| Some(m.min(hb_time)));
                }
            }
        }

        for subscriber in &self.domain_participant.user_defined_subscriber_list {
            for data_reader in &subscriber.data_reader_list {
                if let DurationKind::Finite(deadline) = data_reader.qos.deadline.period {
                    for instance in &data_reader.instance_ownership {
                        let elapsed = now - instance.last_received_time;
                        let remaining = if deadline > elapsed {
                            deadline - elapsed
                        } else {
                            Duration::new(0, 0)
                        };
                        min_time = min_time.map_or(Some(remaining), |m| Some(m.min(remaining)));
                    }
                }

                for sample in &data_reader.sample_list {
                    let timestamp = match data_reader.qos.destination_order.kind {
                        DestinationOrderQosPolicyKind::BySourceTimestamp => sample.source_timestamp,
                        DestinationOrderQosPolicyKind::ByReceptionTimestamp => {
                            Some(sample.reception_timestamp)
                        }
                    };
                    if let Some(timestamp) = timestamp {
                        if let Some(matched_publication) = data_reader
                            .matched_publication_list
                            .iter()
                            .find(|x| x.key().value == sample.writer_guid)
                        {
                            if let DurationKind::Finite(lifespan) =
                                matched_publication.lifespan().duration
                            {
                                let expiry = timestamp + lifespan;
                                let remaining = if expiry > now {
                                    expiry - now
                                } else {
                                    Duration::new(0, 0)
                                };
                                min_time =
                                    min_time.map_or(Some(remaining), |m| Some(m.min(remaining)));
                            }
                        }
                    }
                }
            }
        }

        for publisher in &self.domain_participant.user_defined_publisher_list {
            for data_writer in &publisher.data_writer_list {
                if let DurationKind::Finite(deadline) = data_writer.qos.deadline.period {
                    for instance in &data_writer.registered_instance_info {
                        if let Some(last_write_time) = instance.last_write_time {
                            let elapsed = now - last_write_time;
                            let remaining = if deadline > elapsed {
                                deadline - elapsed
                            } else {
                                Duration::new(0, 0)
                            };
                            min_time = min_time.map_or(Some(remaining), |m| Some(m.min(remaining)));
                        }
                    }
                }

                if let DurationKind::Finite(lifespan) = data_writer.qos.lifespan.duration {
                    if let Some(cc) = data_writer.transport_writer.changes().first() {
                        if let Some(source_timestamp) = cc.source_timestamp {
                            let expiry = Time::from(source_timestamp) + lifespan;
                            let remaining = if expiry > now {
                                expiry - now
                            } else {
                                Duration::new(0, 0)
                            };
                            min_time = min_time.map_or(Some(remaining), |m| Some(m.min(remaining)));
                        }
                    }
                }

                if let Some(pending) = &data_writer.pending_write_sample {
                    if let Some(expiration_time) = pending.expiration_time {
                        let remaining = if expiration_time > now {
                            expiration_time - now
                        } else {
                            Duration::new(0, 0)
                        };
                        min_time = min_time.map_or(Some(remaining), |m| Some(m.min(remaining)));
                    }
                }

                if data_writer.qos.reliability.kind == ReliabilityQosPolicyKind::Reliable
                    && !data_writer.transport_writer.changes().is_empty()
                {
                    if let Some(hb_time) =
                        data_writer.transport_writer.time_until_next_heartbeat(now)
                    {
                        min_time = min_time.map_or(Some(hb_time), |m| Some(m.min(hb_time)));
                    }
                }
            }
        }

        min_time
    }

    pub fn get_instance_handle(&self) -> &InstanceHandle {
        &self.domain_participant.instance_handle
    }
}

pub const BUILT_IN_TOPIC_NAME_LIST: [&str; 6] = [
    DCPS_PARTICIPANT,
    DCPS_TOPIC,
    DCPS_PUBLICATION,
    DCPS_SUBSCRIPTION,
    TYPE_LOOKUP_REQUEST_TOPIC_NAME,
    TYPE_LOOKUP_REPLY_TOPIC_NAME,
];

pub struct FindTopicNotification {
    pub topic_name: String,
    pub deadline: Time,
    pub type_support: DynamicType<'static>,
    pub reply_sender: OneshotSender<DdsResult<(InstanceHandle, String)>>,
}

pub struct DomainParticipantEntity {
    pub domain_id: DomainId,
    pub domain_tag: String,
    pub topic_counter: u16,
    pub instance_handle: InstanceHandle,
    pub qos: DomainParticipantQos,
    pub builtin_subscriber: BuiltinSubscriber,
    pub builtin_publisher: BuiltinPublisher,
    pub user_defined_subscriber_list: Vec<UserDefinedSubscriber>,
    pub default_subscriber_qos: SubscriberQos,
    pub user_defined_publisher_list: Vec<PublisherEntity>,
    pub default_publisher_qos: PublisherQos,
    pub locally_created_topic_list: Vec<TopicEntity>,
    pub content_filtered_topic_list: Vec<ContentFilteredTopicEntity>,
    pub type_register: TypeRegister,
    pub default_topic_qos: TopicQos,
    pub discovered_participant_list: Vec<DiscoveredParticipantInfo>,
    pub discovered_topic_list: Vec<TopicBuiltinTopicData>,
    pub discovered_reader_list: Vec<DiscoveredReaderData>,
    pub discovered_writer_list: Vec<DiscoveredWriterData>,
    pub enabled: bool,
    pub ignored_participants: BTreeSet<InstanceHandle>,
    pub ignored_publications: BTreeSet<InstanceHandle>,
    pub ignored_subscriptions: BTreeSet<InstanceHandle>,
    pub _ignored_topic_list: BTreeSet<InstanceHandle>,
    pub listener_sender: Option<MpscSender<ListenerMail>>,
    pub listener_mask: StatusMask,
    pub find_topic_sender_list: Vec<FindTopicNotification>,
    pub last_announcement_timestamp: Option<Time>,
    pub participant_announcement_interval: Duration,
    pub enable_type_information: bool,
}

impl DomainParticipantEntity {
    #[allow(clippy::too_many_arguments)]
    pub const fn new(
        domain_id: DomainId,
        domain_participant_qos: DomainParticipantQos,
        listener_sender: Option<MpscSender<ListenerMail>>,
        listener_mask: StatusMask,
        instance_handle: InstanceHandle,
        builtin_publisher: BuiltinPublisher,
        builtin_subscriber: BuiltinSubscriber,
        domain_tag: String,
        participant_announcement_interval: Duration,
        enable_type_information: bool,
    ) -> Self {
        Self {
            domain_id,
            instance_handle,
            topic_counter: 0,
            qos: domain_participant_qos,
            builtin_subscriber,
            builtin_publisher,
            user_defined_subscriber_list: Vec::new(),
            default_subscriber_qos: SubscriberQos::const_default(),
            user_defined_publisher_list: Vec::new(),
            default_publisher_qos: PublisherQos::const_default(),
            locally_created_topic_list: Vec::new(),
            content_filtered_topic_list: Vec::new(),
            type_register: TypeRegister::new(),
            default_topic_qos: TopicQos::const_default(),
            discovered_participant_list: Vec::new(),
            discovered_topic_list: Vec::new(),
            discovered_reader_list: Vec::new(),
            discovered_writer_list: Vec::new(),
            enabled: false,
            ignored_participants: BTreeSet::new(),
            ignored_publications: BTreeSet::new(),
            ignored_subscriptions: BTreeSet::new(),
            _ignored_topic_list: BTreeSet::new(),
            listener_sender,
            listener_mask,
            domain_tag,
            find_topic_sender_list: Vec::new(),
            last_announcement_timestamp: None,
            participant_announcement_interval,
            enable_type_information,
        }
    }

    pub fn time_until_participant_announcement(&self, now: Time) -> Option<Duration> {
        if self.enabled {
            match self.last_announcement_timestamp {
                Some(last_announcement) => {
                    let elapsed = now - last_announcement;
                    if elapsed >= self.participant_announcement_interval {
                        Some(Duration::new(0, 0))
                    } else {
                        Some(self.participant_announcement_interval - elapsed)
                    }
                }
                None => Some(Duration::new(0, 0)),
            }
        } else {
            None
        }
    }

    pub fn add_discovered_topic(&mut self, topic_builtin_topic_data: TopicBuiltinTopicData) {
        match self
            .discovered_topic_list
            .iter_mut()
            .find(|t| t.key() == topic_builtin_topic_data.key())
        {
            Some(x) => *x = topic_builtin_topic_data,
            None => self.discovered_topic_list.push(topic_builtin_topic_data),
        }
    }

    pub fn remove_discovered_writer(&mut self, discovered_writer_handle: &InstanceHandle) {
        self.discovered_writer_list
            .retain(|x| &x.dds_publication_data.key().value != discovered_writer_handle.as_ref());
    }

    pub fn get_discovered_topic_data(
        &self,
        topic_handle: &InstanceHandle,
    ) -> Option<&TopicBuiltinTopicData> {
        self.discovered_topic_list
            .iter()
            .find(|x| &x.key().value == topic_handle.as_ref())
    }

    pub fn find_topic(
        &mut self,
        topic_name: &str,
        type_support: DynamicType<'static>,
    ) -> Option<(InstanceHandle, String)> {
        if let Some(topic) = self
            .locally_created_topic_list
            .iter()
            .find(|x| x.topic_name.as_ref() == topic_name)
        {
            Some((topic.instance_handle, topic.type_name.to_string()))
        } else if let Some(discovered_topic_data) = self
            .discovered_topic_list
            .iter()
            .find(|&discovered_topic_data| discovered_topic_data.name() == topic_name)
        {
            let qos = TopicQos {
                topic_data: discovered_topic_data.topic_data().clone(),
                durability: discovered_topic_data.durability().clone(),
                deadline: discovered_topic_data.deadline().clone(),
                latency_budget: discovered_topic_data.latency_budget().clone(),
                liveliness: discovered_topic_data.liveliness().clone(),
                reliability: discovered_topic_data.reliability().clone(),
                destination_order: discovered_topic_data.destination_order().clone(),
                history: discovered_topic_data.history().clone(),
                resource_limits: discovered_topic_data.resource_limits().clone(),
                transport_priority: discovered_topic_data.transport_priority().clone(),
                lifespan: discovered_topic_data.lifespan().clone(),
                ownership: discovered_topic_data.ownership().clone(),
                representation: discovered_topic_data.representation().clone(),
            };
            let type_name = discovered_topic_data.type_name.clone();
            let topic_handle = InstanceHandle::new([
                self.instance_handle[0],
                self.instance_handle[1],
                self.instance_handle[2],
                self.instance_handle[3],
                self.instance_handle[4],
                self.instance_handle[5],
                self.instance_handle[6],
                self.instance_handle[7],
                self.instance_handle[8],
                self.instance_handle[9],
                self.instance_handle[10],
                self.instance_handle[11],
                0,
                self.topic_counter.to_ne_bytes()[0],
                self.topic_counter.to_ne_bytes()[1],
                USER_DEFINED_TOPIC,
            ]);
            self.topic_counter += 1;
            let status_condition = DcpsStatusCondition::default();
            let type_information = self
                .type_register
                .register_local_type(Arc::from(type_name.value.as_str()), type_support);
            let mut topic = TopicEntity::new(
                qos,
                Arc::from(type_name.value.as_str()),
                Arc::from(topic_name),
                topic_handle,
                status_condition,
                None,
                StatusMask::default(),
                type_information,
            );
            topic.enabled = true;

            match self
                .locally_created_topic_list
                .iter_mut()
                .find(|x| x.topic_name == topic.topic_name)
            {
                Some(x) => *x = topic,
                None => self.locally_created_topic_list.push(topic),
            }

            Some((topic_handle, type_name.value))
        } else {
            None
        }
    }

    pub fn add_discovered_reader(&mut self, discovered_reader_data: DiscoveredReaderData) {
        match self.discovered_reader_list.iter_mut().find(|x| {
            x.dds_subscription_data.key() == discovered_reader_data.dds_subscription_data.key()
        }) {
            Some(x) => *x = discovered_reader_data,
            None => self.discovered_reader_list.push(discovered_reader_data),
        }
    }

    pub fn remove_discovered_reader(&mut self, discovered_reader_handle: &InstanceHandle) {
        self.discovered_reader_list
            .retain(|x| &x.dds_subscription_data.key().value != discovered_reader_handle.as_ref());
    }

    pub fn add_discovered_writer(&mut self, discovered_writer_data: DiscoveredWriterData) {
        match self.discovered_writer_list.iter_mut().find(|x| {
            x.dds_publication_data.key() == discovered_writer_data.dds_publication_data.key()
        }) {
            Some(x) => *x = discovered_writer_data,
            None => self.discovered_writer_list.push(discovered_writer_data),
        }
    }

    pub fn remove_subscriber(&mut self, handle: &InstanceHandle) -> Option<UserDefinedSubscriber> {
        let i = self
            .user_defined_subscriber_list
            .iter()
            .position(|x| &x.instance_handle == handle)?;

        Some(self.user_defined_subscriber_list.remove(i))
    }

    pub fn remove_publisher(&mut self, handle: &InstanceHandle) -> Option<PublisherEntity> {
        let i = self
            .user_defined_publisher_list
            .iter()
            .position(|x| &x.instance_handle == handle)?;

        Some(self.user_defined_publisher_list.remove(i))
    }

    pub fn is_empty(&self) -> bool {
        let no_user_defined_topics = self
            .locally_created_topic_list
            .iter()
            .filter(|t| !BUILT_IN_TOPIC_NAME_LIST.contains(&t.topic_name.as_ref()))
            .count()
            == 0;

        self.user_defined_publisher_list.is_empty()
            && self.user_defined_subscriber_list.is_empty()
            && self.content_filtered_topic_list.is_empty()
            && no_user_defined_topics
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_time_until_stale_writer_sample_calculation() {
        let source_timestamp = crate::transport::types::Time::new(100, 0);
        let lifespan = Duration::new(10, 0);
        let now = Time::new(105, 0);

        let remaining = Time::from(source_timestamp) + lifespan - now;
        assert_eq!(remaining, Duration::new(5, 0));
    }

    #[test]
    fn test_time_until_participant_announcement() {
        struct MockWriter {
            buffer: [u8; 512],
        }
        impl crate::transport::interface::WriteMessage for MockWriter {
            fn write_buffer_mut(&mut self) -> &mut [u8] {
                &mut self.buffer
            }
            fn write_message(&mut self, _len: usize, _locators: &[Locator]) {}
        }

        let transport = RtpsTransportParticipant {
            message_writer: Box::new(MockWriter { buffer: [0; 512] }),
            default_unicast_locator_list: Vec::new(),
            metatraffic_unicast_locator_list: Vec::new(),
            metatraffic_multicast_locator_list: Vec::new(),
            default_multicast_locator_list: Vec::new(),
            fragment_size: 65536,
        };

        let mut entity = DomainParticipantEntity::new(
            0,
            DomainParticipantQos::default(),
            None,
            StatusMask::default(),
            InstanceHandle::new([0; 16]),
            BuiltinPublisher::new(GuidPrefix::default(), &transport),
            BuiltinSubscriber::new(GuidPrefix::default()),
            String::new(),
            Duration::new(5, 0),
            true,
        );

        // Disabled entity returns None
        assert_eq!(
            entity.time_until_participant_announcement(Time::new(10, 0)),
            None
        );

        entity.enabled = true;

        // Enabled entity without previous announcement returns 0
        assert_eq!(
            entity.time_until_participant_announcement(Time::new(10, 0)),
            Some(Duration::new(0, 0))
        );

        // After announcement at t=10s, remaining time at t=12s should be 3s
        entity.last_announcement_timestamp = Some(Time::new(10, 0));
        assert_eq!(
            entity.time_until_participant_announcement(Time::new(12, 0)),
            Some(Duration::new(3, 0))
        );

        // At t=15s (5s elapsed), remaining time should be 0s
        assert_eq!(
            entity.time_until_participant_announcement(Time::new(15, 0)),
            Some(Duration::new(0, 0))
        );

        // Past interval (e.g. t=16s), remaining time should still be 0s
        assert_eq!(
            entity.time_until_participant_announcement(Time::new(16, 0)),
            Some(Duration::new(0, 0))
        );
    }
}
