// Modified for Flow-Like: validate XTypes identities and serialize participant disposal keys.
use crate::{
    builtin_topics::{
        BuiltInTopicKey, ParticipantBuiltinTopicData, PublicationBuiltinTopicData,
        SubscriptionBuiltinTopicData, TopicBuiltinTopicData,
    },
    dcps::{
        data_representation_builtin_endpoints::{
            discovered_reader_data::{DiscoveredReaderData, ReaderProxy},
            discovered_topic_data::DiscoveredTopicData,
            discovered_writer_data::{DiscoveredWriterData, WriterProxy},
            parameter_id_values::{PID_ENDPOINT_GUID, PID_PARTICIPANT_GUID, ParameterId},
            rtps_data_representation_serialization::ParameterListSerializer,
            spdp_discovered_participant_data::{
                BuiltinEndpointQos, BuiltinEndpointSet, ParticipantProxy,
                SpdpDiscoveredParticipantData,
            },
            type_lookup::{
                RemoteExceptionCode, ReplyHeader, RequestHeader, SampleIdentity, TypeLookupCall,
                TypeLookupGetTypeDependenciesIn, TypeLookupGetTypeDependenciesOut,
                TypeLookupGetTypeDependenciesResult, TypeLookupGetTypesIn, TypeLookupGetTypesOut,
                TypeLookupGetTypesResult, TypeLookupReply, TypeLookupRequest, TypeLookupReturn,
            },
        },
        dcps_domain_participant::{
            builtin_constants::{
                ENTITYID_SEDP_BUILTIN_PUBLICATIONS_ANNOUNCER,
                ENTITYID_SEDP_BUILTIN_PUBLICATIONS_DETECTOR,
                ENTITYID_SEDP_BUILTIN_SUBSCRIPTIONS_ANNOUNCER,
                ENTITYID_SEDP_BUILTIN_SUBSCRIPTIONS_DETECTOR,
                ENTITYID_SEDP_BUILTIN_TOPICS_ANNOUNCER, ENTITYID_SEDP_BUILTIN_TOPICS_DETECTOR,
                ENTITYID_TL_SVC_REPLY_READER, ENTITYID_TL_SVC_REPLY_WRITER,
                ENTITYID_TL_SVC_REQ_READER, ENTITYID_TL_SVC_REQ_WRITER,
            },
            data_reader_entity::DataReaderEntity,
            data_writer_entity::IncompatibleSubscriptions,
            participant_entity::{
                DcpsDomainParticipant, DiscoveredParticipantInfo, DomainParticipantEntity,
            },
            user_defined_data_reader::UserDefinedDataReader,
            user_defined_data_writer::UserDefinedDataWriter,
        },
        listeners::domain_participant_listener::ListenerMail,
    },
    dds_async::{
        data_reader::DataReaderAsync, data_writer::DataWriterAsync,
        domain_participant::DomainParticipantAsync, publisher::PublisherAsync,
        subscriber::SubscriberAsync, topic::TopicAsync,
    },
    infrastructure::{
        error::DdsError,
        instance::InstanceHandle,
        qos::{DataWriterQos, PublisherQos, SubscriberQos},
        qos_policy::{
            DATA_REPRESENTATION_QOS_POLICY_ID, DEADLINE_QOS_POLICY_ID,
            DESTINATIONORDER_QOS_POLICY_ID, DURABILITY_QOS_POLICY_ID, DurabilityQosPolicyKind,
            LATENCYBUDGET_QOS_POLICY_ID, LIVELINESS_QOS_POLICY_ID, OWNERSHIP_QOS_POLICY_ID,
            PRESENTATION_QOS_POLICY_ID, PartitionQosPolicy, QosPolicyId, RELIABILITY_QOS_POLICY_ID,
            ReliabilityQosPolicyKind, TypeConsistencyEnforcementQosPolicy, TypeConsistencyKind,
            XCDR_DATA_REPRESENTATION,
        },
        status::{
            InconsistentTopicStatus, OfferedDeadlineMissedStatus, OfferedIncompatibleQosStatus,
            PublicationMatchedStatus, QosPolicyCount, StatusKind,
        },
        time::{Duration, DurationKind, Time},
    },
    rtps::types::{PROTOCOLVERSION, VENDOR_ID_S2E},
    transport::{
        self,
        types::{DurabilityKind, ENTITYID_UNKNOWN, Guid, GuidPrefix, ReliabilityKind},
    },
    xtypes::{
        serializer::serialize_cdr2_le,
        type_object::{
            TypeIdentifier, TypeIdentifierTypeObjectPair, TypeIdentifierWithSize, TypeObject,
        },
        type_support::{_String, TypeSupport},
    },
};
use alloc::{
    format,
    string::{String, ToString},
    vec,
    vec::Vec,
};
use regex::Regex;

fn minimal_type_matches(local: &TypeIdentifierWithSize, remote: &TypeIdentifierWithSize) -> bool {
    // Cyclone DDS advertises valid type hashes with a serialized size of zero.
    // The hash identifies the schema independently of that size estimate.
    matches!(local.type_id, TypeIdentifier::EkMinimal { .. }) && local.type_id == remote.type_id
}

impl DcpsDomainParticipant {
    pub fn announce_participant_if_needed(&mut self, now: Time) {
        if let Some(time_until) = self.time_until_participant_announcement(now) {
            if time_until == Duration::new(0, 0) {
                self.announce_participant(now);
            }
        }
    }

    #[tracing::instrument(skip(self))]
    pub fn announce_participant(&mut self, now: Time) {
        if self.domain_participant.enabled {
            self.domain_participant.last_announcement_timestamp = Some(now);
            let builtin_topic_key = *self.domain_participant.instance_handle.as_ref();
            let guid = Guid::from(builtin_topic_key);
            let participant_builtin_topic_data = ParticipantBuiltinTopicData {
                key: BuiltInTopicKey {
                    value: builtin_topic_key,
                },
                user_data: self.domain_participant.qos.user_data.clone(),
            };
            let participant_proxy = ParticipantProxy {
                domain_id: Some(self.domain_participant.domain_id),
                domain_tag: self.domain_participant.domain_tag.clone(),
                protocol_version: PROTOCOLVERSION,
                guid_prefix: guid.prefix(),
                vendor_id: VENDOR_ID_S2E,
                expects_inline_qos: false,
                metatraffic_unicast_locator_list: self
                    .transport
                    .metatraffic_unicast_locator_list
                    .to_vec(),
                metatraffic_multicast_locator_list: self
                    .transport
                    .metatraffic_multicast_locator_list
                    .to_vec(),
                default_unicast_locator_list: self.transport.default_unicast_locator_list.to_vec(),
                default_multicast_locator_list: self
                    .transport
                    .default_multicast_locator_list
                    .to_vec(),
                available_builtin_endpoints: BuiltinEndpointSet::default(),
                manual_liveliness_count: 0,
                builtin_endpoint_qos: BuiltinEndpointQos::default(),
            };
            let spdp_discovered_participant_data = SpdpDiscoveredParticipantData {
                dds_participant_data: participant_builtin_topic_data,
                participant_proxy,
                lease_duration: Duration::new(100, 0),
                discovered_participant_list: self
                    .domain_participant
                    .discovered_participant_list
                    .iter()
                    .map(|p| InstanceHandle::new(p.dds_participant_data.key().value))
                    .collect(),
            };

            {
                let w = &mut self
                    .domain_participant
                    .builtin_publisher
                    .dcps_participant_writer;
                let timestamp = now;
                let sample_instance_handle = self.domain_participant.instance_handle;
                let serialized_data = spdp_discovered_participant_data.into_bytes();
                let sample_timestamp = timestamp;
                let now = timestamp;
                w.write_w_timestamp(
                    sample_instance_handle,
                    serialized_data,
                    sample_timestamp,
                    now,
                )
                .ok();
            }
            self.domain_participant
                .builtin_publisher
                .dcps_participant_writer
                .transport_writer
                .write_message(self.transport.message_writer.as_mut());
        }
    }

    #[tracing::instrument(skip(self))]
    pub fn announce_deleted_participant(&mut self, now: Time) {
        if self.domain_participant.enabled {
            let timestamp = now;

            let dw = &mut self
                .domain_participant
                .builtin_publisher
                .dcps_participant_writer;
            let builtin_topic_key = *self.domain_participant.instance_handle.as_ref();

            let topic_key_data = BuiltInTopicKey {
                value: builtin_topic_key,
            };
            let instance_handle = InstanceHandle::new(topic_key_data.value);
            let serialized_key = topic_key_data.into_bytes(PID_PARTICIPANT_GUID);

            dw.unregister_w_timestamp(instance_handle, serialized_key, timestamp)
                .ok();
            self.domain_participant
                .builtin_publisher
                .dcps_participant_writer
                .transport_writer
                .write_message(self.transport.message_writer.as_mut());
        }
    }

    pub fn notify_find_topic_senders(&mut self, now: Time) {
        let found_topics = self
            .domain_participant
            .find_topic_sender_list
            .extract_if(.., |x| {
                now > x.deadline
                    || self
                        .domain_participant
                        .discovered_topic_list
                        .iter()
                        .any(|t| t.name.value == x.topic_name)
                    || self
                        .domain_participant
                        .locally_created_topic_list
                        .iter()
                        .any(|t| t.topic_name.as_ref() == x.topic_name.as_str())
            })
            .collect::<Vec<_>>();
        for t in found_topics {
            if let Some(value) = self
                .domain_participant
                .find_topic(&t.topic_name, t.type_support)
            {
                t.reply_sender.send(Ok(value))
            } else if now > t.deadline {
                t.reply_sender.send(Err(DdsError::Timeout));
            }
        }
    }

    pub fn remove_stale_participants(&mut self, now: Time) {
        while let Some(handle) = self
            .domain_participant
            .discovered_participant_list
            .iter()
            .find_map(|x| {
                if now - x.last_communication_timestamp > x.lease_duration {
                    Some(InstanceHandle::new(x.dds_participant_data.key.value))
                } else {
                    None
                }
            })
        {
            self.remove_discovered_participant(&handle, now);
        }
    }

    pub fn check_missed_reader_deadline(&mut self, now: Time) {
        for subscriber in &mut self.domain_participant.user_defined_subscriber_list {
            let subscriber_handle = subscriber.instance_handle;
            let subscriber_listener_mask = subscriber.listener_mask;
            let subscriber_listener_sender = subscriber.listener_sender.clone();
            for data_reader in &mut subscriber.data_reader_list {
                if let DurationKind::Finite(deadline) = data_reader.qos.deadline.period {
                    let missed_instances: Vec<_> = data_reader
                        .instances
                        .iter()
                        .filter_map(|x| {
                            if now - x.last_received_time_stamp() > deadline {
                                Some(*x.handle())
                            } else {
                                None
                            }
                        })
                        .collect();
                    for change_instance_handle in missed_instances {
                        data_reader
                            .instance_ownership
                            .retain(|x| x.instance_handle != change_instance_handle);

                        data_reader.requested_deadline_missed_status.total_count += 1;
                        data_reader
                            .requested_deadline_missed_status
                            .total_count_change += 1;
                        data_reader
                            .requested_deadline_missed_status
                            .last_instance_handle = change_instance_handle;

                        let the_participant = DomainParticipantAsync::new(
                            self.dcps_sender.clone(),
                            self.domain_participant.domain_id,
                            self.domain_participant.instance_handle,
                        );
                        let the_subscriber =
                            SubscriberAsync::new(subscriber_handle, the_participant.clone());

                        let type_name = if let Some(content_filtered_topic) = self
                            .domain_participant
                            .content_filtered_topic_list
                            .iter()
                            .find(|t| t.topic_name == data_reader.topic_name)
                        {
                            let topic = self
                                .domain_participant
                                .locally_created_topic_list
                                .iter()
                                .find(|t| t.topic_name == content_filtered_topic.related_topic_name)
                                .expect("Topic is guaranteed to exist");
                            topic.type_name.clone()
                        } else if let Some(topic) = self
                            .domain_participant
                            .locally_created_topic_list
                            .iter()
                            .find(|t| t.topic_name == data_reader.topic_name)
                        {
                            topic.type_name.clone()
                        } else {
                            panic!("Reader is guaranteed to always have a related topic");
                        };

                        let the_reader = DataReaderAsync::new(
                            data_reader.instance_handle,
                            the_subscriber.clone(),
                            data_reader.topic_name.clone(),
                            type_name,
                        );
                        if data_reader
                            .listener_mask
                            .is_enabled(&StatusKind::RequestedDeadlineMissed)
                        {
                            let status = data_reader.requested_deadline_missed_status.clone();
                            data_reader
                                .requested_deadline_missed_status
                                .total_count_change = 0;
                            if let Some(l) = &data_reader.listener_sender {
                                l.send(ListenerMail::RequestedDeadlineMissed {
                                    the_reader,
                                    status,
                                })
                                .ok();
                            }
                        } else if subscriber_listener_mask
                            .is_enabled(&StatusKind::RequestedDeadlineMissed)
                        {
                            let status = data_reader.requested_deadline_missed_status.clone();
                            data_reader
                                .requested_deadline_missed_status
                                .total_count_change = 0;
                            if let Some(l) = &subscriber_listener_sender {
                                l.send(ListenerMail::RequestedDeadlineMissed {
                                    the_reader,
                                    status,
                                })
                                .ok();
                            }
                        } else if self
                            .domain_participant
                            .listener_mask
                            .is_enabled(&StatusKind::RequestedDeadlineMissed)
                        {
                            let status = data_reader.requested_deadline_missed_status.clone();
                            data_reader
                                .requested_deadline_missed_status
                                .total_count_change = 0;
                            if let Some(l) = &self.domain_participant.listener_sender {
                                l.send(ListenerMail::RequestedDeadlineMissed {
                                    the_reader,
                                    status,
                                })
                                .ok();
                            }
                        }

                        data_reader
                            .status_condition
                            .add_communication_state(StatusKind::RequestedDeadlineMissed);
                    }
                } else {
                    continue;
                }
            }
        }
    }

