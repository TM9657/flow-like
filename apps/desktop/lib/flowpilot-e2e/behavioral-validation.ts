import type { AppScenarioRunResult } from "@flow-like/flow-like-ui/lib/app-build/scenarios";

import type {
	FlowPilotAppCreationSnapshot,
	FlowPilotE2EArtifact,
	FlowPilotE2EBehavioralMetrics,
	FlowPilotE2ECheck,
	FlowPilotE2ETier,
} from "./types";

const SUCCESSFUL_RUN_STATUS = "succeeded";
const FAILED_RUN_STATUSES = new Set(["failed", "cancelled", "timed_out"]);

export function resolveFlowPilotE2ETier(
	value: FlowPilotE2ETier | string | null | undefined,
): FlowPilotE2ETier {
	if (value === undefined || value === null || value.trim() === "") {
		return "structural";
	}
	const normalized = value.trim().toLowerCase();
	if (normalized === "structural" || normalized === "behavioral") {
		return normalized;
	}
	throw new Error(`Unknown FlowPilot E2E tier: ${value}`);
}

function emptyBehavioralMetrics(): FlowPilotE2EBehavioralMetrics {
	return {
		scenarios: 0,
		passedScenarios: 0,
		failedScenarios: 0,
		blockedScenarios: 0,
		unknownScenarios: 0,
		startedRuns: 0,
		successfulRuns: 0,
		failedRuns: 0,
		unknownRuns: 0,
	};
}

function addScenarioResult(
	metrics: FlowPilotE2EBehavioralMetrics,
	result: AppScenarioRunResult,
): void {
	metrics.scenarios += 1;
	if (result.status === "pass") metrics.passedScenarios += 1;
	else if (result.status === "fail") metrics.failedScenarios += 1;
	else if (result.status === "blocked") metrics.blockedScenarios += 1;
	else metrics.unknownScenarios += 1;

	for (const run of result.started_runs) {
		metrics.startedRuns += 1;
		const status = run.status?.trim().toLowerCase();
		if (status === SUCCESSFUL_RUN_STATUS) metrics.successfulRuns += 1;
		else if (status && FAILED_RUN_STATUSES.has(status)) metrics.failedRuns += 1;
		else metrics.unknownRuns += 1;
	}
}

export function aggregateFlowPilotE2EBehavioralMetrics(
	artifacts: readonly FlowPilotE2EArtifact[],
): FlowPilotE2EBehavioralMetrics {
	const metrics = emptyBehavioralMetrics();
	for (const artifact of artifacts) {
		for (const result of artifact.snapshot?.behavioralScenarioResults ?? []) {
			addScenarioResult(metrics, result);
		}
	}
	return metrics;
}

export interface FlowPilotBehavioralEvaluation {
	readonly passed: boolean;
	readonly checks: readonly FlowPilotE2ECheck[];
	readonly metrics: FlowPilotE2EBehavioralMetrics;
}

function behaviorCheck(
	code: string,
	passed: boolean,
	message: string,
	actual?: string | number | boolean,
): FlowPilotE2ECheck {
	return {
		code,
		status: passed ? "pass" : "fail",
		message,
		...(actual === undefined ? {} : { actual }),
	};
}

/**
 * Evaluate host scenario evidence for the opt-in tier. Structural artifacts remain the default;
 * requesting this tier with no isolated-runtime evidence produces an explicit failure.
 */
