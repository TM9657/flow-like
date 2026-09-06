use super::*;
use crate::aws::reply_chunks::{TRANSFER_IDLE_TIMEOUT, reply_payloads};
use flow_like_types::base64::{Engine, engine::general_purpose::STANDARD};

fn frames(request_id: &str) -> Vec<ReplyChunk> {
    reply_payloads(&ChannelPush {
        channel_id: "run-1".into(),
        request_id: Some(request_id.into()),
        kind: ChannelPushKind::Reply,
        value: Value::String("😊".repeat(MAX_PAYLOAD_BYTES)),
    })
    .unwrap()
    .iter()
    .map(|payload| serde_json::from_slice(payload).unwrap())
    .collect()
}

#[test]
fn chunks_deliver_only_once_after_the_complete_reply_arrives() {
    let router = PushRouter::new("run-1");
    router.register("req-1");
    let mut receiver = router.take_receiver("req-1").unwrap();
    let frames = frames("req-1");
    for (index, frame) in frames.iter().enumerate() {
        let payload = serde_json::to_vec(frame).unwrap();
        let result = router.route_payload(&payload);
        if index < frames.len() - 1 {
            assert_eq!(result, RouteResult::ChunkBuffered);
            assert!(receiver.try_recv().is_err());
        } else {
            assert_eq!(result, RouteResult::Reply);
            assert_eq!(
                receiver.try_recv().unwrap(),
                Value::String("😊".repeat(MAX_PAYLOAD_BYTES))
            );
        }
        assert_eq!(router.route_payload(&payload), RouteResult::Duplicate);
    }
    assert!(lock(&router.pending)["req-1"].chunks.is_none());
}

#[test]
fn out_of_order_and_conflicting_duplicate_chunks_are_rejected() {
    let router = PushRouter::new("run-1");
    router.register("req-1");
    let frames = frames("req-1");
    let now = Instant::now();
    assert_eq!(
        router.route_chunk(frames[1].clone(), now),
        RouteResult::Malformed
    );
    assert_eq!(
        router.route_chunk(frames[0].clone(), now),
        RouteResult::ChunkBuffered
    );
    assert_eq!(
        router.route_chunk(frames[2].clone(), now),
        RouteResult::Malformed
    );
    let mut conflicting = frames[0].clone();
    let mut bytes = STANDARD.decode(&conflicting.data).unwrap();
    bytes[0] ^= 1;
    conflicting.data = STANDARD.encode(bytes);
    assert_eq!(router.route_chunk(conflicting, now), RouteResult::Malformed);
    assert_eq!(
        router.route_chunk(frames[1].clone(), now),
        RouteResult::ChunkBuffered
    );
}

#[test]
fn fallback_can_restart_a_partial_transfer_without_delivering_fragments() {
    let router = PushRouter::new("run-1");
    router.register("req-1");
    let direct = frames("req-1");
    let fallback = frames("req-1");
    let now = Instant::now();
    assert_ne!(direct[0].transfer_id, fallback[0].transfer_id);
    assert_eq!(
        router.route_chunk(direct[0].clone(), now),
        RouteResult::ChunkBuffered
    );
    assert_eq!(
        router.route_chunk(fallback[0].clone(), now),
        RouteResult::ChunkBuffered
    );
    assert_eq!(
        router.route_chunk(direct[1].clone(), now),
        RouteResult::Malformed
    );
    assert_eq!(
        router.route_chunk(direct[0].clone(), now),
        RouteResult::Malformed
    );
    for frame in fallback.iter().skip(1) {
        router.route_chunk(frame.clone(), now);
    }
    assert_eq!(
        router.take_receiver("req-1").unwrap().try_recv().unwrap(),
        Value::String("😊".repeat(MAX_PAYLOAD_BYTES))
    );
}

#[test]
fn assembly_count_expiration_removal_and_cancel_bound_retained_bytes() {
    let router = PushRouter::new("run-1");
    let now = Instant::now();
    for index in 0..MAX_ACTIVE_TRANSFERS {
        let request_id = format!("req-{index}");
        router.register(&request_id);
        assert_eq!(
            router.route_chunk(frames(&request_id)[0].clone(), now),
            RouteResult::ChunkBuffered
        );
    }
    router.register("overflow");
    let overflow = frames("overflow");
    assert_eq!(
        router.route_chunk(overflow[0].clone(), now),
        RouteResult::Malformed
    );
    let expired_at = now + TRANSFER_IDLE_TIMEOUT;
    assert_eq!(
        router.route_chunk(frames("req-0")[1].clone(), expired_at),
        RouteResult::Malformed
    );
    assert_eq!(
        router.route_chunk(overflow[0].clone(), expired_at),
        RouteResult::ChunkBuffered
    );
    assert_eq!(
        lock(&router.pending)
            .values()
            .filter(|slot| slot.chunks.is_some())
            .count(),
        1
    );
    router.remove("overflow");
    assert!(
        lock(&router.pending)
            .values()
            .all(|slot| slot.chunks.is_none())
    );
    let first = frames("req-0")[0].clone();
    assert_eq!(
        router.route_chunk(first.clone(), expired_at),
        RouteResult::ChunkBuffered
    );
    router.route(ChannelPush {
        channel_id: "run-1".into(),
        request_id: None,
        kind: ChannelPushKind::Cancel,
        value: Value::Null,
    });
    assert!(
        lock(&router.pending)
            .values()
            .all(|slot| slot.chunks.is_none())
    );
    assert_eq!(
        router.route_chunk(first, expired_at),
        RouteResult::UnknownRequest
    );
}

