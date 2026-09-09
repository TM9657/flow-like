//! Content-free observations of external CLI delivery, protocol activity, and shutdown.

use std::sync::{Arc, Mutex};
use std::time::Instant;

use flow_like_types::tokio_util::sync::CancellationToken;
use serde::Serialize;
use serde_json::Value;

use super::backend_types::FlowPilotAgentBackendKind;

type Observer = Arc<dyn Fn(&Value) + Send + Sync>;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
enum StdinStatus {
    Pending,
    NotRequired,
    Writing,
    Sent,
    Failed,
    Incomplete,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
enum TerminalStatus {
    Running,
    Completed,
    Failed,
    Cancelled,
    FutureDropped,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub(super) enum ExternalPhaseFailure {
    Spawn,
    MissingStdin,
    MissingStdout,
    MissingStderr,
    StdinWrite,
    StdinFlush,
    StdoutRead,
    ChildWait,
    ShutdownTimeout,
    Protocol,
    McpConnection,
    ProcessExit,
    ProcessOutcome,
}

#[derive(Debug, Clone, Serialize)]
struct ProtocolObservation {
    kind: &'static str,
    item_kind: Option<&'static str>,
    elapsed_ms: u64,
}

#[derive(Debug, Clone, Serialize)]
struct PhaseSnapshot {
    schema: &'static str,
    phase_id: String,
    observation_sequence: u64,
    backend: &'static str,
    prompt_bytes: u64,
    stdin_prompt_bytes: u64,
    appended_system_prompt_bytes: u64,
    elapsed_ms: u64,
    spawned_at_ms: Option<u64>,
    stdin_started_at_ms: Option<u64>,
    stdin_sent_at_ms: Option<u64>,
    stdin_status: StdinStatus,
    protocol_event_count: u64,
    non_json_stdout_lines: u64,
    last_protocol_event: Option<ProtocolObservation>,
    completion_event_at_ms: Option<u64>,
    exit_observed_at_ms: Option<u64>,
    exit_code: Option<i32>,
    exit_success: Option<bool>,
    cancellation_observed_at_ms: Option<u64>,
    terminal_status: TerminalStatus,
    failure_kind: Option<ExternalPhaseFailure>,
}

/// Stdin writing and stdout draining share this bounded observation state. The callback receives
/// a sequence so a host can reject an older callback delivered after a newer observation.
#[derive(Clone)]
pub(super) struct ExternalPhaseObserver {
    state: Arc<Mutex<PhaseSnapshot>>,
    started: Instant,
    observer: Observer,
}

impl ExternalPhaseObserver {
    fn update(&self, operation: impl FnOnce(&mut PhaseSnapshot, u64)) {
        let snapshot = {
            let mut state = self.state.lock().unwrap_or_else(|error| error.into_inner());
            if state.terminal_status != TerminalStatus::Running {
                return;
            }
            let elapsed_ms = self.started.elapsed().as_millis().min(u64::MAX as u128) as u64;
            operation(&mut state, elapsed_ms);
            state.elapsed_ms = elapsed_ms;
            state.observation_sequence = state.observation_sequence.saturating_add(1);
            state.clone()
        };
        // Release the phase lock before entering host observation storage.
        if let Ok(value) = serde_json::to_value(snapshot) {
            (self.observer)(&value);
        }
    }

    pub(super) fn spawned(&self) {
        self.update(|state, elapsed| state.spawned_at_ms = Some(elapsed));
    }

    pub(super) fn stdin_started(&self) {
        self.update(|state, elapsed| {
            state.stdin_started_at_ms = Some(elapsed);
            state.stdin_status = StdinStatus::Writing;
        });
    }

    pub(super) fn stdin_not_required(&self) {
        self.update(|state, _| state.stdin_status = StdinStatus::NotRequired);
    }

    pub(super) fn stdin_sent(&self) {
        self.update(|state, elapsed| {
            state.stdin_sent_at_ms = Some(elapsed);
            state.stdin_status = StdinStatus::Sent;
        });
    }

    pub(super) fn stdin_failed(&self, kind: ExternalPhaseFailure) {
        self.update(|state, _| {
            state.stdin_status = StdinStatus::Failed;
            state.failure_kind.get_or_insert(kind);
        });
    }

    pub(super) fn protocol_event(&self, event: &Value) {
        let (kind, item_kind) = protocol_kind(event);
        self.update(|state, elapsed| {
            state.protocol_event_count = state.protocol_event_count.saturating_add(1);
            state.last_protocol_event = Some(ProtocolObservation {
                kind,
                item_kind,
                elapsed_ms: elapsed,
            });
            if matches!(kind, "turn.completed" | "result") {
                state.completion_event_at_ms = Some(elapsed);
            }
        });
    }

