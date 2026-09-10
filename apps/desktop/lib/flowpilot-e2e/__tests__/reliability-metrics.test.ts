import { describe, expect, test } from "vitest";
import { summarizeIntakeReliability } from "../reliability-metrics";
import type { FlowPilotE2EArtifact } from "../types";

describe("intake reliability measurements", () => {
	test("keeps overlapping provider windows separate and counts observed board followups once", () => {
		const events = [
			{
				id: "board1",
				kind: "tool",
				stage: "tool_end",
				name: "flowpilot_board",
			},
			{
				id: "board2",
				kind: "tool",
				stage: "tool_end",
				name: "flowpilot_board",
			},
			{
				id: "board2",
				kind: "tool",
				stage: "tool_end",
				name: "flowpilot_board",
			},
			{
				id: "parent",
				kind: "tool",
				stage: "tool_end",
				name: "codex",
				started_at_ms: 0,
				ended_at_ms: 100,
				duration_ms: 100,
			},
			{
				id: "nested",
				kind: "nested",
				stage: "tool_end",
				name: "codex",
				started_at_ms: 10,
				ended_at_ms: 90,
				duration_ms: 80,
			},
		];
		const artifact = {
			caseId: "intake-reliability",
			expectedAppName: "app",
			durationMs: 110,
			assistantTrace: { debugReport: { events } },
			reliability: {
				scope: "host_provisioned_intake",
				stages: [
					{
						phase: "registration",
						startedAtMs: 100,
						endedAtMs: 110,
						status: "error",
					},
				],
			},
		} as FlowPilotE2EArtifact;
		const result = summarizeIntakeReliability([artifact]);
		expect(result.passedRuns).toBe(0);
		expect(result.runs[0]).toMatchObject({
			boardDelegations: 2,
			boardFollowupDelegations: 1,
			failureStage: "registration",
		});
		expect(
			result.runs[0].providerWindows.map((window) => window.durationMs),
		).toEqual([100, 80]);
	});
});
