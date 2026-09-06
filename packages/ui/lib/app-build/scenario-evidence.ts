import { stableStringify } from "../stable-stringify";
import {
	APP_SCENARIO_READ_ONLY_TOOLS,
	APP_SCENARIO_RUNTIME_TOOLS,
	type AppBehaviorScenario,
	type AppScenarioAssertion,
	type AppScenarioReference,
	type AppScenarioRuntimeTool,
	type AppScenarioTool,
	type ScenarioJsonObject,
	type ScenarioJsonValue,
} from "./scenario-contract";
import type {
	AppScenarioAssertionResult,
	AppScenarioRequiredCapability,
	AppScenarioResolvedResource,
	AppScenarioStartedRun,
	AppScenarioStepResult,
	HostRuntimeOutcome,
} from "./scenario-runtime-types";

export const MAX_APP_SCENARIO_ARGUMENT_BYTES = 64 * 1024;

const DEFAULT_SUCCESS_STATUSES = [
	"ok",
	"success",
	"succeeded",
	"completed",
] as const;
const READ_ONLY_TOOL_SET = new Set<string>(APP_SCENARIO_READ_ONLY_TOOLS);
const RUNTIME_TOOL_SET = new Set<string>(APP_SCENARIO_RUNTIME_TOOLS);

export interface ScenarioPointerResult {
	readonly found: boolean;
	readonly value?: unknown;
}

export interface ScenarioStepEvidence {
	readonly value: ScenarioJsonValue;
	readonly state?: ScenarioJsonValue;
	readonly runtimeOutcomes: readonly HostRuntimeOutcome[];
}

export function isScenarioRecord(
	value: unknown,
): value is Record<string, unknown> {
	return Boolean(value) && typeof value === "object" && !Array.isArray(value);
}

function unescapePointerSegment(segment: string): string {
	return segment.replace(/~1/g, "/").replace(/~0/g, "~");
}

/** Resolve an already-validated RFC 6901 pointer without getters or JSONPath code. */
export function resolveScenarioJsonPointer(
	root: unknown,
	pointer: string,
): ScenarioPointerResult {
	if (pointer === "") return { found: true, value: root };
	let current = root;
	for (const rawSegment of pointer.slice(1).split("/")) {
		const segment = unescapePointerSegment(rawSegment);
		if (Array.isArray(current)) {
			if (!/^(0|[1-9][0-9]*)$/.test(segment)) return { found: false };
			const index = Number(segment);
			if (!Number.isSafeInteger(index) || index >= current.length) {
				return { found: false };
			}
			current = current[index];
			continue;
		}
		if (!isScenarioRecord(current) || !Object.hasOwn(current, segment)) {
			return { found: false };
		}
		current = current[segment];
	}
	return { found: true, value: current };
}

export function scenarioJsonValidationIssue(
	value: unknown,
): string | undefined {
	const seen = new WeakSet<object>();
	let entries = 0;
	const visit = (candidate: unknown, depth: number): string | undefined => {
		if (depth > 32) return "JSON evidence exceeds the maximum nesting depth.";
		if (
			candidate === null ||
			typeof candidate === "string" ||
			typeof candidate === "boolean"
		) {
			return undefined;
		}
		if (typeof candidate === "number") {
			return Number.isFinite(candidate)
				? undefined
				: "JSON evidence contains a non-finite number.";
		}
		if (typeof candidate !== "object") {
			return `JSON evidence contains unsupported ${typeof candidate}.`;
		}
		if (seen.has(candidate)) return "JSON evidence contains a cycle.";
		seen.add(candidate);
		const values = Array.isArray(candidate)
			? candidate
			: Object.values(candidate as Record<string, unknown>);
		entries += values.length;
		if (entries > 10_000) return "JSON evidence exceeds the entry limit.";
		for (const entry of values) {
			const issue = visit(entry, depth + 1);
			if (issue) return issue;
		}
		return undefined;
	};
	const issue = visit(value, 0);
	if (issue) return issue;
	try {
		if (
			new TextEncoder().encode(JSON.stringify(value)).byteLength >
			512 * 1024
		) {
			return "JSON evidence exceeds the byte limit.";
		}
	} catch {
		return "JSON evidence cannot be serialized.";
	}
	return undefined;
}

function isScenarioReference(value: unknown): value is AppScenarioReference {
	return isScenarioRecord(value) && Object.hasOwn(value, "$scenario_ref");
}

