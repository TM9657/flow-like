use alloc::{boxed::Box, string::String, vec::Vec};

use crate::{
    builtin_topics::{
        ParticipantBuiltinTopicData, PublicationBuiltinTopicData, SubscriptionBuiltinTopicData,
        TopicBuiltinTopicData,
    },
    dcps::{
        channels::{notification::NotificationSender, oneshot::OneshotSender},
        listeners::{
            data_reader_listener::DcpsDataReaderListener,
            data_writer_listener::DcpsDataWriterListener,
            domain_participant_listener::DcpsDomainParticipantListener,
            publisher_listener::DcpsPublisherListener, subscriber_listener::DcpsSubscriberListener,
            topic_listener::DcpsTopicListener,
        },
        status_condition::StatusConditionEntity,
        status_mask::StatusMask,
    },
    infrastructure::{
        domain::DomainId,
        error::DdsResult,
        instance::InstanceHandle,
        qos::{
            DataReaderQos, DataWriterQos, DomainParticipantFactoryQos, DomainParticipantQos,
            PublisherQos, QosKind, SubscriberQos, TopicQos,
        },
        sample_info::{InstanceStateKind, SampleInfo, SampleStateKind, ViewStateKind},
        status::{
            InconsistentTopicStatus, OfferedDeadlineMissedStatus, PublicationMatchedStatus,
            SubscriptionMatchedStatus,
        },
        time::{Duration, Time},
    },
    transport::{interface::RtpsTransportParticipant, types::GuidPrefix},
    xtypes::dynamic_type::{DynamicData, DynamicType},
};

pub enum DcpsMail {
    ParticipantFactory(ParticipantFactoryMail),
    Participant(ParticipantServiceMail),
    Topic(TopicServiceMail),
    Publisher(PublisherServiceMail),
    Writer(WriterServiceMail),
    Subscriber(SubscriberServiceMail),
    Reader(ReaderServiceMail),
    StatusCondition(StatusConditionMail),
    Message(MessageServiceMail),
}

pub struct CreateParticipantMail {
    pub guid_prefix: GuidPrefix,
    pub domain_id: DomainId,
    pub qos: QosKind<DomainParticipantQos>,
    pub dcps_listener: Option<DcpsDomainParticipantListener>,
    pub listener_mask: StatusMask,
    pub transport_participant: RtpsTransportParticipant,
    pub domain_tag: String,
    pub participant_announcement_interval: core::time::Duration,
    pub enable_type_information: bool,
}

pub enum ParticipantFactoryMail {
    CreateParticipant(Box<CreateParticipantMail>),
    DeleteParticipant {
        participant_handle: InstanceHandle,
    },
    SetDefaultParticipantQos {
        qos: Box<QosKind<DomainParticipantQos>>,
    },
    GetDefaultParticipantQos,
    SetQos {
        qos: Box<QosKind<DomainParticipantFactoryQos>>,
    },
    GetQos,
    LookupParticipant {
        domain_id: DomainId,
    },
}

pub struct CreateTopicMail {
    pub participant_handle: InstanceHandle,
    pub topic_name: String,
    pub type_name: String,
    pub qos: QosKind<TopicQos>,
    pub dcps_listener: Option<DcpsTopicListener>,
    pub listener_mask: StatusMask,
    pub type_support: DynamicType<'static>,
}

pub struct CreateContentFilteredTopicMail {
    pub participant_handle: InstanceHandle,
    pub name: String,
    pub related_topic_name: String,
    pub filter_expression: String,
    pub expression_parameters: Vec<String>,
}

