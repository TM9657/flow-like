use alloc::{string::String, sync::Arc};

use crate::{
    builtin_topics::{DCPS_PARTICIPANT, DCPS_PUBLICATION, DCPS_SUBSCRIPTION, DCPS_TOPIC},
    dcps::{
        dcps_domain_participant::{
            builtin_constants::{TYPE_LOOKUP_REPLY_TOPIC_NAME, TYPE_LOOKUP_REQUEST_TOPIC_NAME},
            participant_entity::{BUILT_IN_TOPIC_NAME_LIST, DcpsDomainParticipant},
            user_defined_data_reader::UserDefinedDataReader,
        },
        listeners::{
            data_reader_listener::DcpsDataReaderListener,
            subscriber_listener::DcpsSubscriberListener,
        },
        status_mask::StatusMask,
    },
    infrastructure::{
        error::{DdsError, DdsResult},
        instance::InstanceHandle,
        qos::{DataReaderQos, QosKind, SubscriberQos},
        qos_policy::ReliabilityQosPolicyKind,
        time::Time,
    },
    rtps::stateful_reader::RtpsStatefulReader,
    runtime::DdsRuntime,
    transport::types::{
        EntityId, Guid, ReliabilityKind, TopicKind, USER_DEFINED_READER_NO_KEY,
        USER_DEFINED_READER_WITH_KEY,
    },
};

impl DcpsDomainParticipant {
    #[allow(clippy::too_many_arguments)]
    #[tracing::instrument(skip(self, dcps_listener, runtime))]
    pub fn create_data_reader(
        &mut self,
        subscriber_handle: &InstanceHandle,
        topic_name: String,
        qos: QosKind<DataReaderQos>,
        dcps_listener: Option<DcpsDataReaderListener>,
        listener_mask: StatusMask,
        runtime: &impl DdsRuntime,
        now: Time,
    ) -> DdsResult<InstanceHandle> {
        let topic = if let Some(content_filtered_topic) = self
            .domain_participant
            .content_filtered_topic_list
            .iter()
            .find(|x| x.topic_name.as_ref() == topic_name.as_str())
        {
            let Some(topic) = self
                .domain_participant
                .locally_created_topic_list
                .iter()
                .find(|x| x.topic_name == content_filtered_topic.related_topic_name)
            else {
                return Err(DdsError::AlreadyDeleted);
            };
            topic
        } else {
            let Some(topic) = self
                .domain_participant
                .locally_created_topic_list
                .iter()
                .find(|x| x.topic_name.as_ref() == topic_name.as_str())
            else {
                return Err(DdsError::AlreadyDeleted);
            };
            topic
        };

        let topic_kind = self
            .domain_participant
            .type_register
            .get_dynamic_type(&topic.type_information.complete.typeid_with_size.type_id)
            .map(|dt| TopicKind::from(&dt))
            .unwrap_or(TopicKind::NoKey);

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
            QosKind::Specific(q) => {
                if q.is_consistent().is_ok() {
                    q
                } else {
                    return Err(DdsError::InconsistentPolicy);
                }
            }
        };

        let entity_kind = match topic_kind {
            TopicKind::NoKey => USER_DEFINED_READER_NO_KEY,
            TopicKind::WithKey => USER_DEFINED_READER_WITH_KEY,
        };
        let entity_id = EntityId::new(
            [
                subscriber.instance_handle[12],
                self.reader_counter.to_ne_bytes()[0],
                self.reader_counter.to_ne_bytes()[1],
            ],
            entity_kind,
        );
        let reader_handle = InstanceHandle::new([
            self.domain_participant.instance_handle[0],
            self.domain_participant.instance_handle[1],
            self.domain_participant.instance_handle[2],
            self.domain_participant.instance_handle[3],
            self.domain_participant.instance_handle[4],
            self.domain_participant.instance_handle[5],
            self.domain_participant.instance_handle[6],
            self.domain_participant.instance_handle[7],
            self.domain_participant.instance_handle[8],
            self.domain_participant.instance_handle[9],
            self.domain_participant.instance_handle[10],
            self.domain_participant.instance_handle[11],
            entity_id.entity_key()[0],
            entity_id.entity_key()[1],
            entity_id.entity_key()[2],
            entity_id.entity_kind(),
        ]);
        self.reader_counter += 1;
        let reliablity_kind = match qos.reliability.kind {
            ReliabilityQosPolicyKind::BestEffort => ReliabilityKind::BestEffort,
            ReliabilityQosPolicyKind::Reliable => ReliabilityKind::Reliable,
        };
        let guid_prefix = Guid::from(*self.domain_participant.instance_handle.as_ref()).prefix();
        let guid = Guid::new(guid_prefix, entity_id);

        let transport_reader = RtpsStatefulReader::new(guid, reliablity_kind);

        let listener_sender = dcps_listener.map(|l| l.spawn(&runtime.spawner()));
        let data_reader = UserDefinedDataReader::new(
            reader_handle,
            qos,
            Arc::from(topic_name),
            listener_sender,
            listener_mask,
            transport_reader,
        );

        let data_reader_handle = data_reader.instance_handle;

        subscriber.data_reader_list.push(data_reader);

