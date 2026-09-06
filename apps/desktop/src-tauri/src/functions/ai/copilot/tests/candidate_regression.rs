use super::*;

#[test]
fn sdk_guard_blocks_tiny_valid_candidate_before_it_can_queue() {
    use std::sync::atomic::{AtomicUsize, Ordering};

    let state = Arc::new(StdMutex::new(WorkflowToolLoopState::default()));
    seed_failed_rich_candidate(&state);
    let handler_calls = Arc::new(AtomicUsize::new(0));
    let calls = handler_calls.clone();
    let handler: copilot_sdk::ToolHandler = Arc::new(move |_name, _args| {
        calls.fetch_add(1, Ordering::SeqCst);
        copilot_sdk::ToolResultObject::text(
            serde_json::json!({ "status": "queued", "queued_count": 1 }).to_string(),
        )
    });
    let mut guarded = guard_sdk_workflow_tools(
        vec![(copilot_sdk::Tool::new("edit_flowscript"), handler)],
        state.clone(),
    );
    let (_, guarded_handler) = guarded.pop().expect("guarded edit tool");
    let tiny = serde_json::json!({
        "flowscript": "eventsSimple() {\n    logInfo({ message: \"works\" })\n}",
        "allow_deletions": true
    });

    let result = guarded_handler("edit_flowscript", &tiny);
    let result_text = result
        .error
        .as_deref()
        .unwrap_or(&result.text_result_for_llm);
    assert_eq!(handler_calls.load(Ordering::SeqCst), 0);
    assert!(result_text.contains("candidate_regression"));
    assert!(result_text.contains("retained_flowscript"));

    let snapshot = state.lock().expect("state lock").snapshot();
    assert_eq!(
        snapshot.last_flowscript.as_deref(),
        Some(rich_support_flowscript())
    );
    assert_eq!(snapshot.edit_attempts, 1);
    assert!(
        snapshot
            .last_errors
            .iter()
            .any(|error| error.contains("one connection still needs repair"))
    );
    assert!(
        snapshot
            .last_errors
            .iter()
            .any(|error| error.contains("severe completeness regression"))
    );
    assert!(!state.lock().expect("state lock").edit_in_flight);
}

#[test]
fn panicking_sdk_workflow_handler_aborts_state_and_recovers_the_operation_gate() {
    use std::sync::atomic::{AtomicUsize, Ordering};

    let state = Arc::new(StdMutex::new(WorkflowToolLoopState::default()));
    let panicking: copilot_sdk::ToolHandler =
        Arc::new(|_name, _args| panic!("simulated tool handler crash"));
    let follow_up_calls = Arc::new(AtomicUsize::new(0));
    let calls = follow_up_calls.clone();
    let benign: copilot_sdk::ToolHandler = Arc::new(move |_name, _args| {
        calls.fetch_add(1, Ordering::SeqCst);
        copilot_sdk::ToolResultObject::text(
            serde_json::json!({
                "status": "validation_errors",
                "errors": ["stub diagnostic"]
            })
            .to_string(),
        )
    });
    // Both guarded handlers share one operation gate, like one live SDK session.
    let mut guarded = guard_sdk_workflow_tools(
        vec![
            (copilot_sdk::Tool::new("edit_flowscript"), panicking),
            (copilot_sdk::Tool::new("edit_flowscript"), benign),
        ],
        state.clone(),
    );
    let (_, benign_handler) = guarded.pop().expect("benign guarded tool");
    let (_, panicking_handler) = guarded.pop().expect("panicking guarded tool");
    let args = serde_json::json!({
        "flowscript": "eventsSimple() {\n    logInfo({ message: \"works\" })\n}"
    });

    let crashed = panicking_handler("edit_flowscript", &args);
    let crashed_text = crashed
        .error
        .as_deref()
        .unwrap_or(&crashed.text_result_for_llm);
    assert!(
        crashed_text.contains("simulated tool handler crash"),
        "{crashed_text}"
    );
    {
        let state = state
            .lock()
            .expect("a handler panic must not poison the workflow loop state");
        assert!(!state.edit_in_flight);
        assert_eq!(state.last_status.as_deref(), Some("error"));
    }

    // The gate must be usable again: the follow-up mutation reaches its handler instead of
    // the permanent retryable "wait" refusal a poisoned gate produced.
    let retried = benign_handler("edit_flowscript", &args);
    let retried_text = retried
        .error
        .as_deref()
        .unwrap_or(&retried.text_result_for_llm);
    assert!(
        !retried_text.contains("Another order-sensitive workflow operation"),
        "{retried_text}"
    );
    assert_eq!(follow_up_calls.load(Ordering::SeqCst), 1);
}

