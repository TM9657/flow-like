import type {
	AppScenarioTool,
	ScenarioJsonObject,
	ScenarioJsonValue,
} from "./scenario-contract";

export type AppScenarioRequiredCapability = "read_only" | "isolated_runtime";

export interface AppScenarioResolvedResource {
	readonly id: string;
	readonly kind: "app" | "chat" | "event" | "page";
	readonly appId?: string;
	/** Event id when a page resource's durable id is the page id. */
	readonly eventId?: string;
	/** Page id when a page resource's durable id is the Event id. */
	readonly pageId?: string;
}

interface HostScenarioAttestationBase {
	/** Literal discriminator. The host adapter, not the scenario, creates this object. */
	readonly source: "host";
	readonly attestationId: string;
	readonly appId: string;
	readonly tools: readonly AppScenarioTool[];
	readonly attestedAtMs: number;
	readonly expiresAtMs: number;
}

export interface HostReadOnlyScenarioAttestation
	extends HostScenarioAttestationBase {
	readonly mode: "read_only";
}

export interface HostIsolatedRuntimeScenarioAttestation
	extends HostScenarioAttestationBase {
	readonly mode: "isolated_runtime";
	readonly isolationId: string;
	readonly strategy:
		| "ephemeral_app"
		| "snapshot_restore"
		| "transaction_rollback";
}

export type HostScenarioAttestation =
	| HostReadOnlyScenarioAttestation
	| HostIsolatedRuntimeScenarioAttestation;

export interface AppScenarioAttestationRequest {
	readonly appId: string;
	readonly scenarioId: string;
	readonly target: AppScenarioResolvedResource;
	readonly tools: readonly AppScenarioTool[];
	readonly requiredCapability: AppScenarioRequiredCapability;
	readonly deadlineAtMs: number;
	readonly signal: AbortSignal;
}

export interface AppScenarioToolInvocationRequest {
	readonly invocationId: string;
	readonly scenarioId: string;
	readonly stepId: string;
	readonly appId: string;
	readonly target: AppScenarioResolvedResource;
	readonly tool: AppScenarioTool;
	readonly arguments: ScenarioJsonObject;
	readonly isolationId?: string;
	readonly deadlineAtMs: number;
	readonly signal: AbortSignal;
}

export type HostRuntimeTerminalState =
	| "succeeded"
	| "failed"
	| "cancelled"
	| "timed_out"
	| "unknown";

/**
 * A terminal workflow outcome normalized by the host from the backend execution lifecycle.
 * Transport acknowledgements and model-authored payloads are not valid sources for this value.
 */
export interface HostRuntimeOutcome {
	readonly sourceRunId: string;
	readonly state: HostRuntimeTerminalState;
	readonly completedAtMs: number;
	/** Count of backend error/fatal log records associated with this run, when available. */
	readonly errorCount?: number;
}

/**
 * Every result is correlated to one runner-created invocation and timestamped by the host.
 * Returning a bare cached payload is intentionally insufficient evidence.
 */
export interface AppScenarioToolObservation {
	readonly invocationId: string;
	readonly observedAtMs: number;
	readonly value: unknown;
	/** Required for every runtime invocation, including every run started by one page action. */
	readonly runtimeOutcomes?: readonly HostRuntimeOutcome[];
	/** Fresh domain state read back from the isolated runtime after the invocation completes. */
	readonly state?: unknown;
}

/** Trusted integration boundary. Scenario/model JSON can never supply these capabilities. */
export interface AppScenarioHostAdapter {
	attest(
		request: AppScenarioAttestationRequest,
	): Promise<HostScenarioAttestation>;
	invoke(
		request: AppScenarioToolInvocationRequest,
	): Promise<AppScenarioToolObservation>;
}

export interface RunAppBehaviorScenarioOptions {
	readonly appId: string;
	readonly resources: Readonly<Record<string, AppScenarioResolvedResource>>;
	readonly adapter: AppScenarioHostAdapter;
	readonly signal?: AbortSignal;
	/** A host ceiling. A scenario can request less time, never more. */
	readonly maxDurationMs?: number;
	/** Test/host correlation hook. It must return a new id for every call. */
	readonly createInvocationId?: (stepId: string, index: number) => string;
}

export type AppScenarioRunStatus =
	| "pass"
	| "fail"
	| "blocked"
	| "cancelled"
	| "timed_out"
	| "unknown";

export interface AppScenarioIssue {
	readonly code: string;
	readonly message: string;
	readonly path?: string;
}

export interface AppScenarioStepResult {
	readonly step_id: string;
	readonly tool: AppScenarioTool;
	readonly invocation_id?: string;
	readonly status:
		| "not_started"
		| "observed"
		| "cancelled"
		| "timed_out"
		| "unknown";
	readonly started_at_ms?: number;
	readonly completed_at_ms?: number;
	readonly observed_at_ms?: number;
	readonly arguments?: ScenarioJsonObject;
	readonly value?: ScenarioJsonValue;
	readonly state?: ScenarioJsonValue;
	readonly runtime_outcomes?: readonly AppScenarioStartedRun[];
	readonly message?: string;
}

export interface AppScenarioAssertionResult {
	readonly assertion_id: string;
	readonly step_id: string;
	readonly status: "pass" | "fail" | "not_evaluated";
	readonly message: string;
	readonly path?: string;
}

export interface AppScenarioStartedRun {
	readonly step_id: string;
	readonly run_id: string;
	readonly status?: string;
	readonly completed_at_ms?: number;
	readonly error_count?: number;
}

export interface AppScenarioRunResult {
	readonly schema: "flowpilot.app-behavior-scenario-result/v1";
	readonly scenario_id: string;
	readonly app_id: string;
	readonly status: AppScenarioRunStatus;
	readonly outcome_known: boolean;
	readonly outstanding: boolean;
	/** Read-only passes validate a contract but never certify runtime behavior. */
	readonly certification: "none" | "contract_only" | "behavioral";
	readonly required_capability: AppScenarioRequiredCapability;
	readonly started_at_ms: number;
	readonly completed_at_ms: number;
	readonly deadline_at_ms: number;
	readonly attestation?: {
		readonly id: string;
		readonly mode: HostScenarioAttestation["mode"];
		readonly isolation_id?: string;
	};
	readonly steps: readonly AppScenarioStepResult[];
	readonly assertions: readonly AppScenarioAssertionResult[];
	readonly metrics: {
		readonly steps_total: number;
		readonly steps_invoked: number;
		readonly assertions_total: number;
		readonly assertions_passed: number;
		readonly started_runs: number;
		readonly successful_runs: number;
		readonly failed_runs: number;
		readonly unknown_runs: number;
	};
	readonly started_runs: readonly AppScenarioStartedRun[];
	readonly issues: readonly AppScenarioIssue[];
}
