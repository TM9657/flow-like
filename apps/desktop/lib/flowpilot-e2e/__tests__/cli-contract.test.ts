import type { AppScenarioRunResult } from "@flow-like/flow-like-ui/lib/app-build/scenarios";
import { describe, expect, test } from "vitest";

import {
	type FlowPilotE2EArtifact,
	type FlowPilotE2ECaseId,
	type FlowPilotE2ECliEnvelope,
	createFlowPilotE2EEvaluationIdentity,
	flowPilotE2ECliExitCode,
	flowPilotE2EModel,
	getFlowPilotAppCreationCase,
	isFlowPilotE2ECliEnvelope,
	normalizeFlowPilotE2ECliEnvelope,
} from "../index";

const runId = "e2e_contract_fixture";
const caseIds: FlowPilotE2ECaseId[] = ["forum", "simple-agent"];
const structuralIdentity = createFlowPilotE2EEvaluationIdentity(
	flowPilotE2EModel("terra"),
	"structural",
	caseIds.map(getFlowPilotAppCreationCase),
);
const behavioralIdentity = createFlowPilotE2EEvaluationIdentity(
	flowPilotE2EModel("terra"),
	"behavioral",
	caseIds.map(getFlowPilotAppCreationCase),
);
const solStructuralIdentity = createFlowPilotE2EEvaluationIdentity(
	flowPilotE2EModel("sol"),
	"structural",
	caseIds.map(getFlowPilotAppCreationCase),
);

function artifact(
	caseId: FlowPilotE2ECaseId,
	passed: boolean,
	behavioralRuns?: number,
): FlowPilotE2EArtifact {
	const appId = `app-${caseId}`;
	const behavioralResult: AppScenarioRunResult | undefined =
		behavioralRuns === undefined
			? undefined
			: {
					schema: "flowpilot.app-behavior-scenario-result/v1",
					scenario_id: `${caseId}.runtime`,
					app_id: appId,
					status: "pass",
					outcome_known: true,
					outstanding: false,
					certification: "behavioral",
					required_capability: "isolated_runtime",
					started_at_ms: 1,
					completed_at_ms: 2,
					deadline_at_ms: 10,
					steps: [],
					assertions: [
						{
							assertion_id: "domain_state",
							step_id: "invoke",
							status: "pass",
							message: "passed",
						},
					],
					metrics: {
						steps_total: 1,
						steps_invoked: 1,
						assertions_total: 1,
						assertions_passed: 1,
						started_runs: behavioralRuns,
						successful_runs: behavioralRuns,
						failed_runs: 0,
						unknown_runs: 0,
					},
					started_runs: Array.from({ length: behavioralRuns }, (_, index) => ({
						step_id: "invoke",
						run_id: `${caseId}.run.${index}`,
						status: "succeeded",
					})),
					issues: [],
				};
	return {
		schema: "flowpilot.app-creation-e2e-artifact/v1",
		generatedAt: "2026-07-22T10:00:00.000Z",
		durationMs: 10,
		requestedModelKey: "terra",
		requestedModel: {
			provider: "codex",
			model: "gpt-5.6-terra",
			reasoningEffort: "high",
		},
		...(behavioralResult ? { requestedTier: "behavioral" as const } : {}),
		caseId,
		expectedAppName: `Fixture ${caseId}`,
		prompt: `Create ${caseId}`,
		runner: { suppressedNavigations: [], issues: [] },
		...(behavioralResult
			? {
					snapshot: {
						appId,
						appName: `Fixture ${caseId}`,
						boards: [],
						pages: [],
						widgets: [],
						tables: [],
						events: [],
						behavioralScenarioResults: [behavioralResult],
					},
				}
			: {}),
		report: {
			schema: "flowpilot.app-creation-e2e-report/v1",
			caseId,
			caseTitle: caseId,
			appId,
			appName: `Fixture ${caseId}`,
			expectedAppName: `Fixture ${caseId}`,
			model: {
				provider: "codex",
				model: "gpt-5.6-terra",
				reasoningEffort: "high",
			},
			evaluationIdentity: behavioralResult
				? behavioralIdentity
				: structuralIdentity,
			passed,
			summary: { checks: 1, passed: passed ? 1 : 0, failed: passed ? 0 : 1 },
			inventory: {
				boards: 1,
				totalNodes: 1,
				pages: 1,
				widgets: 1,
				tables: 1,
				events: 1,
			},
			flowScript: { canonical: [] },
			checks: [],
			failures: [],
		},
	};
}

