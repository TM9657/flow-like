import type { IBackendState } from "../../state/backend-state";
import { appBuildContractGuide } from "./contract";
import { compileAppSpec } from "./compiler";
import {
	advanceBuild,
	validateStagedBuild,
	promoteStagedBuild,
} from "./engine";
import { createAppBuildStore, readRequiredBuild } from "./store";
import {
	createBuild,
	invalidateBuildResources,
	readiness,
	readyResourceWaves,
	type AppBuildState,
} from "./state";
import { createAppBuildHostAdapter } from "./host-adapter";
import { beginAppBuildStaging } from "./staging";
import { verifyAppBuildBehavior } from "./behavior-verifier";
import type { AppScenarioHostAdapter } from "./scenarios";
import type { AppBuildDispatch } from "./resource-materializer";
import { listAppCapabilities, instantiateAppCapability } from "./capabilities";
import { INTERACTIVE_APP_BUILD_CAPABILITIES } from "./interactive-scenario-adapter";

export interface AppBuildToolOptions {
	readonly backend: IBackendState;
	readonly dispatch: AppBuildDispatch;
	readonly scenarios: AppScenarioHostAdapter;
	readonly originalRequest?: string;
	readonly signal?: AbortSignal;
	readonly operationOwnerId?: string;
	readonly deadlineAtMs?: number;
}

export function summarizeAppBuild(build: AppBuildState) {
	return {
		status: "ok",
		app_id: build.app_id,
		build_id: build.build_id,
		revision: build.revision,
		readiness: readiness(build),
		ready_waves: readyResourceWaves(build),
		resources: Object.values(build.resources).map((resource) => ({
			key: resource.key,
			kind: resource.kind,
			id: resource.physical_id,
			status: resource.status,
			attempts: resource.attempts,
			...(resource.last_error ? { message: resource.last_error } : {}),
		})),
		scenarios: Object.values(build.scenarios).map((scenario) => ({
			id: scenario.id,
			status: scenario.status,
			message: scenario.evidence?.message,
		})),
		promotion: build.promotion,
	};
}

/** Public operations accept intent and resource keys. Evidence is produced exclusively by adapters. */
export async function executeAppBuildTool(
	args: Record<string, unknown>,
	options: AppBuildToolOptions,
): Promise<unknown> {
	const operation = String(args.operation ?? "");
	if (operation === "schema")
		return {
			...appBuildContractGuide(),
			host_capabilities: INTERACTIVE_APP_BUILD_CAPABILITIES,
		};
	if (operation === "capabilities")
		return { status: "ok", capabilities: listAppCapabilities() };
	if (operation === "recipe")
		return {
			status: "ok",
			fragment: instantiateAppCapability(
				String(args.capability_id ?? ""),
				String(args.capability_version ?? ""),
				args.parameters ?? {},
			),
		};
	const ref = {
		app_id: String(args.app_id ?? ""),
		build_id: String(args.build_id ?? ""),
	};
	if (!ref.app_id || !ref.build_id)
		throw new Error("app_build requires exact app_id and build_id.");
	const { backend, dispatch, signal } = options;
	dispatch.assertActive();
	signal?.throwIfAborted();
	const store = createAppBuildStore(backend.appState);
	const host = createAppBuildHostAdapter(backend, dispatch);
	if (operation === "begin") {
		const candidate = createBuild(args.spec, {
			...ref,
			original_request: options.originalRequest,
		});
		const existing = await store.read(ref);
		if (existing) {
			if (
				existing.spec_fingerprint !== candidate.spec_fingerprint ||
				existing.original_request_fingerprint !==
					candidate.original_request_fingerprint
			) {
				throw new Error(
					"This build_id already belongs to a different app contract. Resume that contract; do not overwrite it.",
				);
			}
			if (existing.revision === 0) {
				await beginAppBuildStaging(backend, ref.app_id, () => {
					dispatch.assertActive();
					signal?.throwIfAborted();
				});
			}
			return { ...summarizeAppBuild(existing), resumed: true };
		}
		// Persist intent before changing app status. Revision zero retries also complete staging,
		// covering a crash between this create and the app transition.
		dispatch.assertActive();
		signal?.throwIfAborted();
		await store.create(candidate);
		await beginAppBuildStaging(backend, ref.app_id, () => {
			dispatch.assertActive();
			signal?.throwIfAborted();
		});
		dispatch.referenceApp(ref.app_id);
		return summarizeAppBuild(candidate);
	}
	if (operation === "status")
		return summarizeAppBuild(await readRequiredBuild(store, ref));
	if (
		operation === "advance" ||
		operation === "validate" ||
		operation === "promote"
	) {
		const run =
			operation === "advance"
				? advanceBuild
				: operation === "validate"
					? validateStagedBuild
					: promoteStagedBuild;
		const result = await run(store, ref, host, {
			signal,
			max_parallel: 3,
			operation_owner_id: options.operationOwnerId,
			lease_duration_ms: Math.max(
				60_000,
				(options.deadlineAtMs ?? Date.now() + 8.25 * 60 * 60 * 1000) -
					Date.now() +
					30_000,
			),
		});
		return {
			...summarizeAppBuild(result.build),
			action: result.action,
			resource_keys: result.resource_keys,
		};
	}
	const build = await readRequiredBuild(store, ref);
	if (operation === "repair") {
		if (
			!Array.isArray(args.resource_keys) ||
			!args.resource_keys.length ||
			args.resource_keys.some((key) => typeof key !== "string")
		) {
			throw new Error(
				"repair requires a nonempty array of exact logical resource_keys.",
			);
		}
		const next = invalidateBuildResources(
			build,
			args.resource_keys as string[],
			{
				reason:
					"Focused repair requested through the app build host; original requirements retained.",
			},
		);
		dispatch.assertActive();
		await store.compareAndSwap(ref, build.revision, next);
		return summarizeAppBuild(next);
	}
	if (operation === "test") {
		const plan = compileAppSpec(build.spec, ref);
		return summarizeAppBuild(
			await verifyAppBuildBehavior(store, ref, plan, options.scenarios, signal),
		);
	}
	throw new Error(`Unknown app_build operation '${operation}'.`);
}
