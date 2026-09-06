use super::*;

#[test]
fn external_failure_recovery_is_bounded_to_transient_unqueued_work() {
    let retained = WorkflowToolLoopSnapshot {
        last_flowscript: Some("eventsSimple() { logInfo({ message: \"resume\" }) }".into()),
        flowscript_draft_id: Some("resume-source".into()),
        flowscript_draft_retained: true,
        flowscript_revision: Some(2),
        ..Default::default()
    };
    assert_eq!(
        classify_external_agent_failure("transport stream closed unexpectedly", false),
        ExternalAgentExitKind::TransientInfrastructure
    );
    assert!(can_resume_external_workflow_after_failure(
        Some(&retained),
        "transport stream closed unexpectedly",
        false,
    ));
    assert!(!can_resume_external_workflow_after_failure(
        Some(&retained),
        "transport stream closed unexpectedly",
        true,
    ));
    assert!(!can_resume_external_workflow_after_failure(
        Some(&retained),
        "authentication failed: invalid API key",
        false,
    ));

    let declaration_only = WorkflowToolLoopSnapshot {
        last_declarations: Some("declare function logInfo({ message: string }): void;".into()),
        declaration_lookup_complete: true,
        ..Default::default()
    };
    assert!(can_resume_external_workflow_after_failure(
        Some(&declaration_only),
        "connection reset",
        false,
    ));
    assert!(can_resume_external_workflow_after_failure(
        Some(&WorkflowToolLoopSnapshot::default()),
        "connection reset before the first tool call",
        false,
    ));

    let queued = WorkflowToolLoopSnapshot {
        queued: true,
        ..retained.clone()
    };
    assert!(!can_resume_external_workflow_after_failure(
        Some(&queued),
        "connection reset",
        false,
    ));
    assert!(!can_resume_external_workflow_after_failure(
        None,
        "connection reset",
        false,
    ));
}

