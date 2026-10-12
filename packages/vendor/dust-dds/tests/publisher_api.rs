use dust_dds::{
    domain::domain_participant_factory::DomainParticipantFactory,
    infrastructure::{
        listener::NO_LISTENER,
        qos::{DataWriterQos, QosKind, TopicQos},
        qos_policy::{HistoryQosPolicy, HistoryQosPolicyKind, UserDataQosPolicy},
        status::NO_STATUS,
        type_support::DdsType,
    },
    xtypes::type_support::TypeSupport,
};

mod utils;
use crate::utils::domain_id_generator::TEST_DOMAIN_ID_GENERATOR;

#[derive(DdsType)]
struct UserType(i32);

#[test]
fn get_publisher_parent_participant() {
    let domain_id = TEST_DOMAIN_ID_GENERATOR.generate_unique_domain_id();
    let domain_participant_factory = DomainParticipantFactory::get_instance();
    let participant = domain_participant_factory
        .create_participant(domain_id, QosKind::Default, NO_LISTENER, NO_STATUS)
        .unwrap();

    let publisher = participant
        .create_publisher(QosKind::Default, NO_LISTENER, NO_STATUS)
        .unwrap();

    let publisher_parent_participant = publisher.get_participant();

    assert_eq!(
        participant.get_instance_handle(),
        publisher_parent_participant.get_instance_handle()
    );
}

#[test]
fn default_data_writer_qos() {
    let domain_id = TEST_DOMAIN_ID_GENERATOR.generate_unique_domain_id();
    let domain_participant_factory = DomainParticipantFactory::get_instance();
    let participant = domain_participant_factory
        .create_participant(domain_id, QosKind::Default, NO_LISTENER, NO_STATUS)
        .unwrap();
    let topic = participant
        .create_topic::<UserType>(
            "default_data_writer_qos",
            "UserType",
            QosKind::Default,
            NO_LISTENER,
            NO_STATUS,
        )
        .unwrap();

    let publisher = participant
        .create_publisher(QosKind::Default, NO_LISTENER, NO_STATUS)
        .unwrap();

    let user_data = vec![1, 2, 3];
    let qos = DataWriterQos {
        user_data: UserDataQosPolicy {
            value: user_data.clone(),
        },
        ..Default::default()
    };

    publisher
        .set_default_datawriter_qos(QosKind::Specific(qos))
        .unwrap();

    let writer = publisher
        .create_datawriter::<UserType>(&topic, QosKind::Default, NO_LISTENER, NO_STATUS)
        .unwrap();

    assert_eq!(
        &publisher
            .get_default_datawriter_qos()
            .unwrap()
            .user_data
            .value,
        &user_data
    );
    assert_eq!(&writer.get_qos().unwrap().user_data.value, &user_data);
}

#[test]
fn different_writers_have_different_instance_handles() {
    let domain_id = TEST_DOMAIN_ID_GENERATOR.generate_unique_domain_id();
    let domain_participant_factory = DomainParticipantFactory::get_instance();
    let participant = domain_participant_factory
        .create_participant(domain_id, QosKind::Default, NO_LISTENER, NO_STATUS)
        .unwrap();
    let topic = participant
        .create_topic::<UserType>(
            "default_data_writer_qos",
            "UserType",
            QosKind::Default,
            NO_LISTENER,
            NO_STATUS,
        )
        .unwrap();

    let publisher1 = participant
        .create_publisher(QosKind::Default, NO_LISTENER, NO_STATUS)
        .unwrap();

    let publisher2 = participant
        .create_publisher(QosKind::Default, NO_LISTENER, NO_STATUS)
        .unwrap();

    let writer1_1 = publisher1
        .create_datawriter::<UserType>(&topic, QosKind::Default, NO_LISTENER, NO_STATUS)
        .unwrap();
    let writer1_2 = publisher1
        .create_datawriter::<UserType>(&topic, QosKind::Default, NO_LISTENER, NO_STATUS)
        .unwrap();
    let writer2_1 = publisher2
        .create_datawriter::<UserType>(&topic, QosKind::Default, NO_LISTENER, NO_STATUS)
        .unwrap();
    let writer2_2 = publisher2
        .create_datawriter::<UserType>(&topic, QosKind::Default, NO_LISTENER, NO_STATUS)
        .unwrap();

    assert_ne!(
        writer1_1.get_instance_handle(),
        writer1_2.get_instance_handle()
    );
    assert_ne!(
        writer1_2.get_instance_handle(),
        writer2_1.get_instance_handle()
    );
    assert_ne!(
        writer2_1.get_instance_handle(),
        writer2_2.get_instance_handle()
    );
}

#[test]
fn data_writer_get_topic_should_return_same_topic_as_used_for_creation() {
    let domain_id = TEST_DOMAIN_ID_GENERATOR.generate_unique_domain_id();
    let domain_participant_factory = DomainParticipantFactory::get_instance();
    let participant = domain_participant_factory
        .create_participant(domain_id, QosKind::Default, NO_LISTENER, NO_STATUS)
        .unwrap();
    let topic = participant
        .create_topic::<UserType>(
            "default_data_writer_qos",
            "UserType",
            QosKind::Default,
            NO_LISTENER,
            NO_STATUS,
        )
        .unwrap();

    let publisher = participant
        .create_publisher(QosKind::Default, NO_LISTENER, NO_STATUS)
        .unwrap();

    let writer = publisher
        .create_datawriter::<UserType>(&topic, QosKind::Default, NO_LISTENER, NO_STATUS)
        .unwrap();

    assert!(writer.get_topic().get_instance_handle() == topic.get_instance_handle())
}

#[test]
fn data_writer_qos_history_keep_all() {
    let domain_id = TEST_DOMAIN_ID_GENERATOR.generate_unique_domain_id();

    let domain_participant = DomainParticipantFactory::get_instance()
        .create_participant(domain_id, QosKind::Default, NO_LISTENER, NO_STATUS)
        .unwrap();

    let topic_qos = TopicQos {
        history: HistoryQosPolicy {
            kind: HistoryQosPolicyKind::KeepAll,
        },
        ..Default::default()
    };
    let topic = domain_participant
        .create_topic::<UserType>(
            "data_writer_qos_history_keep_all",
            UserType::get_type().get_name(),
            QosKind::Specific(topic_qos),
            NO_LISTENER,
            NO_STATUS,
        )
        .unwrap();

    let publisher = domain_participant
        .create_publisher(QosKind::Default, NO_LISTENER, NO_STATUS)
        .unwrap();
    let writer_qos = DataWriterQos {
        history: HistoryQosPolicy {
            kind: HistoryQosPolicyKind::KeepAll,
        },
        ..Default::default()
    };
    let _writer = publisher
        .create_datawriter::<UserType>(
            &topic,
            QosKind::Specific(writer_qos),
            NO_LISTENER,
            NO_STATUS,
        )
        .unwrap();
}
