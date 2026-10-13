use dust_dds::{
    domain::domain_participant_factory::DomainParticipantFactory,
    infrastructure::{
        error::DdsError,
        instance::InstanceHandle,
        listener::NO_LISTENER,
        qos::{DataReaderQos, DataWriterQos, QosKind},
        qos_policy::{
            DeadlineQosPolicy, DestinationOrderQosPolicy, DestinationOrderQosPolicyKind,
            DurabilityQosPolicy, DurabilityQosPolicyKind, HistoryQosPolicy, HistoryQosPolicyKind,
            Length, LifespanQosPolicy, OwnershipQosPolicy, OwnershipQosPolicyKind,
            OwnershipStrengthQosPolicy, ReliabilityQosPolicy, ReliabilityQosPolicyKind,
            ResourceLimitsQosPolicy, TimeBasedFilterQosPolicy,
        },
        sample_info::{
            ANY_INSTANCE_STATE, ANY_SAMPLE_STATE, ANY_VIEW_STATE, InstanceStateKind,
            SampleStateKind, ViewStateKind,
        },
        status::{NO_STATUS, StatusKind},
        time::{Duration, DurationKind, Time},
        type_support::DdsType,
    },
    wait_set::{Condition, WaitSet},
};

mod utils;
use crate::utils::domain_id_generator::TEST_DOMAIN_ID_GENERATOR;

#[derive(Debug, PartialEq, DdsType)]
struct UserData(u8);

#[derive(Clone, Debug, PartialEq, DdsType)]
struct KeyedData {
    #[dust_dds(key)]
    id: u8,
    value: u32,
}

#[derive(Clone, Debug, PartialEq, DdsType)]
struct LargeData {
    #[dust_dds(key)]
    id: u8,
    value: Vec<u8>,
}

#[test]
fn large_data_should_be_fragmented() {
    let domain_id = TEST_DOMAIN_ID_GENERATOR.generate_unique_domain_id();
    let participant = DomainParticipantFactory::get_instance()
        .create_participant(domain_id, QosKind::Default, NO_LISTENER, NO_STATUS)
        .unwrap();

    let topic = participant
        .create_topic::<LargeData>(
            "LargeDataTopic",
            "LargeData",
            QosKind::Default,
            NO_LISTENER,
            NO_STATUS,
        )
        .unwrap();

    let publisher = participant
        .create_publisher(QosKind::Default, NO_LISTENER, NO_STATUS)
        .unwrap();
    let writer_qos = DataWriterQos {
        reliability: ReliabilityQosPolicy {
            kind: ReliabilityQosPolicyKind::BestEffort,
            max_blocking_time: DurationKind::Finite(Duration::new(1, 0)),
        },
        ..Default::default()
    };
    let writer = publisher
        .create_datawriter(
            &topic,
            QosKind::Specific(writer_qos),
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
        ..Default::default()
    };
    let reader = subscriber
        .create_datareader::<LargeData>(
            &topic,
            QosKind::Specific(reader_qos),
            NO_LISTENER,
            NO_STATUS,
        )
        .unwrap();

    let cond = writer.get_statuscondition();
    cond.set_enabled_statuses(&[StatusKind::PublicationMatched])
        .unwrap();

    let mut wait_set = WaitSet::new();
    wait_set
        .attach_condition(Condition::StatusCondition(cond))
        .unwrap();
    wait_set.wait(Duration::new(10, 0)).unwrap();

    let reader_cond = reader.get_statuscondition();
    reader_cond
        .set_enabled_statuses(&[StatusKind::SubscriptionMatched])
        .unwrap();
    let mut reader_wait_set = WaitSet::new();
    reader_wait_set
        .attach_condition(Condition::StatusCondition(reader_cond))
        .unwrap();
    reader_wait_set.wait(Duration::new(10, 0)).unwrap();

    let even_data = LargeData {
        id: 1,
        value: vec![8; 15000],
    };

    writer.write(even_data.clone(), None).unwrap();

    let cond = reader.get_statuscondition();
    cond.set_enabled_statuses(&[StatusKind::DataAvailable])
        .unwrap();
    let mut reader_wait_set = WaitSet::new();
    reader_wait_set
        .attach_condition(Condition::StatusCondition(cond))
        .unwrap();
    reader_wait_set.wait(Duration::new(10, 0)).unwrap();

    let samples = reader
        .take(3, ANY_SAMPLE_STATE, ANY_VIEW_STATE, ANY_INSTANCE_STATE)
        .unwrap();

    assert_eq!(samples.len(), 1);
    assert_eq!(samples[0].data.as_ref().unwrap(), &even_data);

    let odd_data = LargeData {
        id: 1,
        value: vec![8; 15001],
    };

    writer.write(odd_data.clone(), None).unwrap();

    let cond = reader.get_statuscondition();
    cond.set_enabled_statuses(&[StatusKind::DataAvailable])
        .unwrap();
    let mut reader_wait_set = WaitSet::new();
    reader_wait_set
        .attach_condition(Condition::StatusCondition(cond))
        .unwrap();
    reader_wait_set.wait(Duration::new(10, 0)).unwrap();

    let samples = reader
        .take(3, ANY_SAMPLE_STATE, ANY_VIEW_STATE, ANY_INSTANCE_STATE)
        .unwrap();

    assert_eq!(samples.len(), 1);
    assert_eq!(samples[0].data.as_ref().unwrap(), &odd_data);
}

#[test]
fn large_data_should_be_fragmented_reliable() {
    let domain_id = TEST_DOMAIN_ID_GENERATOR.generate_unique_domain_id();

    let participant = DomainParticipantFactory::get_instance()
        .create_participant(domain_id, QosKind::Default, NO_LISTENER, NO_STATUS)
        .unwrap();

    let topic = participant
        .create_topic::<LargeData>(
            "LargeDataTopic",
            "LargeData",
            QosKind::Default,
            NO_LISTENER,
            NO_STATUS,
        )
        .unwrap();

    let publisher = participant
        .create_publisher(QosKind::Default, NO_LISTENER, NO_STATUS)
        .unwrap();
    let writer_qos = DataWriterQos {
        reliability: ReliabilityQosPolicy {
            kind: ReliabilityQosPolicyKind::Reliable,
            max_blocking_time: DurationKind::Finite(Duration::new(1, 0)),
        },
        ..Default::default()
    };
    let writer = publisher
        .create_datawriter(
            &topic,
            QosKind::Specific(writer_qos),
            NO_LISTENER,
            NO_STATUS,
        )
        .unwrap();

    let subscriber = participant
        .create_subscriber(QosKind::Default, NO_LISTENER, NO_STATUS)
        .unwrap();
    let reader_qos = DataReaderQos {
        reliability: ReliabilityQosPolicy {
            kind: ReliabilityQosPolicyKind::Reliable,
            max_blocking_time: DurationKind::Finite(Duration::new(1, 0)),
        },
        ..Default::default()
    };
    let reader = subscriber
        .create_datareader::<LargeData>(
            &topic,
            QosKind::Specific(reader_qos),
            NO_LISTENER,
            NO_STATUS,
        )
        .unwrap();

    let cond = writer.get_statuscondition();
    cond.set_enabled_statuses(&[StatusKind::PublicationMatched])
        .unwrap();

    let mut wait_set = WaitSet::new();
    wait_set
        .attach_condition(Condition::StatusCondition(cond))
        .unwrap();
    wait_set.wait(Duration::new(5, 0)).unwrap();

    let reader_cond = reader.get_statuscondition();
    reader_cond
        .set_enabled_statuses(&[StatusKind::SubscriptionMatched])
        .unwrap();
    let mut reader_wait_set = WaitSet::new();
    reader_wait_set
        .attach_condition(Condition::StatusCondition(reader_cond))
        .unwrap();
    reader_wait_set.wait(Duration::new(5, 0)).unwrap();

    let data = LargeData {
        id: 1,
        value: vec![8; 15000],
    };

    writer.write(data.clone(), None).unwrap();

    writer
        .wait_for_acknowledgments(Duration::new(10, 0))
        .unwrap();

    let samples = reader
        .take(3, ANY_SAMPLE_STATE, ANY_VIEW_STATE, ANY_INSTANCE_STATE)
        .unwrap();

    assert_eq!(samples.len(), 1);
    assert_eq!(samples[0].data.as_ref().unwrap(), &data);
}

#[test]
fn writer_with_keep_last_1_should_send_only_last_sample_to_reader() {
    let domain_id = TEST_DOMAIN_ID_GENERATOR.generate_unique_domain_id();

    let participant = DomainParticipantFactory::get_instance()
        .create_participant(domain_id, QosKind::Default, NO_LISTENER, NO_STATUS)
        .unwrap();

    let topic = participant
        .create_topic::<KeyedData>(
            "MyTopic",
            "KeyedData",
            QosKind::Default,
            NO_LISTENER,
            NO_STATUS,
        )
        .unwrap();

    let publisher = participant
        .create_publisher(QosKind::Default, NO_LISTENER, NO_STATUS)
        .unwrap();
    let writer_qos = DataWriterQos {
        reliability: ReliabilityQosPolicy {
            kind: ReliabilityQosPolicyKind::Reliable,
            max_blocking_time: DurationKind::Finite(Duration::new(1, 0)),
        },
        durability: DurabilityQosPolicy {
            kind: DurabilityQosPolicyKind::TransientLocal,
        },
        history: HistoryQosPolicy {
            kind: HistoryQosPolicyKind::KeepLast(1),
        },
        ..Default::default()
    };

    let writer = publisher
        .create_datawriter(
            &topic,
            QosKind::Specific(writer_qos),
            NO_LISTENER,
            NO_STATUS,
        )
        .unwrap();

    let subscriber = participant
        .create_subscriber(QosKind::Default, NO_LISTENER, NO_STATUS)
        .unwrap();

    let data1 = KeyedData { id: 1, value: 1 };
    let data2 = KeyedData { id: 1, value: 2 };
    let data3 = KeyedData { id: 1, value: 3 };
    let data4 = KeyedData { id: 1, value: 4 };
    let data5 = KeyedData { id: 1, value: 5 };

    writer.write(data1, None).unwrap();
    writer.write(data2, None).unwrap();
    writer.write(data3, None).unwrap();
    writer.write(data4, None).unwrap();
    writer.write(data5.clone(), None).unwrap();

    let reader_qos = DataReaderQos {
        reliability: ReliabilityQosPolicy {
            kind: ReliabilityQosPolicyKind::Reliable,
            max_blocking_time: DurationKind::Finite(Duration::new(1, 0)),
        },
        durability: DurabilityQosPolicy {
            kind: DurabilityQosPolicyKind::TransientLocal,
        },
        history: HistoryQosPolicy {
            kind: HistoryQosPolicyKind::KeepAll,
        },
        ..Default::default()
    };
    let reader = subscriber
        .create_datareader::<KeyedData>(
            &topic,
            QosKind::Specific(reader_qos),
            NO_LISTENER,
            NO_STATUS,
        )
        .unwrap();

    let cond = reader.get_statuscondition();
    cond.set_enabled_statuses(&[StatusKind::SubscriptionMatched])
        .unwrap();

    let mut wait_set = WaitSet::new();
    wait_set
        .attach_condition(Condition::StatusCondition(cond))
        .unwrap();
    wait_set.wait(Duration::new(10, 0)).unwrap();

    reader
        .wait_for_historical_data(Duration::new(10, 0))
        .unwrap();

    let samples = reader
        .take(5, ANY_SAMPLE_STATE, ANY_VIEW_STATE, ANY_INSTANCE_STATE)
        .unwrap();

    assert_eq!(samples.len(), 1);
    assert_eq!(samples[0].data.as_ref().unwrap(), &data5);
}

#[test]
fn writer_with_keep_last_3_should_send_last_3_samples_to_reader() {
    let domain_id = TEST_DOMAIN_ID_GENERATOR.generate_unique_domain_id();

    let participant = DomainParticipantFactory::get_instance()
        .create_participant(domain_id, QosKind::Default, NO_LISTENER, NO_STATUS)
        .unwrap();

    let topic = participant
        .create_topic::<KeyedData>(
            "MyTopic",
            "KeyedData",
            QosKind::Default,
            NO_LISTENER,
            NO_STATUS,
        )
        .unwrap();

    let publisher = participant
        .create_publisher(QosKind::Default, NO_LISTENER, NO_STATUS)
        .unwrap();
    let writer_qos = DataWriterQos {
        reliability: ReliabilityQosPolicy {
            kind: ReliabilityQosPolicyKind::Reliable,
            max_blocking_time: DurationKind::Finite(Duration::new(1, 0)),
        },
        durability: DurabilityQosPolicy {
            kind: DurabilityQosPolicyKind::TransientLocal,
        },
        history: HistoryQosPolicy {
            kind: HistoryQosPolicyKind::KeepLast(3),
        },
        ..Default::default()
    };

    let writer = publisher
        .create_datawriter(
            &topic,
            QosKind::Specific(writer_qos),
            NO_LISTENER,
            NO_STATUS,
        )
        .unwrap();

    let subscriber = participant
        .create_subscriber(QosKind::Default, NO_LISTENER, NO_STATUS)
        .unwrap();

    let data1 = KeyedData { id: 1, value: 1 };
    let data2 = KeyedData { id: 1, value: 2 };
    let data3 = KeyedData { id: 1, value: 3 };
    let data4 = KeyedData { id: 1, value: 4 };
    let data5 = KeyedData { id: 1, value: 5 };

    writer.write(data1, None).unwrap();
    writer.write(data2, None).unwrap();
    writer.write(data3.clone(), None).unwrap();
    writer.write(data4.clone(), None).unwrap();
    writer.write(data5.clone(), None).unwrap();

    let reader_qos = DataReaderQos {
        reliability: ReliabilityQosPolicy {
            kind: ReliabilityQosPolicyKind::Reliable,
            max_blocking_time: DurationKind::Finite(Duration::new(1, 0)),
        },
        durability: DurabilityQosPolicy {
            kind: DurabilityQosPolicyKind::TransientLocal,
        },
        history: HistoryQosPolicy {
            kind: HistoryQosPolicyKind::KeepAll,
        },
        ..Default::default()
    };
    let reader = subscriber
        .create_datareader::<KeyedData>(
            &topic,
            QosKind::Specific(reader_qos),
            NO_LISTENER,
            NO_STATUS,
        )
        .unwrap();

    let cond = reader.get_statuscondition();
    cond.set_enabled_statuses(&[StatusKind::SubscriptionMatched])
        .unwrap();

    let mut wait_set = WaitSet::new();
    wait_set
        .attach_condition(Condition::StatusCondition(cond))
        .unwrap();
    wait_set.wait(Duration::new(10, 0)).unwrap();

    reader
        .wait_for_historical_data(Duration::new(10, 0))
        .unwrap();

    let samples = reader
        .take(5, ANY_SAMPLE_STATE, ANY_VIEW_STATE, ANY_INSTANCE_STATE)
        .unwrap();

    assert_eq!(
        samples.len(),
        3,
        "Received wrong number of samples. Received samples: {samples:?}",
    );
    assert_eq!(samples[0].data.as_ref().unwrap(), &data3);
    assert_eq!(samples[1].data.as_ref().unwrap(), &data4);
    assert_eq!(samples[2].data.as_ref().unwrap(), &data5);
}

#[test]
fn samples_are_taken() {
    let domain_id = TEST_DOMAIN_ID_GENERATOR.generate_unique_domain_id();

    let participant = DomainParticipantFactory::get_instance()
        .create_participant(domain_id, QosKind::Default, NO_LISTENER, NO_STATUS)
        .unwrap();

    let topic = participant
        .create_topic::<KeyedData>(
            "MyTopic",
            "KeyedData",
            QosKind::Default,
            NO_LISTENER,
            NO_STATUS,
        )
        .unwrap();

    let publisher = participant
        .create_publisher(QosKind::Default, NO_LISTENER, NO_STATUS)
        .unwrap();
    let writer_qos = DataWriterQos {
        reliability: ReliabilityQosPolicy {
            kind: ReliabilityQosPolicyKind::Reliable,
            max_blocking_time: DurationKind::Finite(Duration::new(1, 0)),
        },
        ..Default::default()
    };
    let writer = publisher
        .create_datawriter(
            &topic,
            QosKind::Specific(writer_qos),
            NO_LISTENER,
            NO_STATUS,
        )
        .unwrap();

    let subscriber = participant
        .create_subscriber(QosKind::Default, NO_LISTENER, NO_STATUS)
        .unwrap();
    let reader_qos = DataReaderQos {
        reliability: ReliabilityQosPolicy {
            kind: ReliabilityQosPolicyKind::Reliable,
            max_blocking_time: DurationKind::Finite(Duration::new(1, 0)),
        },
        ..Default::default()
    };
    let reader = subscriber
        .create_datareader::<KeyedData>(
            &topic,
            QosKind::Specific(reader_qos),
            NO_LISTENER,
            NO_STATUS,
        )
        .unwrap();

    let cond = writer.get_statuscondition();
    cond.set_enabled_statuses(&[StatusKind::PublicationMatched])
        .unwrap();

    let mut wait_set = WaitSet::new();
    wait_set
        .attach_condition(Condition::StatusCondition(cond))
        .unwrap();
    wait_set.wait(Duration::new(10, 0)).unwrap();

    let data1 = KeyedData { id: 1, value: 1 };
    let data2 = KeyedData { id: 2, value: 10 };
    let data3 = KeyedData { id: 3, value: 20 };
    let data4 = KeyedData { id: 4, value: 30 };
    let data5 = KeyedData { id: 5, value: 40 };

    writer.write(data1.clone(), None).unwrap();
    writer.write(data2.clone(), None).unwrap();
    writer.write(data3.clone(), None).unwrap();
    writer.write(data4.clone(), None).unwrap();
    writer.write(data5.clone(), None).unwrap();

    writer
        .wait_for_acknowledgments(Duration::new(10, 0))
        .unwrap();

    let samples1 = reader
        .take(3, ANY_SAMPLE_STATE, ANY_VIEW_STATE, ANY_INSTANCE_STATE)
        .unwrap();

    let samples2 = reader
        .take(3, ANY_SAMPLE_STATE, ANY_VIEW_STATE, ANY_INSTANCE_STATE)
        .unwrap();

    assert_eq!(samples1.len(), 3);
    assert_eq!(samples1[0].data.as_ref().unwrap(), &data1);
    assert_eq!(samples1[1].data.as_ref().unwrap(), &data2);
    assert_eq!(samples1[2].data.as_ref().unwrap(), &data3);

    assert_eq!(samples2.len(), 2);
    assert_eq!(samples2[0].data.as_ref().unwrap(), &data4);
    assert_eq!(samples2[1].data.as_ref().unwrap(), &data5);
}

