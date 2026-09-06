use super::*;

#[test]
fn flowpilot_mcp_keeps_long_running_sse_calls_alive() {
    let config = flowpilot_mcp_server_config();
    assert_eq!(config.sse_keep_alive, Some(Duration::from_secs(15)));
    assert!(config.stateful_mode);
}

#[test]
fn sdk_handler_watchdog_uses_every_registered_tool_deadline() {
    assert_eq!(
        sdk_tool_handler_watchdog_timeout("unknown_local_tool"),
        SDK_EVENT_INACTIVITY_TIMEOUT
    );
    assert_eq!(
        sdk_tool_handler_watchdog_timeout("flowpilot_board"),
        Duration::from_secs(MAX_DELEGATED_RUN_DISPATCH_SECS) + SDK_CONTROL_RPC_TIMEOUT
    );
    assert_eq!(
        sdk_tool_handler_watchdog_timeout("ask_user"),
        Duration::from_secs(600) + SDK_CONTROL_RPC_TIMEOUT
    );
    assert_eq!(
        sdk_tool_handler_watchdog_timeout("execute_event"),
        Duration::from_secs(600) + SDK_CONTROL_RPC_TIMEOUT
    );
    assert_eq!(
        sdk_tool_handler_watchdog_timeout("ontology_action_tool"),
        Duration::from_secs(600) + SDK_CONTROL_RPC_TIMEOUT
    );
    assert_eq!(
        sdk_tool_handler_watchdog_timeout("ui_inspect"),
        crate::functions::ai::copilot_sdk_tools::UI_INSPECT_TOOL_TIMEOUT + SDK_CONTROL_RPC_TIMEOUT
    );
    let home_timeout = Duration::from_secs(
        flow_like::flow::copilot::tool_spec::find_home_tool_spec("apply_home_layout")
            .expect("shared Home apply spec")
            .timeout_secs,
    );
    assert_eq!(
        sdk_tool_handler_watchdog_timeout("apply_home_layout"),
        SDK_EVENT_INACTIVITY_TIMEOUT.max(home_timeout + SDK_CONTROL_RPC_TIMEOUT)
    );
}

#[test]
fn home_delegation_gets_long_running_progress_handling() {
    assert!(is_delegated_agent_tool("flowpilot_home"));
    assert!(!is_delegated_agent_tool("apply_home_layout"));
}

#[test]
fn sdk_watchdog_observes_a_handler_before_protocol_v3_broadcast() {
    let registry = Arc::new(SdkToolActivityRegistry::default());
    let mut changes = registry.subscribe();
    let last_sdk_event_at = tokio::time::Instant::now();
    assert_eq!(
        registry.inactivity_deadline(last_sdk_event_at),
        last_sdk_event_at + SDK_EVENT_INACTIVITY_TIMEOUT
    );
    assert!(!changes.has_changed().expect("activity channel is open"));

    let ask_guard = registry.begin("ask_user");
    assert!(changes.has_changed().expect("activity channel is open"));
    let _ = *changes.borrow_and_update();
    let ask_deadline = registry.inactivity_deadline(last_sdk_event_at);
    assert!(ask_deadline >= last_sdk_event_at + Duration::from_secs(600) + SDK_CONTROL_RPC_TIMEOUT);

    let board_guard = registry.begin("flowpilot_board");
    assert!(changes.has_changed().expect("activity channel is open"));
    let _ = *changes.borrow_and_update();
    assert_eq!(
        registry.inactivity_deadline(last_sdk_event_at),
        ask_deadline,
        "a longer concurrent handler must not hide the earlier ask_user deadline"
    );
    drop(ask_guard);
    assert!(changes.has_changed().expect("activity channel is open"));
    let _ = *changes.borrow_and_update();
    assert!(
        registry.inactivity_deadline(last_sdk_event_at) > ask_deadline,
        "dropping the shorter lease must expose the board handler deadline"
    );
    drop(board_guard);
    assert!(changes.has_changed().expect("activity channel is open"));
    assert_eq!(*changes.borrow_and_update(), 4);
    assert_eq!(
        registry.inactivity_deadline(last_sdk_event_at),
        last_sdk_event_at + SDK_EVENT_INACTIVITY_TIMEOUT
    );
}

#[test]
fn mcp_tool_heartbeat_uses_the_callers_progress_token_and_no_fake_total() {
    let token = rmcp::model::ProgressToken(rmcp::model::NumberOrString::String(Arc::<str>::from(
        "flowpilot-request",
    )));
    let first = mcp_progress_heartbeat_notification(
        token.clone(),
        1.0,
        "FlowPilot flowpilot_board is still running",
    );
    let second = mcp_progress_heartbeat_notification(
        token.clone(),
        2.0,
        "FlowPilot flowpilot_board is still running",
    );

    assert_eq!(first.progress_token, token);
    assert!(second.progress > first.progress);
    assert_eq!(first.total, None, "heartbeat ticks are not percentages");
    assert_eq!(
        first.message.as_deref(),
        Some("FlowPilot flowpilot_board is still running")
    );
    assert!(MCP_TOOL_PROGRESS_HEARTBEAT_INTERVAL < Duration::from_secs(300));
    assert_eq!(
        serde_json::to_value(first).expect("serialize progress notification")["progressToken"],
        "flowpilot-request"
    );
}

