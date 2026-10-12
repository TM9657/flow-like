use crate::{
    dcps::infrastructure::error::{DdsError, DdsResult},
    infrastructure::{qos_policy::TypeConsistencyEnforcementQosPolicy, time::Duration},
};

use super::{
    qos_policy::{
        DataRepresentationQosPolicy, DeadlineQosPolicy, DestinationOrderQosPolicy,
        DurabilityQosPolicy, EntityFactoryQosPolicy, GroupDataQosPolicy, HistoryQosPolicy,
        HistoryQosPolicyKind, LatencyBudgetQosPolicy, LifespanQosPolicy, LivelinessQosPolicy,
        OwnershipQosPolicy, OwnershipStrengthQosPolicy, PartitionQosPolicy, PresentationQosPolicy,
        ReaderDataLifecycleQosPolicy, ReliabilityQosPolicy, ReliabilityQosPolicyKind,
        ResourceLimitsQosPolicy, TimeBasedFilterQosPolicy, TopicDataQosPolicy,
        TransportPriorityQosPolicy, UserDataQosPolicy, WriterDataLifecycleQosPolicy,
    },
    time::DurationKind,
};

/// QoS policies applicable to the [`DomainParticipantFactory`](crate::domain::domain_participant_factory::DomainParticipantFactory)
#[derive(Debug, Default, PartialEq, Eq, Clone)]
pub struct DomainParticipantFactoryQos {
    /// Value of the entity factory QoS policy.
    pub entity_factory: EntityFactoryQosPolicy,
}

/// Enumeration representing the kind of Qos to be used
#[derive(Debug)]
pub enum QosKind<T> {
    /// Default QoS variant
    Default,
    /// Specific QoS variant
    Specific(T),
}

/// QoS policies applicable to the [`DomainParticipant`](crate::domain::domain_participant::DomainParticipant)
#[derive(Debug, Default, PartialEq, Eq, Clone)]
pub struct DomainParticipantQos {
    /// Value of the user data QoS policy.
    pub user_data: UserDataQosPolicy,
    /// Value of the entity factory QoS policy.
    pub entity_factory: EntityFactoryQosPolicy,
}

/// QoS policies applicable to the [`Publisher`](crate::publication::publisher::Publisher)
#[derive(Debug, PartialEq, Eq, Clone)]
pub struct PublisherQos {
    /// Value of the presentation QoS policy.
    pub presentation: PresentationQosPolicy,
    /// Value of the partition QoS policy.
    pub partition: PartitionQosPolicy,
    /// Value of the group data QoS policy.
    pub group_data: GroupDataQosPolicy,
    /// Value of the entity factory QoS policy.
    pub entity_factory: EntityFactoryQosPolicy,
}

impl PublisherQos {
    /// Default constructor usable in const contexts
    pub const fn const_default() -> Self {
        Self {
            presentation: PresentationQosPolicy::const_default(),
            partition: PartitionQosPolicy::const_default(),
            group_data: GroupDataQosPolicy::const_default(),
            entity_factory: EntityFactoryQosPolicy::const_default(),
        }
    }
}

impl Default for PublisherQos {
    fn default() -> Self {
        Self::const_default()
    }
}

/// QoS policies applicable to the [`DataWriter`](crate::publication::data_writer::DataWriter)
#[derive(Debug, PartialEq, Eq, Clone)]
pub struct DataWriterQos {
    /// Value of the durability QoS policy.
    pub durability: DurabilityQosPolicy,
    /// Value of the deadline QoS policy.
    pub deadline: DeadlineQosPolicy,
    /// Value of the latency budget QoS policy.
    pub latency_budget: LatencyBudgetQosPolicy,
    /// Value of the liveliness QoS policy.
    pub liveliness: LivelinessQosPolicy,
    /// Value of the reliability QoS policy.
    pub reliability: ReliabilityQosPolicy,
    /// Value of the destination order QoS policy.
    pub destination_order: DestinationOrderQosPolicy,
    /// Value of the history QoS policy.
    pub history: HistoryQosPolicy,
    /// Value of the resource limits QoS policy.
    pub resource_limits: ResourceLimitsQosPolicy,
    /// Value of the transport priority QoS policy.
    pub transport_priority: TransportPriorityQosPolicy,
    /// Value of the lifespan QoS policy.
    pub lifespan: LifespanQosPolicy,
    /// Value of the user_data QoS policy.
    pub user_data: UserDataQosPolicy,
    /// Value of the ownership QoS policy.
    pub ownership: OwnershipQosPolicy,
    /// Value of the ownership strength QoS policy.
    pub ownership_strength: OwnershipStrengthQosPolicy,
    /// Value of the writer data lifecycle QoS policy.
    pub writer_data_lifecycle: WriterDataLifecycleQosPolicy,
    /// Value of the data representation QoS policy.
    pub representation: DataRepresentationQosPolicy,
}

