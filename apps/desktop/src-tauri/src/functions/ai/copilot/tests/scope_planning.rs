use super::*;

/// The gate that fixes the observed zero-progress failure: declarations first, then the plan,
/// then source. Planning before the catalog is known produces unbuildable segments; planning
/// after the first write is too late to keep that write small.
#[test]
fn the_first_source_write_is_gated_on_declarations_then_a_scope_plan() {
    let state = Arc::new(StdMutex::new(WorkflowToolLoopState::default()));
    let write_args = serde_json::json!({
        "draft_id": "ordered-gates",
        "source": "eventsSimple() { logInfo({ message: \"hi\" }) }"
    });

    let declarations_first =
        workflow_tool_preflight_with_args(&state, "write_flowscript", &write_args)
            .expect("declarations gate the first write");
    assert_eq!(
        workflow_call_result_json(&declarations_first)["status"],
        "declaration_lookup_required"
    );

    let lookup = serde_json::json!({ "queries": ["log information"] });
    assert!(workflow_tool_preflight_with_args(&state, "get_declarations", &lookup).is_none());
    workflow_tool_record(
        &state,
        "get_declarations",
        &lookup,
        "declare function logInfo({ message: string }): void;  // impure",
    );

    let plan_next = workflow_tool_preflight_with_args(&state, "write_flowscript", &write_args)
        .expect("the plan gates the first write once declarations are usable");
    let plan_next = workflow_call_result_json(&plan_next);
    assert_eq!(plan_next["code"], "SCOPE_PLAN_REQUIRED");
    assert_eq!(plan_next["next_action"], "plan_board_scope");
    assert_eq!(plan_next["retryable"], true);

    accept_single_segment_plan(&state);
    assert!(
        workflow_tool_preflight_with_args(&state, "write_flowscript", &write_args).is_none(),
        "declarations plus an accepted plan dispatch the first write"
    );
}

#[test]
fn scope_plans_reject_stub_segments_forward_dependencies_and_strategy_mismatches() {
    let state = Arc::new(StdMutex::new(WorkflowToolLoopState::default()));

    let stub = accept_plan(
        &state,
        serde_json::json!({
            "strategy": "staged",
            "segments": [
                { "id": "s1", "title": "Parser", "behavior": "Parse player commands into verbs and nouns." },
                { "id": "s2", "title": "Rest", "behavior": "TODO later" }
            ]
        }),
    );
    assert_eq!(stub["code"], "SCOPE_PLAN_SEGMENT_NOT_CONCRETE");
    assert_eq!(stub["retryable"], true);

    let forward = accept_plan(
        &state,
        serde_json::json!({
            "strategy": "staged",
            "segments": [
                { "id": "s1", "title": "A", "behavior": "Build and wire the world state model.", "depends_on": ["s2"] },
                { "id": "s2", "title": "B", "behavior": "Build and wire the room transition table." }
            ]
        }),
    );
    assert_eq!(forward["code"], "SCOPE_PLAN_CYCLE");

    let mismatch = accept_plan(
        &state,
        serde_json::json!({
            "strategy": "single",
            "segments": [
                { "id": "s1", "title": "A", "behavior": "Build and wire the world state model." },
                { "id": "s2", "title": "B", "behavior": "Build and wire the room transition table." }
            ]
        }),
    );
    assert_eq!(mismatch["code"], "SCOPE_PLAN_STRATEGY_MISMATCH");

    assert!(
        state.lock().expect("state lock").scope_plan.is_none(),
        "a rejected plan never becomes the run's plan"
    );
}

