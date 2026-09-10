use super::*;

use flow_like::flow::ast::{RenderOptions, board_to_flowscript};
use flow_like::flow::copilot::{
    CheckFlowScriptArgs, CommitFlowScriptArgs, FlowIrAcceptanceBinding, FlowScriptPendingDelivery,
    NodeMetadata, PatchFlowScriptArgs, WriteFlowScriptArgs,
};

struct ObservedBoard {
    id: String,
    run: Arc<Mutex<RunObservation>>,
}

impl ObservedBoard {
    fn register(board: &Board) -> Self {
        let run = Arc::new(Mutex::new(RunObservation {
            started: Instant::now(),
            prompt_profile: BoardPromptProfile::Legacy,
            attempts: Vec::new(),
            tool_schema_fingerprint: None,
            usage: None,
            tool_calls: Vec::new(),
            tool_calls_truncated: false,
            retained_source: None,
            transport_events: Vec::new(),
            transport_events_truncated: false,
            transport_counts: BTreeMap::new(),
            external_phases: Vec::new(),
            external_phases_truncated: false,
        }));
        assert!(
            RUNS.lock()
                .unwrap()
                .insert(board.id.clone(), run.clone())
                .is_none()
        );
        Self {
            id: board.id.clone(),
            run,
        }
    }
}

impl Drop for ObservedBoard {
    fn drop(&mut self) {
        RUNS.lock().unwrap().remove(&self.id);
    }
}

fn fixture(id: &str) -> WorkflowBenchmarkCase {
    workflow_benchmark_cases()
        .into_iter()
        .find(|case| case.id == id)
        .unwrap()
}

#[test]
fn benchmark_prompt_override_preserves_explicit_legacy_and_focused() {
    let board = Board::new_detached(None, "prompt-profile-test".into());
    assert_eq!(benchmark_prompt_profile_for_board(Some(&board)), None);
    assert_eq!(benchmark_prompt_profile_for_board(None), None);
    let observed = ObservedBoard::register(&board);
    assert_eq!(
        benchmark_prompt_profile_for_board(Some(&board)),
        Some(BoardPromptProfile::Legacy)
    );
    observed.run.lock().unwrap().prompt_profile = BoardPromptProfile::Focused;
    assert_eq!(
        benchmark_prompt_profile_for_board(Some(&board)),
        Some(BoardPromptProfile::Focused)
    );
    drop(observed);
    assert_eq!(benchmark_prompt_profile_for_board(Some(&board)), None);
}

#[tokio::test]
async fn transport_observations_keep_rejections_separate_and_retain_final_phase_state() {
    let board = initial_board(&fixture("profile-card")).await.unwrap();
    let observed = ObservedBoard::register(&board);
    observe_tool_dispatch(&board.id, "mcp", "check_flowscript", "arrival", None, None);
    observe_tool_dispatch(
        &board.id,
        "mcp",
        "check_flowscript",
        "preflight_short_circuit",
        Some("PREFLIGHT_BLOCKED"),
        Some("blocked"),
    );
    let snapshot = |sequence, status| {
        json!({
            "schema": "flowpilot.external-process-phase/v1", "phase_id": "phase-one",
            "observation_sequence": sequence, "terminal_status": status, "prompt_bytes": 100,
        })
    };
    observe_external_phase_state(&board.id, &snapshot(2, "future_dropped"));
    observe_external_phase_state(&board.id, &snapshot(1, "running"));
    let run = observed.run.lock().unwrap();
    assert_eq!(run.transport_counts["mcp.arrival"], 1);
    assert_eq!(run.transport_counts["mcp.preflight_short_circuit"], 1);
    assert!(!run.transport_counts.contains_key("mcp.dispatched"));
    assert!(run.tool_calls.is_empty());
    assert_eq!(run.external_phases.len(), 1);
    assert_eq!(run.external_phases[0]["terminal_status"], "future_dropped");
    drop(run);
    for index in 0..520 {
        observe_tool_dispatch(&board.id, "mcp", "get_declarations", "arrival", None, None);
        let mut phase = snapshot(1, "running");
        phase["phase_id"] = json!(format!("phase-{index}"));
        observe_external_phase_state(&board.id, &phase);
    }
    let run = observed.run.lock().unwrap();
    assert_eq!(run.transport_counts["mcp.arrival"], 521);
    assert_eq!(run.transport_events.len(), 512);
    assert!(run.transport_events_truncated);
    assert_eq!(run.external_phases.len(), 16);
    assert!(run.external_phases_truncated);
}

