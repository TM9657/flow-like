use super::*;

#[test]
fn board_edit_review_is_deterministic_and_host_derived() {
    let review = board_command_review(
        &[
            BoardCommand::AddNode {
                node_type: "events_generic".to_string(),
                ref_id: Some("event".to_string()),
                position: None,
                friendly_name: None,
                additional_pins: None,
                target_layer: None,
                summary: Some("  Add the entry event.  ".to_string()),
            },
            BoardCommand::AddNode {
                node_type: "log".to_string(),
                ref_id: Some("log".to_string()),
                position: None,
                friendly_name: None,
                additional_pins: None,
                target_layer: None,
                summary: None,
            },
            BoardCommand::RemoveNode {
                node_id: "old-node".to_string(),
                summary: Some("Remove the obsolete node.".to_string()),
            },
        ],
        false,
    );

    assert_eq!(review.command_count, 3);
    assert_eq!(review.command_counts.get("AddNode"), Some(&2));
    assert_eq!(review.command_counts.get("RemoveNode"), Some(&1));
    assert_eq!(
        review.command_summaries,
        vec![
            "Add the entry event.".to_string(),
            "Remove the obsolete node.".to_string(),
        ]
    );
    assert_eq!(
        review.destructive_effects,
        vec!["node `old-node`".to_string()]
    );
}

#[test]
fn terminal_board_edit_identity_blocks_stale_direct_receipt_replay() {
    let now = Instant::now();
    let terminal = board_edit_job_test_record("terminal", BoardEditJobPhase::Applied, now);
    assert!(board_edit_job_matches_terminal_delivery(
        &terminal,
        &terminal.job.app_id,
        &terminal.job.token,
    ));

    let mut pending = terminal.clone();
    pending.job.phase = BoardEditJobPhase::AppliedPendingDelivery;
    assert!(!board_edit_job_matches_terminal_delivery(
        &pending,
        &pending.job.app_id,
        &pending.job.token,
    ));

    let mut other = terminal.job.token.clone();
    other.claim_id.push_str("-other");
    assert!(!board_edit_job_matches_terminal_delivery(
        &terminal,
        &terminal.job.app_id,
        &other,
    ));
}

#[test]
fn board_edit_job_pruning_expires_reviews_and_preserves_the_exact_capacity() {
    let now = Instant::now();
    let expired_at = now
        .checked_sub(BOARD_EDIT_JOB_TTL + Duration::from_secs(1))
        .expect("test instant supports TTL subtraction");
    let mut jobs = HashMap::from([(
        "expired".to_string(),
        board_edit_job_test_record("expired", BoardEditJobPhase::AwaitingApproval, expired_at),
    )]);
    assert!(prune_board_edit_jobs(&mut jobs, now));
    let expired = &jobs["expired"].job;
    assert_eq!(expired.phase, BoardEditJobPhase::Stale);
    assert!(
        expired
            .error
            .as_deref()
            .is_some_and(|error| error.contains("expired"))
    );

    let mut at_capacity = (0..BOARD_EDIT_JOB_MAX_ENTRIES)
        .map(|index| {
            let id = format!("settled-{index:03}");
            (
                id.clone(),
                board_edit_job_test_record(id, BoardEditJobPhase::Applied, now),
            )
        })
        .collect::<HashMap<_, _>>();
    assert!(!prune_board_edit_jobs(&mut at_capacity, now));
    assert_eq!(at_capacity.len(), BOARD_EDIT_JOB_MAX_ENTRIES);

    at_capacity.insert(
        "old-terminal".to_string(),
        board_edit_job_test_record(
            "old-terminal",
            BoardEditJobPhase::Denied,
            now.checked_sub(Duration::from_secs(1))
                .expect("test instant supports subtraction"),
        ),
    );
    assert!(prune_board_edit_jobs(&mut at_capacity, now));
    assert_eq!(at_capacity.len(), BOARD_EDIT_JOB_MAX_ENTRIES);
    assert!(!at_capacity.contains_key("old-terminal"));
}

#[test]
fn failed_board_edit_jobs_remain_retryable_until_the_review_ttl() {
    let now = Instant::now();
    let mut jobs = HashMap::from([(
        "retryable".to_string(),
        board_edit_job_test_record("retryable", BoardEditJobPhase::Failed, now),
    )]);

    assert!(!prune_board_edit_jobs(&mut jobs, now));
    assert_eq!(jobs["retryable"].job.phase, BoardEditJobPhase::Failed);
    assert!(!jobs["retryable"].job.phase.is_terminal());

    let expired_at = now
        .checked_sub(BOARD_EDIT_JOB_TTL + Duration::from_secs(1))
        .expect("test instant supports TTL subtraction");
    jobs.get_mut("retryable").expect("retryable job").touched_at = expired_at;
    assert!(prune_board_edit_jobs(&mut jobs, now));
    assert_eq!(jobs["retryable"].job.phase, BoardEditJobPhase::Stale);
}