/// Boards of one app cannot call each other, so a new board is only ever allocated for an
/// independent entry point — never as a way to split one connected workflow.
#[test]
fn only_multi_board_plans_may_allocate_a_new_board() {
    let state = Arc::new(StdMutex::new(WorkflowToolLoopState::default()));

    let staged_new_board = accept_plan(
        &state,
        serde_json::json!({
            "strategy": "staged",
            "segments": [
                { "id": "s1", "title": "A", "behavior": "Build and wire the ingest workflow." },
                { "id": "s2", "title": "B", "behavior": "Build and wire the digest workflow.", "board_ref": "new:digest" }
            ]
        }),
    );
    assert_eq!(staged_new_board["code"], "SCOPE_PLAN_INVALID_BOARD_REF");

    let no_new_board = accept_plan(
        &state,
        serde_json::json!({
            "strategy": "multi_board",
            "segments": [
                { "id": "s1", "title": "A", "behavior": "Build and wire the ingest workflow." },
                { "id": "s2", "title": "B", "behavior": "Build and wire the digest workflow." }
            ]
        }),
    );
    assert_eq!(no_new_board["code"], "SCOPE_PLAN_STRATEGY_MISMATCH");

    let accepted = accept_plan(
        &state,
        serde_json::json!({
            "strategy": "multi_board",
            "segments": [
                { "id": "s1", "title": "A", "behavior": "Build and wire the ingest workflow." },
                { "id": "s2", "title": "B", "behavior": "Build and wire the digest workflow.", "board_ref": "new:digest" }
            ]
        }),
    );
    assert_eq!(accepted["status"], "scope_plan_accepted");
}

/// A segmented build pays the write/check cycle once per segment. Flat budgets would starve it
/// halfway through a plan the host itself asked for.
#[test]
fn segmented_plans_earn_bounded_budget_and_wall_clock_headroom() {
    let unplanned = WorkflowToolLoopState::default();
    assert_eq!(
        unplanned.flowscript_operation_budget(),
        MAX_EXTERNAL_FLOWSCRIPT_OPERATION_ATTEMPTS
    );
    assert_eq!(
        unplanned.edit_attempt_budget(),
        MAX_EXTERNAL_WORKFLOW_EDIT_ATTEMPTS
    );
    assert_eq!(unplanned.wall_clock_extension(), Duration::ZERO);

    let state = Arc::new(StdMutex::new(WorkflowToolLoopState::default()));
    assert_eq!(
        accept_plan(&state, staged_plan_args(4))["status"],
        "scope_plan_accepted"
    );
    let guard = state.lock().expect("state lock");
    assert_eq!(
        guard.flowscript_operation_budget(),
        MAX_EXTERNAL_FLOWSCRIPT_OPERATION_ATTEMPTS + 3 * EXTERNAL_SEGMENT_OPERATION_ALLOWANCE
    );
    assert_eq!(
        guard.edit_attempt_budget(),
        MAX_EXTERNAL_WORKFLOW_EDIT_ATTEMPTS + 3 * EXTERNAL_SEGMENT_CHECK_ALLOWANCE
    );
    assert_eq!(
        guard.wall_clock_extension(),
        EXTERNAL_SEGMENT_WALL_CLOCK_ALLOWANCE * 3
    );
    assert!(
        NESTED_RUN_WALL_CLOCK_BUDGET + guard.wall_clock_extension()
            <= MAX_EXTERNAL_SEGMENTED_WALL_CLOCK_BUDGET
    );
}

#[test]
fn segmented_budget_headroom_is_capped_at_the_largest_allowed_plan() {
    let state = Arc::new(StdMutex::new(WorkflowToolLoopState::default()));
    assert_eq!(
        accept_plan(&state, staged_plan_args(MAX_BOARD_SCOPE_SEGMENTS))["status"],
        "scope_plan_accepted"
    );
    let guard = state.lock().expect("state lock");
    assert!(
        guard.flowscript_operation_budget() <= MAX_EXTERNAL_SEGMENTED_FLOWSCRIPT_OPERATION_ATTEMPTS
    );
    assert!(guard.edit_attempt_budget() <= MAX_EXTERNAL_SEGMENTED_WORKFLOW_EDIT_ATTEMPTS);
    assert!(
        NESTED_RUN_WALL_CLOCK_BUDGET + guard.wall_clock_extension()
            <= MAX_EXTERNAL_SEGMENTED_WALL_CLOCK_BUDGET,
        "the largest plan still finishes well inside the outer bridge dispatch bound"
    );
}

/// A plan too long to reach one commit inside the nested wall clock is forced to per-segment
/// commits, rather than accepted and then starved at the deadline with nothing applied.
#[test]
fn oversized_staged_plans_are_forced_to_commit_per_segment() {
    let state = Arc::new(StdMutex::new(WorkflowToolLoopState::default()));
    let accepted = accept_plan(
        &state,
        staged_plan_args(FORCED_INCREMENTAL_SEGMENT_THRESHOLD + 1),
    );
    assert_eq!(accepted["strategy"], "incremental");

    let state = Arc::new(StdMutex::new(WorkflowToolLoopState::default()));
    let kept = accept_plan(
        &state,
        staged_plan_args(FORCED_INCREMENTAL_SEGMENT_THRESHOLD),
    );
    assert_eq!(kept["strategy"], "staged");
}

