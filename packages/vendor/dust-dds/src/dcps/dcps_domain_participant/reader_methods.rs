use alloc::vec::Vec;

use crate::{
    builtin_topics::{
        DCPS_PARTICIPANT, DCPS_PUBLICATION, DCPS_SUBSCRIPTION, DCPS_TOPIC,
        PublicationBuiltinTopicData,
    },
    dcps::{
        channels::oneshot::OneshotSender,
        data_representation_builtin_endpoints::{
            discovered_reader_data::DiscoveredReaderData,
            discovered_topic_data::DiscoveredTopicData,
            discovered_writer_data::DiscoveredWriterData,
            spdp_discovered_participant_data::SpdpDiscoveredParticipantData,
        },
        dcps_domain_participant::{
            participant_entity::DcpsDomainParticipant, topic_entity::get_topic_type_support,
        },
        listeners::data_reader_listener::DcpsDataReaderListener,
        status_mask::StatusMask,
    },
    infrastructure::{
        error::{DdsError, DdsResult},
        instance::InstanceHandle,
        qos::{DataReaderQos, QosKind},
        qos_policy::DurabilityQosPolicyKind,
        sample_info::{InstanceStateKind, SampleInfo, SampleStateKind, ViewStateKind},
        status::{StatusKind, SubscriptionMatchedStatus},
        time::Time,
    },
    runtime::DdsRuntime,
    xtypes::{
        deserializer::deserialize_top_level_type,
        dynamic_type::{DynamicData, DynamicType},
        type_support::TypeSupport,
    },
};

pub fn deserialize_topic_type(
    topic_name: &str,
    type_support: DynamicType<'static>,
    data: &[u8],
) -> Option<DynamicData<'static>> {
    match topic_name {
        DCPS_PARTICIPANT => SpdpDiscoveredParticipantData::from_bytes(data)
            .map(|x| x.dds_participant_data.create_dynamic_sample())
            .ok(),
        DCPS_PUBLICATION => DiscoveredWriterData::from_bytes(data)
            .map(|x| x.dds_publication_data.create_dynamic_sample())
            .ok(),
        DCPS_SUBSCRIPTION => DiscoveredReaderData::from_bytes(data)
            .map(|x| x.dds_subscription_data.create_dynamic_sample())
            .ok(),
        DCPS_TOPIC => DiscoveredTopicData::from_bytes(data)
            .map(|x| x.topic_builtin_topic_data.create_dynamic_sample())
            .ok(),
        _ => deserialize_top_level_type(type_support, data).ok(),
    }
}

impl DcpsDomainParticipant {
    #[allow(clippy::too_many_arguments, clippy::type_complexity)]
    #[tracing::instrument(skip(self))]
    pub fn read(
        &mut self,
        subscriber_handle: &InstanceHandle,
        data_reader_handle: &InstanceHandle,
        max_samples: i32,
        sample_states: &[SampleStateKind],
        view_states: &[ViewStateKind],
        instance_states: &[InstanceStateKind],
        specific_instance_handle: &Option<InstanceHandle>,
    ) -> DdsResult<Vec<(Option<DynamicData<'static>>, SampleInfo)>> {
        let (sample_list, topic_name) =
            if subscriber_handle == &self.domain_participant.instance_handle {
                let bs = &mut self.domain_participant.builtin_subscriber;
                if &bs.dcps_participant_reader.instance_handle == data_reader_handle {
                    let sample_list = bs.dcps_participant_reader.read(
                        max_samples,
                        sample_states,
                        view_states,
                        instance_states,
                        specific_instance_handle,
                    )?;
                    (sample_list, bs.dcps_participant_reader.topic_name.as_ref())
                } else if let Some(dr) = bs.find_stateful_data_reader_mut(data_reader_handle) {
                    let sample_list = dr.read(
                        max_samples,
                        sample_states,
                        view_states,
                        instance_states,
                        specific_instance_handle,
                    )?;
                    (sample_list, dr.topic_name.as_ref())
                } else {
                    return Err(DdsError::AlreadyDeleted);
                }
            } else {
                let subscriber = self
                    .domain_participant
                    .user_defined_subscriber_list
                    .iter_mut()
                    .find(|x| &x.instance_handle == subscriber_handle);
                let Some(subscriber) = subscriber else {
                    return Err(DdsError::AlreadyDeleted);
                };
                let Some(data_reader) = subscriber
                    .data_reader_list
                    .iter_mut()
                    .find(|x| &x.instance_handle == data_reader_handle)
                else {
                    return Err(DdsError::AlreadyDeleted);
                };
                let sample_list = data_reader.read(
                    max_samples,
                    sample_states,
                    view_states,
                    instance_states,
                    specific_instance_handle,
                )?;
                (sample_list, data_reader.topic_name.as_ref())
            };

        let Some(type_support) = get_topic_type_support(
            topic_name,
            &self.domain_participant.content_filtered_topic_list,
            &self.domain_participant.locally_created_topic_list,
            &self.domain_participant.type_register,
        ) else {
            return Err(DdsError::AlreadyDeleted);
        };

        Ok(sample_list
            .into_iter()
            .map(|(data, info)| {
                (
                    if info.valid_data {
                        deserialize_topic_type(topic_name, type_support, data.as_ref())
                    } else {
                        None
                    },
                    info,
                )
            })
            .collect())
    }