    pub fn check_missed_writer_deadline(&mut self, now: Time) {
        for publisher in &mut self.domain_participant.user_defined_publisher_list {
            for data_writer in &mut publisher.data_writer_list {
                if let DurationKind::Finite(deadline) = data_writer.qos.deadline.period {
                    let mut missed_handles = Vec::new();
                    for instance in data_writer.registered_instance_info.iter_mut() {
                        if let Some(t) = &mut instance.last_write_time {
                            if now - *t > deadline {
                                *t += deadline;
                                missed_handles.push(instance.instance_handle);
                            }
                        }
                    }

                    for instance_handle in missed_handles {
                        let the_participant = DomainParticipantAsync::new(
                            self.dcps_sender.clone(),
                            self.domain_participant.domain_id,
                            self.domain_participant.instance_handle,
                        );
                        let the_publisher =
                            PublisherAsync::new(publisher.instance_handle, the_participant.clone());
                        let topic = self
                            .domain_participant
                            .locally_created_topic_list
                            .iter()
                            .find(|x| x.topic_name == data_writer.topic_name)
                            .expect("Writer is guaranteed to have matching topic");
                        let the_topic = TopicAsync::new(
                            topic.instance_handle,
                            topic.type_name.clone(),
                            data_writer.topic_name.clone(),
                            the_participant,
                        );
                        let the_writer = DataWriterAsync::new(
                            data_writer.instance_handle,
                            the_publisher,
                            the_topic,
                        );
                        data_writer
                            .offered_deadline_missed_status
                            .last_instance_handle = instance_handle;
                        data_writer.offered_deadline_missed_status.total_count += 1;
                        data_writer
                            .offered_deadline_missed_status
                            .total_count_change += 1;

                        if data_writer
                            .listener_mask
                            .is_enabled(&StatusKind::OfferedDeadlineMissed)
                        {
                            let status = data_writer
                                .offered_deadline_missed_status
                                .get_offered_deadline_missed_status();

                            if let Some(l) = &data_writer.listener_sender {
                                l.send(ListenerMail::OfferedDeadlineMissed { the_writer, status })
                                    .ok();
                            }
                        } else if publisher
                            .listener_mask
                            .is_enabled(&StatusKind::OfferedDeadlineMissed)
                        {
                            let status = data_writer
                                .offered_deadline_missed_status
                                .get_offered_deadline_missed_status();
                            if let Some(l) = &publisher.listener_sender {
                                l.send(ListenerMail::OfferedDeadlineMissed { the_writer, status })
                                    .ok();
                            }
                        } else if self
                            .domain_participant
                            .listener_mask
                            .is_enabled(&StatusKind::OfferedDeadlineMissed)
                        {
                            let status = data_writer
                                .offered_deadline_missed_status
                                .get_offered_deadline_missed_status();
                            if let Some(l) = &self.domain_participant.listener_sender {
                                l.send(ListenerMail::OfferedDeadlineMissed { the_writer, status })
                                    .ok();
                            }
                        }

                        data_writer
                            .status_condition
                            .add_communication_state(StatusKind::OfferedDeadlineMissed);
                    }
                }
            }
        }
    }

    pub fn remove_stale_writer_samples(&mut self, now: Time) {
        for publisher in &mut self.domain_participant.user_defined_publisher_list {
            for data_writer in &mut publisher.data_writer_list {
                if let DurationKind::Finite(lifespan) = data_writer.qos.lifespan.duration {
                    data_writer.transport_writer.changes_mut().retain(|cc| {
                        if let Some(timestamp) = &cc.source_timestamp {
                            Time::from(*timestamp) + lifespan > now
                        } else {
                            true
                        }
                    });
                }
            }
        }
    }

    pub fn remove_stale_reader_samples(&mut self, now: Time) {
        for subscriber in &mut self.domain_participant.user_defined_subscriber_list {
            for data_reader in &mut subscriber.data_reader_list {
                data_reader.remove_stale_samples(now);
            }
        }
    }

    #[tracing::instrument(skip(self))]
    pub fn announce_data_writer(
        &mut self,
        publisher_handle: &InstanceHandle,
        data_writer_handle: &InstanceHandle,
        now: Time,
    ) {
        let Some(publisher) = self
            .domain_participant
            .user_defined_publisher_list
            .iter()
            .find(|x| &x.instance_handle == publisher_handle)
        else {
            return;
        };
        let Some(data_writer) = publisher
            .data_writer_list
            .iter()
            .find(|x| &x.instance_handle == data_writer_handle)
        else {
            return;
        };
        let Some(topic) = self
            .domain_participant
            .locally_created_topic_list
            .iter()
            .find(|x| x.topic_name == data_writer.topic_name)
        else {
            return;
        };

        let topic_data = topic.qos.topic_data.clone();

        let type_information = self
            .domain_participant
            .enable_type_information
            .then(|| topic.type_information.clone());
        let dds_publication_data = PublicationBuiltinTopicData {
            key: BuiltInTopicKey {
                value: data_writer.transport_writer.guid().into(),
            },
            participant_key: BuiltInTopicKey {
                value: self.domain_participant.instance_handle.into(),
            },
            topic_name: data_writer.topic_name.to_string().into(),
            type_name: topic.type_name.to_string().into(),
            type_information,
            durability: data_writer.qos.durability.clone(),
            deadline: data_writer.qos.deadline.clone(),
            latency_budget: data_writer.qos.latency_budget.clone(),
            liveliness: data_writer.qos.liveliness.clone(),
            reliability: data_writer.qos.reliability.clone(),
            lifespan: data_writer.qos.lifespan.clone(),
            user_data: data_writer.qos.user_data.clone(),
            ownership: data_writer.qos.ownership.clone(),
            ownership_strength: data_writer.qos.ownership_strength.clone(),
            destination_order: data_writer.qos.destination_order.clone(),
            presentation: publisher.qos.presentation.clone(),
            partition: publisher.qos.partition.clone(),
            topic_data,
            group_data: publisher.qos.group_data.clone(),
            representation: data_writer.qos.representation.clone(),
        };
        let writer_proxy = WriterProxy {
            remote_writer_guid: data_writer.transport_writer.guid(),
            remote_group_entity_id: ENTITYID_UNKNOWN,
            unicast_locator_list: vec![],
            multicast_locator_list: vec![],
        };
        let discovered_writer_data = DiscoveredWriterData {
            dds_publication_data,
            writer_proxy,
        };

        {
            let dw = &mut self
                .domain_participant
                .builtin_publisher
                .dcps_publications_writer;
            let sample_instance_handle = data_writer.transport_writer.guid().into();
            let serialized_data = discovered_writer_data.into_bytes();
            let sample_timestamp = now;
            dw.write_w_timestamp(
                sample_instance_handle,
                serialized_data,
                sample_timestamp,
                now,
            )
            .ok();
        }
        self.domain_participant
            .builtin_publisher
            .dcps_publications_writer
            .transport_writer
            .write_message(self.transport.message_writer.as_mut(), now);
    }

    #[tracing::instrument(skip(self, data_writer))]
    pub(super) fn announce_deleted_data_writer(
        &mut self,
        data_writer: UserDefinedDataWriter,
        now: Time,
    ) {
        let timestamp = now;
        {
            let dw = &mut self
                .domain_participant
                .builtin_publisher
                .dcps_publications_writer;
            let topic_key_data = BuiltInTopicKey {
                value: data_writer.transport_writer.guid().into(),
            };
            let instance_handle = InstanceHandle::new(topic_key_data.value);
            let serialized_key = topic_key_data.into_bytes(PID_ENDPOINT_GUID);

            dw.unregister_w_timestamp(instance_handle, serialized_key, timestamp)
                .ok();
        }
        self.domain_participant
            .builtin_publisher
            .dcps_publications_writer
            .transport_writer
            .write_message(self.transport.message_writer.as_mut(), now);
    }

    #[tracing::instrument(skip(self))]
    pub fn announce_data_reader(
        &mut self,
        subscriber_handle: &InstanceHandle,
        data_reader_handle: &InstanceHandle,
        now: Time,
    ) {
        let Some(subscriber) = self
            .domain_participant
            .user_defined_subscriber_list
            .iter()
            .find(|x| &x.instance_handle == subscriber_handle)
        else {
            return;
        };
        let Some(data_reader) = subscriber
            .data_reader_list
            .iter()
            .find(|x| &x.instance_handle == data_reader_handle)
        else {
            return;
        };

        let topic = if let Some(content_filtered_topic) = self
            .domain_participant
            .content_filtered_topic_list
            .iter()
            .find(|x| x.topic_name == data_reader.topic_name)
        {
            let Some(t) = self
                .domain_participant
                .locally_created_topic_list
                .iter()
                .find(|x| x.topic_name == content_filtered_topic.related_topic_name)
            else {
                return;
            };
            t
        } else {
            let Some(t) = self
                .domain_participant
                .locally_created_topic_list
                .iter()
                .find(|x| x.topic_name == data_reader.topic_name)
            else {
                return;
            };
            t
        };
        let guid = data_reader.transport_reader.guid();
        let type_information = self
            .domain_participant
            .enable_type_information
            .then(|| topic.type_information.clone());
        let dds_subscription_data = SubscriptionBuiltinTopicData {
            key: BuiltInTopicKey { value: guid.into() },
            participant_key: BuiltInTopicKey {
                value: self.domain_participant.instance_handle.into(),
            },
            topic_name: _String {
                value: topic.topic_name.to_string(),
            },
            type_name: _String {
                value: topic.type_name.to_string(),
            },
            type_information,
            durability: data_reader.qos.durability.clone(),
            deadline: data_reader.qos.deadline.clone(),
            latency_budget: data_reader.qos.latency_budget.clone(),
            liveliness: data_reader.qos.liveliness.clone(),
            reliability: data_reader.qos.reliability.clone(),
            ownership: data_reader.qos.ownership.clone(),
            destination_order: data_reader.qos.destination_order.clone(),
            user_data: data_reader.qos.user_data.clone(),
            time_based_filter: data_reader.qos.time_based_filter.clone(),
            presentation: subscriber.qos.presentation.clone(),
            partition: subscriber.qos.partition.clone(),
            topic_data: topic.qos.topic_data.clone(),
            group_data: subscriber.qos.group_data.clone(),
            representation: data_reader.qos.representation.clone(),
            type_consistency: data_reader.qos.type_consistency.clone(),
        };
        let reader_proxy = ReaderProxy {
            remote_reader_guid: data_reader.transport_reader.guid(),
            remote_group_entity_id: ENTITYID_UNKNOWN,
            unicast_locator_list: vec![],
            multicast_locator_list: vec![],
            expects_inline_qos: false,
        };
        let discovered_reader_data = DiscoveredReaderData {
            dds_subscription_data,
            reader_proxy,
        };

        {
            let dw = &mut self
                .domain_participant
                .builtin_publisher
                .dcps_subscriptions_writer;
            let sample_instance_handle = data_reader.transport_reader.guid().into();
            let serialized_data = discovered_reader_data.into_bytes();
            let sample_timestamp = now;
            dw.write_w_timestamp(
                sample_instance_handle,
                serialized_data,
                sample_timestamp,
                now,
            )
            .ok();
        }
        self.domain_participant
            .builtin_publisher
            .dcps_subscriptions_writer
            .transport_writer
            .write_message(self.transport.message_writer.as_mut(), now);
    }

    #[tracing::instrument(skip(self, data_reader))]
    pub(super) fn announce_deleted_data_reader(
        &mut self,
        data_reader: UserDefinedDataReader,
        now: Time,
    ) {
        let timestamp = now;
        {
            let dw = &mut self
                .domain_participant
                .builtin_publisher
                .dcps_subscriptions_writer;
            let topic_key_data = BuiltInTopicKey {
                value: data_reader.transport_reader.guid().into(),
            };
            let instance_handle = InstanceHandle::new(topic_key_data.value);
            let serialized_key = topic_key_data.into_bytes(PID_ENDPOINT_GUID);

            dw.unregister_w_timestamp(instance_handle, serialized_key, timestamp)
                .ok();
        }
        self.domain_participant
            .builtin_publisher
            .dcps_subscriptions_writer
            .transport_writer
            .write_message(self.transport.message_writer.as_mut(), now);
    }

    #[tracing::instrument(skip(self))]
    pub fn announce_topic(&mut self, topic_name: String, now: Time) {
        let Some(topic) = self
            .domain_participant
            .locally_created_topic_list
            .iter()
            .find(|x| x.topic_name.as_ref() == topic_name.as_str())
        else {
            return;
        };

        let type_information = self
            .domain_participant
            .enable_type_information
            .then(|| topic.type_information.clone());
        let discovered_topic_data = DiscoveredTopicData {
            topic_builtin_topic_data: TopicBuiltinTopicData {
                key: BuiltInTopicKey {
                    value: topic.instance_handle.into(),
                },
                name: topic.topic_name.to_string().into(),
                type_information,
                type_name: topic.type_name.to_string().into(),
                durability: topic.qos.durability.clone(),
                deadline: topic.qos.deadline.clone(),
                latency_budget: topic.qos.latency_budget.clone(),
                liveliness: topic.qos.liveliness.clone(),
                reliability: topic.qos.reliability.clone(),
                transport_priority: topic.qos.transport_priority.clone(),
                lifespan: topic.qos.lifespan.clone(),
                destination_order: topic.qos.destination_order.clone(),
                history: topic.qos.history.clone(),
                resource_limits: topic.qos.resource_limits.clone(),
                ownership: topic.qos.ownership.clone(),
                topic_data: topic.qos.topic_data.clone(),
                representation: topic.qos.representation.clone(),
            },
        };

        {
            let dw = &mut self.domain_participant.builtin_publisher.dcps_topics_writer;
            let sample_instance_handle = topic.instance_handle;
            let serialized_data = discovered_topic_data.into_bytes();
            let sample_timestamp = now;
            dw.write_w_timestamp(
                sample_instance_handle,
                serialized_data,
                sample_timestamp,
                now,
            )
            .ok();
        }
        self.domain_participant
            .builtin_publisher
            .dcps_topics_writer
            .transport_writer
            .write_message(self.transport.message_writer.as_mut(), now);
    }

