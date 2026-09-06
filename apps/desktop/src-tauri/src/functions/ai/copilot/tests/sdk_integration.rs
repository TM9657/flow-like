use super::*;

#[tokio::test]
async fn copilot_sdk_client_starts_and_stops() {
    let Some(client) = build_test_client() else {
        return;
    };

    let start_result = client.start().await;

    if let Err(ref e) = start_result {
        let err_str = format!("{:?}", e);
        if err_str.contains("ProtocolMismatch") {
            panic!(
                "COPILOT SDK PROTOCOL MISMATCH: The copilot-sdk Rust crate (protocol v{}) \
                     is incompatible with the installed Copilot CLI (protocol v3). \
                     Update the copilot-sdk dependency in Cargo.toml to a version supporting \
                     protocol v3. Error: {}",
                copilot_sdk::SDK_PROTOCOL_VERSION,
                err_str
            );
        }
        panic!("client.start() failed: {:?}", e);
    }

    let stop_errors = client.stop().await;
    assert!(
        stop_errors.is_empty(),
        "client.stop() had errors: {:?}",
        stop_errors
    );
}

#[tokio::test]
async fn copilot_sdk_auth_status() {
    let Some(client) = start_test_client().await else {
        return;
    };

    let auth = client.get_auth_status().await;
    assert!(auth.is_ok(), "get_auth_status() failed: {:?}", auth.err());

    let status = auth.unwrap();
    println!(
        "Auth status: authenticated={}, login={:?}",
        status.is_authenticated, status.login
    );
    assert!(
        status.is_authenticated,
        "Copilot is not authenticated. Run `copilot auth login` first."
    );

    let _ = client.stop().await;
}

#[tokio::test]
async fn copilot_sdk_list_models() {
    let Some(client) = start_test_client().await else {
        return;
    };

    let models = client.list_models().await;
    assert!(models.is_ok(), "list_models() failed: {:?}", models.err());

    let models = models.unwrap();
    println!("Available models ({}):", models.len());
    for m in &models {
        println!("  - {} ({})", m.name, m.id);
    }
    assert!(
        !models.is_empty(),
        "No models returned from Copilot SDK — check subscription/auth"
    );

    let _ = client.stop().await;
}

#[tokio::test]
async fn copilot_sdk_create_session_and_chat() {
    let Some(client) = start_test_client().await else {
        return;
    };

    let config = copilot_sdk::SessionConfig {
        streaming: true,
        ..Default::default()
    };

    let session = client.create_session(config).await;
    assert!(
        session.is_ok(),
        "create_session() failed: {:?}",
        session.err()
    );
    let session = session.unwrap();

    let mut events = session.subscribe();
    let send_result = session.send("Reply with only the word 'pong'").await;
    assert!(
        send_result.is_ok(),
        "session.send() failed: {:?}",
        send_result.err()
    );

    let mut got_response = false;
    let mut full_response = String::new();
    let timeout = tokio::time::timeout(std::time::Duration::from_secs(30), async {
        loop {
            match events.recv().await {
                Ok(event) => match &event.data {
                    copilot_sdk::SessionEventData::AssistantMessageDelta(delta) => {
                        full_response.push_str(&delta.delta_content);
                    }
                    copilot_sdk::SessionEventData::AssistantMessage(msg) => {
                        if full_response.is_empty() {
                            full_response = msg.content.clone();
                        }
                        got_response = true;
                    }
                    copilot_sdk::SessionEventData::SessionIdle(_) => break,
                    copilot_sdk::SessionEventData::SessionError(err) => {
                        panic!("Session error: {:?}", err);
                    }
                    _ => {}
                },
                Err(e) => {
                    panic!("Event receive error: {}", e);
                }
            }
        }
    })
    .await;

    assert!(timeout.is_ok(), "Chat timed out after 30s");
    assert!(
        !full_response.is_empty(),
        "Got empty response from Copilot session"
    );
    println!("Chat response: {}", full_response);

    let _ = client.stop().await;
}
