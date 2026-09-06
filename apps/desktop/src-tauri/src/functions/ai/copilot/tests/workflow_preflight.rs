use super::*;

#[test]
fn board_prompt_submits_flowscript_before_database_setup() {
    let prompt = flow_like::copilot::prompts::board_sdk_flowscript_system_prompt("", 0);
    assert!(prompt.contains("database setup is\nnever a prerequisite"));
    assert!(prompt.contains("call `plan_board_scope` exactly once"));
    assert!(prompt.contains("submit its active segment through `write_flowscript`"));
    assert!(prompt.contains("ONE bounded, focused `get_declarations`"));
    assert!(prompt.contains("One such result proves the capability mismatch"));
    assert!(
        prompt.contains(
            "Record any remaining requested schemas as pending and finish/apply the board"
        )
    );
}

#[test]
fn external_workflow_prompt_requires_an_early_retained_checkpoint() {
    let prompt =
        build_external_agent_prompt("system", "build it", CopilotScope::Board, true, false);
    assert!(prompt.contains("ONE bounded, focused get_declarations batch"));
    assert!(prompt.contains("call plan_board_scope exactly ONCE"));
    assert!(prompt.contains("Then call write_flowscript IMMEDIATELY"));
    let declarations = prompt
        .find("ONE bounded, focused get_declarations batch")
        .expect("declaration instruction");
    let plan = prompt
        .find("call plan_board_scope exactly ONCE")
        .expect("scope-plan instruction");
    let write = prompt
        .find("Then call write_flowscript IMMEDIATELY")
        .expect("source-write instruction");
    assert!(declarations < plan && plan < write);
    assert!(prompt.contains("at most six ancillary"));
    assert!(prompt.contains("It may retain compiler diagnostics"));
    assert!(!prompt.contains("every required catalog-signature search"));
}

#[test]
fn predraft_checkpoint_waits_for_declarations_and_an_accepted_scope_plan() {
    let mut state = WorkflowToolLoopState::default();
    assert_eq!(
        workflow_initial_source_checkpoint_phase(&state),
        InitialSourceCheckpointPhase::AwaitingPrerequisites
    );
    state.initial_declaration_lookup_usable = true;
    assert_eq!(
        workflow_initial_source_checkpoint_phase(&state),
        InitialSourceCheckpointPhase::AwaitingPrerequisites,
        "usable declarations alone must not start the source-writing clock"
    );
    state.scope_plan = Some(single_segment_plan());
    assert_eq!(
        workflow_initial_source_checkpoint_phase(&state),
        InitialSourceCheckpointPhase::AwaitingInitialSource
    );
    // An unrelated position/comment operation must not permanently disarm the source
    // checkpoint. Source/typed operation counters are the authoritative transition.
    state.edit_in_flight = true;
    assert_eq!(
        workflow_initial_source_checkpoint_phase(&state),
        InitialSourceCheckpointPhase::AwaitingInitialSource
    );
    state.flowscript_operation_attempts = 1;
    assert_eq!(
        workflow_initial_source_checkpoint_phase(&state),
        InitialSourceCheckpointPhase::Complete
    );
}

#[test]
fn predraft_checkpoint_pauses_for_an_admitted_ancillary_context_read() {
    let board = flowscript_recovery_test_board();
    let manifest = BoardContextManifest::from_board(
        &board,
        &[],
        &[],
        ManifestSource::absent(),
        ManifestAudit::default(),
        ManifestAugmentations::default(),
        default_flowscript_module_templates(),
    )
    .expect("checkpoint test manifest");
    let mut loop_state = WorkflowToolLoopState {
        initial_declaration_lookup_usable: true,
        scope_plan: Some(single_segment_plan()),
        ..Default::default()
    };
    loop_state.attach_shared_session(Some(manifest));
    let state = Arc::new(StdMutex::new(loop_state));
    let args = serde_json::json!({
        "operation": "get_page",
        "page_id": "page-under-inspection"
    });

    let admitted = workflow_predraft_context_preflight_with_lease(&state, "ui_inspect", &args);
    assert!(admitted.result.is_none());
    assert!(admitted.lease.is_some());
    assert_eq!(
        workflow_initial_source_checkpoint_phase(&state.lock().expect("state lock")),
        InitialSourceCheckpointPhase::AncillaryContextInFlight,
        "the source checkpoint must defer to the admitted inspector's own deadline"
    );

    workflow_tool_abort_with_args(
        &state,
        admitted.lease.as_ref(),
        "ui_inspect",
        &args,
        "test inspector finished",
    );
    assert_eq!(
        workflow_initial_source_checkpoint_phase(&state.lock().expect("state lock")),
        InitialSourceCheckpointPhase::AwaitingInitialSource,
        "the source-writing window starts only after the inspector lease settles"
    );
}

