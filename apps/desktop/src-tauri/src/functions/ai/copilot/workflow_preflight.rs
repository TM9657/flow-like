//! Ordered workflow preflight checks and candidate admission.

use super::stream_events::truncate_for_preview;
use super::workflow_declarations::{
    declaration_lookup_queries, declaration_queries_are_related,
    declaration_repair_query_is_bounded, declaration_repair_query_keys,
    diagnostic_declaration_repair_hints,
};
use super::workflow_sdk::{
    is_flowscript_draft_operation_tool, is_order_sensitive_workflow_tool,
    is_typed_ir_operation_tool, is_workflow_commit_tool, is_workflow_loop_tool,
    typed_ir_module_count_hint, typed_ir_operation_budget, workflow_loop_result,
    workflow_loop_state_unavailable_result,
};
use super::workflow_state::{
    MAX_EXTERNAL_SCOPE_PLAN_CALLS, MAX_EXTERNAL_SCOPE_PLAN_REJECTIONS,
    MAX_EXTERNAL_TYPED_IR_STALLED_ATTEMPTS, MAX_EXTERNAL_WORKFLOW_DECLARATION_CALLS,
    MAX_EXTERNAL_WORKFLOW_STALLED_EDIT_ATTEMPTS, MAX_INITIAL_DECLARATION_ATTEMPTS,
    MAX_REPAIR_DECLARATION_ATTEMPTS_PER_KEY, MAX_REPAIR_DECLARATION_QUERIES, TimeExtensionDecision,
    WorkflowMutationPath, WorkflowToolLoopState, submitted_flowscript,
};
use flow_like::flow::copilot::{
    BoardScopePlan, EmitCommandsArgs, PlanBoardScopeArgs, ScopeStrategy,
    emit_validation_requires_flowscript, profile_flowscript_candidate,
    validate_model_facing_emit_commands_scope, workflow_authoring_defers_runtime_tool,
    workflow_runtime_verification_deferred_payload,
};
use std::{
    collections::HashSet,
    sync::{Arc, Mutex as StdMutex},
};

fn emit_commands_representation_rejected(args: &serde_json::Value) -> bool {
    let Ok(args) = serde_json::from_value::<EmitCommandsArgs>(args.clone()) else {
        return false;
    };
    let scope = validate_model_facing_emit_commands_scope(&args);
    emit_validation_requires_flowscript(&scope)
}