export function evaluateFlowPilotBehavioralEvidence(
	snapshot: Pick<
		FlowPilotAppCreationSnapshot,
		"appId" | "behavioralScenarioResults"
	>,
): FlowPilotBehavioralEvaluation {
	const results = snapshot.behavioralScenarioResults ?? [];
	const metrics = emptyBehavioralMetrics();
	for (const result of results) addScenarioResult(metrics, result);

	const ids = results.map((result) => result.scenario_id);
	const uniqueIds = new Set(ids);
	const appScoped = results.every((result) => result.app_id === snapshot.appId);
	const terminalPass = results.every(
		(result) =>
			result.status === "pass" &&
			result.outcome_known &&
			!result.outstanding &&
			(result.certification === "contract_only" ||
				result.certification === "behavioral"),
	);
	const hasBehavioralCertification = results.some(
		(result) => result.certification === "behavioral",
	);
	const assertionsComplete = results.every(
		(result) =>
			result.metrics.assertions_total > 0 &&
			result.assertions.length === result.metrics.assertions_total &&
			result.assertions.every((assertion) => assertion.status === "pass") &&
			result.metrics.assertions_passed === result.metrics.assertions_total,
	);
	const allStartedRunsAccountedFor = results.every(
		(result) =>
			result.started_runs.length === result.metrics.started_runs &&
			result.metrics.started_runs ===
				result.metrics.successful_runs +
					result.metrics.failed_runs +
					result.metrics.unknown_runs,
	);
	const everyStartedRunSucceeded =
		metrics.startedRuns > 0 &&
		metrics.successfulRuns === metrics.startedRuns &&
		metrics.failedRuns === 0 &&
		metrics.unknownRuns === 0;
	const checks = [
		behaviorCheck(
			"behavioral.evidence.present",
			results.length > 0,
			results.length > 0
				? `Captured ${results.length} host scenario result(s).`
				: "Behavioral tier requested, but no host scenario result was captured.",
			results.length,
		),
		behaviorCheck(
			"behavioral.evidence.app_scope",
			results.length > 0 && appScoped,
			appScoped
				? "Every scenario result is scoped to the created app."
				: "A scenario result belongs to another app.",
			appScoped,
		),
		behaviorCheck(
			"behavioral.evidence.unique_scenarios",
			results.length > 0 && uniqueIds.size === results.length,
			uniqueIds.size === results.length
				? "Every scenario has one result."
				: "Behavioral evidence repeats a scenario id.",
			uniqueIds.size,
		),
		behaviorCheck(
			"behavioral.scenarios.terminal_pass",
			results.length > 0 && terminalPass,
			terminalPass
				? "Every scenario has a known passing host outcome."
				: "At least one scenario is failed, blocked, incomplete, or unknown.",
			metrics.passedScenarios,
		),
		behaviorCheck(
			"behavioral.certification.present",
			hasBehavioralCertification,
			hasBehavioralCertification
				? "At least one scenario carries isolated-runtime behavioral certification."
				: "Read-only contract evidence cannot satisfy the behavioral tier.",
			hasBehavioralCertification,
		),
		behaviorCheck(
			"behavioral.assertions.complete",
			results.length > 0 && assertionsComplete,
			assertionsComplete
				? "Every declared scenario assertion passed."
				: "Scenario assertion evidence is absent, incomplete, or failed.",
			assertionsComplete,
		),
		behaviorCheck(
			"behavioral.runs.accounted",
			results.length > 0 && allStartedRunsAccountedFor,
			allStartedRunsAccountedFor
				? "Scenario metrics account for every host-observed started run."
				: "Scenario run metrics do not match the recorded run evidence.",
			metrics.startedRuns,
		),
		behaviorCheck(
			"behavioral.runs.succeeded",
			everyStartedRunSucceeded,
			everyStartedRunSucceeded
				? `All ${metrics.startedRuns} host-observed started run(s) succeeded.`
				: `${metrics.failedRuns} failed and ${metrics.unknownRuns} unknown run outcome(s) remain across ${metrics.startedRuns} started run(s).`,
			metrics.successfulRuns,
		),
	];
	return {
		passed: checks.every((check) => check.status === "pass"),
		checks,
		metrics,
	};
}

export function flowPilotE2EArtifactMeetsTier(
	artifact: FlowPilotE2EArtifact,
	tier: FlowPilotE2ETier = "structural",
): boolean {
	if (artifact.error || artifact.report?.passed !== true) return false;
	if (tier === "structural") return true;
	if (!artifact.snapshot) return false;
	return evaluateFlowPilotBehavioralEvidence(artifact.snapshot).passed;
}
