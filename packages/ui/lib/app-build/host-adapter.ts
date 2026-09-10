import { createId } from "@paralleldrive/cuid2";
import type { IBackendState } from "../../state/backend-state";
import type { AppBuildHostAdapter } from "./engine-types";
import {
	appBuildFoundationForPlan,
	provisionAppBuildFoundation,
} from "./resource-foundation";
import {
	type AppBuildDispatch,
	materializeAppBuildResource,
} from "./resource-materializer";
import { readAppBuildResource } from "./resource-readback";
import { promoteAppBuildResources } from "./staging";

/** The only producer of build evidence in the interactive host. */
export function createAppBuildHostAdapter(
	backend: IBackendState,
	dispatch: AppBuildDispatch,
): AppBuildHostAdapter {
	let preparing: { fingerprint: string; promise: Promise<unknown> } | undefined;
	const prepare = (plan: Parameters<typeof appBuildFoundationForPlan>[0]) => {
		const fingerprint = `${plan.app_id}:${plan.build_id}:${plan.spec_fingerprint}`;
		if (preparing?.fingerprint === fingerprint) return preparing.promise;
		const promise = provisionAppBuildFoundation(
			backend,
			appBuildFoundationForPlan(plan),
			dispatch,
		);
		const current = { fingerprint, promise };
		preparing = current;
		void promise
			.finally(() => {
				if (preparing === current) preparing = undefined;
			})
			.catch(() => undefined);
		return promise;
	};
	return {
		async inspectResource({ build, plan, resource, signal }) {
			dispatch.assertActive();
			signal?.throwIfAborted();
			const binding = {
				inspection_id: createId(),
				physical_id: resource.physical_id,
				desired_fingerprint: resource.desired_fingerprint,
				inspected_at_ms: Date.now(),
			};
			try {
				const observed = await readAppBuildResource(backend, plan, resource);
				if (!observed.exists)
					return {
						...binding,
						status: "missing",
						message: "Reserved resource does not exist.",
					};
				const receipt = [...build.resources[resource.key].receipts]
					.reverse()
					.find(
						(receipt) =>
							receipt.status === "applied" &&
							receipt.desired_fingerprint === resource.desired_fingerprint,
					);
				if (
					receipt &&
					observed.fingerprint &&
					observed.fingerprint === receipt.observed_fingerprint &&
					!observed.issues.length
				) {
					return {
						...binding,
						status: "matches",
						observed_fingerprint: observed.fingerprint,
					};
				}
				return {
					...binding,
					status: receipt ? "drifted" : "unknown",
					observed_fingerprint: observed.fingerprint,
					message:
						observed.issues.join("\n") ||
						(receipt
							? "Persisted resource changed after the last verified receipt."
							: "Resource exists without a matching completed host receipt. Inspect and request a focused repair."),
				};
			} catch (error) {
				return { ...binding, status: "unknown", message: String(error) };
			}
		},
		async applyResource({ build, plan, resource, signal }) {
			dispatch.assertActive();
			signal?.throwIfAborted();
			const binding = {
				receipt_id: createId(),
				operation_id:
					build.resources[resource.key].active_operation?.operation_id ?? "",
				physical_id: resource.physical_id,
				desired_fingerprint: resource.desired_fingerprint,
				recorded_at_ms: Date.now(),
			};
			try {
				const app = await backend.appState.getAppAuthoritative(plan.app_id);
				if (app.status !== "Inactive")
					throw new Error(
						"Build resources may only be applied in an inactive staging app.",
					);
				const prior = build.resources[resource.key];
				// Setup completes before any specialist sees the shared resource contract.
				await prepare(plan);
				dispatch.assertActive();
				signal?.throwIfAborted();
				const diagnostics = [
					prior.last_error,
					prior.last_inspection?.message,
					prior.receipts.at(-1)?.message,
				].filter((message): message is string => !!message);
				await materializeAppBuildResource(
					backend,
					plan,
					resource,
					dispatch,
					diagnostics,
				);
				dispatch.assertActive();
				signal?.throwIfAborted();
				const observed = await readAppBuildResource(backend, plan, resource);
				if (
					(await backend.appState.getAppAuthoritative(plan.app_id)).status !==
					"Inactive"
				) {
					throw new Error(
						"App left inactive staging during generation. Its effects are unknown; inspect before resuming.",
					);
				}
				if (
					!observed.exists ||
					!observed.fingerprint ||
					observed.issues.length
				) {
					return {
						...binding,
						status: "partial",
						observed_fingerprint: observed.fingerprint,
						message:
							observed.issues.join("\n") ||
							"Resource was not present on authoritative readback.",
					};
				}
				return {
					...binding,
					status: "applied",
					observed_fingerprint: observed.fingerprint,
					recorded_at_ms: Date.now(),
				};
			} catch (error) {
				return {
					...binding,
					status: "unknown",
					message: String(error).slice(0, 4000),
				};
			}
		},
		async validateStructure({ build, plan, signal }) {
			dispatch.assertActive();
			signal?.throwIfAborted();
			const issues: string[] = [];
			const fingerprints: Record<string, string> = {};
			for (const resource of plan.resources) {
				dispatch.assertActive();
				signal?.throwIfAborted();
				try {
					const observed = await readAppBuildResource(backend, plan, resource);
					const receipt = [...build.resources[resource.key].receipts]
						.reverse()
						.find((receipt) => receipt.status === "applied");
					if (!observed.exists || !observed.fingerprint)
						issues.push(`${resource.key}: missing resource.`);
					else {
						fingerprints[resource.key] = observed.fingerprint;
						if (observed.fingerprint !== receipt?.observed_fingerprint)
							issues.push(`${resource.key}: changed since generation receipt.`);
					}
					issues.push(
						...observed.issues.map((issue) => `${resource.key}: ${issue}`),
					);
				} catch (error) {
					issues.push(`${resource.key}: ${String(error)}`);
				}
			}
			const app = await backend.appState.getAppAuthoritative(plan.app_id);
			if (app.status !== "Inactive")
				issues.push("App is not in inactive staging.");
			const events = await backend.eventState.getEventsAuthoritative(
				plan.app_id,
			);
			const expectedEvents = new Set(
				plan.resources
					.filter((resource) => resource.kind === "event")
					.map((resource) => resource.physical_id),
			);
			for (const event of events) {
				if (!expectedEvents.has(event.id))
					issues.push(`Unplanned Event '${event.id}' exists.`);
				if (event.active)
					issues.push(`Event '${event.id}' was activated before promotion.`);
			}
			return {
				evidence_id: createId(),
				status: issues.length ? "failed" : "passed",
				build_revision: build.validation_revision ?? build.revision,
				spec_fingerprint: build.spec_fingerprint,
				resource_fingerprints: fingerprints,
				recorded_at_ms: Date.now(),
				issues: issues.slice(0, 128).map((issue) => issue.slice(0, 2000)),
			};
		},
		async promote({ build, plan, signal }) {
			dispatch.assertActive();
			signal?.throwIfAborted();
			const result = await promoteAppBuildResources(backend, plan, () => {
				dispatch.assertActive();
				signal?.throwIfAborted();
			});
			return {
				receipt_id: createId(),
				operation_id: build.promotion.active_operation?.operation_id ?? "",
				status: result.status === "complete" ? "applied" : "partial",
				build_revision: build.validation_revision ?? build.revision,
				spec_fingerprint: build.spec_fingerprint,
				recorded_at_ms: Date.now(),
				details: JSON.parse(JSON.stringify(result)),
				...(result.status === "partial"
					? {
							message:
								"Activation did not finish; inspect promotion subreceipts before retrying.",
						}
					: {}),
			};
		},
	};
}