    #[allow(clippy::too_many_arguments, clippy::type_complexity)]
    #[tracing::instrument(skip(self))]
    pub fn take(
        &mut self,
        subscriber_handle: &InstanceHandle,
        data_reader_handle: &InstanceHandle,
        max_samples: i32,
        sample_states: &[SampleStateKind],
        view_states: &[ViewStateKind],
        instance_states: &[InstanceStateKind],
        specific_instance_handle: &Option<InstanceHandle>,
    ) -> DdsResult<Vec<(Option<DynamicData<'static>>, SampleInfo)>> {
        let Some(subscriber) = self
            .domain_participant
            .user_defined_subscriber_list
            .iter_mut()
            .find(|x| &x.instance_handle == subscriber_handle)
        else {
            return Err(DdsError::AlreadyDeleted);
        };
        let Some(data_reader) = subscriber
            .data_reader_list
            .iter_mut()
            .find(|x| &x.instance_handle == data_reader_handle)
        else {
            return Err(DdsError::AlreadyDeleted);
        };
        let sample_list = data_reader.take(
            max_samples,
            sample_states,
            view_states,
            instance_states,
            specific_instance_handle,
        )?;

        let Some(type_support) = get_topic_type_support(
            &data_reader.topic_name,
            &self.domain_participant.content_filtered_topic_list,
            &self.domain_participant.locally_created_topic_list,
            &self.domain_participant.type_register,
        ) else {
            return Err(DdsError::AlreadyDeleted);
        };

        Ok(sample_list
            .into_iter()
            .map(|(data, info)| {
                (
                    if info.valid_data {
                        deserialize_topic_type(&data_reader.topic_name, type_support, data.as_ref())
                    } else {
                        None
                    },
                    info,
                )
            })
            .collect())
    }

    #[allow(clippy::too_many_arguments, clippy::type_complexity)]
    #[tracing::instrument(skip(self))]
    pub fn read_next_instance(
        &mut self,
        subscriber_handle: &InstanceHandle,
        data_reader_handle: &InstanceHandle,
        max_samples: i32,
        previous_handle: &Option<InstanceHandle>,
        sample_states: &[SampleStateKind],
        view_states: &[ViewStateKind],
        instance_states: &[InstanceStateKind],
    ) -> DdsResult<Vec<(Option<DynamicData<'static>>, SampleInfo)>> {
        let Some(subscriber) = self
            .domain_participant
            .user_defined_subscriber_list
            .iter_mut()
            .find(|x| &x.instance_handle == subscriber_handle)
        else {
            return Err(DdsError::AlreadyDeleted);
        };
        let Some(data_reader) = subscriber
            .data_reader_list
            .iter_mut()
            .find(|x| &x.instance_handle == data_reader_handle)
        else {
            return Err(DdsError::AlreadyDeleted);
        };
        let sample_list = data_reader.read_next_instance(
            max_samples,
            previous_handle,
            sample_states,
            view_states,
            instance_states,
        )?;

        let Some(type_support) = get_topic_type_support(
            &data_reader.topic_name,
            &self.domain_participant.content_filtered_topic_list,
            &self.domain_participant.locally_created_topic_list,
            &self.domain_participant.type_register,
        ) else {
            return Err(DdsError::AlreadyDeleted);
        };

        Ok(sample_list
            .into_iter()
            .map(|(data, info)| {
                (
                    if info.valid_data {
                        deserialize_topic_type(&data_reader.topic_name, type_support, data.as_ref())
                    } else {
                        None
                    },
                    info,
                )
            })
            .collect())
    }

