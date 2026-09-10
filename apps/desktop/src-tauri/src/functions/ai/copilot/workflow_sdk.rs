//! SDK workflow guards, leases, and tool classification.

use super::runtime::SdkToolActivityRegistry;
use super::workflow_observation::{
    workflow_tool_abort_with_args, workflow_tool_record_with_outcome,
};
use super::workflow_preflight::{workflow_candidate_preflight, workflow_tool_preflight_with_args};
use super::workflow_results::{
    annotate_modular_fallback_result, suppress_unchanged_flowscript_source_echo,
};
use super::workflow_state::{
    MAX_EXTERNAL_PREDRAFT_CONTEXT_READS, MAX_EXTERNAL_TYPED_IR_OPERATION_BUDGET,
    MIN_EXTERNAL_TYPED_IR_OPERATION_BUDGET, WorkflowToolLoopState,
};
use flow_like::flow::copilot::workflow_tool_result_succeeded;
use flow_like_types::tokio_util::sync::CancellationToken;
use std::sync::{Arc, Mutex as StdMutex};

type WorkflowDispatchSink =
    Arc<dyn Fn(&str, &'static str, Option<&'static str>, Option<&'static str>) + Send + Sync>;

/// Records only host tool names and fixed dispatch outcomes for an active benchmark board.
#[derive(Clone, Default)]
pub(super) struct WorkflowToolDispatchObserver(Option<WorkflowDispatchSink>);

impl WorkflowToolDispatchObserver {
    pub(super) fn new(
        state: Option<&Arc<StdMutex<WorkflowToolLoopState>>>,
        transport: &'static str,
    ) -> Self {
        let board_id = state.and_then(|state| {
            let state = state.lock().ok()?;
            Some(state.shared_session.as_ref()?.manifest().board.id.clone())
        });
        let Some(board_id) =
            board_id.filter(|id| super::workflow_benchmark::is_benchmark_board(id))
        else {
            return Self::default();
        };
        Self(Some(Arc::new(move |tool, phase, code, status| {
            super::workflow_benchmark::observe_tool_dispatch(
                &board_id, transport, tool, phase, code, status,
            );
        })))
    }

    pub(super) fn record(
        &self,
        tool: &str,
        phase: &'static str,
        code: Option<&'static str>,
        status: Option<&'static str>,
    ) {
        if let Some(sink) = &self.0 {
            sink(tool, phase, code, status);
        }
    }

    pub(super) fn short_circuit_text(&self, tool: &str, text: &str) {
        if self.0.is_none() {
            return;
        }
        let (code, status) = preflight_dispatch_tags(text);
        self.record(tool, "preflight_short_circuit", code, status);
    }

    pub(super) fn short_circuit_mcp(&self, tool: &str, result: &rmcp::model::CallToolResult) {
        let text = result
            .content
            .iter()
            .find_map(|content| match &content.raw {
                rmcp::model::RawContent::Text(text) => Some(text.text.as_str()),
                _ => None,
            });
        self.short_circuit_text(tool, text.unwrap_or_default());
    }
}

fn preflight_dispatch_tags(text: &str) -> (Option<&'static str>, Option<&'static str>) {
    // A preflight may include a retained source preview. Never copy it into telemetry, and cap
    // parsing work independently of the size of that preview.
    if text.len() > 65_536 {
        return (Some("oversized_result"), Some("unknown"));
    }
    let Ok(value) = serde_json::from_str::<serde_json::Value>(text) else {
        return (Some("unstructured_result"), Some("unknown"));
    };
    const CODES: &[&str] = &[
        "WORKFLOW_LOOP_STATE_UNAVAILABLE",
        "board_draft_required_before_database_setup",
        "PREDRAFT_INSPECTION_BUDGET_EXHAUSTED",
        "CONTEXT_ALREADY_IN_MANIFEST",
        "FIRST_ARTIFACT_SLA_BREACHED",
        "DUPLICATE_CONTEXT_READ",
        "WORKFLOW_ZERO_PROGRESS_CIRCUIT_OPEN",
        "DECLARATION_LOOKUP_IN_FLIGHT",
        "WORKFLOW_MUTATION_PATH_CONFLICT",
        "FLOWSCRIPT_RETAINED_REVISION_REQUIRED",
        "FLOWSCRIPT_DRAFT_REQUIRED",
        "TYPED_IR_REPAIR_PROGRESS_STALLED",
        "TYPED_IR_OPERATION_BUDGET_EXHAUSTED",
        "FLOWSCRIPT_REPAIR_PROGRESS_STALLED",
        "FLOWSCRIPT_OPERATION_BUDGET_EXHAUSTED",
        "TIME_EXTENSION_NO_PROGRESS",
        "TIME_EXTENSION_CEILING_REACHED",
        "TIME_EXTENSION_NOT_APPLICABLE",
        "SCOPE_PLAN_BUDGET_EXHAUSTED",
        "SCOPE_PLAN_REJECTION_BUDGET_EXHAUSTED",
        "SCOPE_PLAN_ARGUMENTS_INVALID",
        "DECLARATION_COVERAGE_EXHAUSTED",
        "SCOPE_PLAN_REQUIRED",
        "SCOPE_PLAN_COMMIT_VALIDATED_PREFIX",
        "FLOWSCRIPT_COMMIT_RETRY_BUDGET_EXHAUSTED",
        "DECLARATION_FOLLOW_UP_UNRELATED",
        "candidate_regression",
        "runtime_verification_deferred",
        "SCOPE_PLAN_EMPTY",
        "SCOPE_PLAN_TOO_LARGE",
        "SCOPE_PLAN_STRATEGY_MISMATCH",
        "SCOPE_PLAN_INVALID_SEGMENT",
        "SCOPE_PLAN_DUPLICATE_SEGMENT",
        "SCOPE_PLAN_SEGMENT_NOT_CONCRETE",
        "SCOPE_PLAN_CYCLE",
        "SCOPE_PLAN_INVALID_BOARD_REF",
        "SCOPE_PLAN_REVISION_EXHAUSTED",
        "SCOPE_PLAN_COMMITTED_SEGMENT_REDECLARED",
    ];
    const STATUSES: &[&str] = &[
        "internal_state_unavailable",
        "deferred",
        "predraft_inspection_budget_exhausted",
        "context_preloaded",
        "first_artifact_sla_breached",
        "duplicate_context_read",
        "already_queued",
        "zero_progress_circuit_open",
        "declaration_lookup_in_flight",
        "mutation_path_conflict",
        "retained_revision_required",
        "flowscript_draft_required",
        "edit_in_flight",
        "typed_repair_progress_stalled",
        "typed_repair_budget_exhausted",
        "edit_progress_stalled",
        "edit_budget_exhausted",
        "time_budget_extended",
        "time_budget_refused",
        "scope_plan_budget_exhausted",
        "scope_plan_rejected",
        "declaration_coverage_exhausted",
        "declaration_lookup_required",
        "scope_plan_required",
        "commit_validated_prefix",
        "commit_retry_budget_exhausted",
        "discovery_blocked",
        "already_returned",
        "declaration_batch_required",
        "declaration_follow_up_unrelated",
        "discovery_budget_exhausted",
        "diagnostic_lookup_required",
        "duplicate_declaration_lookup",
        "validation_errors",
        "scope_plan_accepted",
        "scope_plan_revision_required",
        "error",
    ];
    let bounded = |field: &str, allowed: &'static [&'static str]| {
        value
            .get(field)
            .and_then(serde_json::Value::as_str)
            .map(|raw| {
                allowed
                    .iter()
                    .copied()
                    .find(|known| *known == raw)
                    .unwrap_or("other")
            })
    };
    (bounded("code", CODES), bounded("status", STATUSES))
}

