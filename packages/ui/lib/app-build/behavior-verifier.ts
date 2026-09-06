import { createId } from "@paralleldrive/cuid2";
import { isChatEventType } from "../event-definitions";
import { scenarioReceiptDetails } from "./scenario-receipt";
import type { CompiledAppSpec } from "./compiler";
import {
	runAppBehaviorScenario,
	type AppScenarioHostAdapter,
	type AppScenarioResolvedResource,
} from "./scenarios";
import { recordScenarioEvidence, type AppBuildState } from "./state";
import type { AppBuildRef, AppBuildStore } from "./store";
import { readRequiredBuild } from "./store";

/** Bind scenario targets to the host's symbol table, never to model-provided app IDs. */
export function appScenarioResources(
	plan: CompiledAppSpec,
): Record<string, AppScenarioResolvedResource> {
	const resources: Record<string, AppScenarioResolvedResource> = {
		app: { id: plan.app_id, appId: plan.app_id, kind: "app" },
	};
	for (const resource of plan.resources) {
		if (resource.kind !== "event" && resource.kind !== "page") continue;
		const event =
			resource.kind === "page"
				? plan.resources.find(
						(item) =>
							item.kind === "event" && item.config.page === resource.key,
					)
				: resource;
		resources[resource.key] = {
			id: resource.physical_id,
			appId: plan.app_id,
			kind:
				resource.kind === "event" && isChatEventType(resource.config.event_type)
					? "chat"
					: resource.kind,
			...(resource.kind === "page" ? { pageId: resource.physical_id } : {}),
			...(event ? { eventId: event.physical_id } : {}),
		};
	}
	return resources;
}

/** Scenario JSON cannot supply an isolation attestation or write a success receipt. */
export async function verifyAppBuildBehavior(
	store: AppBuildStore,
	ref: AppBuildRef,
	plan: CompiledAppSpec,
	adapter: AppScenarioHostAdapter,
	signal?: AbortSignal,
): Promise<AppBuildState> {
	let build = await readRequiredBuild(store, ref);
	if (
		build.validation_revision === undefined ||
		build.structural_evidence?.status !== "passed"
	) {
		throw new Error(
			"Validate all persisted resources before running acceptance scenarios.",
		);
	}
	for (const scenario of plan.scenarios) {
		signal?.throwIfAborted();
		const result = await runAppBehaviorScenario(scenario, {
			appId: plan.app_id,
			resources: appScenarioResources(plan),
			adapter,
			signal,
		});
		signal?.throwIfAborted();
		const latest = await readRequiredBuild(store, ref);
		if (
			latest.validation_revision !== build.validation_revision ||
			latest.spec_fingerprint !== plan.spec_fingerprint
		) {
			throw new Error(
				"Build changed while its acceptance scenario was running. Results were not accepted.",
			);
		}
		const next = recordScenarioEvidence(latest, {
			evidence_id: createId(),
			scenario_id: scenario.id,
			certification: result.certification,
			status:
				result.status === "pass"
					? "passed"
					: result.status === "fail"
						? "failed"
						: "blocked",
			build_revision: build.validation_revision!,
			spec_fingerprint: plan.spec_fingerprint,
			scenario_fingerprint: scenario.desired_fingerprint,
			recorded_at_ms: Date.now(),
			message: `Host scenario result: ${result.status}.`,
			details: scenarioReceiptDetails(result),
		});
		await store.compareAndSwap(ref, latest.revision, next);
		build = next;
	}
	return build;
}