#[test]
fn continuation_plans_then_writes_after_partial_but_usable_declaration_coverage() {
    let snapshot = WorkflowToolLoopSnapshot {
        last_declarations: Some(
            "declare function emailImapConnect({ host: string }): (connection: Struct);"
                .to_string(),
        ),
        declaration_lookup_complete: false,
        unresolved_declaration_queries: vec!["smtp send".to_string()],
        ..Default::default()
    };
    let prompt =
        build_external_workflow_continuation_prompt("build support mail", Some(&snapshot), 1);
    assert!(prompt.contains("DECLARATIONS ALREADY FETCHED"));
    assert!(
        prompt
            .contains("Call plan_board_scope exactly once, then call write_flowscript immediately")
    );
    assert!(prompt.contains("last status: declarations_ready_no_source"));
    assert!(!prompt.contains("UNRESOLVED DECLARATION COVERAGE"));
    assert!(!prompt.contains("ACCEPTED SCOPE PLAN RETAINED BY THE HOST"));
    let error = external_workflow_incomplete_error(Some(&snapshot), 0);
    assert!(error.contains("last status: declarations_ready_no_source"));
}

#[test]
fn continuation_serializes_the_accepted_plan_and_forbids_replanning() {
    let snapshot = WorkflowToolLoopSnapshot {
        last_declarations: Some("declare function logInfo(...): void".to_string()),
        scope_plan: Some(single_segment_plan()),
        ..Default::default()
    };

    let prompt =
        build_external_workflow_continuation_prompt("build support mail", Some(&snapshot), 1);
    assert!(prompt.contains("ACCEPTED SCOPE PLAN RETAINED BY THE HOST"));
    assert!(prompt.contains("\"active_segment\""));
    assert!(prompt.contains("\"id\": \"s1\""));
    assert!(prompt.contains("\"title\": \"Whole request\""));
    assert!(prompt.contains("DO NOT call plan_board_scope again"));
    assert!(prompt.contains("Call write_flowscript now for the returned active segment"));
    assert!(
        !prompt.contains(
            "Usable declarations are already retained but no scope plan or source exists"
        )
    );
}

#[test]
fn workflow_edit_classifier_allows_read_only_text_answers() {
    for prompt in [
        "explain why this node is not connected to the API Call",
        "what does this FlowScript do?",
        "check if the workflow execution wiring is correct",
        "debug why the For Each loop is not working",
    ] {
        assert!(
            !is_workflow_edit_request(prompt),
            "prompt should stay read-only: {prompt}"
        );
    }
}

#[test]
fn auto_intent_does_not_misclassify_scheduled_check_imperatives_as_questions() {
    for prompt in [
        "check my inbox every hour and notify me",
        "check the API every minute and send an email",
        "Prüfe den Posteingang jede Stunde und sende eine Nachricht",
    ] {
        assert!(
            !is_read_only_workflow_request(&prompt.to_lowercase()),
            "scheduled imperative should enter authoring mode: {prompt}"
        );
    }
    assert!(is_read_only_workflow_request(
        "check why this workflow sends an email"
    ));
}

#[test]
fn workflow_edit_classifier_still_detects_mutations() {
    for prompt in [
        "generate a workflow that fetches the Rust RSS feed",
        "connect the API Call success output to To Text",
        "fix the workflow execution wiring",
        "update this flow to store rows in the database",
        "Bau mir eine App mit IMAP und SMTP für Support-Emails",
        "Erstelle einen Cron-Ablauf und speichere die Ergebnisse in der Datenbank",
        "create a cron event",
        "configure a scheduled trigger",
        "Konfiguriere einen Zeitplan-Auslöser",
    ] {
        assert!(
            is_workflow_edit_request(prompt),
            "prompt should be treated as workflow edit: {prompt}"
        );
    }
}

#[test]
fn workflow_edit_classifier_keeps_german_explanations_read_only() {
    for prompt in [
        "Erkläre mir diesen Workflow",
        "Warum ist dieser Knoten nicht verbunden?",
        "Prüfe, warum der Email-Ablauf einen Fehler zeigt",
    ] {
        assert!(
            !is_workflow_edit_request(prompt),
            "German explanation should stay read-only: {prompt}"
        );
    }
}

#[test]
fn host_unified_wrapper_does_not_turn_raw_ui_request_into_workflow_edit() {
    let raw = "Create a settings page with a form and explain the current button labels";
    let wrapped = format!("[UNIFIED MODE - generate workflow nodes and UI components.]\n\n{raw}");

    assert!(is_workflow_edit_request(&wrapped));
    assert!(!is_workflow_edit_request(raw));
}
