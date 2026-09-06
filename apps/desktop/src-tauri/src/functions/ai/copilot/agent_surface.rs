//! Shared agent context, tool surfaces, and draft recovery.

use super::backend_types::FlowPilotAgentCapabilitySet;
use super::catalog::DesktopCatalogProvider;
use crate::{
    functions::ai::copilot_sdk_tools::{
        SideEffectCommandQueue, retained_flow_ir_draft_store_for_board,
    },
    state::TauriFlowLikeState,
};
use flow_like::{
    a2ui::SurfaceComponent,
    copilot::{CopilotScope, UnifiedChatMessage, UnifiedCopilotResponse},
    flow::{
        board::Board,
        copilot::{
            BoardContextManifest, CatalogProvider, FlowScriptPendingDelivery, GraphContext,
            ManifestAudit, ManifestAugmentations, ManifestSource, ManifestSourceStatus,
            board_fingerprint, default_flowscript_module_templates, flowscript_workspace_envelope,
            global_assistant_system_prompt, workflow_strategy_fingerprint,
        },
        node::Node,
    },
};
use std::sync::{Arc, Mutex as StdMutex};
use tauri::{AppHandle, Manager};

pub(super) struct FlowPilotAgentSurface {
    pub(super) graph_context: Option<Arc<GraphContext>>,
    pub(super) board_arc: Option<Arc<Board>>,
    /// Registry-backed current board. Retained FlowScript source operations lock this at execution
    /// time so the commit fingerprint and host queue boundary cannot rely on a captured clone.
    pub(super) live_board: Option<Arc<flow_like_types::sync::Mutex<Board>>>,
    /// Original host-owned workflow request used to derive a deterministic scope-coverage
    /// contract before the model can author its own capability plan. Bound to the immutable
    /// end-user request (not the per-run composed specialist instruction), so nested repair runs
    /// spawned from the same user turn share draft/acceptance identity.
    pub(super) request_acceptance_prompt: Option<String>,
    pub(super) catalog_provider: Option<Arc<dyn CatalogProvider>>,
    pub(super) side_effect_commands: Arc<StdMutex<SideEffectCommandQueue>>,
    /// Last FlowScript submission that reconciled successfully. Nested external agents return it
    /// to the global bridge so detached boards can apply the validated document.
    pub(super) queued_flowscript: Arc<StdMutex<Option<String>>>,
    /// UI trees captured from successful `emit_ui` calls, for transports that cannot parse tool
    /// results (external-agent MCP bridge).
    pub(super) emitted_surfaces:
        Arc<StdMutex<Vec<crate::functions::ai::copilot_sdk_tools::EmittedSurface>>>,
    /// Host-authorized source recovery for this exact immutable request. The prompt explains it,
    /// while the loop state separately enforces the draft id/revision without trusting the model
    /// to reconstruct those coordinates from prose.
    pub(super) flowscript_recovery: Option<flow_like::flow::copilot::FlowScriptDraftRecovery>,
    /// Immutable provider-neutral facts and lifecycle identity used by every adapter loop.
    pub(super) workflow_manifest: Option<BoardContextManifest>,
    pub(super) system_content: String,
    pub(super) workflow_edit_request: bool,
    pub(super) capabilities: FlowPilotAgentCapabilitySet,
}

/// Resolve the current in-process board once per tool surface. The incoming Tauri `Board` value is
/// a request snapshot; the registry handle continues to reflect edits made while an agent is
/// planning. Detached boards are intentionally left on the captured snapshot fallback.
pub(super) fn live_board_handle(
    app_handle: &AppHandle,
    board: Option<&Board>,
) -> Option<Arc<flow_like_types::sync::Mutex<Board>>> {
    let board_id = board
        .map(|board| board.id.trim())
        .filter(|board_id| !board_id.is_empty())?;
    app_handle
        .try_state::<TauriFlowLikeState>()
        .and_then(|state| state.0.get_board(board_id, None).ok())
}

