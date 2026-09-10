use serde_json::{Value, json};

use super::behavioral_evaluation::{WorkflowBenchmarkCase, WorkflowBenchmarkCheck};

fn check(id: &str, entry: &str, payload: Value, expected_output: Value) -> WorkflowBenchmarkCheck {
    WorkflowBenchmarkCheck {
        id: id.to_string(),
        entry: entry.to_string(),
        payload: Some(payload),
        expected_output,
    }
}

fn case(
    id: &str,
    prompt: &str,
    source: &str,
    required_functions: &[&str],
    checks: Vec<WorkflowBenchmarkCheck>,
) -> WorkflowBenchmarkCase {
    WorkflowBenchmarkCase {
        id: id.to_string(),
        prompt: prompt.to_string(),
        initial_source: None,
        reference_source: source.to_string(),
        required_functions: required_functions
            .iter()
            .map(|name| name.to_string())
            .collect(),
        checks,
    }
}

fn repair(mut case: WorkflowBenchmarkCase, correct: &str, broken: &str) -> WorkflowBenchmarkCase {
    assert_eq!(case.reference_source.matches(correct).count(), 1);
    case.initial_source = Some(case.reference_source.replacen(correct, broken, 1));
    case
}

/// Fixed specifications and host-owned grading fixtures. Only `prompt` belongs in model context.
pub fn workflow_benchmark_cases() -> Vec<WorkflowBenchmarkCase> {
    let mut cases = vec![
        case(
            "profile-card",
            "Create a Generic Event named profileCard with string inputs name and country. Trim name and uppercase country. Return exactly one object with profile: {name: the trimmed name, country: the uppercase country} and labels: [the trimmed name, the uppercase country]. Use the supplied values, including empty strings. Create no app resources or external effects.",
            r#"eventsGeneric profileCard(name: string, country: string) {
    const label = name.trim()
    const code = country.toUpper()
    return { profile: { name: label, country: code }, labels: [label, code] }
}
"#,
            &[],
            vec![
                check(
                    "padded",
                    "profileCard",
                    json!({"name": " Ada ", "country": "de"}),
                    json!({"profile": {"name": "Ada", "country": "DE"}, "labels": ["Ada", "DE"]}),
                ),
                check(
                    "empty",
                    "profileCard",
                    json!({"name": "  ", "country": "us"}),
                    json!({"profile": {"name": "", "country": "US"}, "labels": ["", "US"]}),
                ),
                check(
                    "already-clean",
                    "profileCard",
                    json!({"name": "Lin", "country": "GB"}),
                    json!({"profile": {"name": "Lin", "country": "GB"}, "labels": ["Lin", "GB"]}),
                ),
            ],
        ),
        case(
            "net-total",
            "Create a Generic Event named netTotal with integer inputs subtotal, shipping and discount. Return exactly {net: subtotal + shipping - discount}. Preserve negative results rather than clamping them. Create no app resources or external effects.",
            r#"eventsGeneric netTotal(subtotal: int, shipping: int, discount: int) {
    const net = subtotal + shipping - discount
    return { net: net }
}
"#,
            &[],
            vec![
                check(
                    "ordinary",
                    "netTotal",
                    json!({"subtotal": 100, "shipping": 7, "discount": 12}),
                    json!({"net": 95}),
                ),
                check(
                    "zero",
                    "netTotal",
                    json!({"subtotal": 0, "shipping": 0, "discount": 0}),
                    json!({"net": 0}),
                ),
                check(
                    "negative",
                    "netTotal",
                    json!({"subtotal": 7, "shipping": 2, "discount": 12}),
                    json!({"net": -3}),
                ),
                check(
                    "shipping",
                    "netTotal",
                    json!({"subtotal": 14, "shipping": 3, "discount": 2}),
                    json!({"net": 15}),
                ),
            ],
        ),
        case(
            "access-gate",
            "Create a Generic Event named accessGate with boolean inputs member and suspended. Return exactly {allowed: boolean}. Access is allowed only when member is true and suspended is false. Return one result for every input combination. Create no app resources or external effects.",
            r#"eventsGeneric accessGate(member: bool, suspended: bool) {
    if (suspended) {
        return { allowed: false }
    } else {
        return { allowed: member }
    }
}
"#,
            &[],
            vec![
                check(
                    "active-member",
                    "accessGate",
                    json!({"member": true, "suspended": false}),
                    json!({"allowed": true}),
                ),
                check(
                    "suspended-member",
                    "accessGate",
                    json!({"member": true, "suspended": true}),
                    json!({"allowed": false}),
                ),
                check(
                    "visitor",
                    "accessGate",
                    json!({"member": false, "suspended": false}),
                    json!({"allowed": false}),
                ),
                check(
                    "suspended-visitor",
                    "accessGate",
                    json!({"member": false, "suspended": true}),
                    json!({"allowed": false}),
                ),
            ],
        ),
        case(
            "priority-bands",
            "Create a Generic Event named priorityBand with integer input score. Return exactly {band: string}: low for scores below 50, medium for scores from 50 through 79, and high for scores of 80 or greater. Negative scores are low. Create no app resources or external effects.",
            r#"eventsGeneric priorityBand(score: int) {
    if (score < 50) {
        return { band: "low" }
    } else {
        if (score < 80) {
            return { band: "medium" }
        } else {
            return { band: "high" }
        }
    }
}
"#,
            &[],
            vec![
                check(
                    "negative",
                    "priorityBand",
                    json!({"score": -1}),
                    json!({"band": "low"}),
                ),
                check(
                    "below-medium",
                    "priorityBand",
                    json!({"score": 49}),
                    json!({"band": "low"}),
                ),
                check(
                    "medium-boundary",
                    "priorityBand",
                    json!({"score": 50}),
                    json!({"band": "medium"}),
                ),
                check(
                    "below-high",
                    "priorityBand",
                    json!({"score": 79}),
                    json!({"band": "medium"}),
                ),
                check(
                    "high-boundary",
                    "priorityBand",
                    json!({"score": 80}),
                    json!({"band": "high"}),
                ),
            ],
        ),
        case(
            "normalize-pair-helper",
            "Create a helper function named normalizeLabel that accepts a string and returns its trimmed lowercase form. Create a Generic Event named normalizePair with string inputs left and right. Invoke normalizeLabel separately for each input and return exactly {left: normalized left, right: normalized right}. Keep the helper as a reusable function. Create no app resources or external effects.",
            r#"function normalizeLabel(value: string): (label: string) {
    return value.trim().toLower()
}

