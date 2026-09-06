import type { JsonObject } from "./contract";
import { compileAppSpec } from "./compiler";
import type { HostResourceInspection } from "./engine-types";
import { appBuildFingerprint } from "./fingerprint";
import { readiness, validateBuild } from "./readiness";
import {
	APP_BUILD_STATE_SCHEMA_VERSION,
	appBuildResourceInspectionSchema,
	appBuildResourceReceiptSchema,
	appBuildStateSchema,
	type AppBuildOperationLease,
	type AppBuildResourceReceipt,
	type AppBuildResourceState,
	type AppBuildState,
	type BuildResourceStatus,
	type RetiredResourceState,
} from "./state-contract";
import {
	descendantResourceKeys,
	ensureBuildMutable,
	ensureNoActiveBuildOperation,
	invalidateBuildEvidence,
	latestReceipt,
	nextBuildRevision,
	setValidationSnapshotWhenComplete,
} from "./state-internals";

export * from "./state-contract";
export * from "./state-evidence";
export * from "./state-promotion";
export { readiness, validateBuild } from "./readiness";

export function createBuild(
	input: unknown,
	context: {
		readonly app_id: string;
		readonly build_id: string;
		readonly original_request?: string;
		readonly now_ms?: number;
	},
): AppBuildState {
	const compiled = compileAppSpec(input, context);
	const nowMs = context.now_ms ?? Date.now();
	return appBuildStateSchema.parse({
		schema_version: APP_BUILD_STATE_SCHEMA_VERSION,
		build_id: context.build_id,
		app_id: context.app_id,
		revision: 0,
		original_request: context.original_request,
		original_request_fingerprint: context.original_request
			? appBuildFingerprint("original-request", context.original_request)
			: undefined,
		spec: compiled.spec,
		spec_fingerprint: compiled.spec_fingerprint,
		resources: Object.fromEntries(
			compiled.resources.map((resource) => [
				resource.key,
				{
					key: resource.key,
					kind: resource.kind,
					physical_id: resource.physical_id,
					desired_fingerprint: resource.desired_fingerprint,
					status: "pending",
					attempts: 0,
					receipts: [],
				},
			]),
		),
		retired_resources: {},
		scenarios: Object.fromEntries(
			compiled.scenarios.map((scenario) => [
				scenario.id,
				{
					id: scenario.id,
					desired_fingerprint: scenario.desired_fingerprint,
					status: "pending",
				},
			]),
		),
		promotion: { status: "staged", attempts: 0 },
		created_at_ms: nowMs,
		updated_at_ms: nowMs,
	});
}