#[test]
fn wait_for_samples_to_be_taken_best_effort() {
    let domain_id = TEST_DOMAIN_ID_GENERATOR.generate_unique_domain_id();

    let participant = DomainParticipantFactory::get_instance()
        .create_participant(domain_id, QosKind::Default, NO_LISTENER, NO_STATUS)
        .unwrap();

    let topic = participant
        .create_topic::<KeyedData>(
            "MyTopic",
            "KeyedData",
            QosKind::Default,
            NO_LISTENER,
            NO_STATUS,
        )
        .unwrap();

    let publisher = participant
        .create_publisher(QosKind::Default, NO_LISTENER, NO_STATUS)
        .unwrap();
    let writer_qos = DataWriterQos {
        reliability: ReliabilityQosPolicy {
            kind: ReliabilityQosPolicyKind::BestEffort,
            max_blocking_time: DurationKind::Finite(Duration::new(1, 0)),
        },
        ..Default::default()
    };
    let writer = publisher
        .create_datawriter(
            &topic,
            QosKind::Specific(writer_qos),
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
        ..Default::default()
    };
    let reader = subscriber
        .create_datareader::<KeyedData>(
            &topic,
            QosKind::Specific(reader_qos),
            NO_LISTENER,
            NO_STATUS,
        )
        .unwrap();

    let cond = writer.get_statuscondition();
    cond.set_enabled_statuses(&[StatusKind::PublicationMatched])
        .unwrap();

    let mut wait_set = WaitSet::new();
    wait_set
        .attach_condition(Condition::StatusCondition(cond))
        .unwrap();
    wait_set.wait(Duration::new(10, 0)).unwrap();

    let reader_cond = reader.get_statuscondition();
    reader_cond
        .set_enabled_statuses(&[StatusKind::SubscriptionMatched])
        .unwrap();
    let mut reader_wait_set = WaitSet::new();
    reader_wait_set
        .attach_condition(Condition::StatusCondition(reader_cond))
        .unwrap();
    reader_wait_set.wait(Duration::new(10, 0)).unwrap();

    let data1 = KeyedData { id: 1, value: 1 };
    let data2 = KeyedData { id: 2, value: 10 };
    let data3 = KeyedData { id: 3, value: 20 };
    let data4 = KeyedData { id: 4, value: 30 };
    let data5 = KeyedData { id: 5, value: 40 };

    writer.write(data1.clone(), None).unwrap();
    writer.write(data2.clone(), None).unwrap();
    writer.write(data3.clone(), None).unwrap();
    writer.write(data4.clone(), None).unwrap();
    writer.write(data5.clone(), None).unwrap();

    let start_time = std::time::Instant::now();
    let mut samples = Vec::new();
    while start_time.elapsed() < std::time::Duration::from_secs(10) {
        if let Ok(mut sample) = reader.take(1, ANY_SAMPLE_STATE, ANY_VIEW_STATE, ANY_INSTANCE_STATE)
        {
            samples.push(sample.remove(0))
        }

        if samples.len() >= 5 {
            break;
        }
        std::thread::sleep(std::time::Duration::from_millis(20));
    }

    assert_eq!(samples.len(), 5);
    assert_eq!(samples[0].data.as_ref().unwrap(), &data1);
    assert_eq!(samples[1].data.as_ref().unwrap(), &data2);
    assert_eq!(samples[2].data.as_ref().unwrap(), &data3);
    assert_eq!(samples[3].data.as_ref().unwrap(), &data4);
    assert_eq!(samples[4].data.as_ref().unwrap(), &data5);
}

#[test]
fn read_only_unread_samples() {
    let domain_id = TEST_DOMAIN_ID_GENERATOR.generate_unique_domain_id();

    let participant = DomainParticipantFactory::get_instance()
        .create_participant(domain_id, QosKind::Default, NO_LISTENER, NO_STATUS)
        .unwrap();

    let topic = participant
        .create_topic::<KeyedData>(
            "MyTopic",
            "KeyedData",
            QosKind::Default,
            NO_LISTENER,
            NO_STATUS,
        )
        .unwrap();

    let publisher = participant
        .create_publisher(QosKind::Default, NO_LISTENER, NO_STATUS)
        .unwrap();
    let writer_qos = DataWriterQos {
        reliability: ReliabilityQosPolicy {
            kind: ReliabilityQosPolicyKind::Reliable,
            max_blocking_time: DurationKind::Finite(Duration::new(1, 0)),
        },
        ..Default::default()
    };
    let writer = publisher
        .create_datawriter(
            &topic,
            QosKind::Specific(writer_qos),
            NO_LISTENER,
            NO_STATUS,
        )
        .unwrap();

    let subscriber = participant
        .create_subscriber(QosKind::Default, NO_LISTENER, NO_STATUS)
        .unwrap();
    let reader_qos = DataReaderQos {
        reliability: ReliabilityQosPolicy {
            kind: ReliabilityQosPolicyKind::Reliable,
            max_blocking_time: DurationKind::Finite(Duration::new(1, 0)),
        },
        ..Default::default()
    };
    let reader = subscriber
        .create_datareader::<KeyedData>(
            &topic,
            QosKind::Specific(reader_qos),
            NO_LISTENER,
            NO_STATUS,
        )
        .unwrap();

    let cond = writer.get_statuscondition();
    cond.set_enabled_statuses(&[StatusKind::PublicationMatched])
        .unwrap();

    let mut wait_set = WaitSet::new();
    wait_set
        .attach_condition(Condition::StatusCondition(cond))
        .unwrap();
    wait_set.wait(Duration::new(10, 0)).unwrap();

    let data1 = KeyedData { id: 1, value: 1 };
    let data2 = KeyedData { id: 2, value: 10 };
    let data3 = KeyedData { id: 3, value: 20 };
    let data4 = KeyedData { id: 4, value: 30 };
    let data5 = KeyedData { id: 5, value: 40 };

    writer.write(data1.clone(), None).unwrap();
    writer.write(data2.clone(), None).unwrap();
    writer.write(data3.clone(), None).unwrap();
    writer.write(data4.clone(), None).unwrap();
    writer.write(data5.clone(), None).unwrap();

    writer
        .wait_for_acknowledgments(Duration::new(10, 0))
        .unwrap();

    let samples1 = reader
        .read(
            3,
            &[SampleStateKind::NotRead],
            ANY_VIEW_STATE,
            ANY_INSTANCE_STATE,
        )
        .unwrap();

    let samples2 = reader
        .read(
            3,
            &[SampleStateKind::NotRead],
            ANY_VIEW_STATE,
            ANY_INSTANCE_STATE,
        )
        .unwrap();

    let samples3 = reader.read(
        3,
        &[SampleStateKind::NotRead],
        ANY_VIEW_STATE,
        ANY_INSTANCE_STATE,
    );

    assert_eq!(samples1.len(), 3);
    assert_eq!(samples1[0].data.as_ref().unwrap(), &data1);
    assert_eq!(samples1[1].data.as_ref().unwrap(), &data2);
    assert_eq!(samples1[2].data.as_ref().unwrap(), &data3);

    assert_eq!(samples2.len(), 2);
    assert_eq!(samples2[0].data.as_ref().unwrap(), &data4);
    assert_eq!(samples2[1].data.as_ref().unwrap(), &data5);

    assert!(matches!(samples3, Err(DdsError::NoData)));
}

#[test]
fn read_next_sample() {
    let domain_id = TEST_DOMAIN_ID_GENERATOR.generate_unique_domain_id();

    let participant = DomainParticipantFactory::get_instance()
        .create_participant(domain_id, QosKind::Default, NO_LISTENER, NO_STATUS)
        .unwrap();

    let topic = participant
        .create_topic::<KeyedData>(
            "MyTopic",
            "KeyedData",
            QosKind::Default,
            NO_LISTENER,
            NO_STATUS,
        )
        .unwrap();

    let publisher = participant
        .create_publisher(QosKind::Default, NO_LISTENER, NO_STATUS)
        .unwrap();
    let writer_qos = DataWriterQos {
        reliability: ReliabilityQosPolicy {
            kind: ReliabilityQosPolicyKind::Reliable,
            max_blocking_time: DurationKind::Finite(Duration::new(1, 0)),
        },
        ..Default::default()
    };
    let writer = publisher
        .create_datawriter(
            &topic,
            QosKind::Specific(writer_qos),
            NO_LISTENER,
            NO_STATUS,
        )
        .unwrap();

    let subscriber = participant
        .create_subscriber(QosKind::Default, NO_LISTENER, NO_STATUS)
        .unwrap();
    let reader_qos = DataReaderQos {
        reliability: ReliabilityQosPolicy {
            kind: ReliabilityQosPolicyKind::Reliable,
            max_blocking_time: DurationKind::Finite(Duration::new(1, 0)),
        },
        ..Default::default()
    };
    let reader = subscriber
        .create_datareader::<KeyedData>(
            &topic,
            QosKind::Specific(reader_qos),
            NO_LISTENER,
            NO_STATUS,
        )
        .unwrap();

    let cond = writer.get_statuscondition();
    cond.set_enabled_statuses(&[StatusKind::PublicationMatched])
        .unwrap();

    let mut wait_set = WaitSet::new();
    wait_set
        .attach_condition(Condition::StatusCondition(cond))
        .unwrap();
    wait_set.wait(Duration::new(10, 0)).unwrap();

    let data1 = KeyedData { id: 1, value: 1 };
    let data2 = KeyedData { id: 2, value: 10 };
    let data3 = KeyedData { id: 3, value: 20 };

    writer.write(data1.clone(), None).unwrap();
    writer.write(data2.clone(), None).unwrap();
    writer.write(data3.clone(), None).unwrap();

    writer
        .wait_for_acknowledgments(Duration::new(10, 0))
        .unwrap();

    assert_eq!(
        reader.read_next_sample().unwrap().data.as_ref().unwrap(),
        &data1
    );
    assert_eq!(
        reader.read_next_sample().unwrap().data.as_ref().unwrap(),
        &data2
    );
    assert_eq!(
        reader.read_next_sample().unwrap().data.as_ref().unwrap(),
        &data3
    );
    assert!(matches!(reader.read_next_sample(), Err(DdsError::NoData)));
}

#[test]
fn take_next_sample() {
    let domain_id = TEST_DOMAIN_ID_GENERATOR.generate_unique_domain_id();

    let participant = DomainParticipantFactory::get_instance()
        .create_participant(domain_id, QosKind::Default, NO_LISTENER, NO_STATUS)
        .unwrap();

    let topic = participant
        .create_topic::<KeyedData>(
            "MyTopic",
            "KeyedData",
            QosKind::Default,
            NO_LISTENER,
            NO_STATUS,
        )
        .unwrap();

    let publisher = participant
        .create_publisher(QosKind::Default, NO_LISTENER, NO_STATUS)
        .unwrap();
    let writer_qos = DataWriterQos {
        reliability: ReliabilityQosPolicy {
            kind: ReliabilityQosPolicyKind::Reliable,
            max_blocking_time: DurationKind::Finite(Duration::new(1, 0)),
        },
        ..Default::default()
    };
    let writer = publisher
        .create_datawriter(
            &topic,
            QosKind::Specific(writer_qos),
            NO_LISTENER,
            NO_STATUS,
        )
        .unwrap();

    let subscriber = participant
        .create_subscriber(QosKind::Default, NO_LISTENER, NO_STATUS)
        .unwrap();
    let reader_qos = DataReaderQos {
        reliability: ReliabilityQosPolicy {
            kind: ReliabilityQosPolicyKind::Reliable,
            max_blocking_time: DurationKind::Finite(Duration::new(1, 0)),
        },
        ..Default::default()
    };
    let reader = subscriber
        .create_datareader::<KeyedData>(
            &topic,
            QosKind::Specific(reader_qos),
            NO_LISTENER,
            NO_STATUS,
        )
        .unwrap();

    let cond = writer.get_statuscondition();
    cond.set_enabled_statuses(&[StatusKind::PublicationMatched])
        .unwrap();

    let mut wait_set = WaitSet::new();
    wait_set
        .attach_condition(Condition::StatusCondition(cond))
        .unwrap();
    wait_set.wait(Duration::new(5, 0)).unwrap();

    let data1 = KeyedData { id: 1, value: 1 };
    let data2 = KeyedData { id: 2, value: 10 };
    let data3 = KeyedData { id: 3, value: 20 };

    writer.write(data1.clone(), None).unwrap();
    writer.write(data2.clone(), None).unwrap();
    writer.write(data3.clone(), None).unwrap();

    writer
        .wait_for_acknowledgments(Duration::new(10, 0))
        .unwrap();

    assert_eq!(
        reader.take_next_sample().unwrap().data.as_ref().unwrap(),
        &data1
    );
    assert_eq!(
        reader.take_next_sample().unwrap().data.as_ref().unwrap(),
        &data2
    );
    assert_eq!(
        reader.take_next_sample().unwrap().data.as_ref().unwrap(),
        &data3
    );
    assert!(matches!(reader.take_next_sample(), Err(DdsError::NoData)));
}

#[test]
fn each_key_sample_is_read() {
    let domain_id = TEST_DOMAIN_ID_GENERATOR.generate_unique_domain_id();

    let participant = DomainParticipantFactory::get_instance()
        .create_participant(domain_id, QosKind::Default, NO_LISTENER, NO_STATUS)
        .unwrap();

    let topic = participant
        .create_topic::<KeyedData>(
            "MyTopic",
            "KeyedData",
            QosKind::Default,
            NO_LISTENER,
            NO_STATUS,
        )
        .unwrap();

    let publisher = participant
        .create_publisher(QosKind::Default, NO_LISTENER, NO_STATUS)
        .unwrap();
    let writer_qos = DataWriterQos {
        reliability: ReliabilityQosPolicy {
            kind: ReliabilityQosPolicyKind::Reliable,
            max_blocking_time: DurationKind::Finite(Duration::new(1, 0)),
        },
        ..Default::default()
    };
    let writer = publisher
        .create_datawriter(
            &topic,
            QosKind::Specific(writer_qos),
            NO_LISTENER,
            NO_STATUS,
        )
        .unwrap();

    let subscriber = participant
        .create_subscriber(QosKind::Default, NO_LISTENER, NO_STATUS)
        .unwrap();
    let reader_qos = DataReaderQos {
        reliability: ReliabilityQosPolicy {
            kind: ReliabilityQosPolicyKind::Reliable,
            max_blocking_time: DurationKind::Finite(Duration::new(1, 0)),
        },
        ..Default::default()
    };
    let reader = subscriber
        .create_datareader::<KeyedData>(
            &topic,
            QosKind::Specific(reader_qos),
            NO_LISTENER,
            NO_STATUS,
        )
        .unwrap();

    let cond = writer.get_statuscondition();
    cond.set_enabled_statuses(&[StatusKind::PublicationMatched])
        .unwrap();

    let mut wait_set = WaitSet::new();
    wait_set
        .attach_condition(Condition::StatusCondition(cond))
        .unwrap();
    wait_set.wait(Duration::new(10, 0)).unwrap();

    let data1 = KeyedData { id: 1, value: 1 };
    let data2 = KeyedData { id: 2, value: 10 };
    let data3 = KeyedData { id: 3, value: 20 };

    writer.write(data1.clone(), None).unwrap();
    writer.write(data2.clone(), None).unwrap();
    writer.write(data3.clone(), None).unwrap();

    let data1_handle = writer.lookup_instance(data1.clone()).unwrap();
    let data2_handle = writer.lookup_instance(data2.clone()).unwrap();
    let data3_handle = writer.lookup_instance(data3.clone()).unwrap();

    writer
        .wait_for_acknowledgments(Duration::new(10, 0))
        .unwrap();

    let samples = reader
        .read(3, ANY_SAMPLE_STATE, ANY_VIEW_STATE, ANY_INSTANCE_STATE)
        .unwrap();

    assert_eq!(samples.len(), 3);
    assert_eq!(samples[0].data.as_ref().unwrap(), &data1);
    assert_eq!(
        samples[0].sample_info.instance_handle,
        data1_handle.unwrap(),
    );

    assert_eq!(samples[1].data.as_ref().unwrap(), &data2);
    assert_eq!(
        samples[1].sample_info.instance_handle,
        data2_handle.unwrap(),
    );

    assert_eq!(samples[2].data.as_ref().unwrap(), &data3);
    assert_eq!(
        samples[2].sample_info.instance_handle,
        data3_handle.unwrap(),
    );
}

#[test]
fn read_specific_instance() {
    let domain_id = TEST_DOMAIN_ID_GENERATOR.generate_unique_domain_id();

    let participant = DomainParticipantFactory::get_instance()
        .create_participant(domain_id, QosKind::Default, NO_LISTENER, NO_STATUS)
        .unwrap();

    let topic = participant
        .create_topic::<KeyedData>(
            "MyTopic",
            "KeyedData",
            QosKind::Default,
            NO_LISTENER,
            NO_STATUS,
        )
        .unwrap();

    let publisher = participant
        .create_publisher(QosKind::Default, NO_LISTENER, NO_STATUS)
        .unwrap();
    let writer_qos = DataWriterQos {
        reliability: ReliabilityQosPolicy {
            kind: ReliabilityQosPolicyKind::Reliable,
            max_blocking_time: DurationKind::Finite(Duration::new(1, 0)),
        },
        ..Default::default()
    };
    let writer = publisher
        .create_datawriter(
            &topic,
            QosKind::Specific(writer_qos),
            NO_LISTENER,
            NO_STATUS,
        )
        .unwrap();

    let subscriber = participant
        .create_subscriber(QosKind::Default, NO_LISTENER, NO_STATUS)
        .unwrap();
    let reader_qos = DataReaderQos {
        reliability: ReliabilityQosPolicy {
            kind: ReliabilityQosPolicyKind::Reliable,
            max_blocking_time: DurationKind::Finite(Duration::new(1, 0)),
        },
        ..Default::default()
    };
    let reader = subscriber
        .create_datareader::<KeyedData>(
            &topic,
            QosKind::Specific(reader_qos),
            NO_LISTENER,
            NO_STATUS,
        )
        .unwrap();

    let cond = writer.get_statuscondition();
    cond.set_enabled_statuses(&[StatusKind::PublicationMatched])
        .unwrap();

    let mut wait_set = WaitSet::new();
    wait_set
        .attach_condition(Condition::StatusCondition(cond))
        .unwrap();
    wait_set.wait(Duration::new(10, 0)).unwrap();

    let data1 = KeyedData { id: 1, value: 1 };
    let data2 = KeyedData { id: 2, value: 10 };
    let data3 = KeyedData { id: 3, value: 20 };

    writer.write(data1.clone(), None).unwrap();
    writer.write(data2, None).unwrap();
    writer.write(data3, None).unwrap();

    let data1_handle = writer.lookup_instance(data1.clone()).unwrap();

    writer
        .wait_for_acknowledgments(Duration::new(10, 0))
        .unwrap();

    let samples = reader
        .read_instance(
            3,
            data1_handle.unwrap(),
            ANY_SAMPLE_STATE,
            ANY_VIEW_STATE,
            ANY_INSTANCE_STATE,
        )
        .unwrap();

    assert_eq!(samples.len(), 1);
    assert_eq!(samples[0].data.as_ref().unwrap(), &data1);
}

