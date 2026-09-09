//! Runs fixed behavioral fixtures through the ordinary board authoring transport.

use std::{
    collections::{BTreeMap, HashMap},
    sync::{Arc, LazyLock, Mutex},
    time::{Duration, Instant},
};

use flow_like::{
    copilot::{CopilotScope, prompts::BoardPromptProfile},
    flow::{
        ast::{
            FlowScriptDiagnosticPhase, apply_board_commands_to_board, apply_flowscript_to_board,
        },
        board::{Board, LayerType},
        copilot::{
            FlowIrDraftStore, FlowScriptDraftResponse, behavioral_evaluation::*,
            benchmark_cases::workflow_benchmark_cases, ir_tools::flowscript_catalog_fingerprint,
        },
    },
};
use serde_json::{Value, json};
use tauri::{
    AppHandle,
    ipc::{Channel, InvokeResponseBody},
};

use super::{backend_types::FlowPilotAgentBackendKind, catalog::DesktopCatalogProvider};
use crate::functions::ai::copilot_sdk_tools::{
    release_benchmark_draft_store, retained_flow_ir_draft_store_for_board,
};

const GENERATION_DEADLINE: Duration = Duration::from_secs(300);
static RUNS: LazyLock<Mutex<HashMap<String, Arc<Mutex<RunObservation>>>>> =
    LazyLock::new(Default::default);

struct RunObservation {
    started: Instant,
    prompt_profile: BoardPromptProfile,
    attempts: Vec<WorkflowBenchmarkAttempt>,
    tool_schema_fingerprint: Option<String>,
    usage: Option<WorkflowBenchmarkUsage>,
    tool_calls: Vec<WorkflowBenchmarkToolCall>,
    tool_calls_truncated: bool,
    retained_source: Option<String>,
    transport_events: Vec<WorkflowBenchmarkTransportEvent>,
    transport_events_truncated: bool,
    transport_counts: BTreeMap<String, u64>,
    external_phases: Vec<Value>,
    external_phases_truncated: bool,
}

pub(super) fn prompt_profile_label(profile: BoardPromptProfile) -> &'static str {
    match profile {
        BoardPromptProfile::Legacy => "legacy",
        BoardPromptProfile::Focused => "focused",
    }
}

pub(super) fn default_board_prompt_profile() -> BoardPromptProfile {
    BoardPromptProfile::Legacy
}

pub(super) fn benchmark_prompt_profile_for_board(
    board: Option<&Board>,
) -> Option<BoardPromptProfile> {
    board.and_then(|board| observation(&board.id)).map(|run| {
        run.lock()
            .unwrap_or_else(|error| error.into_inner())
            .prompt_profile
    })
}

pub(super) fn observe_tool_dispatch(
    board_id: &str,
    transport: &'static str,
    tool: &str,
    phase: &'static str,
    code: Option<&'static str>,
    status: Option<&'static str>,
) {
    let Some(run) = observation(board_id) else {
        return;
    };
    if !matches!(transport, "mcp" | "sdk")
        || !matches!(
            phase,
            "arrival" | "preflight_short_circuit" | "dispatched" | "initialized" | "list_tools"
        )
        || tool.len() > 128
    {
        return;
    }
    let mut run = run.lock().unwrap_or_else(|error| error.into_inner());
    let count = run
        .transport_counts
        .entry(format!("{transport}.{phase}"))
        .or_default();
    *count = count.saturating_add(1);
    if run.transport_events.len() >= 512 {
        run.transport_events_truncated = true;
        return;
    }
    let elapsed_ms = run.started.elapsed().as_millis() as u64;
    run.transport_events.push(WorkflowBenchmarkTransportEvent {
        elapsed_ms,
        transport: transport.into(),
        tool: tool.into(),
        phase: phase.into(),
        code: code.map(str::to_owned),
        status: status.map(str::to_owned),
    });
}

