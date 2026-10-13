use dust_dds::{
    dds_async::data_reader::DataReaderAsync,
    domain::domain_participant_factory::DomainParticipantFactory,
    infrastructure::{
        listener::NO_LISTENER,
        qos::{DataReaderQos, DataWriterQos, QosKind},
        qos_policy::{
            HistoryQosPolicy, HistoryQosPolicyKind, ReliabilityQosPolicy, ReliabilityQosPolicyKind,
        },
        status::{NO_STATUS, StatusKind, SubscriptionMatchedStatus},
        time::{Duration, DurationKind},
        type_support::DdsType,
    },
    subscription::data_reader_listener::DataReaderListener,
    wait_set::{Condition, WaitSet},
};

mod utils;
use crate::utils::domain_id_generator::TEST_DOMAIN_ID_GENERATOR;

#[derive(Debug, PartialEq, DdsType)]
struct MyData {
    #[dust_dds(key)]
    id: u8,
    value: u8,
}

#[test]
fn reader_subscription_matched_listener_and_wait_set_should_both_trigger() {
    struct SubscriptionMatchedListener {
        sender: Option<std::sync::mpsc::SyncSender<SubscriptionMatchedStatus>>,
    }

    impl DataReaderListener<MyData> for SubscriptionMatchedListener {
        async fn on_subscription_matched(
            &mut self,
            _the_reader: DataReaderAsync<MyData>,
            status: SubscriptionMatchedStatus,
        ) {
            if let Some(s) = self.sender.take() {
                s.send(status).unwrap()
            };
        }
    }

    let domain_id = TEST_DOMAIN_ID_GENERATOR.generate_unique_domain_id();
    let participant_factory = DomainParticipantFactory::get_instance();

    let participant = participant_factory
        .create_participant(domain_id, QosKind::Default, NO_LISTENER, NO_STATUS)
        .unwrap();
    let topic = participant
        .create_topic::<MyData>(
            "SubscriptionMatchedListenerTopic",
            "MyData",
            QosKind::Default,
            NO_LISTENER,
            NO_STATUS,
        )
        .unwrap();

    let publisher = participant
        .create_publisher(QosKind::Default, NO_LISTENER, NO_STATUS)
        .unwrap();
    let data_writer_qos = DataWriterQos {
        reliability: ReliabilityQosPolicy {
            kind: ReliabilityQosPolicyKind::BestEffort,
            max_blocking_time: DurationKind::Finite(Duration::new(1, 0)),
        },
        history: HistoryQosPolicy {
            kind: HistoryQosPolicyKind::KeepAll,
        },
        ..Default::default()
    };
    let _writer = publisher
        .create_datawriter::<MyData>(
            &topic,
            QosKind::Specific(data_writer_qos),
            NO_LISTENER,
            NO_STATUS,
        )
        .unwrap();

    let subscriber = participant
        .create_subscriber(QosKind::Default, NO_LISTENER, NO_STATUS)
        .unwrap();
    let reader_qos = DataReaderQos {
        reliability: ReliabilityQosPolicy {
            kind: ReliabilityQosPolicyKind::BestEffort,
            max_blocking_time: DurationKind::Finite(Duration::new(1, 0)),
        },
        history: HistoryQosPolicy {
            kind: HistoryQosPolicyKind::KeepAll,
        },

        ..Default::default()
    };

    let (sender, receiver) = std::sync::mpsc::sync_channel(1);
    let reader_listener = SubscriptionMatchedListener {
        sender: Some(sender),
    };

    let reader = subscriber
        .create_datareader(
            &topic,
            QosKind::Specific(reader_qos),
            Some(reader_listener),
            &[StatusKind::SubscriptionMatched],
        )
        .unwrap();
    let condition = reader.get_statuscondition();
    condition
        .set_enabled_statuses(&[StatusKind::SubscriptionMatched])
        .unwrap();
    let mut wait_set = WaitSet::new();
    wait_set
        .attach_condition(Condition::StatusCondition(condition))
        .unwrap();

    wait_set.wait(Duration::new(10, 0)).unwrap();

    let status = receiver
        .recv_timeout(std::time::Duration::from_secs(10))
        .unwrap();
    assert_eq!(status.total_count, 1);
    assert_eq!(status.total_count_change, 1);
}