/// Recover an exact pending source review before starting another model run. The temporary
/// acceptance binding is derived solely from the host's immutable raw request and is released
/// immediately after the read. The pending delivery itself is not consumed or re-claimed, so a
/// disconnected client can retry until it receives the same Apply/Dismiss token.
pub(super) async fn pending_flowscript_redelivery_for_request(
    app_handle: &AppHandle,
    captured_board: &Board,
    request_identity_prompt: &str,
) -> Option<FlowScriptPendingDelivery> {
    let current_board = match live_board_handle(app_handle, Some(captured_board)) {
        Some(live_board) => live_board.lock().await.clone(),
        None => captured_board.clone(),
    };
    let store = retained_flow_ir_draft_store_for_board(&current_board).ok()?;
    let binding =
        store.bind_request_acceptance_contract(&current_board.id, request_identity_prompt);
    let delivery = store.pending_flowscript_delivery_for_binding(&current_board, &binding);
    let _ = store.release_request_acceptance_contract(&binding);
    delivery
}

pub(super) fn pending_flowscript_redelivery_response(
    scope: CopilotScope,
    delivery: FlowScriptPendingDelivery,
) -> UnifiedCopilotResponse {
    let (message, workspace_status) = if delivery.stale_board {
        (
            "Recovered an exact pending FlowScript review, but the board has advanced. The old commands cannot be applied; dismiss this stale review before generating from the current board.",
            "stale",
        )
    } else {
        (
            "Redelivered the already-queued exact FlowScript revision for review. No model generation or duplicate command queueing occurred.",
            "queued",
        )
    };
    UnifiedCopilotResponse {
        message: message.to_string(),
        commands: delivery.commands,
        suggestions: Vec::new(),
        components: Vec::new(),
        canvas_settings: None,
        root_component_id: None,
        flowscript_workspace: Some(flowscript_workspace_envelope(
            &delivery.source,
            workspace_status,
        )),
        flow_ir_commit: Some(delivery.token),
        active_scope: scope,
    }
}

#[cfg_attr(not(test), allow(dead_code))]
pub(super) fn append_typed_ir_recovery_context(
    system_content: &mut String,
    recovery: &flow_like::flow::copilot::FlowIrDraftRecovery,
) {
    match recovery.status {
        flow_like::flow::copilot::FlowIrDraftRecoveryStatus::ExactMatch => {
            if let Ok(recovery_json) = serde_json::to_string_pretty(recovery) {
                system_content.push_str(&format!(
                    "\n\n## EXACT TYPED-DRAFT RECOVERY\nThe host matched this retained typed draft to the normalized immutable raw user request. Auto-resume this exact draft at its retained revision. Do not call begin_flow_ir_draft, switch mutation representations, or reconstruct it from the unchanged board/FlowScript.\n```json\n{recovery_json}\n```"
                ));
            }
        }
        flow_like::flow::copilot::FlowIrDraftRecoveryStatus::RequestMismatch => {
            // Deliberately omit the conflicting draft id/revision from model context. The host
            // owns that recovery decision; resumable coordinates would invite an unrelated
            // request to update or commit the old acceptance contract.
            let conflict = serde_json::json!({
                "status": "request_mismatch",
                "auto_resume": false,
                "conflicting_draft_present": recovery.conflicting_draft.is_some(),
                "next_actions": &recovery.next_actions,
                "message": &recovery.message,
            });
            if let Ok(conflict_json) = serde_json::to_string_pretty(&conflict) {
                system_content.push_str(&format!(
                    "\n\n## TYPED-DRAFT REQUEST MISMATCH\nThe host found retained typed work for this board, but it belongs to another immutable raw user request. It is non-authoritative for this run: do not update, validate, or commit it. Use only the host-owned recover/abandon choices below, or begin a separate draft id for the current request.\n```json\n{conflict_json}\n```"
                ));
            }
        }
        flow_like::flow::copilot::FlowIrDraftRecoveryStatus::None => {}
    }
}

