use std::time::Instant;

use dust_dds::{
    domain::domain_participant_factory::DomainParticipantFactory,
    infrastructure::{
        listener::NO_LISTENER,
        qos::{DataReaderQos, DataWriterQos, PublisherQos, QosKind, SubscriberQos},
        qos_policy::{
            DataRepresentationQosPolicy, OwnershipQosPolicy, OwnershipQosPolicyKind,
            PartitionQosPolicy, UserDataQosPolicy, XCDR_DATA_REPRESENTATION,
            XCDR2_DATA_REPRESENTATION,
        },
        status::{NO_STATUS, StatusKind},
        time::Duration,
        type_support::DdsType,
    },
    wait_set::{Condition, WaitSet},
};

mod utils;
use crate::utils::domain_id_generator::TEST_DOMAIN_ID_GENERATOR;

#[derive(DdsType)]
struct UserType(#[dust_dds(key)] i32);

#[test]
fn writer_discovers_reader_in_same_participant() {
    let domain_id = TEST_DOMAIN_ID_GENERATOR.generate_unique_domain_id();
    let dp = DomainParticipantFactory::get_instance()
        .create_participant(domain_id, QosKind::Default, NO_LISTENER, NO_STATUS)
        .unwrap();
    let topic = dp
        .create_topic::<UserType>(
            "topic_name",
            "UserType",
            QosKind::Default,
            NO_LISTENER,
            NO_STATUS,
        )
        .unwrap();
    let publisher = dp
        .create_publisher(QosKind::Default, NO_LISTENER, NO_STATUS)
        .unwrap();
    let data_writer = publisher
        .create_datawriter::<UserType>(&topic, QosKind::Default, NO_LISTENER, NO_STATUS)
        .unwrap();
    let subscriber = dp
        .create_subscriber(QosKind::Default, NO_LISTENER, NO_STATUS)
        .unwrap();
    let _data_reader = subscriber
        .create_datareader::<UserType>(&topic, QosKind::Default, NO_LISTENER, NO_STATUS)
        .unwrap();
    let cond = data_writer.get_statuscondition();
    cond.set_enabled_statuses(&[StatusKind::PublicationMatched])
        .unwrap();

    let mut wait_set = WaitSet::new();
    wait_set
        .attach_condition(Condition::StatusCondition(cond))
        .unwrap();
    wait_set.wait(Duration::new(5, 0)).unwrap();

    let matched_subscriptions = data_writer.get_matched_subscriptions().unwrap();
    assert_eq!(matched_subscriptions.len(), 1);
    let matched_subscription_data = data_writer
        .get_matched_subscription_data(matched_subscriptions[0])
        .unwrap();
    assert_eq!(
        &matched_subscription_data.participant_key().value,
        dp.get_instance_handle().as_ref()
    )
}

#[test]
fn deleted_readers_are_disposed_from_writer() {
    let domain_id = TEST_DOMAIN_ID_GENERATOR.generate_unique_domain_id();
    let dp = DomainParticipantFactory::get_instance()
        .create_participant(domain_id, QosKind::Default, NO_LISTENER, NO_STATUS)
        .unwrap();
    let topic = dp
        .create_topic::<UserType>(
            "topic_name",
            "UserType",
            QosKind::Default,
            NO_LISTENER,
            NO_STATUS,
        )
        .unwrap();
    let publisher = dp
        .create_publisher(QosKind::Default, NO_LISTENER, NO_STATUS)
        .unwrap();
    let data_writer = publisher
        .create_datawriter::<UserType>(&topic, QosKind::Default, NO_LISTENER, NO_STATUS)
        .unwrap();
    let subscriber = dp
        .create_subscriber(QosKind::Default, NO_LISTENER, NO_STATUS)
        .unwrap();
    let data_reader = subscriber
        .create_datareader::<UserType>(&topic, QosKind::Default, NO_LISTENER, NO_STATUS)
        .unwrap();
    let cond = data_writer.get_statuscondition();
    cond.set_enabled_statuses(&[StatusKind::PublicationMatched])
        .unwrap();

    let mut wait_set = WaitSet::new();
    wait_set
        .attach_condition(Condition::StatusCondition(cond))
        .unwrap();
    wait_set.wait(Duration::new(10, 0)).unwrap();
    data_writer.get_publication_matched_status().unwrap();

    subscriber.delete_datareader(&data_reader).unwrap();

    wait_set.wait(Duration::new(10, 0)).unwrap();

    assert_eq!(data_writer.get_matched_subscriptions().unwrap().len(), 0);
}

#[test]
fn updated_readers_are_announced_to_writer() {
    let domain_id = TEST_DOMAIN_ID_GENERATOR.generate_unique_domain_id();
    let dp = DomainParticipantFactory::get_instance()
        .create_participant(domain_id, QosKind::Default, NO_LISTENER, NO_STATUS)
        .unwrap();
    let topic = dp
        .create_topic::<UserType>(
            "topic_name",
            "UserType",
            QosKind::Default,
            NO_LISTENER,
            NO_STATUS,
        )
        .unwrap();
    let publisher = dp
        .create_publisher(QosKind::Default, NO_LISTENER, NO_STATUS)
        .unwrap();
    let data_writer = publisher
        .create_datawriter::<UserType>(&topic, QosKind::Default, NO_LISTENER, NO_STATUS)
        .unwrap();
    let subscriber = dp
        .create_subscriber(QosKind::Default, NO_LISTENER, NO_STATUS)
        .unwrap();
    let data_reader = subscriber
        .create_datareader::<UserType>(&topic, QosKind::Default, NO_LISTENER, NO_STATUS)
        .unwrap();
    let cond = data_writer.get_statuscondition();
    cond.set_enabled_statuses(&[StatusKind::PublicationMatched])
        .unwrap();

    let mut wait_set = WaitSet::new();
    wait_set
        .attach_condition(Condition::StatusCondition(cond))
        .unwrap();
    wait_set.wait(Duration::new(5, 0)).unwrap();
    data_writer.get_publication_matched_status().unwrap();

    let user_data_qos_policy = UserDataQosPolicy {
        value: vec![1, 2, 3, 4],
    };
    let qos = DataReaderQos {
        user_data: user_data_qos_policy.clone(),
        ..Default::default()
    };

    data_reader.set_qos(QosKind::Specific(qos)).unwrap();

    wait_set.wait(Duration::new(5, 0)).unwrap();

    let matched_subscriptions = data_writer.get_matched_subscriptions().unwrap();
    let matched_subscription_data = data_writer
        .get_matched_subscription_data(matched_subscriptions[0])
        .unwrap();
    assert_eq!(matched_subscription_data.user_data(), &user_data_qos_policy);
}

#[test]
fn reader_discovers_writer_in_same_participant() {
    let domain_id = TEST_DOMAIN_ID_GENERATOR.generate_unique_domain_id();
    let dp = DomainParticipantFactory::get_instance()
        .create_participant(domain_id, QosKind::Default, NO_LISTENER, NO_STATUS)
        .unwrap();
    let topic = dp
        .create_topic::<UserType>(
            "topic_name",
            "UserType",
            QosKind::Default,
            NO_LISTENER,
            NO_STATUS,
        )
        .unwrap();
    let publisher = dp
        .create_publisher(QosKind::Default, NO_LISTENER, NO_STATUS)
        .unwrap();
    let _data_writer = publisher
        .create_datawriter::<UserType>(&topic, QosKind::Default, NO_LISTENER, NO_STATUS)
        .unwrap();
    let subscriber = dp
        .create_subscriber(QosKind::Default, NO_LISTENER, NO_STATUS)
        .unwrap();
    let data_reader = subscriber
        .create_datareader::<UserType>(&topic, QosKind::Default, NO_LISTENER, NO_STATUS)
        .unwrap();
    let cond = data_reader.get_statuscondition();
    cond.set_enabled_statuses(&[StatusKind::SubscriptionMatched])
        .unwrap();

    let mut wait_set = WaitSet::new();
    wait_set
        .attach_condition(Condition::StatusCondition(cond))
        .unwrap();
    wait_set.wait(Duration::new(10, 0)).unwrap();

    let matched_publications = data_reader.get_matched_publications().unwrap();
    assert_eq!(matched_publications.len(), 1);
    let matched_publication_data = data_reader
        .get_matched_publication_data(matched_publications[0])
        .unwrap();
    assert_eq!(
        &matched_publication_data.participant_key().value,
        dp.get_instance_handle().as_ref()
    )
}

#[test]
fn deleted_writers_are_disposed_from_reader() {
    let domain_id = TEST_DOMAIN_ID_GENERATOR.generate_unique_domain_id();
    let dp = DomainParticipantFactory::get_instance()
        .create_participant(domain_id, QosKind::Default, NO_LISTENER, NO_STATUS)
        .unwrap();
    let topic = dp
        .create_topic::<UserType>(
            "topic_name",
            "UserType",
            QosKind::Default,
            NO_LISTENER,
            NO_STATUS,
        )
        .unwrap();
    let publisher = dp
        .create_publisher(QosKind::Default, NO_LISTENER, NO_STATUS)
        .unwrap();
    let data_writer = publisher
        .create_datawriter::<UserType>(&topic, QosKind::Default, NO_LISTENER, NO_STATUS)
        .unwrap();
    let subscriber = dp
        .create_subscriber(QosKind::Default, NO_LISTENER, NO_STATUS)
        .unwrap();
    let data_reader = subscriber
        .create_datareader::<UserType>(&topic, QosKind::Default, NO_LISTENER, NO_STATUS)
        .unwrap();
    let cond = data_reader.get_statuscondition();
    cond.set_enabled_statuses(&[StatusKind::SubscriptionMatched])
        .unwrap();

    let mut wait_set = WaitSet::new();
    wait_set
        .attach_condition(Condition::StatusCondition(cond))
        .unwrap();
    wait_set.wait(Duration::new(10, 0)).unwrap();
    data_reader.get_subscription_matched_status().unwrap();

    publisher.delete_datawriter(&data_writer).unwrap();

    wait_set.wait(Duration::new(5, 0)).unwrap();

    assert_eq!(data_reader.get_matched_publications().unwrap().len(), 0);
}

#[test]
fn updated_writers_are_announced_to_reader() {
    let domain_id = TEST_DOMAIN_ID_GENERATOR.generate_unique_domain_id();
    let dp = DomainParticipantFactory::get_instance()
        .create_participant(domain_id, QosKind::Default, NO_LISTENER, NO_STATUS)
        .unwrap();
    let topic = dp
        .create_topic::<UserType>(
            "topic_name",
            "UserType",
            QosKind::Default,
            NO_LISTENER,
            NO_STATUS,
        )
        .unwrap();
    let publisher = dp
        .create_publisher(QosKind::Default, NO_LISTENER, NO_STATUS)
        .unwrap();
    let data_writer = publisher
        .create_datawriter::<UserType>(&topic, QosKind::Default, NO_LISTENER, NO_STATUS)
        .unwrap();
    let subscriber = dp
        .create_subscriber(QosKind::Default, NO_LISTENER, NO_STATUS)
        .unwrap();
    let data_reader = subscriber
        .create_datareader::<UserType>(&topic, QosKind::Default, NO_LISTENER, NO_STATUS)
        .unwrap();
    let cond = data_reader.get_statuscondition();
    cond.set_enabled_statuses(&[StatusKind::SubscriptionMatched])
        .unwrap();

    let mut wait_set = WaitSet::new();
    wait_set
        .attach_condition(Condition::StatusCondition(cond))
        .unwrap();
    wait_set.wait(Duration::new(5, 0)).unwrap();
    data_reader.get_subscription_matched_status().unwrap();

    let user_data_qos_policy = UserDataQosPolicy {
        value: vec![1, 2, 3, 4],
    };
    let qos = DataWriterQos {
        user_data: user_data_qos_policy.clone(),
        ..Default::default()
    };
    data_writer.set_qos(QosKind::Specific(qos)).unwrap();

    wait_set.wait(Duration::new(5, 0)).unwrap();

    let matched_publications = data_reader.get_matched_publications().unwrap();
    let matched_publication_data = data_reader
        .get_matched_publication_data(matched_publications[0])
        .unwrap();

    assert_eq!(matched_publication_data.user_data(), &user_data_qos_policy);
}

#[test]
fn two_participants_should_get_subscription_matched() {
    let domain_id = TEST_DOMAIN_ID_GENERATOR.generate_unique_domain_id();
    let domain_participant_factory = DomainParticipantFactory::get_instance();

    let dp1 = domain_participant_factory
        .create_participant(domain_id, QosKind::Default, NO_LISTENER, NO_STATUS)
        .unwrap();
    let topic1 = dp1
        .create_topic::<UserType>(
            "topic_name",
            "UserType",
            QosKind::Default,
            NO_LISTENER,
            NO_STATUS,
        )
        .unwrap();
    let publisher = dp1
        .create_publisher(QosKind::Default, NO_LISTENER, NO_STATUS)
        .unwrap();
    let data_writer = publisher
        .create_datawriter::<UserType>(&topic1, QosKind::Default, NO_LISTENER, NO_STATUS)
        .unwrap();

    let dp2 = domain_participant_factory
        .create_participant(domain_id, QosKind::Default, NO_LISTENER, NO_STATUS)
        .unwrap();
    let topic2 = dp2
        .create_topic::<UserType>(
            "topic_name",
            "UserType",
            QosKind::Default,
            NO_LISTENER,
            NO_STATUS,
        )
        .unwrap();
    let subscriber = dp2
        .create_subscriber(QosKind::Default, NO_LISTENER, NO_STATUS)
        .unwrap();

    let _data_reader = subscriber
        .create_datareader::<UserType>(&topic2, QosKind::Default, NO_LISTENER, NO_STATUS)
        .unwrap();
    let cond = data_writer.get_statuscondition();
    cond.set_enabled_statuses(&[StatusKind::PublicationMatched])
        .unwrap();

    let mut wait_set = WaitSet::new();
    wait_set
        .attach_condition(Condition::StatusCondition(cond))
        .unwrap();
    wait_set.wait(Duration::new(10, 0)).unwrap();

    assert_eq!(data_writer.get_matched_subscriptions().unwrap().len(), 1);
}

#[test]
fn participant_records_discovered_topics() {
    let domain_id = TEST_DOMAIN_ID_GENERATOR.generate_unique_domain_id();
    let domain_participant_factory = DomainParticipantFactory::get_instance();

    let participant1 = domain_participant_factory
        .create_participant(domain_id, QosKind::Default, NO_LISTENER, NO_STATUS)
        .unwrap();
    let participant2 = domain_participant_factory
        .create_participant(domain_id, QosKind::Default, NO_LISTENER, NO_STATUS)
        .unwrap();

    let topic_names = ["Topic 1", "Topic 2"];
    let mut topics = Vec::new();
    for name in topic_names {
        topics.push(
            participant1
                .create_topic::<UserType>(
                    name,
                    "UserType",
                    QosKind::Default,
                    NO_LISTENER,
                    NO_STATUS,
                )
                .unwrap(),
        );
    }

    let mut found_topics = Vec::new();
    for name in topic_names {
        found_topics.push(
            participant2
                .find_topic::<UserType>(name, Duration::new(10, 0))
                .unwrap(),
        );
    }

    let discovered_topic_names: Vec<String> = participant2
        .get_discovered_topics()
        .unwrap()
        .iter()
        .map(|&handle| {
            participant2
                .get_discovered_topic_data(handle)
                .unwrap()
                .name()
                .to_string()
        })
        .collect();

    assert!(discovered_topic_names.contains(&"Topic 1".to_string()));
    assert!(discovered_topic_names.contains(&"Topic 2".to_string()));
}

#[test]
fn reader_discovers_disposed_writer_same_participant() {
    let domain_id = TEST_DOMAIN_ID_GENERATOR.generate_unique_domain_id();
    let dp = DomainParticipantFactory::get_instance()
        .create_participant(domain_id, QosKind::Default, NO_LISTENER, NO_STATUS)
        .unwrap();

    let topic = dp
        .create_topic::<UserType>(
            "topic_name",
            "UserType",
            QosKind::Default,
            NO_LISTENER,
            NO_STATUS,
        )
        .unwrap();
    let publisher = dp
        .create_publisher(QosKind::Default, NO_LISTENER, NO_STATUS)
        .unwrap();
    let data_writer = publisher
        .create_datawriter::<UserType>(&topic, QosKind::Default, NO_LISTENER, NO_STATUS)
        .unwrap();
    let subscriber = dp
        .create_subscriber(QosKind::Default, NO_LISTENER, NO_STATUS)
        .unwrap();
    let data_reader = subscriber
        .create_datareader::<UserType>(&topic, QosKind::Default, NO_LISTENER, NO_STATUS)
        .unwrap();
    let cond = data_reader.get_statuscondition();
    cond.set_enabled_statuses(&[StatusKind::SubscriptionMatched])
        .unwrap();

    let mut wait_set = WaitSet::new();
    wait_set
        .attach_condition(Condition::StatusCondition(cond))
        .unwrap();
    wait_set.wait(Duration::new(10, 0)).unwrap();
    data_reader.get_subscription_matched_status().unwrap();

    publisher.delete_datawriter(&data_writer).unwrap();

    wait_set.wait(Duration::new(5, 0)).unwrap();

    assert_eq!(data_reader.get_matched_publications().unwrap().len(), 0);
}

#[test]
fn publisher_and_subscriber_different_partition_not_matched() {
    let domain_id = TEST_DOMAIN_ID_GENERATOR.generate_unique_domain_id();
    let dp = DomainParticipantFactory::get_instance()
        .create_participant(domain_id, QosKind::Default, NO_LISTENER, NO_STATUS)
        .unwrap();

    let topic = dp
        .create_topic::<UserType>(
            "topic_name",
            "UserType",
            QosKind::Default,
            NO_LISTENER,
            NO_STATUS,
        )
        .unwrap();
    let publisher_qos = PublisherQos {
        partition: PartitionQosPolicy {
            name: vec!["A".to_string()],
        },
        ..Default::default()
    };
    let publisher = dp
        .create_publisher(QosKind::Specific(publisher_qos), NO_LISTENER, NO_STATUS)
        .unwrap();
    let _data_writer = publisher
        .create_datawriter::<UserType>(&topic, QosKind::Default, NO_LISTENER, NO_STATUS)
        .unwrap();

    let subscriber_qos = SubscriberQos {
        partition: PartitionQosPolicy {
            name: vec!["B".to_string()],
        },
        ..Default::default()
    };
    let subscriber = dp
        .create_subscriber(QosKind::Specific(subscriber_qos), NO_LISTENER, NO_STATUS)
        .unwrap();
    let data_reader = subscriber
        .create_datareader::<UserType>(&topic, QosKind::Default, NO_LISTENER, NO_STATUS)
        .unwrap();
    let cond = data_reader.get_statuscondition();
    cond.set_enabled_statuses(&[StatusKind::SubscriptionMatched])
        .unwrap();

    let mut wait_set = WaitSet::new();
    wait_set
        .attach_condition(Condition::StatusCondition(cond))
        .unwrap();

    // Maximum amount of time that it needs to wait to verify absence of matching.
    assert!(wait_set.wait(Duration::new(1, 0)).is_err());
}

#[test]
fn publisher_and_subscriber_regex_partition_is_matched() {
    let domain_id = TEST_DOMAIN_ID_GENERATOR.generate_unique_domain_id();
    let dp = DomainParticipantFactory::get_instance()
        .create_participant(domain_id, QosKind::Default, NO_LISTENER, NO_STATUS)
        .unwrap();
    let topic = dp
        .create_topic::<UserType>(
            "topic_name",
            "UserType",
            QosKind::Default,
            NO_LISTENER,
            NO_STATUS,
        )
        .unwrap();
    let publisher_qos = PublisherQos {
        partition: PartitionQosPolicy {
            name: vec!["ABC".to_string()],
        },
        ..Default::default()
    };
    let publisher = dp
        .create_publisher(QosKind::Specific(publisher_qos), NO_LISTENER, NO_STATUS)
        .unwrap();
    let data_writer = publisher
        .create_datawriter::<UserType>(&topic, QosKind::Default, NO_LISTENER, NO_STATUS)
        .unwrap();
    let cond_data_writer = data_writer.get_statuscondition();
    cond_data_writer
        .set_enabled_statuses(&[StatusKind::PublicationMatched])
        .unwrap();

    let subscriber_qos = SubscriberQos {
        partition: PartitionQosPolicy {
            name: vec!["A[B-C]+".to_string()],
        },
        ..Default::default()
    };
    let subscriber = dp
        .create_subscriber(QosKind::Specific(subscriber_qos), NO_LISTENER, NO_STATUS)
        .unwrap();
    let data_reader = subscriber
        .create_datareader::<UserType>(&topic, QosKind::Default, NO_LISTENER, NO_STATUS)
        .unwrap();
    let cond_data_reader = data_reader.get_statuscondition();
    cond_data_reader
        .set_enabled_statuses(&[StatusKind::SubscriptionMatched])
        .unwrap();

    let mut wait_set_data_reader = WaitSet::new();
    wait_set_data_reader
        .attach_condition(Condition::StatusCondition(cond_data_reader))
        .unwrap();

    let mut wait_set_data_writer = WaitSet::new();
    wait_set_data_writer
        .attach_condition(Condition::StatusCondition(cond_data_writer))
        .unwrap();

    assert!(wait_set_data_reader.wait(Duration::new(10, 0)).is_ok());
    assert!(wait_set_data_writer.wait(Duration::new(10, 0)).is_ok());
}

#[test]
fn publisher_regex_and_subscriber_partition_is_matched() {
    let domain_id = TEST_DOMAIN_ID_GENERATOR.generate_unique_domain_id();
    let dp = DomainParticipantFactory::get_instance()
        .create_participant(domain_id, QosKind::Default, NO_LISTENER, NO_STATUS)
        .unwrap();
    let topic = dp
        .create_topic::<UserType>(
            "topic_name",
            "UserType",
            QosKind::Default,
            NO_LISTENER,
            NO_STATUS,
        )
        .unwrap();
    let publisher_qos = PublisherQos {
        partition: PartitionQosPolicy {
            name: vec!["A[1-2]+".to_string()],
        },
        ..Default::default()
    };
    let publisher = dp
        .create_publisher(QosKind::Specific(publisher_qos), NO_LISTENER, NO_STATUS)
        .unwrap();
    let data_writer = publisher
        .create_datawriter::<UserType>(&topic, QosKind::Default, NO_LISTENER, NO_STATUS)
        .unwrap();
    let cond_data_writer = data_writer.get_statuscondition();
    cond_data_writer
        .set_enabled_statuses(&[StatusKind::PublicationMatched])
        .unwrap();

    let subscriber_qos = SubscriberQos {
        partition: PartitionQosPolicy {
            name: vec!["A12".to_string()],
        },
        ..Default::default()
    };
    let subscriber = dp
        .create_subscriber(QosKind::Specific(subscriber_qos), NO_LISTENER, NO_STATUS)
        .unwrap();
    let data_reader = subscriber
        .create_datareader::<UserType>(&topic, QosKind::Default, NO_LISTENER, NO_STATUS)
        .unwrap();
    let cond_data_reader = data_reader.get_statuscondition();
    cond_data_reader
        .set_enabled_statuses(&[StatusKind::SubscriptionMatched])
        .unwrap();

    let mut wait_set_data_reader = WaitSet::new();
    wait_set_data_reader
        .attach_condition(Condition::StatusCondition(cond_data_reader))
        .unwrap();

    let mut wait_set_data_writer = WaitSet::new();
    wait_set_data_writer
        .attach_condition(Condition::StatusCondition(cond_data_writer))
        .unwrap();

    assert!(wait_set_data_reader.wait(Duration::new(10, 0)).is_ok());
    assert!(wait_set_data_writer.wait(Duration::new(10, 0)).is_ok());
}

#[test]
fn publisher_regex_and_subscriber_regex_partition_is_matched() {
    let domain_id = TEST_DOMAIN_ID_GENERATOR.generate_unique_domain_id();
    let dp = DomainParticipantFactory::get_instance()
        .create_participant(domain_id, QosKind::Default, NO_LISTENER, NO_STATUS)
        .unwrap();
    let topic = dp
        .create_topic::<UserType>(
            "topic_name",
            "UserType",
            QosKind::Default,
            NO_LISTENER,
            NO_STATUS,
        )
        .unwrap();
    let publisher_qos = PublisherQos {
        partition: PartitionQosPolicy {
            name: vec!["A[1-2]+".to_string()],
        },
        ..Default::default()
    };
    let publisher = dp
        .create_publisher(QosKind::Specific(publisher_qos), NO_LISTENER, NO_STATUS)
        .unwrap();
    let data_writer = publisher
        .create_datawriter::<UserType>(&topic, QosKind::Default, NO_LISTENER, NO_STATUS)
        .unwrap();
    let cond_data_writer = data_writer.get_statuscondition();
    cond_data_writer
        .set_enabled_statuses(&[StatusKind::PublicationMatched])
        .unwrap();

    let subscriber_qos = SubscriberQos {
        partition: PartitionQosPolicy {
            name: vec!["A[1-2]+".to_string()],
        },
        ..Default::default()
    };
    let subscriber = dp
        .create_subscriber(QosKind::Specific(subscriber_qos), NO_LISTENER, NO_STATUS)
        .unwrap();
    let data_reader = subscriber
        .create_datareader::<UserType>(&topic, QosKind::Default, NO_LISTENER, NO_STATUS)
        .unwrap();
    let cond_data_reader = data_reader.get_statuscondition();
    cond_data_reader
        .set_enabled_statuses(&[StatusKind::SubscriptionMatched])
        .unwrap();

    let mut wait_set_data_reader = WaitSet::new();
    wait_set_data_reader
        .attach_condition(Condition::StatusCondition(cond_data_reader))
        .unwrap();

    let mut wait_set_data_writer = WaitSet::new();
    wait_set_data_writer
        .attach_condition(Condition::StatusCondition(cond_data_writer))
        .unwrap();

    assert!(wait_set_data_reader.wait(Duration::new(5, 0)).is_ok());
    assert!(wait_set_data_writer.wait(Duration::new(5, 0)).is_ok());
}

#[test]
fn writer_matched_to_already_existing_reader_with_matched_writer() {
    let domain_id = TEST_DOMAIN_ID_GENERATOR.generate_unique_domain_id();
    let dp = DomainParticipantFactory::get_instance()
        .create_participant(domain_id, QosKind::Default, NO_LISTENER, NO_STATUS)
        .unwrap();
    let topic = dp
        .create_topic::<UserType>(
            "topic_name",
            "UserType",
            QosKind::Default,
            NO_LISTENER,
            NO_STATUS,
        )
        .unwrap();
    let publisher = dp
        .create_publisher(QosKind::Default, NO_LISTENER, NO_STATUS)
        .unwrap();
    let data_writer = publisher
        .create_datawriter::<UserType>(&topic, QosKind::Default, NO_LISTENER, NO_STATUS)
        .unwrap();
    let subscriber = dp
        .create_subscriber(QosKind::Default, NO_LISTENER, NO_STATUS)
        .unwrap();
    let _data_reader = subscriber
        .create_datareader::<UserType>(&topic, QosKind::Default, NO_LISTENER, NO_STATUS)
        .unwrap();
    let cond = data_writer.get_statuscondition();
    cond.set_enabled_statuses(&[StatusKind::PublicationMatched])
        .unwrap();

    let mut wait_set = WaitSet::new();
    wait_set
        .attach_condition(Condition::StatusCondition(cond))
        .unwrap();
    wait_set.wait(Duration::new(10, 0)).unwrap();

    let data_writer2 = publisher
        .create_datawriter::<UserType>(&topic, QosKind::Default, NO_LISTENER, NO_STATUS)
        .unwrap();
    let dw2_cond = data_writer2.get_statuscondition();
    dw2_cond
        .set_enabled_statuses(&[StatusKind::PublicationMatched])
        .unwrap();
    let mut wait_set2 = WaitSet::new();
    wait_set2
        .attach_condition(Condition::StatusCondition(dw2_cond))
        .unwrap();

    assert!(wait_set2.wait(Duration::new(5, 0)).is_ok());
}

#[test]
fn reader_matched_to_already_existing_writer_with_matched_reader() {
    let domain_id = TEST_DOMAIN_ID_GENERATOR.generate_unique_domain_id();
    let dp = DomainParticipantFactory::get_instance()
        .create_participant(domain_id, QosKind::Default, NO_LISTENER, NO_STATUS)
        .unwrap();

    let topic = dp
        .create_topic::<UserType>(
            "topic_name",
            "UserType",
            QosKind::Default,
            NO_LISTENER,
            NO_STATUS,
        )
        .unwrap();
    let publisher = dp
        .create_publisher(QosKind::Default, NO_LISTENER, NO_STATUS)
        .unwrap();
    let data_writer = publisher
        .create_datawriter::<UserType>(&topic, QosKind::Default, NO_LISTENER, NO_STATUS)
        .unwrap();
    let subscriber = dp
        .create_subscriber(QosKind::Default, NO_LISTENER, NO_STATUS)
        .unwrap();
    let _data_reader = subscriber
        .create_datareader::<UserType>(&topic, QosKind::Default, NO_LISTENER, NO_STATUS)
        .unwrap();
    let cond = data_writer.get_statuscondition();
    cond.set_enabled_statuses(&[StatusKind::PublicationMatched])
        .unwrap();

    let mut wait_set = WaitSet::new();
    wait_set
        .attach_condition(Condition::StatusCondition(cond))
        .unwrap();
    wait_set.wait(Duration::new(10, 0)).unwrap();

    let data_reader2 = subscriber
        .create_datareader::<UserType>(&topic, QosKind::Default, NO_LISTENER, NO_STATUS)
        .unwrap();
    let dr2_cond = data_reader2.get_statuscondition();
    dr2_cond
        .set_enabled_statuses(&[StatusKind::SubscriptionMatched])
        .unwrap();
    let mut wait_set2 = WaitSet::new();
    wait_set2
        .attach_condition(Condition::StatusCondition(dr2_cond))
        .unwrap();

    assert!(wait_set2.wait(Duration::new(5, 0)).is_ok());
}

#[test]
fn discovered_participant_removed_after_deletion() {
    let domain_id = TEST_DOMAIN_ID_GENERATOR.generate_unique_domain_id();
    let domain_participant_factory = DomainParticipantFactory::get_instance();

    let participant1 = domain_participant_factory
        .create_participant(domain_id, QosKind::Default, NO_LISTENER, NO_STATUS)
        .unwrap();

    let participant2 = domain_participant_factory
        .create_participant(domain_id, QosKind::Default, NO_LISTENER, NO_STATUS)
        .unwrap();
    let start_time = Instant::now();
    loop {
        if participant1.get_discovered_participants().unwrap().len() == 2 {
            break;
        }
        if start_time.elapsed() > std::time::Duration::from_secs(10) {
            panic!("Participant not discovered before timeout")
        }
    }

    domain_participant_factory
        .delete_participant(&participant2)
        .unwrap();

    let start_time = Instant::now();
    while start_time.elapsed() <= std::time::Duration::from_secs(10) {
        if participant1.get_discovered_participants().unwrap().len() == 1 {
            break;
        }
    }
    assert_eq!(participant1.get_discovered_participants().unwrap().len(), 1)
}

#[test]
fn writer_offering_xcdr1_should_not_match_reader_requesting_xcdr2() {
    let domain_id = TEST_DOMAIN_ID_GENERATOR.generate_unique_domain_id();
    let dp = DomainParticipantFactory::get_instance()
        .create_participant(domain_id, QosKind::Default, NO_LISTENER, NO_STATUS)
        .unwrap();

    let topic = dp
        .create_topic::<UserType>(
            "topic_name",
            "UserType",
            QosKind::Default,
            NO_LISTENER,
            NO_STATUS,
        )
        .unwrap();
    let publisher = dp
        .create_publisher(QosKind::Default, NO_LISTENER, NO_STATUS)
        .unwrap();
    let writer_qos = DataWriterQos {
        representation: DataRepresentationQosPolicy {
            value: vec![XCDR_DATA_REPRESENTATION],
        },
        ..Default::default()
    };
    let data_writer = publisher
        .create_datawriter::<UserType>(
            &topic,
            QosKind::Specific(writer_qos),
            NO_LISTENER,
            NO_STATUS,
        )
        .unwrap();
    let subscriber = dp
        .create_subscriber(QosKind::Default, NO_LISTENER, NO_STATUS)
        .unwrap();
    let reader_qos = DataReaderQos {
        representation: DataRepresentationQosPolicy {
            value: vec![XCDR2_DATA_REPRESENTATION],
        },
        ..Default::default()
    };
    let _data_reader = subscriber
        .create_datareader::<UserType>(
            &topic,
            QosKind::Specific(reader_qos),
            NO_LISTENER,
            NO_STATUS,
        )
        .unwrap();
    let cond = data_writer.get_statuscondition();
    cond.set_enabled_statuses(&[StatusKind::OfferedIncompatibleQos])
        .unwrap();

    let mut wait_set = WaitSet::new();
    wait_set
        .attach_condition(Condition::StatusCondition(cond))
        .unwrap();
    wait_set.wait(Duration::new(10, 0)).unwrap();
}

#[test]
fn reader_requesting_xcdr2_should_not_match_writer_offering_xcdr1() {
    let domain_id = TEST_DOMAIN_ID_GENERATOR.generate_unique_domain_id();
    let dp = DomainParticipantFactory::get_instance()
        .create_participant(domain_id, QosKind::Default, NO_LISTENER, NO_STATUS)
        .unwrap();

    let topic = dp
        .create_topic::<UserType>(
            "topic_name",
            "UserType",
            QosKind::Default,
            NO_LISTENER,
            NO_STATUS,
        )
        .unwrap();
    let publisher = dp
        .create_publisher(QosKind::Default, NO_LISTENER, NO_STATUS)
        .unwrap();
    let writer_qos = DataWriterQos {
        representation: DataRepresentationQosPolicy {
            value: vec![XCDR_DATA_REPRESENTATION],
        },
        ..Default::default()
    };
    let _data_writer = publisher
        .create_datawriter::<UserType>(
            &topic,
            QosKind::Specific(writer_qos),
            NO_LISTENER,
            NO_STATUS,
        )
        .unwrap();
    let subscriber = dp
        .create_subscriber(QosKind::Default, NO_LISTENER, NO_STATUS)
        .unwrap();
    let reader_qos = DataReaderQos {
        representation: DataRepresentationQosPolicy {
            value: vec![XCDR2_DATA_REPRESENTATION],
        },
        ..Default::default()
    };
    let data_reader = subscriber
        .create_datareader::<UserType>(
            &topic,
            QosKind::Specific(reader_qos),
            NO_LISTENER,
            NO_STATUS,
        )
        .unwrap();
    let cond = data_reader.get_statuscondition();
    cond.set_enabled_statuses(&[StatusKind::RequestedIncompatibleQos])
        .unwrap();

    let mut wait_set = WaitSet::new();
    wait_set
        .attach_condition(Condition::StatusCondition(cond))
        .unwrap();
    wait_set.wait(Duration::new(10, 0)).unwrap();
}

#[test]
fn writer_offering_exclusive_ownership_should_not_match_reader_requesting_shared_ownership() {
    let domain_id = TEST_DOMAIN_ID_GENERATOR.generate_unique_domain_id();
    let dp = DomainParticipantFactory::get_instance()
        .create_participant(domain_id, QosKind::Default, NO_LISTENER, NO_STATUS)
        .unwrap();

    let topic = dp
        .create_topic::<UserType>(
            "topic_name",
            "UserType",
            QosKind::Default,
            NO_LISTENER,
            NO_STATUS,
        )
        .unwrap();
    let publisher = dp
        .create_publisher(QosKind::Default, NO_LISTENER, NO_STATUS)
        .unwrap();
    let writer_qos = DataWriterQos {
        ownership: OwnershipQosPolicy {
            kind: OwnershipQosPolicyKind::Exclusive,
        },
        ..Default::default()
    };
    let data_writer = publisher
        .create_datawriter::<UserType>(
            &topic,
            QosKind::Specific(writer_qos),
            NO_LISTENER,
            NO_STATUS,
        )
        .unwrap();
    let subscriber = dp
        .create_subscriber(QosKind::Default, NO_LISTENER, NO_STATUS)
        .unwrap();
    let reader_qos = DataReaderQos {
        ownership: OwnershipQosPolicy {
            kind: OwnershipQosPolicyKind::Shared,
        },
        ..Default::default()
    };
    let _data_reader = subscriber
        .create_datareader::<UserType>(
            &topic,
            QosKind::Specific(reader_qos),
            NO_LISTENER,
            NO_STATUS,
        )
        .unwrap();
    let cond = data_writer.get_statuscondition();
    cond.set_enabled_statuses(&[StatusKind::OfferedIncompatibleQos])
        .unwrap();

    let mut wait_set = WaitSet::new();
    wait_set
        .attach_condition(Condition::StatusCondition(cond))
        .unwrap();
    wait_set.wait(Duration::new(10, 0)).unwrap();
}

#[test]
fn reader_requesting_exclusive_ownership_should_not_match_writer_offering_shared_ownership() {
    let domain_id = TEST_DOMAIN_ID_GENERATOR.generate_unique_domain_id();
    let dp = DomainParticipantFactory::get_instance()
        .create_participant(domain_id, QosKind::Default, NO_LISTENER, NO_STATUS)
        .unwrap();

    let topic = dp
        .create_topic::<UserType>(
            "topic_name",
            "UserType",
            QosKind::Default,
            NO_LISTENER,
            NO_STATUS,
        )
        .unwrap();
    let publisher = dp
        .create_publisher(QosKind::Default, NO_LISTENER, NO_STATUS)
        .unwrap();
    let writer_qos = DataWriterQos {
        ownership: OwnershipQosPolicy {
            kind: OwnershipQosPolicyKind::Shared,
        },
        ..Default::default()
    };
    let _data_writer = publisher
        .create_datawriter::<UserType>(
            &topic,
            QosKind::Specific(writer_qos),
            NO_LISTENER,
            NO_STATUS,
        )
        .unwrap();
    let subscriber = dp
        .create_subscriber(QosKind::Default, NO_LISTENER, NO_STATUS)
        .unwrap();
    let reader_qos = DataReaderQos {
        ownership: OwnershipQosPolicy {
            kind: OwnershipQosPolicyKind::Exclusive,
        },
        ..Default::default()
    };
    let data_reader = subscriber
        .create_datareader::<UserType>(
            &topic,
            QosKind::Specific(reader_qos),
            NO_LISTENER,
            NO_STATUS,
        )
        .unwrap();
    let cond = data_reader.get_statuscondition();
    cond.set_enabled_statuses(&[StatusKind::RequestedIncompatibleQos])
        .unwrap();

    let mut wait_set = WaitSet::new();
    wait_set
        .attach_condition(Condition::StatusCondition(cond))
        .unwrap();
    wait_set.wait(Duration::new(10, 0)).unwrap();
}

#[test]
fn two_participants_stay_discovered() {
    let domain_id = TEST_DOMAIN_ID_GENERATOR.generate_unique_domain_id();
    let domain_participant_factory = DomainParticipantFactory::get_instance();

    let participant1 = domain_participant_factory
        .create_participant(domain_id, QosKind::Default, NO_LISTENER, NO_STATUS)
        .unwrap();

    let participant2 = domain_participant_factory
        .create_participant(domain_id, QosKind::Default, NO_LISTENER, NO_STATUS)
        .unwrap();

    let start = Instant::now();
    loop {
        if participant1.get_discovered_participants().unwrap().len() == 2
            && participant2.get_discovered_participants().unwrap().len() == 2
        {
            break;
        }
        if start.elapsed() > std::time::Duration::from_secs(10) {
            panic!("Participants not discovered within timeout");
        }
        std::thread::sleep(std::time::Duration::from_millis(50));
    }

    assert_eq!(participant1.get_discovered_participants().unwrap().len(), 2);
    assert_eq!(participant2.get_discovered_participants().unwrap().len(), 2);

    let discovered = participant1.get_discovered_participants().unwrap();
    let remote_handle = discovered
        .into_iter()
        .find(|h| h.as_ref() == participant2.get_instance_handle().as_ref())
        .expect("Participant 2 must be in discovered list");
    let participant_data = participant1
        .get_discovered_participant_data(remote_handle)
        .unwrap();
    assert_eq!(
        participant_data.key().value,
        *participant2.get_instance_handle().as_ref()
    );
}

#[test]
#[ignore]
fn participant_removed_after_lease_duration() {
    let domain_id = TEST_DOMAIN_ID_GENERATOR.generate_unique_domain_id();
    let domain_participant_factory = DomainParticipantFactory::get_instance();

    let participant1 = domain_participant_factory
        .create_participant(domain_id, QosKind::Default, NO_LISTENER, NO_STATUS)
        .unwrap();

    let participant2 = domain_participant_factory
        .create_participant(domain_id, QosKind::Default, NO_LISTENER, NO_STATUS)
        .unwrap();

    std::thread::sleep(std::time::Duration::from_secs(5));

    domain_participant_factory
        .delete_participant(&participant2)
        .unwrap();

    std::thread::sleep(std::time::Duration::from_secs(5));

    let discovered_participant = participant1.get_discovered_participants().unwrap();

    assert_eq!(discovered_participant.len(), 1);
}
