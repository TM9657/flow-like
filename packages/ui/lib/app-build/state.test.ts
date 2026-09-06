import { describe, expect, it } from "vitest";

import {
	beginPromotion,
	createBuild,
	invalidateBuildResources,
	markResourcesApplying,
	reconcileBuild,
	recordPromotionResult,
	recordResourceInspections,
	recordResourceResults,
	recordScenarioEvidence,
	recordStructuralEvidence,
	recoverInterruptedBuild,
	readiness,
	validateBuild,
	type AppBuildState,
} from "./state";
import { appBuildStateSchema } from "./state-contract";
import { exampleAppSpec } from "./test-fixtures";

function applyResource(
	build: AppBuildState,
	key: string,
	observedFingerprint: string,
	nowMs: number,
): AppBuildState {
	const operationId = `operation_${key}_${nowMs}`;
	const applying = markResourcesApplying(build, [key], {
		operation_id: operationId,
		operation_owner_id: "test",
		lease_expires_at_ms: nowMs + 10_000,
		now_ms: nowMs,
	});
	const resource = applying.resources[key];
	if (!resource) throw new Error("missing fixture resource");
	return recordResourceResults(
		applying,
		{
			[key]: {
				receipt_id: `receipt_${key}_${nowMs}`,
				operation_id: operationId,
				status: "applied",
				physical_id: resource.physical_id,
				desired_fingerprint: resource.desired_fingerprint,
				observed_fingerprint: observedFingerprint,
				recorded_at_ms: nowMs + 1,
			},
		},
		{ now_ms: nowMs + 2 },
	);
}

function fullyAppliedBuild(): AppBuildState {
	let build = createBuild(exampleAppSpec(), {
		app_id: "app_1",
		build_id: "build_1",
		original_request: "Build an issue intake that saves submissions.",
		now_ms: 1,
	});
	build = applyResource(build, "issue_flow", "artifact:board:v1", 10);
	build = applyResource(build, "submit_issue", "artifact:event:v1", 20);
	return build;
}

