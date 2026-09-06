use super::*;

#[test]
fn typed_workflow_path_retains_revision_and_cannot_mix_raw_edits() {
    let state = Arc::new(StdMutex::new(WorkflowToolLoopState::default()));

    assert!(workflow_tool_preflight(&state, "plan_flow_ir").is_none());
    assert!(workflow_tool_preflight(&state, "begin_flow_ir_draft").is_none());
    workflow_tool_record(
        &state,
        "begin_flow_ir_draft",
        &serde_json::json!({ "draft_id": "support-agent" }),
        &serde_json::json!({
            "status": "draft_started",
            "draft_id": "support-agent",
            "revision": 0,
            "diagnostics": []
        })
        .to_string(),
    );
    assert!(workflow_tool_preflight(&state, "upsert_flow_ir_module").is_none());
    workflow_tool_record(
        &state,
        "upsert_flow_ir_module",
        &serde_json::json!({ "draft_id": "support-agent", "expected_revision": 0 }),
        &serde_json::json!({
            "status": "module_needs_repair",
            "draft_id": "support-agent",
            "revision": 1,
            "diagnostics": [{
                "code": "IR_INPUT_TYPE",
                "message": "Generic output requires an explicit conversion"
            }]
        })
        .to_string(),
    );

    let guard = state.lock().expect("state lock");
    assert_eq!(guard.typed_draft_id.as_deref(), Some("support-agent"));
    assert_eq!(guard.typed_revision, Some(1));
    assert_eq!(guard.mutation_path, Some(WorkflowMutationPath::TypedIr));
    assert!(guard.last_errors[0].contains("IR_INPUT_TYPE"));
    drop(guard);

    let conflict = workflow_tool_preflight(&state, "edit_flowscript")
        .expect("raw mutation must be blocked after a typed draft starts");
    assert_eq!(conflict.is_error, Some(true));

    let snapshot = state.lock().expect("state lock").snapshot();
    let prompt = build_external_workflow_continuation_prompt(
        "build the support workflow",
        Some(&snapshot),
        1,
    );
    assert!(prompt.contains("draft_id=support-agent"));
    assert!(prompt.contains("latest revision=1"));
    assert!(prompt.contains("do not edit generated FlowScript text"));
}

#[test]
fn typed_ir_budget_counts_every_dispatched_phase() {
    assert_eq!(
        typed_ir_operation_budget(0),
        MIN_EXTERNAL_TYPED_IR_OPERATION_BUDGET
    );
    assert_eq!(
        typed_ir_operation_budget(usize::MAX),
        MAX_EXTERNAL_TYPED_IR_OPERATION_BUDGET
    );
    let state = Arc::new(StdMutex::new(WorkflowToolLoopState::default()));
    let calls = [
        (
            "plan_flow_ir",
            serde_json::json!({ "modules": [{ "name": "classify" }, { "name": "eventsSimple" }] }),
            serde_json::json!({ "feasible": true, "requirements": [] }),
        ),
        (
            "begin_flow_ir_draft",
            serde_json::json!({
                "draft_id": "typed-counts",
                "expected_modules": ["classify", "eventsSimple"]
            }),
            serde_json::json!({
                "status": "draft_started",
                "draft_id": "typed-counts",
                "revision": 0,
                "missing_modules": ["classify", "eventsSimple"]
            }),
        ),
        (
            "update_flow_ir_draft",
            serde_json::json!({ "draft_id": "typed-counts", "expected_revision": 0 }),
            serde_json::json!({
                "status": "draft_updated",
                "draft_id": "typed-counts",
                "revision": 1
            }),
        ),
        (
            "upsert_flow_ir_module",
            serde_json::json!({
                "draft_id": "typed-counts",
                "expected_revision": 1,
                "module": { "kind": "function", "name": "classify", "steps": [] }
            }),
            serde_json::json!({
                "status": "module_validated",
                "draft_id": "typed-counts",
                "revision": 2,
                "missing_modules": ["eventsSimple"]
            }),
        ),
        (
            "validate_flow_ir_draft",
            serde_json::json!({ "draft_id": "typed-counts" }),
            serde_json::json!({
                "status": "draft_valid",
                "draft_id": "typed-counts",
                "revision": 2
            }),
        ),
        (
            "commit_flow_ir_draft",
            serde_json::json!({ "draft_id": "typed-counts", "expected_revision": 2 }),
            serde_json::json!({
                "status": "queued",
                "draft_id": "typed-counts",
                "revision": 2
            }),
        ),
    ];

    for (tool, args, result) in calls {
        assert!(
            workflow_tool_preflight_with_args(&state, tool, &args).is_none(),
            "{tool} should be dispatched"
        );
        workflow_tool_record(&state, tool, &args, &result.to_string());
    }

    let state = state.lock().expect("state lock");
    assert_eq!(state.typed_operation_attempts, 6);
    assert_eq!(state.typed_expected_modules, 2);
    assert!(state.queued);
}

