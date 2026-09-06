//! External CLI version and authentication probes.

use super::backend_types::FlowPilotAgentBackendKind;
use super::cli_resolution::{CliResolution, augmented_path_with_dirs};
use super::stream_events::append_bounded_tail;
use std::{path::PathBuf, time::Duration};

pub(super) async fn probe_external_agent_cli(
    kind: FlowPilotAgentBackendKind,
    executable: &std::path::Path,
    path_dirs: &[PathBuf],
) -> Result<String, String> {
    let output = tokio::time::timeout(
        Duration::from_secs(4),
        tokio::process::Command::new(executable)
            .arg("--version")
            .env("PATH", augmented_path_with_dirs(path_dirs))
            .kill_on_drop(true)
            .output(),
    )
    .await
    .map_err(|_| format!("{} CLI probe timed out after 4s", kind.label()))?
    .map_err(|e| {
        format!(
            "Failed to run {} CLI at {}: {e}",
            kind.label(),
            executable.display()
        )
    })?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        let stdout = String::from_utf8_lossy(&output.stdout);
        let detail = if stderr.trim().is_empty() {
            stdout.trim()
        } else {
            stderr.trim()
        };
        return Err(format!(
            "{} --version exited with status {}{}",
            kind.label(),
            output.status,
            if detail.is_empty() {
                String::new()
            } else {
                format!(": {detail}")
            }
        ));
    }

    let stdout = String::from_utf8_lossy(&output.stdout).trim().to_string();
    if stdout.is_empty() {
        Ok(format!("{} CLI responded to --version", kind.label()))
    } else {
        Ok(stdout)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct ExternalAgentAuthProbe {
    pub(super) authenticated: bool,
    pub(super) login: Option<String>,
    pub(super) detail: String,
}

fn process_output_detail(output: &std::process::Output) -> String {
    let stderr = String::from_utf8_lossy(&output.stderr);
    let stdout = String::from_utf8_lossy(&output.stdout);
    let detail = if stderr.trim().is_empty() {
        stdout.trim()
    } else {
        stderr.trim()
    };
    flow_like::flow::copilot::stream::safe_text_preview(detail, 1_200)
}

pub(super) async fn read_bounded_process_stderr(mut stderr: tokio::process::ChildStderr) -> String {
    use tokio::io::AsyncReadExt;

    let mut retained = String::new();
    let mut buffer = [0u8; 4 * 1024];
    loop {
        match stderr.read(&mut buffer).await {
            Ok(0) => break,
            Ok(read) => append_bounded_tail(
                &mut retained,
                &String::from_utf8_lossy(&buffer[..read]),
                32 * 1024,
            ),
            Err(error) => {
                append_bounded_tail(
                    &mut retained,
                    &format!("\n[failed reading stderr: {error}]"),
                    32 * 1024,
                );
                break;
            }
        }
    }
    retained.trim().to_string()
}

pub(super) async fn stop_external_discovery_process(
    child: &mut tokio::process::Child,
    mut stderr_handle: tokio::task::JoinHandle<String>,
) -> String {
    let _ = child.start_kill();
    match tokio::time::timeout(Duration::from_secs(1), async {
        let (_, stderr) = tokio::join!(child.wait(), &mut stderr_handle);
        stderr.unwrap_or_default()
    })
    .await
    {
        Ok(stderr) => stderr,
        Err(_) => {
            stderr_handle.abort();
            String::new()
        }
    }
}

pub(super) fn claude_auth_probe_from_success(
    stdout: &str,
) -> Result<ExternalAgentAuthProbe, String> {
    let parsed = serde_json::from_str::<serde_json::Value>(stdout.trim()).map_err(|error| {
        format!("Claude Code auth status protocol error: expected JSON with `loggedIn`: {error}")
    })?;
    let authenticated = parsed
        .get("loggedIn")
        .and_then(serde_json::Value::as_bool)
        .ok_or_else(|| {
            "Claude Code auth status protocol error: response omitted boolean `loggedIn`"
                .to_string()
        })?;
    let login = parsed
        .get("email")
        .or_else(|| parsed.get("login"))
        .or_else(|| parsed.get("account"))
        .and_then(serde_json::Value::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(str::to_string);

    let mut summary = Vec::new();
    for (label, keys) in [
        ("method", &["authMethod", "auth_method", "method"][..]),
        (
            "subscription",
            &["subscriptionType", "subscription_type"][..],
        ),
        ("provider", &["apiProvider", "api_provider"][..]),
    ] {
        if let Some(detail) = keys
            .iter()
            .find_map(|key| parsed.get(*key))
            .and_then(serde_json::Value::as_str)
            .map(str::trim)
            .filter(|detail| !detail.is_empty())
        {
            summary.push(format!("{label}: {detail}"));
        }
    }

    Ok(ExternalAgentAuthProbe {
        authenticated,
        login,
        detail: if summary.is_empty() {
            if authenticated {
                "Claude Code is signed in.".to_string()
            } else {
                "Claude Code reported that it is not signed in.".to_string()
            }
        } else {
            format!(
                "Claude Code is {} ({}).",
                if authenticated {
                    "signed in"
                } else {
                    "not signed in"
                },
                summary.join(", ")
            )
        },
    })
}

pub(super) fn external_agent_auth_output_is_signed_out(
    kind: FlowPilotAgentBackendKind,
    stdout: &[u8],
    detail: &str,
) -> bool {
    let claude_reported_signed_out = kind == FlowPilotAgentBackendKind::ClaudeCode
        && serde_json::from_slice::<serde_json::Value>(stdout)
            .ok()
            .and_then(|value| value.get("loggedIn").and_then(serde_json::Value::as_bool))
            == Some(false);
    let normalized = detail.to_ascii_lowercase();
    claude_reported_signed_out
        || [
            "not logged in",
            "not signed in",
            "logged out",
            "login required",
            "sign in required",
            "authentication required",
            "no stored credentials",
            "credentials not found",
        ]
        .iter()
        .any(|marker| normalized.contains(marker))
}

pub(super) async fn probe_external_agent_auth(
    kind: FlowPilotAgentBackendKind,
    cli: &CliResolution,
) -> Result<ExternalAgentAuthProbe, String> {
    let args: &[&str] = match kind {
        FlowPilotAgentBackendKind::Codex => &["login", "status"],
        FlowPilotAgentBackendKind::ClaudeCode => &["auth", "status"],
        FlowPilotAgentBackendKind::GithubCopilot => {
            return Err("GitHub Copilot authentication uses the SDK status API.".to_string());
        }
    };
    let output = tokio::time::timeout(
        Duration::from_secs(6),
        tokio::process::Command::new(&cli.executable)
            .args(args)
            .current_dir(std::env::temp_dir())
            .env("PATH", augmented_path_with_dirs(&cli.path_dirs))
            .kill_on_drop(true)
            .output(),
    )
    .await
    .map_err(|_| {
        format!(
            "{} authentication status check timed out after 6 seconds",
            kind.label()
        )
    })?
    .map_err(|error| {
        format!(
            "Failed to run {} authentication status check at {}: {error}",
            kind.label(),
            cli.executable.display()
        )
    })?;

    if !output.status.success() {
        let detail = process_output_detail(&output);
        let claude_reported_signed_out = kind == FlowPilotAgentBackendKind::ClaudeCode
            && serde_json::from_slice::<serde_json::Value>(&output.stdout)
                .ok()
                .and_then(|value| value.get("loggedIn").and_then(serde_json::Value::as_bool))
                == Some(false);
        let known_signed_out =
            external_agent_auth_output_is_signed_out(kind, &output.stdout, &detail);
        if known_signed_out {
            if claude_reported_signed_out {
                let mut auth =
                    claude_auth_probe_from_success(&String::from_utf8_lossy(&output.stdout))?;
                auth.login = None;
                return Ok(auth);
            }
            return Ok(ExternalAgentAuthProbe {
                authenticated: false,
                login: None,
                detail: if detail.is_empty() {
                    format!("{} is not signed in.", kind.label())
                } else {
                    detail
                },
            });
        }

        return Err(if detail.is_empty() {
            format!(
                "{} authentication status command exited with {}",
                kind.label(),
                output.status
            )
        } else {
            format!(
                "{} authentication status command exited with {}: {detail}",
                kind.label(),
                output.status
            )
        });
    }

    let stdout = String::from_utf8_lossy(&output.stdout);
    match kind {
        FlowPilotAgentBackendKind::Codex => {
            let detail = process_output_detail(&output);
            Ok(ExternalAgentAuthProbe {
                authenticated: true,
                login: None,
                detail: if detail.is_empty() {
                    "Codex is signed in.".to_string()
                } else {
                    flow_like::flow::copilot::stream::safe_text_preview(&detail, 600)
                },
            })
        }
        FlowPilotAgentBackendKind::ClaudeCode => claude_auth_probe_from_success(stdout.as_ref()),
        FlowPilotAgentBackendKind::GithubCopilot => unreachable!(),
    }
}
