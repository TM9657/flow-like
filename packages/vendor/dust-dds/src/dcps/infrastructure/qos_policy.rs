use super::time::{DURATION_ZERO_NSEC, DURATION_ZERO_SEC};
use crate::{
    infrastructure::time::{Duration, DurationKind},
    transport::types::{DurabilityKind, ReliabilityKind},
    xtypes::{
        dynamic_type::{
            DynamicData, DynamicDataFactory, DynamicType, DynamicTypeMember, ExtensibilityKind,
            TypeDescriptor, TypeKind,
        },
        type_support::{Type, TypeSupport},
    },
};
use alloc::{string::String, vec::Vec};
use core::cmp::Ordering;

/// QosPolicyId type alias
pub type QosPolicyId = i32;

#[derive(Debug, PartialEq, Eq, Clone, Copy)]
/// Enumeration representing a Length which be either limited or unlimited.
pub enum Length {
    /// Unlimited length.
    Unlimited,
    /// Limited length with the corresponding associated value.
    Limited(i32),
}

impl From<Length> for i32 {
    fn from(value: Length) -> Self {
        match value {
            Length::Unlimited => LENGTH_UNLIMITED,
            Length::Limited(i) => i,
        }
    }
}

impl From<i32> for Length {
    fn from(value: i32) -> Self {
        if value == LENGTH_UNLIMITED {
            Length::Unlimited
        } else {
            Length::Limited(value)
        }
    }
}

const LENGTH_UNLIMITED: i32 = i32::MAX;

impl PartialOrd for Length {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        match self {
            Length::Unlimited => match other {
                Length::Unlimited => Some(Ordering::Equal),
                Length::Limited(_) => Some(Ordering::Greater),
            },
            Length::Limited(value) => match other {
                Length::Unlimited => Some(Ordering::Less),
                Length::Limited(other) => value.partial_cmp(other),
            },
        }
    }
}

impl PartialEq<usize> for Length {
    fn eq(&self, other: &usize) -> bool {
        match self {
            Length::Unlimited => false,
            Length::Limited(value) => (*value as usize).eq(other),
        }
    }
}

impl PartialOrd<usize> for Length {
    fn partial_cmp(&self, other: &usize) -> Option<Ordering> {
        match self {
            Length::Unlimited => Some(Ordering::Greater),
            Length::Limited(value) => (*value as usize).partial_cmp(other),
        }
    }
}

impl PartialEq<Length> for usize {
    fn eq(&self, other: &Length) -> bool {
        match other {
            Length::Unlimited => false,
            Length::Limited(value) => self.eq(&(*value as usize)),
        }
    }
}

impl PartialOrd<Length> for usize {
    fn partial_cmp(&self, other: &Length) -> Option<Ordering> {
        match other {
            Length::Unlimited => Some(Ordering::Less),
            Length::Limited(value) => self.partial_cmp(&(*value as usize)),
        }
    }
}

/// This class is the abstract root for all the QoS policies.
/// It provides the basic mechanism for an application to specify quality of service parameters. It has an attribute name that is used
/// to identify uniquely each QoS policy. All concrete QosPolicy classes derive from this root and include a value whose type
/// depends on the concrete QoS policy.
/// The type of a QosPolicy value may be atomic, such as an integer or float, or compound (a structure). Compound types are used
/// whenever multiple parameters must be set coherently to define a consistent value for a QosPolicy.
/// Each Entity can be configured with a list of QosPolicy. However, any Entity cannot support any QosPolicy. For instance, a
/// DomainParticipant supports different QosPolicy than a Topic or a Publisher.
/// QosPolicy can be set when the Entity is created, or modified with the set_qos method. Each QosPolicy in the list is treated
/// independently from the others. This approach has the advantage of being very extensible. However, there may be cases where
/// several policies are in conflict. Consistency checking is performed each time the policies are modified via the set_qos
/// operation.
/// When a policy is changed after being set to a given value, it is not required that the new value be applied instantaneously; the
/// Service is allowed to apply it after a transition phase. In addition, some QosPolicy have *immutable* semantics meaning that
/// they can only be specified either at Entity creation time or else prior to calling the enable operation on the Entity.
/// Sub clause 2.2.3, Supported QoS provides the list of all QosPolicy, their meaning, characteristics and possible values, as well
/// as the concrete Entity to which they apply.
pub trait QosPolicy {
    /// Get the name of the QoS policy
    fn name(&self) -> &str;
}

const USERDATA_QOS_POLICY_NAME: &str = "UserData";
const DURABILITY_QOS_POLICY_NAME: &str = "Durability";
const PRESENTATION_QOS_POLICY_NAME: &str = "Presentation";
const DEADLINE_QOS_POLICY_NAME: &str = "Deadline";
const LATENCYBUDGET_QOS_POLICY_NAME: &str = "LatencyBudget";
const OWNERSHIP_QOS_POLICY_NAME: &str = "Ownership";
const OWNERSHIP_STRENGTH_QOS_POLICY_NAME: &str = "OwnershipStrength";
const LIVELINESS_QOS_POLICY_NAME: &str = "Liveliness";
const TIMEBASEDFILTER_QOS_POLICY_NAME: &str = "TimeBasedFilter";
const PARTITION_QOS_POLICY_NAME: &str = "Partition";
const RELIABILITY_QOS_POLICY_NAME: &str = "Reliability";
const DESTINATIONORDER_QOS_POLICY_NAME: &str = "DestinationOrder";
const HISTORY_QOS_POLICY_NAME: &str = "History";
const RESOURCELIMITS_QOS_POLICY_NAME: &str = "ResourceLimits";
const ENTITYFACTORY_QOS_POLICY_NAME: &str = "EntityFactory";
const WRITERDATALIFECYCLE_QOS_POLICY_NAME: &str = "WriterDataLifecycle";
const READERDATALIFECYCLE_QOS_POLICY_NAME: &str = "ReaderDataLifecycle";
const TOPICDATA_QOS_POLICY_NAME: &str = "TopicData";
const TRANSPORTPRIORITY_QOS_POLICY_NAME: &str = "TransportPriority";
const GROUPDATA_QOS_POLICY_NAME: &str = "GroupData";
const LIFESPAN_QOS_POLICY_NAME: &str = "Lifespan";
const DATA_REPRESENTATION_QOS_POLICY_NAME: &str = "DataRepresentation";
const TYPE_CONSISTENCY_ENFORCEMENT_QOS_POLICY_NAME: &str = "TypeConsistencyEnforcement";

/// QosPolicy Id representing an invalid QoS policy
pub const INVALID_QOS_POLICY_ID: QosPolicyId = 0;
/// Id for the UserDataQosPolicy
pub const USERDATA_QOS_POLICY_ID: QosPolicyId = 1;
/// Id for the DurabilityQosPolicy
pub const DURABILITY_QOS_POLICY_ID: QosPolicyId = 2;
/// Id for the PresentationQosPolicy
pub const PRESENTATION_QOS_POLICY_ID: QosPolicyId = 3;
/// Id for the DeadlineQosPolicy
pub const DEADLINE_QOS_POLICY_ID: QosPolicyId = 4;
/// Id for the LatencyBudgetQosPolicy
pub const LATENCYBUDGET_QOS_POLICY_ID: QosPolicyId = 5;
/// Id for the OwnershipQosPolicy
pub const OWNERSHIP_QOS_POLICY_ID: QosPolicyId = 6;
/// Id for the OwnershipStrengthQosPolicy
pub const OWNERSHIP_STRENGTH_QOS_POLICY_ID: QosPolicyId = 7;
/// Id for the LivelinessQosPolicy
pub const LIVELINESS_QOS_POLICY_ID: QosPolicyId = 8;
/// Id for the TimeBasedFilterQosPolicy
pub const TIMEBASEDFILTER_QOS_POLICY_ID: QosPolicyId = 9;
/// Id for the PartitionQosPolicy
pub const PARTITION_QOS_POLICY_ID: QosPolicyId = 10;
/// Id for the ReliabilityQosPolicy
pub const RELIABILITY_QOS_POLICY_ID: QosPolicyId = 11;
/// Id for the DestinationOrderQosPolicy
pub const DESTINATIONORDER_QOS_POLICY_ID: QosPolicyId = 12;
/// Id for the HistoryQosPolicy
pub const HISTORY_QOS_POLICY_ID: QosPolicyId = 13;
/// Id for the ResourceLimitsQosPolicy
pub const RESOURCELIMITS_QOS_POLICY_ID: QosPolicyId = 14;
/// Id for the EntityFactoryQosPolicy
pub const ENTITYFACTORY_QOS_POLICY_ID: QosPolicyId = 15;
/// Id for the WriterDataLifecycleQosPolicy
pub const WRITERDATALIFECYCLE_QOS_POLICY_ID: QosPolicyId = 16;
/// Id for the ReaderDataLifecycleQosPolicy
pub const READERDATALIFECYCLE_QOS_POLICY_ID: QosPolicyId = 17;
/// Id for the TopicDataQosPolicy
pub const TOPICDATA_QOS_POLICY_ID: QosPolicyId = 18;
/// Id for the GroupDataQosPolicy
pub const GROUPDATA_QOS_POLICY_ID: QosPolicyId = 19;
/// Id for the TransportPriorityQosPolicy
pub const TRANSPORTPRIORITY_QOS_POLICY_ID: QosPolicyId = 20;
/// Id for the LifespanQosPolicy
pub const LIFESPAN_QOS_POLICY_ID: QosPolicyId = 21;
/// Id for the DurabilityServiceQosPolicy
pub const DURABILITYSERVICE_QOS_POLICY_ID: QosPolicyId = 22;
/// Id for the DataRepresentationQosPolicy
pub const DATA_REPRESENTATION_QOS_POLICY_ID: QosPolicyId = 23;
/// Id for the TypeConsistencyEnforcementQosPolicy
pub const TYPE_CONSISTENCY_ENFORCEMENT_QOS_POLICY_ID: QosPolicyId = 24;

/// This policy allows the application to attach additional information to the created Entity objects such that when
/// a remote application discovers their existence it can access that information and use it for its own purposes.
#[derive(Debug, PartialEq, Eq, Clone, TypeSupport)]
#[dust_dds(extensibility = "appendable", nested)]
pub struct UserDataQosPolicy {
    /// User data value
    pub value: Vec<u8>,
}