    #[tracing::instrument(skip(self))]
    pub fn process_discovered_readers(&mut self, now: Time) {
        if self.domain_participant.discovered_reader_list.is_empty()
            || self
                .domain_participant
                .user_defined_publisher_list
                .is_empty()
        {
            return;
        }

        let DomainParticipantEntity {
            discovered_reader_list,
            user_defined_publisher_list,
            discovered_participant_list,
            locally_created_topic_list,
            builtin_publisher,
            type_register,
            domain_id,
            instance_handle: participant_instance_handle,
            listener_mask: participant_listener_mask,
            listener_sender: participant_listener_sender,
            ..
        } = &mut self.domain_participant;

        let domain_id = *domain_id;
        let participant_instance_handle = *participant_instance_handle;
        let participant_listener_mask = *participant_listener_mask;
        let dcps_sender = self.dcps_sender.clone();

        for publisher in user_defined_publisher_list.iter_mut() {
            let publisher_handle = publisher.instance_handle;
            let publisher_qos = publisher.qos.clone();
            let publisher_listener_mask = publisher.listener_mask;
            let publisher_listener_sender = publisher.listener_sender.clone();

            for data_writer in &mut publisher.data_writer_list {
                let writer_topic_name = data_writer.topic_name.clone();

                for discovered_reader_data in discovered_reader_list.iter().filter(|x| {
                    x.dds_subscription_data.topic_name.value.as_str() == writer_topic_name.as_ref()
                }) {
                    if let Some(matched) = data_writer
                        .matched_subscription_list
                        .iter()
                        .find(|x| x.key() == discovered_reader_data.dds_subscription_data.key())
                    {
                        if matched == &discovered_reader_data.dds_subscription_data {
                            continue;
                        }
                    }

                    if is_partition_matched(
                        &discovered_reader_data.dds_subscription_data.partition,
                        &publisher_qos.partition,
                    ) {
                        let is_matched_topic_name = discovered_reader_data
                            .dds_subscription_data
                            .topic_name
                            .value
                            .as_str()
                            == writer_topic_name.as_ref();
                        let writer_associated_topic = locally_created_topic_list
                            .iter_mut()
                            .find(|x| x.topic_name == writer_topic_name)
                            .expect("A matched topic to the writer must exist");

                        let is_matched_type = match &discovered_reader_data
                            .dds_subscription_data
                            .type_information
                        {
                            Some(info) if minimal_type_matches(
                                &writer_associated_topic.type_information.minimal.typeid_with_size,
                                &info.minimal.typeid_with_size,
                            ) => true,
                            Some(info) if discovered_reader_data.dds_subscription_data.type_consistency.kind == TypeConsistencyKind::DisallowTypeCoercion
                                && matches!(info.minimal.typeid_with_size.type_id, TypeIdentifier::EkMinimal { .. }) => false,
                            Some(discovered_type_information)
                            // This additional check is done for interoperability with implementations that
                            // do not communicate the correct type information.
                            // In that case we fallback to matching on type name
                                if discovered_type_information
                                    .complete
                                    .typeid_with_size
                                    .typeobject_serialized_size
                                    > 0 =>
                            {
                                let discovered_type_id =
                                    &discovered_type_information.complete.typeid_with_size.type_id;

                                // If the minimal hash match it is guaranteed compatible
                                if writer_associated_topic
                                    .type_information
                                    .minimal
                                    .typeid_with_size
                                    == discovered_type_information.minimal.typeid_with_size
                                {
                                    true
                                } else if let Some(type_object) =
                                    type_register.get_type_object(discovered_type_id)
                                {
                                    match &type_object {
                                        TypeObject::EkComplete { complete } => {
                                            if let Some(TypeObject::EkComplete {
                                                complete: local_complete,
                                            }) = type_register.get_type_object(
                                                &writer_associated_topic
                                                    .type_information
                                                    .complete
                                                    .typeid_with_size
                                                    .type_id,
                                            ) {
                                                let resolver = |id: &TypeIdentifier| {
                                                    if let Some(TypeObject::EkComplete { complete }) =
                                                        type_register.get_type_object(id)
                                                    {
                                                        Some(complete)
                                                    } else {
                                                        None
                                                    }
                                                };
                                                complete.is_assignable_from_w_type_consistency(
                                                    &local_complete,
                                                    &discovered_reader_data
                                                        .dds_subscription_data
                                                        .type_consistency,
                                                    &resolver,
                                                )
                                            } else {
                                                false
                                            }
                                        }
                                        TypeObject::EkMinimal { minimal } => {
                                            if let Some(TypeObject::EkMinimal {
                                                minimal: local_minimal,
                                            }) = type_register.get_type_object(
                                                &writer_associated_topic
                                                    .type_information
                                                    .minimal
                                                    .typeid_with_size
                                                    .type_id,
                                            ) {
                                                &local_minimal == minimal
                                            } else {
                                                false
                                            }
                                        }
                                    }
                                } else {
                                    if discovered_type_information
                                        .complete
                                        .dependent_typeid_count
                                        != 0
                                    {
                                        if !type_register
                                            .is_dependencies_lookup_pending(discovered_type_id)
                                        {
                                            let type_request_writer = &mut builtin_publisher
                                                .type_lookup_request_writer;

                                            let type_lookup_request = TypeLookupRequest {
                                                header: RequestHeader {
                                                    request_id: SampleIdentity {
                                                        writer_guid: type_request_writer
                                                            .transport_writer
                                                            .guid(),
                                                        sequence_number: (type_request_writer
                                                            .last_change_sequence_number
                                                            + 1)
                                                        .into(),
                                                    },
                                                    instance_name: format!(
                                                        "dds.builtin.TOS.{:x}",
                                                        InstanceHandle::new(
                                                            discovered_reader_data
                                                                .dds_subscription_data
                                                                .participant_key()
                                                                .value
                                                        ),
                                                    ),
                                                },
                                                call:
                                                    TypeLookupCall::TypeLookupGetDependenciesHash {
                                                        get_type_dependencies:
                                                            TypeLookupGetTypeDependenciesIn {
                                                                type_ids: vec![
                                                                    discovered_type_id.clone()
                                                                ],
                                                                continuation_point: Vec::new(),
                                                            },
                                                    },
                                            };
                                            let sample_instance_handle = InstanceHandle::default();
                                            let serialized_data = serialize_cdr2_le(
                                                &type_lookup_request.create_dynamic_sample(),
                                            )
                                            .unwrap();
                                            type_request_writer
                                                .write_w_timestamp(
                                                    sample_instance_handle,
                                                    serialized_data,
                                                    now,
                                                    now,
                                                )
                                                .ok();
                                            type_register.add_pending_dependencies_lookup(
                                                discovered_type_id.clone(),
                                            );
                                        }
                                    } else if !type_register.is_types_lookup_pending(core::slice::from_ref(discovered_type_id)) {
                                        let type_request_writer = &mut builtin_publisher
                                            .type_lookup_request_writer;

                                        let type_lookup_request = TypeLookupRequest {
                                            header: RequestHeader {
                                                request_id: SampleIdentity {
                                                    writer_guid: type_request_writer
                                                         .transport_writer
                                                        .guid(),
                                                    sequence_number: (type_request_writer
                                                        .last_change_sequence_number
                                                        + 1)
                                                        .into(),
                                                },
                                                instance_name: format!(
                                                    "dds.builtin.TOS.{:x}",
                                                    InstanceHandle::new(
                                                        discovered_reader_data
                                                            .dds_subscription_data
                                                            .participant_key()
                                                            .value
                                                    ),
                                                ),
                                            },
                                            call: TypeLookupCall::TypeLookupGetTypesHashId {
                                                get_types: TypeLookupGetTypesIn {
                                                    type_ids: vec![discovered_type_id.clone()],
                                                },
                                            },
                                        };
                                        let sample_instance_handle = InstanceHandle::default();
                                        let serialized_data = serialize_cdr2_le(
                                            &type_lookup_request.create_dynamic_sample(),
                                        )
                                        .unwrap();
                                        type_request_writer
                                            .write_w_timestamp(
                                                sample_instance_handle,
                                                serialized_data,
                                                now,
                                                now,
                                            )
                                            .ok();
                                        type_register.add_pending_types_lookup(vec![
                                            discovered_type_id.clone(),
                                        ]);
                                    }
                                    continue;
                                }
                            }
                            _ => {
                                !discovered_reader_data
                                    .dds_subscription_data
                                    .type_consistency
                                    .force_type_validation
                                    && discovered_reader_data.dds_subscription_data.get_type_name()
                                        == writer_associated_topic.type_name.as_ref()
                            }
                        };

                        if is_matched_topic_name {
                            if is_matched_type {
                                let incompatible_qos_policy_list =
                                    get_discovered_reader_incompatible_qos_policy_list(
                                        &data_writer.qos,
                                        &discovered_reader_data.dds_subscription_data,
                                        &publisher_qos,
                                    );
                                if incompatible_qos_policy_list.is_empty() {
                                    let default_unicast_locator_list = if let Some(p) =
                                        discovered_participant_list.iter().find(|p| {
                                            p.guid_prefix
                                                == discovered_reader_data
                                                    .reader_proxy
                                                    .remote_reader_guid
                                                    .prefix()
                                        }) {
                                        p.default_unicast_locator_list.clone()
                                    } else {
                                        vec![]
                                    };

                                    let default_multicast_locator_list = if let Some(p) =
                                        discovered_participant_list.iter().find(|p| {
                                            p.guid_prefix
                                                == discovered_reader_data
                                                    .reader_proxy
                                                    .remote_reader_guid
                                                    .prefix()
                                        }) {
                                        p.default_multicast_locator_list.clone()
                                    } else {
                                        vec![]
                                    };

                                    match data_writer.matched_subscription_list.iter_mut().find(
                                        |x| {
                                            x.key()
                                                == discovered_reader_data
                                                    .dds_subscription_data
                                                    .key()
                                        },
                                    ) {
                                        Some(x) => {
                                            *x =
                                                discovered_reader_data.dds_subscription_data.clone()
                                        }
                                        None => data_writer.matched_subscription_list.push(
                                            discovered_reader_data.dds_subscription_data.clone(),
                                        ),
                                    };
                                    data_writer.publication_matched_status.current_count =
                                        data_writer.matched_subscription_list.len() as i32;
                                    data_writer.publication_matched_status.current_count_change +=
                                        1;
                                    data_writer.publication_matched_status.total_count += 1;
                                    data_writer.publication_matched_status.total_count_change += 1;

                                    let unicast_locator_list = if discovered_reader_data
                                        .reader_proxy
                                        .unicast_locator_list
                                        .is_empty()
                                    {
                                        default_unicast_locator_list
                                    } else {
                                        discovered_reader_data
                                            .reader_proxy
                                            .unicast_locator_list
                                            .clone()
                                    };
                                    let multicast_locator_list = if discovered_reader_data
                                        .reader_proxy
                                        .multicast_locator_list
                                        .is_empty()
                                    {
                                        default_multicast_locator_list
                                    } else {
                                        discovered_reader_data
                                            .reader_proxy
                                            .multicast_locator_list
                                            .clone()
                                    };
                                    let reliability_kind = match discovered_reader_data
                                        .dds_subscription_data
                                        .reliability
                                        .kind
                                    {
                                        ReliabilityQosPolicyKind::BestEffort => {
                                            ReliabilityKind::BestEffort
                                        }
                                        ReliabilityQosPolicyKind::Reliable => {
                                            ReliabilityKind::Reliable
                                        }
                                    };
                                    let durability_kind = match discovered_reader_data
                                        .dds_subscription_data
                                        .durability
                                        .kind
                                    {
                                        DurabilityQosPolicyKind::Volatile => {
                                            DurabilityKind::Volatile
                                        }
                                        DurabilityQosPolicyKind::TransientLocal => {
                                            DurabilityKind::TransientLocal
                                        }
                                        DurabilityQosPolicyKind::Transient => {
                                            DurabilityKind::Transient
                                        }
                                        DurabilityQosPolicyKind::Persistent => {
                                            DurabilityKind::Persistent
                                        }
                                    };

                                    let reader_proxy = transport::types::ReaderProxy {
                                        remote_reader_guid: discovered_reader_data
                                            .reader_proxy
                                            .remote_reader_guid,
                                        remote_group_entity_id: discovered_reader_data
                                            .reader_proxy
                                            .remote_group_entity_id,
                                        reliability_kind,
                                        durability_kind,
                                        unicast_locator_list,
                                        multicast_locator_list,
                                        expects_inline_qos: false,
                                    };
                                    data_writer
                                        .transport_writer
                                        .add_matched_reader(reader_proxy);

                                    let is_listener_enabled = data_writer
                                        .listener_mask
                                        .is_enabled(&StatusKind::PublicationMatched)
                                        || publisher_listener_mask
                                            .is_enabled(&StatusKind::PublicationMatched)
                                        || participant_listener_mask
                                            .is_enabled(&StatusKind::PublicationMatched);

                                    if is_listener_enabled {
                                        let the_participant = DomainParticipantAsync::new(
                                            dcps_sender.clone(),
                                            domain_id,
                                            participant_instance_handle,
                                        );
                                        let the_publisher = PublisherAsync::new(
                                            publisher_handle,
                                            the_participant.clone(),
                                        );
                                        let the_topic = TopicAsync::new(
                                            writer_associated_topic.instance_handle,
                                            writer_associated_topic.type_name.clone(),
                                            writer_topic_name.clone(),
                                            the_participant,
                                        );
                                        let the_writer = DataWriterAsync::new(
                                            data_writer.instance_handle,
                                            the_publisher,
                                            the_topic,
                                        );
                                        let status = data_writer.publication_matched_status.get();
                                        if data_writer
                                            .listener_mask
                                            .is_enabled(&StatusKind::PublicationMatched)
                                        {
                                            if let Some(l) = &data_writer.listener_sender {
                                                l.send(ListenerMail::PublicationMatched {
                                                    the_writer,
                                                    status,
                                                })
                                                .ok();
                                            }
                                        } else if publisher_listener_mask
                                            .is_enabled(&StatusKind::PublicationMatched)
                                        {
                                            if let Some(l) = &publisher_listener_sender {
                                                l.send(ListenerMail::PublicationMatched {
                                                    the_writer,
                                                    status,
                                                })
                                                .ok();
                                            }
                                        } else if participant_listener_mask
                                            .is_enabled(&StatusKind::PublicationMatched)
                                        {
                                            if let Some(l) = participant_listener_sender {
                                                l.send(ListenerMail::PublicationMatched {
                                                    the_writer,
                                                    status,
                                                })
                                                .ok();
                                            }
                                        }
                                    }

                                    data_writer
                                        .status_condition
                                        .add_communication_state(StatusKind::PublicationMatched);
                                } else {
                                    data_writer
                                        .incompatible_subscriptions
                                        .add_incompatible_subscription(
                                            InstanceHandle::new(
                                                discovered_reader_data
                                                    .dds_subscription_data
                                                    .key()
                                                    .value,
                                            ),
                                            incompatible_qos_policy_list,
                                        );

                                    let is_listener_enabled = data_writer
                                        .listener_mask
                                        .is_enabled(&StatusKind::OfferedIncompatibleQos)
                                        || publisher_listener_mask
                                            .is_enabled(&StatusKind::OfferedIncompatibleQos)
                                        || participant_listener_mask
                                            .is_enabled(&StatusKind::OfferedIncompatibleQos);

                                    if is_listener_enabled {
                                        let the_participant = DomainParticipantAsync::new(
                                            dcps_sender.clone(),
                                            domain_id,
                                            participant_instance_handle,
                                        );
                                        let the_publisher = PublisherAsync::new(
                                            publisher_handle,
                                            the_participant.clone(),
                                        );
                                        let the_topic = TopicAsync::new(
                                            writer_associated_topic.instance_handle,
                                            writer_associated_topic.type_name.clone(),
                                            writer_topic_name.clone(),
                                            the_participant,
                                        );
                                        let the_writer = DataWriterAsync::new(
                                            data_writer.instance_handle,
                                            the_publisher,
                                            the_topic,
                                        );
                                        let status = data_writer
                                            .incompatible_subscriptions
                                            .get_offered_incompatible_qos_status();

                                        if data_writer
                                            .listener_mask
                                            .is_enabled(&StatusKind::OfferedIncompatibleQos)
                                        {
                                            if let Some(l) = &data_writer.listener_sender {
                                                l.send(ListenerMail::OfferedIncompatibleQos {
                                                    the_writer,
                                                    status,
                                                })
                                                .ok();
                                            }
                                        } else if publisher_listener_mask
                                            .is_enabled(&StatusKind::OfferedIncompatibleQos)
                                        {
                                            if let Some(l) = &publisher_listener_sender {
                                                l.send(ListenerMail::OfferedIncompatibleQos {
                                                    the_writer,
                                                    status,
                                                })
                                                .ok();
                                            }
                                        } else if participant_listener_mask
                                            .is_enabled(&StatusKind::OfferedIncompatibleQos)
                                        {
                                            if let Some(l) = participant_listener_sender {
                                                l.send(ListenerMail::OfferedIncompatibleQos {
                                                    the_writer,
                                                    status,
                                                })
                                                .ok();
                                            }
                                        }
                                    }

                                    data_writer.status_condition.add_communication_state(
                                        StatusKind::OfferedIncompatibleQos,
                                    );
                                }
                            } else {
                                writer_associated_topic
                                    .inconsistent_topic_status
                                    .total_count += 1;
                                writer_associated_topic
                                    .inconsistent_topic_status
                                    .total_count_change += 1;

                                let is_listener_enabled = writer_associated_topic
                                    .listener_mask
                                    .is_enabled(&StatusKind::InconsistentTopic)
                                    || participant_listener_mask
                                        .is_enabled(&StatusKind::InconsistentTopic);

                                if is_listener_enabled {
                                    let participant = DomainParticipantAsync::new(
                                        dcps_sender.clone(),
                                        domain_id,
                                        participant_instance_handle,
                                    );
                                    let the_topic = TopicAsync::new(
                                        writer_associated_topic.instance_handle,
                                        writer_associated_topic.type_name.clone(),
                                        writer_topic_name.clone(),
                                        participant,
                                    );
                                    if writer_associated_topic
                                        .listener_mask
                                        .is_enabled(&StatusKind::InconsistentTopic)
                                    {
                                        let status = writer_associated_topic
                                            .inconsistent_topic_status
                                            .get_inconsistent_topic_status();
                                        if let Some(l) = &writer_associated_topic.listener_sender {
                                            l.send(ListenerMail::InconsistentTopic {
                                                the_topic,
                                                status,
                                            })
                                            .ok();
                                        }
                                    } else if participant_listener_mask
                                        .is_enabled(&StatusKind::InconsistentTopic)
                                    {
                                        let status = writer_associated_topic
                                            .inconsistent_topic_status
                                            .get_inconsistent_topic_status();
                                        if let Some(l) = participant_listener_sender {
                                            l.send(ListenerMail::InconsistentTopic {
                                                the_topic,
                                                status,
                                            })
                                            .ok();
                                        }
                                    }
                                }
                                writer_associated_topic
                                    .status_condition
                                    .add_communication_state(StatusKind::InconsistentTopic);
                            }
                        }
                    }
                }
            }
        }
    }