function resolveReference(
	reference: AppScenarioReference,
	app: ScenarioJsonObject,
	target: ScenarioJsonObject,
	observations: ReadonlyMap<string, ScenarioJsonValue>,
): ScenarioJsonValue {
	const descriptor = reference.$scenario_ref;
	let source: unknown;
	if (descriptor.source === "app") source = app;
	else if (descriptor.source === "target") source = target;
	else {
		const observation = observations.get(descriptor.step_id ?? "");
		if (observation === undefined) {
			throw new Error(
				`Step reference ${descriptor.step_id ?? "(missing)"} has no fresh observation.`,
			);
		}
		source = observation;
	}
	const resolved = resolveScenarioJsonPointer(source, descriptor.path ?? "");
	if (!resolved.found || scenarioJsonValidationIssue(resolved.value)) {
		throw new Error(
			`Scenario reference ${descriptor.source}${descriptor.step_id ? `:${descriptor.step_id}` : ""}${descriptor.path ?? ""} did not resolve to JSON evidence.`,
		);
	}
	return resolved.value as ScenarioJsonValue;
}

export function resolveScenarioValue(
	value: ScenarioJsonValue,
	app: ScenarioJsonObject,
	target: ScenarioJsonObject,
	observations: ReadonlyMap<string, ScenarioJsonValue>,
): ScenarioJsonValue {
	if (isScenarioReference(value)) {
		if (
			!isScenarioRecord(value.$scenario_ref) ||
			(value.$scenario_ref.source !== "app" &&
				value.$scenario_ref.source !== "target" &&
				value.$scenario_ref.source !== "step")
		) {
			throw new Error("Malformed $scenario_ref was rejected.");
		}
		return resolveReference(value, app, target, observations);
	}
	if (Array.isArray(value)) {
		return value.map((entry) =>
			resolveScenarioValue(entry, app, target, observations),
		);
	}
	if (isScenarioRecord(value)) {
		return Object.fromEntries(
			Object.entries(value).map(([key, entry]) => [
				key,
				resolveScenarioValue(
					entry as ScenarioJsonValue,
					app,
					target,
					observations,
				),
			]),
		) as ScenarioJsonObject;
	}
	return value;
}

export function scenarioTargetEvidence(
	appId: string,
	resourceKey: string,
	resource: AppScenarioResolvedResource,
): ScenarioJsonObject {
	const eventId =
		resource.eventId ??
		(resource.kind === "chat" || resource.kind === "event"
			? resource.id
			: undefined);
	const pageId =
		resource.pageId ?? (resource.kind === "page" ? resource.id : undefined);
	return {
		id: resource.id,
		resource_key: resourceKey,
		kind: resource.kind,
		app_id: appId,
		...(eventId ? { event_id: eventId } : {}),
		...(pageId ? { page_id: pageId } : {}),
	};
}

/** Bind app/interface ids from trusted context and reject cross-app or cross-target overrides. */
export function bindScenarioToolArguments(
	tool: AppScenarioTool,
	resolved: ScenarioJsonObject,
	appId: string,
	target: AppScenarioResolvedResource,
): ScenarioJsonObject {
	const argumentsCopy = { ...resolved } as Record<string, ScenarioJsonValue>;
	const suppliedAppId = argumentsCopy.app_id;
	if (suppliedAppId !== undefined && suppliedAppId !== appId) {
		throw new Error("Scenario arguments cannot select a different app_id.");
	}
	argumentsCopy.app_id = appId;

	if (tool === "describe_app_interface") {
		if (target.kind === "app") {
			throw new Error("describe_app_interface requires an interface target.");
		}
		bindExactArgument(argumentsCopy, "event_id", target.eventId ?? target.id);
	}
	if (tool === "call_app_event") {
		if (target.kind !== "event") {
			throw new Error("call_app_event requires an event target.");
		}
		bindExactArgument(argumentsCopy, "event_id", target.eventId ?? target.id);
	}
	if (tool === "call_app_chat") {
		if (target.kind !== "chat") {
			throw new Error("call_app_chat requires a chat target.");
		}
		bindExactArgument(argumentsCopy, "event_id", target.eventId ?? target.id);
	}
	if (tool === "interact_app_page") {
		if (target.kind !== "page") {
			throw new Error("interact_app_page requires a page target.");
		}
		if (target.eventId) {
			bindExactArgument(argumentsCopy, "event_id", target.eventId);
		} else {
			bindExactArgument(argumentsCopy, "page_id", target.pageId ?? target.id);
		}
	}

	const byteLength = new TextEncoder().encode(
		JSON.stringify(argumentsCopy),
	).byteLength;
	if (byteLength > MAX_APP_SCENARIO_ARGUMENT_BYTES) {
		throw new Error(
			`Scenario tool arguments exceed ${MAX_APP_SCENARIO_ARGUMENT_BYTES} bytes.`,
		);
	}
	return argumentsCopy;
}

