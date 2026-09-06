import {
	type AppBehaviorScenario,
	type AppScenarioTool,
	type ScenarioJsonObject,
	type ScenarioJsonValue,
	appBehaviorScenarioSchema,
} from "./scenario-contract";
import {
	type ScenarioStepEvidence,
	evaluateScenarioAssertion as assertionResult,
	hasScenarioErrorEvidence,
	isScenarioRecord as isRecord,
	isRuntimeScenarioTool as isRuntimeTool,
	scenarioJsonValidationIssue as jsonValidationIssue,
	requiredCapabilityForScenario,
	resolveScenarioValue,
	appScenarioRunMetrics as runMetrics,
	runtimeOutcomeEvidenceIssue,
	scenarioStartedRunsFromHost,
	scenarioTargetEvidence as targetEvidence,
	bindScenarioToolArguments as toolArguments,
	validateRuntimeScenarioAssertions as validateRuntimeAssertions,
} from "./scenario-evidence";
import type {
	AppScenarioIssue,
	AppScenarioRequiredCapability,
	AppScenarioRunResult,
	AppScenarioRunStatus,
	AppScenarioStartedRun,
	AppScenarioStepResult,
	AppScenarioToolObservation,
	HostScenarioAttestation,
	RunAppBehaviorScenarioOptions,
} from "./scenario-runtime-types";

export const DEFAULT_APP_SCENARIO_TIMEOUT_MS = 60_000;
export const MAX_APP_SCENARIO_TIMEOUT_MS = 10 * 60_000;

const OBSERVATION_CLOCK_SKEW_MS = 1_000;

class ScenarioAbort extends Error {
	constructor(readonly kind: "cancelled" | "timed_out") {
		super(
			kind === "cancelled" ? "Scenario was cancelled." : "Scenario timed out.",
		);
	}
}

function attestationIssue(
	attestation: HostScenarioAttestation,
	requestStartedAtMs: number,
	deadlineAtMs: number,
	appId: string,
	tools: readonly AppScenarioTool[],
	requiredCapability: AppScenarioRequiredCapability,
): string | undefined {
	if (!isRecord(attestation) || attestation.source !== "host") {
		return "The adapter did not return a host attestation.";
	}
	if (!attestation.attestationId?.trim()) {
		return "The host attestation has no id.";
	}
	if (attestation.appId !== appId) {
		return "The host attestation is scoped to a different app.";
	}
	if (
		!Number.isFinite(attestation.attestedAtMs) ||
		attestation.attestedAtMs < requestStartedAtMs - OBSERVATION_CLOCK_SKEW_MS ||
		attestation.attestedAtMs > Date.now() + OBSERVATION_CLOCK_SKEW_MS
	) {
		return "The host attestation is stale or has an invalid timestamp.";
	}
	if (
		!Number.isFinite(attestation.expiresAtMs) ||
		attestation.expiresAtMs < deadlineAtMs
	) {
		return "The host attestation expires before the scenario deadline.";
	}
	const attestedTools = new Set(attestation.tools);
	if (tools.some((tool) => !attestedTools.has(tool))) {
		return "The host attestation does not cover every scenario tool.";
	}
	if (requiredCapability === "isolated_runtime") {
		if (attestation.mode !== "isolated_runtime") {
			return "Runtime behavior requires host-attested isolation.";
		}
		if (!attestation.isolationId?.trim()) {
			return "The isolated runtime attestation has no isolation id.";
		}
		if (
			attestation.strategy !== "ephemeral_app" &&
			attestation.strategy !== "snapshot_restore" &&
			attestation.strategy !== "transaction_rollback"
		) {
			return "The isolated runtime attestation has no supported strategy.";
		}
	}
	return undefined;
}

function uniqueTools(scenario: AppBehaviorScenario): AppScenarioTool[] {
	return [...new Set(scenario.steps.map((step) => step.tool))];
}

function scenarioAbortKind(
	timedOut: boolean,
	externalSignal: AbortSignal | undefined,
): "cancelled" | "timed_out" {
	return timedOut || !externalSignal?.aborted ? "timed_out" : "cancelled";
}