export function reconcileBuild(
	build: AppBuildState,
	nextSpec: unknown,
	options: { readonly now_ms?: number } = {},
): AppBuildState {
	ensureBuildMutable(build);
	ensureNoActiveBuildOperation(build);
	const currentValidation = validateBuild(build);
	if (!currentValidation.ok) {
		throw new Error("Cannot reconcile an invalid AppBuildState.");
	}
	const next = compileAppSpec(nextSpec, {
		app_id: build.app_id,
		build_id: build.build_id,
	});
	const nextRequirements = new Map(
		next.spec.requirements.map((requirement) => [
			requirement.id,
			requirement.description,
		]),
	);
	for (const requirement of build.spec.requirements) {
		if (nextRequirements.get(requirement.id) !== requirement.description) {
			throw new Error(
				`Reconciliation cannot remove or rewrite existing requirement '${requirement.id}'. Start a new user-authorized build contract instead.`,
			);
		}
	}
	if (next.spec_fingerprint === build.spec_fingerprint) return build;

	const nextRevisionNumber = build.revision + 1;
	const currentCompiled = currentValidation.compiled;
	const currentByKey = new Map(
		currentCompiled.resources.map((resource) => [resource.key, resource]),
	);
	const nextByKey = new Map(
		next.resources.map((resource) => [resource.key, resource]),
	);
	const changed = new Set<string>();
	for (const resource of next.resources) {
		const current = build.resources[resource.key];
		if (
			!current ||
			current.kind !== resource.kind ||
			current.desired_fingerprint !== resource.desired_fingerprint
		) {
			changed.add(resource.key);
		}
	}
	const removed = currentCompiled.resources.filter(
		(resource) => !nextByKey.has(resource.key),
	);
	const affected = descendantResourceKeys(next, changed);
	for (const removedResource of removed) {
		for (const dependent of descendantResourceKeys(
			currentCompiled,
			new Set([removedResource.key]),
		)) {
			if (nextByKey.has(dependent)) affected.add(dependent);
		}
	}

	const resources: Record<string, AppBuildResourceState> = {};
	for (const resource of next.resources) {
		const current = build.resources[resource.key];
		resources[resource.key] =
			current && current.kind === resource.kind && !affected.has(resource.key)
				? current
				: {
						key: resource.key,
						kind: resource.kind,
						physical_id: resource.physical_id,
						desired_fingerprint: resource.desired_fingerprint,
						status: "pending",
						attempts: current?.attempts ?? 0,
						receipts: current?.receipts ?? [],
					};
	}

	const retiredResources = { ...build.retired_resources };
	for (const resource of removed) {
		const current = build.resources[resource.key];
		if (!current || current.status === "pending" || current.status === "failed")
			continue;
		const retirementId = `${resource.key}@${current.desired_fingerprint.slice(-16)}`;
		retiredResources[retirementId] = {
			retirement_id: retirementId,
			key: resource.key,
			kind: resource.kind,
			physical_id: current.physical_id,
			desired_fingerprint: current.desired_fingerprint,
			cleanup_status: "required",
			retired_at_revision: nextRevisionNumber,
			last_receipt: latestReceipt(current),
		};
	}
	for (const resource of next.resources) {
		const current = build.resources[resource.key];
		if (
			!current ||
			current.kind === resource.kind ||
			current.status === "pending" ||
			current.status === "failed"
		) {
			continue;
		}
		const retirementId = `${resource.key}@${current.desired_fingerprint.slice(-16)}`;
		retiredResources[retirementId] = {
			retirement_id: retirementId,
			key: resource.key,
			kind: current.kind,
			physical_id: current.physical_id,
			desired_fingerprint: current.desired_fingerprint,
			cleanup_status: "required",
			retired_at_revision: nextRevisionNumber,
			last_receipt: latestReceipt(current),
		};
	}

	const scenarios = Object.fromEntries(
		next.scenarios.map((scenario) => [
			scenario.id,
			{
				id: scenario.id,
				desired_fingerprint: scenario.desired_fingerprint,
				status: "pending" as const,
			},
		]),
	);
	const nowMs = options.now_ms ?? Date.now();
	const reconciled = appBuildStateSchema.parse({
		...build,
		revision: nextRevisionNumber,
		spec: next.spec,
		spec_fingerprint: next.spec_fingerprint,
		resources,
		retired_resources: retiredResources,
		scenarios,
		validation_revision: undefined,
		structural_evidence: undefined,
		promotion: { status: "staged", attempts: build.promotion.attempts },
		updated_at_ms: nowMs,
	});
	return setValidationSnapshotWhenComplete(reconciled);
}

export function readyResourceWaves(
	build: AppBuildState,
): readonly (readonly string[])[] {
	if (
		build.promotion.status !== "staged" ||
		Object.values(build.resources).some(
			(resource) => resource.status === "applying",
		)
	) {
		return [];
	}
	const validation = validateBuild(build);
	if (!validation.ok) return [];
	const ready = validation.compiled.resources
		.filter((resource) => {
			const state = build.resources[resource.key];
			return (
				state?.status === "pending" &&
				resource.depends_on.every(
					(dependency) => build.resources[dependency]?.status === "applied",
				)
			);
		})
		.map((resource) => resource.key)
		.sort((left, right) => left.localeCompare(right));
	return ready.length > 0 ? [ready] : [];
}