#[test]
fn external_agent_failures_map_to_actionable_user_diagnostics() {
    assert_eq!(
        classify_external_agent_user_failure(
            "Codex turn failed with 401 Unauthorized: token_invalidated"
        ),
        ExternalAgentFailureCategory::Authentication
    );
    assert_eq!(
        classify_external_agent_user_failure("Claude Code CLI was not found."),
        ExternalAgentFailureCategory::CliMissing
    );
    assert_eq!(
        classify_external_agent_user_failure(
            "CODEX_CLI_PATH is set but empty, so the Codex CLI cannot be resolved.",
        ),
        ExternalAgentFailureCategory::CliMissing
    );
    assert_eq!(
        classify_external_agent_user_failure(
            "CLAUDE_CODE_CLI_PATH points to /opt/claude, but that directory does not contain an executable named claude.",
        ),
        ExternalAgentFailureCategory::CliMissing
    );
    assert_eq!(
        classify_external_agent_user_failure(
            "Failed to start Claude Code: Permission denied (os error 13)"
        ),
        ExternalAgentFailureCategory::CliNotExecutable
    );
    assert_eq!(
        classify_external_agent_user_failure(
            "CODEX_CLI_PATH points to /opt/codex, but that file is not executable."
        ),
        ExternalAgentFailureCategory::CliNotExecutable
    );
    assert_eq!(
        classify_external_agent_user_failure(
            "MCP server connection failed: flowpilot disconnected"
        ),
        ExternalAgentFailureCategory::McpConnection
    );
    assert_eq!(
        classify_external_agent_user_failure("HTTP 403: organization has been disabled"),
        ExternalAgentFailureCategory::AccountAccess
    );
    assert_eq!(
        classify_external_agent_user_failure("Request failed (403)"),
        ExternalAgentFailureCategory::AccountAccess
    );
    assert_eq!(
        classify_external_agent_user_failure("HTTP 402 Payment Required"),
        ExternalAgentFailureCategory::AccountAccess
    );
    assert_eq!(
        classify_external_agent_user_failure("Request failed (429)"),
        ExternalAgentFailureCategory::RateLimit
    );
    assert_eq!(
        classify_external_agent_user_failure("Could not find model codex-future"),
        ExternalAgentFailureCategory::Model
    );
    assert_eq!(
        classify_external_agent_user_failure(
            "Failed to read configuration: No such file or directory"
        ),
        ExternalAgentFailureCategory::Process
    );
    assert_eq!(
        classify_external_agent_user_failure("Failed to read credentials file: permission denied"),
        ExternalAgentFailureCategory::LocalPermission
    );
    assert_eq!(
        classify_external_agent_user_failure("FlowPilot MCP server connection timed out"),
        ExternalAgentFailureCategory::McpConnection
    );
    assert_eq!(
        classify_external_agent_user_failure(
            "authentication status command exited with 2: unknown subcommand 'status'"
        ),
        ExternalAgentFailureCategory::Protocol
    );
    assert_eq!(
        classify_external_agent_user_failure("Codex authentication status command exited with 2"),
        ExternalAgentFailureCategory::Process
    );
    assert_eq!(
        classify_external_agent_user_failure(
            "Codex authentication status command exited with 1: network error"
        ),
        ExternalAgentFailureCategory::Network
    );
    assert_eq!(
        classify_external_agent_user_failure(
            "Codex authentication status check timed out after 6 seconds"
        ),
        ExternalAgentFailureCategory::Process
    );
    assert_eq!(
        classify_external_agent_user_failure("Request failed (401)"),
        ExternalAgentFailureCategory::Authentication
    );
    assert_eq!(
        classify_external_agent_user_failure("FlowPilot external agent run was cancelled"),
        ExternalAgentFailureCategory::UserCancelled
    );
    assert_eq!(
        classify_external_agent_user_failure(
            "NESTED_RUN_WALL_CLOCK_BUDGET_EXHAUSTED: compiler diagnostics retained"
        ),
        ExternalAgentFailureCategory::HostWorkflow
    );
    assert_eq!(
        classify_external_agent_user_failure(
            "The external agent exhausted its FlowScript source operation budget (24/24) without queueing changes."
        ),
        ExternalAgentFailureCategory::HostWorkflow
    );
    assert_eq!(
        classify_external_agent_user_failure("Prompt image 1 is too large (70 MB, max 64 MB)"),
        ExternalAgentFailureCategory::Input
    );
    assert_eq!(
        classify_external_agent_user_failure(
            "Failed to write Claude MCP config: No space left on device"
        ),
        ExternalAgentFailureCategory::LocalEnvironment
    );

    let codex_auth = actionable_external_agent_failure(
        FlowPilotAgentBackendKind::Codex,
        "turn failed: token_invalidated",
    );
    assert!(codex_auth.contains("needs you to sign in again"));
    assert!(codex_auth.contains("`codex login`"));
    assert!(codex_auth.contains("`codex login status`"));
    assert!(codex_auth.contains("Technical details"));

    let claude_api_key = actionable_external_agent_failure(
        FlowPilotAgentBackendKind::ClaudeCode,
        "Invalid API key from ANTHROPIC_API_KEY",
    );
    assert!(claude_api_key.contains("ANTHROPIC_API_KEY"));
    assert!(
        claude_api_key.contains("update or unset") || claude_api_key.contains("Update or unset")
    );

    let claude_missing = actionable_external_agent_failure(
        FlowPilotAgentBackendKind::ClaudeCode,
        "Claude Code CLI was not found.",
    );
    assert!(claude_missing.contains("`claude --version`"));
    assert!(claude_missing.contains("CLAUDE_CODE_CLI_PATH"));

    let cancelled = actionable_external_agent_failure(
        FlowPilotAgentBackendKind::Codex,
        "FlowPilot external agent run was cancelled",
    );
    assert!(cancelled.contains("FlowPilot run was cancelled"));
    assert!(!cancelled.contains("codex --version"));

    let host_limit = actionable_external_agent_failure(
        FlowPilotAgentBackendKind::ClaudeCode,
        "NESTED_RUN_WALL_CLOCK_BUDGET_EXHAUSTED: compiler diagnostics retained",
    );
    assert!(host_limit.contains("stopped before completing"));
    assert!(host_limit.contains("CLI installation and sign-in do not need to be changed"));

    let bad_image = actionable_external_agent_failure(
        FlowPilotAgentBackendKind::Codex,
        "Failed to decode prompt image 1: invalid base64",
    );
    assert!(bad_image.contains("could not use an attached image"));
    assert!(!bad_image.contains("codex --version"));
}

