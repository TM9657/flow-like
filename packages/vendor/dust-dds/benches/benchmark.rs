use criterion::{Criterion, criterion_group, criterion_main};
use dust_dds::{
    dds_async::data_reader::DataReaderAsync,
    domain::domain_participant_factory::DomainParticipantFactory,
    infrastructure::{
        listener::NO_LISTENER,
        qos::{DataWriterQos, QosKind},
        qos_policy::{ReliabilityQosPolicy, ReliabilityQosPolicyKind},
        sample_info::{ANY_INSTANCE_STATE, ANY_SAMPLE_STATE, ANY_VIEW_STATE},
        status::{NO_STATUS, StatusKind},
        time::{Duration, DurationKind},
        type_support::DdsType,
    },
    subscription::data_reader_listener::DataReaderListener,
    wait_set::{Condition, WaitSet},
    xtypes::bytes::ByteBuf,
};

#[derive(Clone, Debug, PartialEq, DdsType)]
struct KeyedData {
    #[dust_dds(key)]
    id: u8,
    value: u8,
}

pub fn best_effort_write_only(c: &mut Criterion) {
    let domain_id = 200;
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
    let publisher = participant
        .create_publisher(QosKind::Default, NO_LISTENER, NO_STATUS)
        .unwrap();
    let writer_qos = DataWriterQos {
        reliability: ReliabilityQosPolicy {
            kind: ReliabilityQosPolicyKind::BestEffort,
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
    let _reader = subscriber
        .create_datareader::<KeyedData>(&topic, QosKind::Default, NO_LISTENER, NO_STATUS)
        .unwrap();

    let cond = writer.get_statuscondition();
    cond.set_enabled_statuses(&[StatusKind::PublicationMatched])
        .unwrap();

    let mut wait_set = WaitSet::new();
    wait_set
        .attach_condition(Condition::StatusCondition(cond))
        .unwrap();
    wait_set.wait(Duration::new(10, 0)).unwrap();

    c.bench_function("best_effort_write_only", |b| {
        b.iter(|| {
            writer.write(KeyedData { id: 1, value: 1 }, None).unwrap();
        })
    });
}

pub fn best_effort_read_only(c: &mut Criterion) {
    let domain_id = 201;
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
    let publisher = participant
        .create_publisher(QosKind::Default, NO_LISTENER, NO_STATUS)
        .unwrap();
    let writer = publisher
        .create_datawriter(&topic, QosKind::Default, NO_LISTENER, NO_STATUS)
        .unwrap();
    let subscriber = participant
        .create_subscriber(QosKind::Default, NO_LISTENER, NO_STATUS)
        .unwrap();
    let reader = subscriber
        .create_datareader::<KeyedData>(&topic, QosKind::Default, NO_LISTENER, NO_STATUS)
        .unwrap();

    let cond = writer.get_statuscondition();
    cond.set_enabled_statuses(&[StatusKind::PublicationMatched])
        .unwrap();

    let mut wait_set = WaitSet::new();
    wait_set
        .attach_condition(Condition::StatusCondition(cond))
        .unwrap();
    wait_set.wait(Duration::new(10, 0)).unwrap();

    writer.write(KeyedData { id: 1, value: 1 }, None).unwrap();

    c.bench_function("best_effort_read_only", |b| {
        b.iter(|| {
            reader
                .read(1, ANY_SAMPLE_STATE, ANY_VIEW_STATE, ANY_INSTANCE_STATE)
                .ok();
        })
    });
}

fn best_effort_write_and_receive(c: &mut Criterion) {
    struct Listener {
        sender: std::sync::mpsc::SyncSender<()>,
    }
    impl DataReaderListener<KeyedData> for Listener {
        async fn on_data_available(&mut self, the_reader: DataReaderAsync<KeyedData>) {
            the_reader
                .read(1, ANY_SAMPLE_STATE, ANY_VIEW_STATE, ANY_INSTANCE_STATE)
                .await
                .ok();
            self.sender.send(()).unwrap();
        }
    }

    let domain_id = 202;
    let participant_factory = DomainParticipantFactory::get_instance();
    let participant = participant_factory
        .create_participant(domain_id, QosKind::Default, NO_LISTENER, NO_STATUS)
        .unwrap();
    let topic = participant
        .create_topic::<KeyedData>(
            "TestTopic",
            "KeyedData",
            QosKind::Default,
            NO_LISTENER,
            NO_STATUS,
        )
        .unwrap();
    let subscriber = participant
        .create_subscriber(QosKind::Default, NO_LISTENER, NO_STATUS)
        .unwrap();

    let (sender, receiver) = std::sync::mpsc::sync_channel(1);

    let listener = Listener { sender };
    let reader = subscriber
        .create_datareader(
            &topic,
            QosKind::Default,
            Some(listener),
            &[StatusKind::DataAvailable],
        )
        .unwrap();
    let reader_cond = reader.get_statuscondition();
    let publisher = participant
        .create_publisher(QosKind::Default, NO_LISTENER, NO_STATUS)
        .unwrap();
    let writer = publisher
        .create_datawriter(&topic, QosKind::Default, NO_LISTENER, NO_STATUS)
        .unwrap();
    let writer_cond = writer.get_statuscondition();
    writer_cond
        .set_enabled_statuses(&[StatusKind::PublicationMatched])
        .unwrap();

    let mut wait_set = WaitSet::new();
    reader_cond
        .set_enabled_statuses(&[StatusKind::SubscriptionMatched])
        .unwrap();
    wait_set
        .attach_condition(Condition::StatusCondition(reader_cond))
        .unwrap();
    wait_set.wait(Duration::new(20, 0)).unwrap();

    let mut wait_set2 = WaitSet::new();
    wait_set2
        .attach_condition(Condition::StatusCondition(writer_cond))
        .unwrap();
    wait_set2.wait(Duration::new(20, 0)).unwrap();

    c.bench_function("best_effort_write_and_receive", |b| {
        b.iter(|| {
            writer.write(KeyedData { id: 1, value: 7 }, None).unwrap();
            receiver
                .recv_timeout(std::time::Duration::from_secs(10))
                .unwrap();
        })
    });
}

#[derive(Clone, Debug, PartialEq, DdsType)]
struct LargeKeyedData {
    #[dust_dds(key)]
    id: u8,
    value: ByteBuf,
}

fn best_effort_write_and_receive_frag(c: &mut Criterion) {
    struct Listener {
        sender: std::sync::mpsc::SyncSender<()>,
    }
    impl DataReaderListener<LargeKeyedData> for Listener {
        async fn on_data_available(&mut self, the_reader: DataReaderAsync<LargeKeyedData>) {
            the_reader
                .read(1, ANY_SAMPLE_STATE, ANY_VIEW_STATE, ANY_INSTANCE_STATE)
                .await
                .ok();
            self.sender.send(()).unwrap();
        }
    }

    let domain_id = 203;
    let participant_factory = DomainParticipantFactory::get_instance();
    let participant = participant_factory
        .create_participant(domain_id, QosKind::Default, NO_LISTENER, NO_STATUS)
        .unwrap();
    let topic = participant
        .create_topic::<LargeKeyedData>(
            "TestTopic",
            "LargeKeyedData",
            QosKind::Default,
            NO_LISTENER,
            NO_STATUS,
        )
        .unwrap();
    let subscriber = participant
        .create_subscriber(QosKind::Default, NO_LISTENER, NO_STATUS)
        .unwrap();

    let (sender, receiver) = std::sync::mpsc::sync_channel(1);

    let listener = Listener { sender };
    let reader = subscriber
        .create_datareader(
            &topic,
            QosKind::Default,
            Some(listener),
            &[StatusKind::DataAvailable],
        )
        .unwrap();
    let reader_cond = reader.get_statuscondition();
    let publisher = participant
        .create_publisher(QosKind::Default, NO_LISTENER, NO_STATUS)
        .unwrap();
    let writer = publisher
        .create_datawriter(&topic, QosKind::Default, NO_LISTENER, NO_STATUS)
        .unwrap();
    let writer_cond = writer.get_statuscondition();
    writer_cond
        .set_enabled_statuses(&[StatusKind::PublicationMatched])
        .unwrap();

    let mut wait_set = WaitSet::new();
    reader_cond
        .set_enabled_statuses(&[StatusKind::SubscriptionMatched])
        .unwrap();
    wait_set
        .attach_condition(Condition::StatusCondition(reader_cond))
        .unwrap();
    wait_set.wait(Duration::new(20, 0)).unwrap();

    let mut wait_set2 = WaitSet::new();
    wait_set2
        .attach_condition(Condition::StatusCondition(writer_cond))
        .unwrap();
    wait_set2.wait(Duration::new(20, 0)).unwrap();

    c.bench_function("best_effort_write_and_receive_frag", |b| {
        b.iter(|| {
            writer
                .write(
                    LargeKeyedData {
                        id: 1,
                        value: ByteBuf(vec![7; 32000]),
                    },
                    None,
                )
                .unwrap();
            receiver
                .recv_timeout(std::time::Duration::from_secs(10))
                .unwrap();
        })
    });
}

criterion_group!(
    benches,
    best_effort_write_only,
    best_effort_read_only,
    best_effort_write_and_receive,
    best_effort_write_and_receive_frag
);
criterion_main!(benches);
