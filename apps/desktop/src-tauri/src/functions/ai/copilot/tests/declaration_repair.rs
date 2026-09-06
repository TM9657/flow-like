use super::*;

#[test]
fn declaration_repairs_allow_new_diagnostic_signatures_but_deduplicate_old_ones() {
    let state = Arc::new(StdMutex::new(WorkflowToolLoopState::default()));
    let initial_lookup = serde_json::json!({ "queries": ["imap receive", "smtp send"] });
    assert!(
        workflow_tool_preflight_with_args(&state, "get_declarations", &initial_lookup).is_none()
    );
    workflow_tool_record(
        &state,
        "get_declarations",
        &initial_lookup,
        concat!(
            "// flowpilot.declaration-batch/v1 {\"processed_count\":2,\"matched_count\":2,\"matched_queries\":[\"imap receive\",\"smtp send\"],\"unmatched_count\":0,\"unmatched_queries\":[],\"output_omitted_count\":0,\"output_omitted_queries\":[],\"complete\":true,\"omitted_count\":0,\"omitted_queries\":[],\"truncated_query_count\":0}\n",
            "declare function mailImapList({ inbox: Struct }): Struct[];\n",
            "declare function emailSmtpSend({ to: string }): void;"
        ),
    );

    let failed_edit = |signature: &str| {
        assert!(workflow_tool_preflight(&state, "edit_flowscript").is_none());
        workflow_tool_record(
                &state,
                "edit_flowscript",
                &serde_json::json!({
                    "flowscript": format!("eventsSimple() {{ {signature}() }}")
                }),
                &serde_json::json!({
                    "status": "validation_errors",
                    "errors": [format!("FlowScript call `{signature}` does not match a catalog declaration; call `get_declarations` and use the exact function name")]
                })
                .to_string(),
            );
    };

    failed_edit("firstMissingCall");
    let first_repair = serde_json::json!({ "queries": ["exact firstMissingCall signature"] });
    assert!(
        workflow_tool_preflight_with_args(&state, "get_declarations", &first_repair).is_none(),
        "the first diagnostic-targeted repair should be dispatched"
    );
    workflow_tool_record(
        &state,
        "get_declarations",
        &first_repair,
        "declare function firstMissingCall({}): void;",
    );

    failed_edit("firstMissingCall");
    let duplicate = workflow_tool_preflight_with_args(
        &state,
        "get_declarations",
        &serde_json::json!({ "queries": ["firstMissingCall"] }),
    )
    .expect("a duplicate targeted signature must be redirected locally");
    assert_eq!(duplicate.is_error, Some(false));
    let duplicate = duplicate
        .content
        .iter()
        .filter_map(|content| match &content.raw {
            rmcp::model::RawContent::Text(text) => Some(text.text.as_str()),
            _ => None,
        })
        .collect::<Vec<_>>()
        .join("\n");
    assert!(
        duplicate.contains("duplicate_declaration_lookup"),
        "{duplicate}"
    );

    failed_edit("secondMissingCall");
    assert!(
        workflow_tool_preflight_with_args(
            &state,
            "get_declarations",
            &serde_json::json!({ "queries": ["secondMissingCall"] }),
        )
        .is_none(),
        "a distinct signature from a new diagnostic must remain available after the old two-call cap"
    );
    assert_eq!(state.lock().expect("state lock").declaration_calls, 3);
}

#[test]
fn declaration_repair_hints_cover_real_pin_and_type_diagnostics() {
    let hints = diagnostic_declaration_repair_hints(&[
            "binary comparison `==` has incompatible operand types `Generic` and `String`"
                .to_string(),
            "argument `condition` on `controlBranch` is not a literal or resolvable node output; skipped connection"
                .to_string(),
            "node `stringContains` has no input pin named `value`; skipped that argument"
                .to_string(),
            "binary comparison `==` has ambiguous operand type; candidates are equal_string, bool_equal, int_equal"
                .to_string(),
        ]);

    assert!(hints.exact_symbols.contains("controlbranch"));
    assert!(hints.exact_symbols.contains("stringcontains"));
    assert!(hints.exact_symbols.contains("equal_string"));
    assert!(hints.exact_symbols.contains("bool_equal"));
    assert!(hints.exact_symbols.contains("int_equal"));
    assert!(!hints.exact_symbols.contains("condition"));
    assert!(!hints.exact_symbols.contains("generic"));
    assert!(hints.topics.contains("comparison"));
    assert!(hints.topics.contains("type_conversion"));
    assert!(hints.topics.contains("string_operations"));
}