        if subscriber.enabled && subscriber.qos.entity_factory.autoenable_created_entities {
            self.enable_data_reader(subscriber_handle, &data_reader_handle, now)?;
        }
        Ok(data_reader_handle)
    }

    #[tracing::instrument(skip(self))]
    pub fn delete_data_reader(
        &mut self,
        subscriber_handle: &InstanceHandle,
        datareader_handle: &InstanceHandle,
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

        if let Some(index) = subscriber
            .data_reader_list
            .iter()
            .position(|x| &x.instance_handle == datareader_handle)
        {
            let data_reader = subscriber.data_reader_list.remove(index);
            self.announce_deleted_data_reader(data_reader, now);
        } else {
            return Err(DdsError::AlreadyDeleted);
        };
        Ok(())
    }

    #[tracing::instrument(skip(self))]
    pub fn lookup_data_reader(
        &mut self,
        subscriber_handle: &InstanceHandle,
        topic_name: String,
    ) -> DdsResult<Option<InstanceHandle>> {
        if !BUILT_IN_TOPIC_NAME_LIST.contains(&topic_name.as_str())
            && !self
                .domain_participant
                .locally_created_topic_list
                .iter()
                .any(|x| x.topic_name.as_ref() == topic_name.as_str())
        {
            return Err(DdsError::BadParameter);
        }

        // Built-in subscriber is identified by the handle of the participant itself
        if &self.domain_participant.instance_handle == subscriber_handle {
            let handle = match topic_name.as_str() {
                DCPS_PARTICIPANT => {
                    self.domain_participant
                        .builtin_subscriber
                        .dcps_participant_reader
                        .instance_handle
                }
                DCPS_TOPIC => {
                    self.domain_participant
                        .builtin_subscriber
                        .dcps_topic_reader
                        .instance_handle
                }
                DCPS_PUBLICATION => {
                    self.domain_participant
                        .builtin_subscriber
                        .dcps_publication_reader
                        .instance_handle
                }
                DCPS_SUBSCRIPTION => {
                    self.domain_participant
                        .builtin_subscriber
                        .dcps_subscription_reader
                        .instance_handle
                }
                TYPE_LOOKUP_REQUEST_TOPIC_NAME => {
                    self.domain_participant
                        .builtin_subscriber
                        .type_lookup_request_reader
                        .instance_handle
                }
                TYPE_LOOKUP_REPLY_TOPIC_NAME => {
                    self.domain_participant
                        .builtin_subscriber
                        .type_lookup_reply_reader
                        .instance_handle
                }
                _ => return Ok(None),
            };
            Ok(Some(handle))
        } else {
            let Some(s) = self
                .domain_participant
                .user_defined_subscriber_list
                .iter_mut()
                .find(|x| &x.instance_handle == subscriber_handle)
            else {
                return Err(DdsError::AlreadyDeleted);
            };
            Ok(s.data_reader_list
                .iter_mut()
                .find(|dr| dr.topic_name.as_ref() == topic_name.as_str())
                .map(|x| x.instance_handle))
        }
    }

    #[tracing::instrument(skip(self))]
    pub fn set_default_data_reader_qos(
        &mut self,
        subscriber_handle: &InstanceHandle,
        qos: QosKind<DataReaderQos>,
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
            QosKind::Default => DataReaderQos::default(),
            QosKind::Specific(q) => q,
        };
        qos.is_consistent()?;
        subscriber.default_data_reader_qos = qos;
        Ok(())
    }

    #[tracing::instrument(skip(self))]
    pub fn get_default_data_reader_qos(
        &mut self,
        subscriber_handle: &InstanceHandle,
    ) -> DdsResult<DataReaderQos> {
        let Some(subscriber) = self
            .domain_participant
            .user_defined_subscriber_list
            .iter_mut()
            .find(|x| &x.instance_handle == subscriber_handle)
        else {
            return Err(DdsError::AlreadyDeleted);
        };

        Ok(subscriber.default_data_reader_qos.clone())
    }

    #[tracing::instrument(skip(self))]
    pub fn set_subscriber_qos(
        &mut self,
        subscriber_handle: &InstanceHandle,
        qos: QosKind<SubscriberQos>,
    ) -> DdsResult<()> {
        let qos = match qos {
            QosKind::Default => self.domain_participant.default_subscriber_qos.clone(),
            QosKind::Specific(q) => q,
        };

        let Some(subscriber) = self
            .domain_participant
            .user_defined_subscriber_list
            .iter_mut()
            .find(|x| &x.instance_handle == subscriber_handle)
        else {
            return Err(DdsError::AlreadyDeleted);
        };

        if subscriber.enabled {
            subscriber.qos.check_immutability(&qos)?;
        }
        subscriber.qos = qos;
        Ok(())
    }

    #[tracing::instrument(skip(self))]
    pub fn get_subscriber_qos(
        &mut self,
        subscriber_handle: &InstanceHandle,
    ) -> DdsResult<SubscriberQos> {
        let Some(subscriber) = self
            .domain_participant
            .user_defined_subscriber_list
            .iter_mut()
            .find(|x| &x.instance_handle == subscriber_handle)
        else {
            return Err(DdsError::AlreadyDeleted);
        };

        Ok(subscriber.qos.clone())
    }

    #[tracing::instrument(skip(self, listener_sender_task, runtime))]
    pub fn set_subscriber_listener(
        &mut self,
        subscriber_handle: &InstanceHandle,
        listener_sender_task: Option<DcpsSubscriberListener>,
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

        let listener_sender = listener_sender_task.map(|l| l.spawn(&runtime.spawner()));
        subscriber.listener_sender = listener_sender;
        subscriber.listener_mask = listener_mask;
        Ok(())
    }
}