function bindExactArgument(
	args: Record<string, ScenarioJsonValue>,
	key: "event_id" | "page_id",
	value: string,
) {
	if (args[key] !== undefined && args[key] !== value) {
		throw new Error(`Scenario arguments cannot select a different ${key}.`);
	}
	args[key] = value;
}

export function requiredCapabilityForScenario(
	scenario: Pick<AppBehaviorScenario, "steps">,
): AppScenarioRequiredCapability {
	return scenario.steps.some((step) => RUNTIME_TOOL_SET.has(step.tool))
		? "isolated_runtime"
		: "read_only";
}

export function validateRuntimeScenarioAssertions(
	scenario: AppBehaviorScenario,
): string[] {
	const issues: string[] = [];
	for (const step of scenario.steps) {
		if (!RUNTIME_TOOL_SET.has(step.tool)) continue;
		const assertions = scenario.assertions.filter(
			(assertion) => assertion.step_id === step.id,
		);
		if (!assertions.some((assertion) => assertion.kind === "run_outcome")) {
			issues.push(
				`Runtime step ${step.id} has no host-terminal run_outcome assertion.`,
			);
		}
		if (!assertions.some((assertion) => assertion.kind === "state")) {
			issues.push(
				`Runtime step ${step.id} has no host-observed domain state assertion.`,
			);
		}
	}
	return issues;
}

export function isRuntimeScenarioTool(
	tool: AppScenarioTool,
): tool is AppScenarioRuntimeTool {
	return RUNTIME_TOOL_SET.has(tool);
}

export function isReadOnlyScenarioTool(
	tool: string,
): tool is (typeof APP_SCENARIO_READ_ONLY_TOOLS)[number] {
	return READ_ONLY_TOOL_SET.has(tool);
}

function normalizedStatus(value: unknown): string | undefined {
	if (!isScenarioRecord(value)) return undefined;
	const status = value.status;
	return typeof status === "string" && status.trim()
		? status.trim().toLowerCase()
		: undefined;
}

const TERMINAL_RUNTIME_STATES = new Set([
	"succeeded",
	"failed",
	"cancelled",
	"timed_out",
	"unknown",
]);

export function runtimeOutcomeEvidenceIssue(
	outcomes: readonly HostRuntimeOutcome[] | undefined,
	invokedAtMs: number,
	observedAtMs: number,
	clockSkewMs: number,
): string | undefined {
	if (!Array.isArray(outcomes) || outcomes.length === 0) {
		return "Runtime observation has no host-normalized terminal outcome.";
	}
	const runIds = new Set<string>();
	for (const outcome of outcomes) {
		const sourceRunId: unknown = outcome?.sourceRunId;
		const state: unknown = outcome?.state;
		const completedAtMs: unknown = outcome?.completedAtMs;
		const errorCount: unknown = outcome?.errorCount;
		if (typeof sourceRunId !== "string" || !sourceRunId.trim()) {
			return "Host-normalized terminal outcome has no source run id.";
		}
		if (runIds.has(sourceRunId)) {
			return `Host-normalized terminal outcome repeats run ${sourceRunId}.`;
		}
		runIds.add(sourceRunId);
		if (typeof state !== "string" || !TERMINAL_RUNTIME_STATES.has(state)) {
			return `Run ${sourceRunId} has an unsupported terminal state.`;
		}
		if (
			typeof completedAtMs !== "number" ||
			!Number.isFinite(completedAtMs) ||
			completedAtMs < invokedAtMs - clockSkewMs ||
			completedAtMs > observedAtMs + clockSkewMs
		) {
			return `Run ${sourceRunId} has stale or invalid terminal timing.`;
		}
		if (
			errorCount !== undefined &&
			(typeof errorCount !== "number" ||
				!Number.isSafeInteger(errorCount) ||
				errorCount < 0)
		) {
			return `Run ${sourceRunId} has an invalid error count.`;
		}
		if (
			state === "succeeded" &&
			typeof errorCount === "number" &&
			errorCount > 0
		) {
			return `Run ${sourceRunId} is marked succeeded but has backend error logs.`;
		}
	}
	return undefined;
}

