//! Workflow result diagnostics and repair classification.

use super::workflow_state::{
    MAX_INJECTED_REPAIR_DECLARATION_BYTES, MAX_INJECTED_REPAIR_DECLARATIONS,
    MAX_RETAINED_STRUCTURED_DIAGNOSTIC_BYTES, MAX_RETAINED_STRUCTURED_DIAGNOSTICS,
};
use std::collections::HashSet;

fn workflow_diagnostic_has_parent(entry: &serde_json::Value) -> bool {
    match entry.get("caused_by") {
        None | Some(serde_json::Value::Null) => false,
        Some(serde_json::Value::String(cause)) => !cause.trim().is_empty(),
        Some(serde_json::Value::Array(causes)) => !causes.is_empty(),
        Some(serde_json::Value::Object(cause)) => !cause.is_empty(),
        Some(_) => true,
    }
}

pub(super) fn workflow_result_diagnostics(parsed: Option<&serde_json::Value>) -> Vec<String> {
    let mut diagnostics = parsed
        .map(|value| {
            [
                "errors",
                "diagnostics",
                "structured_diagnostics",
                "module_budget_violations",
            ]
            .into_iter()
            .filter_map(|key| value.get(key).and_then(serde_json::Value::as_array))
            .flat_map(|entries| entries.iter())
            .filter_map(|entry| {
                if workflow_diagnostic_has_parent(entry) {
                    return None;
                }
                entry.as_str().map(str::to_string).or_else(|| {
                    let code = entry.get("code").and_then(serde_json::Value::as_str);
                    let message = entry.get("message").and_then(serde_json::Value::as_str);
                    match (code, message) {
                        (Some(code), Some(message)) => Some(format!("[{code}] {message}")),
                        (None, Some(message)) => Some(message.to_string()),
                        _ => None,
                    }
                })
            })
            .collect::<Vec<_>>()
        })
        .unwrap_or_default();
    if let Some(value) = parsed {
        if let Some(missing_modules) = value
            .get("missing_modules")
            .and_then(serde_json::Value::as_array)
        {
            diagnostics.extend(missing_modules.iter().filter_map(|module| {
                module
                    .as_str()
                    .map(|module| format!("Missing required module: {module}"))
            }));
        }
        if let Some(budget_violations) = value
            .get("capability_plan")
            .and_then(|plan| plan.get("module_budget_violations"))
            .and_then(serde_json::Value::as_array)
        {
            diagnostics.extend(budget_violations.iter().filter_map(|entry| {
                entry.as_str().map(str::to_string).or_else(|| {
                    let code = entry.get("code").and_then(serde_json::Value::as_str);
                    let message = entry.get("message").and_then(serde_json::Value::as_str);
                    match (code, message) {
                        (Some(code), Some(message)) => Some(format!("[{code}] {message}")),
                        (None, Some(message)) => Some(message.to_string()),
                        _ => None,
                    }
                })
            }));
        }
    }
    let mut seen = HashSet::new();
    diagnostics.retain(|diagnostic| seen.insert(diagnostic.clone()));
    diagnostics
}

/// Compiler-pipeline order: an early-phase diagnostic is the likeliest root cause of later
/// cascades, so retention prefers it when the budget cannot hold everything.
fn structured_diagnostic_phase_rank(entry: &serde_json::Map<String, serde_json::Value>) -> u8 {
    match entry.get("phase").and_then(serde_json::Value::as_str) {
        Some("parse") => 0,
        Some("catalog_resolution") => 1,
        Some("type_check") => 2,
        Some("lowering") => 3,
        Some("execution_wiring") => 4,
        Some("validation" | "validate") => 5,
        _ => 6,
    }
}