impl UserDataQosPolicy {
    /// Default constructor usable in const contexts
    pub const fn const_default() -> Self {
        Self { value: Vec::new() }
    }
}

impl QosPolicy for UserDataQosPolicy {
    fn name(&self) -> &str {
        USERDATA_QOS_POLICY_NAME
    }
}
impl Default for UserDataQosPolicy {
    fn default() -> Self {
        Self::const_default()
    }
}

/// This policy allows the application to attach additional information to the created Topic such that when a
/// remote application discovers their existence it can examine the information and use it in an application-defined way.
#[derive(Debug, PartialEq, Eq, Clone, TypeSupport)]
#[dust_dds(extensibility = "appendable", nested)]
pub struct TopicDataQosPolicy {
    /// Topic data value
    pub value: Vec<u8>,
}

impl TopicDataQosPolicy {
    /// Default constructor usable in const contexts
    pub const fn const_default() -> Self {
        Self { value: Vec::new() }
    }
}

impl Default for TopicDataQosPolicy {
    fn default() -> Self {
        Self::const_default()
    }
}

impl QosPolicy for TopicDataQosPolicy {
    fn name(&self) -> &str {
        TOPICDATA_QOS_POLICY_NAME
    }
}

/// This policy allows the application to attach additional information to the created
/// [`Publisher`](crate::publication::publisher::Publisher) or [`Subscriber`](crate::subscription::subscriber::Subscriber).
///
/// The value is available to the application on the
/// [`DataReader`](crate::subscription::data_reader::DataReader) and [`DataWriter`](crate::publication::data_writer::DataWriter) entities and is propagated by
/// means of the built-in topics.
#[derive(Debug, PartialEq, Eq, Clone, TypeSupport)]
#[dust_dds(extensibility = "appendable", nested)]
pub struct GroupDataQosPolicy {
    /// Group data value
    pub value: Vec<u8>,
}

impl GroupDataQosPolicy {
    /// Default constructor usable in const contexts
    pub const fn const_default() -> Self {
        Self { value: Vec::new() }
    }
}

impl QosPolicy for GroupDataQosPolicy {
    fn name(&self) -> &str {
        GROUPDATA_QOS_POLICY_NAME
    }
}

impl Default for GroupDataQosPolicy {
    fn default() -> Self {
        Self::const_default()
    }
}

/// This policy allows the application to take advantage of transports capable of sending messages with different priorities.
///
/// This policy is considered a hint. The policy depends on the ability of the underlying transports to set a priority on the messages
/// they send. Any value within the range of a 32-bit signed integer may be chosen; higher values indicate higher priority.
/// However, any further interpretation of this policy is specific to a particular transport and a particular implementation of the
/// Service. For example, a particular transport is permitted to treat a range of priority values as equivalent to one another. It is
/// expected that during transport configuration the application would provide a mapping between the values of the
/// [`TransportPriorityQosPolicy`] set on [`DataWriter`](crate::publication::data_writer::DataWriter) and the values meaningful to each transport.
/// This mapping would then be used by the infrastructure when propagating the data written by the [`DataWriter`](crate::publication::data_writer::DataWriter).
#[derive(Debug, PartialEq, Eq, Clone, TypeSupport)]
#[dust_dds(extensibility = "appendable", nested)]
pub struct TransportPriorityQosPolicy {
    /// Transport priority value
    pub value: i32,
}

impl TransportPriorityQosPolicy {
    /// Default constructor usable in const contexts
    pub const fn const_default() -> Self {
        Self { value: 0 }
    }
}

impl QosPolicy for TransportPriorityQosPolicy {
    fn name(&self) -> &str {
        TRANSPORTPRIORITY_QOS_POLICY_NAME
    }
}

impl Default for TransportPriorityQosPolicy {
    fn default() -> Self {
        Self::const_default()
    }
}

/// This policy is used to avoid delivering *stale* data to the application.
///
/// Each data sample written by the [`DataWriter`](crate::publication::data_writer::DataWriter) has an associated 'expiration time' beyond which the data should not be delivered
/// to any application. Once the sample expires, the data will be removed from the [`DataReader`](crate::subscription::data_reader::DataReader) caches as well as from the
/// transient and persistent information caches.
/// The 'expiration time' of each sample is computed by adding the duration specified by the [`LifespanQosPolicy`] to the source
/// timestamp. The source timestamp is either automatically computed by the Service
/// each time the [`DataWriter::write()`](crate::publication::data_writer::DataWriter) operation is called, or else supplied by the application by means
/// of the  [`DataWriter::write_w_timestamp()`](crate::publication::data_writer::DataWriter)
/// operation.
/// This QoS relies on the sender and receiving applications having their clocks sufficiently synchronized. If this is not the case
/// and the Service can detect it, the [`DataReader`](crate::subscription::data_reader::DataReader) is allowed to use the reception timestamp instead of the source timestamp in its
/// computation of the 'expiration time.'
#[derive(Debug, PartialEq, Eq, Clone, TypeSupport)]
#[dust_dds(extensibility = "appendable", nested)]
pub struct LifespanQosPolicy {
    /// Lifespan duration
    pub duration: DurationKind,
}

impl LifespanQosPolicy {
    /// Default constructor usable in const contexts
    pub const fn const_default() -> Self {
        Self {
            duration: DurationKind::Infinite,
        }
    }
}

impl QosPolicy for LifespanQosPolicy {
    fn name(&self) -> &str {
        LIFESPAN_QOS_POLICY_NAME
    }
}

impl Default for LifespanQosPolicy {
    fn default() -> Self {
        Self::const_default()
    }
}

#[derive(Debug, PartialEq, Eq, Clone, Copy, TypeSupport)]
/// Enumeration representing the different types of Durability QoS policies.
pub enum DurabilityQosPolicyKind {
    /// Volatile durability QoS policy
    Volatile,
    /// TransientLocal durability QoS policy
    TransientLocal,
    /// Transient durability QoS policy
    Transient,
    /// Persistent durability QoS policy
    Persistent,
}

impl PartialOrd for DurabilityQosPolicyKind {
    fn partial_cmp(&self, other: &DurabilityQosPolicyKind) -> Option<Ordering> {
        match self {
            DurabilityQosPolicyKind::Volatile => match other {
                DurabilityQosPolicyKind::Volatile => Some(Ordering::Equal),
                DurabilityQosPolicyKind::TransientLocal => Some(Ordering::Less),
                DurabilityQosPolicyKind::Transient => Some(Ordering::Less),
                DurabilityQosPolicyKind::Persistent => Some(Ordering::Less),
            },
            DurabilityQosPolicyKind::TransientLocal => match other {
                DurabilityQosPolicyKind::Volatile => Some(Ordering::Greater),
                DurabilityQosPolicyKind::TransientLocal => Some(Ordering::Equal),
                DurabilityQosPolicyKind::Transient => Some(Ordering::Less),
                DurabilityQosPolicyKind::Persistent => Some(Ordering::Less),
            },
            DurabilityQosPolicyKind::Transient => match other {
                DurabilityQosPolicyKind::Volatile => Some(Ordering::Greater),
                DurabilityQosPolicyKind::TransientLocal => Some(Ordering::Greater),
                DurabilityQosPolicyKind::Transient => Some(Ordering::Equal),
                DurabilityQosPolicyKind::Persistent => Some(Ordering::Less),
            },
            DurabilityQosPolicyKind::Persistent => match other {
                DurabilityQosPolicyKind::Volatile => Some(Ordering::Greater),
                DurabilityQosPolicyKind::TransientLocal => Some(Ordering::Greater),
                DurabilityQosPolicyKind::Transient => Some(Ordering::Greater),
                DurabilityQosPolicyKind::Persistent => Some(Ordering::Equal),
            },
        }
    }
}

/// This policy controls whether the Service will actually make data available to late-joining readers.
///
/// The decoupling between [`DataReader`](crate::subscription::data_reader::DataReader) and [`DataWriter`](crate::publication::data_writer::DataWriter)
/// offered by the Publish/Subscribe paradigm allows an application to write data even if there are
/// no current readers on the network. Moreover, a [`DataReader`](crate::subscription::data_reader::DataReader) that joins the network after some data
/// has been written could potentially be interested in accessing the most current values of the data as well as potentially some
/// history.
/// Note that although related, this does not strictly control what data the Service will maintain internally.
/// That is, the Service may choose to maintain some data for its own purposes (e.g., flow control)
/// and yet not make it available to late-joining readers if the [`DurabilityQosPolicy`] is set to [`DurabilityQosPolicyKind::Volatile`].
/// The value offered is considered compatible with the value requested if and only if the *offered kind >= requested
/// kind* is true. For the purposes of this inequality, the values of [`DurabilityQosPolicyKind`] kind are considered ordered such
/// that *Volatile < TransientLocal*.
#[derive(Debug, PartialEq, Eq, PartialOrd, Clone, TypeSupport)]
#[dust_dds(extensibility = "appendable", nested)]
pub struct DurabilityQosPolicy {
    /// DurabilityQosPolicy kind to be used for this policy
    pub kind: DurabilityQosPolicyKind,
}

impl DurabilityQosPolicy {
    /// Default constructor usable in const contexts
    pub const fn const_default() -> Self {
        Self {
            kind: DurabilityQosPolicyKind::Volatile,
        }
    }
}

impl QosPolicy for DurabilityQosPolicy {
    fn name(&self) -> &str {
        DURABILITY_QOS_POLICY_NAME
    }
}

impl Default for DurabilityQosPolicy {
    fn default() -> Self {
        Self::const_default()
    }
}

#[derive(Debug, PartialEq, Eq, Clone, Copy, TypeSupport)]
/// Enumeration representing the different types of Presentation QoS policy access scope.
pub enum PresentationQosPolicyAccessScopeKind {
    /// Access scope per instance
    Instance,
    /// Access scope per topic
    Topic,
}

impl PartialOrd for PresentationQosPolicyAccessScopeKind {
    fn partial_cmp(&self, other: &PresentationQosPolicyAccessScopeKind) -> Option<Ordering> {
        match self {
            PresentationQosPolicyAccessScopeKind::Instance => match other {
                PresentationQosPolicyAccessScopeKind::Instance => Some(Ordering::Equal),
                PresentationQosPolicyAccessScopeKind::Topic => Some(Ordering::Less),
            },
            PresentationQosPolicyAccessScopeKind::Topic => match other {
                PresentationQosPolicyAccessScopeKind::Instance => Some(Ordering::Greater),
                PresentationQosPolicyAccessScopeKind::Topic => Some(Ordering::Equal),
            },
        }
    }
}

