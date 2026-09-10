//! Development CLI for the ordinary detached-board authoring and host grading path.

use std::{
    collections::HashSet,
    ffi::OsString,
    fs::{File, OpenOptions},
    io::{Read, Seek, SeekFrom, Write},
    panic::AssertUnwindSafe,
    path::PathBuf,
    sync::{
        Arc, Mutex,
        atomic::{AtomicI32, AtomicU64, Ordering},
    },
    time::{Duration, Instant},
};

use flow_like::copilot::prompts::BoardPromptProfile;
use flow_like::flow::copilot::{
    behavioral_evaluation::{WorkflowBenchmarkRunReport, WorkflowBenchmarkRunStatus},
    benchmark_cases::workflow_benchmark_cases,
};
use futures::FutureExt;
use serde::{Deserialize, Deserializer, Serialize};
use serde_json::Value;
use tauri::{
    AppHandle,
    ipc::{Channel, InvokeResponseBody},
};

use super::{
    backend_types::FlowPilotAgentBackendKind,
    backends::{FlowPilotBackendStartOptions, agent_backend},
    runtime::cancel_registered_copilot_run,
    workflow_benchmark::{
        flowpilot_workflow_benchmark_scorecards, prompt_profile_label,
        run_workflow_benchmark_with_profile,
    },
};

const FLAG: &str = "--flowpilot-workflow-benchmark";
const CONFIG_LIMIT: u64 = 8 * 1024;
const STARTUP_ALLOWANCE: Duration = Duration::from_secs(60);
const RUN_ALLOWANCE: Duration = Duration::from_secs(330);
const STOP_ALLOWANCE: Duration = Duration::from_secs(30);

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct BenchmarkCliConfiguration {
    backend: FlowPilotAgentBackendKind,
    model_id: String,
    #[serde(deserialize_with = "required_optional_string")]
    reasoning_effort: Option<String>,
    #[serde(default = "default_prompt_profiles")]
    prompt_profiles: Vec<BoardPromptProfile>,
    case_ids: Vec<String>,
    repeats: u32,
    output: PathBuf,
}

fn default_prompt_profiles() -> Vec<BoardPromptProfile> {
    vec![BoardPromptProfile::Legacy]
}

#[derive(Debug, PartialEq)]
struct ScheduledRun {
    case_id: String,
    repeat_index: u32,
    profile: BoardPromptProfile,
}

fn required_optional_string<'de, D: Deserializer<'de>>(
    deserializer: D,
) -> Result<Option<String>, D::Error> {
    Option::<String>::deserialize(deserializer)
}

impl BenchmarkCliConfiguration {
    fn validate(&mut self) -> Result<(), String> {
        self.model_id = self.model_id.trim().to_owned();
        if self.model_id.is_empty()
            || self.model_id.eq_ignore_ascii_case("default")
            || self.model_id.len() > 200
        {
            return Err(
                "model_id must identify an explicit model and contain at most 200 bytes".into(),
            );
        }
        if self
            .reasoning_effort
            .as_ref()
            .is_some_and(|value| value.len() > 80)
        {
            return Err("reasoning_effort must contain at most 80 bytes".into());
        }
        self.reasoning_effort =
            super::external_invocation::explicit_reasoning_effort(self.reasoning_effort.as_deref())
                .map(str::to_owned);
        if self.prompt_profiles.is_empty()
            || self.prompt_profiles.len() > 2
            || self
                .prompt_profiles
                .iter()
                .enumerate()
                .any(|(index, profile)| self.prompt_profiles[..index].contains(profile))
        {
            return Err("prompt_profiles must contain one or two distinct profiles".into());
        }
        let known: HashSet<_> = workflow_benchmark_cases()
            .into_iter()
            .map(|case| case.id)
            .collect();
        let unique: HashSet<_> = self.case_ids.iter().collect();
        if self.case_ids.is_empty()
            || unique.len() != self.case_ids.len()
            || self.case_ids.iter().any(|id| !known.contains(id))
        {
            return Err(
                "case_ids must contain a nonempty unique selection of known benchmark cases".into(),
            );
        }
        if !(1..=5).contains(&self.repeats) {
            return Err("repeats must be between one and five".into());
        }
        if !self.output.is_absolute() || self.output.file_name().is_none() {
            return Err("output must be an absolute file path".into());
        }
        Ok(())
    }

