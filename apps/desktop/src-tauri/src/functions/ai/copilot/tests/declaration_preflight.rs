use super::*;

#[test]
fn external_workflow_tool_budget_forces_edit_and_retry() {
    let state = Arc::new(StdMutex::new(WorkflowToolLoopState::default()));

    assert!(workflow_tool_preflight(&state, "get_current_flowscript").is_none());
    assert_eq!(
        workflow_tool_preflight(&state, "get_current_flowscript")
            .and_then(|result| result.is_error),
        Some(true),
        "the live document may only be fetched once"
    );
    assert!(
        workflow_tool_preflight_with_args(
            &state,
            "get_declarations",
            &serde_json::json!({ "queries": ["log information"] }),
        )
        .is_none()
    );
    workflow_tool_record(
        &state,
        "get_declarations",
        &serde_json::json!({ "queries": ["log information"] }),
        "declare function logInfo({ message: string }): void;",
    );
    assert_eq!(
        workflow_tool_preflight(&state, "get_declarations").and_then(|result| result.is_error),
        Some(false),
        "a second declaration call is a successful redirect to editing, not a provider error"
    );
    assert_eq!(
        workflow_tool_preflight(&state, "catalog_search").and_then(|result| result.is_error),
        Some(true),
        "legacy catalog discovery is never part of a FlowScript mutation loop"
    );

    assert!(workflow_tool_preflight(&state, "edit_flowscript").is_none());
    assert_eq!(
        workflow_tool_preflight(&state, "edit_flowscript").and_then(|result| result.is_error),
        Some(true),
        "parallel FlowScript drafts must be serialized"
    );
    workflow_tool_record(
        &state,
        "edit_flowscript",
        &serde_json::json!({ "flowscript": "eventsSimple() { brokenCall() }" }),
        &serde_json::json!({
            "status": "validation_errors",
            "errors": ["brokenCall does not match a catalog declaration"]
        })
        .to_string(),
    );
    let repair_lookup = serde_json::json!({ "queries": ["brokenCall"] });
    assert!(
        workflow_tool_preflight_with_args(&state, "get_declarations", &repair_lookup).is_none(),
        "one targeted declaration lookup is available for a repair"
    );
    workflow_tool_record(
        &state,
        "get_declarations",
        &repair_lookup,
        "declare function brokenCall({}): void;",
    );
    assert!(workflow_tool_preflight(&state, "edit_flowscript").is_none());
    workflow_tool_record(
        &state,
        "edit_flowscript",
        &serde_json::json!({ "flowscript": "eventsSimple() { log({ text: \"ok\" }) }" }),
        &serde_json::json!({ "status": "queued", "queued_count": 2 }).to_string(),
    );
    let terminal = workflow_tool_preflight(&state, "get_declarations")
        .expect("queued state returns an explicit terminal result");
    assert_eq!(terminal.is_error, Some(false));
    assert!(
        workflow_tool_preflight(&state, "emit_ui").is_none(),
        "a combined board + UI request may finish its UI after the workflow queues"
    );
    assert!(state.lock().expect("state lock").queued);
}