#[test]
fn declaration_repairs_allow_bounded_type_and_pin_lookup_batches() {
    let state = Arc::new(StdMutex::new(WorkflowToolLoopState::default()));
    let initial_lookup = serde_json::json!({ "queries": ["boolean comparison"] });
    assert!(
        workflow_tool_preflight_with_args(&state, "get_declarations", &initial_lookup).is_none()
    );
    workflow_tool_record(
        &state,
        "get_declarations",
        &initial_lookup,
        "declare function boolEqual({ boolean: boolean, boolean: boolean }): boolean;",
    );
    assert!(workflow_tool_preflight(&state, "edit_flowscript").is_none());
    workflow_tool_record(
        &state,
        "edit_flowscript",
        &serde_json::json!({ "flowscript": "eventsSimple() { brokenCall() }" }),
        &serde_json::json!({
            "status": "validation_errors",
            "errors": [
                "binary comparison `==` has incompatible operand types `Generic` and `String`",
                "node `stringContains` has no input pin named `value`; skipped that argument"
            ]
        })
        .to_string(),
    );

    let repair_lookup = serde_json::json!({
        "queries": [
            "equalString notEqualString exact signatures",
            "stringContains exact signature",
            "stringTrim exact signature",
            "stringStartsWith exact signature",
            "stringReplace exact signature",
            "integer add increment exact signature",
            "convert Generic any to string",
            "try catch error boundary invoke function per item"
        ]
    });
    assert!(
        workflow_tool_preflight_with_args(&state, "get_declarations", &repair_lookup).is_none(),
        "pin and type diagnostics should authorize one bounded corrective batch"
    );
    let repair_result = format!(
        "// flowpilot.declaration-batch/v1 {}\n{}",
        serde_json::json!({
            "processed_count": 8,
            "matched_count": 8,
            "matched_queries": repair_lookup["queries"],
            "unmatched_count": 0,
            "unmatched_queries": [],
            "output_omitted_count": 0,
            "output_omitted_queries": [],
            "complete": true,
            "omitted_count": 0,
            "omitted_queries": [],
            "truncated_query_count": 0
        }),
        concat!(
            "declare function equalString({ left: string, right: string }): boolean;\n",
            "declare function stringContains({ text: string, pattern: string }): boolean;\n",
            "declare function stringTrim({ text: string }): string;\n",
            "declare function stringStartsWith({ text: string, prefix: string }): boolean;\n",
            "declare function stringReplace({ text: string, pattern: string, replacement: string, isRegex: boolean }): string;\n",
            "declare function integerAdd({ integer: integer, integer: integer }): integer;\n",
            "declare function genericToString({ value: any }): string;\n",
            "declare function tryCatch({ invoke: Struct }): Struct;"
        )
    );
    workflow_tool_record(&state, "get_declarations", &repair_lookup, &repair_result);
    let state = state.lock().expect("state lock");
    let completed = &state.completed_repair_lookup_keys;
    assert!(completed.contains("topic:comparison"));
    assert!(completed.contains("topic:type_conversion"));
    assert!(completed.contains("topic:string_operations"));
    assert!(completed.contains("symbol:stringcontains"));
}

