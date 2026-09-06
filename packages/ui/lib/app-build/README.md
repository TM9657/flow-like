# FlowPilot staged app builds

FlowPilot can now keep an app contract and its build progress outside a model conversation. This is an opt-in preview for developers testing app construction. The interactive host cannot yet isolate native workflow side effects, so it cannot certify runtime behavior or promote a build. Ordinary app requests continue through the existing build playbook.

## System model

An `AppSpec` describes requirements, resources, dependencies, and acceptance scenarios. The compiler validates references and assigns stable physical identifiers within an app/build pair. Specialists receive the part they own and a shared symbol table. They generate artifacts; the host reads those artifacts back before recording an applied receipt.

The desired contract fingerprint and the observed artifact fingerprint have different jobs. The former identifies requested intent. The latter detects changes in persisted content. Resource existence, a model's completion message, and a successful tool response cannot substitute for readback evidence.

| Concern | Modules |
| --- | --- |
| Typed intent and dependency planning | `contract.ts`, `compiler.ts`, `resource-links.ts` |
| Durable state, evidence, and readiness | `state-contract.ts`, `state.ts`, `state-evidence.ts`, `state-promotion.ts`, `readiness.ts`, `store.ts` |
| Lease ownership and resumable orchestration | `engine-types.ts`, `engine.ts` |
| Specialist generation and authoritative readback | `resource-materializer.ts`, `resource-readback.ts`, `host-adapter.ts` |
| Staging admission and activation compensation | `staging-lifecycle.ts`, `promotion-readback.ts`, `promotion.ts`, `staging-types.ts` |
| Scenario syntax, isolation attestations, and assertions | `scenario-contract.ts`, `scenario-runtime-types.ts`, `scenario-runner.ts`, `scenario-evidence.ts` |
| Versioned contract scaffolds | `capability-contract.ts`, `capability-foundations.ts`, `capability-registry.ts` |
| Chat integration | `tool-controller.ts`, `widget-generator.ts`, `../../components/global-chat/tools/` |

The Rust store in `packages/core/runtime/src/app_build.rs` persists app-scoped JSON checkpoints. Web/API and desktop/Tauri adapters use the same revision contract: create revision zero, then compare-and-swap from revision `n` to `n + 1`. Cloud stores use conditional writes and fail explicitly when those are unsupported. The explicit local backend uses a cross-process file lock around the revision check and atomic file replacement, because the local object store does not implement conditional updates. Records have a 1 MiB limit. Missing state is distinct from read failure or corrupt state.

Build verification uses separate authoritative reads for app metadata, boards, FlowScript, Events, pages, widgets, table schemas, and routes. These methods never repair remote data, accept a cached fallback after a server error, or substitute a live artifact when a requested version is missing. Staging status writes use the same explicit local/hosted routing. Normal UI reads retain their availability-oriented behavior. Pure Event definitions and component-type metadata are separated from React renderers so host validation does not load the application UI.

Scenario checkpoints retain outcome counts, a result fingerprint, and bounded diagnostics. They do not copy raw app payloads into the build journal. Full result retention belongs to the caller or a trace exporter; a fingerprint is a change detector, not a signature.

## Build operations

Use one stable `app_id` and `build_id` throughout a build. Read `app_build(operation="schema")` first for the current contract and host capabilities. Capability recipes are pinned contract scaffolds with generation briefs, not precompiled or runtime-certified workflow modules.

1. `begin` validates the complete spec and persists intent before staging an eligible empty private app. A revision-zero retry repeats staging, covering interruption between those writes. The original request is bound separately from the model-authored spec.
2. `advance` processes dependency-ready resources with bounded parallelism. A durable operation lease prevents a second caller from replaying active work. Expired operations are inspected before retry; unexplained existing artifacts remain unknown.
3. `repair` invalidates selected logical resources and their dependents. It retains the original requirements and invalidates stale verification. The next specialist brief includes prior host diagnostics. Repair does not delete retired resources or authorize a smaller replacement app.
4. `validate` checks persisted artifacts, linkage, schemas, Event routes, and current staging state. Evidence is tied to the exact applied resource snapshot.
5. `test` runs bounded acceptance scenarios through a host adapter. Read-only observations receive contract-only certification. Runtime certification requires an attested isolated execution context, terminal successful run outcomes, and expected domain state read back by the host.
6. `promote` revalidates current resources and requires behavioral coverage for every requirement. The current interactive adapter cannot supply this evidence, so promotion stays blocked.

`status` reports resource states, retry counts, scenario outcomes, and readiness blockers after interruption. Do not create replacement IDs after a timeout. Unknown or partial promotion requires review of its subreceipts before recovery.

## Activation boundaries

An inactive app is a staging convention, not a runtime sandbox. Event sinks can begin work when an Event becomes active. Promotion therefore versions resources, pins Events while they remain inactive, verifies the pins, activates the app, and activates Events serially. It records attempted writes before awaiting their responses.

This sequence is not a cross-resource transaction. On failure, the host attempts to deactivate the app and disable planned Events, including writes whose responses were lost. `external_effects_may_have_occurred` remains true after any activation attempt. Compensation cannot undo an email, external API write, or workflow that has already started.

Other clients can still mutate or activate the same app between host checks. Conditional checkpoint writes protect build state; they do not provide a server-enforced lock over every board, page, table, and Event write. Post-write readback detects some races but cannot prevent their effects.

## Work required before default rollout

- Enforce execution capabilities at both native-node invocation boundaries and in remote dispatch. Shadow storage alone does not isolate network calls or native side effects. Data-write scenarios need an ephemeral namespace or equivalent verified isolation, plus cleanup receipts.
- Add a pinned-version scenario endpoint that can test inactive resources without exposing live Events. Normalize terminal outcomes from the backend lifecycle and read domain state independently of workflow-provided success text.
- Make app-shell provisioning server-idempotent across object storage, SQL, initial boards, and profile association. The current creation journal records returned IDs and retries incomplete profile registration; a lost initial creation response still has an unknown outcome.
- Add server-side app/build ownership fencing across resource mutations and promotion. Current operation leases protect one build checkpoint. Long-running delegated work uses leases aligned with its dispatch deadline, so an abrupt process loss can delay recovery until expiry. Safe heartbeat-based renewal and relinquishment need separate implementation.
- Qualify capability recipes as compiled, executed modules against explicit runtime/catalog versions before describing them as tested reusable implementations.
- Rebaseline app-level evaluations for the selected provider, model, reasoning settings, harness revision, and runtime. Keep compile/apply success separate from behavioral app success. Unit tests and structural fixtures do not establish a model's app-building reliability.

The desktop evaluator now has separate `structural` and `behavioral` tiers. Its reports carry a declared harness contract version, exact provider/model/reasoning settings, and an evaluated-fixture fingerprint. The comparison guard rejects different cohort identities. This prevents accidental cross-model score comparisons; it does not recalibrate thresholds or attest which source/runtime binary executed. No live model evaluation is part of the deterministic verification below.

## Local verification

Run the deterministic suites without invoking a model or creating live apps:

```sh
bunx --no-install vitest run packages/ui/lib/app-build packages/ui/components/global-chat/tools --maxWorkers=4
RUSTC_WRAPPER= cargo test -p flow-like-runtime --no-default-features --features app app_build::tests --lib
RUSTC_WRAPPER= cargo test -p flow-like-editor tool_spec::tests --lib
```

Run application typechecks and API/desktop checks separately. Existing failures elsewhere in the workspace must be distinguished from failures in this feature; a filtered typecheck is useful for diagnosis but is not a clean repository-wide build.
