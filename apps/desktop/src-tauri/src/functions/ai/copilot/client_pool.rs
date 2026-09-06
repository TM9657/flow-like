//! SDK client pools, leases, and nested-run concurrency gates.

use super::backend_types::FlowPilotAgentBackendKind;
use super::backends::FlowPilotBackendStartOptions;
use super::cli_resolution::{augmented_path, find_copilot_cli_path};
use super::runtime::{SDK_CHAT_ABORT_TIMEOUT, SDK_CONTROL_RPC_TIMEOUT};
use crate::functions::ai::frontend_tool_bridge::FrontendToolContext;
use copilot_sdk::{Client, LogLevel};
use flow_like::{copilot::CopilotScope, flow::board::Board};
use flow_like_types::tokio_util::sync::CancellationToken;
use once_cell::sync::Lazy;
use std::{
    collections::HashMap,
    sync::{
        Arc, LazyLock, Mutex as StdMutex,
        atomic::{AtomicU64, Ordering as AtomicOrdering},
    },
};
use tokio::sync::{Mutex, Semaphore};

/// Global Copilot client instance. The mutex protects only slot replacement; callers clone the
/// `Arc` before awaiting RPCs so a wedged CLI cannot block status/stop/start on this mutex.
pub(super) static COPILOT_CLIENT: Lazy<Mutex<Option<Arc<Client>>>> = Lazy::new(|| Mutex::new(None));

pub(super) static COPILOT_START_GATE: Lazy<Semaphore> = Lazy::new(|| Semaphore::new(1));

/// Options the main Copilot client was started with, reused to start nested pool clients.
/// Cleared on backend stop so a checkout that begins entirely after a stop fails fast instead of
/// spawning a pooled CLI process from stale configuration.
pub(super) static COPILOT_START_OPTIONS: Lazy<Mutex<Option<FlowPilotBackendStartOptions>>> =
    Lazy::new(|| Mutex::new(None));

/// Nested specialists share mutable editor state across backends. Each run acquires a gate for its
/// mutation lane, so runs that can touch the same state do not interleave while independent lanes
/// can proceed concurrently. All four agent backends (Bits/rig, GitHub Copilot SDK, Codex CLI, and
/// Claude Code CLI) use the same gate keys.
pub(super) static NESTED_COPILOT_RUN_GATES: Lazy<StdMutex<HashMap<String, Arc<Semaphore>>>> =
    Lazy::new(|| StdMutex::new(HashMap::new()));

/// Gate key for a nested run. The gate exists to protect MUTABLE state from interleaving, so it is
/// keyed by the *lane* a run writes to, never merely by whatever board happens to be in context.
/// The four authoring specialists own disjoint state: FlowScript drafts (`flowpilot_board`), A2UI
/// surfaces (`flowpilot_widget`), tables/overlays (`data_studio_agent`), and the active profile's
/// Home layout (`flowpilot_home`). A widget, data, workflow, and Home build can run concurrently.
/// Sharing a `board:<id>` key across lanes silently made them queue, which is the single largest
/// source of avoidable latency in a build turn.
pub(super) fn nested_copilot_run_gate_key(
    scope: CopilotScope,
    board: Option<&Board>,
    tool_context: Option<&FrontendToolContext>,
) -> String {
    let context_board = || {
        tool_context
            .and_then(|context| context.board_id.clone())
            .filter(|id| !id.trim().is_empty())
    };
    let context_app = || {
        tool_context
            .and_then(|context| context.app_id.clone())
            .filter(|id| !id.trim().is_empty())
    };
    let request = || {
        tool_context
            .and_then(|context| context.parent_request_id.clone())
            .filter(|id| !id.trim().is_empty())
    };
    let lane = |label: &str, target: Option<String>| match target {
        Some(id) => format!("{label}:{id}"),
        None => label.to_string(),
    };

    match scope {
        // Scout and Research mutate nothing, so there is no state for concurrent runs to corrupt and
        // no reason to serialize them. Keying on the owning delegated request — unique per
        // `project_scout` / `research_agent` call — lets the orchestrator fan several out at once,
        // which is the whole point of investigating candidates or questions in parallel. Without
        // this they would all collapse onto one shared gate and run one at a time. Process
        // concurrency stays bounded by the nested CLI pool, and parallel researchers still share one
        // web budget via the turn's `WebResearchSession`.
        CopilotScope::Scout => lane("scout", request()),
        CopilotScope::Research => lane("research", request()),
        // Widget runs author A2UI surfaces through `emit_ui` and hold no FlowScript draft, so they
        // never contend with a board build. They are still serialized per target page/board so two
        // widget runs cannot race the same surface.
        CopilotScope::Frontend => lane(
            "widget",
            board
                .map(|board| board.id.clone())
                .or_else(context_board)
                .or_else(context_app),
        ),
        // Data runs can touch both an overlay and app-level tables. The app is therefore the
        // smallest safe lock scope: two different overlays in one app may still update the same
        // database catalog, while different apps remain independent.
        CopilotScope::DataStudio => lane("data", context_app()),
        // The active desktop profile is process-wide and FrontendToolContext has no profile id.
        // Serialize Home mutations in their own lane. Optimistic fingerprint checks still reject a
        // stale candidate if the profile changes while a run is in progress.
        CopilotScope::Home => lane("home", None),
        // Retained draft stores are board-scoped and their base-fingerprint integrity requires that
        // two runs mutating the same board never interleave. Runs targeting DIFFERENT boards are
        // independent. A board run with no resolved target yet still has to serialize per app, since
        // it may create the app's first board — but it must not serialize against other apps.
        CopilotScope::Board | CopilotScope::Both => lane(
            "board",
            board
                .map(|board| board.id.clone())
                .filter(|id| !id.trim().is_empty())
                .or_else(context_board)
                .or_else(|| context_app().map(|app| format!("unresolved@{app}"))),
        ),
    }
}