eventsGeneric normalizePair(left: string, right: string) {
    const first = normalizeLabel({ value: left })
    const second = normalizeLabel({ value: right })
    return { left: first, right: second }
}
"#,
            &["normalizeLabel"],
            vec![
                check(
                    "distinct",
                    "normalizePair",
                    json!({"left": " ALICE ", "right": "BOB "}),
                    json!({"left": "alice", "right": "bob"}),
                ),
                check(
                    "reversed",
                    "normalizePair",
                    json!({"left": " BOB ", "right": "ALICE "}),
                    json!({"left": "bob", "right": "alice"}),
                ),
                check(
                    "empty-first",
                    "normalizePair",
                    json!({"left": "  ", "right": " X "}),
                    json!({"left": "", "right": "x"}),
                ),
            ],
        ),
        case(
            "totals-helper",
            "Create a reusable helper function named orderNet that takes integer subtotal, shipping and discount and returns subtotal + shipping - discount without clamping. Create a Generic Event named pairedTotals with integer inputs firstSubtotal, firstShipping, firstDiscount, secondSubtotal, secondShipping and secondDiscount. Invoke orderNet separately for the two orders and return exactly {first: first net, second: second net}. Create no app resources or external effects.",
            r#"function orderNet(subtotal: int, shipping: int, discount: int): (net: int) {
    return subtotal + shipping - discount
}

eventsGeneric pairedTotals(firstSubtotal: int, firstShipping: int, firstDiscount: int, secondSubtotal: int, secondShipping: int, secondDiscount: int) {
    const first = orderNet({ subtotal: firstSubtotal, shipping: firstShipping, discount: firstDiscount })
    const second = orderNet({ subtotal: secondSubtotal, shipping: secondShipping, discount: secondDiscount })
    return { first: first, second: second }
}
"#,
            &["orderNet"],
            vec![
                check(
                    "distinct",
                    "pairedTotals",
                    json!({"firstSubtotal": 10, "firstShipping": 2, "firstDiscount": 3, "secondSubtotal": 8, "secondShipping": 1, "secondDiscount": 9}),
                    json!({"first": 9, "second": 0}),
                ),
                check(
                    "reversed",
                    "pairedTotals",
                    json!({"firstSubtotal": 8, "firstShipping": 1, "firstDiscount": 9, "secondSubtotal": 10, "secondShipping": 2, "secondDiscount": 3}),
                    json!({"first": 0, "second": 9}),
                ),
                check(
                    "negative",
                    "pairedTotals",
                    json!({"firstSubtotal": 2, "firstShipping": 0, "firstDiscount": 5, "secondSubtotal": 0, "secondShipping": 4, "secondDiscount": 0}),
                    json!({"first": -3, "second": 4}),
                ),
            ],
        ),
        case(
            "nested-helpers",
            "Create a helper function named cleanHandle that trims and lowercases a string. Create a second helper named isPriorityHandle that calls cleanHandle and returns whether the cleaned string equals vip exactly. Create a Generic Event named handleSummary with string input text. Return exactly {handle: cleaned text, priority: the boolean from isPriorityHandle}. Keep both helper functions and their nested call. Create no app resources or external effects.",
            r#"function cleanHandle(value: string): (handle: string) {
    return value.trim().toLower()
}

