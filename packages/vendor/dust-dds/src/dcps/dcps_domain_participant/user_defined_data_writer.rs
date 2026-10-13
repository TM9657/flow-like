use crate::{
    builtin_topics::SubscriptionBuiltinTopicData,
    dcps::{
        channels::{mpsc::MpscSender, oneshot::OneshotSender},
        dcps_domain_participant::data_writer_entity::{
            DataWriterEntity, IncompatibleSubscriptions,
        },
        listeners::domain_participant_listener::ListenerMail,
        status_condition::DcpsStatusCondition,
        status_mask::StatusMask,
        xtypes_glue::key_and_instance_handle::KeyHolderType,
    },
    infrastructure::{
        error::DdsResult,
        instance::InstanceHandle,
        qos::DataWriterQos,
        status::{OfferedDeadlineMissedStatus, PublicationMatchedStatus, StatusKind},
        time::Time,
    },
    rtps::stateful_writer::RtpsStatefulWriter,
    xtypes::dynamic_type::DynamicData,
};
use alloc::{sync::Arc, vec::Vec};
use core::ops::{Deref, DerefMut};

pub struct PendingWriteSample {
    pub dynamic_data: DynamicData<'static>,
    pub timestamp: Time,
    pub reply_sender: OneshotSender<DdsResult<()>>,
    pub expiration_time: Option<Time>,
}

pub struct UserDefinedDataWriter {
    pub writer: DataWriterEntity<RtpsStatefulWriter>,
    pub listener_sender: Option<MpscSender<ListenerMail>>,
    pub listener_mask: StatusMask,
    pub status_condition: DcpsStatusCondition,
    pub matched_subscription_list: Vec<SubscriptionBuiltinTopicData>,
    pub publication_matched_status: PublicationMatchedStatus,
    pub incompatible_subscriptions: IncompatibleSubscriptions,
    pub offered_deadline_missed_status: OfferedDeadlineMissedStatus,
    /// Member used for notifying reliable writers which are waiting to send
    /// their samples without losing data
    pub acknowledgement_notification: Option<OneshotSender<()>>,
    /// Member used to notify the external user which called the
    /// wait_for_acknowledgments method
    pub wait_for_acknowledgments_notification: Vec<OneshotSender<DdsResult<()>>>,
    pub pending_write_sample: Option<PendingWriteSample>,
}

impl Deref for UserDefinedDataWriter {
    type Target = DataWriterEntity<RtpsStatefulWriter>;
    fn deref(&self) -> &Self::Target {
        &self.writer
    }
}

impl DerefMut for UserDefinedDataWriter {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.writer
    }
}

impl UserDefinedDataWriter {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        instance_handle: InstanceHandle,
        transport_writer: RtpsStatefulWriter,
        topic_name: Arc<str>,
        listener_sender: Option<MpscSender<ListenerMail>>,
        listener_mask: StatusMask,
        qos: DataWriterQos,
        key_holder_type: KeyHolderType,
    ) -> Self {
        Self {
            writer: DataWriterEntity::new(
                instance_handle,
                transport_writer,
                topic_name,
                qos,
                key_holder_type,
            ),
            listener_sender,
            listener_mask,
            status_condition: DcpsStatusCondition::default(),
            matched_subscription_list: Vec::new(),
            publication_matched_status: PublicationMatchedStatus::const_default(),
            incompatible_subscriptions: IncompatibleSubscriptions::default(),
            offered_deadline_missed_status: OfferedDeadlineMissedStatus::const_default(),
            acknowledgement_notification: None,
            wait_for_acknowledgments_notification: Vec::new(),
            pending_write_sample: None,
        }
    }

    pub fn remove_matched_subscription(&mut self, subscription_handle: &InstanceHandle) {
        let Some(i) = self
            .matched_subscription_list
            .iter()
            .position(|x| &x.key().value == subscription_handle.as_ref())
        else {
            return;
        };
        self.matched_subscription_list.remove(i);

        self.publication_matched_status.current_count = self.matched_subscription_list.len() as i32;
        self.publication_matched_status.current_count_change -= 1;
    }

    pub fn get_offered_deadline_missed_status(&mut self) -> OfferedDeadlineMissedStatus {
        let status = self.offered_deadline_missed_status.clone();
        self.offered_deadline_missed_status.total_count_change = 0;
        self.status_condition
            .remove_communication_state(StatusKind::OfferedDeadlineMissed);

        status
    }
}
