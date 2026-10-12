mod utils;

use dust_dds::{
    dds_async::domain_participant_factory::DomainParticipantFactoryAsync,
    domain::domain_participant::DomainParticipant,
    infrastructure::{listener::NO_LISTENER, qos::QosKind, status::NO_STATUS},
    rtps_messages::{
        overall_structure::RtpsMessageWrite,
        submessage_elements::{Data, ParameterList},
        submessages::data::DataSubmessage,
    },
    transport::{
        interface::{
            RtpsTransportParticipant, TransportDataReceiver, TransportParticipantFactory,
            WriteMessage,
        },
        types::{BUILT_IN_WRITER_WITH_KEY, ENTITYID_UNKNOWN, EntityId, Locator},
    },
};

use crate::utils::domain_id_generator::TEST_DOMAIN_ID_GENERATOR;

struct MockWriter {
    buffer: [u8; 512],
}
impl WriteMessage for MockWriter {
    fn write_buffer_mut(&mut self) -> &mut [u8] {
        &mut self.buffer
    }
    fn write_message(&mut self, _len: usize, _locators: &[Locator]) {}
}

struct MockTransport(std::sync::mpsc::SyncSender<TransportDataReceiver>);
impl TransportParticipantFactory for MockTransport {
    fn create_participant(
        &self,
        _domain_id: i32,
        data_receiver: TransportDataReceiver,
    ) -> RtpsTransportParticipant {
        self.0.send(data_receiver).unwrap();
        RtpsTransportParticipant {
            message_writer: Box::new(MockWriter { buffer: [0; 512] }),
            default_unicast_locator_list: Vec::new(),
            metatraffic_unicast_locator_list: Vec::new(),
            metatraffic_multicast_locator_list: Vec::new(),
            default_multicast_locator_list: Vec::new(),
            fragment_size: 1000,
        }
    }
}

#[test]
fn detect_stale_participant() {
    let (data_receiver_send, data_receiver_recv) = std::sync::mpsc::sync_channel(1);
    let domain_id = TEST_DOMAIN_ID_GENERATOR.generate_unique_domain_id();

    let runtime = dust_dds::std_runtime::StdRuntime::default();
    let app_id = [1, 2, 3, 4];
    let host_id = [5, 6, 7, 8];
    let configuration = Default::default();

    let domain_participant_factory = DomainParticipantFactoryAsync::new(
        runtime,
        app_id,
        host_id,
        MockTransport(data_receiver_send),
        configuration,
    );

    let participant = DomainParticipant::from(
        dust_dds::std_runtime::executor::block_on(domain_participant_factory.create_participant(
            domain_id,
            QosKind::Default,
            NO_LISTENER,
            NO_STATUS,
        ))
        .unwrap(),
    );

    let guid_prefix: [u8; 12] = <[u8; 16]>::from(participant.get_instance_handle())[0..12]
        .try_into()
        .unwrap();

    let data_receiver = data_receiver_recv.recv().unwrap();

    let reader_id = ENTITYID_UNKNOWN;
    const ENTITYID_SPDP_BUILTIN_PARTICIPANT_WRITER: EntityId =
        EntityId::new([0x00, 0x01, 0x00], BUILT_IN_WRITER_WITH_KEY);
    let writer_id = ENTITYID_SPDP_BUILTIN_PARTICIPANT_WRITER;
    let inline_qos_flag = false;
    let data_flag = true;
    let key_flag = false;
    let non_standard_payload_flag = false;
    let writer_sn = 1;
    let inline_qos = ParameterList::empty();
    let serialized_payload = Data::new(
        vec![
            0x00, 0x03, 0x00, 0x00, // PL_CDR_LE
            0x50, 0x00, 16, 0x00, // PID_PARTICIPANT_GUID, Length
            8, 8, 8, 8, // GuidPrefix
            8, 8, 8, 8, // GuidPrefix
            8, 8, 8, 8, // GuidPrefix
            0, 0, 1, 0xc1, // EntityId
            0x15, 0x00, 4, 0x00, // PID_PROTOCOL_VERSION, Length
            0x02, 0x04, 0x00, 0x00, // ProtocolVersion
            0x16, 0x00, 4, 0x00, // PID_VENDORID
            73, 74, 0x00, 0x00, // VendorId
            88, 0x00, 4, 0x00, // PID_BUILTIN_ENDPOINT_SET
            0x02, 0x00, 0x00, 0x00, //
            0x02, 0x00, 8, 0x00, // PID_PARTICIPANT_LEASE_DURATION
            2, 0x00, 0x00, 0x00, // Duration: seconds
            0, 0x00, 0x00, 0x00, // Duration: fraction
            0x01, 0x00, 0x00, 0x00, // PID_SENTINEL
        ]
        .into(),
    );
    let spdp_data_submessage = DataSubmessage::new(
        inline_qos_flag,
        data_flag,
        key_flag,
        non_standard_payload_flag,
        reader_id,
        writer_id,
        writer_sn,
        inline_qos,
        serialized_payload,
    );
    let mut buf = [0u8; 1024];
    let spdp_rtps_message =
        RtpsMessageWrite::from_submessages(&mut buf, &[&spdp_data_submessage], guid_prefix);

    dust_dds::std_runtime::executor::block_on(
        data_receiver.receive_message(spdp_rtps_message.buffer().to_vec()),
    );

    std::thread::sleep(std::time::Duration::from_secs(1));

    assert_eq!(participant.get_discovered_participants().unwrap().len(), 1);

    // Wait longer than lease duration communicated in the discovery message
    std::thread::sleep(std::time::Duration::from_secs(2));

    assert_eq!(participant.get_discovered_participants().unwrap().len(), 0);
}

