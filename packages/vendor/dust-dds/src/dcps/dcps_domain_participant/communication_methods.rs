use alloc::vec::Vec;
use tracing::info;

use crate::{
    builtin_topics::{BuiltInTopicKey, TopicBuiltinTopicData},
    dcps::{
        data_representation_builtin_endpoints::{
            discovered_reader_data::DiscoveredReaderData,
            discovered_topic_data::DiscoveredTopicData,
            discovered_writer_data::DiscoveredWriterData,
            spdp_discovered_participant_data::SpdpDiscoveredParticipantData,
            type_lookup::{TypeLookupReply, TypeLookupRequest},
        },
        dcps_domain_participant::{
            data_reader_entity::AddChangeResult, participant_entity::DcpsDomainParticipant,
            reader_methods::deserialize_topic_type, topic_entity::get_topic_type_support,
        },
        listeners::domain_participant_listener::ListenerMail,
        xtypes_glue::key_and_instance_handle::{
            KeyHolderType, get_instance_handle_from_dynamic_data,
        },
    },
    dds_async::{
        data_reader::DataReaderAsync, domain_participant::DomainParticipantAsync,
        subscriber::SubscriberAsync,
    },
    infrastructure::{
        instance::InstanceHandle,
        qos_policy::{
            HistoryQosPolicy, LifespanQosPolicy, ResourceLimitsQosPolicy,
            TransportPriorityQosPolicy,
        },
        status::StatusKind,
        time::Time,
    },
    rtps::message_receiver::MessageReceiver,
    rtps_messages::{
        overall_structure::{RtpsMessageRead, RtpsSubmessageReadKind},
        submessages::{
            data::DataSubmessage, data_frag::DataFragSubmessage, gap::GapSubmessage,
            heartbeat::HeartbeatSubmessage,
        },
    },
    transport::types::{ChangeKind, Guid, TopicKind},
    xtypes::{
        deserializer::deserialize_top_level_type,
        type_support::{Type, TypeSupport},
    },
};