#[test]
fn exact_source_recovery_seeds_authoritative_loop_coordinates_only() {
    use flow_like::flow::copilot::{
        FlowIrDraftRecoveryStatus, FlowScriptDraftRecovery, FlowScriptEditableDraftContext,
    };

    let context = FlowScriptEditableDraftContext {
        board_id: "recovery-board".into(),
        draft_id: "exact-source".into(),
        revision: 7,
        status: "validation_errors".into(),
        base_fingerprint: "base-1".into(),
        source: Some("eventsSimple() { brokenCall() }".into()),
        diagnostics: Vec::new(),
        checked: false,
        stale_board: false,
    };
    let exact = FlowScriptDraftRecovery {
        status: FlowIrDraftRecoveryStatus::ExactMatch,
        auto_resume: true,
        exact_match: Some(context.clone()),
        conflicting_draft: None,
        next_actions: vec!["resume_exact_flowscript_draft".into()],
        message: "resume".into(),
    };
    let state = Arc::new(StdMutex::new(
        WorkflowToolLoopState::from_flowscript_recovery(Some(&exact)),
    ));
    {
        let guard = state.lock().expect("state lock");
        assert_eq!(guard.mutation_path, Some(WorkflowMutationPath::FlowScript));
        assert_eq!(guard.flowscript_draft_id.as_deref(), Some("exact-source"));
        assert_eq!(guard.flowscript_revision, Some(7));
        assert_eq!(
            guard.last_flowscript.as_deref(),
            Some("eventsSimple() { brokenCall() }")
        );
    }

    let wrong = workflow_tool_preflight_with_args(
        &state,
        "patch_flowscript",
        &serde_json::json!({
            "draft_id": "different-source",
            "expected_revision": 7,
            "old_text": "brokenCall",
            "new_text": "logInfo"
        }),
    )
    .expect("a different source session must be rejected before dispatch");
    let payload = workflow_call_result_json(&wrong);
    assert_eq!(payload["code"], "FLOWSCRIPT_RETAINED_REVISION_REQUIRED");
    assert_eq!(payload["draft_id"], "exact-source");
    assert_eq!(payload["expected_revision"], 7);

    let stale = FlowScriptDraftRecovery {
        auto_resume: false,
        exact_match: Some(FlowScriptEditableDraftContext {
            stale_board: true,
            ..context.clone()
        }),
        ..exact.clone()
    };
    let stale_state = WorkflowToolLoopState::from_flowscript_recovery(Some(&stale));
    assert!(!stale_state.flowscript_draft_retained);
    assert!(stale_state.flowscript_draft_id.is_none());

    let mismatch = FlowScriptDraftRecovery {
        status: FlowIrDraftRecoveryStatus::RequestMismatch,
        auto_resume: false,
        exact_match: None,
        conflicting_draft: Some(FlowScriptEditableDraftContext {
            source: None,
            ..context
        }),
        next_actions: vec!["begin_separate_draft_for_current_request".into()],
        message: "belongs to another request".into(),
    };
    let mismatch_state = WorkflowToolLoopState::from_flowscript_recovery(Some(&mismatch));
    assert!(!mismatch_state.flowscript_draft_retained);
    assert!(mismatch_state.flowscript_draft_id.is_none());
    assert!(mismatch_state.last_flowscript.is_none());
}

