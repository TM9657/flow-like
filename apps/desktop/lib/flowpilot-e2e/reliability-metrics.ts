import { flowPilotE2EArtifactMeetsTier } from "./behavioral-validation";
import type { FlowPilotE2EArtifact } from "./types";

function record(value: unknown): Record<string, unknown> {
	return value && typeof value === "object" && !Array.isArray(value)
		? (value as Record<string, unknown>)
		: {};
}

/** Windows overlap; retain them individually instead of adding provider and host time. */
export function summarizeIntakeReliability(
	artifacts: readonly FlowPilotE2EArtifact[],
) {
	const selected = artifacts.filter(
		(artifact) => artifact.caseId === "intake-reliability",
	);
	const runs = selected.map((artifact) => {
		const trace = record(artifact.assistantTrace?.debugReport);
		const events = Array.isArray(trace.events) ? trace.events.map(record) : [];
		const unique = [
			...new Map(
				events
					.filter((event) => typeof event.id === "string")
					.map((event) => [event.id, event]),
			).values(),
		];
		const boardCalls = unique.filter(
			(event) =>
				event.stage === "tool_end" &&
				event.name === "flowpilot_board" &&
				event.kind === "tool",
		);
		const passed = flowPilotE2EArtifactMeetsTier(artifact, "behavioral");
		const firstFailedStage = artifact.reliability?.stages.find(
			(stage) => stage.status === "error",
		)?.phase;
		return {
			appId: artifact.snapshot?.appId ?? null,
			appName: artifact.expectedAppName,
			passed,
			failureStage: passed
				? null
				: (firstFailedStage ?? (artifact.report ? "acceptance" : "runner")),
			elapsedMs: artifact.durationMs,
			generationAttempts: artifact.flowScriptGenerationRuns?.length ?? 0,
			workflowRepairRuns: Math.max(
				0,
				(artifact.flowScriptGenerationRuns?.length ?? 0) - 1,
			),
			boardDelegations: boardCalls.length,
			// Includes read-only board inspections. Generation receipts count actual build/repair runs.
			boardFollowupDelegations: Math.max(0, boardCalls.length - 1),
			providerWindows: unique
				.filter((event) => event.stage === "tool_end" && event.name === "codex")
				.map((event) => ({
					id: event.id,
					parentRequestId: event.parent_request_id ?? null,
					startedAtMs: event.started_at_ms,
					endedAtMs: event.ended_at_ms,
					durationMs: event.duration_ms,
				})),
			stages: artifact.reliability?.stages ?? [],
			failedChecks: artifact.report?.failures.map((check) => check.code) ?? [],
		};
	});
	return {
		scope: "host_provisioned_intake" as const,
		completedRuns: runs.length,
		passedRuns: runs.filter((run) => run.passed).length,
		runs,
	};
}
