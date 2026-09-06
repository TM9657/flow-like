use super::*;
use flow_like::flow::copilot::{FORCED_INCREMENTAL_SEGMENT_THRESHOLD, MAX_BOARD_SCOPE_SEGMENTS};

mod backend_telemetry;
mod board_edit_jobs;
mod candidate_regression;
mod cli_discovery;
mod declaration_preflight;
mod declaration_repair;
mod nested_runs;
mod provider_invocations;
mod provider_streaming;
mod request_recovery;
mod run_lifecycle;
mod scope_planning;
mod sdk_integration;
mod source_recovery;
mod tool_policy;
mod typed_ir;
mod workflow_preflight;
mod workflow_reporting;

fn sorted_prop_keys(props: &serde_json::Value) -> Vec<&str> {
    let mut keys: Vec<&str> = props
        .as_object()
        .expect("agent backend props are an object")
        .keys()
        .map(String::as_str)
        .collect();
    keys.sort_unstable();
    keys
}

fn flowscript_recovery_test_board() -> Board {
    let mut board = Board::new_detached(
        Some(format!("flowscript-recovery-{}", uuid::Uuid::new_v4())),
        flow_like::flow_like_storage::Path::from("/test"),
    );
    board.name = "Recovery".to_string();
    board.description.clear();
    board.viewport = (0.0, 0.0, 1.0);
    board.hash = None;
    board
}

fn retained_recovery_context(
    draft_id: &str,
    revision: u64,
) -> flow_like::flow::copilot::FlowIrEditableDraftContext {
    flow_like::flow::copilot::FlowIrEditableDraftContext {
        board_id: "board".to_string(),
        draft_id: draft_id.to_string(),
        revision,
        status: "editing".to_string(),
        base_fingerprint: "base".to_string(),
        missing_modules: vec!["send_reply".to_string()],
        remaining_capabilities: vec!["smtp_send".to_string()],
        diagnostics: Vec::new(),
    }
}

fn workflow_call_result_json(result: &rmcp::model::CallToolResult) -> serde_json::Value {
    let text = result
        .content
        .iter()
        .find_map(|content| match &content.raw {
            rmcp::model::RawContent::Text(text) => Some(text.text.as_str()),
            _ => None,
        })
        .expect("workflow guard result contains JSON text");
    serde_json::from_str(text).expect("workflow guard result is valid JSON")
}

fn board_edit_job_test_record(
    job_id: impl Into<String>,
    phase: BoardEditJobPhase,
    touched_at: Instant,
) -> BoardEditJobRecord {
    let job_id = job_id.into();
    BoardEditJobRecord {
        job: BoardEditJob {
            schema_version: BOARD_EDIT_JOB_SCHEMA_VERSION.to_string(),
            job_id: job_id.clone(),
            app_id: "review-app".to_string(),
            board_id: "review-board".to_string(),
            request_id: Some(format!("request-{job_id}")),
            remote_profile_id: None,
            remote_principal_id: None,
            remote_hub: None,
            phase,
            created_at_ms: 1,
            updated_at_ms: 1,
            expires_at_ms: 2,
            token: FlowIrCommitToken {
                board_id: "review-board".to_string(),
                draft_id: format!("draft-{job_id}"),
                revision: 3,
                base_fingerprint: "base".to_string(),
                claim_id: format!("claim-{job_id}"),
                requires_destructive_approval: false,
            },
            approval: flow_like::flow::copilot::tool_spec::ResolvedToolApproval::none(),
            review: BoardEditJobReview {
                command_count: 0,
                command_counts: BTreeMap::new(),
                command_summaries: Vec::new(),
                replacement_mode: false,
                destructive_effects: Vec::new(),
            },
            result: None,
            error: None,
        },
        board_commands: vec![BoardCommand::AddNode {
            node_type: "events_generic".to_string(),
            ref_id: Some(format!("node-{job_id}")),
            position: None,
            friendly_name: None,
            additional_pins: None,
            target_layer: None,
            summary: None,
        }],
        replacement_mode: false,
        touched_at,
        resolution_lock: Arc::new(tokio::sync::Mutex::new(())),
        delivery_lease: None,
    }
}

fn unstarted_pool_client() -> Arc<Client> {
    Arc::new(Client::builder().build().expect("unstarted pool client"))
}

fn leaked_test_pool(size: usize) -> &'static NestedCopilotPool {
    Box::leak(Box::new(NestedCopilotPool::new(size)))
}

fn build_test_client() -> Option<Client> {
    let cli_path = find_copilot_cli_path();
    if cli_path.is_none() {
        eprintln!("SKIP: copilot CLI not found");
        return None;
    }

    let mut builder = Client::builder().use_stdio(true).log_level(LogLevel::Error);

    if let Some(path) = cli_path {
        builder = builder.cli_path(path);
    }
    builder = builder.env("PATH", augmented_path());

    Some(builder.build().expect("Client::builder().build() failed"))
}