impl DataWriterQos {
    /// Default constructor usable in const contexts
    pub const fn const_default() -> Self {
        Self {
            reliability: ReliabilityQosPolicy {
                kind: ReliabilityQosPolicyKind::Reliable,
                max_blocking_time: DurationKind::Finite(Duration::new(
                    0,
                    100_000_000, /*100ms*/
                )),
            },
            durability: DurabilityQosPolicy::const_default(),
            deadline: DeadlineQosPolicy::const_default(),
            latency_budget: LatencyBudgetQosPolicy::const_default(),
            liveliness: LivelinessQosPolicy::const_default(),
            destination_order: DestinationOrderQosPolicy::const_default(),
            history: HistoryQosPolicy::const_default(),
            resource_limits: ResourceLimitsQosPolicy::const_default(),
            user_data: UserDataQosPolicy::const_default(),
            ownership: OwnershipQosPolicy::const_default(),
            ownership_strength: OwnershipStrengthQosPolicy::const_default(),
            lifespan: LifespanQosPolicy::const_default(),
            transport_priority: TransportPriorityQosPolicy::const_default(),
            writer_data_lifecycle: WriterDataLifecycleQosPolicy::const_default(),
            representation: DataRepresentationQosPolicy::const_default(),
        }
    }
}

impl Default for DataWriterQos {
    fn default() -> Self {
        Self::const_default()
    }
}

impl DataWriterQos {
    pub(crate) fn is_consistent(&self) -> DdsResult<()> {
        // On the writer there can be no more than one value on the representation
        if self.representation.value.len() > 1 {
            return Err(DdsError::InconsistentPolicy);
        }

        // The setting of RESOURCE_LIMITS max_samples must be consistent with the max_samples_per_instance. For these two
        // values to be consistent they must verify that *max_samples >= max_samples_per_instanc
        if self.resource_limits.max_samples < self.resource_limits.max_samples_per_instance {
            return Err(DdsError::InconsistentPolicy);
        }

        // The setting of RESOURCE_LIMITS max_samples_per_instance must be consistent with the HISTORY depth. For these two
        // QoS to be consistent, they must verify that *depth <= max_samples_per_instance.*
        match self.history.kind {
            HistoryQosPolicyKind::KeepLast(depth) => {
                if depth as usize > self.resource_limits.max_samples_per_instance {
                    Err(DdsError::InconsistentPolicy)
                } else {
                    Ok(())
                }
            }
            HistoryQosPolicyKind::KeepAll => Ok(()),
        }
    }

    pub(crate) fn check_immutability(&self, other: &Self) -> DdsResult<()> {
        if self.durability != other.durability
            || self.liveliness != other.liveliness
            || self.reliability != other.reliability
            || self.destination_order != other.destination_order
            || self.history != other.history
            || self.resource_limits != other.resource_limits
            || self.ownership != other.ownership
        {
            Err(DdsError::ImmutablePolicy)
        } else {
            Ok(())
        }
    }
}