    fn requested_runs(&self) -> usize {
        self.case_ids.len() * self.repeats as usize * self.prompt_profiles.len()
    }

    fn schedule(&self) -> Vec<ScheduledRun> {
        let mut runs = Vec::with_capacity(self.requested_runs());
        for repeat_index in 0..self.repeats {
            for (case_index, case_id) in self.case_ids.iter().enumerate() {
                let mut profiles = self.prompt_profiles.clone();
                if (case_index + repeat_index as usize) % 2 == 1 {
                    profiles.reverse();
                }
                for profile in profiles {
                    runs.push(ScheduledRun {
                        case_id: case_id.clone(),
                        repeat_index,
                        profile,
                    });
                }
            }
        }
        runs
    }

    fn deadline(&self) -> Duration {
        STARTUP_ALLOWANCE + RUN_ALLOWANCE * self.requested_runs() as u32
    }
}

fn configuration_path(args: &[OsString]) -> Result<PathBuf, String> {
    match args {
        [flag, path] if flag == FLAG && !path.is_empty() => Ok(PathBuf::from(path)),
        _ => Err(format!("Usage: {FLAG} <configuration.json>")),
    }
}

fn read_configuration(path: PathBuf) -> Result<BenchmarkCliConfiguration, String> {
    let file = File::open(path)
        .map_err(|error| format!("Cannot open benchmark configuration: {error}"))?;
    if !file
        .metadata()
        .map_err(|error| error.to_string())?
        .is_file()
    {
        return Err("Benchmark configuration must be a regular file".into());
    }
    let mut bytes = Vec::new();
    file.take(CONFIG_LIMIT + 1)
        .read_to_end(&mut bytes)
        .map_err(|error| error.to_string())?;
    if bytes.len() as u64 > CONFIG_LIMIT {
        return Err("Benchmark configuration exceeds 8 KiB".into());
    }
    let mut config: BenchmarkCliConfiguration = serde_json::from_slice(&bytes)
        .map_err(|error| format!("Invalid benchmark configuration: {error}"))?;
    config.validate()?;
    Ok(config)
}

#[derive(Serialize)]
struct BenchmarkCliError {
    case_id: Option<String>,
    repeat_index: Option<u32>,
    prompt_profile: Option<String>,
    phase: String,
    message: String,
}

#[derive(Serialize)]
struct BenchmarkCliBatch {
    schema: &'static str,
    run_id: String,
    configuration: BenchmarkCliConfiguration,
    in_progress: bool,
    elapsed_ms: u64,
    requested_runs: usize,
    reports: Vec<WorkflowBenchmarkRunReport>,
    errors: Vec<BenchmarkCliError>,
    scorecards: Vec<Value>,
}

impl BenchmarkCliBatch {
    fn new(configuration: BenchmarkCliConfiguration) -> Self {
        Self {
            schema: "flowpilot.workflow-benchmark-cli/v1",
            run_id: format!("workflow-cli-{}", flow_like_types::create_id()),
            requested_runs: configuration.requested_runs(),
            configuration,
            in_progress: true,
            elapsed_ms: 0,
            reports: Vec::new(),
            errors: Vec::new(),
            scorecards: Vec::new(),
        }
    }

    fn error(&mut self, phase: &str, message: String) {
        self.errors.push(BenchmarkCliError {
            case_id: None,
            repeat_index: None,
            prompt_profile: None,
            phase: phase.into(),
            message,
        });
    }