#[test]
fn unknown_requests_foreign_channels_and_oversized_wire_messages_are_rejected() {
    let router = PushRouter::new("run-1");
    let first = frames("req-1")[0].clone();
    assert_eq!(
        router.route_chunk(first.clone(), Instant::now()),
        RouteResult::UnknownRequest
    );
    router.register("req-1");
    assert_eq!(
        router.route_chunk(
            ReplyChunk {
                channel_id: "foreign".into(),
                ..first
            },
            Instant::now()
        ),
        RouteResult::ForeignChannel
    );
    assert_eq!(
        router.route_payload(&vec![b' '; MAX_PAYLOAD_BYTES + 1]),
        RouteResult::Malformed
    );
    assert!(lock(&router.pending)["req-1"].chunks.is_none());
}

#[test]
fn expired_transfer_ids_cannot_restart_after_their_time_budget() {
    let router = PushRouter::new("run-1");
    router.register("req-1");
    let first = frames("req-1")[0].clone();
    let now = Instant::now();
    assert_eq!(
        router.route_chunk(first.clone(), now),
        RouteResult::ChunkBuffered
    );
    assert_eq!(
        router.route_chunk(first, now + TRANSFER_IDLE_TIMEOUT),
        RouteResult::Malformed
    );
    assert!(lock(&router.pending)["req-1"].chunks.is_none());
    assert_eq!(
        router.route_chunk(frames("req-1")[0].clone(), now + TRANSFER_IDLE_TIMEOUT),
        RouteResult::ChunkBuffered
    );
}

#[test]
fn malformed_replacement_frames_preserve_a_healthy_partial_transfer() {
    for data in ["not base64!".to_owned(), STANDARD.encode(b"too short")] {
        let router = PushRouter::new("run-1");
        router.register("req-1");
        let healthy = frames("req-1");
        let mut replacement = frames("req-1")[0].clone();
        replacement.data = data;
        let now = Instant::now();
        assert_eq!(
            router.route_chunk(healthy[0].clone(), now),
            RouteResult::ChunkBuffered
        );
        assert_eq!(router.route_chunk(replacement, now), RouteResult::Malformed);
        assert_eq!(
            lock(&router.pending)["req-1"]
                .chunks
                .as_ref()
                .unwrap()
                .transfer_id,
            healthy[0].transfer_id
        );
        for frame in healthy.iter().skip(1) {
            router.route_chunk(frame.clone(), now);
        }
        assert_eq!(
            router.take_receiver("req-1").unwrap().try_recv().unwrap(),
            Value::String("😊".repeat(MAX_PAYLOAD_BYTES))
        );
    }
}

#[test]
fn slow_transfers_remain_live_while_new_valid_chunks_make_progress() {
    let router = PushRouter::new("run-1");
    router.register("req-1");
    let frames = frames("req-1");
    let now = Instant::now();
    let interval = TRANSFER_IDLE_TIMEOUT / 4;
    assert!(interval * (frames.len() as u32 - 1) > TRANSFER_IDLE_TIMEOUT);
    for (index, frame) in frames.iter().enumerate() {
        assert_eq!(
            router.route_chunk(frame.clone(), now + interval * index as u32),
            if index == frames.len() - 1 {
                RouteResult::Reply
            } else {
                RouteResult::ChunkBuffered
            }
        );
    }
    assert_eq!(
        router.take_receiver("req-1").unwrap().try_recv().unwrap(),
        Value::String("😊".repeat(MAX_PAYLOAD_BYTES))
    );
}

#[test]
fn duplicates_and_rejected_chunks_do_not_extend_the_inactivity_deadline() {
    for duplicate in [true, false] {
        let router = PushRouter::new("run-1");
        router.register("req-1");
        let frames = frames("req-1");
        let now = Instant::now();
        assert_eq!(
            router.route_chunk(frames[0].clone(), now),
            RouteResult::ChunkBuffered
        );
        let before_expiry = now + TRANSFER_IDLE_TIMEOUT - std::time::Duration::from_secs(1);
        let repeated = if duplicate {
            frames[0].clone()
        } else {
            frames[2].clone()
        };
        assert_eq!(
            router.route_chunk(repeated, before_expiry),
            if duplicate {
                RouteResult::Duplicate
            } else {
                RouteResult::Malformed
            }
        );
        assert_eq!(
            router.route_chunk(frames[1].clone(), now + TRANSFER_IDLE_TIMEOUT),
            RouteResult::Malformed
        );
        assert!(lock(&router.pending)["req-1"].chunks.is_none());
    }
}