pub(super) fn workflow_state_has_retained_candidate(
    state: Option<&Arc<StdMutex<WorkflowToolLoopState>>>,
) -> bool {
    let Some(state) = state else {
        return false;
    };
    match state.lock() {
        Ok(state) => {
            state.queued
                || state.last_flowscript.is_some()
                || state.in_flight_flowscript.is_some()
                || state.flowscript_draft_retained
                || state.typed_draft_retained
        }
        // A poisoned loop mutex means the lifecycle may have been interrupted after retaining a
        // draft. Assume there is recoverable work so outer retry logic cannot silently discard it.
        Err(_) => true,
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum InitialSourceCheckpointPhase {
    /// The model still needs usable declarations and an accepted host scope plan. Time spent
    /// obtaining either is not part of the bounded source-composition window.
    AwaitingPrerequisites,
    /// An ancillary database/UI/storage read admitted by the shared workflow session is still
    /// executing. Its own handler deadline is authoritative while it owns a context-read lease.
    AncillaryContextInFlight,
    /// All prerequisites are ready and no recoverable source operation has started yet.
    AwaitingInitialSource,
    /// A source operation started, a draft was retained, or the workflow was already queued.
    Complete,
}

pub(super) fn workflow_initial_source_checkpoint_phase(
    state: &WorkflowToolLoopState,
) -> InitialSourceCheckpointPhase {
    if state.queued
        || state.flowscript_draft_retained
        || state.typed_draft_retained
        || state.flowscript_operation_attempts > 0
        || state.typed_operation_attempts > 0
    {
        return InitialSourceCheckpointPhase::Complete;
    }
    if !state.initial_declaration_lookup_usable || state.scope_plan.is_none() {
        return InitialSourceCheckpointPhase::AwaitingPrerequisites;
    }

    let shared_elapsed_ms = state.shared_session_elapsed_ms();
    if state.shared_session.as_ref().is_some_and(|session| {
        !session
            .snapshot(shared_elapsed_ms)
            .in_flight_context_reads
            .is_empty()
    }) {
        return InitialSourceCheckpointPhase::AncillaryContextInFlight;
    }
    InitialSourceCheckpointPhase::AwaitingInitialSource
}

/// Outcome of preparing the workflow loop budget for one SDK idle continuation.
#[derive(Debug, PartialEq, Eq)]
pub(super) enum IdleContinuationBudget {
    /// No budget is exhausted; the continuation instructions are executable as-is.
    Executable,
    /// The named budget was exhausted and received the same bounded continuation slice the
    /// external phase loop grants, so the instructions are not refused on arrival.
    SliceGranted(String),
    /// Reason the continuation must not be sent: the budget already received a slice for this
    /// exact state and burned it again, or the loop state is unusable. Another continuation
    /// would arrive equally dead; stop honestly.
    Terminal(String),
}

pub(super) fn prepare_sdk_idle_continuation_budget(
    workflow_state: Option<&Arc<StdMutex<WorkflowToolLoopState>>>,
    previous_exhausted_budget: Option<&str>,
) -> IdleContinuationBudget {
    let Some(state) = workflow_state else {
        return IdleContinuationBudget::Executable;
    };
    let Ok(mut state) = state.lock() else {
        return IdleContinuationBudget::Terminal(
            "the host workflow lifecycle state is unavailable".to_string(),
        );
    };
    let Some(exhausted) = state.exhausted_budget() else {
        return IdleContinuationBudget::Executable;
    };
    if Some(exhausted.as_str()) == previous_exhausted_budget {
        // Terminal only when the progress ledger also failed to advance since the previous
        // slice — a run that spent the slice moving forward earns another one.
        let mark = state.progress_mark();
        let progressed = state
            .last_idle_continuation_progress
            .as_ref()
            .is_some_and(|previous| mark.advanced_beyond(previous));
        if !progressed {
            return IdleContinuationBudget::Terminal(format!(
                "the {exhausted} was exhausted again after its granted continuation slice"
            ));
        }
    }
    state.last_idle_continuation_progress = Some(state.progress_mark());
    state.grant_continuation_slice();
    IdleContinuationBudget::SliceGranted(exhausted)
}

pub(super) fn workflow_loop_result(
    payload: serde_json::Value,
    is_error: bool,
) -> rmcp::model::CallToolResult {
    let text = serde_json::to_string_pretty(&payload).unwrap_or_else(|_| payload.to_string());
    if is_error {
        rmcp::model::CallToolResult::error(vec![rmcp::model::Content::text(text)])
    } else {
        rmcp::model::CallToolResult::success(vec![rmcp::model::Content::text(text)])
    }
}

pub(super) fn workflow_loop_state_unavailable_result() -> rmcp::model::CallToolResult {
    workflow_loop_result(
        serde_json::json!({
            "status": "internal_state_unavailable",
            "code": "WORKFLOW_LOOP_STATE_UNAVAILABLE",
            "retryable": false,
            "next_action": "stop_and_resume_in_new_run",
            "message": "The host workflow lifecycle state is unavailable. No tool operation was dispatched; stop this run so a fresh host process can recover any retained draft safely."
        }),
        true,
    )
}

fn workflow_tool_preflight_sdk(
    state: &Arc<StdMutex<WorkflowToolLoopState>>,
    tool_name: &str,
    args: &serde_json::Value,
) -> ExternalSdkToolPreflight {
    if let Some(result) = workflow_database_setup_preflight(state, tool_name, args) {
        return ExternalSdkToolPreflight {
            result: Some(call_tool_result_to_sdk_result(result)),
            lease: None,
        };
    }
    let mut preflight = workflow_predraft_context_preflight_with_lease(state, tool_name, args);
    let result = preflight
        .result
        .take()
        .or_else(|| workflow_tool_preflight_with_args(state, tool_name, args))
        .or_else(|| workflow_candidate_preflight(state, tool_name, args));
    let Some(result) = result else {
        return ExternalSdkToolPreflight {
            result: None,
            lease: preflight.lease,
        };
    };
    if preflight.lease.is_some() {
        workflow_tool_abort_with_args(
            state,
            preflight.lease.as_ref(),
            tool_name,
            args,
            "A later host preflight short-circuited the reserved context read",
        );
    }
    ExternalSdkToolPreflight {
        result: Some(call_tool_result_to_sdk_result(result)),
        lease: None,
    }
}

fn call_tool_result_to_sdk_result(
    result: rmcp::model::CallToolResult,
) -> copilot_sdk::ToolResultObject {
    let message = result
        .content
        .iter()
        .filter_map(|content| match &content.raw {
            rmcp::model::RawContent::Text(text) => Some(text.text.as_str()),
            _ => None,
        })
        .collect::<Vec<_>>()
        .join("\n");
    if result.is_error == Some(true) {
        copilot_sdk::ToolResultObject::error(message)
    } else {
        copilot_sdk::ToolResultObject::text(message)
    }
}

#[derive(Debug, Default)]
pub(super) struct ExternalContextPreflight {
    pub(super) result: Option<rmcp::model::CallToolResult>,
    pub(super) lease: Option<flow_like::flow::copilot::WorkflowToolLease>,
}

#[derive(Debug, Default)]
struct ExternalSdkToolPreflight {
    result: Option<copilot_sdk::ToolResultObject>,
    lease: Option<flow_like::flow::copilot::WorkflowToolLease>,
}

/// Database schema setup is useful, but it must not consume the mutation turn before the board
/// exists. Prompt guidance alone is not sufficient for code agents: a premature `create_table`
/// can open an approval dialog and wait for minutes while no recoverable FlowScript has ever been
/// submitted. Allow read-only database inspection, but require a queued board draft before schema
/// creation. The same guard is used by SDK and MCP providers.
pub(super) fn workflow_database_setup_preflight(
    state: &Arc<StdMutex<WorkflowToolLoopState>>,
    tool_name: &str,
    args: &serde_json::Value,
) -> Option<rmcp::model::CallToolResult> {
    if tool_name != "database_tool"
        || args.get("operation").and_then(serde_json::Value::as_str) != Some("create_table")
    {
        return None;
    }

    let board_draft_queued = match state.lock() {
        Ok(state) => state.queued,
        Err(_) => return Some(workflow_loop_state_unavailable_result()),
    };
    if board_draft_queued {
        return None;
    }

    Some(workflow_loop_result(
        serde_json::json!({
            "status": "deferred",
            "code": "board_draft_required_before_database_setup",
            "retryable": true,
            "next_action": "commit_workflow_draft",
            "message": "Submit and queue the complete board through commit_flowscript before creating database tables. The legacy edit_flowscript path is also accepted for compatibility. The schema request was not dispatched, no approval was opened, and no network request was made. Read-only table/schema inspection remains available."
        }),
        false,
    ))
}

/// Keep ancillary context reads from consuming the entire delegated run before any recoverable
/// source exists. The first few database/UI/storage inspections remain available for authoritative
/// context, but after that the specialist must retain a full-shape draft and let compiler
/// diagnostics drive any additional focused discovery.
#[cfg(test)]
pub(super) fn workflow_predraft_context_preflight(
    state: &Arc<StdMutex<WorkflowToolLoopState>>,
    tool_name: &str,
    args: &serde_json::Value,
) -> Option<rmcp::model::CallToolResult> {
    workflow_predraft_context_preflight_with_lease(state, tool_name, args).result
}

pub(super) fn workflow_predraft_context_preflight_with_lease(
    state: &Arc<StdMutex<WorkflowToolLoopState>>,
    tool_name: &str,
    args: &serde_json::Value,
) -> ExternalContextPreflight {
    if !matches!(tool_name, "database_tool" | "ui_inspect" | "storage_tool") {
        return ExternalContextPreflight::default();
    }

    let Ok(mut state) = state.lock() else {
        return ExternalContextPreflight {
            result: Some(workflow_loop_state_unavailable_result()),
            lease: None,
        };
    };
    let shared_elapsed_ms = state.shared_session_elapsed_ms();
    if let Some(session) = state.shared_session.as_mut() {
        let decision = match session.preflight_tool_call(tool_name, args, shared_elapsed_ms) {
            Ok(decision) => decision,
            Err(_) => {
                return ExternalContextPreflight {
                    result: Some(workflow_loop_state_unavailable_result()),
                    lease: None,
                };
            }
        };
        return ExternalContextPreflight {
            result: decision
                .short_circuit_result()
                .map(|payload| workflow_loop_result(payload, false)),
            lease: decision.lease().cloned(),
        };
    }
    if state.queued || state.flowscript_draft_retained || state.typed_draft_retained {
        return ExternalContextPreflight::default();
    }
    if state.predraft_context_reads >= MAX_EXTERNAL_PREDRAFT_CONTEXT_READS {
        return ExternalContextPreflight {
            result: Some(workflow_loop_result(
                serde_json::json!({
                    "status": "predraft_inspection_budget_exhausted",
                    "code": "PREDRAFT_INSPECTION_BUDGET_EXHAUSTED",
                    "retryable": true,
                    "next_action": if state.initial_declaration_lookup_usable {
                        if state.scope_plan.is_some() {
                            "write_flowscript"
                        } else {
                            "plan_board_scope"
                        }
                    } else {
                        "get_declarations"
                    },
                    "inspection_calls": state.predraft_context_reads,
                    "inspection_budget": MAX_EXTERNAL_PREDRAFT_CONTEXT_READS,
                    "message": "The bounded ancillary inspection budget is exhausted before a recoverable workflow draft exists. Reuse the database, UI, and storage context already returned. After one usable declaration batch, call plan_board_scope exactly once unless a plan is already accepted, then call write_flowscript for its active segment; do not repeat or exhaustively inventory schemas and pages."
                }),
                false,
            )),
            lease: None,
        };
    }
    state.predraft_context_reads = state.predraft_context_reads.saturating_add(1);
    ExternalContextPreflight::default()
}

pub(super) fn guard_sdk_workflow_tools(
    tools: Vec<(copilot_sdk::Tool, copilot_sdk::ToolHandler)>,
    state: Arc<StdMutex<WorkflowToolLoopState>>,
) -> Vec<(copilot_sdk::Tool, copilot_sdk::ToolHandler)> {
    let observer = WorkflowToolDispatchObserver::new(Some(&state), "sdk");
    guard_sdk_workflow_tools_with_observer(tools, state, observer)
}

fn guard_sdk_workflow_tools_with_observer(
    tools: Vec<(copilot_sdk::Tool, copilot_sdk::ToolHandler)>,
    state: Arc<StdMutex<WorkflowToolLoopState>>,
    observer: WorkflowToolDispatchObserver,
) -> Vec<(copilot_sdk::Tool, copilot_sdk::ToolHandler)> {
    let operation_gate = Arc::new(StdMutex::new(()));
    tools
        .into_iter()
        .map(|(tool, handler)| {
            let guarded_state = state.clone();
            let guarded_name = tool.name.clone();
            let operation_gate = operation_gate.clone();
            let observer = observer.clone();
            let guarded_handler: copilot_sdk::ToolHandler = Arc::new(move |called_name, args| {
                observer.record(&guarded_name, "arrival", None, None);
                // The SDK may dispatch sibling tool calls concurrently. Hold a lifecycle gate
                // through preflight, handler execution, and record so a late completion cannot
                // clear or overwrite the state of a newer typed/raw mutation.
                let _operation_guard = if is_order_sensitive_workflow_tool(&guarded_name) {
                    match operation_gate.try_lock() {
                        Ok(guard) => Some(guard),
                        // A handler that panicked while holding the gate poisons it permanently;
                        // its operation was already aborted below. Refusing every later mutation
                        // with the retryable "wait" answer would strand the run, so recover the
                        // gate instead of failing closed forever.
                        Err(std::sync::TryLockError::Poisoned(poisoned)) => {
                            operation_gate.clear_poison();
                            Some(poisoned.into_inner())
                        }
                        Err(std::sync::TryLockError::WouldBlock) => {
                            observer.record(&guarded_name, "preflight_short_circuit", None, Some("edit_in_flight"));
                            return copilot_sdk::ToolResultObject::error(
                                "Another order-sensitive workflow operation is still running. Wait for its retained revision/status before issuing the next mutation.",
                            );
                        }
                    }
                } else {
                    None
                };
                // Catch panics before they unwind past the held gate guard, and route them
                // through the same abort/cleanup as an MCP worker failure so `edit_in_flight`
                // cannot stay stuck for the rest of the session.
                let preflight =
                    workflow_tool_preflight_sdk(&guarded_state, &guarded_name, args);
                if let Some(result) = preflight.result {
                    observer.short_circuit_text(&guarded_name, result.error.as_deref().unwrap_or(&result.text_result_for_llm));
                    return result;
                }
                let lease = preflight.lease;
                let outcome = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                    observer.record(&guarded_name, "dispatched", None, None);
                    let mut result = handler(called_name, args);
                    let succeeded = result.result_type != "error"
                        && result.error.is_none()
                        && workflow_tool_result_succeeded(&result.text_result_for_llm);
                    workflow_tool_record_with_outcome(
                        &guarded_state,
                        lease.as_ref(),
                        &guarded_name,
                        args,
                        &result.text_result_for_llm,
                        succeeded,
                    );
                    annotate_modular_fallback_result(&guarded_state, &guarded_name, &mut result);
                    suppress_unchanged_flowscript_source_echo(&guarded_name, args, &mut result);
                    result
                }));
                match outcome {
                    Ok(result) => result,
                    Err(panic) => {
                        let message = format!(
                            "FlowPilot SDK tool '{guarded_name}' failed: {}",
                            panic_payload_message(panic.as_ref())
                        );
                        workflow_tool_abort_with_args(
                            &guarded_state,
                            lease.as_ref(),
                            &guarded_name,
                            args,
                            &message,
                        );
                        copilot_sdk::ToolResultObject::error(message)
                    }
                }
            });
            (tool, guarded_handler)
        })
        .collect()
}

