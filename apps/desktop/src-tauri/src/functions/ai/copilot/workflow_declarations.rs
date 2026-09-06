//! Declaration coverage, repair queries, and retained context.

use super::stream_events::truncate_for_preview;
use super::workflow_state::{MAX_REPAIR_DECLARATION_QUERY_BYTES, MAX_RETAINED_DECLARATION_BYTES};
use std::collections::HashSet;

/// Enforce a short, edit-first workflow loop for external code agents. Prompt guidance alone is
/// insufficient for CLIs whose default code-agent behavior keeps searching: this per-run gate
/// makes the productive path deterministic while leaving read-only/global sessions untouched.
fn normalize_declaration_signature(value: &str) -> Option<String> {
    let value = value.trim();
    if value.is_empty()
        || !value
            .chars()
            .all(|character| character.is_ascii_alphanumeric() || character == '_')
        || !value
            .chars()
            .next()
            .is_some_and(|character| character.is_ascii_alphabetic() || character == '_')
    {
        return None;
    }
    let normalized = value.to_ascii_lowercase();
    (!matches!(
        normalized.as_str(),
        "get_declarations" | "edit_flowscript" | "flowscript" | "function" | "event" | "events"
    ))
    .then_some(normalized)
}

#[derive(Debug, Default, PartialEq, Eq)]
pub(super) struct DeclarationRepairHints {
    pub(super) exact_symbols: HashSet<String>,
    pub(super) topics: HashSet<String>,
}

impl DeclarationRepairHints {
    pub(super) fn is_empty(&self) -> bool {
        self.exact_symbols.is_empty() && self.topics.is_empty()
    }

    pub(super) fn exposed_targets(&self) -> Vec<String> {
        let mut targets = self
            .exact_symbols
            .iter()
            .map(|symbol| format!("symbol:{symbol}"))
            .chain(self.topics.iter().map(|topic| format!("topic:{topic}")))
            .collect::<Vec<_>>();
        targets.sort();
        targets
    }
}

/// Extract only repairable catalog evidence from reconcile diagnostics. In addition to explicit
/// missing-declaration messages, pin and type diagnostics name the exact node whose declaration is
/// needed. Comparison failures also justify narrowly scoped equality/conversion discovery even
/// when the reconciler cannot know the eventual catalog function name.
pub(super) fn diagnostic_declaration_repair_hints(
    diagnostics: &[String],
) -> DeclarationRepairHints {
    let mut hints = DeclarationRepairHints::default();
    for diagnostic in diagnostics {
        let lower = diagnostic.to_ascii_lowercase();

        let parts = diagnostic.split('`').collect::<Vec<_>>();
        for index in (1..parts.len()).step_by(2) {
            let context = parts[index - 1].trim_end().to_ascii_lowercase();
            let names_catalog_symbol = ["node", "on", "call", "declaration"]
                .iter()
                .any(|marker| context.ends_with(marker));
            if names_catalog_symbol
                && let Some(symbol) = normalize_declaration_signature(parts[index])
            {
                hints.exact_symbols.insert(symbol);
            }
        }

        // Some provider wrappers flatten the canonical backtick formatting. The text before this
        // exact diagnostic phrase is still an explicit signature, not a broad intent guess.
        if let Some(end) = lower.find(" does not match a catalog declaration")
            && let Some(candidate) = diagnostic[..end].split_whitespace().next_back()
            && let Some(signature) =
                normalize_declaration_signature(candidate.trim_matches(|character: char| {
                    !character.is_ascii_alphanumeric() && character != '_'
                }))
        {
            hints.exact_symbols.insert(signature);
        }

        if let Some(candidates) = lower.split("candidates are").nth(1) {
            for candidate in candidates
                .split(|character: char| {
                    character == ',' || character == ';' || character.is_whitespace()
                })
                .filter_map(|candidate| {
                    normalize_declaration_signature(candidate.trim_matches(|character: char| {
                        !character.is_ascii_alphanumeric() && character != '_'
                    }))
                })
            {
                hints.exact_symbols.insert(candidate);
            }
        }

        let comparison_failure = lower.contains("binary comparison")
            || lower.contains("ambiguous operand type")
            || lower.contains("incompatible operand type")
            || lower.contains("two-input catalog node");
        if comparison_failure {
            hints.topics.insert("comparison".to_string());
        }
        if comparison_failure
            && (lower.contains("generic")
                || lower.contains("operand type")
                || lower.contains("incompatible"))
        {
            hints.topics.insert("type_conversion".to_string());
        }
        if lower.contains("string") && (lower.contains("input pin") || lower.contains("argument")) {
            hints.topics.insert("string_operations".to_string());
        }
    }
    hints
}

