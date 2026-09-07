---
title: Develop and evaluate FlowPilot
description: Run app-generation benchmarks and inspect the staged app-build preview
---

This guide covers the desktop evaluation harness and the opt-in staged-build
preview. For everyday authoring, see [FlowPilot](/studio/flowpilot/).

## Run app-generation evaluations

Use the repository-root CLI to evaluate app generation through the real
development desktop runtime. Live runs spend model budget and keep generated
apps for inspection. Install the [desktop prerequisites](/dev/build/), configure
the Codex provider in the app, and close any running desktop instance before
starting the harness:

```sh
bun run flowpilot:e2e -- --case simple-agent
bun run flowpilot:e2e -- --suite smoke --min-chars 1200 --json
bun run flowpilot:e2e -- --case forum --case ops-dashboard --repeat 3 --fail-fast
bun run flowpilot:e2e -- --case ai-adventure --model sol
bun run flowpilot:e2e -- --case simple-agent --tier behavioral
```

`--model` pins the generation model for the parent turn and every nested specialist. It accepts a
benchmark alias or the model id itself: `terra` / `gpt-5.6-terra` (default) and `sol` /
`gpt-5.6-sol`. The harness assigns each result a cohort key derived from its declared harness
contract version, exact provider/model/reasoning tuple, validation tier, and case-suite fingerprint.
The comparison guard rejects scores from different cohorts. Changing a model or fixture therefore
requires a new baseline; the cohort key does not claim that thresholds were recalibrated.

The default `structural` tier checks compiler receipts, persisted readback, and app structure.
`--tier behavioral` additionally requires host-produced scenario results from an attested isolated
runtime, complete assertions, and successful outcomes for every started run. The current desktop
host does not advertise runtime isolation, so this opt-in tier fails explicitly until a real
isolated adapter supplies that evidence. It never treats a transport acknowledgement as workflow
success.

Useful inspection modes do not start Tauri or spend model budget:

```sh
bun run flowpilot:e2e -- --list
bun run flowpilot:e2e -- --case simple-agent --dry-run
```

By default Tauri starts the normal desktop Next dev server at `http://localhost:3000`. For a faster
edit/run loop, keep that Next server running and pass `--frontend-url http://localhost:3000` so the
CLI reuses it. Close any running Flow Like desktop app first: the native single-instance guard
rejects a second desktop process rather than risking concurrent writes to shared local app data,
and a CLI lock rejects parallel benchmark commands. `--keep-desktop` is available for debugging,
but that retained app must be closed before the next CLI run.

Child-process logs go to stderr, so `--json` reserves stdout for exactly one machine-readable result,
including on infrastructure errors. Use `--output /tmp/flowpilot-e2e.json` for a stable artifact
path. Exit code `0` means every requested run passed, `1` means at least one completed benchmark
failed, and `2` means CLI/startup/transport failure. The controller independently verifies the
selection and artifact order and recomputes the final summary instead of trusting the webview's
pass bit. It also recomputes behavioral run totals across every case and repeat and checks each
report against the requested evaluation cohort.

The benchmark uses the shared desktop global-chat and `GlobalToolBridge` path,
with the selected Codex model at `high` reasoning. A nonce-protected loopback
callback returns the webview result to the controller.

The live benchmark is available in a **development** desktop build at
`/developer/flowpilot-e2e`. Production builds fail the runner preflight before
spending model budget because detailed compiler evidence is disabled there.

Quick entry points:

- One case: `/developer/flowpilot-e2e?case=simple-agent&run=1`
- Default three-case smoke suite: `/developer/flowpilot-e2e?suite=smoke&run=1`
- Every case: `/developer/flowpilot-e2e?suite=full&run=1`
- Benchmark the other model: append `&model=sol` (the header toggle does the same for manual runs)
- Override the per-case non-whitespace character floor: append `&minChars=1200`

The heaviest case is `ai-adventure`: an offline AI text-adventure with a three-screen custom game
UI, two repeated widgets, six exactly named tables including a vector-embedded per-adventure memory
store, a savepoint/restore system, and a story agent that plans the campaign up front and then
generates scenes toward the persisted global goal. It exercises far more of the surface than the
single-purpose cases, so expect it to be the slowest and the strictest.

The same runner is callable from the desktop webview console:

```js
await window.flowPilotE2E.run({
  caseId: "forum",
  modelKey: "sol",
  minFlowScriptNonWhitespaceChars: 900,
});
```

Each returned artifact includes the resolved prompt; byte-for-byte authored
FlowScript candidates; exact `check_flowscript` and commit receipts per app/board; canonical board
readbacks; parser and authoritative reconciliation results; app/UI/data/event inventory; persisted
node-capability and generated-ID checks; lower/upper compactness bounds; partial collector failures;
a stable failure fingerprint for grouping regressions; and the assistant debug trace.
The focused Vitest suite tests case construction and artifact evaluation without
invoking a model.

## Staged app-build preview

FlowPilot can keep an app contract and its build progress outside a model conversation. This is an opt-in preview for developers testing app construction. The interactive host cannot yet isolate native workflow side effects, so it cannot certify runtime behavior or promote a build. Ordinary app requests continue through the existing build playbook.

### System model

An `AppSpec` describes requirements, resources, dependencies, and acceptance scenarios. The compiler validates references and assigns stable physical identifiers within an app/build pair. Specialists receive the part they own and a shared symbol table. They generate artifacts; the host reads those artifacts back before recording an applied receipt.

The desired contract fingerprint and the observed artifact fingerprint have different jobs. The former identifies requested intent. The latter detects changes in persisted content. Resource existence, a model's completion message, and a successful tool response cannot substitute for readback evidence.