#[test]
fn first_flowscript_write_requires_one_live_declaration_batch() {
    let state = Arc::new(StdMutex::new(WorkflowToolLoopState::default()));
    let write_args = serde_json::json!({
        "draft_id": "declaration-gated-source",
        "source": "eventsSimple() { logInfo({ message: \"hello\" }) }"
    });

    let redirected = workflow_tool_preflight_with_args(&state, "write_flowscript", &write_args)
        .expect("the first write must be redirected to live declaration discovery");
    assert_eq!(redirected.is_error, Some(false));
    let redirected = redirected
        .content
        .iter()
        .filter_map(|content| match &content.raw {
            rmcp::model::RawContent::Text(text) => Some(text.text.as_str()),
            _ => None,
        })
        .collect::<Vec<_>>()
        .join("\n");
    assert!(redirected.contains("declaration_lookup_required"));
    assert!(redirected.contains("one bounded get_declarations batch"));
    assert!(redirected.contains("call plan_board_scope exactly once"));
    assert!(redirected.contains("active segment"));
    assert!(!state.lock().expect("state lock").edit_in_flight);

    let empty_lookup = workflow_tool_preflight_with_args(
        &state,
        "get_declarations",
        &serde_json::json!({ "queries": ["   "] }),
    )
    .expect("an empty initial declaration batch must be rejected");
    let empty_lookup = empty_lookup
        .content
        .iter()
        .filter_map(|content| match &content.raw {
            rmcp::model::RawContent::Text(text) => Some(text.text.as_str()),
            _ => None,
        })
        .collect::<Vec<_>>()
        .join("\n");
    assert!(empty_lookup.contains("declaration_batch_required"));

    assert!(
        workflow_tool_preflight_with_args(
            &state,
            "get_declarations",
            &serde_json::json!({ "queries": ["log information"] }),
        )
        .is_none()
    );
    assert!(
        workflow_tool_preflight_with_args(&state, "write_flowscript", &write_args).is_some(),
        "dispatching a lookup is not enough; its usable result must be retained"
    );
    workflow_tool_record(
        &state,
        "get_declarations",
        &serde_json::json!({ "queries": ["log information"] }),
        "No FlowScript declarations matched this query.",
    );
    let guard = state.lock().expect("state lock");
    assert_eq!(guard.declaration_calls, 1);
    assert_eq!(guard.initial_declaration_attempts, 1);
    assert!(!guard.initial_declaration_lookup_complete);
    drop(guard);
    assert!(
        workflow_tool_preflight_with_args(
            &state,
            "get_declarations",
            &serde_json::json!({ "queries": ["log information"] }),
        )
        .is_none(),
        "a no-match initial lookup leaves the focused initial lookup available"
    );
    workflow_tool_record(
        &state,
        "get_declarations",
        &serde_json::json!({ "queries": ["log information"] }),
        "declare function logInfo({ message: string }): void;  // impure",
    );
    let planless = workflow_tool_preflight_with_args(&state, "write_flowscript", &write_args)
        .expect("usable declarations still require a scope plan before the first write");
    assert_eq!(
        workflow_call_result_json(&planless)["code"],
        "SCOPE_PLAN_REQUIRED"
    );
    accept_single_segment_plan(&state);
    assert!(
        workflow_tool_preflight_with_args(&state, "write_flowscript", &write_args).is_none(),
        "the exact same source write is dispatched after a usable declaration result and a plan"
    );
}

#[test]
fn partial_declaration_coverage_retains_matches_and_unlocks_first_source_checkpoint() {
    let state = Arc::new(StdMutex::new(WorkflowToolLoopState::default()));
    let initial = serde_json::json!({ "queries": ["imap receive", "smtp send"] });
    assert!(workflow_tool_preflight_with_args(&state, "get_declarations", &initial).is_none());
    workflow_tool_record(
        &state,
        "get_declarations",
        &initial,
        concat!(
            "// flowpilot.declaration-batch/v1 {\"processed_count\":2,\"matched_count\":1,\"matched_queries\":[\"imap receive\"],\"unmatched_count\":1,\"unmatched_queries\":[\"smtp send\"],\"complete\":false,\"omitted_count\":0,\"omitted_queries\":[],\"truncated_query_count\":0}\n",
            "declare function emailImapConnect({ host: string }): (connection: Struct);"
        ),
    );
    {
        let guard = state.lock().expect("state lock");
        assert!(guard.initial_declaration_lookup_usable);
        assert!(!guard.initial_declaration_lookup_complete);
        assert_eq!(guard.unresolved_declaration_queries, ["smtp send"]);
        assert!(
            guard
                .last_declarations
                .as_deref()
                .is_some_and(|declarations| { declarations.contains("emailImapConnect") })
        );
    }
    let write_args = serde_json::json!({
        "draft_id": "coverage-gated",
        "source": "eventsSimple() {}"
    });
    accept_single_segment_plan(&state);
    let unrelated = serde_json::json!({ "queries": ["string replace"] });
    let rejected = workflow_tool_preflight_with_args(&state, "get_declarations", &unrelated)
        .expect("a second pre-draft lookup must redirect to the retained source checkpoint");
    let rejected = workflow_call_result_json(&rejected);
    assert_eq!(rejected["status"], "discovery_budget_exhausted");
    assert_eq!(rejected["next_action"], "write_flowscript");
    assert!(
        rejected["message"]
            .as_str()
            .is_some_and(|message| message.contains("Do not chase omitted or unmatched"))
    );
    {
        let guard = state.lock().expect("state lock");
        assert!(!guard.initial_declaration_lookup_complete);
        assert_eq!(guard.unresolved_declaration_queries, ["smtp send"]);
        assert_eq!(guard.initial_declaration_attempts, 1);
    }
    assert!(
        workflow_tool_preflight_with_args(&state, "write_flowscript", &write_args).is_none(),
        "one usable live signature must unlock a recoverable full-shape source checkpoint"
    );
}

