#![cfg(feature = "opcua-pubsub")]

use flow_like_industrial::{Error, opcua_pubsub::*};
use std::{
    io::{BufRead, BufReader},
    process::{Child, Command, Stdio},
    sync::Arc,
    time::Duration,
};

struct Peer(Child);
impl Drop for Peer {
    fn drop(&mut self) {
        let _ = self.0.kill();
        let _ = self.0.wait();
    }
}

async fn peer(input: &str, output: &str, security: Option<&str>) -> Peer {
    let executable = std::env::var("FLOW_LIKE_OPCUA_PUBSUB_PEER")
        .expect("Set FLOW_LIKE_OPCUA_PUBSUB_PEER to the open62541 fixture executable");
    let mut command = Command::new(executable);
    command.args([input, output]);
    if let Some(security) = security {
        command.arg(security);
    }
    let mut child = command
        .stdout(Stdio::piped())
        .stderr(Stdio::inherit())
        .spawn()
        .expect("start open62541 peer");
    let stdout = child.stdout.take().unwrap();
    let guard = Peer(child);
    let (ready_tx, ready_rx) = tokio::sync::oneshot::channel();
    std::thread::spawn(move || {
        let mut ready_tx = Some(ready_tx);
        for line in BufReader::new(stdout).lines() {
            let line = line.unwrap();
            eprintln!("open62541: {line}");
            if line == "READY"
                && let Some(tx) = ready_tx.take()
            {
                let _ = tx.send(());
            }
        }
    });
    tokio::time::timeout(Duration::from_secs(10), ready_rx)
        .await
        .expect("peer startup deadline")
        .expect("peer startup failed");
    guard
}

fn port() -> u16 {
    std::net::UdpSocket::bind("127.0.0.1:0")
        .unwrap()
        .local_addr()
        .unwrap()
        .port()
}

fn values() -> Vec<PubSubValue> {
    vec![
        PubSubValue::Boolean(true),
        PubSubValue::SByte(-100),
        PubSubValue::Byte(250),
        PubSubValue::Int16(-30000),
        PubSubValue::UInt16(60000),
        PubSubValue::Int32(-2_000_000_000),
        PubSubValue::UInt32(4_000_000_000),
        PubSubValue::Int64(-9_000_000_000_000_000_000),
        PubSubValue::UInt64(18_000_000_000_000_000_000),
        PubSubValue::Float(-12.5),
        PubSubValue::Double(1234.125),
        PubSubValue::String(Some("温度 / Pressure".into())),
        PubSubValue::DateTime(133_000_000_000_000_123),
        PubSubValue::ByteString(Some(vec![0, 255, 128, 42])),
        PubSubValue::StatusCode(0x4000_0000),
    ]
}

fn config(address: String, mode: PubSubMode) -> OpcUaPubSubConfig {
    let reader = mode == PubSubMode::Subscriber;
    OpcUaPubSubConfig {
        address,
        interface: "0.0.0.0".into(),
        mode,
        writer: PubSubWriter {
            publisher_id: PublisherId::UInt16(if reader { 2235 } else { 2234 }),
            writer_group_id: if reader { 101 } else { 100 },
            data_set_writer_id: if reader { 62542 } else { 62541 },
        },
        fields: values()
            .iter()
            .enumerate()
            .map(|(i, v)| PubSubFieldMetaData {
                name: format!("field_{i}"),
                data_type: v.data_type(),
            })
            .collect(),
        configuration_version: None,
        timeout_ms: 2_000,
        max_datagram_bytes: 1400,
        security: None,
    }
}

