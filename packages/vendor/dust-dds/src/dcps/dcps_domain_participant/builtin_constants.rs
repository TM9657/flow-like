use crate::{
    infrastructure::{
        qos::{DataReaderQos, DataWriterQos},
        qos_policy::{
            DataRepresentationQosPolicy, DeadlineQosPolicy, DestinationOrderQosPolicy,
            DurabilityQosPolicy, DurabilityQosPolicyKind, HistoryQosPolicy, HistoryQosPolicyKind,
            LatencyBudgetQosPolicy, LifespanQosPolicy, LivelinessQosPolicy, OwnershipQosPolicy,
            OwnershipStrengthQosPolicy, ReaderDataLifecycleQosPolicy, ReliabilityQosPolicy,
            ReliabilityQosPolicyKind, ResourceLimitsQosPolicy, TimeBasedFilterQosPolicy,
            TransportPriorityQosPolicy, TypeConsistencyEnforcementQosPolicy, UserDataQosPolicy,
            WriterDataLifecycleQosPolicy,
        },
        time::{Duration, DurationKind},
    },
    transport::types::{
        BUILT_IN_READER_GROUP, BUILT_IN_READER_NO_KEY, BUILT_IN_READER_WITH_KEY,
        BUILT_IN_WRITER_GROUP, BUILT_IN_WRITER_NO_KEY, BUILT_IN_WRITER_WITH_KEY, EntityId,
    },
};

pub const ENTITYID_BUILTIN_SUBSCRIBER: EntityId = EntityId::new([0, 0, 0], BUILT_IN_READER_GROUP);
pub const _ENTITYID_BUILTIN_PUBLISHER: EntityId = EntityId::new([0, 0, 0], BUILT_IN_WRITER_GROUP);

pub const ENTITYID_SPDP_BUILTIN_PARTICIPANT_WRITER: EntityId =
    EntityId::new([0x00, 0x01, 0x00], BUILT_IN_WRITER_WITH_KEY);

pub const ENTITYID_SPDP_BUILTIN_PARTICIPANT_READER: EntityId =
    EntityId::new([0x00, 0x01, 0x00], BUILT_IN_READER_WITH_KEY);

pub const ENTITYID_SEDP_BUILTIN_TOPICS_ANNOUNCER: EntityId =
    EntityId::new([0, 0, 0x02], BUILT_IN_WRITER_WITH_KEY);

pub const ENTITYID_SEDP_BUILTIN_TOPICS_DETECTOR: EntityId =
    EntityId::new([0, 0, 0x02], BUILT_IN_READER_WITH_KEY);

pub const ENTITYID_SEDP_BUILTIN_PUBLICATIONS_ANNOUNCER: EntityId =
    EntityId::new([0, 0, 0x03], BUILT_IN_WRITER_WITH_KEY);

pub const ENTITYID_SEDP_BUILTIN_PUBLICATIONS_DETECTOR: EntityId =
    EntityId::new([0, 0, 0x03], BUILT_IN_READER_WITH_KEY);

pub const ENTITYID_SEDP_BUILTIN_SUBSCRIPTIONS_ANNOUNCER: EntityId =
    EntityId::new([0, 0, 0x04], BUILT_IN_WRITER_WITH_KEY);

pub const ENTITYID_SEDP_BUILTIN_SUBSCRIPTIONS_DETECTOR: EntityId =
    EntityId::new([0, 0, 0x04], BUILT_IN_READER_WITH_KEY);

// XTypes Table 61 – Built-in Endpoints added by the XTYPES specification

pub const ENTITYID_TL_SVC_REQ_WRITER: EntityId =
    EntityId::new([0x00, 0x03, 0x00], BUILT_IN_WRITER_NO_KEY);

pub const ENTITYID_TL_SVC_REQ_READER: EntityId =
    EntityId::new([0x00, 0x03, 0x00], BUILT_IN_READER_NO_KEY);

pub const ENTITYID_TL_SVC_REPLY_WRITER: EntityId =
    EntityId::new([0x00, 0x03, 0x01], BUILT_IN_WRITER_NO_KEY);

pub const ENTITYID_TL_SVC_REPLY_READER: EntityId =
    EntityId::new([0x00, 0x03, 0x01], BUILT_IN_READER_NO_KEY);

pub const TYPE_LOOKUP_REQUEST_TOPIC_NAME: &str = "TypeLookupRequest";
pub const TYPE_LOOKUP_REPLY_TOPIC_NAME: &str = "TypeLookupReply";