describe("AppBuildState", () => {
	it("accepts distinct desired and observed fingerprints and pins structural evidence to observed artifacts", () => {
		const build = fullyAppliedBuild();
		expect(build.validation_revision).toBe(build.revision);
		expect(validateBuild(build).ok).toBe(true);

		expect(() =>
			recordStructuralEvidence(build, {
				evidence_id: "wrong-domain",
				status: "passed",
				build_revision: build.validation_revision!,
				spec_fingerprint: build.spec_fingerprint,
				resource_fingerprints: Object.fromEntries(
					Object.values(build.resources).map((resource) => [
						resource.key,
						resource.desired_fingerprint,
					]),
				),
				recorded_at_ms: 30,
				issues: [],
			}),
		).toThrow(/latest observed artifact/);

		const validated = recordStructuralEvidence(
			build,
			{
				evidence_id: "structure_1",
				status: "passed",
				build_revision: build.validation_revision!,
				spec_fingerprint: build.spec_fingerprint,
				resource_fingerprints: {
					issue_flow: "artifact:board:v1",
					submit_issue: "artifact:event:v1",
				},
				recorded_at_ms: 30,
				issues: [],
			},
			{ now_ms: 31 },
		);
		expect(readiness(validated).level).toBe("structural_preview");

		const tampered = {
			...validated,
			resources: {
				...validated.resources,
				issue_flow: {
					...validated.resources.issue_flow!,
					receipts: validated.resources.issue_flow!.receipts.map((receipt) => ({
						...receipt,
						observed_fingerprint: "artifact:board:tampered",
					})),
				},
			},
		};
		const tamperValidation = validateBuild(tampered);
		expect(tamperValidation.ok).toBe(false);
		if (!tamperValidation.ok) {
			expect(tamperValidation.issues.map((issue) => issue.code)).toContain(
				"structural_evidence_resource_mismatch",
			);
		}
	});

	it("requires behavioral certification for every requirement before promotion", () => {
		let build = fullyAppliedBuild();
		build = recordStructuralEvidence(
			build,
			{
				evidence_id: "structure_1",
				status: "passed",
				build_revision: build.validation_revision!,
				spec_fingerprint: build.spec_fingerprint,
				resource_fingerprints: {
					issue_flow: "artifact:board:v1",
					submit_issue: "artifact:event:v1",
				},
				recorded_at_ms: 30,
				issues: [],
			},
			{ now_ms: 30 },
		);
		const scenario = build.scenarios.submit_issue_works!;
		build = recordScenarioEvidence(
			build,
			{
				evidence_id: "scenario_contract_only",
				scenario_id: scenario.id,
				status: "passed",
				certification: "contract_only",
				build_revision: build.validation_revision!,
				spec_fingerprint: build.spec_fingerprint,
				scenario_fingerprint: scenario.desired_fingerprint,
				recorded_at_ms: 31,
			},
			{ now_ms: 31 },
		);
		expect(readiness(build).can_promote).toBe(false);
		expect(readiness(build).blockers.map((blocker) => blocker.code)).toContain(
			"scenario_contract_only",
		);

		build = recordScenarioEvidence(
			build,
			{
				evidence_id: "scenario_behavioral",
				scenario_id: scenario.id,
				status: "passed",
				certification: "behavioral",
				build_revision: build.validation_revision!,
				spec_fingerprint: build.spec_fingerprint,
				scenario_fingerprint: scenario.desired_fingerprint,
				recorded_at_ms: 32,
			},
			{ now_ms: 32 },
		);
		expect(readiness(build)).toMatchObject({
			level: "ready",
			behaviorally_ready: true,
			can_promote: true,
		});

		build = beginPromotion(build, {
			operation_id: "promotion_1",
			operation_owner_id: "test",
			lease_expires_at_ms: 10_000,
			now_ms: 40,
		});
		build = recordPromotionResult(
			build,
			{
				receipt_id: "promotion_receipt_1",
				operation_id: "promotion_1",
				status: "applied",
				build_revision: build.promotion.requested_build_revision!,
				spec_fingerprint: build.spec_fingerprint,
				recorded_at_ms: 41,
			},
			{ now_ms: 41 },
		);
		expect(readiness(build).level).toBe("promoted");
		expect(validateBuild(build).ok).toBe(true);
	});

	it("keeps uncertain work fenced until lease expiry and refuses existence without a prior receipt", () => {
		const initial = createBuild(exampleAppSpec(), {
			app_id: "app_1",
			build_id: "build_1",
			now_ms: 1,
		});
		const applying = markResourcesApplying(initial, ["issue_flow"], {
			operation_id: "slow_operation",
			operation_owner_id: "first-controller",
			lease_expires_at_ms: 100,
			now_ms: 10,
		});
		expect(recoverInterruptedBuild(applying, { now_ms: 99 })).toBe(applying);
		const unknown = recoverInterruptedBuild(applying, { now_ms: 100 });
		expect(unknown.resources.issue_flow?.status).toBe("unknown");

		const resource = unknown.resources.issue_flow!;
		const inspected = recordResourceInspections(unknown, {
			issue_flow: {
				inspection_id: "inspection_1",
				status: "matches",
				physical_id: resource.physical_id,
				desired_fingerprint: resource.desired_fingerprint,
				observed_fingerprint: "artifact:exists",
				inspected_at_ms: 101,
			},
		});
		expect(inspected.resources.issue_flow?.status).toBe("unknown");
		expect(inspected.resources.issue_flow?.last_error).toMatch(
			/no committed applied receipt/,
		);
	});

	it("restores a prior applied artifact only after matching readback", () => {
		let build = createBuild(exampleAppSpec(), {
			app_id: "app_1",
			build_id: "build_1",
			now_ms: 1,
		});
		build = applyResource(build, "issue_flow", "artifact:board:v1", 10);
		build = invalidateBuildResources(build, ["issue_flow"], {
			reason: "Verify and repair the board.",
			now_ms: 20,
		});
		build = markResourcesApplying(build, ["issue_flow"], {
			operation_id: "repair_1",
			operation_owner_id: "test",
			lease_expires_at_ms: 30,
			now_ms: 21,
		});
		build = recoverInterruptedBuild(build, { now_ms: 30 });
		const resource = build.resources.issue_flow!;
		build = recordResourceInspections(build, {
			issue_flow: {
				inspection_id: "inspection_1",
				status: "matches",
				physical_id: resource.physical_id,
				desired_fingerprint: resource.desired_fingerprint,
				observed_fingerprint: "artifact:board:v1",
				inspected_at_ms: 31,
			},
		});
		expect(build.resources.issue_flow?.status).toBe("applied");
	});

	it("accepts a fresh matching inspection after an uncertain repair receipt", () => {
		let build = createBuild(exampleAppSpec(), {
			app_id: "app_1",
			build_id: "build_1",
			now_ms: 1,
		});
		build = applyResource(build, "issue_flow", "artifact:board:v1", 10);
		build = invalidateBuildResources(build, ["issue_flow"], {
			reason: "Recheck the board.",
			now_ms: 20,
		});
		build = markResourcesApplying(build, ["issue_flow"], {
			operation_id: "uncertain_repair",
			operation_owner_id: "test",
			lease_expires_at_ms: 100,
			now_ms: 30,
		});
		const resource = build.resources.issue_flow!;
		build = recordResourceResults(
			build,
			{
				issue_flow: {
					receipt_id: "uncertain_receipt",
					operation_id: "uncertain_repair",
					status: "unknown",
					physical_id: resource.physical_id,
					desired_fingerprint: resource.desired_fingerprint,
					recorded_at_ms: 31,
				},
			},
			{ now_ms: 31 },
		);
		expect(build.resources.issue_flow?.receipts.at(-1)?.status).toBe("unknown");
		build = recordResourceInspections(
			build,
			{
				issue_flow: {
					inspection_id: "matching_inspection",
					status: "matches",
					physical_id: resource.physical_id,
					desired_fingerprint: resource.desired_fingerprint,
					observed_fingerprint: "artifact:board:v1",
					inspected_at_ms: 32,
				},
			},
			{ now_ms: 32 },
		);
		expect(build.resources.issue_flow?.status).toBe("applied");
		expect(validateBuild(build).ok).toBe(true);
	});

	it("downgrades host results that arrive after their operation lease", () => {
		let build = createBuild(exampleAppSpec(), {
			app_id: "app_1",
			build_id: "build_1",
			now_ms: 1,
		});
		build = markResourcesApplying(build, ["issue_flow"], {
			operation_id: "expired_apply",
			operation_owner_id: "test",
			lease_expires_at_ms: 20,
			now_ms: 10,
		});
		const resource = build.resources.issue_flow!;
		build = recordResourceResults(
			build,
			{
				issue_flow: {
					receipt_id: "late_receipt",
					operation_id: "expired_apply",
					status: "applied",
					physical_id: resource.physical_id,
					desired_fingerprint: resource.desired_fingerprint,
					observed_fingerprint: "artifact:late",
					recorded_at_ms: 21,
				},
			},
			{ now_ms: 21 },
		);
		expect(build.resources.issue_flow?.status).toBe("unknown");
		expect(build.resources.issue_flow?.receipts.at(-1)?.status).toBe("unknown");
	});

	it("rejects checkpoint payloads that would exceed durable storage", () => {
		const build = fullyAppliedBuild();
		const oversized = {
			...build,
			resources: {
				...build.resources,
				issue_flow: {
					...build.resources.issue_flow!,
					receipts: build.resources.issue_flow!.receipts.map((receipt) => ({
						...receipt,
						details: { payload: "x".repeat(1_000_000) },
					})),
				},
			},
		};
		expect(appBuildStateSchema.safeParse(oversized).success).toBe(false);
	});

	it("invalidates descendants and preserves the host-bound original contract", () => {
		const build = fullyAppliedBuild();
		const repaired = invalidateBuildResources(build, ["issue_flow"], {
			reason: "Board readback drifted.",
			now_ms: 40,
		});
		expect(repaired.resources.issue_flow?.status).toBe("pending");
		expect(repaired.resources.submit_issue?.status).toBe("pending");
		expect(repaired.original_request).toBe(build.original_request);

		const rewritten = exampleAppSpec();
		rewritten.requirements[0] = {
			...rewritten.requirements[0]!,
			description: "A narrower replacement contract.",
		};
		expect(() => reconcileBuild(build, rewritten)).toThrow(
			/cannot remove or rewrite existing requirement/,
		);
		const replacement = exampleAppSpec();
		replacement.requirements = [
			{ id: "replacement", description: "A replacement requirement." },
		];
		replacement.resources = replacement.resources.map((resource) => ({
			...resource,
			requirement_ids: ["replacement"],
		}));
		replacement.scenarios = replacement.scenarios.map((scenario) => ({
			...scenario,
			requirement_ids: ["replacement"],
		}));
		expect(() => reconcileBuild(build, replacement)).toThrow(
			/cannot remove or rewrite existing requirement/,
		);

		const tamperedRequest = {
			...build,
			original_request: "A different request",
		};
		const validation = validateBuild(tamperedRequest);
		expect(validation.ok).toBe(false);
		if (!validation.ok) {
			expect(validation.issues.map((issue) => issue.code)).toContain(
				"original_request_fingerprint_mismatch",
			);
		}
	});
});