fn panic_payload_message(panic: &(dyn std::any::Any + Send)) -> &str {
    panic
        .downcast_ref::<&str>()
        .copied()
        .or_else(|| panic.downcast_ref::<String>().map(String::as_str))
        .unwrap_or("the tool handler panicked without a message")
}

/// Attach the owning SDK chat cancellation token to synchronous tool handlers. The Copilot SDK
/// retains handlers in its session and may still be executing one after the async event loop is
/// cancelled; the frontend bridge reads this thread-local scope to stop its bounded per-tool wait
/// and emit cancellation to the webview.
pub(super) fn scope_sdk_tool_handlers(
    tools: Vec<(copilot_sdk::Tool, copilot_sdk::ToolHandler)>,
    cancellation: CancellationToken,
    activity: Option<Arc<SdkToolActivityRegistry>>,
) -> Vec<(copilot_sdk::Tool, copilot_sdk::ToolHandler)> {
    tools
        .into_iter()
        .map(|(tool, handler)| {
            let handler_cancellation = cancellation.clone();
            let handler_activity = activity.clone();
            let registered_tool_name = tool.name.clone();
            let scoped_handler: copilot_sdk::ToolHandler = Arc::new(move |called_name, args| {
                if handler_cancellation.is_cancelled() {
                    return copilot_sdk::ToolResultObject::error(
                        "The owning FlowPilot run was cancelled before this tool could execute.",
                    );
                }
                let outcome = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                    let _activity_guard = handler_activity
                        .as_ref()
                        .map(|activity| activity.begin(&registered_tool_name));
                    crate::functions::ai::frontend_tool_bridge::with_frontend_tool_execution_scope(
                        handler_cancellation.clone(),
                        None,
                        || handler(called_name, args),
                    )
                }));
                match outcome {
                    Ok(result) => result,
                    Err(panic) => {
                        let message = format!(
                            "FlowPilot SDK tool '{registered_tool_name}' panicked: {}",
                            panic_payload_message(panic.as_ref())
                        );
                        tracing::error!(tool = %registered_tool_name, error = %message, "SDK tool handler panicked");
                        copilot_sdk::ToolResultObject::error(message)
                    }
                }
            });
            (tool, scoped_handler)
        })
        .collect()
}

