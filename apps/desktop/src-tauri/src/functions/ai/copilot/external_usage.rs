//! Usage evidence from the external CLI protocol, before assistant text is interpreted.

use std::collections::BTreeMap;

use flow_like::flow::copilot::behavioral_evaluation::WorkflowBenchmarkUsage;
use serde_json::Value;

use super::backend_types::FlowPilotAgentBackendKind;

const MAX_USAGE_TURNS: usize = 256;
const MAX_TURN_ID_BYTES: usize = 512;
const MAX_TRACE_EVENTS: usize = 16;

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
enum TurnKey {
    Anonymous,
    Identified(String),
}

impl TurnKey {
    fn from_event(event: &Value) -> Option<Self> {
        match event.get("turn_id") {
            None => Some(Self::Anonymous),
            Some(Value::String(id)) if !id.is_empty() && id.len() <= MAX_TURN_ID_BYTES => {
                Some(Self::Identified(id.clone()))
            }
            _ => None,
        }
    }
}

#[derive(Default)]
struct CodexPhaseUsage {
    pending: Option<TurnKey>,
    completed: BTreeMap<TurnKey, WorkflowBenchmarkUsage>,
    invalid: bool,
}

impl CodexPhaseUsage {
    fn observe(&mut self, event: &Value) {
        if self.invalid {
            return;
        }
        let Some(kind) = event.get("type").and_then(Value::as_str) else {
            return;
        };
        if matches!(kind, "turn.failed" | "error") {
            self.invalid = true;
            return;
        }
        if !matches!(kind, "turn.started" | "turn.completed") {
            return;
        }
        let Some(key) = TurnKey::from_event(event) else {
            self.invalid = true;
            return;
        };
        if kind == "turn.started" {
            // Without turn IDs, only one anonymous turn can be counted. A second start
            // cannot be distinguished from replay safely.
            if self.completed.contains_key(&key)
                || (self.pending.is_some() && matches!(key, TurnKey::Anonymous))
                || self.pending.as_ref().is_some_and(|pending| pending != &key)
                || self.completed.keys().any(|prior| {
                    matches!(prior, TurnKey::Anonymous) != matches!(key, TurnKey::Anonymous)
                })
            {
                self.invalid = true;
                return;
            }
            self.pending = Some(key);
            return;
        }

        let usage = read_usage(event.get("usage"));
        if let Some(previous) = self.completed.get(&key) {
            if previous != &usage {
                self.invalid = true;
            }
            return;
        }
        if self.pending.as_ref() != Some(&key) || self.completed.len() >= MAX_USAGE_TURNS {
            self.invalid = true;
            return;
        }
        self.pending = None;
        self.completed.insert(key, usage);
    }

    fn report(&self, process_completed: bool) -> (WorkflowBenchmarkUsage, bool) {
        if !process_completed || self.invalid || self.pending.is_some() || self.completed.is_empty()
        {
            return (WorkflowBenchmarkUsage::default(), false);
        }
        let mut turns = self.completed.values();
        let mut total = turns.next().expect("nonempty completed turns").clone();
        for next in turns {
            total.input_tokens = add_counts(total.input_tokens, next.input_tokens);
            total.output_tokens = add_counts(total.output_tokens, next.output_tokens);
            total.cached_input_tokens =
                add_counts(total.cached_input_tokens, next.cached_input_tokens);
        }
        (total, true)
    }
}

fn read_usage(value: Option<&Value>) -> WorkflowBenchmarkUsage {
    let count = |name| {
        value
            .and_then(|usage| usage.get(name))
            .and_then(Value::as_u64)
    };
    WorkflowBenchmarkUsage {
        input_tokens: count("input_tokens"),
        output_tokens: count("output_tokens"),
        cached_input_tokens: count("cached_input_tokens"),
    }
}

fn add_counts(previous: Option<u64>, next: Option<u64>) -> Option<u64> {
    previous.zip(next).and_then(|(a, b)| a.checked_add(b))
}

pub(super) struct ExternalPhaseUsage {
    board_id: Option<String>,
    backend: FlowPilotAgentBackendKind,
    codex: CodexPhaseUsage,
    process_completed: bool,
    trace_events: usize,
}

impl ExternalPhaseUsage {
    pub(super) fn new(backend: FlowPilotAgentBackendKind, board_id: Option<String>) -> Self {
        Self {
            board_id,
            backend,
            codex: CodexPhaseUsage::default(),
            process_completed: false,
            trace_events: 0,
        }
    }