    #[tracing::instrument(skip(self))]
    pub(crate) fn remove_discovered_reader(
        &mut self,
        subscription_handle: InstanceHandle,
        publisher_handle: InstanceHandle,
        data_writer_handle: InstanceHandle,
    ) {
        let Some(publisher) = self
            .domain_participant
            .user_defined_publisher_list
            .iter_mut()
            .find(|x| x.instance_handle == publisher_handle)
        else {
            return;
        };
        let Some(data_writer) = publisher
            .data_writer_list
            .iter_mut()
            .find(|x| x.instance_handle == data_writer_handle)
        else {
            return;
        };
        if data_writer
            .matched_subscription_list
            .iter()
            .any(|x| subscription_handle.as_ref() == &x.key().value)
        {
            data_writer.remove_matched_subscription(&subscription_handle);

            data_writer
                .status_condition
                .add_communication_state(StatusKind::PublicationMatched);
        }
    }

    #[tracing::instrument(skip(self))]
    pub fn process_discovered_writers(&mut self, now: Time) {
        if self.domain_participant.discovered_writer_list.is_empty()
            || self
                .domain_participant
                .user_defined_subscriber_list
                .is_empty()
        {
            return;
        }

        let DomainParticipantEntity {
            discovered_writer_list,
            user_defined_subscriber_list,
            discovered_participant_list,
            content_filtered_topic_list,
            locally_created_topic_list,
            builtin_publisher,
            type_register,
            domain_id,
            instance_handle: participant_instance_handle,
            listener_mask: participant_listener_mask,
            listener_sender: participant_listener_sender,
            ..
        } = &mut self.domain_participant;

        let domain_id = *domain_id;
        let participant_instance_handle = *participant_instance_handle;
        let participant_listener_mask = *participant_listener_mask;
        let dcps_sender = self.dcps_sender.clone();

        for subscriber in user_defined_subscriber_list.iter_mut() {
            let subscriber_handle = subscriber.instance_handle;
            let subscriber_qos = subscriber.qos.clone();
            let subscriber_listener_mask = subscriber.listener_mask;
            let subscriber_listener_sender = subscriber.listener_sender.clone();
            for data_reader in &mut subscriber.data_reader_list {
                let reader_topic_name = if let Some(matched_topic) = content_filtered_topic_list
                    .iter()
                    .find(|t| t.topic_name == data_reader.topic_name)
                {
                    matched_topic.related_topic_name.clone()
                } else {
                    data_reader.topic_name.clone()
                };

                for discovered_writer_data in discovered_writer_list
                    .iter()
                    .filter(|x| x.dds_publication_data.topic_name() == reader_topic_name.as_ref())
                {
                    if let Some(matched) = data_reader
                        .matched_publication_list
                        .iter()
                        .find(|x| x.key() == discovered_writer_data.dds_publication_data.key())
                    {
                        if matched == &discovered_writer_data.dds_publication_data {
                            continue;
                        }
                    }

                    if is_partition_matched(
                        &discovered_writer_data.dds_publication_data.partition,
                        &subscriber_qos.partition,
                    ) {
                        let reader_associated_topic = if let Some(matched_topic) =
                            content_filtered_topic_list
                                .iter()
                                .find(|t| t.topic_name == data_reader.topic_name)
                        {
                            if let Some(t) = locally_created_topic_list
                                .iter_mut()
                                .find(|x| x.topic_name == matched_topic.related_topic_name)
                            {
                                t
                            } else {
                                continue;
                            }
                        } else if let Some(t) = locally_created_topic_list
                            .iter_mut()
                            .find(|x| x.topic_name == data_reader.topic_name)
                        {
                            t
                        } else {
                            continue;
                        };

                        let is_matched_topic_name =
                            discovered_writer_data.dds_publication_data.topic_name()
                                == reader_associated_topic.topic_name.as_ref();

                        let is_matched_type = match &discovered_writer_data
                            .dds_publication_data
                            .type_information
                        {
                            Some(info) if minimal_type_matches(
                                &reader_associated_topic.type_information.minimal.typeid_with_size,
                                &info.minimal.typeid_with_size,
                            ) => true,
                            Some(info) if data_reader.qos.type_consistency.kind == TypeConsistencyKind::DisallowTypeCoercion
                                && matches!(info.minimal.typeid_with_size.type_id, TypeIdentifier::EkMinimal { .. }) => false,
                            Some(discovered_type_information)
                            // This additional check is done for interoperability with implementations that
                            // do not communicate the correct type information.
                            // In that case we fallback to matching on type name
                                if discovered_type_information
                                    .complete
                                    .typeid_with_size
                                    .typeobject_serialized_size
                                    > 0 =>
                            {
                                let discovered_type_id =
                                    &discovered_type_information.complete.typeid_with_size.type_id;

                                // If the minimal hash match it is guaranteed compatible
                                if reader_associated_topic
                                    .type_information
                                    .minimal
                                    .typeid_with_size
                                    == discovered_type_information.minimal.typeid_with_size
                                {
                                    true
                                } else if let Some(type_object) =
                                    type_register.get_type_object(discovered_type_id)
                                {
                                    match &type_object {
                                        TypeObject::EkComplete { complete } => {
                                            if let Some(TypeObject::EkComplete {
                                                complete: local_complete,
                                            }) = type_register.get_type_object(
                                                &reader_associated_topic
                                                    .type_information
                                                    .complete
                                                    .typeid_with_size
                                                    .type_id,
                                            ) {
                                                let resolver = |id: &TypeIdentifier| {
                                                    if let Some(TypeObject::EkComplete { complete }) =
                                                        type_register.get_type_object(id)
                                                    {
                                                        Some(complete)
                                                    } else {
                                                        None
                                                    }
                                                };
                                                local_complete
                                                    .is_assignable_from_w_type_consistency(
                                                        complete,
                                                        &data_reader.qos.type_consistency,
                                                        &resolver,
                                                    )
                                            } else {
                                                false
                                            }
                                        }
                                        TypeObject::EkMinimal { minimal } => {
                                            if let Some(TypeObject::EkMinimal {
                                                minimal: local_minimal,
                                            }) = type_register.get_type_object(
                                                &reader_associated_topic
                                                    .type_information
                                                    .minimal
                                                    .typeid_with_size
                                                    .type_id,
                                            ) {
                                                &local_minimal == minimal
                                            } else {
                                                false
                                            }
                                        }
                                    }
                                } else {
                                    if discovered_type_information
                                        .complete
                                        .dependent_typeid_count
                                        != 0
                                    {
                                        if !type_register
                                            .is_dependencies_lookup_pending(discovered_type_id)
                                        {
                                            let type_request_writer = &mut builtin_publisher
                                                .type_lookup_request_writer;

                                            let type_lookup_request = TypeLookupRequest {
                                                header: RequestHeader {
                                                    request_id: SampleIdentity {
                                                        writer_guid: type_request_writer
                                                            .transport_writer
                                                            .guid(),
                                                        sequence_number: (type_request_writer
                                                            .last_change_sequence_number
                                                            + 1)
                                                        .into(),
                                                    },
                                                    instance_name: format!(
                                                        "dds.builtin.TOS.{:x}",
                                                        InstanceHandle::new(
                                                            discovered_writer_data
                                                                .dds_publication_data
                                                                .participant_key()
                                                                .value
                                                        ),
                                                    ),
                                                },
                                                call:
                                                    TypeLookupCall::TypeLookupGetDependenciesHash {
                                                        get_type_dependencies:
                                                            TypeLookupGetTypeDependenciesIn {
                                                                type_ids: vec![
                                                                    discovered_type_id.clone()
                                                                ],
                                                                continuation_point: Vec::new(),
                                                            },
                                                    },
                                            };
                                            let sample_instance_handle = InstanceHandle::default();
                                            let serialized_data = serialize_cdr2_le(
                                                &type_lookup_request.create_dynamic_sample(),
                                            )
                                            .unwrap();
                                            type_request_writer
                                                .write_w_timestamp(
                                                    sample_instance_handle,
                                                    serialized_data,
                                                    now,
                                                    now,
                                                )
                                                .ok();
                                            type_register.add_pending_dependencies_lookup(
                                                discovered_type_id.clone(),
                                            );
                                        }
                                    } else if !type_register.is_types_lookup_pending(core::slice::from_ref(discovered_type_id)) {
                                        let type_request_writer = &mut builtin_publisher
                                            .type_lookup_request_writer;

                                        let type_lookup_request = TypeLookupRequest {
                                            header: RequestHeader {
                                                request_id: SampleIdentity {
                                                    writer_guid: type_request_writer
                                                        .transport_writer
                                                        .guid(),
                                                    sequence_number: (type_request_writer
                                                        .last_change_sequence_number
                                                        + 1)
                                                        .into(),
                                                },
                                                instance_name: format!(
                                                    "dds.builtin.TOS.{:x}",
                                                    InstanceHandle::new(
                                                        discovered_writer_data
                                                            .dds_publication_data
                                                            .participant_key()
                                                            .value
                                                    ),
                                                ),
                                            },
                                            call: TypeLookupCall::TypeLookupGetTypesHashId {
                                                get_types: TypeLookupGetTypesIn {
                                                    type_ids: vec![discovered_type_id.clone()],
                                                },
                                            },
                                        };
                                        let sample_instance_handle = InstanceHandle::default();
                                        let serialized_data = serialize_cdr2_le(
                                            &type_lookup_request.create_dynamic_sample(),
                                        )
                                        .unwrap();
                                        type_request_writer
                                            .write_w_timestamp(
                                                sample_instance_handle,
                                                serialized_data,
                                                now,
                                                now,
                                            )
                                            .ok();
                                        type_register.add_pending_types_lookup(vec![
                                            discovered_type_id.clone(),
                                        ]);
                                    }
                                    continue;
                                }
                            }
                            _ => {
                                !data_reader.qos.type_consistency.force_type_validation
                                    && discovered_writer_data.dds_publication_data.get_type_name()
                                        == reader_associated_topic.type_name.as_ref()
                            }
                        };

                        if is_matched_topic_name {
                            if is_matched_type {
                                let incompatible_qos_policy_list =
                                    get_discovered_writer_incompatible_qos_policy_list(
                                        data_reader,
                                        &discovered_writer_data.dds_publication_data,
                                        &subscriber_qos,
                                    );
                                if incompatible_qos_policy_list.is_empty() {
                                    let default_unicast_locator_list = if let Some(p) =
                                        discovered_participant_list.iter().find(|p| {
                                            p.guid_prefix
                                                == discovered_writer_data
                                                    .writer_proxy
                                                    .remote_writer_guid
                                                    .prefix()
                                        }) {
                                        p.default_unicast_locator_list.clone()
                                    } else {
                                        vec![]
                                    };

                                    let default_multicast_locator_list = if let Some(p) =
                                        discovered_participant_list.iter().find(|p| {
                                            p.guid_prefix
                                                == discovered_writer_data
                                                    .writer_proxy
                                                    .remote_writer_guid
                                                    .prefix()
                                        }) {
                                        p.default_multicast_locator_list.clone()
                                    } else {
                                        vec![]
                                    };

                                    data_reader.add_matched_publication(
                                        discovered_writer_data.dds_publication_data.clone(),
                                    );

                                    let unicast_locator_list = if discovered_writer_data
                                        .writer_proxy
                                        .unicast_locator_list
                                        .is_empty()
                                    {
                                        default_unicast_locator_list
                                    } else {
                                        discovered_writer_data
                                            .writer_proxy
                                            .unicast_locator_list
                                            .clone()
                                    };
                                    let multicast_locator_list = if discovered_writer_data
                                        .writer_proxy
                                        .multicast_locator_list
                                        .is_empty()
                                    {
                                        default_multicast_locator_list
                                    } else {
                                        discovered_writer_data
                                            .writer_proxy
                                            .multicast_locator_list
                                            .clone()
                                    };
                                    let reliability_kind = match data_reader.qos.reliability.kind {
                                        ReliabilityQosPolicyKind::BestEffort => {
                                            ReliabilityKind::BestEffort
                                        }
                                        ReliabilityQosPolicyKind::Reliable => {
                                            ReliabilityKind::Reliable
                                        }
                                    };
                                    let durability_kind = match data_reader.qos.durability.kind {
                                        DurabilityQosPolicyKind::Volatile => {
                                            DurabilityKind::Volatile
                                        }
                                        DurabilityQosPolicyKind::TransientLocal => {
                                            DurabilityKind::TransientLocal
                                        }
                                        DurabilityQosPolicyKind::Transient => {
                                            DurabilityKind::Transient
                                        }
                                        DurabilityQosPolicyKind::Persistent => {
                                            DurabilityKind::Persistent
                                        }
                                    };
                                    let writer_proxy = transport::types::WriterProxy {
                                        remote_writer_guid: discovered_writer_data
                                            .writer_proxy
                                            .remote_writer_guid,
                                        remote_group_entity_id: discovered_writer_data
                                            .writer_proxy
                                            .remote_group_entity_id,
                                        unicast_locator_list,
                                        multicast_locator_list,
                                        reliability_kind,
                                        durability_kind,
                                    };
                                    data_reader
                                        .transport_reader
                                        .add_matched_writer(&writer_proxy);

                                    let is_listener_enabled = data_reader
                                        .listener_mask
                                        .is_enabled(&StatusKind::SubscriptionMatched)
                                        || subscriber_listener_mask
                                            .is_enabled(&StatusKind::SubscriptionMatched)
                                        || participant_listener_mask
                                            .is_enabled(&StatusKind::SubscriptionMatched);

                                    if is_listener_enabled {
                                        let the_participant = DomainParticipantAsync::new(
                                            dcps_sender.clone(),
                                            domain_id,
                                            participant_instance_handle,
                                        );
                                        let the_subscriber = SubscriberAsync::new(
                                            subscriber_handle,
                                            the_participant.clone(),
                                        );
                                        let the_reader = DataReaderAsync::new(
                                            data_reader.instance_handle,
                                            the_subscriber,
                                            data_reader.topic_name.clone(),
                                            reader_associated_topic.type_name.clone(),
                                        );
                                        let status = data_reader.get_subscription_matched_status();
                                        if data_reader
                                            .listener_mask
                                            .is_enabled(&StatusKind::SubscriptionMatched)
                                        {
                                            if let Some(l) = &data_reader.listener_sender {
                                                l.send(ListenerMail::SubscriptionMatched {
                                                    the_reader,
                                                    status,
                                                })
                                                .ok();
                                            }
                                        } else if subscriber_listener_mask
                                            .is_enabled(&StatusKind::SubscriptionMatched)
                                        {
                                            if let Some(l) = &subscriber_listener_sender {
                                                l.send(ListenerMail::SubscriptionMatched {
                                                    the_reader,
                                                    status,
                                                })
                                                .ok();
                                            }
                                        } else if participant_listener_mask
                                            .is_enabled(&StatusKind::SubscriptionMatched)
                                        {
                                            if let Some(l) = participant_listener_sender {
                                                l.send(ListenerMail::SubscriptionMatched {
                                                    the_reader,
                                                    status,
                                                })
                                                .ok();
                                            }
                                        }
                                    }

                                    data_reader
                                        .status_condition
                                        .add_communication_state(StatusKind::SubscriptionMatched);
                                } else {
                                    data_reader.add_requested_incompatible_qos(
                                        InstanceHandle::new(
                                            discovered_writer_data.dds_publication_data.key().value,
                                        ),
                                        incompatible_qos_policy_list,
                                    );

                                    let is_listener_enabled = data_reader
                                        .listener_mask
                                        .is_enabled(&StatusKind::RequestedIncompatibleQos)
                                        || subscriber_listener_mask
                                            .is_enabled(&StatusKind::RequestedIncompatibleQos)
                                        || participant_listener_mask
                                            .is_enabled(&StatusKind::RequestedIncompatibleQos);

                                    if is_listener_enabled {
                                        let the_participant = DomainParticipantAsync::new(
                                            dcps_sender.clone(),
                                            domain_id,
                                            participant_instance_handle,
                                        );
                                        let the_subscriber = SubscriberAsync::new(
                                            subscriber_handle,
                                            the_participant.clone(),
                                        );
                                        let the_reader = DataReaderAsync::new(
                                            data_reader.instance_handle,
                                            the_subscriber,
                                            data_reader.topic_name.clone(),
                                            reader_associated_topic.type_name.clone(),
                                        );
                                        let status =
                                            data_reader.get_requested_incompatible_qos_status();
                                        if data_reader
                                            .listener_mask
                                            .is_enabled(&StatusKind::RequestedIncompatibleQos)
                                        {
                                            if let Some(l) = &data_reader.listener_sender {
                                                l.send(ListenerMail::RequestedIncompatibleQos {
                                                    the_reader,
                                                    status,
                                                })
                                                .ok();
                                            }
                                        } else if subscriber_listener_mask
                                            .is_enabled(&StatusKind::RequestedIncompatibleQos)
                                        {
                                            if let Some(l) = &subscriber_listener_sender {
                                                l.send(ListenerMail::RequestedIncompatibleQos {
                                                    the_reader,
                                                    status,
                                                })
                                                .ok();
                                            }
                                        } else if participant_listener_mask
                                            .is_enabled(&StatusKind::RequestedIncompatibleQos)
                                        {
                                            if let Some(l) = participant_listener_sender {
                                                l.send(ListenerMail::RequestedIncompatibleQos {
                                                    the_reader,
                                                    status,
                                                })
                                                .ok();
                                            }
                                        }
                                    }

                                    data_reader.status_condition.add_communication_state(
                                        StatusKind::RequestedIncompatibleQos,
                                    );
                                }
                            } else {
                                reader_associated_topic
                                    .inconsistent_topic_status
                                    .total_count += 1;
                                reader_associated_topic
                                    .inconsistent_topic_status
                                    .total_count_change += 1;

                                let is_listener_enabled = reader_associated_topic
                                    .listener_mask
                                    .is_enabled(&StatusKind::InconsistentTopic)
                                    || participant_listener_mask
                                        .is_enabled(&StatusKind::InconsistentTopic);

                                if is_listener_enabled {
                                    let participant = DomainParticipantAsync::new(
                                        dcps_sender.clone(),
                                        domain_id,
                                        participant_instance_handle,
                                    );
                                    let the_topic = TopicAsync::new(
                                        reader_associated_topic.instance_handle,
                                        reader_associated_topic.type_name.clone(),
                                        reader_associated_topic.topic_name.clone(),
                                        participant,
                                    );
                                    if reader_associated_topic
                                        .listener_mask
                                        .is_enabled(&StatusKind::InconsistentTopic)
                                    {
                                        let status = reader_associated_topic
                                            .inconsistent_topic_status
                                            .get_inconsistent_topic_status();
                                        if let Some(l) = &reader_associated_topic.listener_sender {
                                            l.send(ListenerMail::InconsistentTopic {
                                                the_topic,
                                                status,
                                            })
                                            .ok();
                                        }
                                    } else if participant_listener_mask
                                        .is_enabled(&StatusKind::InconsistentTopic)
                                    {
                                        let status = reader_associated_topic
                                            .inconsistent_topic_status
                                            .get_inconsistent_topic_status();
                                        if let Some(l) = participant_listener_sender {
                                            l.send(ListenerMail::InconsistentTopic {
                                                the_topic,
                                                status,
                                            })
                                            .ok();
                                        }
                                    }
                                }
                                reader_associated_topic
                                    .status_condition
                                    .add_communication_state(StatusKind::InconsistentTopic);
                            }
                        }
                    }
                }
            }
        }
    }

