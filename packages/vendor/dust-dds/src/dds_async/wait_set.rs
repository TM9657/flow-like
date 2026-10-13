use super::condition::StatusConditionAsync;
use crate::{
    dcps::channels::notification::notification,
    infrastructure::error::{DdsError, DdsResult},
};
use alloc::{string::String, vec::Vec};

/// Async version of [`Condition`](crate::infrastructure::wait_set::Condition).
#[derive(Clone)]
pub enum ConditionAsync {
    /// Status condition variant
    StatusCondition(StatusConditionAsync),
}

impl ConditionAsync {
    /// Async version of [`get_trigger_value`](crate::infrastructure::wait_set::Condition::get_trigger_value).
    #[tracing::instrument(skip(self))]
    pub async fn get_trigger_value(&self) -> DdsResult<bool> {
        match self {
            ConditionAsync::StatusCondition(c) => c.get_trigger_value().await,
        }
    }
}

/// Async version of [`WaitSet`](crate::infrastructure::wait_set::WaitSet).
#[derive(Default, Clone)]
pub struct WaitSetAsync {
    conditions: Vec<ConditionAsync>,
}

impl WaitSetAsync {
    /// Create a new [`WaitSetAsync`]
    #[tracing::instrument]
    pub fn new() -> Self {
        Self::default()
    }

    /// Async version of [`wait`](crate::infrastructure::wait_set::WaitSet::wait).
    #[tracing::instrument(skip(self))]
    pub async fn wait(&self) -> DdsResult<Vec<ConditionAsync>> {
        if self.conditions.is_empty() {
            return Err(DdsError::PreconditionNotMet(String::from(
                "WaitSet has no attached conditions",
            )));
        };

        let mut trigger_conditions = Vec::new();
        // Check if conditions are already triggered
        for condition in &self.conditions {
            if condition.get_trigger_value().await? {
                trigger_conditions.push(condition.clone());
            }
        }

        if !trigger_conditions.is_empty() {
            return Ok(trigger_conditions);
        }

        // No status condition is yet triggered so now we have to wait for at least one status condition to trigger
        let (notification_sender, notification_receiver) = notification();

        for condition in &self.conditions {
            match condition {
                ConditionAsync::StatusCondition(status_condition_async) => {
                    status_condition_async
                        .register_notification(notification_sender.clone())
                        .await?;
                }
            }
        }

        // Wait until a condition sends a notification and then collect the active ones
        notification_receiver.await?;
        for condition in &self.conditions {
            if condition.get_trigger_value().await? {
                trigger_conditions.push(condition.clone());
            }
        }

        Ok(trigger_conditions)
    }

    /// Async version of [`attach_condition`](crate::infrastructure::wait_set::WaitSet::attach_condition).
    #[tracing::instrument(skip(self, cond))]
    pub async fn attach_condition(&mut self, cond: ConditionAsync) -> DdsResult<()> {
        self.conditions.push(cond);
        Ok(())
    }

    /// Async version of [`detach_condition`](crate::infrastructure::wait_set::WaitSet::detach_condition).
    #[tracing::instrument(skip(self, _cond))]
    pub async fn detach_condition(&self, _cond: ConditionAsync) -> DdsResult<()> {
        todo!()
    }

    /// Async version of [`get_conditions`](crate::infrastructure::wait_set::WaitSet::get_conditions).
    #[tracing::instrument(skip(self))]
    pub async fn get_conditions(&self) -> DdsResult<Vec<ConditionAsync>> {
        Ok(self.conditions.clone())
    }
}