    fn exit_code(&self) -> i32 {
        if !self.errors.is_empty() {
            2
        } else if !self.in_progress
            && self.reports.len() == self.requested_runs
            && self
                .reports
                .iter()
                .all(|report| report.status == WorkflowBenchmarkRunStatus::Succeeded)
        {
            0
        } else {
            1
        }
    }
}

fn save_batch(output: &Mutex<File>, batch: &BenchmarkCliBatch) -> Result<(), String> {
    let mut bytes = serde_json::to_vec_pretty(batch).map_err(|error| error.to_string())?;
    bytes.push(b'\n');
    let mut file = output
        .lock()
        .map_err(|_| "Benchmark output lock is unavailable")?;
    file.seek(SeekFrom::Start(0))
        .map_err(|error| error.to_string())?;
    file.write_all(&bytes).map_err(|error| error.to_string())?;
    file.set_len(bytes.len() as u64)
        .map_err(|error| error.to_string())?;
    file.sync_all().map_err(|error| error.to_string())
}

async fn run_cases(
    app: &AppHandle,
    batch: &mut BenchmarkCliBatch,
    active_request: &mut Option<String>,
    output: &Mutex<File>,
    started: Instant,
) -> Result<(), String> {
    let config = batch.configuration.clone();
    tokio::time::timeout(
        STARTUP_ALLOWANCE,
        agent_backend(config.backend).start(FlowPilotBackendStartOptions {
            use_stdio: true,
            cli_url: None,
            app_handle: Some(app.clone()),
        }),
    )
    .await
    .map_err(|_| "Backend startup exceeded 60 seconds".to_string())??;
    for planned in config.schedule() {
        let case_id = &planned.case_id;
        let repeat_index = planned.repeat_index;
        let profile = prompt_profile_label(planned.profile);
        let request_id = format!("{}-{}-{}-{profile}", batch.run_id, repeat_index, case_id);
        *active_request = Some(request_id.clone());
        eprintln!(
            "workflow-benchmark: starting {case_id}, profile {profile}, repetition {}",
            repeat_index + 1
        );
        let events = Arc::new(AtomicU64::new(0));
        let progress_case = case_id.clone();
        let channel = Channel::<String>::new(move |_body: InvokeResponseBody| {
            let count = events.fetch_add(1, Ordering::Relaxed) + 1;
            if count % 100 == 0 {
                eprintln!("workflow-benchmark: {progress_case}, {count} stream events");
            }
            Ok(())
        });
        match run_workflow_benchmark_with_profile(
            app.clone(),
            case_id.clone(),
            config.backend,
            config.model_id.clone(),
            config.reasoning_effort.clone(),
            repeat_index,
            request_id,
            channel,
            planned.profile,
        )
        .await
        {
            Ok(report) => {
                eprintln!(
                    "workflow-benchmark: {case_id} [{profile}] {:?}, {} ms",
                    report.status, report.elapsed_ms
                );
                batch.reports.push(report);
            }
            Err(message) => batch.errors.push(BenchmarkCliError {
                case_id: Some(case_id.clone()),
                repeat_index: Some(repeat_index),
                prompt_profile: Some(profile.into()),
                phase: "run".into(),
                message,
            }),
        }
        *active_request = None;
        batch.elapsed_ms = started.elapsed().as_millis() as u64;
        save_batch(output, batch)?;
    }
    Ok(())
}

