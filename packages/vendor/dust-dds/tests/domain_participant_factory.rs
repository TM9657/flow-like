use dust_dds::{
    domain::domain_participant_factory::DomainParticipantFactory,
    infrastructure::{
        listener::NO_LISTENER,
        qos::{DomainParticipantQos, QosKind},
        qos_policy::UserDataQosPolicy,
        status::NO_STATUS,
        type_support::DdsType,
    },
};

mod utils;
use crate::utils::domain_id_generator::TEST_DOMAIN_ID_GENERATOR;

#[derive(Debug, PartialEq, DdsType)]
struct KeyedData {
    #[dust_dds(key)]
    id: u8,
    value: u8,
}

#[test]
fn default_participant_qos() {
    let domain_id = TEST_DOMAIN_ID_GENERATOR.generate_unique_domain_id();
    let domain_participant_factory = DomainParticipantFactory::get_instance();

    let user_data = vec![1, 2, 3];
    let qos = DomainParticipantQos {
        user_data: UserDataQosPolicy {
            value: user_data.clone(),
        },
        ..Default::default()
    };

    domain_participant_factory
        .set_default_participant_qos(QosKind::Specific(qos))
        .unwrap();

    let participant = domain_participant_factory
        .create_participant(domain_id, QosKind::Default, NO_LISTENER, NO_STATUS)
        .unwrap();

    domain_participant_factory
        .set_default_participant_qos(QosKind::Default)
        .unwrap();

    assert_eq!(participant.get_qos().unwrap().user_data.value, user_data);
}

#[test]
fn create_delete_participant() {
    let domain_id = TEST_DOMAIN_ID_GENERATOR.generate_unique_domain_id();
    let domain_participant_factory = DomainParticipantFactory::get_instance();
    let participant = domain_participant_factory
        .create_participant(domain_id, QosKind::Default, NO_LISTENER, NO_STATUS)
        .unwrap();
    assert!(
        domain_participant_factory
            .delete_participant(&participant)
            .is_ok()
    )
}

#[test]
fn not_allowed_to_delete_participant_with_entities() {
    let domain_id = TEST_DOMAIN_ID_GENERATOR.generate_unique_domain_id();
    let domain_participant_factory = DomainParticipantFactory::get_instance();
    let participant = domain_participant_factory
        .create_participant(domain_id, QosKind::Default, NO_LISTENER, NO_STATUS)
        .unwrap();
    let topic = participant
        .create_topic::<KeyedData>(
            "Test",
            "KeyedData",
            QosKind::Default,
            NO_LISTENER,
            NO_STATUS,
        )
        .expect("Error creating topic");
    let subscriber = participant
        .create_subscriber(QosKind::Default, NO_LISTENER, NO_STATUS)
        .unwrap();
    let _datareader = subscriber
        .create_datareader::<KeyedData>(&topic, QosKind::Default, NO_LISTENER, NO_STATUS)
        .unwrap();

    let publisher = participant
        .create_publisher(QosKind::Default, NO_LISTENER, NO_STATUS)
        .unwrap();
    let _datawriter = publisher
        .create_datawriter::<KeyedData>(&topic, QosKind::Default, NO_LISTENER, NO_STATUS)
        .unwrap();

    assert!(
        domain_participant_factory
            .delete_participant(&participant)
            .is_err()
    );
}

#[test]
fn allowed_to_delete_participant_after_delete_contained_entities() {
    let domain_id = TEST_DOMAIN_ID_GENERATOR.generate_unique_domain_id();
    let domain_participant_factory = DomainParticipantFactory::get_instance();
    let participant = domain_participant_factory
        .create_participant(domain_id, QosKind::Default, NO_LISTENER, NO_STATUS)
        .unwrap();
    let topic = participant
        .create_topic::<KeyedData>(
            "Test",
            "KeyedData",
            QosKind::Default,
            NO_LISTENER,
            NO_STATUS,
        )
        .expect("Error creating topic");
    let subscriber = participant
        .create_subscriber(QosKind::Default, NO_LISTENER, NO_STATUS)
        .unwrap();
    let _datareader = subscriber
        .create_datareader::<KeyedData>(&topic, QosKind::Default, NO_LISTENER, NO_STATUS)
        .unwrap();

    let publisher = participant
        .create_publisher(QosKind::Default, NO_LISTENER, NO_STATUS)
        .unwrap();
    let _datawriter = publisher
        .create_datawriter::<KeyedData>(&topic, QosKind::Default, NO_LISTENER, NO_STATUS)
        .unwrap();

    participant.delete_contained_entities().unwrap();

    assert!(
        domain_participant_factory
            .delete_participant(&participant)
            .is_ok()
    );
}
