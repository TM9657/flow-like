/// Former model-facing contract for the schema-constrained typed IR path. No live prompt builder
/// embeds it anymore; it is retained only as verified fixtures for the typed IR compiler tests.
#[cfg(test)]
const TYPED_FLOW_IR_GUIDANCE: &str = r#"
## TYPED FLOW IR (PRIMARY FOR NEW OR SUBSTANTIAL WORKFLOWS)
When all six tools below are registered, use them for a new workflow or a substantial greenfield
addition. Their JSON schemas are the authority; do not invent fields that are absent from a schema.

1. Call `plan_flow_ir` first with one focused semantic intent and pin contract per capability, and
   estimate every function/event module's materialized node count and `kind`; the planner derives
   function layers and the shared Event `$root` scope. Every required capability must ultimately
   set `exact_node_type`. When an exact live node is not already known, omit only that field on the
   discovery call. A compatible discovery result deliberately remains `feasible:false` and returns
   `selection_required:true` plus semantically filtered `candidates`; copy one candidate's exact
   `node_type` into that requirement and resubmit the complete plan. Never choose a candidate whose
   protocol/service, operation, or algorithm/type differs from the intent. If no compatible
   candidate remains, report that exact missing capability; never silently substitute it.
2. Call `begin_flow_ir_draft` once with a stable `draft_id`, the complete variable/interface header,
   the same required `capability_plan` request, and every required module name in
   `expected_modules`. Neither list may be omitted or empty. Leave `mode` as `additive` so unrelated
   existing board content is preserved. Use `replace` only for an explicit full-board replacement.
3. Repair retained variables/interfaces or remove a mistakenly authored module with
   `update_flow_ir_draft`; this preserves valid modules and increments the revision. Add or repair
   one complete function/event at a time with `upsert_flow_ir_module`, always passing
   the latest returned revision. If the user explicitly reduces requested scope, replace
   `expected_modules` and `capability_plan` together in that same update; every expected module
   still needs exactly one same-name, same-kind module estimate. Reference data only by an exact
   `{ step, pin, occurrence }` output.
   For agent/function-tool registration, use a synthetic `tools`/`fnRefs` argument whose complete
   value is `{ "kind":"function_refs", "functions":["retainedModule"] }`; never encode tool
   targets as a normal list/ref data value.
   For a node with multiple execution outputs, use `exec_arms` for explicit success/error/outcome
   bodies and set `continue_from` to the one exact outcome allowed to reach later sibling steps.
   Never set `allow_scope_reduction` unless the user explicitly asked to remove behavior.
4. Call `validate_flow_ir_draft`. Repair structured root diagnostics at the JSON-pointer `path` in
   the same retained draft. Do not delete requested modules or replace a rich draft with a smoke
   test; worsening replacements are rejected automatically. If provider context was truncated,
   request only the needed retained state with `include_header: true` and/or `modules: ["name"]`.
5. Call `commit_flow_ir_draft` with the exact current revision. This is the only typed operation that
   can queue board commands, and it is atomic and replay-safe. A replace commit must enumerate the
   exact `remove_node_ids`, `remove_variable_ids`, `remove_layer_ids`, and `remove_comment_ids`;
   `allow_deletions` alone authorizes nothing.
   Stop workflow tools after status
   `queued` or idempotent status `already_queued`.

Use `edit_flowscript` instead for a focused edit to an existing anchored board, or as fallback when
the typed tools are unavailable. Do not mix a typed draft and raw FlowScript mutation for the same
change. FlowScript returned by typed validation is an inspection artifact; repair the typed JSON,
not that generated text. Never mix typed IR, raw FlowScript, and direct commands in one mutation.
Use `emit_commands` only for position-only MoveNode and canvas comments. It never accepts
executable behavior, variables, placeholders, pins, connections, function metadata, layer
membership changes, or layer creation/removal.