export function markResourcesApplying(
	build: AppBuildState,
	resourceKeys: readonly string[],
	options: {
		readonly operation_id: string;
		readonly operation_owner_id: string;
		readonly lease_expires_at_ms: number;
		readonly now_ms?: number;
	},
): AppBuildState {
	ensureBuildMutable(build);
	ensureNoActiveBuildOperation(build);
	if (build.promotion.status !== "staged") {
		throw new Error(
			"Resources cannot be applied while promotion is failed or in doubt. Request an explicit repair first.",
		);
	}
	const ready = new Set(readyResourceWaves(build).flat());
	if (
		resourceKeys.length === 0 ||
		resourceKeys.some((key) => !ready.has(key))
	) {
		throw new Error(
			"Only a non-empty ready resource wave can enter applying state.",
		);
	}
	const nowMs = options.now_ms ?? Date.now();
	if (options.lease_expires_at_ms <= nowMs) {
		throw new Error("An applying operation lease must expire in the future.");
	}
	let next = invalidateBuildEvidence(build);
	const resources = { ...next.resources };
	for (const key of resourceKeys) {
		const resource = resources[key];
		if (!resource) throw new Error(`Unknown resource '${key}'.`);
		resources[key] = {
			...resource,
			status: "applying",
			attempts: resource.attempts + 1,
			active_operation: {
				operation_id: options.operation_id,
				owner_id: options.operation_owner_id,
				started_at_ms: nowMs,
				expires_at_ms: options.lease_expires_at_ms,
			},
			last_error: undefined,
		};
	}
	next = { ...next, resources };
	return nextBuildRevision(next, nowMs);
}

export function recoverInterruptedBuild(
	build: AppBuildState,
	options: {
		readonly now_ms?: number;
		readonly abandoned_operation_ids?: readonly string[];
	} = {},
): AppBuildState {
	const nowMs = options.now_ms ?? Date.now();
	const abandoned = new Set(options.abandoned_operation_ids ?? []);
	const leaseIsAbandoned = (lease: AppBuildOperationLease | undefined) =>
		!!lease &&
		(lease.expires_at_ms <= nowMs || abandoned.has(lease.operation_id));
	const hasInterruptedResource = Object.values(build.resources).some(
		(resource) =>
			resource.status === "applying" &&
			leaseIsAbandoned(resource.active_operation),
	);
	const hasInterruptedPromotion =
		build.promotion.status === "promoting" &&
		leaseIsAbandoned(build.promotion.active_operation);
	if (!hasInterruptedResource && !hasInterruptedPromotion) return build;
	let next = invalidateBuildEvidence(build);
	next = {
		...next,
		resources: Object.fromEntries(
			Object.values(next.resources).map((resource) => [
				resource.key,
				resource.status === "applying" &&
				leaseIsAbandoned(resource.active_operation)
					? {
							...resource,
							status: "unknown" as const,
							active_operation: undefined,
							last_error:
								"The host operation lease expired or was abandoned while this resource was applying; inspect it before retrying.",
						}
					: resource,
			]),
		),
		promotion: hasInterruptedPromotion
			? {
					status: "unknown",
					attempts: next.promotion.attempts,
					requested_build_revision: next.promotion.requested_build_revision,
					active_operation: undefined,
				}
			: next.promotion,
	};
	return nextBuildRevision(next, nowMs);
}

export interface HostResourceResult {
	readonly receipt_id: string;
	readonly operation_id: string;
	readonly status: "applied" | "partial" | "unknown" | "failed";
	readonly physical_id: string;
	readonly desired_fingerprint: string;
	readonly observed_fingerprint?: string;
	readonly resource_revision?: string;
	readonly recorded_at_ms: number;
	readonly message?: string;
	readonly details?: JsonObject;
}