#[test]
fn source_operation_budget_counts_writes_patches_and_checks() {
    let state = Arc::new(StdMutex::new(WorkflowToolLoopState {
        initial_declaration_lookup_complete: true,
        mutation_path: Some(WorkflowMutationPath::FlowScript),
        flowscript_draft_id: Some("bounded-source".to_string()),
        flowscript_draft_retained: true,
        flowscript_revision: Some(0),
        last_status: Some("validation_errors".to_string()),
        ..Default::default()
    }));
    for attempt in 0..MAX_EXTERNAL_FLOWSCRIPT_OPERATION_ATTEMPTS {
        assert!(
            workflow_tool_preflight_with_args(
                &state,
                "patch_flowscript",
                &serde_json::json!({
                    "draft_id": "bounded-source",
                    "expected_revision": 0,
                    "edits": []
                }),
            )
            .is_none(),
            "dispatched source operation {attempt} should remain inside the hard budget"
        );
        workflow_tool_abort(&state, "patch_flowscript", "transient worker failure");
    }
    assert_eq!(
        state
            .lock()
            .expect("state lock")
            .flowscript_operation_attempts,
        MAX_EXTERNAL_FLOWSCRIPT_OPERATION_ATTEMPTS
    );
    let exhausted = workflow_tool_preflight_with_args(
        &state,
        "write_flowscript",
        &serde_json::json!({
            "draft_id": "bounded-source",
            "source": "eventsSimple() {}"
        }),
    )
    .expect("the operation after the hard source budget must be rejected");
    let text = exhausted
        .content
        .iter()
        .filter_map(|content| match &content.raw {
            rmcp::model::RawContent::Text(text) => Some(text.text.as_str()),
            _ => None,
        })
        .collect::<Vec<_>>()
        .join("\n");
    assert!(text.contains("FLOWSCRIPT_OPERATION_BUDGET_EXHAUSTED"));
}

#[test]
fn flowscript_repair_history_detects_a_b_a_cycles() {
    let mut state = WorkflowToolLoopState::default();
    for diagnostic in ["diagnostic A", "diagnostic B", "diagnostic A"] {
        state.last_structured_diagnostics.clear();
        state.record_flowscript_repair_progress(
            Some("validation_errors"),
            &[diagnostic.to_string()],
            true,
        );
    }
    assert_eq!(
        state.stalled_edit_attempts, 1,
        "returning to an older compiler state is not fresh progress"
    );
}

#[test]
fn decreasing_grouped_diagnostic_occurrences_count_as_repair_progress() {
    let mut state = WorkflowToolLoopState::default();
    for occurrences in [8, 6, 4] {
        state.last_structured_diagnostics = vec![serde_json::json!({
            "code": "FS_INPUT_PIN_NOT_FOUND",
            "message": "boolOr has no input pin named a",
            "pin": "a",
            "occurrences": occurrences
        })];
        state.record_flowscript_repair_progress(
            Some("validation_errors"),
            &["[FS_INPUT_PIN_NOT_FOUND] boolOr has no input pin named a".to_string()],
            true,
        );
        assert_eq!(state.stalled_edit_attempts, 0);
    }

    state.record_flowscript_repair_progress(
        Some("validation_errors"),
        &["[FS_INPUT_PIN_NOT_FOUND] boolOr has no input pin named a".to_string()],
        true,
    );
    assert_eq!(state.stalled_edit_attempts, 1);
}

#[test]
fn continuation_diagnostics_keep_root_repair_fields_and_drop_cascades() {
    let payload = serde_json::json!({
        "status": "validation_errors",
        "diagnostics": [
            {
                "id": "root-1",
                "code": "FS_PIN_TYPE",
                "message": "pin has the wrong type",
                "source_span": { "line": 4, "column": 9 },
                "pin": "message",
                "expected": "string",
                "actual": "Struct",
                "caused_by": [],
                "fix": { "summary": "Use the declared text output." }
            },
            {
                "id": "cascade-1",
                "code": "FS_EXECUTION_TAIL",
                "message": "execution continuation is missing",
                "caused_by": "root-1"
            }
        ]
    });
    assert_eq!(
        workflow_result_diagnostics(Some(&payload)),
        ["[FS_PIN_TYPE] pin has the wrong type"]
    );
    let structured = workflow_result_structured_diagnostics(Some(&payload));
    assert_eq!(structured.len(), 1);
    assert_eq!(structured[0]["pin"], "message");
    assert_eq!(structured[0]["expected"], "string");
    assert_eq!(structured[0]["actual"], "Struct");
    assert_eq!(structured[0]["source_span"]["line"], 4);
    assert_eq!(
        structured[0]["fix"]["summary"],
        "Use the declared text output."
    );
}

