import { describe, expect, test } from "vitest";

import {
	FLOWPILOT_E2E_HARNESS_CONTRACT_VERSION,
	assertFlowPilotE2ECohortsComparable,
	createFlowPilotE2EEvaluationIdentity,
	flowPilotE2ECaseSuiteFingerprint,
	flowPilotE2EModel,
	getFlowPilotAppCreationCase,
} from "../index";

const cases = [
	getFlowPilotAppCreationCase("forum"),
	getFlowPilotAppCreationCase("simple-agent"),
];

describe("FlowPilot E2E evaluation identity", () => {
	test("pins the declared harness, exact model tuple, tier, and fixture suite", () => {
		const first = createFlowPilotE2EEvaluationIdentity(
			flowPilotE2EModel("terra"),
			"structural",
			cases,
		);
		const second = createFlowPilotE2EEvaluationIdentity(
			flowPilotE2EModel("terra"),
			"structural",
			cases,
		);

		expect(first).toEqual(second);
		expect(first).toMatchObject({
			harnessContractVersion: FLOWPILOT_E2E_HARNESS_CONTRACT_VERSION,
			provider: "codex",
			model: "gpt-5.6-terra",
			reasoningEffort: "high",
			tier: "structural",
		});
		expect(first.caseSuiteFingerprint.startsWith("fp1:")).toBe(true);
		expect(first.cohortKey.startsWith("fp1:")).toBe(true);
		expect(() =>
			assertFlowPilotE2ECohortsComparable(first, second),
		).not.toThrow();
	});

	test("starts a different cohort when any comparison dimension changes", () => {
		const baseline = createFlowPilotE2EEvaluationIdentity(
			flowPilotE2EModel("terra"),
			"structural",
			cases,
		);
		const alternatives = [
			createFlowPilotE2EEvaluationIdentity(
				flowPilotE2EModel("sol"),
				"structural",
				cases,
			),
			createFlowPilotE2EEvaluationIdentity(
				flowPilotE2EModel("terra"),
				"behavioral",
				cases,
			),
			createFlowPilotE2EEvaluationIdentity(
				flowPilotE2EModel("terra"),
				"structural",
				[...cases].reverse(),
			),
			createFlowPilotE2EEvaluationIdentity(
				flowPilotE2EModel("terra"),
				"structural",
				cases,
				1_001,
			),
		];

		for (const alternative of alternatives) {
			expect(alternative.cohortKey).not.toBe(baseline.cohortKey);
			expect(() =>
				assertFlowPilotE2ECohortsComparable(baseline, alternative),
			).toThrow("different evaluation cohorts");
		}
	});

	test("fingerprints evaluated fixture content rather than generated run names", () => {
		const first = flowPilotE2ECaseSuiteFingerprint(cases);
		const equivalent = flowPilotE2ECaseSuiteFingerprint(
			cases.map((caseDefinition) => ({
				...caseDefinition,
				expectedAppName: "ignored generated name",
			})),
		);
		expect(equivalent).toBe(first);
	});
});