#[test]
fn focused_rephrasing_can_resolve_an_unmatched_declaration_query() {
    assert!(declaration_queries_are_related(
        "smtp send approval response",
        "smtp send email"
    ));
    assert!(!declaration_queries_are_related(
        "smtp send email",
        "smtp receive email"
    ));
    assert!(!declaration_queries_are_related(
        "smtp send email",
        "imap receive email"
    ));

    let state = Arc::new(StdMutex::new(WorkflowToolLoopState::default()));
    let initial = serde_json::json!({ "queries": ["smtp send approval response"] });
    assert!(workflow_tool_preflight_with_args(&state, "get_declarations", &initial).is_none());
    workflow_tool_record(
        &state,
        "get_declarations",
        &initial,
        "// flowpilot.declaration-batch/v1 {\"processed_count\":1,\"matched_count\":0,\"matched_queries\":[],\"unmatched_count\":1,\"unmatched_queries\":[\"smtp send approval response\"],\"complete\":false,\"omitted_count\":0,\"omitted_queries\":[],\"truncated_query_count\":0}\nNo declaration matched.",
    );

    let rephrased = serde_json::json!({ "queries": ["smtp send email"] });
    assert!(
        workflow_tool_preflight_with_args(&state, "get_declarations", &rephrased).is_none(),
        "a rephrasing that retains the distinctive smtp capability may repair the miss"
    );
    workflow_tool_record(
        &state,
        "get_declarations",
        &rephrased,
        concat!(
            "// flowpilot.declaration-batch/v1 {\"processed_count\":1,\"matched_count\":1,\"matched_queries\":[\"smtp send email\"],\"unmatched_count\":0,\"unmatched_queries\":[],\"complete\":true,\"omitted_count\":0,\"omitted_queries\":[],\"truncated_query_count\":0}\n",
            "declare function emailSmtpSend({ to: string, bodyText: string }): void;"
        ),
    );
    let guard = state.lock().expect("state lock");
    assert!(guard.initial_declaration_lookup_complete);
    assert!(guard.unresolved_declaration_queries.is_empty());
}

#[test]
fn source_lifecycle_errors_cannot_bypass_initial_declaration_coverage() {
    let state = Arc::new(StdMutex::new(WorkflowToolLoopState::default()));
    let patch_args = serde_json::json!({
        "draft_id": "not-retained",
        "expected_revision": 0,
        "old_text": "before",
        "new_text": "after"
    });
    let rejected = workflow_tool_preflight_with_args(&state, "patch_flowscript", &patch_args)
        .expect("patch cannot create an unretained draft");
    let payload = workflow_call_result_json(&rejected);
    assert_eq!(payload["code"], "FLOWSCRIPT_DRAFT_REQUIRED");
    assert_eq!(payload["next_action"], "get_declarations");

    // Even a stale/legacy worker result cannot turn an arbitrary error status into declaration
    // authorization. This protects old in-flight calls across an app upgrade as well.
    workflow_tool_record(
        &state,
        "patch_flowscript",
        &patch_args,
        &serde_json::json!({
            "status": "error",
            "code": "FLOWSCRIPT_DRAFT_NOT_FOUND",
            "message": "draft missing"
        })
        .to_string(),
    );
    let write = workflow_tool_preflight_with_args(
        &state,
        "write_flowscript",
        &serde_json::json!({
            "draft_id": "still-gated",
            "source": "eventsSimple() {}"
        }),
    )
    .expect("an incidental lifecycle error must not unlock source generation");
    let payload = workflow_call_result_json(&write);
    assert_eq!(payload["status"], "declaration_lookup_required");
}