async fn execute_batch(
    app: AppHandle,
    mut batch: BenchmarkCliBatch,
    output: Arc<Mutex<File>>,
    started: Instant,
    final_exit_code: Arc<AtomicI32>,
) {
    let mut active_request = None;
    let outcome = AssertUnwindSafe(tokio::time::timeout(
        batch
            .configuration
            .deadline()
            .saturating_sub(started.elapsed()),
        run_cases(&app, &mut batch, &mut active_request, &output, started),
    ))
    .catch_unwind()
    .await;
    match outcome {
        Ok(Ok(Ok(()))) => {}
        Ok(Ok(Err(error))) => batch.error("execution", error),
        Ok(Err(_)) => batch.error(
            "deadline",
            "Benchmark batch exceeded its bounded startup and run deadline".into(),
        ),
        Err(_) => batch.error("execution", "Benchmark host panicked".into()),
    }
    if let Some(request_id) = active_request {
        cancel_registered_copilot_run(&request_id);
    }
    match tokio::time::timeout(
        STOP_ALLOWANCE,
        agent_backend(batch.configuration.backend).stop(),
    )
    .await
    {
        Ok(Ok(())) => {}
        Ok(Err(error)) => batch.error("shutdown", error),
        Err(_) => batch.error("shutdown", "Backend shutdown exceeded 30 seconds".into()),
    }
    match flowpilot_workflow_benchmark_scorecards(batch.reports.clone()) {
        Ok(scorecards) => batch.scorecards = scorecards,
        Err(error) => batch.error("scorecards", error),
    }
    batch.in_progress = false;
    batch.elapsed_ms = started.elapsed().as_millis() as u64;
    let code = match save_batch(&output, &batch) {
        Ok(()) => batch.exit_code(),
        Err(error) => {
            eprintln!("workflow-benchmark: could not save report: {error}");
            2
        }
    };
    eprintln!(
        "workflow-benchmark: saved {} reports to {} (exit {code})",
        batch.reports.len(),
        batch.configuration.output.display()
    );
    // Tauri's event loop may return zero even after an explicit nonzero exit request.
    // Publish the saved batch outcome before requesting shutdown.
    final_exit_code.store(code, Ordering::Release);
    app.exit(code);
}