/// This policy controls the extent to which changes to data-instances can be made dependent on each other and also the kind
/// of dependencies that can be propagated and maintained by the Service.
///
/// The setting of [`PresentationQosPolicy::coherent_access`] controls whether the Service will preserve the groupings of changes made by the publishing
/// application by means of the operations `begin_coherent_change()` and `end_coherent_change()`.
/// The setting of  [`PresentationQosPolicy::ordered_access`] controls whether the Service will preserve the order of changes.
/// The granularity is controlled by the setting of the  [`PresentationQosPolicy::access_scope`].
/// If [`PresentationQosPolicy::coherent_access`] is set, then the [`PresentationQosPolicy::access_scope`] controls the maximum extent of coherent changes.
/// The behavior is as follows:
/// - If access_scope is set to INSTANCE, the use of begin_coherent_change and end_coherent_change has no effect on
///   how the subscriber can access the data because with the scope limited to each instance, changes to separate instances
///   are considered independent and thus cannot be grouped by a coherent change.
/// - If access_scope is set to TOPIC, then coherent changes (indicated by their enclosure within calls to
///   begin_coherent_change and end_coherent_change) will be made available as such to each remote DataReader
///   independently. That is, changes made to instances within each individual DataWriter will be available as coherent with
///   respect to other changes to instances in that same DataWriter, but will not be grouped with changes made to instances
///   belonging to a different DataWriter.
///   If ordered_access is set, then the access_scope controls the maximum extent for which order will be preserved by the Service.
/// - If access_scope is set to INSTANCE (the lowest level), then changes to each instance are considered unordered relative
///   to changes to any other instance. That means that changes (creations, deletions, modifications) made to two instances
///   are not necessarily seen in the order they occur. This is the case even if it is the same application thread making the
///   changes using the same DataWriter.
/// - If access_scope is set to TOPIC, changes (creations, deletions, modifications) made by a single DataWriter are made
///   available to subscribers in the same order they occur. Changes made to instances through different DataWriter entities
///   are not necessarily seen in the order they occur. This is the case, even if the changes are made by a single application
///   thread using DataWriter objects attached to the same Publisher.
///
/// Note that this QoS policy controls the scope at which related changes are made available to the subscriber. This means the
/// subscriber can access the changes in a coherent manner and in the proper order; however, it does not necessarily imply that the
/// Subscriber will indeed access the changes in the correct order. For that to occur, the application at the subscriber end must use
/// the proper logic in reading the DataReader objects.
/// The value offered is considered compatible with the value requested if and only if the following conditions are met:
/// 1. The inequality *offered access_scope >= requested access_scope* is true. For the purposes of this
///    inequality, the values of PRESENTATION access_scope are considered ordered such that INSTANCE < TOPIC <
///    GROUP.
/// 2. Requested coherent_access is FALSE, or else both offered and requested coherent_access are TRUE.
/// 3. Requested ordered_access is FALSE, or else both offered and requested ordered _access are TRUE.
#[derive(Debug, PartialEq, Eq, Clone, TypeSupport)]
#[dust_dds(extensibility = "appendable", nested)]
pub struct PresentationQosPolicy {
    /// Presentation access scope kind to be used for this policy
    pub access_scope: PresentationQosPolicyAccessScopeKind,
    /// Coherent access value
    pub coherent_access: bool,
    /// Ordered access value
    pub ordered_access: bool,
}

impl PresentationQosPolicy {
    /// Default constructor usable in const contexts
    pub const fn const_default() -> Self {
        Self {
            access_scope: PresentationQosPolicyAccessScopeKind::Instance,
            coherent_access: false,
            ordered_access: false,
        }
    }
}

impl QosPolicy for PresentationQosPolicy {
    fn name(&self) -> &str {
        PRESENTATION_QOS_POLICY_NAME
    }
}

impl Default for PresentationQosPolicy {
    fn default() -> Self {
        Self::const_default()
    }
}

/// This policy is useful for cases where a [`Topic`](crate::topic_definition::topic::Topic) is expected to have each instance updated periodically. On the publishing side this
/// setting establishes a contract that the application must meet.
///
/// On the subscribing side the setting establishes a minimum
/// requirement for the remote publishers that are expected to supply the data values.
/// When the Service 'matches' a [`DataWriter`](crate::publication::data_writer::DataWriter) and a [`DataReader`](crate::subscription::data_reader::DataReader) it checks whether the settings are compatible (i.e., *offered
/// deadline period <= requested deadline period*) if they are not, the two entities are informed (via the listener or condition
/// mechanism) of the incompatibility of the QoS settings and communication will not occur.
/// Assuming that the reader and writer ends have compatible settings, the fulfillment of this contract is monitored by the Service
/// and the application is informed of any violations by means of the proper listener or condition.
/// The value offered is considered compatible with the value requested if and only if the *offered deadline period <=
/// requested deadline period* is true.
/// The setting of the [`DeadlineQosPolicy`] policy must be set consistently with that of the [`TimeBasedFilterQosPolicy`]. For these two policies
/// to be consistent the settings must be such that *deadline period >= minimum_separation*.
#[derive(Debug, PartialEq, Eq, PartialOrd, Clone, TypeSupport)]
#[dust_dds(extensibility = "appendable", nested)]
pub struct DeadlineQosPolicy {
    /// Deadline period value
    pub period: DurationKind,
}

impl DeadlineQosPolicy {
    /// Default constructor usable in const contexts
    pub const fn const_default() -> Self {
        Self {
            period: DurationKind::Infinite,
        }
    }
}

impl QosPolicy for DeadlineQosPolicy {
    fn name(&self) -> &str {
        DEADLINE_QOS_POLICY_NAME
    }
}

impl Default for DeadlineQosPolicy {
    fn default() -> Self {
        Self::const_default()
    }
}

/// This policy provides a means for the application to indicate to the middleware the *urgency* of the data-communication.
///
/// By having a non-zero duration the Service can optimize its internal operation.
/// This policy is considered a hint. There is no specified mechanism as to how the service should take advantage of this hint.
/// The value offered is considered compatible with the value requested if and only if the *offered duration <=
/// requested duration* is true.
#[derive(PartialOrd, PartialEq, Eq, Debug, Clone, TypeSupport)]
#[dust_dds(extensibility = "appendable", nested)]
pub struct LatencyBudgetQosPolicy {
    /// Latency budget duration value
    pub duration: DurationKind,
}

impl LatencyBudgetQosPolicy {
    /// Default constructor usable in const contexts
    pub const fn const_default() -> Self {
        Self {
            duration: DurationKind::Finite(Duration::new(DURATION_ZERO_SEC, DURATION_ZERO_NSEC)),
        }
    }
}

impl QosPolicy for LatencyBudgetQosPolicy {
    fn name(&self) -> &str {
        LATENCYBUDGET_QOS_POLICY_NAME
    }
}

impl Default for LatencyBudgetQosPolicy {
    fn default() -> Self {
        Self::const_default()
    }
}

/// Enumeration representing the different types of Ownership QoS policies.
#[derive(Debug, PartialEq, Eq, Clone, Copy, TypeSupport)]
pub enum OwnershipQosPolicyKind {
    /// Shared ownership QoS policy
    Shared,
    /// Exclusive ownership QoS policy
    Exclusive,
}

/// This policy controls whether the Service allows multiple [`DataWriter`](crate::publication::data_writer::DataWriter)
/// objects to update the same instance (identified by Topic + key) of a data-object.
///
/// Only [`OwnershipQosPolicyKind::Shared`] can be selected. This setting indicates that the Service does not enforce unique ownership for each instance.
/// In this case, multiple writers can update the same data-object instance. The subscriber to the Topic will be able to access modifications from all DataWriter
/// objects, subject to the settings of other QoS that may filter particular samples (e.g., the [`TimeBasedFilterQosPolicy`] or [`HistoryQosPolicy`]).
/// In any case there is no *filtering* of modifications made based on the identity of the DataWriter that causes the
/// modification.

#[derive(Debug, PartialEq, Eq, Clone, TypeSupport)]
#[dust_dds(extensibility = "appendable", nested)]
pub struct OwnershipQosPolicy {
    /// Kind of ownership QoS associated with this policy
    pub kind: OwnershipQosPolicyKind,
}

impl OwnershipQosPolicy {
    /// Default constructor usable in const contexts
    pub const fn const_default() -> Self {
        Self {
            kind: OwnershipQosPolicyKind::Shared,
        }
    }
}

impl QosPolicy for OwnershipQosPolicy {
    fn name(&self) -> &str {
        OWNERSHIP_QOS_POLICY_NAME
    }
}

impl Default for OwnershipQosPolicy {
    fn default() -> Self {
        Self::const_default()
    }
}