/// Resolve the serialization gate for one gate key. Gates whose only owner is the map itself
/// (no permit holder, no queued waiter — both hold an `Arc` clone) are pruned on every lookup so
/// the map stays bounded by the number of concurrently active/queued nested runs.
pub(super) fn nested_copilot_run_gate(key: &str) -> Arc<Semaphore> {
    let mut gates = NESTED_COPILOT_RUN_GATES
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner());
    gates.retain(|_, gate| Arc::strong_count(gate) > 1);
    gates
        .entry(key.to_string())
        .or_insert_with(|| Arc::new(Semaphore::new(1)))
        .clone()
}

pub(super) async fn acquire_nested_copilot_run_permit(
    gate: Arc<Semaphore>,
    cancellation: CancellationToken,
) -> Result<tokio::sync::OwnedSemaphorePermit, String> {
    tokio::select! {
        permit = gate.acquire_owned() => {
            permit.map_err(|_| "The nested Copilot run gate was closed".to_string())
        }
        _ = cancellation.cancelled() => {
            Err("FlowPilot Copilot run was cancelled before it started".to_string())
        }
    }
}

/// Dedicated CLI processes for nested FlowPilot specialist runs spawned while a parent Copilot
/// session is mid-turn. The copilot CLI serializes
/// requests within one process: a `session.create` sent while the parent session has a pending
/// tool call is never answered, deadlocking the sub-run until the tool bridge times out. Separate
/// processes isolate nested sessions completely. This is a PER-PROCESS constraint, so nested runs
/// use a small pool: clients start lazily with the same options as the main client (up to
/// `NESTED_COPILOT_POOL_SIZE`) and idle processes are reused. A checked-out client is exclusively
/// owned by one run, preserving one-session-at-a-time per process by construction.
/// Sized for the widest fan-out a plan realistically produces in one wavefront: the four authoring
/// lanes (board / widget / data / home) plus a few independent boards, or a scout fan-out across
/// several candidates. Too small a pool silently converts a parallel plan back into a sequential
/// one, since the excess runs block on a slot while holding their turn open.
pub(super) const NESTED_COPILOT_POOL_SIZE: usize = 6;

pub(super) struct NestedCopilotPool {
    pub(super) slots: Arc<Semaphore>,
    pub(super) idle: StdMutex<Vec<Arc<Client>>>,
    /// Every live pooled process, idle or checked out. Quarantine removes an entry so the owning
    /// lease drops the process instead of returning it to `idle`; backend stop drains everything.
    pub(super) registered: StdMutex<Vec<Arc<Client>>>,
    /// Bumped by every `drain` (under the `registered` lock). A client whose startup began before
    /// a drain must not register into the drained pool, or backend stop would leave a live CLI
    /// process behind that the stop path never saw. Lock order is always `registered` → `idle`.
    pub(super) drain_epoch: AtomicU64,
}

impl NestedCopilotPool {
    pub(super) fn new(size: usize) -> Self {
        Self {
            slots: Arc::new(Semaphore::new(size)),
            idle: StdMutex::new(Vec::new()),
            registered: StdMutex::new(Vec::new()),
            drain_epoch: AtomicU64::new(0),
        }
    }

    pub(super) fn take_idle(&self) -> Option<Arc<Client>> {
        self.idle
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
            .pop()
    }