pub(super) fn declaration_lookup_queries(args: &serde_json::Value) -> Vec<&str> {
    let mut queries = Vec::new();
    if let Some(query) = args.get("query").and_then(serde_json::Value::as_str) {
        queries.push(query);
    }
    if let Some(batch) = args.get("queries").and_then(serde_json::Value::as_array) {
        queries.extend(batch.iter().filter_map(serde_json::Value::as_str));
    }
    queries
}

fn declaration_query_matches_signature(query: &str, signature: &str) -> bool {
    let compact_query = query
        .chars()
        .filter(char::is_ascii_alphanumeric)
        .flat_map(char::to_lowercase)
        .collect::<String>();
    let compact_signature = signature
        .chars()
        .filter(char::is_ascii_alphanumeric)
        .collect::<String>();
    !compact_query.is_empty() && compact_query.contains(&compact_signature)
}

pub(super) fn declaration_repair_query_is_bounded(query: &str) -> bool {
    let normalized = query
        .split_whitespace()
        .map(str::to_ascii_lowercase)
        .collect::<Vec<_>>()
        .join(" ");
    query.len() <= MAX_REPAIR_DECLARATION_QUERY_BYTES
        && query.split_whitespace().count() <= 16
        && ![
            "entire catalog",
            "whole catalog",
            "all catalog",
            "all nodes",
            "every node",
            "everything",
            "broad search",
            "search the catalog",
        ]
        .iter()
        .any(|phrase| normalized.contains(phrase))
}

pub(super) fn declaration_repair_query_keys(
    query: &str,
    hints: &DeclarationRepairHints,
) -> HashSet<String> {
    let mut keys = hints
        .exact_symbols
        .iter()
        .filter(|symbol| declaration_query_matches_signature(query, symbol))
        .map(|symbol| format!("symbol:{symbol}"))
        .collect::<HashSet<_>>();
    let compact = query
        .chars()
        .filter(char::is_ascii_alphanumeric)
        .flat_map(char::to_lowercase)
        .collect::<String>();

    if hints.topics.contains("comparison")
        && ["equal", "compare", "comparison", "inequal", "sametext"]
            .iter()
            .any(|term| compact.contains(term))
    {
        keys.insert("topic:comparison".to_string());
    }
    if hints.topics.contains("type_conversion")
        && [
            "convert",
            "conversion",
            "cast",
            "coerce",
            "tostring",
            "tobool",
            "toint",
        ]
        .iter()
        .any(|term| compact.contains(term))
    {
        keys.insert("topic:type_conversion".to_string());
    }
    if hints.topics.contains("string_operations")
        && [
            "stringcontains",
            "stringtrim",
            "stringstartswith",
            "stringreplace",
            "contains",
            "trim",
            "startswith",
            "replace",
        ]
        .iter()
        .any(|term| compact.contains(term))
    {
        keys.insert("topic:string_operations".to_string());
    }
    keys
}

pub(super) fn declaration_result_is_usable(result_text: &str) -> bool {
    result_text.lines().any(declaration_line_is_complete)
}

fn declaration_line_is_complete(line: &str) -> bool {
    let line = line.trim();
    flow_like::flow::ast::is_signature_line(line)
        && line.contains('(')
        && line.contains(')')
        && line.contains(';')
}