export function hasScenarioErrorEvidence(value: ScenarioJsonValue): boolean {
	if (!isScenarioRecord(value)) return false;
	const status = normalizedStatus(value);
	if (status && ["error", "failed", "fatal"].includes(status)) return true;
	const logs = value.logs;
	if (!Array.isArray(logs)) return false;
	return logs.some((entry) => {
		if (!isScenarioRecord(entry)) return false;
		const severity = entry.level ?? entry.log_level ?? entry.severity;
		return (
			(typeof severity === "number" && severity >= 4) ||
			(typeof severity === "string" &&
				["error", "fatal"].includes(severity.trim().toLowerCase()))
		);
	});
}

export function scenarioStartedRunsFromHost(
	stepId: string,
	outcomes: readonly HostRuntimeOutcome[],
): AppScenarioStartedRun[] {
	return outcomes.map((outcome) => ({
		step_id: stepId,
		run_id: outcome.sourceRunId,
		status: outcome.state,
		completed_at_ms: outcome.completedAtMs,
		...(outcome.errorCount === undefined
			? {}
			: { error_count: outcome.errorCount }),
	}));
}

export function collectScenarioStartedRuns(
	stepId: string,
	value: ScenarioJsonValue,
): AppScenarioStartedRun[] {
	const runs: AppScenarioStartedRun[] = [];
	const seenObjects = new WeakSet<object>();
	const visit = (candidate: ScenarioJsonValue) => {
		if (!candidate || typeof candidate !== "object") return;
		if (seenObjects.has(candidate)) return;
		seenObjects.add(candidate);
		if (Array.isArray(candidate)) {
			for (const entry of candidate) visit(entry);
			return;
		}
		const record = candidate as ScenarioJsonObject;
		const id = record.run_id ?? record.runId;
		if (typeof id === "string" && id.trim()) {
			const status = normalizedStatus(record);
			runs.push({
				step_id: stepId,
				run_id: id,
				...(status ? { status } : {}),
			});
		}
		for (const entry of Object.values(record)) visit(entry);
	};
	visit(value);
	const unique = new Map<string, AppScenarioStartedRun>();
	for (const run of runs) unique.set(`${run.step_id}:${run.run_id}`, run);
	return [...unique.values()];
}

function deepEqual(left: unknown, right: unknown): boolean {
	return stableStringify(left) === stableStringify(right);
}