### Compact typed tool-call example (revision progression)
If the exact log node is not yet known, first make this semantic discovery call:
```json
{"requirements":[{"id":"log","intent":"log an informational message","required":true,"inputs":[{"names":["message"],"data_type":"generic"}],"outputs":[]}],"modules":[{"name":"runTask","kind":"function","estimated_nodes":1},{"name":"eventsSimple","kind":"event","estimated_nodes":1}]}
```
Its resolution is intentionally not feasible and includes an excerpt like
`{"selection_required":true,"candidates":[{"node_type":"log_info"}]}`. Select only from that
filtered list, retain every requirement/module, and resubmit:
```json
{"requirements":[{"id":"log","intent":"log a message","required":true,"exact_node_type":"log_info","inputs":[{"names":["message"],"data_type":"generic"}],"outputs":[]}],"modules":[{"name":"runTask","kind":"function","estimated_nodes":1},{"name":"eventsSimple","kind":"event","estimated_nodes":1}]}
```
`begin_flow_ir_draft` starts revision 0 with `expected_modules:["runTask","eventsSimple"]`,
`mode:"additive"`, the same capability request, and an empty/default program. Then:
```flow-ir-verified
{"draft_id":"demo","expected_revision":0,"allow_scope_reduction":false,"module":{"kind":"function","name":"runTask","params":[],"returns":[],"steps":[{"kind":"node","id":"log","node_type":"log_info","args":[{"pin":"message","occurrence":0,"value":{"kind":"literal","value":{"type":"string","value":"hello"}}}],"exec_arms":[]}]}}
```
The successful upsert returns revision 1; upsert the Event with revision 1, validate revision 2,
then commit revision 2.

Canonical JSON output spellings (emit these consistently; do not invent fields):
- Every authored type is an object such as
  `{"data_type":"string","container":"normal"}` or
  `{"data_type":"struct","container":"array","interface":"Ticket"}`. The scalar names are
  `string`, `integer`, `float`, `boolean`, `struct`, `geometry`, `generic`, `date`, `path`, and `bytes`. The
  parser accepts legacy bare scalar strings and `int`/`bool` aliases as input, but canonical model
  output always uses the type object and full scalar name. A parameter is
  `{"name":"ticket","type":{"data_type":"struct","container":"normal","interface":"Ticket"}}`.
  Geometry accepts GeoJSON geometry objects in WGS 84 longitude/latitude order. Use
  `{"data_type":"geometry","geometry_kind":"Point"}` for a concrete subtype; omit `geometry_kind`
  for any geometry. Supported kinds are Point, LineString, Polygon, MultiPoint, MultiLineString,
  MultiPolygon, and GeometryCollection. A concrete subtype can feed any geometry, while the
  reverse requires a validating geometry cast. FlowScript spells these `geometry` and
  `geometry<Point>`. Preserve subtype annotations on variables and function boundaries.
- Parameter/variable/loop references are canonically `{"kind":"ref","name":"ticket"}` and
  function calls use `"kind":"call_function"`. The parser accepts the legacy `param` and `call`
  aliases, but repair output should normalize them. Conditions canonically use
  `{"kind":"if","id":"...","condition":...,"then_steps":[],"else_steps":[]}`; `then`/`else`
  are accepted input aliases only. Object fields are `{"key":"status","value":<FlowIrValue>}`.
- A literal is `{"kind":"literal","value":{"type":"boolean","value":true}}`; node outputs are
  `{"kind":"output","step":"fetch","pin":"message","occurrence":0}`. Only use variants and
  fields present in the advertised tool schema.
- During incremental construction, add each expected module even while other capabilities remain
  outstanding. `missing_modules`/remaining-capability summaries describe unfinished whole-draft
  work; repair JSON-pointer diagnostics that point into the module you just authored, then move to
  the next missing module. Whole-request capability completeness is enforced by validate/commit.

