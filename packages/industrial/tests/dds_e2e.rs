#![cfg(any(feature = "execute", feature = "dds"))]

use flow_like_industrial::{
    Error,
    dds::{DdsClient, DdsConfig, DdsField, DdsPrimitive, DdsReliability, DdsRole, DdsType},
};
use serde_json::json;
use std::{
    process::{Child, Command, Stdio},
    sync::Arc,
    time::{Duration, Instant},
};

struct Reference(Child);
impl Reference {
    fn spawn(domain: i32, prefix: &str, reliability: &str) -> Self {
        let binary = std::env::var_os("FLOW_LIKE_DDS_REFERENCE")
            .expect("set FLOW_LIKE_DDS_REFERENCE to the compiled Cyclone DDS fixture");
        Self(
            Command::new(binary)
                .args([&domain.to_string(), prefix, reliability])
                .stdin(Stdio::null())
                .stdout(Stdio::inherit())
                .stderr(Stdio::inherit())
                .spawn()
                .expect("start independent Cyclone DDS reference"),
        )
    }

    async fn verified(&mut self) {
        tokio::time::timeout(Duration::from_secs(10), async {
            loop {
                if let Some(status) = self.0.try_wait().unwrap() {
                    assert!(
                        status.success(),
                        "Cyclone DDS fixture rejected publication: {status}"
                    );
                    return;
                }
                tokio::time::sleep(Duration::from_millis(20)).await;
            }
        })
        .await
        .expect("Cyclone DDS fixture did not finish");
    }
}
impl Drop for Reference {
    fn drop(&mut self) {
        let _ = self.0.kill();
        let _ = self.0.wait();
    }
}

fn config(domain_id: i32, topic: &str, role: DdsRole, reliability: DdsReliability) -> DdsConfig {
    DdsConfig {
        domain_id,
        topic: topic.into(),
        role,
        reliability,
        timeout_ms: 15_000,
        data_type: DdsType {
            name: "Fixture::Telemetry".into(),
            fields: [
                ("id", DdsPrimitive::UInt32, false, true),
                ("label", DdsPrimitive::String, false, false),
                ("temperature", DdsPrimitive::Float64, false, false),
                ("samples", DdsPrimitive::Int16, true, false),
                ("ready", DdsPrimitive::Boolean, false, false),
                ("counter", DdsPrimitive::UInt64, false, false),
            ]
            .into_iter()
            .map(|(name, data_type, sequence, key)| DdsField {
                name: name.into(),
                data_type,
                sequence,
                key,
            })
            .collect(),
        },
        ..Default::default()
    }
}

async fn exchange(publisher: &DdsClient, subscriber: &DdsClient, peer: &mut Reference) {
    publisher.wait_for_match().await.unwrap();
    subscriber.wait_for_match().await.unwrap();
    publisher
        .publish(json!({
            "id": 42, "label": "flow-like", "temperature": 23.5,
            "samples": [-32768, 0, 32767], "ready": true, "counter": u64::MAX,
        }))
        .await
        .unwrap();
    let sample = subscriber.receive().await.unwrap();
    assert!(sample.valid_data);
    assert_eq!(sample.instance_state, "alive");
    assert_eq!(sample.type_name, "Fixture::Telemetry");
    assert!(sample.source_timestamp_ns.is_some());
    assert_eq!(
        sample.data.unwrap(),
        json!({
            "id": 42, "label": "cyclonedds", "temperature": 24.5,
            "samples": [-7, 42, 1024], "ready": true, "counter": u64::MAX,
        })
    );
    publisher
        .publish(json!({
            "id": 42, "label": "dispose", "temperature": 23.5,
            "samples": [-32768, 0, 32767], "ready": true, "counter": u64::MAX,
        }))
        .await
        .unwrap();
    let disposed = subscriber.receive().await.unwrap();
    assert!(!disposed.valid_data);
    assert!(disposed.data.is_none());
    assert_eq!(disposed.instance_state, "disposed");
    publisher.close().await.unwrap();
    subscriber.close().await.unwrap();
    peer.verified().await;
}