export function evaluateScenarioAssertion(
	assertion: AppScenarioAssertion,
	evidence: ReadonlyMap<string, ScenarioStepEvidence>,
	runtimeStepIds: ReadonlySet<string>,
	app: ScenarioJsonObject,
	target: ScenarioJsonObject,
): AppScenarioAssertionResult {
	const stepEvidence = evidence.get(assertion.step_id);
	if (stepEvidence === undefined) {
		return {
			assertion_id: assertion.id,
			step_id: assertion.step_id,
			status: "not_evaluated",
			message: "The referenced step has no fresh observation.",
		};
	}
	const observation = stepEvidence.value;
	const observations = new Map(
		[...evidence.entries()].map(([stepId, candidate]) => [
			stepId,
			candidate.value,
		]),
	);
	if (assertion.kind === "run_outcome") {
		if (!runtimeStepIds.has(assertion.step_id)) {
			return {
				assertion_id: assertion.id,
				step_id: assertion.step_id,
				status: "fail",
				message:
					"A run outcome must reference a runtime invocation, not a readback.",
			};
		}
		const id = resolveScenarioJsonPointer(observation, assertion.run_id_path);
		const runId =
			id.found && typeof id.value === "string" && id.value.trim()
				? id.value.trim()
				: undefined;
		const terminal = runId
			? stepEvidence.runtimeOutcomes.find(
					(outcome) => outcome.sourceRunId === runId,
				)
			: undefined;
		let statusPasses = true;
		let statusMessage = "";
		if (assertion.status_path !== undefined) {
			const status = resolveScenarioJsonPointer(
				observation,
				assertion.status_path,
			);
			const actual =
				status.found && typeof status.value === "string"
					? status.value.trim().toLowerCase()
					: undefined;
			const allowed = (
				assertion.allowed_statuses ?? DEFAULT_SUCCESS_STATUSES
			).map((candidate) => candidate.trim().toLowerCase());
			statusPasses = Boolean(actual && allowed.includes(actual));
			statusMessage = actual
				? ` with status ${actual}`
				: " but no terminal status was observed";
		}
		const terminalPasses =
			terminal?.state === "succeeded" && (terminal.errorCount ?? 0) === 0;
		const passed = Boolean(runId && terminalPasses && statusPasses);
		return {
			assertion_id: assertion.id,
			step_id: assertion.step_id,
			status: passed ? "pass" : "fail",
			message: passed
				? `Host observed terminal success for run ${runId}${statusMessage}.`
				: !runId
					? "Expected a fresh non-empty run id."
					: !terminal
						? `Run ${runId} has no matching host-normalized terminal outcome.`
						: `Run ${runId} ended ${terminal.state}${statusMessage}.`,
			path: assertion.run_id_path,
		};
	}

	const assertionSource =
		assertion.kind === "state" ? stepEvidence.state : observation;
	if (assertionSource === undefined) {
		return {
			assertion_id: assertion.id,
			step_id: assertion.step_id,
			status: "not_evaluated",
			message: "The runtime step has no fresh host-observed domain state.",
			path: assertion.path,
		};
	}
	const actual = resolveScenarioJsonPointer(assertionSource, assertion.path);
	let expected: ScenarioJsonValue | undefined;
	if (Object.hasOwn(assertion, "expected")) {
		try {
			expected = resolveScenarioValue(
				assertion.expected as ScenarioJsonValue,
				app,
				target,
				observations,
			);
		} catch (error) {
			return {
				assertion_id: assertion.id,
				step_id: assertion.step_id,
				status: "fail",
				message: error instanceof Error ? error.message : String(error),
				path: assertion.path,
			};
		}
	}
	let passed = false;
	switch (assertion.operator) {
		case "exists":
			passed = actual.found;
			break;
		case "equals":
			passed = actual.found && deepEqual(actual.value, expected);
			break;
		case "not_equals":
			passed = actual.found && !deepEqual(actual.value, expected);
			break;
		case "contains":
			passed =
				actual.found &&
				((typeof actual.value === "string" &&
					typeof expected === "string" &&
					actual.value.includes(expected)) ||
					(Array.isArray(actual.value) &&
						actual.value.some((entry) => deepEqual(entry, expected))));
			break;
		case "length_at_least": {
			const length =
				typeof actual.value === "string" || Array.isArray(actual.value)
					? actual.value.length
					: isScenarioRecord(actual.value)
						? Object.keys(actual.value).length
						: undefined;
			passed =
				actual.found &&
				length !== undefined &&
				typeof expected === "number" &&
				length >= expected;
			break;
		}
	}
	return {
		assertion_id: assertion.id,
		step_id: assertion.step_id,
		status: passed ? "pass" : "fail",
		message: passed
			? `${assertion.operator} assertion passed at ${assertion.path || "/"}.`
			: `${assertion.operator} assertion failed at ${assertion.path || "/"}.`,
		path: assertion.path,
	};
}

export function appScenarioRunMetrics(
	stepsTotal: number,
	steps: readonly AppScenarioStepResult[],
	assertionsTotal: number,
	assertions: readonly AppScenarioAssertionResult[],
	startedRuns: readonly AppScenarioStartedRun[],
) {
	const successfulRuns = startedRuns.filter((run) =>
		DEFAULT_SUCCESS_STATUSES.includes(
			(run.status ?? "") as (typeof DEFAULT_SUCCESS_STATUSES)[number],
		),
	).length;
	const failedRuns = startedRuns.filter((run) =>
		[
			"error",
			"failed",
			"cancelled",
			"canceled",
			"timed_out",
			"timeout",
		].includes(run.status ?? ""),
	).length;
	return {
		steps_total: stepsTotal,
		steps_invoked: steps.filter((step) => step.invocation_id).length,
		assertions_total: assertionsTotal,
		assertions_passed: assertions.filter((result) => result.status === "pass")
			.length,
		started_runs: startedRuns.length,
		successful_runs: successfulRuns,
		failed_runs: failedRuns,
		unknown_runs: startedRuns.length - successfulRuns - failedRuns,
	};
}
