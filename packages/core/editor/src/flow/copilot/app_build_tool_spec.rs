//! App construction has a small eager tool surface. The strict contract is loaded on demand.

use super::tool_spec::{
    MAX_DELEGATED_RUN_DISPATCH_SECS, PlatformToolSpec, ToolApprovalSpec, ToolApprovalTiming,
    spec_arg_str,
};
use serde_json::{Value, json};

fn approval_message(args: &Value) -> String {
    format!(
        "FlowPilot wants to {} build '{}' in app '{}'. Promotion activates only a verified build.",
        spec_arg_str(args, "operation", "operation"),
        spec_arg_str(args, "build_id", "buildId"),
        spec_arg_str(args, "app_id", "appId"),
    )
}

pub(super) fn app_build_tool_spec() -> PlatformToolSpec {
    PlatformToolSpec {
        name: "app_build",
        description: "Opt-in staged app build preview. Read schema for the contract and host capabilities before begin; runtime isolation may be unavailable, blocking promotion. The host links logical keys to reserved IDs. advance runs ready work; status resumes inspection; repair invalidates named resources and dependents without shrinking requirements. validate reads artifacts, test records host observations, promote requires current behavioral evidence. capabilities/recipe load versioned contract scaffolds. Never submit success receipts or bypass activation gates. Live apps are not silently deactivated.",
        schema: || {
            json!({
                "type": "object",
                "additionalProperties": false,
                "properties": {
                    "operation": { "type": "string", "enum": ["schema", "capabilities", "recipe", "begin", "status", "advance", "validate", "test", "promote", "repair"] },
                    "app_id": { "type": "string" },
                    "build_id": { "type": "string", "description": "Stable retry identity within this app. Reuse it after interruption." },
                    "spec": { "type": "object", "description": "Complete AppSpec matching operation=schema. Only for begin." },
                    "resource_keys": { "type": "array", "items": { "type": "string" }, "description": "Exact logical resource keys to repair." },
                    "capability_id": { "type": "string" },
                    "capability_version": { "type": "string" },
                    "parameters": { "type": "object" }
                },
                "required": ["operation"]
            })
        },
        approval: ToolApprovalSpec::Execute {
            title: "Approve app build operation",
            message: approval_message,
            timing: ToolApprovalTiming::BeforeExecution,
        },
        timeout_secs: MAX_DELEGATED_RUN_DISPATCH_SECS,
    }
}

#[cfg(test)]
mod tests {
    use super::super::tool_spec::{ToolEffect, resolve_tool_approval, resolve_tool_effect};
    use super::*;

    #[test]
    fn inspections_are_read_only_and_promotion_requires_approval() {
        let spec = app_build_tool_spec();
        for op in ["schema", "capabilities", "recipe", "status"] {
            assert_eq!(
                resolve_tool_effect(&spec, &json!({"operation":op})),
                ToolEffect::ReadOnly
            );
        }
        for op in ["begin", "advance", "validate", "test", "repair", "promote"] {
            assert_eq!(
                resolve_tool_effect(&spec, &json!({"operation":op})),
                ToolEffect::Execute
            );
            let approval =
                resolve_tool_approval(&spec, &json!({"operation":op,"app_id":"a","build_id":"b"}));
            assert_eq!(approval.kind, "execute");
            assert!(approval.session_key.contains("a:b"));
        }
    }
}
