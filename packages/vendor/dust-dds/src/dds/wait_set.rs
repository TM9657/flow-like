use super::condition::StatusCondition;
use crate::{
    dds_async::wait_set::{ConditionAsync, WaitSetAsync},
    infrastructure::{error::DdsResult, time::Duration},
    std_runtime::executor::{block_on, block_timeout},
};
use alloc::vec::Vec;

/// Enumeration of the different Condition objects that can be associated with a [`WaitSet`].
#[derive(Clone)]
pub enum Condition {
    /// Status condition variant
    StatusCondition(StatusCondition),
}
impl Condition {
    #[tracing::instrument(skip(self))]
    /// This operation retrieves the trigger_value of the Condition.
    pub fn get_trigger_value(&self) -> DdsResult<bool> {
        match self {
            Condition::StatusCondition(c) => c.get_trigger_value(),
        }
    }
}

/// A [`WaitSet`] allows an application to wait until one or more of the attached [`Condition`] objects has a `trigger_value` of
/// [`true`] or else until the timeout expires. It is created by calling the [`WaitSet::new`] operation and is not necessarily
/// associated with a single [`DomainParticipant`](crate::domain::domain_participant::DomainParticipant) and could be used to
/// wait on [`Condition`] objects associated with different [`DomainParticipant`](crate::domain::domain_participant::DomainParticipant) objects.
#[derive(Default, Clone)]
pub struct WaitSet {
    waitset_async: WaitSetAsync,
}

impl WaitSet {
    /// Create a new [`WaitSet`]
    #[tracing::instrument]
    pub fn new() -> Self {
        Self::default()
    }

    /// This operation allows an application thread to wait for the occurrence of certain conditions. If none of the conditions attached
    /// to the [`WaitSet`] have a `trigger_value` of [`true`], the wait operation will block suspending the calling thread.
    /// The operation returns the list of all the attached conditions that have a `trigger_value` of [`true`] (i.e., the conditions
    /// that unblocked the wait).
    /// This operation takes a `timeout` argument that specifies the maximum duration for the wait. It this duration is exceeded and
    /// none of the attached [`Condition`] objects is [`true`], wait will return with [`DdsError::Timeout`](crate::infrastructure::error::DdsError::Timeout).
    /// It is not allowed for more than one application thread to be waiting on the same [`WaitSet`]. If the wait operation is invoked on a
    /// [`WaitSet`] that already has a thread blocking on it, the operation will return immediately with the value [`DdsError::PreconditionNotMet`](crate::infrastructure::error::DdsError::PreconditionNotMet).
    #[tracing::instrument(skip(self))]
    pub fn wait(&self, timeout: Duration) -> DdsResult<Vec<Condition>> {
        Ok(block_timeout(timeout.into(), self.waitset_async.wait())??
            .into_iter()
            .map(|c| match c {
                ConditionAsync::StatusCondition(sc) => {
                    Condition::StatusCondition(StatusCondition::new(sc))
                }
            })
            .collect())
    }

    /// Attaches a [`Condition`] to the [`WaitSet`].
    /// It is possible to attach a [`Condition`] on a WaitSet that is currently being waited upon (via the [`WaitSet::wait`] operation). In this case, if the
    /// [`Condition`] has a `trigger_value` of [`true`], then attaching the condition will unblock the [`WaitSet`].
    /// Adding a [`Condition`] that is already attached to the [`WaitSet`] has no effect.
    #[tracing::instrument(skip(self, cond))]
    pub fn attach_condition(&mut self, cond: Condition) -> DdsResult<()> {
        match cond {
            Condition::StatusCondition(sc) => block_on(self.waitset_async.attach_condition(
                ConditionAsync::StatusCondition(sc.condition_async().clone()),
            )),
        }
    }

    /// Detaches a [`Condition`] from the [`WaitSet`].
    /// If the [`Condition`] was not attached to the [`WaitSet`], the operation will return [`DdsError::PreconditionNotMet`](crate::infrastructure::error::DdsError::PreconditionNotMet).
    #[tracing::instrument(skip(self, _cond))]
    pub fn detach_condition(&self, _cond: Condition) -> DdsResult<()> {
        todo!()
    }

    /// This operation retrieves the list of attached conditions.
    #[tracing::instrument(skip(self))]
    pub fn get_conditions(&self) -> DdsResult<Vec<Condition>> {
        Ok(block_on(self.waitset_async.get_conditions())?
            .into_iter()
            .map(|c| match c {
                ConditionAsync::StatusCondition(sc) => {
                    Condition::StatusCondition(StatusCondition::new(sc))
                }
            })
            .collect())
    }
}