/// This policy should be used in combination with the [`OwnershipQosPolicy`]. It only applies to the case where
/// [`OwnershipQosPolicy`] kind is set to [`OwnershipQosPolicyKind::Exclusive`].
///
/// The value of the [`OwnershipStrengthQosPolicy`] is used to determine the ownership of a data-instance (identified by the key).
/// The arbitration is performed by the DataReader.
/// This setting indicates that each instance of a data-object can only be modified by one DataWriter. In other words, at any point
/// in time a single DataWriter "owns" each instance and is the only one whose modifications will be visible to the DataReader
/// objects. The owner is determined by selecting the DataWriter with the highest value of the strength that is both "alive" as
/// defined by the LIVELINESS QoS policy and has not violated its DEADLINE contract with regards to the data-instance.
/// Ownership can therefore change as a result of (a) a DataWriter in the system with a higher value of the strength that modifies
/// the instance, (b) a change in the strength value of the DataWriter that owns the instance, (c) a change in the liveliness of the
/// DataWriter that owns the instance, and (d) a deadline with regards to the instance that is missed by the DataWriter that owns
/// the instance.
/// The behavior of the system is as if the determination was made independently by each DataReader. Each DataReader may
/// detect the change of ownership at a different time. It is not a requirement that at a particular point in time all the DataReader
/// objects for that Topic have a consistent picture of who owns each instance.
/// It is also not a requirement that the DataWriter objects are aware of whether they own a particular instance. There is no error or
/// notification given to a DataWriter that modifies an instance it does not currently own.
/// The requirements are chosen to (a) preserve the decoupling of publishers and subscriber, and (b) allow the policy to be
/// implemented efficiently.
/// It is possible that multiple DataWriter objects with the same strength modify the same instance. If this occurs the Service will
/// pick one of the DataWriter objects as the "owner". It is not specified how the owner is selected. However, it is required that the
/// policy used to select the owner is such that all DataReader objects will make the same choice of the particular DataWriter that
/// is the owner. It is also required that the owner remains the same until there is a change in strength, liveliness, the owner misses
/// a deadline on the instance, a new DataWriter with higher strength modifies the instance, or another DataWriter with the same
/// strength that is deemed by the Service to be the new owner modifies the instance.
/// Exclusive ownership is on an instance-by-instance basis. That is, a subscriber can receive values written by a lower
/// strength DataWriter as long as they affect instances whose values have not been set by the higher-strength
/// DataWriter.
#[derive(Debug, PartialEq, Eq, PartialOrd, Clone, TypeSupport)]
#[dust_dds(extensibility = "appendable", nested)]
pub struct OwnershipStrengthQosPolicy {
    /// Ownership strength value
    pub value: i32,
}

impl OwnershipStrengthQosPolicy {
    /// Default constructor usable in const contexts
    pub const fn const_default() -> Self {
        Self { value: 0 }
    }
}

impl QosPolicy for OwnershipStrengthQosPolicy {
    fn name(&self) -> &str {
        OWNERSHIP_STRENGTH_QOS_POLICY_NAME
    }
}

impl Default for OwnershipStrengthQosPolicy {
    fn default() -> Self {
        Self::const_default()
    }
}

/// Enumeration representing the different types of Liveliness QoS policies.
#[derive(Debug, PartialEq, Eq, Clone, Copy, TypeSupport)]
pub enum LivelinessQosPolicyKind {
    /// Automatic liveliness
    Automatic,
    /// Manual by participant liveliness
    ManualByParticipant,
    /// Manual by topic liveliness
    ManualByTopic,
}

impl PartialOrd for LivelinessQosPolicyKind {
    fn partial_cmp(&self, other: &LivelinessQosPolicyKind) -> Option<Ordering> {
        match self {
            LivelinessQosPolicyKind::Automatic => match other {
                LivelinessQosPolicyKind::Automatic => Some(Ordering::Equal),
                LivelinessQosPolicyKind::ManualByParticipant => Some(Ordering::Less),
                LivelinessQosPolicyKind::ManualByTopic => Some(Ordering::Less),
            },
            LivelinessQosPolicyKind::ManualByParticipant => match other {
                LivelinessQosPolicyKind::Automatic => Some(Ordering::Greater),
                LivelinessQosPolicyKind::ManualByParticipant => Some(Ordering::Equal),
                LivelinessQosPolicyKind::ManualByTopic => Some(Ordering::Less),
            },
            LivelinessQosPolicyKind::ManualByTopic => match other {
                LivelinessQosPolicyKind::Automatic => Some(Ordering::Greater),
                LivelinessQosPolicyKind::ManualByParticipant => Some(Ordering::Greater),
                LivelinessQosPolicyKind::ManualByTopic => Some(Ordering::Equal),
            },
        }
    }
}

/// This policy controls the mechanism and parameters used by the Service to ensure that particular entities on the network are
/// still *alive*.
///
/// The liveliness can also affect the ownership of a particular instance, as determined by the [`OwnershipQosPolicy`].
/// This policy has several settings to support both data-objects that are updated periodically as well as those that are changed
/// sporadically. It also allows customizing for different application requirements in terms of the kinds of failures that will be
/// detected by the liveliness mechanism.
/// The [`LivelinessQosPolicyKind::Automatic`] liveliness setting is most appropriate for applications that only need to detect failures at the process level,
/// but not application-logic failures within a process. The Service takes responsibility for renewing the leases at the
/// required rates and thus, as long as the local process where a [`DomainParticipant`](crate::domain::domain_participant::DomainParticipant) is running and the link connecting it to remote
/// participants remains connected, the entities within the [`DomainParticipant`](crate::domain::domain_participant::DomainParticipant) will be considered alive. This requires the lowest
/// overhead.
/// The manual settings ([`LivelinessQosPolicyKind::ManualByParticipant`], [`LivelinessQosPolicyKind::ManualByTopic`]), require the application on the publishing
/// side to periodically assert the liveliness before the lease expires to indicate the corresponding Entity is still alive. The action
/// can be explicit by calling the `assert_liveliness()` operations, or implicit by writing some data.
/// The two possible manual settings control the granularity at which the application must assert liveliness.
/// The setting [`LivelinessQosPolicyKind::ManualByParticipant`] requires only that one Entity within the publisher is asserted to be alive to
/// deduce all other Entity objects within the same [`DomainParticipant`](crate::domain::domain_participant::DomainParticipant) are also alive.
/// The setting [`LivelinessQosPolicyKind::ManualByTopic`] requires that at least one instance within the [`DataWriter`](crate::publication::data_writer::DataWriter) is asserted.
/// The value offered is considered compatible with the value requested if and only if the inequality *offered kind >= requested kind* is true. For the purposes of this inequality, the values
/// of [`LivelinessQosPolicyKind`] kind are considered ordered such that *Automatic < ManualByParticipant < ManualByTopic*.
/// and the inequality *offered lease_duration <= requested lease_duration* is true.
/// Changes in liveliness must be detected by the Service with a time-granularity greater or equal to the [`LivelinessQosPolicy::lease_duration`]. This
/// ensures that the value of the LivelinessChangedStatus is updated at least once during each [`LivelinessQosPolicy::lease_duration`] and the related
/// Listeners and WaitSets are notified within a [`LivelinessQosPolicy::lease_duration`] from the time the liveliness changed.
#[derive(Debug, PartialEq, Eq, PartialOrd, Clone, TypeSupport)]
#[dust_dds(extensibility = "appendable", nested)]
pub struct LivelinessQosPolicy {
    /// Kind of liveliness QoS associated with this policy
    pub kind: LivelinessQosPolicyKind,
    /// Liveliness duration
    pub lease_duration: DurationKind,
}

impl LivelinessQosPolicy {
    /// Default constructor usable in const contexts
    pub const fn const_default() -> Self {
        Self {
            kind: LivelinessQosPolicyKind::Automatic,
            lease_duration: DurationKind::Infinite,
        }
    }
}

impl QosPolicy for LivelinessQosPolicy {
    fn name(&self) -> &str {
        LIVELINESS_QOS_POLICY_NAME
    }
}

impl Default for LivelinessQosPolicy {
    fn default() -> Self {
        Self::const_default()
    }
}

/// This policy allows a [`DataReader`](crate::subscription::data_reader::DataReader) to indicate that it does not necessarily want to
/// see all values of each instance published under the [`Topic`](crate::topic_definition::topic::Topic).
/// Rather, it wants to see at most one change every [`TimeBasedFilterQosPolicy::minimum_separation`] period.
///
/// The [`TimeBasedFilterQosPolicy`] applies to each instance separately, that is, the constraint is that the [`DataReader`](crate::subscription::data_reader::DataReader)
/// does not want to see more than one sample of each instance per [`TimeBasedFilterQosPolicy::minimum_separation`] period.
/// This setting allows a [`DataReader`](crate::subscription::data_reader::DataReader) to further decouple itself from the
/// [`DataWriter`](crate::publication::data_writer::DataWriter) objects. It can be used to protect applications
/// that are running on a heterogeneous network where some nodes are capable of generating data much faster than others can
/// consume it. It also accommodates the fact that for fast-changing data different subscribers may have different requirements as
/// to how frequently they need to be notified of the most current values.
/// The setting of a [`TimeBasedFilterQosPolicy`], that is, the selection of a  [`TimeBasedFilterQosPolicy::minimum_separation`] with a value greater
/// than zero is compatible with all settings of the [`HistoryQosPolicy`] and [`ReliabilityQosPolicy`].
/// The [`TimeBasedFilterQosPolicy`] specifies the samples that are of interest to the [`DataReader`](crate::subscription::data_reader::DataReader).
/// The [`HistoryQosPolicy`] and [`ReliabilityQosPolicy`] affect the behavior of the middleware with
/// respect to the samples that have been determined to be of interest to the [`DataReader`](crate::subscription::data_reader::DataReader),
/// that is, they apply after the [`TimeBasedFilterQosPolicy`] has been applied.
/// In the case where the reliability [`ReliabilityQosPolicyKind::Reliable`]  then in steady-state, defined as the situation where
/// the [`DataWriter`](crate::publication::data_writer::DataWriter) does not write new samples for a period *long* compared to
/// the [`TimeBasedFilterQosPolicy::minimum_separation`], the system should guarantee delivery the last sample to the [`DataReader`](crate::subscription::data_reader::DataReader).
/// The setting of the  [`TimeBasedFilterQosPolicy::minimum_separation`] minimum_separation must be consistent with the [`DeadlineQosPolicy::period`]. For these
/// two QoS policies to be consistent they must verify that *[`DeadlineQosPolicy::period`] >= [`TimeBasedFilterQosPolicy::minimum_separation`]*.
#[derive(Debug, PartialEq, Eq, Clone, TypeSupport)]
#[dust_dds(extensibility = "appendable", nested)]
pub struct TimeBasedFilterQosPolicy {
    /// Minimum separation between samples
    pub minimum_separation: DurationKind,
}

impl TimeBasedFilterQosPolicy {
    /// Default constructor usable in const contexts
    pub const fn const_default() -> Self {
        Self {
            minimum_separation: DurationKind::Finite(Duration::new(
                DURATION_ZERO_SEC,
                DURATION_ZERO_NSEC,
            )),
        }
    }
}

impl QosPolicy for TimeBasedFilterQosPolicy {
    fn name(&self) -> &str {
        TIMEBASEDFILTER_QOS_POLICY_NAME
    }
}

impl Default for TimeBasedFilterQosPolicy {
    fn default() -> Self {
        Self::const_default()
    }
}

