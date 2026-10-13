use dust_dds::{
    builtin_topics::{
        ParticipantBuiltinTopicData, PublicationBuiltinTopicData, SubscriptionBuiltinTopicData,
        TopicBuiltinTopicData,
    },
    domain::domain_participant_factory::DomainParticipantFactory,
    infrastructure::{
        error::DdsError,
        listener::NO_LISTENER,
        qos::{
            DataReaderQos, DataWriterQos, DomainParticipantQos, PublisherQos, QosKind,
            SubscriberQos, TopicQos,
        },
        qos_policy::{
            GroupDataQosPolicy, ReliabilityQosPolicy, ReliabilityQosPolicyKind, TopicDataQosPolicy,
            UserDataQosPolicy,
        },
        sample_info::{ANY_INSTANCE_STATE, ANY_SAMPLE_STATE, ANY_VIEW_STATE},
        status::{NO_STATUS, StatusKind},
        time::{Duration, DurationKind},
        type_support::DdsType,
    },
    wait_set::{Condition, WaitSet},
};

mod utils;
use crate::utils::domain_id_generator::TEST_DOMAIN_ID_GENERATOR;

#[derive(DdsType)]
struct TestType(u8);

#[derive(Debug, PartialEq, DdsType)]
struct MyData {
    #[dust_dds(key)]
    id: u8,
    value: u8,
}

#[test]
fn create_delete_publisher() {
    let domain_id = TEST_DOMAIN_ID_GENERATOR.generate_unique_domain_id();
    let domain_participant_factory = DomainParticipantFactory::get_instance();
    let participant = domain_participant_factory
        .create_participant(domain_id, QosKind::Default, NO_LISTENER, NO_STATUS)
        .unwrap();

    let publisher = participant
        .create_publisher(QosKind::Default, NO_LISTENER, NO_STATUS)
        .unwrap();

    assert_eq!(participant.delete_publisher(&publisher), Ok(()));
    assert_eq!(publisher.get_qos(), Err(DdsError::AlreadyDeleted));
    assert_eq!(
        participant.delete_publisher(&publisher),
        Err(DdsError::AlreadyDeleted)
    );
}

#[test]
fn create_delete_subscriber() {
    let domain_id = TEST_DOMAIN_ID_GENERATOR.generate_unique_domain_id();
    let domain_participant_factory = DomainParticipantFactory::get_instance();
    let participant = domain_participant_factory
        .create_participant(domain_id, QosKind::Default, NO_LISTENER, NO_STATUS)
        .unwrap();

    let subscriber = participant
        .create_subscriber(QosKind::Default, NO_LISTENER, NO_STATUS)
        .unwrap();

    assert_eq!(participant.delete_subscriber(&subscriber), Ok(()));
    assert_eq!(subscriber.get_qos(), Err(DdsError::AlreadyDeleted));
    assert_eq!(
        participant.delete_subscriber(&subscriber),
        Err(DdsError::AlreadyDeleted)
    );
}

#[test]
fn create_delete_topic() {
    let domain_id = TEST_DOMAIN_ID_GENERATOR.generate_unique_domain_id();
    let domain_participant_factory = DomainParticipantFactory::get_instance();
    let participant = domain_participant_factory
        .create_participant(domain_id, QosKind::Default, NO_LISTENER, NO_STATUS)
        .unwrap();

    let topic = participant
        .create_topic::<TestType>("abc", "TestType", QosKind::Default, NO_LISTENER, NO_STATUS)
        .unwrap();

    assert_eq!(participant.delete_topic(&topic), Ok(()));
    assert_eq!(
        participant.delete_topic(&topic),
        Err(DdsError::AlreadyDeleted)
    );
}

#[test]
fn not_allowed_to_delete_publisher_from_different_participant() {
    let domain_id = TEST_DOMAIN_ID_GENERATOR.generate_unique_domain_id();
    let domain_participant_factory = DomainParticipantFactory::get_instance();
    let participant = domain_participant_factory
        .create_participant(domain_id, QosKind::Default, NO_LISTENER, NO_STATUS)
        .unwrap();
    let other_participant = domain_participant_factory
        .create_participant(domain_id, QosKind::Default, NO_LISTENER, NO_STATUS)
        .unwrap();

    let publisher = participant
        .create_publisher(QosKind::Default, NO_LISTENER, NO_STATUS)
        .unwrap();
    assert_eq!(
        other_participant.delete_publisher(&publisher),
        Err(DdsError::PreconditionNotMet(
            "Publisher can only be deleted from its parent participant".to_string()
        ))
    );
}

