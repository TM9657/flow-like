import { compileAppSpec, type CompiledAppSpec } from "./compiler";
import type {
	AppBuildEngineOptions,
	AppBuildHostAdapter,
	HostResourceInspection,
} from "./engine-types";
import { appBuildFingerprint } from "./fingerprint";
import {
	beginPromotion,
	recordPromotionResult,
	recordResourceInspections,
	recordResourceResults,
	recordStructuralEvidence,
	recoverInterruptedBuild,
	readyResourceWaves,
	readiness,
	validateBuild,
	markResourcesApplying,
	type AppBuildState,
	type HostResourceResult,
	type PromotionReceipt,
	type StructuralEvidence,
} from "./state";
import {
	readRequiredBuild,
	type AppBuildRef,
	type AppBuildStore,
} from "./store";

export interface AppBuildEngineResult {
	readonly build: AppBuildState;
	readonly plan: CompiledAppSpec;
	readonly action:
		| "idle"
		| "in_progress"
		| "recovered"
		| "inspected"
		| "applied"
		| "validated"
		| "promoted"
		| "promotion_partial"
		| "promotion_unknown"
		| "promotion_failed";
	readonly resource_keys: readonly string[];
}

function clock(options: AppBuildEngineOptions): number {
	return options.now_ms?.() ?? Date.now();
}

function assertNotAborted(signal: AbortSignal | undefined): void {
	if (signal?.aborted) {
		throw signal.reason instanceof Error
			? signal.reason
			: new Error("App build operation was cancelled.");
	}
}

function planFor(build: AppBuildState): CompiledAppSpec {
	const validation = validateBuild(build);
	if (!validation.ok) {
		throw new Error(
			`Invalid AppBuildState: ${validation.issues
				.map((issue) => issue.message)
				.join("; ")}`,
		);
	}
	return validation.compiled;
}

function activeResourceKeys(build: AppBuildState): string[] {
	return Object.values(build.resources)
		.filter((resource) => resource.status === "applying")
		.map((resource) => resource.key)
		.sort((left, right) => left.localeCompare(right));
}

async function commit(
	store: AppBuildStore,
	ref: AppBuildRef,
	previous: AppBuildState,
	next: AppBuildState,
): Promise<AppBuildState> {
	if (next === previous) return previous;
	await store.compareAndSwap(ref, previous.revision, next);
	return next;
}

async function mapBounded<T, R>(
	values: readonly T[],
	limit: number,
	mapper: (value: T, index: number) => Promise<R>,
): Promise<R[]> {
	const results = new Array<R>(values.length);
	let cursor = 0;
	const worker = async () => {
		while (cursor < values.length) {
			const index = cursor;
			cursor += 1;
			results[index] = await mapper(values[index] as T, index);
		}
	};
	const workerCount = Math.min(Math.max(1, limit), values.length);
	await Promise.all(Array.from({ length: workerCount }, () => worker()));
	return results;
}

function operationId(
	build: AppBuildState,
	resourceKey: string,
	kind: "inspection" | "receipt" | "evidence" | "promotion",
	nowMs: number,
): string {
	return appBuildFingerprint("engine-operation-evidence", {
		app_id: build.app_id,
		build_id: build.build_id,
		revision: build.revision,
		resource_key: resourceKey,
		kind,
		now_ms: nowMs,
	});
}

const DEFAULT_OPERATION_LEASE_MS = 8 * 60 * 60 * 1_000 + 15 * 60 * 1_000;

function createOperationId(
	build: AppBuildState,
	ownerId: string,
	nowMs: number,
	options: AppBuildEngineOptions,
): string {
	const supplied = options.create_operation_id?.();
	if (supplied?.trim()) return supplied;
	const randomId = globalThis.crypto?.randomUUID?.();
	return randomId
		? `op_${randomId}`
		: appBuildFingerprint("operation", {
				app_id: build.app_id,
				build_id: build.build_id,
				revision: build.revision,
				owner_id: ownerId,
				now_ms: nowMs,
			});
}