#[test]
fn a_scope_plan_may_be_revised_once_and_never_re_declares_applied_segments() {
    let state = Arc::new(StdMutex::new(WorkflowToolLoopState::default()));
    let original = staged_plan_args(3);
    assert_eq!(
        accept_plan(&state, original.clone())["status"],
        "scope_plan_accepted"
    );

    let repeated = accept_plan(&state, original);
    assert_eq!(repeated["status"], "scope_plan_accepted");
    assert_eq!(repeated["idempotent"], true);
    {
        let guard = state.lock().expect("state lock");
        assert_eq!(
            guard.scope_plan_calls, 1,
            "an equivalent replay is not a plan revision"
        );
        assert_eq!(guard.scope_plan.as_ref().expect("plan").revisions, 0);
    }

    let revised = accept_plan(&state, staged_plan_args(2));
    assert_eq!(revised["status"], "scope_plan_accepted");

    let replayed_revision = accept_plan(&state, staged_plan_args(2));
    assert_eq!(replayed_revision["status"], "scope_plan_accepted");
    assert_eq!(replayed_revision["idempotent"], true);

    let third = workflow_tool_preflight_with_args(&state, "plan_board_scope", &staged_plan_args(4))
        .expect("a third materially different plan call is refused");
    assert_eq!(
        workflow_call_result_json(&third)["code"],
        "SCOPE_PLAN_BUDGET_EXHAUSTED"
    );
}

/// Losing every segment at the deadline is worse than applying the validated prefix. A staged
/// plan that has validated at least one segment degrades instead.
#[test]
fn a_staged_plan_commits_its_validated_prefix_when_the_wall_clock_runs_short() {
    let state = Arc::new(StdMutex::new(WorkflowToolLoopState {
        initial_declaration_lookup_complete: true,
        ..Default::default()
    }));
    assert_eq!(
        accept_plan(&state, staged_plan_args(3))["status"],
        "scope_plan_accepted"
    );

    {
        let mut guard = state.lock().expect("state lock");
        assert!(
            !guard.request_staged_prefix_commit(),
            "nothing validated yet, so there is no coherent prefix to commit"
        );
        guard.record_staged_segment_validated();
        assert!(guard.request_staged_prefix_commit());
    }

    let write_args = serde_json::json!({
        "draft_id": "staged-prefix",
        "source": "eventsSimple() { logInfo({ message: \"hi\" }) }"
    });
    let redirected = workflow_tool_preflight_with_args(&state, "write_flowscript", &write_args)
        .expect("growing the draft further is redirected to a commit");
    let redirected = workflow_call_result_json(&redirected);
    assert_eq!(redirected["code"], "SCOPE_PLAN_COMMIT_VALIDATED_PREFIX");
    assert_eq!(redirected["next_action"], "commit_flowscript");
    assert_eq!(redirected["segments_validated"], 1);
    assert_eq!(redirected["segments_remaining"], 2);
}

/// A queued commit is real progress. The remaining work has to be reported by name so the
/// caller says "3 of 5 applied" instead of presenting a half-built board as finished.
#[test]
fn a_queued_commit_records_applied_segments_and_names_what_is_missing() {
    let state = Arc::new(StdMutex::new(WorkflowToolLoopState::default()));
    assert_eq!(
        accept_plan(
            &state,
            serde_json::json!({
                "strategy": "incremental",
                "segments": [
                    { "id": "s1", "title": "World state", "behavior": "Build and wire the world state model." },
                    { "id": "s2", "title": "Command parser", "behavior": "Build and wire the player command parser." },
                    { "id": "s3", "title": "Renderer", "behavior": "Build and wire the scene output renderer." }
                ]
            })
        )["status"],
        "scope_plan_accepted"
    );

    state.lock().expect("state lock").record_scope_plan_commit();
    let guard = state.lock().expect("state lock");
    let plan = guard.scope_plan.as_ref().expect("plan");
    assert_eq!(plan.committed, ["s1"]);
    assert_eq!(
        plan.uncommitted_titles(),
        ["Command parser", "Renderer"],
        "a per-segment commit applies exactly one segment"
    );

    let summary = workflow_run_summary_scope_plan(plan);
    assert_eq!(summary["segments_applied"], 1);
    assert_eq!(summary["segments_remaining"], 2);
}