function envelope(
	artifacts: FlowPilotE2EArtifact[],
	overrides: Partial<FlowPilotE2ECliEnvelope> = {},
): FlowPilotE2ECliEnvelope {
	return {
		schema: "flowpilot.app-creation-e2e-cli-result/v1",
		runId,
		startedAt: "2026-07-22T10:00:00.000Z",
		completedAt: "2026-07-22T10:00:01.000Z",
		durationMs: 1_000,
		selection: {
			caseIds,
			modelKey: "terra",
			tier: "structural",
			evaluationIdentity: structuralIdentity,
			repeat: 1,
			failFast: false,
		},
		artifacts,
		passed: true,
		summary: {
			requestedRuns: 999,
			completedRuns: 999,
			passed: 999,
			failed: 0,
			skipped: 0,
		},
		...overrides,
	};
}

const expectation = {
	runId,
	caseIds,
	modelKey: "terra",
	repeat: 1,
	minFlowScriptNonWhitespaceChars: undefined,
	failFast: false,
} as const;

describe("FlowPilot E2E CLI callback contract", () => {
	test("recomputes a webview summary and refuses a false green", () => {
		const normalized = normalizeFlowPilotE2ECliEnvelope(
			envelope([artifact("forum", true), artifact("simple-agent", false)]),
			expectation,
		);

		expect(normalized.passed).toBe(false);
		expect(normalized.summary).toEqual({
			requestedRuns: 2,
			completedRuns: 2,
			passed: 1,
			failed: 1,
			skipped: 0,
		});
		expect(flowPilotE2ECliExitCode(normalized)).toBe(1);
	});

	test("accepts an intentional fail-fast prefix only after a failed artifact", () => {
		const failFastExpectation = { ...expectation, failFast: true };
		const normalized = normalizeFlowPilotE2ECliEnvelope(
			envelope([artifact("forum", false)], {
				selection: {
					caseIds,
					modelKey: "terra",
					tier: "structural",
					evaluationIdentity: structuralIdentity,
					repeat: 1,
					failFast: true,
				},
			}),
			failFastExpectation,
		);

		expect(normalized.summary.skipped).toBe(1);
		expect(flowPilotE2ECliExitCode(normalized)).toBe(1);
		expect(() =>
			normalizeFlowPilotE2ECliEnvelope(
				envelope([artifact("forum", true)], {
					selection: {
						caseIds,
						modelKey: "terra",
						tier: "structural",
						evaluationIdentity: structuralIdentity,
						repeat: 1,
						failFast: true,
					},
				}),
				failFastExpectation,
			),
		).toThrow("before all requested runs completed");
		expect(() =>
			normalizeFlowPilotE2ECliEnvelope(
				envelope([artifact("forum", false), artifact("simple-agent", true)], {
					selection: {
						caseIds,
						modelKey: "terra",
						tier: "structural",
						evaluationIdentity: structuralIdentity,
						repeat: 1,
						failFast: true,
					},
				}),
				failFastExpectation,
			),
		).toThrow("after a fail-fast failure");
	});

	test("preserves partial evidence while classifying runner errors as infrastructure", () => {
		const normalized = normalizeFlowPilotE2ECliEnvelope(
			envelope([artifact("forum", true)], { error: "collector crashed" }),
			expectation,
		);

		expect(normalized.artifacts).toHaveLength(1);
		expect(normalized.summary.skipped).toBe(1);
		expect(flowPilotE2ECliExitCode(normalized)).toBe(2);
	});

	test("rejects stale selections, wrong order, and malformed envelopes", () => {
		expect(() =>
			normalizeFlowPilotE2ECliEnvelope(
				envelope([artifact("forum", true), artifact("simple-agent", true)], {
					selection: {
						caseIds: ["forum"],
						modelKey: "terra",
						repeat: 1,
						failFast: false,
					},
				}),
				expectation,
			),
		).toThrow("selection does not match");
		expect(() =>
			normalizeFlowPilotE2ECliEnvelope(
				envelope([artifact("simple-agent", true), artifact("forum", true)]),
				expectation,
			),
		).toThrow("expected forum");
		expect(isFlowPilotE2ECliEnvelope({ runId }, runId)).toBe(false);
	});

	test("rejects callback and report scores from another evaluation cohort", () => {
		expect(() =>
			normalizeFlowPilotE2ECliEnvelope(
				envelope([artifact("forum", true), artifact("simple-agent", true)], {
					selection: {
						caseIds,
						modelKey: "terra",
						tier: "structural",
						evaluationIdentity: {
							...structuralIdentity,
							cohortKey: "fp1:another-cohort",
						},
						repeat: 1,
						failFast: false,
					},
				}),
				expectation,
			),
		).toThrow("different evaluation cohorts");

		const wrongReport = artifact("forum", true);
		if (!wrongReport.report) throw new Error("fixture report is missing");
		wrongReport.report = {
			...wrongReport.report,
			evaluationIdentity: behavioralIdentity,
		};
		expect(() =>
			normalizeFlowPilotE2ECliEnvelope(
				envelope([wrongReport, artifact("simple-agent", true)]),
				expectation,
			),
		).toThrow("different evaluation cohorts");
	});

	test("refuses a run that benchmarked another model than requested", () => {
		const solExpectation = { ...expectation, modelKey: "sol" } as const;
		expect(() =>
			normalizeFlowPilotE2ECliEnvelope(
				envelope([artifact("forum", true), artifact("simple-agent", true)]),
				solExpectation,
			),
		).toThrow("selection does not match");
		expect(() =>
			normalizeFlowPilotE2ECliEnvelope(
				envelope([artifact("forum", true), artifact("simple-agent", true)], {
					selection: {
						caseIds,
						modelKey: "sol",
						tier: "structural",
						evaluationIdentity: solStructuralIdentity,
						repeat: 1,
						failFast: false,
					},
				}),
				solExpectation,
			),
		).toThrow("requested model terra; expected sol");
	});

	test("validates repeated case ordering", () => {
		const repeatedExpectation = { ...expectation, repeat: 2 };
		const normalized = normalizeFlowPilotE2ECliEnvelope(
			envelope(
				[
					artifact("forum", true),
					artifact("simple-agent", true),
					artifact("forum", true),
					artifact("simple-agent", true),
				],
				{
					selection: {
						caseIds,
						modelKey: "terra",
						tier: "structural",
						evaluationIdentity: structuralIdentity,
						repeat: 2,
						failFast: false,
					},
				},
			),
			repeatedExpectation,
		);

		expect(normalized.passed).toBe(true);
		expect(normalized.summary.requestedRuns).toBe(4);
		expect(flowPilotE2ECliExitCode(normalized)).toBe(0);
	});

	test("aggregates every behavioral started run across repeats", () => {
		const repeatedExpectation = {
			...expectation,
			repeat: 2,
			tier: "behavioral" as const,
		};
		const normalized = normalizeFlowPilotE2ECliEnvelope(
			envelope(
				[
					artifact("forum", true, 1),
					artifact("simple-agent", true, 2),
					artifact("forum", true, 3),
					artifact("simple-agent", true, 4),
				],
				{
					selection: {
						caseIds,
						modelKey: "terra",
						tier: "behavioral",
						evaluationIdentity: behavioralIdentity,
						repeat: 2,
						failFast: false,
					},
				},
			),
			repeatedExpectation,
		);

		expect(normalized.passed).toBe(true);
		expect(normalized.summary.behavioral).toMatchObject({
			scenarios: 4,
			startedRuns: 10,
			successfulRuns: 10,
			failedRuns: 0,
			unknownRuns: 0,
		});
	});
});