#[tokio::test]
async fn tool_observation_keeps_handler_results_and_compiler_failures_separate() {
    let board = initial_board(&fixture("profile-card")).await.unwrap();
    let observed = ObservedBoard::register(&board);
    let payload =
        json!({"status": "validation_errors", "diagnostics": [{"code": "FS_UNKNOWN_PIN"}]})
            .to_string();
    let expected = payload.clone();
    let handler: copilot_sdk::ToolHandler =
        Arc::new(move |_, _| copilot_sdk::ToolResultObject::text(payload.clone()));
    let mut tools = vec![(copilot_sdk::Tool::new("write_flowscript"), handler)];
    observe_tools(Some(&board), &mut tools);
    let result = tools[0].1("write_flowscript", &json!({}));
    assert_eq!(result.text_result_for_llm, expected);
    let run = observed.run.lock().unwrap();
    assert_eq!(run.tool_calls.len(), 1);
    assert_eq!(run.tool_calls[0].status, "validation_errors");
    assert_eq!(run.tool_calls[0].diagnostic_codes, ["FS_UNKNOWN_PIN"]);
    assert!(run.tool_calls[0].duration_ms.is_some());
    assert!(run.attempts.is_empty());
}

#[tokio::test]
async fn external_phase_usage_preserves_u64_and_cannot_recover_missing_totals() {
    let board = initial_board(&fixture("profile-card")).await.unwrap();
    let observed = ObservedBoard::register(&board);
    let usage = WorkflowBenchmarkUsage {
        input_tokens: Some(u64::MAX - 1),
        output_tokens: Some(3),
        cached_input_tokens: Some(0),
    };
    observe_external_phase_usage(&board.id, &usage, true);
    assert_eq!(
        observed
            .run
            .lock()
            .unwrap()
            .usage
            .as_ref()
            .unwrap()
            .input_tokens,
        Some(u64::MAX - 1)
    );
    observe_external_phase_usage(&board.id, &usage, false);
    observe_external_phase_usage(&board.id, &usage, true);
    let run = observed.run.lock().unwrap();
    let measured = run.usage.as_ref().unwrap();
    assert_eq!(measured.input_tokens, None);
    assert_eq!(measured.output_tokens, None);
    assert_eq!(measured.cached_input_tokens, None);
}

fn blank_report(
    case: &WorkflowBenchmarkCase,
    observed: &ObservedBoard,
    catalog: &[NodeMetadata],
    repeat_index: u32,
) -> WorkflowBenchmarkRunReport {
    let run = observed.run.lock().unwrap();
    WorkflowBenchmarkRunReport {
        schema: WORKFLOW_BENCHMARK_REPORT_SCHEMA.into(),
        run_id: format!("{}-{repeat_index}", observed.id),
        case_id: case.id.clone(),
        repeat_index,
        cohort: WorkflowBenchmarkCohort {
            provider: "host-test".into(),
            model: "no-model-invoked".into(),
            reasoning_effort: "none".into(),
            prompt_profile: "legacy".into(),
            harness_revision: "host-test-v1".into(),
            fixture_fingerprint: case.fixture_fingerprint(),
            catalog_fingerprint: flowscript_catalog_fingerprint(catalog),
            prompt_fingerprint: fingerprint(&json!(case.public_prompt())),
            tool_schema_fingerprint: "unobserved".into(),
            budget_fingerprint: "host-test-budget".into(),
        },
        status: WorkflowBenchmarkRunStatus::Failed,
        elapsed_ms: run.started.elapsed().as_millis() as u64,
        usage: run.usage.clone(),
        observed_tool_schema_fingerprint: run.tool_schema_fingerprint.clone(),
        tool_calls: run.tool_calls.clone(),
        tool_calls_truncated: run.tool_calls_truncated,
        retained_source: run.retained_source.clone(),
        transport_events: run.transport_events.clone(),
        transport_events_truncated: run.transport_events_truncated,
        transport_counts: run.transport_counts.clone(),
        external_phases: run.external_phases.clone(),
        external_phases_truncated: run.external_phases_truncated,
        attempts: run.attempts.clone(),
        graded_candidate: None,
        checks: Vec::new(),
        grading_errors: Vec::new(),
    }
}