async fn exchange(multicast: bool) {
    let host = if multicast {
        "239.255.42.77"
    } else {
        "127.0.0.1"
    };
    let input = format!("opc.udp://{host}:{}", port());
    let output = format!("opc.udp://{host}:{}", port());
    let subscriber_config = config(output.clone(), PubSubMode::Subscriber);
    let _peer = peer(&input, &output, None).await;
    let subscriber = OpcUaPubSubClient::connect(subscriber_config.clone())
        .await
        .unwrap();
    let publisher = OpcUaPubSubClient::connect(config(input.clone(), PubSubMode::Publisher))
        .await
        .unwrap();
    // The initial frame proves the independent publisher is running before testing the round trip.
    let initial = subscriber.receive().await.unwrap();
    assert!(initial.timestamp.is_some());
    assert!(initial.sequence_number.is_some());
    let version = ConfigurationVersion {
        major: initial.configuration_major_version.unwrap(),
        minor: initial.configuration_minor_version.unwrap(),
    };
    let expected = values();
    let message = tokio::time::timeout(Duration::from_secs(8), async {
        loop {
            publisher.publish(&expected).await.unwrap();
            let message = subscriber.receive().await.unwrap();
            if message
                .fields
                .iter()
                .map(|f| f.value.clone())
                .collect::<Vec<_>>()
                == expected
            {
                break message;
            }
        }
    })
    .await
    .expect("open62541 must decode all typed values, update target variables, and republish them");
    assert_eq!(message.writer, subscriber_config.writer);
    assert_eq!(message.fields[11].name, "field_11");
    subscriber.close().await.unwrap();

    let mut versioned = subscriber_config.clone();
    versioned.configuration_version = Some(version);
    let reconnected = OpcUaPubSubClient::connect(versioned.clone()).await.unwrap();
    assert_eq!(
        reconnected.receive().await.unwrap().fields[13].value,
        expected[13]
    );
    reconnected.close().await.unwrap();
    versioned.configuration_version.as_mut().unwrap().major ^= 1;
    let wrong_version = OpcUaPubSubClient::connect(versioned).await.unwrap();
    assert!(
        wrong_version
            .receive()
            .await
            .unwrap_err()
            .to_string()
            .contains("version")
    );
    wrong_version.close().await.unwrap();
    let mut wrong_type = subscriber_config.clone();
    wrong_type.fields[5].data_type = PubSubDataType::String;
    let wrong_type = OpcUaPubSubClient::connect(wrong_type).await.unwrap();
    assert!(
        wrong_type
            .receive()
            .await
            .unwrap_err()
            .to_string()
            .contains("field type")
    );
    wrong_type.close().await.unwrap();

    let mut other_writer = subscriber_config;
    other_writer.writer.data_set_writer_id += 1;
    other_writer.timeout_ms = 150;
    let ignored = OpcUaPubSubClient::connect(other_writer.clone())
        .await
        .unwrap();
    assert!(matches!(ignored.receive().await, Err(Error::Timeout)));
    ignored.close().await.unwrap();
    other_writer.timeout_ms = 30_000;
    let ignored = Arc::new(OpcUaPubSubClient::connect(other_writer).await.unwrap());
    let pending = {
        let ignored = ignored.clone();
        tokio::spawn(async move { ignored.receive().await })
    };
    tokio::time::sleep(Duration::from_millis(50)).await;
    ignored.close().await.unwrap();
    assert!(
        tokio::time::timeout(Duration::from_secs(1), pending)
            .await
            .unwrap()
            .unwrap()
            .is_err()
    );
    publisher.close().await.unwrap();
    assert!(publisher.publish(&expected).await.is_err());
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
#[ignore = "requires the open62541 fixture executable"]
async fn open62541_unicast_typed_roundtrip_metadata_filtering_and_lifecycle() {
    exchange(false).await;
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
#[ignore = "requires the open62541 fixture executable and multicast networking"]
async fn open62541_multicast_typed_roundtrip_metadata_filtering_and_lifecycle() {
    exchange(true).await;
}

struct SecurityState(std::path::PathBuf);
impl SecurityState {
    fn new() -> Self {
        let mut random = [0; 16];
        getrandom::fill(&mut random).unwrap();
        let name: String = random.iter().map(|b| format!("{b:02x}")).collect();
        Self(std::env::temp_dir().join(format!("flow-pubsub-e2e-{name}")))
    }
}
impl Drop for SecurityState {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

fn security(mode: PubSubSecurityMode, reader: bool) -> PubSubSecurity {
    PubSubSecurity {
        mode,
        security_token_id: if reader { 43 } else { 42 },
        signing_key: vec![if reader { 33 } else { 11 }; 32],
        encrypting_key: vec![if reader { 44 } else { 22 }; 32],
        key_nonce: if reader {
            vec![5, 6, 7, 8]
        } else {
            vec![1, 2, 3, 4]
        },
    }
}

async fn captured(socket: &tokio::net::UdpSocket) -> Vec<u8> {
    let mut buffer = vec![0; 65507];
    let count = tokio::time::timeout(Duration::from_secs(3), socket.recv(&mut buffer))
        .await
        .expect("independent peer publishes a secure datagram")
        .unwrap();
    buffer.truncate(count);
    buffer
}

async fn secure_exchange(mode: PubSubSecurityMode) {
    let state = SecurityState::new();
    let input = format!("opc.udp://127.0.0.1:{}", port());
    let capture_address = format!("127.0.0.1:{}", port());
    let peer_output = format!("opc.udp://{capture_address}");
    let destination = format!("127.0.0.1:{}", port());
    let mut subscriber_config = config(format!("opc.udp://{destination}"), PubSubMode::Subscriber);
    subscriber_config.timeout_ms = 200;
    subscriber_config.security = Some(security(mode, true));
    let subscriber =
        OpcUaPubSubClient::connect_with_security_store(subscriber_config.clone(), &state.0)
            .await
            .unwrap();
    let mut publisher_config = config(input.clone(), PubSubMode::Publisher);
    publisher_config.security = Some(security(mode, false));
    assert!(
        OpcUaPubSubClient::connect(publisher_config.clone())
            .await
            .is_err()
    );
    let publisher =
        OpcUaPubSubClient::connect_with_security_store(publisher_config.clone(), &state.0)
            .await
            .unwrap();
    let _peer = peer(
        &input,
        &peer_output,
        Some(if mode == PubSubSecurityMode::Sign {
            "sign"
        } else {
            "encrypt"
        }),
    )
    .await;
    let capture = tokio::net::UdpSocket::bind(&capture_address).await.unwrap();
    let expected = values();
    let message = tokio::time::timeout(Duration::from_secs(8), async {
        loop {
            publisher.publish(&expected).await.unwrap();
            let bytes = captured(&capture).await;
            capture.send_to(&bytes, &destination).await.unwrap();
            let message = subscriber.receive().await.unwrap();
            if message
                .fields
                .iter()
                .map(|f| f.value.clone())
                .collect::<Vec<_>>()
                == expected
            {
                break message;
            }
        }
    })
    .await
    .expect("open62541 verifies/decrypts typed fields and independently signs/encrypts the echo");
    assert_eq!(message.security_token_id, Some(43));
    assert!(message.security_sequence_number.unwrap() > 0);
    assert!(message.timestamp.is_some());

    // A wrong signing key must not update the independent subscriber's variables.
    let mut wrong_publisher = publisher_config.clone();
    wrong_publisher.security.as_mut().unwrap().signing_key[0] ^= 1;
    let wrong_publisher = OpcUaPubSubClient::connect_with_security_store(wrong_publisher, &state.0)
        .await
        .unwrap();
    let mut forbidden = expected.clone();
    forbidden[0] = PubSubValue::Boolean(false);
    wrong_publisher.publish(&forbidden).await.unwrap();
    for _ in 0..5 {
        let bytes = captured(&capture).await;
        capture.send_to(&bytes, &destination).await.unwrap();
        assert_eq!(
            subscriber.receive().await.unwrap().fields[0].value,
            expected[0]
        );
    }
    wrong_publisher.close().await.unwrap();

    let authentic = captured(&capture).await;
    let mut tampered = authentic.clone();
    let last = tampered.len() - 1;
    tampered[last] ^= 1;
    capture.send_to(&tampered, &destination).await.unwrap();
    assert!(
        subscriber
            .receive()
            .await
            .unwrap_err()
            .to_string()
            .contains("signature")
    );
    // Rejection must not consume the authenticated packet's replay counter.
    capture.send_to(&authentic, &destination).await.unwrap();
    let accepted = subscriber.receive().await.unwrap();
    capture.send_to(&authentic, &destination).await.unwrap();
    assert!(matches!(subscriber.receive().await, Err(Error::Timeout)));
    subscriber.close().await.unwrap();
    let restarted =
        OpcUaPubSubClient::connect_with_security_store(subscriber_config.clone(), &state.0)
            .await
            .unwrap();
    capture.send_to(&authentic, &destination).await.unwrap();
    assert!(matches!(restarted.receive().await, Err(Error::Timeout)));
    restarted.close().await.unwrap();

    let mut wrong_key = subscriber_config.clone();
    wrong_key.security.as_mut().unwrap().signing_key[0] ^= 1;
    let wrong_key = OpcUaPubSubClient::connect_with_security_store(wrong_key, &state.0)
        .await
        .unwrap();
    capture.send_to(&authentic, &destination).await.unwrap();
    assert!(
        wrong_key
            .receive()
            .await
            .unwrap_err()
            .to_string()
            .contains("signature")
    );
    wrong_key.close().await.unwrap();
    let mut wrong_token = subscriber_config.clone();
    wrong_token.security.as_mut().unwrap().security_token_id += 1;
    let wrong_token = OpcUaPubSubClient::connect_with_security_store(wrong_token, &state.0)
        .await
        .unwrap();
    capture.send_to(&authentic, &destination).await.unwrap();
    assert!(
        wrong_token
            .receive()
            .await
            .unwrap_err()
            .to_string()
            .contains("token")
    );
    wrong_token.close().await.unwrap();

    if mode == PubSubSecurityMode::SignAndEncrypt {
        let mut wrong_key = subscriber_config.clone();
        wrong_key.security.as_mut().unwrap().encrypting_key[0] ^= 1;
        let wrong_key = OpcUaPubSubClient::connect_with_security_store(wrong_key, &state.0)
            .await
            .unwrap();
        capture.send_to(&authentic, &destination).await.unwrap();
        assert!(wrong_key.receive().await.is_err());
        wrong_key.close().await.unwrap();
        assert!(
            !authentic
                .windows("温度 / Pressure".len())
                .any(|part| part == "温度 / Pressure".as_bytes())
        );
    }
    publisher.close().await.unwrap();
    let publisher = OpcUaPubSubClient::connect_with_security_store(publisher_config, &state.0)
        .await
        .unwrap();
    publisher.publish(&expected).await.unwrap();
    let subscriber = OpcUaPubSubClient::connect_with_security_store(subscriber_config, &state.0)
        .await
        .unwrap();
    let next = captured(&capture).await;
    capture.send_to(&next, &destination).await.unwrap();
    assert!(
        subscriber.receive().await.unwrap().security_sequence_number
            > accepted.security_sequence_number
    );
    subscriber.close().await.unwrap();
    publisher.close().await.unwrap();
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
#[ignore = "requires the open62541 fixture executable with PubSub encryption"]
async fn open62541_signed_roundtrip_rejects_tampering_wrong_keys_and_replays() {
    secure_exchange(PubSubSecurityMode::Sign).await;
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
#[ignore = "requires the open62541 fixture executable with PubSub encryption"]
async fn open62541_encrypted_roundtrip_rejects_tampering_wrong_keys_and_replays() {
    secure_exchange(PubSubSecurityMode::SignAndEncrypt).await;
}
