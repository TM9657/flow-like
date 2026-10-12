use std::time::Instant;

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

#[derive(DdsType, Debug, Clone)]
pub struct PingPongType(Vec<u8>);

fn main() {
    let size = 1000;
    let number_loops = 100;
    let warmup = std::time::Duration::from_secs(1);

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
            &pong_topic,
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
            &ping_topic,
            QosKind::Specific(writer_qos),
            NO_LISTENER,
            NO_STATUS,
        )
        .unwrap();

    let writer_status_condition = writer.get_statuscondition();
    writer_status_condition
        .set_enabled_statuses(&[StatusKind::PublicationMatched])
        .unwrap();
    let mut wait_set = WaitSet::new();
    wait_set
        .attach_condition(Condition::StatusCondition(writer_status_condition))
        .unwrap();

    wait_set.wait(Duration::new(60, 0)).unwrap();

    let mut reader_wait_set = WaitSet::new();
    let reader_status_condition = reader.get_statuscondition();
    reader_status_condition
        .set_enabled_statuses(&[StatusKind::DataAvailable])
        .unwrap();
    reader_wait_set
        .attach_condition(Condition::StatusCondition(reader_status_condition))
        .unwrap();

    let mut samples = Vec::with_capacity(number_loops);

    let data = PingPongType((0usize..size).map(|i| (i % 10) as u8).collect::<Vec<u8>>());

    println!("Warming up for {warmup:?}...");
    let now = Instant::now();
    while now.elapsed() < warmup {
        writer.write(data.clone(), None).unwrap();

        reader_wait_set.wait(Duration::new(i32::MAX, 0)).unwrap();
        let _ = reader.take_next_sample().unwrap();
    }

    for _ in 0..number_loops {
        let new_data = data.clone();
        let write_time = Instant::now();
        writer.write(new_data, None).unwrap();

        reader_wait_set.wait(Duration::new(i32::MAX, 0)).unwrap();
        let _ = reader.take_next_sample().unwrap();
        let ts = write_time.elapsed().as_micros();
        samples.push(ts);
    }

    for (i, rtt) in samples.iter().enumerate().take(number_loops) {
        println!(
            "{} bytes: seq={} rtt={:?}µs lat={:?}µs",
            size,
            i,
            rtt,
            rtt / 2
        );
    }
}