pub enum ParticipantServiceMail {
    CreateUserDefinedPublisher {
        participant_handle: InstanceHandle,
        qos: QosKind<PublisherQos>,
        dcps_listener: Option<DcpsPublisherListener>,
        listener_mask: StatusMask,
    },
    DeleteUserDefinedPublisher {
        participant_handle: InstanceHandle,
        parent_participant_handle: InstanceHandle,
        publisher_handle: InstanceHandle,
    },
    CreateUserDefinedSubscriber {
        participant_handle: InstanceHandle,
        qos: QosKind<SubscriberQos>,
        dcps_listener: Option<DcpsSubscriberListener>,
        listener_mask: StatusMask,
    },
    DeleteUserDefinedSubscriber {
        participant_handle: InstanceHandle,
        parent_participant_handle: InstanceHandle,
        subscriber_handle: InstanceHandle,
    },
    CreateTopic(Box<CreateTopicMail>),
    DeleteUserDefinedTopic {
        participant_handle: InstanceHandle,
        parent_participant_handle: InstanceHandle,
        topic_name: String,
    },
    CreateContentFilteredTopic(Box<CreateContentFilteredTopicMail>),
    DeleteContentFilteredTopic {
        participant_handle: InstanceHandle,
        name: String,
    },
    FindTopic {
        participant_handle: InstanceHandle,
        topic_name: String,
        type_support: DynamicType<'static>,
        timeout: Duration,
        reply_sender: OneshotSender<DdsResult<(InstanceHandle, String)>>,
    },
    LookupTopicdescription {
        participant_handle: InstanceHandle,
        topic_name: String,
    },
    IgnoreParticipant {
        participant_handle: InstanceHandle,
        handle: InstanceHandle,
    },
    IgnoreSubscription {
        participant_handle: InstanceHandle,
        handle: InstanceHandle,
    },
    IgnorePublication {
        participant_handle: InstanceHandle,
        handle: InstanceHandle,
    },
    DeleteContainedEntities {
        participant_handle: InstanceHandle,
    },
    SetDefaultPublisherQos {
        participant_handle: InstanceHandle,
        qos: Box<QosKind<PublisherQos>>,
    },
    GetDefaultPublisherQos {
        participant_handle: InstanceHandle,
    },
    SetDefaultSubscriberQos {
        participant_handle: InstanceHandle,
        qos: Box<QosKind<SubscriberQos>>,
    },
    GetDefaultSubscriberQos {
        participant_handle: InstanceHandle,
    },
    SetDefaultTopicQos {
        participant_handle: InstanceHandle,
        qos: Box<QosKind<TopicQos>>,
    },
    GetDefaultTopicQos {
        participant_handle: InstanceHandle,
    },
    GetDiscoveredParticipants {
        participant_handle: InstanceHandle,
    },
    GetDiscoveredParticipantData {
        participant_handle: InstanceHandle,
        discovered_participant_handle: InstanceHandle,
    },
    GetDiscoveredTopics {
        participant_handle: InstanceHandle,
    },
    GetDiscoveredTopicData {
        participant_handle: InstanceHandle,
        topic_handle: InstanceHandle,
    },
    GetCurrentTime {
        participant_handle: InstanceHandle,
    },
    SetQos {
        participant_handle: InstanceHandle,
        qos: Box<QosKind<DomainParticipantQos>>,
    },
    GetQos {
        participant_handle: InstanceHandle,
    },
    SetListener {
        participant_handle: InstanceHandle,
        dcps_listener: Option<DcpsDomainParticipantListener>,
        listener_mask: StatusMask,
    },
    Enable {
        participant_handle: InstanceHandle,
    },
}

pub enum TopicServiceMail {
    GetInconsistentTopicStatus {
        participant_handle: InstanceHandle,
        topic_name: String,
    },
    SetQos {
        participant_handle: InstanceHandle,
        topic_name: String,
        topic_qos: Box<QosKind<TopicQos>>,
    },
    GetQos {
        participant_handle: InstanceHandle,
        topic_name: String,
    },
    Enable {
        participant_handle: InstanceHandle,
        topic_name: String,
    },
    GetTypeSupport {
        participant_handle: InstanceHandle,
        topic_name: String,
    },
}

pub struct CreateDataWriterMail {
    pub participant_handle: InstanceHandle,
    pub publisher_handle: InstanceHandle,
    pub topic_name: String,
    pub qos: QosKind<DataWriterQos>,
    pub dcps_listener: Option<DcpsDataWriterListener>,
    pub listener_mask: StatusMask,
}