/// The circling cut-off. Time is bought with evidence, and a run that repeats itself has none,
/// so it is refused at its current deadline exactly as before this existed.
#[test]
fn time_is_earned_by_progress_and_refused_to_a_run_that_repeats_itself() {
    let mut state = WorkflowToolLoopState::default();

    assert_eq!(
        state.try_grant_time_extension(),
        TimeExtensionDecision::NoProgress {
            earned: Duration::ZERO
        },
        "a run that has produced nothing cannot buy time"
    );

    record_forward_progress(&mut state, "eventsSimple() { logInfo({ message: \"a\" }) }");
    assert_eq!(
        state.try_grant_time_extension(),
        TimeExtensionDecision::Granted {
            earned: EXTERNAL_TIME_EXTENSION_SLICE,
            grants: 1,
        }
    );

    assert_eq!(
        state.try_grant_time_extension(),
        TimeExtensionDecision::NoProgress {
            earned: EXTERNAL_TIME_EXTENSION_SLICE
        },
        "the same ledger cannot be spent twice"
    );

    record_forward_progress(
        &mut state,
        "eventsSimple() { logInfo({ message: \"a\" }) logInfo({ message: \"b\" }) }",
    );
    assert_eq!(
        state.try_grant_time_extension(),
        TimeExtensionDecision::Granted {
            earned: EXTERNAL_TIME_EXTENSION_SLICE * 2,
            grants: 2,
        }
    );
}

/// Reaching a NEW compiler state is progress; revisiting an old one is not. This is what
/// separates a long repair from a loop.
#[test]
fn a_new_compiler_state_is_progress_but_a_repeated_one_is_not() {
    let mut state = WorkflowToolLoopState::default();
    state
        .flowscript_seen_repair_signatures
        .insert("first-diagnostic".to_string());
    assert!(matches!(
        state.try_grant_time_extension(),
        TimeExtensionDecision::Granted { .. }
    ));

    // Re-seeing the same state leaves the set unchanged, so the mark does not advance.
    state
        .flowscript_seen_repair_signatures
        .insert("first-diagnostic".to_string());
    assert!(matches!(
        state.try_grant_time_extension(),
        TimeExtensionDecision::NoProgress { .. }
    ));

    state
        .flowscript_seen_repair_signatures
        .insert("second-diagnostic".to_string());
    assert!(matches!(
        state.try_grant_time_extension(),
        TimeExtensionDecision::Granted { .. }
    ));
}

#[test]
fn earned_time_stops_at_the_ceiling_however_much_progress_is_made() {
    let mut state = WorkflowToolLoopState::default();
    let mut grants = 0u32;
    loop {
        record_forward_progress(&mut state, &"x".repeat(grants as usize + 1));
        match state.try_grant_time_extension() {
            TimeExtensionDecision::Granted { .. } => grants += 1,
            TimeExtensionDecision::CeilingReached { earned } => {
                assert!(NESTED_RUN_WALL_CLOCK_BUDGET + earned <= MAX_EXTERNAL_EARNED_WALL_CLOCK);
                break;
            }
            other => panic!("unexpected decision with fresh progress: {other:?}"),
        }
        assert!(grants < 1_000, "the ceiling must terminate this loop");
    }
    assert!(grants > 0);
    assert!(
        NESTED_RUN_WALL_CLOCK_BUDGET + state.wall_clock_extension()
            <= MAX_EXTERNAL_EARNED_WALL_CLOCK
    );
}

