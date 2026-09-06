import { describe, expect, it } from "vitest";

import { advanceBuild, promoteStagedBuild } from "./engine";
import type {
	AppBuildHostAdapter,
	AppBuildHostContext,
	AppBuildHostResourceContext,
	HostResourceInspection,
} from "./engine-types";
import {
	createBuild,
	markResourcesApplying,
	recordResourceResults,
	recordScenarioEvidence,
	recordStructuralEvidence,
	type AppBuildState,
	type HostResourceResult,
} from "./state";
import type { AppBuildRef, AppBuildStore } from "./store";
import { exampleAppSpec } from "./test-fixtures";

class MemoryBuildStore implements AppBuildStore {
	constructor(public value: AppBuildState | null = null) {}

	async read(ref: AppBuildRef): Promise<AppBuildState | null> {
		if (!this.value) return null;
		if (
			this.value.app_id !== ref.app_id ||
			this.value.build_id !== ref.build_id
		) {
			return null;
		}
		return this.value;
	}

	async create(build: AppBuildState): Promise<void> {
		if (this.value) throw new Error("conflict");
		this.value = build;
	}

	async compareAndSwap(
		ref: AppBuildRef,
		expectedRevision: number,
		next: AppBuildState,
	): Promise<void> {
		if (
			!this.value ||
			this.value.app_id !== ref.app_id ||
			this.value.build_id !== ref.build_id ||
			this.value.revision !== expectedRevision
		) {
			throw new Error("conflict");
		}
		this.value = next;
	}
}

const ref = { app_id: "app_1", build_id: "build_1" } as const;

function adapterWith(
	overrides: Partial<AppBuildHostAdapter>,
): AppBuildHostAdapter {
	return {
		async inspectResource(context): Promise<HostResourceInspection> {
			const state = context.build.resources[context.resource.key]!;
			return {
				inspection_id: "default_inspection",
				status: "missing",
				physical_id: state.physical_id,
				desired_fingerprint: state.desired_fingerprint,
				inspected_at_ms: context.build.updated_at_ms,
				message: "missing",
			};
		},
		async applyResource(context): Promise<HostResourceResult> {
			const state = context.build.resources[context.resource.key]!;
			return {
				receipt_id: `receipt_${context.resource.key}`,
				operation_id: state.active_operation!.operation_id,
				status: "applied",
				physical_id: state.physical_id,
				desired_fingerprint: state.desired_fingerprint,
				observed_fingerprint: `artifact:${context.resource.key}:v1`,
				recorded_at_ms: state.active_operation!.started_at_ms,
			};
		},
		async validateStructure(context) {
			return {
				evidence_id: "default_structure",
				status: "passed",
				build_revision: context.build.validation_revision!,
				spec_fingerprint: context.build.spec_fingerprint,
				resource_fingerprints: Object.fromEntries(
					Object.values(context.build.resources).map((resource) => [
						resource.key,
						resource.receipts.at(-1)!.observed_fingerprint!,
					]),
				),
				recorded_at_ms: context.build.updated_at_ms,
				issues: [],
			};
		},
		async promote(context) {
			return {
				receipt_id: "default_promotion",
				operation_id: context.build.promotion.active_operation!.operation_id,
				status: "applied",
				build_revision: context.build.promotion.requested_build_revision!,
				spec_fingerprint: context.build.spec_fingerprint,
				recorded_at_ms: context.build.promotion.active_operation!.started_at_ms,
			};
		},
		...overrides,
	};
}

function applyDirectly(
	build: AppBuildState,
	key: string,
	nowMs: number,
): AppBuildState {
	const operationId = `operation_${key}`;
	const applying = markResourcesApplying(build, [key], {
		operation_id: operationId,
		operation_owner_id: "fixture",
		lease_expires_at_ms: nowMs + 1_000,
		now_ms: nowMs,
	});
	const resource = applying.resources[key]!;
	return recordResourceResults(
		applying,
		{
			[key]: {
				receipt_id: `receipt_${key}`,
				operation_id: operationId,
				status: "applied",
				physical_id: resource.physical_id,
				desired_fingerprint: resource.desired_fingerprint,
				observed_fingerprint: `artifact:${key}:v1`,
				recorded_at_ms: nowMs + 1,
			},
		},
		{ now_ms: nowMs + 2 },
	);
}