pub(super) fn workflow_result_structured_diagnostics(
    parsed: Option<&serde_json::Value>,
) -> Vec<serde_json::Value> {
    const RETAINED_FIELDS: &[&str] = &[
        "id",
        "code",
        "phase",
        "severity",
        "message",
        "source_span",
        "ast_path",
        "scope",
        "expected",
        "actual",
        "declaration",
        "pin",
        "fix",
        "occurrences",
        "related_messages",
    ];

    let Some(parsed) = parsed else {
        return Vec::new();
    };
    let mut candidates = Vec::new();
    let mut seen = HashSet::new();
    for entry in ["structured_diagnostics", "diagnostics"]
        .into_iter()
        .filter_map(|key| parsed.get(key).and_then(serde_json::Value::as_array))
        .flat_map(|entries| entries.iter())
    {
        let Some(source) = entry.as_object() else {
            continue;
        };
        if workflow_diagnostic_has_parent(entry) {
            continue;
        }
        let mut object = serde_json::Map::new();
        for field in RETAINED_FIELDS {
            if let Some(value) = source.get(*field) {
                object.insert((*field).to_string(), value.clone());
            }
        }
        if object.is_empty() {
            continue;
        }
        if seen.insert(serde_json::to_string(&object).unwrap_or_default()) {
            candidates.push(object);
        }
    }

    // Root causes first: earlier compiler phases outrank later ones, and the first occurrence of
    // each distinct code outranks its repeats, so truncation drops cascades instead of causes.
    let mut ordered = candidates
        .into_iter()
        .enumerate()
        .map(|(index, object)| (structured_diagnostic_phase_rank(&object), index, object))
        .collect::<Vec<_>>();
    ordered.sort_by_key(|(phase_rank, index, _)| (*phase_rank, *index));
    let mut seen_codes = HashSet::new();
    let mut ranked = ordered
        .into_iter()
        .enumerate()
        .map(|(rank_index, (phase_rank, _, object))| {
            let repeated_code = match object.get("code") {
                Some(code) => !seen_codes.insert(code.to_string()),
                None => false,
            };
            (phase_rank, repeated_code, rank_index, object)
        })
        .collect::<Vec<_>>();
    ranked.sort_by_key(|(phase_rank, repeated_code, rank_index, _)| {
        (*phase_rank, *repeated_code, *rank_index)
    });

    let total = ranked.len();
    let mut retained = Vec::new();
    let mut retained_bytes = 0usize;
    for (_, _, _, mut object) in ranked {
        if retained.len() >= MAX_RETAINED_STRUCTURED_DIAGNOSTICS {
            break;
        }
        let mut encoded = serde_json::to_string(&object).unwrap_or_default();
        if retained_bytes.saturating_add(encoded.len()) > MAX_RETAINED_STRUCTURED_DIAGNOSTIC_BYTES {
            // Exact repair declarations are retained separately. Prefer keeping the diagnostic's
            // location/type fields over dropping the entire item because a fix payload is large.
            object.remove("fix");
            object.remove("related_messages");
            encoded = serde_json::to_string(&object).unwrap_or_default();
        }
        if retained_bytes.saturating_add(encoded.len()) > MAX_RETAINED_STRUCTURED_DIAGNOSTIC_BYTES {
            // One oversized item must not silently discard every remaining smaller diagnostic.
            continue;
        }
        retained_bytes = retained_bytes.saturating_add(encoded.len());
        retained.push(serde_json::Value::Object(object));
    }
    if retained.len() < total {
        let omitted = total - retained.len();
        retained.push(serde_json::json!({
            "truncated": true,
            "omitted_count": omitted,
            "message": format!(
                "{omitted} additional structured diagnostic(s) exceeded the retention budget and were omitted; root-cause phases and first occurrences of each code were kept first."
            ),
        }));
    }
    retained
}

/// Preserve exact catalog signatures carried by structured FlowScript fixes. Diagnostic text is
/// intentionally flattened for progress/stall accounting, but a fresh external-agent process
/// needs the richer repair payload to avoid guessing the same declaration or pin again.
pub(super) fn workflow_result_repair_declarations(
    parsed: Option<&serde_json::Value>,
) -> Vec<String> {
    let Some(parsed) = parsed else {
        return Vec::new();
    };

    let mut declarations = Vec::new();
    let mut seen = HashSet::new();
    let mut retained_bytes = 0usize;
    for diagnostic in ["diagnostics", "structured_diagnostics"]
        .into_iter()
        .filter_map(|key| parsed.get(key).and_then(serde_json::Value::as_array))
        .flat_map(|entries| entries.iter())
    {
        let Some(fix) = diagnostic.get("fix") else {
            continue;
        };
        for signatures in ["catalog_declarations", "companion_declarations"]
            .into_iter()
            .filter_map(|key| fix.get(key).and_then(serde_json::Value::as_array))
        {
            for signature in signatures.iter().filter_map(serde_json::Value::as_str) {
                let signature = signature.trim();
                if signature.is_empty() || seen.contains(signature) {
                    continue;
                }
                let next_bytes = retained_bytes.saturating_add(signature.len());
                if declarations.len() >= MAX_INJECTED_REPAIR_DECLARATIONS
                    || next_bytes > MAX_INJECTED_REPAIR_DECLARATION_BYTES
                {
                    return declarations;
                }
                seen.insert(signature.to_string());
                declarations.push(signature.to_string());
                retained_bytes = next_bytes;
            }
        }
    }
    declarations
}

/// Count of reviewer-facing notes carried by a lifecycle tool result. `None` when the result did
/// not include the field, so an unrelated follow-up result does not erase the last known count.
pub(super) fn workflow_result_review_notes(parsed: Option<&serde_json::Value>) -> Option<usize> {
    parsed?
        .get("review_notes")
        .and_then(serde_json::Value::as_array)
        .map(Vec::len)
}

