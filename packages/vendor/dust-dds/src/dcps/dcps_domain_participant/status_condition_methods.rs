use crate::{
    dcps::{
        channels::notification::NotificationSender,
        dcps_participant_factory::DcpsParticipantFactory, status_condition::StatusConditionEntity,
        status_mask::StatusMask,
    },
    infrastructure::error::{DdsError, DdsResult},
    runtime::DdsRuntime,
};

impl<R: DdsRuntime> DcpsParticipantFactory<R> {
    pub fn get_status_condition_enabled_statuses(
        &mut self,
        entity: StatusConditionEntity,
    ) -> DdsResult<StatusMask> {
        match entity {
            StatusConditionEntity::Subscriber {
                participant_handle,
                subscriber_handle,
            } => {
                let dp = self.find_participant(&participant_handle)?;
                let s = dp
                    .domain_participant
                    .user_defined_subscriber_list
                    .iter_mut()
                    .find(|s| s.instance_handle == subscriber_handle)
                    .ok_or(DdsError::AlreadyDeleted)?;
                return Ok(s.status_condition.get_enabled_statuses());
            }
            StatusConditionEntity::Topic {
                participant_handle,
                topic_handle,
            } => {
                let dp = self.find_participant(&participant_handle)?;
                for t in dp.domain_participant.locally_created_topic_list.iter_mut() {
                    if t.instance_handle == topic_handle {
                        return Ok(t.status_condition.get_enabled_statuses());
                    }
                }
                return Err(DdsError::AlreadyDeleted);
            }
            StatusConditionEntity::DataWriter {
                participant_handle,
                publisher_handle,
                writer_handle,
            } => {
                let dp = self.find_participant(&participant_handle)?;
                for p in dp.domain_participant.user_defined_publisher_list.iter_mut() {
                    if p.instance_handle == publisher_handle {
                        for dw in p.data_writer_list.iter_mut() {
                            if dw.instance_handle == writer_handle {
                                return Ok(dw.status_condition.get_enabled_statuses());
                            }
                        }
                    }
                }
            }
            StatusConditionEntity::DataReader {
                participant_handle,
                subscriber_handle,
                reader_handle,
            } => {
                let dp = self.find_participant(&participant_handle)?;
                let status_condition = if let Some(s) = dp
                    .domain_participant
                    .user_defined_subscriber_list
                    .iter_mut()
                    .find(|s| s.instance_handle == subscriber_handle)
                {
                    s.data_reader_list
                        .iter_mut()
                        .find(|dr| dr.instance_handle == reader_handle)
                        .map(|dr| &mut dr.status_condition)
                } else {
                    None
                };
                if let Some(sc) = status_condition {
                    return Ok(sc.get_enabled_statuses());
                }
            }
        }

        Err(DdsError::AlreadyDeleted)
    }

    pub fn set_status_condition_enabled_statuses(
        &mut self,
        entity: StatusConditionEntity,
        status_mask: StatusMask,
    ) -> DdsResult<()> {
        match entity {
            StatusConditionEntity::Subscriber {
                participant_handle,
                subscriber_handle,
            } => {
                let dp = self.find_participant(&participant_handle)?;
                let s = dp
                    .domain_participant
                    .user_defined_subscriber_list
                    .iter_mut()
                    .find(|s| s.instance_handle == subscriber_handle)
                    .ok_or(DdsError::AlreadyDeleted)?;
                s.status_condition.set_enabled_statuses(status_mask);
                return Ok(());
            }
            StatusConditionEntity::Topic {
                participant_handle,
                topic_handle,
            } => {
                let dp = self.find_participant(&participant_handle)?;
                for t in dp.domain_participant.locally_created_topic_list.iter_mut() {
                    if t.instance_handle == topic_handle {
                        t.status_condition.set_enabled_statuses(status_mask);
                        return Ok(());
                    }
                }
                return Err(DdsError::AlreadyDeleted);
            }
            StatusConditionEntity::DataWriter {
                participant_handle,
                publisher_handle,
                writer_handle,
            } => {
                let dp = self.find_participant(&participant_handle)?;
                for p in dp.domain_participant.user_defined_publisher_list.iter_mut() {
                    if p.instance_handle == publisher_handle {
                        for dw in p.data_writer_list.iter_mut() {
                            if dw.instance_handle == writer_handle {
                                dw.status_condition.set_enabled_statuses(status_mask);
                                return Ok(());
                            }
                        }
                    }
                }
            }
            StatusConditionEntity::DataReader {
                participant_handle,
                subscriber_handle,
                reader_handle,
            } => {
                let dp = self.find_participant(&participant_handle)?;
                let status_condition = if let Some(s) = dp
                    .domain_participant
                    .user_defined_subscriber_list
                    .iter_mut()
                    .find(|s| s.instance_handle == subscriber_handle)
                {
                    s.data_reader_list
                        .iter_mut()
                        .find(|dr| dr.instance_handle == reader_handle)
                        .map(|dr| &mut dr.status_condition)
                } else {
                    None
                };
                if let Some(sc) = status_condition {
                    sc.set_enabled_statuses(status_mask);
                    return Ok(());
                }
            }
        }

        Err(DdsError::AlreadyDeleted)
    }

