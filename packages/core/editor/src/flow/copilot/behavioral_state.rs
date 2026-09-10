//! Revision-bound runtime evidence for opt-in host repair controllers.
//!
//! This state does not change compiler validation, commit policy, or benchmark fixtures.

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};
use serde_json::{Value, json};

const MAX_CHECKS: usize = 32;
const MAX_EXPECTED_BYTES: usize = 16_384;
const MAX_FEEDBACK_CHECKS: usize = 8;
const MAX_VALUE_PREVIEW_BYTES: usize = 1_024;
const MAX_ERROR_BYTES: usize = 512;
const MAX_ERRORS: usize = 4;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct FlowScriptBehavioralRevision {
    pub board_id: String,
    pub draft_id: String,
    pub revision: u64,
    pub source_fingerprint: String,
    pub base_fingerprint: String,
    pub catalog_fingerprint: String,
    pub commands_fingerprint: String,
}

impl FlowScriptBehavioralRevision {
    fn validate(&self) -> Result<(), FlowScriptBehavioralError> {
        for value in [
            &self.board_id,
            &self.draft_id,
            &self.source_fingerprint,
            &self.base_fingerprint,
            &self.catalog_fingerprint,
            &self.commands_fingerprint,
        ] {
            validate_identity(value)?;
        }
        Ok(())
    }