#[test]
fn read_next_instance() {
    let domain_id = TEST_DOMAIN_ID_GENERATOR.generate_unique_domain_id();

    let participant = DomainParticipantFactory::get_instance()
        .create_participant(domain_id, QosKind::Default, NO_LISTENER, NO_STATUS)
        .unwrap();

    let topic = participant
        .create_topic::<KeyedData>(
            "MyTopic",
            "KeyedData",
            QosKind::Default,
            NO_LISTENER,
            NO_STATUS,
        )
        .unwrap();

    let publisher = participant
        .create_publisher(QosKind::Default, NO_LISTENER, NO_STATUS)
        .unwrap();
    let writer_qos = DataWriterQos {
        reliability: ReliabilityQosPolicy {
            kind: ReliabilityQosPolicyKind::Reliable,
            max_blocking_time: DurationKind::Finite(Duration::new(1, 0)),
        },
        ..Default::default()
    };
    let writer = publisher
        .create_datawriter(
            &topic,
            QosKind::Specific(writer_qos),
            NO_LISTENER,
            NO_STATUS,
        )
        .unwrap();

    let subscriber = participant
        .create_subscriber(QosKind::Default, NO_LISTENER, NO_STATUS)
        .unwrap();
    let reader_qos = DataReaderQos {
        reliability: ReliabilityQosPolicy {
            kind: ReliabilityQosPolicyKind::Reliable,
            max_blocking_time: DurationKind::Finite(Duration::new(1, 0)),
        },
        ..Default::default()
    };
    let reader = subscriber
        .create_datareader::<KeyedData>(
            &topic,
            QosKind::Specific(reader_qos),
            NO_LISTENER,
            NO_STATUS,
        )
        .unwrap();

    let cond = writer.get_statuscondition();
    cond.set_enabled_statuses(&[StatusKind::PublicationMatched])
        .unwrap();

    let mut wait_set = WaitSet::new();
    wait_set
        .attach_condition(Condition::StatusCondition(cond))
        .unwrap();
    wait_set.wait(Duration::new(5, 0)).unwrap();

    let data1 = KeyedData { id: 1, value: 1 };
    let data2 = KeyedData { id: 2, value: 10 };
    let data3 = KeyedData { id: 3, value: 20 };

    writer.write(data1.clone(), None).unwrap();
    writer.write(data2.clone(), None).unwrap();
    writer.write(data3.clone(), None).unwrap();

    writer
        .wait_for_acknowledgments(Duration::new(10, 0))
        .unwrap();

    let samples1 = reader
        .read_next_instance(
            3,
            None,
            ANY_SAMPLE_STATE,
            ANY_VIEW_STATE,
            ANY_INSTANCE_STATE,
        )
        .unwrap();

    let samples2 = reader
        .read_next_instance(
            3,
            Some(samples1[0].sample_info.instance_handle),
            ANY_SAMPLE_STATE,
            ANY_VIEW_STATE,
            ANY_INSTANCE_STATE,
        )
        .unwrap();

    let samples3 = reader
        .read_next_instance(
            3,
            Some(samples2[0].sample_info.instance_handle),
            ANY_SAMPLE_STATE,
            ANY_VIEW_STATE,
            ANY_INSTANCE_STATE,
        )
        .unwrap();

    let samples4 = reader.read_next_instance(
        3,
        Some(samples3[0].sample_info.instance_handle),
        ANY_SAMPLE_STATE,
        ANY_VIEW_STATE,
        ANY_INSTANCE_STATE,
    );

    assert_eq!(samples1[0].data.as_ref().unwrap(), &data1);
    assert_eq!(samples2[0].data.as_ref().unwrap(), &data2);
    assert_eq!(samples3[0].data.as_ref().unwrap(), &data3);
    assert!(matches!(samples4, Err(DdsError::NoData)));
}

#[test]
fn take_next_instance() {
    let domain_id = TEST_DOMAIN_ID_GENERATOR.generate_unique_domain_id();

    let participant = DomainParticipantFactory::get_instance()
        .create_participant(domain_id, QosKind::Default, NO_LISTENER, NO_STATUS)
        .unwrap();

    let topic = participant
        .create_topic::<KeyedData>(
            "MyTopic",
            "KeyedData",
            QosKind::Default,
            NO_LISTENER,
            NO_STATUS,
        )
        .unwrap();

    let publisher = participant
        .create_publisher(QosKind::Default, NO_LISTENER, NO_STATUS)
        .unwrap();
    let writer_qos = DataWriterQos {
        reliability: ReliabilityQosPolicy {
            kind: ReliabilityQosPolicyKind::Reliable,
            max_blocking_time: DurationKind::Finite(Duration::new(1, 0)),
        },
        ..Default::default()
    };
    let writer = publisher
        .create_datawriter(
            &topic,
            QosKind::Specific(writer_qos),
            NO_LISTENER,
            NO_STATUS,
        )
        .unwrap();

    let subscriber = participant
        .create_subscriber(QosKind::Default, NO_LISTENER, NO_STATUS)
        .unwrap();
    let reader_qos = DataReaderQos {
        reliability: ReliabilityQosPolicy {
            kind: ReliabilityQosPolicyKind::Reliable,
            max_blocking_time: DurationKind::Finite(Duration::new(1, 0)),
        },
        ..Default::default()
    };
    let reader = subscriber
        .create_datareader::<KeyedData>(
            &topic,
            QosKind::Specific(reader_qos),
            NO_LISTENER,
            NO_STATUS,
        )
        .unwrap();

    let cond = writer.get_statuscondition();
    cond.set_enabled_statuses(&[StatusKind::PublicationMatched])
        .unwrap();

    let mut wait_set = WaitSet::new();
    wait_set
        .attach_condition(Condition::StatusCondition(cond))
        .unwrap();
    wait_set.wait(Duration::new(10, 0)).unwrap();

    let data1 = KeyedData { id: 1, value: 1 };
    let data2 = KeyedData { id: 2, value: 10 };
    let data3 = KeyedData { id: 3, value: 20 };

    writer.write(data1.clone(), None).unwrap();
    writer.write(data2.clone(), None).unwrap();
    writer.write(data3.clone(), None).unwrap();

    writer
        .wait_for_acknowledgments(Duration::new(10, 0))
        .unwrap();

    let samples1 = reader
        .take_next_instance(
            3,
            None,
            ANY_SAMPLE_STATE,
            ANY_VIEW_STATE,
            ANY_INSTANCE_STATE,
        )
        .unwrap();

    let samples2 = reader
        .take_next_instance(
            3,
            Some(samples1[0].sample_info.instance_handle),
            ANY_SAMPLE_STATE,
            ANY_VIEW_STATE,
            ANY_INSTANCE_STATE,
        )
        .unwrap();

    let samples3 = reader
        .take_next_instance(
            3,
            Some(samples2[0].sample_info.instance_handle),
            ANY_SAMPLE_STATE,
            ANY_VIEW_STATE,
            ANY_INSTANCE_STATE,
        )
        .unwrap();

    let samples4 = reader.take_next_instance(
        3,
        Some(samples3[0].sample_info.instance_handle),
        ANY_SAMPLE_STATE,
        ANY_VIEW_STATE,
        ANY_INSTANCE_STATE,
    );

    assert_eq!(samples1[0].data.as_ref().unwrap(), &data1);
    assert_eq!(samples2[0].data.as_ref().unwrap(), &data2);
    assert_eq!(samples3[0].data.as_ref().unwrap(), &data3);
    assert!(matches!(samples4, Err(DdsError::NoData)));
}

#[test]
fn take_specific_instance() {
    let domain_id = TEST_DOMAIN_ID_GENERATOR.generate_unique_domain_id();

    let participant = DomainParticipantFactory::get_instance()
        .create_participant(domain_id, QosKind::Default, NO_LISTENER, NO_STATUS)
        .unwrap();

    let topic = participant
        .create_topic::<KeyedData>(
            "MyTopic",
            "KeyedData",
            QosKind::Default,
            NO_LISTENER,
            NO_STATUS,
        )
        .unwrap();

    let publisher = participant
        .create_publisher(QosKind::Default, NO_LISTENER, NO_STATUS)
        .unwrap();
    let writer_qos = DataWriterQos {
        reliability: ReliabilityQosPolicy {
            kind: ReliabilityQosPolicyKind::Reliable,
            max_blocking_time: DurationKind::Finite(Duration::new(1, 0)),
        },
        ..Default::default()
    };
    let writer = publisher
        .create_datawriter(
            &topic,
            QosKind::Specific(writer_qos),
            NO_LISTENER,
            NO_STATUS,
        )
        .unwrap();

    let subscriber = participant
        .create_subscriber(QosKind::Default, NO_LISTENER, NO_STATUS)
        .unwrap();
    let reader_qos = DataReaderQos {
        reliability: ReliabilityQosPolicy {
            kind: ReliabilityQosPolicyKind::Reliable,
            max_blocking_time: DurationKind::Finite(Duration::new(1, 0)),
        },
        ..Default::default()
    };
    let reader = subscriber
        .create_datareader::<KeyedData>(
            &topic,
            QosKind::Specific(reader_qos),
            NO_LISTENER,
            NO_STATUS,
        )
        .unwrap();

    let cond = writer.get_statuscondition();
    cond.set_enabled_statuses(&[StatusKind::PublicationMatched])
        .unwrap();

    let mut wait_set = WaitSet::new();
    wait_set
        .attach_condition(Condition::StatusCondition(cond))
        .unwrap();
    wait_set.wait(Duration::new(10, 0)).unwrap();

    let data1 = KeyedData { id: 1, value: 1 };
    let data2 = KeyedData { id: 2, value: 10 };
    let data3 = KeyedData { id: 3, value: 20 };

    writer.write(data1.clone(), None).unwrap();
    writer.write(data2, None).unwrap();
    writer.write(data3, None).unwrap();
    let data1_handle = writer.lookup_instance(data1.clone()).unwrap();

    writer
        .wait_for_acknowledgments(Duration::new(10, 0))
        .unwrap();

    let samples = reader
        .take_instance(
            3,
            data1_handle.unwrap(),
            ANY_SAMPLE_STATE,
            ANY_VIEW_STATE,
            ANY_INSTANCE_STATE,
        )
        .unwrap();

    assert_eq!(samples.len(), 1);
    assert_eq!(samples[0].data.as_ref().unwrap(), &data1);
}

#[test]
fn take_specific_unknown_instance() {
    let domain_id = TEST_DOMAIN_ID_GENERATOR.generate_unique_domain_id();

    let participant = DomainParticipantFactory::get_instance()
        .create_participant(domain_id, QosKind::Default, NO_LISTENER, NO_STATUS)
        .unwrap();

    let topic = participant
        .create_topic::<KeyedData>(
            "MyTopic",
            "KeyedData",
            QosKind::Default,
            NO_LISTENER,
            NO_STATUS,
        )
        .unwrap();

    let publisher = participant
        .create_publisher(QosKind::Default, NO_LISTENER, NO_STATUS)
        .unwrap();
    let writer_qos = DataWriterQos {
        reliability: ReliabilityQosPolicy {
            kind: ReliabilityQosPolicyKind::Reliable,
            max_blocking_time: DurationKind::Finite(Duration::new(1, 0)),
        },
        ..Default::default()
    };
    let writer = publisher
        .create_datawriter(
            &topic,
            QosKind::Specific(writer_qos),
            NO_LISTENER,
            NO_STATUS,
        )
        .unwrap();

    let subscriber = participant
        .create_subscriber(QosKind::Default, NO_LISTENER, NO_STATUS)
        .unwrap();
    let reader_qos = DataReaderQos {
        reliability: ReliabilityQosPolicy {
            kind: ReliabilityQosPolicyKind::Reliable,
            max_blocking_time: DurationKind::Finite(Duration::new(1, 0)),
        },
        ..Default::default()
    };
    let reader = subscriber
        .create_datareader::<KeyedData>(
            &topic,
            QosKind::Specific(reader_qos),
            NO_LISTENER,
            NO_STATUS,
        )
        .unwrap();

    let cond = writer.get_statuscondition();
    cond.set_enabled_statuses(&[StatusKind::PublicationMatched])
        .unwrap();

    let mut wait_set = WaitSet::new();
    wait_set
        .attach_condition(Condition::StatusCondition(cond))
        .unwrap();
    wait_set.wait(Duration::new(10, 0)).unwrap();

    let data1 = KeyedData { id: 1, value: 1 };
    let data2 = KeyedData { id: 2, value: 10 };
    let data3 = KeyedData { id: 3, value: 20 };

    writer.write(data1, None).unwrap();
    writer.write(data2, None).unwrap();
    writer.write(data3, None).unwrap();

    writer
        .wait_for_acknowledgments(Duration::new(10, 0))
        .unwrap();

    assert!(matches!(
        reader.take_instance(
            3,
            InstanceHandle::new([99; 16]),
            ANY_SAMPLE_STATE,
            ANY_VIEW_STATE,
            ANY_INSTANCE_STATE,
        ),
        Err(DdsError::BadParameter)
    ));
}

#[test]
fn write_read_disposed_samples() {
    let domain_id = TEST_DOMAIN_ID_GENERATOR.generate_unique_domain_id();
    let participant_factory = DomainParticipantFactory::get_instance();

    let participant = participant_factory
        .create_participant(domain_id, QosKind::Default, NO_LISTENER, NO_STATUS)
        .unwrap();

    let topic = participant
        .create_topic::<KeyedData>(
            "MyTopic",
            "KeyedData",
            QosKind::Default,
            NO_LISTENER,
            NO_STATUS,
        )
        .unwrap();

    let publisher = participant
        .create_publisher(QosKind::Default, NO_LISTENER, NO_STATUS)
        .unwrap();
    let writer_qos = DataWriterQos {
        reliability: ReliabilityQosPolicy {
            kind: ReliabilityQosPolicyKind::Reliable,
            max_blocking_time: DurationKind::Finite(Duration::new(1, 0)),
        },
        history: HistoryQosPolicy {
            kind: HistoryQosPolicyKind::KeepAll,
        },
        ..Default::default()
    };
    let writer = publisher
        .create_datawriter(
            &topic,
            QosKind::Specific(writer_qos),
            NO_LISTENER,
            NO_STATUS,
        )
        .unwrap();

    let subscriber = participant
        .create_subscriber(QosKind::Default, NO_LISTENER, NO_STATUS)
        .unwrap();
    let reader_qos = DataReaderQos {
        reliability: ReliabilityQosPolicy {
            kind: ReliabilityQosPolicyKind::Reliable,
            max_blocking_time: DurationKind::Finite(Duration::new(1, 0)),
        },
        history: HistoryQosPolicy {
            kind: HistoryQosPolicyKind::KeepAll,
        },
        ..Default::default()
    };

    let reader = subscriber
        .create_datareader::<KeyedData>(
            &topic,
            QosKind::Specific(reader_qos),
            NO_LISTENER,
            NO_STATUS,
        )
        .unwrap();

    let cond = writer.get_statuscondition();
    cond.set_enabled_statuses(&[StatusKind::PublicationMatched])
        .unwrap();

    let mut wait_set = WaitSet::new();
    wait_set
        .attach_condition(Condition::StatusCondition(cond))
        .unwrap();
    wait_set.wait(Duration::new(10, 0)).unwrap();

    let data1 = KeyedData { id: 1, value: 1 };

    writer.write(data1.clone(), None).unwrap();
    writer.dispose(data1, None).unwrap();

    writer
        .wait_for_acknowledgments(Duration::new(10, 0))
        .unwrap();

    let samples = reader
        .read(2, ANY_SAMPLE_STATE, ANY_VIEW_STATE, ANY_INSTANCE_STATE)
        .unwrap();

    assert_eq!(samples.len(), 2);
    assert_eq!(
        samples[0].sample_info.instance_state,
        InstanceStateKind::NotAliveDisposed
    );
    assert_eq!(
        samples[1].sample_info.instance_state,
        InstanceStateKind::NotAliveDisposed
    );
}

#[test]
#[ignore = "Dispose message not always sent after timing changes"]
fn write_read_disposed_samples_when_writer_is_immediately_deleted() {
    let domain_id = TEST_DOMAIN_ID_GENERATOR.generate_unique_domain_id();
    let participant_factory = DomainParticipantFactory::get_instance();

    let participant1 = participant_factory
        .create_participant(domain_id, QosKind::Default, NO_LISTENER, NO_STATUS)
        .unwrap();

    let participant2 = participant_factory
        .create_participant(domain_id, QosKind::Default, NO_LISTENER, NO_STATUS)
        .unwrap();

    let topic1 = participant1
        .create_topic::<KeyedData>(
            "MyTopic",
            "KeyedData",
            QosKind::Default,
            NO_LISTENER,
            NO_STATUS,
        )
        .unwrap();

    let publisher = participant1
        .create_publisher(QosKind::Default, NO_LISTENER, NO_STATUS)
        .unwrap();
    let writer_qos = DataWriterQos {
        history: HistoryQosPolicy {
            kind: HistoryQosPolicyKind::KeepAll,
        },
        ..Default::default()
    };
    let writer = publisher
        .create_datawriter(
            &topic1,
            QosKind::Specific(writer_qos),
            NO_LISTENER,
            NO_STATUS,
        )
        .unwrap();

    let topic2 = participant2
        .create_topic::<KeyedData>(
            "MyTopic",
            "KeyedData",
            QosKind::Default,
            NO_LISTENER,
            NO_STATUS,
        )
        .unwrap();

    let subscriber = participant2
        .create_subscriber(QosKind::Default, NO_LISTENER, NO_STATUS)
        .unwrap();
    let reader_qos = DataReaderQos {
        history: HistoryQosPolicy {
            kind: HistoryQosPolicyKind::KeepAll,
        },
        ..Default::default()
    };

    let reader = subscriber
        .create_datareader::<KeyedData>(
            &topic2,
            QosKind::Specific(reader_qos),
            NO_LISTENER,
            NO_STATUS,
        )
        .unwrap();

    let cond = writer.get_statuscondition();
    cond.set_enabled_statuses(&[StatusKind::PublicationMatched])
        .unwrap();

    let mut wait_set = WaitSet::new();
    wait_set
        .attach_condition(Condition::StatusCondition(cond))
        .unwrap();
    wait_set.wait(Duration::new(10, 0)).unwrap();

    let data1 = KeyedData { id: 1, value: 1 };

    writer.write(data1.clone(), None).unwrap();
    writer.dispose(data1, None).unwrap();
    publisher.delete_datawriter(&writer).unwrap();

    let start_loop_time = std::time::Instant::now();
    let wait_for_disposed_timeout = std::time::Duration::from_secs(10);
    let samples = loop {
        if let Ok(samples) = reader.read(2, ANY_SAMPLE_STATE, ANY_VIEW_STATE, ANY_INSTANCE_STATE) {
            if samples.len() == 2 {
                break samples;
            }
        }

        if start_loop_time.elapsed() > wait_for_disposed_timeout {
            panic!("Disposed sample not received within expected time.")
        }
    };

    assert_eq!(samples.len(), 2);
    assert_eq!(
        samples[0].sample_info.instance_state,
        InstanceStateKind::NotAliveDisposed
    );
    assert_eq!(
        samples[1].sample_info.instance_state,
        InstanceStateKind::NotAliveDisposed
    );
}