    pub fn handle_type_lookup_request(
        &mut self,
        type_lookup_request: TypeLookupRequest,
        now: Time,
    ) {
        match type_lookup_request.call {
            TypeLookupCall::TypeLookupGetTypesHashId { get_types } => {
                let mut types = Vec::new();
                for type_id in get_types.type_ids {
                    if let Some(type_object) = self
                        .domain_participant
                        .type_register
                        .get_type_object(&type_id)
                    {
                        types.push(TypeIdentifierTypeObjectPair {
                            type_identifier: type_id,
                            type_object,
                        });
                    }
                }
                if !types.is_empty() {
                    let type_lookup_reply_writer = &mut self
                        .domain_participant
                        .builtin_publisher
                        .type_lookup_reply_writer;
                    let type_lookup_reply = TypeLookupReply {
                        header: ReplyHeader {
                            related_request_id: type_lookup_request.header.request_id.clone(),
                            remote_ex: RemoteExceptionCode::Ok,
                        },
                        r#return: TypeLookupReturn::TypeLookupGetTypesHash {
                            get_type: TypeLookupGetTypesResult::Ok {
                                result: TypeLookupGetTypesOut {
                                    types,
                                    complete_to_minimal: Vec::new(),
                                },
                            },
                        },
                    };
                    let serialized_data =
                        serialize_cdr2_le(&type_lookup_reply.create_dynamic_sample()).unwrap();

                    type_lookup_reply_writer
                        .write_w_timestamp(InstanceHandle::default(), serialized_data, now, now)
                        .ok();
                    type_lookup_reply_writer
                        .transport_writer
                        .write_message(self.transport.message_writer.as_mut(), now);
                }
            }
            TypeLookupCall::TypeLookupGetDependenciesHash {
                get_type_dependencies,
            } => {
                for type_id in get_type_dependencies.type_ids {
                    if let Some(dependent_typeids) = self
                        .domain_participant
                        .type_register
                        .get_type_dependencies_with_size(&type_id)
                    {
                        let type_lookup_reply_writer = &mut self
                            .domain_participant
                            .builtin_publisher
                            .type_lookup_reply_writer;
                        let type_lookup_reply = TypeLookupReply {
                            header: ReplyHeader {
                                related_request_id: type_lookup_request.header.request_id.clone(),
                                remote_ex: RemoteExceptionCode::Ok,
                            },
                            r#return: TypeLookupReturn::TypeLookupGetDependenciesHash {
                                get_type_dependencies: TypeLookupGetTypeDependenciesResult::Ok {
                                    result: TypeLookupGetTypeDependenciesOut {
                                        dependent_typeids,
                                        continuation_point: Vec::new(),
                                    },
                                },
                            },
                        };
                        let serialized_data =
                            serialize_cdr2_le(&type_lookup_reply.create_dynamic_sample()).unwrap();

                        type_lookup_reply_writer
                            .write_w_timestamp(InstanceHandle::default(), serialized_data, now, now)
                            .ok();
                        type_lookup_reply_writer
                            .transport_writer
                            .write_message(self.transport.message_writer.as_mut(), now);
                    }
                }
            }
        }
    }

    pub fn handle_type_lookup_reply(
        &mut self,
        type_lookup_reply: TypeLookupReply,
        now: Time,
    ) -> bool {
        let mut type_lookup_reply_received = false;
        match &type_lookup_reply.r#return {
            TypeLookupReturn::TypeLookupGetDependenciesHash {
                get_type_dependencies: TypeLookupGetTypeDependenciesResult::Ok { result },
            } => {
                let pending_id = self
                    .domain_participant
                    .type_register
                    .get_pending_dependencies_type_id();

                if let Some(type_id) = pending_id {
                    self.domain_participant
                        .type_register
                        .remove_pending_dependencies_lookup(&type_id);
                    self.domain_participant
                        .type_register
                        .register_type_dependencies(&type_id, result.dependent_typeids.clone());

                    let unresolved = self
                        .domain_participant
                        .type_register
                        .get_unresolved_type_ids(&type_id);
                    if !unresolved.is_empty()
                        && !self
                            .domain_participant
                            .type_register
                            .is_types_lookup_pending(&unresolved)
                    {
                        let type_request_writer = &mut self
                            .domain_participant
                            .builtin_publisher
                            .type_lookup_request_writer;

                        let type_lookup_request = TypeLookupRequest {
                            header: RequestHeader {
                                request_id: SampleIdentity {
                                    writer_guid: type_request_writer.transport_writer.guid(),
                                    sequence_number: (type_request_writer
                                        .last_change_sequence_number
                                        + 1)
                                    .into(),
                                },
                                instance_name: format!(
                                    "dds.builtin.TOS.{:x}",
                                    self.domain_participant.instance_handle,
                                ),
                            },
                            call: TypeLookupCall::TypeLookupGetTypesHashId {
                                get_types: TypeLookupGetTypesIn {
                                    type_ids: unresolved.clone(),
                                },
                            },
                        };
                        let sample_instance_handle = InstanceHandle::default();
                        let serialized_data =
                            serialize_cdr2_le(&type_lookup_request.create_dynamic_sample())
                                .unwrap();
                        type_request_writer
                            .write_w_timestamp(sample_instance_handle, serialized_data, now, now)
                            .ok();
                        self.domain_participant
                            .type_register
                            .add_pending_types_lookup(unresolved);
                    }
                }
            }
            TypeLookupReturn::TypeLookupGetTypesHash {
                get_type: TypeLookupGetTypesResult::Ok { result },
            } => {
                let mut received_ids = Vec::new();
                for type_identifier_pair in &result.types {
                    received_ids.push(type_identifier_pair.type_identifier.clone());
                    self.domain_participant
                        .type_register
                        .register_discovered_type_object(
                            type_identifier_pair.type_identifier.clone(),
                            type_identifier_pair.type_object.clone(),
                        );
                    type_lookup_reply_received = true;

                    for topic in &mut self.domain_participant.locally_created_topic_list {
                        let matches_discovered_topic = self
                            .domain_participant
                            .discovered_topic_list
                            .iter()
                            .any(|dt| {
                                dt.name.value.as_str() == topic.topic_name.as_ref()
                                    && dt.type_information.as_ref().is_some_and(|ti| {
                                        ti.complete.typeid_with_size.type_id
                                            == type_identifier_pair.type_identifier
                                    })
                            });

                        if !matches_discovered_topic {
                            continue;
                        }

                        let local_has_readers = self
                            .domain_participant
                            .user_defined_subscriber_list
                            .iter()
                            .flat_map(|s| s.data_reader_list.iter())
                            .any(|dr| dr.topic_name == topic.topic_name);
                        let discovered_has_readers = self
                            .domain_participant
                            .discovered_reader_list
                            .iter()
                            .any(|dr| {
                                dr.dds_subscription_data.topic_name() == topic.topic_name.as_ref()
                            });

                        let ignore_sequence_bounds =
                            if !local_has_readers && !discovered_has_readers {
                                true
                            } else {
                                self.domain_participant
                                    .user_defined_subscriber_list
                                    .iter()
                                    .flat_map(|s| s.data_reader_list.iter())
                                    .all(|dr| {
                                        dr.topic_name != topic.topic_name
                                            || dr.qos.type_consistency.ignore_sequence_bounds
                                    })
                                    && self.domain_participant.discovered_reader_list.iter().all(
                                        |dr| {
                                            dr.dds_subscription_data.topic_name()
                                                != topic.topic_name.as_ref()
                                                || dr
                                                    .dds_subscription_data
                                                    .type_consistency
                                                    .ignore_sequence_bounds
                                        },
                                    )
                            };
                        let ignore_string_bounds =
                            if !local_has_readers && !discovered_has_readers {
                                true
                            } else {
                                self.domain_participant
                                    .user_defined_subscriber_list
                                    .iter()
                                    .flat_map(|s| s.data_reader_list.iter())
                                    .all(|dr| {
                                        dr.topic_name != topic.topic_name
                                            || dr.qos.type_consistency.ignore_string_bounds
                                    })
                                    && self.domain_participant.discovered_reader_list.iter().all(
                                        |dr| {
                                            dr.dds_subscription_data.topic_name()
                                                != topic.topic_name.as_ref()
                                                || dr
                                                    .dds_subscription_data
                                                    .type_consistency
                                                    .ignore_string_bounds
                                        },
                                    )
                            };
                        let ignore_member_names =
                            if !local_has_readers && !discovered_has_readers {
                                true
                            } else {
                                self.domain_participant
                                    .user_defined_subscriber_list
                                    .iter()
                                    .flat_map(|s| s.data_reader_list.iter())
                                    .all(|dr| {
                                        dr.topic_name != topic.topic_name
                                            || dr.qos.type_consistency.ignore_member_names
                                    })
                                    && self.domain_participant.discovered_reader_list.iter().all(
                                        |dr| {
                                            dr.dds_subscription_data.topic_name()
                                                != topic.topic_name.as_ref()
                                                || dr
                                                    .dds_subscription_data
                                                    .type_consistency
                                                    .ignore_member_names
                                        },
                                    )
                            };
                        let prevent_type_widening =
                            if !local_has_readers && !discovered_has_readers {
                                false
                            } else {
                                self.domain_participant
                                    .user_defined_subscriber_list
                                    .iter()
                                    .flat_map(|s| s.data_reader_list.iter())
                                    .any(|dr| {
                                        dr.topic_name == topic.topic_name
                                            && dr.qos.type_consistency.prevent_type_widening
                                    })
                                    || self.domain_participant.discovered_reader_list.iter().any(
                                        |dr| {
                                            dr.dds_subscription_data.topic_name()
                                                == topic.topic_name.as_ref()
                                                && dr
                                                    .dds_subscription_data
                                                    .type_consistency
                                                    .prevent_type_widening
                                        },
                                    )
                            };
                        let topic_type_consistency = TypeConsistencyEnforcementQosPolicy {
                            ignore_member_names,
                            ignore_sequence_bounds,
                            ignore_string_bounds,
                            prevent_type_widening,
                            ..TypeConsistencyEnforcementQosPolicy::const_default()
                        };

                        let is_type_assignable = match &type_identifier_pair.type_object {
                            TypeObject::EkComplete { complete } => {
                                if let Some(TypeObject::EkComplete {
                                    complete: local_type,
                                }) = self.domain_participant.type_register.get_type_object(
                                    &topic.type_information.complete.typeid_with_size.type_id,
                                ) {
                                    let resolver = |id: &TypeIdentifier| {
                                        if let Some(TypeObject::EkComplete { complete }) = self
                                            .domain_participant
                                            .type_register
                                            .get_type_object(id)
                                        {
                                            Some(complete)
                                        } else {
                                            None
                                        }
                                    };
                                    local_type.is_assignable_from_w_type_consistency(
                                        complete,
                                        &topic_type_consistency,
                                        &resolver,
                                    ) || complete.is_assignable_from_w_type_consistency(
                                        &local_type,
                                        &topic_type_consistency,
                                        &resolver,
                                    )
                                } else {
                                    false
                                }
                            }
                            TypeObject::EkMinimal { minimal } => {
                                if let Some(TypeObject::EkMinimal {
                                    minimal: local_type,
                                }) = self.domain_participant.type_register.get_type_object(
                                    &topic.type_information.minimal.typeid_with_size.type_id,
                                ) {
                                    &local_type == minimal
                                } else {
                                    false
                                }
                            }
                        };

                        if !is_type_assignable {
                            topic.inconsistent_topic_status.total_count += 1;
                            topic.inconsistent_topic_status.total_count_change += 1;
                            let participant = DomainParticipantAsync::new(
                                self.dcps_sender.clone(),
                                self.domain_participant.domain_id,
                                self.domain_participant.instance_handle,
                            );
                            let the_topic = TopicAsync::new(
                                topic.instance_handle,
                                topic.type_name.clone(),
                                topic.topic_name.clone(),
                                participant,
                            );
                            if topic
                                .listener_mask
                                .is_enabled(&StatusKind::InconsistentTopic)
                            {
                                let status = topic
                                    .inconsistent_topic_status
                                    .get_inconsistent_topic_status();
                                if let Some(l) = &topic.listener_sender {
                                    l.send(ListenerMail::InconsistentTopic { the_topic, status })
                                        .ok();
                                }
                            } else if self
                                .domain_participant
                                .listener_mask
                                .is_enabled(&StatusKind::InconsistentTopic)
                            {
                                let status = topic
                                    .inconsistent_topic_status
                                    .get_inconsistent_topic_status();
                                if let Some(l) = &self.domain_participant.listener_sender {
                                    l.send(ListenerMail::InconsistentTopic { the_topic, status })
                                        .ok();
                                }
                            }
                            topic
                                .status_condition
                                .add_communication_state(StatusKind::InconsistentTopic);
                        }
                    }
                }
                self.domain_participant
                    .type_register
                    .remove_pending_types_lookup(&received_ids);
            }
        }
        type_lookup_reply_received
    }

    pub fn request_topic_type_representation(&mut self, now: Time) {
        if self.domain_participant.discovered_topic_list.is_empty()
            || self
                .domain_participant
                .locally_created_topic_list
                .is_empty()
        {
            return;
        }
        for topic in &self.domain_participant.locally_created_topic_list {
            for discovered_topic in self
                .domain_participant
                .discovered_topic_list
                .iter()
                .filter(|t| t.name.value.as_str() == topic.topic_name.as_ref())
            {
                if let Some(discovered_type_information) = &discovered_topic.type_information {
                    let discovered_type_id = &discovered_type_information
                        .complete
                        .typeid_with_size
                        .type_id;
                    if discovered_type_information.minimal != topic.type_information.minimal
                        && !self
                            .domain_participant
                            .type_register
                            .is_type_resolved(discovered_type_id)
                    {
                        if discovered_type_information.complete.dependent_typeid_count != 0 {
                            if !self
                                .domain_participant
                                .type_register
                                .is_dependencies_lookup_pending(discovered_type_id)
                            {
                                let type_request_writer = &mut self
                                    .domain_participant
                                    .builtin_publisher
                                    .type_lookup_request_writer;

                                let type_lookup_request = TypeLookupRequest {
                                    header: RequestHeader {
                                        request_id: SampleIdentity {
                                            writer_guid: type_request_writer
                                                .transport_writer
                                                .guid(),
                                            sequence_number: (type_request_writer
                                                .last_change_sequence_number
                                                + 1)
                                            .into(),
                                        },
                                        instance_name: format!(
                                            "dds.builtin.TOS.{:x}",
                                            self.domain_participant.instance_handle,
                                        ),
                                    },
                                    call: TypeLookupCall::TypeLookupGetDependenciesHash {
                                        get_type_dependencies: TypeLookupGetTypeDependenciesIn {
                                            type_ids: vec![discovered_type_id.clone()],
                                            continuation_point: Vec::new(),
                                        },
                                    },
                                };
                                let sample_instance_handle = InstanceHandle::default();
                                let serialized_data =
                                    serialize_cdr2_le(&type_lookup_request.create_dynamic_sample())
                                        .unwrap();
                                type_request_writer
                                    .write_w_timestamp(
                                        sample_instance_handle,
                                        serialized_data,
                                        now,
                                        now,
                                    )
                                    .ok();
                                type_request_writer
                                    .transport_writer
                                    .write_message(self.transport.message_writer.as_mut(), now);
                                self.domain_participant
                                    .type_register
                                    .add_pending_dependencies_lookup(discovered_type_id.clone());
                            }
                        } else if !self
                            .domain_participant
                            .type_register
                            .is_types_lookup_pending(core::slice::from_ref(discovered_type_id))
                        {
                            let type_request_writer = &mut self
                                .domain_participant
                                .builtin_publisher
                                .type_lookup_request_writer;

                            let type_lookup_request = TypeLookupRequest {
                                header: RequestHeader {
                                    request_id: SampleIdentity {
                                        writer_guid: type_request_writer.transport_writer.guid(),
                                        sequence_number: (type_request_writer
                                            .last_change_sequence_number
                                            + 1)
                                        .into(),
                                    },
                                    instance_name: format!(
                                        "dds.builtin.TOS.{:x}",
                                        self.domain_participant.instance_handle,
                                    ),
                                },
                                call: TypeLookupCall::TypeLookupGetTypesHashId {
                                    get_types: TypeLookupGetTypesIn {
                                        type_ids: vec![discovered_type_id.clone()],
                                    },
                                },
                            };
                            let sample_instance_handle = InstanceHandle::default();
                            let serialized_data =
                                serialize_cdr2_le(&type_lookup_request.create_dynamic_sample())
                                    .unwrap();
                            type_request_writer
                                .write_w_timestamp(
                                    sample_instance_handle,
                                    serialized_data,
                                    now,
                                    now,
                                )
                                .ok();
                            type_request_writer
                                .transport_writer
                                .write_message(self.transport.message_writer.as_mut(), now);
                            self.domain_participant
                                .type_register
                                .add_pending_types_lookup(vec![discovered_type_id.clone()]);
                        }
                    }
                }
            }
        }
    }

    #[tracing::instrument(skip(self))]
    pub(crate) fn add_discovered_participant(
        &mut self,
        discovered_participant_data: &SpdpDiscoveredParticipantData,
        now: Time,
    ) {
        // Check that the domainId of the discovered participant equals the local one.
        // If it is not equal then there the local endpoints are not configured to
        // communicate with the discovered participant.
        // AND
        // Check that the domainTag of the discovered participant equals the local one.
        // If it is not equal then there the local endpoints are not configured to
        // communicate with the discovered participant.
        // IN CASE no domain id was transmitted the a local domain id is assumed
        // (as specified in Table 9.19 - ParameterId mapping and default values)
        let is_domain_id_matching = match discovered_participant_data.participant_proxy.domain_id {
            Some(id) => id == self.domain_participant.domain_id,
            None => true,
        };
        let is_domain_tag_matching = discovered_participant_data.participant_proxy.domain_tag
            == self.domain_participant.domain_tag;
        let is_participant_discovered = self
            .domain_participant
            .discovered_participant_list
            .iter()
            .any(|p| {
                p.dds_participant_data.key().value
                    == discovered_participant_data.dds_participant_data.key.value
            });
        let is_participant_ignored = self
            .domain_participant
            .ignored_participants
            .iter()
            .any(|handle| handle == &discovered_participant_data.dds_participant_data.key.value);

        if is_domain_id_matching && is_domain_tag_matching && !is_participant_ignored {
            if !is_participant_discovered {
                let discovered_participant_info = DiscoveredParticipantInfo {
                    dds_participant_data: discovered_participant_data.dds_participant_data.clone(),
                    guid_prefix: discovered_participant_data.participant_proxy.guid_prefix,
                    default_unicast_locator_list: discovered_participant_data
                        .participant_proxy
                        .default_unicast_locator_list
                        .clone(),
                    default_multicast_locator_list: discovered_participant_data
                        .participant_proxy
                        .default_multicast_locator_list
                        .clone(),
                    lease_duration: discovered_participant_data.lease_duration,
                    last_communication_timestamp: now,
                };
                self.domain_participant
                    .discovered_participant_list
                    .push(discovered_participant_info);

                self.announce_participant(now);

                self.add_matched_publications_detector(discovered_participant_data);
                self.add_matched_publications_announcer(discovered_participant_data);
                self.add_matched_subscriptions_detector(discovered_participant_data);
                self.add_matched_subscriptions_announcer(discovered_participant_data);
                self.add_matched_topics_detector(discovered_participant_data);
                self.add_matched_topics_announcer(discovered_participant_data);

                self.add_matched_service_request_data_reader(discovered_participant_data);
                self.add_matched_service_request_data_writer(discovered_participant_data);
                self.add_matched_service_reply_data_reader(discovered_participant_data);
                self.add_matched_service_reply_data_writer(discovered_participant_data);
            } else if let Some(p) = self
                .domain_participant
                .discovered_participant_list
                .iter_mut()
                .find(|p| {
                    p.dds_participant_data.key().value
                        == discovered_participant_data.dds_participant_data.key.value
                })
            {
                p.dds_participant_data = discovered_participant_data.dds_participant_data.clone();
                p.default_unicast_locator_list = discovered_participant_data
                    .participant_proxy
                    .default_unicast_locator_list
                    .clone();
                p.default_multicast_locator_list = discovered_participant_data
                    .participant_proxy
                    .default_multicast_locator_list
                    .clone();
                p.lease_duration = discovered_participant_data.lease_duration;
                p.last_communication_timestamp = now;
            }
        }
    }

    /// Remove discovered [domain participant](SpdpDiscoveredParticipantData) with the speficied [handle](InstanceHandle).
    pub fn remove_discovered_participant(&mut self, handle: &InstanceHandle, now: Time) {
        self.domain_participant
            .discovered_participant_list
            .retain(|domain_participant| {
                &domain_participant.dds_participant_data.key.value != handle
            });

        let prefix = Guid::from(<[u8; 16]>::from(*handle)).prefix();

        for subscriber in &mut self.domain_participant.user_defined_subscriber_list {
            let (subscriber_status_condition, data_reader_list) = (
                &mut subscriber.status_condition,
                &mut subscriber.data_reader_list,
            );
            for data_reader in data_reader_list {
                let removed_writer_guids: Vec<_> = data_reader
                    .matched_publication_list
                    .iter()
                    .filter(|m| m.key.value[0..12] == prefix)
                    .map(|m| m.key.value)
                    .collect();
                for key in removed_writer_guids {
                    data_reader
                        .transport_reader
                        .delete_matched_writer(key.into());
                    data_reader.remove_matched_publication(&InstanceHandle::new(key), now);
                    subscriber_status_condition.add_communication_state(StatusKind::DataOnReaders);
                }
            }
        }

        for publisher in &mut self.domain_participant.user_defined_publisher_list {
            for data_writer in &mut publisher.data_writer_list {
                for matched_subscription in &data_writer.matched_subscription_list {
                    if matched_subscription.key.value[..12] == prefix {
                        // Remove readers
                        data_writer
                            .writer
                            .transport_writer
                            .delete_matched_reader(matched_subscription.key.value.into());
                    }
                }
                data_writer
                    .matched_subscription_list
                    .retain(|subscription| subscription.key.value[..12] != prefix);
            }
        }

        self.remove_matched_publications_detector(prefix);
        self.remove_matched_publications_announcer(prefix);

        self.remove_matched_subscriptions_detector(prefix);
        self.remove_matched_subscriptions_announcer(prefix);

        self.remove_matched_topics_detector(prefix);
        self.remove_matched_topics_announcer(prefix);

        self.remove_matched_service_request_data_reader(prefix);
        self.remove_matched_service_request_data_writer(prefix);
        self.remove_matched_service_reply_data_reader(prefix);
        self.remove_matched_service_reply_data_writer(prefix);
    }

    #[tracing::instrument(skip(self))]
    fn add_matched_publications_detector(
        &mut self,
        discovered_participant_data: &SpdpDiscoveredParticipantData,
    ) {
        if discovered_participant_data
            .participant_proxy
            .available_builtin_endpoints
            .has(BuiltinEndpointSet::BUILTIN_ENDPOINT_PUBLICATIONS_DETECTOR)
        {
            let remote_reader_guid = Guid::new(
                discovered_participant_data.participant_proxy.guid_prefix,
                ENTITYID_SEDP_BUILTIN_PUBLICATIONS_DETECTOR,
            );
            let remote_group_entity_id = ENTITYID_UNKNOWN;
            let expects_inline_qos = false;
            let reader_proxy = transport::types::ReaderProxy {
                remote_reader_guid,
                remote_group_entity_id,
                reliability_kind: ReliabilityKind::Reliable,
                durability_kind: DurabilityKind::TransientLocal,
                unicast_locator_list: discovered_participant_data
                    .participant_proxy
                    .metatraffic_unicast_locator_list
                    .to_vec(),
                multicast_locator_list: discovered_participant_data
                    .participant_proxy
                    .metatraffic_multicast_locator_list
                    .to_vec(),
                expects_inline_qos,
            };
            {
                let dw = &mut self
                    .domain_participant
                    .builtin_publisher
                    .dcps_publications_writer;
                dw.transport_writer.add_matched_reader(reader_proxy);
            }
        }
    }

    #[tracing::instrument(skip(self))]
    fn remove_matched_publications_detector(&mut self, prefix: GuidPrefix) {
        {
            let dw = &mut self
                .domain_participant
                .builtin_publisher
                .dcps_publications_writer;
            let guid = Guid::new(prefix, ENTITYID_SEDP_BUILTIN_PUBLICATIONS_DETECTOR);
            dw.transport_writer.delete_matched_reader(guid);
        }
    }

    #[tracing::instrument(skip(self))]
    fn add_matched_publications_announcer(
        &mut self,
        discovered_participant_data: &SpdpDiscoveredParticipantData,
    ) {
        if discovered_participant_data
            .participant_proxy
            .available_builtin_endpoints
            .has(BuiltinEndpointSet::BUILTIN_ENDPOINT_PUBLICATIONS_ANNOUNCER)
        {
            let remote_writer_guid = Guid::new(
                discovered_participant_data.participant_proxy.guid_prefix,
                ENTITYID_SEDP_BUILTIN_PUBLICATIONS_ANNOUNCER,
            );
            let remote_group_entity_id = ENTITYID_UNKNOWN;

            let writer_proxy = transport::types::WriterProxy {
                remote_writer_guid,
                remote_group_entity_id,
                unicast_locator_list: discovered_participant_data
                    .participant_proxy
                    .metatraffic_unicast_locator_list
                    .to_vec(),
                multicast_locator_list: discovered_participant_data
                    .participant_proxy
                    .metatraffic_multicast_locator_list
                    .to_vec(),
                reliability_kind: ReliabilityKind::Reliable,
                durability_kind: DurabilityKind::TransientLocal,
            };
            let dr = &mut self
                .domain_participant
                .builtin_subscriber
                .dcps_publication_reader;
            dr.transport_reader.add_matched_writer(&writer_proxy);
        }
    }

    #[tracing::instrument(skip(self))]
    fn remove_matched_publications_announcer(&mut self, prefix: GuidPrefix) {
        let dr = &mut self
            .domain_participant
            .builtin_subscriber
            .dcps_publication_reader;
        let guid = Guid::new(prefix, ENTITYID_SEDP_BUILTIN_PUBLICATIONS_ANNOUNCER);
        dr.transport_reader.delete_matched_writer(guid);
    }

    #[tracing::instrument(skip(self))]
    fn add_matched_subscriptions_detector(
        &mut self,
        discovered_participant_data: &SpdpDiscoveredParticipantData,
    ) {
        if discovered_participant_data
            .participant_proxy
            .available_builtin_endpoints
            .has(BuiltinEndpointSet::BUILTIN_ENDPOINT_SUBSCRIPTIONS_DETECTOR)
        {
            let remote_reader_guid = Guid::new(
                discovered_participant_data.participant_proxy.guid_prefix,
                ENTITYID_SEDP_BUILTIN_SUBSCRIPTIONS_DETECTOR,
            );
            let remote_group_entity_id = ENTITYID_UNKNOWN;
            let expects_inline_qos = false;
            let reader_proxy = transport::types::ReaderProxy {
                remote_reader_guid,
                remote_group_entity_id,
                reliability_kind: ReliabilityKind::Reliable,
                durability_kind: DurabilityKind::TransientLocal,
                unicast_locator_list: discovered_participant_data
                    .participant_proxy
                    .metatraffic_unicast_locator_list
                    .to_vec(),
                multicast_locator_list: discovered_participant_data
                    .participant_proxy
                    .metatraffic_multicast_locator_list
                    .to_vec(),
                expects_inline_qos,
            };
            {
                let dw = &mut self
                    .domain_participant
                    .builtin_publisher
                    .dcps_subscriptions_writer;
                dw.transport_writer.add_matched_reader(reader_proxy);
            }
        }
    }

    #[tracing::instrument(skip(self))]
    fn remove_matched_subscriptions_detector(&mut self, prefix: GuidPrefix) {
        {
            let dw = &mut self
                .domain_participant
                .builtin_publisher
                .dcps_subscriptions_writer;
            let guid = Guid::new(prefix, ENTITYID_SEDP_BUILTIN_SUBSCRIPTIONS_DETECTOR);
            dw.transport_writer.delete_matched_reader(guid);
        }
    }

    #[tracing::instrument(skip(self))]
    fn add_matched_subscriptions_announcer(
        &mut self,
        discovered_participant_data: &SpdpDiscoveredParticipantData,
    ) {
        if discovered_participant_data
            .participant_proxy
            .available_builtin_endpoints
            .has(BuiltinEndpointSet::BUILTIN_ENDPOINT_SUBSCRIPTIONS_ANNOUNCER)
        {
            let remote_writer_guid = Guid::new(
                discovered_participant_data.participant_proxy.guid_prefix,
                ENTITYID_SEDP_BUILTIN_SUBSCRIPTIONS_ANNOUNCER,
            );
            let remote_group_entity_id = ENTITYID_UNKNOWN;

            let writer_proxy = transport::types::WriterProxy {
                remote_writer_guid,
                remote_group_entity_id,
                reliability_kind: ReliabilityKind::Reliable,
                durability_kind: DurabilityKind::TransientLocal,
                unicast_locator_list: discovered_participant_data
                    .participant_proxy
                    .metatraffic_unicast_locator_list
                    .to_vec(),
                multicast_locator_list: discovered_participant_data
                    .participant_proxy
                    .metatraffic_multicast_locator_list
                    .to_vec(),
            };
            let dr = &mut self
                .domain_participant
                .builtin_subscriber
                .dcps_subscription_reader;
            dr.transport_reader.add_matched_writer(&writer_proxy);
        }
    }

    #[tracing::instrument(skip(self))]
    fn remove_matched_subscriptions_announcer(&mut self, prefix: GuidPrefix) {
        let dr = &mut self
            .domain_participant
            .builtin_subscriber
            .dcps_subscription_reader;
        let guid = Guid::new(prefix, ENTITYID_SEDP_BUILTIN_SUBSCRIPTIONS_ANNOUNCER);
        dr.transport_reader.delete_matched_writer(guid);
    }

    #[tracing::instrument(skip(self))]
    fn add_matched_topics_detector(
        &mut self,
        discovered_participant_data: &SpdpDiscoveredParticipantData,
    ) {
        if discovered_participant_data
            .participant_proxy
            .available_builtin_endpoints
            .has(BuiltinEndpointSet::BUILTIN_ENDPOINT_TOPICS_DETECTOR)
        {
            let remote_reader_guid = Guid::new(
                discovered_participant_data.participant_proxy.guid_prefix,
                ENTITYID_SEDP_BUILTIN_TOPICS_DETECTOR,
            );
            let remote_group_entity_id = ENTITYID_UNKNOWN;
            let expects_inline_qos = false;
            let reader_proxy = transport::types::ReaderProxy {
                remote_reader_guid,
                remote_group_entity_id,
                reliability_kind: ReliabilityKind::Reliable,
                durability_kind: DurabilityKind::TransientLocal,
                unicast_locator_list: discovered_participant_data
                    .participant_proxy
                    .metatraffic_unicast_locator_list
                    .to_vec(),
                multicast_locator_list: discovered_participant_data
                    .participant_proxy
                    .metatraffic_multicast_locator_list
                    .to_vec(),
                expects_inline_qos,
            };
            {
                let dw = &mut self.domain_participant.builtin_publisher.dcps_topics_writer;
                dw.transport_writer.add_matched_reader(reader_proxy);
            }
        }
    }

    #[tracing::instrument(skip(self))]
    fn remove_matched_topics_detector(&mut self, prefix: GuidPrefix) {
        {
            let dw = &mut self.domain_participant.builtin_publisher.dcps_topics_writer;
            let guid = Guid::new(prefix, ENTITYID_SEDP_BUILTIN_TOPICS_DETECTOR);
            dw.transport_writer.delete_matched_reader(guid);
        }
    }

    #[tracing::instrument(skip(self))]
    fn add_matched_topics_announcer(
        &mut self,
        discovered_participant_data: &SpdpDiscoveredParticipantData,
    ) {
        if discovered_participant_data
            .participant_proxy
            .available_builtin_endpoints
            .has(BuiltinEndpointSet::BUILTIN_ENDPOINT_TOPICS_ANNOUNCER)
        {
            let remote_writer_guid = Guid::new(
                discovered_participant_data.participant_proxy.guid_prefix,
                ENTITYID_SEDP_BUILTIN_TOPICS_ANNOUNCER,
            );
            let remote_group_entity_id = ENTITYID_UNKNOWN;

            let writer_proxy = transport::types::WriterProxy {
                remote_writer_guid,
                remote_group_entity_id,
                reliability_kind: ReliabilityKind::Reliable,
                durability_kind: DurabilityKind::TransientLocal,
                unicast_locator_list: discovered_participant_data
                    .participant_proxy
                    .metatraffic_unicast_locator_list
                    .to_vec(),
                multicast_locator_list: discovered_participant_data
                    .participant_proxy
                    .metatraffic_multicast_locator_list
                    .to_vec(),
            };
            let dr = &mut self.domain_participant.builtin_subscriber.dcps_topic_reader;
            dr.transport_reader.add_matched_writer(&writer_proxy);
        }
    }

    #[tracing::instrument(skip(self))]
    fn remove_matched_topics_announcer(&mut self, prefix: GuidPrefix) {
        let dr = &mut self.domain_participant.builtin_subscriber.dcps_topic_reader;
        let guid = Guid::new(prefix, ENTITYID_SEDP_BUILTIN_TOPICS_ANNOUNCER);
        dr.transport_reader.delete_matched_writer(guid);
    }

    #[tracing::instrument(skip(self))]
    fn add_matched_service_request_data_reader(
        &mut self,
        discovered_participant_data: &SpdpDiscoveredParticipantData,
    ) {
        if discovered_participant_data
            .participant_proxy
            .available_builtin_endpoints
            .has(BuiltinEndpointSet::BUILTIN_ENDPOINT_TYPE_LOOKUP_SERVICE_REQUEST_DATA_READER)
        {
            let remote_reader_guid = Guid::new(
                discovered_participant_data.participant_proxy.guid_prefix,
                ENTITYID_TL_SVC_REQ_READER,
            );
            let remote_group_entity_id = ENTITYID_UNKNOWN;
            let expects_inline_qos = false;
            let reader_proxy = transport::types::ReaderProxy {
                remote_reader_guid,
                remote_group_entity_id,
                reliability_kind: ReliabilityKind::Reliable,
                durability_kind: DurabilityKind::Volatile,
                unicast_locator_list: discovered_participant_data
                    .participant_proxy
                    .metatraffic_unicast_locator_list
                    .to_vec(),
                multicast_locator_list: discovered_participant_data
                    .participant_proxy
                    .metatraffic_multicast_locator_list
                    .to_vec(),
                expects_inline_qos,
            };
            {
                let dw = &mut self
                    .domain_participant
                    .builtin_publisher
                    .type_lookup_request_writer;
                dw.transport_writer.add_matched_reader(reader_proxy);
            }
        }
    }

    #[tracing::instrument(skip(self))]
    fn remove_matched_service_request_data_reader(&mut self, prefix: GuidPrefix) {
        {
            let dw = &mut self
                .domain_participant
                .builtin_publisher
                .type_lookup_request_writer;
            let guid = Guid::new(prefix, ENTITYID_TL_SVC_REQ_READER);
            dw.transport_writer.delete_matched_reader(guid);
        }
    }

    #[tracing::instrument(skip(self))]
    fn add_matched_service_request_data_writer(
        &mut self,
        discovered_participant_data: &SpdpDiscoveredParticipantData,
    ) {
        if discovered_participant_data
            .participant_proxy
            .available_builtin_endpoints
            .has(BuiltinEndpointSet::BUILTIN_ENDPOINT_TYPE_LOOKUP_SERVICE_REQUEST_DATA_WRITER)
        {
            let remote_writer_guid = Guid::new(
                discovered_participant_data.participant_proxy.guid_prefix,
                ENTITYID_TL_SVC_REQ_WRITER,
            );
            let remote_group_entity_id = ENTITYID_UNKNOWN;

            let writer_proxy = transport::types::WriterProxy {
                remote_writer_guid,
                remote_group_entity_id,
                reliability_kind: ReliabilityKind::Reliable,
                durability_kind: DurabilityKind::Volatile,
                unicast_locator_list: discovered_participant_data
                    .participant_proxy
                    .metatraffic_unicast_locator_list
                    .to_vec(),
                multicast_locator_list: discovered_participant_data
                    .participant_proxy
                    .metatraffic_multicast_locator_list
                    .to_vec(),
            };
            let dr = &mut self
                .domain_participant
                .builtin_subscriber
                .type_lookup_request_reader;
            dr.transport_reader.add_matched_writer(&writer_proxy);
        }
    }

    #[tracing::instrument(skip(self))]
    fn remove_matched_service_request_data_writer(&mut self, prefix: GuidPrefix) {
        let dr = &mut self
            .domain_participant
            .builtin_subscriber
            .type_lookup_request_reader;
        let guid = Guid::new(prefix, ENTITYID_TL_SVC_REQ_WRITER);
        dr.transport_reader.delete_matched_writer(guid);
    }

    #[tracing::instrument(skip(self))]
    fn add_matched_service_reply_data_reader(
        &mut self,
        discovered_participant_data: &SpdpDiscoveredParticipantData,
    ) {
        if discovered_participant_data
            .participant_proxy
            .available_builtin_endpoints
            .has(BuiltinEndpointSet::BUILTIN_ENDPOINT_TYPE_LOOKUP_SERVICE_REPLY_DATA_READER)
        {
            let remote_reader_guid = Guid::new(
                discovered_participant_data.participant_proxy.guid_prefix,
                ENTITYID_TL_SVC_REPLY_READER,
            );
            let remote_group_entity_id = ENTITYID_UNKNOWN;
            let expects_inline_qos = false;
            let reader_proxy = transport::types::ReaderProxy {
                remote_reader_guid,
                remote_group_entity_id,
                reliability_kind: ReliabilityKind::Reliable,
                durability_kind: DurabilityKind::Volatile,
                unicast_locator_list: discovered_participant_data
                    .participant_proxy
                    .metatraffic_unicast_locator_list
                    .to_vec(),
                multicast_locator_list: discovered_participant_data
                    .participant_proxy
                    .metatraffic_multicast_locator_list
                    .to_vec(),
                expects_inline_qos,
            };
            {
                let dw = &mut self
                    .domain_participant
                    .builtin_publisher
                    .type_lookup_reply_writer;
                dw.transport_writer.add_matched_reader(reader_proxy);
            }
        }
    }

    #[tracing::instrument(skip(self))]
    fn remove_matched_service_reply_data_reader(&mut self, prefix: GuidPrefix) {
        {
            let dw = &mut self
                .domain_participant
                .builtin_publisher
                .type_lookup_reply_writer;
            let guid = Guid::new(prefix, ENTITYID_TL_SVC_REPLY_READER);
            dw.transport_writer.delete_matched_reader(guid);
        }
    }

    #[tracing::instrument(skip(self))]
    fn add_matched_service_reply_data_writer(
        &mut self,
        discovered_participant_data: &SpdpDiscoveredParticipantData,
    ) {
        if discovered_participant_data
            .participant_proxy
            .available_builtin_endpoints
            .has(BuiltinEndpointSet::BUILTIN_ENDPOINT_TYPE_LOOKUP_SERVICE_REPLY_DATA_WRITER)
        {
            let remote_writer_guid = Guid::new(
                discovered_participant_data.participant_proxy.guid_prefix,
                ENTITYID_TL_SVC_REPLY_WRITER,
            );
            let remote_group_entity_id = ENTITYID_UNKNOWN;

            let writer_proxy = transport::types::WriterProxy {
                remote_writer_guid,
                remote_group_entity_id,
                reliability_kind: ReliabilityKind::Reliable,
                durability_kind: DurabilityKind::Volatile,
                unicast_locator_list: discovered_participant_data
                    .participant_proxy
                    .metatraffic_unicast_locator_list
                    .to_vec(),
                multicast_locator_list: discovered_participant_data
                    .participant_proxy
                    .metatraffic_multicast_locator_list
                    .to_vec(),
            };
            let dr = &mut self
                .domain_participant
                .builtin_subscriber
                .type_lookup_reply_reader;
            dr.transport_reader.add_matched_writer(&writer_proxy);
        }
    }

    #[tracing::instrument(skip(self))]
    fn remove_matched_service_reply_data_writer(&mut self, prefix: GuidPrefix) {
        let dr = &mut self
            .domain_participant
            .builtin_subscriber
            .type_lookup_reply_reader;
        let guid = Guid::new(prefix, ENTITYID_TL_SVC_REPLY_WRITER);
        dr.transport_reader.delete_matched_writer(guid);
    }

    #[tracing::instrument(skip(self))]
    pub fn _request_type_lookup(&mut self, type_ids: Vec<TypeIdentifier>, now: Time) {
        {
            let w = &mut self
                .domain_participant
                .builtin_publisher
                .type_lookup_request_writer;
            let dynamic_data = TypeLookupRequest {
                header: RequestHeader {
                    request_id: SampleIdentity {
                        writer_guid: w.transport_writer.guid(),
                        sequence_number: (w.last_change_sequence_number + 1).into(),
                    },
                    instance_name: String::from(""),
                },
                call: TypeLookupCall::TypeLookupGetTypesHashId {
                    get_types: TypeLookupGetTypesIn { type_ids },
                },
            }
            .create_dynamic_sample();

            let timestamp = now;
            let sample_instance_handle = self.domain_participant.instance_handle;
            let serialized_data = serialize_cdr2_le(&dynamic_data).unwrap();
            let sample_timestamp = timestamp;
            let now = timestamp;
            w.write_w_timestamp(
                sample_instance_handle,
                serialized_data,
                sample_timestamp,
                now,
            )
            .ok();
        }
    }
}

