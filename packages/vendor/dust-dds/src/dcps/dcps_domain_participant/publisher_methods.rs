use alloc::string::String;

use crate::{
    dcps::{
        dcps_domain_participant::{
            participant_entity::DcpsDomainParticipant,
            user_defined_data_writer::UserDefinedDataWriter,
        },
        listeners::{
            data_writer_listener::DcpsDataWriterListener, publisher_listener::DcpsPublisherListener,
        },
        status_mask::StatusMask,
    },
    infrastructure::{
        error::{DdsError, DdsResult},
        instance::InstanceHandle,
        qos::{DataWriterQos, PublisherQos, QosKind},
        time::Time,
    },
    rtps::stateful_writer::RtpsStatefulWriter,
    runtime::DdsRuntime,
    transport::types::{
        EntityId, Guid, TopicKind, USER_DEFINED_WRITER_NO_KEY, USER_DEFINED_WRITER_WITH_KEY,
    },
};

impl DcpsDomainParticipant {
    #[allow(clippy::too_many_arguments)]
    #[tracing::instrument(skip(self, dcps_listener, runtime))]
    pub fn create_data_writer(
        &mut self,
        publisher_handle: &InstanceHandle,
        topic_name: String,
        qos: QosKind<DataWriterQos>,
        dcps_listener: Option<DcpsDataWriterListener>,
        listener_mask: StatusMask,
        runtime: &impl DdsRuntime,
        now: Time,
    ) -> DdsResult<InstanceHandle> {
        let Some(topic) = self
            .domain_participant
            .locally_created_topic_list
            .iter()
            .find(|x| x.topic_name.as_ref() == topic_name.as_str())
        else {
            return Err(DdsError::AlreadyDeleted);
        };

        let topic_kind = self
            .domain_participant
            .type_register
            .get_dynamic_type(&topic.type_information.complete.typeid_with_size.type_id)
            .map(|dt| TopicKind::from(&dt))
            .unwrap_or(TopicKind::NoKey);

        let Some(publisher) = self
            .domain_participant
            .user_defined_publisher_list
            .iter_mut()
            .find(|x| &x.instance_handle == publisher_handle)
        else {
            return Err(DdsError::AlreadyDeleted);
        };

        let entity_kind = match topic_kind {
            TopicKind::WithKey => USER_DEFINED_WRITER_WITH_KEY,
            TopicKind::NoKey => USER_DEFINED_WRITER_NO_KEY,
        };

        let entity_id = EntityId::new(
            [
                publisher.instance_handle[12],
                self.writer_counter.to_le_bytes()[0],
                self.writer_counter.to_le_bytes()[1],
            ],
            entity_kind,
        );

        let writer_handle = InstanceHandle::new([
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
        self.writer_counter += 1;

        let qos = match qos {
            QosKind::Default => publisher.default_datawriter_qos.clone(),
            QosKind::Specific(q) => {
                if q.is_consistent().is_ok() {
                    q
                } else {
                    return Err(DdsError::InconsistentPolicy);
                }
            }
        };
        let guid = Guid::from(*self.domain_participant.instance_handle.as_ref());
        let transport_writer = RtpsStatefulWriter::new(
            Guid::new(guid.prefix(), entity_id),
            self.transport.fragment_size,
        );
        let key_holder_type = super::topic_entity::get_topic_type_support(
            &topic.topic_name,
            &self.domain_participant.content_filtered_topic_list,
            &self.domain_participant.locally_created_topic_list,
            &self.domain_participant.type_register,
        )
        .as_ref()
        .map(crate::dcps::xtypes_glue::key_and_instance_handle::KeyHolderType::new)
        .unwrap_or_default();

        let listener_sender = dcps_listener.map(|l| l.spawn(&runtime.spawner()));
        let data_writer = UserDefinedDataWriter::new(
            writer_handle,
            transport_writer,
            topic.topic_name.clone(),
            listener_sender,
            listener_mask,
            qos,
            key_holder_type,
        );
        let data_writer_handle = data_writer.instance_handle;

        publisher.data_writer_list.push(data_writer);

        if publisher.enabled && publisher.qos.entity_factory.autoenable_created_entities {
            self.enable_data_writer(publisher_handle, &writer_handle, now)?;
        }

        Ok(data_writer_handle)
    }

    #[tracing::instrument(skip(self))]
    pub fn delete_data_writer(
        &mut self,
        publisher_handle: &InstanceHandle,
        datawriter_handle: &InstanceHandle,
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

        if let Some(index) = publisher
            .data_writer_list
            .iter()
            .position(|x| &x.instance_handle == datawriter_handle)
        {
            let data_writer = publisher.data_writer_list.remove(index);
            self.announce_deleted_data_writer(data_writer, now);
            Ok(())
        } else {
            Err(DdsError::AlreadyDeleted)
        }
    }

    #[tracing::instrument(skip(self))]
    pub fn get_default_datawriter_qos(
        &mut self,
        publisher_handle: &InstanceHandle,
    ) -> DdsResult<DataWriterQos> {
        let Some(publisher) = self
            .domain_participant
            .user_defined_publisher_list
            .iter_mut()
            .find(|x| &x.instance_handle == publisher_handle)
        else {
            return Err(DdsError::AlreadyDeleted);
        };
        Ok(publisher.default_datawriter_qos.clone())
    }

    #[tracing::instrument(skip(self))]
    pub fn set_default_datawriter_qos(
        &mut self,
        publisher_handle: &InstanceHandle,
        qos: QosKind<DataWriterQos>,
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
            QosKind::Default => DataWriterQos::default(),
            QosKind::Specific(q) => q,
        };
        qos.is_consistent()?;
        publisher.default_datawriter_qos = qos;
        Ok(())
    }

    #[tracing::instrument(skip(self))]
    pub fn set_publisher_qos(
        &mut self,
        publisher_handle: &InstanceHandle,
        qos: QosKind<PublisherQos>,
    ) -> DdsResult<()> {
        let qos = match qos {
            QosKind::Default => self.domain_participant.default_publisher_qos.clone(),
            QosKind::Specific(q) => q,
        };
        let Some(publisher) = self
            .domain_participant
            .user_defined_publisher_list
            .iter_mut()
            .find(|x| &x.instance_handle == publisher_handle)
        else {
            return Err(DdsError::AlreadyDeleted);
        };

        publisher.qos = qos;
        Ok(())
    }

    #[tracing::instrument(skip(self))]
    pub fn get_publisher_qos(
        &mut self,
        publisher_handle: &InstanceHandle,
    ) -> DdsResult<PublisherQos> {
        let Some(publisher) = self
            .domain_participant
            .user_defined_publisher_list
            .iter_mut()
            .find(|x| &x.instance_handle == publisher_handle)
        else {
            return Err(DdsError::AlreadyDeleted);
        };

        Ok(publisher.qos.clone())
    }

    #[tracing::instrument(skip(self, dcps_listener, runtime))]
    pub fn set_publisher_listener(
        &mut self,
        publisher_handle: &InstanceHandle,
        dcps_listener: Option<DcpsPublisherListener>,
        listener_mask: StatusMask,
        runtime: &impl DdsRuntime,
    ) -> DdsResult<()> {
        let Some(publisher) = self
            .domain_participant
            .user_defined_publisher_list
            .iter_mut()
            .find(|x| &x.instance_handle == publisher_handle)
        else {
            return Err(DdsError::AlreadyDeleted);
        };
        let listener_sender = dcps_listener.map(|l| l.spawn(&runtime.spawner()));
        publisher.listener_sender = listener_sender;
        publisher.listener_mask = listener_mask;
        Ok(())
    }
}