export function recordResourceResults(
	build: AppBuildState,
	results: Readonly<Record<string, HostResourceResult>>,
	options: { readonly now_ms?: number } = {},
): AppBuildState {
	ensureBuildMutable(build);
	if (Object.keys(results).length === 0) return build;
	const nowMs = options.now_ms ?? Date.now();
	let next = invalidateBuildEvidence(build);
	const resources = { ...next.resources };
	for (const [key, rawResult] of Object.entries(results)) {
		const resource = resources[key];
		if (!resource) throw new Error(`Unknown resource '${key}'.`);
		if (resource.status !== "applying" || !resource.active_operation) {
			throw new Error(
				`Resource '${key}' cannot accept a host result from status '${resource.status}'.`,
			);
		}
		const parsed = appBuildResourceReceiptSchema.parse(rawResult);
		if (parsed.operation_id !== resource.active_operation.operation_id) {
			throw new Error(`Resource '${key}' received a stale operation result.`);
		}
		const bindingMatches =
			parsed.physical_id === resource.physical_id &&
			parsed.desired_fingerprint === resource.desired_fingerprint;
		const leaseMatches =
			parsed.recorded_at_ms >= resource.active_operation.started_at_ms &&
			parsed.recorded_at_ms <= resource.active_operation.expires_at_ms &&
			parsed.recorded_at_ms <= nowMs &&
			nowMs <= resource.active_operation.expires_at_ms;
		const readbackMatches =
			parsed.status !== "applied" || parsed.observed_fingerprint !== undefined;
		const status: BuildResourceStatus =
			bindingMatches && leaseMatches && readbackMatches
				? parsed.status
				: "unknown";
		const receipt: AppBuildResourceReceipt =
			status === parsed.status
				? parsed
				: {
						...parsed,
						status: "unknown",
						message: leaseMatches
							? "Host result did not match the reserved physical id, desired fingerprint, or include an authoritative applied readback."
							: "Host result arrived outside its durable operation lease and cannot be accepted.",
					};
		resources[key] = {
			...resource,
			status,
			active_operation: undefined,
			receipts: [...resource.receipts, receipt].slice(-32),
			last_error:
				status === "failed" || status === "partial" || status === "unknown"
					? (receipt.message ?? `Resource ended in ${status} state.`)
					: undefined,
		};
	}
	next = nextBuildRevision({ ...next, resources }, nowMs);
	return setValidationSnapshotWhenComplete(next);
}

/**
 * Reconcile uncertain work from a fresh host read. A matching artifact is accepted only
 * when it matches a previously committed applied receipt for this exact desired plan.
 */