/// QoS policies applicable to the [`Subscriber`](crate::subscription::subscriber::Subscriber)
#[derive(Debug, PartialEq, Eq, Clone)]
pub struct SubscriberQos {
    /// Value of the presentation QoS policy.
    pub presentation: PresentationQosPolicy,
    /// Value of the partition QoS policy.
    pub partition: PartitionQosPolicy,
    /// Value of the group data QoS policy.
    pub group_data: GroupDataQosPolicy,
    /// Value of the entity factory QoS policy.
    pub entity_factory: EntityFactoryQosPolicy,
}

impl SubscriberQos {
    /// Default constructor usable in const contexts
    pub const fn const_default() -> Self {
        Self {
            presentation: PresentationQosPolicy::const_default(),
            partition: PartitionQosPolicy::const_default(),
            group_data: GroupDataQosPolicy::const_default(),
            entity_factory: EntityFactoryQosPolicy::const_default(),
        }
    }

    pub(crate) fn check_immutability(&self, other: &Self) -> DdsResult<()> {
        if self.presentation != other.presentation {
            Err(DdsError::ImmutablePolicy)
        } else {
            Ok(())
        }
    }
}

impl Default for SubscriberQos {
    fn default() -> Self {
        Self::const_default()
    }
}
/// QoS policies applicable to the [`DataReader`](crate::subscription::data_reader::DataReader)
#[derive(Debug, PartialEq, Eq, Clone)]
pub struct DataReaderQos {
    /// Value of the durability QoS policy.
    pub durability: DurabilityQosPolicy,
    /// Value of the deadline QoS policy.
    pub deadline: DeadlineQosPolicy,
    /// Value of the latency budget QoS policy.
    pub latency_budget: LatencyBudgetQosPolicy,
    /// Value of the liveliness QoS policy.
    pub liveliness: LivelinessQosPolicy,
    /// Value of the reliability QoS policy.
    pub reliability: ReliabilityQosPolicy,
    /// Value of the destination order QoS policy.
    pub destination_order: DestinationOrderQosPolicy,
    /// Value of the history QoS policy.
    pub history: HistoryQosPolicy,
    /// Value of the resource limits QoS policy.
    pub resource_limits: ResourceLimitsQosPolicy,
    /// Value of the user data QoS policy.
    pub user_data: UserDataQosPolicy,
    /// Value of the ownership QoS policy.
    pub ownership: OwnershipQosPolicy,
    /// Value of the time based filter QoS policy.
    pub time_based_filter: TimeBasedFilterQosPolicy,
    /// Value of the reader data lifecycle QoS policy.
    pub reader_data_lifecycle: ReaderDataLifecycleQosPolicy,
    /// Value of the data representation QoS policy.
    pub representation: DataRepresentationQosPolicy,
    /// Value of the type consistency enforcement QoS policy,
    pub type_consistency: TypeConsistencyEnforcementQosPolicy,
}

impl DataReaderQos {
    /// Default constructor usable in const contexts
    pub const fn const_default() -> Self {
        Self {
            reliability: ReliabilityQosPolicy {
                kind: ReliabilityQosPolicyKind::BestEffort,
                max_blocking_time: DurationKind::Finite(Duration::new(
                    0,
                    100_000_000, /*100ms*/
                )),
            },
            durability: DurabilityQosPolicy::const_default(),
            deadline: DeadlineQosPolicy::const_default(),
            latency_budget: LatencyBudgetQosPolicy::const_default(),
            liveliness: LivelinessQosPolicy::const_default(),
            destination_order: DestinationOrderQosPolicy::const_default(),
            history: HistoryQosPolicy::const_default(),
            resource_limits: ResourceLimitsQosPolicy::const_default(),
            user_data: UserDataQosPolicy::const_default(),
            ownership: OwnershipQosPolicy::const_default(),
            time_based_filter: TimeBasedFilterQosPolicy::const_default(),
            reader_data_lifecycle: ReaderDataLifecycleQosPolicy::const_default(),
            representation: DataRepresentationQosPolicy::const_default(),
            type_consistency: TypeConsistencyEnforcementQosPolicy::const_default(),
        }
    }
}

impl Default for DataReaderQos {
    fn default() -> Self {
        Self::const_default()
    }
}