pub enum PublisherServiceMail {
    CreateDataWriter(Box<CreateDataWriterMail>),
    DeleteDataWriter {
        participant_handle: InstanceHandle,
        publisher_handle: InstanceHandle,
        datawriter_handle: InstanceHandle,
    },
    GetDefaultDataWriterQos {
        participant_handle: InstanceHandle,
        publisher_handle: InstanceHandle,
    },
    SetDefaultDataWriterQos {
        participant_handle: InstanceHandle,
        publisher_handle: InstanceHandle,
        qos: Box<QosKind<DataWriterQos>>,
    },
    GetPublisherQos {
        participant_handle: InstanceHandle,
        publisher_handle: InstanceHandle,
    },
    SetPublisherQos {
        participant_handle: InstanceHandle,
        publisher_handle: InstanceHandle,
        qos: Box<QosKind<PublisherQos>>,
    },
    SetPublisherListener {
        participant_handle: InstanceHandle,
        publisher_handle: InstanceHandle,
        dcps_listener: Option<DcpsPublisherListener>,
        listener_mask: StatusMask,
    },
}

pub struct CreateDataReaderMail {
    pub participant_handle: InstanceHandle,
    pub subscriber_handle: InstanceHandle,
    pub topic_name: String,
    pub qos: QosKind<DataReaderQos>,
    pub dcps_listener: Option<DcpsDataReaderListener>,
    pub listener_mask: StatusMask,
}

pub enum SubscriberServiceMail {
    CreateDataReader(Box<CreateDataReaderMail>),
    DeleteDataReader {
        participant_handle: InstanceHandle,
        subscriber_handle: InstanceHandle,
        datareader_handle: InstanceHandle,
    },
    LookupDataReader {
        participant_handle: InstanceHandle,
        subscriber_handle: InstanceHandle,
        topic_name: String,
    },
    SetDefaultDataReaderQos {
        participant_handle: InstanceHandle,
        subscriber_handle: InstanceHandle,
        qos: Box<QosKind<DataReaderQos>>,
    },
    GetDefaultDataReaderQos {
        participant_handle: InstanceHandle,
        subscriber_handle: InstanceHandle,
    },
    SetQos {
        participant_handle: InstanceHandle,
        subscriber_handle: InstanceHandle,
        qos: Box<QosKind<SubscriberQos>>,
    },
    GetSubscriberQos {
        participant_handle: InstanceHandle,
        subscriber_handle: InstanceHandle,
    },
    SetListener {
        participant_handle: InstanceHandle,
        subscriber_handle: InstanceHandle,
        dcps_listener: Option<DcpsSubscriberListener>,
        listener_mask: StatusMask,
    },
}

pub enum WriterServiceMail {
    SetListener {
        participant_handle: InstanceHandle,
        publisher_handle: InstanceHandle,
        data_writer_handle: InstanceHandle,
        dcps_listener: Option<DcpsDataWriterListener>,
        listener_mask: StatusMask,
    },
    GetDataWriterQos {
        participant_handle: InstanceHandle,
        publisher_handle: InstanceHandle,
        data_writer_handle: InstanceHandle,
    },
    GetMatchedSubscriptions {
        participant_handle: InstanceHandle,
        publisher_handle: InstanceHandle,
        data_writer_handle: InstanceHandle,
    },
    GetMatchedSubscriptionData {
        participant_handle: InstanceHandle,
        publisher_handle: InstanceHandle,
        data_writer_handle: InstanceHandle,
        subscription_handle: InstanceHandle,
    },
    GetPublicationMatchedStatus {
        participant_handle: InstanceHandle,
        publisher_handle: InstanceHandle,
        data_writer_handle: InstanceHandle,
    },
    RegisterInstance {
        participant_handle: InstanceHandle,
        publisher_handle: InstanceHandle,
        data_writer_handle: InstanceHandle,
        dynamic_data: DynamicData<'static>,
        timestamp: Option<Time>,
    },
    UnregisterInstance {
        participant_handle: InstanceHandle,
        publisher_handle: InstanceHandle,
        data_writer_handle: InstanceHandle,
        dynamic_data: DynamicData<'static>,
        timestamp: Option<Time>,
    },
    LookupInstance {
        participant_handle: InstanceHandle,
        publisher_handle: InstanceHandle,
        data_writer_handle: InstanceHandle,
        dynamic_data: DynamicData<'static>,
    },
    WriteWTimestamp {
        participant_handle: InstanceHandle,
        publisher_handle: InstanceHandle,
        data_writer_handle: InstanceHandle,
        dynamic_data: DynamicData<'static>,
        timestamp: Option<Time>,
        reply_sender: OneshotSender<DdsResult<()>>,
    },
    DisposeWTimestamp {
        participant_handle: InstanceHandle,
        publisher_handle: InstanceHandle,
        data_writer_handle: InstanceHandle,
        dynamic_data: DynamicData<'static>,
        timestamp: Option<Time>,
    },
    GetOfferedDeadlineMissedStatus {
        participant_handle: InstanceHandle,
        publisher_handle: InstanceHandle,
        data_writer_handle: InstanceHandle,
    },
    EnableDataWriter {
        participant_handle: InstanceHandle,
        publisher_handle: InstanceHandle,
        data_writer_handle: InstanceHandle,
    },
    SetDataWriterQos {
        participant_handle: InstanceHandle,
        publisher_handle: InstanceHandle,
        data_writer_handle: InstanceHandle,
        qos: Box<QosKind<DataWriterQos>>,
    },
}