#[test]
fn write_read_sample_view_state() {
    let domain_id = TEST_DOMAIN_ID_GENERATOR.generate_unique_domain_id();
    let participant_factory = DomainParticipantFactory::get_instance();

    let participant = participant_factory
        .create_participant(domain_id, QosKind::Default, NO_LISTENER, NO_STATUS)
        .unwrap();

    let topic = participant
        .create_topic::<KeyedData>(
            "OtherTopic",
            "KeyedData",
            QosKind::Default,
            NO_LISTENER,
            NO_STATUS,
        )
        .unwrap();

    let publisher = participant
        .create_publisher(QosKind::Default, NO_LISTENER, NO_STATUS)
        .unwrap();
    let writer_qos = DataWriterQos {
        reliability: ReliabilityQosPolicy {
            kind: ReliabilityQosPolicyKind::Reliable,
            max_blocking_time: DurationKind::Finite(Duration::new(1, 0)),
        },
        ..Default::default()
    };
    let writer = publisher
        .create_datawriter(
            &topic,
            QosKind::Specific(writer_qos),
            NO_LISTENER,
            NO_STATUS,
        )
        .unwrap();

    let subscriber = participant
        .create_subscriber(QosKind::Default, NO_LISTENER, NO_STATUS)
        .unwrap();
    let reader_qos = DataReaderQos {
        reliability: ReliabilityQosPolicy {
            kind: ReliabilityQosPolicyKind::Reliable,
            max_blocking_time: DurationKind::Finite(Duration::new(1, 0)),
        },
        ..Default::default()
    };
    let reader = subscriber
        .create_datareader::<KeyedData>(
            &topic,
            QosKind::Specific(reader_qos),
            NO_LISTENER,
            NO_STATUS,
        )
        .unwrap();

    let cond = writer.get_statuscondition();
    cond.set_enabled_statuses(&[StatusKind::PublicationMatched])
        .unwrap();

    let mut wait_set = WaitSet::new();
    wait_set
        .attach_condition(Condition::StatusCondition(cond))
        .unwrap();
    wait_set.wait(Duration::new(5, 0)).unwrap();

    let data1 = KeyedData { id: 1, value: 1 };

    writer.write(data1, None).unwrap();

    writer
        .wait_for_acknowledgments(Duration::new(10, 0))
        .unwrap();

    reader
        .read(1, ANY_SAMPLE_STATE, ANY_VIEW_STATE, ANY_INSTANCE_STATE)
        .unwrap();

    let data1_2 = KeyedData { id: 1, value: 2 };
    let data2 = KeyedData { id: 2, value: 1 };

    writer.write(data1_2, None).unwrap();
    writer.write(data2, None).unwrap();

    writer
        .wait_for_acknowledgments(Duration::new(2, 0))
        .unwrap();

    let samples = reader
        .read(2, ANY_SAMPLE_STATE, ANY_VIEW_STATE, ANY_INSTANCE_STATE)
        .unwrap();

    assert_eq!(samples.len(), 2);
    assert_eq!(samples[0].sample_info.view_state, ViewStateKind::NotNew);
    assert_eq!(samples[1].sample_info.view_state, ViewStateKind::New);
}

#[test]
fn reader_with_minimum_time_separation_qos() {
    let domain_id = TEST_DOMAIN_ID_GENERATOR.generate_unique_domain_id();

    let participant = DomainParticipantFactory::get_instance()
        .create_participant(domain_id, QosKind::Default, NO_LISTENER, NO_STATUS)
        .unwrap();

    let topic = participant
        .create_topic::<KeyedData>(
            "MyTopic",
            "KeyedData",
            QosKind::Default,
            NO_LISTENER,
            NO_STATUS,
        )
        .unwrap();

    let publisher = participant
        .create_publisher(QosKind::Default, NO_LISTENER, NO_STATUS)
        .unwrap();
    let writer_qos = DataWriterQos {
        history: HistoryQosPolicy {
            kind: HistoryQosPolicyKind::KeepAll,
        },
        reliability: ReliabilityQosPolicy {
            kind: ReliabilityQosPolicyKind::Reliable,
            max_blocking_time: DurationKind::Finite(Duration::new(1, 0)),
        },
        ..Default::default()
    };
    let writer = publisher
        .create_datawriter(
            &topic,
            QosKind::Specific(writer_qos),
            NO_LISTENER,
            NO_STATUS,
        )
        .unwrap();

    let subscriber = participant
        .create_subscriber(QosKind::Default, NO_LISTENER, NO_STATUS)
        .unwrap();
    let reader_qos = DataReaderQos {
        history: HistoryQosPolicy {
            kind: HistoryQosPolicyKind::KeepAll,
        },
        reliability: ReliabilityQosPolicy {
            kind: ReliabilityQosPolicyKind::Reliable,
            max_blocking_time: DurationKind::Finite(Duration::new(1, 0)),
        },
        time_based_filter: TimeBasedFilterQosPolicy {
            minimum_separation: DurationKind::Finite(Duration::new(2, 0)),
        },
        ..Default::default()
    };
    let reader = subscriber
        .create_datareader::<KeyedData>(
            &topic,
            QosKind::Specific(reader_qos),
            NO_LISTENER,
            NO_STATUS,
        )
        .unwrap();

    let cond = writer.get_statuscondition();
    cond.set_enabled_statuses(&[StatusKind::PublicationMatched])
        .unwrap();

    let mut wait_set = WaitSet::new();
    wait_set
        .attach_condition(Condition::StatusCondition(cond))
        .unwrap();
    wait_set.wait(Duration::new(10, 0)).unwrap();

    let data1_1 = KeyedData { id: 1, value: 1 };
    let data1_2 = KeyedData { id: 1, value: 2 };
    let data1_3 = KeyedData { id: 1, value: 3 };
    let data2_1 = KeyedData { id: 2, value: 10 };
    let data2_2 = KeyedData { id: 2, value: 20 };
    let data2_3 = KeyedData { id: 2, value: 30 };

    writer
        .write_w_timestamp(data1_1.clone(), None, Time::new(1, 0))
        .unwrap();
    writer
        .write_w_timestamp(data1_2, None, Time::new(2, 0))
        .unwrap();
    writer
        .write_w_timestamp(data1_3.clone(), None, Time::new(3, 0))
        .unwrap();
    writer
        .write_w_timestamp(data2_1.clone(), None, Time::new(4, 0))
        .unwrap();
    writer
        .write_w_timestamp(data2_2, None, Time::new(5, 0))
        .unwrap();
    writer
        .write_w_timestamp(data2_3.clone(), None, Time::new(6, 0))
        .unwrap();

    writer
        .wait_for_acknowledgments(Duration::new(10, 0))
        .unwrap();

    let samples = reader
        .read(10, ANY_SAMPLE_STATE, ANY_VIEW_STATE, ANY_INSTANCE_STATE)
        .unwrap();

    assert_eq!(samples.len(), 4);
    assert_eq!(samples[0].data.as_ref().unwrap(), &data1_1);
    assert_eq!(samples[1].data.as_ref().unwrap(), &data1_3);
    assert_eq!(samples[2].data.as_ref().unwrap(), &data2_1);
    assert_eq!(samples[3].data.as_ref().unwrap(), &data2_3);
}

#[test]
fn reader_with_minimum_time_separation_qos_taking_samples() {
    let domain_id = TEST_DOMAIN_ID_GENERATOR.generate_unique_domain_id();

    let participant = DomainParticipantFactory::get_instance()
        .create_participant(domain_id, QosKind::Default, NO_LISTENER, NO_STATUS)
        .unwrap();

    let topic = participant
        .create_topic::<KeyedData>(
            "MyTopic",
            "KeyedData",
            QosKind::Default,
            NO_LISTENER,
            NO_STATUS,
        )
        .unwrap();

    let publisher = participant
        .create_publisher(QosKind::Default, NO_LISTENER, NO_STATUS)
        .unwrap();
    let writer_qos = DataWriterQos {
        history: HistoryQosPolicy {
            kind: HistoryQosPolicyKind::KeepAll,
        },
        reliability: ReliabilityQosPolicy {
            kind: ReliabilityQosPolicyKind::Reliable,
            max_blocking_time: DurationKind::Finite(Duration::new(1, 0)),
        },
        ..Default::default()
    };
    let writer = publisher
        .create_datawriter(
            &topic,
            QosKind::Specific(writer_qos),
            NO_LISTENER,
            NO_STATUS,
        )
        .unwrap();

    let subscriber = participant
        .create_subscriber(QosKind::Default, NO_LISTENER, NO_STATUS)
        .unwrap();
    let reader_qos = DataReaderQos {
        history: HistoryQosPolicy {
            kind: HistoryQosPolicyKind::KeepAll,
        },
        reliability: ReliabilityQosPolicy {
            kind: ReliabilityQosPolicyKind::Reliable,
            max_blocking_time: DurationKind::Finite(Duration::new(1, 0)),
        },
        time_based_filter: TimeBasedFilterQosPolicy {
            minimum_separation: DurationKind::Finite(Duration::new(2, 0)),
        },
        ..Default::default()
    };
    let reader = subscriber
        .create_datareader::<KeyedData>(
            &topic,
            QosKind::Specific(reader_qos),
            NO_LISTENER,
            NO_STATUS,
        )
        .unwrap();

    let cond = writer.get_statuscondition();
    cond.set_enabled_statuses(&[StatusKind::PublicationMatched])
        .unwrap();

    let mut wait_set = WaitSet::new();
    wait_set
        .attach_condition(Condition::StatusCondition(cond))
        .unwrap();
    wait_set.wait(Duration::new(10, 0)).unwrap();

    let data1_1 = KeyedData { id: 1, value: 1 };
    let data1_2 = KeyedData { id: 1, value: 2 };
    let data1_3 = KeyedData { id: 1, value: 3 };

    writer
        .write_w_timestamp(data1_1.clone(), None, Time::new(1, 0))
        .unwrap();
    writer
        .wait_for_acknowledgments(Duration::new(10, 0))
        .unwrap();

    let samples = reader
        .take(10, ANY_SAMPLE_STATE, ANY_VIEW_STATE, ANY_INSTANCE_STATE)
        .unwrap();
    assert_eq!(samples.len(), 1);
    assert_eq!(samples[0].data.as_ref().unwrap(), &data1_1);

    writer
        .write_w_timestamp(data1_2, None, Time::new(2, 0))
        .unwrap();
    writer
        .wait_for_acknowledgments(Duration::new(10, 0))
        .unwrap();

    let samples = reader.take(10, ANY_SAMPLE_STATE, ANY_VIEW_STATE, ANY_INSTANCE_STATE);
    assert_eq!(samples, Err(DdsError::NoData));

    writer
        .write_w_timestamp(data1_3.clone(), None, Time::new(3, 0))
        .unwrap();
    writer
        .wait_for_acknowledgments(Duration::new(10, 0))
        .unwrap();

    let samples = reader
        .take(10, ANY_SAMPLE_STATE, ANY_VIEW_STATE, ANY_INSTANCE_STATE)
        .unwrap();
    assert_eq!(samples.len(), 1);
    assert_eq!(samples[0].data.as_ref().unwrap(), &data1_3);
}

#[test]
fn transient_local_writer_reader_wait_for_historical_data() {
    let domain_id = TEST_DOMAIN_ID_GENERATOR.generate_unique_domain_id();

    let participant = DomainParticipantFactory::get_instance()
        .create_participant(domain_id, QosKind::Default, NO_LISTENER, NO_STATUS)
        .unwrap();

    let topic = participant
        .create_topic::<KeyedData>(
            "MyTopic",
            "KeyedData",
            QosKind::Default,
            NO_LISTENER,
            NO_STATUS,
        )
        .unwrap();

    let publisher = participant
        .create_publisher(QosKind::Default, NO_LISTENER, NO_STATUS)
        .unwrap();
    let writer_qos = DataWriterQos {
        durability: DurabilityQosPolicy {
            kind: DurabilityQosPolicyKind::TransientLocal,
        },
        history: HistoryQosPolicy {
            kind: HistoryQosPolicyKind::KeepAll,
        },
        reliability: ReliabilityQosPolicy {
            kind: ReliabilityQosPolicyKind::Reliable,
            max_blocking_time: DurationKind::Finite(Duration::new(1, 0)),
        },
        ..Default::default()
    };
    let writer = publisher
        .create_datawriter(
            &topic,
            QosKind::Specific(writer_qos),
            NO_LISTENER,
            NO_STATUS,
        )
        .unwrap();
    let data1 = KeyedData { id: 1, value: 1 };
    let data2 = KeyedData { id: 2, value: 2 };
    writer.write(data1.clone(), None).unwrap();
    writer.write(data2.clone(), None).unwrap();

    let subscriber = participant
        .create_subscriber(QosKind::Default, NO_LISTENER, NO_STATUS)
        .unwrap();
    let reader_qos = DataReaderQos {
        durability: DurabilityQosPolicy {
            kind: DurabilityQosPolicyKind::TransientLocal,
        },
        history: HistoryQosPolicy {
            kind: HistoryQosPolicyKind::KeepAll,
        },
        reliability: ReliabilityQosPolicy {
            kind: ReliabilityQosPolicyKind::Reliable,
            max_blocking_time: DurationKind::Finite(Duration::new(1, 0)),
        },
        time_based_filter: TimeBasedFilterQosPolicy {
            minimum_separation: DurationKind::Finite(Duration::new(2, 0)),
        },
        ..Default::default()
    };

    let reader = subscriber
        .create_datareader::<KeyedData>(
            &topic,
            QosKind::Specific(reader_qos),
            NO_LISTENER,
            NO_STATUS,
        )
        .unwrap();

    let cond = reader.get_statuscondition();
    cond.set_enabled_statuses(&[StatusKind::SubscriptionMatched])
        .unwrap();

    let mut wait_set = WaitSet::new();
    wait_set
        .attach_condition(Condition::StatusCondition(cond))
        .unwrap();
    wait_set.wait(Duration::new(5, 0)).unwrap();

    reader
        .wait_for_historical_data(Duration::new(10, 0))
        .unwrap();
    let samples = reader
        .read(10, ANY_SAMPLE_STATE, ANY_VIEW_STATE, ANY_INSTANCE_STATE)
        .unwrap();

    assert_eq!(samples.len(), 2);
    assert_eq!(samples[0].data.as_ref().unwrap(), &data1);
    assert_eq!(samples[1].data.as_ref().unwrap(), &data2);
}

#[test]
fn volatile_writer_reader_receives_only_new_samples() {
    let domain_id = TEST_DOMAIN_ID_GENERATOR.generate_unique_domain_id();

    let participant = DomainParticipantFactory::get_instance()
        .create_participant(domain_id, QosKind::Default, NO_LISTENER, NO_STATUS)
        .unwrap();

    let topic = participant
        .create_topic::<KeyedData>(
            "MyTopic",
            "KeyedData",
            QosKind::Default,
            NO_LISTENER,
            NO_STATUS,
        )
        .unwrap();

    let publisher = participant
        .create_publisher(QosKind::Default, NO_LISTENER, NO_STATUS)
        .unwrap();
    let writer_qos = DataWriterQos {
        durability: DurabilityQosPolicy {
            kind: DurabilityQosPolicyKind::Volatile,
        },
        history: HistoryQosPolicy {
            kind: HistoryQosPolicyKind::KeepAll,
        },
        reliability: ReliabilityQosPolicy {
            kind: ReliabilityQosPolicyKind::Reliable,
            max_blocking_time: DurationKind::Finite(Duration::new(1, 0)),
        },
        ..Default::default()
    };
    let writer = publisher
        .create_datawriter(
            &topic,
            QosKind::Specific(writer_qos),
            NO_LISTENER,
            NO_STATUS,
        )
        .unwrap();

    let data1 = KeyedData { id: 1, value: 1 };
    writer.write(data1, None).unwrap();

    let subscriber = participant
        .create_subscriber(QosKind::Default, NO_LISTENER, NO_STATUS)
        .unwrap();
    let reader_qos = DataReaderQos {
        durability: DurabilityQosPolicy {
            kind: DurabilityQosPolicyKind::Volatile,
        },
        history: HistoryQosPolicy {
            kind: HistoryQosPolicyKind::KeepAll,
        },
        reliability: ReliabilityQosPolicy {
            kind: ReliabilityQosPolicyKind::Reliable,
            max_blocking_time: DurationKind::Finite(Duration::new(1, 0)),
        },
        ..Default::default()
    };
    let reader = subscriber
        .create_datareader::<KeyedData>(
            &topic,
            QosKind::Specific(reader_qos),
            NO_LISTENER,
            NO_STATUS,
        )
        .unwrap();

    let cond = writer.get_statuscondition();
    cond.set_enabled_statuses(&[StatusKind::PublicationMatched])
        .unwrap();

    let mut wait_set = WaitSet::new();
    wait_set
        .attach_condition(Condition::StatusCondition(cond))
        .unwrap();
    wait_set.wait(Duration::new(5, 0)).unwrap();

    let data2 = KeyedData { id: 2, value: 10 };
    writer.write(data2.clone(), None).unwrap();

    writer
        .wait_for_acknowledgments(Duration::new(10, 0))
        .unwrap();

    let samples = reader
        .read(10, ANY_SAMPLE_STATE, ANY_VIEW_STATE, ANY_INSTANCE_STATE)
        .unwrap();

    assert_eq!(samples.len(), 1);
    assert_eq!(samples[0].data.as_ref().unwrap(), &data2);
}

