import { describe, expect, test, vi } from "vitest";

import {
	APP_BEHAVIOR_SCENARIO_SCHEMA,
	type AppBehaviorScenario,
	type AppScenarioAttestationRequest,
	type AppScenarioHostAdapter,
	type AppScenarioToolInvocationRequest,
	appBehaviorScenarioSchema,
	runAppBehaviorScenario,
} from "./scenarios";

function runtimeScenario(
	overrides: Partial<AppBehaviorScenario> = {},
): AppBehaviorScenario {
	return {
		schema: APP_BEHAVIOR_SCENARIO_SCHEMA,
		id: "create_record",
		description: "Create one record and observe the new workflow run.",
		requirement_ids: ["records.create"],
		target: { resource_key: "create_record_event", kind: "event" },
		timeout_ms: 1_000,
		steps: [
			{
				id: "create",
				tool: "call_app_event",
				arguments: { payload: { title: "One" } },
			},
		],
		assertions: [
			{
				id: "record_created",
				kind: "state",
				step_id: "create",
				path: "/record/title",
				operator: "equals",
				expected: "One",
			},
			{
				id: "run_succeeded",
				kind: "run_outcome",
				step_id: "create",
				run_id_path: "/run_id",
			},
		],
		...overrides,
	};
}

function readOnlyScenario(): AppBehaviorScenario {
	return {
		schema: APP_BEHAVIOR_SCENARIO_SCHEMA,
		id: "interface_contract",
		description: "Read the exact app interface contract.",
		requirement_ids: ["records.contract"],
		target: { resource_key: "create_record_event", kind: "event" },
		timeout_ms: 1_000,
		steps: [
			{
				id: "describe",
				tool: "describe_app_interface",
				arguments: {},
			},
		],
		assertions: [
			{
				id: "description_succeeded",
				kind: "json",
				step_id: "describe",
				path: "/status",
				operator: "equals",
				expected: "ok",
			},
		],
	};
}

function resources() {
	return {
		create_record_event: {
			id: "event-create",
			kind: "event" as const,
			appId: "app-fixture",
		},
	};
}

function isolatedAttestation(request: AppScenarioAttestationRequest) {
	return {
		source: "host" as const,
		mode: "isolated_runtime" as const,
		attestationId: "attestation-fixture",
		appId: request.appId,
		tools: request.tools,
		attestedAtMs: Date.now(),
		expiresAtMs: request.deadlineAtMs + 1_000,
		isolationId: "ephemeral-fixture",
		strategy: "ephemeral_app" as const,
	};
}

function readOnlyAttestation(request: AppScenarioAttestationRequest) {
	return {
		source: "host" as const,
		mode: "read_only" as const,
		attestationId: "read-fixture",
		appId: request.appId,
		tools: request.tools,
		attestedAtMs: Date.now(),
		expiresAtMs: request.deadlineAtMs + 1_000,
	};
}

describe("App behavior scenario contract", () => {
	test("accepts only bounded, ordered scenarios with assertions", () => {
		expect(appBehaviorScenarioSchema.safeParse(runtimeScenario()).success).toBe(
			true,
		);
		expect(
			appBehaviorScenarioSchema.safeParse(runtimeScenario({ assertions: [] }))
				.success,
		).toBe(false);
		expect(
			appBehaviorScenarioSchema.safeParse({
				...runtimeScenario(),
				safe_to_run: true,
			}).success,
		).toBe(false);
	});

	test("rejects unknown tools, forward references, and JSONPath expressions", () => {
		const base = runtimeScenario();
		expect(
			appBehaviorScenarioSchema.safeParse({
				...base,
				steps: [{ ...base.steps[0], tool: "fetch" }],
			}).success,
		).toBe(false);
		expect(
			appBehaviorScenarioSchema.safeParse({
				...base,
				steps: [
					{
						...base.steps[0],
						arguments: {
							payload: {
								$scenario_ref: {
									source: "step",
									step_id: "later",
									path: "/value",
								},
							},
						},
					},
					{
						id: "later",
						tool: "query_execution_logs",
						arguments: {},
					},
				],
			}).success,
		).toBe(false);
		expect(
			appBehaviorScenarioSchema.safeParse({
				...base,
				assertions: [{ ...base.assertions[0], path: "$.status" }],
			}).success,
		).toBe(false);
		expect(
			appBehaviorScenarioSchema.safeParse({
				...base,
				assertions: [
					base.assertions[0],
					{ ...base.assertions[1], status_path: "/status" },
				],
			}).success,
		).toBe(false);
	});
});