#[test]
fn reader_on_data_available_matched_listener_and_wait_set_should_both_trigger() {
    struct DataAvailableListener {
        sender: Option<std::sync::mpsc::SyncSender<()>>,
    }

    impl DataReaderListener<MyData> for DataAvailableListener {
        async fn on_data_available(&mut self, _the_reader: DataReaderAsync<MyData>) {
            if let Some(s) = self.sender.take() {
                s.send(()).unwrap()
            };
        }
    }

    let domain_id = TEST_DOMAIN_ID_GENERATOR.generate_unique_domain_id();
    let participant_factory = DomainParticipantFactory::get_instance();

    let participant = participant_factory
        .create_participant(domain_id, QosKind::Default, NO_LISTENER, NO_STATUS)
        .unwrap();
    let topic = participant
        .create_topic::<MyData>(
            "DataAvailableMatchedListenerTopic",
            "MyData",
            QosKind::Default,
            NO_LISTENER,
            NO_STATUS,
        )
        .unwrap();

    let publisher = participant
        .create_publisher(QosKind::Default, NO_LISTENER, NO_STATUS)
        .unwrap();
    let data_writer_qos = DataWriterQos {
        reliability: ReliabilityQosPolicy {
            kind: ReliabilityQosPolicyKind::BestEffort,
            max_blocking_time: DurationKind::Finite(Duration::new(1, 0)),
        },
        history: HistoryQosPolicy {
            kind: HistoryQosPolicyKind::KeepAll,
        },
        ..Default::default()
    };
    let writer = publisher
        .create_datawriter::<MyData>(
            &topic,
            QosKind::Specific(data_writer_qos),
            NO_LISTENER,
            NO_STATUS,
        )
        .unwrap();

    let subscriber = participant
        .create_subscriber(QosKind::Default, NO_LISTENER, NO_STATUS)
        .unwrap();
    let reader_qos = DataReaderQos {
        reliability: ReliabilityQosPolicy {
            kind: ReliabilityQosPolicyKind::BestEffort,
            max_blocking_time: DurationKind::Finite(Duration::new(1, 0)),
        },
        history: HistoryQosPolicy {
            kind: HistoryQosPolicyKind::KeepAll,
        },

        ..Default::default()
    };

    let (sender, receiver) = std::sync::mpsc::sync_channel(1);
    let reader_listener = DataAvailableListener {
        sender: Some(sender),
    };

    let reader = subscriber
        .create_datareader(
            &topic,
            QosKind::Specific(reader_qos),
            Some(reader_listener),
            &[StatusKind::DataAvailable],
        )
        .unwrap();
    let condition = writer.get_statuscondition();
    condition
        .set_enabled_statuses(&[StatusKind::PublicationMatched])
        .unwrap();
    let mut wait_set = WaitSet::new();
    wait_set
        .attach_condition(Condition::StatusCondition(condition))
        .unwrap();
    wait_set.wait(Duration::new(10, 0)).unwrap();

    let reader_condition = reader.get_statuscondition();
    reader_condition
        .set_enabled_statuses(&[StatusKind::SubscriptionMatched])
        .unwrap();
    let mut reader_wait_set = WaitSet::new();
    reader_wait_set
        .attach_condition(Condition::StatusCondition(reader_condition))
        .unwrap();
    reader_wait_set.wait(Duration::new(10, 0)).unwrap();

    writer.write(MyData { id: 1, value: 1 }, None).unwrap();

    let condition = reader.get_statuscondition();
    condition
        .set_enabled_statuses(&[StatusKind::DataAvailable])
        .unwrap();
    let mut wait_set = WaitSet::new();
    wait_set
        .attach_condition(Condition::StatusCondition(condition))
        .unwrap();

    wait_set.wait(Duration::new(10, 0)).unwrap();

    assert!(
        receiver
            .recv_timeout(std::time::Duration::from_secs(10))
            .is_ok()
    );
}