pub(super) fn observe_external_phase_state(board_id: &str, snapshot: &Value) {
    let Some(run) = observation(board_id) else {
        return;
    };
    let Some(phase_id) = snapshot
        .get("phase_id")
        .and_then(Value::as_str)
        .filter(|id| !id.is_empty() && id.len() <= 128)
    else {
        return;
    };
    let Some(sequence) = snapshot.get("observation_sequence").and_then(Value::as_u64) else {
        return;
    };
    if snapshot.get("schema").and_then(Value::as_str) != Some("flowpilot.external-process-phase/v1")
        || serde_json::to_vec(snapshot).map_or(true, |bytes| bytes.len() > 8192)
    {
        return;
    }
    let mut run = run.lock().unwrap_or_else(|error| error.into_inner());
    if let Some(existing) = run
        .external_phases
        .iter_mut()
        .find(|existing| existing.get("phase_id").and_then(Value::as_str) == Some(phase_id))
    {
        if existing
            .get("observation_sequence")
            .and_then(Value::as_u64)
            .is_none_or(|prior| sequence > prior)
        {
            *existing = snapshot.clone();
        }
    } else if run.external_phases.len() < 16 {
        run.external_phases.push(snapshot.clone());
    } else {
        run.external_phases_truncated = true;
    }
}

pub(crate) fn is_benchmark_board(board_id: &str) -> bool {
    // This reserved host-generated namespace remains ephemeral after cancellation, including
    // any late tool completion that races session cleanup.
    board_id.starts_with("workflow-benchmark-")
}

pub(crate) fn benchmark_board_active(board_id: &str) -> bool {
    observation(board_id).is_some()
}

fn observation(board_id: &str) -> Option<Arc<Mutex<RunObservation>>> {
    RUNS.lock().ok()?.get(board_id).cloned()
}

pub(super) fn observe_external_phase_usage(
    board_id: &str,
    usage: &WorkflowBenchmarkUsage,
    complete: bool,
) {
    let Some(run) = observation(board_id) else {
        return;
    };
    let mut run = run.lock().unwrap_or_else(|error| error.into_inner());
    let next = if complete {
        usage.clone()
    } else {
        WorkflowBenchmarkUsage {
            input_tokens: None,
            output_tokens: None,
            cached_input_tokens: None,
        }
    };
    run.usage = Some(match run.usage.take() {
        None => next,
        Some(previous) => WorkflowBenchmarkUsage {
            input_tokens: previous
                .input_tokens
                .zip(next.input_tokens)
                .and_then(|(a, b)| a.checked_add(b)),
            output_tokens: previous
                .output_tokens
                .zip(next.output_tokens)
                .and_then(|(a, b)| a.checked_add(b)),
            cached_input_tokens: previous
                .cached_input_tokens
                .zip(next.cached_input_tokens)
                .and_then(|(a, b)| a.checked_add(b)),
        },
    });
}

pub(super) fn observe_usage(
    board: Option<&Board>,
    input: Option<f64>,
    output: Option<f64>,
    cached: Option<f64>,
) {
    let Some(run) = board.and_then(|board| observation(&board.id)) else {
        return;
    };
    let count = |value: Option<f64>| {
        value
            .filter(|value| {
                value.is_finite()
                    && *value >= 0.0
                    && *value <= 9_007_199_254_740_991.0
                    && value.fract() == 0.0
            })
            .map(|value| value as u64)
    };
    let next = WorkflowBenchmarkUsage {
        input_tokens: count(input),
        output_tokens: count(output),
        cached_input_tokens: count(cached),
    };
    let mut run = run.lock().unwrap_or_else(|error| error.into_inner());
    run.usage = Some(match run.usage.take() {
        None => next,
        Some(previous) => WorkflowBenchmarkUsage {
            input_tokens: previous
                .input_tokens
                .zip(next.input_tokens)
                .and_then(|(a, b)| a.checked_add(b)),
            output_tokens: previous
                .output_tokens
                .zip(next.output_tokens)
                .and_then(|(a, b)| a.checked_add(b)),
            cached_input_tokens: previous
                .cached_input_tokens
                .zip(next.cached_input_tokens)
                .and_then(|(a, b)| a.checked_add(b)),
        },
    });
}