function leaseDuration(options: AppBuildEngineOptions): number {
	const duration = options.lease_duration_ms ?? DEFAULT_OPERATION_LEASE_MS;
	if (
		!Number.isSafeInteger(duration) ||
		duration <= 0 ||
		duration > 24 * 60 * 60 * 1_000
	) {
		throw new Error(
			"lease_duration_ms must be an integer between 1 ms and 24 hours.",
		);
	}
	return duration;
}

function unknownInspection(
	build: AppBuildState,
	resourceKey: string,
	message: string,
	nowMs: number,
): HostResourceInspection {
	const resource = build.resources[resourceKey];
	if (!resource) throw new Error(`Unknown resource '${resourceKey}'.`);
	return {
		status: "unknown",
		inspection_id: operationId(build, resourceKey, "inspection", nowMs),
		physical_id: resource.physical_id,
		desired_fingerprint: resource.desired_fingerprint,
		inspected_at_ms: nowMs,
		message,
	};
}

function unknownApplyResult(
	build: AppBuildState,
	resourceKey: string,
	message: string,
	nowMs: number,
): HostResourceResult {
	const resource = build.resources[resourceKey];
	if (!resource) throw new Error(`Unknown resource '${resourceKey}'.`);
	return {
		receipt_id: operationId(build, resourceKey, "receipt", nowMs),
		operation_id:
			resource.active_operation?.operation_id ??
			operationId(build, resourceKey, "receipt", nowMs),
		status: "unknown",
		physical_id: resource.physical_id,
		desired_fingerprint: resource.desired_fingerprint,
		recorded_at_ms: nowMs,
		message,
	};
}

function errorMessage(error: unknown): string {
	return error instanceof Error ? error.message : String(error);
}

/**
 * Advance at most one durable step: recover a crash marker, inspect uncertain work,
 * or apply one ready dependency wave. Every effect is bracketed by CAS writes.
 */