#[tokio::test(flavor = "multi_thread")]
#[ignore = "requires the independent Cyclone DDS reference and multicast networking"]
async fn reliable_typed_samples_interoperate_in_both_directions() {
    let prefix = "FlowLikeReliable";
    let mut peer = Reference::spawn(171, prefix, "reliable");
    let publisher = DdsClient::connect(config(
        171,
        &format!("{prefix}Input"),
        DdsRole::Publisher,
        DdsReliability::Reliable,
    ))
    .await
    .unwrap();
    let subscriber = DdsClient::connect(config(
        171,
        &format!("{prefix}Output"),
        DdsRole::Subscriber,
        DdsReliability::Reliable,
    ))
    .await
    .unwrap();
    exchange(&publisher, &subscriber, &mut peer).await;
}

#[tokio::test(flavor = "multi_thread")]
#[ignore = "requires the independent Cyclone DDS reference and multicast networking"]
async fn best_effort_interoperates_and_reliable_reader_rejects_incompatible_qos() {
    let prefix = "FlowLikeBestEffort";
    let mut peer = Reference::spawn(172, prefix, "best_effort");
    let publisher = DdsClient::connect(config(
        172,
        &format!("{prefix}Input"),
        DdsRole::Publisher,
        DdsReliability::BestEffort,
    ))
    .await
    .unwrap();
    let subscriber = DdsClient::connect(config(
        172,
        &format!("{prefix}Output"),
        DdsRole::Subscriber,
        DdsReliability::BestEffort,
    ))
    .await
    .unwrap();
    subscriber.wait_for_match().await.unwrap();
    let mut incompatible = config(
        172,
        &format!("{prefix}Output"),
        DdsRole::Subscriber,
        DdsReliability::Reliable,
    );
    incompatible.timeout_ms = 500;
    let incompatible = DdsClient::connect(incompatible).await.unwrap();
    assert!(matches!(
        incompatible.wait_for_match().await,
        Err(Error::Timeout)
    ));
    incompatible.close().await.unwrap();
    exchange(&publisher, &subscriber, &mut peer).await;
}

#[tokio::test(flavor = "multi_thread")]
#[ignore = "requires the independent Cyclone DDS reference and multicast networking"]
async fn discovery_respects_domains_topics_and_incompatible_schemas() {
    let prefix = "FlowLikeIsolation";
    let mut peer = Reference::spawn(173, prefix, "reliable");
    let publisher = DdsClient::connect(config(
        173,
        &format!("{prefix}Input"),
        DdsRole::Publisher,
        DdsReliability::Reliable,
    ))
    .await
    .unwrap();
    let subscriber = DdsClient::connect(config(
        173,
        &format!("{prefix}Output"),
        DdsRole::Subscriber,
        DdsReliability::Reliable,
    ))
    .await
    .unwrap();
    subscriber.wait_for_match().await.unwrap();
    for (domain, topic) in [
        (174, format!("{prefix}Output")),
        (173, "NoSuchDdsTopic".into()),
    ] {
        let mut wrong = config(
            domain,
            &topic,
            DdsRole::Subscriber,
            DdsReliability::Reliable,
        );
        wrong.timeout_ms = 200;
        let wrong = DdsClient::connect(wrong).await.unwrap();
        assert!(matches!(wrong.wait_for_match().await, Err(Error::Timeout)));
        wrong.close().await.unwrap();
    }
    let mut incompatible = config(
        173,
        &format!("{prefix}Output"),
        DdsRole::Subscriber,
        DdsReliability::Reliable,
    );
    incompatible.data_type.fields[2].data_type = DdsPrimitive::String;
    incompatible.timeout_ms = 750;
    let incompatible = DdsClient::connect(incompatible).await.unwrap();
    let match_result = incompatible.wait_for_match().await;
    assert!(
        matches!(match_result, Err(Error::Timeout)),
        "same topic with an incompatible field type must not match: {match_result:?}"
    );
    incompatible.close().await.unwrap();
    exchange(&publisher, &subscriber, &mut peer).await;
}