function awaitWithAbort<T>(
	promise: Promise<T>,
	signal: AbortSignal,
): Promise<T> {
	if (signal.aborted) return Promise.reject(new ScenarioAbort("cancelled"));
	return new Promise<T>((resolve, reject) => {
		const abort = () => reject(new ScenarioAbort("cancelled"));
		signal.addEventListener("abort", abort, { once: true });
		promise.then(
			(value) => {
				signal.removeEventListener("abort", abort);
				resolve(value);
			},
			(error) => {
				signal.removeEventListener("abort", abort);
				reject(error);
			},
		);
	});
}

function emptyResult(
	scenarioId: string,
	appId: string,
	status: AppScenarioRunStatus,
	requiredCapability: AppScenarioRequiredCapability,
	startedAtMs: number,
	deadlineAtMs: number,
	issues: readonly AppScenarioIssue[],
	counts: { steps: number; assertions: number } = { steps: 0, assertions: 0 },
): AppScenarioRunResult {
	const completedAtMs = Date.now();
	return {
		schema: "flowpilot.app-behavior-scenario-result/v1",
		scenario_id: scenarioId,
		app_id: appId,
		status,
		outcome_known:
			status !== "unknown" && status !== "timed_out" && status !== "cancelled",
		outstanding: status === "blocked",
		certification: "none",
		required_capability: requiredCapability,
		started_at_ms: startedAtMs,
		completed_at_ms: completedAtMs,
		deadline_at_ms: deadlineAtMs,
		steps: [],
		assertions: [],
		metrics: runMetrics(counts.steps, [], counts.assertions, [], []),
		started_runs: [],
		issues,
	};
}

/**
 * Execute a deterministic, bounded scenario against a trusted host adapter.
 *
 * Runtime calls are blocked before the first tool invocation unless the host attests a fresh,
 * app-scoped isolation boundary. A scenario's own `safe`, `safe_to_run`, or effect-like data is
 * rejected by the strict schema and can never relax this decision.
 */