#[test]
fn wrapped_log_helper_is_still_a_candidate_regression_but_domain_helper_can_pass() {
    let smoke_state = Arc::new(StdMutex::new(WorkflowToolLoopState::default()));
    seed_failed_rich_candidate(&smoke_state);
    assert!(workflow_tool_preflight(&smoke_state, "edit_flowscript").is_none());
    let wrapped_smoke = serde_json::json!({
        "flowscript": r#"function smoke() {
    logInfo({ message: "works" })
}
eventsSimple() {
    smoke()
}"#
    });
    let rejected = workflow_candidate_preflight(&smoke_state, "edit_flowscript", &wrapped_smoke)
        .expect("wrapping a log smoke test must not bypass completeness retention");
    assert_eq!(rejected.is_error, Some(true));
    assert!(!smoke_state.lock().expect("state lock").edit_in_flight);

    let domain_state = Arc::new(StdMutex::new(WorkflowToolLoopState::default()));
    seed_failed_rich_candidate(&domain_state);
    assert!(workflow_tool_preflight(&domain_state, "edit_flowscript").is_none());
    let modular_domain = serde_json::json!({
        "flowscript": r#"function pollInbox() {
    emailImapConnect({ host: "imap.example.com" })
}
eventsSimple() {
    pollInbox()
}"#
    });
    assert!(
        workflow_candidate_preflight(&domain_state, "edit_flowscript", &modular_domain,).is_none(),
        "a non-empty domain helper invoked by a separate Event is a valid modular partial"
    );
    workflow_tool_record(
        &domain_state,
        "edit_flowscript",
        &modular_domain,
        &serde_json::json!({
            "status": "validation_errors",
            "errors": ["domain helper still needs one pin fix"]
        })
        .to_string(),
    );
    let failed_snapshot = domain_state.lock().expect("state lock").snapshot();
    assert!(!failed_snapshot.queued);
    assert!(failed_snapshot.modular_fallback.is_none());

    assert!(workflow_tool_preflight(&domain_state, "edit_flowscript").is_none());
    assert!(
        workflow_candidate_preflight(&domain_state, "edit_flowscript", &modular_domain,).is_none()
    );
    let mut queued_result = copilot_sdk::ToolResultObject::text(
        serde_json::json!({ "status": "queued", "queued_count": 2 }).to_string(),
    );
    workflow_tool_record(
        &domain_state,
        "edit_flowscript",
        &modular_domain,
        &queued_result.text_result_for_llm,
    );
    annotate_modular_fallback_result(&domain_state, "edit_flowscript", &mut queued_result);
    assert!(
        queued_result
            .text_result_for_llm
            .contains("partial_working_slice")
    );

    let snapshot = domain_state.lock().expect("state lock").snapshot();
    assert!(snapshot.queued);
    assert!(snapshot.modular_fallback.is_some());
    assert_eq!(
        snapshot.retained_full_source.as_deref(),
        Some(rich_support_flowscript())
    );
    let envelope = flowscript_response_workspace_envelope(
        submitted_flowscript(&modular_domain).unwrap(),
        "queued",
        Some(&snapshot),
    );
    let envelope: serde_json::Value = serde_json::from_str(&envelope).unwrap();
    assert_eq!(envelope["completion"], "partial_working_slice");
    assert_eq!(envelope["retained_full_source"], rich_support_flowscript());
}

#[test]
fn best_failed_candidate_survives_smaller_actionable_retries() {
    let state = Arc::new(StdMutex::new(WorkflowToolLoopState::default()));
    seed_failed_rich_candidate(&state);
    workflow_tool_record(
        &state,
        "edit_flowscript",
        &serde_json::json!({
            "flowscript": "eventsSimple() {\n    emailImapConnect({ host: \"imap.example.com\" })\n    mailImapList({ inbox: inbox })\n    logInfo({ message: \"partial\" })\n}"
        }),
        &serde_json::json!({ "status": "no_changes" }).to_string(),
    );

    let snapshot = state.lock().expect("state lock").snapshot();
    assert_eq!(
        snapshot.last_flowscript.as_deref(),
        Some(rich_support_flowscript())
    );
    assert_eq!(snapshot.last_status.as_deref(), Some("validation_errors"));
    assert_eq!(
        snapshot.last_errors,
        vec!["one connection still needs repair"]
    );
}