#[derive(Debug, Default, PartialEq, Eq)]
pub(super) struct DeclarationBatchCoverage {
    pub(super) processed_count: usize,
    pub(super) complete: bool,
    pub(super) matched_count: usize,
    pub(super) matched_queries: Vec<String>,
    pub(super) unmatched_queries: Vec<String>,
    pub(super) output_omitted_queries: Vec<String>,
    pub(super) omitted_queries: Vec<String>,
    pub(super) unmatched_count: usize,
    pub(super) output_omitted_count: usize,
    pub(super) omitted_count: usize,
    pub(super) truncated_query_count: usize,
    pub(super) query_names_omitted_for_size: bool,
}

fn declaration_query_key(query: &str) -> String {
    query
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
        .to_lowercase()
}

pub(super) fn declaration_queries_are_related(left: &str, right: &str) -> bool {
    let left_key = declaration_query_key(left);
    let right_key = declaration_query_key(right);
    if left_key == right_key {
        return true;
    }
    const GENERIC_TERMS: &[&str] = &[
        "add", "approval", "build", "create", "data", "delete", "email", "fetch", "file", "find",
        "for", "from", "get", "into", "list", "mail", "make", "message", "node", "open", "read",
        "receive", "remove", "response", "run", "send", "set", "string", "the", "update", "use",
        "with", "workflow", "write",
    ];
    let distinctive = |value: &str| {
        value
            .split_whitespace()
            .filter(|token| token.len() >= 3 && !GENERIC_TERMS.contains(token))
            .map(str::to_string)
            .collect::<HashSet<_>>()
    };
    let left_terms = distinctive(&left_key);
    let right_terms = distinctive(&right_key);
    if left_terms.is_disjoint(&right_terms) {
        return false;
    }

    // Connector identity alone is not enough: `smtp send` and `smtp receive` are different
    // capabilities even though both retain the distinctive `smtp` token. When both phrasings name
    // an operation, require the operation family itself to survive the rephrase.
    const OPERATION_TERMS: &[&str] = &[
        "add", "compare", "connect", "convert", "create", "delete", "fetch", "find", "get", "list",
        "parse", "read", "receive", "remove", "replace", "search", "send", "set", "trim", "update",
        "write",
    ];
    let operations = |value: &str| {
        value
            .split_whitespace()
            .filter(|token| OPERATION_TERMS.contains(token))
            .map(str::to_string)
            .collect::<HashSet<_>>()
    };
    let left_operations = operations(&left_key);
    let right_operations = operations(&right_key);
    left_operations.is_empty()
        || right_operations.is_empty()
        || !left_operations.is_disjoint(&right_operations)
}

pub(super) fn declaration_batch_coverage(result_text: &str) -> Option<DeclarationBatchCoverage> {
    const PREFIX: &str = "// flowpilot.declaration-batch/v1 ";
    let metadata = result_text
        .lines()
        .find_map(|line| line.strip_prefix(PREFIX))?;
    let value = serde_json::from_str::<serde_json::Value>(metadata).ok()?;
    let strings = |key: &str| {
        value
            .get(key)
            .and_then(serde_json::Value::as_array)
            .into_iter()
            .flatten()
            .filter_map(serde_json::Value::as_str)
            .map(str::to_string)
            .collect::<Vec<_>>()
    };
    Some(DeclarationBatchCoverage {
        processed_count: value
            .get("processed_count")
            .and_then(serde_json::Value::as_u64)
            .and_then(|count| usize::try_from(count).ok())
            .unwrap_or_default(),
        complete: value
            .get("complete")
            .and_then(serde_json::Value::as_bool)
            .unwrap_or(false),
        matched_count: value
            .get("matched_count")
            .and_then(serde_json::Value::as_u64)
            .and_then(|count| usize::try_from(count).ok())
            .unwrap_or_default(),
        matched_queries: strings("matched_queries"),
        unmatched_queries: strings("unmatched_queries"),
        output_omitted_queries: strings("output_omitted_queries"),
        omitted_queries: strings("omitted_queries"),
        unmatched_count: value
            .get("unmatched_count")
            .and_then(serde_json::Value::as_u64)
            .and_then(|count| usize::try_from(count).ok())
            .unwrap_or_default(),
        output_omitted_count: value
            .get("output_omitted_count")
            .and_then(serde_json::Value::as_u64)
            .and_then(|count| usize::try_from(count).ok())
            .unwrap_or_default(),
        omitted_count: value
            .get("omitted_count")
            .and_then(serde_json::Value::as_u64)
            .and_then(|count| usize::try_from(count).ok())
            .unwrap_or_default(),
        truncated_query_count: value
            .get("truncated_query_count")
            .and_then(serde_json::Value::as_u64)
            .and_then(|count| usize::try_from(count).ok())
            .unwrap_or_default(),
        query_names_omitted_for_size: value
            .get("query_names_omitted_for_size")
            .and_then(serde_json::Value::as_bool)
            .unwrap_or(false),
    })
}