The [app-build modules](https://github.com/Rheosoph/flow-like/tree/dev/packages/ui/lib/app-build)
contain the contract compiler, resumable engine, resource readback, staging
lifecycle, and scenario runner.

The Rust store in `packages/core/runtime/src/app_build.rs` persists app-scoped JSON checkpoints. Web/API and desktop/Tauri adapters use the same revision contract: create revision zero, then compare-and-swap from revision `n` to `n + 1`. Cloud stores use conditional writes and fail explicitly when those are unsupported. The explicit local backend uses a cross-process file lock around the revision check and atomic file replacement, because the local object store does not implement conditional updates. Records have a 1 MiB limit. Missing state is distinct from read failure or corrupt state.

Build verification uses separate authoritative reads for app metadata, boards, FlowScript, Events, pages, widgets, table schemas, and routes. These methods never repair remote data, accept a cached fallback after a server error, or substitute a live artifact when a requested version is missing. Staging status writes use the same explicit local/hosted routing. Normal UI reads retain their availability-oriented behavior. Pure Event definitions and component-type metadata are separated from React renderers so host validation does not load the application UI.

Scenario checkpoints retain outcome counts, a result fingerprint, and bounded diagnostics. They do not copy raw app payloads into the build journal. Full result retention belongs to the caller or a trace exporter; a fingerprint is a change detector, not a signature.

### Build operations

Use one stable `app_id` and `build_id` throughout a build. Read `app_build(operation="schema")` first for the current contract and host capabilities. Capability recipes are pinned contract scaffolds with generation briefs, not precompiled or runtime-certified workflow modules.

1. `begin` validates the complete spec and persists intent before staging an eligible empty private app. A revision-zero retry repeats staging, covering interruption between those writes. The original request is bound separately from the model-authored spec.
2. `advance` processes dependency-ready resources with bounded parallelism. A durable operation lease prevents a second caller from replaying active work. Expired operations are inspected before retry; unexplained existing artifacts remain unknown.
3. `repair` invalidates selected logical resources and their dependents. It retains the original requirements and invalidates stale verification. The next specialist brief includes prior host diagnostics. Repair does not delete retired resources or authorize a smaller replacement app.
4. `validate` checks persisted artifacts, linkage, schemas, Event routes, and current staging state. Evidence is tied to the exact applied resource snapshot.
5. `test` runs bounded acceptance scenarios through a host adapter. Read-only observations receive contract-only certification. Runtime certification requires an attested isolated execution context, terminal successful run outcomes, and expected domain state read back by the host.
6. `promote` revalidates current resources and requires behavioral coverage for every requirement. The current interactive adapter cannot supply this evidence, so promotion stays blocked.

`status` reports resource states, retry counts, scenario outcomes, and readiness blockers after interruption. Do not create replacement IDs after a timeout. Unknown or partial promotion requires review of its subreceipts before recovery.

### Activation boundaries

An inactive app is a staging convention, not a runtime sandbox. Event sinks can begin work when an Event becomes active. Promotion therefore versions resources, pins Events while they remain inactive, verifies the pins, activates the app, and activates Events serially. It records attempted writes before awaiting their responses.

This sequence is not a cross-resource transaction. On failure, the host attempts to deactivate the app and disable planned Events, including writes whose responses were lost. `external_effects_may_have_occurred` remains true after any activation attempt. Compensation cannot undo an email, external API write, or workflow that has already started.

Other clients can still mutate or activate the same app between host checks. Conditional checkpoint writes protect build state; they do not provide a server-enforced lock over every board, page, table, and Event write. Post-write readback detects some races but cannot prevent their effects.

### Current integration limits

Runtime isolation requires capability enforcement at native-node invocation
boundaries and remote dispatch. Shadow storage alone does not isolate network
calls or native side effects. An adapter for data-write scenarios needs an
ephemeral namespace or equivalent verified isolation, cleanup receipts, and a
pinned-version endpoint that can test inactive resources without exposing live
Events. It must derive terminal outcomes from the backend lifecycle and read
domain state independently of workflow-provided success text.

App-shell provisioning is not a server-idempotent operation across object
storage, SQL, initial boards, and profile association. The creation journal
records returned IDs and retries incomplete profile registration, but a lost
initial creation response still has an unknown outcome.

Operation leases protect one build checkpoint. Resource mutation and promotion
still need server-side app/build ownership fencing. Long-running delegated work
uses leases aligned with its dispatch deadline, so abrupt process loss can delay
recovery until expiry. The host has no heartbeat-based lease renewal or
relinquishment mechanism.

Capability recipes remain contract scaffolds until they have been compiled and
executed against explicit runtime and catalog versions. Unit tests and structural
fixtures do not establish a model's app-building reliability. Rebaseline live
evaluations when the provider, model, reasoning settings, harness, or runtime
changes, and report behavioral results separately from compile/apply success.

Evaluation cohorts identify the declared harness contract and fixture, provider,
model, reasoning settings, and tier. They do not attest which source or runtime
binary executed; retain that context with any baseline.

### Local verification

Run the deterministic suites without invoking a model or creating live apps:

```sh
bunx --no-install vitest run packages/ui/lib/app-build packages/ui/components/global-chat/tools --maxWorkers=4
RUSTC_WRAPPER= cargo test -p flow-like-runtime --no-default-features --features app app_build::tests --lib
RUSTC_WRAPPER= cargo test -p flow-like-editor tool_spec::tests --lib
```

Run application typechecks and API/desktop checks separately. A filtered
typecheck can help diagnose a feature failure, but does not verify the whole
repository.
