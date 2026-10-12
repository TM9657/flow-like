use dust_dds::{
    domain::domain_participant_factory::DomainParticipantFactory,
    infrastructure::{
        listener::NO_LISTENER,
        qos::{DataReaderQos, QosKind},
        qos_policy::UserDataQosPolicy,
        status::NO_STATUS,
        type_support::DdsType,
    },
    topic_definition::topic_description::TopicDescription,
};

mod utils;
use crate::utils::domain_id_generator::TEST_DOMAIN_ID_GENERATOR;

#[derive(DdsType)]
struct UserType(i32);

#[test]
fn get_subscriber_parent_participant() {
    let domain_id = TEST_DOMAIN_ID_GENERATOR.generate_unique_domain_id();
    let domain_participant_factory = DomainParticipantFactory::get_instance();
    let participant = domain_participant_factory
        .create_participant(domain_id, QosKind::Default, NO_LISTENER, NO_STATUS)
        .unwrap();

    let subscriber = participant
        .create_subscriber(QosKind::Default, NO_LISTENER, NO_STATUS)
        .unwrap();

    let subscriber_parent_participant = subscriber.get_participant();

    assert_eq!(
        participant.get_instance_handle(),
        subscriber_parent_participant.get_instance_handle()
    );
}

#[test]
fn default_data_reader_qos() {
    let domain_id = TEST_DOMAIN_ID_GENERATOR.generate_unique_domain_id();
    let domain_participant_factory = DomainParticipantFactory::get_instance();
    let participant = domain_participant_factory
        .create_participant(domain_id, QosKind::Default, NO_LISTENER, NO_STATUS)
        .unwrap();
    let topic = participant
        .create_topic::<UserType>(
            "default_data_reader_qos",
            "UserType",
            QosKind::Default,
            NO_LISTENER,
            NO_STATUS,
        )
        .unwrap();

    let subscriber = participant
        .create_subscriber(QosKind::Default, NO_LISTENER, NO_STATUS)
        .unwrap();

    let user_data = vec![1, 2, 3];
    let qos = DataReaderQos {
        user_data: UserDataQosPolicy {
            value: user_data.clone(),
        },
        ..Default::default()
    };

    subscriber
        .set_default_datareader_qos(QosKind::Specific(qos))
        .unwrap();

    let reader = subscriber
        .create_datareader::<UserType>(&topic, QosKind::Default, NO_LISTENER, NO_STATUS)
        .unwrap();

    assert_eq!(
        &subscriber
            .get_default_datareader_qos()
            .unwrap()
            .user_data
            .value,
        &user_data
    );
    assert_eq!(&reader.get_qos().unwrap().user_data.value, &user_data);
}

#[test]
fn different_readers_have_different_instance_handles() {
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

    let subscriber1 = participant
        .create_subscriber(QosKind::Default, NO_LISTENER, NO_STATUS)
        .unwrap();

    let subscriber2 = participant
        .create_subscriber(QosKind::Default, NO_LISTENER, NO_STATUS)
        .unwrap();

    let reader1_1 = subscriber1
        .create_datareader::<UserType>(&topic, QosKind::Default, NO_LISTENER, NO_STATUS)
        .unwrap();
    let reader1_2 = subscriber1
        .create_datareader::<UserType>(&topic, QosKind::Default, NO_LISTENER, NO_STATUS)
        .unwrap();
    let reader2_1 = subscriber2
        .create_datareader::<UserType>(&topic, QosKind::Default, NO_LISTENER, NO_STATUS)
        .unwrap();
    let reader2_2 = subscriber2
        .create_datareader::<UserType>(&topic, QosKind::Default, NO_LISTENER, NO_STATUS)
        .unwrap();

    assert_ne!(
        reader1_1.get_instance_handle(),
        reader1_2.get_instance_handle()
    );
    assert_ne!(
        reader1_2.get_instance_handle(),
        reader2_1.get_instance_handle()
    );
    assert_ne!(
        reader2_1.get_instance_handle(),
        reader2_2.get_instance_handle()
    );
}

#[test]
fn data_reader_get_topicdescription_should_return_same_topic_as_used_for_creation() {
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

    let subscriber = participant
        .create_subscriber(QosKind::Default, NO_LISTENER, NO_STATUS)
        .unwrap();

    let reader = subscriber
        .create_datareader::<UserType>(&topic, QosKind::Default, NO_LISTENER, NO_STATUS)
        .unwrap();

    assert_eq!(reader.get_topicdescription().get_name(), topic.get_name())
}