#[test]
fn write_read_unkeyed_topic() {
    let domain_id = TEST_DOMAIN_ID_GENERATOR.generate_unique_domain_id();
    let participant_factory = DomainParticipantFactory::get_instance();

    let participant = participant_factory
        .create_participant(domain_id, QosKind::Default, NO_LISTENER, NO_STATUS)
        .unwrap();

    let topic = participant
        .create_topic::<UserData>(
            "write_read_unkeyed_topic",
            "UserData",
            QosKind::Default,
            NO_LISTENER,
            NO_STATUS,
        )
        .unwrap();

    let publisher = participant
        .create_publisher(QosKind::Default, NO_LISTENER, NO_STATUS)
        .unwrap();
    let writer_qos = DataWriterQos {
        reliability: ReliabilityQosPolicy {
            kind: ReliabilityQosPolicyKind::Reliable,
            max_blocking_time: DurationKind::Finite(Duration::new(1, 0)),
        },
        ..Default::default()
    };
    let writer = publisher
        .create_datawriter(
            &topic,
            QosKind::Specific(writer_qos),
            NO_LISTENER,
            NO_STATUS,
        )
        .unwrap();

    let subscriber = participant
        .create_subscriber(QosKind::Default, NO_LISTENER, NO_STATUS)
        .unwrap();
    let reader_qos = DataReaderQos {
        reliability: ReliabilityQosPolicy {
            kind: ReliabilityQosPolicyKind::Reliable,
            max_blocking_time: DurationKind::Finite(Duration::new(1, 0)),
        },
        ..Default::default()
    };
    let reader = subscriber
        .create_datareader::<UserData>(
            &topic,
            QosKind::Specific(reader_qos),
            NO_LISTENER,
            NO_STATUS,
        )
        .unwrap();

    let cond = writer.get_statuscondition();
    cond.set_enabled_statuses(&[StatusKind::PublicationMatched])
        .unwrap();

    let mut wait_set = WaitSet::new();
    wait_set
        .attach_condition(Condition::StatusCondition(cond))
        .unwrap();
    wait_set.wait(Duration::new(10, 0)).unwrap();

    writer.write(UserData(8), None).unwrap();

    writer
        .wait_for_acknowledgments(Duration::new(5, 0))
        .unwrap();

    let samples = reader.read(1, ANY_SAMPLE_STATE, ANY_VIEW_STATE, ANY_INSTANCE_STATE);

    assert_eq!(samples.unwrap()[0].data.as_ref().unwrap(), &UserData(8));
}

#[test]
fn data_reader_resource_limits() {
    let domain_id = TEST_DOMAIN_ID_GENERATOR.generate_unique_domain_id();
    let participant_factory = DomainParticipantFactory::get_instance();

    let participant = participant_factory
        .create_participant(domain_id, QosKind::Default, NO_LISTENER, NO_STATUS)
        .unwrap();

    let topic = participant
        .create_topic::<UserData>(
            "data_reader_resource_limits",
            "UserData",
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
            kind: ReliabilityQosPolicyKind::Reliable,
            max_blocking_time: DurationKind::Finite(Duration::new(1, 0)),
        },
        history: HistoryQosPolicy {
            kind: HistoryQosPolicyKind::KeepAll,
        },
        ..Default::default()
    };
    let writer = publisher
        .create_datawriter(
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
            kind: ReliabilityQosPolicyKind::Reliable,
            max_blocking_time: DurationKind::Finite(Duration::new(1, 0)),
        },
        history: HistoryQosPolicy {
            kind: HistoryQosPolicyKind::KeepAll,
        },
        resource_limits: ResourceLimitsQosPolicy {
            max_samples: Length::Limited(2),
            max_instances: Length::Unlimited,
            max_samples_per_instance: Length::Limited(2),
        },
        ..Default::default()
    };
    let reader = subscriber
        .create_datareader::<UserData>(
            &topic,
            QosKind::Specific(reader_qos),
            NO_LISTENER,
            NO_STATUS,
        )
        .unwrap();

    let cond = writer.get_statuscondition();
    cond.set_enabled_statuses(&[StatusKind::PublicationMatched])
        .unwrap();

    let mut wait_set = WaitSet::new();
    wait_set
        .attach_condition(Condition::StatusCondition(cond))
        .unwrap();
    wait_set.wait(Duration::new(10, 0)).unwrap();

    writer.write(UserData(1), None).unwrap();
    writer.write(UserData(2), None).unwrap();
    writer.write(UserData(3), None).unwrap();

    let reader_cond = reader.get_statuscondition();
    reader_cond
        .set_enabled_statuses(&[StatusKind::SampleRejected])
        .unwrap();
    let mut wait_set = WaitSet::new();
    wait_set
        .attach_condition(Condition::StatusCondition(reader_cond))
        .unwrap();

    wait_set.wait(Duration::new(5, 0)).unwrap();

    let samples = reader
        .read(3, ANY_SAMPLE_STATE, ANY_VIEW_STATE, ANY_INSTANCE_STATE)
        .unwrap();

    assert_eq!(samples.len(), 2);
}

#[test]
fn data_reader_order_by_source_timestamp() {
    let domain_id = TEST_DOMAIN_ID_GENERATOR.generate_unique_domain_id();
    let participant_factory = DomainParticipantFactory::get_instance();

    let participant = participant_factory
        .create_participant(domain_id, QosKind::Default, NO_LISTENER, NO_STATUS)
        .unwrap();

    let topic = participant
        .create_topic::<UserData>(
            "MyTopic",
            "UserData",
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
            kind: ReliabilityQosPolicyKind::Reliable,
            max_blocking_time: DurationKind::Finite(Duration::new(1, 0)),
        },
        destination_order: DestinationOrderQosPolicy {
            kind: DestinationOrderQosPolicyKind::BySourceTimestamp,
        },
        history: HistoryQosPolicy {
            kind: HistoryQosPolicyKind::KeepAll,
        },
        ..Default::default()
    };
    let writer = publisher
        .create_datawriter(
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
            kind: ReliabilityQosPolicyKind::Reliable,
            max_blocking_time: DurationKind::Finite(Duration::new(1, 0)),
        },
        history: HistoryQosPolicy {
            kind: HistoryQosPolicyKind::KeepAll,
        },
        destination_order: DestinationOrderQosPolicy {
            kind: DestinationOrderQosPolicyKind::BySourceTimestamp,
        },
        ..Default::default()
    };
    let reader = subscriber
        .create_datareader::<UserData>(
            &topic,
            QosKind::Specific(reader_qos),
            NO_LISTENER,
            NO_STATUS,
        )
        .unwrap();

    let cond = writer.get_statuscondition();
    cond.set_enabled_statuses(&[StatusKind::PublicationMatched])
        .unwrap();

    let mut wait_set = WaitSet::new();
    wait_set
        .attach_condition(Condition::StatusCondition(cond))
        .unwrap();
    wait_set.wait(Duration::new(10, 0)).unwrap();

    writer
        .write_w_timestamp(UserData(1), None, Time::new(30, 0))
        .unwrap();
    writer
        .write_w_timestamp(UserData(2), None, Time::new(20, 0))
        .unwrap();
    writer
        .write_w_timestamp(UserData(3), None, Time::new(10, 0))
        .unwrap();

    writer
        .wait_for_acknowledgments(Duration::new(10, 0))
        .unwrap();

    let samples = reader
        .read(3, ANY_SAMPLE_STATE, ANY_VIEW_STATE, ANY_INSTANCE_STATE)
        .unwrap();

    assert_eq!(samples.len(), 3);
    assert_eq!(samples[0].data.as_ref().unwrap(), &UserData(3));
    assert_eq!(samples[1].data.as_ref().unwrap(), &UserData(2));
    assert_eq!(samples[2].data.as_ref().unwrap(), &UserData(1));
}

#[test]
fn data_reader_publication_handle_sample_info() {
    let domain_id = TEST_DOMAIN_ID_GENERATOR.generate_unique_domain_id();
    let participant_factory = DomainParticipantFactory::get_instance();

    let participant = participant_factory
        .create_participant(domain_id, QosKind::Default, NO_LISTENER, NO_STATUS)
        .unwrap();

    let topic = participant
        .create_topic::<UserData>(
            "MyTopic",
            "UserData",
            QosKind::Default,
            NO_LISTENER,
            NO_STATUS,
        )
        .unwrap();

    let publisher = participant
        .create_publisher(QosKind::Default, NO_LISTENER, NO_STATUS)
        .unwrap();

    let writer_qos = DataWriterQos {
        reliability: ReliabilityQosPolicy {
            kind: ReliabilityQosPolicyKind::Reliable,
            max_blocking_time: DurationKind::Finite(Duration::new(1, 0)),
        },
        ..Default::default()
    };
    let writer = publisher
        .create_datawriter(
            &topic,
            QosKind::Specific(writer_qos),
            NO_LISTENER,
            NO_STATUS,
        )
        .unwrap();

    let subscriber = participant
        .create_subscriber(QosKind::Default, NO_LISTENER, NO_STATUS)
        .unwrap();

    let reader_qos = DataReaderQos {
        reliability: ReliabilityQosPolicy {
            kind: ReliabilityQosPolicyKind::Reliable,
            max_blocking_time: DurationKind::Finite(Duration::new(1, 0)),
        },
        ..Default::default()
    };
    let reader = subscriber
        .create_datareader::<UserData>(
            &topic,
            QosKind::Specific(reader_qos),
            NO_LISTENER,
            NO_STATUS,
        )
        .unwrap();

    let cond = writer.get_statuscondition();
    cond.set_enabled_statuses(&[StatusKind::PublicationMatched])
        .unwrap();

    let mut wait_set = WaitSet::new();
    wait_set
        .attach_condition(Condition::StatusCondition(cond))
        .unwrap();
    wait_set.wait(Duration::new(10, 0)).unwrap();

    writer.write(UserData(1), None).unwrap();

    writer
        .wait_for_acknowledgments(Duration::new(10, 0))
        .unwrap();

    let samples = reader
        .read(1, ANY_SAMPLE_STATE, ANY_VIEW_STATE, ANY_INSTANCE_STATE)
        .unwrap();

    assert_eq!(samples.len(), 1);
    assert!(
        reader
            .get_matched_publication_data(samples[0].sample_info.publication_handle)
            .is_ok()
    );
}

#[test]
fn volatile_writer_with_reader_new_reader_receives_only_new_samples() {
    let domain_id = TEST_DOMAIN_ID_GENERATOR.generate_unique_domain_id();

    let participant = DomainParticipantFactory::get_instance()
        .create_participant(domain_id, QosKind::Default, NO_LISTENER, NO_STATUS)
        .unwrap();

    let topic = participant
        .create_topic::<KeyedData>(
            "MyTopic",
            "KeyedData",
            QosKind::Default,
            NO_LISTENER,
            NO_STATUS,
        )
        .unwrap();

    let publisher = participant
        .create_publisher(QosKind::Default, NO_LISTENER, NO_STATUS)
        .unwrap();
    let writer_qos = DataWriterQos {
        durability: DurabilityQosPolicy {
            kind: DurabilityQosPolicyKind::Volatile,
        },
        history: HistoryQosPolicy {
            kind: HistoryQosPolicyKind::KeepAll,
        },
        reliability: ReliabilityQosPolicy {
            kind: ReliabilityQosPolicyKind::Reliable,
            max_blocking_time: DurationKind::Finite(Duration::new(1, 0)),
        },
        ..Default::default()
    };
    let writer = publisher
        .create_datawriter(
            &topic,
            QosKind::Specific(writer_qos),
            NO_LISTENER,
            NO_STATUS,
        )
        .unwrap();

    let subscriber = participant
        .create_subscriber(QosKind::Default, NO_LISTENER, NO_STATUS)
        .unwrap();
    let reader_qos = DataReaderQos {
        durability: DurabilityQosPolicy {
            kind: DurabilityQosPolicyKind::Volatile,
        },
        history: HistoryQosPolicy {
            kind: HistoryQosPolicyKind::KeepAll,
        },
        reliability: ReliabilityQosPolicy {
            kind: ReliabilityQosPolicyKind::Reliable,
            max_blocking_time: DurationKind::Finite(Duration::new(1, 0)),
        },
        ..Default::default()
    };
    let _reader = subscriber
        .create_datareader::<KeyedData>(
            &topic,
            QosKind::Specific(reader_qos.clone()),
            NO_LISTENER,
            NO_STATUS,
        )
        .unwrap();

    let cond = writer.get_statuscondition();
    cond.set_enabled_statuses(&[StatusKind::PublicationMatched])
        .unwrap();

    let mut wait_set = WaitSet::new();
    wait_set
        .attach_condition(Condition::StatusCondition(cond))
        .unwrap();
    wait_set.wait(Duration::new(10, 0)).unwrap();
    writer.get_publication_matched_status().unwrap(); // To reset wait_set for subsequent calls

    let data1 = KeyedData { id: 1, value: 1 };
    writer.write(data1, None).unwrap();

    writer
        .wait_for_acknowledgments(Duration::new(3, 0))
        .unwrap();

    let reader_new = subscriber
        .create_datareader::<KeyedData>(
            &topic,
            QosKind::Specific(reader_qos),
            NO_LISTENER,
            NO_STATUS,
        )
        .unwrap();

    // Wait for writer to match reader
    wait_set.wait(Duration::new(10, 0)).unwrap();

    let data2 = KeyedData { id: 2, value: 10 };
    writer.write(data2.clone(), None).unwrap();

    writer
        .wait_for_acknowledgments(Duration::new(10, 0))
        .unwrap();

    let samples = reader_new
        .read(10, ANY_SAMPLE_STATE, ANY_VIEW_STATE, ANY_INSTANCE_STATE)
        .unwrap();

    assert_eq!(samples.len(), 1);
    assert_eq!(samples[0].data.as_ref().unwrap(), &data2);
}

#[test]
fn write_read_unregistered_samples_not_alive_no_writers() {
    let domain_id = TEST_DOMAIN_ID_GENERATOR.generate_unique_domain_id();

    let participant = DomainParticipantFactory::get_instance()
        .create_participant(domain_id, QosKind::Default, NO_LISTENER, NO_STATUS)
        .unwrap();

    let topic = participant
        .create_topic::<KeyedData>(
            "MyTopic",
            "KeyedData",
            QosKind::Default,
            NO_LISTENER,
            NO_STATUS,
        )
        .unwrap();

    let publisher = participant
        .create_publisher(QosKind::Default, NO_LISTENER, NO_STATUS)
        .unwrap();
    let writer_qos = DataWriterQos {
        reliability: ReliabilityQosPolicy {
            kind: ReliabilityQosPolicyKind::Reliable,
            max_blocking_time: DurationKind::Finite(Duration::new(1, 0)),
        },
        history: HistoryQosPolicy {
            kind: HistoryQosPolicyKind::KeepAll,
        },
        ..Default::default()
    };
    let writer = publisher
        .create_datawriter(
            &topic,
            QosKind::Specific(writer_qos),
            NO_LISTENER,
            NO_STATUS,
        )
        .unwrap();

    let subscriber = participant
        .create_subscriber(QosKind::Default, NO_LISTENER, NO_STATUS)
        .unwrap();
    let reader_qos = DataReaderQos {
        reliability: ReliabilityQosPolicy {
            kind: ReliabilityQosPolicyKind::Reliable,
            max_blocking_time: DurationKind::Finite(Duration::new(1, 0)),
        },
        history: HistoryQosPolicy {
            kind: HistoryQosPolicyKind::KeepAll,
        },
        ..Default::default()
    };

    let reader = subscriber
        .create_datareader::<KeyedData>(
            &topic,
            QosKind::Specific(reader_qos),
            NO_LISTENER,
            NO_STATUS,
        )
        .unwrap();

    let cond = writer.get_statuscondition();
    cond.set_enabled_statuses(&[StatusKind::PublicationMatched])
        .unwrap();

    let mut wait_set = WaitSet::new();
    wait_set
        .attach_condition(Condition::StatusCondition(cond))
        .unwrap();
    wait_set.wait(Duration::new(10, 0)).unwrap();

    let data1 = KeyedData { id: 1, value: 1 };

    writer.write(data1.clone(), None).unwrap();
    writer.unregister_instance(data1, None).unwrap();

    writer
        .wait_for_acknowledgments(Duration::new(10, 0))
        .unwrap();

    let samples = reader
        .read(2, ANY_SAMPLE_STATE, ANY_VIEW_STATE, ANY_INSTANCE_STATE)
        .unwrap();

    assert_eq!(samples.len(), 2);
    assert_eq!(
        samples[0].sample_info.instance_state,
        InstanceStateKind::NotAliveNoWriters
    );
    assert_eq!(
        samples[1].sample_info.instance_state,
        InstanceStateKind::NotAliveNoWriters
    );
}

#[test]
fn multiple_writers_unregister_instance() {
    let domain_id = TEST_DOMAIN_ID_GENERATOR.generate_unique_domain_id();

    let participant = DomainParticipantFactory::get_instance()
        .create_participant(domain_id, QosKind::Default, NO_LISTENER, NO_STATUS)
        .unwrap();

    let topic = participant
        .create_topic::<KeyedData>(
            "MyTopic",
            "KeyedData",
            QosKind::Default,
            NO_LISTENER,
            NO_STATUS,
        )
        .unwrap();

    let publisher = participant
        .create_publisher(QosKind::Default, NO_LISTENER, NO_STATUS)
        .unwrap();
    let writer_qos = DataWriterQos {
        reliability: ReliabilityQosPolicy {
            kind: ReliabilityQosPolicyKind::Reliable,
            max_blocking_time: DurationKind::Finite(Duration::new(1, 0)),
        },
        history: HistoryQosPolicy {
            kind: HistoryQosPolicyKind::KeepAll,
        },
        ..Default::default()
    };
    let writer1 = publisher
        .create_datawriter(
            &topic,
            QosKind::Specific(writer_qos.clone()),
            NO_LISTENER,
            NO_STATUS,
        )
        .unwrap();
    let writer2 = publisher
        .create_datawriter(
            &topic,
            QosKind::Specific(writer_qos),
            NO_LISTENER,
            NO_STATUS,
        )
        .unwrap();

    let subscriber = participant
        .create_subscriber(QosKind::Default, NO_LISTENER, NO_STATUS)
        .unwrap();
    let reader_qos = DataReaderQos {
        reliability: ReliabilityQosPolicy {
            kind: ReliabilityQosPolicyKind::Reliable,
            max_blocking_time: DurationKind::Finite(Duration::new(1, 0)),
        },
        history: HistoryQosPolicy {
            kind: HistoryQosPolicyKind::KeepAll,
        },
        ..Default::default()
    };

    let reader = subscriber
        .create_datareader::<KeyedData>(
            &topic,
            QosKind::Specific(reader_qos),
            NO_LISTENER,
            NO_STATUS,
        )
        .unwrap();

    let start_time = std::time::Instant::now();
    while std::time::Instant::now().duration_since(start_time) < std::time::Duration::from_secs(10)
    {
        if reader.get_matched_publications().unwrap().len() >= 2 {
            break;
        }
        std::thread::sleep(std::time::Duration::from_millis(20));
    }
    assert_eq!(
        reader.get_matched_publications().unwrap().len(),
        2,
        "Reader must have 2 matched writers"
    );

    let cond1 = writer1.get_statuscondition();
    cond1
        .set_enabled_statuses(&[StatusKind::PublicationMatched])
        .unwrap();
    let mut wait_set1 = WaitSet::new();
    wait_set1
        .attach_condition(Condition::StatusCondition(cond1))
        .unwrap();
    wait_set1.wait(Duration::new(10, 0)).unwrap();

    let cond2 = writer2.get_statuscondition();
    cond2
        .set_enabled_statuses(&[StatusKind::PublicationMatched])
        .unwrap();
    let mut wait_set2 = WaitSet::new();
    wait_set2
        .attach_condition(Condition::StatusCondition(cond2))
        .unwrap();
    wait_set2.wait(Duration::new(10, 0)).unwrap();

    let data1 = KeyedData { id: 1, value: 1 };

    writer1.write(data1.clone(), None).unwrap();
    writer2.write(data1.clone(), None).unwrap();
    writer1.unregister_instance(data1.clone(), None).unwrap();

    writer1
        .wait_for_acknowledgments(Duration::new(10, 0))
        .unwrap();
    writer2
        .wait_for_acknowledgments(Duration::new(10, 0))
        .unwrap();

    let samples = reader
        .read(3, ANY_SAMPLE_STATE, ANY_VIEW_STATE, ANY_INSTANCE_STATE)
        .unwrap();

    assert_eq!(samples.len(), 3);
    // Since writer1 unregisters with autodispose=true and writer2 is still registered, instance is NotAliveDisposed
    assert_eq!(
        samples[0].sample_info.instance_state,
        InstanceStateKind::NotAliveDisposed
    );
    assert_eq!(
        samples[1].sample_info.instance_state,
        InstanceStateKind::NotAliveDisposed
    );
    assert_eq!(
        samples[2].sample_info.instance_state,
        InstanceStateKind::NotAliveDisposed
    );

    // Now writer2 unregisters as well
    writer2.unregister_instance(data1, None).unwrap();
    writer2
        .wait_for_acknowledgments(Duration::new(10, 0))
        .unwrap();

    let samples2 = reader
        .read(4, ANY_SAMPLE_STATE, ANY_VIEW_STATE, ANY_INSTANCE_STATE)
        .unwrap();

    assert_eq!(samples2.len(), 4);
    // Since instance was disposed by writer1 (autodispose=true), it remains NotAliveDisposed
    assert_eq!(
        samples2[3].sample_info.instance_state,
        InstanceStateKind::NotAliveDisposed
    );
}

