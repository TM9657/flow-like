use crate::{
    builtin_topics::PublicationBuiltinTopicData,
    dcps::{
        channels::{mpsc::MpscSender, oneshot::OneshotSender},
        listeners::domain_participant_listener::ListenerMail,
        status_condition::DcpsStatusCondition,
        status_mask::StatusMask,
    },
    infrastructure::{
        error::{DdsError, DdsResult},
        instance::InstanceHandle,
        qos::DataReaderQos,
        qos_policy::QosPolicyId,
        sample_info::{InstanceStateKind, SampleStateKind, ViewStateKind},
        status::{
            QosPolicyCount, RequestedDeadlineMissedStatus, RequestedIncompatibleQosStatus,
            SampleRejectedStatus, SampleRejectedStatusKind, StatusKind, SubscriptionMatchedStatus,
        },
        time::Time,
    },
    rtps::stateful_reader::RtpsStatefulReader,
    transport::types::ChangeKind,
};
use alloc::{sync::Arc, vec::Vec};
use core::ops::{Deref, DerefMut};

use super::data_reader_entity::{AddChangeResult, DataReaderEntity, SampleList};

pub struct UserDefinedDataReader {
    pub reader: DataReaderEntity<RtpsStatefulReader>,
    pub listener_sender: Option<MpscSender<ListenerMail>>,
    pub listener_mask: StatusMask,
    pub status_condition: DcpsStatusCondition,
    pub requested_deadline_missed_status: RequestedDeadlineMissedStatus,
    pub requested_incompatible_qos_status: RequestedIncompatibleQosStatus,
    pub sample_rejected_status: SampleRejectedStatus,
    pub subscription_matched_status: SubscriptionMatchedStatus,
    pub incompatible_writer_list: Vec<InstanceHandle>,
    pub wait_for_historical_data_notification: Vec<OneshotSender<DdsResult<()>>>,
}

impl Deref for UserDefinedDataReader {
    type Target = DataReaderEntity<RtpsStatefulReader>;
    fn deref(&self) -> &Self::Target {
        &self.reader
    }
}

impl DerefMut for UserDefinedDataReader {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.reader
    }
}

impl UserDefinedDataReader {
    pub fn new(
        instance_handle: InstanceHandle,
        qos: DataReaderQos,
        topic_name: Arc<str>,
        listener_sender: Option<MpscSender<ListenerMail>>,
        listener_mask: StatusMask,
        transport_reader: RtpsStatefulReader,
    ) -> Self {
        Self {
            reader: DataReaderEntity::new(instance_handle, qos, topic_name, transport_reader),
            listener_sender,
            listener_mask,
            status_condition: DcpsStatusCondition::default(),
            requested_deadline_missed_status: RequestedDeadlineMissedStatus::const_default(),
            requested_incompatible_qos_status: RequestedIncompatibleQosStatus::const_default(),
            sample_rejected_status: SampleRejectedStatus::const_default(),
            subscription_matched_status: SubscriptionMatchedStatus::const_default(),
            incompatible_writer_list: Vec::new(),
            wait_for_historical_data_notification: Vec::new(),
        }
    }

    pub fn add_matched_publication(
        &mut self,
        publication_builtin_topic_data: PublicationBuiltinTopicData,
    ) {
        match self
            .matched_publication_list
            .iter_mut()
            .find(|x| x.key() == publication_builtin_topic_data.key())
        {
            Some(x) => *x = publication_builtin_topic_data,
            None => self
                .matched_publication_list
                .push(publication_builtin_topic_data),
        }
        self.subscription_matched_status.current_count = self.matched_publication_list.len() as i32;
        self.subscription_matched_status.current_count_change += 1;
        self.subscription_matched_status.total_count += 1;
        self.subscription_matched_status.total_count_change += 1;
    }

    pub fn remove_matched_publication(
        &mut self,
        publication_handle: &InstanceHandle,
        timestamp: Time,
    ) {
        let Some(i) = self
            .matched_publication_list
            .iter()
            .position(|x| &x.key().value == publication_handle.as_ref())
        else {
            return;
        };

        let writer_guid: [u8; 16] = *publication_handle.as_ref();

        let instance_handles: Vec<InstanceHandle> = self
            .instances
            .iter()
            .filter(|x| x.has_registered_writer(&writer_guid))
            .map(|x| *x.handle())
            .collect();

        for instance_handle in instance_handles {
            if let Ok(AddChangeResult::Added) = self.add_reader_change(
                writer_guid.into(),
                Arc::from([]),
                ChangeKind::NotAliveUnregistered,
                *instance_handle.as_ref(),
                None,
                timestamp,
            ) {
                self.status_condition
                    .add_communication_state(StatusKind::DataAvailable);
            }
        }

        for instance in &mut self.instances {
            instance.remove_writer(&writer_guid);
        }

        self.matched_publication_list.remove(i);

        if let Some(i) = self
            .instance_ownership
            .iter()
            .position(|x| x.owner_handle == writer_guid)
        {
            self.instance_ownership.remove(i);
        }

        self.subscription_matched_status.current_count = self.matched_publication_list.len() as i32;
        self.subscription_matched_status.current_count_change -= 1;
        self.status_condition
            .add_communication_state(StatusKind::SubscriptionMatched);
    }