#[tokio::test(flavor = "multi_thread")]
#[ignore = "requires local DDS sockets"]
async fn timeout_close_and_concurrent_disconnect_release_participants() {
    let mut settings = config(
        175,
        "FlowLikeIdle",
        DdsRole::Subscriber,
        DdsReliability::Reliable,
    );
    settings.timeout_ms = 200;
    let client = Arc::new(DdsClient::connect(settings).await.unwrap());
    assert!(matches!(client.receive().await, Err(Error::Timeout)));
    assert!(client.publish(json!({})).await.is_err());
    let receiving = {
        let client = client.clone();
        tokio::spawn(async move { client.receive().await })
    };
    tokio::task::yield_now().await;
    let started = Instant::now();
    let (first, second) = tokio::join!(client.close(), client.close());
    first.unwrap();
    second.unwrap();
    assert!(started.elapsed() < Duration::from_secs(2));
    assert!(receiving.await.unwrap().is_err());
    assert!(client.receive().await.is_err());
    assert!(client.wait_for_match().await.is_err());
    assert!(dust_dds::dds_async::domain_participant_factory::DomainParticipantFactoryAsync::get_instance()
        .lookup_participant(175).await.unwrap().is_none(), "both close callers must wait for actual participant deletion");
}

#[tokio::test(flavor = "multi_thread")]
#[ignore = "requires local DDS sockets"]
async fn cancellation_of_close_waiter_and_drop_still_delete_participants() {
    for domain in [176, 177] {
        let client = DdsClient::connect(config(
            domain,
            "FlowLikeCleanup",
            DdsRole::Subscriber,
            DdsReliability::Reliable,
        ))
        .await
        .unwrap();
        if domain == 176 {
            let mut closing = Box::pin(client.close());
            tokio::select! {
                _ = &mut closing => {},
                _ = tokio::task::yield_now() => {},
            }
            drop(closing);
        }
        drop(client);
        tokio::time::timeout(Duration::from_secs(3), async {
            loop {
                let found = dust_dds::dds_async::domain_participant_factory::DomainParticipantFactoryAsync::get_instance()
                    .lookup_participant(domain).await.unwrap();
                if found.is_none() { break; }
                tokio::time::sleep(Duration::from_millis(10)).await;
            }
        }).await.expect("dropped DDS client leaked its domain participant");
    }
}

fn security_directory() -> std::path::PathBuf {
    std::env::var_os("FLOW_LIKE_DDS_SECURITY_DIR")
        .expect("set FLOW_LIKE_DDS_SECURITY_DIR to generated E2E test credentials")
        .into()
}
fn security_file(name: &str) -> String {
    std::fs::read_to_string(security_directory().join(name))
        .expect("read generated DDS test credential")
}
fn security() -> flow_like_industrial::dds::DdsSecurity {
    flow_like_industrial::dds::DdsSecurity {
        identity_ca_pem: security_file("ca.pem"),
        identity_certificate_pem: security_file("flow-like.pem"),
        private_key_pem: security_file("flow-like.key"),
        private_key_password: String::new(),
        permissions_ca_pem: security_file("ca.pem"),
        governance_signed: security_file("governance.p7s"),
        permissions_signed: security_file("flow-like-permissions.p7s"),
    }
}
fn secure_config(domain: i32, topic: &str, role: DdsRole) -> DdsConfig {
    DdsConfig {
        security: Some(security()),
        ..config(domain, topic, role, DdsReliability::Reliable)
    }
}
impl Reference {
    fn secure(domain: i32, prefix: &str) -> Self {
        Self::secure_with_directory(domain, prefix, security_directory())
    }
    fn secure_with_directory(domain: i32, prefix: &str, directory: std::path::PathBuf) -> Self {
        let binary =
            std::env::var_os("FLOW_LIKE_DDS_REFERENCE").expect("set FLOW_LIKE_DDS_REFERENCE");
        Self(
            Command::new(binary)
                .args([&domain.to_string(), prefix, "reliable"])
                .arg(directory)
                .stdin(Stdio::null())
                .stdout(Stdio::inherit())
                .stderr(Stdio::inherit())
                .spawn()
                .expect("start independent secure Cyclone DDS reference"),
        )
    }
}