    pub(crate) fn from_receipt(receipt: &Value) -> Result<Self, FlowScriptBehavioralError> {
        Ok(Self {
            board_id: receipt_string(receipt, "board_id")?,
            draft_id: receipt_string(receipt, "draft_id")?,
            revision: receipt
                .get("revision")
                .and_then(Value::as_u64)
                .ok_or_else(|| malformed("Missing unsigned revision"))?,
            source_fingerprint: receipt_string(receipt, "source_fingerprint")?,
            base_fingerprint: receipt_string(receipt, "base_fingerprint")?,
            catalog_fingerprint: receipt_string(receipt, "catalog_fingerprint")?,
            commands_fingerprint: receipt_string(receipt, "commands_fingerprint")?,
        })
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum FlowScriptBehavioralOutcome {
    Passed,
    Failed,
    Blocked,
    Stale,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum FlowScriptBehavioralDecisionKind {
    NotConfigured,
    Ready,
    RepairRequired,
    Unverified,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct FlowScriptBehavioralDecision {
    pub status: FlowScriptBehavioralDecisionKind,
    pub passed_check_ids: Vec<String>,
    pub failed_check_ids: Vec<String>,
    pub blocked_check_ids: Vec<String>,
    pub pending_check_ids: Vec<String>,
    /// A previously observed failure remains an obligation until this revision passes its check.
    pub outstanding_failure_ids: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
struct BehavioralObservation {
    revision: FlowScriptBehavioralRevision,
    outcome: FlowScriptBehavioralOutcome,
    actual_json: Option<String>,
    actual_output_fingerprint: Option<String>,
    actual_truncated: bool,
    errors: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
struct BehavioralCheck {
    entry: String,
    payload_fingerprint: String,
    expected_output_fingerprint: String,
    #[serde(deserialize_with = "required_value")]
    expected_output: Value,
    #[serde(default)]
    input: Option<BehavioralInput>,
    outstanding_failure: bool,
    observation: Option<BehavioralObservation>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
struct BehavioralInput {
    payload: Option<Value>,
}

/// Expectations are registered by the host before observing a receipt. A changed user contract
/// starts a new state with a new context fingerprint; test calls cannot replace an expectation.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(try_from = "BehavioralStateSnapshot")]
pub struct FlowScriptBehavioralState {
    context_fingerprint: String,
    current_revision: Option<FlowScriptBehavioralRevision>,
    checks: BTreeMap<String, BehavioralCheck>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct BehavioralStateSnapshot {
    context_fingerprint: String,
    current_revision: Option<FlowScriptBehavioralRevision>,
    checks: BTreeMap<String, BehavioralCheck>,
}

impl TryFrom<BehavioralStateSnapshot> for FlowScriptBehavioralState {
    type Error = FlowScriptBehavioralError;

    fn try_from(snapshot: BehavioralStateSnapshot) -> Result<Self, Self::Error> {
        let mut state = Self::new(snapshot.context_fingerprint)?;
        if let Some(revision) = snapshot.current_revision {
            state.advance_revision(revision)?;
        }
        for (id, check) in snapshot.checks {
            let expected_id = state.register_check(
                &check.entry,
                &check.payload_fingerprint,
                &check.expected_output,
            )?;
            if id != expected_id
                || check.expected_output_fingerprint != expected_fingerprint(&check.expected_output)
            {
                return Err(malformed(
                    "Stored check fingerprints do not match its fixed expectation",
                ));
            }
            if let Some(input) = &check.input {
                if flowscript_behavioral_payload_fingerprint(&input.payload)
                    != check.payload_fingerprint
                    || serde_json::to_vec(&input.payload)
                        .expect("JSON serializes")
                        .len()
                        > MAX_EXPECTED_BYTES
                {
                    return Err(malformed(
                        "Stored input does not match its fingerprint or exceeds 16 KiB",
                    ));
                }
            }
            if let Some(observation) = &check.observation {
                observation.revision.validate()?;
                let expected_preview =
                    bounded_json(&check.expected_output, MAX_VALUE_PREVIEW_BYTES);
                if observation.outcome == FlowScriptBehavioralOutcome::Stale
                    || state
                        .current_revision
                        .as_ref()
                        .is_some_and(|current| current.board_id != observation.revision.board_id)
                    || observation
                        .actual_json
                        .as_ref()
                        .is_some_and(|actual| actual.len() > MAX_VALUE_PREVIEW_BYTES)
                    || observation.errors.len() > MAX_ERRORS
                    || observation
                        .errors
                        .iter()
                        .any(|error| error.len() > MAX_ERROR_BYTES)
                    || (observation.outcome == FlowScriptBehavioralOutcome::Failed
                        && !check.outstanding_failure)
                    || (observation.outcome == FlowScriptBehavioralOutcome::Passed
                        && check.outstanding_failure)
                    || (observation.outcome == FlowScriptBehavioralOutcome::Passed
                        && (observation.actual_output_fingerprint.as_ref()
                            != Some(&check.expected_output_fingerprint)
                            || observation.actual_json.as_ref() != Some(&expected_preview.0)
                            || observation.actual_truncated != expected_preview.1
                            || !observation.errors.is_empty()))
                {
                    return Err(malformed(
                        "Stored observation violates the behavioral state contract",
                    ));
                }
            }
            state.checks.insert(id, check);
        }
        Ok(state)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(tag = "code", content = "message", rename_all = "snake_case")]
pub enum FlowScriptBehavioralError {
    InvalidIdentity(String),
    InvalidRevision(String),
    ExpectationConflict(String),
    UnregisteredCheck(String),
    LimitExceeded(String),
    MalformedReceipt(String),
}

impl std::fmt::Display for FlowScriptBehavioralError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let message = match self {
            Self::InvalidIdentity(message)
            | Self::InvalidRevision(message)
            | Self::ExpectationConflict(message)
            | Self::UnregisteredCheck(message)
            | Self::LimitExceeded(message)
            | Self::MalformedReceipt(message) => message,
        };
        formatter.write_str(message)
    }
}

impl std::error::Error for FlowScriptBehavioralError {}

impl FlowScriptBehavioralState {
    pub fn new(context_fingerprint: String) -> Result<Self, FlowScriptBehavioralError> {
        validate_identity(&context_fingerprint)?;
        Ok(Self {
            context_fingerprint,
            current_revision: None,
            checks: BTreeMap::new(),
        })
    }

    pub fn current_revision(&self) -> Option<&FlowScriptBehavioralRevision> {
        self.current_revision.as_ref()
    }

    pub fn context_fingerprint(&self) -> &str {
        &self.context_fingerprint
    }

    /// A newly retained source has no checked runtime identity until the host captures it.
    /// Registered expectations and previously observed failure obligations survive this reset.
    pub fn invalidate_revision(&mut self) {
        self.current_revision = None;
    }

    /// Catalog or checked-command changes invalidate prior evidence even at the same source
    /// revision. Previously failed checks remain registered, but require a fresh observation.
    pub fn advance_revision(
        &mut self,
        revision: FlowScriptBehavioralRevision,
    ) -> Result<bool, FlowScriptBehavioralError> {
        revision.validate()?;
        if let Some(current) = &self.current_revision {
            if current.board_id != revision.board_id {
                return Err(FlowScriptBehavioralError::InvalidRevision(
                    "A behavioral context cannot switch boards".into(),
                ));
            }
            if current.draft_id == revision.draft_id
                && (revision.revision < current.revision
                    || (revision.revision == current.revision
                        && (revision.source_fingerprint != current.source_fingerprint
                            || revision.base_fingerprint != current.base_fingerprint)))
            {
                return Err(FlowScriptBehavioralError::InvalidRevision("A retained draft revision cannot move backwards or change its source or base identity".into()));
            }
            if current == &revision {
                return Ok(false);
            }
        }
        self.current_revision = Some(revision);
        Ok(true)
    }

    pub fn register_check(
        &mut self,
        entry: &str,
        payload_fingerprint: &str,
        expected_output: &Value,
    ) -> Result<String, FlowScriptBehavioralError> {
        validate_identity(entry)?;
        validate_identity(payload_fingerprint)?;
        if serde_json::to_vec(expected_output)
            .expect("JSON serializes")
            .len()
            > MAX_EXPECTED_BYTES
        {
            return Err(FlowScriptBehavioralError::LimitExceeded(
                "An expected output may contain at most 16 KiB of JSON".into(),
            ));
        }
        let expected_output_fingerprint = expected_fingerprint(expected_output);
        if let Some((id, existing)) = self.checks.iter().find(|(_, check)| {
            check.entry == entry && check.payload_fingerprint == payload_fingerprint
        }) {
            if existing.expected_output_fingerprint != expected_output_fingerprint {
                return Err(FlowScriptBehavioralError::ExpectationConflict(format!(
                    "The registered expectation for {entry} and this input cannot be replaced within the same context"
                )));
            }
            return Ok(id.clone());
        }
        if self.checks.len() >= MAX_CHECKS {
            return Err(FlowScriptBehavioralError::LimitExceeded(
                "A behavioral context may register at most 32 checks".into(),
            ));
        }
        let id = value_fingerprint(
            "flowpilot.behavioral-check/v1",
            &json!({
                "context": self.context_fingerprint, "entry": entry, "payload": payload_fingerprint,
                "expected": expected_output_fingerprint,
            }),
        );
        self.checks.insert(
            id.clone(),
            BehavioralCheck {
                entry: entry.into(),
                payload_fingerprint: payload_fingerprint.into(),
                expected_output_fingerprint,
                expected_output: expected_output.clone(),
                input: None,
                outstanding_failure: false,
                observation: None,
            },
        );
        Ok(id)
    }

    /// Retain a dispatched input for bounded repair feedback. Hosts that only have an input
    /// digest can use `register_check`; both paths enforce the same immutable expectation.
    pub fn register_check_with_payload(
        &mut self,
        entry: &str,
        payload: &Option<Value>,
        expected_output: &Value,
    ) -> Result<String, FlowScriptBehavioralError> {
        if serde_json::to_vec(payload).expect("JSON serializes").len() > MAX_EXPECTED_BYTES {
            return Err(FlowScriptBehavioralError::LimitExceeded(
                "A test input may contain at most 16 KiB of JSON".into(),
            ));
        }
        let fingerprint = flowscript_behavioral_payload_fingerprint(payload);
        let id = self.register_check(entry, &fingerprint, expected_output)?;
        self.checks.get_mut(&id).expect("registered check").input = Some(BehavioralInput {
            payload: payload.clone(),
        });
        Ok(id)
    }

    /// Consume the existing `test_flowscript` receipt shape without trusting its `passed` flag
    /// alone. The context comes from the host session, never from model tool arguments.
    pub fn observe_test_receipt(
        &mut self,
        context_fingerprint: &str,
        receipt: &Value,
    ) -> Result<FlowScriptBehavioralOutcome, FlowScriptBehavioralError> {
        if context_fingerprint != self.context_fingerprint {
            return Ok(FlowScriptBehavioralOutcome::Stale);
        }
        if receipt.get("schema").and_then(Value::as_str)
            != Some("flowpilot.flowscript-draft-test/v1")
            || receipt.get("certification").and_then(Value::as_str) != Some("draft_output_only")
            || receipt.get("applied") != Some(&Value::Bool(false))
        {
            return Err(malformed(
                "Receipt does not identify an isolated retained-draft test",
            ));
        }
        let revision = FlowScriptBehavioralRevision::from_receipt(receipt)?;
        revision.validate()?;
        if self.current_revision.as_ref() != Some(&revision)
            || receipt.get("status").and_then(Value::as_str) == Some("stale")
        {
            return Ok(FlowScriptBehavioralOutcome::Stale);
        }
        let entry = receipt_string(receipt, "entry")?;
        let payload = receipt_string(receipt, "payload_fingerprint")?;
        let expected = receipt
            .get("expected_output")
            .ok_or_else(|| malformed("Receipt omitted expected_output"))?;
        let check = self
            .checks
            .values_mut()
            .find(|check| check.entry == entry && check.payload_fingerprint == payload)
            .ok_or_else(|| {
                FlowScriptBehavioralError::UnregisteredCheck(format!(
                    "No host check is registered for {entry} and this input"
                ))
            })?;
        if check.expected_output_fingerprint != expected_fingerprint(expected) {
            return Err(FlowScriptBehavioralError::ExpectationConflict(format!(
                "Test expectation differs from the registered {entry} check"
            )));
        }
        let runtime = receipt.get("runtime");
        let outputs = runtime
            .and_then(|runtime| runtime.get("outputs"))
            .and_then(Value::as_array);
        let actual = outputs
            .filter(|outputs| outputs.len() == 1)
            .map(|outputs| &outputs[0]);
        let error_free = runtime
            .and_then(|runtime| runtime.get("errors"))
            .and_then(Value::as_array)
            .is_some_and(Vec::is_empty);
        let runtime_success = runtime
            .and_then(|runtime| runtime.get("status"))
            .and_then(Value::as_str)
            == Some("success");
        let outcome = match receipt.get("status").and_then(Value::as_str) {
            Some("blocked") => FlowScriptBehavioralOutcome::Blocked,
            Some("success")
                if receipt.get("passed") == Some(&Value::Bool(true))
                    && runtime_success
                    && error_free
                    && actual == Some(&check.expected_output)
                    && runtime.and_then(|runtime| runtime.get("output")) == actual =>
            {
                FlowScriptBehavioralOutcome::Passed
            }
            Some("success" | "failed") => FlowScriptBehavioralOutcome::Failed,
            _ => return Err(malformed("Unknown draft test status")),
        };
        let (actual_json, actual_truncated) = actual
            .map(|value| bounded_json(value, MAX_VALUE_PREVIEW_BYTES))
            .map_or((None, false), |(json, truncated)| (Some(json), truncated));
        let mut errors = runtime
            .and_then(|runtime| runtime.get("errors"))
            .and_then(Value::as_array)
            .into_iter()
            .flatten()
            .take(MAX_ERRORS)
            .map(|error| {
                bounded_text(
                    error
                        .as_str()
                        .map(str::to_owned)
                        .unwrap_or_else(|| error.to_string()),
                    MAX_ERROR_BYTES,
                )
                .0
            })
            .collect::<Vec<_>>();
        if outcome != FlowScriptBehavioralOutcome::Passed && errors.is_empty() {
            let message = if outcome == FlowScriptBehavioralOutcome::Blocked {
                receipt
                    .get("message")
                    .and_then(Value::as_str)
                    .unwrap_or("The runtime could not execute this check")
            } else {
                "The runtime did not return one error-free result matching the registered expectation"
            };
            errors.push(bounded_text(message.into(), MAX_ERROR_BYTES).0);
        }
        match outcome {
            FlowScriptBehavioralOutcome::Passed => check.outstanding_failure = false,
            FlowScriptBehavioralOutcome::Failed => check.outstanding_failure = true,
            _ => {}
        }
        check.observation = Some(BehavioralObservation {
            revision,
            outcome,
            actual_json,
            actual_output_fingerprint: actual.map(expected_fingerprint),
            actual_truncated,
            errors,
        });
        Ok(outcome)
    }

    pub fn decision(&self) -> FlowScriptBehavioralDecision {
        let mut decision = FlowScriptBehavioralDecision {
            status: FlowScriptBehavioralDecisionKind::NotConfigured,
            passed_check_ids: Vec::new(),
            failed_check_ids: Vec::new(),
            blocked_check_ids: Vec::new(),
            pending_check_ids: Vec::new(),
            outstanding_failure_ids: Vec::new(),
        };
        for (id, check) in &self.checks {
            if check.outstanding_failure {
                decision.outstanding_failure_ids.push(id.clone());
            }
            match check
                .observation
                .as_ref()
                .filter(|observation| Some(&observation.revision) == self.current_revision.as_ref())
                .map(|observation| observation.outcome)
            {
                Some(FlowScriptBehavioralOutcome::Passed) => {
                    decision.passed_check_ids.push(id.clone())
                }
                Some(FlowScriptBehavioralOutcome::Failed) => {
                    decision.failed_check_ids.push(id.clone())
                }
                Some(FlowScriptBehavioralOutcome::Blocked) => {
                    decision.blocked_check_ids.push(id.clone())
                }
                _ => decision.pending_check_ids.push(id.clone()),
            }
        }
        decision.status = if self.checks.is_empty() {
            FlowScriptBehavioralDecisionKind::NotConfigured
        } else if !decision.failed_check_ids.is_empty() {
            FlowScriptBehavioralDecisionKind::RepairRequired
        } else if decision.passed_check_ids.len() == self.checks.len() {
            FlowScriptBehavioralDecisionKind::Ready
        } else {
            FlowScriptBehavioralDecisionKind::Unverified
        };
        decision
    }

    /// A separate bounded channel for runtime feedback. Compiler diagnostics remain untouched.
    pub fn feedback(&self) -> FlowScriptBehavioralFeedback {
        let pending = self
            .checks
            .iter()
            .filter(|(_, check)| {
                !check.observation.as_ref().is_some_and(|observation| {
                    observation.outcome == FlowScriptBehavioralOutcome::Passed
                        && Some(&observation.revision) == self.current_revision.as_ref()
                })
            })
            .collect::<Vec<_>>();
        let checks = pending
            .iter()
            .take(MAX_FEEDBACK_CHECKS)
            .map(|(id, check)| {
                let (expected_json, expected_truncated) =
                    bounded_json(&check.expected_output, MAX_VALUE_PREVIEW_BYTES);
                let observation = check.observation.as_ref();
                let input = check.input.as_ref().map(|input| {
                    bounded_json(
                        input.payload.as_ref().unwrap_or(&Value::Null),
                        MAX_VALUE_PREVIEW_BYTES,
                    )
                });
                let evidence_current = observation.is_some_and(|observation| {
                    Some(&observation.revision) == self.current_revision.as_ref()
                });
                FlowScriptBehavioralFeedbackCheck {
                    check_id: (*id).clone(),
                    entry: check.entry.clone(),
                    payload_fingerprint: check.payload_fingerprint.clone(),
                    expected_output_fingerprint: check.expected_output_fingerprint.clone(),
                    expected_json,
                    expected_truncated,
                    input_json: input.as_ref().map(|(json, _)| json.clone()),
                    input_truncated: input.is_some_and(|(_, truncated)| truncated),
                    evidence_current,
                    observed_revision: observation.map(|observation| observation.revision.revision),
                    outcome: observation.map(|observation| observation.outcome),
                    actual_json: observation
                        .and_then(|observation| observation.actual_json.clone()),
                    actual_truncated: observation
                        .is_some_and(|observation| observation.actual_truncated),
                    errors: observation
                        .map(|observation| observation.errors.clone())
                        .unwrap_or_default(),
                    outstanding_failure: check.outstanding_failure,
                }
            })
            .collect();
        FlowScriptBehavioralFeedback {
            context_fingerprint: self.context_fingerprint.clone(),
            current_revision: self.current_revision.clone(),
            decision: self.decision(),
            checks,
            omitted_checks: pending.len().saturating_sub(MAX_FEEDBACK_CHECKS),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct FlowScriptBehavioralFeedbackCheck {
    pub check_id: String,
    pub entry: String,
    pub payload_fingerprint: String,
    pub expected_output_fingerprint: String,
    pub expected_json: String,
    pub expected_truncated: bool,
    pub input_json: Option<String>,
    pub input_truncated: bool,
    pub actual_json: Option<String>,
    pub actual_truncated: bool,
    pub outcome: Option<FlowScriptBehavioralOutcome>,
    pub observed_revision: Option<u64>,
    pub evidence_current: bool,
    pub outstanding_failure: bool,
    pub errors: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct FlowScriptBehavioralFeedback {
    pub context_fingerprint: String,
    pub current_revision: Option<FlowScriptBehavioralRevision>,
    pub decision: FlowScriptBehavioralDecision,
    pub checks: Vec<FlowScriptBehavioralFeedbackCheck>,
    pub omitted_checks: usize,
}

fn validate_identity(value: &str) -> Result<(), FlowScriptBehavioralError> {
    if value.trim().is_empty() || value.len() > 256 {
        return Err(FlowScriptBehavioralError::InvalidIdentity(
            "Identity fields must contain 1 to 256 bytes".into(),
        ));
    }
    Ok(())
}

fn malformed(message: &str) -> FlowScriptBehavioralError {
    FlowScriptBehavioralError::MalformedReceipt(message.into())
}

fn receipt_string(receipt: &Value, name: &str) -> Result<String, FlowScriptBehavioralError> {
    let value = receipt
        .get(name)
        .and_then(Value::as_str)
        .ok_or_else(|| malformed(&format!("Receipt omitted {name}")))?;
    validate_identity(value)?;
    Ok(value.into())
}

fn expected_fingerprint(value: &Value) -> String {
    value_fingerprint("flowpilot.behavioral-expected/v1", value)
}

/// Matches the existing `test_flowscript` receipt protocol. The tool's optional payload
/// represents an omitted input and explicit JSON null with the same serialized value.
pub fn flowscript_behavioral_payload_fingerprint(payload: &Option<Value>) -> String {
    blake3::hash(&serde_json::to_vec(payload).expect("JSON serializes"))
        .to_hex()
        .to_string()
}

fn required_value<'de, D: serde::Deserializer<'de>>(deserializer: D) -> Result<Value, D::Error> {
    Value::deserialize(deserializer)
}

fn value_fingerprint(domain: &str, value: &Value) -> String {
    let mut hash = blake3::Hasher::new();
    hash.update(domain.as_bytes());
    hash.update(b"\n");
    hash.update(&serde_json::to_vec(&canonical_json(value)).expect("JSON serializes"));
    hash.finalize().to_hex().to_string()
}

fn canonical_json(value: &Value) -> Value {
    match value {
        Value::Object(object) => Value::Object(
            object
                .iter()
                .collect::<BTreeMap<_, _>>()
                .into_iter()
                .map(|(key, value)| (key.clone(), canonical_json(value)))
                .collect(),
        ),
        Value::Array(array) => Value::Array(array.iter().map(canonical_json).collect()),
        value => value.clone(),
    }
}

fn bounded_json(value: &Value, limit: usize) -> (String, bool) {
    bounded_text(canonical_json(value).to_string(), limit)
}

fn bounded_text(mut value: String, limit: usize) -> (String, bool) {
    if value.len() <= limit {
        return (value, false);
    }
    let mut end = limit;
    while !value.is_char_boundary(end) {
        end -= 1;
    }
    value.truncate(end);
    (value, true)
}

#[cfg(test)]
mod tests {
    use super::*;

    const CONTEXT: &str = "request-contract-v1";

    fn revision(number: u64) -> FlowScriptBehavioralRevision {
        FlowScriptBehavioralRevision {
            board_id: "board".into(),
            draft_id: "draft".into(),
            revision: number,
            source_fingerprint: format!("source-{number}"),
            base_fingerprint: "base".into(),
            catalog_fingerprint: "catalog".into(),
            commands_fingerprint: format!("commands-{number}"),
        }
    }

    fn state(expected: &Value) -> (FlowScriptBehavioralState, String) {
        let mut state = FlowScriptBehavioralState::new(CONTEXT.into()).unwrap();
        state.advance_revision(revision(0)).unwrap();
        let check = state
            .register_check("normalize", "input-a", expected)
            .unwrap();
        (state, check)
    }

    fn receipt(revision: &FlowScriptBehavioralRevision, expected: &Value, actual: &Value) -> Value {
        let mut receipt = serde_json::to_value(revision).unwrap();
        receipt.as_object_mut().unwrap().extend(json!({
            "schema": "flowpilot.flowscript-draft-test/v1",
            "certification": "draft_output_only", "applied": false,
            "entry": "normalize", "payload_fingerprint": "input-a",
            "expected_output": expected,
            "status": if actual == expected { "success" } else { "failed" },
            "passed": actual == expected,
            "runtime": {"status": "success", "outputs": [actual], "output": actual, "errors": []},
        }).as_object().unwrap().clone());
        receipt
    }

    #[test]
    fn repairs_require_the_same_check_to_pass_the_current_revision() {
        let expected = json!({"name": "ada"});
        let (mut state, check) = state(&expected);
        let failed = receipt(&revision(0), &expected, &json!({"name": "ADA"}));
        assert_eq!(
            state.observe_test_receipt(CONTEXT, &failed).unwrap(),
            FlowScriptBehavioralOutcome::Failed
        );
        assert_eq!(
            state.decision().status,
            FlowScriptBehavioralDecisionKind::RepairRequired
        );
        assert_eq!(
            state.decision().outstanding_failure_ids,
            vec![check.clone()]
        );

        state.advance_revision(revision(1)).unwrap();
        let after_edit = state.decision();
        assert_eq!(
            after_edit.status,
            FlowScriptBehavioralDecisionKind::Unverified
        );
        assert_eq!(after_edit.pending_check_ids, vec![check.clone()]);
        assert_eq!(after_edit.outstanding_failure_ids, vec![check.clone()]);
        assert!(after_edit.failed_check_ids.is_empty());
        let old_feedback = state.feedback();
        assert!(!old_feedback.checks[0].evidence_current);
        assert_eq!(old_feedback.checks[0].observed_revision, Some(0));

        let old_pass = receipt(&revision(0), &expected, &expected);
        assert_eq!(
            state.observe_test_receipt(CONTEXT, &old_pass).unwrap(),
            FlowScriptBehavioralOutcome::Stale
        );
        assert_eq!(state.decision(), after_edit);
        assert_eq!(
            state
                .observe_test_receipt(CONTEXT, &receipt(&revision(1), &expected, &expected))
                .unwrap(),
            FlowScriptBehavioralOutcome::Passed
        );
        assert_eq!(
            state.decision().status,
            FlowScriptBehavioralDecisionKind::Ready
        );
        assert!(state.decision().outstanding_failure_ids.is_empty());
        assert!(state.feedback().checks.is_empty());

        let ready = state.clone();
        assert_eq!(
            state.observe_test_receipt(CONTEXT, &failed).unwrap(),
            FlowScriptBehavioralOutcome::Stale
        );
        assert_eq!(
            state, ready,
            "late failures must not create a new obligation"
        );
    }

    #[test]
    fn expectations_cannot_be_weakened_or_replaced_by_a_different_input() {
        let expected = json!(3);
        let (mut state, original) = state(&expected);
        state
            .observe_test_receipt(CONTEXT, &receipt(&revision(0), &expected, &json!(2)))
            .unwrap();
        assert!(matches!(
            state.register_check("normalize", "input-a", &json!(2)),
            Err(FlowScriptBehavioralError::ExpectationConflict(_))
        ));
        assert!(matches!(
            state.observe_test_receipt(CONTEXT, &receipt(&revision(0), &json!(2), &json!(2))),
            Err(FlowScriptBehavioralError::ExpectationConflict(_))
        ));
        assert_eq!(
            state
                .register_check("normalize", "input-a", &expected)
                .unwrap(),
            original
        );

        let other = state
            .register_check("normalize", "input-b", &json!(2))
            .unwrap();
        let mut other_pass = receipt(&revision(0), &json!(2), &json!(2));
        other_pass["payload_fingerprint"] = json!("input-b");
        state.observe_test_receipt(CONTEXT, &other_pass).unwrap();
        let decision = state.decision();
        assert_eq!(decision.passed_check_ids, vec![other]);
        assert_eq!(decision.failed_check_ids, vec![original.clone()]);
        assert_eq!(decision.outstanding_failure_ids, vec![original]);
        assert_eq!(
            decision.status,
            FlowScriptBehavioralDecisionKind::RepairRequired
        );
    }

    #[test]
    fn every_host_and_revision_identity_part_scopes_evidence() {
        let expected = Value::Null;
        let (mut state, _) = state(&expected);
        let valid = receipt(&revision(0), &expected, &expected);
        let before = state.clone();
        assert_eq!(
            state
                .observe_test_receipt("another-contract", &valid)
                .unwrap(),
            FlowScriptBehavioralOutcome::Stale
        );
        for field in [
            "board_id",
            "draft_id",
            "source_fingerprint",
            "base_fingerprint",
            "catalog_fingerprint",
            "commands_fingerprint",
        ] {
            let mut changed = valid.clone();
            changed[field] = json!("different");
            assert_eq!(
                state.observe_test_receipt(CONTEXT, &changed).unwrap(),
                FlowScriptBehavioralOutcome::Stale,
                "{field}"
            );
            assert_eq!(state, before);
        }
        let mut changed = valid.clone();
        changed["revision"] = json!(1);
        assert_eq!(
            state.observe_test_receipt(CONTEXT, &changed).unwrap(),
            FlowScriptBehavioralOutcome::Stale
        );
        changed = valid.clone();
        changed["status"] = json!("stale");
        assert_eq!(
            state.observe_test_receipt(CONTEXT, &changed).unwrap(),
            FlowScriptBehavioralOutcome::Stale
        );
        state.observe_test_receipt(CONTEXT, &valid).unwrap();
        assert!(!state.advance_revision(revision(0)).unwrap());

        let mut revised_catalog = revision(0);
        revised_catalog.catalog_fingerprint = "catalog-v2".into();
        assert!(state.advance_revision(revised_catalog.clone()).unwrap());
        assert_eq!(
            state.decision().status,
            FlowScriptBehavioralDecisionKind::Unverified
        );
        assert_eq!(
            state.observe_test_receipt(CONTEXT, &valid).unwrap(),
            FlowScriptBehavioralOutcome::Stale
        );
        state
            .observe_test_receipt(CONTEXT, &receipt(&revised_catalog, &expected, &expected))
            .unwrap();
        revised_catalog.commands_fingerprint = "new-checked-commands".into();
        state.advance_revision(revised_catalog).unwrap();
        assert_eq!(
            state.decision().status,
            FlowScriptBehavioralDecisionKind::Unverified
        );
    }

    #[test]
    fn retained_revision_cannot_silently_change_source_base_or_board() {
        let (mut state, _) = state(&json!(1));
        let before = state.clone();
        for field in ["source_fingerprint", "base_fingerprint", "board_id"] {
            let mut changed = serde_json::to_value(revision(0)).unwrap();
            changed[field] = json!("different");
            assert!(matches!(
                state.advance_revision(serde_json::from_value(changed).unwrap()),
                Err(FlowScriptBehavioralError::InvalidRevision(_))
            ));
            assert_eq!(state, before);
        }
        state.advance_revision(revision(1)).unwrap();
        assert!(matches!(
            state.advance_revision(revision(0)),
            Err(FlowScriptBehavioralError::InvalidRevision(_))
        ));
        let mut new_draft = revision(0);
        new_draft.draft_id = "replacement-draft".into();
        assert!(state.advance_revision(new_draft).unwrap());
    }

    #[test]
    fn receipt_pass_requires_one_exact_error_free_runtime_output_including_null() {
        let expected = Value::Null;
        for change in [
            json!({"errors": ["runtime failed"]}),
            json!({"outputs": []}),
            json!({"outputs": [null, null]}),
            json!({"output": "different"}),
            json!({"status": "failed"}),
            json!({"status": "timeout"}),
        ] {
            let (mut state, _) = state(&expected);
            let mut candidate = receipt(&revision(0), &expected, &expected);
            candidate["runtime"]
                .as_object_mut()
                .unwrap()
                .extend(change.as_object().unwrap().clone());
            candidate["message"] = json!("The test passed");
            assert_eq!(
                state.observe_test_receipt(CONTEXT, &candidate).unwrap(),
                FlowScriptBehavioralOutcome::Failed
            );
            assert_eq!(
                state.decision().status,
                FlowScriptBehavioralDecisionKind::RepairRequired
            );
            assert!(
                !state.feedback().checks[0]
                    .errors
                    .iter()
                    .any(|error| error == "The test passed")
            );
        }
        for missing in ["errors", "output", "outputs", "status"] {
            let (mut state, _) = state(&expected);
            let mut candidate = receipt(&revision(0), &expected, &expected);
            candidate["runtime"]
                .as_object_mut()
                .unwrap()
                .remove(missing);
            assert_eq!(
                state.observe_test_receipt(CONTEXT, &candidate).unwrap(),
                FlowScriptBehavioralOutcome::Failed,
                "{missing}"
            );
        }
        let (mut state, _) = state(&expected);
        let valid = receipt(&revision(0), &expected, &expected);
        let mut missing_expected = valid.clone();
        missing_expected
            .as_object_mut()
            .unwrap()
            .remove("expected_output");
        assert!(matches!(
            state.observe_test_receipt(CONTEXT, &missing_expected),
            Err(FlowScriptBehavioralError::MalformedReceipt(_))
        ));
        assert_eq!(
            state.observe_test_receipt(CONTEXT, &valid).unwrap(),
            FlowScriptBehavioralOutcome::Passed
        );
        assert_eq!(
            state.decision().status,
            FlowScriptBehavioralDecisionKind::Ready
        );
    }

    #[test]
    fn blocked_execution_is_unverified_and_keeps_an_existing_failure_obligation() {
        let expected = json!(1);
        let (mut state, check) = state(&expected);
        let mut blocked = receipt(&revision(0), &expected, &expected);
        blocked["status"] = json!("blocked");
        blocked["passed"] = json!(false);
        blocked["message"] = json!("Unsupported node: external service");
        blocked.as_object_mut().unwrap().remove("runtime");
        assert_eq!(
            state.observe_test_receipt(CONTEXT, &blocked).unwrap(),
            FlowScriptBehavioralOutcome::Blocked
        );
        assert_eq!(
            state.decision().status,
            FlowScriptBehavioralDecisionKind::Unverified
        );
        assert_eq!(state.decision().blocked_check_ids, vec![check.clone()]);
        assert!(state.decision().outstanding_failure_ids.is_empty());
        assert_eq!(
            state.feedback().checks[0].errors,
            vec!["Unsupported node: external service"]
        );

        state
            .observe_test_receipt(CONTEXT, &receipt(&revision(0), &expected, &json!(0)))
            .unwrap();
        state.advance_revision(revision(1)).unwrap();
        blocked["revision"] = json!(1);
        blocked["source_fingerprint"] = json!("source-1");
        blocked["commands_fingerprint"] = json!("commands-1");
        state.observe_test_receipt(CONTEXT, &blocked).unwrap();
        assert_eq!(
            state.decision().status,
            FlowScriptBehavioralDecisionKind::Unverified
        );
        assert_eq!(state.decision().outstanding_failure_ids, vec![check]);
    }

    #[test]
    fn runtime_feedback_bounds_unicode_previews_errors_and_check_count() {
        let expected = json!("é".repeat(3000));
        let (mut state, _) = state(&expected);
        let mut failed = receipt(&revision(0), &expected, &json!("あ".repeat(3000)));
        failed["runtime"]["errors"] = json!(vec!["あ".repeat(1000); 10]);
        state.observe_test_receipt(CONTEXT, &failed).unwrap();
        let feedback = state.feedback();
        let check = &feedback.checks[0];
        assert!(check.expected_truncated && check.actual_truncated);
        assert!(check.expected_json.len() <= MAX_VALUE_PREVIEW_BYTES);
        assert!(check.actual_json.as_ref().unwrap().len() <= MAX_VALUE_PREVIEW_BYTES);
        assert_eq!(check.errors.len(), MAX_ERRORS);
        assert!(
            check
                .errors
                .iter()
                .all(|error| error.len() <= MAX_ERROR_BYTES)
        );
        assert!(check.evidence_current);
        assert_eq!(check.outcome, Some(FlowScriptBehavioralOutcome::Failed));

        for index in 0..10 {
            state
                .register_check("normalize", &format!("other-{index}"), &json!(index))
                .unwrap();
        }
        let feedback = state.feedback();
        assert_eq!(feedback.checks.len(), MAX_FEEDBACK_CHECKS);
        assert_eq!(feedback.omitted_checks, 3);
        assert_eq!(feedback.decision.pending_check_ids.len(), 10);
    }

    #[test]
    fn persisted_state_preserves_obligations_and_rejects_corrupted_expectations() {
        let (mut state, _) = state(&Value::Null);
        state
            .observe_test_receipt(CONTEXT, &receipt(&revision(0), &Value::Null, &json!(1)))
            .unwrap();
        state.invalidate_revision();
        let unverified: FlowScriptBehavioralState =
            serde_json::from_value(serde_json::to_value(&state).unwrap()).unwrap();
        assert_eq!(
            unverified.decision().status,
            FlowScriptBehavioralDecisionKind::Unverified
        );
        assert_eq!(unverified.decision().outstanding_failure_ids.len(), 1);
        state.advance_revision(revision(1)).unwrap();
        let serialized = serde_json::to_value(&state).unwrap();
        let mut restored: FlowScriptBehavioralState =
            serde_json::from_value(serialized.clone()).unwrap();
        assert_eq!(restored, state);
        restored
            .observe_test_receipt(CONTEXT, &receipt(&revision(1), &Value::Null, &Value::Null))
            .unwrap();
        let ready: FlowScriptBehavioralState =
            serde_json::from_str(&serde_json::to_string(&restored).unwrap()).unwrap();
        assert_eq!(
            ready.decision().status,
            FlowScriptBehavioralDecisionKind::Ready
        );
        for remove in [false, true] {
            let mut corrupted = serialized.clone();
            let check = corrupted["checks"]
                .as_object_mut()
                .unwrap()
                .values_mut()
                .next()
                .unwrap();
            if remove {
                check.as_object_mut().unwrap().remove("expected_output");
            } else {
                check["expected_output"] = json!(1);
            }
            assert!(serde_json::from_value::<FlowScriptBehavioralState>(corrupted).is_err());
        }
        let mut corrupted = serialized;
        corrupted["context_fingerprint"] = json!("another-contract");
        assert!(serde_json::from_value::<FlowScriptBehavioralState>(corrupted).is_err());
    }

    #[test]
    fn retained_inputs_are_bounded_and_restored_only_with_their_exact_fingerprint() {
        let mut state = FlowScriptBehavioralState::new(CONTEXT.into()).unwrap();
        state.advance_revision(revision(0)).unwrap();
        let payload = Some(json!({"name": "あ".repeat(1000)}));
        let check_id = state
            .register_check_with_payload("normalize", &payload, &json!(1))
            .unwrap();
        let feedback = state.feedback();
        assert!(feedback.checks[0].input_truncated);
        assert!(feedback.checks[0].input_json.as_ref().unwrap().len() <= MAX_VALUE_PREVIEW_BYTES);
        let serialized = serde_json::to_value(&state).unwrap();
        assert_eq!(
            serde_json::from_value::<FlowScriptBehavioralState>(serialized.clone()).unwrap(),
            state
        );
        let mut corrupted = serialized;
        corrupted["checks"][&check_id]["input"]["payload"] = json!({"name": "different"});
        assert!(serde_json::from_value::<FlowScriptBehavioralState>(corrupted).is_err());
        assert!(matches!(
            state.register_check_with_payload(
                "normalize",
                &Some(json!("x".repeat(MAX_EXPECTED_BYTES))),
                &json!(1)
            ),
            Err(FlowScriptBehavioralError::LimitExceeded(_))
        ));

        let null_id = state
            .register_check_with_payload("normalize", &None, &Value::Null)
            .unwrap();
        assert_eq!(
            state
                .register_check_with_payload("normalize", &Some(Value::Null), &Value::Null)
                .unwrap(),
            null_id
        );
        let null_feedback = state
            .feedback()
            .checks
            .into_iter()
            .find(|check| check.check_id == null_id)
            .unwrap();
        assert_eq!(null_feedback.input_json.as_deref(), Some("null"));
        assert!(!null_feedback.input_truncated);
    }

    #[test]
    fn restored_passes_require_exact_complete_or_truncated_actual_evidence() {
        for expected in [Value::Null, json!({"z": "あ".repeat(1000), "a": 1})] {
            let (mut state, check_id) = state(&expected);
            state
                .observe_test_receipt(CONTEXT, &receipt(&revision(0), &expected, &expected))
                .unwrap();
            let serialized = serde_json::to_value(&state).unwrap();
            assert_eq!(
                serde_json::from_value::<FlowScriptBehavioralState>(serialized.clone())
                    .unwrap()
                    .decision()
                    .status,
                FlowScriptBehavioralDecisionKind::Ready
            );
            for (field, corrupted_value) in [
                ("actual_json", Value::Null),
                ("actual_json", json!("false")),
                ("actual_output_fingerprint", json!("another-result")),
                ("errors", json!(["runtime failure"])),
                (
                    "actual_truncated",
                    json!(
                        !state.checks[&check_id]
                            .observation
                            .as_ref()
                            .unwrap()
                            .actual_truncated
                    ),
                ),
            ] {
                let mut corrupted = serialized.clone();
                corrupted["checks"][&check_id]["observation"][field] = corrupted_value;
                assert!(
                    serde_json::from_value::<FlowScriptBehavioralState>(corrupted).is_err(),
                    "{field}"
                );
            }
        }
    }

    #[test]
    fn only_explicit_bounded_host_checks_can_create_evidence() {
        let mut state = FlowScriptBehavioralState::new(CONTEXT.into()).unwrap();
        assert_eq!(
            state.decision().status,
            FlowScriptBehavioralDecisionKind::NotConfigured
        );
        state.advance_revision(revision(0)).unwrap();
        assert!(matches!(
            state.observe_test_receipt(CONTEXT, &receipt(&revision(0), &Value::Null, &Value::Null)),
            Err(FlowScriptBehavioralError::UnregisteredCheck(_))
        ));
        for index in 0..MAX_CHECKS {
            state
                .register_check("normalize", &format!("input-{index}"), &json!(index))
                .unwrap();
        }
        assert_eq!(
            state.decision().status,
            FlowScriptBehavioralDecisionKind::Unverified
        );
        assert!(matches!(
            state.register_check("normalize", "overflow", &Value::Null),
            Err(FlowScriptBehavioralError::LimitExceeded(_))
        ));
        assert!(matches!(
            state.register_check(
                "normalize",
                "input-0",
                &json!("x".repeat(MAX_EXPECTED_BYTES))
            ),
            Err(FlowScriptBehavioralError::LimitExceeded(_))
        ));
        assert!(FlowScriptBehavioralState::new(" ".into()).is_err());

        let payload = Some(json!({"a": 1, "b": [2, 3]}));
        assert_eq!(
            flowscript_behavioral_payload_fingerprint(&payload),
            blake3::hash(&serde_json::to_vec(&payload).unwrap())
                .to_hex()
                .to_string()
        );
        assert_eq!(
            flowscript_behavioral_payload_fingerprint(&None),
            flowscript_behavioral_payload_fingerprint(&Some(Value::Null))
        );
    }
}