    pub fn add_requested_incompatible_qos(
        &mut self,
        handle: InstanceHandle,
        incompatible_qos_policy_list: Vec<QosPolicyId>,
    ) {
        if !self.incompatible_writer_list.contains(&handle) {
            self.incompatible_writer_list.push(handle);
            self.requested_incompatible_qos_status.total_count += 1;
            self.requested_incompatible_qos_status.total_count_change += 1;
            self.requested_incompatible_qos_status.last_policy_id = incompatible_qos_policy_list[0];
            for incompatible_qos_policy in incompatible_qos_policy_list.into_iter() {
                if let Some(policy_count) = self
                    .requested_incompatible_qos_status
                    .policies
                    .iter_mut()
                    .find(|x| x.policy_id == incompatible_qos_policy)
                {
                    policy_count.count += 1;
                } else {
                    self.requested_incompatible_qos_status
                        .policies
                        .push(QosPolicyCount {
                            policy_id: incompatible_qos_policy,
                            count: 1,
                        })
                }
            }
        }
    }

    pub fn get_requested_incompatible_qos_status(&mut self) -> RequestedIncompatibleQosStatus {
        let status = self.requested_incompatible_qos_status.clone();
        self.requested_incompatible_qos_status.total_count_change = 0;
        status
    }

    pub fn increment_sample_rejected_status(
        &mut self,
        sample_handle: InstanceHandle,
        sample_rejected_status_kind: SampleRejectedStatusKind,
    ) {
        self.sample_rejected_status.last_instance_handle = sample_handle;
        self.sample_rejected_status.last_reason = sample_rejected_status_kind;
        self.sample_rejected_status.total_count += 1;
        self.sample_rejected_status.total_count_change += 1;
    }

    pub fn get_sample_rejected_status(&mut self) -> SampleRejectedStatus {
        let status = self.sample_rejected_status.clone();
        self.sample_rejected_status.total_count_change = 0;
        status
    }

    pub fn get_subscription_matched_status(&mut self) -> SubscriptionMatchedStatus {
        let status = self.subscription_matched_status.clone();
        self.subscription_matched_status.total_count_change = 0;
        self.subscription_matched_status.current_count_change = 0;
        status
    }

    pub fn read(
        &mut self,
        max_samples: i32,
        sample_states: &[SampleStateKind],
        view_states: &[ViewStateKind],
        instance_states: &[InstanceStateKind],
        specific_instance_handle: &Option<InstanceHandle>,
    ) -> DdsResult<SampleList> {
        self.status_condition
            .remove_communication_state(StatusKind::DataAvailable);
        self.reader.read(
            max_samples,
            sample_states,
            view_states,
            instance_states,
            specific_instance_handle,
        )
    }

    pub fn take(
        &mut self,
        max_samples: i32,
        sample_states: &[SampleStateKind],
        view_states: &[ViewStateKind],
        instance_states: &[InstanceStateKind],
        specific_instance_handle: &Option<InstanceHandle>,
    ) -> DdsResult<SampleList> {
        self.status_condition
            .remove_communication_state(StatusKind::DataAvailable);
        self.reader.take(
            max_samples,
            sample_states,
            view_states,
            instance_states,
            specific_instance_handle,
        )
    }

    pub fn take_next_instance(
        &mut self,
        max_samples: i32,
        previous_handle: &Option<InstanceHandle>,
        sample_states: &[SampleStateKind],
        view_states: &[ViewStateKind],
        instance_states: &[InstanceStateKind],
    ) -> DdsResult<SampleList> {
        if !self.enabled {
            return Err(DdsError::NotEnabled);
        }

        match self.next_instance(previous_handle) {
            Some(next_handle) => self.take(
                max_samples,
                sample_states,
                view_states,
                instance_states,
                &Some(next_handle),
            ),
            None => Err(DdsError::NoData),
        }
    }

    pub fn read_next_instance(
        &mut self,
        max_samples: i32,
        previous_handle: &Option<InstanceHandle>,
        sample_states: &[SampleStateKind],
        view_states: &[ViewStateKind],
        instance_states: &[InstanceStateKind],
    ) -> DdsResult<SampleList> {
        if !self.enabled {
            return Err(DdsError::NotEnabled);
        }

        match self.next_instance(previous_handle) {
            Some(next_handle) => self.read(
                max_samples,
                sample_states,
                view_states,
                instance_states,
                &Some(next_handle),
            ),
            None => Err(DdsError::NoData),
        }
    }
}
