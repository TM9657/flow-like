use alloc::string::String;

use crate::{
    dcps::dcps_domain_participant::participant_entity::DcpsDomainParticipant,
    infrastructure::{
        error::{DdsError, DdsResult},
        qos::{QosKind, TopicQos},
        status::{InconsistentTopicStatus, StatusKind},
        time::Time,
    },
    xtypes::dynamic_type::DynamicType,
};

impl DcpsDomainParticipant {
    #[tracing::instrument(skip(self))]
    pub fn get_inconsistent_topic_status(
        &mut self,
        topic_name: String,
    ) -> DdsResult<InconsistentTopicStatus> {
        let Some(topic) = self
            .domain_participant
            .locally_created_topic_list
            .iter_mut()
            .find(|x| x.topic_name.as_ref() == topic_name.as_str())
        else {
            return Err(DdsError::AlreadyDeleted);
        };
        let status = topic.inconsistent_topic_status.clone();
        topic.inconsistent_topic_status.total_count_change = 0;
        topic
            .status_condition
            .remove_communication_state(StatusKind::InconsistentTopic);

        Ok(status)
    }

    #[tracing::instrument(skip(self))]
    pub fn set_topic_qos(
        &mut self,
        topic_name: String,
        topic_qos: QosKind<TopicQos>,
    ) -> DdsResult<()> {
        let qos = match topic_qos {
            QosKind::Default => self.domain_participant.default_topic_qos.clone(),
            QosKind::Specific(q) => q,
        };
        let Some(topic) = self
            .domain_participant
            .locally_created_topic_list
            .iter_mut()
            .find(|x| x.topic_name.as_ref() == topic_name.as_str())
        else {
            return Err(DdsError::AlreadyDeleted);
        };

        qos.is_consistent()?;

        if topic.enabled
            && (topic.qos.durability != qos.durability
                || topic.qos.liveliness != qos.liveliness
                || topic.qos.reliability != qos.reliability
                || topic.qos.destination_order != qos.destination_order
                || topic.qos.history != qos.history
                || topic.qos.resource_limits != qos.resource_limits
                || topic.qos.ownership != qos.ownership)
        {
            return Err(DdsError::ImmutablePolicy);
        }

        topic.qos = qos;
        Ok(())
    }

    #[tracing::instrument(skip(self))]
    pub fn get_topic_qos(&mut self, topic_name: String) -> DdsResult<TopicQos> {
        let Some(topic) = self
            .domain_participant
            .locally_created_topic_list
            .iter_mut()
            .find(|x| x.topic_name.as_ref() == topic_name.as_str())
        else {
            return Err(DdsError::AlreadyDeleted);
        };

        Ok(topic.qos.clone())
    }

    #[tracing::instrument(skip(self))]
    pub fn enable_topic(&mut self, topic_name: String, now: Time) -> DdsResult<()> {
        let Some(topic) = self
            .domain_participant
            .locally_created_topic_list
            .iter_mut()
            .find(|x| x.topic_name.as_ref() == topic_name.as_str())
        else {
            return Err(DdsError::AlreadyDeleted);
        };

        if !topic.enabled {
            topic.enabled = true;
            self.announce_topic(topic_name, now);
        }

        Ok(())
    }

    #[tracing::instrument(skip(self))]
    pub fn get_type_support(&mut self, topic_name: String) -> DdsResult<DynamicType<'static>> {
        let topic = self
            .domain_participant
            .locally_created_topic_list
            .iter()
            .find(|x| x.topic_name.as_ref() == topic_name.as_str())
            .ok_or(DdsError::AlreadyDeleted)?;
        self.domain_participant
            .type_register
            .get_dynamic_type(&topic.type_information.complete.typeid_with_size.type_id)
            .ok_or_else(|| {
                DdsError::PreconditionNotMet(String::from("Type not found in type register"))
            })
    }
}
