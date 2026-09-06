use super::*;
use serde_json::json;

fn reply(value: flow_like_types::Value) -> ChannelPush {
    ChannelPush {
        channel_id: "run-1".into(),
        request_id: Some("req-1".into()),
        kind: ChannelPushKind::Reply,
        value,
    }
}

#[test]
fn exact_payload_limit_includes_channel_and_request_envelope() {
    let overhead = serde_json::to_vec(&reply(json!(""))).unwrap().len();
    let exact = reply(json!("x".repeat(MAX_PAYLOAD_BYTES - overhead)));
    assert_eq!(reply_payloads(&exact).unwrap().len(), 1);
    let above = reply(json!("x".repeat(MAX_PAYLOAD_BYTES - overhead + 1)));
    let payloads = reply_payloads(&above).unwrap();
    assert_eq!(payloads.len(), 3);
    assert!(
        payloads
            .iter()
            .all(|payload| payload.len() <= MAX_PAYLOAD_BYTES)
    );
}

#[test]
fn maximum_home_layout_with_three_comparisons_round_trips_utf8() {
    let layout = json!({ "version": 1, "widgets": [{
        "id": "info", "type": "information", "size": { "columns": 12, "rows": 3 },
        "appearance": { "variant": "card", "accent": "neutral" }, "config": { "body": "" }
    }] });
    let overhead = serde_json::to_vec(&layout).unwrap().len();
    let remaining = 128 * 1024 - overhead;
    let mut layout = layout;
    layout["widgets"][0]["config"]["body"] = json!(format!(
        "{}{}",
        "😊".repeat(remaining / 4),
        "x".repeat(remaining % 4)
    ));
    assert_eq!(serde_json::to_vec(&layout).unwrap().len(), 128 * 1024);
    let original = reply(json!({ "approved": true, "result": {
        "layout": layout, "base_layout": layout, "default_layout": layout, "comparison_layout": layout,
    } }));
    let payloads = reply_payloads(&original).unwrap();
    let frames: Vec<ReplyChunk> = payloads
        .iter()
        .map(|payload| {
            assert!(payload.len() <= MAX_PAYLOAD_BYTES);
            // The transport extension fails closed on receivers that only know ChannelPush.
            assert!(serde_json::from_slice::<ChannelPush>(payload).is_err());
            serde_json::from_slice(payload).unwrap()
        })
        .collect();
    let mut assembly = ReplyAssembly::new(&frames[0], Instant::now()).unwrap();
    for (index, frame) in frames.iter().enumerate() {
        match assembly.append(frame, Instant::now()).unwrap() {
            AppendResult::Pending => assert!(index < frames.len() - 1),
            AppendResult::Complete(push) => assert_eq!(push, original),
            AppendResult::Duplicate => panic!("No duplicate was sent"),
        }
    }
}

#[test]
fn oversized_or_nonreply_messages_return_bounded_errors() {
    let oversized = reply(json!("x".repeat(MAX_REPLY_BYTES)));
    let error = reply_payloads(&oversized).unwrap_err().to_string();
    assert!(error.contains("2 MiB"));
    assert!(error.len() < 200);
    let mut inbound = reply(json!("x".repeat(MAX_PAYLOAD_BYTES)));
    inbound.kind = ChannelPushKind::Inbound;
    assert!(reply_payloads(&inbound).is_err());
    inbound.kind = ChannelPushKind::Reply;
    inbound.request_id = None;
    assert!(reply_payloads(&inbound).is_err());
    let mut long_id = reply(json!("x".repeat(MAX_PAYLOAD_BYTES)));
    long_id.channel_id = "x".repeat(50_000);
    assert!(
        reply_payloads(&long_id)
            .unwrap_err()
            .to_string()
            .contains("envelope")
    );
}

#[test]
fn metadata_count_bytes_and_frame_lengths_are_bounded_before_buffering() {
    let payloads = reply_payloads(&reply(json!("x".repeat(MAX_PAYLOAD_BYTES)))).unwrap();
    let first: ReplyChunk = serde_json::from_slice(&payloads[0]).unwrap();
    for (total, total_bytes) in [
        (MAX_CHUNKS + 1, MAX_REPLY_BYTES),
        (1, MAX_REPLY_BYTES + 1),
        (0, 0),
    ] {
        let bad = ReplyChunk {
            total,
            total_bytes,
            ..first.clone()
        };
        assert!(ReplyAssembly::new(&bad, Instant::now()).is_err());
    }
    let mut assembly = ReplyAssembly::new(&first, Instant::now()).unwrap();
    assert!(
        assembly
            .append(
                &ReplyChunk {
                    data: "x".repeat(MAX_PAYLOAD_BYTES),
                    ..first.clone()
                },
                Instant::now()
            )
            .is_err()
    );
    assert!(
        assembly
            .append(
                &ReplyChunk {
                    data: STANDARD.encode(b"too short"),
                    ..first
                },
                Instant::now()
            )
            .is_err()
    );
    assert!(assembly.bytes.is_empty());
}

#[test]
fn a_chunk_cannot_deliver_cancel_or_a_reply_for_another_request() {
    for (kind, request_id, channel_id) in [
        (ChannelPushKind::Cancel, "req-1", "run-1"),
        (ChannelPushKind::Inbound, "req-1", "run-1"),
        (ChannelPushKind::Reply, "other-request", "run-1"),
        (ChannelPushKind::Reply, "req-1", "other-channel"),
    ] {
        let body = serde_json::to_vec(&ChannelPush {
            kind,
            request_id: Some(request_id.into()),
            channel_id: channel_id.into(),
            value: json!(true),
        })
        .unwrap();
        let frame = ReplyChunk {
            channel_id: "run-1".into(),
            request_id: "req-1".into(),
            kind: ChunkKind::ReplyChunk,
            transfer_id: "transfer-1".into(),
            index: 0,
            total: 1,
            total_bytes: body.len(),
            data: STANDARD.encode(body),
        };
        let mut assembly = ReplyAssembly::new(&frame, Instant::now()).unwrap();
        assert!(assembly.append(&frame, Instant::now()).is_err());
    }
}
