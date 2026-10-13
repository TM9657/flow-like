use alloc::{string::String, vec::Vec};

use crate::{
    builtin_topics::SubscriptionBuiltinTopicData,
    dcps::{
        channels::oneshot::OneshotSender,
        dcps_domain_participant::{
            data_writer_entity::serialize, participant_entity::DcpsDomainParticipant,
            user_defined_data_writer::PendingWriteSample,
        },
        listeners::data_writer_listener::DcpsDataWriterListener,
        status_mask::StatusMask,
        xtypes_glue::key_and_instance_handle::{
            KeyHolderData, KeyHolderType, get_instance_handle_from_dynamic_data_and_key_holder,
            get_instance_handle_from_key_holder_data,
        },
    },
    infrastructure::{
        error::{DdsError, DdsResult},
        instance::InstanceHandle,
        qos::{DataWriterQos, QosKind},
        qos_policy::{HistoryQosPolicyKind, ReliabilityQosPolicyKind},
        status::{OfferedDeadlineMissedStatus, PublicationMatchedStatus, StatusKind},
        time::{DurationKind, Time},
    },
    runtime::DdsRuntime,
    transport::types::TopicKind,
    xtypes::dynamic_type::DynamicData,
};

impl DcpsDomainParticipant {
    #[tracing::instrument(skip(self))]
    pub fn get_publication_matched_status(
        &mut self,
        publisher_handle: &InstanceHandle,
        data_writer_handle: &InstanceHandle,
    ) -> DdsResult<PublicationMatchedStatus> {
        let Some(publisher) = self
            .domain_participant
            .user_defined_publisher_list
            .iter_mut()
            .find(|x| &x.instance_handle == publisher_handle)
        else {
            return Err(DdsError::AlreadyDeleted);
        };
        let Some(data_writer) = publisher
            .data_writer_list
            .iter_mut()
            .find(|x| &x.instance_handle == data_writer_handle)
        else {
            return Err(DdsError::AlreadyDeleted);
        };

        let status = data_writer.publication_matched_status.get();

        data_writer
            .status_condition
            .remove_communication_state(StatusKind::PublicationMatched);
        Ok(status)
    }

    #[tracing::instrument(skip(self, dcps_listener, runtime))]
    pub fn set_listener_data_writer(
        &mut self,
        publisher_handle: &InstanceHandle,
        data_writer_handle: &InstanceHandle,
        dcps_listener: Option<DcpsDataWriterListener>,
        listener_mask: StatusMask,
        runtime: &impl DdsRuntime,
    ) -> DdsResult<()> {
        let Some(publisher) = self
            .domain_participant
            .user_defined_publisher_list
            .iter_mut()
            .find(|x| &x.instance_handle == publisher_handle)
        else {
            return Ok(());
        };
        let Some(data_writer) = publisher
            .data_writer_list
            .iter_mut()
            .find(|x| &x.instance_handle == data_writer_handle)
        else {
            return Ok(());
        };

        let listener_sender = dcps_listener.map(|l| l.spawn(&runtime.spawner()));
        data_writer.listener_sender = listener_sender;
        data_writer.listener_mask = listener_mask;

        Ok(())
    }

    #[tracing::instrument(skip(self))]
    pub fn get_data_writer_qos(
        &mut self,
        publisher_handle: &InstanceHandle,
        data_writer_handle: &InstanceHandle,
    ) -> DdsResult<DataWriterQos> {
        let Some(publisher) = self
            .domain_participant
            .user_defined_publisher_list
            .iter_mut()
            .find(|x| &x.instance_handle == publisher_handle)
        else {
            return Err(DdsError::AlreadyDeleted);
        };
        let Some(data_writer) = publisher
            .data_writer_list
            .iter_mut()
            .find(|x| &x.instance_handle == data_writer_handle)
        else {
            return Err(DdsError::AlreadyDeleted);
        };

        Ok(data_writer.qos.clone())
    }