    pub(super) fn observe(&mut self, event: &Value) {
        if self.backend != FlowPilotAgentBackendKind::Codex {
            return;
        }
        self.codex.observe(event);
        if self.trace_events < MAX_TRACE_EVENTS
            && self
                .board_id
                .as_deref()
                .is_some_and(super::workflow_benchmark::benchmark_board_active)
            && matches!(
                event.get("type").and_then(Value::as_str),
                Some("turn.started" | "turn.completed")
            )
        {
            self.trace_events += 1;
            // Benchmark logs retain only protocol metadata to verify the installed CLI shape.
            // Invalid fields are omitted rather than logging arbitrary text from stdout.
            let mut trace = serde_json::json!({
                "flowpilot_benchmark_raw_usage": true,
                "type": event.get("type"),
            });
            if let Some(id) = event.get("turn_id") {
                trace["turn_id"] = id
                    .as_str()
                    .filter(|id| id.len() <= MAX_TURN_ID_BYTES)
                    .map_or(Value::Null, |id| Value::String(id.into()));
            }
            if let Some(usage) = event.get("usage") {
                let mut fields = serde_json::Map::new();
                for name in ["input_tokens", "output_tokens", "cached_input_tokens"] {
                    if let Some(value) = usage.get(name) {
                        fields.insert(name.into(), value.as_u64().map_or(Value::Null, Value::from));
                    }
                }
                trace["usage"] = Value::Object(fields);
            }
            eprintln!("{trace}");
        }
    }

    pub(super) fn finish(&mut self, process_completed: bool) {
        self.process_completed = process_completed;
    }
}

