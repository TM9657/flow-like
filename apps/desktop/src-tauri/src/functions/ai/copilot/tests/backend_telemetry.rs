use super::*;

#[test]
fn agent_backend_labels_are_stable_snake_case() {
    assert_eq!(
        backend_label(FlowPilotAgentBackendKind::ClaudeCode),
        "claude_code"
    );
    assert_eq!(backend_label(FlowPilotAgentBackendKind::Codex), "codex");
    assert_eq!(
        backend_label(FlowPilotAgentBackendKind::GithubCopilot),
        "github_copilot"
    );
}

#[test]
fn agent_errors_map_onto_the_closed_vocabulary() {
    assert_eq!(
        classify_agent_error(
            "Claude Code CLI was not found. Install it or set CLAUDE_CODE_CLI_PATH to its executable path."
        ),
        "binary_not_found"
    );
    assert_eq!(
        classify_agent_error("spawn codex: No such file or directory (os error 2)"),
        "binary_not_found"
    );
    assert_eq!(
        classify_agent_error("failed to spawn agent: permission denied (os error 13)"),
        "permission_denied"
    );
    assert_eq!(
        classify_agent_error("Not authenticated. Run the CLI login flow first."),
        "auth_required"
    );
    assert_eq!(
        classify_agent_error("stored credentials expired; please re-authenticate"),
        "auth_expired"
    );
    assert_eq!(
        classify_agent_error("Codex CLI probe timed out after 5s"),
        "timeout"
    );
    assert_eq!(
        classify_agent_error("Codex --version exited with status exit status: 127"),
        "non_zero_exit"
    );
    assert_eq!(
        classify_agent_error("app-server returned a malformed JSON-RPC frame"),
        "protocol_error"
    );
    assert_eq!(
        classify_agent_error("unsupported model: opus-does-not-exist"),
        "unsupported_model"
    );
    assert_eq!(
        classify_agent_error("something nobody predicted"),
        "unknown"
    );
    assert_eq!(classify_agent_error(""), "unknown");
}

#[test]
fn agent_error_classification_never_echoes_the_failure_text() {
    let sensitive = "Failed to run Claude Code CLI at /home/alice/.local/bin/claude --resume s3cr3t: No such file or directory";
    let error_kind = classify_agent_error(sensitive);
    assert_eq!(error_kind, "binary_not_found");

    let rendered = agent_backend_lifecycle_props(
        FlowPilotAgentBackendKind::ClaudeCode,
        AGENT_STAGE_SPAWN,
        Some(error_kind),
        12,
    )
    .to_string();
    assert!(!rendered.contains("/home/alice"));
    assert!(!rendered.contains("--resume"));
    assert!(!rendered.contains("s3cr3t"));
}

#[test]
fn agent_backend_props_carry_only_aggregate_keys() {
    let ok = agent_backend_lifecycle_props(
        FlowPilotAgentBackendKind::ClaudeCode,
        AGENT_STAGE_SPAWN,
        None,
        42,
    );
    assert_eq!(
        sorted_prop_keys(&ok),
        vec!["backend", "duration_ms", "outcome", "stage"]
    );
    assert_eq!(ok["backend"], "claude_code");
    assert_eq!(ok["stage"], "spawn");
    assert_eq!(ok["outcome"], "ok");
    assert_eq!(ok["duration_ms"], 42);

    let failed = agent_backend_lifecycle_props(
        FlowPilotAgentBackendKind::Codex,
        AGENT_STAGE_RUN,
        Some("timeout"),
        7,
    );
    assert_eq!(
        sorted_prop_keys(&failed),
        vec!["backend", "duration_ms", "error_kind", "outcome", "stage"]
    );
    assert_eq!(failed["backend"], "codex");
    assert_eq!(failed["outcome"], "error");
    assert_eq!(failed["error_kind"], "timeout");

    let timed = agent_backend_error_props(
        FlowPilotAgentBackendKind::GithubCopilot,
        AGENT_STAGE_AUTH,
        "auth_required",
        Some(3),
    );
    assert_eq!(
        sorted_prop_keys(&timed),
        vec!["backend", "duration_ms", "error_kind", "stage"]
    );
    assert_eq!(timed["backend"], "github_copilot");
    assert_eq!(timed["stage"], "auth");

    let untimed = agent_backend_error_props(
        FlowPilotAgentBackendKind::Codex,
        AGENT_STAGE_STOP,
        "unknown",
        None,
    );
    assert_eq!(
        sorted_prop_keys(&untimed),
        vec!["backend", "error_kind", "stage"]
    );
}

#[test]
fn agent_backend_stage_vocabulary_is_closed() {
    for stage in [
        AGENT_STAGE_SPAWN,
        AGENT_STAGE_AUTH,
        AGENT_STAGE_MODELS,
        AGENT_STAGE_RUN,
        AGENT_STAGE_STOP,
    ] {
        assert!(matches!(
            stage,
            "spawn" | "auth" | "models" | "run" | "stop"
        ));
    }
    for (class, markers) in AGENT_ERROR_CLASSES {
        assert!(matches!(
            *class,
            "binary_not_found"
                | "permission_denied"
                | "auth_required"
                | "auth_expired"
                | "timeout"
                | "non_zero_exit"
                | "protocol_error"
                | "unsupported_model"
        ));
        for marker in *markers {
            assert_eq!(classify_agent_error(marker), *class);
        }
    }
}

#[test]
fn resolves_matching_copilot_app_contexts() {
    assert_eq!(
        resolve_copilot_app_id(Some(" app-1 "), Some("app-1"), None).unwrap(),
        Some("app-1".to_string())
    );
}

#[test]
fn rejects_conflicting_copilot_app_contexts() {
    assert!(resolve_copilot_app_id(Some("app-1"), None, Some("app-2")).is_err());
}
