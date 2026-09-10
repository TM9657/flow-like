import {
	aggregateFlowPilotE2EBehavioralMetrics,
	flowPilotE2EArtifactMeetsTier,
	resolveFlowPilotE2ETier,
} from "./behavioral-validation";
import {
	flowPilotE2EModel,
	getFlowPilotAppCreationCase,
	isFlowPilotE2EModelKey,
} from "./cases";
import {
	assertFlowPilotE2ECohortsComparable,
	createFlowPilotE2EEvaluationIdentity,
} from "./evaluation-identity";
import { summarizeIntakeReliability } from "./reliability-metrics";
import {
	compareRetrievalArtifacts,
	retrievalPairOrder,
} from "./retrieval-comparison";
import type {
	FlowPilotE2EArtifact,
	FlowPilotE2ECaseId,
	FlowPilotE2ECliEnvelope,
	FlowPilotE2EEvaluationIdentity,
	FlowPilotE2EModelKey,
	FlowPilotE2ETier,
} from "./types";

export interface FlowPilotE2ECliExpectation {
	runtimeSourceFingerprint?: string;
	runId: string;
	retrievalComparison?: boolean;
	retrievalRuntimeFingerprint?: string;
	caseIds: readonly FlowPilotE2ECaseId[];
	modelKey: FlowPilotE2EModelKey;
	tier?: FlowPilotE2ETier;
	evaluationIdentity?: FlowPilotE2EEvaluationIdentity;
	repeat: number;
	minFlowScriptNonWhitespaceChars?: number;
	failFast: boolean;
}

function isRecord(value: unknown): value is Record<string, unknown> {
	return Boolean(value) && typeof value === "object" && !Array.isArray(value);
}

function isNonNegativeInteger(value: unknown): value is number {
	return Number.isSafeInteger(value) && Number(value) >= 0;
}

function isEvaluationIdentity(
	value: unknown,
): value is FlowPilotE2EEvaluationIdentity {
	return (
		isRecord(value) &&
		typeof value.harnessContractVersion === "string" &&
		typeof value.caseSuiteFingerprint === "string" &&
		typeof value.cohortKey === "string" &&
		typeof value.provider === "string" &&
		typeof value.model === "string" &&
		typeof value.reasoningEffort === "string" &&
		(value.tier === "structural" || value.tier === "behavioral")
	);
}

export function isArtifact(value: unknown): value is FlowPilotE2EArtifact {
	if (!isRecord(value)) return false;
	if (
		value.schema !== "flowpilot.app-creation-e2e-artifact/v1" ||
		typeof value.caseId !== "string" ||
		!isFlowPilotE2EModelKey(value.requestedModelKey) ||
		typeof value.expectedAppName !== "string" ||
		typeof value.prompt !== "string" ||
		typeof value.generatedAt !== "string" ||
		typeof value.durationMs !== "number" ||
		!isRecord(value.runner) ||
		!Array.isArray(value.runner.suppressedNavigations) ||
		!Array.isArray(value.runner.issues)
	) {
		return false;
	}
	if (value.error !== undefined && typeof value.error !== "string")
		return false;
	if (
		value.requestedTier !== undefined &&
		value.requestedTier !== "structural" &&
		value.requestedTier !== "behavioral"
	) {
		return false;
	}
	if (value.report === undefined) return true;
	return (
		isRecord(value.report) &&
		value.report.schema === "flowpilot.app-creation-e2e-report/v1" &&
		typeof value.report.passed === "boolean" &&
		isEvaluationIdentity(value.report.evaluationIdentity) &&
		Array.isArray(value.report.checks) &&
		Array.isArray(value.report.failures)
	);
}

/**
 * Structural check performed before the controller trusts or prints a webview callback.
 * The nonce authenticates the sender; this guard catches stale runner/controller contracts.
 */