#[test]
fn request_identity_mismatch_does_not_adopt_rejected_source_coordinates() {
    let state = Arc::new(StdMutex::new(WorkflowToolLoopState {
        initial_declaration_lookup_complete: true,
        scope_plan: Some(single_segment_plan()),
        ..Default::default()
    }));
    let rejected_args = serde_json::json!({
        "draft_id": "foreign-draft",
        "source": "eventsSimple() { logInfo({ message: \"must not retain\" }) }"
    });
    assert!(
        workflow_tool_preflight_with_args(&state, "write_flowscript", &rejected_args).is_none()
    );
    let mismatch = serde_json::json!({
        "status": "request_identity_mismatch",
        "code": "FLOWSCRIPT_DRAFT_REQUEST_IDENTITY_MISMATCH",
        "message": "This FlowScript draft belongs to a different immutable user request."
    })
    .to_string();
    workflow_tool_record(&state, "write_flowscript", &rejected_args, &mismatch);

    {
        let state = state.lock().expect("state lock");
        assert!(!state.flowscript_draft_retained);
        assert!(state.flowscript_draft_id.is_none());
        assert!(state.flowscript_revision.is_none());
        assert!(state.last_flowscript.is_none());
        assert_eq!(
            state.last_status.as_deref(),
            Some("request_identity_mismatch")
        );
    }
    assert!(flowpilot_tool_result_is_error(
        &copilot_sdk::ToolResultObject::text(mismatch)
    ));
    assert!(
        workflow_tool_preflight_with_args(
            &state,
            "write_flowscript",
            &serde_json::json!({
                "draft_id": "current-request-draft",
                "source": "eventsSimple() {}"
            }),
        )
        .is_none(),
        "a distinct draft id for the current request remains available"
    );
}

#[test]
fn typed_request_mismatch_does_not_adopt_rejected_coordinates() {
    let state = Arc::new(StdMutex::new(WorkflowToolLoopState {
        typed_draft_id: Some("current-draft".to_string()),
        typed_draft_retained: true,
        typed_revision: Some(4),
        ..Default::default()
    }));
    let rejected_args = serde_json::json!({
        "draft_id": "foreign-draft",
        "expected_revision": 99,
        "modules": []
    });
    // Include coordinates defensively: the core now omits them, but a stale/custom provider
    // response must not be able to poison the host-owned loop state either.
    let mismatch = serde_json::json!({
        "status": "request_identity_mismatch",
        "code": "IR_DRAFT_REQUEST_IDENTITY_MISMATCH",
        "draft_id": "foreign-draft",
        "revision": 99,
        "message": "This typed draft belongs to a different immutable user request."
    })
    .to_string();
    workflow_tool_record(&state, "validate_flow_ir_draft", &rejected_args, &mismatch);

    let state = state.lock().expect("state lock");
    assert_eq!(state.typed_draft_id.as_deref(), Some("current-draft"));
    assert_eq!(state.typed_revision, Some(4));
    assert!(state.typed_draft_retained);
    assert_eq!(
        state.last_status.as_deref(),
        Some("request_identity_mismatch")
    );
}

#[test]
fn missing_or_stale_base_releases_unusable_draft_coordinates_for_restart() {
    for (code, include_source) in [
        ("FLOWSCRIPT_DRAFT_MISSING", false),
        ("FLOWSCRIPT_BASE_REVISION_CONFLICT", true),
    ] {
        let old_source = "eventsSimple() { logInfo({ message: \"preserve me\" }) }";
        let state = Arc::new(StdMutex::new(WorkflowToolLoopState {
            initial_declaration_lookup_complete: true,
            mutation_path: Some(WorkflowMutationPath::FlowScript),
            flowscript_draft_id: Some("expired-draft".to_string()),
            flowscript_draft_retained: true,
            flowscript_revision: Some(4),
            last_flowscript: Some(old_source.to_string()),
            last_status: Some("valid".to_string()),
            scope_plan: Some(single_segment_plan()),
            ..Default::default()
        }));
        let check_args = serde_json::json!({
            "draft_id": "expired-draft",
            "expected_revision": 4
        });
        assert!(
            workflow_tool_preflight_with_args(&state, "check_flowscript", &check_args).is_none()
        );
        let mut result = serde_json::json!({
            "status": "error",
            "code": code,
            "message": "the retained coordinates can no longer be continued"
        });
        if include_source {
            result["source"] = serde_json::json!(old_source);
        }
        workflow_tool_record(&state, "check_flowscript", &check_args, &result.to_string());

        {
            let state = state.lock().expect("state lock");
            assert!(!state.flowscript_draft_retained, "{code}");
            assert!(state.flowscript_draft_id.is_none(), "{code}");
            assert!(state.flowscript_revision.is_none(), "{code}");
            assert_eq!(state.last_flowscript.as_deref(), Some(old_source), "{code}");
        }
        assert!(
            workflow_tool_preflight_with_args(
                &state,
                "write_flowscript",
                &serde_json::json!({
                    "draft_id": format!("fresh-{code}"),
                    "source": old_source
                }),
            )
            .is_none(),
            "{code} must allow a fresh host-authorized draft id"
        );
    }
}