#[test]
fn declaration_repairs_reject_broad_queries_even_when_they_name_a_target() {
    let hints = diagnostic_declaration_repair_hints(&[
        "node `stringContains` has no input pin named `value`".to_string(),
    ]);
    assert!(!declaration_repair_query_is_bounded(
        "stringContains and every node in the entire catalog"
    ));
    assert!(
        declaration_repair_query_keys("stringContains exact signature", &hints)
            .contains("symbol:stringcontains")
    );
}

#[test]
fn authoritative_repair_no_match_is_deduplicated_but_abort_is_retryable() {
    let make_state = || {
        Arc::new(StdMutex::new(WorkflowToolLoopState {
                declaration_calls: 1,
                initial_declaration_lookup_complete: true,
                last_errors: vec!["FlowScript call `missingCall` does not match a catalog declaration; call `get_declarations` and use the exact function name".to_string()],
                ..Default::default()
            }))
    };
    let args = serde_json::json!({ "queries": ["missingCall exact signature"] });

    let no_match_state = make_state();
    assert!(
        workflow_tool_preflight_with_args(&no_match_state, "get_declarations", &args).is_none()
    );
    workflow_tool_record(
        &no_match_state,
        "get_declarations",
        &args,
        "// flowpilot.declaration-batch/v1 {\"processed_count\":1,\"matched_count\":0,\"matched_queries\":[],\"unmatched_count\":1,\"unmatched_queries\":[\"missingCall exact signature\"],\"output_omitted_count\":0,\"output_omitted_queries\":[],\"complete\":false,\"omitted_count\":0,\"omitted_queries\":[],\"truncated_query_count\":0}\nNo declaration matched.",
    );
    {
        let mut state = no_match_state.lock().expect("state lock");
        assert!(
            state
                .completed_repair_lookup_keys
                .contains("symbol:missingcall")
        );
        // A normal edit/check result opens the next diagnostic phase. Reproduce that boundary
        // directly so this assertion exercises persistent key deduplication, not merely the
        // one-discovery-per-edit guard.
        state.declarations_since_edit = 0;
    }
    let duplicate = workflow_tool_preflight_with_args(&no_match_state, "get_declarations", &args)
        .expect("a definitive catalog miss must not reopen the same repair forever");
    assert_eq!(
        workflow_call_result_json(&duplicate)["status"],
        "duplicate_declaration_lookup"
    );

    let aborted_state = make_state();
    assert!(workflow_tool_preflight_with_args(&aborted_state, "get_declarations", &args).is_none());
    workflow_tool_abort(&aborted_state, "get_declarations", "worker disconnected");
    assert!(
        workflow_tool_preflight_with_args(&aborted_state, "get_declarations", &args).is_none(),
        "a transport abort consumes neither the diagnostic key nor its retry allowance"
    );
}

#[test]
fn output_omitted_repair_gets_one_focused_retry_then_stops() {
    let state = Arc::new(StdMutex::new(WorkflowToolLoopState {
            declaration_calls: 1,
            initial_declaration_lookup_complete: true,
            last_errors: vec!["FlowScript call `missingCall` does not match a catalog declaration; call `get_declarations` and use the exact function name".to_string()],
            ..Default::default()
        }));
    let args = serde_json::json!({ "queries": ["missingCall exact signature"] });
    let omitted = "// flowpilot.declaration-batch/v1 {\"processed_count\":1,\"matched_count\":0,\"matched_queries\":[],\"unmatched_count\":0,\"unmatched_queries\":[],\"output_omitted_count\":1,\"output_omitted_queries\":[\"missingCall exact signature\"],\"complete\":false,\"omitted_count\":0,\"omitted_queries\":[],\"truncated_query_count\":0}\nThe exact signature did not fit.";

    for attempt in 0..MAX_REPAIR_DECLARATION_ATTEMPTS_PER_KEY {
        assert!(
            workflow_tool_preflight_with_args(&state, "get_declarations", &args).is_none(),
            "omitted-signature attempt {attempt} should dispatch"
        );
        workflow_tool_record(&state, "get_declarations", &args, omitted);
    }
    let exhausted = workflow_tool_preflight_with_args(&state, "get_declarations", &args)
        .expect("the per-key omission cap must terminate exact repeats");
    assert_eq!(
        workflow_call_result_json(&exhausted)["status"],
        "duplicate_declaration_lookup"
    );
}

