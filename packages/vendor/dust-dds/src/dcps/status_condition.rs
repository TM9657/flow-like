use crate::{
    dcps::{channels::notification::NotificationSender, status_mask::StatusMask},
    infrastructure::{instance::InstanceHandle, status::StatusKind},
};
use alloc::vec::Vec;

#[derive(Clone)]
pub enum StatusConditionEntity {
    Subscriber {
        participant_handle: InstanceHandle,
        subscriber_handle: InstanceHandle,
    },
    Topic {
        participant_handle: InstanceHandle,
        topic_handle: InstanceHandle,
    },
    DataWriter {
        participant_handle: InstanceHandle,
        publisher_handle: InstanceHandle,
        writer_handle: InstanceHandle,
    },
    DataReader {
        participant_handle: InstanceHandle,
        subscriber_handle: InstanceHandle,
        reader_handle: InstanceHandle,
    },
}

pub struct DcpsStatusCondition {
    enabled_statuses: StatusMask,
    status_changes: Vec<StatusKind>,
    registered_notifications: Vec<NotificationSender>,
}

impl Default for DcpsStatusCondition {
    fn default() -> Self {
        Self {
            enabled_statuses: [
                StatusKind::InconsistentTopic,
                StatusKind::OfferedDeadlineMissed,
                StatusKind::RequestedDeadlineMissed,
                StatusKind::OfferedIncompatibleQos,
                StatusKind::RequestedIncompatibleQos,
                StatusKind::SampleLost,
                StatusKind::SampleRejected,
                StatusKind::DataOnReaders,
                StatusKind::DataAvailable,
                StatusKind::LivelinessLost,
                StatusKind::LivelinessChanged,
                StatusKind::PublicationMatched,
                StatusKind::SubscriptionMatched,
            ]
            .iter()
            .collect(),
            status_changes: Vec::new(),
            registered_notifications: Vec::new(),
        }
    }
}

impl DcpsStatusCondition {
    pub fn add_communication_state(&mut self, state: StatusKind) {
        self.status_changes.push(state);
        if self.get_trigger_value() {
            for w in self.registered_notifications.drain(..) {
                // Do not care if there is no channel waiting for response
                w.notify();
            }
        }
    }

    pub fn remove_communication_state(&mut self, state: StatusKind) {
        self.status_changes.retain(|x| x != &state);
    }

    pub fn get_enabled_statuses(&self) -> StatusMask {
        self.enabled_statuses
    }

    pub fn set_enabled_statuses(&mut self, mask: StatusMask) {
        self.enabled_statuses = mask;
    }

    pub fn get_trigger_value(&self) -> bool {
        for status in &self.status_changes {
            if self.enabled_statuses.is_enabled(status) {
                return true;
            }
        }
        false
    }

    pub fn register_notification(&mut self, notification_sender: NotificationSender) {
        if self.get_trigger_value() {
            notification_sender.notify();
        } else {
            self.registered_notifications.push(notification_sender);
        }
    }
}