export function isFlowPilotE2ECliEnvelope(
	value: unknown,
	runId: string,
): value is FlowPilotE2ECliEnvelope {
	if (!isRecord(value)) return false;
	if (
		value.schema !== "flowpilot.app-creation-e2e-cli-result/v1" ||
		value.runId !== runId ||
		typeof value.startedAt !== "string" ||
		typeof value.completedAt !== "string" ||
		typeof value.durationMs !== "number" ||
		typeof value.passed !== "boolean" ||
		!isRecord(value.selection) ||
		!Array.isArray(value.selection.caseIds) ||
		!value.selection.caseIds.every((caseId) => typeof caseId === "string") ||
		!isFlowPilotE2EModelKey(value.selection.modelKey) ||
		(value.selection.tier !== undefined &&
			value.selection.tier !== "structural" &&
			value.selection.tier !== "behavioral") ||
		!isEvaluationIdentity(value.selection.evaluationIdentity) ||
		!isNonNegativeInteger(value.selection.repeat) ||
		typeof value.selection.failFast !== "boolean" ||
		!Array.isArray(value.artifacts) ||
		!value.artifacts.every(isArtifact) ||
		!isRecord(value.summary) ||
		!isNonNegativeInteger(value.summary.requestedRuns) ||
		!isNonNegativeInteger(value.summary.completedRuns) ||
		!isNonNegativeInteger(value.summary.passed) ||
		!isNonNegativeInteger(value.summary.failed) ||
		!isNonNegativeInteger(value.summary.skipped)
	) {
		return false;
	}
	if (
		value.selection.minFlowScriptNonWhitespaceChars !== undefined &&
		!isNonNegativeInteger(value.selection.minFlowScriptNonWhitespaceChars)
	) {
		return false;
	}
	return value.error === undefined || typeof value.error === "string";
}

export function flowPilotE2EArtifactPassed(
	artifact: FlowPilotE2EArtifact,
	tier: FlowPilotE2ETier = artifact.requestedTier ?? "structural",
): boolean {
	return flowPilotE2EArtifactMeetsTier(artifact, tier);
}

function sameCaseIds(
	actual: readonly FlowPilotE2ECaseId[],
	expected: readonly FlowPilotE2ECaseId[],
): boolean {
	return (
		actual.length === expected.length &&
		actual.every((caseId, index) => caseId === expected[index])
	);
}