    pub(super) fn epoch(&self) -> u64 {
        self.drain_epoch.load(AtomicOrdering::SeqCst)
    }

    /// Register a freshly started client, but only if no drain happened since `observed_epoch`
    /// was captured. Returns whether the client joined the pool; a rejected client must be
    /// stopped by the caller because no pool teardown path will ever see it.
    pub(super) fn register_started(&self, client: Arc<Client>, observed_epoch: u64) -> bool {
        let mut registered = self
            .registered
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        if self.drain_epoch.load(AtomicOrdering::SeqCst) != observed_epoch {
            return false;
        }
        registered.push(client);
        true
    }

    pub(super) fn deregister(&self, client: &Arc<Client>) -> bool {
        let mut registered = self
            .registered
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        let before = registered.len();
        registered.retain(|entry| !Arc::ptr_eq(entry, client));
        let removed = registered.len() != before;
        self.idle
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
            .retain(|entry| !Arc::ptr_eq(entry, client));
        removed
    }

    #[cfg_attr(not(test), allow(dead_code))]
    pub(super) fn is_registered(&self, client: &Arc<Client>) -> bool {
        self.registered
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
            .iter()
            .any(|entry| Arc::ptr_eq(entry, client))
    }

    pub(super) fn return_to_idle(&self, client: Arc<Client>) {
        // Hold the `registered` lock across the membership check AND the idle push: a concurrent
        // drain would otherwise interleave between them and leave a stopped client in `idle`.
        let registered = self
            .registered
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        if registered.iter().any(|entry| Arc::ptr_eq(entry, &client)) {
            self.idle
                .lock()
                .unwrap_or_else(|poisoned| poisoned.into_inner())
                .push(client);
        }
    }

    pub(super) fn drain(&self) -> Vec<Arc<Client>> {
        let mut registered = self
            .registered
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        self.drain_epoch.fetch_add(1, AtomicOrdering::SeqCst);
        self.idle
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
            .clear();
        registered.drain(..).collect()
    }
}

pub(super) static NESTED_COPILOT_POOL: Lazy<NestedCopilotPool> =
    Lazy::new(|| NestedCopilotPool::new(NESTED_COPILOT_POOL_SIZE));

/// Exclusive checkout of one pooled nested CLI process. Dropping the lease returns the client to
/// the idle pool unless it was quarantined/deregistered first; the pool slot frees either way so
/// a replacement process can start lazily on the next checkout.
pub(super) struct NestedCopilotClientLease {
    pub(super) pool: &'static NestedCopilotPool,
    pub(super) client: Arc<Client>,
    pub(super) _slot: tokio::sync::OwnedSemaphorePermit,
}

impl NestedCopilotClientLease {
    pub(super) fn client(&self) -> Arc<Client> {
        self.client.clone()
    }

    /// Remove the leased client from the pool without stopping it, for drop paths that cannot
    /// await session cleanup: leaking one process is safe, re-pooling a client whose previous
    /// session may still be pending is not.
    pub(super) fn deregister(&self) {
        self.pool.deregister(&self.client);
    }
}

impl Drop for NestedCopilotClientLease {
    fn drop(&mut self) {
        self.pool.return_to_idle(self.client.clone());
    }
}

pub(super) async fn checkout_nested_copilot_client_from<F, Fut>(
    pool: &'static NestedCopilotPool,
    cancellation: CancellationToken,
    start_client: F,
) -> Result<NestedCopilotClientLease, String>
where
    F: FnOnce() -> Fut,
    Fut: std::future::Future<Output = Result<Arc<Client>, String>>,
{
    let slot = tokio::select! {
        permit = pool.slots.clone().acquire_owned() => {
            permit.map_err(|_| "The nested Copilot process pool was closed".to_string())?
        }
        _ = cancellation.cancelled() => {
            return Err("FlowPilot Copilot run was cancelled before it started".to_string());
        }
    };
    if let Some(client) = pool.take_idle() {
        return Ok(NestedCopilotClientLease {
            pool,
            client,
            _slot: slot,
        });
    }
    let observed_epoch = pool.epoch();
    let client = start_client().await?;
    if !pool.register_started(client.clone(), observed_epoch) {
        let _ = tokio::time::timeout(SDK_CHAT_ABORT_TIMEOUT, client.force_stop()).await;
        return Err(
            "The nested Copilot process pool was drained while a replacement client was starting"
                .to_string(),
        );
    }
    Ok(NestedCopilotClientLease {
        pool,
        client,
        _slot: slot,
    })
}

pub(super) async fn nested_copilot_start_options() -> Result<FlowPilotBackendStartOptions, String> {
    COPILOT_START_OPTIONS
        .lock()
        .await
        .clone()
        .ok_or_else(|| "Copilot client not started".to_string())
}