#[test]
fn typed_ir_module_scaled_budget_returns_recoverable_draft_state() {
    let state = Arc::new(StdMutex::new(WorkflowToolLoopState::default()));
    let begin_args = serde_json::json!({
        "draft_id": "support-timeout-recovery",
        "expected_modules": (0..10)
            .map(|index| format!("module_{index}"))
            .collect::<Vec<_>>()
    });
    assert!(
        workflow_tool_preflight_with_args(&state, "begin_flow_ir_draft", &begin_args).is_none()
    );
    workflow_tool_record(
        &state,
        "begin_flow_ir_draft",
        &begin_args,
        &serde_json::json!({
            "status": "draft_started",
            "draft_id": "support-timeout-recovery",
            "revision": 0
        })
        .to_string(),
    );

    let budget = typed_ir_operation_budget(10);
    for attempt in 1..budget {
        let args = serde_json::json!({ "draft_id": "support-timeout-recovery" });
        assert!(
            workflow_tool_preflight_with_args(&state, "validate_flow_ir_draft", &args).is_none(),
            "operation {attempt} should fit the module-scaled budget"
        );
        workflow_tool_record(
            &state,
            "validate_flow_ir_draft",
            &args,
            &serde_json::json!({
                "status": "validation_errors",
                "draft_id": "support-timeout-recovery",
                "revision": attempt,
                "diagnostics": [{
                    "code": "IR_REMAINING",
                    "message": format!("remaining repair {attempt}")
                }],
                "missing_modules": ["send_reply"]
            })
            .to_string(),
        );
    }

    let rejected = workflow_tool_preflight_with_args(
        &state,
        "commit_flow_ir_draft",
        &serde_json::json!({
            "draft_id": "support-timeout-recovery",
            "expected_revision": budget - 1
        }),
    )
    .expect("the operation after the hard cap must stop locally");
    assert_eq!(rejected.is_error, Some(true));
    let payload = workflow_call_result_json(&rejected);
    assert_eq!(payload["status"], "typed_repair_budget_exhausted");
    assert_eq!(payload["code"], "TYPED_IR_OPERATION_BUDGET_EXHAUSTED");
    assert_eq!(payload["draft_retained"], true);
    assert_eq!(payload["draft_id"], "support-timeout-recovery");
    assert_eq!(payload["revision"], u64::from(budget - 1));
    assert_eq!(payload["operation_attempts"], u64::from(budget));
    assert_eq!(payload["operation_budget"], u64::from(budget));
    assert_eq!(
        payload["missing_modules"],
        serde_json::json!(["send_reply"])
    );
    assert!(
        payload["remaining_diagnostics"][0]
            .as_str()
            .is_some_and(|diagnostic| diagnostic.contains("IR_REMAINING"))
    );
    assert_eq!(
        state.lock().expect("state lock").typed_operation_attempts,
        budget
    );
    let redirected = workflow_tool_preflight(&state, "get_current_flowscript")
        .expect("all workflow-loop tools stay terminal after typed budget exhaustion");
    assert_eq!(
        workflow_call_result_json(&redirected)["status"],
        "typed_repair_budget_exhausted"
    );
}