### Multi-outcome + selected-arm value example
The tail may reference data produced inside the one `continue_from` arm because it executes there:
```flow-ir-verified
{"draft_id":"http-demo","expected_revision":0,"allow_scope_reduction":false,"module":{"kind":"event","name":"eventsSimple","node_type":"events_simple","params":[],"steps":[{"kind":"node","id":"fetch","node_type":"http_fetch","args":[{"pin":"request","occurrence":0,"value":{"kind":"literal","value":{"type":"json","value":{"method":"GET","url":"https://example.com"}}}}],"continue_from":"exec_success","exec_arms":[{"pin":"exec_success","steps":[{"kind":"node","id":"successMessage","node_type":"string_format","args":[{"pin":"format_string","occurrence":0,"value":{"kind":"literal","value":{"type":"string","value":"request succeeded"}}}],"exec_arms":[]}]},{"pin":"exec_error","steps":[{"kind":"node","id":"errorLog","node_type":"log_error","args":[{"pin":"message","occurrence":0,"value":{"kind":"literal","value":{"type":"string","value":"request failed"}}}],"exec_arms":[]}]}]},{"kind":"node","id":"successLog","node_type":"log_info","args":[{"pin":"message","occurrence":0,"value":{"kind":"output","step":"successMessage","pin":"formatted_string","occurrence":0}}],"exec_arms":[]}]}}
```
"#;

use super::*;
use crate::flow::ast::reconcile_text_with_catalog;
use crate::flow::board::{Board, ExecutionMode, ExecutionStage};
use crate::flow::copilot::{
    FlowIrProgram, NodeMetadata, PinMetadata, UpsertFlowIrModuleArgs, compile_flow_ir,
};
use crate::flow::execution::LogLevel;
use flow_like_ast::{Container, SigParam, Signature, SignatureSet, parse};
use flow_like_storage::Path;
use std::collections::HashMap;
use std::time::SystemTime;

fn verified_microexamples() -> Vec<&'static str> {
    FLOWSCRIPT_FEW_SHOT_EXAMPLES
        .split("```flowscript-verified\n")
        .skip(1)
        .map(|rest| {
            rest.split_once("\n```")
                .expect("verified FlowScript fence must be closed")
                .0
        })
        .collect()
}

fn verified_typed_upserts() -> Vec<UpsertFlowIrModuleArgs> {
    TYPED_FLOW_IR_GUIDANCE
        .split("```flow-ir-verified\n")
        .skip(1)
        .map(|rest| {
            let json = rest
                .split_once("\n```")
                .expect("verified typed IR fence must be closed")
                .0;
            serde_json::from_str(json).expect("verified typed tool call must match its schema")
        })
        .collect()
}

fn empty_board() -> Board {
    {
        let mut board = Board::new_detached(None, flow_like_storage::Path::default());
        board.id = "verified-prompt-examples".to_string();
        board.name = "Verified Prompt Examples".to_string();
        board.description = String::new();
        board.nodes = HashMap::new();
        board.variables = HashMap::new();
        board.comments = HashMap::new();
        board.viewport = (0.0, 0.0, 1.0);
        board.version = (0, 0, 1);
        board.stage = ExecutionStage::Dev;
        board.log_level = LogLevel::Info;
        board.execution_mode = ExecutionMode::Hybrid;
        board.refs = HashMap::new();
        board.layers = HashMap::new();
        board.page_ids = Vec::new();
        board.hash = None;
        board.created_at = SystemTime::now();
        board.updated_at = SystemTime::now();
        board.parent = None;
        board.board_dir = Path::from("/test");
        board.logic_nodes = HashMap::new();
        board.app_state = None;
        board
    }
}

fn metadata_pin(param: &SigParam) -> PinMetadata {
    let data_type = match param.ty.base.as_str() {
        "any" => "Generic",
        "bool" => "Boolean",
        "bytes" => "Byte",
        "geometry" => "Geometry",
        "float" => "Float",
        "int" => "Integer",
        "string" => "String",
        other => other,
    };
    let value_type = match param.ty.container {
        Container::Normal => "Normal",
        Container::Array => "Array",
        Container::Map => "HashMap",
        Container::Set => "HashSet",
    };
    PinMetadata {
        name: param.name.clone(),
        friendly_name: param.name.clone(),
        description: param.doc.clone().unwrap_or_default(),
        data_type: data_type.to_string(),
        value_type: value_type.to_string(),
        default_value: param.optional.then(|| "null".to_string()),
        schema: param.schema.clone(),
        is_generic: param.ty.base == "any",
        valid_values: None,
        enforce_schema: false,
    }
}