pub(super) fn is_workflow_loop_tool(tool_name: &str) -> bool {
    matches!(
        tool_name,
        "catalog_search"
            | "list_board_nodes"
            | "get_node_details"
            | "get_unconfigured_nodes"
            | "get_current_flowscript"
            | "get_declarations"
            | "plan_board_scope"
            | "extend_time_budget"
            | "write_flowscript"
            | "patch_flowscript"
            | "check_flowscript"
            | "test_flowscript"
            | "commit_flowscript"
            // Compatibility-only typed IR tools. New model surfaces do not advertise these.
            | "plan_flow_ir"
            | "begin_flow_ir_draft"
            | "update_flow_ir_draft"
            | "upsert_flow_ir_module"
            | "validate_flow_ir_draft"
            | "commit_flow_ir_draft"
            | "edit_flowscript"
            | "emit_commands"
    )
}

pub(super) fn is_workflow_commit_tool(tool_name: &str) -> bool {
    matches!(
        tool_name,
        "emit_commands" | "edit_flowscript" | "commit_flowscript" | "commit_flow_ir_draft"
    )
}

pub(super) fn is_flowscript_draft_operation_tool(tool_name: &str) -> bool {
    matches!(
        tool_name,
        "write_flowscript" | "patch_flowscript" | "check_flowscript" | "commit_flowscript"
    )
}