#[test]
fn repair_declarations_survive_transient_source_failures_and_clear_on_valid() {
    let exact =
            "declare function emailSmtpSend({ connection: Struct, from: string, to: string, bodyText: string }): void;"
                .to_string();
    let state = Arc::new(StdMutex::new(WorkflowToolLoopState {
        last_repair_declarations: vec![exact.clone()],
        ..Default::default()
    }));

    workflow_tool_record(
        &state,
        "patch_flowscript",
        &serde_json::json!({ "draft_id": "support-flow", "expected_revision": 3 }),
        &serde_json::json!({
            "status": "revision_conflict",
            "message": "The retained revision advanced before this patch."
        })
        .to_string(),
    );
    assert_eq!(
        state.lock().expect("state lock").last_repair_declarations,
        [exact],
        "a transient response without replacement fixes must retain exact repair signatures"
    );

    workflow_tool_record(
        &state,
        "check_flowscript",
        &serde_json::json!({ "draft_id": "support-flow", "expected_revision": 4 }),
        &serde_json::json!({ "status": "valid", "diagnostics": [] }).to_string(),
    );
    assert!(
        state
            .lock()
            .expect("state lock")
            .last_repair_declarations
            .is_empty(),
        "a valid source no longer needs stale repair signatures"
    );
}

#[test]
fn source_lifecycle_results_retain_exact_revision_for_repair() {
    let state = Arc::new(StdMutex::new(WorkflowToolLoopState::default()));
    assert!(
        workflow_tool_preflight_with_args(
            &state,
            "get_declarations",
            &serde_json::json!({ "queries": ["support email workflow"] }),
        )
        .is_none()
    );
    workflow_tool_record(
        &state,
        "get_declarations",
        &serde_json::json!({ "queries": ["support email workflow"] }),
        "declare function emailImapConnect({ host: string }): Struct;",
    );
    accept_single_segment_plan(&state);
    assert!(
        workflow_tool_preflight_with_args(
            &state,
            "write_flowscript",
            &serde_json::json!({
                "draft_id": "support-flow",
                "source": rich_support_flowscript(),
            }),
        )
        .is_none()
    );
    workflow_tool_record(
            &state,
            "write_flowscript",
            &serde_json::json!({
                "draft_id": "support-flow",
                "source": rich_support_flowscript(),
            }),
            &serde_json::json!({
                "status": "validation_errors",
                "draft_id": "support-flow",
                "revision": 0,
                "source": rich_support_flowscript(),
                "diagnostics": [{
                    "code": "FS_UNKNOWN_DECLARATION",
                    "message": "unknown declaration sendReply",
                    "fix": {
                        "summary": "Use the live SMTP declaration.",
                        "catalog_declarations": [
                            "declare function emailSmtpSend(connection: Struct, to: string, bodyText: string): void"
                        ],
                        "companion_declarations": [
                            "declare function emailSmtpConnect(host: string, port: int): (connection: Struct)"
                        ]
                    }
                }]
            })
            .to_string(),
        );

    let snapshot = state.lock().expect("state lock").snapshot();
    assert_eq!(
        snapshot.mutation_path,
        Some(WorkflowMutationPath::FlowScript)
    );
    assert!(snapshot.flowscript_draft_retained);
    assert_eq!(
        snapshot.flowscript_draft_id.as_deref(),
        Some("support-flow")
    );
    assert_eq!(snapshot.flowscript_revision, Some(0));
    assert_eq!(
        snapshot.last_flowscript.as_deref(),
        Some(rich_support_flowscript())
    );
    assert!(snapshot.last_errors[0].contains("FS_UNKNOWN_DECLARATION"));
    assert_eq!(
        snapshot.last_repair_declarations,
        [
            "declare function emailSmtpSend(connection: Struct, to: string, bodyText: string): void",
            "declare function emailSmtpConnect(host: string, port: int): (connection: Struct)",
        ]
    );
}

