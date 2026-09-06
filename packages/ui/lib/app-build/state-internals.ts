import type { CompiledAppSpec } from "./compiler";
import {
	appBuildStateSchema,
	type AppBuildResourceReceipt,
	type AppBuildResourceState,
	type AppBuildState,
} from "./state-contract";

export function nextBuildRevision(
	build: AppBuildState,
	nowMs: number,
): AppBuildState {
	return appBuildStateSchema.parse({
		...build,
		revision: build.revision + 1,
		updated_at_ms: nowMs,
	});
}

export function latestAppliedReceipt(
	state: AppBuildResourceState,
): AppBuildResourceReceipt | undefined {
	return [...state.receipts]
		.reverse()
		.find((receipt) => receipt.status === "applied");
}

export function latestReceipt(
	state: AppBuildResourceState,
): AppBuildResourceReceipt | undefined {
	return state.receipts.at(-1);
}

export function invalidateBuildEvidence(build: AppBuildState): AppBuildState {
	return {
		...build,
		validation_revision: undefined,
		structural_evidence: undefined,
		scenarios: Object.fromEntries(
			Object.values(build.scenarios).map((scenario) => [
				scenario.id,
				{ ...scenario, status: "pending" as const, evidence: undefined },
			]),
		),
		promotion: { status: "staged", attempts: build.promotion.attempts },
	};
}

export function setValidationSnapshotWhenComplete(
	build: AppBuildState,
): AppBuildState {
	if (
		Object.values(build.resources).every(
			(resource) => resource.status === "applied",
		) &&
		!Object.values(build.retired_resources).some(
			(resource) => resource.cleanup_status === "required",
		)
	) {
		return { ...build, validation_revision: build.revision };
	}
	return { ...build, validation_revision: undefined };
}

export function descendantResourceKeys(
	compiled: CompiledAppSpec,
	seeds: ReadonlySet<string>,
): Set<string> {
	const result = new Set(seeds);
	const byKey = new Map(
		compiled.resources.map((resource) => [resource.key, resource]),
	);
	const queue = [...seeds];
	while (queue.length > 0) {
		const current = queue.shift();
		if (!current) continue;
		for (const dependent of byKey.get(current)?.dependents ?? []) {
			if (result.has(dependent)) continue;
			result.add(dependent);
			queue.push(dependent);
		}
	}
	return result;
}

export function ensureBuildMutable(build: AppBuildState): void {
	if (build.promotion.status === "promoted") {
		throw new Error(
			"A promoted build is immutable. Start a new staged build before repairing or changing its spec.",
		);
	}
}

export function ensureNoActiveBuildOperation(build: AppBuildState): void {
	if (
		build.promotion.status === "promoting" ||
		Object.values(build.resources).some(
			(resource) => resource.status === "applying",
		)
	) {
		throw new Error(
			"Cannot change a build while a host operation lease is active.",
		);
	}
}
