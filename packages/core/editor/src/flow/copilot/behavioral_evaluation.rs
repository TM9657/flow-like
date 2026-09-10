//! Host-owned fixtures and portable evidence for isolated workflow behavior benchmarks.

use std::collections::{BTreeMap, BTreeSet};

use serde::{Deserialize, Deserializer, Serialize};
use serde_json::Value;

pub const WORKFLOW_BENCHMARK_REPORT_SCHEMA: &str = "flowpilot.workflow-behavioral-benchmark/v1";

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct WorkflowBenchmarkCheck {
    pub id: String,
    pub entry: String,
    #[serde(
        default,
        deserialize_with = "present_json_value",
        skip_serializing_if = "Option::is_none"
    )]
    pub payload: Option<Value>,
    #[serde(deserialize_with = "required_json_value")]
    pub expected_output: Value,
}

fn required_json_value<'de, D: Deserializer<'de>>(deserializer: D) -> Result<Value, D::Error> {
    Value::deserialize(deserializer)
}

fn present_json_value<'de, D: Deserializer<'de>>(
    deserializer: D,
) -> Result<Option<Value>, D::Error> {
    Value::deserialize(deserializer).map(Some)
}

/// The host retains reference source and checks. Only `public_prompt` reaches the model.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct WorkflowBenchmarkCase {
    pub id: String,
    pub prompt: String,
    pub initial_source: Option<String>,
    pub reference_source: String,
    pub required_functions: Vec<String>,
    pub checks: Vec<WorkflowBenchmarkCheck>,
}

impl WorkflowBenchmarkCase {
    pub fn public_prompt(&self) -> &str {
        &self.prompt
    }

    pub fn validate(&self) -> Result<(), String> {
        for (name, value) in [
            ("case id", self.id.as_str()),
            ("prompt", self.prompt.as_str()),
            ("reference source", self.reference_source.as_str()),
        ] {
            require_nonempty(name, value)?;
        }
        if self.checks.is_empty() {
            return Err("A benchmark case requires at least one host check".into());
        }
        unique_nonempty(
            "check id",
            self.checks.iter().map(|check| check.id.as_str()),
        )?;
        unique_nonempty(
            "required function",
            self.required_functions.iter().map(String::as_str),
        )?;
        for check in &self.checks {
            require_nonempty("check entry", &check.entry)?;
        }
        Ok(())
    }

    /// Includes hidden expectations, reference source, initial source and function requirements.
    pub fn fixture_fingerprint(&self) -> String {
        let value = serde_json::to_value(self).expect("benchmark fixtures serialize to JSON");
        let bytes = serde_json::to_vec(&canonical_json(value))
            .expect("canonical benchmark fixtures serialize to JSON");
        let mut hash = blake3::Hasher::new();
        hash.update(b"flowpilot.workflow-benchmark-fixture/v1\n");
        hash.update(&bytes);
        hash.finalize().to_hex().to_string()
    }
}