#[test]
fn data_reader_instance_state_not_alive_no_writers_when_writer_is_deleted() {
    let domain_id = TEST_DOMAIN_ID_GENERATOR.generate_unique_domain_id();

    let participant_factory = DomainParticipantFactory::get_instance();
    let participant1 = participant_factory
        .create_participant(domain_id, QosKind::Default, NO_LISTENER, NO_STATUS)
        .unwrap();
    let participant2 = participant_factory
        .create_participant(domain_id, QosKind::Default, NO_LISTENER, NO_STATUS)
        .unwrap();

    let topic1 = participant1
        .create_topic::<KeyedData>(
            "MyTopic",
            "KeyedData",
            QosKind::Default,
            NO_LISTENER,
            NO_STATUS,
        )
        .unwrap();
    let topic2 = participant2
        .create_topic::<KeyedData>(
            "MyTopic",
            "KeyedData",
            QosKind::Default,
            NO_LISTENER,
            NO_STATUS,
        )
        .unwrap();

    let publisher = participant1
        .create_publisher(QosKind::Default, NO_LISTENER, NO_STATUS)
        .unwrap();
    let writer_qos = DataWriterQos {
        reliability: ReliabilityQosPolicy {
            kind: ReliabilityQosPolicyKind::Reliable,
            max_blocking_time: DurationKind::Finite(Duration::new(1, 0)),
        },
        history: HistoryQosPolicy {
            kind: HistoryQosPolicyKind::KeepAll,
        },
        ..Default::default()
    };
    let writer = publisher
        .create_datawriter(
            &topic1,
            QosKind::Specific(writer_qos),
            NO_LISTENER,
            NO_STATUS,
        )
        .unwrap();

    let subscriber = participant2
        .create_subscriber(QosKind::Default, NO_LISTENER, NO_STATUS)
        .unwrap();
    let reader_qos = DataReaderQos {
        reliability: ReliabilityQosPolicy {
            kind: ReliabilityQosPolicyKind::Reliable,
            max_blocking_time: DurationKind::Finite(Duration::new(1, 0)),
        },
        history: HistoryQosPolicy {
            kind: HistoryQosPolicyKind::KeepAll,
        },
        ..Default::default()
    };
    let reader = subscriber
        .create_datareader::<KeyedData>(
            &topic2,
            QosKind::Specific(reader_qos),
            NO_LISTENER,
            NO_STATUS,
        )
        .unwrap();

    let cond = writer.get_statuscondition();
    cond.set_enabled_statuses(&[StatusKind::PublicationMatched])
        .unwrap();
    let mut wait_set = WaitSet::new();
    wait_set
        .attach_condition(Condition::StatusCondition(cond))
        .unwrap();
    wait_set.wait(Duration::new(10, 0)).unwrap();

    let data1 = KeyedData { id: 1, value: 1 };
    writer.write(data1.clone(), None).unwrap();
    writer
        .wait_for_acknowledgments(Duration::new(10, 0))
        .unwrap();

    let samples = reader
        .take(1, ANY_SAMPLE_STATE, ANY_VIEW_STATE, ANY_INSTANCE_STATE)
        .unwrap();
    assert_eq!(samples.len(), 1);
    assert_eq!(
        samples[0].sample_info.instance_state,
        InstanceStateKind::Alive
    );

    reader.get_subscription_matched_status().unwrap();

    // Delete writer / publisher / participant1
    participant1.delete_contained_entities().unwrap();
    participant_factory
        .delete_participant(&participant1)
        .unwrap();

    // Wait for subscription matched status to update or reader to detect writer deletion
    let cond_reader = reader.get_statuscondition();
    cond_reader
        .set_enabled_statuses(&[StatusKind::SubscriptionMatched])
        .unwrap();
    let mut wait_set_reader = WaitSet::new();
    wait_set_reader
        .attach_condition(Condition::StatusCondition(cond_reader))
        .unwrap();
    wait_set_reader.wait(Duration::new(10, 0)).unwrap();

    let samples_after_delete = reader
        .read(1, ANY_SAMPLE_STATE, ANY_VIEW_STATE, ANY_INSTANCE_STATE)
        .unwrap();

    assert_eq!(
        samples_after_delete[0].sample_info.instance_state,
        InstanceStateKind::NotAliveNoWriters
    );
}

#[test]
fn data_reader_instance_state_not_alive_disposed_when_writer_disposes_and_is_deleted() {
    let domain_id = TEST_DOMAIN_ID_GENERATOR.generate_unique_domain_id();

    let participant_factory = DomainParticipantFactory::get_instance();
    let participant1 = participant_factory
        .create_participant(domain_id, QosKind::Default, NO_LISTENER, NO_STATUS)
        .unwrap();
    let participant2 = participant_factory
        .create_participant(domain_id, QosKind::Default, NO_LISTENER, NO_STATUS)
        .unwrap();

    let topic1 = participant1
        .create_topic::<KeyedData>(
            "MyTopic",
            "KeyedData",
            QosKind::Default,
            NO_LISTENER,
            NO_STATUS,
        )
        .unwrap();
    let topic2 = participant2
        .create_topic::<KeyedData>(
            "MyTopic",
            "KeyedData",
            QosKind::Default,
            NO_LISTENER,
            NO_STATUS,
        )
        .unwrap();

    let publisher = participant1
        .create_publisher(QosKind::Default, NO_LISTENER, NO_STATUS)
        .unwrap();
    let writer_qos = DataWriterQos {
        reliability: ReliabilityQosPolicy {
            kind: ReliabilityQosPolicyKind::Reliable,
            max_blocking_time: DurationKind::Finite(Duration::new(1, 0)),
        },
        history: HistoryQosPolicy {
            kind: HistoryQosPolicyKind::KeepAll,
        },
        ..Default::default()
    };
    let writer = publisher
        .create_datawriter(
            &topic1,
            QosKind::Specific(writer_qos),
            NO_LISTENER,
            NO_STATUS,
        )
        .unwrap();

    let subscriber = participant2
        .create_subscriber(QosKind::Default, NO_LISTENER, NO_STATUS)
        .unwrap();
    let reader_qos = DataReaderQos {
        reliability: ReliabilityQosPolicy {
            kind: ReliabilityQosPolicyKind::Reliable,
            max_blocking_time: DurationKind::Finite(Duration::new(1, 0)),
        },
        history: HistoryQosPolicy {
            kind: HistoryQosPolicyKind::KeepAll,
        },
        ..Default::default()
    };
    let reader = subscriber
        .create_datareader::<KeyedData>(
            &topic2,
            QosKind::Specific(reader_qos),
            NO_LISTENER,
            NO_STATUS,
        )
        .unwrap();

    let cond = writer.get_statuscondition();
    cond.set_enabled_statuses(&[StatusKind::PublicationMatched])
        .unwrap();
    let mut wait_set = WaitSet::new();
    wait_set
        .attach_condition(Condition::StatusCondition(cond))
        .unwrap();
    wait_set.wait(Duration::new(10, 0)).unwrap();

    let data1 = KeyedData { id: 1, value: 1 };
    writer.write(data1.clone(), None).unwrap();
    writer
        .wait_for_acknowledgments(Duration::new(10, 0))
        .unwrap();

    let samples = reader
        .take(1, ANY_SAMPLE_STATE, ANY_VIEW_STATE, ANY_INSTANCE_STATE)
        .unwrap();
    assert_eq!(samples.len(), 1);
    assert_eq!(
        samples[0].sample_info.instance_state,
        InstanceStateKind::Alive
    );

    writer.dispose(data1, None).unwrap();
    writer
        .wait_for_acknowledgments(Duration::new(10, 0))
        .unwrap();

    reader.get_subscription_matched_status().unwrap();

    // Delete writer / publisher / participant1
    participant1.delete_contained_entities().unwrap();
    participant_factory
        .delete_participant(&participant1)
        .unwrap();

    // Wait for subscription matched status to update or reader to detect writer deletion
    let cond_reader = reader.get_statuscondition();
    cond_reader
        .set_enabled_statuses(&[StatusKind::SubscriptionMatched])
        .unwrap();
    let mut wait_set_reader = WaitSet::new();
    wait_set_reader
        .attach_condition(Condition::StatusCondition(cond_reader))
        .unwrap();
    wait_set_reader.wait(Duration::new(10, 0)).unwrap();

    let samples_after_delete = reader
        .take(1, ANY_SAMPLE_STATE, ANY_VIEW_STATE, ANY_INSTANCE_STATE)
        .unwrap();

    assert_eq!(
        samples_after_delete[0].sample_info.instance_state,
        InstanceStateKind::NotAliveDisposed
    );
}

#[test]
fn transient_local_writer_does_not_deliver_lifespan_expired_data_at_write() {
    let domain_id = TEST_DOMAIN_ID_GENERATOR.generate_unique_domain_id();

    let participant = DomainParticipantFactory::get_instance()
        .create_participant(domain_id, QosKind::Default, NO_LISTENER, NO_STATUS)
        .unwrap();

    let topic = participant
        .create_topic::<KeyedData>(
            "MyTopic",
            "KeyedData",
            QosKind::Default,
            NO_LISTENER,
            NO_STATUS,
        )
        .unwrap();

    let publisher = participant
        .create_publisher(QosKind::Default, NO_LISTENER, NO_STATUS)
        .unwrap();
    let writer_qos = DataWriterQos {
        durability: DurabilityQosPolicy {
            kind: DurabilityQosPolicyKind::TransientLocal,
        },
        history: HistoryQosPolicy {
            kind: HistoryQosPolicyKind::KeepAll,
        },
        reliability: ReliabilityQosPolicy {
            kind: ReliabilityQosPolicyKind::Reliable,
            max_blocking_time: DurationKind::Finite(Duration::new(1, 0)),
        },
        lifespan: LifespanQosPolicy {
            duration: DurationKind::Finite(Duration::new(1, 0)),
        },
        ..Default::default()
    };
    let writer = publisher
        .create_datawriter(
            &topic,
            QosKind::Specific(writer_qos),
            NO_LISTENER,
            NO_STATUS,
        )
        .unwrap();
    let data1 = KeyedData { id: 1, value: 1 };
    let data2 = KeyedData { id: 2, value: 2 };
    writer
        .write_w_timestamp(data1, None, Time::new(0, 0))
        .unwrap();
    writer
        .write_w_timestamp(data2.clone(), None, Time::new(i32::MAX, 0))
        .unwrap(); // Never stale sample

    let subscriber = participant
        .create_subscriber(QosKind::Default, NO_LISTENER, NO_STATUS)
        .unwrap();
    let reader_qos = DataReaderQos {
        durability: DurabilityQosPolicy {
            kind: DurabilityQosPolicyKind::TransientLocal,
        },
        history: HistoryQosPolicy {
            kind: HistoryQosPolicyKind::KeepAll,
        },
        reliability: ReliabilityQosPolicy {
            kind: ReliabilityQosPolicyKind::Reliable,
            max_blocking_time: DurationKind::Finite(Duration::new(1, 0)),
        },
        ..Default::default()
    };

    let reader = subscriber
        .create_datareader::<KeyedData>(
            &topic,
            QosKind::Specific(reader_qos),
            NO_LISTENER,
            NO_STATUS,
        )
        .unwrap();

    let cond = reader.get_statuscondition();
    cond.set_enabled_statuses(&[StatusKind::SubscriptionMatched])
        .unwrap();

    let mut wait_set = WaitSet::new();
    wait_set
        .attach_condition(Condition::StatusCondition(cond))
        .unwrap();
    wait_set.wait(Duration::new(5, 0)).unwrap();

    reader
        .wait_for_historical_data(Duration::new(10, 0))
        .unwrap();
    let samples = reader
        .read(10, ANY_SAMPLE_STATE, ANY_VIEW_STATE, ANY_INSTANCE_STATE)
        .unwrap();

    assert_eq!(samples.len(), 1);
    assert_eq!(samples[0].data.as_ref().unwrap(), &data2);
}

#[ignore = "Channel changes made test flaky"]
#[test]
fn transient_local_writer_does_not_deliver_lifespan_expired_data_after_write() {
    const LIFESPAN_MS: u32 = 100;
    let domain_id = TEST_DOMAIN_ID_GENERATOR.generate_unique_domain_id();

    let participant = DomainParticipantFactory::get_instance()
        .create_participant(domain_id, QosKind::Default, NO_LISTENER, NO_STATUS)
        .unwrap();

    let topic = participant
        .create_topic::<KeyedData>(
            "MyTopic",
            "KeyedData",
            QosKind::Default,
            NO_LISTENER,
            NO_STATUS,
        )
        .unwrap();

    let publisher = participant
        .create_publisher(QosKind::Default, NO_LISTENER, NO_STATUS)
        .unwrap();
    let writer_qos = DataWriterQos {
        durability: DurabilityQosPolicy {
            kind: DurabilityQosPolicyKind::TransientLocal,
        },
        history: HistoryQosPolicy {
            kind: HistoryQosPolicyKind::KeepAll,
        },
        reliability: ReliabilityQosPolicy {
            kind: ReliabilityQosPolicyKind::Reliable,
            max_blocking_time: DurationKind::Finite(Duration::new(1, 0)),
        },
        lifespan: LifespanQosPolicy {
            duration: DurationKind::Finite(Duration::new(0, LIFESPAN_MS * 1_000_000)),
        },
        ..Default::default()
    };
    let writer = publisher
        .create_datawriter(
            &topic,
            QosKind::Specific(writer_qos),
            NO_LISTENER,
            NO_STATUS,
        )
        .unwrap();
    let data1 = KeyedData { id: 1, value: 1 };
    let data2 = KeyedData { id: 2, value: 2 };
    writer.write(data1, None).unwrap();
    writer
        .write_w_timestamp(data2.clone(), None, Time::new(i32::MAX, 0))
        .unwrap(); // Never stale sample

    std::thread::sleep(std::time::Duration::from_millis(LIFESPAN_MS as u64 * 10));

    let subscriber = participant
        .create_subscriber(QosKind::Default, NO_LISTENER, NO_STATUS)
        .unwrap();
    let reader_qos = DataReaderQos {
        durability: DurabilityQosPolicy {
            kind: DurabilityQosPolicyKind::TransientLocal,
        },
        history: HistoryQosPolicy {
            kind: HistoryQosPolicyKind::KeepAll,
        },
        reliability: ReliabilityQosPolicy {
            kind: ReliabilityQosPolicyKind::Reliable,
            max_blocking_time: DurationKind::Finite(Duration::new(1, 0)),
        },
        ..Default::default()
    };

    let reader = subscriber
        .create_datareader::<KeyedData>(
            &topic,
            QosKind::Specific(reader_qos),
            NO_LISTENER,
            NO_STATUS,
        )
        .unwrap();

    let cond = reader.get_statuscondition();
    cond.set_enabled_statuses(&[StatusKind::SubscriptionMatched])
        .unwrap();

    let mut wait_set = WaitSet::new();
    wait_set
        .attach_condition(Condition::StatusCondition(cond))
        .unwrap();
    wait_set.wait(Duration::new(5, 0)).unwrap();

    reader
        .wait_for_historical_data(Duration::new(10, 0))
        .unwrap();
    let samples = reader
        .read(10, ANY_SAMPLE_STATE, ANY_VIEW_STATE, ANY_INSTANCE_STATE)
        .unwrap();

    assert_eq!(samples.len(), 1);
    assert_eq!(samples[0].data.as_ref().unwrap(), &data2);
}

#[test]
fn reader_joining_after_writer_writes_many_samples() {
    let domain_id = TEST_DOMAIN_ID_GENERATOR.generate_unique_domain_id();

    let participant = DomainParticipantFactory::get_instance()
        .create_participant(domain_id, QosKind::Default, NO_LISTENER, NO_STATUS)
        .unwrap();

    let topic = participant
        .create_topic::<KeyedData>(
            "MyTopic",
            "KeyedData",
            QosKind::Default,
            NO_LISTENER,
            NO_STATUS,
        )
        .unwrap();

    let publisher = participant
        .create_publisher(QosKind::Default, NO_LISTENER, NO_STATUS)
        .unwrap();
    let writer_qos = DataWriterQos {
        reliability: ReliabilityQosPolicy {
            kind: ReliabilityQosPolicyKind::Reliable,
            max_blocking_time: DurationKind::Finite(Duration::new(5, 0)),
        },
        ..Default::default()
    };

    let writer = publisher
        .create_datawriter(
            &topic,
            QosKind::Specific(writer_qos),
            NO_LISTENER,
            NO_STATUS,
        )
        .unwrap();

    let subscriber = participant
        .create_subscriber(QosKind::Default, NO_LISTENER, NO_STATUS)
        .unwrap();

    for value in 0..500 {
        let data = KeyedData { id: 1, value };
        writer.write(data, None).unwrap();
    }

    let reader_qos = DataReaderQos {
        reliability: ReliabilityQosPolicy {
            kind: ReliabilityQosPolicyKind::Reliable,
            max_blocking_time: DurationKind::Finite(Duration::new(5, 0)),
        },
        ..Default::default()
    };
    let reader = subscriber
        .create_datareader::<KeyedData>(
            &topic,
            QosKind::Specific(reader_qos),
            NO_LISTENER,
            NO_STATUS,
        )
        .unwrap();

    let cond = writer.get_statuscondition();
    cond.set_enabled_statuses(&[StatusKind::PublicationMatched])
        .unwrap();

    let mut wait_set = WaitSet::new();
    wait_set
        .attach_condition(Condition::StatusCondition(cond))
        .unwrap();
    wait_set.wait(Duration::new(10, 0)).unwrap();

    let reader_cond = reader.get_statuscondition();
    reader_cond
        .set_enabled_statuses(&[StatusKind::SubscriptionMatched])
        .unwrap();
    let mut reader_wait_set = WaitSet::new();
    reader_wait_set
        .attach_condition(Condition::StatusCondition(reader_cond))
        .unwrap();
    reader_wait_set.wait(Duration::new(10, 0)).unwrap();

    let new_data = KeyedData { id: 1, value: 1000 };
    writer.write(new_data.clone(), None).unwrap();
    writer
        .wait_for_acknowledgments(Duration::new(20, 0))
        .unwrap();

    let samples = reader
        .take(5, ANY_SAMPLE_STATE, ANY_VIEW_STATE, ANY_INSTANCE_STATE)
        .unwrap();

    assert_eq!(samples.len(), 1);
    assert_eq!(samples[0].data.as_ref().unwrap(), &new_data);
}