fn invalidate_usage(board_id: &str) {
    if let Some(run) = observation(board_id) {
        run.lock().unwrap_or_else(|error| error.into_inner()).usage =
            Some(WorkflowBenchmarkUsage::default());
    }
}

/// Observe accepted retained revisions, including invalid source. Read/check/test calls cannot
/// inflate the repair count, and rejected patches that kept the old revision cannot add attempts.
pub(crate) fn observe_source(board_id: &str, result: &FlowScriptDraftResponse) {
    let Some(run) = observation(board_id) else {
        return;
    };
    let (Some(draft_id), Some(revision), Some(source)) =
        (&result.draft_id, result.revision, &result.source)
    else {
        return;
    };
    if result.status == "error" {
        return;
    }
    let mut run = run.lock().unwrap_or_else(|error| error.into_inner());
    if run
        .attempts
        .iter()
        .any(|attempt| attempt.draft_id == *draft_id && attempt.revision == revision)
    {
        return;
    }
    let parse_valid = !result
        .diagnostics
        .iter()
        .any(|diagnostic| matches!(diagnostic.phase, FlowScriptDiagnosticPhase::Parse));
    let typed_valid = parse_valid
        && !result.diagnostics.iter().any(|diagnostic| {
            matches!(
                diagnostic.phase,
                FlowScriptDiagnosticPhase::CatalogResolution | FlowScriptDiagnosticPhase::TypeCheck
            )
        });
    let attempt = WorkflowBenchmarkAttempt {
        attempt_index: run.attempts.len() as u32 + 1,
        draft_id: draft_id.clone(),
        revision,
        source_fingerprint: blake3::hash(source.as_bytes()).to_hex().to_string(),
        elapsed_ms: run.started.elapsed().as_millis() as u64,
        parse_valid,
        typed_valid,
        reconcile_valid: typed_valid && result.diagnostics.is_empty(),
        diagnostic_keys: result
            .diagnostics
            .iter()
            .map(|diagnostic| diagnostic.id.clone())
            .collect(),
    };
    run.attempts.push(attempt);
    run.retained_source = Some(source.clone());
}

pub(super) fn observe_tools(
    board: Option<&Board>,
    tools: &mut [(copilot_sdk::Tool, copilot_sdk::ToolHandler)],
) {
    let Some(run) = board.and_then(|board| observation(&board.id)) else {
        return;
    };
    let mut definitions: Vec<_> = tools
        .iter()
        .map(|(tool, _)| serde_json::to_value(tool).expect("tool schema is JSON"))
        .collect();
    definitions.sort_by_key(|tool| tool["name"].as_str().unwrap_or_default().to_owned());
    run.lock()
        .unwrap_or_else(|error| error.into_inner())
        .tool_schema_fingerprint = Some(fingerprint(&json!(definitions)));
    for (tool, handler) in tools {
        let original = handler.clone();
        let run = run.clone();
        let tool_name = tool.name.clone();
        *handler = Arc::new(move |name, args| {
            let started = Instant::now();
            let index = {
                let mut run = run.lock().unwrap_or_else(|error| error.into_inner());
                if run.tool_calls.len() >= 256 {
                    run.tool_calls_truncated = true;
                    None
                } else {
                    let index = run.tool_calls.len();
                    let started_ms = run.started.elapsed().as_millis() as u64;
                    run.tool_calls.push(WorkflowBenchmarkToolCall {
                        index: index as u32 + 1,
                        tool: tool_name.clone(),
                        started_ms,
                        duration_ms: None,
                        status: "running".into(),
                        diagnostic_codes: Vec::new(),
                    });
                    Some(index)
                }
            };
            let result = original(name, args);
            if let Some(index) = index {
                let value = serde_json::from_str::<Value>(&result.text_result_for_llm).ok();
                let mut run = run.lock().unwrap_or_else(|error| error.into_inner());
                let call = &mut run.tool_calls[index];
                call.duration_ms = Some(started.elapsed().as_millis() as u64);
                call.status = value
                    .as_ref()
                    .and_then(|value| value["status"].as_str())
                    .unwrap_or(&result.result_type)
                    .chars()
                    .take(64)
                    .collect();
                if let Some(value) = value {
                    call.diagnostic_codes = value
                        .get("code")
                        .into_iter()
                        .chain(
                            value
                                .get("diagnostics")
                                .and_then(Value::as_array)
                                .into_iter()
                                .flatten()
                                .filter_map(|diagnostic| diagnostic.get("code")),
                        )
                        .filter_map(Value::as_str)
                        .take(16)
                        .map(|code| code.chars().take(128).collect())
                        .collect();
                }
            }
            result
        });
    }
}