fn execution_pin(name: &str) -> PinMetadata {
    PinMetadata {
        name: name.to_string(),
        friendly_name: name.to_string(),
        description: String::new(),
        data_type: "Execution".to_string(),
        value_type: "Normal".to_string(),
        default_value: None,
        schema: None,
        is_generic: false,
        valid_values: None,
        enforce_schema: false,
    }
}

/// `signatures.json` intentionally omits execution pins. Recreate the concrete execution
/// shapes exercised by the verified examples while deriving every data pin from the generated
/// catalog registry. A registry rename/type change therefore breaks this test instead of
/// leaving stale prompt code behind.
fn metadata_from_signature(signature: &Signature) -> NodeMetadata {
    let mut inputs = signature
        .inputs
        .iter()
        .map(metadata_pin)
        .collect::<Vec<_>>();
    let mut outputs = signature
        .outputs
        .iter()
        .map(metadata_pin)
        .collect::<Vec<_>>();

    if signature.impure {
        match signature.node_type.as_str() {
            node_type if node_type.starts_with("events_") => {
                outputs.insert(0, execution_pin("exec_out"));
            }
            "control_branch" => {
                inputs.insert(0, execution_pin("exec_in"));
                outputs.insert(0, execution_pin("false"));
                outputs.insert(0, execution_pin("true"));
            }
            "control_for_each" => {
                inputs.insert(0, execution_pin("exec_in"));
                outputs.insert(0, execution_pin("done"));
                outputs.insert(0, execution_pin("exec_out"));
            }
            "http_fetch" => {
                inputs.insert(0, execution_pin("exec_in"));
                outputs.insert(0, execution_pin("exec_error"));
                outputs.insert(0, execution_pin("exec_success"));
            }
            _ => {
                inputs.insert(0, execution_pin("exec_in"));
                outputs.insert(0, execution_pin("exec_out"));
            }
        }
    }

    NodeMetadata {
        name: signature.node_type.clone(),
        friendly_name: signature
            .friendly
            .clone()
            .unwrap_or_else(|| signature.display.clone()),
        description: signature.doc.clone().unwrap_or_default(),
        inputs,
        outputs,
        category: signature.category.clone(),
        required_inputs: signature
            .inputs
            .iter()
            .filter(|param| !param.optional)
            .map(|param| param.name.clone())
            .collect(),
        companion_nodes: Vec::new(),
        capability_tags: Vec::new(),
        namespace: signature.namespace.clone(),
        alias: signature.alias.clone(),
        receiver: signature.receiver.clone(),
    }
}

fn generated_catalog_metadata() -> Vec<NodeMetadata> {
    let signatures: SignatureSet =
        serde_json::from_str(include_str!("../../../../../../ast/signatures.json"))
            .expect("generated FlowScript signature registry must deserialize");
    signatures
        .signatures
        .iter()
        .map(metadata_from_signature)
        .collect()
}

#[test]
fn verified_flowscript_microexamples_parse() {
    let examples = verified_microexamples();
    assert_eq!(
        examples.len(),
        5,
        "keep the verified suite intentionally small"
    );
    for (index, example) in examples.iter().enumerate() {
        parse(example).unwrap_or_else(|error| {
            panic!("verified FlowScript example {index} failed to parse: {error}\n{example}")
        });
    }
}

#[test]
fn verified_flowscript_microexamples_reconcile_against_generated_catalog() {
    let catalog = generated_catalog_metadata();
    for (index, example) in verified_microexamples().iter().enumerate() {
        let result = reconcile_text_with_catalog(&empty_board(), example, &catalog);
        assert!(
            result.diagnostics.is_empty(),
            "verified FlowScript example {index} did not reconcile: {:?}\n{example}",
            result.diagnostics
        );
        assert!(
            !result.commands.is_empty(),
            "verified FlowScript example {index} produced no materialization commands"
        );
    }
}