impl DcpsDomainParticipant {
    #[tracing::instrument(skip(self))]
    pub fn process_user_defined_received_cache_changes(&mut self, reception_timestamp: Time) {
        let dcps_sender = self.dcps_sender.clone();
        let domain_id = self.domain_participant.domain_id;
        let dp_instance_handle = self.domain_participant.instance_handle;
        let dp_listener_mask = self.domain_participant.listener_mask;
        let dp_listener_sender = &self.domain_participant.listener_sender;
        let discovered_participant_list = &mut self.domain_participant.discovered_participant_list;
        let content_filtered_topic_list = &self.domain_participant.content_filtered_topic_list;
        let locally_created_topic_list = &self.domain_participant.locally_created_topic_list;
        let type_register = &self.domain_participant.type_register;

        for subscriber in &mut self.domain_participant.user_defined_subscriber_list {
            let subscriber_handle = subscriber.instance_handle;
            let subscriber_listener_mask = subscriber.listener_mask;
            let subscriber_listener_sender = subscriber.listener_sender.clone();
            let (subscriber_status_condition, data_reader_list) = (
                &mut subscriber.status_condition,
                &mut subscriber.data_reader_list,
            );
            'data_readers: for data_reader in data_reader_list {
                if data_reader.transport_reader.changes_mut().is_empty() {
                    continue;
                }

                let Some(type_support) = get_topic_type_support(
                    &data_reader.topic_name,
                    content_filtered_topic_list,
                    locally_created_topic_list,
                    type_register,
                ) else {
                    tracing::warn!(topic_name = ?data_reader.topic_name, "Failed to find type support for reader");
                    continue 'data_readers;
                };

                let content_filtered_topic = content_filtered_topic_list
                    .iter()
                    .find(|t| t.topic_name == data_reader.topic_name);

                let reader_type_name = if let Some(content_filtered_topic) = content_filtered_topic
                {
                    let Some(reader_topic) = locally_created_topic_list
                        .iter()
                        .find(|t| t.topic_name == content_filtered_topic.related_topic_name)
                    else {
                        tracing::warn!(topic_name = ?data_reader.topic_name, "Failed to find related_topic_name for reader");
                        continue 'data_readers;
                    };
                    &reader_topic.type_name
                } else {
                    let Some(reader_topic) = locally_created_topic_list
                        .iter()
                        .find(|t| t.topic_name == data_reader.topic_name)
                    else {
                        tracing::warn!(topic_name = ?data_reader.topic_name, "Failed to find topic for reader");
                        continue 'data_readers;
                    };
                    &reader_topic.type_name
                };

                let changes = core::mem::take(data_reader.transport_reader.changes_mut());
                let data_reader_handle = data_reader.instance_handle;
                tracing::trace!(subscriber_handle=?subscriber_handle, data_reader_handle=?data_reader_handle, "Processing {} reader cache changes", changes.len());

                for cache_change in changes {
                    if let Some(matched_participant) = discovered_participant_list
                        .iter_mut()
                        .find(|x| x.guid_prefix == cache_change.writer_guid.prefix())
                    {
                        matched_participant.last_communication_timestamp = reception_timestamp;
                    }

                    if let Some(content_filtered_topic) = content_filtered_topic {
                        if cache_change.kind == ChangeKind::Alive {
                            let Some(data) = deserialize_topic_type(
                                &data_reader.topic_name,
                                type_support,
                                cache_change.data_value.as_ref(),
                            ) else {
                                continue 'data_readers;
                            };
                            enum Operator {
                                LessThan,
                                Equal,
                            }

                            impl Operator {
                                fn to_str(&self) -> &'static str {
                                    match self {
                                        Self::Equal => "=",
                                        Self::LessThan => "<=",
                                    }
                                }

                                fn compare_string(&self, lhs: &str, rhs: &str) -> bool {
                                    match self {
                                        Self::Equal => lhs == rhs,
                                        Self::LessThan => lhs <= rhs,
                                    }
                                }
                                fn compare_int32(&self, lhs: &i32, rhs: &i32) -> bool {
                                    match self {
                                        Self::Equal => lhs == rhs,
                                        Self::LessThan => lhs <= rhs,
                                    }
                                }
                            }

                            let mut operators = [Operator::LessThan, Operator::Equal].iter();
                            let filter = loop {
                                if let Some(operator) = operators.next() {
                                    if let Some((variable_name, value_expr)) =
                                        content_filtered_topic
                                            .filter_expression
                                            .split_once(operator.to_str())
                                    {
                                        let trimmed_val = value_expr.trim();
                                        let value_str =
                                            if let Some(stripped) = trimmed_val.strip_prefix('%') {
                                                if let Ok(index) = stripped.parse::<usize>() {
                                                    content_filtered_topic
                                                        .expression_parameters
                                                        .get(index)
                                                        .map(|s| s.as_str())
                                                        .unwrap_or(trimmed_val)
                                                } else {
                                                    trimmed_val
                                                }
                                            } else {
                                                trimmed_val.trim_matches('\'').trim_matches('"')
                                            };
                                        break Some((variable_name, operator, value_str));
                                    }
                                } else {
                                    break None;
                                };
                            };

                            if let Some((variable_name, comparison_function, value_str)) = filter {
                                let Some(member_id) =
                                    data.get_member_id_by_name(variable_name.trim())
                                else {
                                    continue 'data_readers;
                                };
                                let Ok(member_descriptor) = data.get_descriptor(member_id) else {
                                    continue 'data_readers;
                                };
                                match member_descriptor.r#type.get_kind() {
                                    crate::xtypes::dynamic_type::TypeKind::NONE => todo!(),
                                    crate::xtypes::dynamic_type::TypeKind::BOOLEAN => todo!(),
                                    crate::xtypes::dynamic_type::TypeKind::BYTE => todo!(),
                                    crate::xtypes::dynamic_type::TypeKind::INT16 => todo!(),
                                    crate::xtypes::dynamic_type::TypeKind::INT32 => {
                                        let member_value = data.get_int32_value(member_id).unwrap();
                                        let Ok(rhs) = value_str.parse::<i32>() else {
                                            continue 'data_readers;
                                        };
                                        if !comparison_function.compare_int32(member_value, &rhs) {
                                            continue 'data_readers;
                                        }
                                    }
                                    crate::xtypes::dynamic_type::TypeKind::INT64 => todo!(),
                                    crate::xtypes::dynamic_type::TypeKind::UINT16 => todo!(),
                                    crate::xtypes::dynamic_type::TypeKind::UINT32 => todo!(),
                                    crate::xtypes::dynamic_type::TypeKind::UINT64 => todo!(),
                                    crate::xtypes::dynamic_type::TypeKind::FLOAT32 => todo!(),
                                    crate::xtypes::dynamic_type::TypeKind::FLOAT64 => todo!(),
                                    crate::xtypes::dynamic_type::TypeKind::FLOAT128 => todo!(),
                                    crate::xtypes::dynamic_type::TypeKind::INT8 => todo!(),
                                    crate::xtypes::dynamic_type::TypeKind::UINT8 => todo!(),
                                    crate::xtypes::dynamic_type::TypeKind::CHAR8 => todo!(),
                                    crate::xtypes::dynamic_type::TypeKind::CHAR16 => todo!(),
                                    crate::xtypes::dynamic_type::TypeKind::STRING8
                                    | crate::xtypes::dynamic_type::TypeKind::STRING16 => {
                                        let member_value =
                                            data.get_string_value(member_id).unwrap();
                                        if !comparison_function
                                            .compare_string(member_value.as_str(), value_str)
                                        {
                                            continue 'data_readers;
                                        }
                                    }
                                    crate::xtypes::dynamic_type::TypeKind::ALIAS => todo!(),
                                    crate::xtypes::dynamic_type::TypeKind::ENUM => todo!(),
                                    crate::xtypes::dynamic_type::TypeKind::BITMASK => todo!(),
                                    crate::xtypes::dynamic_type::TypeKind::ANNOTATION => {
                                        todo!()
                                    }
                                    crate::xtypes::dynamic_type::TypeKind::STRUCTURE => todo!(),
                                    crate::xtypes::dynamic_type::TypeKind::UNION => todo!(),
                                    crate::xtypes::dynamic_type::TypeKind::BITSET => todo!(),
                                    crate::xtypes::dynamic_type::TypeKind::SEQUENCE => todo!(),
                                    crate::xtypes::dynamic_type::TypeKind::ARRAY => todo!(),
                                    crate::xtypes::dynamic_type::TypeKind::MAP => todo!(),
                                }
                            } else {
                                continue 'data_readers;
                            };
                        }
                    }

                    let change_instance_handle =
                        if TopicKind::from(&type_support) == TopicKind::NoKey {
                            InstanceHandle::default()
                        } else if let Some(i) = cache_change.instance_handle {
                            InstanceHandle::new(i)
                        } else {
                            match cache_change.kind {
                                ChangeKind::Alive | ChangeKind::AliveFiltered => {
                                    let Some(data_value) = deserialize_topic_type(
                                        &data_reader.topic_name,
                                        type_support,
                                        cache_change.data_value.as_ref(),
                                    ) else {
                                        tracing::warn!("Failed to deserialize user defined data");
                                        continue 'data_readers;
                                    };
                                    let Ok(instance_handle) =
                                        get_instance_handle_from_dynamic_data(&data_value)
                                    else {
                                        tracing::warn!(
                                            "Failed to get instance handle from dynamic_data"
                                        );
                                        continue 'data_readers;
                                    };
                                    instance_handle
                                }
                                ChangeKind::NotAliveDisposed
                                | ChangeKind::NotAliveUnregistered
                                | ChangeKind::NotAliveDisposedUnregistered => {
                                    let key_holder = KeyHolderType::new(&type_support);
                                    let Some(dynamic_type) = key_holder.as_dynamic_type() else {
                                        tracing::warn!("Failed to create key holder");
                                        continue 'data_readers;
                                    };

                                    let Ok(data_value) = deserialize_top_level_type(
                                        dynamic_type,
                                        cache_change.data_value.as_ref(),
                                    ) else {
                                        tracing::warn!(
                                            "Failed to deserialize disposed user defined data"
                                        );
                                        continue 'data_readers;
                                    };

                                    let Ok(instance_handle) =
                                        get_instance_handle_from_dynamic_data(&data_value)
                                    else {
                                        tracing::warn!(
                                            "Failed to deserialize disposed key user defined data"
                                        );
                                        continue 'data_readers;
                                    };
                                    instance_handle
                                }
                            }
                        };

                    match data_reader.add_reader_change(
                        cache_change.writer_guid,
                        cache_change.data_value,
                        cache_change.kind,
                        change_instance_handle.into(),
                        cache_change.source_timestamp.map(Into::into),
                        reception_timestamp,
                    ) {
                        Ok(AddChangeResult::Added) => {
                            tracing::info!("New change added");

                            let data_reader_on_data_available_active = data_reader
                                .listener_mask
                                .is_enabled(&StatusKind::DataAvailable);

                            if subscriber_listener_mask.is_enabled(&StatusKind::DataOnReaders) {
                                if let Some(l) = &subscriber_listener_sender {
                                    let the_participant = DomainParticipantAsync::new(
                                        dcps_sender.clone(),
                                        domain_id,
                                        dp_instance_handle,
                                    );
                                    let the_subscriber =
                                        SubscriberAsync::new(subscriber_handle, the_participant);
                                    l.send(ListenerMail::DataOnReaders { the_subscriber }).ok();
                                }
                            } else if data_reader_on_data_available_active {
                                if let Some(l) = &data_reader.listener_sender {
                                    info!("Triggering data reader DataAvailable listener");
                                    let the_participant = DomainParticipantAsync::new(
                                        dcps_sender.clone(),
                                        domain_id,
                                        dp_instance_handle,
                                    );
                                    let the_subscriber =
                                        SubscriberAsync::new(subscriber_handle, the_participant);
                                    let the_reader = DataReaderAsync::new(
                                        data_reader_handle,
                                        the_subscriber,
                                        data_reader.topic_name.clone(),
                                        reader_type_name.clone(),
                                    );
                                    l.send(ListenerMail::DataAvailable { the_reader }).ok();
                                }
                            }

                            subscriber_status_condition
                                .add_communication_state(StatusKind::DataOnReaders);

                            data_reader
                                .status_condition
                                .add_communication_state(StatusKind::DataAvailable);
                        }
                        Ok(AddChangeResult::NotAdded) => (), // Do nothing
                        Ok(AddChangeResult::Rejected(
                            instance_handle,
                            sample_rejected_status_kind,
                        )) => {
                            tracing::info!("Change rejected");
                            data_reader.increment_sample_rejected_status(
                                instance_handle,
                                sample_rejected_status_kind,
                            );

                            let is_listener_enabled = data_reader
                                .listener_mask
                                .is_enabled(&StatusKind::SampleRejected)
                                || subscriber_listener_mask.is_enabled(&StatusKind::SampleRejected)
                                || dp_listener_mask.is_enabled(&StatusKind::SampleRejected);

                            if is_listener_enabled {
                                let the_participant = DomainParticipantAsync::new(
                                    dcps_sender.clone(),
                                    domain_id,
                                    dp_instance_handle,
                                );
                                let the_subscriber =
                                    SubscriberAsync::new(subscriber_handle, the_participant);
                                let the_reader = DataReaderAsync::new(
                                    data_reader_handle,
                                    the_subscriber,
                                    data_reader.topic_name.clone(),
                                    reader_type_name.clone(),
                                );
                                let status = data_reader.get_sample_rejected_status();

                                if data_reader
                                    .listener_mask
                                    .is_enabled(&StatusKind::SampleRejected)
                                {
                                    if let Some(l) = &data_reader.listener_sender {
                                        l.send(ListenerMail::SampleRejected { the_reader, status })
                                            .ok();
                                    };
                                } else if subscriber_listener_mask
                                    .is_enabled(&StatusKind::SampleRejected)
                                {
                                    if let Some(l) = &subscriber_listener_sender {
                                        l.send(ListenerMail::SampleRejected { status, the_reader })
                                            .ok();
                                    }
                                } else if dp_listener_mask.is_enabled(&StatusKind::SampleRejected) {
                                    if let Some(l) = dp_listener_sender {
                                        l.send(ListenerMail::SampleRejected { status, the_reader })
                                            .ok();
                                    }
                                }
                            }

                            data_reader
                                .status_condition
                                .add_communication_state(StatusKind::SampleRejected);
                        }
                        Err(_) => (),
                    }
                }
            }
        }
    }

    pub fn process_builtin_cache_changes(&mut self, reception_timestamp: Time) {
        // 1. SPDP Participant Reader
        if !self
            .domain_participant
            .builtin_subscriber
            .dcps_participant_reader
            .transport_reader
            .changes_mut()
            .is_empty()
        {
            let changes = core::mem::take(
                self.domain_participant
                    .builtin_subscriber
                    .dcps_participant_reader
                    .transport_reader
                    .changes_mut(),
            );
            for cache_change in changes {
                if cache_change.kind == ChangeKind::Alive
                    && cache_change.data_value.as_ref().len() >= 4
                {
                    if let Ok(discovered_participant_data) =
                        SpdpDiscoveredParticipantData::from_bytes(cache_change.data_value.as_ref())
                    {
                        let instance_handle = InstanceHandle::new(
                            discovered_participant_data.dds_participant_data.key.value,
                        );
                        self.add_discovered_participant(
                            &discovered_participant_data,
                            reception_timestamp,
                        );
                        self.domain_participant
                            .builtin_subscriber
                            .dcps_participant_reader
                            .add_reader_change(
                                cache_change.writer_guid,
                                cache_change.data_value,
                                cache_change.kind,
                                instance_handle.into(),
                                cache_change.source_timestamp.map(Into::into),
                                reception_timestamp,
                            )
                            .ok();
                    }
                } else {
                    let instance_handle =
                        InstanceHandle::new(cache_change.instance_handle.unwrap_or_default());
                    self.remove_discovered_participant(&instance_handle, reception_timestamp);
                    self.domain_participant
                        .builtin_subscriber
                        .dcps_participant_reader
                        .add_reader_change(
                            cache_change.writer_guid,
                            cache_change.data_value,
                            cache_change.kind,
                            instance_handle.into(),
                            cache_change.source_timestamp.map(Into::into),
                            reception_timestamp,
                        )
                        .ok();
                }
            }
        }

        // 2. SEDP Topics Reader
        if !self
            .domain_participant
            .builtin_subscriber
            .dcps_topic_reader
            .transport_reader
            .changes_mut()
            .is_empty()
        {
            let changes = core::mem::take(
                self.domain_participant
                    .builtin_subscriber
                    .dcps_topic_reader
                    .transport_reader
                    .changes_mut(),
            );
            for cache_change in changes {
                if let Some(matched_participant) = self
                    .domain_participant
                    .discovered_participant_list
                    .iter_mut()
                    .find(|x| x.guid_prefix == cache_change.writer_guid.prefix())
                {
                    matched_participant.last_communication_timestamp = reception_timestamp;
                }

                if cache_change.kind == ChangeKind::Alive
                    || cache_change.kind == ChangeKind::AliveFiltered
                {
                    if let Ok(discovered_topic_data) =
                        DiscoveredTopicData::from_bytes(cache_change.data_value.as_ref())
                    {
                        let instance_handle = InstanceHandle::new(
                            discovered_topic_data.topic_builtin_topic_data.key.value,
                        );
                        self.domain_participant
                            .add_discovered_topic(discovered_topic_data.topic_builtin_topic_data);
                        self.domain_participant
                            .builtin_subscriber
                            .dcps_topic_reader
                            .add_reader_change(
                                cache_change.writer_guid,
                                cache_change.data_value,
                                cache_change.kind,
                                instance_handle.into(),
                                cache_change.source_timestamp.map(Into::into),
                                reception_timestamp,
                            )
                            .ok();
                    }
                }
            }
        }

        // 3. SEDP Publications Reader
        if !self
            .domain_participant
            .builtin_subscriber
            .dcps_publication_reader
            .transport_reader
            .changes_mut()
            .is_empty()
        {
            let changes = core::mem::take(
                self.domain_participant
                    .builtin_subscriber
                    .dcps_publication_reader
                    .transport_reader
                    .changes_mut(),
            );
            for cache_change in changes {
                if let Some(matched_participant) = self
                    .domain_participant
                    .discovered_participant_list
                    .iter_mut()
                    .find(|x| x.guid_prefix == cache_change.writer_guid.prefix())
                {
                    matched_participant.last_communication_timestamp = reception_timestamp;
                }

                if cache_change.kind == ChangeKind::Alive
                    || cache_change.kind == ChangeKind::AliveFiltered
                {
                    if let Ok(discovered_writer_data) =
                        DiscoveredWriterData::from_bytes(cache_change.data_value.as_ref())
                    {
                        let publication_builtin_topic_data =
                            &discovered_writer_data.dds_publication_data;
                        let instance_handle =
                            InstanceHandle::new(publication_builtin_topic_data.key().value);
                        if !self
                            .domain_participant
                            .discovered_topic_list
                            .iter()
                            .any(|x| x.name.value == publication_builtin_topic_data.topic_name())
                        {
                            let writer_topic = TopicBuiltinTopicData {
                                key: BuiltInTopicKey::default(),
                                name: publication_builtin_topic_data.topic_name.clone(),
                                type_name: publication_builtin_topic_data.type_name.clone(),
                                type_information: publication_builtin_topic_data
                                    .type_information
                                    .clone(),
                                durability: publication_builtin_topic_data.durability().clone(),
                                deadline: publication_builtin_topic_data.deadline().clone(),
                                latency_budget: publication_builtin_topic_data
                                    .latency_budget()
                                    .clone(),
                                liveliness: publication_builtin_topic_data.liveliness().clone(),
                                reliability: publication_builtin_topic_data.reliability().clone(),
                                transport_priority: TransportPriorityQosPolicy::default(),
                                lifespan: publication_builtin_topic_data.lifespan().clone(),
                                destination_order: publication_builtin_topic_data
                                    .destination_order()
                                    .clone(),
                                history: HistoryQosPolicy::default(),
                                resource_limits: ResourceLimitsQosPolicy::default(),
                                ownership: publication_builtin_topic_data.ownership().clone(),
                                topic_data: publication_builtin_topic_data.topic_data().clone(),
                                representation: publication_builtin_topic_data
                                    .representation()
                                    .clone(),
                            };
                            self.domain_participant.add_discovered_topic(writer_topic);
                        }

                        self.domain_participant
                            .add_discovered_writer(discovered_writer_data);
                        self.domain_participant
                            .builtin_subscriber
                            .dcps_publication_reader
                            .add_reader_change(
                                cache_change.writer_guid,
                                cache_change.data_value,
                                cache_change.kind,
                                instance_handle.into(),
                                cache_change.source_timestamp.map(Into::into),
                                reception_timestamp,
                            )
                            .ok();
                    }
                } else {
                    let instance_handle =
                        InstanceHandle::new(cache_change.instance_handle.unwrap_or_default());
                    self.domain_participant
                        .remove_discovered_writer(&instance_handle);

                    for subscriber in &mut self.domain_participant.user_defined_subscriber_list {
                        let (subscriber_status_condition, data_reader_list) = (
                            &mut subscriber.status_condition,
                            &mut subscriber.data_reader_list,
                        );
                        for data_reader in data_reader_list {
                            if data_reader
                                .matched_publication_list
                                .iter()
                                .any(|x| &x.key().value == instance_handle.as_ref())
                            {
                                data_reader.remove_matched_publication(
                                    &instance_handle,
                                    reception_timestamp,
                                );
                                subscriber_status_condition
                                    .add_communication_state(StatusKind::DataOnReaders);
                            }
                        }
                    }
                    self.domain_participant
                        .builtin_subscriber
                        .dcps_publication_reader
                        .add_reader_change(
                            cache_change.writer_guid,
                            cache_change.data_value,
                            cache_change.kind,
                            instance_handle.into(),
                            cache_change.source_timestamp.map(Into::into),
                            reception_timestamp,
                        )
                        .ok();
                }
            }
            self.process_discovered_writers(reception_timestamp);
        }

        // 4. SEDP Subscriptions Reader
        if !self
            .domain_participant
            .builtin_subscriber
            .dcps_subscription_reader
            .transport_reader
            .changes_mut()
            .is_empty()
        {
            let changes = core::mem::take(
                self.domain_participant
                    .builtin_subscriber
                    .dcps_subscription_reader
                    .transport_reader
                    .changes_mut(),
            );
            for cache_change in changes {
                if let Some(matched_participant) = self
                    .domain_participant
                    .discovered_participant_list
                    .iter_mut()
                    .find(|x| x.guid_prefix == cache_change.writer_guid.prefix())
                {
                    matched_participant.last_communication_timestamp = reception_timestamp;
                }

                if cache_change.kind == ChangeKind::Alive
                    || cache_change.kind == ChangeKind::AliveFiltered
                {
                    if let Ok(discovered_reader_data) =
                        DiscoveredReaderData::from_bytes(cache_change.data_value.as_ref())
                    {
                        let instance_handle = InstanceHandle::new(
                            discovered_reader_data.dds_subscription_data.key().value,
                        );
                        if !self
                            .domain_participant
                            .discovered_topic_list
                            .iter()
                            .any(|x| {
                                x.name.value
                                    == discovered_reader_data.dds_subscription_data.topic_name()
                            })
                        {
                            let reader_topic = TopicBuiltinTopicData {
                                key: BuiltInTopicKey::default(),
                                name: discovered_reader_data
                                    .dds_subscription_data
                                    .topic_name
                                    .clone(),
                                type_name: discovered_reader_data
                                    .dds_subscription_data
                                    .type_name
                                    .clone(),
                                type_information: discovered_reader_data
                                    .dds_subscription_data
                                    .type_information
                                    .clone(),
                                topic_data: discovered_reader_data
                                    .dds_subscription_data
                                    .topic_data()
                                    .clone(),
                                durability: discovered_reader_data
                                    .dds_subscription_data
                                    .durability()
                                    .clone(),
                                deadline: discovered_reader_data
                                    .dds_subscription_data
                                    .deadline()
                                    .clone(),
                                latency_budget: discovered_reader_data
                                    .dds_subscription_data
                                    .latency_budget()
                                    .clone(),
                                liveliness: discovered_reader_data
                                    .dds_subscription_data
                                    .liveliness()
                                    .clone(),
                                reliability: discovered_reader_data
                                    .dds_subscription_data
                                    .reliability()
                                    .clone(),
                                destination_order: discovered_reader_data
                                    .dds_subscription_data
                                    .destination_order()
                                    .clone(),
                                history: HistoryQosPolicy::default(),
                                resource_limits: ResourceLimitsQosPolicy::default(),
                                transport_priority: TransportPriorityQosPolicy::default(),
                                lifespan: LifespanQosPolicy::default(),
                                ownership: discovered_reader_data
                                    .dds_subscription_data
                                    .ownership()
                                    .clone(),
                                representation: discovered_reader_data
                                    .dds_subscription_data
                                    .representation()
                                    .clone(),
                            };
                            self.domain_participant.add_discovered_topic(reader_topic);
                        }

                        self.domain_participant
                            .add_discovered_reader(discovered_reader_data);
                        self.domain_participant
                            .builtin_subscriber
                            .dcps_subscription_reader
                            .add_reader_change(
                                cache_change.writer_guid,
                                cache_change.data_value,
                                cache_change.kind,
                                instance_handle.into(),
                                cache_change.source_timestamp.map(Into::into),
                                reception_timestamp,
                            )
                            .ok();
                    }
                } else {
                    let instance_handle =
                        InstanceHandle::new(cache_change.instance_handle.unwrap_or_default());
                    self.domain_participant
                        .remove_discovered_reader(&instance_handle);

                    let mut handle_list = Vec::new();
                    for publisher in &self.domain_participant.user_defined_publisher_list {
                        for data_writer in publisher.data_writer_list.iter() {
                            handle_list
                                .push((publisher.instance_handle, data_writer.instance_handle));
                        }
                    }

                    for (publisher_handle, data_writer_handle) in handle_list {
                        self.remove_discovered_reader(
                            instance_handle,
                            publisher_handle,
                            data_writer_handle,
                        );
                    }
                    self.domain_participant
                        .builtin_subscriber
                        .dcps_subscription_reader
                        .add_reader_change(
                            cache_change.writer_guid,
                            cache_change.data_value,
                            cache_change.kind,
                            instance_handle.into(),
                            cache_change.source_timestamp.map(Into::into),
                            reception_timestamp,
                        )
                        .ok();
                }
            }
            self.process_discovered_readers(reception_timestamp);
        }

        // 5. TypeLookup Request Reader
        if !self
            .domain_participant
            .builtin_subscriber
            .type_lookup_request_reader
            .transport_reader
            .changes_mut()
            .is_empty()
        {
            let changes = core::mem::take(
                self.domain_participant
                    .builtin_subscriber
                    .type_lookup_request_reader
                    .transport_reader
                    .changes_mut(),
            );
            for cache_change in changes {
                if let Some(type_lookup_request) = deserialize_top_level_type(
                    TypeLookupRequest::TYPE,
                    cache_change.data_value.as_ref(),
                )
                .ok()
                .and_then(|mut d| TypeLookupRequest::create_sample(&mut d))
                {
                    self.handle_type_lookup_request(type_lookup_request, reception_timestamp);
                }
            }
        }

        // 6. TypeLookup Reply Reader
        if !self
            .domain_participant
            .builtin_subscriber
            .type_lookup_reply_reader
            .transport_reader
            .changes_mut()
            .is_empty()
        {
            let changes = core::mem::take(
                self.domain_participant
                    .builtin_subscriber
                    .type_lookup_reply_reader
                    .transport_reader
                    .changes_mut(),
            );
            let mut type_lookup_reply_received = false;
            for cache_change in changes {
                if let Some(type_lookup_reply) = deserialize_top_level_type(
                    TypeLookupReply::TYPE,
                    cache_change.data_value.as_ref(),
                )
                .ok()
                .and_then(|mut d| TypeLookupReply::create_sample(&mut d))
                {
                    if self.handle_type_lookup_reply(type_lookup_reply, reception_timestamp) {
                        type_lookup_reply_received = true;
                    }
                }
            }
            if type_lookup_reply_received {
                self.process_discovered_readers(reception_timestamp);
                self.process_discovered_writers(reception_timestamp);
            }
        }
    }

    #[tracing::instrument(skip(self, data_message))]
    pub fn handle_data(&mut self, data_message: &[u8], now: Time) {
        if let Ok(rtps_message) = RtpsMessageRead::try_from(data_message) {
            if let Some(matched_participant) = self
                .domain_participant
                .discovered_participant_list
                .iter_mut()
                .find(|x| x.guid_prefix == rtps_message.header().guid_prefix())
            {
                matched_participant.last_communication_timestamp = now;
            }

            let mut message_receiver = MessageReceiver::new(&rtps_message);

            while let Some(submessage) = message_receiver.next() {
                match submessage {
                    RtpsSubmessageReadKind::Data(data_submessage) => {
                        self.handle_data_submessage(&message_receiver, data_submessage);
                    }
                    RtpsSubmessageReadKind::DataFrag(data_frag_submessage) => {
                        self.handle_data_frag_submessage(&message_receiver, data_frag_submessage);
                    }
                    RtpsSubmessageReadKind::Gap(gap_submessage) => {
                        self.handle_gap_submessage(&message_receiver, gap_submessage);
                    }
                    RtpsSubmessageReadKind::Heartbeat(heartbeat_submessage) => {
                        self.handle_heartbeat_submessage(&message_receiver, heartbeat_submessage);
                    }
                    RtpsSubmessageReadKind::HeartbeatFrag(heartbeat_frag_submessage) => {
                        for dr in self
                            .domain_participant
                            .user_defined_subscriber_list
                            .iter_mut()
                            .flat_map(|s| s.data_reader_list.iter_mut().map(|dr| &mut dr.reader))
                            .chain(
                                self.domain_participant
                                    .builtin_subscriber
                                    .stateful_data_reader_list_mut()
                                    .into_iter(),
                            )
                        {
                            let writer_guid = Guid::new(
                                message_receiver.source_guid_prefix(),
                                heartbeat_frag_submessage.writer_id(),
                            );
                            let reader_guid = dr.transport_reader.guid();
                            if let Some(writer_proxy) =
                                dr.transport_reader.matched_writer_lookup(writer_guid)
                            {
                                if writer_proxy.last_received_heartbeat_frag_count()
                                    < heartbeat_frag_submessage.count()
                                {
                                    writer_proxy.set_last_received_heartbeat_frag_count(
                                        heartbeat_frag_submessage.count(),
                                    );
                                    writer_proxy.write_message(
                                        &reader_guid,
                                        self.transport.message_writer.as_mut(),
                                    );
                                }
                            }
                        }
                    }
                    RtpsSubmessageReadKind::AckNack(ack_nack_submessage) => {
                        for dw in self
                            .domain_participant
                            .user_defined_publisher_list
                            .iter_mut()
                            .flat_map(|p| p.data_writer_list.iter_mut())
                        {
                            if dw
                                .transport_writer
                                .on_acknack_submessage_received(
                                    ack_nack_submessage,
                                    message_receiver.source_guid_prefix(),
                                    self.transport.message_writer.as_mut(),
                                    now,
                                )
                                .is_some()
                            {
                                if let Some(x) = dw.acknowledgement_notification.take() {
                                    x.send(());
                                }

                                if dw
                                    .transport_writer
                                    .is_change_acknowledged(dw.last_change_sequence_number)
                                {
                                    for n in dw.wait_for_acknowledgments_notification.drain(..) {
                                        n.send(Ok(()));
                                    }
                                }
                            }
                        }
                        for dw in self
                            .domain_participant
                            .builtin_publisher
                            .stateful_data_writer_list_mut()
                        {
                            dw.transport_writer.on_acknack_submessage_received(
                                ack_nack_submessage,
                                message_receiver.source_guid_prefix(),
                                self.transport.message_writer.as_mut(),
                                now,
                            );
                        }
                        self.process_pending_write_samples(now);
                    }
                    RtpsSubmessageReadKind::NackFrag(nack_frag_submessage) => {
                        for dw in self
                            .domain_participant
                            .user_defined_publisher_list
                            .iter_mut()
                            .flat_map(|p| p.data_writer_list.iter_mut())
                        {
                            dw.transport_writer.on_nack_frag_submessage_received(
                                nack_frag_submessage,
                                message_receiver.source_guid_prefix(),
                                self.transport.message_writer.as_mut(),
                            );
                        }
                        for dw in self
                            .domain_participant
                            .builtin_publisher
                            .stateful_data_writer_list_mut()
                        {
                            dw.transport_writer.on_nack_frag_submessage_received(
                                nack_frag_submessage,
                                message_receiver.source_guid_prefix(),
                                self.transport.message_writer.as_mut(),
                            );
                        }
                    }
                    _ => (),
                }
            }
        }
    }

    fn handle_data_submessage(
        &mut self,
        message_receiver: &MessageReceiver<'_>,
        data_submessage: &DataSubmessage,
    ) {
        self.domain_participant
            .builtin_subscriber
            .dcps_participant_reader
            .transport_reader
            .on_data_submessage(
                data_submessage,
                message_receiver.source_guid_prefix(),
                message_receiver.source_timestamp(),
            );
        for dr in self
            .domain_participant
            .user_defined_subscriber_list
            .iter_mut()
            .flat_map(|s| s.data_reader_list.iter_mut().map(|dr| &mut dr.reader))
            .chain(
                self.domain_participant
                    .builtin_subscriber
                    .stateful_data_reader_list_mut(),
            )
        {
            dr.transport_reader.on_data_submessage(
                data_submessage,
                message_receiver.source_guid_prefix(),
                message_receiver.source_timestamp(),
            );
        }
    }

    #[inline]
    fn handle_gap_submessage(
        &mut self,
        message_receiver: &MessageReceiver<'_>,
        gap_submessage: &GapSubmessage,
    ) {
        for dr in self
            .domain_participant
            .user_defined_subscriber_list
            .iter_mut()
            .flat_map(|s| s.data_reader_list.iter_mut().map(|dr| &mut dr.reader))
            .chain(
                self.domain_participant
                    .builtin_subscriber
                    .stateful_data_reader_list_mut(),
            )
        {
            let writer_guid = Guid::new(
                message_receiver.source_guid_prefix(),
                gap_submessage.writer_id(),
            );
            if let Some(writer_proxy) = dr.transport_reader.matched_writer_lookup(writer_guid) {
                for seq_num in gap_submessage.gap_start()..gap_submessage.gap_list().base() {
                    writer_proxy.irrelevant_change_set(seq_num)
                }

                for seq_num in gap_submessage.gap_list().set() {
                    writer_proxy.irrelevant_change_set(seq_num)
                }
            }
        }
    }

    fn handle_heartbeat_submessage(
        &mut self,
        message_receiver: &MessageReceiver<'_>,
        heartbeat_submessage: &HeartbeatSubmessage,
    ) {
        for s in self
            .domain_participant
            .user_defined_subscriber_list
            .iter_mut()
        {
            for dr in s.data_reader_list.iter_mut() {
                let writer_guid = Guid::new(
                    message_receiver.source_guid_prefix(),
                    heartbeat_submessage.writer_id(),
                );
                let reader_guid = dr.transport_reader.guid();
                if let Some(writer_proxy) = dr.transport_reader.matched_writer_lookup(writer_guid) {
                    if writer_proxy.last_received_heartbeat_count() < heartbeat_submessage.count() {
                        writer_proxy
                            .set_last_received_heartbeat_count(heartbeat_submessage.count());
                        writer_proxy.missing_changes_update(heartbeat_submessage.last_sn());
                        writer_proxy.lost_changes_update(heartbeat_submessage.first_sn());

                        let must_send_acknacks = !heartbeat_submessage.final_flag()
                            || (!heartbeat_submessage.liveliness_flag()
                                && writer_proxy.missing_changes().count() > 0);
                        writer_proxy.set_must_send_acknacks(must_send_acknacks);

                        writer_proxy
                            .write_message(&reader_guid, self.transport.message_writer.as_mut());
                    }
                }

                if dr.transport_reader.is_historical_data_received() {
                    for n in dr.wait_for_historical_data_notification.drain(..) {
                        n.send(Ok(()));
                    }
                }
            }
        }
        for dr in self
            .domain_participant
            .builtin_subscriber
            .stateful_data_reader_list_mut()
        {
            let writer_guid = Guid::new(
                message_receiver.source_guid_prefix(),
                heartbeat_submessage.writer_id(),
            );
            let reader_guid = dr.transport_reader.guid();
            if let Some(writer_proxy) = dr.transport_reader.matched_writer_lookup(writer_guid) {
                if writer_proxy.last_received_heartbeat_count() < heartbeat_submessage.count() {
                    writer_proxy.set_last_received_heartbeat_count(heartbeat_submessage.count());
                    writer_proxy.missing_changes_update(heartbeat_submessage.last_sn());
                    writer_proxy.lost_changes_update(heartbeat_submessage.first_sn());

                    let must_send_acknacks = !heartbeat_submessage.final_flag()
                        || (!heartbeat_submessage.liveliness_flag()
                            && writer_proxy.missing_changes().count() > 0);
                    writer_proxy.set_must_send_acknacks(must_send_acknacks);

                    writer_proxy
                        .write_message(&reader_guid, self.transport.message_writer.as_mut());
                }
            }
        }
    }

    fn handle_data_frag_submessage(
        &mut self,
        message_receiver: &MessageReceiver<'_>,
        data_frag_submessage: &DataFragSubmessage,
    ) {
        for dr in self
            .domain_participant
            .user_defined_subscriber_list
            .iter_mut()
            .flat_map(|s| s.data_reader_list.iter_mut().map(|dr| &mut dr.reader))
            .chain(
                self.domain_participant
                    .builtin_subscriber
                    .stateful_data_reader_list_mut(),
            )
        {
            dr.transport_reader.on_data_frag_submessage(
                data_frag_submessage,
                message_receiver.source_guid_prefix(),
                message_receiver.source_timestamp(),
            );
        }
    }

    pub fn poke(&mut self, now: Time) {
        self.domain_participant
            .builtin_publisher
            .dcps_participant_writer
            .transport_writer
            .write_message(self.transport.message_writer.as_mut());
        for dw in self
            .domain_participant
            .builtin_publisher
            .stateful_data_writer_list_mut()
        {
            dw.transport_writer
                .write_message(self.transport.message_writer.as_mut(), now);
        }
        for dw in self
            .domain_participant
            .user_defined_publisher_list
            .iter_mut()
            .flat_map(|p| p.data_writer_list.iter_mut())
        {
            dw.transport_writer
                .write_message(self.transport.message_writer.as_mut(), now);
        }
    }
}