/// This policy allows the introduction of a logical partition concept inside the 'physical' partition induced by a domain.
///
/// For a [`DataReader`](crate::subscription::data_reader::DataReader) to see the changes made to an instance by a [`DataWriter`](crate::publication::data_writer::DataWriter),
/// not only the [`Topic`](crate::topic_definition::topic::Topic) must match, but also they must share a common partition.
/// Each string in the list that defines this QoS policy defines a partition name. A partition name may
/// contain wildcards. Sharing a common partition means that one of the partition names matches.
/// Failure to match partitions is not considered an *incompatible* QoS and does not trigger any listeners nor conditions.
/// This policy is changeable. A change of this policy can potentially modify the *match* of existing [`DataReader`](crate::subscription::data_reader::DataReader)
/// and [`DataWriter`](crate::publication::data_writer::DataWriter) entities. It may establish new *matchs* that did not exist before, or break existing matchs.
/// Partition names can be regular expressions and include wildcards as defined by the POSIX fnmatch API (1003.2-1992
/// section B.6). Either [`Publisher`](crate::publication::publisher::Publisher) or [`Subscriber`](crate::subscription::subscriber::Subscriber)
/// may include regular expressions in partition names, but no two names that both
/// contain wildcards will ever be considered to match. This means that although regular expressions may be used both at
/// publisher as well as subscriber side, the service will not try to match two regular expressions (between publishers and
/// subscribers).
/// Partitions are different from creating Entity objects in different domains in several ways. First, entities belonging to different
/// domains are completely isolated from each other; there is no traffic, meta-traffic or any other way for an application or the
/// Service itself to see entities in a domain it does not belong to. Second, an Entity can only belong to one domain whereas an
/// Entity can be in multiple partitions. Finally, as far as the DDS Service is concerned, each unique data instance is identified by
/// the tuple (domainId, Topic, key). Therefore two Entity objects in different domains cannot refer to the same data instance. On
/// the other hand, the same data-instance can be made available (published) or requested (subscribed) on one or more partitions.
#[derive(Debug, PartialEq, Eq, Clone, TypeSupport)]
#[dust_dds(extensibility = "appendable", nested)]
pub struct PartitionQosPolicy {
    /// Name of the partition
    pub name: Vec<String>,
}

impl PartitionQosPolicy {
    /// Default constructor usable in const contexts
    pub const fn const_default() -> Self {
        Self { name: Vec::new() }
    }
}

impl QosPolicy for PartitionQosPolicy {
    fn name(&self) -> &str {
        PARTITION_QOS_POLICY_NAME
    }
}

impl Default for PartitionQosPolicy {
    fn default() -> Self {
        Self::const_default()
    }
}

/// Enumeration representing the different types of reliability QoS policies.
#[derive(Debug, PartialEq, Eq, Clone, Copy, TypeSupport)]
#[repr(i32)]
pub enum ReliabilityQosPolicyKind {
    /// Best-effort reliability.
    BestEffort = 1,
    /// Reliable reliability.
    Reliable = 2,
}

impl PartialOrd for ReliabilityQosPolicyKind {
    fn partial_cmp(&self, other: &ReliabilityQosPolicyKind) -> Option<Ordering> {
        match self {
            ReliabilityQosPolicyKind::BestEffort => match other {
                ReliabilityQosPolicyKind::BestEffort => Some(Ordering::Equal),
                ReliabilityQosPolicyKind::Reliable => Some(Ordering::Less),
            },
            ReliabilityQosPolicyKind::Reliable => match other {
                ReliabilityQosPolicyKind::BestEffort => Some(Ordering::Greater),
                ReliabilityQosPolicyKind::Reliable => Some(Ordering::Equal),
            },
        }
    }
}

/// This policy indicates the level of reliability requested by a [`DataReader`](crate::subscription::data_reader::DataReader)
/// or offered by a [`DataWriter`](crate::publication::data_writer::DataWriter).
///
/// These levels are ordered, [`ReliabilityQosPolicyKind::BestEffort`] being lower than [`ReliabilityQosPolicyKind::Reliable`].
/// A [`DataWriter`](crate::publication::data_writer::DataWriter) offering a level is implicitly offering all levels below.
/// The setting of this policy has a dependency on the setting of the [`ResourceLimitsQosPolicy`] policy.
/// In case the [`ReliabilityQosPolicyKind`] kind is set to [`ReliabilityQosPolicyKind::Reliable`] the write operation
/// on the [`DataWriter`](crate::publication::data_writer::DataWriter) may block if the modification would cause data to be lost or else
/// cause one of the limits in specified in the [`ResourceLimitsQosPolicy`] to be exceeded. Under these circumstances, the
///  [`ReliabilityQosPolicy::max_blocking_time`] configures the maximum duration the write operation may block.
/// If the [`ReliabilityQosPolicyKind`] kind is set to [`ReliabilityQosPolicyKind::Reliable`], data-samples originating from a
/// single [`DataWriter`](crate::publication::data_writer::DataWriter) cannot be made available
/// to the [`DataReader`](crate::subscription::data_reader::DataReader) if there are previous data-samples that have not been received
/// yet due to a communication error. In other words, the service will repair the error and re-transmit data-samples as needed
/// in order to re-construct a correct snapshot of the [`DataWriter`](crate::publication::data_writer::DataWriter) history before
/// it is accessible by the [`DataReader`](crate::subscription::data_reader::DataReader).
/// If the [`ReliabilityQosPolicyKind`] is set to [`ReliabilityQosPolicyKind::BestEffort`], the service will not re-transmit missing data samples.
/// However for data samples originating from any one [`DataWriter`](crate::publication::data_writer::DataWriter) the service will ensure
/// they are stored in the [`DataReader`](crate::subscription::data_reader::DataReader) history in the same
/// order they originated in the [`DataWriter`](crate::publication::data_writer::DataWriter).
/// In other words, the [`DataReader`](crate::subscription::data_reader::DataReader) may miss some data samples but it will never see the
/// value of a data-object change from a newer value to an older value.
/// The value offered is considered compatible with the value requested if and only if the inequality *offered kind >= requested
/// kind* is true. For the purposes of this inequality, the values of [`ReliabilityQosPolicyKind`] are considered ordered such
/// that *BestEffort < Reliable*.
#[derive(Debug, PartialEq, Eq, Clone, TypeSupport)]
#[dust_dds(extensibility = "appendable", nested)]
pub struct ReliabilityQosPolicy {
    /// Kind of reliability QoS
    pub kind: ReliabilityQosPolicyKind,
    /// Maximum blocking time to block. This only applies when kind is set to [`ReliabilityQosPolicyKind::Reliable`]
    pub max_blocking_time: DurationKind,
}

impl QosPolicy for ReliabilityQosPolicy {
    fn name(&self) -> &str {
        RELIABILITY_QOS_POLICY_NAME
    }
}

impl PartialOrd for ReliabilityQosPolicy {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        self.kind.partial_cmp(&other.kind)
    }
}

// default for Reliability is different for reader and writer, hence
// added here as constants
const DEFAULT_MAX_BLOCKING_TIME: Duration = Duration::new(0, 100_000_000);
pub(crate) const DEFAULT_RELIABILITY_QOS_POLICY_DATA_READER_AND_TOPICS: ReliabilityQosPolicy =
    ReliabilityQosPolicy {
        kind: ReliabilityQosPolicyKind::BestEffort,
        max_blocking_time: DurationKind::Finite(DEFAULT_MAX_BLOCKING_TIME),
    };
pub(crate) const DEFAULT_RELIABILITY_QOS_POLICY_DATA_WRITER: ReliabilityQosPolicy =
    ReliabilityQosPolicy {
        kind: ReliabilityQosPolicyKind::Reliable,
        max_blocking_time: DurationKind::Finite(DEFAULT_MAX_BLOCKING_TIME),
    };

/// Enumeration representing the different types of destination order QoS policies.
#[derive(Debug, PartialEq, Eq, Clone, Copy, TypeSupport)]
pub enum DestinationOrderQosPolicyKind {
    /// Ordered by reception timestamp.
    ByReceptionTimestamp,
    /// Ordered by source timestamp.
    BySourceTimestamp,
}

impl PartialOrd for DestinationOrderQosPolicyKind {
    fn partial_cmp(&self, other: &DestinationOrderQosPolicyKind) -> Option<Ordering> {
        match self {
            DestinationOrderQosPolicyKind::ByReceptionTimestamp => match other {
                DestinationOrderQosPolicyKind::ByReceptionTimestamp => Some(Ordering::Equal),
                DestinationOrderQosPolicyKind::BySourceTimestamp => Some(Ordering::Less),
            },
            DestinationOrderQosPolicyKind::BySourceTimestamp => match other {
                DestinationOrderQosPolicyKind::ByReceptionTimestamp => Some(Ordering::Greater),
                DestinationOrderQosPolicyKind::BySourceTimestamp => Some(Ordering::Equal),
            },
        }
    }
}

/// This policy controls how each subscriber resolves the final value of a data instance that is written by multiple [`DataWriter`](crate::publication::data_writer::DataWriter)
/// objects (which may be associated with different [`Publisher`](crate::publication::publisher::Publisher) objects) running on different nodes.
///
/// The setting [`DestinationOrderQosPolicyKind::ByReceptionTimestamp`] indicates that, assuming the [`OwnershipQosPolicy`] policy allows it, the latest received
/// value for the instance should be the one whose value is kept. This is the default value.
/// The setting [`DestinationOrderQosPolicyKind::BySourceTimestamp`] indicates that, assuming the [`OwnershipQosPolicy`] policy allows it, a timestamp placed at
/// the source should be used. This is the only setting that, in the case of concurrent same-strength [`DataWriter`](crate::publication::data_writer::DataWriter) objects updating the
/// same instance, ensures all subscribers will end up with the same final value for the instance. The mechanism to set the source
/// timestamp is middleware dependent.
/// The value offered is considered compatible with the value requested if and only if the inequality *offered kind >= requested
/// kind* is true. For the purposes of this inequality, the values of [`DestinationOrderQosPolicyKind`] kind are considered
/// ordered such that *DestinationOrderQosPolicyKind::ByReceptionTimestamp < DestinationOrderQosPolicyKind::BySourceTimestamp*.
#[derive(Debug, PartialEq, Eq, PartialOrd, Clone, TypeSupport)]
#[dust_dds(extensibility = "appendable", nested)]
pub struct DestinationOrderQosPolicy {
    /// Kind of destination order QoS associated with this policy.
    pub kind: DestinationOrderQosPolicyKind,
}