    #[tracing::instrument(skip(self))]
    pub fn get_matched_subscriptions(
        &mut self,
        publisher_handle: &InstanceHandle,
        data_writer_handle: &InstanceHandle,
    ) -> DdsResult<Vec<InstanceHandle>> {
        let Some(publisher) = self
            .domain_participant
            .user_defined_publisher_list
            .iter_mut()
            .find(|x| &x.instance_handle == publisher_handle)
        else {
            return Err(DdsError::AlreadyDeleted);
        };
        let Some(data_writer) = publisher
            .data_writer_list
            .iter_mut()
            .find(|x| &x.instance_handle == data_writer_handle)
        else {
            return Err(DdsError::AlreadyDeleted);
        };
        Ok(data_writer
            .matched_subscription_list
            .iter()
            .map(|x| InstanceHandle::new(x.key().value))
            .collect())
    }

    #[tracing::instrument(skip(self))]
    pub fn get_matched_subscription_data(
        &mut self,
        publisher_handle: &InstanceHandle,
        data_writer_handle: &InstanceHandle,
        subscription_handle: &InstanceHandle,
    ) -> DdsResult<SubscriptionBuiltinTopicData> {
        let Some(publisher) = self
            .domain_participant
            .user_defined_publisher_list
            .iter_mut()
            .find(|x| &x.instance_handle == publisher_handle)
        else {
            return Err(DdsError::AlreadyDeleted);
        };
        let Some(data_writer) = publisher
            .data_writer_list
            .iter_mut()
            .find(|x| &x.instance_handle == data_writer_handle)
        else {
            return Err(DdsError::AlreadyDeleted);
        };
        data_writer
            .matched_subscription_list
            .iter()
            .find(|x| subscription_handle.as_ref() == &x.key().value)
            .ok_or(DdsError::BadParameter)
            .cloned()
    }

    #[tracing::instrument(skip(self))]
    pub fn register_instance(
        &mut self,
        publisher_handle: &InstanceHandle,
        data_writer_handle: &InstanceHandle,
        dynamic_data: &DynamicData<'static>,
        timestamp: Time,
    ) -> DdsResult<Option<InstanceHandle>> {
        let Some(publisher) = self
            .domain_participant
            .user_defined_publisher_list
            .iter_mut()
            .find(|x| &x.instance_handle == publisher_handle)
        else {
            return Err(DdsError::AlreadyDeleted);
        };
        let Some(data_writer) = publisher
            .data_writer_list
            .iter_mut()
            .find(|x| &x.instance_handle == data_writer_handle)
        else {
            return Err(DdsError::AlreadyDeleted);
        };
        let topic = self
            .domain_participant
            .locally_created_topic_list
            .iter()
            .find(|x| x.topic_name == data_writer.topic_name)
            .expect("Writer topic must exist");
        let type_support = self
            .domain_participant
            .type_register
            .get_dynamic_type(&topic.type_information.complete.typeid_with_size.type_id)
            .expect("Type must exist in type_register");

        data_writer.register_w_timestamp(dynamic_data, &type_support, timestamp)
    }