#[tokio::test]
#[ignore = "requires Cyclone DDS 11 security plugins, OpenSSL 3, and independent C peer"]
async fn secure_pkidh_encrypted_typed_exchange_with_independent_c_peer() {
    let mut peer = Reference::secure(178, "secure_exchange");
    let publisher = DdsClient::connect(secure_config(
        178,
        "secure_exchangeInput",
        DdsRole::Publisher,
    ))
    .await
    .unwrap();
    let subscriber = DdsClient::connect(secure_config(
        178,
        "secure_exchangeOutput",
        DdsRole::Subscriber,
    ))
    .await
    .unwrap();
    exchange(&publisher, &subscriber, &mut peer).await;
}

#[tokio::test]
#[ignore = "requires Cyclone DDS 11 security runtime and generated credentials"]
async fn secure_backend_rejects_untrusted_identity_and_signed_policy_failures() {
    let mut wrong_ca = secure_config(179, "secure_identity", DdsRole::Publisher);
    wrong_ca.security.as_mut().unwrap().identity_ca_pem = security_file("wrong-ca.pem");
    assert!(DdsClient::connect(wrong_ca).await.is_err());
    let mut untrusted = secure_config(183, "secure_remote_trust", DdsRole::Publisher);
    untrusted.timeout_ms = 700;
    untrusted.security.as_mut().unwrap().identity_ca_pem = security_file("wrong-ca.pem");
    untrusted
        .security
        .as_mut()
        .unwrap()
        .identity_certificate_pem = security_file("untrusted.pem");
    let trusted = DdsClient::connect(DdsConfig {
        timeout_ms: 700,
        ..secure_config(183, "secure_remote_trustOutput", DdsRole::Subscriber)
    })
    .await
    .unwrap();
    // Different native local trust policies must not use Cyclone's in-process matching shortcut.
    assert!(DdsClient::connect(untrusted).await.is_err());
    let _remote = Reference::secure_with_directory(
        183,
        "secure_remote_trust",
        security_directory().join("untrusted"),
    );
    assert!(matches!(
        trusted.wait_for_match().await,
        Err(Error::Timeout)
    ));
    assert!(matches!(trusted.receive().await, Err(Error::Timeout)));
    trusted.close().await.unwrap();
    let mut wrong_permissions_ca = secure_config(179, "secure_permissions_ca", DdsRole::Publisher);
    wrong_permissions_ca
        .security
        .as_mut()
        .unwrap()
        .permissions_ca_pem = security_file("wrong-ca.pem");
    assert!(DdsClient::connect(wrong_permissions_ca).await.is_err());
    let mut denied = secure_config(179, "secure_denied", DdsRole::Publisher);
    denied.security.as_mut().unwrap().permissions_signed = security_file("denied-permissions.p7s");
    assert!(DdsClient::connect(denied).await.is_err());
    let mut modified = secure_config(179, "secure_modified", DdsRole::Publisher);
    modified.security.as_mut().unwrap().permissions_signed =
        security_file("flow-like-permissions.p7s").replace("secure_*", "*");
    assert!(DdsClient::connect(modified).await.is_err());
    for name in ["governance-none.p7s", "governance-sign.p7s"] {
        let mut unencrypted = secure_config(179, "secure_unencrypted", DdsRole::Publisher);
        unencrypted.security.as_mut().unwrap().governance_signed = security_file(name);
        assert!(DdsClient::connect(unencrypted).await.is_err());
    }
}