    pub fn get_status_condition_trigger_value(
        &mut self,
        entity: StatusConditionEntity,
    ) -> DdsResult<bool> {
        match entity {
            StatusConditionEntity::Subscriber {
                participant_handle,
                subscriber_handle,
            } => {
                let dp = self.find_participant(&participant_handle)?;
                let s = dp
                    .domain_participant
                    .user_defined_subscriber_list
                    .iter_mut()
                    .find(|s| s.instance_handle == subscriber_handle)
                    .ok_or(DdsError::AlreadyDeleted)?;
                return Ok(s.status_condition.get_trigger_value());
            }
            StatusConditionEntity::Topic {
                participant_handle,
                topic_handle,
            } => {
                let dp = self.find_participant(&participant_handle)?;
                for t in dp.domain_participant.locally_created_topic_list.iter_mut() {
                    if t.instance_handle == topic_handle {
                        return Ok(t.status_condition.get_trigger_value());
                    }
                }
                return Err(DdsError::AlreadyDeleted);
            }
            StatusConditionEntity::DataWriter {
                participant_handle,
                publisher_handle,
                writer_handle,
            } => {
                let dp = self.find_participant(&participant_handle)?;
                for p in dp.domain_participant.user_defined_publisher_list.iter_mut() {
                    if p.instance_handle == publisher_handle {
                        for dw in p.data_writer_list.iter_mut() {
                            if dw.instance_handle == writer_handle {
                                return Ok(dw.status_condition.get_trigger_value());
                            }
                        }
                    }
                }
            }
            StatusConditionEntity::DataReader {
                participant_handle,
                subscriber_handle,
                reader_handle,
            } => {
                let dp = self.find_participant(&participant_handle)?;
                let status_condition = if let Some(s) = dp
                    .domain_participant
                    .user_defined_subscriber_list
                    .iter_mut()
                    .find(|s| s.instance_handle == subscriber_handle)
                {
                    s.data_reader_list
                        .iter_mut()
                        .find(|dr| dr.instance_handle == reader_handle)
                        .map(|dr| &mut dr.status_condition)
                } else {
                    None
                };
                if let Some(sc) = status_condition {
                    return Ok(sc.get_trigger_value());
                }
            }
        }

        Err(DdsError::AlreadyDeleted)
    }

    pub fn register_notification(
        &mut self,
        entity: StatusConditionEntity,
        notification_sender: NotificationSender,
    ) -> DdsResult<()> {
        match entity {
            StatusConditionEntity::Subscriber {
                participant_handle,
                subscriber_handle,
            } => {
                let dp = self.find_participant(&participant_handle)?;
                let s = dp
                    .domain_participant
                    .user_defined_subscriber_list
                    .iter_mut()
                    .find(|s| s.instance_handle == subscriber_handle)
                    .ok_or(DdsError::AlreadyDeleted)?;
                s.status_condition
                    .register_notification(notification_sender);
                return Ok(());
            }
            StatusConditionEntity::Topic {
                participant_handle,
                topic_handle,
            } => {
                let dp = self.find_participant(&participant_handle)?;
                for t in dp.domain_participant.locally_created_topic_list.iter_mut() {
                    if t.instance_handle == topic_handle {
                        t.status_condition
                            .register_notification(notification_sender);
                        return Ok(());
                    }
                }
                return Err(DdsError::AlreadyDeleted);
            }
            StatusConditionEntity::DataWriter {
                participant_handle,
                publisher_handle,
                writer_handle,
            } => {
                let dp = self.find_participant(&participant_handle)?;
                for p in dp.domain_participant.user_defined_publisher_list.iter_mut() {
                    if p.instance_handle == publisher_handle {
                        for dw in p.data_writer_list.iter_mut() {
                            if dw.instance_handle == writer_handle {
                                dw.status_condition
                                    .register_notification(notification_sender);
                                return Ok(());
                            }
                        }
                    }
                }
            }
            StatusConditionEntity::DataReader {
                participant_handle,
                subscriber_handle,
                reader_handle,
            } => {
                let dp = self.find_participant(&participant_handle)?;
                let status_condition = if let Some(s) = dp
                    .domain_participant
                    .user_defined_subscriber_list
                    .iter_mut()
                    .find(|s| s.instance_handle == subscriber_handle)
                {
                    s.data_reader_list
                        .iter_mut()
                        .find(|dr| dr.instance_handle == reader_handle)
                        .map(|dr| &mut dr.status_condition)
                } else {
                    None
                };
                if let Some(sc) = status_condition {
                    sc.register_notification(notification_sender);
                    return Ok(());
                }
            }
        }

        Err(DdsError::AlreadyDeleted)
    }
}
