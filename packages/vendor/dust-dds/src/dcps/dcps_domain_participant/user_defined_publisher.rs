use crate::{
    dcps::{
        channels::mpsc::MpscSender,
        dcps_domain_participant::user_defined_data_writer::UserDefinedDataWriter,
        listeners::domain_participant_listener::ListenerMail, status_mask::StatusMask,
    },
    infrastructure::{
        instance::InstanceHandle,
        qos::{DataWriterQos, PublisherQos},
    },
};
use alloc::vec::Vec;

pub struct PublisherEntity {
    pub qos: PublisherQos,
    pub instance_handle: InstanceHandle,
    pub data_writer_list: Vec<UserDefinedDataWriter>,
    pub enabled: bool,
    pub default_datawriter_qos: DataWriterQos,
    pub listener_sender: Option<MpscSender<ListenerMail>>,
    pub listener_mask: StatusMask,
}

impl PublisherEntity {
    pub const fn new(
        qos: PublisherQos,
        instance_handle: InstanceHandle,
        data_writer_list: Vec<UserDefinedDataWriter>,
        listener_sender: Option<MpscSender<ListenerMail>>,
        listener_mask: StatusMask,
    ) -> Self {
        Self {
            qos,
            instance_handle,
            data_writer_list,
            enabled: false,
            default_datawriter_qos: DataWriterQos::const_default(),
            listener_sender,
            listener_mask,
        }
    }
}
