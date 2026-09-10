//! Workflow tool outcomes, progress accounting, and abort handling.

use super::stream_events::truncate_for_preview;
use super::workflow_declarations::{
    complete_declaration_coverage_is_coherent, declaration_batch_coverage,
    declaration_lookup_queries, declaration_queries_are_related, declaration_result_is_usable,
    retain_declaration_result,
};
use super::workflow_diagnostics::{
    typed_ir_missing_modules, typed_ir_repair_fingerprint, workflow_result_clears_repair,
    workflow_result_diagnostics, workflow_result_fallback_message,
    workflow_result_repair_declarations, workflow_result_requires_repair,
    workflow_result_review_notes, workflow_result_structured_diagnostics,
};
use super::workflow_sdk::{
    is_flowscript_draft_operation_tool, is_order_sensitive_workflow_tool,
    is_typed_ir_operation_tool, is_workflow_commit_tool, typed_ir_operation_target,
    typed_ir_result_proves_retained_draft,
};
use super::workflow_state::{WorkflowToolLoopState, submitted_flowscript};
use std::sync::{Arc, Mutex as StdMutex};

/// Route every backend's completed workflow tool observation through the provider-neutral core
/// lifecycle. Ancillary context tools are reserved by `workflow_predraft_context_preflight`
/// before dispatch, so they are intentionally not counted a second time here.
fn record_shared_external_workflow_observation(
    state: &Arc<StdMutex<WorkflowToolLoopState>>,
    lease: Option<&flow_like::flow::copilot::WorkflowToolLease>,
    tool_name: &str,
    args: &serde_json::Value,
    result_text: &str,
    succeeded: bool,
) {
    let Ok(mut state) = state.lock() else {
        return;
    };
    let elapsed_ms = state.shared_session_elapsed_ms();
    let Some(session) = state.shared_session.as_mut() else {
        return;
    };
    let _ = session.complete_tool_call(lease, tool_name, args, result_text, succeeded, elapsed_ms);
}

#[cfg(test)]
pub(super) fn workflow_tool_record(
    state: &Arc<StdMutex<WorkflowToolLoopState>>,
    tool_name: &str,
    args: &serde_json::Value,
    result_text: &str,
) {
    workflow_tool_record_with_outcome(state, None, tool_name, args, result_text, true);
}

