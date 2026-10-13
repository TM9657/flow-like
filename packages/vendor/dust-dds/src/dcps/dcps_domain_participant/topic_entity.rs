use super::builtin_constants::{TYPE_LOOKUP_REPLY_TOPIC_NAME, TYPE_LOOKUP_REQUEST_TOPIC_NAME};
use crate::{
    builtin_topics::{
        DCPS_PARTICIPANT, DCPS_PUBLICATION, DCPS_SUBSCRIPTION, DCPS_TOPIC,
        ParticipantBuiltinTopicData, PublicationBuiltinTopicData, SubscriptionBuiltinTopicData,
        TopicBuiltinTopicData,
    },
    dcps::{
        channels::mpsc::MpscSender,
        data_representation_builtin_endpoints::type_lookup::{TypeLookupReply, TypeLookupRequest},
        dcps_domain_participant::type_register::TypeRegister,
        listeners::domain_participant_listener::ListenerMail,
        status_condition::DcpsStatusCondition,
        status_mask::StatusMask,
    },
    infrastructure::{instance::InstanceHandle, qos::TopicQos, status::InconsistentTopicStatus},
    xtypes::{dynamic_type::DynamicType, type_object::TypeInformation, type_support::Type},
};
use alloc::{string::String, sync::Arc, vec::Vec};

pub struct ContentFilteredTopicEntity {
    pub topic_name: Arc<str>,
    pub related_topic_name: Arc<str>,
    pub filter_expression: String,
    pub expression_parameters: Vec<String>,
}

impl ContentFilteredTopicEntity {
    pub fn new(
        name: Arc<str>,
        related_topic_name: Arc<str>,
        filter_expression: String,
        expression_parameters: Vec<String>,
    ) -> Self {
        Self {
            topic_name: name,
            related_topic_name,
            filter_expression,
            expression_parameters,
        }
    }
}

pub struct TopicEntity {
    pub qos: TopicQos,
    pub type_name: Arc<str>,
    pub topic_name: Arc<str>,
    pub instance_handle: InstanceHandle,
    pub enabled: bool,
    pub inconsistent_topic_status: InconsistentTopicStatus,
    pub status_condition: DcpsStatusCondition,
    pub listener_sender: Option<MpscSender<ListenerMail>>,
    pub listener_mask: StatusMask,
    pub type_information: TypeInformation,
}

impl TopicEntity {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        qos: TopicQos,
        type_name: Arc<str>,
        topic_name: Arc<str>,
        instance_handle: InstanceHandle,
        status_condition: DcpsStatusCondition,
        listener_sender: Option<MpscSender<ListenerMail>>,
        listener_mask: StatusMask,
        type_information: TypeInformation,
    ) -> Self {
        Self {
            qos,
            type_name,
            topic_name,
            instance_handle,
            enabled: false,
            inconsistent_topic_status: InconsistentTopicStatus::const_default(),
            status_condition,
            listener_sender,
            listener_mask,
            type_information,
        }
    }
}

pub fn get_topic_type_support<'a>(
    topic_name: &str,
    content_filtered_topic_list: &[ContentFilteredTopicEntity],
    locally_created_topic_list: &'a [TopicEntity],
    type_register: &'a TypeRegister,
) -> Option<DynamicType<'static>> {
    let resolved_topic_name = if let Some(cf_topic) = content_filtered_topic_list
        .iter()
        .find(|t| t.topic_name.as_ref() == topic_name)
    {
        cf_topic.related_topic_name.as_ref()
    } else {
        topic_name
    };

    match resolved_topic_name {
        DCPS_PARTICIPANT => Some(ParticipantBuiltinTopicData::TYPE),
        DCPS_TOPIC => Some(TopicBuiltinTopicData::TYPE),
        DCPS_PUBLICATION => Some(PublicationBuiltinTopicData::TYPE),
        DCPS_SUBSCRIPTION => Some(SubscriptionBuiltinTopicData::TYPE),
        TYPE_LOOKUP_REQUEST_TOPIC_NAME => Some(TypeLookupRequest::TYPE),
        TYPE_LOOKUP_REPLY_TOPIC_NAME => Some(TypeLookupReply::TYPE),
        _ => locally_created_topic_list
            .iter()
            .find(|t| t.topic_name.as_ref() == resolved_topic_name)
            .and_then(|t| {
                type_register
                    .get_dynamic_type(&t.type_information.complete.typeid_with_size.type_id)
            }),
    }
}