fn canonical_json(value: Value) -> Value {
    match value {
        Value::Object(object) => Value::Object(
            object
                .into_iter()
                .collect::<BTreeMap<_, _>>()
                .into_iter()
                .map(|(key, value)| (key, canonical_json(value)))
                .collect(),
        ),
        Value::Array(values) => Value::Array(values.into_iter().map(canonical_json).collect()),
        value => value,
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct WorkflowBenchmarkCohort {
    pub provider: String,
    pub model: String,
    pub reasoning_effort: String,
    #[serde(default = "legacy_prompt_profile")]
    pub prompt_profile: String,
    pub harness_revision: String,
    pub fixture_fingerprint: String,
    pub catalog_fingerprint: String,
    pub prompt_fingerprint: String,
    pub tool_schema_fingerprint: String,
    pub budget_fingerprint: String,
}

fn legacy_prompt_profile() -> String {
    "legacy".into()
}

impl WorkflowBenchmarkCohort {
    pub fn validate(&self) -> Result<(), String> {
        for (name, value) in [
            ("provider", &self.provider),
            ("model", &self.model),
            ("reasoning effort", &self.reasoning_effort),
            ("prompt profile", &self.prompt_profile),
            ("harness revision", &self.harness_revision),
            ("fixture fingerprint", &self.fixture_fingerprint),
            ("catalog fingerprint", &self.catalog_fingerprint),
            ("prompt fingerprint", &self.prompt_fingerprint),
            ("tool schema fingerprint", &self.tool_schema_fingerprint),
            ("budget fingerprint", &self.budget_fingerprint),
        ] {
            require_nonempty(name, value)?;
        }
        Ok(())
    }

    /// Aggregate only identical model, harness, fixture and budget configurations.
    pub fn ensure_same_cohort(&self, other: &Self) -> Result<(), String> {
        self.validate()?;
        other.validate()?;
        if self != other {
            return Err("Benchmark reports belong to different model or harness cohorts".into());
        }
        Ok(())
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct WorkflowBenchmarkAttempt {
    pub attempt_index: u32,
    pub draft_id: String,
    pub revision: u64,
    pub source_fingerprint: String,
    /// Time from run start to this source submission's compilation result.
    pub elapsed_ms: u64,
    pub parse_valid: bool,
    pub typed_valid: bool,
    pub reconcile_valid: bool,
    pub diagnostic_keys: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct WorkflowBenchmarkGradedCandidate {
    pub attempt_index: u32,
    pub draft_id: String,
    pub revision: u64,
    pub source_fingerprint: String,
    pub base_fingerprint: String,
    pub catalog_fingerprint: String,
    pub commands_fingerprint: String,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum WorkflowBenchmarkCheckStatus {
    Passed,
    Failed,
    Blocked,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct WorkflowBenchmarkCheckResult {
    pub check_id: String,
    pub status: WorkflowBenchmarkCheckStatus,
    /// `Some(Null)` is an observed null result; `None` means no single result was available.
    #[serde(
        default,
        deserialize_with = "present_json_value",
        skip_serializing_if = "Option::is_none"
    )]
    pub actual_output: Option<Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub message: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct WorkflowBenchmarkObservation {
    pub check_id: String,
    pub runtime: Value,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct WorkflowBenchmarkGrade {
    pub passed: bool,
    pub checks: Vec<WorkflowBenchmarkCheckResult>,
    pub errors: Vec<String>,
}

/// Grade raw host runtime results against the complete fixed fixture set.
pub fn grade_workflow_benchmark_checks(
    case: &WorkflowBenchmarkCase,
    observations: &[WorkflowBenchmarkObservation],
) -> Result<WorkflowBenchmarkGrade, String> {
    case.validate()?;
    let expected_ids = case
        .checks
        .iter()
        .map(|check| check.id.as_str())
        .collect::<BTreeSet<_>>();
    let mut by_id: BTreeMap<&str, Vec<&Value>> = BTreeMap::new();
    let mut errors = Vec::new();
    for observation in observations {
        if !expected_ids.contains(observation.check_id.as_str()) {
            errors.push(format!("Unexpected check result: {}", observation.check_id));
        }
        by_id
            .entry(&observation.check_id)
            .or_default()
            .push(&observation.runtime);
    }
    let checks = case.checks.iter().map(|check| {
        let values = by_id.get(check.id.as_str());
        let runtime = match values.map(Vec::as_slice) {
            Some([runtime]) => *runtime,
            values => {
                let message = match values {
                    None => format!("Missing check result: {}", check.id),
                    _ => format!("Duplicate check results: {}", check.id),
                };
                errors.push(message.clone());
                return WorkflowBenchmarkCheckResult {
                    check_id: check.id.clone(), status: WorkflowBenchmarkCheckStatus::Blocked,
                    actual_output: None, message: Some(message),
                };
            }
        };
        let status = runtime.get("status").and_then(Value::as_str);
        let outputs = runtime.get("outputs").and_then(Value::as_array);
        let actual = outputs.filter(|outputs| outputs.len() == 1).map(|outputs| outputs[0].clone());
        let errors_clear = runtime.get("errors").is_some_and(|errors| {
            errors.as_array().is_some_and(Vec::is_empty)
        });
        let passed = status == Some("success") && errors_clear
            && actual.as_ref() == Some(&check.expected_output)
            && runtime.get("output") == actual.as_ref();
        WorkflowBenchmarkCheckResult {
            check_id: check.id.clone(),
            status: if passed { WorkflowBenchmarkCheckStatus::Passed }
                else if matches!(status, Some("blocked" | "unavailable" | "stale")) {
                    WorkflowBenchmarkCheckStatus::Blocked
                } else { WorkflowBenchmarkCheckStatus::Failed },
            actual_output: actual,
            message: (!passed).then(|| {
                let detail = runtime.get("errors").and_then(Value::as_array)
                    .into_iter().flatten().take(4).filter_map(Value::as_str)
                    .map(|error| error.chars().take(512).collect::<String>()).collect::<Vec<_>>().join("; ");
                if detail.is_empty() {
                    "Runtime evidence did not contain one successful, error-free result matching the host expectation".into()
                } else {
                    format!("Runtime {}: {detail}", status.unwrap_or("unknown"))
                }
            }),
        }
    }).collect::<Vec<_>>();
    Ok(WorkflowBenchmarkGrade {
        passed: errors.is_empty()
            && checks
                .iter()
                .all(|check| check.status == WorkflowBenchmarkCheckStatus::Passed),
        checks,
        errors,
    })
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum WorkflowBenchmarkRunStatus {
    Succeeded,
    Failed,
    Cancelled,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct WorkflowBenchmarkUsage {
    pub input_tokens: Option<u64>,
    pub output_tokens: Option<u64>,
    pub cached_input_tokens: Option<u64>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct WorkflowBenchmarkToolCall {
    pub index: u32,
    pub tool: String,
    pub started_ms: u64,
    pub duration_ms: Option<u64>,
    pub status: String,
    pub diagnostic_codes: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct WorkflowBenchmarkTransportEvent {
    pub elapsed_ms: u64,
    pub transport: String,
    pub tool: String,
    pub phase: String,
    pub code: Option<String>,
    pub status: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct WorkflowBenchmarkRunReport {
    pub schema: String,
    pub run_id: String,
    pub case_id: String,
    pub repeat_index: u32,
    pub cohort: WorkflowBenchmarkCohort,
    /// Actual advertised schemas when provider setup reached the tool surface. Setup failures
    /// still belong to the planned cohort and remain in its denominator.
    #[serde(default)]
    pub observed_tool_schema_fingerprint: Option<String>,
    #[serde(default)]
    pub tool_calls: Vec<WorkflowBenchmarkToolCall>,
    #[serde(default)]
    pub tool_calls_truncated: bool,
    #[serde(default)]
    pub transport_events: Vec<WorkflowBenchmarkTransportEvent>,
    #[serde(default)]
    pub transport_events_truncated: bool,
    #[serde(default)]
    pub transport_counts: BTreeMap<String, u64>,
    #[serde(default)]
    pub external_phases: Vec<Value>,
    #[serde(default)]
    pub external_phases_truncated: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub retained_source: Option<String>,
    pub status: WorkflowBenchmarkRunStatus,
    pub elapsed_ms: u64,
    pub usage: Option<WorkflowBenchmarkUsage>,
    pub attempts: Vec<WorkflowBenchmarkAttempt>,
    pub graded_candidate: Option<WorkflowBenchmarkGradedCandidate>,
    pub checks: Vec<WorkflowBenchmarkCheckResult>,
    /// Includes host requirements outside output checks, such as required function definitions.
    pub grading_errors: Vec<String>,
}

pub fn validate_workflow_benchmark_report(
    case: &WorkflowBenchmarkCase,
    report: &WorkflowBenchmarkRunReport,
) -> Result<(), String> {
    case.validate()?;
    report.cohort.validate()?;
    require_nonempty("run id", &report.run_id)?;
    if report.schema != WORKFLOW_BENCHMARK_REPORT_SCHEMA
        || report.case_id != case.id
        || report.cohort.fixture_fingerprint != case.fixture_fingerprint()
    {
        return Err("Report schema or host fixture identity does not match".into());
    }
    let mut revisions = BTreeSet::new();
    let mut elapsed = 0;
    for (index, attempt) in report.attempts.iter().enumerate() {
        require_nonempty("draft id", &attempt.draft_id)?;
        require_nonempty("source fingerprint", &attempt.source_fingerprint)?;
        if attempt.attempt_index as usize != index + 1
            || attempt.elapsed_ms < elapsed
            || attempt.elapsed_ms > report.elapsed_ms
            || (attempt.typed_valid && !attempt.parse_valid)
            || (attempt.reconcile_valid && !attempt.typed_valid)
            || !revisions.insert((&attempt.draft_id, attempt.revision))
        {
            return Err("Source attempts require ordered indices, times and distinct draft revisions with consistent validation".into());
        }
        elapsed = attempt.elapsed_ms;
    }
    if let Some(candidate) = &report.graded_candidate {
        let attempt = report
            .attempts
            .iter()
            .find(|attempt| attempt.attempt_index == candidate.attempt_index)
            .ok_or("Graded candidate does not identify a recorded source attempt")?;
        if !attempt.reconcile_valid
            || candidate.draft_id != attempt.draft_id
            || candidate.revision != attempt.revision
            || candidate.source_fingerprint != attempt.source_fingerprint
            || candidate.catalog_fingerprint != report.cohort.catalog_fingerprint
        {
            return Err("Graded candidate identity or validation does not match its source attempt and catalog".into());
        }
        require_nonempty("base fingerprint", &candidate.base_fingerprint)?;
        require_nonempty("commands fingerprint", &candidate.commands_fingerprint)?;
        unique_nonempty(
            "graded check id",
            report.checks.iter().map(|check| check.check_id.as_str()),
        )?;
        if report.checks.len() != case.checks.len() {
            return Err("Graded candidate must account for every host check".into());
        }
        for result in &report.checks {
            let expected = case
                .checks
                .iter()
                .find(|check| check.id == result.check_id)
                .ok_or("Report contains an undeclared check")?;
            if result.status == WorkflowBenchmarkCheckStatus::Passed
                && result.actual_output.as_ref() != Some(&expected.expected_output)
            {
                return Err("Passing check does not match the host expectation".into());
            }
        }
    } else if !report.checks.is_empty() || report.status == WorkflowBenchmarkRunStatus::Succeeded {
        return Err("Behavioral evidence requires an exact graded candidate".into());
    }
    if report.status == WorkflowBenchmarkRunStatus::Succeeded
        && (!report.grading_errors.is_empty()
            || report
                .checks
                .iter()
                .any(|check| check.status != WorkflowBenchmarkCheckStatus::Passed))
    {
        return Err(
            "A successful benchmark report requires all host checks and requirements to pass"
                .into(),
        );
    }
    Ok(())
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct WorkflowBenchmarkRate {
    pub numerator: u64,
    pub denominator: u64,
    pub rate: Option<f64>,
}

impl WorkflowBenchmarkRate {
    fn new(numerator: u64, denominator: u64) -> Self {
        Self {
            numerator,
            denominator,
            rate: (denominator > 0).then(|| numerator as f64 / denominator as f64),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct WorkflowBenchmarkMeasuredMetric {
    pub samples: u64,
    pub total: u64,
    pub mean: Option<f64>,
}

impl WorkflowBenchmarkMeasuredMetric {
    fn from_values(values: impl Iterator<Item = u64>) -> Result<Self, String> {
        let mut samples = 0_u64;
        let mut total = 0_u64;
        for value in values {
            samples += 1;
            total = total
                .checked_add(value)
                .ok_or("Benchmark resource total overflow")?;
        }
        Ok(Self {
            samples,
            total,
            mean: (samples > 0).then(|| total as f64 / samples as f64),
        })
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct WorkflowBenchmarkScorecard {
    pub schema: String,
    pub cohort: Option<WorkflowBenchmarkCohort>,
    pub runs_total: u64,
    pub source_attempts: u64,
    pub repair_attempts: u64,
    pub final_behavior_pass: WorkflowBenchmarkRate,
    /// Verified successes of the graded candidate only. Untested earlier candidates add no wins.
    pub verified_behavior_at_1: WorkflowBenchmarkRate,
    pub verified_behavior_within_3: WorkflowBenchmarkRate,
    pub elapsed_ms: WorkflowBenchmarkMeasuredMetric,
    pub time_to_verified_behavior_ms: WorkflowBenchmarkMeasuredMetric,
    pub input_tokens: WorkflowBenchmarkMeasuredMetric,
    pub output_tokens: WorkflowBenchmarkMeasuredMetric,
    pub cached_input_tokens: WorkflowBenchmarkMeasuredMetric,
}

pub fn evaluate_workflow_benchmark_runs(
    case: &WorkflowBenchmarkCase,
    reports: &[WorkflowBenchmarkRunReport],
) -> Result<WorkflowBenchmarkScorecard, String> {
    case.validate()?;
    unique_nonempty(
        "run id",
        reports.iter().map(|report| report.run_id.as_str()),
    )?;
    let mut repeats = BTreeSet::new();
    for report in reports {
        validate_workflow_benchmark_report(case, report)?;
        if !repeats.insert(report.repeat_index) {
            return Err("A cohort cannot count the same case repetition twice".into());
        }
        if let Some(first) = reports.first() {
            first.cohort.ensure_same_cohort(&report.cohort)?;
        }
    }
    let observed_schemas = reports
        .iter()
        .filter_map(|report| report.observed_tool_schema_fingerprint.as_deref())
        .collect::<BTreeSet<_>>();
    if observed_schemas.len() > 1 {
        return Err("A cohort cannot mix different observed tool surfaces".into());
    }
    let successful = reports
        .iter()
        .filter(|report| report.status == WorkflowBenchmarkRunStatus::Succeeded)
        .collect::<Vec<_>>();
    let total = reports.len() as u64;
    let within = |limit| {
        successful
            .iter()
            .filter(|report| {
                report
                    .graded_candidate
                    .as_ref()
                    .is_some_and(|candidate| candidate.attempt_index <= limit)
            })
            .count() as u64
    };
    Ok(WorkflowBenchmarkScorecard {
        schema: WORKFLOW_BENCHMARK_REPORT_SCHEMA.into(),
        cohort: reports.first().map(|report| report.cohort.clone()),
        runs_total: total,
        source_attempts: reports
            .iter()
            .map(|report| report.attempts.len() as u64)
            .sum(),
        repair_attempts: reports
            .iter()
            .map(|report| report.attempts.len().saturating_sub(1) as u64)
            .sum(),
        final_behavior_pass: WorkflowBenchmarkRate::new(successful.len() as u64, total),
        verified_behavior_at_1: WorkflowBenchmarkRate::new(within(1), total),
        verified_behavior_within_3: WorkflowBenchmarkRate::new(within(3), total),
        elapsed_ms: WorkflowBenchmarkMeasuredMetric::from_values(
            reports.iter().map(|report| report.elapsed_ms),
        )?,
        time_to_verified_behavior_ms: WorkflowBenchmarkMeasuredMetric::from_values(
            successful.iter().map(|report| report.elapsed_ms),
        )?,
        input_tokens: WorkflowBenchmarkMeasuredMetric::from_values(
            reports
                .iter()
                .filter_map(|report| report.usage.as_ref()?.input_tokens),
        )?,
        output_tokens: WorkflowBenchmarkMeasuredMetric::from_values(
            reports
                .iter()
                .filter_map(|report| report.usage.as_ref()?.output_tokens),
        )?,
        cached_input_tokens: WorkflowBenchmarkMeasuredMetric::from_values(
            reports
                .iter()
                .filter_map(|report| report.usage.as_ref()?.cached_input_tokens),
        )?,
    })
}

fn require_nonempty(name: &str, value: &str) -> Result<(), String> {
    if value.trim().is_empty() {
        Err(format!("Benchmark {name} must not be empty"))
    } else {
        Ok(())
    }
}

fn unique_nonempty<'a>(name: &str, values: impl Iterator<Item = &'a str>) -> Result<(), String> {
    let mut seen = BTreeSet::new();
    for value in values {
        require_nonempty(name, value)?;
        if !seen.insert(value) {
            return Err(format!("Duplicate benchmark {name}: {value}"));
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn case() -> WorkflowBenchmarkCase {
        WorkflowBenchmarkCase {
            id: "transform-record".into(),
            prompt: "Return the input with its counter increased by one.".into(),
            initial_source: Some("eventsGeneric transform() {}".into()),
            reference_source: "hidden reference implementation".into(),
            required_functions: vec!["increment".into()],
            checks: vec![WorkflowBenchmarkCheck {
                id: "ordinary-record".into(),
                entry: "transform".into(),
                payload: Some(json!({"counter": 4, "name": "Ada"})),
                expected_output: json!({"counter": 5, "name": "Ada"}),
            }],
        }
    }

    fn runtime(output: Value) -> Value {
        json!({"status": "success", "output": output, "outputs": [output], "errors": []})
    }

    fn observation(case: &WorkflowBenchmarkCase) -> WorkflowBenchmarkObservation {
        WorkflowBenchmarkObservation {
            check_id: case.checks[0].id.clone(),
            runtime: runtime(case.checks[0].expected_output.clone()),
        }
    }

    fn report(
        case: &WorkflowBenchmarkCase,
        repeat: u32,
        attempt_count: u32,
    ) -> WorkflowBenchmarkRunReport {
        let attempts = (1..=attempt_count)
            .map(|index| WorkflowBenchmarkAttempt {
                attempt_index: index,
                draft_id: "draft".into(),
                revision: u64::from(index - 1),
                source_fingerprint: format!("source-{index}"),
                elapsed_ms: u64::from(index) * 100,
                parse_valid: true,
                typed_valid: true,
                reconcile_valid: true,
                diagnostic_keys: vec![],
            })
            .collect::<Vec<_>>();
        let candidate = attempts
            .last()
            .map(|attempt| WorkflowBenchmarkGradedCandidate {
                attempt_index: attempt.attempt_index,
                draft_id: attempt.draft_id.clone(),
                revision: attempt.revision,
                source_fingerprint: attempt.source_fingerprint.clone(),
                base_fingerprint: "empty-board".into(),
                catalog_fingerprint: "catalog".into(),
                commands_fingerprint: "checked-commands".into(),
            });
        let grade = grade_workflow_benchmark_checks(case, &[observation(case)]).unwrap();
        WorkflowBenchmarkRunReport {
            schema: WORKFLOW_BENCHMARK_REPORT_SCHEMA.into(),
            run_id: format!("run-{repeat}"),
            observed_tool_schema_fingerprint: None,
            tool_calls: Vec::new(),
            tool_calls_truncated: false,
            transport_events: Vec::new(),
            transport_events_truncated: false,
            transport_counts: BTreeMap::new(),
            external_phases: Vec::new(),
            external_phases_truncated: false,
            retained_source: None,
            case_id: case.id.clone(),
            repeat_index: repeat,
            cohort: WorkflowBenchmarkCohort {
                provider: "provider".into(),
                model: "pinned-model".into(),
                reasoning_effort: "high".into(),
                prompt_profile: "legacy".into(),
                harness_revision: "revision".into(),
                fixture_fingerprint: case.fixture_fingerprint(),
                catalog_fingerprint: "catalog".into(),
                prompt_fingerprint: "prompt".into(),
                tool_schema_fingerprint: "tools".into(),
                budget_fingerprint: "budget".into(),
            },
            status: if candidate.is_some() {
                WorkflowBenchmarkRunStatus::Succeeded
            } else {
                WorkflowBenchmarkRunStatus::Failed
            },
            elapsed_ms: u64::from(attempt_count) * 100 + 50,
            usage: None,
            attempts,
            checks: if candidate.is_some() {
                grade.checks
            } else {
                vec![]
            },
            graded_candidate: candidate,
            grading_errors: vec![],
        }
    }

    #[test]
    fn public_prompt_excludes_host_answers_and_fingerprint_covers_the_contract() {
        let fixture = case();
        assert_eq!(fixture.public_prompt(), fixture.prompt);
        assert!(!fixture.public_prompt().contains(&fixture.reference_source));
        assert!(
            !fixture
                .public_prompt()
                .contains(&fixture.checks[0].expected_output.to_string())
        );
        let fingerprint = fixture.fixture_fingerprint();
        for field in [
            "id",
            "prompt",
            "initial_source",
            "reference_source",
            "required_functions",
            "checks",
        ] {
            let mut changed = fixture.clone();
            match field {
                "id" => changed.id.push('2'),
                "prompt" => changed.prompt.push('2'),
                "initial_source" => changed.initial_source = None,
                "reference_source" => changed.reference_source.push('2'),
                "required_functions" => changed.required_functions.push("second".into()),
                "checks" => changed.checks[0].expected_output = Value::Null,
                _ => unreachable!(),
            }
            assert_ne!(changed.fixture_fingerprint(), fingerprint, "{field}");
        }
        let mut reordered = fixture.clone();
        reordered.checks[0].payload =
            Some(serde_json::from_str(r#"{"name":"Ada","counter":4}"#).unwrap());
        assert_eq!(reordered.fixture_fingerprint(), fingerprint);
        let mut absent_payload = fixture.clone();
        absent_payload.checks[0].payload = None;
        let mut null_payload = absent_payload.clone();
        null_payload.checks[0].payload = Some(Value::Null);
        assert_ne!(
            absent_payload.fixture_fingerprint(),
            null_payload.fixture_fingerprint()
        );
    }

    #[test]
    fn missing_expectations_fail_and_observed_null_survives_report_serialization() {
        assert!(
            serde_json::from_value::<WorkflowBenchmarkCheck>(
                json!({"id":"null", "entry":"transform", "payload":null})
            )
            .is_err()
        );
        let mut fixture = case();
        fixture.checks[0] = serde_json::from_value(json!({
            "id":"null", "entry":"transform", "payload":null, "expected_output":null,
        }))
        .unwrap();
        let original = report(&fixture, 0, 1);
        assert_eq!(original.checks[0].actual_output, Some(Value::Null));
        let serialized = serde_json::to_value(&original).unwrap();
        let restored: WorkflowBenchmarkRunReport =
            serde_json::from_value(serialized.clone()).unwrap();
        assert_eq!(restored.checks[0].actual_output, Some(Value::Null));
        validate_workflow_benchmark_report(&fixture, &restored).unwrap();
        let mut unknown = serialized;
        unknown["self_certified"] = json!(true);
        assert!(serde_json::from_value::<WorkflowBenchmarkRunReport>(unknown).is_err());
    }

    #[test]
    fn exact_grading_rejects_runtime_errors_missing_output_alias_and_ambiguous_results() {
        let fixture = case();
        let good = observation(&fixture);
        assert!(
            grade_workflow_benchmark_checks(&fixture, &[good.clone()])
                .unwrap()
                .passed
        );
        let expected = &fixture.checks[0].expected_output;
        let invalid = [
            json!({"status":"failed", "output":expected, "outputs":[expected], "errors":[]}),
            json!({"status":"success", "output":expected, "outputs":[expected], "errors":["upstream failed"]}),
            json!({"status":"success", "output":expected, "outputs":[expected]}),
            json!({"status":"success", "outputs":[expected], "errors":[]}),
            json!({"status":"success", "output":expected, "outputs":[expected, expected], "errors":[]}),
            runtime(json!({"counter":5})),
            runtime(json!({"counter":4, "name":"Ada"})),
            json!({"status":"stale", "output":expected, "outputs":[expected], "errors":[]}),
        ];
        for value in invalid {
            let grade = grade_workflow_benchmark_checks(
                &fixture,
                &[WorkflowBenchmarkObservation {
                    runtime: value.clone(),
                    ..good.clone()
                }],
            )
            .unwrap();
            assert!(!grade.passed, "{value}");
            assert_ne!(grade.checks[0].status, WorkflowBenchmarkCheckStatus::Passed);
        }
    }

    #[test]
    fn every_declared_host_check_must_have_exactly_one_result() {
        let fixture = case();
        let good = observation(&fixture);
        for observations in [
            vec![],
            vec![good.clone(), good.clone()],
            vec![
                good.clone(),
                WorkflowBenchmarkObservation {
                    check_id: "undeclared".into(),
                    ..good.clone()
                },
            ],
        ] {
            let grade = grade_workflow_benchmark_checks(&fixture, &observations).unwrap();
            assert!(!grade.passed);
            assert!(!grade.errors.is_empty());
            assert_eq!(grade.checks.len(), fixture.checks.len());
        }
        let mut invalid = fixture.clone();
        invalid.checks.push(invalid.checks[0].clone());
        assert!(grade_workflow_benchmark_checks(&invalid, &[]).is_err());
        invalid.checks.clear();
        assert!(grade_workflow_benchmark_checks(&invalid, &[]).is_err());
    }

    #[test]
    fn scorecard_counts_zero_attempt_failures_and_does_not_infer_earlier_behavior() {
        let fixture = case();
        let mut first = report(&fixture, 0, 1);
        first.usage = Some(WorkflowBenchmarkUsage {
            input_tokens: Some(100),
            output_tokens: Some(20),
            cached_input_tokens: Some(0),
        });
        let mut late = report(&fixture, 1, 4);
        late.usage = Some(WorkflowBenchmarkUsage {
            input_tokens: Some(200),
            output_tokens: None,
            cached_input_tokens: None,
        });
        first.observed_tool_schema_fingerprint = Some("advertised-tools".into());
        late.observed_tool_schema_fingerprint = Some("advertised-tools".into());
        let mut changed_surface = late.clone();
        changed_surface.observed_tool_schema_fingerprint = Some("other-tools".into());
        assert!(
            evaluate_workflow_benchmark_runs(&fixture, &[first.clone(), changed_surface]).is_err()
        );
        let never_started = report(&fixture, 2, 0);
        let score =
            evaluate_workflow_benchmark_runs(&fixture, &[first, late, never_started]).unwrap();
        assert_eq!(score.runs_total, 3);
        assert_eq!(score.source_attempts, 5);
        assert_eq!(score.repair_attempts, 3);
        assert_eq!(score.final_behavior_pass, WorkflowBenchmarkRate::new(2, 3));
        assert_eq!(
            score.verified_behavior_at_1,
            WorkflowBenchmarkRate::new(1, 3)
        );
        assert_eq!(
            score.verified_behavior_within_3,
            WorkflowBenchmarkRate::new(1, 3)
        );
        assert_eq!(score.elapsed_ms.samples, 3);
        assert_eq!(score.elapsed_ms.total, 650);
        assert_eq!(score.time_to_verified_behavior_ms.samples, 2);
        assert_eq!(score.input_tokens.samples, 2);
        assert_eq!(score.input_tokens.total, 300);
        assert_eq!(score.output_tokens.samples, 1);
        assert_eq!(score.cached_input_tokens.samples, 1);
        assert_eq!(score.cached_input_tokens.total, 0);
        let empty = evaluate_workflow_benchmark_runs(&fixture, &[]).unwrap();
        assert_eq!(empty.final_behavior_pass.rate, None);
        assert_eq!(empty.input_tokens.mean, None);
    }

    #[test]
    fn report_rejects_unbound_candidates_inconsistent_validation_and_false_passes() {
        let fixture = case();
        let valid = report(&fixture, 0, 1);
        for mutation in 0..9 {
            let mut invalid = valid.clone();
            match mutation {
                0 => {
                    invalid
                        .graded_candidate
                        .as_mut()
                        .unwrap()
                        .source_fingerprint = "other".into()
                }
                1 => invalid.graded_candidate.as_mut().unwrap().revision += 1,
                2 => {
                    invalid
                        .graded_candidate
                        .as_mut()
                        .unwrap()
                        .catalog_fingerprint = "other".into()
                }
                3 => invalid.attempts[0].attempt_index = 3,
                4 => invalid.attempts[0].parse_valid = false,
                5 => invalid.checks[0].actual_output = None,
                6 => invalid.checks.clear(),
                7 => invalid
                    .grading_errors
                    .push("Required helper missing".into()),
                8 => invalid.graded_candidate = None,
                _ => unreachable!(),
            }
            assert!(
                validate_workflow_benchmark_report(&fixture, &invalid).is_err(),
                "mutation {mutation}"
            );
        }
        let mut duplicate_revision = report(&fixture, 0, 2);
        duplicate_revision.attempts[1].revision = 0;
        assert!(validate_workflow_benchmark_report(&fixture, &duplicate_revision).is_err());
    }

    #[test]
    fn aggregation_rejects_duplicate_runs_repetitions_and_mixed_configurations() {
        let fixture = case();
        let first = report(&fixture, 0, 1);
        assert!(
            evaluate_workflow_benchmark_runs(&fixture, &[first.clone(), first.clone()]).is_err()
        );
        let mut repeat = first.clone();
        repeat.run_id = "different-run".into();
        assert!(evaluate_workflow_benchmark_runs(&fixture, &[first.clone(), repeat]).is_err());
        for field in [
            "model", "harness", "budget", "prompt", "profile", "tools", "fixture",
        ] {
            let mut changed = report(&fixture, 1, 1);
            match field {
                "model" => changed.cohort.model = "other-model".into(),
                "harness" => changed.cohort.harness_revision = "other-revision".into(),
                "budget" => changed.cohort.budget_fingerprint = "other-budget".into(),
                "prompt" => changed.cohort.prompt_fingerprint = "other-prompt".into(),
                "profile" => changed.cohort.prompt_profile = "focused".into(),
                "tools" => changed.cohort.tool_schema_fingerprint = "other-tools".into(),
                "fixture" => changed.cohort.fixture_fingerprint = "other-fixture".into(),
                _ => unreachable!(),
            }
            assert!(
                evaluate_workflow_benchmark_runs(&fixture, &[first.clone(), changed]).is_err(),
                "{field}"
            );
        }
    }
}
