use super::*;
use flow_like_types::json::json;
use rmcp::{
    ServiceExt,
    model::{CallToolRequestParams, ClientCapabilities, ClientInfo, Implementation},
};
use std::sync::{
    Mutex,
    atomic::{AtomicBool, Ordering},
};

#[tokio::test]
async fn discovers_and_calls_the_editors_original_tool_schema() {
    let messages = Arc::new(Mutex::new(Vec::<Value>::new()));
    let observed = messages.clone();
    let schema = json!({"type":"object","properties":{"radius":{"type":"number","minimum":0}},"required":["radius"]});
    let expected_schema = schema.clone();
    let requester: Requester = Arc::new(move |message| {
        observed.lock().unwrap().push(message.clone());
        let schema = schema.clone();
        Box::pin(async move {
            let result = match message["method"].as_str().unwrap() {
                "initialize" => json!({
                    "protocolVersion": message["params"]["protocolVersion"],
                    "serverInfo":{"name":"upstream-editor","version":"1.0"},
                    "capabilities":{"tools":{}}
                }),
                "notifications/initialized" => return Ok(Value::Null),
                "tools/list" => {
                    json!({"tools":[{"name":"shape_set_radius","description":"Set a radius","inputSchema":schema}]})
                }
                "tools/call" => {
                    assert_eq!(message["params"]["name"], "shape_set_radius");
                    assert_eq!(message["params"]["arguments"], json!({"radius":7}));
                    json!({"content":[{"type":"text","text":"radius=7"}],"isError":false})
                }
                other => panic!("Unexpected MCP request {other}"),
            };
            Ok(json!({"jsonrpc":"2.0","id":message["id"],"result":result}))
        })
    });
    let transport = transport(requester, CancellationToken::new());
    let client = tokio::time::timeout(
        Duration::from_secs(2),
        ClientInfo::new(
            ClientCapabilities::default(),
            Implementation::new("test", "1"),
        )
        .serve(transport),
    )
    .await
    .unwrap()
    .unwrap();
    let tools = client.list_tools(None).await.unwrap().tools;
    assert_eq!(tools.len(), 1);
    assert_eq!(tools[0].name, "shape_set_radius");
    assert_eq!(
        json::to_value(&tools[0].input_schema).unwrap(),
        expected_schema
    );
    let mut params = CallToolRequestParams::new(tools[0].name.to_string());
    params.arguments = Some(json!({"radius":7}).as_object().unwrap().clone());
    let result = client.call_tool(params).await.unwrap();
    assert_eq!(
        json::to_value(result).unwrap()["content"][0]["text"],
        "radius=7"
    );
    assert_eq!(messages.lock().unwrap().len(), 4);
    client.cancel().await.unwrap();
}

#[tokio::test]
async fn sends_do_not_wait_for_frontend_and_close_cancels_pending_work() {
    struct Pending(Arc<AtomicBool>);
    impl Drop for Pending {
        fn drop(&mut self) {
            self.0.store(true, Ordering::SeqCst);
        }
    }
    let dropped = Arc::new(AtomicBool::new(false));
    let started = Arc::new(tokio::sync::Notify::new());
    let requester: Requester = Arc::new({
        let dropped = dropped.clone();
        let started = started.clone();
        move |_| {
            let pending = Pending(dropped.clone());
            let started = started.clone();
            Box::pin(async move {
                let _pending = pending;
                started.notify_one();
                std::future::pending().await
            })
        }
    });
    let mut transport = transport(requester, CancellationToken::new());
    let request = json::from_value(json!({"jsonrpc":"2.0","id":1,"method":"ping"})).unwrap();
    tokio::time::timeout(Duration::from_secs(1), transport.send(request))
        .await
        .unwrap()
        .unwrap();
    started.notified().await;
    tokio::time::timeout(Duration::from_secs(1), transport.close())
        .await
        .unwrap()
        .unwrap();
    assert!(dropped.load(Ordering::SeqCst));
    assert!(transport.receive().await.is_none());
    assert!(
        transport
            .send(json::from_value(json!({"jsonrpc":"2.0","id":2,"method":"ping"})).unwrap())
            .await
            .is_err()
    );
}