fn fingerprint(value: &Value) -> String {
    blake3::hash(&serde_json::to_vec(value).expect("JSON serializes"))
        .to_hex()
        .to_string()
}

fn harness_revision() -> String {
    env!("FLOWPILOT_BENCHMARK_BUILD_ID").to_owned()
}

struct BenchmarkLease {
    board_id: String,
    store: Arc<FlowIrDraftStore>,
}

impl Drop for BenchmarkLease {
    fn drop(&mut self) {
        if let Ok(mut runs) = RUNS.lock() {
            runs.remove(&self.board_id);
        }
        release_benchmark_draft_store(&self.board_id, &self.store);
    }
}

#[tauri::command]
pub fn flowpilot_workflow_benchmark_cases() -> Vec<Value> {
    workflow_benchmark_cases()
        .into_iter()
        .map(|case| json!({"id": case.id, "prompt": case.prompt}))
        .collect()
}

#[tauri::command]
pub fn flowpilot_workflow_benchmark_scorecards(
    reports: Vec<WorkflowBenchmarkRunReport>,
) -> Result<Vec<Value>, String> {
    let cases = workflow_benchmark_cases();
    let mut groups = std::collections::BTreeMap::<String, Vec<WorkflowBenchmarkRunReport>>::new();
    for report in reports {
        let case = cases
            .iter()
            .find(|case| case.id == report.case_id)
            .ok_or("Unknown workflow benchmark case")?;
        validate_workflow_benchmark_report(case, &report)?;
        groups
            .entry(format!(
                "{}:{}",
                report.case_id,
                fingerprint(&json!(report.cohort))
            ))
            .or_default()
            .push(report);
    }
    groups.into_values().map(|reports| {
        let case = cases.iter().find(|case| case.id == reports[0].case_id).unwrap();
        Ok(json!({"case_id": case.id, "scorecard": evaluate_workflow_benchmark_runs(case, &reports)?}))
    }).collect()
}

async fn initial_board(case: &WorkflowBenchmarkCase) -> Result<Board, String> {
    let board = Board::new_detached(
        Some(format!(
            "workflow-benchmark-{}",
            flow_like_types::create_id()
        )),
        "workflow-benchmark".into(),
    );
    let Some(source) = case.initial_source.clone() else {
        return Ok(board);
    };
    flow_like_catalog::draft_test::prepare_draft_board(board, |mut board, state| async move {
        let catalog = state
            .node_registry
            .read()
            .await
            .get_nodes()
            .map_err(|error| error.to_string())?;
        let applied = apply_flowscript_to_board(&mut board, &source, &catalog, state, None, false)
            .await
            .map_err(|error| error.to_string())?;
        if !applied.diagnostics.is_empty() {
            return Err(applied.diagnostics.join("; "));
        }
        Ok(board)
    })
    .await
}

#[tauri::command]
pub async fn flowpilot_run_workflow_benchmark(
    app_handle: AppHandle,
    case_id: String,
    backend: FlowPilotAgentBackendKind,
    model_id: String,
    reasoning_effort: Option<String>,
    repeat_index: u32,
    request_id: String,
    channel: Channel<String>,
) -> Result<WorkflowBenchmarkRunReport, String> {
    run_workflow_benchmark_with_profile(
        app_handle,
        case_id,
        backend,
        model_id,
        reasoning_effort,
        repeat_index,
        request_id,
        channel,
        default_board_prompt_profile(),
    )
    .await
}

