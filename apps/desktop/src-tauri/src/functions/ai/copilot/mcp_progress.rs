//! MCP tool definitions, cancellation, and progress heartbeats.

use super::runtime::MCP_TOOL_PROGRESS_HEARTBEAT_INTERVAL;
use super::workflow_state::{
    MAX_EXTERNAL_FLOWSCRIPT_COMMIT_ATTEMPTS, MAX_EXTERNAL_FLOWSCRIPT_OPERATION_ATTEMPTS,
    MAX_EXTERNAL_WORKFLOW_EDIT_ATTEMPTS, WorkflowToolLoopState,
};
use flow_like_types::tokio_util::sync::CancellationToken;
use std::{
    sync::{Arc, LazyLock, Mutex as StdMutex},
    time::{Duration, Instant},
};

#[derive(Clone)]
pub(super) struct FlowPilotMcpTool {
    pub(super) definition: copilot_sdk::Tool,
    pub(super) handler: copilot_sdk::ToolHandler,
}

/// Cancels the synchronous tool bridge if the async MCP request future is dropped (for example,
/// when Claude/Codex disconnects its HTTP transport mid-call). `spawn_blocking` tasks are detached
/// when their JoinHandle is dropped, so aborting the async request alone is otherwise insufficient.
pub(super) struct McpToolCancellationGuard {
    pub(super) cancellation: CancellationToken,
    pub(super) armed: bool,
}

impl McpToolCancellationGuard {
    pub(super) fn new(cancellation: CancellationToken) -> Self {
        Self {
            cancellation,
            armed: true,
        }
    }

    pub(super) fn disarm(&mut self) {
        self.armed = false;
    }
}

impl Drop for McpToolCancellationGuard {
    fn drop(&mut self) {
        if self.armed {
            self.cancellation.cancel();
        }
    }
}

/// Delegation tools whose MCP call blocks the outer agent on a nested FlowPilot run.
pub(super) fn is_delegated_agent_tool(tool_name: &str) -> bool {
    matches!(
        tool_name,
        "flowpilot_board"
            | "flowpilot_widget"
            | "flowpilot_home"
            | "project_scout"
            | "research_agent"
    )
}

#[derive(Clone, Debug)]
pub(super) struct DelegatedRunToolProgress {
    tool_name: String,
    total_tool_calls: u64,
    budget_summary: Option<String>,
}

/// Most recent tool progress reported by any FlowPilot MCP run in this process. While the outer
/// agent waits on a delegated FlowPilot specialist, its only signal is the progress heartbeat, so
/// this single bounded slot gives those heartbeats substance (last tool used plus loop budget
/// counts) without cross-run plumbing. Diagnostic prose only, never used for control flow.
pub(super) static LATEST_DELEGATED_RUN_TOOL_PROGRESS: LazyLock<
    StdMutex<Option<(Instant, DelegatedRunToolProgress)>>,
> = LazyLock::new(|| StdMutex::new(None));

/// A stale entry (e.g. from an earlier finished run) must not narrate a hung wait as progress.
pub(super) const DELEGATED_RUN_PROGRESS_FRESHNESS: Duration = Duration::from_secs(3 * 60);

pub(super) fn record_delegated_run_tool_progress(
    tool_name: &str,
    total_tool_calls: u64,
    workflow_state: Option<&Arc<StdMutex<WorkflowToolLoopState>>>,
) {
    // The delegation tools themselves are what the outer agent is waiting ON; recording them
    // would overwrite the nested run's substance with the wait itself.
    if is_delegated_agent_tool(tool_name) {
        return;
    }
    let budget_summary = workflow_state
        .and_then(|state| state.lock().ok())
        .map(|state| {
            let snapshot = state.snapshot();
            format!(
                "checks {}/{MAX_EXTERNAL_WORKFLOW_EDIT_ATTEMPTS}, source operations {}/{MAX_EXTERNAL_FLOWSCRIPT_OPERATION_ATTEMPTS}, commit attempts {}/{MAX_EXTERNAL_FLOWSCRIPT_COMMIT_ATTEMPTS}",
                snapshot.edit_attempts,
                snapshot.flowscript_operation_attempts,
                snapshot.flowscript_commit_attempts,
            )
        });
    if let Ok(mut latest) = LATEST_DELEGATED_RUN_TOOL_PROGRESS.lock() {
        *latest = Some((
            Instant::now(),
            DelegatedRunToolProgress {
                tool_name: tool_name.to_string(),
                total_tool_calls,
                budget_summary,
            },
        ));
    }
}