#[test]
fn applied_receipt_stays_recoverable_until_renderer_delivery_is_acknowledged() {
    let now = Instant::now();
    let older_than_review_ttl = now
        .checked_sub(BOARD_EDIT_JOB_TTL + Duration::from_secs(1))
        .expect("test instant supports TTL subtraction");
    let mut record = board_edit_job_test_record(
        "delivery",
        BoardEditJobPhase::AppliedPendingDelivery,
        older_than_review_ttl,
    );
    record.delivery_lease = Some(BoardEditJobDeliveryLease {
        lease_id: "abandoned-renderer".to_string(),
        expires_at: now
            .checked_sub(Duration::from_millis(1))
            .expect("test instant supports subtraction"),
    });
    let mut jobs = HashMap::from([("delivery".to_string(), record)]);

    assert!(!prune_board_edit_jobs(&mut jobs, now));

    let retained = &jobs["delivery"];
    assert_eq!(
        retained.job.phase,
        BoardEditJobPhase::AppliedPendingDelivery
    );
    assert!(!retained.job.phase.is_terminal());
    assert!(retained.delivery_lease.is_none());
}

#[test]
fn board_edit_job_commit_identity_is_exact_and_claim_scoped() {
    let token = FlowIrCommitToken {
        board_id: "board".to_string(),
        draft_id: "draft".to_string(),
        revision: 7,
        base_fingerprint: "base".to_string(),
        claim_id: "claim-a".to_string(),
        requires_destructive_approval: false,
    };
    assert_eq!(
        flow_ir_commit_identity(&token),
        flow_ir_commit_identity(&token.clone())
    );

    let mut next_claim = token.clone();
    next_claim.claim_id = "claim-b".to_string();
    assert_ne!(
        flow_ir_commit_identity(&token),
        flow_ir_commit_identity(&next_claim)
    );
    let mut next_revision = token.clone();
    next_revision.revision += 1;
    assert_ne!(
        flow_ir_commit_identity(&token),
        flow_ir_commit_identity(&next_revision)
    );
}

#[test]
fn board_edit_job_serializes_the_frontend_contract() {
    let mut record = board_edit_job_test_record(
        "wire-contract",
        BoardEditJobPhase::AwaitingApproval,
        Instant::now(),
    );
    record.job.review.command_count = 2;
    record
        .job
        .review
        .command_counts
        .insert("AddNode".to_string(), 2);
    let value = serde_json::to_value(record.job).expect("serialize board-edit job");

    assert_eq!(value["schemaVersion"], BOARD_EDIT_JOB_SCHEMA_VERSION);
    assert_eq!(value["jobId"], "wire-contract");
    assert_eq!(value["phase"], "awaiting_approval");
    assert_eq!(value["review"]["commandCount"], 2);
    assert_eq!(value["review"]["commandCounts"]["AddNode"], 2);
    assert_eq!(value["token"]["board_id"], "review-board");
    assert!(value.get("result").is_none());
    assert!(value.get("error").is_none());
}

#[test]
fn actual_executed_receipt_is_bounded_before_remote_persistence() {
    let mut command = flow_like::flow::board::commands::nodes::copy_paste::CopyPasteCommand::new(
        Vec::new(),
        Vec::new(),
        Vec::new(),
        (0.0, 0.0, 0.0),
    );
    command.original_refs.insert(
        "large-public-ref".to_string(),
        "x".repeat(BOARD_EDIT_JOB_MAX_REMOTE_COMMAND_BYTES),
    );
    let result = ApplyFlowIrCommitResult {
        status: "applied".to_string(),
        replayed: false,
        code: None,
        message: "applied".to_string(),
        commands: vec![GenericCommand::CopyPaste(command)],
        board_commands: Vec::new(),
        diagnostics: Vec::new(),
        final_board_node_count: Some(0),
    };

    let error = validate_board_edit_delivery_bounds(&result, true)
        .expect_err("an unsendable executed command must fail before board save");
    assert!(error.contains("delivery limit"), "{error}");
}