#[test]
fn reader_with_exclusive_ownership_should_not_read_samples_from_second_weaker_writer() {
    let domain_id = TEST_DOMAIN_ID_GENERATOR.generate_unique_domain_id();

    let participant = DomainParticipantFactory::get_instance()
        .create_participant(domain_id, QosKind::Default, NO_LISTENER, NO_STATUS)
        .unwrap();

    let topic = participant
        .create_topic::<KeyedData>(
            "MyTopic",
            "KeyedData",
            QosKind::Default,
            NO_LISTENER,
            NO_STATUS,
        )
        .unwrap();

    let publisher = participant
        .create_publisher(QosKind::Default, NO_LISTENER, NO_STATUS)
        .unwrap();
    let writer1_qos = DataWriterQos {
        reliability: ReliabilityQosPolicy {
            kind: ReliabilityQosPolicyKind::Reliable,
            max_blocking_time: DurationKind::Finite(Duration::new(1, 0)),
        },
        ownership: OwnershipQosPolicy {
            kind: OwnershipQosPolicyKind::Exclusive,
        },
        ownership_strength: OwnershipStrengthQosPolicy { value: 10 },
        ..Default::default()
    };
    let writer1 = publisher
        .create_datawriter(
            &topic,
            QosKind::Specific(writer1_qos),
            NO_LISTENER,
            NO_STATUS,
        )
        .unwrap();
    let writer2_qos = DataWriterQos {
        reliability: ReliabilityQosPolicy {
            kind: ReliabilityQosPolicyKind::Reliable,
            max_blocking_time: DurationKind::Finite(Duration::new(1, 0)),
        },
        ownership: OwnershipQosPolicy {
            kind: OwnershipQosPolicyKind::Exclusive,
        },
        ownership_strength: OwnershipStrengthQosPolicy { value: 1 },
        ..Default::default()
    };
    let writer2 = publisher
        .create_datawriter(
            &topic,
            QosKind::Specific(writer2_qos),
            NO_LISTENER,
            NO_STATUS,
        )
        .unwrap();

    let subscriber = participant
        .create_subscriber(QosKind::Default, NO_LISTENER, NO_STATUS)
        .unwrap();
    let reader_qos = DataReaderQos {
        reliability: ReliabilityQosPolicy {
            kind: ReliabilityQosPolicyKind::Reliable,
            max_blocking_time: DurationKind::Finite(Duration::new(1, 0)),
        },
        history: HistoryQosPolicy {
            kind: HistoryQosPolicyKind::KeepAll,
        },
        ownership: OwnershipQosPolicy {
            kind: OwnershipQosPolicyKind::Exclusive,
        },
        ..Default::default()
    };

    let reader = subscriber
        .create_datareader::<KeyedData>(
            &topic,
            QosKind::Specific(reader_qos),
            NO_LISTENER,
            NO_STATUS,
        )
        .unwrap();

    let start_time = std::time::Instant::now();
    while std::time::Instant::now().duration_since(start_time) < std::time::Duration::from_secs(10)
    {
        if reader.get_matched_publications().unwrap().len() >= 2 {
            break;
        }
        std::thread::sleep(std::time::Duration::from_millis(20));
    }
    assert_eq!(
        reader.get_matched_publications().unwrap().len(),
        2,
        "Reader must have 2 matched writers"
    );

    let cond = writer1.get_statuscondition();
    cond.set_enabled_statuses(&[StatusKind::PublicationMatched])
        .unwrap();
    let mut wait_set = WaitSet::new();
    wait_set
        .attach_condition(Condition::StatusCondition(cond))
        .unwrap();
    wait_set.wait(Duration::new(5, 0)).unwrap();

    let cond = writer2.get_statuscondition();
    cond.set_enabled_statuses(&[StatusKind::PublicationMatched])
        .unwrap();
    let mut wait_set = WaitSet::new();
    wait_set
        .attach_condition(Condition::StatusCondition(cond))
        .unwrap();
    wait_set.wait(Duration::new(5, 0)).unwrap();

    let data1 = KeyedData { id: 1, value: 10 };
    writer1.write(data1.clone(), None).unwrap();
    writer1
        .wait_for_acknowledgments(Duration::new(10, 0))
        .unwrap();

    let data2 = KeyedData { id: 1, value: 20 };
    writer2.write(data2, None).unwrap();
    writer2
        .wait_for_acknowledgments(Duration::new(10, 0))
        .unwrap();

    let samples = reader
        .read(10, ANY_SAMPLE_STATE, ANY_VIEW_STATE, ANY_INSTANCE_STATE)
        .unwrap();

    assert_eq!(samples.len(), 1);
    assert_eq!(samples[0].data.as_ref().unwrap(), &data1);
}

#[test]
fn reader_with_exclusive_ownership_should_read_samples_from_second_writer_with_higher_ownership_strength()
 {
    let domain_id = TEST_DOMAIN_ID_GENERATOR.generate_unique_domain_id();

    let participant = DomainParticipantFactory::get_instance()
        .create_participant(domain_id, QosKind::Default, NO_LISTENER, NO_STATUS)
        .unwrap();

    let topic = participant
        .create_topic::<KeyedData>(
            "MyTopic",
            "KeyedData",
            QosKind::Default,
            NO_LISTENER,
            NO_STATUS,
        )
        .unwrap();

    let publisher = participant
        .create_publisher(QosKind::Default, NO_LISTENER, NO_STATUS)
        .unwrap();
    let writer1_qos = DataWriterQos {
        reliability: ReliabilityQosPolicy {
            kind: ReliabilityQosPolicyKind::Reliable,
            max_blocking_time: DurationKind::Finite(Duration::new(1, 0)),
        },
        ownership: OwnershipQosPolicy {
            kind: OwnershipQosPolicyKind::Exclusive,
        },
        ownership_strength: OwnershipStrengthQosPolicy { value: 1 },
        ..Default::default()
    };
    let writer1 = publisher
        .create_datawriter(
            &topic,
            QosKind::Specific(writer1_qos),
            NO_LISTENER,
            NO_STATUS,
        )
        .unwrap();

    let writer2_qos = DataWriterQos {
        reliability: ReliabilityQosPolicy {
            kind: ReliabilityQosPolicyKind::Reliable,
            max_blocking_time: DurationKind::Finite(Duration::new(1, 0)),
        },
        ownership: OwnershipQosPolicy {
            kind: OwnershipQosPolicyKind::Exclusive,
        },
        ownership_strength: OwnershipStrengthQosPolicy { value: 10 },
        ..Default::default()
    };
    let writer2 = publisher
        .create_datawriter(
            &topic,
            QosKind::Specific(writer2_qos),
            NO_LISTENER,
            NO_STATUS,
        )
        .unwrap();

    let subscriber = participant
        .create_subscriber(QosKind::Default, NO_LISTENER, NO_STATUS)
        .unwrap();
    let reader_qos = DataReaderQos {
        reliability: ReliabilityQosPolicy {
            kind: ReliabilityQosPolicyKind::Reliable,
            max_blocking_time: DurationKind::Finite(Duration::new(1, 0)),
        },
        history: HistoryQosPolicy {
            kind: HistoryQosPolicyKind::KeepAll,
        },
        ownership: OwnershipQosPolicy {
            kind: OwnershipQosPolicyKind::Exclusive,
        },
        ..Default::default()
    };

    let reader = subscriber
        .create_datareader::<KeyedData>(
            &topic,
            QosKind::Specific(reader_qos),
            NO_LISTENER,
            NO_STATUS,
        )
        .unwrap();

    let start_time = std::time::Instant::now();
    while std::time::Instant::now().duration_since(start_time) < std::time::Duration::from_secs(10)
    {
        if reader.get_matched_publications().unwrap().len() >= 2 {
            break;
        }
        std::thread::sleep(std::time::Duration::from_millis(20));
    }
    assert_eq!(
        reader.get_matched_publications().unwrap().len(),
        2,
        "Reader must have 2 matched writers"
    );

    let cond = writer1.get_statuscondition();
    cond.set_enabled_statuses(&[StatusKind::PublicationMatched])
        .unwrap();
    let mut wait_set = WaitSet::new();
    wait_set
        .attach_condition(Condition::StatusCondition(cond))
        .unwrap();
    wait_set.wait(Duration::new(5, 0)).unwrap();

    let cond = writer2.get_statuscondition();
    cond.set_enabled_statuses(&[StatusKind::PublicationMatched])
        .unwrap();
    let mut wait_set = WaitSet::new();
    wait_set
        .attach_condition(Condition::StatusCondition(cond))
        .unwrap();
    wait_set.wait(Duration::new(5, 0)).unwrap();

    let data1 = KeyedData { id: 1, value: 10 };
    writer1.write(data1.clone(), None).unwrap();
    writer1
        .wait_for_acknowledgments(Duration::new(10, 0))
        .unwrap();

    let data2 = KeyedData { id: 1, value: 20 };
    writer2.write(data2.clone(), None).unwrap();
    writer2
        .wait_for_acknowledgments(Duration::new(10, 0))
        .unwrap();

    let samples = reader
        .read(10, ANY_SAMPLE_STATE, ANY_VIEW_STATE, ANY_INSTANCE_STATE)
        .unwrap();

    assert_eq!(samples.len(), 2);
    assert_eq!(samples[0].data.as_ref().unwrap(), &data1);
    assert_eq!(samples[1].data.as_ref().unwrap(), &data2);
}

#[test]
fn reader_with_exclusive_ownership_should_read_samples_from_second_writer_after_deadline_missed() {
    let domain_id = TEST_DOMAIN_ID_GENERATOR.generate_unique_domain_id();

    let participant = DomainParticipantFactory::get_instance()
        .create_participant(domain_id, QosKind::Default, NO_LISTENER, NO_STATUS)
        .unwrap();

    let topic = participant
        .create_topic::<KeyedData>(
            "MyTopic",
            "KeyedData",
            QosKind::Default,
            NO_LISTENER,
            NO_STATUS,
        )
        .unwrap();

    let publisher = participant
        .create_publisher(QosKind::Default, NO_LISTENER, NO_STATUS)
        .unwrap();
    let writer1_qos = DataWriterQos {
        reliability: ReliabilityQosPolicy {
            kind: ReliabilityQosPolicyKind::Reliable,
            max_blocking_time: DurationKind::Finite(Duration::new(1, 0)),
        },
        ownership: OwnershipQosPolicy {
            kind: OwnershipQosPolicyKind::Exclusive,
        },
        ownership_strength: OwnershipStrengthQosPolicy { value: 10 },
        deadline: DeadlineQosPolicy {
            period: DurationKind::Finite(Duration::new(1, 0)),
        },
        ..Default::default()
    };
    let writer1 = publisher
        .create_datawriter(
            &topic,
            QosKind::Specific(writer1_qos),
            NO_LISTENER,
            NO_STATUS,
        )
        .unwrap();

    let writer2_qos = DataWriterQos {
        reliability: ReliabilityQosPolicy {
            kind: ReliabilityQosPolicyKind::Reliable,
            max_blocking_time: DurationKind::Finite(Duration::new(1, 0)),
        },
        ownership: OwnershipQosPolicy {
            kind: OwnershipQosPolicyKind::Exclusive,
        },
        ownership_strength: OwnershipStrengthQosPolicy { value: 1 },
        deadline: DeadlineQosPolicy {
            period: DurationKind::Finite(Duration::new(1, 0)),
        },
        ..Default::default()
    };
    let writer2 = publisher
        .create_datawriter(
            &topic,
            QosKind::Specific(writer2_qos),
            NO_LISTENER,
            NO_STATUS,
        )
        .unwrap();

    let subscriber = participant
        .create_subscriber(QosKind::Default, NO_LISTENER, NO_STATUS)
        .unwrap();
    let reader_qos = DataReaderQos {
        reliability: ReliabilityQosPolicy {
            kind: ReliabilityQosPolicyKind::Reliable,
            max_blocking_time: DurationKind::Finite(Duration::new(1, 0)),
        },
        history: HistoryQosPolicy {
            kind: HistoryQosPolicyKind::KeepAll,
        },
        ownership: OwnershipQosPolicy {
            kind: OwnershipQosPolicyKind::Exclusive,
        },
        deadline: DeadlineQosPolicy {
            period: DurationKind::Finite(Duration::new(1, 0)),
        },
        ..Default::default()
    };

    let reader = subscriber
        .create_datareader::<KeyedData>(
            &topic,
            QosKind::Specific(reader_qos),
            NO_LISTENER,
            NO_STATUS,
        )
        .unwrap();

    let start_time = std::time::Instant::now();
    while std::time::Instant::now().duration_since(start_time) < std::time::Duration::from_secs(10)
    {
        if reader.get_matched_publications().unwrap().len() >= 2 {
            break;
        }
        std::thread::sleep(std::time::Duration::from_millis(20));
    }
    assert_eq!(
        reader.get_matched_publications().unwrap().len(),
        2,
        "Reader must have 2 matched writers"
    );

    let cond = writer1.get_statuscondition();
    cond.set_enabled_statuses(&[StatusKind::PublicationMatched])
        .unwrap();
    let mut wait_set = WaitSet::new();
    wait_set
        .attach_condition(Condition::StatusCondition(cond))
        .unwrap();
    wait_set.wait(Duration::new(5, 0)).unwrap();

    let cond = writer2.get_statuscondition();
    cond.set_enabled_statuses(&[StatusKind::PublicationMatched])
        .unwrap();
    let mut wait_set = WaitSet::new();
    wait_set
        .attach_condition(Condition::StatusCondition(cond))
        .unwrap();
    wait_set.wait(Duration::new(5, 0)).unwrap();

    let data1 = KeyedData { id: 1, value: 10 };
    writer1.write(data1.clone(), None).unwrap();
    writer1
        .wait_for_acknowledgments(Duration::new(10, 0))
        .unwrap();

    let cond = reader.get_statuscondition();
    cond.set_enabled_statuses(&[StatusKind::RequestedDeadlineMissed])
        .unwrap();
    let mut wait_set = WaitSet::new();
    wait_set
        .attach_condition(Condition::StatusCondition(cond))
        .unwrap();
    wait_set.wait(Duration::new(5, 0)).unwrap();

    let data2 = KeyedData { id: 1, value: 20 };
    writer2.write(data2.clone(), None).unwrap();
    writer2
        .wait_for_acknowledgments(Duration::new(10, 0))
        .unwrap();

    let samples = reader
        .read(10, ANY_SAMPLE_STATE, ANY_VIEW_STATE, ANY_INSTANCE_STATE)
        .unwrap();

    assert_eq!(samples.len(), 2);
    assert_eq!(samples[0].data.as_ref().unwrap(), &data1);
    assert_eq!(samples[1].data.as_ref().unwrap(), &data2);
}

#[test]
fn reader_with_exclusive_ownership_should_read_samples_from_second_weaker_writer_after_unregister()
{
    let domain_id = TEST_DOMAIN_ID_GENERATOR.generate_unique_domain_id();

    let participant = DomainParticipantFactory::get_instance()
        .create_participant(domain_id, QosKind::Default, NO_LISTENER, NO_STATUS)
        .unwrap();

    let topic = participant
        .create_topic::<KeyedData>(
            "MyTopic",
            "KeyedData",
            QosKind::Default,
            NO_LISTENER,
            NO_STATUS,
        )
        .unwrap();

    let publisher = participant
        .create_publisher(QosKind::Default, NO_LISTENER, NO_STATUS)
        .unwrap();
    let writer1_qos = DataWriterQos {
        reliability: ReliabilityQosPolicy {
            kind: ReliabilityQosPolicyKind::Reliable,
            max_blocking_time: DurationKind::Finite(Duration::new(1, 0)),
        },
        ownership: OwnershipQosPolicy {
            kind: OwnershipQosPolicyKind::Exclusive,
        },
        ownership_strength: OwnershipStrengthQosPolicy { value: 10 },
        ..Default::default()
    };
    let writer1 = publisher
        .create_datawriter(
            &topic,
            QosKind::Specific(writer1_qos),
            NO_LISTENER,
            NO_STATUS,
        )
        .unwrap();
    let writer2_qos = DataWriterQos {
        reliability: ReliabilityQosPolicy {
            kind: ReliabilityQosPolicyKind::Reliable,
            max_blocking_time: DurationKind::Finite(Duration::new(1, 0)),
        },
        ownership: OwnershipQosPolicy {
            kind: OwnershipQosPolicyKind::Exclusive,
        },
        ownership_strength: OwnershipStrengthQosPolicy { value: 1 },
        ..Default::default()
    };
    let writer2 = publisher
        .create_datawriter(
            &topic,
            QosKind::Specific(writer2_qos),
            NO_LISTENER,
            NO_STATUS,
        )
        .unwrap();

    let subscriber = participant
        .create_subscriber(QosKind::Default, NO_LISTENER, NO_STATUS)
        .unwrap();
    let reader_qos = DataReaderQos {
        reliability: ReliabilityQosPolicy {
            kind: ReliabilityQosPolicyKind::Reliable,
            max_blocking_time: DurationKind::Finite(Duration::new(1, 0)),
        },
        history: HistoryQosPolicy {
            kind: HistoryQosPolicyKind::KeepAll,
        },
        ownership: OwnershipQosPolicy {
            kind: OwnershipQosPolicyKind::Exclusive,
        },
        ..Default::default()
    };

    let reader = subscriber
        .create_datareader::<KeyedData>(
            &topic,
            QosKind::Specific(reader_qos),
            NO_LISTENER,
            NO_STATUS,
        )
        .unwrap();

    let start_time = std::time::Instant::now();
    while std::time::Instant::now().duration_since(start_time) < std::time::Duration::from_secs(10)
    {
        if reader.get_matched_publications().unwrap().len() >= 2 {
            break;
        }
        std::thread::sleep(std::time::Duration::from_millis(20));
    }
    assert_eq!(
        reader.get_matched_publications().unwrap().len(),
        2,
        "Reader must have 2 matched writers"
    );

    let cond = writer1.get_statuscondition();
    cond.set_enabled_statuses(&[StatusKind::PublicationMatched])
        .unwrap();
    let mut wait_set = WaitSet::new();
    wait_set
        .attach_condition(Condition::StatusCondition(cond))
        .unwrap();
    wait_set.wait(Duration::new(5, 0)).unwrap();

    let cond = writer2.get_statuscondition();
    cond.set_enabled_statuses(&[StatusKind::PublicationMatched])
        .unwrap();
    let mut wait_set = WaitSet::new();
    wait_set
        .attach_condition(Condition::StatusCondition(cond))
        .unwrap();
    wait_set.wait(Duration::new(5, 0)).unwrap();

    let data1 = KeyedData { id: 1, value: 10 };
    writer1.write(data1.clone(), None).unwrap();
    writer1
        .wait_for_acknowledgments(Duration::new(10, 0))
        .unwrap();

    writer1.unregister_instance(data1.clone(), None).unwrap();
    writer1
        .wait_for_acknowledgments(Duration::new(10, 0))
        .unwrap();

    let data2 = KeyedData { id: 1, value: 20 };
    writer2.write(data2.clone(), None).unwrap();
    writer2
        .wait_for_acknowledgments(Duration::new(10, 0))
        .unwrap();

    let samples = reader
        .read(10, ANY_SAMPLE_STATE, ANY_VIEW_STATE, ANY_INSTANCE_STATE)
        .unwrap();

    assert_eq!(samples.len(), 3);
    assert_eq!(samples[0].data.as_ref().unwrap(), &data1);
    assert_eq!(samples[2].data.as_ref().unwrap(), &data2);
}