    pub(super) fn non_json_stdout_line(&self) {
        self.update(|state, _| {
            state.non_json_stdout_lines = state.non_json_stdout_lines.saturating_add(1);
        });
    }

    pub(super) fn failure(&self, kind: ExternalPhaseFailure) {
        self.update(|state, _| {
            state.failure_kind.get_or_insert(kind);
        });
    }

    pub(super) fn cancelled(&self) {
        self.update(|state, elapsed| {
            state.cancellation_observed_at_ms.get_or_insert(elapsed);
        });
    }

    pub(super) fn exited(&self, code: Option<i32>, success: bool) {
        self.update(|state, elapsed| {
            state.exit_observed_at_ms = Some(elapsed);
            state.exit_code = code;
            state.exit_success = Some(success);
            if !success {
                state
                    .failure_kind
                    .get_or_insert(ExternalPhaseFailure::ProcessExit);
            }
        });
    }

    fn finish(&self, terminal_status: TerminalStatus, cancelled: bool) {
        self.update(|state, elapsed| {
            if cancelled {
                state.cancellation_observed_at_ms.get_or_insert(elapsed);
            }
            state.terminal_status = if cancelled {
                TerminalStatus::Cancelled
            } else {
                terminal_status
            };
            if matches!(
                state.stdin_status,
                StdinStatus::Pending | StdinStatus::Writing
            ) {
                state.stdin_status = StdinStatus::Incomplete;
            }
            if state.terminal_status == TerminalStatus::Failed {
                state
                    .failure_kind
                    .get_or_insert(ExternalPhaseFailure::ProcessOutcome);
            }
        });
    }
}

/// Dropping the invocation future synchronously publishes its last phase state. Observer clones
/// in detached pipe tasks cannot alter the final snapshot after this guard closes it.
pub(super) struct ExternalPhaseTelemetry {
    observer: ExternalPhaseObserver,
    cancellation: CancellationToken,
}

impl ExternalPhaseTelemetry {
    pub(super) fn new(
        backend: FlowPilotAgentBackendKind,
        stdin_prompt_bytes: usize,
        appended_system_prompt_bytes: usize,
        cancellation: CancellationToken,
        observer: Observer,
    ) -> Self {
        let stdin_prompt_bytes = stdin_prompt_bytes as u64;
        let appended_system_prompt_bytes = appended_system_prompt_bytes as u64;
        let observer = ExternalPhaseObserver {
            state: Arc::new(Mutex::new(PhaseSnapshot {
                schema: "flowpilot.external-process-phase/v1",
                phase_id: uuid::Uuid::new_v4().to_string(),
                observation_sequence: 0,
                backend: match backend {
                    FlowPilotAgentBackendKind::Codex => "codex",
                    FlowPilotAgentBackendKind::ClaudeCode => "claude-code",
                    FlowPilotAgentBackendKind::GithubCopilot => "github-copilot",
                },
                prompt_bytes: stdin_prompt_bytes.saturating_add(appended_system_prompt_bytes),
                stdin_prompt_bytes,
                appended_system_prompt_bytes,
                elapsed_ms: 0,
                spawned_at_ms: None,
                stdin_started_at_ms: None,
                stdin_sent_at_ms: None,
                stdin_status: StdinStatus::Pending,
                protocol_event_count: 0,
                non_json_stdout_lines: 0,
                last_protocol_event: None,
                completion_event_at_ms: None,
                exit_observed_at_ms: None,
                exit_code: None,
                exit_success: None,
                cancellation_observed_at_ms: None,
                terminal_status: TerminalStatus::Running,
                failure_kind: None,
            })),
            started: Instant::now(),
            observer,
        };
        observer.update(|_, _| {});
        Self {
            observer,
            cancellation,
        }
    }

    pub(super) fn observer(&self) -> ExternalPhaseObserver {
        self.observer.clone()
    }