export async function runAppBehaviorScenario(
	scenarioInput: unknown,
	options: RunAppBehaviorScenarioOptions,
): Promise<AppScenarioRunResult> {
	const startedAtMs = Date.now();
	const parsed = appBehaviorScenarioSchema.safeParse(scenarioInput);
	if (!parsed.success) {
		return emptyResult(
			isRecord(scenarioInput) && typeof scenarioInput.id === "string"
				? scenarioInput.id
				: "invalid_scenario",
			options.appId,
			"fail",
			"read_only",
			startedAtMs,
			startedAtMs,
			parsed.error.issues.map((issue) => ({
				code: "scenario.invalid",
				message: issue.message,
				path: issue.path.join("."),
			})),
		);
	}
	const scenario = parsed.data;
	const requiredCapability = requiredCapabilityForScenario(scenario);
	const requestedDuration =
		scenario.timeout_ms ?? DEFAULT_APP_SCENARIO_TIMEOUT_MS;
	const hostCeiling = Math.max(
		100,
		Math.min(
			options.maxDurationMs ?? MAX_APP_SCENARIO_TIMEOUT_MS,
			MAX_APP_SCENARIO_TIMEOUT_MS,
		),
	);
	const timeoutMs = Math.min(requestedDuration, hostCeiling);
	const deadlineAtMs = startedAtMs + timeoutMs;
	if (!options.appId.trim()) {
		return emptyResult(
			scenario.id,
			options.appId,
			"fail",
			requiredCapability,
			startedAtMs,
			deadlineAtMs,
			[{ code: "context.app_missing", message: "Scenario appId is empty." }],
			{ steps: scenario.steps.length, assertions: scenario.assertions.length },
		);
	}
	const target = options.resources[scenario.target.resource_key];
	if (
		!target ||
		!target.id.trim() ||
		target.kind !== scenario.target.kind ||
		(target.appId !== undefined && target.appId !== options.appId)
	) {
		return emptyResult(
			scenario.id,
			options.appId,
			"fail",
			requiredCapability,
			startedAtMs,
			deadlineAtMs,
			[
				{
					code: "context.target_unresolved",
					message: `Scenario target ${scenario.target.resource_key} did not resolve to the exact ${scenario.target.kind} resource in this app.`,
				},
			],
			{ steps: scenario.steps.length, assertions: scenario.assertions.length },
		);
	}
	const missingRuntimeAssertions = validateRuntimeAssertions(scenario);
	if (missingRuntimeAssertions.length > 0) {
		return emptyResult(
			scenario.id,
			options.appId,
			"fail",
			requiredCapability,
			startedAtMs,
			deadlineAtMs,
			missingRuntimeAssertions.map((message) => ({
				code: "scenario.runtime_step_unasserted",
				message,
			})),
			{ steps: scenario.steps.length, assertions: scenario.assertions.length },
		);
	}

	const controller = new AbortController();
	let timedOut = false;
	const abortFromCaller = () => controller.abort(options.signal?.reason);
	if (options.signal?.aborted) abortFromCaller();
	else
		options.signal?.addEventListener("abort", abortFromCaller, { once: true });
	const timeout = setTimeout(() => {
		timedOut = true;
		controller.abort(new ScenarioAbort("timed_out"));
	}, timeoutMs);
	const tools = uniqueTools(scenario);
	const targetJson = targetEvidence(
		options.appId,
		scenario.target.resource_key,
		target,
	);
	const appJson: ScenarioJsonObject = {
		id: options.appId,
		app_id: options.appId,
	};
	const observations = new Map<string, ScenarioJsonValue>();
	const evidence = new Map<string, ScenarioStepEvidence>();
	const stepResults: AppScenarioStepResult[] = [];
	const startedRuns: AppScenarioStartedRun[] = [];
	const invocationIds = new Set<string>();
	let verifiedAttestation: HostScenarioAttestation | undefined;

	try {
		if (controller.signal.aborted) {
			throw new ScenarioAbort(scenarioAbortKind(timedOut, options.signal));
		}
		const attestationStartedAtMs = Date.now();
		let attestation: HostScenarioAttestation;
		try {
			attestation = await awaitWithAbort(
				options.adapter.attest({
					appId: options.appId,
					scenarioId: scenario.id,
					target,
					tools,
					requiredCapability,
					deadlineAtMs,
					signal: controller.signal,
				}),
				controller.signal,
			);
		} catch (error) {
			if (controller.signal.aborted || error instanceof ScenarioAbort) {
				throw new ScenarioAbort(scenarioAbortKind(timedOut, options.signal));
			}
			return emptyResult(
				scenario.id,
				options.appId,
				"blocked",
				requiredCapability,
				startedAtMs,
				deadlineAtMs,
				[
					{
						code: "host.attestation_unavailable",
						message: `Host capability attestation failed: ${error instanceof Error ? error.message : String(error)}`,
					},
				],
				{
					steps: scenario.steps.length,
					assertions: scenario.assertions.length,
				},
			);
		}
		const capabilityIssue = attestationIssue(
			attestation,
			attestationStartedAtMs,
			deadlineAtMs,
			options.appId,
			tools,
			requiredCapability,
		);
		if (capabilityIssue) {
			return emptyResult(
				scenario.id,
				options.appId,
				"blocked",
				requiredCapability,
				startedAtMs,
				deadlineAtMs,
				[
					{
						code:
							requiredCapability === "isolated_runtime"
								? "host.runtime_isolation_required"
								: "host.read_capability_required",
						message: capabilityIssue,
					},
				],
				{
					steps: scenario.steps.length,
					assertions: scenario.assertions.length,
				},
			);
		}
		verifiedAttestation = attestation;
		for (const [index, step] of scenario.steps.entries()) {
			if (controller.signal.aborted || Date.now() >= deadlineAtMs) {
				throw new ScenarioAbort(
					scenarioAbortKind(
						timedOut || Date.now() >= deadlineAtMs,
						options.signal,
					),
				);
			}
			let args: ScenarioJsonObject;
			try {
				const resolved = resolveScenarioValue(
					step.arguments,
					appJson,
					targetJson,
					observations,
				) as ScenarioJsonObject;
				args = toolArguments(step.tool, resolved, options.appId, target);
			} catch (error) {
				const message = error instanceof Error ? error.message : String(error);
				stepResults.push({
					step_id: step.id,
					tool: step.tool,
					status: "unknown",
					message,
				});
				return {
					schema: "flowpilot.app-behavior-scenario-result/v1",
					scenario_id: scenario.id,
					app_id: options.appId,
					status: "fail",
					outcome_known: true,
					outstanding: false,
					certification: "none",
					required_capability: requiredCapability,
					started_at_ms: startedAtMs,
					completed_at_ms: Date.now(),
					deadline_at_ms: deadlineAtMs,
					attestation: {
						id: attestation.attestationId,
						mode: attestation.mode,
						...(attestation.mode === "isolated_runtime"
							? { isolation_id: attestation.isolationId }
							: {}),
					},
					steps: stepResults,
					assertions: [],
					metrics: runMetrics(
						scenario.steps.length,
						stepResults,
						scenario.assertions.length,
						[],
						startedRuns,
					),
					started_runs: startedRuns,
					issues: [{ code: "scenario.arguments_invalid", message }],
				};
			}
			const generatedId =
				options.createInvocationId?.(step.id, index) ??
				`${scenario.id}:${step.id}:${startedAtMs}:${index}`;
			if (!generatedId.trim() || invocationIds.has(generatedId)) {
				return {
					schema: "flowpilot.app-behavior-scenario-result/v1",
					scenario_id: scenario.id,
					app_id: options.appId,
					status: "fail",
					outcome_known: true,
					outstanding: false,
					certification: "none",
					required_capability: requiredCapability,
					started_at_ms: startedAtMs,
					completed_at_ms: Date.now(),
					deadline_at_ms: deadlineAtMs,
					steps: stepResults,
					assertions: [],
					metrics: runMetrics(
						scenario.steps.length,
						stepResults,
						scenario.assertions.length,
						[],
						startedRuns,
					),
					started_runs: startedRuns,
					issues: [
						{
							code: "host.invocation_id_invalid",
							message: "The host invocation id is empty or was reused.",
						},
					],
				};
			}
			invocationIds.add(generatedId);
			const invokedAtMs = Date.now();
			let observation: AppScenarioToolObservation;
			try {
				observation = await awaitWithAbort(
					options.adapter.invoke({
						invocationId: generatedId,
						scenarioId: scenario.id,
						stepId: step.id,
						appId: options.appId,
						target,
						tool: step.tool,
						arguments: args,
						...(attestation.mode === "isolated_runtime"
							? { isolationId: attestation.isolationId }
							: {}),
						deadlineAtMs,
						signal: controller.signal,
					}),
					controller.signal,
				);
			} catch (error) {
				if (controller.signal.aborted || error instanceof ScenarioAbort) {
					stepResults.push({
						step_id: step.id,
						tool: step.tool,
						invocation_id: generatedId,
						status: scenarioAbortKind(timedOut, options.signal),
						started_at_ms: invokedAtMs,
						completed_at_ms: Date.now(),
						arguments: args,
					});
					throw new ScenarioAbort(scenarioAbortKind(timedOut, options.signal));
				}
				const message = error instanceof Error ? error.message : String(error);
				stepResults.push({
					step_id: step.id,
					tool: step.tool,
					invocation_id: generatedId,
					status: "unknown",
					started_at_ms: invokedAtMs,
					completed_at_ms: Date.now(),
					arguments: args,
					message,
				});
				return {
					schema: "flowpilot.app-behavior-scenario-result/v1",
					scenario_id: scenario.id,
					app_id: options.appId,
					status: "unknown",
					outcome_known: false,
					outstanding: false,
					certification: "none",
					required_capability: requiredCapability,
					started_at_ms: startedAtMs,
					completed_at_ms: Date.now(),
					deadline_at_ms: deadlineAtMs,
					attestation: {
						id: attestation.attestationId,
						mode: attestation.mode,
						...(attestation.mode === "isolated_runtime"
							? { isolation_id: attestation.isolationId }
							: {}),
					},
					steps: stepResults,
					assertions: [],
					metrics: runMetrics(
						scenario.steps.length,
						stepResults,
						scenario.assertions.length,
						[],
						startedRuns,
					),
					started_runs: startedRuns,
					issues: [
						{
							code: "host.invocation_unknown",
							message: `Tool ${step.tool} outcome is unknown: ${message}`,
						},
					],
				};
			}
			const completedAtMs = Date.now();
			const evidenceIssue = jsonValidationIssue(observation.value);
			const observationIsFresh =
				observation.invocationId === generatedId &&
				Number.isFinite(observation.observedAtMs) &&
				observation.observedAtMs >= invokedAtMs &&
				observation.observedAtMs <= completedAtMs + OBSERVATION_CLOCK_SKEW_MS;
			if (!observationIsFresh || evidenceIssue) {
				const message =
					evidenceIssue ?? "Host evidence is stale or miscorrelated.";
				stepResults.push({
					step_id: step.id,
					tool: step.tool,
					invocation_id: generatedId,
					status: "unknown",
					started_at_ms: invokedAtMs,
					completed_at_ms: completedAtMs,
					arguments: args,
					message,
				});
				return {
					schema: "flowpilot.app-behavior-scenario-result/v1",
					scenario_id: scenario.id,
					app_id: options.appId,
					status: "unknown",
					outcome_known: false,
					outstanding: false,
					certification: "none",
					required_capability: requiredCapability,
					started_at_ms: startedAtMs,
					completed_at_ms: completedAtMs,
					deadline_at_ms: deadlineAtMs,
					attestation: {
						id: attestation.attestationId,
						mode: attestation.mode,
						...(attestation.mode === "isolated_runtime"
							? { isolation_id: attestation.isolationId }
							: {}),
					},
					steps: stepResults,
					assertions: [],
					metrics: runMetrics(
						scenario.steps.length,
						stepResults,
						scenario.assertions.length,
						[],
						startedRuns,
					),
					started_runs: startedRuns,
					issues: [
						{
							code: "host.evidence_not_fresh",
							message,
						},
					],
				};
			}
			const value = observation.value as ScenarioJsonValue;
			let state: ScenarioJsonValue | undefined;
			const runtimeOutcomes = observation.runtimeOutcomes ?? [];
			if (isRuntimeTool(step.tool)) {
				const normalizedIssue = runtimeOutcomeEvidenceIssue(
					observation.runtimeOutcomes,
					invokedAtMs,
					observation.observedAtMs,
					OBSERVATION_CLOCK_SKEW_MS,
				);
				const stateIssue = Object.hasOwn(observation, "state")
					? jsonValidationIssue(observation.state)
					: "Runtime observation has no fresh host-observed domain state.";
				const errorConflict =
					!normalizedIssue &&
					runtimeOutcomes.some((outcome) => outcome.state === "succeeded") &&
					hasScenarioErrorEvidence(value)
						? "Transport payload contains error logs despite a succeeded host outcome."
						: undefined;
				const runtimeEvidenceIssue =
					normalizedIssue ?? stateIssue ?? errorConflict;
				if (!normalizedIssue) {
					const normalizedRuns = scenarioStartedRunsFromHost(
						step.id,
						runtimeOutcomes,
					);
					for (const run of normalizedRuns) {
						if (
							!startedRuns.some(
								(candidate) =>
									candidate.step_id === run.step_id &&
									candidate.run_id === run.run_id,
							)
						) {
							startedRuns.push(run);
						}
					}
				}
				if (runtimeEvidenceIssue) {
					stepResults.push({
						step_id: step.id,
						tool: step.tool,
						invocation_id: generatedId,
						status: "unknown",
						started_at_ms: invokedAtMs,
						completed_at_ms: completedAtMs,
						observed_at_ms: observation.observedAtMs,
						arguments: args,
						value,
						...(Object.hasOwn(observation, "state") && !stateIssue
							? { state: observation.state as ScenarioJsonValue }
							: {}),
						...(!normalizedIssue
							? {
									runtime_outcomes: scenarioStartedRunsFromHost(
										step.id,
										runtimeOutcomes,
									),
								}
							: {}),
						message: runtimeEvidenceIssue,
					});
					return {
						schema: "flowpilot.app-behavior-scenario-result/v1",
						scenario_id: scenario.id,
						app_id: options.appId,
						status: "unknown",
						outcome_known: false,
						outstanding: false,
						certification: "none",
						required_capability: requiredCapability,
						started_at_ms: startedAtMs,
						completed_at_ms: completedAtMs,
						deadline_at_ms: deadlineAtMs,
						attestation: {
							id: attestation.attestationId,
							mode: attestation.mode,
							...(attestation.mode === "isolated_runtime"
								? { isolation_id: attestation.isolationId }
								: {}),
						},
						steps: stepResults,
						assertions: [],
						metrics: runMetrics(
							scenario.steps.length,
							stepResults,
							scenario.assertions.length,
							[],
							startedRuns,
						),
						started_runs: startedRuns,
						issues: [
							{
								code: errorConflict
									? "host.runtime_outcome_conflict"
									: normalizedIssue
										? "host.runtime_outcome_invalid"
										: "host.runtime_state_invalid",
								message: runtimeEvidenceIssue,
							},
						],
					};
				}
				state = observation.state as ScenarioJsonValue;
			}
			observations.set(step.id, value);
			evidence.set(step.id, { value, state, runtimeOutcomes });
			stepResults.push({
				step_id: step.id,
				tool: step.tool,
				invocation_id: generatedId,
				status: "observed",
				started_at_ms: invokedAtMs,
				completed_at_ms: completedAtMs,
				observed_at_ms: observation.observedAtMs,
				arguments: args,
				value,
				...(state === undefined ? {} : { state }),
				...(runtimeOutcomes.length === 0
					? {}
					: {
							runtime_outcomes: scenarioStartedRunsFromHost(
								step.id,
								runtimeOutcomes,
							),
						}),
			});
		}

		const runtimeStepIds = new Set(
			scenario.steps
				.filter((step) => isRuntimeTool(step.tool))
				.map((step) => step.id),
		);
		const assertionResults = scenario.assertions.map((assertion) =>
			assertionResult(assertion, evidence, runtimeStepIds, appJson, targetJson),
		);
		const runtimeOutcomes = [...evidence.values()].flatMap(
			(candidate) => candidate.runtimeOutcomes,
		);
		const hasUnknownRuntimeOutcome = runtimeOutcomes.some(
			(outcome) => outcome.state === "unknown",
		);
		const everyRuntimeRunSucceeded = runtimeOutcomes.every(
			(outcome) =>
				outcome.state === "succeeded" && (outcome.errorCount ?? 0) === 0,
		);
		const passed =
			assertionResults.every((result) => result.status === "pass") &&
			everyRuntimeRunSucceeded;
		return {
			schema: "flowpilot.app-behavior-scenario-result/v1",
			scenario_id: scenario.id,
			app_id: options.appId,
			status: passed ? "pass" : hasUnknownRuntimeOutcome ? "unknown" : "fail",
			outcome_known: !hasUnknownRuntimeOutcome,
			outstanding: false,
			certification: passed
				? requiredCapability === "isolated_runtime"
					? "behavioral"
					: "contract_only"
				: "none",
			required_capability: requiredCapability,
			started_at_ms: startedAtMs,
			completed_at_ms: Date.now(),
			deadline_at_ms: deadlineAtMs,
			attestation: {
				id: attestation.attestationId,
				mode: attestation.mode,
				...(attestation.mode === "isolated_runtime"
					? { isolation_id: attestation.isolationId }
					: {}),
			},
			steps: stepResults,
			assertions: assertionResults,
			metrics: runMetrics(
				scenario.steps.length,
				stepResults,
				scenario.assertions.length,
				assertionResults,
				startedRuns,
			),
			started_runs: startedRuns,
			issues: passed
				? []
				: [
						...assertionResults
							.filter((result) => result.status !== "pass")
							.map((result) => ({
								code: "assertion.failed",
								message: `${result.assertion_id}: ${result.message}`,
								path: result.path,
							})),
						...(!everyRuntimeRunSucceeded &&
						assertionResults.every((result) => result.status === "pass")
							? [
									{
										code: "runtime.non_successful_run",
										message:
											"At least one host-observed started run did not terminate successfully.",
									},
								]
							: []),
					],
		};
	} catch (error) {
		const kind =
			error instanceof ScenarioAbort
				? scenarioAbortKind(
						error.kind === "timed_out" || timedOut,
						options.signal,
					)
				: "unknown";
		const completedAtMs = Date.now();
		return {
			schema: "flowpilot.app-behavior-scenario-result/v1",
			scenario_id: scenario.id,
			app_id: options.appId,
			status: kind,
			outcome_known: false,
			outstanding: false,
			certification: "none",
			required_capability: requiredCapability,
			started_at_ms: startedAtMs,
			completed_at_ms: completedAtMs,
			deadline_at_ms: deadlineAtMs,
			...(verifiedAttestation
				? {
						attestation: {
							id: verifiedAttestation.attestationId,
							mode: verifiedAttestation.mode,
							...(verifiedAttestation.mode === "isolated_runtime"
								? { isolation_id: verifiedAttestation.isolationId }
								: {}),
						},
					}
				: {}),
			steps: stepResults,
			assertions: [],
			metrics: runMetrics(
				scenario.steps.length,
				stepResults,
				scenario.assertions.length,
				[],
				startedRuns,
			),
			started_runs: startedRuns,
			issues: [
				{
					code:
						kind === "timed_out"
							? "scenario.deadline"
							: kind === "cancelled"
								? "scenario.cancelled"
								: "scenario.unknown",
					message: error instanceof Error ? error.message : String(error),
				},
			],
		};
	} finally {
		clearTimeout(timeout);
		options.signal?.removeEventListener("abort", abortFromCaller);
	}
}
