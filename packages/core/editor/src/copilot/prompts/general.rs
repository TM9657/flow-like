//! Prompt builders for combined workflow and frontend sessions.

use super::board::{
    A2UI_STATE_GUIDANCE, BOARD_ORGANIZATION_GUIDANCE, DASHBOARD_A2UI_GUIDANCE,
    DATABASE_WORKFLOW_GUIDANCE, DYNAMIC_PIN_GUIDANCE, EXECUTION_FLOW_GUIDANCE,
    EXPLANATION_WORKFLOW_GUIDANCE, FLOW_PATH_ACCESSOR_GUIDANCE, FUNCTION_CACHE_GUIDANCE,
    NUMBERS_CONVERSIONS_GUIDANCE,
};
use super::shared::{
    AUTONOMY_PLACEHOLDER_GUIDANCE, SCOPE_SEGMENTATION_GUIDANCE, TESTING_GUIDANCE,
    TOOL_ENFORCEMENT_RULES, UNBUILDABLE_UNIT_GUIDANCE,
};

/// Header shared by both general-prompt variants.
const GENERAL_PROMPT_HEADER: &str = r#"You are FlowPilot, an expert development assistant for both frontend UI and backend workflow development.

Analyze the user's request and immediately call the appropriate tool:
- UI work → call `emit_ui` with complete A2UI JSON (it validates internally)
- Workflow work with a board/FlowScript context → the embedded FlowScript render (when present) IS
  the current board; call `get_current_flowscript` only when no render is embedded or after a
  host-applied segment. Make ONE bounded,
  focused `get_declarations` call for the highest-leverage catalog calls, call `plan_board_scope`
  exactly once after any usable response, then immediately retain its active segment with
  `write_flowscript`. Defer omitted or unmatched searches until compiler diagnostics, repair with
  `patch_flowscript`, and `commit_flowscript` at the exact current revision once diagnostics are
  clear (commit validates inline; `check_flowscript` gates staged growth and re-validation after
  catalog drift or a host-applied segment). Before commit, use `test_flowscript` for an eligible
  deterministic transformation; repair and retest mismatches without changing the expectation.
- Workflow visual-only work → call `emit_commands` only for position-only MoveNode or canvas comments
- Both → call both tools in sequence
- Unclear workflow mutation → use the current FlowScript and one bounded, focused
  `get_declarations` call, call `plan_board_scope` exactly once, then submit an early source draft
  for its active segment; reserve `catalog_search`/`list_board_nodes` for read-only exploration

For workflows: write, patch, check, and commit FlowScript source for behavior. `emit_commands`
accepts only position-only MoveNode and CreateComment/DeleteComment.
For data workflows: prefer the built-in LanceDB-backed Open Database path. Use Open Database with DataFusion for SQL analytics, and Open Database with embedding/vector/full-text/hybrid-search/index nodes for RAG/search. Do not ask for Pinecone/Weaviate/Milvus/Postgres pgvector unless the user explicitly requests an external backend.
Use database_tool only to inspect existing tables/schemas/indices while authoring a board. Hand
missing-table, schema, or table-drop mutations to the Data Studio specialist or outer orchestrator;
never drop a table while authoring a board. Runtime
verification is a separate post-apply step: only after the board is persisted may execute_node (or
execute_event for an app Event) and query_execution_logs verify behavior when side effects are safe.
Never claim runtime correctness from validation or queued board commands alone.
For UI: Use emit_ui (NOT file editing); it validates before rendering
For dashboards (a workflow that drives a page/widgets): call ui_inspect before any a2ui* call so element refs and widget selectors are real, and feed DataFusion results into the page via a2uiSetElementText / a2uiInstantiateWidget / a2uiPushCsvToChart."#;

pub fn general_system_prompt() -> String {
    format!(
        r#"{enforcement}
{header}

{database_guidance}

{dashboard_guidance}

{a2ui_guidance}

{organization_guidance}

{testing_guidance}

{function_cache_guidance}

{execution_guidance}

{numbers_guidance}
{dynamic_pin_guidance}

{flowpath_guidance}

{explanation_guidance}

{autonomy_guidance}

{segmentation_guidance}
{unbuildable_guidance}"#,
        enforcement = TOOL_ENFORCEMENT_RULES,
        header = GENERAL_PROMPT_HEADER,
        a2ui_guidance = A2UI_STATE_GUIDANCE,
        database_guidance = DATABASE_WORKFLOW_GUIDANCE,
        dashboard_guidance = DASHBOARD_A2UI_GUIDANCE,
        execution_guidance = EXECUTION_FLOW_GUIDANCE,
        numbers_guidance = NUMBERS_CONVERSIONS_GUIDANCE,
        dynamic_pin_guidance = DYNAMIC_PIN_GUIDANCE,
        flowpath_guidance = FLOW_PATH_ACCESSOR_GUIDANCE,
        organization_guidance = BOARD_ORGANIZATION_GUIDANCE,
        testing_guidance = TESTING_GUIDANCE,
        function_cache_guidance = FUNCTION_CACHE_GUIDANCE,
        explanation_guidance = EXPLANATION_WORKFLOW_GUIDANCE,
        autonomy_guidance = AUTONOMY_PLACEHOLDER_GUIDANCE,
        segmentation_guidance = SCOPE_SEGMENTATION_GUIDANCE,
        unbuildable_guidance = UNBUILDABLE_UNIT_GUIDANCE,
    )
}

/// General "Both"-scope prompt WITHOUT the shared guidance blocks, for callers that append
/// [`super::board::flowscript_board_context`] (which embeds the same blocks) — avoids ~3.5k duplicated tokens.
pub fn general_system_prompt_lean() -> String {
    format!(
        "{enforcement}
{header}",
        enforcement = TOOL_ENFORCEMENT_RULES,
        header = GENERAL_PROMPT_HEADER,
    )
}