impl DestinationOrderQosPolicy {
    /// Default constructor usable in const contexts
    pub const fn const_default() -> Self {
        Self {
            kind: DestinationOrderQosPolicyKind::ByReceptionTimestamp,
        }
    }
}

impl QosPolicy for DestinationOrderQosPolicyKind {
    fn name(&self) -> &str {
        DESTINATIONORDER_QOS_POLICY_NAME
    }
}

impl Default for DestinationOrderQosPolicy {
    fn default() -> Self {
        Self::const_default()
    }
}

/// Enumeration representing the different types of history QoS policies.
#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub enum HistoryQosPolicyKind {
    /// Keep number of samples indicated by the associated value.
    KeepLast(u32),
    /// Keep all samples.
    KeepAll,
}

impl Type for HistoryQosPolicyKind {
    const TYPE: DynamicType<'static> = DynamicType {
        descriptor: &TypeDescriptor {
            kind: TypeKind::ENUM,
            name: "HistoryQosPolicyKind",
            base_type: None,
            discriminator_type: Some(i32::TYPE),
            bound: &[],
            element_type: None,
            key_element_type: None,
            extensibility_kind: ExtensibilityKind::Final,
            is_nested: false,
            is_autoid_hash: false,
        },
        member_list: &[],
    };
}

impl TypeSupport for HistoryQosPolicyKind {
    fn create_sample(src: &mut DynamicData<'static>) -> Option<Self> {
        let discriminant = src.get_int32_value(0).ok()?;

        match discriminant {
            0 => Some(Self::KeepLast(1)),
            1 => Some(Self::KeepAll),
            _ => None,
        }
    }

    fn create_dynamic_sample(self) -> DynamicData<'static> {
        let mut data = DynamicDataFactory::create_data(Self::TYPE);

        let discriminant = match self {
            HistoryQosPolicyKind::KeepLast(_) => 0,
            HistoryQosPolicyKind::KeepAll => 1,
        };
        data.set_int32_value(0, discriminant).unwrap();

        data
    }
}

/// This policy controls the behavior of the Service when the value of an instance changes before it is finally
/// communicated to some of its existing [`DataReader`](crate::subscription::data_reader::DataReader) entities.
///
/// If the kind is set to [`HistoryQosPolicyKind::KeepLast`], then the Service will only attempt to keep the latest values of the instance and
/// discard the older ones. In this case, the value of depth regulates the maximum number of values (up to and including
/// the most current one) the Service will maintain and deliver. The default (and most common setting) for depth is one,
/// indicating that only the most recent value should be delivered.
/// If the kind is set to [`HistoryQosPolicyKind::KeepAll`], then the Service will attempt to maintain and deliver all the values of the instance to
/// existing subscribers. The resources that the Service can use to keep this history are limited by the settings of the
/// [`ResourceLimitsQosPolicy`]. If the limit is reached, then the behavior of the Service will depend on the
/// [`ReliabilityQosPolicy`]. If the reliability kind is [`ReliabilityQosPolicyKind::BestEffort`], then the old values will be discarded. If reliability is
/// [`ReliabilityQosPolicyKind::Reliable`], then the Service will block the [`DataWriter`](crate::publication::data_writer::DataWriter) until it can deliver the necessary old values to all subscribers.
/// The setting of [`HistoryQosPolicy`] depth must be consistent with the [`ResourceLimitsQosPolicy::max_samples_per_instance`]. For these two
/// QoS to be consistent, they must verify that *depth <= max_samples_per_instance*.
#[derive(Debug, PartialEq, Eq, Clone)]
pub struct HistoryQosPolicy {
    /// Kind of history QoS associated with this policy.
    pub kind: HistoryQosPolicyKind,
}

impl HistoryQosPolicy {
    /// Default constructor usable in const contexts
    pub const fn const_default() -> Self {
        Self {
            kind: HistoryQosPolicyKind::KeepLast(1),
        }
    }
}

impl Type for HistoryQosPolicy {
    const TYPE: DynamicType<'static> = DynamicType {
        descriptor: &TypeDescriptor {
            kind: TypeKind::STRUCTURE,
            name: "HistoryQosPolicy",
            base_type: None,
            discriminator_type: None,
            bound: &[],
            element_type: None,
            key_element_type: None,
            extensibility_kind: ExtensibilityKind::Appendable,
            is_nested: true,
            is_autoid_hash: false,
        },
        member_list: &[
            DynamicTypeMember {
                descriptor: dust_dds::xtypes::dynamic_type::MemberDescriptor {
                    name: "kind",
                    id: 0,
                    r#type: HistoryQosPolicyKind::TYPE,
                    default_value: None,
                    index: 0u32,
                    try_construct_kind:
                        dust_dds::xtypes::dynamic_type::TryConstructKind::UseDefault,
                    label: &[],
                    is_key: false,
                    is_optional: false,
                    is_must_understand: true,
                    is_shared: false,
                    is_default_label: false,
                    is_external: false,
                },
            },
            DynamicTypeMember {
                descriptor: dust_dds::xtypes::dynamic_type::MemberDescriptor {
                    name: "depth",
                    id: 1,
                    r#type: i32::TYPE,
                    default_value: None,
                    index: 1u32,
                    try_construct_kind:
                        dust_dds::xtypes::dynamic_type::TryConstructKind::UseDefault,
                    label: &[],
                    is_key: false,
                    is_optional: false,
                    is_must_understand: true,
                    is_shared: false,
                    is_default_label: false,
                    is_external: false,
                },
            },
        ],
    };
}

impl TypeSupport for HistoryQosPolicy {
    fn create_sample(src: &mut DynamicData<'static>) -> Option<Self> {
        let mut kind = src.get_complex_value(0).cloned().ok()?;
        let kind = HistoryQosPolicyKind::create_sample(&mut kind)?;
        let depth = src.get_int32_value(1).ok()?;

        match kind {
            HistoryQosPolicyKind::KeepLast(_) => Some(Self {
                kind: HistoryQosPolicyKind::KeepLast(*depth as u32),
            }),
            HistoryQosPolicyKind::KeepAll => Some(Self {
                kind: HistoryQosPolicyKind::KeepAll,
            }),
        }
    }

    fn create_dynamic_sample(self) -> DynamicData<'static> {
        let mut data = DynamicDataFactory::create_data(Self::TYPE);

        let kind = self.kind.create_dynamic_sample();
        let depth = match self.kind {
            HistoryQosPolicyKind::KeepLast(depth) => depth as i32,
            HistoryQosPolicyKind::KeepAll => -1,
        };

        data.set_complex_value(0, kind).unwrap();
        data.set_int32_value(1, depth).unwrap();

        data
    }
}

impl QosPolicy for HistoryQosPolicy {
    fn name(&self) -> &str {
        HISTORY_QOS_POLICY_NAME
    }
}

impl Default for HistoryQosPolicy {
    fn default() -> Self {
        Self::const_default()
    }
}

/// This policy controls the resources that the Service can use in order to meet the requirements imposed by the application and
/// other QoS settings.
///
/// If the [`DataWriter`](crate::publication::data_writer::DataWriter) objects are communicating samples faster than they are ultimately
/// taken by the [`DataReader`](crate::subscription::data_reader::DataReader) objects, the
/// middleware will eventually hit against some of the QoS-imposed resource limits. Note that this may occur when just a single
/// [`DataReader`](crate::subscription::data_reader::DataReader) cannot keep up with its corresponding [`DataWriter`](crate::publication::data_writer::DataWriter).
/// The behavior in this case depends on the setting for the [`ReliabilityQosPolicy`].
/// If reliability is [`ReliabilityQosPolicyKind::BestEffort`] then the Service is allowed to drop samples. If the reliability is
/// [`ReliabilityQosPolicyKind::Reliable`], the Service will block the DataWriter or discard the sample at the
/// [`DataReader`](crate::subscription::data_reader::DataReader) in order not to lose existing samples.
/// The constant [`Length::Unlimited`] may be used to indicate the absence of a particular limit. For example setting
/// [`ResourceLimitsQosPolicy::max_samples_per_instance`] to [`Length::Unlimited`] will cause the middleware to not enforce
/// this particular limit.
/// The setting of [`ResourceLimitsQosPolicy::max_samples`] must be consistent with the [`ResourceLimitsQosPolicy::max_samples_per_instance`].
/// For these two values to be consistent they must verify that *max_samples >= max_samples_per_instance*.
/// The setting of [`ResourceLimitsQosPolicy::max_samples_per_instance`] must be consistent with the
/// [`HistoryQosPolicy`] depth. For these two QoS to be consistent, they must verify
/// that *HistoryQosPolicy depth <= [`ResourceLimitsQosPolicy::max_samples_per_instance`]*.
#[derive(Debug, PartialEq, Eq, Clone)]
pub struct ResourceLimitsQosPolicy {
    /// Maximum number of samples limit.
    pub max_samples: Length,
    /// Maximum number of instances limit.
    pub max_instances: Length,
    /// Maximum number of samples per instance limit.
    pub max_samples_per_instance: Length,
}

#[automatically_derived]
impl TypeSupport for ResourceLimitsQosPolicy {
    fn create_sample(src: &mut DynamicData) -> Option<Self> {
        Some(Self {
            max_samples: (*src.get_int32_value(0).ok()?).into(),
            max_instances: (*src.get_int32_value(1).ok()?).into(),
            max_samples_per_instance: (*src.get_int32_value(2).ok()?).into(),
        })
    }
    fn create_dynamic_sample(self) -> DynamicData<'static> {
        let mut data = DynamicDataFactory::create_data(Self::TYPE);
        data.set_int32_value(0, self.max_samples.into())
            .expect("Must succeed");
        data.set_int32_value(1, self.max_instances.into())
            .expect("Must succeed");
        data.set_int32_value(2, self.max_samples_per_instance.into())
            .expect("Must succeed");
        data
    }
}