#[test]
fn not_allowed_to_delete_subscriber_from_different_participant() {
    let domain_id = TEST_DOMAIN_ID_GENERATOR.generate_unique_domain_id();
    let domain_participant_factory = DomainParticipantFactory::get_instance();
    let participant = domain_participant_factory
        .create_participant(domain_id, QosKind::Default, NO_LISTENER, NO_STATUS)
        .unwrap();
    let other_participant = domain_participant_factory
        .create_participant(domain_id, QosKind::Default, NO_LISTENER, NO_STATUS)
        .unwrap();

    let subscriber = participant
        .create_subscriber(QosKind::Default, NO_LISTENER, NO_STATUS)
        .unwrap();
    assert_eq!(
        other_participant.delete_subscriber(&subscriber),
        Err(DdsError::PreconditionNotMet(
            "Subscriber can only be deleted from its parent participant".to_string()
        ))
    );
}

#[test]
fn not_allowed_to_delete_topic_from_different_participant() {
    let domain_id = TEST_DOMAIN_ID_GENERATOR.generate_unique_domain_id();
    let domain_participant_factory = DomainParticipantFactory::get_instance();
    let participant = domain_participant_factory
        .create_participant(domain_id, QosKind::Default, NO_LISTENER, NO_STATUS)
        .unwrap();
    let other_participant = domain_participant_factory
        .create_participant(domain_id, QosKind::Default, NO_LISTENER, NO_STATUS)
        .unwrap();
    let topic = participant
        .create_topic::<TestType>("abc", "TestType", QosKind::Default, NO_LISTENER, NO_STATUS)
        .unwrap();
    assert_eq!(
        other_participant.delete_topic(&topic),
        Err(DdsError::PreconditionNotMet(
            "Topic can only be deleted from its parent participant".to_string()
        ))
    );
}

#[test]
fn not_allowed_to_delete_publisher_with_writer() {
    let domain_id = TEST_DOMAIN_ID_GENERATOR.generate_unique_domain_id();
    let domain_participant_factory = DomainParticipantFactory::get_instance();
    let participant = domain_participant_factory
        .create_participant(domain_id, QosKind::Default, NO_LISTENER, NO_STATUS)
        .unwrap();

    let writer_topic = participant
        .create_topic::<TestType>("Test", "TestType", QosKind::Default, NO_LISTENER, NO_STATUS)
        .expect("Error creating topic");
    let publisher = participant
        .create_publisher(QosKind::Default, NO_LISTENER, NO_STATUS)
        .unwrap();
    let _a_datawriter = publisher
        .create_datawriter::<TestType>(&writer_topic, QosKind::Default, NO_LISTENER, NO_STATUS)
        .unwrap();

    assert_eq!(
        participant.delete_publisher(&publisher),
        Err(DdsError::PreconditionNotMet(
            "Publisher still contains data writers".to_string()
        ))
    );
}

#[test]
fn not_allowed_to_delete_subscriber_with_reader() {
    let domain_id = TEST_DOMAIN_ID_GENERATOR.generate_unique_domain_id();
    let domain_participant_factory = DomainParticipantFactory::get_instance();
    let participant = domain_participant_factory
        .create_participant(domain_id, QosKind::Default, NO_LISTENER, NO_STATUS)
        .unwrap();

    let reader_topic = participant
        .create_topic::<TestType>("Test", "TestType", QosKind::Default, NO_LISTENER, NO_STATUS)
        .expect("Error creating topic");
    let subscriber = participant
        .create_subscriber(QosKind::Default, NO_LISTENER, NO_STATUS)
        .unwrap();
    let _a_datareader = subscriber
        .create_datareader::<TestType>(&reader_topic, QosKind::Default, NO_LISTENER, NO_STATUS)
        .unwrap();

    assert_eq!(
        participant.delete_subscriber(&subscriber),
        Err(DdsError::PreconditionNotMet(
            "Subscriber still contains data readers".to_string()
        ))
    );
}