fn write_source(
    store: &FlowIrDraftStore,
    board: &Board,
    catalog: &[NodeMetadata],
    binding: &FlowIrAcceptanceBinding,
    source: &str,
    replace_existing: bool,
) -> FlowScriptDraftResponse {
    let result = store.write_flowscript_with_acceptance_binding(
        board,
        catalog,
        serde_json::from_value::<WriteFlowScriptArgs>(json!({
            "draft_id": "observed-source",
            "source": source,
            "replace_existing": replace_existing,
            "allow_scope_reduction": true,
        }))
        .unwrap(),
        binding,
    );
    observe_source(&board.id, &result);
    result
}

fn retained_delivery(
    case: &WorkflowBenchmarkCase,
    board: &Board,
    catalog: &[NodeMetadata],
    source: &str,
) -> FlowScriptPendingDelivery {
    let store = FlowIrDraftStore::new();
    let binding = store.bind_request_acceptance_contract(&board.id, case.public_prompt());
    let written = write_source(&store, board, catalog, &binding, source, false);
    assert_eq!(written.revision, Some(0), "{written:#?}");
    assert!(written.diagnostics.is_empty(), "{written:#?}");
    let checked = store.check_flowscript_with_acceptance_binding(
        board,
        catalog,
        CheckFlowScriptArgs {
            draft_id: "observed-source".into(),
            expected_revision: 0,
        },
        &binding,
    );
    assert_eq!(checked.status, "valid", "{checked:#?}");
    observe_source(&board.id, &checked);
    let committed = store.commit_flowscript_with_acceptance_binding(
        board,
        catalog,
        serde_json::from_value::<CommitFlowScriptArgs>(json!({
            "draft_id": "observed-source", "expected_revision": 0,
        }))
        .unwrap(),
        &binding,
    );
    assert_eq!(committed.status, "queued", "{committed:#?}");
    observe_source(&board.id, &committed);
    let delivery = store
        .pending_flowscript_delivery_for_binding(board, &binding)
        .expect("the ordinary commit path retains its exact source and command claim");
    assert!(!delivery.stale_board);
    assert_eq!(delivery.source, source);
    delivery
}

async fn graded_source(case: &WorkflowBenchmarkCase, source: &str) -> WorkflowBenchmarkRunReport {
    let board = initial_board(case).await.unwrap();
    graded_board_source(case, board, source).await
}

async fn graded_board_source(
    case: &WorkflowBenchmarkCase,
    board: Board,
    source: &str,
) -> WorkflowBenchmarkRunReport {
    let before = serde_json::to_value(&board).unwrap();
    let observed = ObservedBoard::register(&board);
    let catalog = DesktopCatalogProvider::new(None).all_metadata();
    let delivery = retained_delivery(case, &board, &catalog, source);
    assert_eq!(
        serde_json::to_value(&board).unwrap(),
        before,
        "retained authoring must leave the base board unchanged"
    );
    let mut report = blank_report(case, &observed, &catalog, 0);
    grade_delivery(case, board, delivery, &mut report).await;
    if report.graded_candidate.is_some() && report.checks.is_empty() {
        report.checks = grade_workflow_benchmark_checks(case, &[]).unwrap().checks;
    }
    report.elapsed_ms = observed.run.lock().unwrap().started.elapsed().as_millis() as u64;
    validate_workflow_benchmark_report(case, &report).unwrap();
    report
}