pub(super) fn workflow_tool_record_with_outcome(
    state: &Arc<StdMutex<WorkflowToolLoopState>>,
    lease: Option<&flow_like::flow::copilot::WorkflowToolLease>,
    tool_name: &str,
    args: &serde_json::Value,
    result_text: &str,
    succeeded: bool,
) {
    record_shared_external_workflow_observation(
        state,
        lease,
        tool_name,
        args,
        result_text,
        succeeded,
    );
    if tool_name == "get_declarations" {
        if let Ok(mut state) = state.lock() {
            let was_initial_lookup = state.needs_initial_declaration_coverage();
            let usable = declaration_result_is_usable(result_text);
            let mut coverage = declaration_batch_coverage(result_text);
            if let Some(parsed_coverage) = coverage.as_mut()
                && !complete_declaration_coverage_is_coherent(parsed_coverage, args, result_text)
            {
                let requested = declaration_lookup_queries(args)
                    .into_iter()
                    .filter(|query| !query.trim().is_empty())
                    .map(str::to_string)
                    .collect::<Vec<_>>();
                parsed_coverage.complete = false;
                parsed_coverage.matched_count = 0;
                parsed_coverage.matched_queries.clear();
                parsed_coverage.output_omitted_count = requested.len();
                parsed_coverage.output_omitted_queries = requested;
            }
            state.declaration_lookup_in_flight = false;
            let repair_lookup_keys = std::mem::take(&mut state.in_flight_repair_lookup_keys);
            // A parsed coverage envelope is an authoritative catalog outcome even when nothing
            // matched (or an exact signature could not fit in the bounded response). Consume the
            // diagnostic target so the model cannot reopen the same unavailable lookup forever.
            // Only a missing/unparseable result behaves like a transport failure and releases the
            // lease for an exact retry.
            let authoritative_outcome = coverage.is_some();
            let legacy_multi_query_incomplete = !authoritative_outcome
                && usable
                && !repair_lookup_keys.is_empty()
                && declaration_lookup_queries(args).len() > 1;
            let retryable_omission = legacy_multi_query_incomplete
                || coverage.as_ref().is_some_and(|coverage| {
                    coverage.output_omitted_count > 0
                        || coverage.omitted_count > 0
                        || coverage.truncated_query_count > 0
                });
            let repair_lookup_failed =
                !usable && !authoritative_outcome && !repair_lookup_keys.is_empty();
            if (usable || authoritative_outcome) && !retryable_omission {
                state
                    .completed_repair_lookup_keys
                    .extend(repair_lookup_keys.iter().cloned());
            } else if repair_lookup_failed {
                state.declaration_calls = state.declaration_calls.saturating_sub(1);
                state.declarations_since_edit = state.declarations_since_edit.saturating_sub(1);
                for key in &repair_lookup_keys {
                    let remove = if let Some(attempts) = state.repair_lookup_attempts.get_mut(key) {
                        *attempts = attempts.saturating_sub(1);
                        *attempts == 0
                    } else {
                        false
                    };
                    if remove {
                        state.repair_lookup_attempts.remove(key);
                    }
                }
            } else if retryable_omission && !repair_lookup_keys.is_empty() {
                // A large batch can omit a declaration that fits when queried alone. Release the
                // per-edit discovery lease for one focused retry while the per-key counter and
                // global call budget prevent an omission loop.
                state.declarations_since_edit = 0;
            }
            if usable {
                state.initial_declaration_lookup_usable = true;
                state.last_declarations = Some(retain_declaration_result(
                    state.last_declarations.as_deref(),
                    result_text,
                ));
            }
            if was_initial_lookup {
                match coverage {
                    Some(coverage) => {
                        let previous_unresolved =
                            std::mem::take(&mut state.unresolved_declaration_queries);
                        let mut matched_queries = coverage.matched_queries.clone();
                        if coverage.complete && coverage.query_names_omitted_for_size {
                            // The compact metadata header intentionally omits identities. The
                            // exact dispatched arguments remain host-owned and are safe to use as
                            // the matched set for this one complete batch.
                            matched_queries.extend(
                                declaration_lookup_queries(args)
                                    .into_iter()
                                    .map(str::to_string),
                            );
                        }
                        let mut processed_queries = matched_queries
                            .iter()
                            .map(String::as_str)
                            .chain(coverage.unmatched_queries.iter().map(String::as_str))
                            .collect::<Vec<_>>();
                        let mut unresolved = Vec::new();
                        for previous in previous_unresolved {
                            if let Some(index) = processed_queries
                                .iter()
                                .position(|query| declaration_queries_are_related(&previous, query))
                            {
                                // A successful focused rephrasing resolves the previous miss. An
                                // unsuccessful one replaces it with the new wording below, rather
                                // than accumulating aliases that can never all match exactly.
                                processed_queries.remove(index);
                            } else {
                                unresolved.push(previous);
                            }
                        }
                        let named_unmatched = coverage.unmatched_queries.len();
                        let named_output_omitted = coverage.output_omitted_queries.len();
                        let named_omitted = coverage.omitted_queries.len();
                        unresolved.extend(coverage.unmatched_queries);
                        unresolved.extend(coverage.output_omitted_queries);
                        unresolved.extend(coverage.omitted_queries);
                        let unnamed_unmatched =
                            coverage.unmatched_count.saturating_sub(named_unmatched);
                        let unnamed_omitted = coverage.omitted_count.saturating_sub(named_omitted);
                        let unnamed_output_omitted = coverage
                            .output_omitted_count
                            .saturating_sub(named_output_omitted);
                        if unnamed_unmatched > 0 {
                            unresolved.push(format!(
                                "{} additional unmatched declaration query or queries (names omitted for size)",
                                unnamed_unmatched
                            ));
                        }
                        if unnamed_omitted > 0 {
                            unresolved.push(format!(
                                "{} additional omitted declaration query or queries (names omitted for size)",
                                unnamed_omitted
                            ));
                        }
                        if unnamed_output_omitted > 0 {
                            unresolved.push(format!(
                                "{} additional declaration query or queries matched but their exact signatures were omitted from the bounded response",
                                unnamed_output_omitted
                            ));
                        }
                        if coverage.truncated_query_count > 0 {
                            unresolved.push(format!(
                                "{} overlong declaration query or queries must be shortened",
                                coverage.truncated_query_count
                            ));
                        }
                        if !coverage.complete
                            && unresolved.is_empty()
                            && coverage.unmatched_count == 0
                            && coverage.output_omitted_count == 0
                            && coverage.omitted_count == 0
                            && coverage.truncated_query_count == 0
                        {
                            unresolved.push(
                                "Declaration batch reported incomplete coverage without query identities."
                                    .to_string(),
                            );
                        }
                        unresolved.sort_unstable();
                        unresolved.dedup();
                        state.initial_declaration_lookup_complete =
                            coverage.complete && unresolved.is_empty();
                        state.unresolved_declaration_queries = unresolved;
                    }
                    None if usable && declaration_lookup_queries(args).len() == 1 => {
                        // Backward compatibility for direct SDK/tests and older tool workers that
                        // predate coverage metadata: one requested capability with one actual
                        // declaration remains usable. Multi-query legacy results cannot prove
                        // complete coverage and stay gated.
                        state.initial_declaration_lookup_complete = true;
                        state.unresolved_declaration_queries.clear();
                    }
                    None => {
                        state.initial_declaration_lookup_complete = false;
                        state.unresolved_declaration_queries = declaration_lookup_queries(args)
                            .into_iter()
                            .filter(|query| !query.trim().is_empty())
                            .map(str::to_string)
                            .collect();
                        if state.unresolved_declaration_queries.is_empty() {
                            state.unresolved_declaration_queries = vec![
                                "No requested capability matched a live catalog declaration."
                                    .to_string(),
                            ];
                        }
                    }
                }
                if !usable && !state.initial_declaration_lookup_complete {
                    // No usable signature was returned, so permit a bounded focused retry. Once
                    // any live signature is retained, the next checkpoint must be source; omitted
                    // and unmatched capabilities are handled from compiler diagnostics later.
                    state.declarations_since_edit = 0;
                }
            }
        }
        return;
    }
    if tool_name == "test_flowscript" {
        // The shared session above owns revision-bound runtime evidence. This legacy field
        // only releases dispatch serialization; compiler status and diagnostics stay intact.
        if let Ok(mut state) = state.lock() {
            state.edit_in_flight = false;
        }
        return;
    }
    if is_flowscript_draft_operation_tool(tool_name) {
        let parsed = serde_json::from_str::<serde_json::Value>(result_text).ok();
        let Ok(mut state) = state.lock() else {
            return;
        };
        state.edit_in_flight = false;
        let interrupted_source = state.in_flight_flowscript.take();
        let previous_status = state.last_status.clone();
        let response_status = parsed
            .as_ref()
            .and_then(|value| value.get("status"))
            .and_then(serde_json::Value::as_str);
        let response_code = parsed
            .as_ref()
            .and_then(|value| value.get("code"))
            .and_then(serde_json::Value::as_str);

        if matches!(
            response_code,
            Some("FLOWSCRIPT_DRAFT_MISSING" | "FLOWSCRIPT_BASE_REVISION_CONFLICT")
        ) {
            // These responses prove that the retained coordinates can no longer be continued.
            // Release only the local authorization lease so the same request may write a fresh
            // draft id against the current board. Preserve an explicitly returned old source as
            // reference, but never synthesize retained coordinates from the rejected arguments.
            if let Some(source) = parsed
                .as_ref()
                .and_then(|value| value.get("source"))
                .and_then(serde_json::Value::as_str)
            {
                state.last_flowscript = Some(source.to_string());
            }
            state.flowscript_draft_retained = false;
            state.flowscript_draft_id = None;
            state.flowscript_revision = None;
            state.flowscript_commit_attempts = 0;
            state.last_status = response_status.map(str::to_string);
            state.last_errors = parsed
                .as_ref()
                .and_then(workflow_result_fallback_message)
                .into_iter()
                .collect();
            state.last_structured_diagnostics.clear();
            state.pending_modular_fallback = None;
            state.declarations_since_edit = 0;
            return;
        }

        if response_status == Some("request_identity_mismatch") {
            // The core deliberately returns a minimal envelope for a draft owned by another
            // immutable request. Do not reconstruct its coordinates or treat the rejected source
            // arguments as retained state. A subsequent write with a distinct draft id remains
            // possible for the current request.
            state.last_status = Some("request_identity_mismatch".to_string());
            state.last_errors = parsed
                .as_ref()
                .and_then(workflow_result_fallback_message)
                .into_iter()
                .collect();
            state.last_structured_diagnostics.clear();
            state.pending_modular_fallback = None;
            return;
        }

        let preserve_checked_valid_after_transient_commit = tool_name == "commit_flowscript"
            && previous_status.as_deref() == Some("valid")
            && response_code == Some("FLOWSCRIPT_DRAFT_STORE_UNAVAILABLE");
        if preserve_checked_valid_after_transient_commit {
            // A store-lock failure happened before the exact checked command claim could be
            // inspected or changed. Preserve the host-checked revision and its validation state;
            // the separate commit-attempt cap bounds idempotent retries.
            state.last_status = previous_status;
            return;
        }

        let response_draft_id = parsed
            .as_ref()
            .and_then(|value| value.get("draft_id"))
            .and_then(serde_json::Value::as_str)
            .map(str::to_string);
        let response_revision = parsed
            .as_ref()
            .and_then(|value| value.get("revision"))
            .and_then(serde_json::Value::as_u64);
        let source = parsed
            .as_ref()
            .and_then(|value| value.get("source").or_else(|| value.get("flowscript")))
            .and_then(serde_json::Value::as_str)
            .map(str::to_string)
            .or_else(|| submitted_flowscript(args).map(str::to_string))
            .or(interrupted_source)
            .or_else(|| {
                // Envelope results replace `source` with `source_bytes` but never change the
                // retained bytes, so the last recorded source still describes this revision.
                parsed
                    .as_ref()
                    .is_some_and(|value| value.get("source_bytes").is_some())
                    .then(|| state.last_flowscript.clone())
                    .flatten()
            });
        let current_draft_id = state.flowscript_draft_id.clone();
        state.flowscript_draft_id = response_draft_id
            .clone()
            .or_else(|| {
                args.get("draft_id")
                    .and_then(serde_json::Value::as_str)
                    .map(str::to_string)
            })
            .or(current_draft_id);
        state.flowscript_revision = response_revision.or(state.flowscript_revision);
        if response_draft_id.is_some() && response_revision.is_some() && source.is_some() {
            state.flowscript_draft_retained = true;
        }

        state.last_status = response_status.map(str::to_string);
        if state.last_status.as_deref() == Some("valid") {
            state.flowscript_commit_attempts = 0;
        }
        if let Some(parsed) = parsed.as_ref() {
            let repair_declarations = workflow_result_repair_declarations(Some(parsed));
            if !repair_declarations.is_empty() {
                state.last_repair_declarations = repair_declarations;
            } else if workflow_result_clears_repair(parsed) {
                state.last_repair_declarations.clear();
            }
        }
        let mut diagnostics = workflow_result_diagnostics(parsed.as_ref());
        state.last_structured_diagnostics = workflow_result_structured_diagnostics(parsed.as_ref());
        if let Some(review_notes) = workflow_result_review_notes(parsed.as_ref()) {
            state.last_review_notes = review_notes;
        }
        let requires_repair = parsed.as_ref().is_none_or(|value| {
            !workflow_result_clears_repair(value)
                && workflow_result_requires_repair(value, &diagnostics)
        });
        if requires_repair && diagnostics.is_empty() {
            if let Some(message) = parsed.as_ref().and_then(workflow_result_fallback_message) {
                diagnostics.push(message);
            } else if !result_text.trim().is_empty() {
                diagnostics.push(truncate_for_preview(result_text.trim(), 2_000));
            } else {
                diagnostics.push(
                    "The FlowScript source operation failed without diagnostics.".to_string(),
                );
            }
        }

        if let Some(source) = source {
            if requires_repair {
                state.pending_modular_fallback = None;
                if state
                    .repair_tracker
                    .record_failed_with_diagnostics(&source, Some(diagnostics.len()))
                {
                    state.best_failed_errors = diagnostics.clone();
                    state.candidate_regression_warning = None;
                }
            }
            state.last_flowscript = Some(source);
        }
        state.last_errors = diagnostics;
        let status = state.last_status.clone();
        let progress_diagnostics = state.last_errors.clone();
        state.record_flowscript_repair_progress(
            status.as_deref(),
            &progress_diagnostics,
            requires_repair,
        );
        if status.as_deref() == Some("valid") {
            state.valid_checks = state.valid_checks.saturating_add(1);
            state.record_staged_segment_validated();
        }
        if matches!(status.as_deref(), Some("queued" | "already_queued")) {
            if status.as_deref() == Some("queued") {
                state.record_scope_plan_commit();
            }
            state.queued = true;
            state.has_previous_validation_result = false;
            state.previous_validation_diagnostics.clear();
        }
        return;
    }
    if is_typed_ir_operation_tool(tool_name) {
        let parsed = serde_json::from_str::<serde_json::Value>(result_text).ok();
        let Ok(mut state) = state.lock() else {
            return;
        };
        if is_order_sensitive_workflow_tool(tool_name) {
            state.edit_in_flight = false;
        }
        let status = parsed
            .as_ref()
            .and_then(|value| value.get("status"))
            .and_then(serde_json::Value::as_str);
        if status == Some("request_identity_mismatch") {
            // A typed draft owned by another immutable request is not recovery state for this
            // loop. Ignore both rejected arguments and any coordinates a stale/custom provider
            // might return; preserve only already-authorized local coordinates.
            state.last_status = Some("request_identity_mismatch".to_string());
            state.last_errors = parsed
                .as_ref()
                .and_then(workflow_result_fallback_message)
                .into_iter()
                .collect();
            return;
        }
        let current_draft_id = state.typed_draft_id.clone();
        let response_draft_id = parsed
            .as_ref()
            .and_then(|value| value.get("draft_id"))
            .and_then(serde_json::Value::as_str)
            .map(str::to_string);
        if response_draft_id.is_some()
            && parsed
                .as_ref()
                .is_some_and(typed_ir_result_proves_retained_draft)
        {
            state.typed_draft_retained = true;
        }
        state.typed_draft_id = response_draft_id
            .or_else(|| {
                args.get("draft_id")
                    .and_then(serde_json::Value::as_str)
                    .map(str::to_string)
            })
            .or(current_draft_id);
        state.typed_revision = parsed
            .as_ref()
            .and_then(|value| value.get("revision"))
            .and_then(serde_json::Value::as_u64)
            .or(state.typed_revision);
        let preserve_retained_draft_context =
            tool_name == "plan_flow_ir" && state.typed_draft_retained;
        if !preserve_retained_draft_context {
            state.last_status = status.map(str::to_string);
        }
        let mut diagnostics = workflow_result_diagnostics(parsed.as_ref());
        let requires_repair = parsed.as_ref().is_none_or(|value| {
            !workflow_result_clears_repair(value)
                && workflow_result_requires_repair(value, &diagnostics)
        });
        if requires_repair && diagnostics.is_empty() {
            if let Some(message) = parsed.as_ref().and_then(workflow_result_fallback_message) {
                diagnostics.push(message);
            } else if !result_text.trim().is_empty() {
                diagnostics.push(truncate_for_preview(result_text.trim(), 2_000));
            } else {
                diagnostics.push("The typed-IR operation failed without diagnostics.".to_string());
            }
        }
        let missing_modules = typed_ir_missing_modules(parsed.as_ref());
        if let Some(review_notes) = workflow_result_review_notes(parsed.as_ref()) {
            state.last_review_notes = review_notes;
        }
        if !preserve_retained_draft_context {
            state.typed_missing_modules = missing_modules.clone();
            state.last_errors = diagnostics.clone();
        }
        if let Some(flowscript) = parsed
            .as_ref()
            .and_then(|value| value.get("flowscript"))
            .and_then(serde_json::Value::as_str)
        {
            state.last_flowscript = Some(flowscript.to_string());
        }

        if requires_repair {
            let target = typed_ir_operation_target(tool_name, args);
            let fingerprint = typed_ir_repair_fingerprint(status, &diagnostics, &missing_modules);
            let is_repeated = !state
                .typed_seen_repair_signatures
                .entry(target)
                .or_default()
                .insert(fingerprint);
            if is_repeated {
                state.typed_stalled_attempts = state.typed_stalled_attempts.saturating_add(1);
            } else {
                state.typed_stalled_attempts = 0;
            }
        } else {
            state.typed_stalled_attempts = 0;
        }

        if matches!(status, Some("queued" | "already_queued")) {
            state.queued = true;
            state.typed_stalled_attempts = 0;
        }
        return;
    }
    if !is_workflow_commit_tool(tool_name) {
        return;
    }
    let Ok(mut state) = state.lock() else {
        return;
    };
    state.edit_in_flight = false;
    state.in_flight_flowscript = None;

    let parsed = serde_json::from_str::<serde_json::Value>(result_text).ok();
    state.last_status = parsed
        .as_ref()
        .and_then(|value| value.get("status"))
        .and_then(serde_json::Value::as_str)
        .map(str::to_string);
    let parsed_errors = workflow_result_diagnostics(parsed.as_ref());
    state.last_structured_diagnostics = workflow_result_structured_diagnostics(parsed.as_ref());
    if let Some(review_notes) = workflow_result_review_notes(parsed.as_ref()) {
        state.last_review_notes = review_notes;
    }

    if tool_name == "commit_flow_ir_draft" {
        let current_draft_id = state.typed_draft_id.clone();
        state.typed_draft_id = parsed
            .as_ref()
            .and_then(|value| value.get("draft_id"))
            .and_then(serde_json::Value::as_str)
            .map(str::to_string)
            .or(current_draft_id);
        state.typed_revision = parsed
            .as_ref()
            .and_then(|value| value.get("revision"))
            .and_then(serde_json::Value::as_u64)
            .or(state.typed_revision);
    }

    let submitted_flowscript = submitted_flowscript(args).map(str::to_string).or_else(|| {
        parsed
            .as_ref()
            .and_then(|value| value.get("flowscript"))
            .and_then(serde_json::Value::as_str)
            .map(str::to_string)
    });
    if let Some(submitted_flowscript) = submitted_flowscript {
        let submission_failed = !matches!(
            state.last_status.as_deref(),
            Some("queued" | "already_queued")
        );
        if submission_failed {
            state.pending_modular_fallback = None;
            if state
                .repair_tracker
                .record_failed_with_diagnostics(&submitted_flowscript, Some(parsed_errors.len()))
            {
                state.best_failed_errors = parsed_errors.clone();
                state.candidate_regression_warning = None;
            }
        }
        state.last_flowscript = Some(submitted_flowscript);
    }
    state.last_errors = parsed_errors;
    let status = state.last_status.clone();
    let progress_diagnostics = state.last_errors.clone();
    let requires_repair = parsed.as_ref().is_none_or(|value| {
        !workflow_result_clears_repair(value)
            && workflow_result_requires_repair(value, &progress_diagnostics)
    });
    state.record_flowscript_repair_progress(
        status.as_deref(),
        &progress_diagnostics,
        requires_repair,
    );
    if matches!(status.as_deref(), Some("queued" | "already_queued")) {
        if status.as_deref() == Some("queued") {
            state.record_scope_plan_commit();
        }
        state.queued = true;
        state.has_previous_validation_result = false;
        state.previous_validation_diagnostics.clear();
    }
}