#[test]
fn not_allowed_to_delete_topic_attached_to_reader() {
    let domain_id = TEST_DOMAIN_ID_GENERATOR.generate_unique_domain_id();
    let domain_participant_factory = DomainParticipantFactory::get_instance();
    let participant = domain_participant_factory
        .create_participant(domain_id, QosKind::Default, NO_LISTENER, NO_STATUS)
        .unwrap();
    let reader_topic = participant
        .create_topic::<TestType>("Test", "TestType", QosKind::Default, NO_LISTENER, NO_STATUS)
        .expect("Error creating topic");
    let subscriber = participant
        .create_subscriber(QosKind::Default, NO_LISTENER, NO_STATUS)
        .unwrap();
    let _a_datareader = subscriber
        .create_datareader::<TestType>(&reader_topic, QosKind::Default, NO_LISTENER, NO_STATUS)
        .unwrap();

    assert_eq!(
        participant.delete_topic(&reader_topic),
        Err(DdsError::PreconditionNotMet(
            "Topic still attached to some data writer or data reader".to_string()
        ))
    );
}

#[test]
fn not_allowed_to_delete_topic_attached_to_writer() {
    let domain_id = TEST_DOMAIN_ID_GENERATOR.generate_unique_domain_id();
    let domain_participant_factory = DomainParticipantFactory::get_instance();
    let participant = domain_participant_factory
        .create_participant(domain_id, QosKind::Default, NO_LISTENER, NO_STATUS)
        .unwrap();
    let writer_topic = participant
        .create_topic::<TestType>("Test", "TestType", QosKind::Default, NO_LISTENER, NO_STATUS)
        .expect("Error creating topic");
    let publisher = participant
        .create_publisher(QosKind::Default, NO_LISTENER, NO_STATUS)
        .unwrap();
    let _a_datawriter = publisher
        .create_datawriter::<TestType>(&writer_topic, QosKind::Default, NO_LISTENER, NO_STATUS)
        .unwrap();

    assert_eq!(
        participant.delete_topic(&writer_topic),
        Err(DdsError::PreconditionNotMet(
            "Topic still attached to some data writer or data reader".to_string()
        ))
    );
}

#[test]
fn not_allowed_to_create_topic_with_builtin_topic_name() {
    let domain_id = TEST_DOMAIN_ID_GENERATOR.generate_unique_domain_id();
    let domain_participant_factory = DomainParticipantFactory::get_instance();
    let participant = domain_participant_factory
        .create_participant(domain_id, QosKind::Default, NO_LISTENER, NO_STATUS)
        .unwrap();

    assert!(
        participant
            .create_topic::<TestType>(
                "DCPSParticipant",
                "TestType",
                QosKind::Default,
                NO_LISTENER,
                NO_STATUS
            )
            .is_err()
    );

    assert!(
        participant
            .create_topic::<TestType>(
                "DCPSTopic",
                "TestType",
                QosKind::Default,
                NO_LISTENER,
                NO_STATUS
            )
            .is_err()
    );

    assert!(
        participant
            .create_topic::<TestType>(
                "DCPSPublication",
                "TestType",
                QosKind::Default,
                NO_LISTENER,
                NO_STATUS
            )
            .is_err()
    );

    assert!(
        participant
            .create_topic::<TestType>(
                "DCPSSubscription",
                "TestType",
                QosKind::Default,
                NO_LISTENER,
                NO_STATUS
            )
            .is_err()
    );
}