export async function advanceBuild(
	store: AppBuildStore,
	ref: AppBuildRef,
	adapter: AppBuildHostAdapter,
	options: AppBuildEngineOptions = {},
): Promise<AppBuildEngineResult> {
	assertNotAborted(options.signal);
	let build = await readRequiredBuild(store, ref);
	let plan = planFor(build);
	if (
		build.promotion.status === "partial" ||
		build.promotion.status === "unknown"
	) {
		return {
			build,
			plan,
			action: `promotion_${build.promotion.status}`,
			resource_keys: [],
		};
	}
	if (build.promotion.status === "promoting") {
		return {
			build,
			plan,
			action: "in_progress",
			resource_keys: [],
		};
	}
	const interrupted = activeResourceKeys(build).length > 0;
	if (interrupted) {
		const recovered = recoverInterruptedBuild(build, {
			now_ms: clock(options),
		});
		if (recovered === build) {
			return {
				build,
				plan,
				action: "in_progress",
				resource_keys: activeResourceKeys(build),
			};
		}
		build = await commit(store, ref, build, recovered);
		plan = planFor(build);
	}

	const uncertainKeys = Object.values(build.resources)
		.filter(
			(resource) =>
				resource.status === "unknown" || resource.status === "partial",
		)
		.map((resource) => resource.key)
		.sort((left, right) => left.localeCompare(right));
	if (uncertainKeys.length > 0) {
		const byKey = new Map(
			plan.resources.map((resource) => [resource.key, resource]),
		);
		const concurrency = Math.min(16, Math.max(1, options.max_parallel ?? 4));
		const inspected = await mapBounded(
			uncertainKeys,
			concurrency,
			async (key): Promise<[string, HostResourceInspection]> => {
				const resource = byKey.get(key);
				if (!resource) throw new Error(`Unknown compiled resource '${key}'.`);
				try {
					assertNotAborted(options.signal);
					return [
						key,
						await adapter.inspectResource({
							build,
							plan,
							resource,
							signal: options.signal,
						}),
					];
				} catch (error) {
					const nowMs = clock(options);
					return [
						key,
						unknownInspection(
							build,
							key,
							`Authoritative inspection failed: ${errorMessage(error)}`,
							nowMs,
						),
					];
				}
			},
		);
		const next = recordResourceInspections(
			build,
			Object.fromEntries(inspected),
			{
				now_ms: clock(options),
			},
		);
		build = await commit(store, ref, build, next);
		return {
			build,
			plan: planFor(build),
			action: "inspected",
			resource_keys: uncertainKeys,
		};
	}

	const ready = readyResourceWaves(build)[0] ?? [];
	if (ready.length === 0) {
		return {
			build,
			plan,
			action: interrupted ? "recovered" : "idle",
			resource_keys: [],
		};
	}
	const concurrency = Math.min(16, Math.max(1, options.max_parallel ?? 4));
	const resourceKeys = ready.slice(0, concurrency);
	const startedAtMs = clock(options);
	const operationOwnerId =
		options.operation_owner_id?.trim() || "app-build-engine";
	const operationIdValue = createOperationId(
		build,
		operationOwnerId,
		startedAtMs,
		options,
	);
	const applying = markResourcesApplying(build, resourceKeys, {
		operation_id: operationIdValue,
		operation_owner_id: operationOwnerId,
		lease_expires_at_ms: startedAtMs + leaseDuration(options),
		now_ms: startedAtMs,
	});
	build = await commit(store, ref, build, applying);
	plan = planFor(build);
	const byKey = new Map(
		plan.resources.map((resource) => [resource.key, resource]),
	);
	const applied = await mapBounded(
		resourceKeys,
		concurrency,
		async (key): Promise<[string, HostResourceResult]> => {
			const resource = byKey.get(key);
			if (!resource) throw new Error(`Unknown compiled resource '${key}'.`);
			try {
				assertNotAborted(options.signal);
				return [
					key,
					await adapter.applyResource({
						build,
						plan,
						resource,
						signal: options.signal,
					}),
				];
			} catch (error) {
				const nowMs = clock(options);
				return [
					key,
					unknownApplyResult(
						build,
						key,
						`Apply did not return authoritative completion: ${errorMessage(error)}`,
						nowMs,
					),
				];
			}
		},
	);
	const next = recordResourceResults(build, Object.fromEntries(applied), {
		now_ms: clock(options),
	});
	build = await commit(store, ref, build, next);
	return {
		build,
		plan: planFor(build),
		action: "applied",
		resource_keys: resourceKeys,
	};
}

async function validateAndCommit(
	store: AppBuildStore,
	ref: AppBuildRef,
	adapter: AppBuildHostAdapter,
	options: AppBuildEngineOptions,
): Promise<AppBuildEngineResult> {
	const build = await readRequiredBuild(store, ref);
	const plan = planFor(build);
	if (build.validation_revision === undefined) {
		throw new Error(
			"Structural validation requires a complete applied resource snapshot.",
		);
	}
	assertNotAborted(options.signal);
	let evidence: StructuralEvidence;
	try {
		evidence = await adapter.validateStructure({
			build,
			plan,
			signal: options.signal,
		});
	} catch (error) {
		const nowMs = clock(options);
		evidence = {
			evidence_id: operationId(build, "structure", "evidence", nowMs),
			status: "failed",
			build_revision: build.validation_revision,
			spec_fingerprint: build.spec_fingerprint,
			resource_fingerprints: {},
			recorded_at_ms: nowMs,
			issues: [`Structural validation failed: ${errorMessage(error)}`],
		};
	}
	const next = recordStructuralEvidence(build, evidence, {
		now_ms: clock(options),
	});
	const committed = await commit(store, ref, build, next);
	return {
		build: committed,
		plan: planFor(committed),
		action: "validated",
		resource_keys: [],
	};
}