#[test]
fn verified_typed_tool_calls_compile_against_generated_catalog() {
    let catalog = generated_catalog_metadata();
    let examples = verified_typed_upserts();
    assert_eq!(examples.len(), 2, "keep the typed few-shot suite compact");
    for (index, example) in examples.into_iter().enumerate() {
        let program = FlowIrProgram {
            modules: vec![example.module],
            ..Default::default()
        };
        let compiled = compile_flow_ir(&program, &catalog);
        assert!(
            compiled.diagnostics.is_empty(),
            "verified typed example {index} failed to compile: {:?}\n{}",
            compiled.diagnostics,
            compiled.flowscript
        );
    }
}

#[test]
fn flowscript_examples_use_real_helper_declaration_syntax() {
    for helper in [
        "either",
        "generateReport",
        "ingestRows",
        "search",
        "loadConfig",
        "processAllSources",
        "loadOverview",
        "runResearch",
        "briefingPageLoad",
        "fillArticles",
        "renderTrend",
    ] {
        assert!(
            FLOWSCRIPT_FEW_SHOT_EXAMPLES.contains(&format!("function {helper}(")),
            "few-shot helper {helper} must include the function keyword"
        );
        assert!(
            !FLOWSCRIPT_FEW_SHOT_EXAMPLES.contains(&format!("\n{helper}(")),
            "few-shot helper {helper} must not look like an Event/interface declaration"
        );
    }
    // The inverse contract: a `tools:`/`fnRefs:` target must be a HANDLER block, never a
    // `function`. A `function` compiles to a Function layer with no entry node, so apply
    // rejects the reference outright ("has no referenceable event/handler entry") and rolls
    // the whole edit back — see `check_function_ref_targets`. These examples previously taught
    // the broken shape.
    for tool_target in ["echoTool", "fetchPage"] {
        assert!(
            !FLOWSCRIPT_FEW_SHOT_EXAMPLES.contains(&format!("function {tool_target}(")),
            "agent/widget tool target {tool_target} must NOT be declared as a `function` — \
                 a Function layer cannot be referenced as a tool"
        );
        assert!(
            FLOWSCRIPT_FEW_SHOT_EXAMPLES.contains(&format!("{tool_target}(")),
            "agent/widget tool target {tool_target} must still be declared as a handler block"
        );
    }
    // The UI half of the same contract: a component inside a widget must fire the fixed
    // `widget_event` verb carrying the action id, not an action named after the id. Only
    // `widget_event` reaches the widget's action bindings in ActionHandler.tsx.
    let frontend = frontend_system_prompt("{}", "");
    assert!(
        frontend.contains(
            r#""actions": [{"name": "widget_event", "context": {"actionId": "approve"}}]"#
        ),
        "the widget prompt must document the widget_event contract"
    );
    assert!(
        !frontend.contains(r#""actions": [{"name": "approve"}]"#),
        "the widget prompt must not teach an action named after the widget action id"
    );

    // A widget action target is stricter still: `a2ui_instantiate_widget` validates that every
    // `fnRefs` entry is an `events_widget_action` node and errors otherwise, so a plain handler
    // block (which lowers to `events_generic`) is NOT sufficient here.
    assert!(
        FLOWSCRIPT_FEW_SHOT_EXAMPLES.contains("eventsWidgetAction openBriefing("),
        "a widget `fnRefs` target must be declared as an `eventsWidgetAction` event"
    );
    assert!(
        !FLOWSCRIPT_FEW_SHOT_EXAMPLES.contains("function openBriefing("),
        "a widget `fnRefs` target must not be declared as a `function`"
    );
    assert!(
        !FLOWSCRIPT_FEW_SHOT_EXAMPLES
            .contains("makeHistoryMessage({ role: \"User\", type: \"Text\", text:")
    );
    assert!(
        FLOWSCRIPT_FEW_SHOT_EXAMPLES
            .contains("history::fromString({ modelName: \"\", message: task })")
    );
    assert!(
        !FLOWSCRIPT_FEW_SHOT_EXAMPLES.contains("db::open({ name: \"email_vectors\" }).database")
    );
}