async fn start_test_client() -> Option<Client> {
    let client = build_test_client()?;
    match client.start().await {
        Ok(()) => Some(client),
        Err(e) => {
            let err_str = format!("{:?}", e);
            if err_str.contains("ProtocolMismatch") {
                eprintln!(
                    "SKIP: protocol mismatch — SDK expects v{}, CLI reports v3. \
                         Update copilot-sdk dependency.",
                    copilot_sdk::SDK_PROTOCOL_VERSION
                );
            } else {
                eprintln!("SKIP: client.start() failed: {}", err_str);
            }
            None
        }
    }
}

fn accept_plan(
    state: &Arc<StdMutex<WorkflowToolLoopState>>,
    args: serde_json::Value,
) -> serde_json::Value {
    workflow_call_result_json(
        &workflow_tool_preflight_with_args(state, "plan_board_scope", &args)
            .expect("plan_board_scope is answered by the host loop"),
    )
}

fn staged_plan_args(segments: usize) -> serde_json::Value {
    let segments = (1..=segments)
        .map(|index| {
            serde_json::json!({
                "id": format!("s{index}"),
                "title": format!("Segment {index}"),
                "behavior": format!("Build and fully wire the nodes belonging to slice {index}."),
            })
        })
        .collect::<Vec<_>>();
    serde_json::json!({ "strategy": "staged", "segments": segments })
}

/// Move the ledger forward the way a real run does: one more revision checked clean and a
/// larger retained document.
fn record_forward_progress(state: &mut WorkflowToolLoopState, source: &str) {
    state.valid_checks = state.valid_checks.saturating_add(1);
    state.last_flowscript = Some(source.to_string());
}

/// The one-segment plan an ordinary edit declares, for tests that build loop state directly.
fn single_segment_plan() -> BoardScopePlan {
    use flow_like::flow::copilot::{CURRENT_BOARD_REF, PlannedSegment};

    accept_scope_plan(PlanBoardScopeArgs {
        strategy: ScopeStrategy::Single,
        segments: vec![PlannedSegment {
            id: "s1".to_string(),
            title: "Whole request".to_string(),
            behavior: "Build the complete requested workflow as one document.".to_string(),
            depends_on: Vec::new(),
            board_ref: CURRENT_BOARD_REF.to_string(),
        }],
        rationale: String::new(),
    })
    .expect("a one-segment single-strategy plan is valid")
}

/// Preflight requires an accepted scope plan before the first source write. Lifecycle tests
/// that are about what happens AFTER that take the one-segment plan an ordinary edit declares.
fn accept_single_segment_plan(state: &Arc<StdMutex<WorkflowToolLoopState>>) {
    let result = workflow_tool_preflight_with_args(
        state,
        "plan_board_scope",
        &serde_json::json!({
            "strategy": "single",
            "segments": [{
                "id": "s1",
                "title": "Whole request",
                "behavior": "Build the complete requested workflow as one document."
            }]
        }),
    )
    .expect("plan_board_scope is answered by the host loop");
    assert_eq!(result.is_error, Some(false));
}

fn rich_support_flowscript() -> &'static str {
    r#"@secret
const IMAP_HOST: string = ""

function pollSupportInbox() {
    const connection = emailImapConnect({ host: IMAP_HOST })
    const inbox = mailImapInbox({ connection: connection.connection })
    const refs = mailImapList({ inbox: inbox.inbox })
    const first = arrayGet({ array: refs.refs, index: 0 })
    const mail = emailImapInboxFetchMail({ emailRef: first.value })
    logInfo({ message: mail.subject })
}

function requestApproval() {
    smtpSendEmail({ to: "example@example.com", subject: "Review" })
}

eventsSimple() {
    pollSupportInbox()
    requestApproval()
}

eventsGeneric(payload: Struct) {
    structGet({ struct: payload, field: "ticket_id" })
}
"#
}

fn seed_failed_rich_candidate(state: &Arc<StdMutex<WorkflowToolLoopState>>) {
    workflow_tool_record(
        state,
        "edit_flowscript",
        &serde_json::json!({ "flowscript": rich_support_flowscript() }),
        &serde_json::json!({
            "status": "validation_errors",
            "errors": ["one connection still needs repair"]
        })
        .to_string(),
    );
}

// 1x1 transparent PNG — enough for arg/stdin plumbing assertions (real
// snapshots must be larger for the model to perceive them).
fn test_chat_image() -> ChatImage {
    ChatImage {
            data: "iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAYAAAAfFcSJAAAADUlEQVR42mNk+M9QDwADhgGAWjR9awAAAABJRU5ErkJggg==".to_string(),
            media_type: "image/png".to_string(),
        }
}