#[test]
fn public_case_list_exposes_only_ids_and_requests() {
    let public = flowpilot_workflow_benchmark_cases();
    let private = workflow_benchmark_cases();
    assert_eq!(public.len(), private.len());
    for (visible, fixture) in public.iter().zip(private) {
        let object = visible.as_object().unwrap();
        assert_eq!(object.len(), 2);
        assert_eq!(object["id"], fixture.id);
        assert_eq!(object["prompt"], fixture.public_prompt());
        for hidden in [
            "checks",
            "reference_source",
            "initial_source",
            "required_functions",
        ] {
            assert!(!object.contains_key(hidden));
        }
    }
}

#[tokio::test]
async fn absent_and_cancelled_candidates_remain_failures_in_the_denominator() {
    let case = fixture("net-total");
    let board = initial_board(&case).await.unwrap();
    let observed = ObservedBoard::register(&board);
    let catalog = DesktopCatalogProvider::new(None).all_metadata();
    let mut absent = blank_report(&case, &observed, &catalog, 0);
    absent
        .grading_errors
        .push("No committed FlowScript candidate was returned".into());
    let mut cancelled = blank_report(&case, &observed, &catalog, 1);
    cancelled.status = WorkflowBenchmarkRunStatus::Cancelled;
    cancelled.grading_errors.push("Generation cancelled".into());
    let score = evaluate_workflow_benchmark_runs(&case, &[absent, cancelled]).unwrap();
    assert_eq!(score.runs_total, 2);
    assert_eq!(score.source_attempts, 0);
    assert_eq!(score.final_behavior_pass.numerator, 0);
    assert_eq!(score.final_behavior_pass.denominator, 2);
    assert_eq!(score.verified_behavior_at_1.denominator, 2);
    assert_eq!(score.input_tokens.samples, 0);
}

#[tokio::test]
async fn source_observation_counts_retained_revisions_and_preserves_compiler_stages() {
    let case = fixture("net-total");
    let board = initial_board(&case).await.unwrap();
    let observed = ObservedBoard::register(&board);
    let catalog = DesktopCatalogProvider::new(None).all_metadata();
    let store = FlowIrDraftStore::new();
    let binding = store.bind_request_acceptance_contract(&board.id, "Return the requested result.");
    let invalid = "eventsGeneric observed() {";
    let first = write_source(&store, &board, &catalog, &binding, invalid, false);
    assert_eq!(first.revision, Some(0), "{first:#?}");
    assert!(!first.diagnostics.is_empty());
    observe_source(&board.id, &first);
    let checked = store.check_flowscript_with_acceptance_binding(
        &board,
        &catalog,
        CheckFlowScriptArgs {
            draft_id: "observed-source".into(),
            expected_revision: 0,
        },
        &binding,
    );
    observe_source(&board.id, &checked);
    assert_eq!(observed.run.lock().unwrap().attempts.len(), 1);
    let repeated = write_source(&store, &board, &catalog, &binding, invalid, true);
    assert_eq!(repeated.revision, Some(1));
    let unresolved = write_source(
        &store,
        &board,
        &catalog,
        &binding,
        "eventsGeneric observed() {\n    const result = benchmarkMissingNode({})\n    return result\n}",
        true,
    );
    assert_eq!(unresolved.revision, Some(2), "{unresolved:#?}");
    assert!(
        unresolved.diagnostics.iter().any(|diagnostic| matches!(
            diagnostic.phase,
            FlowScriptDiagnosticPhase::CatalogResolution | FlowScriptDiagnosticPhase::TypeCheck
        )),
        "{unresolved:#?}"
    );
    let valid = write_source(
        &store,
        &board,
        &catalog,
        &binding,
        "eventsGeneric observed() {\n    return \"ok\"\n}",
        true,
    );
    assert_eq!(valid.revision, Some(3), "{valid:#?}");
    assert!(valid.diagnostics.is_empty(), "{valid:#?}");
    let rejected = store.patch_flowscript_with_acceptance_binding(
        &board,
        &catalog,
        PatchFlowScriptArgs {
            draft_id: "observed-source".into(),
            expected_revision: 3,
            old_text: "text that never occurred".into(),
            new_text: "replacement".into(),
            allow_scope_reduction: false,
        },
        &binding,
    );
    assert_eq!(rejected.status, "error");
    observe_source(&board.id, &rejected);
    let run = observed.run.lock().unwrap();
    assert_eq!(run.attempts.len(), 4);
    assert_eq!(
        run.attempts[0].source_fingerprint,
        run.attempts[1].source_fingerprint
    );
    assert!(!run.attempts[0].parse_valid);
    assert!(!run.attempts[0].typed_valid);
    assert!(!run.attempts[0].reconcile_valid);
    assert!(run.attempts[2].parse_valid);
    assert!(!run.attempts[2].typed_valid);
    assert!(!run.attempts[2].reconcile_valid);
    assert!(
        run.attempts[3].parse_valid
            && run.attempts[3].typed_valid
            && run.attempts[3].reconcile_valid
    );
    assert_eq!(
        run.attempts
            .iter()
            .map(|attempt| attempt.attempt_index)
            .collect::<Vec<_>>(),
        [1, 2, 3, 4]
    );
}