#[tokio::test]
async fn frontend_failures_and_mismatched_responses_keep_the_request_id() {
    for wrong_response in [false, true] {
        let requester: Requester = Arc::new(move |_| {
            Box::pin(async move {
                if wrong_response {
                    Ok(json!({"jsonrpc":"2.0","id":"another-request","result":{}}))
                } else {
                    Err(flow_like_types::anyhow!("The document was closed"))
                }
            })
        });
        let mut transport = transport(requester, CancellationToken::new());
        transport
            .send(json::from_value(json!({"jsonrpc":"2.0","id":17,"method":"ping"})).unwrap())
            .await
            .unwrap();
        let response = tokio::time::timeout(Duration::from_secs(1), transport.receive())
            .await
            .unwrap()
            .unwrap();
        let response = json::to_value(response).unwrap();
        assert_eq!(response["id"], 17);
        assert_eq!(response["error"]["code"], -32000);
        assert!(
            response["error"]["message"]
                .as_str()
                .unwrap()
                .contains(if wrong_response {
                    "different request"
                } else {
                    "document was closed"
                })
        );
        transport.close().await.unwrap();
    }
}

#[tokio::test]
async fn parent_run_cancellation_closes_the_editor_peer() {
    let cancellation = CancellationToken::new();
    let requester: Requester = Arc::new(|_| Box::pin(std::future::pending()));
    let mut transport = transport(requester, cancellation.child_token());
    cancellation.cancel();
    assert!(
        tokio::time::timeout(Duration::from_secs(1), transport.receive())
            .await
            .unwrap()
            .is_none()
    );
    transport.close().await.unwrap();
}

#[tokio::test]
async fn dropping_a_peer_cancels_its_pending_request() {
    struct Pending(Option<tokio::sync::oneshot::Sender<()>>);
    impl Drop for Pending {
        fn drop(&mut self) {
            let _ = self.0.take().unwrap().send(());
        }
    }
    let (dropped, stopped) = tokio::sync::oneshot::channel();
    let dropped = Arc::new(Mutex::new(Some(dropped)));
    let started = Arc::new(tokio::sync::Notify::new());
    let requester: Requester = Arc::new({
        let started = started.clone();
        move |_| {
            let guard = Pending(dropped.lock().unwrap().take());
            let started = started.clone();
            Box::pin(async move {
                let _guard = guard;
                started.notify_one();
                std::future::pending().await
            })
        }
    });
    let mut transport = transport(requester, CancellationToken::new());
    transport
        .send(json::from_value(json!({"jsonrpc":"2.0","id":1,"method":"ping"})).unwrap())
        .await
        .unwrap();
    started.notified().await;
    drop(transport);
    tokio::time::timeout(Duration::from_secs(1), stopped)
        .await
        .unwrap()
        .unwrap();
}

fn transport(requester: Requester, cancellation: CancellationToken) -> EditorMcpTransport {
    EditorMcpTransport::with_requester(
        requester,
        Arc::new(|_| Box::pin(async { Ok(Value::Null) })),
        cancellation,
    )
}

#[tokio::test]
async fn independent_peers_namespace_wire_ids_and_restore_the_callers_id() {
    let messages = Arc::new(Mutex::new(Vec::new()));
    let requester: Requester = Arc::new({
        let messages = messages.clone();
        move |message| {
            messages.lock().unwrap().push(message.clone());
            Box::pin(async move { Ok(json!({"jsonrpc":"2.0", "id":message["id"], "result":{}})) })
        }
    });
    let mut first = transport(requester.clone(), CancellationToken::new());
    let mut second = transport(requester, CancellationToken::new());
    for peer in [&mut first, &mut second] {
        peer.send(json::from_value(json!({"jsonrpc":"2.0","id":1,"method":"ping"})).unwrap())
            .await
            .unwrap();
        assert_eq!(
            json::to_value(peer.receive().await.unwrap()).unwrap()["id"],
            1
        );
    }
    let ids: Vec<_> = messages
        .lock()
        .unwrap()
        .iter()
        .map(|message| message["id"].clone())
        .collect();
    assert!(ids.iter().all(Value::is_string));
    assert_ne!(ids[0], ids[1]);
    first.close().await.unwrap();
    second.close().await.unwrap();
}

