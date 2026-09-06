import { expect, test } from "vitest";
import { runAppBehaviorScenario } from "./scenarios";
import { exampleAppSpec } from "./test-fixtures";
import { scenarioReceiptDetails } from "./scenario-receipt";

test("checkpoints retain outcomes without copying raw scenario payloads", async () => {
	const result = await runAppBehaviorScenario(exampleAppSpec().scenarios[0], {
		appId: "app",
		resources: { submit_issue: { id: "event", kind: "event", appId: "app" } },
		adapter: {
			attest: async () => {
				throw new Error("isolation unavailable");
			},
			invoke: async () => {
				throw new Error("must not invoke");
			},
		},
	});
	const large = {
		...result,
		steps: [
			{
				step_id: "submit",
				tool: "call_app_event" as const,
				status: "observed" as const,
				value: { secret: "sensitive-payload".repeat(16_384) },
			},
		],
	};
	const receipt = scenarioReceiptDetails(large);
	expect(receipt.status).toBe("blocked");
	expect(receipt.raw_observations_retained).toBe(false);
	expect(JSON.stringify(receipt)).not.toContain("sensitive-payload");
	expect(JSON.stringify(receipt).length).toBeLessThan(2000);
	expect(
		scenarioReceiptDetails({ ...large, status: "unknown" }).result_fingerprint,
	).not.toBe(receipt.result_fingerprint);
});