function isPriorityHandle(value: string): (priority: bool) {
    const cleaned = cleanHandle({ value: value })
    return cleaned == "vip"
}

eventsGeneric handleSummary(text: string) {
    const cleaned = cleanHandle({ value: text })
    const classified = isPriorityHandle({ value: text })
    return { handle: cleaned, priority: classified }
}
"#,
            &["cleanHandle", "isPriorityHandle"],
            vec![
                check(
                    "priority",
                    "handleSummary",
                    json!({"text": " VIP "}),
                    json!({"handle": "vip", "priority": true}),
                ),
                check(
                    "suffix",
                    "handleSummary",
                    json!({"text": " Vip2 "}),
                    json!({"handle": "vip2", "priority": false}),
                ),
                check(
                    "empty",
                    "handleSummary",
                    json!({"text": "  "}),
                    json!({"handle": "", "priority": false}),
                ),
                check(
                    "ordinary",
                    "handleSummary",
                    json!({"text": " Ada "}),
                    json!({"handle": "ada", "priority": false}),
                ),
            ],
        ),
        case(
            "row-helper",
            "Create a reusable helper function named nextRow with string code and integer quantity inputs. Return a Struct containing code trimmed and uppercased, and next equal to quantity + 1. Create a Generic Event named pairedRows with firstCode, firstQuantity, secondCode and secondQuantity inputs. Call nextRow once per pair and return exactly {rows: [first row, second row]} in that order. Create no app resources or external effects.",
            r#"function nextRow(code: string, quantity: int): (row: Struct) {
    const label = code.trim().toUpper()
    const next = quantity + 1
    return { code: label, next: next }
}