#[test]
fn incomplete_declaration_coverage_stops_after_bounded_attempts() {
    let state = Arc::new(StdMutex::new(WorkflowToolLoopState::default()));
    let args = serde_json::json!({ "queries": ["unavailable capability"] });
    let no_match = "// flowpilot.declaration-batch/v1 {\"processed_count\":1,\"matched_count\":0,\"matched_queries\":[],\"unmatched_count\":1,\"unmatched_queries\":[\"unavailable capability\"],\"complete\":false,\"omitted_count\":0,\"omitted_queries\":[],\"truncated_query_count\":0}\nNo FlowScript declarations matched this query.";
    for _ in 0..MAX_INITIAL_DECLARATION_ATTEMPTS {
        assert!(workflow_tool_preflight_with_args(&state, "get_declarations", &args).is_none());
        workflow_tool_record(&state, "get_declarations", &args, no_match);
    }
    let rejected = workflow_tool_preflight_with_args(
        &state,
        "write_flowscript",
        &serde_json::json!({
            "draft_id": "must-not-start",
            "source": "eventsSimple() {}"
        }),
    )
    .expect("guessing source after incomplete coverage must stop locally");
    let payload = workflow_call_result_json(&rejected);
    assert_eq!(payload["code"], "DECLARATION_COVERAGE_EXHAUSTED");
    assert_eq!(payload["attempts"], MAX_INITIAL_DECLARATION_ATTEMPTS);
    assert_eq!(payload["unresolved_queries"][0], "unavailable capability");
}

#[test]
fn aborted_initial_declaration_lookup_releases_the_required_slot() {
    let state = Arc::new(StdMutex::new(WorkflowToolLoopState::default()));
    let args = serde_json::json!({ "queries": ["imap inbox messages"] });

    assert!(workflow_tool_preflight_with_args(&state, "get_declarations", &args).is_none());
    assert_eq!(state.lock().expect("state lock").declaration_calls, 1);

    workflow_tool_abort(&state, "get_declarations", "worker disconnected");
    let guard = state.lock().expect("state lock");
    assert_eq!(guard.declaration_calls, 0);
    assert_eq!(guard.declarations_since_edit, 0);
    drop(guard);

    assert!(
        workflow_tool_preflight_with_args(&state, "get_declarations", &args).is_none(),
        "the focused initial lookup can be retried after a worker abort"
    );
}

#[test]
fn declaration_lookup_lease_blocks_parallel_lookup_and_source_dispatch() {
    let state = Arc::new(StdMutex::new(WorkflowToolLoopState::default()));
    let args = serde_json::json!({ "queries": ["imap inbox messages"] });
    assert!(workflow_tool_preflight_with_args(&state, "get_declarations", &args).is_none());

    let duplicate = workflow_tool_preflight_with_args(&state, "get_declarations", &args)
        .expect("a second lookup must not dispatch while the first result is pending");
    assert_eq!(
        workflow_call_result_json(&duplicate)["code"],
        "DECLARATION_LOOKUP_IN_FLIGHT"
    );
    let write = workflow_tool_preflight_with_args(
        &state,
        "write_flowscript",
        &serde_json::json!({
            "draft_id": "must-wait",
            "source": "eventsSimple() {}"
        }),
    )
    .expect("source authoring must wait for declaration authority");
    assert_eq!(
        workflow_call_result_json(&write)["code"],
        "DECLARATION_LOOKUP_IN_FLIGHT"
    );
    assert_eq!(state.lock().expect("state lock").declaration_calls, 1);

    workflow_tool_abort(&state, "get_declarations", "worker disconnected");
    assert!(workflow_tool_preflight_with_args(&state, "get_declarations", &args).is_none());
}

#[test]
fn poisoned_workflow_state_fails_closed_before_tool_dispatch() {
    let state = Arc::new(StdMutex::new(WorkflowToolLoopState::default()));
    let poisoned = state.clone();
    let _ = std::panic::catch_unwind(move || {
        let _guard = poisoned.lock().expect("initial lock");
        panic!("poison workflow loop state for regression coverage");
    });

    let rejected = workflow_tool_preflight(&state, "get_current_flowscript")
        .expect("a poisoned lifecycle state must return a terminal host error");
    assert_eq!(rejected.is_error, Some(true));
    assert_eq!(
        workflow_call_result_json(&rejected)["code"],
        "WORKFLOW_LOOP_STATE_UNAVAILABLE"
    );
    assert!(workflow_state_has_retained_candidate(Some(&state)));
}

