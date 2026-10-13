use dust_dds::{
    domain::domain_participant_factory::DomainParticipantFactory,
    infrastructure::{
        listener::NO_LISTENER,
        qos::{DataReaderQos, DataWriterQos, QosKind},
        qos_policy::{ReliabilityQosPolicy, ReliabilityQosPolicyKind},
        status::{NO_STATUS, StatusKind},
        time::{Duration, DurationKind},
    },
    wait_set::{Condition, WaitSet},
};
use dust_dds_derive::DdsType;

#[derive(DdsType)]
pub struct PingPongType(Vec<u8>);

fn main() {
    let domain_id = 1;
    let participant_factory = DomainParticipantFactory::get_instance();

    let participant = participant_factory
        .create_participant(domain_id, QosKind::Default, NO_LISTENER, NO_STATUS)
        .unwrap();

    let ping_topic = participant
        .create_topic::<PingPongType>(
            "PingTopic",
            "PingPongType",
            QosKind::Default,
            NO_LISTENER,
            NO_STATUS,
        )
        .unwrap();

    let pong_topic = participant
        .create_topic::<PingPongType>(
            "PongTopic",
            "PingPongType",
            QosKind::Default,
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
            max_blocking_time: DurationKind::Infinite,
        },
        ..Default::default()
    };
    let reader = subscriber
        .create_datareader::<PingPongType>(
            &ping_topic,
            QosKind::Specific(reader_qos),
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
            max_blocking_time: DurationKind::Infinite,
        },
        ..Default::default()
    };
    let writer = publisher
        .create_datawriter::<PingPongType>(
            &pong_topic,
            QosKind::Specific(writer_qos),
            NO_LISTENER,
            NO_STATUS,
        )
        .unwrap();

    let mut reader_wait_set = WaitSet::new();
    let reader_status_condition = reader.get_statuscondition();
    reader_status_condition
        .set_enabled_statuses(&[StatusKind::DataAvailable])
        .unwrap();
    reader_wait_set
        .attach_condition(Condition::StatusCondition(reader_status_condition))
        .unwrap();
    loop {
        reader_wait_set.wait(Duration::new(i32::MAX, 0)).unwrap();
        let sample = reader.take_next_sample().unwrap();

        writer.write(sample.data.unwrap(), None).unwrap();
    }
}