#[test]
fn allowed_to_delete_publisher_with_created_and_deleted_writer() {
    let domain_id = TEST_DOMAIN_ID_GENERATOR.generate_unique_domain_id();
    let domain_participant_factory = DomainParticipantFactory::get_instance();
    let participant = domain_participant_factory
        .create_participant(domain_id, QosKind::Default, NO_LISTENER, NO_STATUS)
        .unwrap();

    let writer_topic = participant
        .create_topic::<TestType>("Test", "TestType", QosKind::Default, NO_LISTENER, NO_STATUS)
        .expect("Error creating topic");
    let publisher = participant
        .create_publisher(QosKind::Default, NO_LISTENER, NO_STATUS)
        .unwrap();
    let a_datawriter = publisher
        .create_datawriter::<TestType>(&writer_topic, QosKind::Default, NO_LISTENER, NO_STATUS)
        .unwrap();
    publisher
        .delete_datawriter(&a_datawriter)
        .expect("Failed to delete datawriter");
    assert_eq!(participant.delete_publisher(&publisher), Ok(()));
}

#[test]
fn allowed_to_delete_subscriber_with_created_and_deleted_reader() {
    let domain_id = TEST_DOMAIN_ID_GENERATOR.generate_unique_domain_id();
    let domain_participant_factory = DomainParticipantFactory::get_instance();
    let participant = domain_participant_factory
        .create_participant(domain_id, QosKind::Default, NO_LISTENER, NO_STATUS)
        .unwrap();
    let reader_topic = participant
        .create_topic::<TestType>("Test", "TestType", QosKind::Default, NO_LISTENER, NO_STATUS)
        .expect("Error creating topic");
    let subscriber = participant
        .create_subscriber(QosKind::Default, NO_LISTENER, NO_STATUS)
        .unwrap();
    let a_datareader = subscriber
        .create_datareader::<TestType>(&reader_topic, QosKind::Default, NO_LISTENER, NO_STATUS)
        .unwrap();
    subscriber
        .delete_datareader(&a_datareader)
        .expect("Failed to delete datareader");
    assert_eq!(participant.delete_subscriber(&subscriber), Ok(()));
}

#[test]
fn allowed_to_delete_topic_with_created_and_deleted_writer() {
    let domain_id = TEST_DOMAIN_ID_GENERATOR.generate_unique_domain_id();
    let domain_participant_factory = DomainParticipantFactory::get_instance();
    let participant = domain_participant_factory
        .create_participant(domain_id, QosKind::Default, NO_LISTENER, NO_STATUS)
        .unwrap();
    let writer_topic = participant
        .create_topic::<TestType>("Test", "TestType", QosKind::Default, NO_LISTENER, NO_STATUS)
        .expect("Error creating topic");
    let publisher = participant
        .create_publisher(QosKind::Default, NO_LISTENER, NO_STATUS)
        .unwrap();
    let a_datawriter = publisher
        .create_datawriter::<TestType>(&writer_topic, QosKind::Default, NO_LISTENER, NO_STATUS)
        .unwrap();
    publisher
        .delete_datawriter(&a_datawriter)
        .expect("Failed to delete datawriter");
    assert_eq!(participant.delete_topic(&writer_topic), Ok(()));
}

#[test]
fn allowed_to_delete_topic_with_created_and_deleted_reader() {
    let domain_id = TEST_DOMAIN_ID_GENERATOR.generate_unique_domain_id();
    let domain_participant_factory = DomainParticipantFactory::get_instance();
    let participant = domain_participant_factory
        .create_participant(domain_id, QosKind::Default, NO_LISTENER, NO_STATUS)
        .unwrap();
    let reader_topic = participant
        .create_topic::<TestType>("Test", "TestType", QosKind::Default, NO_LISTENER, NO_STATUS)
        .expect("Error creating topic");
    let subscriber = participant
        .create_subscriber(QosKind::Default, NO_LISTENER, NO_STATUS)
        .unwrap();
    let a_datareader = subscriber
        .create_datareader::<TestType>(&reader_topic, QosKind::Default, NO_LISTENER, NO_STATUS)
        .unwrap();
    subscriber
        .delete_datareader(&a_datareader)
        .expect("Failed to delete datareader");
    assert_eq!(participant.delete_topic(&reader_topic), Ok(()));
}

