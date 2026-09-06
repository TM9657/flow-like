use super::*;

#[test]
fn shared_tool_enforcement_is_role_neutral() {
    for specialist_term in [
        "FlowScript",
        "A2UI",
        "emit_ui",
        "get_declarations",
        "write_flowscript",
        "database_tool",
        "storage_tool",
        "execute_node",
    ] {
        assert!(
            !TOOL_ENFORCEMENT_RULES.contains(specialist_term),
            "shared enforcement leaked specialist instruction `{specialist_term}`"
        );
    }
    assert!(TOOL_ENFORCEMENT_RULES.contains("role-specific specialist boundary"));
    assert!(TOOL_ENFORCEMENT_RULES.contains("actually registered in this session"));
}

#[test]
fn frontend_prompts_enforce_ui_only_ownership_and_board_handoff() {
    let prompts = [
        frontend_system_prompt("{}", ""),
        frontend_sdk_system_prompt(),
    ];

    for prompt in prompts {
        assert!(prompt.contains("## SPECIALIST BOUNDARY: UI ONLY"));
        assert!(prompt.contains("You own only pages, widgets, and A2UI component trees"));
        assert!(prompt.contains("Never inspect, author, validate, submit, or explain FlowScript"));
        assert!(prompt.contains("Never author app data"));
        assert!(prompt.contains("Runtime VERIFICATION of persisted work is in scope"));
        assert!(prompt.contains("Board specialist must handle workflow wiring."));
        assert!(prompt.contains("Do not claim that fetching"));

        for workflow_tool in [
            "get_current_flowscript",
            "get_declarations",
            "write_flowscript",
            "patch_flowscript",
            "check_flowscript",
            "commit_flowscript",
            "edit_flowscript",
            "emit_commands",
        ] {
            assert!(
                !prompt.contains(workflow_tool),
                "frontend prompt exposed workflow lifecycle tool `{workflow_tool}`"
            );
        }
    }
}

#[test]
fn board_prompts_enforce_workflow_only_ownership_and_read_only_support() {
    let prompts = [
        board_system_prompt("{}", "", 0, false, false),
        board_sdk_system_prompt(),
        board_sdk_flowscript_system_prompt("", 0),
    ];

    for prompt in prompts {
        assert!(prompt.contains("## SPECIALIST BOUNDARY: WORKFLOW BOARD ONLY"));
        assert!(prompt.contains("Never create or edit pages, widgets, or A2UI component trees"));
        assert!(prompt.contains("Cross-domain support is inspection-only"));
        assert!(prompt.contains("Never create, update, or delete app data"));
        assert!(prompt.contains("Do not execute the queued draft in that same"));
        assert!(prompt.contains("database_tool"));
        assert!(prompt.contains("list_tables/describe_table/read-only query only"));
        assert!(prompt.contains("storage_tool (list/read only)"));
        assert!(prompt.contains("Post-apply runtime verification belongs to a later orchestrator"));
    }
}