#[tokio::test]
async fn raw_usage_sums_known_counts_and_keeps_missing_measurements_unknown() {
    let board = initial_board(&fixture("net-total")).await.unwrap();
    let observed = ObservedBoard::register(&board);
    assert!(observed.run.lock().unwrap().usage.is_none());
    observe_usage(Some(&board), Some(100.0), Some(20.0), Some(0.0));
    observe_usage(Some(&board), Some(40.0), Some(10.0), None);
    let measured = observed.run.lock().unwrap().usage.clone().unwrap();
    assert_eq!(measured.input_tokens, Some(140));
    assert_eq!(measured.output_tokens, Some(30));
    assert_eq!(measured.cached_input_tokens, None);
    observe_usage(Some(&board), None, Some(5.0), Some(10.0));
    let partial = observed.run.lock().unwrap().usage.clone().unwrap();
    assert_eq!(partial.input_tokens, None);
    assert_eq!(partial.output_tokens, Some(35));
    assert_eq!(partial.cached_input_tokens, None);
    observe_usage(
        Some(&board),
        Some(f64::NAN),
        Some(-1.0),
        Some(f64::INFINITY),
    );
    let invalid = observed.run.lock().unwrap().usage.clone().unwrap();
    assert_eq!(invalid.input_tokens, None);
    assert_eq!(invalid.output_tokens, None);
    assert_eq!(invalid.cached_input_tokens, None);
}

#[tokio::test]
async fn unfinished_sdk_usage_and_inexact_numbers_remain_unknown() {
    let board = initial_board(&fixture("net-total")).await.unwrap();
    let observed = ObservedBoard::register(&board);
    observe_usage(Some(&board), Some(100.0), Some(20.0), Some(0.0));
    invalidate_usage(&board.id);
    observe_usage(Some(&board), Some(40.0), Some(10.0), Some(0.0));
    assert_eq!(
        observed.run.lock().unwrap().usage,
        Some(WorkflowBenchmarkUsage::default())
    );
    observed.run.lock().unwrap().usage = None;
    observe_usage(
        Some(&board),
        Some(1.5),
        Some(9_007_199_254_740_992.0),
        Some(f64::MAX),
    );
    assert_eq!(
        observed.run.lock().unwrap().usage,
        Some(WorkflowBenchmarkUsage::default())
    );
}