/// Wall clock alone is useless: the count budgets would end a productive run inside the first
/// hour, so each earned slice has to raise them too.
#[test]
fn each_earned_slice_also_raises_the_volume_budgets() {
    let mut state = WorkflowToolLoopState::default();
    let base_ops = state.flowscript_operation_budget();
    let base_checks = state.edit_attempt_budget();
    let base_commits = state.commit_attempt_budget();
    let base_continuations = state.continuation_budget();

    record_forward_progress(&mut state, "some source");
    assert!(matches!(
        state.try_grant_time_extension(),
        TimeExtensionDecision::Granted { .. }
    ));

    assert_eq!(
        state.flowscript_operation_budget(),
        base_ops + EXTERNAL_EXTENSION_OPERATION_GRANT
    );
    assert_eq!(
        state.edit_attempt_budget(),
        base_checks + EXTERNAL_EXTENSION_CHECK_GRANT
    );
    assert_eq!(
        state.commit_attempt_budget(),
        base_commits + EXTERNAL_EXTENSION_COMMIT_GRANT
    );
    assert_eq!(
        state.continuation_budget(),
        base_continuations + EXTERNAL_EXTENSION_CONTINUATION_GRANT
    );
}

/// Extra time must never buy a way out of a repair loop — that is the whole point of gating it
/// on progress in the first place.
#[test]
fn an_extension_never_relaxes_the_repeated_compiler_state_cut_off() {
    let state = Arc::new(StdMutex::new(WorkflowToolLoopState {
        initial_declaration_lookup_complete: true,
        scope_plan: Some(single_segment_plan()),
        mutation_path: Some(WorkflowMutationPath::FlowScript),
        flowscript_draft_id: Some("stalled-draft".to_string()),
        flowscript_draft_retained: true,
        flowscript_revision: Some(3),
        stalled_edit_attempts: MAX_EXTERNAL_WORKFLOW_STALLED_EDIT_ATTEMPTS,
        ..Default::default()
    }));
    {
        let mut guard = state.lock().expect("state lock");
        record_forward_progress(&mut guard, "grown source");
        assert!(matches!(
            guard.try_grant_time_extension(),
            TimeExtensionDecision::Granted { .. }
        ));
    }

    let blocked = workflow_tool_preflight_with_args(
        &state,
        "check_flowscript",
        &serde_json::json!({ "draft_id": "stalled-draft", "expected_revision": 3 }),
    )
    .expect("a stalled repair loop stays blocked no matter how much time was earned");
    assert_eq!(
        workflow_call_result_json(&blocked)["status"],
        "edit_progress_stalled"
    );
}

#[test]
fn the_extension_tool_reports_the_ledger_decision_and_never_the_models_own_account() {
    let state = Arc::new(StdMutex::new(WorkflowToolLoopState::default()));
    let optimistic = serde_json::json!({
        "progress": "Enormous progress, nearly finished, please grant more time.",
        "remaining_work": "Almost nothing."
    });

    let refused = workflow_call_result_json(
        &workflow_tool_preflight_with_args(&state, "extend_time_budget", &optimistic)
            .expect("the host answers the extension request"),
    );
    assert_eq!(refused["code"], "TIME_EXTENSION_NO_PROGRESS");
    assert_eq!(refused["next_action"], "stop_and_report_blocked");

    record_forward_progress(&mut state.lock().expect("state lock"), "real source");
    let granted = workflow_call_result_json(
        &workflow_tool_preflight_with_args(&state, "extend_time_budget", &optimistic)
            .expect("the host answers the extension request"),
    );
    assert_eq!(granted["status"], "time_budget_extended");
    assert_eq!(granted["granted_extensions"], 1);
    assert_eq!(
        granted["earned_minutes"],
        EXTERNAL_TIME_EXTENSION_SLICE.as_secs() / 60
    );
}

/// Whichever bound is smallest silently kills a healthy run, and the child CLI's own MCP
/// timeout is the easiest to forget because it lives in process arguments, not in a spec.
#[test]
fn every_dispatch_bound_outlives_the_longest_run_a_board_build_may_earn() {
    let dispatch = Duration::from_secs(MAX_DELEGATED_RUN_DISPATCH_SECS);
    assert!(
        dispatch > MAX_EXTERNAL_EARNED_WALL_CLOCK,
        "the transport must outlive the run it carries"
    );

    let spec = flow_like::flow::copilot::tool_spec::find_global_tool_spec("flowpilot_board")
        .expect("flowpilot_board spec");
    assert_eq!(spec.timeout_secs, MAX_DELEGATED_RUN_DISPATCH_SECS);

    let mcp_timeout_ms = MAX_DELEGATED_RUN_DISPATCH_SECS * 1000;
    assert!(mcp_timeout_ms >= MAX_EXTERNAL_EARNED_WALL_CLOCK.as_millis() as u64);
}