    pub(super) fn finish(&self, completed: bool) {
        self.observer.finish(
            if completed {
                TerminalStatus::Completed
            } else {
                TerminalStatus::Failed
            },
            self.cancellation.is_cancelled(),
        );
    }
}

impl Drop for ExternalPhaseTelemetry {
    fn drop(&mut self) {
        self.observer.finish(
            TerminalStatus::FutureDropped,
            self.cancellation.is_cancelled(),
        );
    }
}

/// Unknown discriminator strings never enter the observation. Only fixed protocol labels are
/// retained; IDs, model text, tool names, arguments, auth data, and error text are excluded.
fn protocol_kind(event: &Value) -> (&'static str, Option<&'static str>) {
    let kind = match event.get("type").and_then(Value::as_str) {
        Some("thread.started") => "thread.started",
        Some("turn.started") => "turn.started",
        Some("turn.completed") => "turn.completed",
        Some("turn.failed") => "turn.failed",
        Some("item.started") => "item.started",
        Some("item.updated") => "item.updated",
        Some("item.completed") => "item.completed",
        Some("agent_message_delta") => "agent_message_delta",
        Some("assistant_message_delta") => "assistant_message_delta",
        Some("system") => "system",
        Some("assistant") => "assistant",
        Some("user") => "user",
        Some("result") => "result",
        Some("stream_event") => "stream_event",
        Some("rate_limit_event") => "rate_limit_event",
        Some("auth_status") => "auth_status",
        Some("error") => "error",
        _ => "other_json",
    };
    let nested = match kind {
        "item.started" | "item.updated" | "item.completed" => {
            event.get("item").and_then(|item| item.get("type"))
        }
        "stream_event" => event.get("event").and_then(|event| event.get("type")),
        "system" | "result" => event.get("subtype"),
        _ => None,
    };
    let item_kind = nested.map(|kind| match kind.as_str() {
        Some("mcp_tool_call") => "mcp_tool_call",
        Some("command_execution") => "command_execution",
        Some("file_change") => "file_change",
        Some("web_search") => "web_search",
        Some("todo_list") => "todo_list",
        Some("agent_message") => "agent_message",
        Some("assistant_message") => "assistant_message",
        Some("reasoning") => "reasoning",
        Some("error") => "error",
        Some("init") => "init",
        Some("success") => "success",
        Some("message_start") => "message_start",
        Some("message_delta") => "message_delta",
        Some("message_stop") => "message_stop",
        Some("content_block_start") => "content_block_start",
        Some("content_block_delta") => "content_block_delta",
        Some("content_block_stop") => "content_block_stop",
        Some("error_max_turns") => "error_max_turns",
        Some("error_during_execution") => "error_during_execution",
        Some("error_max_budget_usd") => "error_max_budget_usd",
        _ => "other",
    });
    (kind, item_kind)
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn fixture(
        cancellation: CancellationToken,
    ) -> (ExternalPhaseTelemetry, Arc<Mutex<Vec<Value>>>) {
        let snapshots = Arc::new(Mutex::new(Vec::new()));
        let sink = snapshots.clone();
        let phase = ExternalPhaseTelemetry::new(
            FlowPilotAgentBackendKind::Codex,
            "é".len(),
            4,
            cancellation,
            Arc::new(move |snapshot| sink.lock().unwrap().push(snapshot.clone())),
        );
        (phase, snapshots)
    }

    #[test]
    fn phase_milestones_distinguish_protocol_completion_from_process_exit() {
        let (phase, snapshots) = fixture(CancellationToken::new());
        let observer = phase.observer();
        observer.spawned();
        observer.stdin_started();
        observer.stdin_sent();
        observer.protocol_event(&json!({"type": "turn.started"}));
        observer.protocol_event(&json!({"type": "turn.completed"}));
        let before_exit = snapshots.lock().unwrap().last().unwrap().clone();
        assert_eq!(before_exit["prompt_bytes"], 6);
        assert_eq!(before_exit["stdin_prompt_bytes"], 2);
        assert_eq!(before_exit["appended_system_prompt_bytes"], 4);
        assert_eq!(before_exit["stdin_status"], "sent");
        assert!(before_exit["stdin_sent_at_ms"].is_u64());
        assert!(before_exit["completion_event_at_ms"].is_u64());
        assert!(before_exit["exit_observed_at_ms"].is_null());
        assert_eq!(before_exit["terminal_status"], "running");
        observer.exited(Some(0), true);
        phase.finish(true);
        drop(phase);
        let snapshots = snapshots.lock().unwrap();
        let final_snapshot = snapshots.last().unwrap();
        assert_eq!(final_snapshot["terminal_status"], "completed");
        assert_eq!(final_snapshot["exit_code"], 0);
        assert_eq!(final_snapshot["exit_success"], true);
        assert!(
            snapshots
                .windows(2)
                .all(|pair| pair[0]["observation_sequence"].as_u64()
                    < pair[1]["observation_sequence"].as_u64())
        );
        assert!(
            snapshots
                .iter()
                .all(|snapshot| snapshot["phase_id"] == final_snapshot["phase_id"])
        );
    }

    #[test]
    fn dropped_invocation_retains_latest_event_and_freezes_late_pipe_observers() {
        let (phase, snapshots) = fixture(CancellationToken::new());
        let observer = phase.observer();
        observer.spawned();
        observer.stdin_started();
        observer
            .protocol_event(&json!({"type": "item.started", "item": {"type": "mcp_tool_call"}}));
        drop(phase);
        let final_snapshot = snapshots.lock().unwrap().last().unwrap().clone();
        assert_eq!(final_snapshot["terminal_status"], "future_dropped");
        assert_eq!(final_snapshot["stdin_status"], "incomplete");
        assert_eq!(
            final_snapshot["last_protocol_event"]["kind"],
            "item.started"
        );
        assert_eq!(
            final_snapshot["last_protocol_event"]["item_kind"],
            "mcp_tool_call"
        );
        assert!(final_snapshot["exit_success"].is_null());
        let count = snapshots.lock().unwrap().len();
        observer.stdin_sent();
        observer.protocol_event(&json!({"type": "turn.completed"}));
        assert_eq!(snapshots.lock().unwrap().len(), count);
    }

    #[test]
    fn cancellation_and_pipe_failures_have_fixed_metadata_without_error_contents() {
        let cancellation = CancellationToken::new();
        let (phase, snapshots) = fixture(cancellation.clone());
        phase.observer().stdin_started();
        phase
            .observer()
            .stdin_failed(ExternalPhaseFailure::StdinWrite);
        cancellation.cancel();
        drop(phase);
        let snapshots = snapshots.lock().unwrap();
        let snapshot = snapshots.last().unwrap();
        assert_eq!(snapshot["terminal_status"], "cancelled");
        assert_eq!(snapshot["stdin_status"], "failed");
        assert_eq!(snapshot["failure_kind"], "stdin_write");
        assert!(snapshot["cancellation_observed_at_ms"].is_u64());
    }

    #[tokio::test]
    async fn dropping_a_pending_process_future_publishes_its_latest_phase_synchronously() {
        let (phase, snapshots) = fixture(CancellationToken::new());
        {
            let mut invocation = Box::pin(async move {
                let _phase_guard = phase;
                let observer = _phase_guard.observer();
                observer.spawned();
                observer.stdin_started();
                observer.stdin_sent();
                observer.protocol_event(&json!({"type": "thread.started", "thread_id": "private"}));
                std::future::pending::<()>().await;
            });
            tokio::select! {
                biased;
                _ = &mut invocation => panic!("pending invocation completed"),
                _ = tokio::task::yield_now() => {},
            }
        }
        let snapshots = snapshots.lock().unwrap();
        let last = snapshots.last().unwrap();
        assert_eq!(last["terminal_status"], "future_dropped");
        assert_eq!(last["stdin_status"], "sent");
        assert_eq!(last["last_protocol_event"]["kind"], "thread.started");
        assert!(last["completion_event_at_ms"].is_null());
        assert!(last["exit_observed_at_ms"].is_null());
    }

    #[test]
    fn explicit_process_failure_is_not_reclassified_by_guard_drop() {
        let (phase, snapshots) = fixture(CancellationToken::new());
        phase.observer().failure(ExternalPhaseFailure::Spawn);
        phase.finish(false);
        let count = snapshots.lock().unwrap().len();
        drop(phase);
        let snapshots = snapshots.lock().unwrap();
        assert_eq!(snapshots.len(), count);
        assert_eq!(snapshots.last().unwrap()["terminal_status"], "failed");
        assert_eq!(snapshots.last().unwrap()["failure_kind"], "spawn");
        assert!(snapshots.last().unwrap()["spawned_at_ms"].is_null());
    }

    #[test]
    fn protocol_observations_never_retain_content_or_unknown_discriminators() {
        let (phase, snapshots) = fixture(CancellationToken::new());
        let observer = phase.observer();
        observer.protocol_event(&json!({"type": "item.completed", "item": {
            "type": "agent_message", "text": "MODEL_SECRET", "id": "PRIVATE_ID",
            "arguments": {"password": "AUTH_SECRET"}, "error": "STDERR_SECRET"
        }}));
        observer.protocol_event(&json!({"type": "UNKNOWN_SECRET", "session_id": "SESSION_SECRET"}));
        observer.protocol_event(
            &json!({"type": "system", "subtype": "SUBTYPE_SECRET", "apiKey": "KEY_SECRET"}),
        );
        observer.non_json_stdout_line();
        drop(phase);
        let snapshots = snapshots.lock().unwrap();
        let serialized = serde_json::to_string(&*snapshots).unwrap();
        assert!(!serialized.contains("SECRET"));
        assert!(!serialized.contains("PRIVATE_ID"));
        assert_eq!(
            snapshots.last().unwrap()["last_protocol_event"]["item_kind"],
            "other"
        );
        assert_eq!(snapshots.last().unwrap()["protocol_event_count"], 3);
        assert_eq!(snapshots.last().unwrap()["non_json_stdout_lines"], 1);
        assert!(
            snapshots
                .iter()
                .all(|snapshot| serde_json::to_vec(snapshot).unwrap().len() < 8 * 1024)
        );
    }
}