export async function validateStagedBuild(
	store: AppBuildStore,
	ref: AppBuildRef,
	adapter: AppBuildHostAdapter,
	options: AppBuildEngineOptions = {},
): Promise<AppBuildEngineResult> {
	const build = await readRequiredBuild(store, ref);
	const plan = planFor(build);
	if (build.promotion.status === "promoted") {
		return { build, plan, action: "idle", resource_keys: [] };
	}
	const activeKeys = activeResourceKeys(build);
	if (activeKeys.length > 0 || build.promotion.status === "promoting") {
		return {
			build,
			plan,
			action: "in_progress",
			resource_keys: activeKeys,
		};
	}
	return validateAndCommit(store, ref, adapter, options);
}

/** Validate fresh persisted state, then promote only the exact validated revision. */
export async function promoteStagedBuild(
	store: AppBuildStore,
	ref: AppBuildRef,
	adapter: AppBuildHostAdapter,
	options: AppBuildEngineOptions = {},
): Promise<AppBuildEngineResult> {
	let current = await readRequiredBuild(store, ref);
	let currentPlan = planFor(current);
	if (current.promotion.status === "promoted") {
		return {
			build: current,
			plan: currentPlan,
			action: "promoted",
			resource_keys: [],
		};
	}
	if (
		current.promotion.status === "partial" ||
		current.promotion.status === "unknown"
	) {
		return {
			build: current,
			plan: currentPlan,
			action: `promotion_${current.promotion.status}`,
			resource_keys: [],
		};
	}
	if (current.promotion.status === "promoting") {
		const recovered = recoverInterruptedBuild(current, {
			now_ms: clock(options),
		});
		if (recovered === current) {
			return {
				build: current,
				plan: currentPlan,
				action: "in_progress",
				resource_keys: [],
			};
		}
		current = await commit(store, ref, current, recovered);
		currentPlan = planFor(current);
		return {
			build: current,
			plan: currentPlan,
			action: "recovered",
			resource_keys: [],
		};
	}
	const validationResult = await validateAndCommit(
		store,
		ref,
		adapter,
		options,
	);
	let build = validationResult.build;
	let plan = validationResult.plan;
	if (!readiness(build).can_promote) return validationResult;
	const startedAtMs = clock(options);
	const operationOwnerId =
		options.operation_owner_id?.trim() || "app-build-engine";
	const promotionOperationId = createOperationId(
		build,
		operationOwnerId,
		startedAtMs,
		options,
	);
	const promoting = beginPromotion(build, {
		operation_id: promotionOperationId,
		operation_owner_id: operationOwnerId,
		lease_expires_at_ms: startedAtMs + leaseDuration(options),
		now_ms: startedAtMs,
	});
	build = await commit(store, ref, build, promoting);
	plan = compileAppSpec(build.spec, {
		app_id: build.app_id,
		build_id: build.build_id,
	});
	assertNotAborted(options.signal);
	let receipt: PromotionReceipt;
	try {
		receipt = await adapter.promote({
			build,
			plan,
			signal: options.signal,
		});
	} catch (error) {
		const nowMs = clock(options);
		receipt = {
			receipt_id: operationId(build, "app", "promotion", nowMs),
			operation_id: promotionOperationId,
			status: "unknown",
			build_revision: build.promotion.requested_build_revision as number,
			spec_fingerprint: build.spec_fingerprint,
			recorded_at_ms: nowMs,
			message: `Promotion did not return authoritative completion: ${errorMessage(error)}`,
		};
	}
	const next = recordPromotionResult(build, receipt, {
		now_ms: clock(options),
	});
	build = await commit(store, ref, build, next);
	const action: AppBuildEngineResult["action"] =
		build.promotion.status === "promoted"
			? "promoted"
			: build.promotion.status === "partial"
				? "promotion_partial"
				: build.promotion.status === "failed"
					? "promotion_failed"
					: "promotion_unknown";
	return {
		build,
		plan: planFor(build),
		action,
		resource_keys: [],
	};
}