pub(super) async fn run_workflow_benchmark_with_profile(
    app_handle: AppHandle,
    case_id: String,
    backend: FlowPilotAgentBackendKind,
    model_id: String,
    reasoning_effort: Option<String>,
    repeat_index: u32,
    request_id: String,
    channel: Channel<String>,
    prompt_profile: BoardPromptProfile,
) -> Result<WorkflowBenchmarkRunReport, String> {
    let model_id = model_id.trim().to_owned();
    let reasoning_effort =
        super::external_invocation::explicit_reasoning_effort(reasoning_effort.as_deref())
            .map(str::to_owned);
    if model_id.is_empty()
        || model_id.eq_ignore_ascii_case("default")
        || model_id.len() > 200
        || request_id.trim().is_empty()
        || request_id.len() > 200
    {
        return Err("An explicit model and bounded nonempty request id are required".into());
    }
    let case = workflow_benchmark_cases()
        .into_iter()
        .find(|case| case.id == case_id)
        .ok_or("Unknown workflow benchmark case")?;
    let board = initial_board(&case).await?;
    let started = Instant::now();
    let observed = Arc::new(Mutex::new(RunObservation {
        started,
        prompt_profile,
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
    RUNS.lock()
        .map_err(|error| error.to_string())?
        .insert(board.id.clone(), observed.clone());
    let store = match retained_flow_ir_draft_store_for_board(&board) {
        Ok(store) => store,
        Err(error) => {
            RUNS.lock()
                .unwrap_or_else(|error| error.into_inner())
                .remove(&board.id);
            return Err(error);
        }
    };
    let _lease = BenchmarkLease {
        board_id: board.id.clone(),
        store: store.clone(),
    };
    let observed_channel = Channel::new(move |body: InvokeResponseBody| {
        let chunk = match body {
            InvokeResponseBody::Json(json) => serde_json::from_str::<String>(&json).ok(),
            InvokeResponseBody::Raw(bytes) => String::from_utf8(bytes).ok(),
        };
        if let Some(chunk) = chunk {
            let _ = channel.send(chunk);
        }
        Ok(())
    });
    // Only this public request enters the model session. The fixture, expected outputs and
    // reference implementation stay in the host and never enter a repair continuation.
    let prompt = case.public_prompt().to_owned();
    let generation = async {
        match backend {
            FlowPilotAgentBackendKind::GithubCopilot => {
                super::sdk_chat::copilot_sdk_chat_internal(
                    app_handle,
                    &model_id,
                    reasoning_effort.as_deref(),
                    CopilotScope::Board,
                    Some(&board),
                    None,
                    &[],
                    None,
                    None,
                    prompt.clone(),
                    prompt.clone(),
                    prompt.clone(),
                    None,
                    None,
                    Vec::new(),
                    observed_channel,
                    None,
                    None,
                    None,
                    Some(request_id.clone()),
                    false,
                    false,
                )
                .await
            }
            _ => {
                super::external_chat::external_code_agent_chat_internal(
                    app_handle,
                    backend,
                    &model_id,
                    reasoning_effort.as_deref(),
                    CopilotScope::Board,
                    Some(&board),
                    None,
                    &[],
                    None,
                    None,
                    prompt.clone(),
                    prompt.clone(),
                    prompt.clone(),
                    None,
                    None,
                    Vec::new(),
                    observed_channel,
                    None,
                    None,
                    None,
                    Some(request_id.clone()),
                    false,
                    false,
                )
                .await
            }
        }
    };
    let response = match tokio::time::timeout(GENERATION_DEADLINE, generation).await {
        Ok(response) => response,
        Err(_) => {
            super::runtime::cancel_registered_copilot_run(&request_id);
            Err("Workflow benchmark generation exceeded its 300 second deadline".into())
        }
    };
    if backend == FlowPilotAgentBackendKind::GithubCopilot && response.is_err() {
        // SDK events from earlier responses cannot account for a later unfinished response.
        invalidate_usage(&board.id);
    }
    let metadata = DesktopCatalogProvider::new(None).all_metadata();
    let catalog_fingerprint = flowscript_catalog_fingerprint(&metadata);
    let (
        attempts,
        usage,
        tool_schema_fingerprint,
        tool_calls,
        tool_calls_truncated,
        retained_source,
        transport_events,
        transport_events_truncated,
        transport_counts,
        external_phases,
        external_phases_truncated,
    ) = {
        let observed = observed.lock().unwrap_or_else(|error| error.into_inner());
        (
            observed.attempts.clone(),
            observed.usage.clone(),
            observed.tool_schema_fingerprint.clone(),
            observed.tool_calls.clone(),
            observed.tool_calls_truncated,
            observed.retained_source.clone(),
            observed.transport_events.clone(),
            observed.transport_events_truncated,
            observed.transport_counts.clone(),
            observed.external_phases.clone(),
            observed.external_phases_truncated,
        )
    };
    let mut report = WorkflowBenchmarkRunReport {
        schema: WORKFLOW_BENCHMARK_REPORT_SCHEMA.into(),
        run_id: request_id,
        case_id: case.id.clone(),
        repeat_index,
        cohort: WorkflowBenchmarkCohort {
            provider: backend.cli_name().into(),
            model: model_id,
            reasoning_effort: reasoning_effort.unwrap_or_else(|| "provider-default".into()),
            prompt_profile: prompt_profile_label(prompt_profile).into(),
            harness_revision: harness_revision(),
            fixture_fingerprint: case.fixture_fingerprint(),
            catalog_fingerprint: catalog_fingerprint.clone(),
            prompt_fingerprint: fingerprint(
                &json!({"profile": prompt_profile_label(prompt_profile), "public_prompt": prompt, "initial_source": case.initial_source,
                "templates": [include_str!("../../../../../../../packages/core/editor/src/copilot/prompts/board/mod.rs"),
                include_str!("../../../../../../../packages/core/editor/src/copilot/prompts/board/guidance.rs"),
                include_str!("../../../../../../../packages/core/editor/src/copilot/prompts/board/examples.rs")]}),
            ),
            tool_schema_fingerprint: fingerprint(
                &json!({"build": harness_revision(), "scope": "board-authoring", "catalog": catalog_fingerprint}),
            ),
            budget_fingerprint: fingerprint(
                &json!({"deadline_secs": GENERATION_DEADLINE.as_secs(), "ordinary_loop": include_str!("workflow_state.rs")}),
            ),
        },
        observed_tool_schema_fingerprint: tool_schema_fingerprint,
        tool_calls,
        tool_calls_truncated,
        retained_source,
        transport_events,
        transport_events_truncated,
        transport_counts,
        external_phases,
        external_phases_truncated,
        status: WorkflowBenchmarkRunStatus::Failed,
        elapsed_ms: 0,
        usage,
        attempts,
        graded_candidate: None,
        checks: Vec::new(),
        grading_errors: Vec::new(),
    };
    match response {
        Ok(response) => match response.flow_ir_commit {
            Some(token) => {
                let binding = store.bind_request_acceptance_contract(&board.id, &prompt);
                let delivery = store.pending_flowscript_delivery_for_binding(&board, &binding);
                let _ = store.release_request_acceptance_contract(&binding);
                match delivery.filter(|delivery| !delivery.stale_board && delivery.token == token) {
                    Some(delivery) => grade_delivery(&case, board, delivery, &mut report).await,
                    None => report.grading_errors.push(
                        "Returned commit does not match an exact current retained source claim"
                            .into(),
                    ),
                }
            }
            None => report
                .grading_errors
                .push("No committed FlowScript candidate was returned".into()),
        },
        Err(error) => {
            if error.to_ascii_lowercase().contains("cancelled") {
                report.status = WorkflowBenchmarkRunStatus::Cancelled;
            }
            report.grading_errors.push(error);
        }
    }
    if report.graded_candidate.is_some() && report.checks.is_empty() {
        report.checks = grade_workflow_benchmark_checks(&case, &[])?.checks;
    }
    report.elapsed_ms = started.elapsed().as_millis() as u64;
    validate_workflow_benchmark_report(&case, &report)?;
    Ok(report)
}

#[cfg(test)]
#[path = "workflow_benchmark_tests.rs"]
mod tests;

async fn grade_delivery(
    case: &WorkflowBenchmarkCase,
    board: Board,
    delivery: flow_like::flow::copilot::FlowScriptPendingDelivery,
    report: &mut WorkflowBenchmarkRunReport,
) {
    let source_fingerprint = blake3::hash(delivery.source.as_bytes())
        .to_hex()
        .to_string();
    let Some(attempt) = report.attempts.iter().find(|attempt| {
        attempt.draft_id == delivery.token.draft_id
            && attempt.revision == delivery.token.revision
            && attempt.source_fingerprint == source_fingerprint
    }) else {
        report
            .grading_errors
            .push("Committed source has no observed submission revision".into());
        return;
    };
    report.graded_candidate = Some(WorkflowBenchmarkGradedCandidate {
        attempt_index: attempt.attempt_index,
        draft_id: delivery.token.draft_id,
        revision: delivery.token.revision,
        source_fingerprint,
        base_fingerprint: delivery.token.base_fingerprint,
        catalog_fingerprint: report.cohort.catalog_fingerprint.clone(),
        commands_fingerprint: fingerprint(&json!(delivery.commands)),
    });
    let commands = delivery.commands;
    let existing_entries: Vec<_> = board
        .nodes
        .values()
        .chain(board.layers.values().flat_map(|layer| layer.nodes.values()))
        .filter(|node| node.start == Some(true))
        .map(|node| {
            (
                node.id.clone(),
                node.name.clone(),
                node.friendly_name.clone(),
            )
        })
        .collect();
    let prepared =
        flow_like_catalog::draft_test::prepare_draft_board(board, |mut board, state| async move {
            let catalog = state
                .node_registry
                .read()
                .await
                .get_nodes()
                .map_err(|error| error.to_string())?;
            let applied =
                apply_board_commands_to_board(&mut board, commands, &catalog, state, None)
                    .await
                    .map_err(|error| error.to_string())?;
            if !applied.diagnostics.is_empty() {
                return Err(applied.diagnostics.join("; "));
            }
            Ok(board)
        })
        .await;
    let board = match prepared {
        Ok(board) => board,
        Err(error) => {
            report.grading_errors.push(error);
            return;
        }
    };
    for (id, name, friendly_name) in existing_entries {
        if !board
            .nodes
            .values()
            .chain(board.layers.values().flat_map(|layer| layer.nodes.values()))
            .any(|node| {
                node.id == id
                    && node.name == name
                    && node.friendly_name == friendly_name
                    && node.start == Some(true)
            })
        {
            report.grading_errors.push(format!(
                "Existing entry identity changed: {friendly_name} ({id})"
            ));
        }
    }
    let mut observations = Vec::new();
    let mut called_helpers = std::collections::HashSet::new();
    for check in &case.checks {
        let runtime = match flow_like_catalog::draft_test::test_draft_board(
            board.clone(),
            &check.entry,
            check.payload.clone(),
        )
        .await
        {
            Ok(result) => {
                called_helpers.extend(
                    result
                        .helper_calls
                        .iter()
                        .filter(|(_, count)| **count > 0)
                        .map(|(id, _)| id.clone()),
                );
                serde_json::to_value(result).expect("runtime result is JSON")
            }
            Err(error) => json!({"status": "blocked", "errors": [error], "outputs": []}),
        };
        observations.push(WorkflowBenchmarkObservation {
            check_id: check.id.clone(),
            runtime,
        });
    }
    for name in &case.required_functions {
        if !board.layers.values().any(|layer| {
            matches!(layer.r#type, LayerType::Function)
                && layer.name == *name
                && called_helpers.contains(&layer.id)
        }) {
            report.grading_errors.push(format!(
                "Required called helper function is missing: {name}"
            ));
        }
    }
    match grade_workflow_benchmark_checks(case, &observations) {
        Ok(grade) => {
            report.checks = grade.checks;
            report.grading_errors.extend(grade.errors);
            if grade.passed && report.grading_errors.is_empty() {
                report.status = WorkflowBenchmarkRunStatus::Succeeded;
            }
        }
        Err(error) => report.grading_errors.push(error),
    }
}
