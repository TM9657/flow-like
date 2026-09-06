//! Shared FlowPilot system prompts
//!
//! Consolidates the system prompts and behavioral rules used by both
//! the rig-based (bits) path and the Copilot SDK path to ensure
//! consistent tool usage and approval workflows.

mod board;
mod data_studio;
mod frontend;
mod general;
mod home;
mod research;
mod scout;
mod shared;

pub use board::{
    A2UI_STATE_GUIDANCE, BOARD_ORGANIZATION_GUIDANCE, BOARD_SPECIALIST_BOUNDARY,
    DASHBOARD_A2UI_GUIDANCE, DATABASE_WORKFLOW_GUIDANCE, DYNAMIC_PIN_GUIDANCE,
    EVENT_ENTRY_GUIDANCE, EXECUTION_FLOW_GUIDANCE, EXPLANATION_WORKFLOW_GUIDANCE,
    FLOW_PATH_ACCESSOR_GUIDANCE, FLOWSCRIPT_DOMAIN_EXAMPLES, FLOWSCRIPT_FEW_SHOT_EXAMPLES,
    FUNCTION_CACHE_GUIDANCE, NUMBERS_CONVERSIONS_GUIDANCE, board_sdk_flowscript_system_prompt,
    board_sdk_system_prompt, board_system_prompt, flowscript_board_context,
};
pub use data_studio::{
    DATA_STUDIO_TARGETING_GUIDANCE, DATA_STUDIO_TOOL_GUIDANCE, DATA_STUDIO_TRANSPARENCY_GUIDANCE,
    DATA_STUDIO_VOCAB_GUIDANCE, data_studio_system_prompt, ontology_query_system_prompt,
};
pub use frontend::{
    UI_DESIGN_GUIDANCE, UI_SPECIALIST_BOUNDARY, frontend_sdk_system_prompt, frontend_system_prompt,
};
pub use general::{general_system_prompt, general_system_prompt_lean};
pub use home::{HOME_SPECIALIST_GUIDANCE, home_system_prompt};
pub use research::{
    RESEARCH_ANSWER_CONTRACT_GUIDANCE, RESEARCH_SCOPE_GUIDANCE, research_system_prompt,
};
pub use scout::{
    SCOUT_PLAN_CONTRACT_GUIDANCE, SCOUT_TOOL_GUIDANCE, SCOUT_VOCAB_GUIDANCE, scout_system_prompt,
};
pub use shared::{
    AUTONOMY_PLACEHOLDER_GUIDANCE, PRIOR_ART_GUIDANCE, SCOPE_SEGMENTATION_GUIDANCE,
    TESTING_GUIDANCE, TOOL_ENFORCEMENT_RULES, UNBUILDABLE_UNIT_GUIDANCE, UNIMPLEMENTED_STUB_MARKER,
    WEB_RESEARCH_GUIDANCE,
};

#[cfg(test)]
mod tests;