#[test]
fn default_publisher_qos() {
    let domain_id = TEST_DOMAIN_ID_GENERATOR.generate_unique_domain_id();
    let domain_participant_factory = DomainParticipantFactory::get_instance();
    let participant = domain_participant_factory
        .create_participant(domain_id, QosKind::Default, NO_LISTENER, NO_STATUS)
        .unwrap();

    let group_data = vec![1, 2, 3];
    let qos = PublisherQos {
        group_data: GroupDataQosPolicy {
            value: group_data.clone(),
        },
        ..Default::default()
    };

    participant
        .set_default_publisher_qos(QosKind::Specific(qos))
        .unwrap();

    let publisher = participant
        .create_publisher(QosKind::Default, NO_LISTENER, NO_STATUS)
        .unwrap();

    assert_eq!(
        &participant
            .get_default_publisher_qos()
            .unwrap()
            .group_data
            .value,
        &group_data
    );
    assert_eq!(&publisher.get_qos().unwrap().group_data.value, &group_data);
}

#[test]
fn default_subscriber_qos() {
    let domain_id = TEST_DOMAIN_ID_GENERATOR.generate_unique_domain_id();
    let domain_participant_factory = DomainParticipantFactory::get_instance();
    let participant = domain_participant_factory
        .create_participant(domain_id, QosKind::Default, NO_LISTENER, NO_STATUS)
        .unwrap();

    let group_data = vec![1, 2, 3];
    let qos = SubscriberQos {
        group_data: GroupDataQosPolicy {
            value: group_data.clone(),
        },
        ..Default::default()
    };

    participant
        .set_default_subscriber_qos(QosKind::Specific(qos))
        .unwrap();

    let subscriber = participant
        .create_subscriber(QosKind::Default, NO_LISTENER, NO_STATUS)
        .unwrap();

    assert_eq!(
        &participant
            .get_default_subscriber_qos()
            .unwrap()
            .group_data
            .value,
        &group_data
    );
    assert_eq!(&subscriber.get_qos().unwrap().group_data.value, &group_data);
}

#[test]
fn default_topic_qos() {
    let domain_id = TEST_DOMAIN_ID_GENERATOR.generate_unique_domain_id();
    let domain_participant_factory = DomainParticipantFactory::get_instance();
    let participant = domain_participant_factory
        .create_participant(domain_id, QosKind::Default, NO_LISTENER, NO_STATUS)
        .unwrap();
    let topic_data = vec![1, 2, 3];
    let qos = TopicQos {
        topic_data: TopicDataQosPolicy {
            value: topic_data.clone(),
        },
        ..Default::default()
    };

    participant
        .set_default_topic_qos(QosKind::Specific(qos))
        .unwrap();

    let topic = participant
        .create_topic::<TestType>(
            "default_topic_qos",
            "TestType",
            QosKind::Default,
            NO_LISTENER,
            NO_STATUS,
        )
        .unwrap();

    assert_eq!(
        &participant
            .get_default_topic_qos()
            .unwrap()
            .topic_data
            .value,
        &topic_data
    );

    assert_eq!(&topic.get_qos().unwrap().topic_data.value, &topic_data)
}

#[test]
fn builtin_topic_access() {
    let domain_id = TEST_DOMAIN_ID_GENERATOR.generate_unique_domain_id();

    let participant = DomainParticipantFactory::get_instance()
        .create_participant(domain_id, QosKind::Default, NO_LISTENER, NO_STATUS)
        .unwrap();

    assert!(
        participant
            .lookup_topicdescription("DCPSParticipant")
            .is_ok()
    );

    assert!(participant.lookup_topicdescription("DCPSTopic").is_ok());

    assert!(
        participant
            .lookup_topicdescription("DCPSPublication")
            .is_ok()
    );

    assert!(
        participant
            .lookup_topicdescription("DCPSSubscription")
            .is_ok()
    );
}

#[test]
fn builtin_topics_accessible_after_delete_contained_entities() {
    let domain_id = TEST_DOMAIN_ID_GENERATOR.generate_unique_domain_id();

    let participant = DomainParticipantFactory::get_instance()
        .create_participant(domain_id, QosKind::Default, NO_LISTENER, NO_STATUS)
        .unwrap();

    participant.delete_contained_entities().unwrap();

    assert!(
        participant
            .lookup_topicdescription("DCPSParticipant")
            .is_ok()
    );

    assert!(participant.lookup_topicdescription("DCPSTopic").is_ok());

    assert!(
        participant
            .lookup_topicdescription("DCPSPublication")
            .is_ok()
    );

    assert!(
        participant
            .lookup_topicdescription("DCPSSubscription")
            .is_ok()
    );
}