describe("AppBuild engine", () => {
	it("allows only one host mutation while an operation lease is active", async () => {
		const store = new MemoryBuildStore(
			createBuild(exampleAppSpec(), { ...ref, now_ms: 1 }),
		);
		let applyCount = 0;
		let announceStarted!: () => void;
		let releaseApply!: () => void;
		const started = new Promise<void>((resolve) => {
			announceStarted = resolve;
		});
		const gate = new Promise<void>((resolve) => {
			releaseApply = resolve;
		});
		const adapter = adapterWith({
			async applyResource(
				context: AppBuildHostResourceContext,
			): Promise<HostResourceResult> {
				applyCount += 1;
				announceStarted();
				await gate;
				const resource = context.build.resources[context.resource.key]!;
				return {
					receipt_id: "slow_receipt",
					operation_id: resource.active_operation!.operation_id,
					status: "applied",
					physical_id: resource.physical_id,
					desired_fingerprint: resource.desired_fingerprint,
					observed_fingerprint: "artifact:board:v1",
					recorded_at_ms: 10,
				};
			},
		});

		const first = advanceBuild(store, ref, adapter, {
			now_ms: () => 10,
			lease_duration_ms: 1_000,
			create_operation_id: () => "first_operation",
		});
		await started;
		const second = await advanceBuild(store, ref, adapter, {
			now_ms: () => 11,
			lease_duration_ms: 1_000,
			create_operation_id: () => "second_operation",
		});
		expect(second.action).toBe("in_progress");
		expect(applyCount).toBe(1);

		releaseApply();
		const completed = await first;
		expect(completed.action).toBe("applied");
		expect(completed.build.resources.issue_flow?.status).toBe("applied");
		expect(applyCount).toBe(1);
	});

	it("inspects expired work before allowing a retry", async () => {
		let build = createBuild(exampleAppSpec(), { ...ref, now_ms: 1 });
		build = markResourcesApplying(build, ["issue_flow"], {
			operation_id: "abandoned_operation",
			operation_owner_id: "old_controller",
			lease_expires_at_ms: 100,
			now_ms: 10,
		});
		const store = new MemoryBuildStore(build);
		let inspectionCount = 0;
		let applyCount = 0;
		const adapter = adapterWith({
			async inspectResource(context) {
				inspectionCount += 1;
				return {
					inspection_id: "expired_inspection",
					status: "missing",
					physical_id: context.resource.physical_id,
					desired_fingerprint: context.resource.desired_fingerprint,
					inspected_at_ms: 100,
					message: "Authoritative readback confirmed absence.",
				};
			},
			async applyResource(context) {
				applyCount += 1;
				return adapterWith({}).applyResource(context);
			},
		});

		const fenced = await advanceBuild(store, ref, adapter, {
			now_ms: () => 99,
		});
		expect(fenced.action).toBe("in_progress");
		expect(inspectionCount).toBe(0);
		expect(applyCount).toBe(0);

		const inspected = await advanceBuild(store, ref, adapter, {
			now_ms: () => 100,
		});
		expect(inspected.action).toBe("inspected");
		expect(inspected.build.resources.issue_flow?.status).toBe("pending");
		expect(inspectionCount).toBe(1);
		expect(applyCount).toBe(0);

		await advanceBuild(store, ref, adapter, {
			now_ms: () => 101,
			create_operation_id: () => "retry_operation",
		});
		expect(applyCount).toBe(1);
	});

	it("revalidates the exact artifacts immediately before promotion", async () => {
		let build = createBuild(exampleAppSpec(), { ...ref, now_ms: 1 });
		build = applyDirectly(build, "issue_flow", 10);
		build = applyDirectly(build, "submit_issue", 20);
		build = recordStructuralEvidence(
			build,
			{
				evidence_id: "old_structure",
				status: "passed",
				build_revision: build.validation_revision!,
				spec_fingerprint: build.spec_fingerprint,
				resource_fingerprints: {
					issue_flow: "artifact:issue_flow:v1",
					submit_issue: "artifact:submit_issue:v1",
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
				evidence_id: "behavioral_1",
				scenario_id: scenario.id,
				status: "passed",
				certification: "behavioral",
				build_revision: build.validation_revision!,
				spec_fingerprint: build.spec_fingerprint,
				scenario_fingerprint: scenario.desired_fingerprint,
				recorded_at_ms: 31,
			},
			{ now_ms: 31 },
		);
		const store = new MemoryBuildStore(build);
		const calls: string[] = [];
		const defaults = adapterWith({});
		const adapter = adapterWith({
			async validateStructure(context: AppBuildHostContext) {
				calls.push("validate");
				return defaults.validateStructure(context);
			},
			async promote(context: AppBuildHostContext) {
				calls.push("promote");
				return defaults.promote(context);
			},
		});

		const result = await promoteStagedBuild(store, ref, adapter, {
			now_ms: () => 40,
			create_operation_id: () => "promotion_operation",
		});
		expect(calls).toEqual(["validate", "promote"]);
		expect(result.build.promotion.status).toBe("promoted");
		expect(result.action).toBe("promoted");
	});
});