#[test]
fn aggregate_executed_receipt_must_fit_one_atomic_remote_request() {
    let commands = (0..2)
        .map(|index| {
            let mut command =
                flow_like::flow::board::commands::nodes::copy_paste::CopyPasteCommand::new(
                    Vec::new(),
                    Vec::new(),
                    Vec::new(),
                    (0.0, 0.0, 0.0),
                );
            command.original_refs.insert(
                format!("large-public-ref-{index}"),
                "x".repeat(BOARD_EDIT_JOB_MAX_REMOTE_COMMAND_BYTES / 2),
            );
            GenericCommand::CopyPaste(command)
        })
        .collect();
    let result = ApplyFlowIrCommitResult {
        status: "applied".to_string(),
        replayed: false,
        code: None,
        message: "applied".to_string(),
        commands,
        board_commands: Vec::new(),
        diagnostics: Vec::new(),
        final_board_node_count: Some(0),
    };

    let error = validate_board_edit_delivery_bounds(&result, true)
        .expect_err("individually small commands must not be split across board saves");
    assert!(error.contains("atomic executed command batch"), "{error}");
}

#[test]
fn durable_receipt_drops_redundant_compiler_commands() {
    let result = ApplyFlowIrCommitResult {
        status: "applied".to_string(),
        replayed: false,
        code: None,
        message: "applied".to_string(),
        commands: Vec::new(),
        board_commands: vec![BoardCommand::RemoveNode {
            node_id: "old-node".to_string(),
            summary: None,
        }],
        diagnostics: Vec::new(),
        final_board_node_count: Some(0),
    };

    let compact = compact_durable_apply_receipt(&result);
    assert!(compact.board_commands.is_empty());
    assert_eq!(compact.commands.len(), result.commands.len());
}

#[test]
fn board_edit_job_snapshot_round_trips_the_exact_host_batch_and_policy() {
    let mut record = board_edit_job_test_record(
        "durable-batch",
        BoardEditJobPhase::AwaitingApproval,
        Instant::now(),
    );
    record.replacement_mode = true;
    record.job.review.replacement_mode = true;
    let reviewed_commands = record.board_commands.clone();
    let encoded = serde_json::to_vec(&PersistedBoardEditJobEntry::Current(
        PersistedBoardEditJobRecord {
            job: record.job,
            board_commands: record.board_commands,
            replacement_mode: record.replacement_mode,
        },
    ))
    .expect("serialize persisted board-edit job");
    let persisted: PersistedBoardEditJobEntry =
        serde_json::from_slice(&encoded).expect("deserialize persisted board-edit job");
    let recovered = board_edit_job_record_from_persisted(persisted)
        .expect("persisted exact batch remains recoverable");

    assert_eq!(recovered.job.phase, BoardEditJobPhase::AwaitingApproval);
    assert!(recovered.replacement_mode);
    assert_eq!(recovered.job.review.command_count, reviewed_commands.len());
    assert!(recovered.job.review.replacement_mode);
    assert!(exact_board_command_batch_matches(
        &reviewed_commands,
        &recovered.board_commands
    ));
}

#[test]
fn legacy_board_edit_job_without_an_exact_batch_fails_closed_after_restart() {
    let legacy = board_edit_job_test_record(
        "legacy-batch",
        BoardEditJobPhase::AwaitingApproval,
        Instant::now(),
    )
    .job;
    let recovered =
        board_edit_job_record_from_persisted(PersistedBoardEditJobEntry::Legacy(legacy))
            .expect("legacy job remains visible for an actionable stale result");

    assert_eq!(recovered.job.phase, BoardEditJobPhase::Stale);
    assert!(recovered.board_commands.is_empty());
    assert!(
        recovered
            .job
            .error
            .as_deref()
            .is_some_and(|message| message.contains("cannot be applied safely"))
    );
}

#[test]
fn interrupted_apply_restarts_as_retryable_failed_with_the_exact_batch() {
    let record = board_edit_job_test_record(
        "interrupted-apply",
        BoardEditJobPhase::Applying,
        Instant::now(),
    );
    let reviewed_commands = record.board_commands.clone();
    let recovered = board_edit_job_record_from_persisted(PersistedBoardEditJobEntry::Current(
        PersistedBoardEditJobRecord {
            job: record.job,
            board_commands: record.board_commands,
            replacement_mode: record.replacement_mode,
        },
    ))
    .expect("interrupted exact apply remains retryable");

    assert_eq!(recovered.job.phase, BoardEditJobPhase::Failed);
    assert!(exact_board_command_batch_matches(
        &reviewed_commands,
        &recovered.board_commands
    ));
    assert!(
        recovered
            .job
            .error
            .as_deref()
            .is_some_and(|message| message.contains("restarted"))
    );
}

