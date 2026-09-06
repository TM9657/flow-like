import { describe, expect, test } from "vitest";
import { createInteractiveScenarioAdapter } from "./interactive-scenario-adapter";

describe("interactive scenario capability boundary", () => {
	test("refuses runtime attestation without starting a workflow", async () => {
		let invoked = false;
		const adapter = createInteractiveScenarioAdapter(
			"app",
			async () => {
				invoked = true;
			},
			() => {},
		);
		await expect(
			adapter.attest({
				appId: "app",
				scenarioId: "verify",
				target: { id: "event", kind: "event" },
				tools: ["call_app_event"],
				requiredCapability: "isolated_runtime",
				deadlineAtMs: Date.now() + 1000,
				signal: new AbortController().signal,
			}),
		).rejects.toThrow("host-enforced isolation");
		expect(invoked).toBe(false);
	});
	test("binds read arguments to the host app and rejects a different app", async () => {
		const calls: Record<string, unknown>[] = [];
		const adapter = createInteractiveScenarioAdapter(
			"app",
			async (_tool, args) => {
				calls.push(args);
				return { status: "ok" };
			},
			() => {},
		);
		const request = {
			invocationId: "invoke",
			scenarioId: "verify",
			stepId: "read",
			appId: "app",
			target: { id: "event", kind: "event" as const },
			tool: "describe_app_interface" as const,
			arguments: { app_id: "other", event_id: "event" },
			deadlineAtMs: Date.now() + 1000,
			signal: new AbortController().signal,
		};
		expect(await adapter.invoke(request)).toMatchObject({
			invocationId: "invoke",
			value: { status: "ok" },
		});
		expect(calls).toEqual([{ app_id: "app", event_id: "event" }]);
		await expect(
			adapter.invoke({ ...request, appId: "other" }),
		).rejects.toThrow("different app");
		expect(calls).toHaveLength(1);
	});
});