impl Type for ResourceLimitsQosPolicy {
    const TYPE: DynamicType<'static> = DynamicType {
        descriptor: &TypeDescriptor {
            kind: TypeKind::STRUCTURE,
            name: "ResourceLimitsQosPolicy",
            base_type: None,
            discriminator_type: None,
            bound: &[],
            element_type: None,
            key_element_type: None,
            extensibility_kind: ExtensibilityKind::Appendable,
            is_nested: true,
            is_autoid_hash: false,
        },
        member_list: &[
            dust_dds::xtypes::dynamic_type::DynamicTypeMember {
                descriptor: dust_dds::xtypes::dynamic_type::MemberDescriptor {
                    name: "max_samples",
                    id: 0,
                    r#type: <i32 as Type>::TYPE,
                    default_value: None,
                    index: 0_u32,
                    try_construct_kind: dust_dds::xtypes::dynamic_type::TryConstructKind::Discard,
                    label: &[],
                    is_key: false,
                    is_optional: false,
                    is_must_understand: false,
                    is_shared: false,
                    is_default_label: false,
                    is_external: false,
                },
            },
            dust_dds::xtypes::dynamic_type::DynamicTypeMember {
                descriptor: dust_dds::xtypes::dynamic_type::MemberDescriptor {
                    name: "max_instances",
                    id: 1,
                    r#type: <i32 as Type>::TYPE,
                    default_value: None,
                    index: 1_u32,
                    try_construct_kind: dust_dds::xtypes::dynamic_type::TryConstructKind::Discard,
                    label: &[],
                    is_key: false,
                    is_optional: false,
                    is_must_understand: false,
                    is_shared: false,
                    is_default_label: false,
                    is_external: false,
                },
            },
            dust_dds::xtypes::dynamic_type::DynamicTypeMember {
                descriptor: dust_dds::xtypes::dynamic_type::MemberDescriptor {
                    name: "max_samples_per_instance",
                    id: 2,
                    r#type: <i32 as Type>::TYPE,
                    default_value: None,
                    index: 2_u32,
                    try_construct_kind: dust_dds::xtypes::dynamic_type::TryConstructKind::Discard,
                    label: &[],
                    is_key: false,
                    is_optional: false,
                    is_must_understand: false,
                    is_shared: false,
                    is_default_label: false,
                    is_external: false,
                },
            },
        ],
    };
}

impl ResourceLimitsQosPolicy {
    /// Default constructor usable in const contexts
    pub const fn const_default() -> Self {
        Self {
            max_samples: Length::Unlimited,
            max_instances: Length::Unlimited,
            max_samples_per_instance: Length::Unlimited,
        }
    }
}

impl QosPolicy for ResourceLimitsQosPolicy {
    fn name(&self) -> &str {
        RESOURCELIMITS_QOS_POLICY_NAME
    }
}

impl Default for ResourceLimitsQosPolicy {
    fn default() -> Self {
        Self::const_default()
    }
}

/// This policy controls the behavior of the Entity as a factory for other entities.
///
/// This policy concerns only DomainParticipant (as factory for Publisher, Subscriber, and Topic), Publisher (as factory for
/// DataWriter), and Subscriber (as factory for DataReader).
/// This policy is mutable. A change in the policy affects only the entities created after the change; not the previously created
/// entities.
/// The setting of `autoenable_created_entities` to [`true`] indicates that the factory `create_<entity>` operation will automatically
/// invoke the enable operation each time a new Entity is created. Therefore, the Entity returned by `create_...` will already
/// be enabled. A setting of [`false`] indicates that the Entity will not be automatically enabled. The application will need to enable
/// it explicitly by means of the `enable()` operation.
/// The default setting of `autoenable_created_entities` is [`true`] which means that, by default, it is not necessary to explicitly call `enable()`
/// on newly created entities.
#[derive(Debug, PartialEq, Eq, Clone)]
pub struct EntityFactoryQosPolicy {
    /// Value of auto-enable created entities.
    pub autoenable_created_entities: bool,
}

impl EntityFactoryQosPolicy {
    /// Default constructor usable in const contexts
    pub const fn const_default() -> Self {
        Self {
            autoenable_created_entities: true,
        }
    }
}

impl QosPolicy for EntityFactoryQosPolicy {
    fn name(&self) -> &str {
        ENTITYFACTORY_QOS_POLICY_NAME
    }
}

impl Default for EntityFactoryQosPolicy {
    fn default() -> Self {
        Self::const_default()
    }
}

/// This policy controls the behavior of the [`DataWriter`](crate::publication::data_writer::DataWriter) with regards to the lifecycle
/// of the data-instances it manages, that is, the data-instances that have been either explicitly registered with the
/// [`DataWriter::register`](crate::publication::data_writer::DataWriter) or implicitly by using [`DataWriter::write`](crate::publication::data_writer::DataWriter)
///
/// The [`WriterDataLifecycleQosPolicy::autodispose_unregistered_instances`] flag controls the behavior when the
/// DataWriter unregisters an instance by means of the [`DataWriter::unregister_instance`](crate::publication::data_writer::DataWriter) operations:
/// - The setting [`WriterDataLifecycleQosPolicy::autodispose_unregistered_instances`] = [`true`] causes the [`DataWriter::unregister_instance`](crate::publication::data_writer::DataWriter)
///   to dispose the instance each time it is unregistered.
///   The behavior is identical to explicitly calling one of the [`DataWriter::dispose`](crate::publication::data_writer::DataWriter) operations on the
///   instance prior to calling the unregister operation.
/// - The setting [`WriterDataLifecycleQosPolicy::autodispose_unregistered_instances`] = [`false`]  will not cause this automatic disposition upon unregistering.
///   The application can still call one of the dispose operations prior to unregistering the instance and accomplish the same effect.
///
/// Note that the deletion of a [`DataWriter`](crate::publication::data_writer::DataWriter) automatically unregisters all data-instances it manages.
/// Therefore the setting of the [`WriterDataLifecycleQosPolicy::autodispose_unregistered_instances`] flag will determine whether instances are ultimately disposed when the
/// [`DataWriter`](crate::publication::data_writer::DataWriter) is deleted.
#[derive(Debug, PartialEq, Eq, Clone)]
pub struct WriterDataLifecycleQosPolicy {
    /// Value of auto-dispose unregistered instances.
    pub autodispose_unregistered_instances: bool,
}

impl WriterDataLifecycleQosPolicy {
    /// Default constructor usable in const contexts
    pub const fn const_default() -> Self {
        Self {
            autodispose_unregistered_instances: true,
        }
    }
}

impl QosPolicy for WriterDataLifecycleQosPolicy {
    fn name(&self) -> &str {
        WRITERDATALIFECYCLE_QOS_POLICY_NAME
    }
}

impl Default for WriterDataLifecycleQosPolicy {
    fn default() -> Self {
        Self::const_default()
    }
}

/// This policy controls the behavior of the [`DataReader`](crate::subscription::data_reader::DataReader) with regards to the lifecycle of the data-instances it manages, that is, the
/// data-instances that have been received and for which the [`DataReader`](crate::subscription::data_reader::DataReader) maintains some internal resources.
///
/// The [`DataReader`](crate::subscription::data_reader::DataReader) internally maintains the samples that have not been taken by the application, subject to the constraints
/// imposed by other QoS policies such as [`HistoryQosPolicy`] and [`ResourceLimitsQosPolicy`].
/// The [`DataReader`](crate::subscription::data_reader::DataReader) also maintains information regarding the identity, view_state and instance_state
/// of data-instances even after all samples have been 'taken.' This is needed to properly compute the states when future samples arrive.
/// Under normal circumstances the [`DataReader`](crate::subscription::data_reader::DataReader) can only reclaim all resources for instances for which there are no writers and for
/// which all samples have been 'taken'. The last sample the [`DataReader`](crate::subscription::data_reader::DataReader) will have taken for that instance will have an
/// `instance_state` of either [`InstanceStateKind::NotAliveNoWriters`](crate::subscription::sample_info::InstanceStateKind) or
/// [`InstanceStateKind::NotAliveDisposed`](crate::subscription::sample_info::InstanceStateKind) depending on whether the last writer
/// that had ownership of the instance disposed it or not.  In the absence of the [`ReaderDataLifecycleQosPolicy`] this behavior could cause problems if the
/// application *forgets* to 'take' those samples. The 'untaken' samples will prevent the [`DataReader`](crate::subscription::data_reader::DataReader) from reclaiming the
/// resources and they would remain in the [`DataReader`](crate::subscription::data_reader::DataReader) indefinitely.
/// The [`ReaderDataLifecycleQosPolicy::autopurge_nowriter_samples_delay`] defines the maximum duration for which the [`DataReader`](crate::subscription::data_reader::DataReader) will maintain information
/// regarding an instance once its `instance_state` becomes [`InstanceStateKind::NotAliveNoWriters`](crate::subscription::sample_info::InstanceStateKind). After this time elapses, the [`DataReader`](crate::subscription::data_reader::DataReader)
/// will purge all internal information regarding the instance, any untaken samples will also be lost.
/// The [`ReaderDataLifecycleQosPolicy::autopurge_disposed_samples_delay`] defines the maximum duration for which the [`DataReader`](crate::subscription::data_reader::DataReader) will maintain samples for
/// an instance once its `instance_state` becomes [`InstanceStateKind::NotAliveDisposed`](crate::subscription::sample_info::InstanceStateKind). After this time elapses, the [`DataReader`](crate::subscription::data_reader::DataReader) will purge all
/// samples for the instance.
#[derive(Debug, PartialEq, Eq, Clone)]
pub struct ReaderDataLifecycleQosPolicy {
    /// Time duration to auto purge samples with no writer.
    pub autopurge_nowriter_samples_delay: DurationKind,
    /// Time duration to auto purge disposed samples.
    pub autopurge_disposed_samples_delay: DurationKind,
}

impl ReaderDataLifecycleQosPolicy {
    /// Default constructor usable in const contexts
    pub const fn const_default() -> Self {
        Self {
            autopurge_nowriter_samples_delay: DurationKind::Infinite,
            autopurge_disposed_samples_delay: DurationKind::Infinite,
        }
    }
}

impl QosPolicy for ReaderDataLifecycleQosPolicy {
    fn name(&self) -> &str {
        READERDATALIFECYCLE_QOS_POLICY_NAME
    }
}

impl Default for ReaderDataLifecycleQosPolicy {
    fn default() -> Self {
        Self::const_default()
    }
}