pub(super) fn workflow_tool_preflight_with_args(
    state: &Arc<StdMutex<WorkflowToolLoopState>>,
    tool_name: &str,
    args: &serde_json::Value,
) -> Option<rmcp::model::CallToolResult> {
    let Ok(mut state) = state.lock() else {
        return Some(workflow_loop_state_unavailable_result());
    };

    // Board commands are staged until the specialist returns and the host applies them. Executing
    // during a mutation session would therefore test the pre-edit graph and could produce a false
    // green verification result. A later read/verification turn has no workflow guard and can run
    // these tools against the persisted board normally.
    if workflow_authoring_defers_runtime_tool(tool_name) {
        return Some(workflow_loop_result(
            workflow_runtime_verification_deferred_payload(),
            true,
        ));
    }

    if state.queued && is_workflow_loop_tool(tool_name) {
        return Some(workflow_loop_result(
            serde_json::json!({
                "status": "already_queued",
                "next_action": "stop",
                "message": "Workflow changes are already queued. Stop workflow tools and return a brief summary. If the user also requested UI, finish it with the UI tool only."
            }),
            false,
        ));
    }

    if is_order_sensitive_workflow_tool(tool_name)
        && let Some(circuit) = state
            .shared_session
            .as_ref()
            .map(|session| session.snapshot(state.shared_session_elapsed_ms()))
            .and_then(|snapshot| snapshot.circuit)
    {
        return Some(workflow_loop_result(
            serde_json::json!({
                "status": "zero_progress_circuit_open",
                "code": "WORKFLOW_ZERO_PROGRESS_CIRCUIT_OPEN",
                // The circuit cannot close within this provider phase (progress is only
                // recordable from a dispatched tool, and dispatch is refused while it is open).
                // retryable:true made external CLIs retry the refused call for the whole phase.
                "retryable": false,
                "next_action": "stop_for_host_continuation",
                "reason": circuit.reason,
                "consecutive_zero_progress_attempts": circuit.consecutive_zero_progress_attempts,
                "message": "The shared FlowPilot repair circuit opened after two host-observed attempts without lifecycle progress. Stop this provider phase. The host will preserve the retained artifact and latest diagnostics for one bounded, materially different continuation."
            }),
            true,
        ));
    }

    if state.declaration_lookup_in_flight
        && (tool_name == "get_declarations" || is_order_sensitive_workflow_tool(tool_name))
    {
        return Some(workflow_loop_result(
            serde_json::json!({
                "status": "declaration_lookup_in_flight",
                "code": "DECLARATION_LOOKUP_IN_FLIGHT",
                "retryable": true,
                "next_action": "wait",
                "message": "A declaration batch is already in flight. Wait for its authoritative coverage result before starting another lookup or writing source."
            }),
            false,
        ));
    }

    let requested_path = match tool_name {
        "plan_flow_ir"
        | "begin_flow_ir_draft"
        | "update_flow_ir_draft"
        | "upsert_flow_ir_module"
        | "validate_flow_ir_draft"
        | "commit_flow_ir_draft" => Some(WorkflowMutationPath::TypedIr),
        "edit_flowscript" | "write_flowscript" | "patch_flowscript" | "check_flowscript"
        | "commit_flowscript" => Some(WorkflowMutationPath::FlowScript),
        "emit_commands" if emit_commands_representation_rejected(args) => None,
        "emit_commands" => Some(WorkflowMutationPath::DirectCommands),
        _ => None,
    };
    if tool_name == "emit_commands" && requested_path.is_none() {
        // The command tool itself returns the representation guidance. Do not let a rejected
        // executable command batch reserve an operation lease or claim a mutation path before the
        // model switches to FlowScript.
        return None;
    }
    if let (Some(active), Some(requested)) = (state.mutation_path, requested_path)
        && active != requested
    {
        return Some(workflow_loop_result(
            serde_json::json!({
                "status": "mutation_path_conflict",
                "code": "WORKFLOW_MUTATION_PATH_CONFLICT",
                "retryable": false,
                "next_action": match active {
                    WorkflowMutationPath::TypedIr => "continue_typed_draft",
                    WorkflowMutationPath::FlowScript => "continue_flowscript_draft",
                    WorkflowMutationPath::DirectCommands => "continue_direct_commands",
                },
                "message": "A workflow mutation path is already active for this change. Continue the retained FlowScript source path (or the legacy compatibility path that already owns this run); do not mix mutation representations in one atomic edit."
            }),
            true,
        ));
    }

    if is_flowscript_draft_operation_tool(tool_name) && state.flowscript_draft_retained {
        let requested_draft_id = args.get("draft_id").and_then(serde_json::Value::as_str);
        let exact_draft_id = state.flowscript_draft_id.as_deref();
        let wrong_draft = requested_draft_id.is_some() && requested_draft_id != exact_draft_id;
        let missing_draft = !args.is_null() && requested_draft_id.is_none();
        let expected_revision = args
            .get("expected_revision")
            .and_then(serde_json::Value::as_u64);
        let revision_required = matches!(
            tool_name,
            "patch_flowscript" | "check_flowscript" | "commit_flowscript"
        );
        let wrong_revision =
            revision_required && !args.is_null() && expected_revision != state.flowscript_revision;
        if wrong_draft || missing_draft || wrong_revision {
            return Some(workflow_loop_result(
                serde_json::json!({
                    "status": "retained_revision_required",
                    "code": "FLOWSCRIPT_RETAINED_REVISION_REQUIRED",
                    "retryable": true,
                    "next_action": match state.last_status.as_deref() {
                        Some("valid" | "draft_started" | "draft_updated") => "commit_flowscript",
                        Some("validation_errors" | "error" | "no_changes") => "patch_flowscript",
                        _ => "check_flowscript",
                    },
                    "draft_id": exact_draft_id,
                    "expected_revision": state.flowscript_revision,
                    "message": "This run owns an exact retained FlowScript draft. Continue its host-authorized draft id and revision; a different or stale source session was not dispatched."
                }),
                false,
            ));
        }
    }

    if is_flowscript_draft_operation_tool(tool_name)
        && tool_name != "write_flowscript"
        && !state.flowscript_draft_retained
    {
        return Some(workflow_loop_result(
            serde_json::json!({
                "status": "flowscript_draft_required",
                "code": "FLOWSCRIPT_DRAFT_REQUIRED",
                "retryable": true,
                "next_action": if state.needs_initial_declaration_coverage() {
                    "get_declarations"
                } else if state.scope_plan.is_none() {
                    "plan_board_scope"
                } else {
                    "write_flowscript"
                },
                "message": "No host-authorized FlowScript draft is retained for this run. Obtain live declaration coverage, call plan_board_scope exactly once unless a plan is already accepted, then call write_flowscript for its active segment; patch, check, and commit cannot create or guess a draft."
            }),
            false,
        ));
    }

    if state.edit_in_flight && is_order_sensitive_workflow_tool(tool_name) {
        return Some(workflow_loop_result(
            serde_json::json!({
                "status": "edit_in_flight",
                "next_action": "wait",
                "message": "Another order-sensitive workflow operation is still running. Wait for its retained revision/status before issuing the next FlowScript or compatibility mutation."
            }),
            true,
        ));
    }

    let typed_operation = is_typed_ir_operation_tool(tool_name);
    if typed_operation && let Some(module_count) = typed_ir_module_count_hint(tool_name, args) {
        state.typed_expected_modules = state.typed_expected_modules.max(module_count);
    }
    let typed_loop_active =
        state.mutation_path == Some(WorkflowMutationPath::TypedIr) || typed_operation;
    if typed_loop_active && is_workflow_loop_tool(tool_name) {
        let operation_budget = typed_ir_operation_budget(state.typed_expected_modules);
        if state.typed_stalled_attempts >= MAX_EXTERNAL_TYPED_IR_STALLED_ATTEMPTS {
            return Some(workflow_loop_result(
                serde_json::json!({
                    "status": "typed_repair_progress_stalled",
                    "code": "TYPED_IR_REPAIR_PROGRESS_STALLED",
                    "retryable": false,
                    "next_action": if state.typed_draft_retained {
                        "stop_and_resume_retained_draft_in_new_run"
                    } else {
                        "stop_and_report_begin_failure"
                    },
                    "draft_retained": state.typed_draft_retained,
                    "draft_id": state.typed_draft_id.as_deref(),
                    "revision": state.typed_revision,
                    "operation_attempts": state.typed_operation_attempts,
                    "operation_budget": operation_budget,
                    "stalled_attempts": state.typed_stalled_attempts,
                    "remaining_diagnostics": &state.last_errors,
                    "missing_modules": &state.typed_missing_modules,
                    "message": if state.typed_draft_retained {
                        "The same typed module or draft repair has repeated an already-seen diagnostic state. No operation was dispatched. Stop this run and report the retained draft id, revision, missing modules, and remaining diagnostics; a later run can resume that exact draft."
                    } else {
                        "The typed planner/begin loop repeated an already-seen diagnostic state before a draft was retained. No operation was dispatched. Stop this run and report the attempted draft id and remaining diagnostics."
                    }
                }),
                true,
            ));
        }
        if state.typed_operation_attempts >= operation_budget {
            return Some(workflow_loop_result(
                serde_json::json!({
                    "status": "typed_repair_budget_exhausted",
                    "code": "TYPED_IR_OPERATION_BUDGET_EXHAUSTED",
                    "retryable": false,
                    "next_action": if state.typed_draft_retained {
                        "stop_and_resume_retained_draft_in_new_run"
                    } else {
                        "stop_and_report_begin_failure"
                    },
                    "draft_retained": state.typed_draft_retained,
                    "draft_id": state.typed_draft_id.as_deref(),
                    "revision": state.typed_revision,
                    "operation_attempts": state.typed_operation_attempts,
                    "operation_budget": operation_budget,
                    "stalled_attempts": state.typed_stalled_attempts,
                    "remaining_diagnostics": &state.last_errors,
                    "missing_modules": &state.typed_missing_modules,
                    "message": if state.typed_draft_retained {
                        "The module-scaled typed-IR operation budget is exhausted. No operation was dispatched. Stop this run and report the retained draft id, revision, missing modules, and remaining diagnostics; a later run can resume that exact draft."
                    } else {
                        "The typed planner/begin operation budget is exhausted before a draft was retained. No operation was dispatched. Stop this run and report the attempted draft id and remaining diagnostics."
                    }
                }),
                true,
            ));
        }
    }

    let checked_valid_commit =
        tool_name == "commit_flowscript" && state.last_status.as_deref() == Some("valid");
    let flowscript_operation_budget = state.flowscript_operation_budget();
    if is_flowscript_draft_operation_tool(tool_name) && !checked_valid_commit {
        if state.stalled_edit_attempts >= MAX_EXTERNAL_WORKFLOW_STALLED_EDIT_ATTEMPTS {
            return Some(workflow_loop_result(
                serde_json::json!({
                    "status": "edit_progress_stalled",
                    "code": "FLOWSCRIPT_REPAIR_PROGRESS_STALLED",
                    "retryable": false,
                    "next_action": "stop_and_resume_retained_draft_in_new_run",
                    "draft_retained": state.flowscript_draft_retained,
                    "draft_id": state.flowscript_draft_id.as_deref(),
                    "revision": state.flowscript_revision,
                    "operation_attempts": state.flowscript_operation_attempts,
                    "operation_budget": flowscript_operation_budget,
                    "errors": state.last_errors,
                    "message": "The FlowScript repair loop revisited an already-seen compiler state too many times. No source operation was dispatched. Stop this run and report the retained revision and remaining diagnostics."
                }),
                true,
            ));
        }
        if state.flowscript_operation_attempts >= flowscript_operation_budget {
            return Some(workflow_loop_result(
                serde_json::json!({
                    "status": "edit_budget_exhausted",
                    "code": "FLOWSCRIPT_OPERATION_BUDGET_EXHAUSTED",
                    "retryable": false,
                    "next_action": "stop_and_resume_retained_draft_in_new_run",
                    "draft_retained": state.flowscript_draft_retained,
                    "draft_id": state.flowscript_draft_id.as_deref(),
                    "revision": state.flowscript_revision,
                    "operation_attempts": state.flowscript_operation_attempts,
                    "operation_budget": flowscript_operation_budget,
                    "errors": state.last_errors,
                    "message": "The total FlowScript write/patch/check operation budget is exhausted. No source operation was dispatched; the latest retained revision remains available for a later run."
                }),
                true,
            ));
        }
    }

    match tool_name {
        "plan_flow_ir" => {
            state
                .mutation_path
                .get_or_insert(WorkflowMutationPath::TypedIr);
            state.typed_operation_attempts = state.typed_operation_attempts.saturating_add(1);
            None
        }
        "begin_flow_ir_draft"
        | "update_flow_ir_draft"
        | "upsert_flow_ir_module"
        | "validate_flow_ir_draft" => {
            state
                .mutation_path
                .get_or_insert(WorkflowMutationPath::TypedIr);
            state.typed_operation_attempts = state.typed_operation_attempts.saturating_add(1);
            state.edit_in_flight = true;
            None
        }
        // The decision comes from the progress ledger, never from the model's account of itself.
        "extend_time_budget" => {
            state.last_extension_rationale = args
                .get("progress")
                .and_then(serde_json::Value::as_str)
                .map(str::trim)
                .filter(|progress| !progress.is_empty())
                .map(str::to_string);
            let decision = state.try_grant_time_extension();
            let remaining_titles = state
                .scope_plan
                .as_ref()
                .map(BoardScopePlan::uncommitted_titles)
                .unwrap_or_default();
            let payload = match decision {
                TimeExtensionDecision::Granted { earned, grants } => serde_json::json!({
                    "status": "time_budget_extended",
                    "retryable": false,
                    "next_action": "continue_building",
                    "granted_extensions": grants,
                    "earned_minutes": earned.as_secs() / 60,
                    "remaining_segments": remaining_titles,
                    "message": "Progress since the last extension is confirmed, so this run earned another slice of wall clock along with more write, check and commit budget. Keep building the active segment."
                }),
                TimeExtensionDecision::NoProgress { earned } => serde_json::json!({
                    "status": "time_budget_refused",
                    "code": "TIME_EXTENSION_NO_PROGRESS",
                    "retryable": false,
                    "next_action": "stop_and_report_blocked",
                    "earned_minutes": earned.as_secs() / 60,
                    "message": "Nothing measurable advanced since the last extension: no segment reached the board, no revision checked valid, the retained document did not grow, and no new compiler state was reached. More time would repeat the same work. Commit whatever already validates, then report the remaining diagnostics honestly."
                }),
                TimeExtensionDecision::CeilingReached { earned } => serde_json::json!({
                    "status": "time_budget_refused",
                    "code": "TIME_EXTENSION_CEILING_REACHED",
                    "retryable": false,
                    "next_action": "commit_flowscript",
                    "earned_minutes": earned.as_secs() / 60,
                    "remaining_segments": remaining_titles,
                    "message": "This run is at the maximum wall clock a single board build may use. Commit what already validates so the completed work reaches the board, and report which segments remain."
                }),
                TimeExtensionDecision::NotExtendable => serde_json::json!({
                    "status": "time_budget_refused",
                    "code": "TIME_EXTENSION_NOT_APPLICABLE",
                    "retryable": false,
                    "next_action": "stop",
                    "message": "Work is already queued for review; there is nothing left in this run to spend more time on."
                }),
            };
            Some(workflow_loop_result(payload, false))
        }
        // Planning is host-owned state, so the accepted plan is recorded here rather than trusted
        // from the tool's own response. The tool handler mirrors this exact validation.
        "plan_board_scope" => {
            // A fresh external provider process can replay the plan it received in continuation
            // context. Recognize that before enforcing the revision-call ceiling so an identical
            // replay remains a read of host state rather than a second planning mutation.
            if state.scope_plan.is_some()
                && let Ok(parsed) = serde_json::from_value::<PlanBoardScopeArgs>(args.clone())
                && let Some(payload) = state.repeated_scope_plan_payload(&parsed)
            {
                return Some(workflow_loop_result(payload, false));
            }
            if state.scope_plan_calls >= MAX_EXTERNAL_SCOPE_PLAN_CALLS {
                return Some(workflow_loop_result(
                    serde_json::json!({
                        "status": "scope_plan_budget_exhausted",
                        "code": "SCOPE_PLAN_BUDGET_EXHAUSTED",
                        "retryable": false,
                        "next_action": "write_flowscript",
                        "message": "The scope plan may be revised once. Build the active segment with the plan you already have, and report honestly if it cannot be completed."
                    }),
                    true,
                ));
            }
            if state.scope_plan_rejections >= MAX_EXTERNAL_SCOPE_PLAN_REJECTIONS {
                return Some(workflow_loop_result(
                    serde_json::json!({
                        "status": "scope_plan_budget_exhausted",
                        "code": "SCOPE_PLAN_REJECTION_BUDGET_EXHAUSTED",
                        "retryable": false,
                        "next_action": "stop_and_report_blocked",
                        "message": "Too many malformed scope plans in a row. Stop and report what is blocking a plan instead of reshaping it again."
                    }),
                    true,
                ));
            }
            let parsed: PlanBoardScopeArgs = match serde_json::from_value(args.clone()) {
                Ok(parsed) => parsed,
                Err(error) => {
                    state.scope_plan_rejections = state.scope_plan_rejections.saturating_add(1);
                    return Some(workflow_loop_result(
                        serde_json::json!({
                            "status": "scope_plan_rejected",
                            "code": "SCOPE_PLAN_ARGUMENTS_INVALID",
                            "retryable": true,
                            "next_action": "plan_board_scope",
                            "message": format!(
                                "Failed to parse plan_board_scope arguments against its advertised schema: {error}"
                            ),
                        }),
                        true,
                    ));
                }
            };
            // Only an ACCEPTED plan consumes the revision budget. A malformed proposal must stay
            // fixable, or one schema slip would strand the run with no plan and no way to make one.
            match state.accept_scope_plan_args(parsed) {
                Ok(payload) => {
                    state.scope_plan_calls = state.scope_plan_calls.saturating_add(1);
                    Some(workflow_loop_result(payload, false))
                }
                Err(rejection) => {
                    state.scope_plan_rejections = state.scope_plan_rejections.saturating_add(1);
                    Some(workflow_loop_result(rejection.payload(), true))
                }
            }
        }
        "write_flowscript" if state.needs_initial_declaration_coverage() => {
            if state.initial_declaration_attempts >= MAX_INITIAL_DECLARATION_ATTEMPTS {
                return Some(workflow_loop_result(
                    serde_json::json!({
                        "status": "declaration_coverage_exhausted",
                        "code": "DECLARATION_COVERAGE_EXHAUSTED",
                        "retryable": false,
                        "next_action": "stop_and_report_unavailable_capabilities",
                        "attempts": state.initial_declaration_attempts,
                        "unresolved_queries": state.unresolved_declaration_queries,
                        "message": "No bounded initial declaration attempt returned a usable live signature. No FlowScript source write was dispatched; report the unavailable core capabilities instead of guessing names or pins."
                    }),
                    true,
                ));
            }
            Some(workflow_loop_result(
                serde_json::json!({
                    "status": "declaration_lookup_required",
                    "retryable": true,
                    "next_action": "get_declarations",
                    "message": "Before the first FlowScript draft, make one bounded get_declarations batch for the highest-leverage catalog calls needed to establish the end-to-end shape. Do not enumerate every utility operation. After any usable live result, call plan_board_scope exactly once, then write and retain its active segment immediately; compiler diagnostics authorize focused later lookups."
                }),
                false,
            ))
        }
        // Declarations first, then the plan, then source. Planning before the catalog is known
        // produces segments that cannot be built; planning after the first write is too late to
        // make that write small, which is the entire point.
        "write_flowscript" if state.scope_plan.is_none() => Some(workflow_loop_result(
            serde_json::json!({
                "status": "scope_plan_required",
                "code": "SCOPE_PLAN_REQUIRED",
                "retryable": true,
                "next_action": "plan_board_scope",
                "message": "Call plan_board_scope before the first source write. An ordinary edit is one segment with strategy \"single\" and proceeds exactly as before; split only a build too large to compose in one pass, so that this first write stays small enough to land."
            }),
            false,
        )),
        // Growing the draft further would spend wall clock the run no longer has. Commit the
        // validated prefix so the segments already built survive as real applied progress.
        "write_flowscript"
            if state.staged_prefix_commit_requested
                && state.scope_plan.as_ref().is_some_and(|plan| {
                    plan.strategy == ScopeStrategy::Staged && plan.active > 0 && !plan.is_complete()
                }) =>
        {
            let (validated, remaining, titles) = state
                .scope_plan
                .as_ref()
                .map(|plan| (plan.active, plan.remaining(), plan.uncommitted_titles()))
                .unwrap_or_default();
            Some(workflow_loop_result(
                serde_json::json!({
                    "status": "commit_validated_prefix",
                    "code": "SCOPE_PLAN_COMMIT_VALIDATED_PREFIX",
                    "retryable": false,
                    "next_action": "commit_flowscript",
                    "segments_validated": validated,
                    "segments_remaining": remaining,
                    "remaining_titles": titles,
                    "message": "This run is running out of wall clock to grow the draft further. Commit the revision that already checked valid so the completed segments reach the board; the remaining segments continue in a fresh run against the applied board. Do not shrink or rewrite the validated source first."
                }),
                false,
            ))
        }
        "write_flowscript" | "patch_flowscript" => {
            state
                .mutation_path
                .get_or_insert(WorkflowMutationPath::FlowScript);
            state.flowscript_operation_attempts =
                state.flowscript_operation_attempts.saturating_add(1);
            state.edit_in_flight = true;
            if tool_name == "write_flowscript" {
                state.in_flight_flowscript = submitted_flowscript(args).map(str::to_string);
            }
            None
        }
        "check_flowscript"
            if state.stalled_edit_attempts >= MAX_EXTERNAL_WORKFLOW_STALLED_EDIT_ATTEMPTS =>
        {
            Some(workflow_loop_result(
                serde_json::json!({
                    "status": "edit_progress_stalled",
                    "next_action": "stop",
                    "errors": state.last_errors,
                    "message": "The last FlowScript checks repeated the same unresolved compiler diagnostics. Stop this bounded loop and report those diagnostics; the complete retained source remains resumable."
                }),
                true,
            ))
        }
        "check_flowscript" if state.edit_attempts >= state.edit_attempt_budget() => {
            Some(workflow_loop_result(
                serde_json::json!({
                    "status": "edit_budget_exhausted",
                    "next_action": "stop",
                    "errors": state.last_errors,
                    "message": "The bounded FlowScript check/repair budget is exhausted. Stop broad discovery and report the remaining compiler diagnostics honestly."
                }),
                true,
            ))
        }
        "check_flowscript" => {
            state
                .mutation_path
                .get_or_insert(WorkflowMutationPath::FlowScript);
            state.edit_attempts = state.edit_attempts.saturating_add(1);
            state.flowscript_operation_attempts =
                state.flowscript_operation_attempts.saturating_add(1);
            state.edit_in_flight = true;
            None
        }
        "commit_flowscript"
            if state.last_status.as_deref() != Some("valid")
                && state.stalled_edit_attempts >= MAX_EXTERNAL_WORKFLOW_STALLED_EDIT_ATTEMPTS =>
        {
            Some(workflow_loop_result(
                serde_json::json!({
                    "status": "edit_progress_stalled",
                    "next_action": "stop",
                    "errors": state.last_errors,
                    "message": "Commit cannot bypass a repeatedly failing FlowScript check. Stop and report the retained revision and its remaining compiler diagnostics."
                }),
                true,
            ))
        }
        "commit_flowscript"
            if state.last_status.as_deref() != Some("valid")
                && state.edit_attempts >= state.edit_attempt_budget() =>
        {
            Some(workflow_loop_result(
                serde_json::json!({
                    "status": "edit_budget_exhausted",
                    "next_action": "stop",
                    "errors": state.last_errors,
                    "message": "Commit requires a valid exact revision, and the bounded FlowScript check budget is exhausted. Nothing was queued."
                }),
                true,
            ))
        }
        // A successful check is the bounded validation attempt. Commit only claims that exact
        // retained revision, so a valid revision remains committable even at the check ceiling.
        "commit_flowscript" => {
            if state.flowscript_commit_attempts >= state.commit_attempt_budget() {
                return Some(workflow_loop_result(
                    serde_json::json!({
                        "status": "commit_retry_budget_exhausted",
                        "code": "FLOWSCRIPT_COMMIT_RETRY_BUDGET_EXHAUSTED",
                        "retryable": false,
                        "next_action": "stop_and_resume_retained_draft_in_new_run",
                        "draft_retained": state.flowscript_draft_retained,
                        "draft_id": state.flowscript_draft_id.as_deref(),
                        "revision": state.flowscript_revision,
                        "attempts": state.flowscript_commit_attempts,
                        "message": "The exact valid revision could not complete its bounded commit attempts. Stop this run without rewriting the checked source; the retained revision can be resumed later."
                    }),
                    true,
                ));
            }
            state
                .mutation_path
                .get_or_insert(WorkflowMutationPath::FlowScript);
            state.flowscript_operation_attempts =
                state.flowscript_operation_attempts.saturating_add(1);
            state.flowscript_commit_attempts = state.flowscript_commit_attempts.saturating_add(1);
            state.edit_in_flight = true;
            None
        }
        "catalog_search" | "list_board_nodes" | "get_node_details" | "get_unconfigured_nodes" => {
            Some(workflow_loop_result(
                serde_json::json!({
                    "status": "discovery_blocked",
                    "next_action": if state.flowscript_draft_retained {
                        "continue_workflow_draft"
                    } else if state.needs_initial_declaration_coverage() {
                        "get_declarations"
                    } else if state.scope_plan.is_none() {
                        "plan_board_scope"
                    } else {
                        "write_flowscript"
                    },
                    "message": "This is a workflow mutation run. Broad catalog/graph discovery is disabled. The embedded FlowScript render IS the current board. Use one bounded get_declarations batch for the highest-leverage calls, call plan_board_scope exactly once unless the host already retained a plan, then write_flowscript, patch the retained source, and commit_flowscript once diagnostics are clear."
                }),
                true,
            ))
        }
        "get_current_flowscript" if state.current_reads >= 1 => Some(workflow_loop_result(
            serde_json::json!({
                "status": "already_returned",
                "next_action": if state.needs_initial_declaration_coverage() {
                    "get_declarations"
                } else if state.scope_plan.is_none() {
                    "plan_board_scope"
                } else {
                    "write_flowscript"
                },
                "message": "The current FlowScript was already returned in this run; do not fetch it again. Continue in order with one usable declaration batch, one accepted scope plan, then write_flowscript for its active segment."
            }),
            true,
        )),
        "get_current_flowscript" => {
            state.current_reads = state.current_reads.saturating_add(1);
            None
        }
        "get_declarations" if state.needs_initial_declaration_coverage() => {
            let queries = declaration_lookup_queries(args);
            if !queries.iter().any(|query| !query.trim().is_empty()) {
                return Some(workflow_loop_result(
                    serde_json::json!({
                        "status": "declaration_batch_required",
                        "retryable": true,
                        "next_action": "get_declarations",
                        "unresolved_queries": state.unresolved_declaration_queries,
                        "message": "The initial declaration lookup must contain focused queries in `query` or `queries`. Submit one bounded batch for the highest-leverage catalog calls needed to establish the end-to-end shape; an empty guidance lookup does not unlock FlowScript authoring."
                    }),
                    false,
                ));
            }
            if !state.unresolved_declaration_queries.is_empty()
                && queries.iter().any(|query| {
                    !query.trim().is_empty()
                        && !state
                            .unresolved_declaration_queries
                            .iter()
                            .any(|unresolved| declaration_queries_are_related(unresolved, query))
                })
            {
                return Some(workflow_loop_result(
                    serde_json::json!({
                        "status": "declaration_follow_up_unrelated",
                        "code": "DECLARATION_FOLLOW_UP_UNRELATED",
                        "retryable": true,
                        "next_action": "get_declarations",
                        "unresolved_queries": state.unresolved_declaration_queries,
                        "message": "A partial declaration batch may be followed only by the exact unresolved capabilities or focused rephrasings that retain a distinctive capability term. Unrelated lookups do not consume an attempt and cannot unlock source authoring."
                    }),
                    false,
                ));
            }
            if state.initial_declaration_attempts >= MAX_INITIAL_DECLARATION_ATTEMPTS {
                return Some(workflow_loop_result(
                    serde_json::json!({
                        "status": "declaration_coverage_exhausted",
                        "code": "DECLARATION_COVERAGE_EXHAUSTED",
                        "retryable": false,
                        "next_action": "stop_and_report_unavailable_capabilities",
                        "attempts": state.initial_declaration_attempts,
                        "unresolved_queries": state.unresolved_declaration_queries,
                        "message": "The bounded initial declaration lookup did not return a usable live signature. No source write was dispatched. Stop and report the exact unmatched capabilities instead of guessing names or pins."
                    }),
                    true,
                ));
            }
            state.initial_declaration_attempts =
                state.initial_declaration_attempts.saturating_add(1);
            state.declaration_calls = state.declaration_calls.saturating_add(1);
            state.declarations_since_edit = state.declarations_since_edit.saturating_add(1);
            state.declaration_lookup_in_flight = true;
            None
        }
        "get_declarations"
            if state.declaration_calls == 0
                && !declaration_lookup_queries(args)
                    .iter()
                    .any(|query| !query.trim().is_empty()) =>
        {
            Some(workflow_loop_result(
                serde_json::json!({
                    "status": "declaration_batch_required",
                    "retryable": true,
                    "next_action": "get_declarations",
                    "message": "Declaration lookup requires at least one focused capability query."
                }),
                false,
            ))
        }
        "get_declarations" if state.declarations_since_edit >= 1 => Some(workflow_loop_result(
            serde_json::json!({
                "status": "discovery_budget_exhausted",
                "next_action": if state.flowscript_draft_retained {
                    "patch_flowscript"
                } else if state.scope_plan.is_none() {
                    "plan_board_scope"
                } else {
                    "write_flowscript"
                },
                "message": "A usable declaration batch is retained. If no source exists, call plan_board_scope exactly once unless the host already retained a plan, then submit its active segment with write_flowscript. Otherwise patch the retained revision. Do not chase omitted or unmatched entries before the first draft; use compiler diagnostics for focused follow-up lookups."
            }),
            false,
        )),
        "get_declarations" if state.declaration_calls == 0 => {
            state.declaration_calls = state.declaration_calls.saturating_add(1);
            state.declarations_since_edit = state.declarations_since_edit.saturating_add(1);
            state.declaration_lookup_in_flight = true;
            None
        }
        "get_declarations" => {
            if state.declaration_calls >= MAX_EXTERNAL_WORKFLOW_DECLARATION_CALLS {
                return Some(workflow_loop_result(
                    serde_json::json!({
                        "status": "discovery_budget_exhausted",
                        "next_action": "patch_flowscript",
                        "message": "The diagnostic-driven declaration lookup safety cap is exhausted. Continue repairing the retained draft with the declarations already returned."
                    }),
                    false,
                ));
            }

            let eligible = diagnostic_declaration_repair_hints(&state.last_errors);
            let queries = declaration_lookup_queries(args);
            if queries.is_empty()
                || queries.len() > MAX_REPAIR_DECLARATION_QUERIES
                || queries
                    .iter()
                    .any(|query| !declaration_repair_query_is_bounded(query))
            {
                return Some(workflow_loop_result(
                    serde_json::json!({
                        "status": "diagnostic_lookup_required",
                        "next_action": "patch_flowscript",
                        "eligible_targets": eligible.exposed_targets(),
                        "max_queries": MAX_REPAIR_DECLARATION_QUERIES,
                        "message": "Repair declaration discovery must be a short bounded batch tied to the latest validation diagnostics. Broad, oversized, or empty discovery was not dispatched."
                    }),
                    false,
                ));
            }

            let matches = queries
                .iter()
                .map(|query| declaration_repair_query_keys(query, &eligible))
                .collect::<Vec<_>>();
            let matched_query_count = matches.iter().filter(|keys| !keys.is_empty()).count();
            let requested = matches.into_iter().flatten().collect::<HashSet<_>>();
            // Permit a bounded batch to include a few plausible alternatives, but require at least
            // half of its focused searches to be justified by the current diagnostics. Completed
            // target keys prevent using the same match to reopen broad discovery after each edit.
            if eligible.is_empty()
                || requested.is_empty()
                || matched_query_count.saturating_mul(2) < queries.len()
            {
                return Some(workflow_loop_result(
                    serde_json::json!({
                        "status": "diagnostic_lookup_required",
                        "next_action": "patch_flowscript",
                        "eligible_targets": eligible.exposed_targets(),
                        "message": "A repair declaration batch must keep at least half of its focused searches tied to a node/function from the latest pin or catalog diagnostic, or to the comparison/conversion/string-operation topic identified by that diagnostic. Unrelated discovery was not dispatched."
                    }),
                    false,
                ));
            }

            let new_requested = requested
                .difference(&state.completed_repair_lookup_keys)
                .filter(|key| !state.in_flight_repair_lookup_keys.contains(*key))
                .filter(|key| {
                    state
                        .repair_lookup_attempts
                        .get(*key)
                        .copied()
                        .unwrap_or_default()
                        < MAX_REPAIR_DECLARATION_ATTEMPTS_PER_KEY
                })
                .cloned()
                .collect::<HashSet<_>>();
            if new_requested.is_empty() {
                return Some(workflow_loop_result(
                    serde_json::json!({
                        "status": "duplicate_declaration_lookup",
                        "next_action": "patch_flowscript",
                        "targets": requested,
                        "message": "These diagnostic repair targets were already resolved, definitively unavailable, or exhausted their bounded exact-signature retry. The duplicate request was not dispatched; apply retained declarations or report the unavailable capability."
                    }),
                    false,
                ));
            }

            for key in &new_requested {
                let attempts = state.repair_lookup_attempts.entry(key.clone()).or_default();
                *attempts = attempts.saturating_add(1);
            }
            state.in_flight_repair_lookup_keys = new_requested;
            state.declaration_calls = state.declaration_calls.saturating_add(1);
            state.declarations_since_edit = state.declarations_since_edit.saturating_add(1);
            state.declaration_lookup_in_flight = true;
            None
        }
        "commit_flow_ir_draft" => {
            state
                .mutation_path
                .get_or_insert(WorkflowMutationPath::TypedIr);
            state.typed_operation_attempts = state.typed_operation_attempts.saturating_add(1);
            state.edit_attempts = state.edit_attempts.saturating_add(1);
            state.edit_in_flight = true;
            None
        }
        tool if is_workflow_commit_tool(tool) && state.edit_in_flight => {
            Some(workflow_loop_result(
                serde_json::json!({
                    "status": "edit_in_flight",
                    "next_action": "wait",
                    "message": "Another workflow commit is still running. Do not submit parallel commits; wait for its validation result and revise that same typed revision or FlowScript draft if needed."
                }),
                true,
            ))
        }
        tool if is_workflow_commit_tool(tool)
            && state.stalled_edit_attempts >= MAX_EXTERNAL_WORKFLOW_STALLED_EDIT_ATTEMPTS =>
        {
            Some(workflow_loop_result(
                serde_json::json!({
                    "status": "edit_progress_stalled",
                    "next_action": "stop",
                    "errors": state.last_errors,
                    "message": "The last repair attempts repeated the same unresolved validation diagnostics. Stop this bounded loop and report those diagnostics; the best full-scope draft remains retained."
                }),
                true,
            ))
        }
        tool if is_workflow_commit_tool(tool)
            && state.edit_attempts >= state.edit_attempt_budget() =>
        {
            Some(workflow_loop_result(
                serde_json::json!({
                    "status": "edit_budget_exhausted",
                    "next_action": "stop",
                    "errors": state.last_errors,
                    "message": "The bounded FlowScript repair budget is exhausted. Stop broad discovery and report the remaining validation diagnostics honestly."
                }),
                true,
            ))
        }
        tool if is_workflow_commit_tool(tool) => {
            if let Some(requested_path) = requested_path {
                state.mutation_path.get_or_insert(requested_path);
            }
            state.edit_attempts = state.edit_attempts.saturating_add(1);
            state.edit_in_flight = true;
            None
        }
        _ => None,
    }
}