#[tokio::test]
async fn cancellation_passes_a_busy_request_and_prevents_a_queued_request_from_running() {
    let started = Arc::new(tokio::sync::Notify::new());
    let release = Arc::new(tokio::sync::Notify::new());
    let (cancel_tx, mut cancel_rx) = mpsc::channel(1);
    let messages = Arc::new(Mutex::new(Vec::new()));
    let requester: Requester = Arc::new({
        let started = started.clone();
        let release = release.clone();
        let messages = messages.clone();
        move |message| {
            messages.lock().unwrap().push(message.clone());
            let started = started.clone();
            let release = release.clone();
            Box::pin(async move {
                started.notify_one();
                release.notified().await;
                Ok(json!({"jsonrpc":"2.0","id":message["id"],"result":{}}))
            })
        }
    });
    let cancellation_requester: Requester = Arc::new(move |message| {
        let cancel_tx = cancel_tx.clone();
        Box::pin(async move {
            cancel_tx.send(message).await.unwrap();
            Ok(Value::Null)
        })
    });
    let mut peer = EditorMcpTransport::with_requester(
        requester,
        cancellation_requester,
        CancellationToken::new(),
    );
    peer.send(json::from_value(json!({"jsonrpc":"2.0","id":1,"method":"ping"})).unwrap())
        .await
        .unwrap();
    started.notified().await;
    peer.send(json::from_value(json!({"jsonrpc":"2.0","id":2,"method":"ping"})).unwrap())
        .await
        .unwrap();
    peer.send(
        json::from_value(
            json!({"jsonrpc":"2.0","method":"notifications/cancelled","params":{"requestId":2}}),
        )
        .unwrap(),
    )
    .await
    .unwrap();
    let cancellation = tokio::time::timeout(Duration::from_secs(1), cancel_rx.recv())
        .await
        .unwrap()
        .unwrap();
    assert_eq!(cancellation["params"]["requestId"], peer.wire_id(&json!(2)));
    release.notify_one();
    assert_eq!(
        json::to_value(peer.receive().await.unwrap()).unwrap()["id"],
        1
    );
    let skipped = json::to_value(peer.receive().await.unwrap()).unwrap();
    assert_eq!(skipped["id"], 2);
    assert!(
        skipped["error"]["message"]
            .as_str()
            .unwrap()
            .contains("cancelled")
    );
    assert_eq!(messages.lock().unwrap().len(), 1);
    peer.close().await.unwrap();
}

#[tokio::test]
async fn close_and_drop_cancel_the_posted_request_with_its_wire_id() {
    for drop_peer in [false, true] {
        let (posted_tx, mut posted_rx) = mpsc::channel(1);
        let (cancel_tx, mut cancel_rx) = mpsc::channel(1);
        let requester: Requester = Arc::new(move |message| {
            let posted_tx = posted_tx.clone();
            Box::pin(async move {
                posted_tx.send(message).await.unwrap();
                std::future::pending().await
            })
        });
        let cancellation_requester: Requester = Arc::new(move |message| {
            let cancel_tx = cancel_tx.clone();
            Box::pin(async move {
                cancel_tx.send(message).await.unwrap();
                Ok(Value::Null)
            })
        });
        let mut peer = EditorMcpTransport::with_requester(
            requester,
            cancellation_requester,
            CancellationToken::new(),
        );
        peer.send(
            json::from_value(json!({"jsonrpc":"2.0","id":"call-1","method":"ping"})).unwrap(),
        )
        .await
        .unwrap();
        let posted = posted_rx.recv().await.unwrap();
        if drop_peer {
            drop(peer);
        } else {
            peer.close().await.unwrap();
        }
        let cancellation = tokio::time::timeout(Duration::from_secs(1), cancel_rx.recv())
            .await
            .unwrap()
            .unwrap();
        assert_eq!(cancellation["method"], "notifications/cancelled");
        assert_eq!(cancellation["params"]["requestId"], posted["id"]);
        assert_ne!(posted["id"], "call-1");
    }
}