#[tracing::instrument]
fn get_discovered_reader_incompatible_qos_policy_list(
    writer_qos: &DataWriterQos,
    discovered_reader_data: &SubscriptionBuiltinTopicData,
    publisher_qos: &PublisherQos,
) -> Vec<QosPolicyId> {
    let mut incompatible_qos_policy_list = Vec::new();
    if &writer_qos.durability < discovered_reader_data.durability() {
        incompatible_qos_policy_list.push(DURABILITY_QOS_POLICY_ID);
    }
    if publisher_qos.presentation.access_scope < discovered_reader_data.presentation().access_scope
        || publisher_qos.presentation.coherent_access
            != discovered_reader_data.presentation().coherent_access
        || publisher_qos.presentation.ordered_access
            != discovered_reader_data.presentation().ordered_access
    {
        incompatible_qos_policy_list.push(PRESENTATION_QOS_POLICY_ID);
    }
    if &writer_qos.deadline > discovered_reader_data.deadline() {
        incompatible_qos_policy_list.push(DEADLINE_QOS_POLICY_ID);
    }
    if &writer_qos.latency_budget > discovered_reader_data.latency_budget() {
        incompatible_qos_policy_list.push(LATENCYBUDGET_QOS_POLICY_ID);
    }
    if &writer_qos.liveliness < discovered_reader_data.liveliness() {
        incompatible_qos_policy_list.push(LIVELINESS_QOS_POLICY_ID);
    }
    if writer_qos.reliability.kind < discovered_reader_data.reliability().kind {
        incompatible_qos_policy_list.push(RELIABILITY_QOS_POLICY_ID);
    }
    if &writer_qos.destination_order < discovered_reader_data.destination_order() {
        incompatible_qos_policy_list.push(DESTINATIONORDER_QOS_POLICY_ID);
    }
    if writer_qos.ownership.kind != discovered_reader_data.ownership().kind {
        incompatible_qos_policy_list.push(OWNERSHIP_QOS_POLICY_ID);
    }

    let writer_offered_representation = writer_qos
        .representation
        .value
        .first()
        .unwrap_or(&XCDR_DATA_REPRESENTATION);
    if !(discovered_reader_data
        .representation()
        .value
        .contains(writer_offered_representation)
        || (writer_offered_representation == &XCDR_DATA_REPRESENTATION
            && discovered_reader_data.representation().value.is_empty()))
    {
        incompatible_qos_policy_list.push(DATA_REPRESENTATION_QOS_POLICY_ID);
    }

    incompatible_qos_policy_list
}

