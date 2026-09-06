//! Backend lifecycle and failure telemetry.

use super::backend_types::FlowPilotAgentBackendKind;
use std::time::Instant;
use tauri::AppHandle;

// =============================================================================
// Agent backend telemetry (aggregate-only)
//
// The external agent CLIs run on the user's machine against their own prompts,
// files and credentials, so nothing observed here may be reported verbatim. Only
// the closed vocabularies below plus a duration ever reach the telemetry buffer:
// no path, argument, environment value, prompt, model output, stderr text or
// user name. Failures are classified into `AGENT_ERROR_CLASSES` and the matched
// constant — never the message that produced it — is what gets emitted.
// =============================================================================

const AGENT_BACKEND_LIFECYCLE_EVENT: &str = "agent_backend_start";

const AGENT_BACKEND_ERROR_EVENT: &str = "agent_backend_error";

pub(super) const AGENT_STAGE_SPAWN: &str = "spawn";

pub(super) const AGENT_STAGE_AUTH: &str = "auth";

pub(super) const AGENT_STAGE_MODELS: &str = "models";

pub(super) const AGENT_STAGE_RUN: &str = "run";

pub(super) const AGENT_STAGE_STOP: &str = "stop";

/// Ordered failure vocabulary. The first class whose markers appear in the
/// lowercased failure text wins, so broader markers must come last: timeouts
/// wrap whichever operation they interrupted, and `no such file` would otherwise
/// swallow `model not found`.
pub(super) const AGENT_ERROR_CLASSES: &[(&str, &[&str])] = &[
    (
        "timeout",
        &["timed out", "timeout", "deadline exceeded", "etimedout"],
    ),
    (
        "unsupported_model",
        &[
            "unsupported model",
            "unknown model",
            "model not found",
            "invalid model",
            "no such model",
            "model is not supported",
        ],
    ),
    (
        "auth_expired",
        &[
            "token expired",
            "session expired",
            "credentials expired",
            "expired credential",
            "auth expired",
            "re-authenticate",
            "reauthenticate",
            "refresh token",
        ],
    ),
    (
        "auth_required",
        &[
            "not authenticated",
            "unauthenticated",
            "authentication required",
            "authentication failed",
            "requires authentication",
            "unauthorized",
            "forbidden",
            "not logged in",
            "login required",
            "please log in",
            "please sign in",
            "invalid api key",
            "missing api key",
            "http 401",
            "http 403",
        ],
    ),
    (
        "permission_denied",
        &[
            "permission denied",
            "access is denied",
            "operation not permitted",
            "eacces",
            "eperm",
        ],
    ),
    (
        "binary_not_found",
        &[
            "was not found",
            "not installed",
            "no such file",
            "enoent",
            "command not found",
            "cannot find the file",
            "cannot find the path",
            "is not recognized as an internal or external command",
        ],
    ),
    (
        "non_zero_exit",
        &[
            "exited with",
            "exit code",
            "exit status",
            "non-zero exit",
            "killed by signal",
            "terminated by signal",
        ],
    ),
    (
        "protocol_error",
        &[
            "protocol",
            "unexpected eof",
            "broken pipe",
            "invalid json",
            "malformed",
            "jsonrpc",
            "json-rpc",
            "handshake",
            "stream closed",
            "connection closed",
            "connection reset",
            "unexpected message",
            "parse error",
            "decode error",
        ],
    ),
];

pub(super) fn backend_label(kind: FlowPilotAgentBackendKind) -> &'static str {
    match kind {
        FlowPilotAgentBackendKind::GithubCopilot => "github_copilot",
        FlowPilotAgentBackendKind::Codex => "codex",
        FlowPilotAgentBackendKind::ClaudeCode => "claude_code",
    }
}

/// Maps a backend failure onto the closed error vocabulary. The returned value is
/// always a `'static` constant, so the failure text itself cannot escape.
pub(super) fn classify_agent_error(error: &str) -> &'static str {
    let normalized = error.to_ascii_lowercase();
    for (class, markers) in AGENT_ERROR_CLASSES {
        if markers.iter().any(|marker| normalized.contains(marker)) {
            return class;
        }
    }
    "unknown"
}

pub(super) fn agent_backend_lifecycle_props(
    backend: FlowPilotAgentBackendKind,
    stage: &'static str,
    error_kind: Option<&'static str>,
    duration_ms: u64,
) -> serde_json::Value {
    let mut props = serde_json::Map::new();
    props.insert("backend".to_string(), backend_label(backend).into());
    props.insert("stage".to_string(), stage.into());
    props.insert(
        "outcome".to_string(),
        if error_kind.is_some() { "error" } else { "ok" }.into(),
    );
    props.insert("duration_ms".to_string(), duration_ms.into());
    if let Some(error_kind) = error_kind {
        props.insert("error_kind".to_string(), error_kind.into());
    }
    serde_json::Value::Object(props)
}

pub(super) fn agent_backend_error_props(
    backend: FlowPilotAgentBackendKind,
    stage: &'static str,
    error_kind: &'static str,
    duration_ms: Option<u64>,
) -> serde_json::Value {
    let mut props = serde_json::Map::new();
    props.insert("backend".to_string(), backend_label(backend).into());
    props.insert("stage".to_string(), stage.into());
    props.insert("error_kind".to_string(), error_kind.into());
    if let Some(duration_ms) = duration_ms {
        props.insert("duration_ms".to_string(), duration_ms.into());
    }
    serde_json::Value::Object(props)
}

/// Fire-and-forget capture of one backend stage outcome. Detached onto the
/// runtime so neither the buffer write nor a panic inside it can reach the
/// backend call path, and a silent no-op without usage consent. Without a live
/// runtime handle nothing is recorded rather than panicking, so instrumentation
/// can never become a failure mode of the backend it observes.
fn record_agent_backend_stage(
    app_handle: &AppHandle,
    backend: FlowPilotAgentBackendKind,
    stage: &'static str,
    error: Option<&str>,
    started: Instant,
) {
    let Ok(runtime) = tokio::runtime::Handle::try_current() else {
        return;
    };
    let duration_ms = u64::try_from(started.elapsed().as_millis()).unwrap_or(u64::MAX);
    let error_kind = error.map(classify_agent_error);
    let lifecycle = agent_backend_lifecycle_props(backend, stage, error_kind, duration_ms);
    let failure =
        error_kind.map(|kind| agent_backend_error_props(backend, stage, kind, Some(duration_ms)));
    let app_handle = app_handle.clone();
    runtime.spawn(async move {
        crate::functions::telemetry::track(
            &app_handle,
            AGENT_BACKEND_LIFECYCLE_EVENT,
            Some(lifecycle),
        )
        .await;
        if let Some(failure) = failure {
            crate::functions::telemetry::track(
                &app_handle,
                AGENT_BACKEND_ERROR_EVENT,
                Some(failure),
            )
            .await;
        }
    });
}

/// Awaits one backend stage and records its aggregate outcome afterwards. The
/// stage result is returned untouched, so instrumentation can neither alter
/// control flow nor add a failure mode.
pub(super) async fn instrumented_agent_stage<T, F>(
    app_handle: &AppHandle,
    backend: FlowPilotAgentBackendKind,
    stage: &'static str,
    stage_future: F,
) -> Result<T, String>
where
    F: std::future::Future<Output = Result<T, String>>,
{
    let started = Instant::now();
    let result = stage_future.await;
    record_agent_backend_stage(
        app_handle,
        backend,
        stage,
        result.as_ref().err().map(String::as_str),
        started,
    );
    result
}