    #[allow(clippy::too_many_arguments, clippy::type_complexity)]
    #[tracing::instrument(skip(self))]
    pub fn take_next_instance(
        &mut self,
        subscriber_handle: &InstanceHandle,
        data_reader_handle: &InstanceHandle,
        max_samples: i32,
        previous_handle: &Option<InstanceHandle>,
        sample_states: &[SampleStateKind],
        view_states: &[ViewStateKind],
        instance_states: &[InstanceStateKind],
    ) -> DdsResult<Vec<(Option<DynamicData<'static>>, SampleInfo)>> {
        let Some(subscriber) = self
            .domain_participant
            .user_defined_subscriber_list
            .iter_mut()
            .find(|x| &x.instance_handle == subscriber_handle)
        else {
            return Err(DdsError::AlreadyDeleted);
        };
        let Some(data_reader) = subscriber
            .data_reader_list
            .iter_mut()
            .find(|x| &x.instance_handle == data_reader_handle)
        else {
            return Err(DdsError::AlreadyDeleted);
        };
        let sample_list = data_reader.take_next_instance(
            max_samples,
            previous_handle,
            sample_states,
            view_states,
            instance_states,
        )?;

        let Some(type_support) = get_topic_type_support(
            &data_reader.topic_name,
            &self.domain_participant.content_filtered_topic_list,
            &self.domain_participant.locally_created_topic_list,
            &self.domain_participant.type_register,
        ) else {
            return Err(DdsError::AlreadyDeleted);
        };

        Ok(sample_list
            .into_iter()
            .map(|(data, info)| {
                (
                    if info.valid_data {
                        deserialize_topic_type(&data_reader.topic_name, type_support, data.as_ref())
                    } else {
                        None
                    },
                    info,
                )
            })
            .collect())
    }

    #[tracing::instrument(skip(self))]
    pub fn get_subscription_matched_status(
        &mut self,
        subscriber_handle: &InstanceHandle,
        data_reader_handle: &InstanceHandle,
    ) -> DdsResult<SubscriptionMatchedStatus> {
        let Some(subscriber) = self
            .domain_participant
            .user_defined_subscriber_list
            .iter_mut()
            .find(|x| &x.instance_handle == subscriber_handle)
        else {
            return Err(DdsError::AlreadyDeleted);
        };
        let Some(data_reader) = subscriber
            .data_reader_list
            .iter_mut()
            .find(|x| &x.instance_handle == data_reader_handle)
        else {
            return Err(DdsError::AlreadyDeleted);
        };
        let status = data_reader.get_subscription_matched_status();
        data_reader
            .status_condition
            .remove_communication_state(StatusKind::SubscriptionMatched);
        Ok(status)
    }

    #[tracing::instrument(skip(self))]
    pub fn get_matched_publication_data(
        &mut self,
        subscriber_handle: &InstanceHandle,
        data_reader_handle: &InstanceHandle,
        publication_handle: &InstanceHandle,
    ) -> DdsResult<PublicationBuiltinTopicData> {
        let Some(subscriber) = self
            .domain_participant
            .user_defined_subscriber_list
            .iter_mut()
            .find(|x| &x.instance_handle == subscriber_handle)
        else {
            return Err(DdsError::AlreadyDeleted);
        };
        let Some(data_reader) = subscriber
            .data_reader_list
            .iter()
            .find(|x| &x.instance_handle == data_reader_handle)
        else {
            return Err(DdsError::AlreadyDeleted);
        };
        if !data_reader.enabled {
            return Err(DdsError::NotEnabled);
        }

        data_reader
            .matched_publication_list
            .iter()
            .find(|x| &x.key().value == publication_handle.as_ref())
            .cloned()
            .ok_or(DdsError::BadParameter)
    }

    #[tracing::instrument(skip(self))]
    pub fn get_matched_publications(
        &mut self,
        subscriber_handle: &InstanceHandle,
        data_reader_handle: &InstanceHandle,
    ) -> DdsResult<Vec<InstanceHandle>> {
        let Some(subscriber) = self
            .domain_participant
            .user_defined_subscriber_list
            .iter_mut()
            .find(|x| &x.instance_handle == subscriber_handle)
        else {
            return Err(DdsError::AlreadyDeleted);
        };
        let Some(data_reader) = subscriber
            .data_reader_list
            .iter()
            .find(|x| &x.instance_handle == data_reader_handle)
        else {
            return Err(DdsError::AlreadyDeleted);
        };

        Ok(data_reader.get_matched_publications())
    }

    #[tracing::instrument(skip(self))]
    pub fn set_data_reader_qos(
        &mut self,
        subscriber_handle: &InstanceHandle,
        data_reader_handle: &InstanceHandle,
        qos: QosKind<DataReaderQos>,
        now: Time,
    ) -> DdsResult<()> {
        let Some(subscriber) = self
            .domain_participant
            .user_defined_subscriber_list
            .iter_mut()
            .find(|x| &x.instance_handle == subscriber_handle)
        else {
            return Err(DdsError::AlreadyDeleted);
        };
        let qos = match qos {
            QosKind::Default => subscriber.default_data_reader_qos.clone(),
            QosKind::Specific(q) => q,
        };
        let Some(data_reader) = subscriber
            .data_reader_list
            .iter_mut()
            .find(|x| &x.instance_handle == data_reader_handle)
        else {
            return Err(DdsError::AlreadyDeleted);
        };

        qos.is_consistent()?;
        if data_reader.enabled {
            data_reader.qos.check_immutability(&qos)?
        }

        data_reader.qos = qos;

        if data_reader.enabled {
            self.announce_data_reader(subscriber_handle, data_reader_handle, now);
        }
        Ok(())
    }