pub(super) fn workflow_tool_abort(
    state: &Arc<StdMutex<WorkflowToolLoopState>>,
    tool_name: &str,
    error: &str,
) {
    if tool_name == "get_declarations" {
        if let Ok(mut state) = state.lock()
            && state.declaration_lookup_in_flight
        {
            let initial_lookup = state.needs_initial_declaration_coverage();
            state.declaration_lookup_in_flight = false;
            let repair_lookup_keys = std::mem::take(&mut state.in_flight_repair_lookup_keys);
            for key in repair_lookup_keys {
                let remove = if let Some(attempts) = state.repair_lookup_attempts.get_mut(&key) {
                    *attempts = attempts.saturating_sub(1);
                    *attempts == 0
                } else {
                    false
                };
                if remove {
                    state.repair_lookup_attempts.remove(&key);
                }
            }
            state.declaration_calls = state.declaration_calls.saturating_sub(1);
            state.declarations_since_edit = state.declarations_since_edit.saturating_sub(1);
            if initial_lookup {
                // Preflight reserves initial coverage before dispatch. A worker abort produced no
                // catalog evidence, so release only that attempt while preserving earlier partial
                // declarations and unresolved identities.
                state.initial_declaration_attempts =
                    state.initial_declaration_attempts.saturating_sub(1);
            }
        }
        return;
    }
    if tool_name == "test_flowscript" {
        if let Ok(mut state) = state.lock() {
            state.edit_in_flight = false;
        }
        return;
    }
    if tool_name == "commit_flowscript"
        && let Ok(mut state) = state.lock()
        && state.edit_in_flight
        && state.last_status.as_deref() == Some("valid")
    {
        // A transport/worker abort does not invalidate the host-checked source revision.
        // Preserve its status so the bounded idempotent commit retry path remains available.
        state.edit_in_flight = false;
        state.in_flight_flowscript = None;
        return;
    }
    if !is_order_sensitive_workflow_tool(tool_name) {
        return;
    }
    if let Ok(mut state) = state.lock() {
        state.edit_in_flight = false;
        if let Some(interrupted) = state.in_flight_flowscript.take() {
            if state.repair_tracker.record_failed(&interrupted) {
                state.best_failed_errors = vec![error.to_string()];
                state.candidate_regression_warning = None;
            }
            state.last_flowscript = Some(interrupted);
        }
        state.last_status = Some("error".to_string());
        state.last_errors = vec![error.to_string()];
        state.pending_modular_fallback = None;
        state.declarations_since_edit = 0;
    }
}

pub(super) fn workflow_tool_abort_with_args(
    state: &Arc<StdMutex<WorkflowToolLoopState>>,
    lease: Option<&flow_like::flow::copilot::WorkflowToolLease>,
    tool_name: &str,
    args: &serde_json::Value,
    error: &str,
) {
    if matches!(tool_name, "database_tool" | "ui_inspect" | "storage_tool") {
        if let Ok(mut state) = state.lock() {
            let elapsed_ms = state.shared_session_elapsed_ms();
            if let Some(session) = state.shared_session.as_mut() {
                let _ = session.abort_tool_call(lease, tool_name, args, elapsed_ms);
            } else if !state.queued
                && !state.flowscript_draft_retained
                && !state.typed_draft_retained
            {
                state.predraft_context_reads = state.predraft_context_reads.saturating_sub(1);
            }
        }
        return;
    }
    workflow_tool_abort(state, tool_name, error);
}