    #[tracing::instrument(skip(self))]
    pub fn unregister_instance(
        &mut self,
        publisher_handle: &InstanceHandle,
        data_writer_handle: &InstanceHandle,
        dynamic_data: &DynamicData<'static>,
        timestamp: Time,
        now: Time,
    ) -> DdsResult<()> {
        let Some(publisher) = self
            .domain_participant
            .user_defined_publisher_list
            .iter_mut()
            .find(|x| &x.instance_handle == publisher_handle)
        else {
            return Err(DdsError::AlreadyDeleted);
        };
        let Some(data_writer) = publisher
            .data_writer_list
            .iter_mut()
            .find(|x| &x.instance_handle == data_writer_handle)
        else {
            return Err(DdsError::AlreadyDeleted);
        };
        let topic = self
            .domain_participant
            .locally_created_topic_list
            .iter()
            .find(|x| x.topic_name == data_writer.topic_name)
            .expect("Writer topic must exist");
        let type_support = self
            .domain_participant
            .type_register
            .get_dynamic_type(&topic.type_information.complete.typeid_with_size.type_id)
            .expect("Type must exist in type_register");

        if TopicKind::from(&type_support) == TopicKind::NoKey {
            return Err(DdsError::IllegalOperation);
        }

        let key_holder_type = KeyHolderType::new(&dynamic_data.r#type());
        let key_holder_data = KeyHolderData::from_dynamic_data(dynamic_data, &key_holder_type)?;
        let instance_handle = get_instance_handle_from_key_holder_data(&key_holder_data)?;
        let serialized_key = serialize(
            key_holder_data.as_dynamic_data(),
            &data_writer.qos.representation,
        )?;

        let res = data_writer.unregister_w_timestamp(instance_handle, serialized_key, timestamp);
        if res.is_ok() {
            data_writer
                .transport_writer
                .write_message(self.transport.message_writer.as_mut(), now);
        }
        res
    }

    #[tracing::instrument(skip(self))]
    pub fn lookup_instance(
        &mut self,
        publisher_handle: &InstanceHandle,
        data_writer_handle: &InstanceHandle,
        dynamic_data: &DynamicData<'static>,
    ) -> DdsResult<Option<InstanceHandle>> {
        let Some(publisher) = self
            .domain_participant
            .user_defined_publisher_list
            .iter_mut()
            .find(|x| &x.instance_handle == publisher_handle)
        else {
            return Err(DdsError::AlreadyDeleted);
        };
        let Some(data_writer) = publisher
            .data_writer_list
            .iter_mut()
            .find(|x| &x.instance_handle == data_writer_handle)
        else {
            return Err(DdsError::AlreadyDeleted);
        };

        if !data_writer.enabled {
            return Err(DdsError::NotEnabled);
        }

        let instance_handle = match get_instance_handle_from_dynamic_data_and_key_holder(
            dynamic_data,
            &data_writer.key_holder_type,
        ) {
            Ok(k) => k,
            Err(e) => {
                return Err(e.into());
            }
        };

        Ok(data_writer
            .registered_instance_info
            .iter()
            .any(|x| x.instance_handle == instance_handle)
            .then_some(instance_handle))
    }

    #[allow(clippy::too_many_arguments)]
    #[tracing::instrument(skip(self, reply_sender))]
    pub fn write_w_timestamp(
        &mut self,
        publisher_handle: &InstanceHandle,
        data_writer_handle: &InstanceHandle,
        dynamic_data: &DynamicData<'static>,
        timestamp: Time,
        now: Time,
        reply_sender: OneshotSender<DdsResult<()>>,
    ) {
        let Some(publisher) = self
            .domain_participant
            .user_defined_publisher_list
            .iter_mut()
            .find(|x| &x.instance_handle == publisher_handle)
        else {
            reply_sender.send(Err(DdsError::AlreadyDeleted));
            return;
        };
        let Some(data_writer) = publisher
            .data_writer_list
            .iter_mut()
            .find(|x| &x.instance_handle == data_writer_handle)
        else {
            reply_sender.send(Err(DdsError::AlreadyDeleted));
            return;
        };

        if !data_writer.enabled {
            reply_sender.send(Err(DdsError::NotEnabled));
            return;
        }

        let serialized_data = match serialize(dynamic_data, &data_writer.qos.representation) {
            Ok(s) => s,
            Err(e) => {
                reply_sender.send(Err(e));
                return;
            }
        };

        let instance_handle = match get_instance_handle_from_dynamic_data_and_key_holder(
            dynamic_data,
            &data_writer.key_holder_type,
        ) {
            Ok(h) => h,
            Err(e) => {
                reply_sender.send(Err(e.into()));
                return;
            }
        };

        if let HistoryQosPolicyKind::KeepLast(depth) = data_writer.qos.history.kind {
            let smallest_seq_num_instance = data_writer
                .registered_instance_info
                .iter()
                .find(|x| x.instance_handle == instance_handle)
                .and_then(|s| {
                    if s.samples.len() == depth as usize {
                        s.samples.front().copied()
                    } else {
                        None
                    }
                });

            if let Some(smallest_seq_num_instance) = smallest_seq_num_instance {
                if data_writer.qos.reliability.kind == ReliabilityQosPolicyKind::Reliable
                    && !data_writer
                        .transport_writer
                        .is_change_acknowledged(smallest_seq_num_instance)
                {
                    if data_writer.pending_write_sample.is_some() {
                        reply_sender.send(Err(DdsError::Error(String::from(
                            "Another writer already waiting for acknowledgements.",
                        ))));
                        return;
                    }
                    let expiration_time = match data_writer.qos.reliability.max_blocking_time {
                        DurationKind::Finite(t) => Some(now + t),
                        DurationKind::Infinite => None,
                    };
                    data_writer.pending_write_sample = Some(PendingWriteSample {
                        dynamic_data: dynamic_data.clone(),
                        timestamp,
                        reply_sender,
                        expiration_time,
                    });
                    return;
                }

                if let Some(s) = data_writer
                    .registered_instance_info
                    .iter_mut()
                    .find(|x| x.instance_handle == instance_handle)
                {
                    if let Some(smallest_seq_num_instance) = s.samples.pop_front() {
                        data_writer
                            .transport_writer
                            .remove_change(smallest_seq_num_instance);
                    }
                }
            }
        }

        let write_result =
            data_writer.write_w_timestamp(instance_handle, serialized_data, timestamp, now);
        if write_result.is_err() {
            reply_sender.send(write_result);
            return;
        }

        reply_sender.send(Ok(()));

        data_writer
            .transport_writer
            .write_message(self.transport.message_writer.as_mut(), now);
    }

    #[tracing::instrument(skip(self))]
    pub fn dispose_w_timestamp(
        &mut self,
        publisher_handle: &InstanceHandle,
        data_writer_handle: &InstanceHandle,
        dynamic_data: &DynamicData<'static>,
        timestamp: Time,
        now: Time,
    ) -> DdsResult<()> {
        let Some(publisher) = self
            .domain_participant
            .user_defined_publisher_list
            .iter_mut()
            .find(|x| &x.instance_handle == publisher_handle)
        else {
            return Err(DdsError::AlreadyDeleted);
        };
        let Some(data_writer) = publisher
            .data_writer_list
            .iter_mut()
            .find(|x| &x.instance_handle == data_writer_handle)
        else {
            return Err(DdsError::AlreadyDeleted);
        };
        let topic = self
            .domain_participant
            .locally_created_topic_list
            .iter()
            .find(|x| x.topic_name == data_writer.topic_name)
            .expect("Writer topic must exist");
        let type_support = self
            .domain_participant
            .type_register
            .get_dynamic_type(&topic.type_information.complete.typeid_with_size.type_id)
            .expect("Type must exist in type_register");

        if TopicKind::from(&type_support) == TopicKind::NoKey {
            return Err(DdsError::IllegalOperation);
        }

        let key_holder_type = KeyHolderType::new(&dynamic_data.r#type());
        let key_holder_data = KeyHolderData::from_dynamic_data(dynamic_data, &key_holder_type)?;
        let instance_handle = get_instance_handle_from_key_holder_data(&key_holder_data)?;
        let serialized_key = serialize(
            key_holder_data.as_dynamic_data(),
            &data_writer.qos.representation,
        )?;

        let res = data_writer.dispose_w_timestamp(instance_handle, serialized_key, timestamp);
        if res.is_ok() {
            data_writer
                .transport_writer
                .write_message(self.transport.message_writer.as_mut(), now);
        }
        res
    }

    #[tracing::instrument(skip(self))]
    pub fn get_offered_deadline_missed_status(
        &mut self,
        publisher_handle: &InstanceHandle,
        data_writer_handle: &InstanceHandle,
    ) -> DdsResult<OfferedDeadlineMissedStatus> {
        let Some(publisher) = self
            .domain_participant
            .user_defined_publisher_list
            .iter_mut()
            .find(|x| &x.instance_handle == publisher_handle)
        else {
            return Err(DdsError::AlreadyDeleted);
        };
        let Some(data_writer) = publisher
            .data_writer_list
            .iter_mut()
            .find(|x| &x.instance_handle == data_writer_handle)
        else {
            return Err(DdsError::AlreadyDeleted);
        };

        Ok(data_writer.get_offered_deadline_missed_status())
    }

    #[tracing::instrument(skip(self))]
    pub fn enable_data_writer(
        &mut self,
        publisher_handle: &InstanceHandle,
        data_writer_handle: &InstanceHandle,
        now: Time,
    ) -> DdsResult<()> {
        let Some(publisher) = self
            .domain_participant
            .user_defined_publisher_list
            .iter_mut()
            .find(|x| &x.instance_handle == publisher_handle)
        else {
            return Err(DdsError::AlreadyDeleted);
        };
        let Some(data_writer) = publisher
            .data_writer_list
            .iter_mut()
            .find(|x| &x.instance_handle == data_writer_handle)
        else {
            return Err(DdsError::AlreadyDeleted);
        };
        if !data_writer.enabled {
            data_writer.enabled = true;

            self.announce_data_writer(publisher_handle, data_writer_handle, now);
            self.process_discovered_readers(now);

            if let Some(publisher) = self
                .domain_participant
                .user_defined_publisher_list
                .iter_mut()
                .find(|x| &x.instance_handle == publisher_handle)
            {
                if let Some(data_writer) = publisher
                    .data_writer_list
                    .iter_mut()
                    .find(|x| &x.instance_handle == data_writer_handle)
                {
                    data_writer
                        .transport_writer
                        .write_message(self.transport.message_writer.as_mut(), now);
                }
            }
        }
        Ok(())
    }

    #[tracing::instrument(skip(self))]
    pub fn set_data_writer_qos(
        &mut self,
        publisher_handle: &InstanceHandle,
        data_writer_handle: &InstanceHandle,
        qos: QosKind<DataWriterQos>,
        now: Time,
    ) -> DdsResult<()> {
        let Some(publisher) = self
            .domain_participant
            .user_defined_publisher_list
            .iter_mut()
            .find(|x| &x.instance_handle == publisher_handle)
        else {
            return Err(DdsError::AlreadyDeleted);
        };
        let qos = match qos {
            QosKind::Default => publisher.default_datawriter_qos.clone(),
            QosKind::Specific(q) => q,
        };
        let Some(data_writer) = publisher
            .data_writer_list
            .iter_mut()
            .find(|x| &x.instance_handle == data_writer_handle)
        else {
            return Err(DdsError::AlreadyDeleted);
        };

        qos.is_consistent()?;
        if data_writer.enabled {
            data_writer.qos.check_immutability(&qos)?;
        }
        data_writer.qos = qos;

        if data_writer.enabled {
            self.announce_data_writer(publisher_handle, data_writer_handle, now);
        }
        Ok(())
    }

    // This is a slighlty special function in the send that the answer is sent from
    // here directly because the reply sender is used as the notification mechanism
    // to notify the caller that all the changes are acknowledged.
    #[tracing::instrument(skip(self, notify_sender))]
    pub fn notify_acknowledgments(
        &mut self,
        publisher_handle: &InstanceHandle,
        data_writer_handle: &InstanceHandle,
        notify_sender: OneshotSender<DdsResult<()>>,
    ) {
        let Some(publisher) = self
            .domain_participant
            .user_defined_publisher_list
            .iter_mut()
            .find(|x| &x.instance_handle == publisher_handle)
        else {
            return notify_sender.send(Err(DdsError::AlreadyDeleted));
        };

        let Some(data_writer) = publisher
            .data_writer_list
            .iter_mut()
            .find(|x| &x.instance_handle == data_writer_handle)
        else {
            return notify_sender.send(Err(DdsError::AlreadyDeleted));
        };

        if data_writer
            .transport_writer
            .is_change_acknowledged(data_writer.last_change_sequence_number)
        {
            notify_sender.send(Ok(()));
        } else {
            data_writer
                .wait_for_acknowledgments_notification
                .push(notify_sender);
        }
    }

    pub fn process_pending_write_samples(&mut self, now: Time) {
        for publisher in &mut self.domain_participant.user_defined_publisher_list {
            for data_writer in &mut publisher.data_writer_list {
                if let Some(pending) = &data_writer.pending_write_sample {
                    if !data_writer.enabled {
                        continue;
                    }
                    let Ok(instance_handle) = get_instance_handle_from_dynamic_data_and_key_holder(
                        &pending.dynamic_data,
                        &data_writer.key_holder_type,
                    ) else {
                        continue;
                    };

                    let can_write = if let HistoryQosPolicyKind::KeepLast(depth) =
                        data_writer.qos.history.kind
                    {
                        let smallest_seq_num_instance = data_writer
                            .registered_instance_info
                            .iter()
                            .find(|x| x.instance_handle == instance_handle)
                            .and_then(|s| {
                                if s.samples.len() == depth as usize {
                                    s.samples.front().copied()
                                } else {
                                    None
                                }
                            });

                        if let Some(smallest_seq_num_instance) = smallest_seq_num_instance {
                            !(data_writer.qos.reliability.kind
                                == ReliabilityQosPolicyKind::Reliable)
                                || data_writer
                                    .transport_writer
                                    .is_change_acknowledged(smallest_seq_num_instance)
                        } else {
                            true
                        }
                    } else {
                        true
                    };

                    if can_write {
                        let pending = data_writer.pending_write_sample.take().unwrap();
                        let serialized_data =
                            match serialize(&pending.dynamic_data, &data_writer.qos.representation)
                            {
                                Ok(s) => s,
                                Err(e) => {
                                    pending.reply_sender.send(Err(e));
                                    continue;
                                }
                            };

                        if let HistoryQosPolicyKind::KeepLast(depth) = data_writer.qos.history.kind
                        {
                            if let Some(s) = data_writer
                                .registered_instance_info
                                .iter_mut()
                                .find(|x| x.instance_handle == instance_handle)
                            {
                                if s.samples.len() == depth as usize {
                                    if let Some(smallest_seq_num_instance) = s.samples.pop_front() {
                                        data_writer
                                            .transport_writer
                                            .remove_change(smallest_seq_num_instance);
                                    }
                                }
                            }
                        }

                        let write_result = data_writer.write_w_timestamp(
                            instance_handle,
                            serialized_data,
                            pending.timestamp,
                            now,
                        );
                        if write_result.is_err() {
                            pending.reply_sender.send(write_result);
                        } else {
                            pending.reply_sender.send(Ok(()));
                            data_writer
                                .transport_writer
                                .write_message(self.transport.message_writer.as_mut(), now);
                        }
                    }
                }
            }
        }
    }

    pub fn check_pending_writer_sample_timeout(&mut self, now: Time) {
        for publisher in &mut self.domain_participant.user_defined_publisher_list {
            for data_writer in &mut publisher.data_writer_list {
                if let Some(pending) = &data_writer.pending_write_sample {
                    if let Some(expiration_time) = pending.expiration_time {
                        if now >= expiration_time {
                            let pending = data_writer.pending_write_sample.take().unwrap();
                            pending.reply_sender.send(Err(DdsError::Timeout));
                        }
                    }
                }
            }
        }
    }
}