/// Recover retained model-authored source across SDK/external requests using the same immutable
/// raw-request identity and stale-board rules as the built-in Rig path. The core renderer is the
/// authority for what may enter model context: exact matches include source, stale exact matches
/// include it only as a reference for a fresh draft, and request mismatches hide it completely.
#[cfg(test)]
pub(super) fn append_flowscript_recovery_context(
    system_content: &mut String,
    board: &Board,
    raw_user_prompt: &str,
) {
    let Ok(store) = retained_flow_ir_draft_store_for_board(board) else {
        return;
    };
    let recovery = store.editable_flowscript_draft_recovery(board, raw_user_prompt);
    append_flowscript_recovery_payload(system_content, &recovery);
}

fn append_flowscript_recovery_payload(
    system_content: &mut String,
    recovery: &flow_like::flow::copilot::FlowScriptDraftRecovery,
) {
    let Some(instruction) =
        flow_like::flow::copilot::flowscript_recovery_system_instruction(recovery)
    else {
        return;
    };
    system_content.push_str("\n\n");
    system_content.push_str(&instruction);
}

#[allow(clippy::too_many_arguments)]
pub(super) fn build_flowpilot_agent_surface(
    scope: CopilotScope,
    board: Option<&Board>,
    catalog_nodes: Option<Vec<Node>>,
    selected_node_ids: &[String],
    current_surface: Option<&Vec<SurfaceComponent>>,
    // The surface's persisted canvasSettings, customCss included. Without it the UI specialist
    // edits a page whose stylesheet it cannot see, and can only replace it blind.
    current_canvas_settings: Option<&serde_json::Value>,
    history: &[UnifiedChatMessage],
    _original_user_prompt: &str,
    // Immutable end-user request that owns retained drafts and the acceptance contract. For a
    // nested specialist run this differs from `original_user_prompt` (the per-run composed
    // instruction), so every identity bind below must use this value.
    request_identity_prompt: &str,
    host_context_guidance: Option<&str>,
    global: Option<&str>,
    board_context_augmentation: Option<&serde_json::Value>,
    // Read-only sub-run (flowpilot_board explain): keep the board copilot out of workflow-edit mode
    // so it streams and returns its answer instead of being coerced to emit an edit and, failing
    // that, returning a canned "could not produce board commands" message.
    read_only: bool,
) -> FlowPilotAgentSurface {
    use flow_like::flow::copilot::prepare_context;

    let graph_context = match scope {
        CopilotScope::Board | CopilotScope::Both => board
            .and_then(|board| prepare_context(board, selected_node_ids).ok())
            .map(Arc::new),
        CopilotScope::Frontend
        | CopilotScope::DataStudio
        | CopilotScope::Scout
        | CopilotScope::Home
        | CopilotScope::Research => None,
    };

    let board_arc: Option<Arc<Board>> = match scope {
        CopilotScope::Board | CopilotScope::Both => board.map(|b| Arc::new(b.clone())),
        CopilotScope::Frontend
        | CopilotScope::DataStudio
        | CopilotScope::Scout
        | CopilotScope::Home
        | CopilotScope::Research => None,
    };

    let desktop_catalog_provider = match scope {
        CopilotScope::Board | CopilotScope::Both => {
            Some(Arc::new(DesktopCatalogProvider::new(catalog_nodes)))
        }
        CopilotScope::Frontend
        | CopilotScope::DataStudio
        | CopilotScope::Scout
        | CopilotScope::Home
        | CopilotScope::Research => None,
    };

    let catalog_provider: Option<Arc<dyn CatalogProvider>> = match scope {
        CopilotScope::Board | CopilotScope::Both => desktop_catalog_provider
            .as_ref()
            .map(|provider| provider.clone() as Arc<dyn CatalogProvider>),
        CopilotScope::Frontend
        | CopilotScope::DataStudio
        | CopilotScope::Scout
        | CopilotScope::Home
        | CopilotScope::Research => None,
    };

    let board_flowscript = board_arc.as_ref().map(|board| {
        flow_like::flow::ast::board_to_flowscript(
            board,
            &flow_like::flow::ast::RenderOptions {
                anchors: true,
                ..Default::default()
            },
        )
    });

    let catalog_node_count = desktop_catalog_provider
        .as_ref()
        .map(|provider| provider.len())
        .unwrap_or_else(|| flow_like_catalog::get_catalog().len());

    let workflow_edit_request = !read_only
        && matches!(scope, CopilotScope::Board | CopilotScope::Both)
        && board_arc.is_some();

    let mut system_content = if global.is_some() {
        global_assistant_system_prompt()
    } else {
        match scope {
            CopilotScope::Board => match board_flowscript.as_deref() {
                Some(flowscript) => {
                    flow_like::copilot::prompts::board_sdk_flowscript_system_prompt(
                        flowscript,
                        catalog_node_count,
                    )
                }
                None => flow_like::copilot::prompts::board_sdk_system_prompt(),
            },
            CopilotScope::Frontend => flow_like::copilot::prompts::frontend_sdk_system_prompt(),
            CopilotScope::DataStudio if read_only => {
                flow_like::copilot::prompts::ontology_query_system_prompt()
            }
            CopilotScope::DataStudio => flow_like::copilot::prompts::data_studio_system_prompt(""),
            CopilotScope::Scout => flow_like::copilot::prompts::scout_system_prompt(""),
            CopilotScope::Home => flow_like::copilot::prompts::home_system_prompt(""),
            CopilotScope::Research => {
                flow_like::copilot::prompts::research_system_prompt(&format!(
                    "Current UTC date: {}.",
                    chrono::Utc::now().format("%Y-%m-%d")
                ))
            }
            CopilotScope::Both => match board_flowscript.as_deref() {
                // flowscript_board_context embeds the shared guidance blocks itself; the lean
                // header avoids duplicating them (~3.5k tokens).
                Some(flowscript) => {
                    let mut prompt = flow_like::copilot::prompts::general_system_prompt_lean();
                    prompt.push_str("\n\n");
                    prompt.push_str(&flow_like::copilot::prompts::flowscript_board_context(
                        flowscript,
                        catalog_node_count,
                    ));
                    prompt
                }
                None => flow_like::copilot::prompts::general_system_prompt(),
            },
        }
    };

    if let Some(context) = global
        && !context.is_empty()
    {
        system_content.push_str("\n\n");
        system_content.push_str(context);
    }

    if let Some(guidance) = host_context_guidance.filter(|guidance| !guidance.trim().is_empty()) {
        system_content.push_str("\n\n");
        system_content.push_str(guidance);
    }

    let flowscript_recovery = workflow_edit_request
        .then_some(board_arc.as_deref())
        .flatten()
        .and_then(|board| {
            retained_flow_ir_draft_store_for_board(board)
                .ok()
                .map(|store| {
                    store.editable_flowscript_draft_recovery(board, request_identity_prompt)
                })
        });
    if let Some(recovery) = flowscript_recovery.as_ref() {
        append_flowscript_recovery_payload(&mut system_content, recovery);
    }

    let workflow_manifest = workflow_edit_request
        .then_some(board_arc.as_deref())
        .flatten()
        .and_then(|board| {
            let retained = flowscript_recovery
                .as_ref()
                .and_then(|recovery| recovery.exact_match.as_ref())
                .filter(|context| !context.stale_board)
                .and_then(|context| {
                    context.source.as_ref().map(|source| {
                        (
                            source.clone(),
                            context.revision,
                            (!context.diagnostics.is_empty()).then(|| {
                                workflow_strategy_fingerprint(&serde_json::json!({
                                    "diagnostics": context.diagnostics,
                                }))
                            }),
                        )
                    })
                });
            let source = match retained {
                Some((source, revision, diagnostic_fingerprint)) => ManifestSource::new(
                    ManifestSourceStatus::Retained,
                    Some(revision),
                    Some(source),
                    diagnostic_fingerprint,
                ),
                None => ManifestSource::new(
                    ManifestSourceStatus::Existing,
                    None,
                    board_flowscript.clone(),
                    None,
                ),
            };
            BoardContextManifest::from_board(
                board,
                selected_node_ids,
                &desktop_catalog_provider
                    .as_ref()
                    .map(|provider| provider.all_metadata())
                    .unwrap_or_default(),
                source,
                ManifestAudit {
                    request_identity: request_identity_prompt.to_string(),
                    base_fingerprint: board_fingerprint(board),
                    acceptance_contract_fingerprint: Some(workflow_strategy_fingerprint(
                        &serde_json::json!({ "request_identity": request_identity_prompt }),
                    )),
                    build_id: None,
                    attributes: std::collections::BTreeMap::from([(
                        "orchestrator".to_string(),
                        "flowpilot-shared".to_string(),
                    )]),
                },
                ManifestAugmentations::from_host_value(board_context_augmentation),
                default_flowscript_module_templates(),
            )
            .ok()
        });
    if let Some(manifest_prompt) = workflow_manifest
        .as_ref()
        .and_then(|manifest| manifest.render_authoring_prompt().ok())
    {
        system_content.push_str("\n\n");
        system_content.push_str(&manifest_prompt);
    }

    if matches!(scope, CopilotScope::Frontend | CopilotScope::Both) {
        if let Some(components) = current_surface
            && !components.is_empty()
        {
            let components_json =
                serde_json::to_string_pretty(components).unwrap_or_else(|_| "[]".to_string());
            system_content.push_str(&format!(
                "\n\n## CURRENT UI COMPONENTS\nThe user has the following existing UI. You can modify or extend it:\n```json\n{}\n```",
                components_json
            ));
        }

        // The stylesheet is shown verbatim, uncapped: editing a design system requires seeing the
        // classes it already defines, and emit_ui replaces customCss wholesale rather than merging
        // rule by rule.
        if let Some(canvas_settings) = current_canvas_settings
            && canvas_settings
                .as_object()
                .is_some_and(|map| !map.is_empty())
        {
            let canvas_json =
                serde_json::to_string_pretty(canvas_settings).unwrap_or_else(|_| "{}".to_string());
            system_content.push_str(&format!(
                "\n\n## CURRENT CANVAS SETTINGS\nThis surface's live canvasSettings, customCss included:\n```json\n{}\n```\nReuse the classes this stylesheet already defines instead of inventing parallel ones. OMIT `canvasSettings.customCss` from emit_ui to leave it untouched; include it only to change it, and then send the COMPLETE stylesheet — the value replaces the previous one, so any rule you leave out is deleted.",
                canvas_json
            ));
        }
    }

    let mut context_parts = vec![];
    for msg in history {
        let role = match msg.role {
            flow_like::flow::copilot::ChatRole::User => "User",
            flow_like::flow::copilot::ChatRole::Assistant => "Assistant",
        };
        context_parts.push(format!("{}: {}", role, msg.content));
    }
    if !context_parts.is_empty() {
        system_content.push_str(&format!(
            "\n\nConversation history:\n{}",
            context_parts.join("\n\n")
        ));
    }

    let capabilities = FlowPilotAgentCapabilitySet::for_surface(
        scope,
        board_arc.is_some(),
        graph_context.is_some(),
        global.is_some(),
    );

    FlowPilotAgentSurface {
        graph_context,
        board_arc,
        live_board: None,
        request_acceptance_prompt: workflow_edit_request
            .then(|| request_identity_prompt.to_string()),
        catalog_provider,
        side_effect_commands: Arc::new(StdMutex::new(SideEffectCommandQueue::default())),
        queued_flowscript: Arc::new(StdMutex::new(None)),
        emitted_surfaces: Arc::new(StdMutex::new(Vec::new())),
        flowscript_recovery,
        workflow_manifest,
        system_content,
        workflow_edit_request,
        capabilities,
    }
}