pub(super) fn typed_ir_missing_modules(parsed: Option<&serde_json::Value>) -> Vec<String> {
    let mut modules = parsed
        .and_then(|value| value.get("missing_modules"))
        .and_then(serde_json::Value::as_array)
        .into_iter()
        .flatten()
        .filter_map(serde_json::Value::as_str)
        .map(str::to_string)
        .collect::<Vec<_>>();
    modules.sort_unstable();
    modules.dedup();
    modules
}

pub(super) fn typed_ir_repair_fingerprint(
    status: Option<&str>,
    diagnostics: &[String],
    missing_modules: &[String],
) -> String {
    let mut normalized_diagnostics = diagnostics
        .iter()
        .map(|diagnostic| {
            diagnostic
                .split_whitespace()
                .collect::<Vec<_>>()
                .join(" ")
                .to_ascii_lowercase()
        })
        .collect::<Vec<_>>();
    normalized_diagnostics.sort_unstable();
    normalized_diagnostics.dedup();
    let mut normalized_modules = missing_modules
        .iter()
        .map(|module| module.trim().to_ascii_lowercase())
        .collect::<Vec<_>>();
    normalized_modules.sort_unstable();
    normalized_modules.dedup();
    format!(
        "{}\u{1f}{}\u{1f}{}",
        status.unwrap_or("<missing-status>"),
        normalized_diagnostics.join("\u{1e}"),
        normalized_modules.join("\u{1e}")
    )
}

pub(super) fn flowscript_repair_fingerprint(
    status: Option<&str>,
    diagnostics: &[String],
    structured_diagnostics: &[serde_json::Value],
) -> String {
    let mut normalized = diagnostics
        .iter()
        .map(|diagnostic| {
            diagnostic
                .split_whitespace()
                .collect::<Vec<_>>()
                .join(" ")
                .to_ascii_lowercase()
        })
        .collect::<Vec<_>>();
    normalized.sort_unstable();
    normalized.dedup();
    let mut structured_subjects = structured_diagnostics
        .iter()
        .map(|diagnostic| {
            serde_json::json!({
                "code": diagnostic.get("code"),
                "message": diagnostic.get("message"),
                "ast_path": diagnostic.get("ast_path"),
                "declaration": diagnostic.get("declaration"),
                "pin": diagnostic.get("pin"),
                "occurrences": diagnostic.get("occurrences"),
            })
            .to_string()
            .to_ascii_lowercase()
        })
        .collect::<Vec<_>>();
    structured_subjects.sort_unstable();
    structured_subjects.dedup();
    format!(
        "{}\u{1f}{}\u{1f}{}",
        status.unwrap_or("<missing-status>").to_ascii_lowercase(),
        normalized.join("\u{1e}"),
        structured_subjects.join("\u{1e}")
    )
}

pub(super) fn workflow_result_requires_repair(
    parsed: &serde_json::Value,
    diagnostics: &[String],
) -> bool {
    let status = parsed
        .get("status")
        .and_then(serde_json::Value::as_str)
        .unwrap_or_default();
    let failed_status = workflow_status_requires_repair(status);
    let infeasible_plan = parsed.get("feasible").and_then(serde_json::Value::as_bool)
        == Some(false)
        || parsed
            .get("capability_plan")
            .and_then(|plan| plan.get("feasible"))
            .and_then(serde_json::Value::as_bool)
            == Some(false);
    failed_status || infeasible_plan || !diagnostics.is_empty()
}

pub(super) fn workflow_status_requires_repair(status: &str) -> bool {
    matches!(
        status,
        "error"
            | "cancelled"
            | "timeout"
            | "validation_error"
            | "validation_errors"
            | "stale"
            | "no_changes"
            | "infeasible"
            | "candidate_regression"
            | "scope_reduction_blocked"
            | "resource_limit_rejected"
            | "revision_conflict"
            | "request_identity_mismatch"
            | "module_needs_repair"
            | "draft_needs_repair"
            | "discovery_blocked"
            | "discovery_budget_exhausted"
            | "edit_budget_exhausted"
            | "edit_in_flight"
            | "internal_state_unavailable"
    )
}

pub(super) fn workflow_result_clears_repair(parsed: &serde_json::Value) -> bool {
    matches!(
        parsed.get("status").and_then(serde_json::Value::as_str),
        Some(
            "queued"
                | "already_queued"
                | "rendered"
                | "valid"
                | "draft_valid"
                | "draft_updated"
                | "module_validated"
                | "draft_started"
        )
    )
}

pub(super) fn workflow_result_fallback_message(parsed: &serde_json::Value) -> Option<String> {
    let code = parsed.get("code").and_then(serde_json::Value::as_str);
    let message = parsed.get("message").and_then(serde_json::Value::as_str);
    match (code, message) {
        (Some(code), Some(message)) => Some(format!("[{code}] {message}")),
        (None, Some(message)) => Some(message.to_string()),
        (Some(code), None) => Some(format!("[{code}] Workflow validation needs repair.")),
        (None, None) => None,
    }
}