impl DataReaderQos {
    pub(crate) fn is_consistent(&self) -> DdsResult<()> {
        // The setting of RESOURCE_LIMITS max_samples must be consistent with the max_samples_per_instance. For these two
        // values to be consistent they must verify that *max_samples >= max_samples_per_instance.*
        if self.resource_limits.max_samples < self.resource_limits.max_samples_per_instance {
            return Err(DdsError::InconsistentPolicy);
        }

        // The setting of RESOURCE_LIMITS max_samples_per_instance must be consistent with the HISTORY depth. For these two
        // QoS to be consistent, they must verify that *depth <= max_samples_per_instance.*
        match self.history.kind {
            HistoryQosPolicyKind::KeepLast(depth) => {
                if depth as usize > self.resource_limits.max_samples_per_instance {
                    return Err(DdsError::InconsistentPolicy);
                }
            }
            HistoryQosPolicyKind::KeepAll => (),
        }

        // The setting of the DEADLINE policy must be set consistently with that of the TIME_BASED_FILTER. For these two policies
        // to be consistent the settings must be such that *deadline period>= minimum_separation.*
        if self.deadline.period < self.time_based_filter.minimum_separation {
            return Err(DdsError::InconsistentPolicy);
        }

        Ok(())
    }

    pub(crate) fn check_immutability(&self, other: &Self) -> DdsResult<()> {
        if self.durability != other.durability
            || self.liveliness != other.liveliness
            || self.reliability != other.reliability
            || self.destination_order != other.destination_order
            || self.history != other.history
            || self.resource_limits != other.resource_limits
            || self.ownership != other.ownership
        {
            Err(DdsError::ImmutablePolicy)
        } else {
            Ok(())
        }
    }
}

/// QoS policies applicable to the [`Topic`](crate::topic_definition::topic::Topic)
#[derive(Debug, PartialEq, Eq, Clone)]
pub struct TopicQos {
    /// Value of the topic data QoS policy.
    pub topic_data: TopicDataQosPolicy,
    /// Value of the durability QoS policy.
    pub durability: DurabilityQosPolicy,
    /// Value of the deadline QoS policy.
    pub deadline: DeadlineQosPolicy,
    /// Value of the latency budget QoS policy.
    pub latency_budget: LatencyBudgetQosPolicy,
    /// Value of the liveliness QoS policy.
    pub liveliness: LivelinessQosPolicy,
    /// Value of the reliability QoS policy.
    pub reliability: ReliabilityQosPolicy,
    /// Value of the destination order QoS policy.
    pub destination_order: DestinationOrderQosPolicy,
    /// Value of the history QoS policy.
    pub history: HistoryQosPolicy,
    /// Value of the resource limits QoS policy.
    pub resource_limits: ResourceLimitsQosPolicy,
    /// Value of the transport priority QoS policy.
    pub transport_priority: TransportPriorityQosPolicy,
    /// Value of the lifespan QoS policy.
    pub lifespan: LifespanQosPolicy,
    /// Value of the ownership QoS policy.
    pub ownership: OwnershipQosPolicy,
    /// Value of the data representation QoS policy.
    pub representation: DataRepresentationQosPolicy,
}

impl TopicQos {
    /// Default constructor usable in const contexts
    pub const fn const_default() -> Self {
        Self {
            reliability: ReliabilityQosPolicy {
                kind: ReliabilityQosPolicyKind::BestEffort,
                max_blocking_time: DurationKind::Finite(Duration::new(
                    0,
                    100_000_000, /*100ms*/
                )),
            },
            topic_data: TopicDataQosPolicy::const_default(),
            durability: DurabilityQosPolicy::const_default(),
            deadline: DeadlineQosPolicy::const_default(),
            latency_budget: LatencyBudgetQosPolicy::const_default(),
            liveliness: LivelinessQosPolicy::const_default(),
            destination_order: DestinationOrderQosPolicy::const_default(),
            history: HistoryQosPolicy::const_default(),
            resource_limits: ResourceLimitsQosPolicy::const_default(),
            transport_priority: TransportPriorityQosPolicy::const_default(),
            lifespan: LifespanQosPolicy::const_default(),
            ownership: OwnershipQosPolicy::const_default(),
            representation: DataRepresentationQosPolicy::const_default(),
        }
    }
}

