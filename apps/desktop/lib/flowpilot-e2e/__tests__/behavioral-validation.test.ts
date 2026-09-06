import type { AppScenarioRunResult } from "@flow-like/flow-like-ui/lib/app-build/scenarios";
import { describe, expect, test } from "vitest";

import {
	type FlowPilotAppCreationSnapshot,
	type FlowPilotE2EArtifact,
	aggregateFlowPilotE2EBehavioralMetrics,
	evaluateFlowPilotBehavioralEvidence,
	flowPilotE2EArtifactMeetsTier,
	resolveFlowPilotE2ETier,
} from "../index";

function scenarioResult(
	id: string,
	overrides: Partial<AppScenarioRunResult> = {},
): AppScenarioRunResult {
	return {
		schema: "flowpilot.app-behavior-scenario-result/v1",
		scenario_id: id,
		app_id: "app-fixture",
		status: "pass",
		outcome_known: true,
		outstanding: false,
		certification: "behavioral",
		required_capability: "isolated_runtime",
		started_at_ms: 100,
		completed_at_ms: 200,
		deadline_at_ms: 1_000,
		steps: [],
		assertions: [
			{
				assertion_id: `${id}.assertion`,
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
			started_runs: 1,
			successful_runs: 1,
			failed_runs: 0,
			unknown_runs: 0,
		},
		started_runs: [
			{
				step_id: "invoke",
				run_id: `${id}.run`,
				status: "succeeded",
				completed_at_ms: 180,
			},
		],
		issues: [],
		...overrides,
	};
}

function artifact(
	results?: readonly AppScenarioRunResult[],
): FlowPilotE2EArtifact & { snapshot: FlowPilotAppCreationSnapshot } {
	return {
		schema: "flowpilot.app-creation-e2e-artifact/v1",
		generatedAt: "2026-09-06T10:00:00.000Z",
		durationMs: 100,
		requestedModelKey: "terra",
		requestedModel: {
			provider: "codex",
			model: "gpt-5.6-terra",
			reasoningEffort: "high",
		},
		caseId: "simple-agent",
		expectedAppName: "Fixture",
		prompt: "Create fixture",
		snapshot: {
			appId: "app-fixture",
			appName: "Fixture",
			boards: [],
			pages: [],
			widgets: [],
			tables: [],
			events: [],
			behavioralScenarioResults: results,
		},
		runner: { suppressedNavigations: [], issues: [] },
		report: {
			schema: "flowpilot.app-creation-e2e-report/v1",
			caseId: "simple-agent",
			caseTitle: "Fixture",
			appId: "app-fixture",
			appName: "Fixture",
			expectedAppName: "Fixture",
			model: {
				provider: "codex",
				model: "gpt-5.6-terra",
				reasoningEffort: "high",
			},
			evaluationIdentity: {
				harnessContractVersion: "flowpilot.app-creation-e2e-harness/v1",
				caseSuiteFingerprint: "fp1:fixture",
				cohortKey: "fp1:cohort",
				provider: "codex",
				model: "gpt-5.6-terra",
				reasoningEffort: "high",
				tier: "structural",
			},
			passed: true,
			summary: { checks: 1, passed: 1, failed: 0 },
			inventory: {
				boards: 0,
				totalNodes: 0,
				pages: 0,
				widgets: 0,
				tables: 0,
				events: 0,
			},
			flowScript: { canonical: [] },
			checks: [],
			failures: [],
		},
	};
}

describe("FlowPilot E2E behavioral tier", () => {
	test("preserves structural as the default", () => {
		expect(resolveFlowPilotE2ETier(undefined)).toBe("structural");
		expect(flowPilotE2EArtifactMeetsTier(artifact())).toBe(true);
	});

	test("fails closed when behavioral evidence is absent, blocked, or unknown", () => {
		expect(
			evaluateFlowPilotBehavioralEvidence(artifact().snapshot).passed,
		).toBe(false);
		for (const status of ["blocked", "unknown"] as const) {
			const result = scenarioResult("runtime", {
				status,
				outcome_known: false,
				outstanding: status === "blocked",
				certification: "none",
			});
			expect(
				evaluateFlowPilotBehavioralEvidence(artifact([result]).snapshot).passed,
			).toBe(false);
			expect(
				flowPilotE2EArtifactMeetsTier(artifact([result]), "behavioral"),
			).toBe(false);
		}
	});

	test("accepts complete contract and isolated-runtime scenario evidence", () => {
		const contract = scenarioResult("contract", {
			certification: "contract_only",
			required_capability: "read_only",
			metrics: {
				steps_total: 1,
				steps_invoked: 1,
				assertions_total: 1,
				assertions_passed: 1,
				started_runs: 0,
				successful_runs: 0,
				failed_runs: 0,
				unknown_runs: 0,
			},
			started_runs: [],
		});
		const candidate = artifact([contract, scenarioResult("runtime")]);
		const evaluation = evaluateFlowPilotBehavioralEvidence(candidate.snapshot);

		expect(evaluation.passed).toBe(true);
		expect(evaluation.metrics).toMatchObject({
			scenarios: 2,
			passedScenarios: 2,
			startedRuns: 1,
			successfulRuns: 1,
		});
		expect(flowPilotE2EArtifactMeetsTier(candidate, "behavioral")).toBe(true);
	});

	test("aggregates every started run across repeated artifacts", () => {
		const twoRuns = scenarioResult("round_one", {
			metrics: {
				steps_total: 1,
				steps_invoked: 1,
				assertions_total: 1,
				assertions_passed: 1,
				started_runs: 2,
				successful_runs: 1,
				failed_runs: 1,
				unknown_runs: 0,
			},
			started_runs: [
				{ step_id: "invoke", run_id: "run-one", status: "succeeded" },
				{ step_id: "invoke", run_id: "run-two", status: "failed" },
			],
		});
		const metrics = aggregateFlowPilotE2EBehavioralMetrics([
			artifact([twoRuns]),
			artifact([scenarioResult("round_two")]),
		]);

		expect(metrics).toMatchObject({
			scenarios: 2,
			startedRuns: 3,
			successfulRuns: 2,
			failedRuns: 1,
			unknownRuns: 0,
		});
	});
});