    #[tracing::instrument(skip(self))]
    pub fn get_data_reader_qos(
        &mut self,
        subscriber_handle: &InstanceHandle,
        data_reader_handle: &InstanceHandle,
    ) -> DdsResult<DataReaderQos> {
        let Some(subscriber) = self
            .domain_participant
            .user_defined_subscriber_list
            .iter_mut()
            .find(|x| &x.instance_handle == subscriber_handle)
        else {
            return Err(DdsError::AlreadyDeleted);
        };

        let Some(data_reader) = subscriber
            .data_reader_list
            .iter_mut()
            .find(|x| &x.instance_handle == data_reader_handle)
        else {
            return Err(DdsError::AlreadyDeleted);
        };
        Ok(data_reader.qos.clone())
    }

    #[tracing::instrument(skip(self, dcps_listener, runtime))]
    pub fn set_data_reader_listener(
        &mut self,
        subscriber_handle: &InstanceHandle,
        data_reader_handle: &InstanceHandle,
        dcps_listener: Option<DcpsDataReaderListener>,
        listener_mask: StatusMask,
        runtime: &impl DdsRuntime,
    ) -> DdsResult<()> {
        let Some(subscriber) = self
            .domain_participant
            .user_defined_subscriber_list
            .iter_mut()
            .find(|x| &x.instance_handle == subscriber_handle)
        else {
            return Err(DdsError::AlreadyDeleted);
        };
        let Some(data_reader) = subscriber
            .data_reader_list
            .iter_mut()
            .find(|x| &x.instance_handle == data_reader_handle)
        else {
            return Err(DdsError::AlreadyDeleted);
        };
        let listener_sender = dcps_listener.map(|l| l.spawn(&runtime.spawner()));
        data_reader.listener_sender = listener_sender;
        data_reader.listener_mask = listener_mask;
        Ok(())
    }

    #[tracing::instrument(skip(self, notify_sender))]
    pub fn notify_historical_data(
        &mut self,
        subscriber_handle: &InstanceHandle,
        data_reader_handle: &InstanceHandle,
        notify_sender: OneshotSender<DdsResult<()>>,
    ) {
        let Some(subscriber) = self
            .domain_participant
            .user_defined_subscriber_list
            .iter_mut()
            .find(|x| &x.instance_handle == subscriber_handle)
        else {
            return notify_sender.send(Err(DdsError::AlreadyDeleted));
        };

        let Some(data_reader) = subscriber
            .data_reader_list
            .iter_mut()
            .find(|x| &x.instance_handle == data_reader_handle)
        else {
            return notify_sender.send(Err(DdsError::AlreadyDeleted));
        };
        if !data_reader.enabled {
            return notify_sender.send(Err(DdsError::NotEnabled));
        }

        match data_reader.qos.durability.kind {
            DurabilityQosPolicyKind::Volatile => {
                return notify_sender.send(Err(DdsError::IllegalOperation));
            }
            DurabilityQosPolicyKind::TransientLocal
            | DurabilityQosPolicyKind::Transient
            | DurabilityQosPolicyKind::Persistent => (),
        }

        if data_reader.transport_reader.is_historical_data_received() {
            notify_sender.send(Ok(()));
        } else {
            data_reader
                .wait_for_historical_data_notification
                .push(notify_sender);
        }
    }

    #[tracing::instrument(skip(self))]
    pub fn enable_data_reader(
        &mut self,
        subscriber_handle: &InstanceHandle,
        data_reader_handle: &InstanceHandle,
        now: Time,
    ) -> DdsResult<()> {
        let Some(subscriber) = self
            .domain_participant
            .user_defined_subscriber_list
            .iter_mut()
            .find(|x| &x.instance_handle == subscriber_handle)
        else {
            return Err(DdsError::AlreadyDeleted);
        };
        let Some(data_reader) = subscriber
            .data_reader_list
            .iter_mut()
            .find(|x| &x.instance_handle == data_reader_handle)
        else {
            return Err(DdsError::AlreadyDeleted);
        };
        if !data_reader.enabled {
            data_reader.enabled = true;

            self.announce_data_reader(subscriber_handle, data_reader_handle, now);
            self.process_discovered_writers(now);
        }
        Ok(())
    }
}
