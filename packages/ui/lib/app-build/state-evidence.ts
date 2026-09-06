import {
	scenarioEvidenceSchema,
	structuralEvidenceSchema,
	type AppBuildState,
	type ScenarioEvidence,
	type StructuralEvidence,
} from "./state-contract";
import {
	ensureBuildMutable,
	ensureNoActiveBuildOperation,
	latestAppliedReceipt,
	nextBuildRevision,
} from "./state-internals";

export function recordStructuralEvidence(
	build: AppBuildState,
	evidence: StructuralEvidence,
	options: { readonly now_ms?: number } = {},
): AppBuildState {
	ensureBuildMutable(build);
	ensureNoActiveBuildOperation(build);
	const parsed = structuralEvidenceSchema.parse(evidence);
	const nowMs = options.now_ms ?? Date.now();
	if (build.validation_revision === undefined) {
		throw new Error(
			"Structural evidence requires a complete validation snapshot.",
		);
	}
	if (
		parsed.build_revision !== build.validation_revision ||
		parsed.spec_fingerprint !== build.spec_fingerprint
	) {
		throw new Error(
			"Structural evidence targets a stale build revision or spec.",
		);
	}
	if (
		parsed.recorded_at_ms < build.updated_at_ms ||
		parsed.recorded_at_ms > nowMs
	) {
		throw new Error(
			"Structural evidence predates the current durable state or reports a future timestamp.",
		);
	}
	if (parsed.status === "passed") {
		for (const resource of Object.values(build.resources)) {
			const observedFingerprint =
				latestAppliedReceipt(resource)?.observed_fingerprint;
			if (
				!observedFingerprint ||
				parsed.resource_fingerprints[resource.key] !== observedFingerprint
			) {
				throw new Error(
					`Structural evidence does not attest the latest observed artifact for resource '${resource.key}'.`,
				);
			}
		}
	}
	return nextBuildRevision({ ...build, structural_evidence: parsed }, nowMs);
}

export function recordScenarioEvidence(
	build: AppBuildState,
	evidence: ScenarioEvidence,
	options: { readonly now_ms?: number } = {},
): AppBuildState {
	ensureBuildMutable(build);
	ensureNoActiveBuildOperation(build);
	const parsed = scenarioEvidenceSchema.parse(evidence);
	const nowMs = options.now_ms ?? Date.now();
	const scenario = build.scenarios[parsed.scenario_id];
	if (!scenario) throw new Error(`Unknown scenario '${parsed.scenario_id}'.`);
	if (build.validation_revision === undefined) {
		throw new Error(
			"Scenario evidence requires a complete validation snapshot.",
		);
	}
	if (
		parsed.build_revision !== build.validation_revision ||
		parsed.spec_fingerprint !== build.spec_fingerprint ||
		parsed.scenario_fingerprint !== scenario.desired_fingerprint
	) {
		throw new Error(
			"Scenario evidence targets a stale build, spec, or scenario.",
		);
	}
	if (
		parsed.recorded_at_ms < build.updated_at_ms ||
		parsed.recorded_at_ms > nowMs
	) {
		throw new Error(
			"Scenario evidence predates the current durable state or reports a future timestamp.",
		);
	}
	return nextBuildRevision(
		{
			...build,
			scenarios: {
				...build.scenarios,
				[scenario.id]: {
					...scenario,
					status: parsed.status,
					evidence: parsed,
				},
			},
		},
		nowMs,
	);
}