pub(super) async fn checkout_nested_copilot_client(
    cancellation: CancellationToken,
) -> Result<NestedCopilotClientLease, String> {
    checkout_nested_copilot_client_from(&NESTED_COPILOT_POOL, cancellation, || async {
        let options = nested_copilot_start_options().await?;
        flowpilot_debug_log!("[copilot_sdk_chat] starting dedicated CLI process for a nested run");
        Ok(Arc::new(build_and_start_copilot_client(&options).await?))
    })
    .await
}

/// The per-process serialization constraint that forced nested runs into a pool applies just as
/// hard to two TOP-LEVEL turns: the user can now have several chat replies generating at once, and
/// a second `session.create` on a process whose first session is mid-tool-call is never answered.
///
/// The shared `COPILOT_CLIENT` stays the fast path — a single turn, which is still the common
/// case, spawns nothing extra. This gate makes that claim exclusive; concurrent turns fall through
/// to their own pool of processes instead of wedging on the shared one.
pub(super) static COPILOT_SINGLETON_CLIENT_GATE: LazyLock<Arc<Semaphore>> =
    LazyLock::new(|| Arc::new(Semaphore::new(1)));

/// Matches the frontend's `MAX_CONCURRENT_GLOBAL_CHAT_RUNS`, minus the shared client that serves
/// the first turn. Sends past the frontend cap are queued, so this never has to grow.
const TOP_LEVEL_COPILOT_POOL_SIZE: usize = 3;

pub(super) static TOP_LEVEL_COPILOT_POOL: Lazy<NestedCopilotPool> =
    Lazy::new(|| NestedCopilotPool::new(TOP_LEVEL_COPILOT_POOL_SIZE));

pub(super) async fn checkout_top_level_copilot_client(
    cancellation: CancellationToken,
) -> Result<NestedCopilotClientLease, String> {
    checkout_nested_copilot_client_from(&TOP_LEVEL_COPILOT_POOL, cancellation, || async {
        let options = nested_copilot_start_options().await?;
        flowpilot_debug_log!(
            "[copilot_sdk_chat] starting dedicated CLI process for a concurrent top-level run"
        );
        Ok(Arc::new(build_and_start_copilot_client(&options).await?))
    })
    .await
}

/// Drop a client that must not be reused (its previous session may still be pending). Checks both
/// pools — a lease can come from either, and the caller does not track which.
pub(super) async fn quarantine_nested_copilot_client(client: &Arc<Client>) {
    if NESTED_COPILOT_POOL.deregister(client) || TOP_LEVEL_COPILOT_POOL.deregister(client) {
        let _ = tokio::time::timeout(SDK_CHAT_ABORT_TIMEOUT, client.force_stop()).await;
    }
}

pub(super) async fn build_and_start_copilot_client(
    options: &FlowPilotBackendStartOptions,
) -> Result<Client, String> {
    let mut builder = Client::builder()
        .use_stdio(options.use_stdio)
        .log_level(LogLevel::Error);

    if let Some(url) = options.cli_url.clone() {
        builder = builder.cli_url(url);
    } else if let Some(cli_path) = find_copilot_cli_path() {
        builder = builder.cli_path(cli_path);
    }

    // In production builds the app inherits a minimal PATH that often does
    // not include directories where `node` lives. The copilot CLI is a
    // Node.js script (#!/usr/bin/env node), so the spawned process needs
    // node on its PATH. Augment PATH with common Node/tool directories.
    builder = builder.env("PATH", augmented_path());

    let client = builder
        .build()
        .map_err(|e| format!("Failed to build Copilot client: {}", e))?;
    match tokio::time::timeout(SDK_CONTROL_RPC_TIMEOUT, client.start()).await {
        Ok(Ok(())) => {}
        Ok(Err(error)) => {
            let _ = tokio::time::timeout(SDK_CHAT_ABORT_TIMEOUT, client.force_stop()).await;
            return Err(format!("Failed to start Copilot client: {error}"));
        }
        Err(_) => {
            let _ = tokio::time::timeout(SDK_CHAT_ABORT_TIMEOUT, client.force_stop()).await;
            return Err(format!(
                "Copilot client startup exceeded {} seconds",
                SDK_CONTROL_RPC_TIMEOUT.as_secs()
            ));
        }
    }
    Ok(client)
}

pub(super) static EXTERNAL_AGENT_BACKENDS: Lazy<
    Mutex<std::collections::HashSet<FlowPilotAgentBackendKind>>,
> = Lazy::new(|| Mutex::new(std::collections::HashSet::new()));