pub const SPDP_READER_QOS: DataReaderQos = DataReaderQos {
    durability: DurabilityQosPolicy {
        kind: DurabilityQosPolicyKind::TransientLocal,
    },
    history: HistoryQosPolicy {
        kind: HistoryQosPolicyKind::KeepLast(1),
    },
    reliability: ReliabilityQosPolicy {
        kind: ReliabilityQosPolicyKind::BestEffort,
        max_blocking_time: DurationKind::Finite(Duration::new(0, 0)),
    },
    deadline: DeadlineQosPolicy::const_default(),
    latency_budget: LatencyBudgetQosPolicy::const_default(),
    liveliness: LivelinessQosPolicy::const_default(),
    destination_order: DestinationOrderQosPolicy::const_default(),
    resource_limits: ResourceLimitsQosPolicy::const_default(),
    user_data: UserDataQosPolicy::const_default(),
    ownership: OwnershipQosPolicy::const_default(),
    time_based_filter: TimeBasedFilterQosPolicy::const_default(),
    reader_data_lifecycle: ReaderDataLifecycleQosPolicy::const_default(),
    representation: DataRepresentationQosPolicy::const_default(),
    type_consistency: TypeConsistencyEnforcementQosPolicy::const_default(),
};

pub const SEDP_DATA_READER_QOS: DataReaderQos = DataReaderQos {
    durability: DurabilityQosPolicy {
        kind: DurabilityQosPolicyKind::TransientLocal,
    },
    history: HistoryQosPolicy {
        kind: HistoryQosPolicyKind::KeepLast(1),
    },
    reliability: ReliabilityQosPolicy {
        kind: ReliabilityQosPolicyKind::Reliable,
        max_blocking_time: DurationKind::Finite(Duration::new(0, 0)),
    },
    deadline: DeadlineQosPolicy::const_default(),
    latency_budget: LatencyBudgetQosPolicy::const_default(),
    liveliness: LivelinessQosPolicy::const_default(),
    destination_order: DestinationOrderQosPolicy::const_default(),
    resource_limits: ResourceLimitsQosPolicy::const_default(),
    user_data: UserDataQosPolicy::const_default(),
    ownership: OwnershipQosPolicy::const_default(),
    time_based_filter: TimeBasedFilterQosPolicy::const_default(),
    reader_data_lifecycle: ReaderDataLifecycleQosPolicy::const_default(),
    representation: DataRepresentationQosPolicy::const_default(),
    type_consistency: TypeConsistencyEnforcementQosPolicy::const_default(),
};

// DDS RPC default QoS as specified in DDS-RPC standard 7.10.2 Default QoS
pub const TYPE_LOOKUP_READER_QOS: DataReaderQos = DataReaderQos {
    durability: DurabilityQosPolicy {
        kind: DurabilityQosPolicyKind::Volatile,
    },
    history: HistoryQosPolicy {
        kind: HistoryQosPolicyKind::KeepAll,
    },
    reliability: ReliabilityQosPolicy {
        kind: ReliabilityQosPolicyKind::Reliable,
        max_blocking_time: DurationKind::Finite(Duration::new(0, 0)),
    },
    deadline: DeadlineQosPolicy::const_default(),
    latency_budget: LatencyBudgetQosPolicy::const_default(),
    liveliness: LivelinessQosPolicy::const_default(),
    destination_order: DestinationOrderQosPolicy::const_default(),
    resource_limits: ResourceLimitsQosPolicy::const_default(),
    user_data: UserDataQosPolicy::const_default(),
    ownership: OwnershipQosPolicy::const_default(),
    time_based_filter: TimeBasedFilterQosPolicy::const_default(),
    reader_data_lifecycle: ReaderDataLifecycleQosPolicy::const_default(),
    representation: DataRepresentationQosPolicy::const_default(),
    type_consistency: TypeConsistencyEnforcementQosPolicy::const_default(),
};

pub const TYPE_LOOKUP_WRITER_QOS: DataWriterQos = DataWriterQos {
    durability: DurabilityQosPolicy {
        kind: DurabilityQosPolicyKind::Volatile,
    },
    history: HistoryQosPolicy {
        kind: HistoryQosPolicyKind::KeepAll,
    },
    reliability: ReliabilityQosPolicy {
        kind: ReliabilityQosPolicyKind::Reliable,
        max_blocking_time: DurationKind::Finite(Duration::new(0, 0)),
    },
    deadline: DeadlineQosPolicy::const_default(),
    latency_budget: LatencyBudgetQosPolicy::const_default(),
    liveliness: LivelinessQosPolicy::const_default(),
    destination_order: DestinationOrderQosPolicy::const_default(),
    resource_limits: ResourceLimitsQosPolicy::const_default(),
    user_data: UserDataQosPolicy::const_default(),
    ownership: OwnershipQosPolicy::const_default(),
    ownership_strength: OwnershipStrengthQosPolicy::const_default(),
    lifespan: LifespanQosPolicy::const_default(),
    transport_priority: TransportPriorityQosPolicy::const_default(),
    writer_data_lifecycle: WriterDataLifecycleQosPolicy::const_default(),
    representation: DataRepresentationQosPolicy::const_default(),
};