#[test]
fn checked_valid_source_remains_committable_at_repair_budget_ceiling() {
    let valid = Arc::new(StdMutex::new(WorkflowToolLoopState {
        initial_declaration_lookup_complete: true,
        mutation_path: Some(WorkflowMutationPath::FlowScript),
        edit_attempts: MAX_EXTERNAL_WORKFLOW_EDIT_ATTEMPTS,
        flowscript_draft_id: Some("valid-source".to_string()),
        flowscript_draft_retained: true,
        flowscript_revision: Some(7),
        last_flowscript: Some("eventsSimple() {}".to_string()),
        last_status: Some("valid".to_string()),
        ..Default::default()
    }));
    assert!(
        workflow_tool_preflight_with_args(
            &valid,
            "commit_flowscript",
            &serde_json::json!({
                "draft_id": "valid-source",
                "expected_revision": 7
            }),
        )
        .is_none()
    );

    let invalid = Arc::new(StdMutex::new(WorkflowToolLoopState {
        initial_declaration_lookup_complete: true,
        mutation_path: Some(WorkflowMutationPath::FlowScript),
        edit_attempts: MAX_EXTERNAL_WORKFLOW_EDIT_ATTEMPTS,
        flowscript_draft_id: Some("invalid-source".to_string()),
        flowscript_draft_retained: true,
        flowscript_revision: Some(2),
        last_status: Some("validation_errors".to_string()),
        last_errors: vec!["missing execution edge".to_string()],
        ..Default::default()
    }));
    assert!(workflow_tool_preflight(&invalid, "check_flowscript").is_some());
    assert!(workflow_tool_preflight(&invalid, "commit_flowscript").is_some());
}

#[test]
fn transient_commit_store_failures_preserve_valid_revision_for_bounded_retry() {
    let args = serde_json::json!({
        "draft_id": "valid-source",
        "expected_revision": 7
    });
    let state = Arc::new(StdMutex::new(WorkflowToolLoopState {
        initial_declaration_lookup_complete: true,
        mutation_path: Some(WorkflowMutationPath::FlowScript),
        edit_attempts: MAX_EXTERNAL_WORKFLOW_EDIT_ATTEMPTS,
        flowscript_operation_attempts: MAX_EXTERNAL_FLOWSCRIPT_OPERATION_ATTEMPTS,
        flowscript_draft_id: Some("valid-source".to_string()),
        flowscript_draft_retained: true,
        flowscript_revision: Some(7),
        last_flowscript: Some("eventsSimple() {}".to_string()),
        last_status: Some("valid".to_string()),
        ..Default::default()
    }));

    for attempt in 0..MAX_EXTERNAL_FLOWSCRIPT_COMMIT_ATTEMPTS {
        assert!(
            workflow_tool_preflight_with_args(&state, "commit_flowscript", &args).is_none(),
            "valid commit retry {attempt} should bypass the exhausted repair budget"
        );
        workflow_tool_record(
            &state,
            "commit_flowscript",
            &args,
            &serde_json::json!({
                "status": "error",
                "code": "FLOWSCRIPT_DRAFT_STORE_UNAVAILABLE",
                "message": "FlowScript draft store lock is unavailable"
            })
            .to_string(),
        );
        assert_eq!(
            state.lock().expect("state lock").last_status.as_deref(),
            Some("valid")
        );
    }

    let exhausted = workflow_tool_preflight_with_args(&state, "commit_flowscript", &args)
        .expect("the bounded commit retry cap must terminate store failures");
    assert_eq!(
        workflow_call_result_json(&exhausted)["code"],
        "FLOWSCRIPT_COMMIT_RETRY_BUDGET_EXHAUSTED"
    );
}