#[test]
fn dropped_mcp_tool_request_cancels_its_blocking_handler() {
    let cancellation = CancellationToken::new();
    {
        let _guard = McpToolCancellationGuard::new(cancellation.clone());
        assert!(!cancellation.is_cancelled());
    }
    assert!(cancellation.is_cancelled());
}

#[test]
fn completed_mcp_tool_request_does_not_emit_cancellation() {
    let cancellation = CancellationToken::new();
    {
        let mut guard = McpToolCancellationGuard::new(cancellation.clone());
        guard.disarm();
    }
    assert!(!cancellation.is_cancelled());
}

#[test]
fn mcp_handler_registry_tracks_workers_until_their_guard_drops() {
    let activity = Arc::new(StdMutex::new(McpToolActivityState::default()));
    let quiescence = Arc::new(tokio::sync::Notify::new());
    let cancellation = CancellationToken::new();
    let guard = register_mcp_active_handler(&activity, &quiescence, cancellation.clone())
        .expect("handler registration");

    {
        let state = activity.lock().expect("handler registry");
        assert_eq!(state.active_handlers.len(), 1);
        state
            .active_handlers
            .values()
            .for_each(CancellationToken::cancel);
    }
    assert!(cancellation.is_cancelled());

    drop(guard);
    assert!(
        activity
            .lock()
            .expect("handler registry")
            .active_handlers
            .is_empty(),
        "a provider continuation may start only after the worker leaves the registry"
    );
}

#[test]
fn dropping_copilot_run_guard_cancels_owned_tool_scope() {
    let (cancellation, guard) = register_copilot_run(None);
    assert!(!cancellation.is_cancelled());
    drop(guard);
    assert!(cancellation.is_cancelled());
}

#[tokio::test]
async fn nested_copilot_gate_waits_for_owner_without_a_queue_deadline() {
    let gate = Arc::new(Semaphore::new(1));
    let owner = gate
        .clone()
        .acquire_owned()
        .await
        .expect("initial nested owner");
    let cancellation = CancellationToken::new();
    let waiter = tokio::spawn(acquire_nested_copilot_run_permit(gate, cancellation));

    tokio::task::yield_now().await;
    assert!(
        !waiter.is_finished(),
        "the queued specialist must remain pending while another run owns the CLI"
    );
    drop(owner);

    let permit = tokio::time::timeout(Duration::from_secs(1), waiter)
        .await
        .expect("queued specialist should acquire promptly after release")
        .expect("waiter task")
        .expect("nested permit");
    drop(permit);
}

#[tokio::test]
async fn nested_copilot_gate_wait_is_explicitly_cancellable() {
    let gate = Arc::new(Semaphore::new(1));
    let _owner = gate
        .clone()
        .acquire_owned()
        .await
        .expect("initial nested owner");
    let cancellation = CancellationToken::new();
    let waiter = tokio::spawn(acquire_nested_copilot_run_permit(
        gate,
        cancellation.clone(),
    ));

    cancellation.cancel();
    let error = tokio::time::timeout(Duration::from_secs(1), waiter)
        .await
        .expect("cancelled waiter should return promptly")
        .expect("waiter task")
        .expect_err("cancelled nested permit");
    assert!(error.contains("cancelled"));
}

#[test]
fn retained_agent_text_is_bounded_and_utf8_safe() {
    let mut retained = String::new();
    assert!(append_bounded_text(&mut retained, "hello", 32));
    assert!(!append_bounded_text(&mut retained, &"🦀".repeat(32), 32));
    assert!(retained.len() <= 32);
    assert!(std::str::from_utf8(retained.as_bytes()).is_ok());
    let capped = retained.clone();
    assert!(!append_bounded_text(&mut retained, "ignored", 32));
    assert_eq!(retained, capped);
}

#[test]
fn resumable_global_chat_buffer_has_hard_bounds() {
    let mut buffer = GlobalChatRunBuffer::default();
    for _ in 0..(GLOBAL_CHAT_RUN_MAX_CHUNKS + 10) {
        buffer.push("x");
    }
    assert!(buffer.truncated);
    assert!(buffer.chunks.len() <= GLOBAL_CHAT_RUN_MAX_CHUNKS);
    assert!(buffer.bytes <= GLOBAL_CHAT_RUN_MAX_BUFFER_BYTES);
}

#[test]
fn registered_agent_run_can_be_cancelled_and_is_removed_by_guard() {
    let request_id = format!("native-cancel-test-{}", uuid::Uuid::new_v4());
    let (token, guard) = register_copilot_run(Some(&request_id));
    assert!(!token.is_cancelled());
    assert_eq!(cancel_copilot_chat(request_id.clone()), Ok(true));
    assert!(token.is_cancelled());
    drop(guard);
    assert!(!ACTIVE_COPILOT_RUNS.contains_key(&request_id));
    assert_eq!(cancel_copilot_chat(request_id), Ok(false));
}