/** Revalidates selection/order and derives every acceptance field on the CLI side. */
export function normalizeFlowPilotE2ECliEnvelope(
	envelope: FlowPilotE2ECliEnvelope,
	expected: FlowPilotE2ECliExpectation,
): FlowPilotE2ECliEnvelope {
	const expectedTier = resolveFlowPilotE2ETier(expected.tier);
	const actualTier = resolveFlowPilotE2ETier(envelope.selection.tier);
	const expectedIdentity =
		expected.evaluationIdentity ??
		createFlowPilotE2EEvaluationIdentity(
			flowPilotE2EModel(expected.modelKey),
			expectedTier,
			expected.caseIds.map(getFlowPilotAppCreationCase),
			expected.minFlowScriptNonWhitespaceChars,
		);
	if (envelope.runId !== expected.runId) {
		throw new Error(
			"FlowPilot E2E callback run id does not match the controller run.",
		);
	}
	if (
		envelope.selection.runtimeSourceFingerprint !==
			expected.runtimeSourceFingerprint ||
		Boolean(envelope.selection.retrievalComparison) !==
			Boolean(expected.retrievalComparison) ||
		envelope.selection.retrievalRuntimeFingerprint !==
			expected.retrievalRuntimeFingerprint ||
		!sameCaseIds(envelope.selection.caseIds, expected.caseIds) ||
		envelope.selection.modelKey !== expected.modelKey ||
		actualTier !== expectedTier ||
		envelope.selection.repeat !== expected.repeat ||
		envelope.selection.minFlowScriptNonWhitespaceChars !==
			expected.minFlowScriptNonWhitespaceChars ||
		envelope.selection.failFast !== expected.failFast
	) {
		throw new Error(
			"FlowPilot E2E callback selection does not match the request.",
		);
	}
	if (!envelope.selection.evaluationIdentity) {
		throw new Error(
			"FlowPilot E2E callback has no evaluation cohort identity.",
		);
	}
	assertFlowPilotE2ECohortsComparable(
		envelope.selection.evaluationIdentity,
		expectedIdentity,
	);

	const expectedOrder = Array.from({ length: expected.repeat }, () =>
		expected.caseIds.flatMap((id) =>
			expected.retrievalComparison ? [id, id] : [id],
		),
	).flat();
	if (envelope.artifacts.length > expectedOrder.length) {
		throw new Error(
			"FlowPilot E2E callback returned more artifacts than requested.",
		);
	}
	for (const [index, artifact] of envelope.artifacts.entries()) {
		if (
			artifact.caseId === "intake-reliability" &&
			(artifact.reliability?.scope !== "host_provisioned_intake" ||
				artifact.reliability.runtimeSourceFingerprint !==
					expected.runtimeSourceFingerprint)
		) {
			throw new Error(
				`Reliability artifact ${index + 1} has a mismatched runtime identity.`,
			);
		}
		if (artifact.caseId !== expectedOrder[index]) {
			throw new Error(
				`FlowPilot E2E artifact ${index + 1} is for ${artifact.caseId}; expected ${expectedOrder[index]}.`,
			);
		}
		if (expected.retrievalComparison) {
			const evidence = artifact.retrievalComparison;
			const round = Math.floor(index / (expected.caseIds.length * 2));
			const order = index % 2;
			if (
				!evidence ||
				evidence.mode !== retrievalPairOrder(round)[order] ||
				evidence.order !== order ||
				evidence.round !== round ||
				evidence.runtimeFingerprint !== expected.retrievalRuntimeFingerprint
			) {
				throw new Error(
					`Retrieval artifact ${index + 1} has a mismatched mode, order, or runtime fingerprint.`,
				);
			}
		}
		if (artifact.requestedModelKey !== expected.modelKey) {
			throw new Error(
				`FlowPilot E2E artifact ${index + 1} requested model ${artifact.requestedModelKey}; expected ${expected.modelKey}.`,
			);
		}
		if (resolveFlowPilotE2ETier(artifact.requestedTier) !== expectedTier) {
			throw new Error(
				`FlowPilot E2E artifact ${index + 1} requested tier ${resolveFlowPilotE2ETier(artifact.requestedTier)}; expected ${expectedTier}.`,
			);
		}
		if (artifact.report) {
			assertFlowPilotE2ECohortsComparable(
				artifact.report.evaluationIdentity,
				expectedIdentity,
			);
		}
	}

	let comparisonError: string | undefined;
	let retrievalComparisonResults:
		| ReturnType<typeof compareRetrievalArtifacts>
		| undefined;
	if (
		expected.retrievalComparison &&
		envelope.artifacts.length === expectedOrder.length
	) {
		try {
			retrievalComparisonResults = compareRetrievalArtifacts(
				envelope.artifacts,
			);
		} catch (error) {
			comparisonError = error instanceof Error ? error.message : String(error);
		}
	}
	// Preserve the completed raw artifacts even when source/model invariance invalidates comparison.
	const error = envelope.error?.trim() || comparisonError || undefined;
	const passedRuns = envelope.artifacts.filter((artifact) =>
		flowPilotE2EArtifactPassed(artifact, expectedTier),
	).length;
	const requestedRuns = expectedOrder.length;
	const firstFailedArtifact = envelope.artifacts.findIndex(
		(artifact) => !flowPilotE2EArtifactPassed(artifact, expectedTier),
	);
	if (
		expected.failFast &&
		firstFailedArtifact >= 0 &&
		firstFailedArtifact !== envelope.artifacts.length - 1
	) {
		throw new Error(
			"FlowPilot E2E callback returned artifacts after a fail-fast failure.",
		);
	}
	const shortRun = envelope.artifacts.length < requestedRuns;
	if (shortRun && !error) {
		const lastArtifact = envelope.artifacts.at(-1);
		if (
			!expected.failFast ||
			!lastArtifact ||
			flowPilotE2EArtifactPassed(lastArtifact, expectedTier)
		) {
			throw new Error(
				"FlowPilot E2E callback ended before all requested runs completed.",
			);
		}
	}

	const passed =
		!error &&
		requestedRuns > 0 &&
		envelope.artifacts.length === requestedRuns &&
		passedRuns === requestedRuns;
	return {
		...envelope,
		reliabilitySummary: expected.caseIds.includes("intake-reliability")
			? summarizeIntakeReliability(envelope.artifacts)
			: undefined,
		retrievalComparisonResults,
		selection: {
			runtimeSourceFingerprint: expected.runtimeSourceFingerprint,
			...(expected.retrievalComparison
				? {
						retrievalComparison: true,
						retrievalRuntimeFingerprint: expected.retrievalRuntimeFingerprint,
					}
				: {}),
			caseIds: [...expected.caseIds],
			modelKey: expected.modelKey,
			tier: expectedTier,
			evaluationIdentity: expectedIdentity,
			repeat: expected.repeat,
			minFlowScriptNonWhitespaceChars: expected.minFlowScriptNonWhitespaceChars,
			failFast: expected.failFast,
		},
		passed,
		summary: {
			requestedRuns,
			completedRuns: envelope.artifacts.length,
			passed: passedRuns,
			failed: envelope.artifacts.length - passedRuns,
			skipped: requestedRuns - envelope.artifacts.length,
			...(expectedTier === "behavioral"
				? {
						behavioral: aggregateFlowPilotE2EBehavioralMetrics(
							envelope.artifacts,
						),
					}
				: {}),
		},
		error,
	};
}

export function flowPilotE2ECliExitCode(
	envelope: FlowPilotE2ECliEnvelope,
): 0 | 1 | 2 {
	if (envelope.error) return 2;
	return envelope.passed ? 0 : 1;
}