#[test]
fn builtin_reader_access() {
    let domain_id = TEST_DOMAIN_ID_GENERATOR.generate_unique_domain_id();

    let participant = DomainParticipantFactory::get_instance()
        .create_participant(domain_id, QosKind::Default, NO_LISTENER, NO_STATUS)
        .unwrap();

    let builtin_subscriber = participant.get_builtin_subscriber();

    assert!(
        builtin_subscriber
            .lookup_datareader::<ParticipantBuiltinTopicData>("DCPSParticipant")
            .is_ok()
    );

    assert!(
        builtin_subscriber
            .lookup_datareader::<TopicBuiltinTopicData>("DCPSTopic")
            .is_ok()
    );

    assert!(
        builtin_subscriber
            .lookup_datareader::<PublicationBuiltinTopicData>("DCPSPublication")
            .is_ok()
    );

    assert!(
        builtin_subscriber
            .lookup_datareader::<SubscriptionBuiltinTopicData>("DCPSSubscription")
            .is_ok()
    );
}

#[test]
fn get_discovery_data_from_builtin_reader() {
    let domain_id = TEST_DOMAIN_ID_GENERATOR.generate_unique_domain_id();
    let participant_user_data = vec![1, 2];
    let topic_user_data = vec![3, 4];
    let reader_user_data = vec![5, 6];
    let writer_user_data = vec![7, 8];

    let participant = DomainParticipantFactory::get_instance()
        .create_participant(
            domain_id,
            QosKind::Specific(DomainParticipantQos {
                user_data: UserDataQosPolicy {
                    value: participant_user_data.clone(),
                },
                ..Default::default()
            }),
            NO_LISTENER,
            NO_STATUS,
        )
        .unwrap();

    let topic = participant
        .create_topic::<TestType>(
            "topic_name",
            "TestType",
            QosKind::Specific(TopicQos {
                topic_data: TopicDataQosPolicy {
                    value: topic_user_data.clone(),
                },
                ..Default::default()
            }),
            NO_LISTENER,
            NO_STATUS,
        )
        .unwrap();

    let publisher = participant
        .create_publisher(QosKind::Default, NO_LISTENER, NO_STATUS)
        .unwrap();

    let data_writer = publisher
        .create_datawriter::<MyData>(
            &topic,
            QosKind::Specific(DataWriterQos {
                user_data: UserDataQosPolicy {
                    value: writer_user_data.clone(),
                },
                ..Default::default()
            }),
            NO_LISTENER,
            NO_STATUS,
        )
        .unwrap();

    let subscriber = participant
        .create_subscriber(QosKind::Default, NO_LISTENER, NO_STATUS)
        .unwrap();

    let data_reader = subscriber
        .create_datareader::<MyData>(
            &topic,
            QosKind::Specific(DataReaderQos {
                user_data: UserDataQosPolicy {
                    value: reader_user_data.clone(),
                },
                ..Default::default()
            }),
            NO_LISTENER,
            NO_STATUS,
        )
        .unwrap();

    // Wait for reader to be matched to data writer
    let cond = data_writer.get_statuscondition();
    cond.set_enabled_statuses(&[StatusKind::PublicationMatched])
        .unwrap();

    let mut wait_set = WaitSet::new();
    wait_set
        .attach_condition(Condition::StatusCondition(cond))
        .unwrap();
    wait_set.wait(Duration::new(10, 0)).unwrap();

    // Wait for the writer to be matched to data reader
    let cond = data_reader.get_statuscondition();
    cond.set_enabled_statuses(&[StatusKind::SubscriptionMatched])
        .unwrap();

    let mut wait_set = WaitSet::new();
    wait_set
        .attach_condition(Condition::StatusCondition(cond))
        .unwrap();
    wait_set.wait(Duration::new(10, 0)).unwrap();

    let builtin_subscriber = participant.get_builtin_subscriber();

    let participants_reader = builtin_subscriber
        .lookup_datareader::<ParticipantBuiltinTopicData>("DCPSParticipant")
        .unwrap()
        .unwrap();

    let topics_reader = builtin_subscriber
        .lookup_datareader::<TopicBuiltinTopicData>("DCPSTopic")
        .unwrap()
        .unwrap();

    let publications_reader = builtin_subscriber
        .lookup_datareader::<PublicationBuiltinTopicData>("DCPSPublication")
        .unwrap()
        .unwrap();

    let subscriptions_reader = builtin_subscriber
        .lookup_datareader::<SubscriptionBuiltinTopicData>("DCPSSubscription")
        .unwrap()
        .unwrap();

    let participant_samples = participants_reader
        .read(1, ANY_SAMPLE_STATE, ANY_VIEW_STATE, ANY_INSTANCE_STATE)
        .unwrap();

    let subscription_samples = subscriptions_reader
        .read(1, ANY_SAMPLE_STATE, ANY_VIEW_STATE, ANY_INSTANCE_STATE)
        .unwrap();

    let publication_samples = publications_reader
        .read(1, ANY_SAMPLE_STATE, ANY_VIEW_STATE, ANY_INSTANCE_STATE)
        .unwrap();

    let start_time = std::time::Instant::now();
    loop {
        if let Ok(topic_samples) =
            topics_reader.read(1, ANY_SAMPLE_STATE, ANY_VIEW_STATE, ANY_INSTANCE_STATE)
        {
            assert_eq!(
                &topic_samples[0].data.as_ref().unwrap().topic_data().value,
                &topic_user_data
            );
            break;
        }

        if start_time.elapsed() > std::time::Duration::from_secs(10) {
            panic!("Topic discovery data not received")
        }
    }

    assert_eq!(
        &participant_samples[0]
            .data
            .as_ref()
            .unwrap()
            .user_data()
            .value,
        &participant_user_data
    );

    assert_eq!(
        &subscription_samples[0]
            .data
            .as_ref()
            .unwrap()
            .user_data()
            .value,
        &reader_user_data
    );

    assert_eq!(
        &publication_samples[0]
            .data
            .as_ref()
            .unwrap()
            .user_data()
            .value,
        &writer_user_data
    );
}