#[test]
fn declaration_headers_cannot_claim_complete_without_exact_bodies() {
    let state = Arc::new(StdMutex::new(WorkflowToolLoopState::default()));
    let args = serde_json::json!({ "queries": ["smtp send"] });
    assert!(workflow_tool_preflight_with_args(&state, "get_declarations", &args).is_none());
    workflow_tool_record(
        &state,
        "get_declarations",
        &args,
        "// flowpilot.declaration-batch/v1 {\"processed_count\":1,\"matched_count\":1,\"matched_queries\":[\"smtp send\"],\"unmatched_count\":0,\"unmatched_queries\":[],\"output_omitted_count\":0,\"output_omitted_queries\":[],\"complete\":true,\"omitted_count\":0,\"omitted_queries\":[],\"truncated_query_count\":0}\n// declaration body was lost",
    );
    {
        let state = state.lock().expect("state lock");
        assert!(!state.initial_declaration_lookup_complete);
        assert_eq!(state.unresolved_declaration_queries, ["smtp send"]);
    }
    assert!(
        workflow_tool_preflight_with_args(&state, "get_declarations", &args).is_none(),
        "the exact omitted capability gets one bounded focused retry"
    );
}

#[test]
fn metadata_less_multi_query_results_do_not_unlock_source() {
    let state = Arc::new(StdMutex::new(WorkflowToolLoopState::default()));
    let args = serde_json::json!({ "queries": ["imap receive", "smtp send"] });
    assert!(workflow_tool_preflight_with_args(&state, "get_declarations", &args).is_none());
    workflow_tool_record(
        &state,
        "get_declarations",
        &args,
        "declare function emailImapConnect({ host: string }): Struct;",
    );
    let state = state.lock().expect("state lock");
    assert!(!state.initial_declaration_lookup_complete);
    assert_eq!(
        state.unresolved_declaration_queries,
        ["imap receive", "smtp send"]
    );
}

#[test]
fn mixed_unnamed_declaration_omissions_are_counted_per_category() {
    let state = Arc::new(StdMutex::new(WorkflowToolLoopState::default()));
    let output_queries = (0..10)
        .map(|index| format!("output omitted {index}"))
        .collect::<Vec<_>>();
    let mut all_queries = output_queries.clone();
    all_queries.extend((0..5).map(|index| format!("input omitted {index}")));
    let args = serde_json::json!({ "queries": all_queries });
    assert!(workflow_tool_preflight_with_args(&state, "get_declarations", &args).is_none());
    let header = format!(
        "// flowpilot.declaration-batch/v1 {}\nNo exact declaration body fit.",
        serde_json::json!({
            "processed_count": 10,
            "matched_count": 0,
            "matched_queries": [],
            "unmatched_count": 0,
            "unmatched_queries": [],
            "output_omitted_count": 10,
            "output_omitted_queries": output_queries,
            "complete": false,
            "omitted_count": 5,
            "omitted_queries": [],
            "query_names_omitted_for_size": true,
            "truncated_query_count": 0
        })
    );
    workflow_tool_record(&state, "get_declarations", &args, &header);

    let state = state.lock().expect("state lock");
    assert_eq!(state.unresolved_declaration_queries.len(), 11);
    assert!(
        state
            .unresolved_declaration_queries
            .iter()
            .any(|query| { query.contains("5 additional omitted declaration query") })
    );
}

#[test]
fn declaration_retention_prioritizes_complete_new_signatures_without_partial_lines() {
    let older = format!(
        "declare function oldCapability({{ input: string }}): string;\n{}",
        "old documentation ".repeat(2_000)
    );
    let newest = concat!(
        "// flowpilot.declaration-batch/v1 {}\n",
        "declare function emailSmtpConnect({ host: string, port: int }): Struct;\n",
        "declare function emailSmtpSend({ connection: Struct, to: string, bodyText: string }): void;\n",
        "// Usage: connect first, then pass the returned connection into the send call."
    );
    let retained = retain_declaration_result(Some(&older), newest);
    assert!(retained.contains("oldCapability"));
    assert!(retained.contains("emailSmtpConnect"));
    assert!(retained.contains("emailSmtpSend"));
    assert!(retained.contains("connect first"));
    assert!(!retained.contains("old documentation"));
    assert!(!retained.contains("…"));
}