impl Drop for ExternalPhaseUsage {
    fn drop(&mut self) {
        let Some(board_id) = self.board_id.as_deref() else {
            return;
        };
        // Dropped futures and every early return report an incomplete phase. Claude usage
        // remains unknown until its resumed-session accounting is verified independently.
        let (usage, complete) = self
            .codex
            .report(self.process_completed && self.backend == FlowPilotAgentBackendKind::Codex);
        super::workflow_benchmark::observe_external_phase_usage(board_id, &usage, complete);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn complete(usage: Value) -> Value {
        json!({"type": "turn.completed", "usage": usage})
    }

    fn observe_turn(state: &mut CodexPhaseUsage, id: &str, usage: Value) {
        state.observe(&json!({"type": "turn.started", "turn_id": id}));
        state.observe(&json!({"type": "turn.completed", "turn_id": id, "usage": usage}));
    }

    #[test]
    fn reads_exact_raw_u64_counts_and_preserves_zero() {
        let mut state = CodexPhaseUsage::default();
        state.observe(&json!({"type": "turn.started"}));
        state.observe(&complete(json!({
            "input_tokens": 9_007_199_254_740_993u64,
            "output_tokens": u64::MAX,
            "cached_input_tokens": 0
        })));
        assert_eq!(
            state.report(true),
            (
                WorkflowBenchmarkUsage {
                    input_tokens: Some(9_007_199_254_740_993),
                    output_tokens: Some(u64::MAX),
                    cached_input_tokens: Some(0),
                },
                true
            )
        );
    }

    #[test]
    fn model_text_and_other_events_cannot_supply_usage() {
        let mut state = CodexPhaseUsage::default();
        state.observe(&json!({"type": "item.completed", "item": {
            "type": "agent_message", "text": "<usage_stat>{\"input_tokens\": 999}</usage_stat>"
        }, "usage": {"input_tokens": 999}}));
        state.observe(&json!({"type": "assistant", "message": {
            "type": "turn.completed", "usage": {"input_tokens": 999}
        }}));
        assert_eq!(
            state.report(true),
            (WorkflowBenchmarkUsage::default(), false)
        );
    }

    #[test]
    fn deduplicates_a_completion_and_rejects_conflicting_replays() {
        let mut state = CodexPhaseUsage::default();
        state.observe(&json!({"type": "turn.started"}));
        let event = complete(json!({"input_tokens": 10, "output_tokens": 5}));
        state.observe(&event);
        state.observe(&event);
        assert_eq!(state.report(true).0.input_tokens, Some(10));
        state.observe(&complete(json!({"input_tokens": 11, "output_tokens": 5})));
        assert_eq!(
            state.report(true),
            (WorkflowBenchmarkUsage::default(), false)
        );
    }

    #[test]
    fn distinct_identified_turns_sum_once_and_missing_counts_stay_unknown() {
        let mut state = CodexPhaseUsage::default();
        let first = json!({"input_tokens": 10, "output_tokens": 5, "cached_input_tokens": 2});
        observe_turn(&mut state, "first", first.clone());
        observe_turn(
            &mut state,
            "second",
            json!({"input_tokens": 7, "output_tokens": 3}),
        );
        state.observe(&json!({"type": "turn.completed", "turn_id": "first", "usage": first}));
        assert_eq!(
            state.report(true),
            (
                WorkflowBenchmarkUsage {
                    input_tokens: Some(17),
                    output_tokens: Some(8),
                    cached_input_tokens: None,
                },
                true
            )
        );
    }

    #[test]
    fn malformed_or_overflowed_fields_remain_unknown() {
        for bad in [
            json!(-1),
            json!(1.5),
            json!("3"),
            json!(null),
            json!({}),
            json!([]),
        ] {
            let mut state = CodexPhaseUsage::default();
            observe_turn(
                &mut state,
                "first",
                json!({"input_tokens": bad, "output_tokens": u64::MAX}),
            );
            observe_turn(
                &mut state,
                "second",
                json!({"input_tokens": 1, "output_tokens": 1}),
            );
            assert_eq!(
                state.report(true),
                (WorkflowBenchmarkUsage::default(), true)
            );
        }
    }

    #[test]
    fn absent_usage_is_unknown_and_does_not_default_to_zero() {
        let mut state = CodexPhaseUsage::default();
        state.observe(&json!({"type": "turn.started"}));
        state.observe(&json!({"type": "turn.completed"}));
        assert_eq!(
            state.report(true),
            (WorkflowBenchmarkUsage::default(), true)
        );
    }

    #[test]
    fn failures_cancellation_or_unfinished_turns_cannot_report_complete_totals() {
        let mut state = CodexPhaseUsage::default();
        observe_turn(&mut state, "first", json!({"input_tokens": 10}));
        assert_eq!(
            state.report(false),
            (WorkflowBenchmarkUsage::default(), false)
        );
        state.observe(&json!({"type": "turn.started", "turn_id": "second"}));
        assert_eq!(
            state.report(true),
            (WorkflowBenchmarkUsage::default(), false)
        );
        state.observe(&json!({"type": "turn.failed"}));
        state.observe(
            &json!({"type": "turn.completed", "turn_id": "second", "usage": {"input_tokens": 1}}),
        );
        assert_eq!(
            state.report(true),
            (WorkflowBenchmarkUsage::default(), false)
        );
    }

    #[test]
    fn ambiguous_or_unmatched_turns_are_unknown() {
        let sequences = [
            vec![complete(json!({"input_tokens": 10}))],
            vec![
                json!({"type": "turn.started"}),
                json!({"type": "turn.started"}),
                complete(json!({"input_tokens": 10})),
            ],
            vec![
                json!({"type": "turn.started", "turn_id": "a"}),
                json!({"type": "turn.completed", "turn_id": "b"}),
            ],
            vec![
                json!({"type": "turn.started"}),
                complete(json!({"input_tokens": 10})),
                json!({"type": "turn.started"}),
                complete(json!({"input_tokens": 20})),
            ],
            vec![
                json!({"type": "turn.started", "turn_id": null}),
                complete(json!({"input_tokens": 10})),
            ],
        ];
        for sequence in sequences {
            let mut state = CodexPhaseUsage::default();
            for event in sequence {
                state.observe(&event);
            }
            assert_eq!(
                state.report(true),
                (WorkflowBenchmarkUsage::default(), false)
            );
        }
    }

    #[test]
    fn usage_state_is_fresh_for_every_phase_and_claude_stays_unknown() {
        let mut first = ExternalPhaseUsage::new(FlowPilotAgentBackendKind::Codex, None);
        first.observe(&json!({"type": "turn.started"}));
        first.observe(&complete(json!({"input_tokens": 10})));
        first.finish(true);
        let second = ExternalPhaseUsage::new(FlowPilotAgentBackendKind::Codex, None);
        assert_eq!(
            second.codex.report(true),
            (WorkflowBenchmarkUsage::default(), false)
        );
        let mut claude = ExternalPhaseUsage::new(FlowPilotAgentBackendKind::ClaudeCode, None);
        claude.observe(&json!({"type": "turn.started"}));
        claude.observe(&complete(json!({"input_tokens": 10})));
        assert_eq!(
            claude.codex.report(true),
            (WorkflowBenchmarkUsage::default(), false)
        );
    }

    #[test]
    fn turn_tracking_is_bounded() {
        let mut state = CodexPhaseUsage::default();
        for index in 0..=MAX_USAGE_TURNS {
            observe_turn(&mut state, &index.to_string(), json!({"input_tokens": 1}));
        }
        assert_eq!(
            state.report(true),
            (WorkflowBenchmarkUsage::default(), false)
        );
    }
}