pub struct ReadMail {
    pub participant_handle: InstanceHandle,
    pub subscriber_handle: InstanceHandle,
    pub data_reader_handle: InstanceHandle,
    pub max_samples: i32,
    pub sample_states: Vec<SampleStateKind>,
    pub view_states: Vec<ViewStateKind>,
    pub instance_states: Vec<InstanceStateKind>,
    pub specific_instance_handle: Option<InstanceHandle>,
}

pub struct ReadNextInstanceMail {
    pub participant_handle: InstanceHandle,
    pub subscriber_handle: InstanceHandle,
    pub data_reader_handle: InstanceHandle,
    pub max_samples: i32,
    pub previous_handle: Option<InstanceHandle>,
    pub sample_states: Vec<SampleStateKind>,
    pub view_states: Vec<ViewStateKind>,
    pub instance_states: Vec<InstanceStateKind>,
}

pub enum ReaderServiceMail {
    Enable {
        participant_handle: InstanceHandle,
        subscriber_handle: InstanceHandle,
        data_reader_handle: InstanceHandle,
    },
    Read(Box<ReadMail>),
    Take(Box<ReadMail>),
    ReadNextInstance(Box<ReadNextInstanceMail>),
    TakeNextInstance(Box<ReadNextInstanceMail>),
    GetSubscriptionMatchedStatus {
        participant_handle: InstanceHandle,
        subscriber_handle: InstanceHandle,
        data_reader_handle: InstanceHandle,
    },
    GetMatchedPublicationData {
        participant_handle: InstanceHandle,
        subscriber_handle: InstanceHandle,
        data_reader_handle: InstanceHandle,
        publication_handle: InstanceHandle,
    },
    GetMatchedPublications {
        participant_handle: InstanceHandle,
        subscriber_handle: InstanceHandle,
        data_reader_handle: InstanceHandle,
    },
    SetQos {
        participant_handle: InstanceHandle,
        subscriber_handle: InstanceHandle,
        data_reader_handle: InstanceHandle,
        qos: Box<QosKind<DataReaderQos>>,
    },
    GetQos {
        participant_handle: InstanceHandle,
        subscriber_handle: InstanceHandle,
        data_reader_handle: InstanceHandle,
    },
    SetListener {
        participant_handle: InstanceHandle,
        subscriber_handle: InstanceHandle,
        data_reader_handle: InstanceHandle,
        dcps_listener: Option<DcpsDataReaderListener>,
        listener_mask: StatusMask,
    },
}

pub enum StatusConditionMail {
    GetStatusConditionEnabledStatuses {
        entity: StatusConditionEntity,
    },
    SetStatusConditionEnabledStatuses {
        entity: StatusConditionEntity,
        status_mask: StatusMask,
    },
    GetStatusConditionTriggerValue {
        entity: StatusConditionEntity,
    },
    RegisterNotification {
        entity: StatusConditionEntity,
        notification_sender: NotificationSender,
    },
}

pub enum MessageServiceMail {
    NotifyAcknowledgments {
        participant_handle: InstanceHandle,
        publisher_handle: InstanceHandle,
        data_writer_handle: InstanceHandle,
        reply_sender: OneshotSender<DdsResult<()>>,
    },
    NotifyHistoricalData {
        participant_handle: InstanceHandle,
        subscriber_handle: InstanceHandle,
        data_reader_handle: InstanceHandle,
        reply_sender: OneshotSender<DdsResult<()>>,
    },
}

pub struct WireMail {
    pub participant_handle: InstanceHandle,
    pub data_message: Vec<u8>,
}