eventsGeneric pairedRows(firstCode: string, firstQuantity: int, secondCode: string, secondQuantity: int) {
    const first = nextRow({ code: firstCode, quantity: firstQuantity })
    const second = nextRow({ code: secondCode, quantity: secondQuantity })
    return { rows: [first, second] }
}
"#,
            &["nextRow"],
            vec![
                check(
                    "distinct",
                    "pairedRows",
                    json!({"firstCode": " a ", "firstQuantity": 2, "secondCode": "b ", "secondQuantity": 8}),
                    json!({"rows": [{"code": "A", "next": 3}, {"code": "B", "next": 9}]}),
                ),
                check(
                    "reversed",
                    "pairedRows",
                    json!({"firstCode": " b ", "firstQuantity": 8, "secondCode": "a ", "secondQuantity": 2}),
                    json!({"rows": [{"code": "B", "next": 9}, {"code": "A", "next": 3}]}),
                ),
                check(
                    "empty-and-negative",
                    "pairedRows",
                    json!({"firstCode": "  ", "firstQuantity": -2, "secondCode": "x", "secondQuantity": 0}),
                    json!({"rows": [{"code": "", "next": -1}, {"code": "X", "next": 1}]}),
                ),
            ],
        ),
        repair(
            case(
                "repair-casing",
                "Repair the existing normalizeText Generic Event. Its text input must be trimmed and lowercased, and its output must remain exactly {label: normalized text}. Keep the existing normalizationStatus entry and its output unchanged. Preserve the existing board structure and entry identities. Create no app resources or external effects.",
                r#"eventsGeneric normalizeText(text: string) {
    const label = text.trim().toLower()
    return { label: label }
}

eventsGeneric normalizationStatus() {
    return { version: 1 }
}
"#,
                &[],
                vec![
                    check(
                        "mixed",
                        "normalizeText",
                        json!({"text": " HeLLo "}),
                        json!({"label": "hello"}),
                    ),
                    check(
                        "already-lower",
                        "normalizeText",
                        json!({"text": " ada "}),
                        json!({"label": "ada"}),
                    ),
                    check(
                        "empty",
                        "normalizeText",
                        json!({"text": "  "}),
                        json!({"label": ""}),
                    ),
                    check(
                        "preserve-status",
                        "normalizationStatus",
                        json!({}),
                        json!({"version": 1}),
                    ),
                ],
            ),
            "text.trim().toLower()",
            "text.trim().toUpper()",
        ),
        repair(
            case(
                "repair-boundary",
                "Repair the existing alertBand Generic Event so integer scores of 80 or greater return exactly {band: critical}, while scores below 80 return exactly {band: standard}. Its current boundary is wrong. Keep the existing alertHealth entry and its output unchanged. Preserve the existing entry identities. Create no app resources or external effects.",
                r#"eventsGeneric alertBand(score: int) {
    if (score < 80) {
        return { band: "standard" }
    } else {
        return { band: "critical" }
    }
}

eventsGeneric alertHealth() {
    return { ok: true }
}
"#,
                &[],
                vec![
                    check(
                        "below",
                        "alertBand",
                        json!({"score": 79}),
                        json!({"band": "standard"}),
                    ),
                    check(
                        "boundary",
                        "alertBand",
                        json!({"score": 80}),
                        json!({"band": "critical"}),
                    ),
                    check(
                        "above",
                        "alertBand",
                        json!({"score": 81}),
                        json!({"band": "critical"}),
                    ),
                    check(
                        "preserve-health",
                        "alertHealth",
                        json!({}),
                        json!({"ok": true}),
                    ),
                ],
            ),
            "score < 80",
            "score < 81",
        ),
        repair(
            case(
                "repair-helper-arguments",
                "Repair the existing remainingAmount helper so it returns integer total minus discount, including negative results. Keep it as a reusable function and keep amountSummary returning exactly {remaining: the helper result}. Preserve the existing amountHealth entry, its output, and all existing entry identities. Create no app resources or external effects.",
                r#"function remainingAmount(total: int, discount: int): (amount: int) {
    return total - discount
}

eventsGeneric amountSummary(total: int, discount: int) {
    const result = remainingAmount({ total: total, discount: discount })
    return { remaining: result }
}

eventsGeneric amountHealth() {
    return { currency: "EUR" }
}
"#,
                &["remainingAmount"],
                vec![
                    check(
                        "positive",
                        "amountSummary",
                        json!({"total": 10, "discount": 2}),
                        json!({"remaining": 8}),
                    ),
                    check(
                        "negative",
                        "amountSummary",
                        json!({"total": 2, "discount": 10}),
                        json!({"remaining": -8}),
                    ),
                    check(
                        "zero",
                        "amountSummary",
                        json!({"total": 0, "discount": 0}),
                        json!({"remaining": 0}),
                    ),
                    check(
                        "preserve-health",
                        "amountHealth",
                        json!({}),
                        json!({"currency": "EUR"}),
                    ),
                ],
            ),
            "total - discount",
            "discount - total",
        ),
        repair(
            case(
                "repair-nested-field",
                "Repair the existing customerSummary Generic Event. Its customer Struct input contains displayName and accountName; output customer must use the trimmed displayName. Keep the integer amount input unchanged in exactly {customer: trimmed displayName, amount: amount}. Preserve customerHealth, its output, and the existing entry identities. Create no app resources or external effects.",
                r#"eventsGeneric customerSummary(customer: Struct, amount: int) {
    const label = customer.displayName.trim()
    return { customer: label, amount: amount }
}

eventsGeneric customerHealth() {
    return { schema: "customer-v1" }
}
"#,
                &[],
                vec![
                    check(
                        "distinct-fields",
                        "customerSummary",
                        json!({"customer": {"displayName": " Ada ", "accountName": "wrong"}, "amount": 25}),
                        json!({"customer": "Ada", "amount": 25}),
                    ),
                    check(
                        "zero",
                        "customerSummary",
                        json!({"customer": {"displayName": " Lin ", "accountName": "other"}, "amount": 0}),
                        json!({"customer": "Lin", "amount": 0}),
                    ),
                    check(
                        "empty",
                        "customerSummary",
                        json!({"customer": {"displayName": "  ", "accountName": "decoy"}, "amount": -4}),
                        json!({"customer": "", "amount": -4}),
                    ),
                    check(
                        "preserve-health",
                        "customerHealth",
                        json!({}),
                        json!({"schema": "customer-v1"}),
                    ),
                ],
            ),
            "customer.displayName",
            "customer.accountName",
        ),
    ];
    cases.extend(holdout_cases());
    cases
}

