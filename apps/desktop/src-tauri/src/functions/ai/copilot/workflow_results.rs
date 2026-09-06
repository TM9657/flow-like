//! Modular fallback results and repeated source suppression.

use super::workflow_state::{WorkflowToolLoopState, submitted_flowscript};
use flow_like::flow::copilot::render_flowscript_modular_partial_result;
use std::sync::{Arc, Mutex as StdMutex};

pub(super) fn annotate_modular_fallback_result(
    state: &Arc<StdMutex<WorkflowToolLoopState>>,
    tool_name: &str,
    result: &mut copilot_sdk::ToolResultObject,
) {
    if tool_name != "edit_flowscript" || result.result_type == "error" || result.error.is_some() {
        return;
    }
    let Ok(mut payload) = serde_json::from_str::<serde_json::Value>(&result.text_result_for_llm)
    else {
        return;
    };
    if payload.get("status").and_then(serde_json::Value::as_str) != Some("queued") {
        return;
    }
    let modular_fallback = state.lock().ok().and_then(|state| {
        state.pending_modular_fallback.clone().map(|regression| {
            (
                regression,
                state
                    .repair_tracker
                    .best_failed_source()
                    .map(str::to_string),
            )
        })
    });
    if let Some((regression, retained_full_source)) = modular_fallback
        && let Some(object) = payload.as_object_mut()
    {
        let notice =
            render_flowscript_modular_partial_result(&result.text_result_for_llm, &regression);
        object.insert(
            "completion".to_string(),
            serde_json::Value::String("partial_working_slice".to_string()),
        );
        object.insert(
            "partial_working_slice_notice".to_string(),
            serde_json::Value::String(notice),
        );
        if let Some(retained_full_source) = retained_full_source {
            object.insert(
                "retained_full_source".to_string(),
                serde_json::Value::String(retained_full_source),
            );
        }
        result.text_result_for_llm =
            serde_json::to_string_pretty(&payload).unwrap_or_else(|_| payload.to_string());
    }
}

pub(super) fn flowscript_source_fingerprint(source: &str) -> String {
    use std::hash::{Hash, Hasher};
    let mut hasher = std::collections::hash_map::DefaultHasher::new();
    source.hash(&mut hasher);
    format!("{:016x}", hasher.finish())
}

/// Drop the multi-kilobyte source echo from a model-facing FlowScript tool result when the host
/// provably did not change the source the model itself just submitted or last received:
/// - `write_flowscript`: the response source is byte-identical to the submitted document.
/// - `check_flowscript` / `commit_flowscript`: the response revision equals `expected_revision`;
///   neither operation mutates the retained source.
///
/// `patch_flowscript` keeps its echo because the merged result is host-computed. This runs after
/// `workflow_tool_record`, so host retention/continuation state keeps the complete source.
pub(super) fn suppress_unchanged_flowscript_source_echo(
    tool_name: &str,
    args: &serde_json::Value,
    result: &mut copilot_sdk::ToolResultObject,
) {
    if !matches!(
        tool_name,
        "write_flowscript" | "check_flowscript" | "commit_flowscript"
    ) {
        return;
    }
    let Ok(mut payload) = serde_json::from_str::<serde_json::Value>(&result.text_result_for_llm)
    else {
        return;
    };
    let Some(object) = payload.as_object_mut() else {
        return;
    };
    let Some(source) = object.get("source").and_then(serde_json::Value::as_str) else {
        return;
    };
    let revision = object.get("revision").and_then(serde_json::Value::as_u64);
    let unchanged = match tool_name {
        "write_flowscript" => submitted_flowscript(args) == Some(source),
        _ => {
            revision.is_some()
                && revision
                    == args
                        .get("expected_revision")
                        .and_then(serde_json::Value::as_u64)
        }
    };
    if !unchanged {
        return;
    }
    let summary = format!(
        "Source retained at revision {} ({} lines, fingerprint {}) — unchanged from your submitted document, so it is not re-echoed.",
        revision
            .map(|revision| revision.to_string())
            .unwrap_or_else(|| "<unknown>".to_string()),
        source.lines().count(),
        flowscript_source_fingerprint(source),
    );
    object.remove("source");
    object.insert(
        "source_echo".to_string(),
        serde_json::Value::String(summary),
    );
    result.text_result_for_llm =
        serde_json::to_string_pretty(&payload).unwrap_or_else(|_| payload.to_string());
}
