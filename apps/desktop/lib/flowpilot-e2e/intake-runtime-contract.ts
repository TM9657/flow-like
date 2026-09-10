import type {
	AppBehaviorScenario,
	HostRuntimeOutcome,
} from "@flow-like/flow-like-ui/lib/app-build/scenarios";

export const INTAKE_RUNTIME_POLICY = "flowpilot.intake-local-runtime/v1";

export interface IntakeRuntimeAttestation {
	policy: string;
	isolationId: string;
	appId: string;
	eventId: string;
	pageId: string;
	boardId: string;
	boardHash: string;
	actionIds: string[];
	attestedAtMs: number;
	expiresAtMs: number;
}

export interface NativeIntakeRuntimeOutcome extends HostRuntimeOutcome {
	appId: string;
	eventId: string;
	maxLogLevel: number;
}

export const INTAKE_RUNTIME_CASES = [
	{
		id: "interruption",
		summary: "There is a service interruption",
		queue: "cobalt-response",
		minutes: 17,
	},
	{
		id: "refund",
		summary: "Please refund the duplicate payment",
		queue: "amber-review",
		minutes: 731,
	},
	{
		id: "general",
		summary: "Please change the display name",
		queue: "general-desk",
		minutes: 2880,
	},
	{
		id: "overlap",
		summary: "I need a refund because of this outage",
		queue: "cobalt-response",
		minutes: 17,
	},
	{
		id: "case-insensitive",
		summary: "I CANNOT LOG IN and was CHARGED TWICE",
		queue: "cobalt-response",
		minutes: 17,
	},
] as const;

export function intakeRuntimeScenario(
	fixture: (typeof INTAKE_RUNTIME_CASES)[number],
	nonce: string,
): AppBehaviorScenario {
	const summary = `${fixture.summary} [${nonce}]`;
	return {
		schema: "flowpilot.app-behavior-scenario/v1",
		id: `intake.${fixture.id}`,
		description: `Submit ${fixture.id} through the Page and inspect the resulting local ticket.`,
		requirement_ids: [
			"intake.persistence",
			"intake.routing",
			"intake.rendered-result",
		],
		target: { resource_key: "intake_page", kind: "page" },
		timeout_ms: 90_000,
		steps: [
			{
				id: "submit",
				tool: "interact_app_page",
				arguments: {
					actions: [
						{
							action: "set_value",
							component_id: "summary_input",
							value: summary,
						},
						{
							action: "trigger",
							component_id: "submit_ticket",
							event: "click",
						},
					],
				},
			},
		],
		assertions: [
			{
				id: "run.succeeded",
				kind: "run_outcome",
				step_id: "submit",
				run_id_path: "/runs/0/run_id",
			},
			{
				id: "one.run",
				kind: "json",
				step_id: "submit",
				path: "/run_count",
				operator: "equals",
				expected: 1,
			},
			{
				id: "actions.completed",
				kind: "state",
				step_id: "submit",
				path: "/actionsCompleted",
				operator: "equals",
				expected: true,
			},
			{
				id: "one.new.row",
				kind: "state",
				step_id: "submit",
				path: "/rowCountDelta",
				operator: "equals",
				expected: 1,
			},
			{
				id: "one.match",
				kind: "state",
				step_id: "submit",
				path: "/matchingRows",
				operator: "equals",
				expected: 1,
			},
			{
				id: "summary.preserved",
				kind: "state",
				step_id: "submit",
				path: "/row/summary",
				operator: "equals",
				expected: summary,
			},
			{
				id: "queue.correct",
				kind: "state",
				step_id: "submit",
				path: "/row/queue",
				operator: "equals",
				expected: fixture.queue,
			},
			{
				id: "deadline.correct",
				kind: "state",
				step_id: "submit",
				path: "/row/response_minutes",
				operator: "equals",
				expected: fixture.minutes,
			},
			{
				id: "queue.rendered",
				kind: "state",
				step_id: "submit",
				path: "/renderedQueue",
				operator: "contains",
				expected: fixture.queue,
			},
		],
	};
}

export function requireIntakeRuntimeAttestation(
	value: IntakeRuntimeAttestation,
	scope: { appId: string; eventId: string; pageId: string },
	now = Date.now(),
): void {
	if (
		value.policy !== INTAKE_RUNTIME_POLICY ||
		!value.isolationId ||
		value.appId !== scope.appId ||
		value.eventId !== scope.eventId ||
		value.pageId !== scope.pageId ||
		!value.boardId ||
		!value.boardHash ||
		!value.actionIds.length ||
		value.actionIds.some((id) => !id.startsWith("pa1_")) ||
		value.attestedAtMs > now ||
		value.expiresAtMs <= now
	) {
		throw new Error(
			"Native intake runtime attestation is invalid, stale, or outside this Page.",
		);
	}
}

export function validateIntakeNativeOutcomes(
	outcomes: readonly NativeIntakeRuntimeOutcome[],
	runIds: readonly string[],
	scope: { appId: string; eventId: string },
	startedAtMs: number,
	now = Date.now(),
): HostRuntimeOutcome[] {
	if (
		!runIds.length ||
		new Set(runIds).size !== runIds.length ||
		outcomes.length !== runIds.length
	)
		throw new Error(
			"Every submitted run requires one native terminal outcome.",
		);
	return runIds.map((id) => {
		const matches = outcomes.filter((entry) => entry.sourceRunId === id);
		const outcome = matches[0];
		if (
			matches.length !== 1 ||
			!outcome ||
			outcome.appId !== scope.appId ||
			outcome.eventId !== scope.eventId ||
			outcome.completedAtMs < startedAtMs ||
			outcome.completedAtMs > now ||
			!["succeeded", "failed", "cancelled", "timed_out", "unknown"].includes(
				outcome.state,
			)
		)
			throw new Error(
				"Native outcome is stale, duplicated, or belongs to another invocation.",
			);
		return {
			sourceRunId: id,
			state:
				outcome.state === "succeeded" && outcome.maxLogLevel >= 3
					? "failed"
					: outcome.state,
			completedAtMs: outcome.completedAtMs,
		};
	});
}