#[test]
fn xtypes_mismatch_does_not_abort_discovery() {
    let (data_receiver_send, data_receiver_recv) = std::sync::mpsc::sync_channel(1);
    let domain_id = TEST_DOMAIN_ID_GENERATOR.generate_unique_domain_id();

    let runtime = dust_dds::std_runtime::StdRuntime::default();
    let app_id = [1, 2, 3, 4];
    let host_id = [5, 6, 7, 8];
    let configuration = Default::default();

    let domain_participant_factory = DomainParticipantFactoryAsync::new(
        runtime,
        app_id,
        host_id,
        MockTransport(data_receiver_send),
        configuration,
    );

    let participant = DomainParticipant::from(
        dust_dds::std_runtime::executor::block_on(domain_participant_factory.create_participant(
            domain_id,
            QosKind::Default,
            NO_LISTENER,
            NO_STATUS,
        ))
        .unwrap(),
    );

    let data_receiver = data_receiver_recv.recv().unwrap();

    #[derive(dust_dds::infrastructure::type_support::DdsType)]
    struct LocalType1 {
        id: i32,
    }
    #[derive(dust_dds::infrastructure::type_support::DdsType)]
    struct LocalType2 {
        id: i32,
    }

    let topic1 = participant
        .create_topic::<LocalType1>(
            "Topic1",
            "LocalType1",
            QosKind::Default,
            NO_LISTENER,
            NO_STATUS,
        )
        .unwrap();
    let topic2 = participant
        .create_topic::<LocalType2>(
            "Topic2",
            "LocalType2",
            QosKind::Default,
            NO_LISTENER,
            NO_STATUS,
        )
        .unwrap();

    let publisher = participant
        .create_publisher(QosKind::Default, NO_LISTENER, NO_STATUS)
        .unwrap();
    let _writer1 = publisher
        .create_datawriter::<LocalType1>(&topic1, QosKind::Default, NO_LISTENER, NO_STATUS)
        .unwrap();
    let writer2 = publisher
        .create_datawriter::<LocalType2>(&topic2, QosKind::Default, NO_LISTENER, NO_STATUS)
        .unwrap();

    // 1. Announce remote participant via SPDP
    let remote_guid_prefix = [8; 12];
    const ENTITYID_SPDP_BUILTIN_PARTICIPANT_WRITER: EntityId =
        EntityId::new([0x00, 0x01, 0x00], BUILT_IN_WRITER_WITH_KEY);
    let spdp_payload = Data::new(
        vec![
            0x00, 0x03, 0x00, 0x00, // PL_CDR_LE
            0x50, 0x00, 16, 0x00, // PID_PARTICIPANT_GUID
            8, 8, 8, 8, 8, 8, 8, 8, 8, 8, 8, 8, 0, 0, 1, 0xc1, 0x15, 0x00, 4,
            0x00, // PID_PROTOCOL_VERSION
            0x02, 0x04, 0x00, 0x00, 0x16, 0x00, 4, 0x00, // PID_VENDORID
            73, 74, 0x00, 0x00, 0x58, 0x00, 4, 0x00, // PID_BUILTIN_ENDPOINT_SET
            0x3f, 0x00, 0x00, 0x00, 0x02, 0x00, 8, 0x00, // PID_PARTICIPANT_LEASE_DURATION
            100, 0x00, 0x00, 0x00, 0, 0x00, 0x00, 0x00, 0x01, 0x00, 0x00,
            0x00, // PID_SENTINEL
        ]
        .into(),
    );
    let spdp_submsg = DataSubmessage::new(
        false,
        true,
        false,
        false,
        ENTITYID_UNKNOWN,
        ENTITYID_SPDP_BUILTIN_PARTICIPANT_WRITER,
        1,
        ParameterList::empty(),
        spdp_payload,
    );
    let mut buf = [0u8; 1024];
    let spdp_msg =
        RtpsMessageWrite::from_submessages(&mut buf, &[&spdp_submsg], remote_guid_prefix);
    dust_dds::std_runtime::executor::block_on(
        data_receiver.receive_message(spdp_msg.buffer().to_vec()),
    );

    std::thread::sleep(std::time::Duration::from_millis(100));

    // 2. Announce Reader 1 on Topic1 with mismatched TypeInformation (serialized_size > 0)
    const ENTITYID_SEDP_BUILTIN_SUBSCRIPTIONS_WRITER: EntityId =
        EntityId::new([0x00, 0x00, 0x04], BUILT_IN_WRITER_WITH_KEY);

    let reader1_sedp_payload = Data::new(
        vec![
            0x00, 0x03, 0x00, 0x00, // PL_CDR_LE
            0x5a, 0x00, 16, 0x00, // PID_ENDPOINT_GUID
            8, 8, 8, 8, 8, 8, 8, 8, 8, 8, 8, 8, 0, 0, 1, 0x04, 0x50, 0x00, 16,
            0x00, // PID_PARTICIPANT_GUID
            8, 8, 8, 8, 8, 8, 8, 8, 8, 8, 8, 8, 0, 0, 1, 0xc1, 0x05, 0x00, 12,
            0x00, // PID_TOPIC_NAME (Topic1)
            7, 0x00, 0x00, 0x00, b'T', b'o', b'p', b'i', b'c', b'1', 0, 0, 0x07, 0x00, 16,
            0x00, // PID_TYPE_NAME (LocalType1)
            11, 0x00, 0x00, 0x00, b'L', b'o', b'c', b'a', b'l', b'T', b'y', b'p', b'e', b'1', 0, 0,
            // PID_TYPE_INFORMATION: XCDR2 mutable struct TypeInformation with serialized_size > 0
            0x75, 0x00, 76, 0x00, 72, 0x00, 0x00, 0x00, // DHEADER (72 bytes)
            // minimal (id 0x1001, lc 4)
            0x01, 0x10, 0x00, 0x40, // emheader
            28, 0x00, 0x00, 0x00, // length
            100, 0x00, 0x00, 0x00, // typeobject_serialized_size = 100 > 0
            0xf1, 9, 9, 9, 9, 9, 9, 9, 9, 9, 9, 9, 9, 9, 9,
            0, // type_id (EK_MINIMAL + 14-byte hash + pad)
            0, 0, 0, 0, // dependent_typeid_count
            0, 0, 0, 0, // dependent_typeids len
            // complete (id 0x1002, lc 4)
            0x02, 0x10, 0x00, 0x40, // emheader
            28, 0x00, 0x00, 0x00, // length
            100, 0x00, 0x00, 0x00, // typeobject_serialized_size = 100 > 0
            0xf2, 9, 9, 9, 9, 9, 9, 9, 9, 9, 9, 9, 9, 9, 9,
            0, // type_id (EK_COMPLETE + 14-byte hash + pad)
            0, 0, 0, 0, // dependent_typeid_count
            0, 0, 0, 0, // dependent_typeids len
            0x01, 0x00, 0x00, 0x00, // PID_SENTINEL
        ]
        .into(),
    );
    let sedp_submsg1 = DataSubmessage::new(
        false,
        true,
        false,
        false,
        ENTITYID_UNKNOWN,
        ENTITYID_SEDP_BUILTIN_SUBSCRIPTIONS_WRITER,
        1,
        ParameterList::empty(),
        reader1_sedp_payload,
    );

    // 3. Announce Reader 2 on Topic2 (matching LocalType2)
    let reader2_sedp_payload = Data::new(
        vec![
            0x00, 0x03, 0x00, 0x00, // PL_CDR_LE
            0x5a, 0x00, 16, 0x00, // PID_ENDPOINT_GUID
            8, 8, 8, 8, 8, 8, 8, 8, 8, 8, 8, 8, 0, 0, 2, 0x04, 0x50, 0x00, 16,
            0x00, // PID_PARTICIPANT_GUID
            8, 8, 8, 8, 8, 8, 8, 8, 8, 8, 8, 8, 0, 0, 1, 0xc1, 0x05, 0x00, 12,
            0x00, // PID_TOPIC_NAME (Topic2)
            7, 0x00, 0x00, 0x00, b'T', b'o', b'p', b'i', b'c', b'2', 0, 0, 0x07, 0x00, 16,
            0x00, // PID_TYPE_NAME (LocalType2)
            11, 0x00, 0x00, 0x00, b'L', b'o', b'c', b'a', b'l', b'T', b'y', b'p', b'e', b'2', 0, 0,
            0x01, 0x00, 0x00, 0x00, // PID_SENTINEL
        ]
        .into(),
    );
    let sedp_submsg2 = DataSubmessage::new(
        false,
        true,
        false,
        false,
        ENTITYID_UNKNOWN,
        ENTITYID_SEDP_BUILTIN_SUBSCRIPTIONS_WRITER,
        2,
        ParameterList::empty(),
        reader2_sedp_payload,
    );

    let mut buf = [0u8; 2048];
    let sedp_msg = RtpsMessageWrite::from_submessages(
        &mut buf,
        &[&sedp_submsg1, &sedp_submsg2],
        remote_guid_prefix,
    );
    dust_dds::std_runtime::executor::block_on(
        data_receiver.receive_message(sedp_msg.buffer().to_vec()),
    );

    std::thread::sleep(std::time::Duration::from_millis(500));

    // writer2 on Topic2 should have matched Reader 2, even though Reader 1 had a type mismatch
    assert_eq!(
        writer2.get_matched_subscriptions().unwrap().len(),
        1,
        "Writer 2 should have discovered Reader 2 despite Reader 1 type mismatch"
    );
}
