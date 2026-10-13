use crate::{
    dcps::dcps_mail::WireMail, dds_async::domain_participant_factory::WireSender,
    infrastructure::instance::InstanceHandle, transport::types::Locator,
};
use alloc::{boxed::Box, vec::Vec};

pub trait WriteMessage {
    fn write_buffer_mut(&mut self) -> &mut [u8];
    fn write_message(&mut self, len: usize, locators: &[Locator]);
}

#[derive(Clone)]
pub struct TransportDataReceiver {
    participant_handle: InstanceHandle,
    wire_sender: WireSender,
}
impl TransportDataReceiver {
    pub(crate) fn new(participant_handle: InstanceHandle, wire_sender: WireSender) -> Self {
        Self {
            participant_handle,
            wire_sender,
        }
    }

    pub async fn receive_message(&self, data_message: Vec<u8>) {
        self.wire_sender
            .send(WireMail {
                participant_handle: self.participant_handle,
                data_message,
            })
            .await;
    }
}

pub struct RtpsTransportParticipant {
    pub message_writer: Box<dyn WriteMessage + Send + Sync>,
    pub default_unicast_locator_list: Vec<Locator>,
    pub metatraffic_unicast_locator_list: Vec<Locator>,
    pub metatraffic_multicast_locator_list: Vec<Locator>,
    pub default_multicast_locator_list: Vec<Locator>,
    pub fragment_size: usize,
}
pub trait TransportParticipantFactory: Send + 'static {
    fn create_participant(
        &self,
        domain_id: i32,
        data_receiver: TransportDataReceiver,
    ) -> RtpsTransportParticipant;
}