#[tokio::test]
#[ignore = "requires Cyclone DDS 11 security runtime and generated credentials"]
async fn secure_backend_never_matches_or_falls_back_to_plaintext_peers() {
    let secure_publisher = DdsClient::connect(DdsConfig {
        timeout_ms: 600,
        ..secure_config(180, "secure_no_plaintext", DdsRole::Publisher)
    })
    .await
    .unwrap();
    let plain_subscriber = DdsClient::connect(DdsConfig {
        timeout_ms: 600,
        ..config(
            180,
            "secure_no_plaintext",
            DdsRole::Subscriber,
            DdsReliability::Reliable,
        )
    })
    .await
    .unwrap();
    assert!(matches!(
        secure_publisher.wait_for_match().await,
        Err(Error::Timeout)
    ));
    assert!(matches!(
        plain_subscriber.receive().await,
        Err(Error::Timeout)
    ));
    let secure_subscriber = Arc::new(
        DdsClient::connect(DdsConfig {
            timeout_ms: 600,
            ..secure_config(181, "secure_no_plaintext", DdsRole::Subscriber)
        })
        .await
        .unwrap(),
    );
    let plain_publisher = DdsClient::connect(DdsConfig {
        timeout_ms: 600,
        ..config(
            181,
            "secure_no_plaintext",
            DdsRole::Publisher,
            DdsReliability::Reliable,
        )
    })
    .await
    .unwrap();
    assert!(matches!(
        plain_publisher.wait_for_match().await,
        Err(Error::Timeout)
    ));
    assert!(matches!(
        secure_subscriber.receive().await,
        Err(Error::Timeout)
    ));
    let receiving = secure_subscriber.clone();
    let waiter = tokio::spawn(async move { receiving.receive().await });
    tokio::task::yield_now().await;
    let (first, second) = tokio::join!(secure_subscriber.close(), secure_subscriber.close());
    first.unwrap();
    second.unwrap();
    assert!(waiter.await.unwrap().is_err());
    secure_publisher.close().await.unwrap();
    plain_subscriber.close().await.unwrap();
    plain_publisher.close().await.unwrap();
}

#[tokio::test]
#[ignore = "requires Cyclone DDS 11 security runtime and generated credentials"]
async fn secure_backend_errors_do_not_print_private_keys_or_passwords() {
    const CHILD: &str = "FLOW_LIKE_DDS_REDACTION_CHILD";
    const MARKER: &str = "PRIVATE_DDS_SECRET_MUST_NOT_APPEAR";
    if std::env::var_os(CHILD).is_some() {
        let mut invalid = secure_config(182, "secure_bad_key", DdsRole::Publisher);
        invalid.security.as_mut().unwrap().private_key_pem =
            format!("-----BEGIN PRIVATE KEY-----\n{MARKER}\n-----END PRIVATE KEY-----");
        invalid.security.as_mut().unwrap().private_key_password = MARKER.into();
        assert!(!format!("{invalid:?}").contains(MARKER));
        assert!(DdsClient::connect(invalid).await.is_err());
        let mut valid = secure_config(182, "secure_trace_key", DdsRole::Publisher);
        // An unencrypted key accepts a password, letting the trace exercise both properties.
        valid.security.as_mut().unwrap().private_key_password = MARKER.into();
        let client = DdsClient::connect(valid).await.unwrap();
        client.close().await.unwrap();
        return;
    }
    let output = Command::new(std::env::current_exe().unwrap())
        .args(["--ignored", "--exact", "secure_backend_errors_do_not_print_private_keys_or_passwords", "--nocapture"])
        .env(CHILD, "1")
        .env("CYCLONEDDS_URI", "<CycloneDDS><Domain><Tracing><Verbosity>finest</Verbosity><OutputFile>stderr</OutputFile></Tracing></Domain></CycloneDDS>")
        .output().unwrap();
    assert!(output.status.success());
    for bytes in [output.stdout, output.stderr] {
        let text = String::from_utf8_lossy(&bytes);
        assert!(
            !text.contains(MARKER),
            "native DDS diagnostics exposed secret material"
        );
        assert!(
            !text.contains("BEGIN PRIVATE KEY"),
            "native DDS diagnostics exposed a private key"
        );
    }
}
