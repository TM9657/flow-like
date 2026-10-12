use dust_dds::{
    domain::domain_participant_factory::DomainParticipantFactory,
    infrastructure::{
        error::DdsError,
        instance::HANDLE_NIL,
        listener::NO_LISTENER,
        qos::{DomainParticipantFactoryQos, QosKind},
        qos_policy::EntityFactoryQosPolicy,
        status::NO_STATUS,
    },
};

mod utils;
use crate::utils::domain_id_generator::TEST_DOMAIN_ID_GENERATOR;

#[test]
fn create_not_enabled_entities() {
    let domain_id = TEST_DOMAIN_ID_GENERATOR.generate_unique_domain_id();
    let domain_participant_factory = DomainParticipantFactory::get_instance();

    let qos = DomainParticipantFactoryQos {
        entity_factory: EntityFactoryQosPolicy {
            autoenable_created_entities: false,
        },
    };

    domain_participant_factory
        .set_qos(QosKind::Specific(qos))
        .unwrap();

    let participant = domain_participant_factory
        .create_participant(domain_id, QosKind::Default, NO_LISTENER, NO_STATUS)
        .unwrap();

    // Call an operation that should return a NotEnabled error as a check the QoS is taken
    let result = participant.ignore_publication(HANDLE_NIL);

    // Teardown before assert: Set qos back to original to prevent it affecting other test
    domain_participant_factory
        .set_qos(QosKind::Default)
        .unwrap();

    assert_eq!(result, Err(DdsError::NotEnabled));
}