#[test]
fn board_mutation_gate_reserves_matching_apply_recovery_and_delivery_phases() {
    let now = Instant::now();
    let jobs = HashMap::from([
        (
            "awaiting".to_string(),
            board_edit_job_test_record("awaiting", BoardEditJobPhase::AwaitingApproval, now),
        ),
        (
            "applying".to_string(),
            board_edit_job_test_record("applying", BoardEditJobPhase::Applying, now),
        ),
    ]);

    assert!(board_mutation_is_reserved(
        &jobs,
        "review-app",
        "review-board"
    ));
    assert!(!board_mutation_is_reserved(
        &jobs,
        "another-app",
        "review-board"
    ));
    assert!(another_board_edit_job_reserves_mutation(
        &jobs,
        "awaiting",
        "review-app",
        "review-board"
    ));
    assert!(!another_board_edit_job_reserves_mutation(
        &jobs,
        "applying",
        "review-app",
        "review-board"
    ));
    assert!(!board_mutation_is_reserved(
        &jobs,
        "review-app",
        "another-board"
    ));

    let delivery = HashMap::from([(
        "delivery".to_string(),
        board_edit_job_test_record("delivery", BoardEditJobPhase::AppliedPendingDelivery, now),
    )]);
    assert!(board_mutation_is_reserved(
        &delivery,
        "review-app",
        "review-board"
    ));
    let failed = HashMap::from([(
        "failed".to_string(),
        board_edit_job_test_record("failed", BoardEditJobPhase::Failed, now),
    )]);
    assert!(board_mutation_is_reserved(
        &failed,
        "review-app",
        "review-board"
    ));
}

#[test]
fn atomic_typed_apply_errors_keep_the_full_apply_result_contract() {
    let response =
        ApplyFlowIrCommitResult::empty("stale", "IR_COMMIT_REVIEW_STALE", "Nothing was applied.");
    let value = serde_json::to_value(response).expect("serialize atomic apply response");

    assert_eq!(value["status"], "stale");
    assert_eq!(value["code"], "IR_COMMIT_REVIEW_STALE");
    assert_eq!(value["commands"], serde_json::json!([]));
    assert_eq!(value["board_commands"], serde_json::json!([]));
    assert_eq!(value["diagnostics"], serde_json::json!([]));
}

#[test]
fn typed_replacement_and_deletions_are_both_destructive_review_gated() {
    assert_eq!(
        typed_commit_destructive_review_items(true, &[]),
        vec!["The draft uses full-board replacement semantics."]
    );
    assert!(typed_commit_destructive_review_items(false, &[]).is_empty());
    assert_eq!(
        typed_commit_destructive_review_items(
            false,
            &[BoardCommand::RemoveNode {
                node_id: "existing-node".to_string(),
                summary: None,
            }],
        ),
        vec!["node `existing-node`"]
    );
}

#[test]
fn native_destructive_dialog_window_revalidates_the_exact_batch() {
    let reviewed = vec![BoardCommand::RemoveNode {
        node_id: "reviewed-node".to_string(),
        summary: None,
    }];
    assert!(exact_board_command_batch_matches(&reviewed, &reviewed));

    let changed = vec![BoardCommand::RemoveNode {
        node_id: "different-node".to_string(),
        summary: None,
    }];
    assert!(!exact_board_command_batch_matches(&reviewed, &changed));
    assert!(!exact_board_command_batch_matches(&reviewed, &[]));
}

#[test]
fn atomic_typed_apply_receipt_replays_exact_success_after_lost_response() {
    let token = FlowIrCommitToken {
        board_id: "receipt-board".to_string(),
        draft_id: "receipt-draft".to_string(),
        revision: 7,
        base_fingerprint: "base".to_string(),
        claim_id: uuid::Uuid::new_v4().to_string(),
        requires_destructive_approval: false,
    };
    let result = ApplyFlowIrCommitResult {
        status: "applied".to_string(),
        replayed: false,
        code: None,
        message: "Applied exact typed batch.".to_string(),
        commands: Vec::new(),
        board_commands: Vec::new(),
        diagnostics: Vec::new(),
        final_board_node_count: Some(3),
    };

    retain_flow_ir_applied_receipt("receipt-app", &token, &result);
    let replay = replay_flow_ir_applied_receipt("receipt-app", &token)
        .expect("exact token replays its applied receipt");
    assert_eq!(replay.status, "applied");
    assert!(replay.replayed);
    assert_eq!(replay.final_board_node_count, Some(3));
    assert!(replay.message.contains("idempotent replay"));

    let mut wrong_claim = token.clone();
    wrong_claim.claim_id = uuid::Uuid::new_v4().to_string();
    assert!(replay_flow_ir_applied_receipt("receipt-app", &wrong_claim).is_none());

    FLOW_IR_APPLIED_RECEIPTS
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
        .remove(&flow_ir_applied_receipt_key("receipt-app", &token));
}