#[test]
fn metadata_less_multi_query_repair_does_not_claim_every_target_resolved() {
    let state = Arc::new(StdMutex::new(WorkflowToolLoopState {
            declaration_calls: 1,
            initial_declaration_lookup_complete: true,
            last_errors: vec!["FlowScript call `missingCall` does not match a catalog declaration; call `get_declarations` and use the exact function name".to_string()],
            ..Default::default()
        }));
    let broad = serde_json::json!({
        "queries": ["missingCall exact signature", "missingCall input pins"]
    });
    assert!(workflow_tool_preflight_with_args(&state, "get_declarations", &broad).is_none());
    workflow_tool_record(
        &state,
        "get_declarations",
        &broad,
        "declare function missingCall({ input: string }): void;",
    );
    assert!(
        state
            .lock()
            .expect("state lock")
            .completed_repair_lookup_keys
            .is_empty()
    );
    assert!(
        workflow_tool_preflight_with_args(
            &state,
            "get_declarations",
            &serde_json::json!({ "queries": ["missingCall exact signature"] }),
        )
        .is_none(),
        "one focused retry remains available when a legacy multi-result cannot prove coverage"
    );
}

#[test]
fn declaration_repair_rejects_unrelated_broad_searches() {
    let state = Arc::new(StdMutex::new(WorkflowToolLoopState::default()));
    let initial_lookup = serde_json::json!({ "queries": ["missing call"] });
    assert!(
        workflow_tool_preflight_with_args(&state, "get_declarations", &initial_lookup).is_none()
    );
    workflow_tool_record(
        &state,
        "get_declarations",
        &initial_lookup,
        "declare function missingCall({}): void;",
    );
    assert!(workflow_tool_preflight(&state, "edit_flowscript").is_none());
    workflow_tool_record(
            &state,
            "edit_flowscript",
            &serde_json::json!({ "flowscript": "eventsSimple() { missingCall() }" }),
            &serde_json::json!({
                "status": "validation_errors",
                "errors": ["FlowScript call `missingCall` does not match a catalog declaration; call `get_declarations` and use the exact function name"]
            })
            .to_string(),
        );

    let redirected = workflow_tool_preflight_with_args(
        &state,
        "get_declarations",
        &serde_json::json!({ "queries": ["search the entire mail catalog"] }),
    )
    .expect("unrelated repair discovery must be redirected");
    assert_eq!(redirected.is_error, Some(false));
    let text = redirected
        .content
        .iter()
        .filter_map(|content| match &content.raw {
            rmcp::model::RawContent::Text(text) => Some(text.text.as_str()),
            _ => None,
        })
        .collect::<Vec<_>>()
        .join("\n");
    assert!(text.contains("diagnostic_lookup_required"), "{text}");
    assert_eq!(state.lock().expect("state lock").declaration_calls, 1);
}

#[test]
fn workflow_edit_session_defers_runtime_verification_until_persisted() {
    let state = Arc::new(StdMutex::new(WorkflowToolLoopState::default()));

    for tool in [
        "execute_event",
        "execute_node",
        "query_execution_logs",
        "run_board_tests",
    ] {
        let result = workflow_tool_preflight(&state, tool)
            .expect("mutation sessions must return an explicit runtime deferral");
        assert_eq!(result.is_error, Some(true));
        let text = result
            .content
            .iter()
            .filter_map(|content| match &content.raw {
                rmcp::model::RawContent::Text(text) => Some(text.text.as_str()),
                _ => None,
            })
            .collect::<Vec<_>>()
            .join("\n");
        assert!(text.contains("runtime_verification_deferred"), "{text}");
        assert!(text.contains("later turn"), "{text}");
    }
}