#[test]
fn retained_source_snapshot_prefers_latest_valid_revision_over_older_failure() {
    let mut state = WorkflowToolLoopState {
        mutation_path: Some(WorkflowMutationPath::FlowScript),
        flowscript_draft_id: Some("latest-source".to_string()),
        flowscript_draft_retained: true,
        flowscript_revision: Some(2),
        last_flowscript: Some("eventsSimple() { logInfo({ message: \"valid\" }) }".to_string()),
        last_status: Some("valid".to_string()),
        ..Default::default()
    };
    state
        .repair_tracker
        .record_failed(rich_support_flowscript());

    let snapshot = state.snapshot();
    assert_eq!(snapshot.last_status.as_deref(), Some("valid"));
    assert_eq!(
        snapshot.last_flowscript.as_deref(),
        Some("eventsSimple() { logInfo({ message: \"valid\" }) }")
    );
}

#[test]
fn workflow_snapshot_retains_a_draft_interrupted_during_validation() {
    let mut state = WorkflowToolLoopState {
        edit_attempts: 1,
        edit_in_flight: true,
        in_flight_flowscript: Some(rich_support_flowscript().to_string()),
        ..Default::default()
    };

    let snapshot = state.snapshot();
    assert_eq!(
        snapshot.last_flowscript.as_deref(),
        Some(rich_support_flowscript())
    );
    assert_eq!(snapshot.last_status.as_deref(), Some("edit_interrupted"));
    assert_eq!(
        snapshot.retained_full_source.as_deref(),
        Some(rich_support_flowscript())
    );

    state.finish_interrupted_phase();
    assert!(!state.edit_in_flight);
    assert!(
        state
            .repair_tracker
            .queued_candidate_regression("eventsSimple() {\n    logInfo({ message: \"test\" })\n}")
            .is_some(),
        "a phase boundary must not allow a tiny valid draft to replace the retained workflow"
    );
    let snapshot = state.snapshot();
    assert_eq!(
        snapshot.last_flowscript.as_deref(),
        Some(rich_support_flowscript())
    );
    assert_eq!(snapshot.last_status.as_deref(), Some("validation_errors"));
    assert!(
        snapshot
            .last_errors
            .iter()
            .any(|error| error.contains("interrupted"))
    );
}

#[test]
fn interrupted_typed_phase_reports_only_proven_recovery_state() {
    let mut retained = WorkflowToolLoopState {
        edit_in_flight: true,
        mutation_path: Some(WorkflowMutationPath::TypedIr),
        typed_draft_id: Some("resume-me".to_string()),
        typed_draft_retained: true,
        typed_revision: Some(7),
        ..Default::default()
    };
    retained.finish_interrupted_phase();
    assert!(retained.last_errors[0].contains("resume-me"));
    assert!(retained.last_errors[0].contains("revision 7"));

    let mut unconfirmed = WorkflowToolLoopState {
        edit_in_flight: true,
        mutation_path: Some(WorkflowMutationPath::TypedIr),
        typed_draft_id: Some("attempted-only".to_string()),
        ..Default::default()
    };
    unconfirmed.finish_interrupted_phase();
    assert!(unconfirmed.last_errors[0].contains("before draft retention could be confirmed"));
    assert!(!unconfirmed.last_errors[0].contains("remains resumable"));
}

#[test]
fn interrupted_source_phase_reports_the_retained_revision() {
    let mut state = WorkflowToolLoopState {
        edit_in_flight: true,
        mutation_path: Some(WorkflowMutationPath::FlowScript),
        flowscript_draft_id: Some("resume-source".to_string()),
        flowscript_draft_retained: true,
        flowscript_revision: Some(5),
        last_flowscript: Some(rich_support_flowscript().to_string()),
        ..Default::default()
    };
    state.finish_interrupted_phase();
    assert!(state.last_errors[0].contains("resume-source"));
    assert!(state.last_errors[0].contains("revision 5"));
}