describe("runAppBehaviorScenario", () => {
	test("never invokes a runtime tool without host-attested isolation", async () => {
		const invoke = vi.fn();
		const adapter: AppScenarioHostAdapter = {
			attest: async (request) => readOnlyAttestation(request),
			invoke,
		};

		const result = await runAppBehaviorScenario(runtimeScenario(), {
			appId: "app-fixture",
			resources: resources(),
			adapter,
		});

		expect(result.status).toBe("blocked");
		expect(result.outstanding).toBe(true);
		expect(result.certification).toBe("none");
		expect(result.issues.map(({ code }) => code)).toContain(
			"host.runtime_isolation_required",
		);
		expect(invoke).not.toHaveBeenCalled();
	});

	test("does not trust model-authored safety flags", async () => {
		const adapter: AppScenarioHostAdapter = {
			attest: vi.fn(),
			invoke: vi.fn(),
		};
		const result = await runAppBehaviorScenario(
			{ ...runtimeScenario(), safe_to_run: true },
			{
				appId: "app-fixture",
				resources: resources(),
				adapter,
			},
		);

		expect(result.status).toBe("fail");
		expect(adapter.attest).not.toHaveBeenCalled();
		expect(adapter.invoke).not.toHaveBeenCalled();
	});

	test("correlates fresh evidence, injects the exact target, and counts every started run", async () => {
		const invoke = vi.fn(async (request: AppScenarioToolInvocationRequest) => ({
			invocationId: request.invocationId,
			observedAtMs: Date.now(),
			runtimeOutcomes: [
				{
					sourceRunId: "run-primary",
					state: "succeeded" as const,
					completedAtMs: Date.now(),
					errorCount: 0,
				},
				{
					sourceRunId: "run-secondary",
					state: "failed" as const,
					completedAtMs: Date.now(),
					errorCount: 1,
				},
			],
			state: { record: { title: "One" } },
			value: {
				status: "ok",
				run_id: "run-primary",
				runs: [
					{ run_id: "run-primary", status: "ok" },
					{ run_id: "run-secondary", status: "failed" },
				],
			},
		}));
		const adapter: AppScenarioHostAdapter = {
			attest: async (request) => isolatedAttestation(request),
			invoke,
		};

		const result = await runAppBehaviorScenario(runtimeScenario(), {
			appId: "app-fixture",
			resources: resources(),
			adapter,
			createInvocationId: () => "invocation-fresh",
		});

		expect(result.status).toBe("fail");
		expect(result.certification).toBe("none");
		expect(result.metrics).toMatchObject({
			steps_invoked: 1,
			assertions_passed: 2,
			started_runs: 2,
			successful_runs: 1,
			failed_runs: 1,
		});
		expect(invoke).toHaveBeenCalledWith(
			expect.objectContaining({
				appId: "app-fixture",
				tool: "call_app_event",
				isolationId: "ephemeral-fixture",
				arguments: {
					app_id: "app-fixture",
					event_id: "event-create",
					payload: { title: "One" },
				},
			}),
		);
	});

	test("does not treat transport success as workflow success", async () => {
		const adapter: AppScenarioHostAdapter = {
			attest: async (request) => isolatedAttestation(request),
			invoke: async (request) => ({
				invocationId: request.invocationId,
				observedAtMs: Date.now(),
				value: { status: "ok", run_id: "transport-only" },
				state: { record: { title: "One" } },
			}),
		};

		const result = await runAppBehaviorScenario(runtimeScenario(), {
			appId: "app-fixture",
			resources: resources(),
			adapter,
		});

		expect(result.status).toBe("unknown");
		expect(result.certification).toBe("none");
		expect(result.issues.map(({ code }) => code)).toContain(
			"host.runtime_outcome_invalid",
		);
	});

	test("certifies only a matching host-terminal success plus fresh domain state", async () => {
		const adapter: AppScenarioHostAdapter = {
			attest: async (request) => isolatedAttestation(request),
			invoke: async (request) => ({
				invocationId: request.invocationId,
				observedAtMs: Date.now(),
				runtimeOutcomes: [
					{
						sourceRunId: "run-primary",
						state: "succeeded",
						completedAtMs: Date.now(),
						errorCount: 0,
					},
				],
				state: { record: { title: "One" } },
				value: { status: "ok", run_id: "run-primary" },
			}),
		};
		const result = await runAppBehaviorScenario(runtimeScenario(), {
			appId: "app-fixture",
			resources: resources(),
			adapter,
		});

		expect(result.status).toBe("pass");
		expect(result.certification).toBe("behavioral");
		expect(result.metrics).toMatchObject({
			started_runs: 1,
			successful_runs: 1,
			unknown_runs: 0,
		});
	});

	test("does not accept terminal evidence for a different source run", async () => {
		const adapter: AppScenarioHostAdapter = {
			attest: async (request) => isolatedAttestation(request),
			invoke: async (request) => ({
				invocationId: request.invocationId,
				observedAtMs: Date.now(),
				runtimeOutcomes: [
					{
						sourceRunId: "another-run",
						state: "succeeded",
						completedAtMs: Date.now(),
					},
				],
				state: { record: { title: "One" } },
				value: { status: "ok", run_id: "claimed-run" },
			}),
		};
		const result = await runAppBehaviorScenario(runtimeScenario(), {
			appId: "app-fixture",
			resources: resources(),
			adapter,
		});

		expect(result.status).toBe("fail");
		expect(result.certification).toBe("none");
		expect(result.assertions).toEqual(
			expect.arrayContaining([
				expect.objectContaining({
					assertion_id: "run_succeeded",
					status: "fail",
				}),
			]),
		);
	});

	test("requires terminal success with no backend error evidence", async () => {
		for (const evidence of [
			{
				state: "failed" as const,
				errorCount: 1,
				logs: [{ level: "error", message: "write failed" }],
			},
			{
				state: "succeeded" as const,
				errorCount: 0,
				logs: [{ log_level: 4, message: "backend error" }],
			},
		] as const) {
			const adapter: AppScenarioHostAdapter = {
				attest: async (request) => isolatedAttestation(request),
				invoke: async (request) => ({
					invocationId: request.invocationId,
					observedAtMs: Date.now(),
					runtimeOutcomes: [
						{
							sourceRunId: "run-primary",
							state: evidence.state,
							completedAtMs: Date.now(),
							errorCount: evidence.errorCount,
						},
					],
					state: { record: { title: "One" } },
					value: {
						status: "ok",
						run_id: "run-primary",
						logs: evidence.logs,
					},
				}),
			};
			const result = await runAppBehaviorScenario(runtimeScenario(), {
				appId: "app-fixture",
				resources: resources(),
				adapter,
			});

			expect(result.status).not.toBe("pass");
			expect(result.certification).toBe("none");
		}
	});

	test("does not certify a runtime step without a domain state assertion", async () => {
		const scenario = runtimeScenario({
			assertions: [runtimeScenario().assertions[1]],
		});
		const adapter: AppScenarioHostAdapter = {
			attest: vi.fn(),
			invoke: vi.fn(),
		};
		const result = await runAppBehaviorScenario(scenario, {
			appId: "app-fixture",
			resources: resources(),
			adapter,
		});

		expect(result.status).toBe("fail");
		expect(result.issues.map(({ code }) => code)).toContain(
			"scenario.runtime_step_unasserted",
		);
		expect(adapter.attest).not.toHaveBeenCalled();
	});

	test("does not allow stale or miscorrelated evidence to pass", async () => {
		for (const stale of ["timestamp", "invocation"] as const) {
			const adapter: AppScenarioHostAdapter = {
				attest: async (request) => isolatedAttestation(request),
				invoke: async (request) => ({
					invocationId:
						stale === "invocation" ? "old-invocation" : request.invocationId,
					observedAtMs: stale === "timestamp" ? 1 : Date.now(),
					value: { status: "ok", run_id: "old-run" },
				}),
			};
			const result = await runAppBehaviorScenario(runtimeScenario(), {
				appId: "app-fixture",
				resources: resources(),
				adapter,
			});

			expect(result.status, stale).toBe("unknown");
			expect(result.certification, stale).toBe("none");
			expect(result.assertions, stale).toHaveLength(0);
		}
	});

	test("honors cancellation before attestation and while a tool is in flight", async () => {
		const preCancelled = new AbortController();
		preCancelled.abort();
		const untouched: AppScenarioHostAdapter = {
			attest: vi.fn(),
			invoke: vi.fn(),
		};
		const before = await runAppBehaviorScenario(readOnlyScenario(), {
			appId: "app-fixture",
			resources: resources(),
			adapter: untouched,
			signal: preCancelled.signal,
		});
		expect(before.status).toBe("cancelled");
		expect(untouched.attest).not.toHaveBeenCalled();

		const inFlight = new AbortController();
		const adapter: AppScenarioHostAdapter = {
			attest: async (request) => readOnlyAttestation(request),
			invoke: (request) =>
				new Promise((_resolve, reject) => {
					inFlight.abort();
					request.signal.addEventListener("abort", () =>
						reject(request.signal.reason),
					);
				}),
		};
		const during = await runAppBehaviorScenario(readOnlyScenario(), {
			appId: "app-fixture",
			resources: resources(),
			adapter,
			signal: inFlight.signal,
		});
		expect(during.status).toBe("cancelled");
		expect(during.certification).toBe("none");
	});

	test("enforces the scenario deadline even when the adapter never settles", async () => {
		let hostSignal: AbortSignal | undefined;
		const adapter: AppScenarioHostAdapter = {
			attest: async (request) => readOnlyAttestation(request),
			invoke: (request) => {
				hostSignal = request.signal;
				return new Promise(() => undefined);
			},
		};
		const result = await runAppBehaviorScenario(
			{ ...readOnlyScenario(), timeout_ms: 100 },
			{
				appId: "app-fixture",
				resources: resources(),
				adapter,
			},
		);

		expect(result.status).toBe("timed_out");
		expect(result.outcome_known).toBe(false);
		expect(hostSignal?.aborted).toBe(true);
	});

	test("marks read-only checks as contract evidence, never behavioral certification", async () => {
		const adapter: AppScenarioHostAdapter = {
			attest: async (request) => readOnlyAttestation(request),
			invoke: async (request) => ({
				invocationId: request.invocationId,
				observedAtMs: Date.now(),
				value: { status: "ok", input_schema: { type: "object" } },
			}),
		};
		const result = await runAppBehaviorScenario(readOnlyScenario(), {
			appId: "app-fixture",
			resources: resources(),
			adapter,
		});

		expect(result.status).toBe("pass");
		expect(result.certification).toBe("contract_only");
		expect(result.metrics.started_runs).toBe(0);
	});
});