#[tracing::instrument(skip(data_reader))]
fn get_discovered_writer_incompatible_qos_policy_list<R>(
    data_reader: &DataReaderEntity<R>,
    publication_builtin_topic_data: &PublicationBuiltinTopicData,
    subscriber_qos: &SubscriberQos,
) -> Vec<QosPolicyId> {
    let mut incompatible_qos_policy_list = Vec::new();

    if subscriber_qos.presentation.access_scope
        > publication_builtin_topic_data.presentation().access_scope
        || subscriber_qos.presentation.coherent_access
            != publication_builtin_topic_data
                .presentation()
                .coherent_access
        || subscriber_qos.presentation.ordered_access
            != publication_builtin_topic_data.presentation().ordered_access
    {
        incompatible_qos_policy_list.push(PRESENTATION_QOS_POLICY_ID);
    }
    if &data_reader.qos.durability > publication_builtin_topic_data.durability() {
        incompatible_qos_policy_list.push(DURABILITY_QOS_POLICY_ID);
    }
    if &data_reader.qos.deadline < publication_builtin_topic_data.deadline() {
        incompatible_qos_policy_list.push(DEADLINE_QOS_POLICY_ID);
    }
    if &data_reader.qos.latency_budget < publication_builtin_topic_data.latency_budget() {
        incompatible_qos_policy_list.push(LATENCYBUDGET_QOS_POLICY_ID);
    }
    if &data_reader.qos.liveliness > publication_builtin_topic_data.liveliness() {
        incompatible_qos_policy_list.push(LIVELINESS_QOS_POLICY_ID);
    }
    if data_reader.qos.reliability.kind > publication_builtin_topic_data.reliability().kind {
        incompatible_qos_policy_list.push(RELIABILITY_QOS_POLICY_ID);
    }
    if &data_reader.qos.destination_order > publication_builtin_topic_data.destination_order() {
        incompatible_qos_policy_list.push(DESTINATIONORDER_QOS_POLICY_ID);
    }
    if data_reader.qos.ownership.kind != publication_builtin_topic_data.ownership().kind {
        incompatible_qos_policy_list.push(OWNERSHIP_QOS_POLICY_ID);
    }

    let writer_offered_representation = publication_builtin_topic_data
        .representation()
        .value
        .first()
        .unwrap_or(&XCDR_DATA_REPRESENTATION);
    if !data_reader
        .qos
        .representation
        .value
        .contains(writer_offered_representation)
    {
        // Empty list is interpreted as containing XCDR_DATA_REPRESENTATION
        if !(writer_offered_representation == &XCDR_DATA_REPRESENTATION
            && data_reader.qos.representation.value.is_empty())
        {
            incompatible_qos_policy_list.push(DATA_REPRESENTATION_QOS_POLICY_ID)
        }
    }

    incompatible_qos_policy_list
}

