//! Tauri backend commands and specialized SDK sessions.

use super::backend_types::{
    CopilotAuthStatus, CopilotModelInfo, FlowPilotAgentBackendKind, FlowPilotBackendStatus,
};
use super::backends::{FlowPilotBackendStartOptions, agent_backend, parse_agent_backend};
use super::client_pool::COPILOT_CLIENT;
use super::runtime::SDK_CONTROL_RPC_TIMEOUT;
use super::telemetry::{
    AGENT_STAGE_AUTH, AGENT_STAGE_MODELS, AGENT_STAGE_SPAWN, AGENT_STAGE_STOP,
    instrumented_agent_stage,
};
use serde::{Deserialize, Serialize};
use tauri::AppHandle;

#[tauri::command]
pub async fn flowpilot_agent_backend_start(
    app_handle: AppHandle,
    backend: String,
    use_stdio: Option<bool>,
    cli_url: Option<String>,
) -> Result<(), String> {
    let kind = parse_agent_backend(backend)?;
    let backend = agent_backend(kind);
    let options = FlowPilotBackendStartOptions {
        use_stdio: use_stdio.unwrap_or(true),
        cli_url,
        app_handle: Some(app_handle.clone()),
    };
    instrumented_agent_stage(&app_handle, kind, AGENT_STAGE_SPAWN, backend.start(options)).await
}

#[tauri::command]
pub async fn flowpilot_agent_backend_stop(
    app_handle: AppHandle,
    backend: String,
) -> Result<(), String> {
    let kind = parse_agent_backend(backend)?;
    let backend = agent_backend(kind);
    instrumented_agent_stage(&app_handle, kind, AGENT_STAGE_STOP, backend.stop()).await
}

#[tauri::command]
pub async fn flowpilot_agent_backend_is_running(backend: String) -> Result<bool, String> {
    agent_backend(parse_agent_backend(backend)?)
        .is_running()
        .await
}

#[tauri::command]
pub async fn flowpilot_agent_backend_list_models(
    app_handle: AppHandle,
    backend: String,
) -> Result<Vec<CopilotModelInfo>, String> {
    let kind = parse_agent_backend(backend)?;
    let backend = agent_backend(kind);
    instrumented_agent_stage(
        &app_handle,
        kind,
        AGENT_STAGE_MODELS,
        backend.list_models(Some(&app_handle)),
    )
    .await
}

#[tauri::command]
pub async fn flowpilot_agent_backend_get_auth_status(
    app_handle: AppHandle,
    backend: String,
) -> Result<CopilotAuthStatus, String> {
    let kind = parse_agent_backend(backend)?;
    let backend = agent_backend(kind);
    instrumented_agent_stage(
        &app_handle,
        kind,
        AGENT_STAGE_AUTH,
        backend.get_auth_status(Some(&app_handle)),
    )
    .await
}

#[tauri::command]
pub async fn flowpilot_agent_backend_status(
    app_handle: AppHandle,
    backend: String,
) -> Result<FlowPilotBackendStatus, String> {
    Ok(agent_backend(parse_agent_backend(backend)?)
        .status(Some(&app_handle))
        .await)
}

#[tauri::command]
pub async fn flowpilot_agent_backend_list(
    app_handle: AppHandle,
) -> Result<Vec<FlowPilotBackendStatus>, String> {
    let mut statuses = Vec::new();
    for backend in [
        FlowPilotAgentBackendKind::GithubCopilot,
        FlowPilotAgentBackendKind::Codex,
        FlowPilotAgentBackendKind::ClaudeCode,
    ] {
        statuses.push(agent_backend(backend).status(Some(&app_handle)).await);
    }
    Ok(statuses)
}

/// Start the GitHub Copilot SDK client
#[tauri::command]
pub async fn copilot_sdk_start(
    app_handle: AppHandle,
    use_stdio: Option<bool>,
    cli_url: Option<String>,
) -> Result<(), String> {
    flowpilot_agent_backend_start(app_handle, "github-copilot".to_string(), use_stdio, cli_url)
        .await
}

/// Stop the GitHub Copilot SDK client
#[tauri::command]
pub async fn copilot_sdk_stop(app_handle: AppHandle) -> Result<(), String> {
    flowpilot_agent_backend_stop(app_handle, "github-copilot".to_string()).await
}

/// Check if the Copilot SDK client is running
#[tauri::command]
pub async fn copilot_sdk_is_running() -> Result<bool, String> {
    flowpilot_agent_backend_is_running("github-copilot".to_string()).await
}

/// List available GitHub Copilot models
#[tauri::command]
pub async fn copilot_sdk_list_models(
    app_handle: AppHandle,
) -> Result<Vec<CopilotModelInfo>, String> {
    flowpilot_agent_backend_list_models(app_handle, "github-copilot".to_string()).await
}

/// Get GitHub Copilot authentication status
#[tauri::command]
pub async fn copilot_sdk_get_auth_status(
    app_handle: AppHandle,
) -> Result<CopilotAuthStatus, String> {
    flowpilot_agent_backend_get_auth_status(app_handle, "github-copilot".to_string()).await
}

// =============================================================================
// Specialized Agents Configuration
// =============================================================================

/// Specialized agent type
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum SpecializedAgentType {
    General,
    Frontend,
    Backend,
}

/// System prompts for specialized agents — delegate to the shared prompts module
/// in `flow_like::copilot::prompts` for consistency between bits and SDK paths.
fn frontend_agent_prompt() -> String {
    flow_like::copilot::prompts::frontend_sdk_system_prompt()
}

fn backend_agent_prompt() -> String {
    flow_like::copilot::prompts::board_sdk_system_prompt()
}

fn general_agent_prompt() -> String {
    flow_like::copilot::prompts::general_system_prompt()
}

/// Get the system prompt for a specialized agent
fn get_agent_prompt(agent_type: &SpecializedAgentType) -> String {
    match agent_type {
        SpecializedAgentType::General => general_agent_prompt(),
        SpecializedAgentType::Frontend => frontend_agent_prompt(),
        SpecializedAgentType::Backend => backend_agent_prompt(),
    }
}

/// Create a session with a specialized agent using Copilot SDK
#[tauri::command]
pub async fn copilot_sdk_create_agent_session(
    agent_type: SpecializedAgentType,
    model_id: Option<String>,
) -> Result<String, String> {
    let client = COPILOT_CLIENT
        .lock()
        .await
        .clone()
        .ok_or("Copilot client not started")?;

    let system_prompt = get_agent_prompt(&agent_type);

    let config = copilot_sdk::SessionConfig {
        model: model_id,
        streaming: true,
        system_message: Some(copilot_sdk::SystemMessageConfig {
            content: Some(system_prompt),
            mode: Some(copilot_sdk::SystemMessageMode::Append),
        }),
        infinite_sessions: Some(copilot_sdk::InfiniteSessionConfig::enabled()),
        ..Default::default()
    };

    let session = tokio::time::timeout(SDK_CONTROL_RPC_TIMEOUT, client.create_session(config))
        .await
        .map_err(|_| "Timed out creating specialized Copilot session".to_string())?
        .map_err(|e| format!("Failed to create session: {}", e))?;

    Ok(session.session_id().to_string())
}