#[test]
fn ignore_participant() {
    let domain_id = TEST_DOMAIN_ID_GENERATOR.generate_unique_domain_id();
    let domain_participant_factory = DomainParticipantFactory::get_instance();
    let participant1 = domain_participant_factory
        .create_participant(domain_id, QosKind::Default, NO_LISTENER, NO_STATUS)
        .unwrap();

    let participant2 = domain_participant_factory
        .create_participant(domain_id, QosKind::Default, NO_LISTENER, NO_STATUS)
        .unwrap();

    participant1
        .ignore_participant(participant2.get_instance_handle())
        .unwrap();

    std::thread::sleep(std::time::Duration::from_secs(5));

    // Participant 1
    let participants = participant1.get_discovered_participants().unwrap();
    assert_eq!(participants.len(), 1);
    assert_eq!(participants[0], participant1.get_instance_handle());

    // Participant 2
    let participants = participant2.get_discovered_participants().unwrap();
    assert_eq!(participants.len(), 2);
    assert!(participants.contains(&participant2.get_instance_handle()));
    assert!(participants.contains(&participant1.get_instance_handle()));
}

#[test]
#[ignore = "Functionality needs to be revisited"]
fn ignore_publication() {
    let domain_id = TEST_DOMAIN_ID_GENERATOR.generate_unique_domain_id();
    let domain_participant_factory = DomainParticipantFactory::get_instance();
    let participant = domain_participant_factory
        .create_participant(domain_id, QosKind::Default, NO_LISTENER, NO_STATUS)
        .unwrap();
    let topic = participant
        .create_topic::<MyData>(
            "MyTopic",
            "MyData",
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
        .create_datawriter::<MyData>(
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

    participant
        .ignore_publication(writer.get_instance_handle())
        .unwrap();

    let reader = subscriber
        .create_datareader::<MyData>(
            &topic,
            QosKind::Specific(reader_qos),
            NO_LISTENER,
            NO_STATUS,
        )
        .unwrap();

    // Readers and writers from ignored participant should never match
    let cond = reader.get_statuscondition();
    cond.set_enabled_statuses(&[StatusKind::SubscriptionMatched])
        .unwrap();

    let mut wait_set = WaitSet::new();
    wait_set
        .attach_condition(Condition::StatusCondition(cond))
        .unwrap();
    assert!(wait_set.wait(Duration::new(2, 0)).is_err());
}

#[test]
#[ignore = "Functionality needs to be revisited"]
fn ignore_subscription() {
    let domain_id = TEST_DOMAIN_ID_GENERATOR.generate_unique_domain_id();
    let domain_participant_factory = DomainParticipantFactory::get_instance();
    let participant = domain_participant_factory
        .create_participant(domain_id, QosKind::Default, NO_LISTENER, NO_STATUS)
        .unwrap();
    let topic = participant
        .create_topic::<MyData>(
            "MyTopic",
            "MyData",
            QosKind::Default,
            NO_LISTENER,
            NO_STATUS,
        )
        .unwrap();

    let publisher = participant
        .create_publisher(QosKind::Default, NO_LISTENER, NO_STATUS)
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
        .create_datareader::<MyData>(
            &topic,
            QosKind::Specific(reader_qos),
            NO_LISTENER,
            NO_STATUS,
        )
        .unwrap();

    participant
        .ignore_subscription(reader.get_instance_handle())
        .unwrap();

    let writer_qos = DataWriterQos {
        reliability: ReliabilityQosPolicy {
            kind: ReliabilityQosPolicyKind::Reliable,
            max_blocking_time: DurationKind::Finite(Duration::new(1, 0)),
        },
        ..Default::default()
    };
    let writer = publisher
        .create_datawriter::<MyData>(
            &topic,
            QosKind::Specific(writer_qos),
            NO_LISTENER,
            NO_STATUS,
        )
        .unwrap();

    // Readers and writers from ignored participant should never match
    let cond = writer.get_statuscondition();
    cond.set_enabled_statuses(&[StatusKind::PublicationMatched])
        .unwrap();

    let mut wait_set = WaitSet::new();
    wait_set
        .attach_condition(Condition::StatusCondition(cond))
        .unwrap();
    assert!(wait_set.wait(Duration::new(2, 0)).is_err());
}

#[test]
fn find_topic_timeout() {
    let domain_id = TEST_DOMAIN_ID_GENERATOR.generate_unique_domain_id();
    let domain_participant_factory = DomainParticipantFactory::get_instance();
    let participant = domain_participant_factory
        .create_participant(domain_id, QosKind::Default, NO_LISTENER, NO_STATUS)
        .unwrap();

    let topic = participant.find_topic::<TestType>("abc", Duration::new(0, 500_000));

    assert!(matches!(topic, Err(DdsError::Timeout)));
}

#[test]
#[ignore = "Not yet implemented"]
fn create_delete_content_filtered_topic() {
    let domain_id = TEST_DOMAIN_ID_GENERATOR.generate_unique_domain_id();
    let domain_participant_factory = DomainParticipantFactory::get_instance();
    let participant = domain_participant_factory
        .create_participant(domain_id, QosKind::Default, NO_LISTENER, NO_STATUS)
        .unwrap();

    let topic = participant
        .create_topic::<TestType>("abc", "TestType", QosKind::Default, NO_LISTENER, NO_STATUS)
        .unwrap();

    let filter_expression = String::from(r#"value =  %0 "#);
    let expression_parameters = vec![String::from(r#"10"#)];
    let content_filtered_topic = participant
        .create_contentfilteredtopic(
            "filter_abc",
            &topic,
            filter_expression,
            expression_parameters,
        )
        .unwrap();

    assert_eq!(
        participant.delete_contentfilteredtopic(&content_filtered_topic),
        Ok(())
    );
    assert_eq!(
        participant.delete_contentfilteredtopic(&content_filtered_topic),
        Err(DdsError::AlreadyDeleted)
    );
}
