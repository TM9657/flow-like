import { appBuildFingerprint } from "@flow-like/flow-like-ui/lib/app-build/fingerprint";

import type {
	FlowPilotE2ECaseDefinition,
	FlowPilotE2EEvaluationIdentity,
	FlowPilotE2EModelConfig,
	FlowPilotE2ETier,
} from "./types";

/**
 * Declares the evaluator contract and fixture identity format. This is a comparison boundary,
 * not source/runtime attestation and not evidence that thresholds were recalibrated.
 */
export const FLOWPILOT_E2E_HARNESS_CONTRACT_VERSION =
	"flowpilot.app-creation-e2e-harness/v3" as const;

function evaluatedFixture(caseDefinition: FlowPilotE2ECaseDefinition) {
	return {
		id: caseDefinition.id,
		title: caseDefinition.title,
		description: caseDefinition.description,
		app_name: caseDefinition.appName,
		prompt: caseDefinition.prompt,
		smoke: caseDefinition.smoke,
		run_timeout_ms: caseDefinition.runTimeoutMs,
		requirements: caseDefinition.requirements,
	};
}

function withMinimumOverride(
	caseDefinition: FlowPilotE2ECaseDefinition,
	minimum: number | undefined,
): FlowPilotE2ECaseDefinition {
	if (minimum === undefined) return caseDefinition;
	if (
		!Number.isSafeInteger(minimum) ||
		minimum < 1 ||
		minimum > caseDefinition.requirements.maxFlowScriptNonWhitespaceChars
	) {
		throw new Error(
			`Invalid FlowScript minimum ${minimum} for evaluation case ${caseDefinition.id}.`,
		);
	}
	return {
		...caseDefinition,
		requirements: {
			...caseDefinition.requirements,
			minFlowScriptNonWhitespaceChars: minimum,
		},
	};
}

/** Order is significant because the CLI evaluates and reports cases in that order. */
export function flowPilotE2ECaseSuiteFingerprint(
	caseDefinitions: readonly FlowPilotE2ECaseDefinition[],
	minimumFlowScriptNonWhitespaceChars?: number,
): string {
	if (caseDefinitions.length === 0) {
		throw new Error(
			"An evaluation cohort requires at least one declared case fixture.",
		);
	}
	return appBuildFingerprint(
		"flowpilot-e2e-case-suite",
		caseDefinitions.map((caseDefinition) =>
			evaluatedFixture(
				withMinimumOverride(
					caseDefinition,
					minimumFlowScriptNonWhitespaceChars,
				),
			),
		),
	);
}

export function createFlowPilotE2EEvaluationIdentity(
	model: FlowPilotE2EModelConfig,
	tier: FlowPilotE2ETier,
	caseDefinitions: readonly FlowPilotE2ECaseDefinition[],
	minimumFlowScriptNonWhitespaceChars?: number,
): FlowPilotE2EEvaluationIdentity {
	const caseSuiteFingerprint = flowPilotE2ECaseSuiteFingerprint(
		caseDefinitions,
		minimumFlowScriptNonWhitespaceChars,
	);
	const tuple = {
		harness_contract_version: FLOWPILOT_E2E_HARNESS_CONTRACT_VERSION,
		case_suite_fingerprint: caseSuiteFingerprint,
		provider: model.provider,
		model: model.model,
		reasoning_effort: model.reasoningEffort,
		tier,
	};
	return {
		harnessContractVersion: FLOWPILOT_E2E_HARNESS_CONTRACT_VERSION,
		caseSuiteFingerprint,
		cohortKey: appBuildFingerprint("flowpilot-e2e-cohort", tuple),
		provider: model.provider,
		model: model.model,
		reasoningEffort: model.reasoningEffort,
		tier,
	};
}

/** Refuse score comparisons when any declared evaluator, fixture, or model dimension differs. */
export function assertFlowPilotE2ECohortsComparable(
	left: FlowPilotE2EEvaluationIdentity,
	right: FlowPilotE2EEvaluationIdentity,
): void {
	const comparable =
		left.cohortKey === right.cohortKey &&
		left.harnessContractVersion === right.harnessContractVersion &&
		left.caseSuiteFingerprint === right.caseSuiteFingerprint &&
		left.provider === right.provider &&
		left.model === right.model &&
		left.reasoningEffort === right.reasoningEffort &&
		left.tier === right.tier;
	if (!comparable) {
		throw new Error(
			`FlowPilot E2E scores belong to different evaluation cohorts (${left.cohortKey} vs ${right.cohortKey}). Establish a new baseline instead of comparing them directly.`,
		);
	}
}
