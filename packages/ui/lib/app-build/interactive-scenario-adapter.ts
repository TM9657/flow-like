import { createId } from "@paralleldrive/cuid2";
import type { AppScenarioHostAdapter } from "./scenarios";

export const INTERACTIVE_APP_BUILD_CAPABILITIES = {
	rollout: "opt_in_preview",
	durable_staging: true,
	structural_verification: true,
	read_only_scenarios: true,
	runtime_verification_available: false,
	automatic_activation_available: false,
	limitation:
		"This interactive host has no attested isolated runtime adapter. Behavioral certification and promotion remain blocked; read-only checks provide contract evidence only.",
} as const;

const READ_ONLY_TOOLS = new Set([
	"describe_app_interface",
	"query_execution_logs",
]);

/** Native workflow nodes are not isolated here. Only app-scoped inspection is available. */
export function createInteractiveScenarioAdapter(
	appId: string,
	delegate: (tool: string, args: Record<string, unknown>) => Promise<unknown>,
	assertActive: () => void,
): AppScenarioHostAdapter {
	const assertAllowed = (requestedApp: string, tools: readonly string[]) => {
		assertActive();
		if (requestedApp !== appId)
			throw new Error("Scenario targets a different app.");
		if (tools.some((tool) => !READ_ONLY_TOOLS.has(tool))) {
			throw new Error("Runtime verification requires host-enforced isolation.");
		}
	};
	return {
		async attest(request) {
			request.signal.throwIfAborted();
			assertAllowed(request.appId, request.tools);
			if (request.requiredCapability !== "read_only")
				throw new Error("Runtime isolation is unavailable.");
			return {
				source: "host",
				mode: "read_only",
				attestationId: createId(),
				appId,
				tools: request.tools,
				attestedAtMs: Date.now(),
				expiresAtMs: request.deadlineAtMs,
			};
		},
		async invoke(request) {
			request.signal.throwIfAborted();
			assertAllowed(request.appId, [request.tool]);
			const value = await delegate(request.tool, {
				...request.arguments,
				app_id: appId,
			});
			request.signal.throwIfAborted();
			assertActive();
			return {
				invocationId: request.invocationId,
				observedAtMs: Date.now(),
				value,
			};
		},
	};
}
