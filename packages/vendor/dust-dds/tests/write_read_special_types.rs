mod utils;
use crate::utils::domain_id_generator::TEST_DOMAIN_ID_GENERATOR;
use dust_dds::{
    domain::domain_participant_factory::DomainParticipantFactory,
    infrastructure::{
        listener::NO_LISTENER,
        qos::{DataReaderQos, DataWriterQos, QosKind},
        qos_policy::{ReliabilityQosPolicy, ReliabilityQosPolicyKind},
        sample_info::{ANY_INSTANCE_STATE, ANY_SAMPLE_STATE, ANY_VIEW_STATE, InstanceStateKind},
        status::{NO_STATUS, StatusKind},
        time::{Duration, DurationKind},
        type_support::DdsType,
    },
    wait_set::{Condition, WaitSet},
    xtypes::dynamic_type::{
        DynamicData, DynamicDataFactory, DynamicTypeBuilderFactory, MemberDescriptor,
        TypeDescriptor,
    },
};

#[test]
fn foo_with_non_consecutive_key_should_read_and_write() {
    #[derive(Clone, Debug, PartialEq, DdsType)]
    struct NonConsecutiveKey {
        #[dust_dds(key)]
        id: u32,
        value: Vec<u8>,
        #[dust_dds(key)]
        another_id: u64,
    }

    let domain_id = TEST_DOMAIN_ID_GENERATOR.generate_unique_domain_id();

    let participant = DomainParticipantFactory::get_instance()
        .create_participant(domain_id, QosKind::Default, NO_LISTENER, NO_STATUS)
        .unwrap();

    let topic = participant
        .create_topic::<NonConsecutiveKey>(
            "MyTopic",
            "NonConsecutiveKey",
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
    let reader_qos = DataReaderQos {
        reliability: ReliabilityQosPolicy {
            kind: ReliabilityQosPolicyKind::Reliable,
            max_blocking_time: DurationKind::Finite(Duration::new(1, 0)),
        },
        ..Default::default()
    };
    let reader = subscriber
        .create_datareader::<NonConsecutiveKey>(
            &topic,
            QosKind::Specific(reader_qos),
            NO_LISTENER,
            NO_STATUS,
        )
        .unwrap();

    let cond = writer.get_statuscondition();
    cond.set_enabled_statuses(&[StatusKind::PublicationMatched])
        .unwrap();

    let mut wait_set = WaitSet::new();
    wait_set
        .attach_condition(Condition::StatusCondition(cond))
        .unwrap();
    wait_set.wait(Duration::new(10, 0)).unwrap();

    let data = NonConsecutiveKey {
        id: 1,
        value: vec![1, 2, 3, 4],
        another_id: 20,
    };

    writer.write(data.clone(), None).unwrap();

    writer
        .wait_for_acknowledgments(Duration::new(10, 0))
        .unwrap();

    let samples = reader
        .take(3, ANY_SAMPLE_STATE, ANY_VIEW_STATE, ANY_INSTANCE_STATE)
        .unwrap();

    assert_eq!(samples.len(), 1);
    assert_eq!(samples[0].data.as_ref().unwrap(), &data);
}

#[test]
fn foo_enumerator_should_read_and_write() {
    #[derive(Clone, Debug, PartialEq, DdsType)]
    enum MyEnum {
        VariantA = 5,
        VariantB = 6,
        VariantC,
    }

    let domain_id = TEST_DOMAIN_ID_GENERATOR.generate_unique_domain_id();

    let participant = DomainParticipantFactory::get_instance()
        .create_participant(domain_id, QosKind::Default, NO_LISTENER, NO_STATUS)
        .unwrap();

    let topic = participant
        .create_topic::<MyEnum>(
            "MyEnumTopic",
            "MyEnum",
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
    let reader_qos = DataReaderQos {
        reliability: ReliabilityQosPolicy {
            kind: ReliabilityQosPolicyKind::Reliable,
            max_blocking_time: DurationKind::Finite(Duration::new(1, 0)),
        },
        ..Default::default()
    };
    let reader = subscriber
        .create_datareader::<MyEnum>(
            &topic,
            QosKind::Specific(reader_qos),
            NO_LISTENER,
            NO_STATUS,
        )
        .unwrap();

    let cond = writer.get_statuscondition();
    cond.set_enabled_statuses(&[StatusKind::PublicationMatched])
        .unwrap();

    let mut wait_set = WaitSet::new();
    wait_set
        .attach_condition(Condition::StatusCondition(cond))
        .unwrap();
    wait_set.wait(Duration::new(10, 0)).unwrap();

    let data = MyEnum::VariantB;

    writer.write(data.clone(), None).unwrap();

    writer
        .wait_for_acknowledgments(Duration::new(10, 0))
        .unwrap();

    let samples = reader
        .take(3, ANY_SAMPLE_STATE, ANY_VIEW_STATE, ANY_INSTANCE_STATE)
        .unwrap();

    assert_eq!(samples.len(), 1);
    assert_eq!(samples[0].data.as_ref().unwrap(), &data);
}

#[test]
fn nested_types_should_read_and_write() {
    #[derive(Clone, PartialEq, Eq, Debug, DdsType)]
    struct InnerType {
        a: i32,
        b: u8,
    }

    #[derive(Clone, PartialEq, Eq, Debug, DdsType)]
    struct OuterType {
        inner: Box<InnerType>,
        flag: bool,
    }

    let domain_id = TEST_DOMAIN_ID_GENERATOR.generate_unique_domain_id();

    let participant = DomainParticipantFactory::get_instance()
        .create_participant(domain_id, QosKind::Default, NO_LISTENER, NO_STATUS)
        .unwrap();

    let topic = participant
        .create_topic::<OuterType>(
            "MyEnumTopic",
            "MyEnum",
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
    let reader_qos = DataReaderQos {
        reliability: ReliabilityQosPolicy {
            kind: ReliabilityQosPolicyKind::Reliable,
            max_blocking_time: DurationKind::Finite(Duration::new(1, 0)),
        },
        ..Default::default()
    };
    let reader = subscriber
        .create_datareader::<OuterType>(
            &topic,
            QosKind::Specific(reader_qos),
            NO_LISTENER,
            NO_STATUS,
        )
        .unwrap();

    let cond = writer.get_statuscondition();
    cond.set_enabled_statuses(&[StatusKind::PublicationMatched])
        .unwrap();

    let mut wait_set = WaitSet::new();
    wait_set
        .attach_condition(Condition::StatusCondition(cond))
        .unwrap();
    wait_set.wait(Duration::new(10, 0)).unwrap();

    let data = OuterType {
        inner: Box::new(InnerType { a: 20, b: 5 }),
        flag: true,
    };

    writer.write(data.clone(), None).unwrap();

    writer
        .wait_for_acknowledgments(Duration::new(10, 0))
        .unwrap();

    let samples = reader
        .take(3, ANY_SAMPLE_STATE, ANY_VIEW_STATE, ANY_INSTANCE_STATE)
        .unwrap();

    assert_eq!(samples.len(), 1);
    assert_eq!(samples[0].data.as_ref().unwrap(), &data);
}

#[test]
fn foo_xtypes_union_should_read_and_write() {
    #[derive(Clone, Debug, PartialEq, DdsType)]
    struct MyInnerType(u32);

    #[derive(Clone, Debug, PartialEq, DdsType)]
    #[dust_dds(switch(u8))]
    enum MyEnum {
        #[dust_dds(case = 5)]
        _VariantA(MyInnerType),
        #[dust_dds(case = 0)]
        VariantB { a: u32 },
        #[dust_dds(case = 7)]
        VariantC,
    }

    let domain_id = TEST_DOMAIN_ID_GENERATOR.generate_unique_domain_id();

    let participant = DomainParticipantFactory::get_instance()
        .create_participant(domain_id, QosKind::Default, NO_LISTENER, NO_STATUS)
        .unwrap();

    let topic = participant
        .create_topic::<MyEnum>(
            "MyEnumTopic",
            "MyEnum",
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
    let reader_qos = DataReaderQos {
        reliability: ReliabilityQosPolicy {
            kind: ReliabilityQosPolicyKind::Reliable,
            max_blocking_time: DurationKind::Finite(Duration::new(1, 0)),
        },
        ..Default::default()
    };
    let reader = subscriber
        .create_datareader::<MyEnum>(
            &topic,
            QosKind::Specific(reader_qos),
            NO_LISTENER,
            NO_STATUS,
        )
        .unwrap();

    let cond = writer.get_statuscondition();
    cond.set_enabled_statuses(&[StatusKind::PublicationMatched])
        .unwrap();

    let mut wait_set = WaitSet::new();
    wait_set
        .attach_condition(Condition::StatusCondition(cond))
        .unwrap();
    wait_set.wait(Duration::new(10, 0)).unwrap();

    let data = MyEnum::VariantB { a: 10 };

    writer.write(data.clone(), None).unwrap();

    writer
        .wait_for_acknowledgments(Duration::new(10, 0))
        .unwrap();

    let samples = reader
        .take(3, ANY_SAMPLE_STATE, ANY_VIEW_STATE, ANY_INSTANCE_STATE)
        .unwrap();

    assert_eq!(samples.len(), 1);
    assert_eq!(samples[0].data.as_ref().unwrap(), &data);

    let data = MyEnum::VariantC;

    writer.write(data.clone(), None).unwrap();

    writer
        .wait_for_acknowledgments(Duration::new(10, 0))
        .unwrap();

    let samples = reader
        .take(3, ANY_SAMPLE_STATE, ANY_VIEW_STATE, ANY_INSTANCE_STATE)
        .unwrap();

    assert_eq!(samples.len(), 1);
    assert_eq!(samples[0].data.as_ref().unwrap(), &data);
}

#[test]
fn dynamic_data_should_read_and_write() {
    let mut type_buider = DynamicTypeBuilderFactory::create_type(TypeDescriptor {
        kind: dust_dds::xtypes::dynamic_type::TypeKind::STRUCTURE,
        name: "KeyedData",
        base_type: None,
        discriminator_type: None,
        bound: &[],
        element_type: None,
        key_element_type: None,
        extensibility_kind: dust_dds::xtypes::dynamic_type::ExtensibilityKind::Final,
        is_nested: false,
        is_autoid_hash: false,
    });
    type_buider
        .add_member(MemberDescriptor {
            name: "id",
            id: 0,
            r#type: <u8 as dust_dds::xtypes::type_support::Type>::TYPE,
            default_value: None,
            index: 0_u32,
            try_construct_kind: dust_dds::xtypes::dynamic_type::TryConstructKind::UseDefault,
            label: &[],
            is_key: true,
            is_optional: false,
            is_must_understand: true,
            is_shared: false,
            is_default_label: false,
            is_external: false,
        })
        .unwrap();
    type_buider
        .add_member(MemberDescriptor {
            name: "value",
            id: 1,
            r#type: <u32 as dust_dds::xtypes::type_support::Type>::TYPE,
            default_value: None,
            index: 1_u32,
            try_construct_kind: dust_dds::xtypes::dynamic_type::TryConstructKind::UseDefault,
            label: &[],
            is_key: false,
            is_optional: false,
            is_must_understand: true,
            is_shared: false,
            is_default_label: false,
            is_external: false,
        })
        .unwrap();
    let dynamic_type = type_buider.build();

    let domain_id = TEST_DOMAIN_ID_GENERATOR.generate_unique_domain_id();

    let participant = DomainParticipantFactory::get_instance()
        .create_participant(domain_id, QosKind::Default, NO_LISTENER, NO_STATUS)
        .unwrap();

    let topic = participant
        .create_dynamic_topic(
            "MyEnumTopic",
            "MyEnum",
            QosKind::Default,
            NO_LISTENER,
            NO_STATUS,
            dynamic_type,
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
    let reader_qos = DataReaderQos {
        reliability: ReliabilityQosPolicy {
            kind: ReliabilityQosPolicyKind::Reliable,
            max_blocking_time: DurationKind::Finite(Duration::new(1, 0)),
        },
        ..Default::default()
    };
    let reader = subscriber
        .create_datareader::<DynamicData>(
            &topic,
            QosKind::Specific(reader_qos),
            NO_LISTENER,
            NO_STATUS,
        )
        .unwrap();

    let cond = writer.get_statuscondition();
    cond.set_enabled_statuses(&[StatusKind::PublicationMatched])
        .unwrap();

    let mut wait_set = WaitSet::new();
    wait_set
        .attach_condition(Condition::StatusCondition(cond))
        .unwrap();
    wait_set.wait(Duration::new(10, 0)).unwrap();

    let mut data = DynamicDataFactory::create_data(dynamic_type);
    data.set_uint8_value(0, 10).unwrap();
    data.set_uint32_value(1, 100).unwrap();

    writer.write(data.clone(), None).unwrap();

    writer
        .wait_for_acknowledgments(Duration::new(10, 0))
        .unwrap();

    let samples = reader
        .take(3, ANY_SAMPLE_STATE, ANY_VIEW_STATE, ANY_INSTANCE_STATE)
        .unwrap();

    assert_eq!(samples.len(), 1);
    assert_eq!(samples[0].data.as_ref().unwrap(), &data);
}

#[test]
fn enum_should_be_always_same_instance() {
    #[derive(Clone, Debug, PartialEq, DdsType)]
    enum UnKeyedData {
        On,
        Off,
    }

    let domain_id = TEST_DOMAIN_ID_GENERATOR.generate_unique_domain_id();
    let participant = DomainParticipantFactory::get_instance()
        .create_participant(domain_id, QosKind::Default, NO_LISTENER, NO_STATUS)
        .unwrap();
    let topic = participant
        .create_topic::<UnKeyedData>(
            "MyTopic",
            "UnKeyedData",
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
        .create_datareader::<UnKeyedData>(&topic, QosKind::Default, NO_LISTENER, NO_STATUS)
        .unwrap();

    let cond = reader.get_statuscondition();
    cond.set_enabled_statuses(&[StatusKind::SubscriptionMatched])
        .unwrap();
    let mut wait_set = WaitSet::new();
    wait_set
        .attach_condition(Condition::StatusCondition(cond))
        .unwrap();
    wait_set.wait(Duration::new(10, 0)).unwrap();

    let cond1 = writer.get_statuscondition();
    cond1
        .set_enabled_statuses(&[StatusKind::PublicationMatched])
        .unwrap();
    let mut wait_set1 = WaitSet::new();
    wait_set1
        .attach_condition(Condition::StatusCondition(cond1))
        .unwrap();
    wait_set1.wait(Duration::new(10, 0)).unwrap();

    let cond = reader.get_statuscondition();
    cond.set_enabled_statuses(&[StatusKind::DataAvailable])
        .unwrap();
    let mut wait_set = WaitSet::new();
    wait_set
        .attach_condition(Condition::StatusCondition(cond))
        .unwrap();

    writer.write(UnKeyedData::On, None).unwrap();
    wait_set.wait(Duration::new(10, 0)).unwrap();
    let sample = reader
        .take(1, ANY_SAMPLE_STATE, ANY_VIEW_STATE, ANY_INSTANCE_STATE)
        .unwrap();
    assert_eq!(sample[0].data.as_ref().unwrap(), &UnKeyedData::On);

    writer.write(UnKeyedData::Off, None).unwrap();
    wait_set.wait(Duration::new(10, 0)).unwrap();
    let samples = reader
        .take(1, ANY_SAMPLE_STATE, ANY_VIEW_STATE, ANY_INSTANCE_STATE)
        .unwrap();
    assert_eq!(samples.len(), 1);
    assert_eq!(samples[0].data.as_ref().unwrap(), &UnKeyedData::Off);
}

#[test]
fn empty_type_should_read_and_write() {
    #[derive(Clone, Debug, PartialEq, DdsType)]
    struct EmptyType;

    let domain_id = TEST_DOMAIN_ID_GENERATOR.generate_unique_domain_id();

    let participant = DomainParticipantFactory::get_instance()
        .create_participant(domain_id, QosKind::Default, NO_LISTENER, NO_STATUS)
        .unwrap();

    let topic = participant
        .create_topic::<EmptyType>(
            "MyTopic",
            "EmptyType",
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
    let reader_qos = DataReaderQos {
        reliability: ReliabilityQosPolicy {
            kind: ReliabilityQosPolicyKind::Reliable,
            max_blocking_time: DurationKind::Finite(Duration::new(1, 0)),
        },
        ..Default::default()
    };
    let reader = subscriber
        .create_datareader::<EmptyType>(
            &topic,
            QosKind::Specific(reader_qos),
            NO_LISTENER,
            NO_STATUS,
        )
        .unwrap();

    let cond = writer.get_statuscondition();
    cond.set_enabled_statuses(&[StatusKind::PublicationMatched])
        .unwrap();

    let mut wait_set = WaitSet::new();
    wait_set
        .attach_condition(Condition::StatusCondition(cond))
        .unwrap();
    wait_set.wait(Duration::new(10, 0)).unwrap();

    let data = EmptyType;

    writer.write(data.clone(), None).unwrap();

    writer
        .wait_for_acknowledgments(Duration::new(10, 0))
        .unwrap();

    let samples = reader
        .take(3, ANY_SAMPLE_STATE, ANY_VIEW_STATE, ANY_INSTANCE_STATE)
        .unwrap();

    assert_eq!(samples.len(), 1);
    assert_eq!(samples[0].data.as_ref().unwrap(), &data);
}

#[test]
fn type_with_optional_should_read_and_write() {
    #[derive(Clone, Debug, PartialEq, DdsType)]
    struct FooWithOptional {
        #[dust_dds(key)]
        a: u8,
        #[dust_dds(optional)]
        maybe_b: Option<i32>,
    }

    let domain_id = TEST_DOMAIN_ID_GENERATOR.generate_unique_domain_id();

    let participant = DomainParticipantFactory::get_instance()
        .create_participant(domain_id, QosKind::Default, NO_LISTENER, NO_STATUS)
        .unwrap();

    let topic = participant
        .create_topic::<FooWithOptional>(
            "MyTopic",
            "FooWithOptional",
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
    let reader_qos = DataReaderQos {
        reliability: ReliabilityQosPolicy {
            kind: ReliabilityQosPolicyKind::Reliable,
            max_blocking_time: DurationKind::Finite(Duration::new(1, 0)),
        },
        ..Default::default()
    };
    let reader = subscriber
        .create_datareader::<FooWithOptional>(
            &topic,
            QosKind::Specific(reader_qos),
            NO_LISTENER,
            NO_STATUS,
        )
        .unwrap();

    let cond = writer.get_statuscondition();
    cond.set_enabled_statuses(&[StatusKind::PublicationMatched])
        .unwrap();

    let mut wait_set = WaitSet::new();
    wait_set
        .attach_condition(Condition::StatusCondition(cond))
        .unwrap();
    wait_set.wait(Duration::new(10, 0)).unwrap();

    let data = FooWithOptional {
        a: 8,
        maybe_b: None,
    };

    writer.write(data.clone(), None).unwrap();

    writer
        .wait_for_acknowledgments(Duration::new(10, 0))
        .unwrap();

    let samples = reader
        .take(3, ANY_SAMPLE_STATE, ANY_VIEW_STATE, ANY_INSTANCE_STATE)
        .unwrap();

    assert_eq!(samples.len(), 1);
    assert_eq!(samples[0].data.as_ref().unwrap(), &data);
}

#[test]
fn arrays_sequence_of_different_types_should_read_and_write() {
    #[derive(Clone, Debug, PartialEq, DdsType)]
    enum MyEnum {
        VariantA = 5,
        VariantB = 6,
        VariantC,
    }

    #[derive(Clone, Debug, PartialEq, DdsType)]
    struct MyStruct {
        id: u32,
        another_id: u64,
    }

    #[derive(Clone, Debug, PartialEq, DdsType)]
    struct MyEnumArray {
        sequence_enum: Vec<MyEnum>,
        array_enum: [MyEnum; 3],
        sequence_struct: Vec<MyStruct>,
        array_struct: [MyStruct; 2],
        array_bool: [bool; 4],
        sequence_bool: Vec<bool>,
        array_char: [char; 4],
        sequence_char: Vec<char>,
    }

    let domain_id = TEST_DOMAIN_ID_GENERATOR.generate_unique_domain_id();

    let participant = DomainParticipantFactory::get_instance()
        .create_participant(domain_id, QosKind::Default, NO_LISTENER, NO_STATUS)
        .unwrap();

    let topic = participant
        .create_topic::<MyEnumArray>(
            "MyEnumArrayTopic",
            "MyEnumArray",
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
    let reader_qos = DataReaderQos {
        reliability: ReliabilityQosPolicy {
            kind: ReliabilityQosPolicyKind::Reliable,
            max_blocking_time: DurationKind::Finite(Duration::new(1, 0)),
        },
        ..Default::default()
    };
    let reader = subscriber
        .create_datareader::<MyEnumArray>(
            &topic,
            QosKind::Specific(reader_qos),
            NO_LISTENER,
            NO_STATUS,
        )
        .unwrap();

    let cond = writer.get_statuscondition();
    cond.set_enabled_statuses(&[StatusKind::PublicationMatched])
        .unwrap();

    let mut wait_set = WaitSet::new();
    wait_set
        .attach_condition(Condition::StatusCondition(cond))
        .unwrap();
    wait_set.wait(Duration::new(10, 0)).unwrap();

    let data = MyEnumArray {
        sequence_enum: vec![
            MyEnum::VariantC,
            MyEnum::VariantB,
            MyEnum::VariantA,
            MyEnum::VariantC,
        ],
        array_enum: [MyEnum::VariantC, MyEnum::VariantB, MyEnum::VariantA],
        sequence_struct: vec![
            MyStruct {
                id: 123,
                another_id: 45678,
            },
            MyStruct {
                id: 987,
                another_id: 65432,
            },
        ],
        array_struct: [
            MyStruct {
                id: 123,
                another_id: 45678,
            },
            MyStruct {
                id: 987,
                another_id: 65432,
            },
        ],
        array_bool: [true, false, false, true],
        sequence_bool: vec![true, false, false, true],
        array_char: ['a', 'b', 'c', 'd'],
        sequence_char: vec!['a', 'b', 'c', 'd'],
    };

    writer.write(data.clone(), None).unwrap();

    writer
        .wait_for_acknowledgments(Duration::new(10, 0))
        .unwrap();

    let samples = reader
        .take(3, ANY_SAMPLE_STATE, ANY_VIEW_STATE, ANY_INSTANCE_STATE)
        .unwrap();

    assert_eq!(samples.len(), 1);
    assert_eq!(samples[0].data.as_ref().unwrap(), &data);
}

#[test]
fn type_with_nested_keys_should_dispose_correct_sample() {
    #[derive(DdsType, Clone)]
    struct Inner {
        #[dust_dds(key)]
        id: u8,
        b: u16,
    }

    #[derive(DdsType, Clone)]
    struct Nested {
        a: Inner,
    }

    let domain_id = TEST_DOMAIN_ID_GENERATOR.generate_unique_domain_id();
    let participant_factory = DomainParticipantFactory::get_instance();

    let participant = participant_factory
        .create_participant(domain_id, QosKind::Default, NO_LISTENER, NO_STATUS)
        .unwrap();

    let topic = participant
        .create_topic::<Nested>(
            "NestedTopic",
            "Nested",
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
    let reader_qos = DataReaderQos {
        reliability: ReliabilityQosPolicy {
            kind: ReliabilityQosPolicyKind::Reliable,
            max_blocking_time: DurationKind::Finite(Duration::new(1, 0)),
        },
        ..Default::default()
    };

    let reader = subscriber
        .create_datareader::<Nested>(
            &topic,
            QosKind::Specific(reader_qos),
            NO_LISTENER,
            NO_STATUS,
        )
        .unwrap();

    let cond = writer.get_statuscondition();
    cond.set_enabled_statuses(&[StatusKind::PublicationMatched])
        .unwrap();

    let mut wait_set = WaitSet::new();
    wait_set
        .attach_condition(Condition::StatusCondition(cond))
        .unwrap();
    wait_set.wait(Duration::new(10, 0)).unwrap();

    let nested1 = Nested {
        a: Inner { id: 1, b: 3 },
    };
    let nested2 = Nested {
        a: Inner { id: 2, b: 30 },
    };

    writer.write(nested1.clone(), None).unwrap();
    writer.write(nested2.clone(), None).unwrap();
    writer.dispose(nested1, None).unwrap();

    writer
        .wait_for_acknowledgments(Duration::new(10, 0))
        .unwrap();

    let samples = reader
        .read(4, ANY_SAMPLE_STATE, ANY_VIEW_STATE, ANY_INSTANCE_STATE)
        .unwrap();

    assert_eq!(samples.len(), 2);
    assert_eq!(
        samples[0].sample_info.instance_state,
        InstanceStateKind::Alive
    );
    assert_eq!(
        samples[1].sample_info.instance_state,
        InstanceStateKind::NotAliveDisposed
    );
}

#[test]
fn inherited_type_should_dispose_correct_sample() {
    #[derive(DdsType, Clone)]
    struct Parent {
        #[dust_dds(key)]
        id: u8,
        b: u16,
    }

    #[derive(DdsType, Clone)]
    #[dust_dds(base_type = Parent)]
    struct Child {
        parent: Parent,
        a: i32,
    }

    let domain_id = TEST_DOMAIN_ID_GENERATOR.generate_unique_domain_id();
    let participant_factory = DomainParticipantFactory::get_instance();

    let participant = participant_factory
        .create_participant(domain_id, QosKind::Default, NO_LISTENER, NO_STATUS)
        .unwrap();

    let topic = participant
        .create_topic::<Child>(
            "NestedTopic",
            "Nested",
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
    let reader_qos = DataReaderQos {
        reliability: ReliabilityQosPolicy {
            kind: ReliabilityQosPolicyKind::Reliable,
            max_blocking_time: DurationKind::Finite(Duration::new(1, 0)),
        },
        ..Default::default()
    };

    let reader = subscriber
        .create_datareader::<Child>(
            &topic,
            QosKind::Specific(reader_qos),
            NO_LISTENER,
            NO_STATUS,
        )
        .unwrap();

    let cond = writer.get_statuscondition();
    cond.set_enabled_statuses(&[StatusKind::PublicationMatched])
        .unwrap();

    let mut wait_set = WaitSet::new();
    wait_set
        .attach_condition(Condition::StatusCondition(cond))
        .unwrap();
    wait_set.wait(Duration::new(10, 0)).unwrap();

    let nested1 = Child {
        parent: Parent { id: 1, b: 3 },
        a: 1,
    };
    let nested2 = Child {
        parent: Parent { id: 2, b: 30 },
        a: 10,
    };

    writer.write(nested1.clone(), None).unwrap();
    writer.write(nested2.clone(), None).unwrap();
    writer.dispose(nested1, None).unwrap();

    writer
        .wait_for_acknowledgments(Duration::new(10, 0))
        .unwrap();

    let samples = reader
        .read(4, ANY_SAMPLE_STATE, ANY_VIEW_STATE, ANY_INSTANCE_STATE)
        .unwrap();

    assert_eq!(samples.len(), 2);
    assert_eq!(
        samples[0].sample_info.instance_state,
        InstanceStateKind::Alive
    );
    assert_eq!(
        samples[1].sample_info.instance_state,
        InstanceStateKind::NotAliveDisposed
    );
}

#[test]
fn enum_array_from_dynamic_type_should_write_read() {
    let domain_id = TEST_DOMAIN_ID_GENERATOR.generate_unique_domain_id();
    let participant1 = DomainParticipantFactory::get_instance()
        .create_participant(domain_id, QosKind::Default, NO_LISTENER, NO_STATUS)
        .unwrap();

    let type_xml = r#"
    <dds>
        <types>
            <module name="Test">
                <enum name="E1" bitBound="32" extensibility="appendable">
                    <enumerator name="VAL0" value="0"/>
                    <enumerator name="VAL1" value="1"/>
                    <enumerator name="VAL2" value="2"/>
                </enum>
                <struct name="enum1x10"   extensibility="final">
                    <member name="x1"   type="nonBasic" nonBasicTypeName="E1" arrayDimensions="10"  />
                </struct>
            </module>
        </types>
    </dds>
    "#;
    let type_builder =
        DynamicTypeBuilderFactory::create_type_w_document(type_xml, "enum1x10", vec![]).unwrap();
    let a1_dynamic_type = type_builder.build();
    let topic1 = participant1
        .create_dynamic_topic(
            "A",
            "A",
            QosKind::Default,
            NO_LISTENER,
            NO_STATUS,
            a1_dynamic_type,
        )
        .unwrap();
    let publisher = participant1
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
        .create_datawriter(
            &topic1,
            QosKind::Specific(writer_qos),
            NO_LISTENER,
            NO_STATUS,
        )
        .unwrap();

    let subscriber = participant1
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
        .create_datareader::<DynamicData<'static>>(
            &topic1,
            QosKind::Specific(reader_qos),
            NO_LISTENER,
            NO_STATUS,
        )
        .unwrap();

    let cond = writer.get_statuscondition();
    cond.set_enabled_statuses(&[StatusKind::PublicationMatched])
        .unwrap();

    let mut wait_set = WaitSet::new();
    wait_set
        .attach_condition(Condition::StatusCondition(cond))
        .unwrap();
    wait_set.wait(Duration::new(10, 0)).unwrap();

    let mut data = DynamicDataFactory::create_data(a1_dynamic_type);
    data.from_xml(
        "<struct>
            <x1>
                <item>VAL1</item>
                <item>VAL1</item>
                <item>VAL1</item>
                <item>VAL1</item>
                <item>VAL1</item>
                <item>VAL1</item>
                <item>VAL1</item>
                <item>VAL1</item>
                <item>VAL1</item>
                <item>VAL1</item>
            </x1>
        </struct>",
    )
    .unwrap();

    writer.write(data.clone(), None).unwrap();
    writer
        .wait_for_acknowledgments(Duration::new(10, 0))
        .unwrap();

    assert_eq!(
        reader.read_next_sample().unwrap().data.as_ref().unwrap(),
        &data
    );
}