pub(super) fn complete_declaration_coverage_is_coherent(
    coverage: &DeclarationBatchCoverage,
    args: &serde_json::Value,
    result_text: &str,
) -> bool {
    if !coverage.complete {
        return true;
    }
    let requested_count = declaration_lookup_queries(args)
        .into_iter()
        .map(declaration_query_key)
        .filter(|query| !query.is_empty())
        .collect::<HashSet<_>>()
        .len();
    let exact_declaration_count = result_text
        .lines()
        .filter(|line| declaration_line_is_complete(line))
        .count();
    coverage.processed_count > 0
        && coverage.processed_count == requested_count
        && coverage.matched_count == coverage.processed_count
        && (coverage.query_names_omitted_for_size
            || coverage.matched_queries.len() == coverage.matched_count)
        && coverage.unmatched_count == 0
        && coverage.output_omitted_count == 0
        && coverage.omitted_count == 0
        && coverage.truncated_query_count == 0
        && exact_declaration_count >= coverage.matched_count
}

pub(super) fn retain_declaration_result(existing: Option<&str>, result_text: &str) -> String {
    // Keep the newest bounded batch whole: its catalog-authored notes carry non-obvious ordering,
    // repeated-pin, schema-field, and companion-call guidance that signatures alone cannot encode.
    // Then retain older unique exact signatures while they fit. Never byte-truncate a declaration
    // line: a partial signature is worse than an explicit omission because it looks authoritative
    // to the next model process.
    let complete_lines = |text: &str| {
        text.lines()
            .map(str::trim)
            .filter(|line| declaration_line_is_complete(line))
            .map(str::to_string)
            .collect::<Vec<_>>()
    };
    let newest = complete_lines(result_text);
    let older = existing.map(complete_lines).unwrap_or_default();
    let mut seen = newest.iter().cloned().collect::<HashSet<_>>();
    let newest_full = result_text.trim();
    let mut retained = if newest_full.len() <= MAX_RETAINED_DECLARATION_BYTES {
        newest_full.to_string()
    } else {
        // Defensive fallback for a non-conforming worker: retain only whole exact signatures.
        newest.join("\n")
    };
    for line in &older {
        if !seen.insert(line.clone()) {
            continue;
        }
        let separator_bytes = usize::from(!retained.is_empty());
        if retained
            .len()
            .saturating_add(separator_bytes)
            .saturating_add(line.len())
            > MAX_RETAINED_DECLARATION_BYTES
        {
            continue;
        }
        if !retained.is_empty() {
            retained.push('\n');
        }
        retained.push_str(line);
    }
    if retained.is_empty() {
        // The caller normally invokes this only for a usable result. Keep a bounded diagnostic
        // fallback for defensive compatibility with older workers.
        truncate_for_preview(result_text, MAX_RETAINED_DECLARATION_BYTES)
    } else {
        retained
    }
}