impl From<&ReliabilityQosPolicy> for ReliabilityKind {
    fn from(value: &ReliabilityQosPolicy) -> Self {
        match value.kind {
            ReliabilityQosPolicyKind::BestEffort => ReliabilityKind::BestEffort,
            ReliabilityQosPolicyKind::Reliable => ReliabilityKind::Reliable,
        }
    }
}

impl From<&DurabilityQosPolicy> for DurabilityKind {
    fn from(value: &DurabilityQosPolicy) -> Self {
        match value.kind {
            DurabilityQosPolicyKind::Volatile => DurabilityKind::Volatile,
            DurabilityQosPolicyKind::TransientLocal => DurabilityKind::TransientLocal,
            DurabilityQosPolicyKind::Transient => DurabilityKind::Transient,
            DurabilityQosPolicyKind::Persistent => DurabilityKind::Persistent,
        }
    }
}

/*******  DDS X-TYPES Extension **********/

type DataRepresentationId = u16;
/// XCDR data representation
pub const XCDR_DATA_REPRESENTATION: DataRepresentationId = 0;
/// XML data representation
pub const XML_DATA_REPRESENTATION: DataRepresentationId = 1;
/// XCDR2 data representation
pub const XCDR2_DATA_REPRESENTATION: DataRepresentationId = 2;

type DataRepresentationIdSeq = Vec<DataRepresentationId>;

/// This policy is a DDS-XTypes extension and represents the standard data Representations available.
/// [`DataWriter`](crate::publication::data_writer::DataWriter) and [`DataReader`](crate::subscription::data_reader::DataReader) must be able to negotiate which data representation(s) to use.
#[derive(Debug, PartialEq, Eq, Clone, TypeSupport)]
#[dust_dds(extensibility = "appendable", nested)]
pub struct DataRepresentationQosPolicy {
    /// List of data representation values
    pub value: DataRepresentationIdSeq,
}

impl DataRepresentationQosPolicy {
    /// Default constructor usable in const contexts
    pub const fn const_default() -> Self {
        Self {
            value: DataRepresentationIdSeq::new(),
        }
    }
}

impl QosPolicy for DataRepresentationQosPolicy {
    fn name(&self) -> &str {
        DATA_REPRESENTATION_QOS_POLICY_NAME
    }
}

impl Default for DataRepresentationQosPolicy {
    fn default() -> Self {
        Self::const_default()
    }
}

/// Enumeration representing the different types of Type Consistency QoS policies.
#[derive(Debug, PartialEq, Eq, Clone, Copy, TypeSupport)]
#[dust_dds(bit_bound = "16")]
pub enum TypeConsistencyKind {
    /// Disallow type coercion for matched types
    DisallowTypeCoercion,
    /// Allow type coercion for matched types
    AllowTypeCoercion,
}

/// This policy defines the rules for determining whether the data type used by a [`DataWriter`](crate::publication::data_writer::DataWriter)
/// is consistent with the type used by a [`DataReader`](crate::subscription::data_reader::DataReader).
///
/// This policy is a DDS-XTypes extension. It allows a [`DataReader`](crate::subscription::data_reader::DataReader) to define the rules
/// for type consistency when matching with a [`DataWriter`](crate::publication::data_writer::DataWriter). It provides a mechanism
/// to enforce stricter static type checking than the default assignability rules, ensuring that the data received by a consumer
/// is compatible with its expected type.
#[derive(Debug, PartialEq, Eq, Clone, TypeSupport)]
#[dust_dds(extensibility = "appendable", nested)]
pub struct TypeConsistencyEnforcementQosPolicy {
    /// Type consistency kind that determines the coercion strategy (whether to allow type coercion or not).
    pub kind: TypeConsistencyKind,
    /// Controls whether sequence bounds are ignored when determining type compatibility.
    /// If true, sequences with different bounds are considered consistent.
    pub ignore_sequence_bounds: bool,
    /// Controls whether string bounds are ignored when determining type compatibility.
    /// If true, strings with different bounds are considered consistent.
    pub ignore_string_bounds: bool,
    /// Controls whether member names are ignored when determining type compatibility.
    /// If true, compatibility is determined solely by member IDs rather than names.
    pub ignore_member_names: bool,
    /// Controls whether type widening is prohibited.
    /// If true, prevents the reader from accepting a "wider" type (a type with more members) than expected.
    pub prevent_type_widening: bool,
    /// Controls whether explicit type validation is required.
    /// If true, requires the explicit type information (typically via `TypeObject`) to be available to complete the matching process.
    pub force_type_validation: bool,
}

impl TypeConsistencyEnforcementQosPolicy {
    /// Default constructor usable in const contexts
    pub const fn const_default() -> Self {
        Self {
            kind: TypeConsistencyKind::AllowTypeCoercion,
            ignore_sequence_bounds: true,
            ignore_string_bounds: true,
            ignore_member_names: false,
            prevent_type_widening: false,
            force_type_validation: false,
        }
    }
}

impl QosPolicy for TypeConsistencyEnforcementQosPolicy {
    fn name(&self) -> &str {
        TYPE_CONSISTENCY_ENFORCEMENT_QOS_POLICY_NAME
    }
}
impl Default for TypeConsistencyEnforcementQosPolicy {
    fn default() -> Self {
        Self::const_default()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn durability_qos_policy_kind_ordering() {
        assert!(DurabilityQosPolicyKind::Volatile < DurabilityQosPolicyKind::TransientLocal);

        assert!(DurabilityQosPolicyKind::Volatile == DurabilityQosPolicyKind::Volatile);
        assert!(DurabilityQosPolicyKind::Volatile < DurabilityQosPolicyKind::TransientLocal);

        assert!(DurabilityQosPolicyKind::TransientLocal > DurabilityQosPolicyKind::Volatile);
        assert!(DurabilityQosPolicyKind::TransientLocal == DurabilityQosPolicyKind::TransientLocal);
    }

    #[test]
    fn presentation_qos_policy_access_scope_kind_ordering() {
        assert!(
            PresentationQosPolicyAccessScopeKind::Instance
                < PresentationQosPolicyAccessScopeKind::Topic
        );

        assert!(
            PresentationQosPolicyAccessScopeKind::Instance
                == PresentationQosPolicyAccessScopeKind::Instance
        );
        assert!(
            PresentationQosPolicyAccessScopeKind::Instance
                < PresentationQosPolicyAccessScopeKind::Topic
        );

        assert!(
            PresentationQosPolicyAccessScopeKind::Topic
                > PresentationQosPolicyAccessScopeKind::Instance
        );
        assert!(
            PresentationQosPolicyAccessScopeKind::Topic
                == PresentationQosPolicyAccessScopeKind::Topic
        );
    }

    #[test]
    fn liveliness_qos_policy_kind_ordering() {
        assert!(LivelinessQosPolicyKind::Automatic < LivelinessQosPolicyKind::ManualByParticipant);
        assert!(
            LivelinessQosPolicyKind::ManualByParticipant < LivelinessQosPolicyKind::ManualByTopic
        );

        assert!(LivelinessQosPolicyKind::Automatic == LivelinessQosPolicyKind::Automatic);
        assert!(LivelinessQosPolicyKind::Automatic < LivelinessQosPolicyKind::ManualByParticipant);
        assert!(LivelinessQosPolicyKind::Automatic < LivelinessQosPolicyKind::ManualByTopic);

        assert!(LivelinessQosPolicyKind::ManualByParticipant > LivelinessQosPolicyKind::Automatic);
        assert!(
            LivelinessQosPolicyKind::ManualByParticipant
                == LivelinessQosPolicyKind::ManualByParticipant
        );
        assert!(
            LivelinessQosPolicyKind::ManualByParticipant < LivelinessQosPolicyKind::ManualByTopic
        );

        assert!(LivelinessQosPolicyKind::ManualByTopic > LivelinessQosPolicyKind::Automatic);
        assert!(
            LivelinessQosPolicyKind::ManualByTopic > LivelinessQosPolicyKind::ManualByParticipant
        );
        assert!(LivelinessQosPolicyKind::ManualByTopic == LivelinessQosPolicyKind::ManualByTopic);
    }

    #[test]
    fn reliability_qos_policy_kind_ordering() {
        assert!(ReliabilityQosPolicyKind::BestEffort < ReliabilityQosPolicyKind::Reliable);

        assert!(ReliabilityQosPolicyKind::BestEffort == ReliabilityQosPolicyKind::BestEffort);
        assert!(ReliabilityQosPolicyKind::BestEffort < ReliabilityQosPolicyKind::Reliable);

        assert!(ReliabilityQosPolicyKind::Reliable > ReliabilityQosPolicyKind::BestEffort);
        assert!(ReliabilityQosPolicyKind::Reliable == ReliabilityQosPolicyKind::Reliable);
    }

    #[test]
    fn destination_order_qos_policy_kind_ordering() {
        assert!(
            DestinationOrderQosPolicyKind::ByReceptionTimestamp
                < DestinationOrderQosPolicyKind::BySourceTimestamp
        );

        assert!(
            DestinationOrderQosPolicyKind::ByReceptionTimestamp
                == DestinationOrderQosPolicyKind::ByReceptionTimestamp
        );
        assert!(
            DestinationOrderQosPolicyKind::ByReceptionTimestamp
                < DestinationOrderQosPolicyKind::BySourceTimestamp
        );

        assert!(
            DestinationOrderQosPolicyKind::BySourceTimestamp
                > DestinationOrderQosPolicyKind::ByReceptionTimestamp
        );
        assert!(
            DestinationOrderQosPolicyKind::BySourceTimestamp
                == DestinationOrderQosPolicyKind::BySourceTimestamp
        );
    }

    #[test]
    fn length_ordering() {
        assert!(Length::Unlimited > Length::Limited(10));
        assert!(Length::Unlimited == Length::Unlimited);
        assert!(Length::Limited(10) < Length::Unlimited);
        assert!(Length::Limited(10) < Length::Limited(20));
        assert!(Length::Limited(20) > Length::Limited(10));
    }

    #[test]
    fn length_usize_ordering() {
        assert!(Length::Unlimited > 10usize);
        assert!(10usize < Length::Unlimited);
        assert!(Length::Limited(20) > 10usize);
        assert!(10usize < Length::Limited(20));
        assert!(Length::Limited(10) == 10usize);
        assert!(10usize == Length::Limited(10));
    }
}