/// Compose one heartbeat line for a delegation tool: the base "still running" text plus the
/// freshest nested-run tool/budget progress, so waiting turns are not blind.
pub(super) fn delegated_run_heartbeat_message(base: &str) -> String {
    let progress = LATEST_DELEGATED_RUN_TOOL_PROGRESS
        .lock()
        .ok()
        .and_then(|latest| {
            latest.as_ref().and_then(|(recorded_at, progress)| {
                (recorded_at.elapsed() <= DELEGATED_RUN_PROGRESS_FRESHNESS)
                    .then(|| progress.clone())
            })
        });
    let Some(progress) = progress else {
        return base.to_string();
    };
    match progress.budget_summary.as_deref() {
        Some(budgets) => format!(
            "{base}; the delegated run last used {} (tool call {}; {budgets})",
            progress.tool_name, progress.total_tool_calls
        ),
        None => format!(
            "{base}; the delegated run last used {} (tool call {})",
            progress.tool_name, progress.total_tool_calls
        ),
    }
}

/// Keeps a long-running frontend-backed MCP call observable to clients with an idle watchdog.
/// MCP progress is opt-in: the server may only emit it when the caller supplied a progress token
/// in the request metadata. The task is tied to both the request cancellation token and this RAII
/// guard, so a completed, cancelled, or dropped request cannot leave a detached notifier behind.
pub(super) struct McpProgressHeartbeat {
    pub(super) cancellation: CancellationToken,
    pub(super) task: tokio::task::JoinHandle<()>,
}

impl McpProgressHeartbeat {
    pub(super) fn start(
        context: &rmcp::service::RequestContext<rmcp::RoleServer>,
        request_cancellation: CancellationToken,
        tool_name: &str,
    ) -> Option<Self> {
        let progress_token = context.meta.get_progress_token()?;
        let peer = context.peer.clone();
        let cancellation = CancellationToken::new();
        let task_cancellation = cancellation.clone();
        let message = format!("FlowPilot {tool_name} is still running");
        let delegated_tool = is_delegated_agent_tool(tool_name);
        let task = tokio::spawn(async move {
            let mut ticker = tokio::time::interval_at(
                tokio::time::Instant::now() + MCP_TOOL_PROGRESS_HEARTBEAT_INTERVAL,
                MCP_TOOL_PROGRESS_HEARTBEAT_INTERVAL,
            );
            ticker.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);
            let mut progress = 0.0_f64;

            loop {
                tokio::select! {
                    biased;
                    _ = task_cancellation.cancelled() => break,
                    _ = request_cancellation.cancelled() => break,
                    _ = ticker.tick() => {
                        progress += 1.0;
                        // Recomputed per tick: a delegation wait should narrate the nested run's
                        // latest tool/budget movement, not repeat a blind static line.
                        let tick_message = if delegated_tool {
                            delegated_run_heartbeat_message(&message)
                        } else {
                            message.clone()
                        };
                        let notification = mcp_progress_heartbeat_notification(
                            progress_token.clone(),
                            progress,
                            &tick_message,
                        );
                        let result = tokio::select! {
                            biased;
                            _ = task_cancellation.cancelled() => break,
                            _ = request_cancellation.cancelled() => break,
                            result = peer.notify_progress(notification) => result,
                        };
                        if result.is_err() {
                            // The peer/transport is gone. Retrying forever would only retain the
                            // request state; the owning handler still has its own cancellation.
                            break;
                        }
                    }
                }
            }
        });

        Some(Self { cancellation, task })
    }
}

impl Drop for McpProgressHeartbeat {
    fn drop(&mut self) {
        self.cancellation.cancel();
        self.task.abort();
    }
}

pub(super) fn mcp_progress_heartbeat_notification(
    progress_token: rmcp::model::ProgressToken,
    progress: f64,
    message: &str,
) -> rmcp::model::ProgressNotificationParam {
    rmcp::model::ProgressNotificationParam::new(progress_token, progress).with_message(message)
}