#[test]
fn workflow_edit_session_defers_table_creation_until_board_draft_is_queued() {
    let state = Arc::new(StdMutex::new(WorkflowToolLoopState::default()));
    let create_table = serde_json::json!({
        "operation": "create_table",
        "table_name": "support_tickets",
        "fields": [{ "name": "ticket_id", "type": "string" }]
    });

    let result = workflow_database_setup_preflight(&state, "database_tool", &create_table)
        .expect("schema creation before the first board draft must be rejected locally");
    assert_eq!(result.is_error, Some(false));
    let text = result
        .content
        .iter()
        .filter_map(|content| match &content.raw {
            rmcp::model::RawContent::Text(text) => Some(text.text.as_str()),
            _ => None,
        })
        .collect::<Vec<_>>()
        .join("\n");
    assert!(text.contains("board_draft_required_before_database_setup"));
    assert!(text.contains("no approval was opened"));

    assert!(
        workflow_database_setup_preflight(
            &state,
            "database_tool",
            &serde_json::json!({ "operation": "list_tables" }),
        )
        .is_none(),
        "read-only database inspection remains available before the edit"
    );

    state.lock().expect("state lock").queued = true;
    assert!(
        workflow_database_setup_preflight(&state, "database_tool", &create_table).is_none(),
        "schema setup may proceed after a complete board draft has queued"
    );
}

#[test]
fn ancillary_predraft_inspection_is_bounded_until_source_is_retained() {
    let state = Arc::new(StdMutex::new(WorkflowToolLoopState::default()));
    let tools = ["database_tool", "ui_inspect", "storage_tool"];
    for call in 0..MAX_EXTERNAL_PREDRAFT_CONTEXT_READS {
        assert!(
            workflow_predraft_context_preflight(
                &state,
                tools[usize::from(call) % tools.len()],
                &serde_json::Value::Null,
            )
            .is_none(),
            "ancillary context call {call} should fit the pre-draft budget"
        );
    }

    state
        .lock()
        .expect("state lock")
        .initial_declaration_lookup_usable = true;
    let blocked =
        workflow_predraft_context_preflight(&state, "database_tool", &serde_json::Value::Null)
            .expect("the next exhaustive inspection must redirect to source retention");
    let blocked = workflow_call_result_json(&blocked);
    assert_eq!(blocked["status"], "predraft_inspection_budget_exhausted");
    assert_eq!(blocked["next_action"], "plan_board_scope");
    assert_eq!(
        blocked["inspection_budget"],
        u64::from(MAX_EXTERNAL_PREDRAFT_CONTEXT_READS)
    );

    state.lock().expect("state lock").flowscript_draft_retained = true;
    assert!(
        workflow_predraft_context_preflight(&state, "database_tool", &serde_json::Value::Null,)
            .is_none(),
        "focused inspection is available again after a recoverable source exists"
    );
}

#[test]
fn failed_empty_event_retry_preserves_last_actionable_flowscript() {
    let state = Arc::new(StdMutex::new(WorkflowToolLoopState::default()));
    let actionable = r#"eventsSimple() {
    logInfo({ message: "poll support inbox" })
}"#;

    workflow_tool_record(
        &state,
        "edit_flowscript",
        &serde_json::json!({ "flowscript": actionable }),
        &serde_json::json!({
            "status": "validation_errors",
            "errors": ["fix one connection"]
        })
        .to_string(),
    );
    workflow_tool_record(
        &state,
        "edit_flowscript",
        &serde_json::json!({ "flowscript": "eventsSimple() {\n}" }),
        &serde_json::json!({
            "status": "validation_errors",
            "errors": ["empty event entries do not implement a workflow"]
        })
        .to_string(),
    );

    let snapshot = state.lock().expect("state lock").snapshot();
    assert_eq!(snapshot.last_flowscript.as_deref(), Some(actionable));
    assert_eq!(snapshot.last_status.as_deref(), Some("validation_errors"));
    assert_eq!(snapshot.last_errors, vec!["fix one connection"]);
}