#[test]
fn typed_ir_loop_stops_repeated_identical_module_diagnostics() {
    let state = Arc::new(StdMutex::new(WorkflowToolLoopState::default()));
    let begin_args = serde_json::json!({
        "draft_id": "stalled-module",
        "expected_modules": ["classify"]
    });
    assert!(
        workflow_tool_preflight_with_args(&state, "begin_flow_ir_draft", &begin_args).is_none()
    );
    workflow_tool_record(
        &state,
        "begin_flow_ir_draft",
        &begin_args,
        &serde_json::json!({
            "status": "draft_started",
            "draft_id": "stalled-module",
            "revision": 0
        })
        .to_string(),
    );

    for revision in 1..=u64::from(MAX_EXTERNAL_TYPED_IR_STALLED_ATTEMPTS) + 1 {
        let args = serde_json::json!({
            "draft_id": "stalled-module",
            "expected_revision": revision - 1,
            "module": { "kind": "function", "name": "classify", "steps": [] }
        });
        assert!(
            workflow_tool_preflight_with_args(&state, "upsert_flow_ir_module", &args).is_none()
        );
        workflow_tool_record(
            &state,
            "upsert_flow_ir_module",
            &args,
            &serde_json::json!({
                "status": "module_needs_repair",
                "draft_id": "stalled-module",
                "revision": revision,
                "diagnostics": [{
                    "code": "IR_INPUT_TYPE",
                    "message": "the exact same conversion is still missing"
                }]
            })
            .to_string(),
        );
    }

    let attempts_before_rejection = state.lock().expect("state lock").typed_operation_attempts;
    let rejected = workflow_tool_preflight_with_args(
        &state,
        "upsert_flow_ir_module",
        &serde_json::json!({
            "draft_id": "stalled-module",
            "expected_revision": 4,
            "module": { "kind": "function", "name": "classify", "steps": [] }
        }),
    )
    .expect("the next repeated module repair must stop locally");
    let payload = workflow_call_result_json(&rejected);
    assert_eq!(payload["status"], "typed_repair_progress_stalled");
    assert_eq!(payload["draft_id"], "stalled-module");
    assert_eq!(payload["revision"], 4);
    assert_eq!(payload["stalled_attempts"], 3);
    assert_eq!(
        state.lock().expect("state lock").typed_operation_attempts,
        attempts_before_rejection,
        "the rejected preflight is not a dispatched operation or progress"
    );
}

#[test]
fn typed_ir_schema_failures_do_not_claim_an_unstarted_draft_is_retained() {
    let state = Arc::new(StdMutex::new(WorkflowToolLoopState::default()));
    let args = serde_json::json!({
        "draft_id": "never-started",
        "expected_modules": ["eventsSimple"],
        "program": { "interfaces": [{ "type": "string" }] }
    });
    for _ in 0..=MAX_EXTERNAL_TYPED_IR_STALLED_ATTEMPTS {
        assert!(workflow_tool_preflight_with_args(&state, "begin_flow_ir_draft", &args).is_none());
        workflow_tool_record(
            &state,
            "begin_flow_ir_draft",
            &args,
            "invalid params: FlowIrType must use the canonical object shape",
        );
    }

    let rejected = workflow_tool_preflight_with_args(&state, "begin_flow_ir_draft", &args)
        .expect("repeated invalid begin calls must stop locally");
    let payload = workflow_call_result_json(&rejected);
    assert_eq!(payload["status"], "typed_repair_progress_stalled");
    assert_eq!(payload["draft_retained"], false);
    assert_eq!(payload["draft_id"], "never-started");
    assert_eq!(payload["revision"], serde_json::Value::Null);
    assert_eq!(payload["next_action"], "stop_and_report_begin_failure");
}