#[tokio::test]
async fn retained_commits_are_graded_against_fixed_outputs_without_live_application() {
    let case = fixture("net-total");
    let correct = graded_source(&case, &case.reference_source).await;
    assert_eq!(
        correct.status,
        WorkflowBenchmarkRunStatus::Succeeded,
        "{correct:#?}"
    );
    assert_eq!(
        correct.attempts.len(),
        1,
        "check and commit must not add source attempts"
    );
    assert!(
        correct
            .checks
            .iter()
            .all(|check| check.status == WorkflowBenchmarkCheckStatus::Passed)
    );
    let broken = case.reference_source.replace(
        "subtotal + shipping - discount",
        "subtotal + shipping + discount",
    );
    assert_ne!(broken, case.reference_source);
    let incorrect = graded_source(&case, &broken).await;
    assert_eq!(
        incorrect.status,
        WorkflowBenchmarkRunStatus::Failed,
        "{incorrect:#?}"
    );
    assert!(
        incorrect
            .checks
            .iter()
            .any(|check| check.status == WorkflowBenchmarkCheckStatus::Failed)
    );
}

#[tokio::test]
async fn equivalent_outputs_do_not_replace_a_required_reusable_helper() {
    let case = fixture("normalize-pair-helper");
    let inline = r#"eventsGeneric normalizePair(left: string, right: string) {
    const first = left.trim().toLower()
    const second = right.trim().toLower()
    return { left: first, right: second }
}
"#;
    let result = graded_source(&case, inline).await;
    assert!(
        result
            .checks
            .iter()
            .all(|check| check.status == WorkflowBenchmarkCheckStatus::Passed),
        "{result:#?}"
    );
    assert_eq!(result.status, WorkflowBenchmarkRunStatus::Failed);
    assert!(
        result
            .grading_errors
            .iter()
            .any(|error| error.contains("normalizeLabel")),
        "{result:#?}"
    );
}

#[tokio::test]
async fn repair_grading_preserves_the_unrelated_entry_contract() {
    let case = fixture("repair-casing");
    let board = initial_board(&case).await.unwrap();
    let anchored = board_to_flowscript(
        &board,
        &RenderOptions {
            anchors: true,
            ..RenderOptions::default()
        },
    );
    assert!(anchored.contains("//@n:"), "{anchored}");
    // Repair the anchored document that the ordinary board tool exposes to the model.
    let corrected_source = anchored.replace("toUpper", "toLower");
    assert_ne!(corrected_source, anchored, "{anchored}");
    let correct = graded_board_source(&case, board.clone(), &corrected_source).await;
    assert_eq!(
        correct.status,
        WorkflowBenchmarkRunStatus::Succeeded,
        "{correct:#?}"
    );
    assert_eq!(
        corrected_source.matches("\"version\":1").count(),
        1,
        "{corrected_source}"
    );
    let broken = corrected_source.replace("\"version\":1", "\"version\":2");
    let result = graded_board_source(&case, board, &broken).await;
    assert_eq!(
        result.status,
        WorkflowBenchmarkRunStatus::Failed,
        "{result:#?}"
    );
    assert_eq!(
        result
            .checks
            .iter()
            .find(|check| check.check_id == "preserve-status")
            .unwrap()
            .status,
        WorkflowBenchmarkCheckStatus::Failed
    );
    assert!(
        result
            .checks
            .iter()
            .filter(|check| check.check_id != "preserve-status")
            .all(|check| check.status == WorkflowBenchmarkCheckStatus::Passed)
    );
}

#[tokio::test]
async fn helper_calls_in_an_unexecuted_entry_cannot_satisfy_the_required_behavior() {
    let case = fixture("normalize-pair-helper");
    let decoy = r#"function normalizeLabel(value: string): (label: string) {
    return value.trim().toLower()
}

eventsGeneric normalizePair(left: string, right: string) {
    const first = left.trim().toLower()
    const second = right.trim().toLower()
    return { left: first, right: second }
}

eventsGeneric unusedHelperEntry(value: string) {
    const normalized = normalizeLabel({ value: value })
    return normalized
}
"#;
    let result = graded_source(&case, decoy).await;
    assert!(
        result
            .checks
            .iter()
            .all(|check| check.status == WorkflowBenchmarkCheckStatus::Passed),
        "{result:#?}"
    );
    assert_eq!(
        result.status,
        WorkflowBenchmarkRunStatus::Failed,
        "{result:#?}"
    );
    assert!(
        result
            .grading_errors
            .iter()
            .any(|error| error.contains("normalizeLabel")),
        "{result:#?}"
    );
}