#[cfg(test)]
pub(super) fn workflow_tool_preflight(
    state: &Arc<StdMutex<WorkflowToolLoopState>>,
    tool_name: &str,
) -> Option<rmcp::model::CallToolResult> {
    workflow_tool_preflight_with_args(state, tool_name, &serde_json::Value::Null)
}

/// Stop an external agent from satisfying the edit loop with a tiny valid smoke test after it has
/// already authored a substantially richer candidate. This runs after the ordinary edit preflight
/// (so the attempt is bounded) but before the reconcile handler can append commands.
pub(super) fn workflow_candidate_preflight(
    state: &Arc<StdMutex<WorkflowToolLoopState>>,
    tool_name: &str,
    args: &serde_json::Value,
) -> Option<rmcp::model::CallToolResult> {
    if !is_workflow_commit_tool(tool_name) {
        return None;
    }
    let submitted = submitted_flowscript(args)?.trim();
    if submitted.is_empty() {
        return None;
    }

    let mut state = match state.lock() {
        Ok(state) => state,
        Err(_) => return Some(workflow_loop_state_unavailable_result()),
    };
    let Some(regression) = state.repair_tracker.queued_candidate_regression(submitted) else {
        let modular_fallback = state
            .repair_tracker
            .queued_candidate_modular_fallback(submitted);
        state.pending_modular_fallback = modular_fallback;
        state.in_flight_flowscript = Some(submitted.to_string());
        return None;
    };
    let retained = state.repair_tracker.best_failed_source()?.to_string();
    let retained_profile = profile_flowscript_candidate(&retained);
    let submitted_profile = profile_flowscript_candidate(submitted);

    let message = format!(
        "The submitted FlowScript is a severe completeness regression: it has {} executable call(s) and {} distinct call type(s), while the retained repair candidate has {} executable call(s) and {} distinct call type(s). A tiny valid smoke test cannot replace the requested multi-step workflow. Revise the retained candidate and preserve its functions, event entries, variables, and behavior.",
        submitted_profile.call_sites,
        submitted_profile.call_names.len(),
        retained_profile.call_sites,
        retained_profile.call_names.len(),
    );

    // workflow_tool_preflight marked the attempt in flight. Complete it here because the actual
    // reconcile handler will not run, and retain the rich source for this process/continuations.
    state.edit_in_flight = false;
    state.in_flight_flowscript = None;
    state.last_status = Some("validation_errors".to_string());
    state.last_errors = vec![message.clone()];
    state.candidate_regression_warning = Some(message.clone());
    state.pending_modular_fallback = None;
    state.declarations_since_edit = 0;

    Some(workflow_loop_result(
        serde_json::json!({
            "status": "validation_errors",
            "code": "candidate_regression",
            "retryable": true,
            "next_action": "revise_retained_candidate",
            "errors": [message],
            "regression": {
                "previous_call_sites": regression.previous_call_sites,
                "candidate_call_sites": regression.candidate_call_sites,
                "previous_statements": regression.previous_statements,
                "candidate_statements": regression.candidate_statements,
                "previous_scope_symbols": regression.previous_scope_symbols,
                "retained_scope_symbols": regression.retained_scope_symbols,
            },
            "retained_candidate_profile": {
                "call_sites": retained_profile.call_sites,
                "meaningful_statements": retained_profile.meaningful_statements,
                "helper_functions": retained_profile.helper_functions.len(),
                "event_entries": retained_profile.event_entries,
                "top_level_variables": retained_profile.top_level_variables.len(),
            },
            "submitted_candidate_profile": {
                "call_sites": submitted_profile.call_sites,
                "meaningful_statements": submitted_profile.meaningful_statements,
                "helper_functions": submitted_profile.helper_functions.len(),
                "event_entries": submitted_profile.event_entries,
                "top_level_variables": submitted_profile.top_level_variables.len(),
            },
            "retained_flowscript": truncate_for_preview(&retained, 30_000),
            "message": "Nothing was queued. Continue from retained_flowscript and fix its diagnostics. Preserve the requested scope, or refactor real work into non-empty named helpers invoked by a separate Event; do not replace it with a smoke test or empty Event shell."
        }),
        true,
    ))
}