#[test]
fn typed_ir_infeasible_begin_with_draft_id_is_not_resumable() {
    let state = Arc::new(StdMutex::new(WorkflowToolLoopState::default()));
    let args = serde_json::json!({
        "draft_id": "infeasible-not-stored",
        "expected_modules": ["eventsSimple"]
    });
    assert!(workflow_tool_preflight_with_args(&state, "begin_flow_ir_draft", &args).is_none());
    workflow_tool_record(
        &state,
        "begin_flow_ir_draft",
        &args,
        &serde_json::json!({
            "status": "infeasible",
            "code": "IR_CAPABILITY_PLAN_INFEASIBLE",
            "draft_id": "infeasible-not-stored",
            "revision": null,
            "message": "The draft was not started",
            "diagnostics": [{
                "code": "IR_CAPABILITY_UNAVAILABLE",
                "message": "SMTP is unavailable"
            }]
        })
        .to_string(),
    );

    let state = state.lock().expect("state lock");
    assert_eq!(
        state.typed_draft_id.as_deref(),
        Some("infeasible-not-stored")
    );
    assert!(!state.typed_draft_retained);
    let snapshot = state.snapshot();
    drop(state);
    let continuation =
        build_external_workflow_continuation_prompt("build support mail", Some(&snapshot), 1);
    assert!(continuation.contains("TYPED DRAFT WAS NOT STARTED"));
    assert!(!continuation.contains("RETAINED TYPED DRAFT"));
}

#[test]
fn raw_workflow_path_cannot_switch_to_typed_mid_repair() {
    let state = Arc::new(StdMutex::new(WorkflowToolLoopState::default()));
    assert!(workflow_tool_preflight(&state, "edit_flowscript").is_none());
    workflow_tool_record(
        &state,
        "edit_flowscript",
        &serde_json::json!({ "flowscript": "eventsSimple() { brokenCall() }" }),
        &serde_json::json!({
            "status": "validation_errors",
            "errors": ["brokenCall does not match a catalog declaration"]
        })
        .to_string(),
    );
    let conflict = workflow_tool_preflight(&state, "begin_flow_ir_draft")
        .expect("typed mutation must be blocked after raw repair starts");
    assert_eq!(conflict.is_error, Some(true));
    assert_eq!(
        state.lock().expect("state lock").mutation_path,
        Some(WorkflowMutationPath::FlowScript)
    );
    assert_eq!(
        state.lock().expect("state lock").typed_operation_attempts,
        0,
        "a mutation-path preflight rejection is neither a dispatch nor progress"
    );
}

#[test]
fn representation_rejected_emit_does_not_block_the_flowscript_path() {
    let state = Arc::new(StdMutex::new(WorkflowToolLoopState::default()));
    let forbidden = serde_json::json!({
        "commands": [{
            "command_type": "AddNode",
            "node_type": "log_info",
            "ref_id": "$0",
            "position": { "x": 0, "y": 0 },
            "summary": "Add log"
        }],
        "explanation": "Build behavior"
    });

    assert!(workflow_tool_preflight_with_args(&state, "emit_commands", &forbidden).is_none());
    assert_eq!(state.lock().expect("state lock").mutation_path, None);

    assert!(
        workflow_tool_preflight_with_args(
            &state,
            "get_declarations",
            &serde_json::json!({ "queries": ["log information"] }),
        )
        .is_none()
    );
    workflow_tool_record(
        &state,
        "get_declarations",
        &serde_json::json!({ "queries": ["log information"] }),
        "declare function logInfo({ message: string }): void;  // impure",
    );
    accept_single_segment_plan(&state);

    assert!(
        workflow_tool_preflight_with_args(
            &state,
            "write_flowscript",
            &serde_json::json!({
                "draft_id": "redirected-source",
                "source": "eventsSimple() { logInfo({ message: \"hello\" }) }"
            }),
        )
        .is_none()
    );
    assert_eq!(
        state.lock().expect("state lock").mutation_path,
        Some(WorkflowMutationPath::FlowScript)
    );

    let visual_state = Arc::new(StdMutex::new(WorkflowToolLoopState::default()));
    assert!(
        workflow_tool_preflight_with_args(
            &visual_state,
            "emit_commands",
            &serde_json::json!({
                "commands": [{
                    "command_type": "MoveNode",
                    "node_id": "node-1",
                    "position": { "x": 20, "y": 40 },
                    "summary": "Move node"
                }],
                "explanation": "Align nodes"
            }),
        )
        .is_none()
    );
    assert_eq!(
        visual_state.lock().expect("state lock").mutation_path,
        Some(WorkflowMutationPath::DirectCommands)
    );
}