/// Redaction without truncation: compiler-receipt evidence needs the byte-for-byte authored
/// source, while tool results can still carry secret-shaped values that must never reach the
/// frontend un-redacted.
pub(super) fn full_redacted_tool_result(text: &str) -> String {
    flow_like::flow::copilot::stream::safe_tool_result_preview(text, usize::MAX)
}

pub(super) fn is_typed_ir_operation_tool(tool_name: &str) -> bool {
    matches!(
        tool_name,
        "plan_flow_ir"
            | "begin_flow_ir_draft"
            | "update_flow_ir_draft"
            | "upsert_flow_ir_module"
            | "validate_flow_ir_draft"
            | "commit_flow_ir_draft"
    )
}

pub(super) fn is_order_sensitive_workflow_tool(tool_name: &str) -> bool {
    matches!(
        tool_name,
        "write_flowscript"
            | "patch_flowscript"
            | "check_flowscript"
            | "test_flowscript"
            | "commit_flowscript"
            | "plan_flow_ir"
            | "begin_flow_ir_draft"
            | "update_flow_ir_draft"
            | "upsert_flow_ir_module"
            | "validate_flow_ir_draft"
            | "commit_flow_ir_draft"
            | "edit_flowscript"
            | "emit_commands"
    )
}

pub(super) fn typed_ir_module_count_hint(
    tool_name: &str,
    args: &serde_json::Value,
) -> Option<usize> {
    let array_len = |value: Option<&serde_json::Value>| value?.as_array().map(Vec::len);
    match tool_name {
        "plan_flow_ir" => [
            array_len(args.get("modules")),
            array_len(args.get("module_estimates")),
        ]
        .into_iter()
        .flatten()
        .max(),
        "begin_flow_ir_draft" | "update_flow_ir_draft" => [
            array_len(args.get("expected_modules")),
            array_len(
                args.get("capability_plan")
                    .and_then(|plan| plan.get("modules")),
            ),
        ]
        .into_iter()
        .flatten()
        .max(),
        "upsert_flow_ir_module" | "validate_flow_ir_draft" | "commit_flow_ir_draft" => Some(0),
        _ => None,
    }
}