export function recordResourceInspections(
	build: AppBuildState,
	inspections: Readonly<Record<string, HostResourceInspection>>,
	options: { readonly now_ms?: number } = {},
): AppBuildState {
	ensureBuildMutable(build);
	if (Object.keys(inspections).length === 0) return build;
	const nowMs = options.now_ms ?? Date.now();
	let next = invalidateBuildEvidence(build);
	const resources = { ...next.resources };
	for (const [key, rawInspection] of Object.entries(inspections)) {
		const resource = resources[key];
		if (!resource) throw new Error(`Unknown resource '${key}'.`);
		if (resource.status !== "unknown" && resource.status !== "partial") {
			throw new Error(
				`Resource '${key}' cannot accept an inspection from status '${resource.status}'.`,
			);
		}
		const inspection = appBuildResourceInspectionSchema.parse(rawInspection);
		const inspectionIsFresh =
			inspection.inspected_at_ms >= build.updated_at_ms &&
			inspection.inspected_at_ms <= nowMs;
		const bindingMatches =
			inspection.physical_id === resource.physical_id &&
			inspection.desired_fingerprint === resource.desired_fingerprint;
		const priorAppliedReceipt = [...resource.receipts]
			.reverse()
			.find(
				(receipt) =>
					receipt.status === "applied" &&
					receipt.physical_id === resource.physical_id &&
					receipt.desired_fingerprint === resource.desired_fingerprint &&
					receipt.observed_fingerprint,
			);
		const provenMatch =
			inspectionIsFresh &&
			bindingMatches &&
			inspection.status === "matches" &&
			inspection.observed_fingerprint !== undefined &&
			inspection.observed_fingerprint ===
				priorAppliedReceipt?.observed_fingerprint;
		let status: BuildResourceStatus;
		let message = inspection.message;
		if (!inspectionIsFresh) {
			status = "unknown";
			message =
				"Inspection predates the current durable state or reports a future timestamp.";
		} else if (!bindingMatches) {
			status = "unknown";
			message = "Inspection did not match the reserved resource binding.";
		} else if (provenMatch) {
			status = "applied";
		} else if (inspection.status === "missing") {
			status = "pending";
		} else if (inspection.status === "drifted") {
			status = "partial";
		} else {
			status = "unknown";
			if (inspection.status === "matches" && !priorAppliedReceipt) {
				message =
					"The artifact exists, but no committed applied receipt proves it implements this desired plan.";
			}
		}
		resources[key] = {
			...resource,
			status,
			active_operation: undefined,
			last_inspection: inspection,
			last_error:
				status === "applied" || status === "pending"
					? undefined
					: (message ?? `Inspection ended in ${status} state.`),
		};
	}
	next = nextBuildRevision({ ...next, resources }, nowMs);
	return setValidationSnapshotWhenComplete(next);
}

export function invalidateBuildResources(
	build: AppBuildState,
	resourceKeys: readonly string[],
	options: { readonly now_ms?: number; readonly reason: string },
): AppBuildState {
	ensureBuildMutable(build);
	ensureNoActiveBuildOperation(build);
	const validation = validateBuild(build);
	if (!validation.ok)
		throw new Error("Cannot repair an invalid AppBuildState.");
	const unknown = resourceKeys.filter((key) => !build.resources[key]);
	if (resourceKeys.length === 0 || unknown.length > 0) {
		throw new Error(
			unknown.length > 0
				? `Unknown repair resources: ${unknown.join(", ")}.`
				: "Repair requires at least one resource key.",
		);
	}
	const affected = descendantResourceKeys(
		validation.compiled,
		new Set(resourceKeys),
	);
	let next = invalidateBuildEvidence(build);
	next = {
		...next,
		resources: Object.fromEntries(
			Object.values(next.resources).map((resource) => [
				resource.key,
				affected.has(resource.key)
					? {
							...resource,
							status: "pending" as const,
							last_error: options.reason,
						}
					: resource,
			]),
		),
	};
	return nextBuildRevision(next, options.now_ms ?? Date.now());
}

export function recordRetiredResourceCleanup(
	build: AppBuildState,
	retirementId: string,
	receipt: HostResourceResult,
	options: { readonly retained?: boolean; readonly now_ms?: number } = {},
): AppBuildState {
	ensureBuildMutable(build);
	ensureNoActiveBuildOperation(build);
	const retired = build.retired_resources[retirementId];
	if (!retired) throw new Error(`Unknown retired resource '${retirementId}'.`);
	const parsed = appBuildResourceReceiptSchema.parse(receipt);
	if (
		parsed.physical_id !== retired.physical_id ||
		parsed.desired_fingerprint !== retired.desired_fingerprint ||
		parsed.status !== "applied"
	) {
		throw new Error(
			"Retired resource cleanup requires an exact applied host receipt.",
		);
	}
	let next = invalidateBuildEvidence(build);
	next = {
		...next,
		retired_resources: {
			...next.retired_resources,
			[retirementId]: {
				...retired,
				cleanup_status: options.retained ? "retained" : "cleaned",
				cleanup_receipt: parsed,
			},
		},
	};
	next = nextBuildRevision(next, options.now_ms ?? Date.now());
	return setValidationSnapshotWhenComplete(next);
}