#[test]
fn best_failed_candidate_advances_when_a_same_scope_draft_has_fewer_diagnostics() {
    let state = Arc::new(StdMutex::new(WorkflowToolLoopState::default()));
    workflow_tool_record(
        &state,
        "edit_flowscript",
        &serde_json::json!({ "flowscript": rich_support_flowscript() }),
        &serde_json::json!({
            "status": "validation_errors",
            "errors": ["first unresolved edge", "second unresolved edge"]
        })
        .to_string(),
    );
    let improved = rich_support_flowscript().replace(
        "    structGet({ struct: payload, field: \"ticket_id\" })\n",
        "",
    );
    assert!(
        profile_flowscript_candidate(&improved).completeness_score()
            < profile_flowscript_candidate(rich_support_flowscript()).completeness_score()
    );
    workflow_tool_record(
        &state,
        "edit_flowscript",
        &serde_json::json!({ "flowscript": improved.clone() }),
        &serde_json::json!({
            "status": "validation_errors",
            "errors": ["second unresolved edge"]
        })
        .to_string(),
    );

    let snapshot = state.lock().expect("state lock").snapshot();
    assert_eq!(snapshot.last_flowscript.as_deref(), Some(improved.as_str()));
    assert_eq!(snapshot.last_errors, vec!["second unresolved edge"]);
}

#[test]
fn workflow_edit_budget_keeps_a_higher_hard_safety_cap() {
    let state = Arc::new(StdMutex::new(WorkflowToolLoopState::default()));
    for attempt in 0..MAX_EXTERNAL_WORKFLOW_EDIT_ATTEMPTS {
        assert!(
            workflow_tool_preflight(&state, "edit_flowscript").is_none(),
            "attempt {attempt} should be available"
        );
        workflow_tool_abort(&state, "edit_flowscript", "retry");
    }
    let exhausted = workflow_tool_preflight(&state, "edit_flowscript")
        .expect("the attempt after the hard safety cap must be rejected");
    assert_eq!(exhausted.is_error, Some(true));
}

#[test]
fn workflow_edit_loop_allows_more_than_five_repairs_when_diagnostics_progress() {
    let state = Arc::new(StdMutex::new(WorkflowToolLoopState::default()));
    for attempt in 0..8 {
        assert!(
            workflow_tool_preflight(&state, "edit_flowscript").is_none(),
            "genuine diagnostic progress must keep attempt {attempt} available"
        );
        workflow_tool_record(
            &state,
            "edit_flowscript",
            &serde_json::json!({
                "flowscript": format!("eventsSimple() {{ repair{attempt}() }}")
            }),
            &serde_json::json!({
                "status": "validation_errors",
                "errors": [format!("remaining diagnostic {attempt}")]
            })
            .to_string(),
        );
    }
    assert_eq!(state.lock().expect("state lock").edit_attempts, 8);
    assert_eq!(state.lock().expect("state lock").stalled_edit_attempts, 0);
    assert!(workflow_tool_preflight(&state, "edit_flowscript").is_none());
}

#[test]
fn workflow_edit_loop_stops_after_repeated_identical_diagnostics() {
    let state = Arc::new(StdMutex::new(WorkflowToolLoopState::default()));
    for attempt in 0..=MAX_EXTERNAL_WORKFLOW_STALLED_EDIT_ATTEMPTS {
        assert!(
            workflow_tool_preflight(&state, "edit_flowscript").is_none(),
            "initial failure plus bounded stalled attempt {attempt} should run"
        );
        workflow_tool_record(
            &state,
            "edit_flowscript",
            &serde_json::json!({
                "flowscript": format!("eventsSimple() {{ stillBroken{attempt}() }}")
            }),
            &serde_json::json!({
                "status": "validation_errors",
                "errors": ["the exact same unresolved diagnostic"]
            })
            .to_string(),
        );
    }

    let stalled = workflow_tool_preflight(&state, "edit_flowscript")
        .expect("the next identical-diagnostic repair must stop locally");
    assert_eq!(stalled.is_error, Some(true));
    let text = stalled
        .content
        .iter()
        .filter_map(|content| match &content.raw {
            rmcp::model::RawContent::Text(text) => Some(text.text.as_str()),
            _ => None,
        })
        .collect::<Vec<_>>()
        .join("\n");
    assert!(text.contains("edit_progress_stalled"), "{text}");
}