pub(super) fn typed_ir_operation_budget(expected_modules: usize) -> u16 {
    u16::try_from(expected_modules)
        .unwrap_or(u16::MAX)
        .saturating_mul(3)
        .saturating_add(8)
        .clamp(
            MIN_EXTERNAL_TYPED_IR_OPERATION_BUDGET,
            MAX_EXTERNAL_TYPED_IR_OPERATION_BUDGET,
        )
}

pub(super) fn typed_ir_operation_target(tool_name: &str, args: &serde_json::Value) -> String {
    match tool_name {
        "plan_flow_ir" => "$plan".to_string(),
        "begin_flow_ir_draft" => "$draft".to_string(),
        "update_flow_ir_draft" => "$header".to_string(),
        "upsert_flow_ir_module" => args
            .pointer("/module/name")
            .and_then(serde_json::Value::as_str)
            .map(|name| format!("module:{name}"))
            .unwrap_or_else(|| "module:<invalid>".to_string()),
        "validate_flow_ir_draft" => "$validation".to_string(),
        "commit_flow_ir_draft" => "$commit".to_string(),
        _ => tool_name.to_string(),
    }
}

pub(super) fn typed_ir_result_proves_retained_draft(parsed: &serde_json::Value) -> bool {
    let revision_retained = parsed
        .get("revision")
        .and_then(serde_json::Value::as_u64)
        .is_some();
    let status = parsed.get("status").and_then(serde_json::Value::as_str);
    revision_retained
        && matches!(
            status,
            Some(
                "draft_started"
                    | "draft_updated"
                    | "draft_needs_repair"
                    | "module_validated"
                    | "module_needs_repair"
                    | "draft_valid"
                    | "scope_reduction_blocked"
                    | "candidate_regression"
                    | "resource_limit_rejected"
                    | "queued"
                    | "already_queued"
                    | "validation_errors"
                    | "infeasible"
                    | "revision_conflict"
                    | "error"
            )
        )
}

