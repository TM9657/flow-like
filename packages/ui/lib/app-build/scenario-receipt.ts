import type { JsonObject } from "./contract";
import { appBuildFingerprint } from "./fingerprint";
import type { AppScenarioRunResult } from "./scenarios";

/** Keep checkpoints bounded and avoid copying app payloads into the build journal. */
export function scenarioReceiptDetails(
	result: AppScenarioRunResult,
): JsonObject {
	return {
		schema: "flowpilot.app-scenario-receipt/v1",
		scenario_id: result.scenario_id,
		status: result.status,
		certification: result.certification,
		started_at_ms: result.started_at_ms,
		completed_at_ms: result.completed_at_ms,
		metrics: { ...result.metrics },
		// Digests link this compact receipt to a full result retained by a caller or trace exporter.
		// They detect changes; they are not signatures or independent proof of runtime isolation.
		result_fingerprint: appBuildFingerprint("scenario-result", result),
		attestation_id: result.attestation?.id ?? null,
		raw_observations_retained: false,
		issue_count: result.issues.length,
		issues: result.issues
			.slice(0, 3)
			.map((issue) => ({
				code: issue.code.slice(0, 100),
				message: issue.message.slice(0, 240),
			})),
	};
}