fn holdout_cases() -> Vec<WorkflowBenchmarkCase> {
    vec![
        case(
            "holdout-adjusted-gaps",
            "Create a reusable helper named applyOffset with integer base and delta inputs that returns base + delta. Create a second reusable helper named adjustedGap with integer measured, baseline and offset inputs. It must call applyOffset to adjust measured by offset, then return the adjusted value minus baseline. Create a Generic Event named gapPair with integer left, right and offset inputs. Call adjustedGap once with left measured against right, and once with right measured against left, applying the same offset in each direction. Return exactly {forward: the first gap, reverse: the second gap}. Keep both helpers and their nested call. Bind calls by the declared argument names and preserve negative results. Create no app resources or external effects.",
            r#"function applyOffset(base: int, delta: int): (adjusted: int) {
    return base + delta
}

function adjustedGap(measured: int, baseline: int, offset: int): (gap: int) {
    const adjusted = applyOffset({ delta: offset, base: measured })
    return adjusted - baseline
}

eventsGeneric gapPair(left: int, right: int, offset: int) {
    const forward = adjustedGap({ offset: offset, baseline: right, measured: left })
    const reverse = adjustedGap({ baseline: left, measured: right, offset: offset })
    return { forward: forward, reverse: reverse }
}
"#,
            &["applyOffset", "adjustedGap"],
            vec![
                check(
                    "distinct",
                    "gapPair",
                    json!({"left": 10, "right": 3, "offset": 2}),
                    json!({"forward": 9, "reverse": -5}),
                ),
                check(
                    "reversed-negative-offset",
                    "gapPair",
                    json!({"left": 3, "right": 10, "offset": -2}),
                    json!({"forward": -9, "reverse": 5}),
                ),
                check(
                    "negative-inputs",
                    "gapPair",
                    json!({"left": -4, "right": -9, "offset": 3}),
                    json!({"forward": 8, "reverse": -2}),
                ),
                check(
                    "zero",
                    "gapPair",
                    json!({"left": 0, "right": 0, "offset": 0}),
                    json!({"forward": 0, "reverse": 0}),
                ),
                check(
                    "equal-inputs",
                    "gapPair",
                    json!({"left": 7, "right": 7, "offset": -1}),
                    json!({"forward": -1, "reverse": -1}),
                ),
            ],
        ),
        case(
            "holdout-review-route",
            "Create a Generic Event named reviewRoute with boolean urgent and blocked inputs, integer attempt, and string label. Return exactly {route: string, ticket: {label: the trimmed input label, nextAttempt: integer}}. A blocked ticket always uses route hold and keeps attempt unchanged, including when urgent is true. An unblocked ticket increments attempt by one and uses route fast when urgent is true, or normal otherwise. Preserve negative attempt values and empty trimmed labels. Return one result for every input combination. Create no app resources or external effects.",
            r#"eventsGeneric reviewRoute(urgent: bool, blocked: bool, attempt: int, label: string) {
    const cleaned = label.trim()
    if (blocked) {
        return { route: "hold", ticket: { label: cleaned, nextAttempt: attempt } }
    } else {
        const next = attempt + 1
        if (urgent) {
            return { route: "fast", ticket: { label: cleaned, nextAttempt: next } }
        } else {
            return { route: "normal", ticket: { label: cleaned, nextAttempt: next } }
        }
    }
}
"#,
            &[],
            vec![
                check(
                    "blocked-urgent",
                    "reviewRoute",
                    json!({"urgent": true, "blocked": true, "attempt": 4, "label": " Case A "}),
                    json!({"route": "hold", "ticket": {"label": "Case A", "nextAttempt": 4}}),
                ),
                check(
                    "blocked-ordinary",
                    "reviewRoute",
                    json!({"urgent": false, "blocked": true, "attempt": -1, "label": " Hold "}),
                    json!({"route": "hold", "ticket": {"label": "Hold", "nextAttempt": -1}}),
                ),
                check(
                    "urgent",
                    "reviewRoute",
                    json!({"urgent": true, "blocked": false, "attempt": 2, "label": " Fast "}),
                    json!({"route": "fast", "ticket": {"label": "Fast", "nextAttempt": 3}}),
                ),
                check(
                    "ordinary",
                    "reviewRoute",
                    json!({"urgent": false, "blocked": false, "attempt": 0, "label": " Open "}),
                    json!({"route": "normal", "ticket": {"label": "Open", "nextAttempt": 1}}),
                ),
                check(
                    "empty-negative",
                    "reviewRoute",
                    json!({"urgent": false, "blocked": false, "attempt": -2, "label": "  "}),
                    json!({"route": "normal", "ticket": {"label": "", "nextAttempt": -1}}),
                ),
            ],
        ),
        case(
            "holdout-record-envelopes",
            "Create a Generic Event named recordEnvelopes with a Struct input record and a string input tag. Return exactly {original: record, envelopes: [{label: the trimmed tag, record: record}, {label: the trimmed and uppercased tag, record: record}]} in that order. Preserve every field of record unchanged in all three copies, including unknown fields, nested objects, arrays, nulls and booleans. Do not add fields to record or replace it with a selected subset. Empty records and tags are valid. Create no app resources or external effects.",
            r#"eventsGeneric recordEnvelopes(record: Struct, tag: string) {
    const label = tag.trim()
    const upper = label.toUpper()
    const first = { label: label, record: record }
    const second = { label: upper, record: record }
    return { original: record, envelopes: [first, second] }
}
"#,
            &[],
            vec![
                envelope_check(
                    "unknown-nested",
                    json!({"id": "p-17", "nested": {"flag": false, "notes": ["a", null]}, "extra": 3}),
                    " north ",
                    "north",
                    "NORTH",
                ),
                envelope_check(
                    "null-and-empty-array",
                    json!({"id": null, "items": [], "metadata": {"color": "blue"}}),
                    " Mx ",
                    "Mx",
                    "MX",
                ),
                envelope_check("empty-record-and-tag", json!({}), "  ", "", ""),
                envelope_check(
                    "matrix-and-boolean",
                    json!({"count": -5, "matrix": [[1, 2], []], "active": true}),
                    "Already",
                    "Already",
                    "ALREADY",
                ),
            ],
        ),
        case(
            "holdout-float-stages",
            "Create a Generic Event named floatStages with float inputs raw, offset and correction. Compute intermediate as raw + offset, then total as intermediate + correction. Return exactly {intermediate: intermediate, total: total, stages: [intermediate, total]} in that order. Use the supplied values and preserve fractional, negative and zero results. Do not round, clamp or convert results to strings or integers. Create no app resources or external effects.",
            r#"eventsGeneric floatStages(raw: float, offset: float, correction: float) {
    const intermediate = raw + offset
    const total = intermediate + correction
    return { intermediate: intermediate, total: total, stages: [intermediate, total] }
}
"#,
            &[],
            vec![
                check(
                    "fractional",
                    "floatStages",
                    json!({"raw": 1.25, "offset": 0.5, "correction": -0.125}),
                    json!({"intermediate": 1.75, "total": 1.625, "stages": [1.75, 1.625]}),
                ),
                check(
                    "negative",
                    "floatStages",
                    json!({"raw": -2.5, "offset": 0.25, "correction": 0.5}),
                    json!({"intermediate": -2.25, "total": -1.75, "stages": [-2.25, -1.75]}),
                ),
                check(
                    "zero",
                    "floatStages",
                    json!({"raw": 0.0, "offset": 0.0, "correction": 0.0}),
                    json!({"intermediate": 0.0, "total": 0.0, "stages": [0.0, 0.0]}),
                ),
                check(
                    "larger-magnitude",
                    "floatStages",
                    json!({"raw": 1024.125, "offset": -0.125, "correction": 0.5}),
                    json!({"intermediate": 1024.0, "total": 1024.5, "stages": [1024.0, 1024.5]}),
                ),
                check(
                    "cancellation",
                    "floatStages",
                    json!({"raw": 0.125, "offset": 0.125, "correction": -0.25}),
                    json!({"intermediate": 0.25, "total": 0.0, "stages": [0.25, 0.0]}),
                ),
            ],
        ),
    ]
}

fn envelope_check(
    id: &str,
    record: Value,
    tag: &str,
    label: &str,
    upper: &str,
) -> WorkflowBenchmarkCheck {
    check(
        id,
        "recordEnvelopes",
        json!({"record": record, "tag": tag}),
        json!({"original": record, "envelopes": [{"label": label, "record": record}, {"label": upper, "record": record}]}),
    )
}