#[cfg(test)]
mod dispatch_telemetry_tests {
    use super::*;
    use std::sync::atomic::{AtomicUsize, Ordering};

    #[test]
    fn sdk_arrivals_distinguish_dispatched_handlers_from_preflight_short_circuits() {
        let events = Arc::new(StdMutex::new(Vec::new()));
        let recorded = events.clone();
        let observer =
            WorkflowToolDispatchObserver(Some(Arc::new(move |tool, phase, code, status| {
                recorded
                    .lock()
                    .unwrap()
                    .push((tool.to_string(), phase, code, status));
            })));
        let state = Arc::new(StdMutex::new(WorkflowToolLoopState::default()));
        state.lock().unwrap().current_reads = 0;
        let dispatched = Arc::new(AtomicUsize::new(0));
        let calls = dispatched.clone();
        let handler: copilot_sdk::ToolHandler = Arc::new(move |_, _| {
            calls.fetch_add(1, Ordering::SeqCst);
            copilot_sdk::ToolResultObject::text("{}")
        });
        let mut tools = guard_sdk_workflow_tools_with_observer(
            vec![(copilot_sdk::Tool::new("get_current_flowscript"), handler)],
            state,
            observer,
        );
        let (_, handler) = tools.pop().unwrap();
        let args = serde_json::json!({"ignored_private_argument": "never retained"});
        handler("get_current_flowscript", &args);
        handler("get_current_flowscript", &args);
        assert_eq!(dispatched.load(Ordering::SeqCst), 1);
        assert_eq!(
            *events.lock().unwrap(),
            vec![
                ("get_current_flowscript".into(), "arrival", None, None),
                ("get_current_flowscript".into(), "dispatched", None, None),
                ("get_current_flowscript".into(), "arrival", None, None),
                (
                    "get_current_flowscript".into(),
                    "preflight_short_circuit",
                    None,
                    Some("already_returned")
                ),
            ]
        );
    }

