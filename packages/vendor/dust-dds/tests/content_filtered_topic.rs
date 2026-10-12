use dust_dds::{
    domain::domain_participant_factory::DomainParticipantFactory,
    infrastructure::{
        listener::NO_LISTENER,
        qos::{DataReaderQos, DataWriterQos, QosKind},
        qos_policy::{ReliabilityQosPolicy, ReliabilityQosPolicyKind},
        sample_info::{ANY_INSTANCE_STATE, ANY_SAMPLE_STATE, ANY_VIEW_STATE},
        status::{NO_STATUS, StatusKind},
        time::{Duration, DurationKind},
    },
    wait_set::{Condition, WaitSet},
};
use dust_dds_derive::DdsType;

mod utils;
use crate::utils::domain_id_generator::TEST_DOMAIN_ID_GENERATOR;

#[derive(Clone, Debug, PartialEq, DdsType)]
struct KeyedData {
    #[dust_dds(key)]
    id: u8,
    value: String,
}

#[derive(Clone, Debug, PartialEq, DdsType)]
struct ShapeType {
    #[dust_dds(key)]
    color: String,
    x: i32,
    y: i32,
    shapesize: i32,
}

#[test]
fn samples_should_be_content_filtered() {
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

    let content_filtered_topic = participant
        .create_contentfilteredtopic(
            "MyTopicFiltered",
            &topic,
            String::from("value = %0"),
            vec![String::from("RED")],
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
            &content_filtered_topic,
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

    let data1 = KeyedData {
        id: 1,
        value: String::from("BLUE"),
    };
    let data2 = KeyedData {
        id: 2,
        value: String::from("RED"),
    };

    writer.write(data1.clone(), None).unwrap();
    writer.write(data2.clone(), None).unwrap();
    writer
        .wait_for_acknowledgments(Duration::new(10, 0))
        .unwrap();

    let samples = reader
        .take(5, ANY_SAMPLE_STATE, ANY_VIEW_STATE, ANY_INSTANCE_STATE)
        .unwrap();

    assert_eq!(samples.len(), 1);
    assert_eq!(samples[0].data.as_ref().unwrap(), &data2);
}

#[test]
fn samples_should_be_content_filtered_by_shapesize() {
    let domain_id = TEST_DOMAIN_ID_GENERATOR.generate_unique_domain_id();

    let participant = DomainParticipantFactory::get_instance()
        .create_participant(domain_id, QosKind::Default, NO_LISTENER, NO_STATUS)
        .unwrap();

    let topic = participant
        .create_topic::<ShapeType>(
            "ShapeTopic",
            "ShapeType",
            QosKind::Default,
            NO_LISTENER,
            NO_STATUS,
        )
        .unwrap();

    let content_filtered_topic = participant
        .create_contentfilteredtopic(
            "ShapeTopicFiltered",
            &topic,
            String::from("shapesize <= 20"),
            vec![],
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
        .create_datareader::<ShapeType>(
            &content_filtered_topic,
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

    let data1 = ShapeType {
        color: String::from("BLUE"),
        x: 100,
        y: 100,
        shapesize: 30,
    };
    let data2 = ShapeType {
        color: String::from("RED"),
        x: 100,
        y: 100,
        shapesize: 20,
    };

    writer.write(data1.clone(), None).unwrap();
    writer.write(data2.clone(), None).unwrap();
    writer
        .wait_for_acknowledgments(Duration::new(10, 0))
        .unwrap();

    let samples = reader
        .take(5, ANY_SAMPLE_STATE, ANY_VIEW_STATE, ANY_INSTANCE_STATE)
        .unwrap();

    assert_eq!(samples.len(), 1);
    assert_eq!(samples[0].data.as_ref().unwrap(), &data2);
}

#[test]
fn samples_should_be_content_filtered_by_parameterized_shapesize() {
    let domain_id = TEST_DOMAIN_ID_GENERATOR.generate_unique_domain_id();

    let participant = DomainParticipantFactory::get_instance()
        .create_participant(domain_id, QosKind::Default, NO_LISTENER, NO_STATUS)
        .unwrap();

    let topic = participant
        .create_topic::<ShapeType>(
            "ShapeTopic",
            "ShapeType",
            QosKind::Default,
            NO_LISTENER,
            NO_STATUS,
        )
        .unwrap();

    let content_filtered_topic = participant
        .create_contentfilteredtopic(
            "ShapeTopicFiltered",
            &topic,
            String::from("shapesize <= %0"),
            vec![String::from("20")],
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
        .create_datareader::<ShapeType>(
            &content_filtered_topic,
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

    let data1 = ShapeType {
        color: String::from("BLUE"),
        x: 100,
        y: 100,
        shapesize: 30,
    };
    let data2 = ShapeType {
        color: String::from("RED"),
        x: 100,
        y: 100,
        shapesize: 20,
    };

    writer.write(data1.clone(), None).unwrap();
    writer.write(data2.clone(), None).unwrap();
    writer
        .wait_for_acknowledgments(Duration::new(10, 0))
        .unwrap();

    let samples = reader
        .take(5, ANY_SAMPLE_STATE, ANY_VIEW_STATE, ANY_INSTANCE_STATE)
        .unwrap();

    assert_eq!(samples.len(), 1);
    assert_eq!(samples[0].data.as_ref().unwrap(), &data2);
}

#[test]
fn samples_should_be_content_filtered_by_literal_string() {
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

    let content_filtered_topic = participant
        .create_contentfilteredtopic(
            "MyTopicFiltered",
            &topic,
            String::from("value = 'RED'"),
            vec![],
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
            &content_filtered_topic,
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

    let data1 = KeyedData {
        id: 1,
        value: String::from("BLUE"),
    };
    let data2 = KeyedData {
        id: 2,
        value: String::from("RED"),
    };

    writer.write(data1.clone(), None).unwrap();
    writer.write(data2.clone(), None).unwrap();
    writer
        .wait_for_acknowledgments(Duration::new(10, 0))
        .unwrap();

    let samples = reader
        .take(5, ANY_SAMPLE_STATE, ANY_VIEW_STATE, ANY_INSTANCE_STATE)
        .unwrap();

    assert_eq!(samples.len(), 1);
    assert_eq!(samples[0].data.as_ref().unwrap(), &data2);
}
