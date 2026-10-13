use crate::{
    dcps::{
        channels::mpsc::MpscSender,
        dcps_domain_participant::{
            subscriber_entity::SubscriberEntity, user_defined_data_reader::UserDefinedDataReader,
        },
        listeners::domain_participant_listener::ListenerMail,
        status_condition::DcpsStatusCondition,
        status_mask::StatusMask,
    },
    infrastructure::{
        instance::InstanceHandle,
        qos::{DataReaderQos, SubscriberQos},
    },
};
use alloc::vec::Vec;
use core::ops::{Deref, DerefMut};

pub struct UserDefinedSubscriber {
    pub subscriber_entity: SubscriberEntity,
    pub default_data_reader_qos: DataReaderQos,
    pub status_condition: DcpsStatusCondition,
    pub listener_sender: Option<MpscSender<ListenerMail>>,
    pub listener_mask: StatusMask,
    pub data_reader_list: Vec<UserDefinedDataReader>,
}

impl UserDefinedSubscriber {
    pub fn new(
        instance_handle: InstanceHandle,
        qos: SubscriberQos,
        listener_sender: Option<MpscSender<ListenerMail>>,
        listener_mask: StatusMask,
    ) -> Self {
        Self {
            subscriber_entity: SubscriberEntity::new(instance_handle, qos),
            default_data_reader_qos: DataReaderQos::const_default(),
            status_condition: DcpsStatusCondition::default(),
            listener_sender,
            listener_mask,
            data_reader_list: Vec::new(),
        }
    }
}

impl Deref for UserDefinedSubscriber {
    type Target = SubscriberEntity;

    fn deref(&self) -> &Self::Target {
        &self.subscriber_entity
    }
}

impl DerefMut for UserDefinedSubscriber {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.subscriber_entity
    }
}
