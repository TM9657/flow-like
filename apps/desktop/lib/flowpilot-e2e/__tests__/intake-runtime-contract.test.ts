import { appBehaviorScenarioSchema } from "@flow-like/flow-like-ui/lib/app-build/scenarios";
import { describe, expect, test } from "vitest";
import {
	INTAKE_RUNTIME_CASES,
	INTAKE_RUNTIME_POLICY,
	type IntakeRuntimeAttestation,
	type NativeIntakeRuntimeOutcome,
	intakeRuntimeScenario,
	requireIntakeRuntimeAttestation,
	validateIntakeNativeOutcomes,
} from "../intake-runtime-contract";

const scope = { appId: "app", eventId: "event", pageId: "page" };
const attestation: IntakeRuntimeAttestation = {
	...scope,
	policy: INTAKE_RUNTIME_POLICY,
	isolationId: "native-owned-root",
	boardId: "board",
	boardHash: "hash",
	actionIds: ["pa1_submit"],
	attestedAtMs: 100,
	expiresAtMs: 1000,
};
const outcome: NativeIntakeRuntimeOutcome = {
	appId: "app",
	eventId: "event",
	sourceRunId: "run",
	completedAtMs: 200,
	state: "succeeded",
	maxLogLevel: 1,
};

describe("fixed intake runtime acceptance", () => {
	test("includes native run, persistence, rendered state and overlapping policy assertions", () => {
		expect(INTAKE_RUNTIME_CASES).toHaveLength(5);
		for (const fixture of INTAKE_RUNTIME_CASES) {
			const scenario = intakeRuntimeScenario(fixture, "nonce");
			expect(appBehaviorScenarioSchema.safeParse(scenario).success).toBe(true);
			expect(
				scenario.assertions.some(
					(assertion) => assertion.kind === "run_outcome",
				),
			).toBe(true);
			expect(
				scenario.assertions.some(
					(assertion) =>
						assertion.kind === "state" &&
						assertion.path === "/row/response_minutes" &&
						assertion.expected === fixture.minutes,
				),
			).toBe(true);
		}
		expect(
			INTAKE_RUNTIME_CASES.find((fixture) => fixture.id === "overlap"),
		).toMatchObject({ queue: "cobalt-response", minutes: 17 });
	});

	test("rejects isolation claims with the wrong policy, scope or expiry", () => {
		expect(() =>
			requireIntakeRuntimeAttestation(attestation, scope, 200),
		).not.toThrow();
		for (const patch of [
			{ policy: "browser" },
			{ isolationId: "" },
			{ appId: "other" },
			{ eventId: "other" },
			{ pageId: "other" },
			{ boardHash: "" },
			{ actionIds: ["raw-node"] },
			{ attestedAtMs: 201 },
			{ expiresAtMs: 200 },
		]) {
			expect(() =>
				requireIntakeRuntimeAttestation(
					{ ...attestation, ...patch },
					scope,
					200,
				),
			).toThrow();
		}
	});

	test("requires fresh native outcome for every distinct started run", () => {
		expect(
			validateIntakeNativeOutcomes([outcome], ["run"], scope, 150, 250),
		).toEqual([{ sourceRunId: "run", state: "succeeded", completedAtMs: 200 }]);
		for (const [outcomes, ids] of [
			[[], ["run"]],
			[[outcome], []],
			[[outcome], ["run", "run"]],
			[[outcome, outcome], ["run"]],
			[[outcome], ["other"]],
		] as [NativeIntakeRuntimeOutcome[], string[]][]) {
			expect(() =>
				validateIntakeNativeOutcomes(outcomes, ids, scope, 150, 250),
			).toThrow();
		}
		for (const patch of [
			{ appId: "other" },
			{ eventId: "other" },
			{ completedAtMs: 149 },
			{ completedAtMs: 251 },
		]) {
			expect(() =>
				validateIntakeNativeOutcomes(
					[{ ...outcome, ...patch }],
					["run"],
					scope,
					150,
					250,
				),
			).toThrow();
		}
	});

	test("never promotes failed, cancelled, unknown or error-logged native runs", () => {
		for (const state of [
			"failed",
			"cancelled",
			"unknown",
			"timed_out",
		] as const) {
			expect(
				validateIntakeNativeOutcomes(
					[{ ...outcome, state }],
					["run"],
					scope,
					150,
					250,
				)[0].state,
			).toBe(state);
		}
		expect(
			validateIntakeNativeOutcomes(
				[{ ...outcome, maxLogLevel: 3 }],
				["run"],
				scope,
				150,
				250,
			)[0].state,
		).toBe("failed");
	});
});