fn is_partition_matched(p1: &PartitionQosPolicy, p2: &PartitionQosPolicy) -> bool {
    if p1 == p2 {
        return true;
    }
    if p1.name.iter().any(|n| p2.name.contains(n)) {
        return true;
    }
    let p1_has_wildcard = p1.name.iter().any(|n| n.contains(['*', '?', '[', '+']));
    let p2_has_wildcard = p2.name.iter().any(|n| n.contains(['*', '?', '[', '+']));
    if p1_has_wildcard
        && p1
            .name
            .iter()
            .filter(|n| n.contains(['*', '?', '[', '+']))
            .filter_map(|n| Regex::new(&fnmatch_to_regex(n)).ok())
            .any(|regex| p2.name.iter().any(|n| regex.is_match(n)))
    {
        return true;
    }
    if p2_has_wildcard
        && p2
            .name
            .iter()
            .filter(|n| n.contains(['*', '?', '[', '+']))
            .filter_map(|n| Regex::new(&fnmatch_to_regex(n)).ok())
            .any(|regex| p1.name.iter().any(|n| regex.is_match(n)))
    {
        return true;
    }
    false
}

fn fnmatch_to_regex(pattern: &str) -> String {
    fn flush_literal(out: &mut String, lit: &mut String) {
        if !lit.is_empty() {
            out.push_str(&regex::escape(lit));
            lit.clear();
        }
    }

    let mut out = String::from("^");
    let mut literal = String::new();
    let mut chars = pattern.chars().peekable();

    while let Some(c) = chars.next() {
        match c {
            // backslash escapes next char literally
            '\\' => {
                if let Some(next) = chars.next() {
                    literal.push(next);
                } else {
                    literal.push('\\');
                }
            }

            // glob wildcards
            '*' => {
                flush_literal(&mut out, &mut literal);
                out.push_str(".*");
            }
            '?' => {
                flush_literal(&mut out, &mut literal);
                out.push('.');
            }

            // character class
            '[' => {
                flush_literal(&mut out, &mut literal);

                let mut class = String::from("[");
                // handle fnmatch negation [!...] -> regex [^...]
                if let Some(&next) = chars.peek() {
                    if next == '!' {
                        chars.next();
                        class.push('^');
                    } else if next == '^' {
                        // treat ^ the same if user used it
                        chars.next();
                        class.push('^');
                    }
                }

                let mut closed = false;
                while let Some(ch) = chars.next() {
                    class.push(ch);
                    if ch == ']' {
                        closed = true;
                        break;
                    }
                    // preserve escaped chars inside class
                    if ch == '\\' {
                        if let Some(esc) = chars.next() {
                            class.push(esc);
                        }
                    }
                }

                if closed {
                    out.push_str(&class);
                } else {
                    // unclosed '[' — treat as literal
                    literal.push('[');
                    literal.push_str(&class[1..]); // append rest as literal
                }
            }

            '+' => {
                flush_literal(&mut out, &mut literal);
                out.push('+'); // regex plus (quantifier)
            }

            // default: accumulate literal characters (will be escaped when flushed)
            other => literal.push(other),
        }
    }

    flush_literal(&mut out, &mut literal);
    out.push('$');
    out
}

impl PublicationMatchedStatus {
    pub(crate) fn get(&mut self) -> Self {
        let status = self.clone();
        self.current_count_change = 0;
        self.total_count_change = 0;

        status
    }
}

impl IncompatibleSubscriptions {
    fn add_incompatible_subscription(
        &mut self,
        handle: InstanceHandle,
        incompatible_qos_policy_list: Vec<QosPolicyId>,
    ) {
        if !self.incompatible_subscription_list.contains(&handle) {
            self.offered_incompatible_qos_status.total_count += 1;
            self.offered_incompatible_qos_status.total_count_change += 1;
            self.offered_incompatible_qos_status.last_policy_id = incompatible_qos_policy_list[0];

            self.incompatible_subscription_list.push(handle);
            for incompatible_qos_policy in incompatible_qos_policy_list.into_iter() {
                if let Some(policy_count) = self
                    .offered_incompatible_qos_status
                    .policies
                    .iter_mut()
                    .find(|x| x.policy_id == incompatible_qos_policy)
                {
                    policy_count.count += 1;
                } else {
                    self.offered_incompatible_qos_status
                        .policies
                        .push(QosPolicyCount {
                            policy_id: incompatible_qos_policy,
                            count: 1,
                        })
                }
            }
        }
    }

    fn get_offered_incompatible_qos_status(&mut self) -> OfferedIncompatibleQosStatus {
        let status = self.offered_incompatible_qos_status.clone();
        self.offered_incompatible_qos_status.total_count_change = 0;
        status
    }
}

impl OfferedDeadlineMissedStatus {
    fn get_offered_deadline_missed_status(&mut self) -> OfferedDeadlineMissedStatus {
        let status = self.clone();
        self.total_count_change = 0;

        status
    }
}

impl InconsistentTopicStatus {
    fn get_inconsistent_topic_status(&mut self) -> InconsistentTopicStatus {
        let status = self.clone();
        self.total_count_change = 0;

        status
    }
}

impl BuiltInTopicKey {
    fn into_bytes(self, parameter_id: ParameterId) -> Vec<u8> {
        let mut buffer = Vec::new();
        let mut pl = ParameterListSerializer::new(&mut buffer);
        pl.write_header();
        pl.write_xcdr1_parameter(parameter_id, self);
        pl.write_sentinel();
        buffer
    }
}

#[cfg(test)]
mod type_identity_tests {
    use super::*;

    #[test]
    fn participant_and_endpoint_deletion_use_their_own_guid_parameters() {
        let key = BuiltInTopicKey { value: [7; 16] };
        for parameter_id in [PID_PARTICIPANT_GUID, PID_ENDPOINT_GUID] {
            let bytes = key.clone().into_bytes(parameter_id);
            let mut expected = vec![0, 3, 0, 0];
            expected.extend_from_slice(&parameter_id.to_le_bytes());
            expected.extend_from_slice(&16_u16.to_le_bytes());
            expected.extend_from_slice(&[7; 16]);
            expected.extend_from_slice(&[1, 0, 0, 0]);
            assert_eq!(bytes, expected);
        }
    }

    #[test]
    fn minimal_type_identity_does_not_depend_on_serialized_size() {
        let local = TypeIdentifierWithSize {
            type_id: TypeIdentifier::EkMinimal {
                equivalence_hash: [1; 14],
            },
            typeobject_serialized_size: 192,
        };
        let mut remote = local.clone();
        remote.typeobject_serialized_size = 0;
        assert!(minimal_type_matches(&local, &remote));
        remote.type_id = TypeIdentifier::EkMinimal {
            equivalence_hash: [2; 14],
        };
        assert!(!minimal_type_matches(&local, &remote));
        remote.type_id = TypeIdentifier::TkNone;
        assert!(!minimal_type_matches(&remote, &remote));
    }
}