impl Default for TopicQos {
    fn default() -> Self {
        Self::const_default()
    }
}

impl TopicQos {
    pub(crate) fn is_consistent(&self) -> DdsResult<()> {
        // The setting of RESOURCE_LIMITS max_samples must be consistent with the max_samples_per_instance. For these two
        // values to be consistent they must verify that *max_samples >= max_samples_per_instance.*
        if self.resource_limits.max_samples < self.resource_limits.max_samples_per_instance {
            return Err(DdsError::InconsistentPolicy);
        }

        // The setting of RESOURCE_LIMITS max_samples_per_instance must be consistent with the HISTORY depth. For these two
        // QoS to be consistent, they must verify that *depth <= max_samples_per_instance.*
        match self.history.kind {
            HistoryQosPolicyKind::KeepLast(depth) => {
                if depth as usize > self.resource_limits.max_samples_per_instance {
                    Err(DdsError::InconsistentPolicy)
                } else {
                    Ok(())
                }
            }
            HistoryQosPolicyKind::KeepAll => Ok(()),
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::infrastructure::qos_policy::Length;

    use super::*;

    #[test]
    fn data_writer_qos_consistency() {
        assert_eq!(DataWriterQos::default().is_consistent(), Ok(()));
        assert_eq!(
            DataWriterQos {
                resource_limits: ResourceLimitsQosPolicy {
                    max_samples_per_instance: Length::Limited(2),
                    max_samples: Length::Limited(1),
                    ..Default::default()
                },
                ..Default::default()
            }
            .is_consistent(),
            Err(DdsError::InconsistentPolicy)
        );
        assert_eq!(
            DataWriterQos {
                history: HistoryQosPolicy {
                    kind: HistoryQosPolicyKind::KeepLast(3),
                },
                resource_limits: ResourceLimitsQosPolicy {
                    max_samples_per_instance: Length::Limited(2),
                    ..Default::default()
                },
                ..Default::default()
            }
            .is_consistent(),
            Err(DdsError::InconsistentPolicy)
        );
    }

    #[test]
    fn data_reader_qos_consistency() {
        assert_eq!(DataReaderQos::default().is_consistent(), Ok(()));
        assert_eq!(
            DataReaderQos {
                resource_limits: ResourceLimitsQosPolicy {
                    max_samples_per_instance: Length::Limited(2),
                    max_samples: Length::Limited(1),
                    ..Default::default()
                },
                ..Default::default()
            }
            .is_consistent(),
            Err(DdsError::InconsistentPolicy)
        );
        assert_eq!(
            DataReaderQos {
                history: HistoryQosPolicy {
                    kind: HistoryQosPolicyKind::KeepLast(3),
                },
                resource_limits: ResourceLimitsQosPolicy {
                    max_samples_per_instance: Length::Limited(2),
                    ..Default::default()
                },
                ..Default::default()
            }
            .is_consistent(),
            Err(DdsError::InconsistentPolicy)
        );
    }

    #[test]
    fn topic_qos_consistency() {
        assert_eq!(TopicQos::default().is_consistent(), Ok(()));
        assert_eq!(
            TopicQos {
                resource_limits: ResourceLimitsQosPolicy {
                    max_samples_per_instance: Length::Limited(2),
                    max_samples: Length::Limited(1),
                    ..Default::default()
                },
                ..Default::default()
            }
            .is_consistent(),
            Err(DdsError::InconsistentPolicy)
        );
        assert_eq!(
            TopicQos {
                history: HistoryQosPolicy {
                    kind: HistoryQosPolicyKind::KeepLast(3),
                },
                resource_limits: ResourceLimitsQosPolicy {
                    max_samples_per_instance: Length::Limited(2),
                    ..Default::default()
                },
                ..Default::default()
            }
            .is_consistent(),
            Err(DdsError::InconsistentPolicy)
        );
    }
}
