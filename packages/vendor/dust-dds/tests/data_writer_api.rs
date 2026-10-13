use dust_dds::{
    domain::domain_participant_factory::DomainParticipantFactory,
    infrastructure::{
        error::DdsError,
        listener::NO_LISTENER,
        qos::{DataWriterQos, QosKind},
        qos_policy::{HistoryQosPolicy, HistoryQosPolicyKind, Length, ResourceLimitsQosPolicy},
        status::NO_STATUS,
        type_support::DdsType,
    },
};

mod utils;
use crate::utils::domain_id_generator::TEST_DOMAIN_ID_GENERATOR;

#[derive(Clone, Debug, PartialEq, DdsType)]
struct KeyedData {
    #[dust_dds(key)]
    id: u8,
    value: u32,
}

#[test]
fn data_writer_write_more_than_max_instances_should_fail() {
    let domain_id = TEST_DOMAIN_ID_GENERATOR.generate_unique_domain_id();
    let domain_participant_factory = DomainParticipantFactory::get_instance();
    let participant = domain_participant_factory
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

    let data_writer_qos = DataWriterQos {
        resource_limits: ResourceLimitsQosPolicy {
            max_samples: Length::Unlimited,
            max_instances: Length::Limited(1),
            max_samples_per_instance: Length::Unlimited,
        },
        ..Default::default()
    };
    let data_writer = publisher
        .create_datawriter(
            &topic,
            QosKind::Specific(data_writer_qos),
            NO_LISTENER,
            NO_STATUS,
        )
        .unwrap();
    let data_instance1 = KeyedData { id: 1, value: 0 };
    let data_instance2 = KeyedData { id: 2, value: 0 };
    data_writer.write(data_instance1, None).unwrap();

    let result = data_writer.write(data_instance2, None);
    assert_eq!(result, Err(DdsError::OutOfResources));
}

#[test]
fn data_writer_write_more_than_max_samples_per_instances_should_fail() {
    let domain_id = TEST_DOMAIN_ID_GENERATOR.generate_unique_domain_id();
    let domain_participant_factory = DomainParticipantFactory::get_instance();
    let participant = domain_participant_factory
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

    let data_writer_qos = DataWriterQos {
        resource_limits: ResourceLimitsQosPolicy {
            max_samples: Length::Unlimited,
            max_instances: Length::Unlimited,
            max_samples_per_instance: Length::Limited(1),
        },
        history: HistoryQosPolicy {
            kind: HistoryQosPolicyKind::KeepAll,
        },
        ..Default::default()
    };
    let data_writer = publisher
        .create_datawriter(
            &topic,
            QosKind::Specific(data_writer_qos),
            NO_LISTENER,
            NO_STATUS,
        )
        .unwrap();
    let data1_instance1 = KeyedData { id: 1, value: 0 };
    let data2_instance1 = KeyedData { id: 1, value: 1 };
    data_writer.write(data1_instance1, None).unwrap();

    let result = data_writer.write(data2_instance1, None);
    assert_eq!(result, Err(DdsError::OutOfResources));
}

#[test]
fn data_writer_write_more_than_max_samples_should_fail() {
    let domain_id = TEST_DOMAIN_ID_GENERATOR.generate_unique_domain_id();
    let domain_participant_factory = DomainParticipantFactory::get_instance();
    let participant = domain_participant_factory
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

    let data_writer_qos = DataWriterQos {
        resource_limits: ResourceLimitsQosPolicy {
            max_samples: Length::Limited(2),
            max_instances: Length::Unlimited,
            max_samples_per_instance: Length::Limited(1),
        },
        ..Default::default()
    };
    let data_writer = publisher
        .create_datawriter(
            &topic,
            QosKind::Specific(data_writer_qos),
            NO_LISTENER,
            NO_STATUS,
        )
        .unwrap();
    let data_instance1 = KeyedData { id: 1, value: 0 };
    let data_instance2 = KeyedData { id: 2, value: 0 };
    let data_instance3 = KeyedData { id: 3, value: 0 };
    data_writer.write(data_instance1, None).unwrap();
    data_writer.write(data_instance2, None).unwrap();

    let result = data_writer.write(data_instance3, None);
    assert_eq!(result, Err(DdsError::OutOfResources));
}