#[test]
fn reader_with_exclusive_ownership_should_read_samples_from_second_weaker_writer_after_dispose() {
    let domain_id = TEST_DOMAIN_ID_GENERATOR.generate_unique_domain_id();

    let participant = DomainParticipantFactory::get_instance()
        .create_participant(domain_id, QosKind::Default, NO_LISTENER, NO_STATUS)
        .unwrap();

    let topic = participant
        .create_topic::<KeyedData>(
            "MyTopic",
            "KeyedData",
            QosKind::Default,
            NO_LISTENER,
            NO_STATUS,
        )
        .unwrap();

    let publisher = participant
        .create_publisher(QosKind::Default, NO_LISTENER, NO_STATUS)
        .unwrap();
    let writer1_qos = DataWriterQos {
        reliability: ReliabilityQosPolicy {
            kind: ReliabilityQosPolicyKind::Reliable,
            max_blocking_time: DurationKind::Finite(Duration::new(1, 0)),
        },
        ownership: OwnershipQosPolicy {
            kind: OwnershipQosPolicyKind::Exclusive,
        },
        ownership_strength: OwnershipStrengthQosPolicy { value: 10 },
        ..Default::default()
    };
    let writer1 = publisher
        .create_datawriter(
            &topic,
            QosKind::Specific(writer1_qos),
            NO_LISTENER,
            NO_STATUS,
        )
        .unwrap();
    let writer2_qos = DataWriterQos {
        reliability: ReliabilityQosPolicy {
            kind: ReliabilityQosPolicyKind::Reliable,
            max_blocking_time: DurationKind::Finite(Duration::new(1, 0)),
        },
        ownership: OwnershipQosPolicy {
            kind: OwnershipQosPolicyKind::Exclusive,
        },
        ownership_strength: OwnershipStrengthQosPolicy { value: 1 },
        ..Default::default()
    };
    let writer2 = publisher
        .create_datawriter(
            &topic,
            QosKind::Specific(writer2_qos),
            NO_LISTENER,
            NO_STATUS,
        )
        .unwrap();

    let subscriber = participant
        .create_subscriber(QosKind::Default, NO_LISTENER, NO_STATUS)
        .unwrap();
    let reader_qos = DataReaderQos {
        reliability: ReliabilityQosPolicy {
            kind: ReliabilityQosPolicyKind::Reliable,
            max_blocking_time: DurationKind::Finite(Duration::new(1, 0)),
        },
        history: HistoryQosPolicy {
            kind: HistoryQosPolicyKind::KeepAll,
        },
        ownership: OwnershipQosPolicy {
            kind: OwnershipQosPolicyKind::Exclusive,
        },
        ..Default::default()
    };

    let reader = subscriber
        .create_datareader::<KeyedData>(
            &topic,
            QosKind::Specific(reader_qos),
            NO_LISTENER,
            NO_STATUS,
        )
        .unwrap();

    let start_time = std::time::Instant::now();
    while std::time::Instant::now().duration_since(start_time) < std::time::Duration::from_secs(10)
    {
        if reader.get_matched_publications().unwrap().len() >= 2 {
            break;
        }
        std::thread::sleep(std::time::Duration::from_millis(20));
    }
    assert_eq!(
        reader.get_matched_publications().unwrap().len(),
        2,
        "Reader must have 2 matched writers"
    );

    let cond = writer1.get_statuscondition();
    cond.set_enabled_statuses(&[StatusKind::PublicationMatched])
        .unwrap();
    let mut wait_set = WaitSet::new();
    wait_set
        .attach_condition(Condition::StatusCondition(cond))
        .unwrap();
    wait_set.wait(Duration::new(5, 0)).unwrap();

    let cond = writer2.get_statuscondition();
    cond.set_enabled_statuses(&[StatusKind::PublicationMatched])
        .unwrap();
    let mut wait_set = WaitSet::new();
    wait_set
        .attach_condition(Condition::StatusCondition(cond))
        .unwrap();
    wait_set.wait(Duration::new(5, 0)).unwrap();

    let data1 = KeyedData { id: 1, value: 10 };
    writer1.write(data1.clone(), None).unwrap();
    writer1
        .wait_for_acknowledgments(Duration::new(10, 0))
        .unwrap();

    writer1.dispose(data1.clone(), None).unwrap();
    writer1
        .wait_for_acknowledgments(Duration::new(10, 0))
        .unwrap();

    let data2 = KeyedData { id: 1, value: 20 };
    writer2.write(data2.clone(), None).unwrap();
    writer2
        .wait_for_acknowledgments(Duration::new(10, 0))
        .unwrap();

    let samples = reader
        .read(10, ANY_SAMPLE_STATE, ANY_VIEW_STATE, ANY_INSTANCE_STATE)
        .unwrap();

    assert_eq!(samples.len(), 3);
    assert_eq!(samples[0].data.as_ref().unwrap(), &data1);
    assert_eq!(samples[2].data.as_ref().unwrap(), &data2);
}

#[test]
fn samples_are_transfered_between_two_participants() {
    let domain_id = TEST_DOMAIN_ID_GENERATOR.generate_unique_domain_id();

    let participant1 = DomainParticipantFactory::get_instance()
        .create_participant(domain_id, QosKind::Default, NO_LISTENER, NO_STATUS)
        .unwrap();

    let topic = participant1
        .create_topic::<KeyedData>(
            "MyTopic",
            "KeyedData",
            QosKind::Default,
            NO_LISTENER,
            NO_STATUS,
        )
        .unwrap();

    let publisher = participant1
        .create_publisher(QosKind::Default, NO_LISTENER, NO_STATUS)
        .unwrap();
    let writer_qos = DataWriterQos {
        reliability: ReliabilityQosPolicy {
            kind: ReliabilityQosPolicyKind::Reliable,
            max_blocking_time: DurationKind::Finite(Duration::new(1, 0)),
        },
        ..Default::default()
    };
    let writer = publisher
        .create_datawriter(
            &topic,
            QosKind::Specific(writer_qos),
            NO_LISTENER,
            NO_STATUS,
        )
        .unwrap();

    let participant2 = DomainParticipantFactory::get_instance()
        .create_participant(domain_id, QosKind::Default, NO_LISTENER, NO_STATUS)
        .unwrap();
    let subscriber = participant2
        .create_subscriber(QosKind::Default, NO_LISTENER, NO_STATUS)
        .unwrap();
    let reader_qos = DataReaderQos {
        reliability: ReliabilityQosPolicy {
            kind: ReliabilityQosPolicyKind::Reliable,
            max_blocking_time: DurationKind::Finite(Duration::new(1, 0)),
        },
        ..Default::default()
    };
    let topic = participant2
        .create_topic::<KeyedData>(
            "MyTopic",
            "KeyedData",
            QosKind::Default,
            NO_LISTENER,
            NO_STATUS,
        )
        .unwrap();
    let reader = subscriber
        .create_datareader::<KeyedData>(
            &topic,
            QosKind::Specific(reader_qos),
            NO_LISTENER,
            NO_STATUS,
        )
        .unwrap();

    let cond = writer.get_statuscondition();
    cond.set_enabled_statuses(&[StatusKind::PublicationMatched])
        .unwrap();

    let mut wait_set = WaitSet::new();
    wait_set
        .attach_condition(Condition::StatusCondition(cond))
        .unwrap();
    wait_set.wait(Duration::new(100, 0)).unwrap();

    let cond = reader.get_statuscondition();
    cond.set_enabled_statuses(&[StatusKind::SubscriptionMatched])
        .unwrap();
    let mut wait_set = WaitSet::new();
    wait_set
        .attach_condition(Condition::StatusCondition(cond))
        .unwrap();
    wait_set.wait(Duration::new(10, 0)).unwrap();

    let data = KeyedData { id: 1, value: 1 };

    writer.write(data.clone(), None).unwrap();

    writer
        .wait_for_acknowledgments(Duration::new(10, 0))
        .unwrap();

    let samples = reader
        .take(1, ANY_SAMPLE_STATE, ANY_VIEW_STATE, ANY_INSTANCE_STATE)
        .unwrap();

    assert_eq!(samples.len(), 1);
    assert_eq!(samples[0].data.as_ref().unwrap(), &data);
}

#[test]
fn shared_ownership_writer1_should_write_and_writer2_should_dispose_same_data() {
    let domain_id = TEST_DOMAIN_ID_GENERATOR.generate_unique_domain_id();

    let participant = DomainParticipantFactory::get_instance()
        .create_participant(domain_id, QosKind::Default, NO_LISTENER, NO_STATUS)
        .unwrap();

    let topic = participant
        .create_topic::<KeyedData>(
            "MyTopic",
            "KeyedData",
            QosKind::Default,
            NO_LISTENER,
            NO_STATUS,
        )
        .unwrap();

    let publisher = participant
        .create_publisher(QosKind::Default, NO_LISTENER, NO_STATUS)
        .unwrap();
    let writer1_qos = DataWriterQos {
        reliability: ReliabilityQosPolicy {
            kind: ReliabilityQosPolicyKind::Reliable,
            max_blocking_time: DurationKind::Finite(Duration::new(1, 0)),
        },
        ownership: OwnershipQosPolicy {
            kind: OwnershipQosPolicyKind::Shared,
        },
        ..Default::default()
    };
    let writer1 = publisher
        .create_datawriter(
            &topic,
            QosKind::Specific(writer1_qos),
            NO_LISTENER,
            NO_STATUS,
        )
        .unwrap();
    let writer2_qos = DataWriterQos {
        reliability: ReliabilityQosPolicy {
            kind: ReliabilityQosPolicyKind::Reliable,
            max_blocking_time: DurationKind::Finite(Duration::new(1, 0)),
        },
        ownership: OwnershipQosPolicy {
            kind: OwnershipQosPolicyKind::Shared,
        },
        ..Default::default()
    };
    let writer2 = publisher
        .create_datawriter(
            &topic,
            QosKind::Specific(writer2_qos),
            NO_LISTENER,
            NO_STATUS,
        )
        .unwrap();

    let subscriber = participant
        .create_subscriber(QosKind::Default, NO_LISTENER, NO_STATUS)
        .unwrap();
    let reader_qos = DataReaderQos {
        reliability: ReliabilityQosPolicy {
            kind: ReliabilityQosPolicyKind::Reliable,
            max_blocking_time: DurationKind::Finite(Duration::new(1, 0)),
        },
        ownership: OwnershipQosPolicy {
            kind: OwnershipQosPolicyKind::Shared,
        },
        ..Default::default()
    };

    let reader = subscriber
        .create_datareader::<KeyedData>(
            &topic,
            QosKind::Specific(reader_qos),
            NO_LISTENER,
            NO_STATUS,
        )
        .unwrap();

    let start_time = std::time::Instant::now();
    while std::time::Instant::now().duration_since(start_time) < std::time::Duration::from_secs(10)
    {
        if reader.get_matched_publications().unwrap().len() >= 2 {
            break;
        }
        std::thread::sleep(std::time::Duration::from_millis(20));
    }
    assert_eq!(
        reader.get_matched_publications().unwrap().len(),
        2,
        "Reader must have 2 matched writers"
    );

    let cond = writer1.get_statuscondition();
    cond.set_enabled_statuses(&[StatusKind::PublicationMatched])
        .unwrap();
    let mut wait_set = WaitSet::new();
    wait_set
        .attach_condition(Condition::StatusCondition(cond))
        .unwrap();
    wait_set.wait(Duration::new(5, 0)).unwrap();

    let cond = writer2.get_statuscondition();
    cond.set_enabled_statuses(&[StatusKind::PublicationMatched])
        .unwrap();
    let mut wait_set = WaitSet::new();
    wait_set
        .attach_condition(Condition::StatusCondition(cond))
        .unwrap();
    wait_set.wait(Duration::new(5, 0)).unwrap();

    let data = KeyedData { id: 1, value: 10 };
    writer1.write(data.clone(), None).unwrap();
    writer1
        .wait_for_acknowledgments(Duration::new(10, 0))
        .unwrap();

    let samples = reader
        .take(3, ANY_SAMPLE_STATE, ANY_VIEW_STATE, ANY_INSTANCE_STATE)
        .unwrap();
    assert_eq!(samples.len(), 1);
    assert_eq!(samples[0].data.as_ref().unwrap(), &data);
    assert_eq!(
        samples[0].sample_info.instance_state,
        InstanceStateKind::Alive
    );

    let _handle = writer2.register_instance(data.clone()).unwrap();
    writer2.dispose(data, None).unwrap();
    writer2
        .wait_for_acknowledgments(Duration::new(10, 0))
        .unwrap();

    let samples = reader
        .take(3, ANY_SAMPLE_STATE, ANY_VIEW_STATE, ANY_INSTANCE_STATE)
        .unwrap();
    assert_eq!(samples.len(), 1);
    assert!(samples[0].data.is_none());
    assert_eq!(
        samples[0].sample_info.instance_state,
        InstanceStateKind::NotAliveDisposed
    );
}

#[test]
fn data_reader_does_not_read_lifespan_expired_samples() {
    let domain_id = TEST_DOMAIN_ID_GENERATOR.generate_unique_domain_id();

    let participant1 = DomainParticipantFactory::get_instance()
        .create_participant(domain_id, QosKind::Default, NO_LISTENER, NO_STATUS)
        .unwrap();

    let topic1 = participant1
        .create_topic::<KeyedData>(
            "LifespanTopic",
            "KeyedData",
            QosKind::Default,
            NO_LISTENER,
            NO_STATUS,
        )
        .unwrap();

    let publisher = participant1
        .create_publisher(QosKind::Default, NO_LISTENER, NO_STATUS)
        .unwrap();
    let writer_qos = DataWriterQos {
        history: HistoryQosPolicy {
            kind: HistoryQosPolicyKind::KeepAll,
        },
        reliability: ReliabilityQosPolicy {
            kind: ReliabilityQosPolicyKind::Reliable,
            max_blocking_time: DurationKind::Finite(Duration::new(1, 0)),
        },
        lifespan: LifespanQosPolicy {
            duration: DurationKind::Finite(Duration::new(1, 0)),
        },
        ..Default::default()
    };
    let writer = publisher
        .create_datawriter(
            &topic1,
            QosKind::Specific(writer_qos),
            NO_LISTENER,
            NO_STATUS,
        )
        .unwrap();

    let participant2 = DomainParticipantFactory::get_instance()
        .create_participant(domain_id, QosKind::Default, NO_LISTENER, NO_STATUS)
        .unwrap();

    let topic2 = participant2
        .create_topic::<KeyedData>(
            "LifespanTopic",
            "KeyedData",
            QosKind::Default,
            NO_LISTENER,
            NO_STATUS,
        )
        .unwrap();

    let subscriber = participant2
        .create_subscriber(QosKind::Default, NO_LISTENER, NO_STATUS)
        .unwrap();
    let reader_qos = DataReaderQos {
        history: HistoryQosPolicy {
            kind: HistoryQosPolicyKind::KeepAll,
        },
        reliability: ReliabilityQosPolicy {
            kind: ReliabilityQosPolicyKind::Reliable,
            max_blocking_time: DurationKind::Finite(Duration::new(1, 0)),
        },
        ..Default::default()
    };

    let reader = subscriber
        .create_datareader::<KeyedData>(
            &topic2,
            QosKind::Specific(reader_qos),
            NO_LISTENER,
            NO_STATUS,
        )
        .unwrap();

    let cond = writer.get_statuscondition();
    cond.set_enabled_statuses(&[StatusKind::PublicationMatched])
        .unwrap();
    let mut wait_set = WaitSet::new();
    wait_set
        .attach_condition(Condition::StatusCondition(cond))
        .unwrap();
    wait_set.wait(Duration::new(5, 0)).unwrap();

    let cond = reader.get_statuscondition();
    cond.set_enabled_statuses(&[StatusKind::SubscriptionMatched])
        .unwrap();
    let mut wait_set = WaitSet::new();
    wait_set
        .attach_condition(Condition::StatusCondition(cond))
        .unwrap();
    wait_set.wait(Duration::new(5, 0)).unwrap();

    let data1 = KeyedData { id: 1, value: 1 };
    let data2 = KeyedData { id: 1, value: 2 };

    writer
        .write_w_timestamp(data1, None, Time::new(0, 0))
        .unwrap();
    writer
        .write_w_timestamp(data2.clone(), None, Time::new(i32::MAX, 0))
        .unwrap();

    writer
        .wait_for_acknowledgments(Duration::new(10, 0))
        .unwrap();

    let samples = reader
        .take(10, ANY_SAMPLE_STATE, ANY_VIEW_STATE, ANY_INSTANCE_STATE)
        .unwrap();

    assert_eq!(samples.len(), 1);
    assert_eq!(samples[0].data.as_ref().unwrap(), &data2);
}