    #[test]
    fn mcp_preflight_tags_keep_only_fixed_codes_and_statuses() {
        let events = Arc::new(StdMutex::new(Vec::new()));
        let recorded = events.clone();
        let observer =
            WorkflowToolDispatchObserver(Some(Arc::new(move |_, phase, code, status| {
                recorded.lock().unwrap().push((phase, code, status));
            })));
        let result = workflow_loop_result(
            serde_json::json!({
                "code": "DECLARATION_LOOKUP_IN_FLIGHT", "status": "declaration_lookup_in_flight",
                "source": "private source", "message": "private message", "args": {"token": "private token"},
            }),
            true,
        );
        observer.short_circuit_mcp("get_declarations", &result);
        observer.short_circuit_text(
            "get_declarations",
            r#"{"code":"private code","status":"private status"}"#,
        );
        assert_eq!(
            *events.lock().unwrap(),
            vec![
                (
                    "preflight_short_circuit",
                    Some("DECLARATION_LOOKUP_IN_FLIGHT"),
                    Some("declaration_lookup_in_flight")
                ),
                ("preflight_short_circuit", Some("other"), Some("other")),
            ]
        );
        assert_eq!(
            preflight_dispatch_tags(&"x".repeat(65_537)),
            (Some("oversized_result"), Some("unknown"))
        );
        assert_eq!(
            preflight_dispatch_tags("unstructured private error"),
            (Some("unstructured_result"), Some("unknown"))
        );
    }
}
