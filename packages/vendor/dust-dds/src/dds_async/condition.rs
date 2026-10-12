use crate::{
    dcps::{
        channels::notification::NotificationSender,
        dcps_mail::{DcpsMail, StatusConditionMail},
        status_condition::StatusConditionEntity,
    },
    dds_async::domain_participant_factory::DcpsSender,
    infrastructure::{error::DdsResult, status::StatusKind},
};

/// Async version of [`StatusCondition`](crate::infrastructure::condition::StatusCondition).
#[derive(Clone)]
pub struct StatusConditionAsync {
    dcps_sender: DcpsSender,
    entity: StatusConditionEntity,
}

impl StatusConditionAsync {
    pub(crate) fn new(dcps_sender: DcpsSender, entity: StatusConditionEntity) -> Self {
        Self {
            dcps_sender,
            entity,
        }
    }

    pub(crate) async fn register_notification(
        &self,
        notification_sender: NotificationSender,
    ) -> DdsResult<()> {
        self.dcps_sender
            .call(DcpsMail::StatusCondition(
                StatusConditionMail::RegisterNotification {
                    entity: self.entity.clone(),
                    notification_sender,
                },
            ))
            .await?
            .expect_ok()
    }
}

impl StatusConditionAsync {
    /// Async version of [`get_enabled_statuses`](crate::infrastructure::condition::StatusCondition::get_enabled_statuses).
    #[tracing::instrument(skip(self))]
    pub async fn get_enabled_statuses(&self) -> DdsResult<impl IntoIterator<Item = StatusKind>> {
        self.dcps_sender
            .call(DcpsMail::StatusCondition(
                StatusConditionMail::GetStatusConditionEnabledStatuses {
                    entity: self.entity.clone(),
                },
            ))
            .await?
            .expect_status_mask()
    }

    /// Async version of [`set_enabled_statuses`](crate::infrastructure::condition::StatusCondition::set_enabled_statuses).
    #[tracing::instrument(skip(self))]
    pub async fn set_enabled_statuses(&self, mask: &[StatusKind]) -> DdsResult<()> {
        self.dcps_sender
            .call(DcpsMail::StatusCondition(
                StatusConditionMail::SetStatusConditionEnabledStatuses {
                    entity: self.entity.clone(),
                    status_mask: mask.iter().collect(),
                },
            ))
            .await?
            .expect_ok()
    }

    /// Async version of [`get_entity`](crate::infrastructure::condition::StatusCondition::get_entity).
    #[tracing::instrument(skip(self))]
    pub async fn get_entity(&self) {
        todo!()
    }
}

impl StatusConditionAsync {
    /// Async version of [`get_trigger_value`](crate::infrastructure::condition::StatusCondition::get_trigger_value).
    #[tracing::instrument(skip(self))]
    pub async fn get_trigger_value(&self) -> DdsResult<bool> {
        self.dcps_sender
            .call(DcpsMail::StatusCondition(
                StatusConditionMail::GetStatusConditionTriggerValue {
                    entity: self.entity.clone(),
                },
            ))
            .await?
            .expect_trigger_value()
    }
}
