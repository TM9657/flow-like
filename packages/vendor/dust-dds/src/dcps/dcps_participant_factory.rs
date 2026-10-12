use crate::{
    dcps::{
        dcps_domain_participant::participant_entity::DcpsDomainParticipant,
        listeners::domain_participant_listener::DcpsDomainParticipantListener,
        status_mask::StatusMask,
    },
    dds_async::domain_participant_factory::DcpsSender,
    infrastructure::{
        domain::DomainId,
        error::{DdsError, DdsResult},
        instance::InstanceHandle,
        qos::{DomainParticipantFactoryQos, DomainParticipantQos, QosKind},
        time::{Duration, Time},
    },
    runtime::DdsRuntime,
    transport::{interface::RtpsTransportParticipant, types::GuidPrefix},
};
use alloc::{string::String, vec::Vec};

pub struct DcpsParticipantFactory<R: DdsRuntime> {
    pub domain_participant_list: Vec<DcpsDomainParticipant>,
    pub qos: DomainParticipantFactoryQos,
    pub default_participant_qos: DomainParticipantQos,
    pub runtime: R,
    pub dcps_sender: DcpsSender,
}

impl<R: DdsRuntime> DcpsParticipantFactory<R> {
    pub fn new(runtime: R, dcps_sender: DcpsSender) -> Self {
        Self {
            domain_participant_list: Default::default(),
            qos: Default::default(),
            default_participant_qos: Default::default(),
            runtime,
            dcps_sender,
        }
    }

    #[allow(clippy::too_many_arguments)]
    pub fn create_participant(
        &mut self,
        guid_prefix: GuidPrefix,
        domain_id: DomainId,
        qos: QosKind<DomainParticipantQos>,
        dcps_listener: Option<DcpsDomainParticipantListener>,
        listener_mask: StatusMask,
        transport_participant: RtpsTransportParticipant,
        domain_tag: String,
        participant_announcement_interval: core::time::Duration,
        enable_type_information: bool,
        now: Time,
    ) -> DdsResult<InstanceHandle> {
        let domain_participant_qos = match qos {
            QosKind::Default => self.default_participant_qos.clone(),
            QosKind::Specific(q) => q,
        };

        let listener_sender = dcps_listener.map(|l| l.spawn(&self.runtime.spawner()));

        let mut dcps_participant = DcpsDomainParticipant::new(
            domain_id,
            domain_tag,
            guid_prefix,
            domain_participant_qos,
            listener_sender,
            listener_mask,
            transport_participant,
            self.dcps_sender.clone(),
            participant_announcement_interval,
            enable_type_information,
        );
        let participant_handle = *dcps_participant.get_instance_handle();

        if self.qos.entity_factory.autoenable_created_entities {
            dcps_participant.enable_domain_participant(now)?;
        }

        self.domain_participant_list.push(dcps_participant);

        Ok(participant_handle)
    }

    pub fn delete_participant(
        &mut self,
        participant_handle: &InstanceHandle,
        now: Time,
    ) -> DdsResult<()> {
        let index = self
            .domain_participant_list
            .iter()
            .position(|h| h.get_instance_handle() == participant_handle)
            .ok_or(DdsError::AlreadyDeleted)?;
        if !self.domain_participant_list[index].is_participant_empty() {
            return Err(DdsError::PreconditionNotMet(String::from(
                "Domain participant still contains other entities",
            )));
        }
        let mut participant = self.domain_participant_list.remove(index);
        participant.announce_deleted_participant(now);
        Ok(())
    }

    pub fn find_participant(
        &mut self,
        participant_handle: &InstanceHandle,
    ) -> DdsResult<&mut DcpsDomainParticipant> {
        self.domain_participant_list
            .iter_mut()
            .find(|x| x.get_instance_handle() == participant_handle)
            .ok_or(DdsError::AlreadyDeleted)
    }

    pub fn set_default_participant_qos(
        &mut self,
        qos: QosKind<DomainParticipantQos>,
    ) -> DdsResult<()> {
        let qos = match qos {
            QosKind::Default => DomainParticipantQos::default(),
            QosKind::Specific(q) => q,
        };

        self.default_participant_qos = qos;

        Ok(())
    }

    pub fn get_default_participant_qos(&mut self) -> DomainParticipantQos {
        self.default_participant_qos.clone()
    }

    pub fn set_qos(&mut self, qos: QosKind<DomainParticipantFactoryQos>) -> DdsResult<()> {
        let qos = match qos {
            QosKind::Default => DomainParticipantFactoryQos::default(),
            QosKind::Specific(q) => q,
        };

        self.qos = qos;
        Ok(())
    }

    pub fn get_qos(&mut self) -> DomainParticipantFactoryQos {
        self.qos.clone()
    }

    pub fn time_until_next_event(&self, now: Time) -> Option<Duration> {
        self.domain_participant_list
            .iter()
            .filter_map(|x| x.time_until_next_event(now))
            .min()
    }
}