pub(crate) fn run_workflow_benchmark_cli(args: Vec<OsString>) -> i32 {
    let started = Instant::now();
    let config = match configuration_path(&args).and_then(read_configuration) {
        Ok(config) => config,
        Err(error) => {
            eprintln!("workflow-benchmark: {error}");
            return 2;
        }
    };
    let file = match OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&config.output)
    {
        Ok(file) => file,
        Err(error) => {
            eprintln!("workflow-benchmark: cannot create output file: {error}");
            return 2;
        }
    };
    let output = Arc::new(Mutex::new(file));
    let mut batch = BenchmarkCliBatch::new(config);
    if let Err(error) = save_batch(&output, &batch) {
        eprintln!("workflow-benchmark: cannot initialize output file: {error}");
        return 2;
    }
    let context = std::thread::spawn(crate::application_context).join();
    let mut context = match context {
        Ok(context) => context,
        Err(_) => {
            batch.in_progress = false;
            batch.error("setup", "Tauri context construction failed".into());
            let _ = save_batch(&output, &batch);
            return 2;
        }
    };
    context.config_mut().app.windows.clear();
    context.config_mut().identifier = "com.flow-like.workflow-benchmark-cli".into();
    let configuration = batch.configuration.clone();
    let run_id = batch.run_id.clone();
    let task_output = output.clone();
    // An event loop that ends before finalization must not turn an incomplete batch green.
    let final_exit_code = Arc::new(AtomicI32::new(2));
    let task_exit_code = final_exit_code.clone();
    let app = tauri::Builder::default()
        .setup(move |app| {
            #[cfg(target_os = "macos")]
            app.set_activation_policy(tauri::ActivationPolicy::Prohibited);
            tauri::async_runtime::spawn(execute_batch(
                app.handle().clone(),
                batch,
                task_output,
                started,
                task_exit_code,
            ));
            Ok(())
        })
        .build(context);
    match app {
        Ok(app) => {
            app.run_return(|_, event| {
                if let tauri::RunEvent::ExitRequested {
                    code: None, api, ..
                } = event
                {
                    api.prevent_exit();
                }
            });
            final_exit_code.load(Ordering::Acquire)
        }
        Err(error) => {
            let mut failed = BenchmarkCliBatch::new(configuration);
            failed.run_id = run_id;
            failed.in_progress = false;
            failed.error("setup", error.to_string());
            let _ = save_batch(&output, &failed);
            eprintln!("workflow-benchmark: Tauri host setup failed: {error}");
            2
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn configuration() -> Value {
        json!({"backend":"codex","model_id":"explicit-model","reasoning_effort":"ultra","case_ids":["profile-card"],"repeats":1,"output":std::env::temp_dir().join("workflow-benchmark-output.json")})
    }

    fn report(status: WorkflowBenchmarkRunStatus) -> WorkflowBenchmarkRunReport {
        serde_json::from_value(json!({
            "schema": flow_like::flow::copilot::behavioral_evaluation::WORKFLOW_BENCHMARK_REPORT_SCHEMA,
            "run_id": "exit-status-test", "case_id": "profile-card", "repeat_index": 0,
            "cohort": {
                "provider": "host-test", "model": "no-model-invoked", "reasoning_effort": "none",
                "prompt_profile": "legacy", "harness_revision": "exit-status-test",
                "fixture_fingerprint": "fixture", "catalog_fingerprint": "catalog",
                "prompt_fingerprint": "prompt", "tool_schema_fingerprint": "tools",
                "budget_fingerprint": "budget"
            },
            "status": status, "elapsed_ms": 1, "usage": null, "attempts": [],
            "graded_candidate": null, "checks": [], "grading_errors": []
        })).unwrap()
    }

    #[test]
    fn cli_exit_status_requires_a_final_complete_successful_batch() {
        let config = serde_json::from_value(configuration()).unwrap();
        let mut batch = BenchmarkCliBatch::new(config);
        assert_eq!(batch.exit_code(), 1, "no completed reports");
        batch
            .reports
            .push(report(WorkflowBenchmarkRunStatus::Succeeded));
        assert_eq!(batch.exit_code(), 1, "still finalizing");
        batch.in_progress = false;
        assert_eq!(batch.exit_code(), 0);

        for status in [
            WorkflowBenchmarkRunStatus::Failed,
            WorkflowBenchmarkRunStatus::Cancelled,
        ] {
            batch.reports[0].status = status;
            assert_eq!(batch.exit_code(), 1, "{status:?}");
        }
        batch.reports[0].status = WorkflowBenchmarkRunStatus::Succeeded;
        batch.requested_runs = 2;
        assert_eq!(batch.exit_code(), 1, "missing a requested report");
        batch
            .reports
            .push(report(WorkflowBenchmarkRunStatus::Failed));
        assert_eq!(
            batch.exit_code(),
            1,
            "one failure in an otherwise complete batch"
        );
        batch.reports[1].status = WorkflowBenchmarkRunStatus::Succeeded;
        assert_eq!(batch.exit_code(), 0);
    }

    #[test]
    fn cli_exit_status_keeps_setup_and_infrastructure_errors_nonzero() {
        for phase in ["setup", "execution", "deadline", "shutdown", "scorecards"] {
            let config = serde_json::from_value(configuration()).unwrap();
            let mut batch = BenchmarkCliBatch::new(config);
            batch.in_progress = false;
            batch.error(phase, "deliberate host failure".into());
            assert_eq!(batch.exit_code(), 2, "{phase}");
            batch
                .reports
                .push(report(WorkflowBenchmarkRunStatus::Succeeded));
            assert_eq!(
                batch.exit_code(),
                2,
                "a successful report cannot erase {phase}"
            );
        }
    }

    #[test]
    fn cli_requires_exact_flag_and_config_path() {
        assert!(configuration_path(&[FLAG.into(), "fixture.json".into()]).is_ok());
        assert!(configuration_path(&[FLAG.into()]).is_err());
        assert!(configuration_path(&[FLAG.into(), "fixture.json".into(), "extra".into()]).is_err());
        assert!(
            configuration_path(&["--flowpilot-workflow-benchmark=fixture.json".into()]).is_err()
        );
    }

    #[test]
    fn cli_configuration_has_explicit_bounded_model_and_selection() {
        let mut valid: BenchmarkCliConfiguration = serde_json::from_value(configuration()).unwrap();
        valid.validate().unwrap();
        assert_eq!(valid.requested_runs(), 1);
        assert_eq!(valid.deadline(), Duration::from_secs(390));
        for (key, value) in [
            ("model_id", json!("default")),
            ("model_id", json!("  ")),
            ("case_ids", json!([])),
            ("case_ids", json!(["profile-card", "profile-card"])),
            ("case_ids", json!(["unknown"])),
            ("repeats", json!(0)),
            ("repeats", json!(6)),
            ("output", json!("relative.json")),
            ("prompt_profiles", json!([])),
            ("prompt_profiles", json!(["legacy", "legacy"])),
        ] {
            let mut input = configuration();
            input[key] = value;
            let mut candidate: BenchmarkCliConfiguration = serde_json::from_value(input).unwrap();
            assert!(candidate.validate().is_err(), "{key}");
        }
        for missing in [
            "backend",
            "model_id",
            "reasoning_effort",
            "case_ids",
            "repeats",
            "output",
        ] {
            let mut input = configuration();
            input.as_object_mut().unwrap().remove(missing);
            assert!(
                serde_json::from_value::<BenchmarkCliConfiguration>(input).is_err(),
                "{missing}"
            );
        }
        let mut extra = configuration();
        extra["prompt"] = json!("must not override the fixed public task");
        assert!(serde_json::from_value::<BenchmarkCliConfiguration>(extra).is_err());
    }

    #[test]
    fn cli_counterbalances_profile_order_and_preserves_every_repetition() {
        let mut value = configuration();
        value["case_ids"] = json!(["profile-card", "repair-casing"]);
        value["repeats"] = json!(2);
        value["prompt_profiles"] = json!(["legacy", "focused"]);
        let mut config: BenchmarkCliConfiguration = serde_json::from_value(value).unwrap();
        config.validate().unwrap();
        let schedule = config.schedule();
        assert_eq!(config.requested_runs(), 8);
        assert_eq!(schedule.len(), 8);
        assert_eq!(
            schedule
                .iter()
                .map(|run| prompt_profile_label(run.profile))
                .collect::<Vec<_>>(),
            [
                "legacy", "focused", "focused", "legacy", "focused", "legacy", "legacy", "focused"
            ]
        );
        for repeat in 0..2 {
            for case in ["profile-card", "repair-casing"] {
                for profile in [BoardPromptProfile::Legacy, BoardPromptProfile::Focused] {
                    assert_eq!(
                        schedule
                            .iter()
                            .filter(|run| run.repeat_index == repeat
                                && run.case_id == case
                                && run.profile == profile)
                            .count(),
                        1
                    );
                }
            }
        }
    }

    #[test]
    fn cli_rejects_oversized_configuration_before_starting_the_host() {
        let path = std::env::temp_dir().join(format!(
            "workflow-benchmark-config-{}.json",
            flow_like_types::create_id()
        ));
        std::fs::write(&path, vec![b' '; CONFIG_LIMIT as usize + 1]).unwrap();
        let error = read_configuration(path.clone()).unwrap_err();
        std::fs::remove_file(path).unwrap();
        assert!(error.contains("8 KiB"));
    }

    #[test]
    fn cli_refuses_to_replace_an_output_before_starting_the_host() {
        let directory = std::env::temp_dir().join(format!(
            "workflow-benchmark-preflight-{}",
            flow_like_types::create_id()
        ));
        std::fs::create_dir(&directory).unwrap();
        let output = directory.join("existing.json");
        let config_path = directory.join("configuration.json");
        std::fs::write(&output, b"original artifact").unwrap();
        let mut input = configuration();
        input["output"] = json!(output);
        std::fs::write(&config_path, serde_json::to_vec(&input).unwrap()).unwrap();
        assert_eq!(
            run_workflow_benchmark_cli(vec![FLAG.into(), config_path.into_os_string()]),
            2
        );
        assert_eq!(std::fs::read(&output).unwrap(), b"original artifact");
        std::fs::remove_dir_all(directory).unwrap();
    }
}