#[test]
fn mcp_marks_semantic_edit_failures_as_errors() {
    for status in ["validation_errors", "no_changes", "error"] {
        let result = flowpilot_tool_result_to_mcp(copilot_sdk::ToolResultObject::text(
            serde_json::json!({ "status": status, "next_action": "revise_and_resubmit" })
                .to_string(),
        ));
        assert_eq!(
            result.is_error,
            Some(true),
            "semantic status {status} must be a red MCP tool result"
        );
    }

    let queued = flowpilot_tool_result_to_mcp(copilot_sdk::ToolResultObject::text(
        serde_json::json!({ "status": "queued", "next_action": "stop" }).to_string(),
    ));
    assert_eq!(queued.is_error, Some(false));
}

#[test]
fn provider_exit_recovery_only_uses_successful_mutating_platform_tools() {
    assert!(is_recoverable_platform_mutation("flowpilot_board"));
    assert!(is_recoverable_platform_mutation("create_app"));
    assert!(is_recoverable_platform_mutation("apply_home_layout"));
    assert!(!is_recoverable_platform_mutation("list_apps"));
    assert!(!is_recoverable_platform_mutation("validate_home_layout"));
    assert!(!is_recoverable_platform_mutation("get_declarations"));

    let success = copilot_sdk::ToolResultObject::text(
        serde_json::json!({ "status": "ok", "applied_commands": 65 }).to_string(),
    );
    assert!(!flowpilot_tool_result_is_error(&success));

    let staged_home = copilot_sdk::ToolResultObject::text(
        serde_json::json!({ "status": "staged", "changed": true }).to_string(),
    );
    assert!(!flowpilot_tool_result_is_error(&staged_home));

    let stale_home = copilot_sdk::ToolResultObject::text(
        serde_json::json!({ "status": "stale", "code": "home_layout_changed" }).to_string(),
    );
    assert!(flowpilot_tool_result_is_error(&stale_home));

    let failure = copilot_sdk::ToolResultObject::text(
        serde_json::json!({ "status": "validation_errors" }).to_string(),
    );
    assert!(flowpilot_tool_result_is_error(&failure));
}

#[test]
fn denied_home_apply_is_an_error_and_never_records_successful_recovery() {
    let activity = Arc::new(StdMutex::new(McpToolActivityState::default()));
    for status in [
        "denied",
        "declined",
        "approval_required",
        "DENIED",
        "cancelled",
    ] {
        let result = copilot_sdk::ToolResultObject::text(
            serde_json::json!({
                "status": status,
                "tool": "apply_home_layout",
                "message": "The Home draft was not approved."
            })
            .to_string(),
        );
        assert!(flowpilot_tool_result_is_error(&result), "{status}");
        record_recoverable_platform_mutation(&activity, "apply_home_layout", &result);
        assert!(
            activity.lock().unwrap().last_successful_mutation.is_none(),
            "{status}"
        );
        assert_eq!(
            flowpilot_tool_result_to_mcp(result).is_error,
            Some(true),
            "{status}"
        );
    }

    let staged = copilot_sdk::ToolResultObject::text(
        serde_json::json!({ "status": "staged", "changed": true }).to_string(),
    );
    record_recoverable_platform_mutation(&activity, "apply_home_layout", &staged);
    let recorded = activity
        .lock()
        .unwrap()
        .last_successful_mutation
        .clone()
        .unwrap();
    assert_eq!(recorded.tool_name, "apply_home_layout");
    assert_eq!(recorded.result_text, staged.text_result_for_llm);
}

#[test]
fn recovered_mutation_message_preserves_result_and_redacts_secrets() {
    let completion = McpToolCompletion {
        tool_name: "flowpilot_board".to_string(),
        result_text: serde_json::json!({
            "status": "ok",
            "message": "Workflow persisted",
            "applied_commands": 65,
            "password": "must-not-leak",
        })
        .to_string(),
    };

    let message = render_recovered_mutation_message(&completion);
    assert!(message.contains("flowpilot_board"));
    assert!(message.contains("completed successfully"));
    assert!(message.contains("Workflow persisted"));
    assert!(message.contains("<redacted>"));
    assert!(!message.contains("must-not-leak"));
}