#[derive(Debug)]
pub enum DcpsReply {
    Ok(DdsResult<()>),
    InstanceHandle(DdsResult<InstanceHandle>),
    OptionInstanceHandle(DdsResult<Option<InstanceHandle>>),
    ParticipantQos(DdsResult<DomainParticipantQos>),
    PublisherQos(DdsResult<PublisherQos>),
    SubscriberQos(DdsResult<SubscriberQos>),
    TopicQos(DdsResult<Box<TopicQos>>),
    DataWriterQos(DdsResult<Box<DataWriterQos>>),
    DataReaderQos(DdsResult<Box<DataReaderQos>>),
    FactoryQos(DdsResult<DomainParticipantFactoryQos>),
    InstanceHandleList(DdsResult<Vec<InstanceHandle>>),
    ParticipantBuiltinTopicData(DdsResult<Box<ParticipantBuiltinTopicData>>),
    TopicBuiltinTopicData(DdsResult<Box<TopicBuiltinTopicData>>),
    PublicationBuiltinTopicData(DdsResult<Box<PublicationBuiltinTopicData>>),
    SubscriptionBuiltinTopicData(DdsResult<Box<SubscriptionBuiltinTopicData>>),
    InconsistentTopicStatus(DdsResult<InconsistentTopicStatus>),
    PublicationMatchedStatus(DdsResult<PublicationMatchedStatus>),
    SubscriptionMatchedStatus(DdsResult<SubscriptionMatchedStatus>),
    OfferedDeadlineMissedStatus(DdsResult<OfferedDeadlineMissedStatus>),
    Time(DdsResult<Time>),
    DynamicType(DdsResult<DynamicType<'static>>),
    TopicDescription(DdsResult<Option<String>>),
    StatusMask(DdsResult<StatusMask>),
    TriggerValue(DdsResult<bool>),
    Samples(DdsResult<Vec<(Option<DynamicData<'static>>, SampleInfo)>>),
}

impl DcpsReply {
    pub fn expect_ok(self) -> DdsResult<()> {
        match self {
            DcpsReply::Ok(res) => res,
            other => panic!("expected Ok reply, got {:?}", other),
        }
    }

    pub fn expect_instance_handle(self) -> DdsResult<InstanceHandle> {
        match self {
            DcpsReply::InstanceHandle(res) => res,
            other => panic!("expected InstanceHandle reply, got {:?}", other),
        }
    }

    pub fn expect_option_instance_handle(self) -> DdsResult<Option<InstanceHandle>> {
        match self {
            DcpsReply::OptionInstanceHandle(res) => res,
            other => panic!("expected Option<InstanceHandle> reply, got {:?}", other),
        }
    }

    pub fn expect_participant_qos(self) -> DdsResult<DomainParticipantQos> {
        match self {
            DcpsReply::ParticipantQos(res) => res,
            other => panic!("expected ParticipantQos reply, got {:?}", other),
        }
    }

    pub fn expect_publisher_qos(self) -> DdsResult<PublisherQos> {
        match self {
            DcpsReply::PublisherQos(res) => res,
            other => panic!("expected PublisherQos reply, got {:?}", other),
        }
    }

    pub fn expect_subscriber_qos(self) -> DdsResult<SubscriberQos> {
        match self {
            DcpsReply::SubscriberQos(res) => res,
            other => panic!("expected SubscriberQos reply, got {:?}", other),
        }
    }

    pub fn expect_topic_qos(self) -> DdsResult<TopicQos> {
        match self {
            DcpsReply::TopicQos(res) => res.map(|b| *b),
            other => panic!("expected TopicQos reply, got {:?}", other),
        }
    }

    pub fn expect_data_writer_qos(self) -> DdsResult<DataWriterQos> {
        match self {
            DcpsReply::DataWriterQos(res) => res.map(|b| *b),
            other => panic!("expected DataWriterQos reply, got {:?}", other),
        }
    }

    pub fn expect_data_reader_qos(self) -> DdsResult<DataReaderQos> {
        match self {
            DcpsReply::DataReaderQos(res) => res.map(|b| *b),
            other => panic!("expected DataReaderQos reply, got {:?}", other),
        }
    }

    pub fn expect_factory_qos(self) -> DdsResult<DomainParticipantFactoryQos> {
        match self {
            DcpsReply::FactoryQos(res) => res,
            other => panic!("expected FactoryQos reply, got {:?}", other),
        }
    }

    pub fn expect_instance_handle_list(self) -> DdsResult<Vec<InstanceHandle>> {
        match self {
            DcpsReply::InstanceHandleList(res) => res,
            other => panic!("expected InstanceHandleList reply, got {:?}", other),
        }
    }

    pub fn expect_participant_builtin_topic_data(self) -> DdsResult<ParticipantBuiltinTopicData> {
        match self {
            DcpsReply::ParticipantBuiltinTopicData(res) => res.map(|b| *b),
            other => panic!(
                "expected ParticipantBuiltinTopicData reply, got {:?}",
                other
            ),
        }
    }

    pub fn expect_topic_builtin_topic_data(self) -> DdsResult<TopicBuiltinTopicData> {
        match self {
            DcpsReply::TopicBuiltinTopicData(res) => res.map(|b| *b),
            other => panic!("expected TopicBuiltinTopicData reply, got {:?}", other),
        }
    }

    pub fn expect_publication_builtin_topic_data(self) -> DdsResult<PublicationBuiltinTopicData> {
        match self {
            DcpsReply::PublicationBuiltinTopicData(res) => res.map(|b| *b),
            other => panic!(
                "expected PublicationBuiltinTopicData reply, got {:?}",
                other
            ),
        }
    }

    pub fn expect_subscription_builtin_topic_data(self) -> DdsResult<SubscriptionBuiltinTopicData> {
        match self {
            DcpsReply::SubscriptionBuiltinTopicData(res) => res.map(|b| *b),
            other => panic!(
                "expected SubscriptionBuiltinTopicData reply, got {:?}",
                other
            ),
        }
    }

    pub fn expect_inconsistent_topic_status(self) -> DdsResult<InconsistentTopicStatus> {
        match self {
            DcpsReply::InconsistentTopicStatus(res) => res,
            other => panic!("expected InconsistentTopicStatus reply, got {:?}", other),
        }
    }

    pub fn expect_publication_matched_status(self) -> DdsResult<PublicationMatchedStatus> {
        match self {
            DcpsReply::PublicationMatchedStatus(res) => res,
            other => panic!("expected PublicationMatchedStatus reply, got {:?}", other),
        }
    }

    pub fn expect_subscription_matched_status(self) -> DdsResult<SubscriptionMatchedStatus> {
        match self {
            DcpsReply::SubscriptionMatchedStatus(res) => res,
            other => panic!("expected SubscriptionMatchedStatus reply, got {:?}", other),
        }
    }

    pub fn expect_offered_deadline_missed_status(self) -> DdsResult<OfferedDeadlineMissedStatus> {
        match self {
            DcpsReply::OfferedDeadlineMissedStatus(res) => res,
            other => panic!(
                "expected OfferedDeadlineMissedStatus reply, got {:?}",
                other
            ),
        }
    }

    pub fn expect_time(self) -> DdsResult<Time> {
        match self {
            DcpsReply::Time(res) => res,
            other => panic!("expected Time reply, got {:?}", other),
        }
    }

    pub fn expect_dynamic_type(self) -> DdsResult<DynamicType<'static>> {
        match self {
            DcpsReply::DynamicType(res) => res,
            other => panic!("expected DynamicType reply, got {:?}", other),
        }
    }

    pub fn expect_topic_description(self) -> DdsResult<Option<String>> {
        match self {
            DcpsReply::TopicDescription(res) => res,
            other => panic!("expected TopicDescription reply, got {:?}", other),
        }
    }

    pub fn expect_status_mask(self) -> DdsResult<StatusMask> {
        match self {
            DcpsReply::StatusMask(res) => res,
            other => panic!("expected StatusMask reply, got {:?}", other),
        }
    }

    pub fn expect_trigger_value(self) -> DdsResult<bool> {
        match self {
            DcpsReply::TriggerValue(res) => res,
            other => panic!("expected TriggerValue reply, got {:?}", other),
        }
    }

    pub fn expect_samples(self) -> DdsResult<Vec<(Option<DynamicData<'static>>, SampleInfo)>> {
        match self {
            DcpsReply::Samples(res) => res,
            other => panic!("expected Samples reply, got {:?}", other),
        }
    }
}
