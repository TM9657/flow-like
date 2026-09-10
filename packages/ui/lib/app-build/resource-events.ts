import { upsertAppEvent } from "../../components/global-chat/tools/event-tools";
import type { IBackendState } from "../../state/backend-state";
import type { AppResource } from "./contract";
import { appBuildFingerprint } from "./fingerprint";
import { resolveBuildEntry } from "./resource-links";

export interface AppBuildEventBinding {
	readonly id: string;
	readonly name: string;
	readonly event_type: Extract<
		AppResource,
		{ kind: "event" }
	>["config"]["event_type"];
	readonly board_id: string;
	readonly entry_node?: string;
	readonly page_id?: string;
	readonly route?: string;
	readonly config?: Record<string, unknown>;
}

export interface AppBuildEventEvidence {
	readonly id: string;
	readonly board_id: string;
	readonly node_id?: string;
	readonly page_id?: string;
	readonly route?: string;
	readonly active: false;
	readonly fingerprint: string;
}

/** Bind host-reserved Events only after their exact page or executable entry exists. */
export async function provisionAppBuildEvents(
	backend: IBackendState,
	appId: string,
	bindings: readonly AppBuildEventBinding[],
	options: { assertActive(): void; referenceApp?(appId: string): void },
): Promise<AppBuildEventEvidence[]> {
	if (new Set(bindings.map((binding) => binding.id)).size !== bindings.length)
		throw new Error("Duplicate reserved Event IDs.");
	const assertStaged = async () => {
		options.assertActive();
		if (
			(await backend.appState.getAppAuthoritative(appId)).status !== "Inactive"
		)
			throw new Error("Event binding requires an inactive staging app.");
		options.assertActive();
	};
	await assertStaged();
	const existing = await backend.eventState.getEventsAuthoritative(appId);
	const resolved: { binding: AppBuildEventBinding; nodeId?: string }[] = [];
	for (const binding of bindings) {
		options.assertActive();
		if (!binding.id || !binding.board_id || !binding.name)
			throw new Error(
				"Event binding requires exact host-reserved IDs and a name.",
			);
		if (existing.some((event) => event.id === binding.id && event.active))
			throw new Error(`Reserved Event '${binding.id}' is already active.`);
		if (binding.page_id) {
			if (binding.event_type !== "page" || binding.entry_node)
				throw new Error(
					"Page Events require a page target without a workflow entry.",
				);
			const page = await backend.pageState.getPageAuthoritative(
				appId,
				binding.page_id,
				binding.board_id,
			);
			if (page.boardId !== binding.board_id || page.id !== binding.page_id)
				throw new Error(
					`Page '${binding.page_id}' does not belong to its reserved board.`,
				);
			resolved.push({ binding });
		} else {
			if (binding.event_type === "page")
				throw new Error("Page Events require an exact page target.");
			resolved.push({
				binding,
				nodeId: await resolveBuildEntry(
					backend,
					appId,
					binding.board_id,
					binding.entry_node,
					binding.event_type,
				),
			});
		}
	}
	const evidence: AppBuildEventEvidence[] = [];
	for (const { binding, nodeId } of resolved) {
		await assertStaged();
		const result = await upsertAppEvent(
			backend,
			{
				app_id: appId,
				name: binding.name,
				event_type: binding.event_type,
				board_id: binding.board_id,
				node_id: nodeId,
				page_id: binding.page_id,
				route: binding.route,
				config: binding.config,
				active: false,
			},
			{
				assertActive: options.assertActive,
				referenceApp: options.referenceApp ?? (() => undefined),
				reservedEventId: binding.id,
			},
		);
		if (result.status !== "ok")
			throw new Error(
				`Event '${binding.id}' was not fully provisioned: ${JSON.stringify(result).slice(0, 2000)}`,
			);
		const event = await backend.eventState.getEventAuthoritative(
			appId,
			binding.id,
		);
		if (
			event.id !== binding.id ||
			event.active ||
			event.event_type !== binding.event_type ||
			event.board_id !== binding.board_id ||
			(binding.page_id
				? event.default_page_id !== binding.page_id || !!event.node_id
				: event.node_id !== nodeId)
		)
			throw new Error(
				`Event '${binding.id}' did not read back with its exact inactive target.`,
			);
		if (binding.route) {
			const route = await backend.routeState.getRouteByPathAuthoritative(
				appId,
				binding.route,
			);
			if (route?.eventId !== binding.id)
				throw new Error(`Event '${binding.id}' route did not read back.`);
		}
		evidence.push({
			id: binding.id,
			board_id: binding.board_id,
			node_id: nodeId,
			page_id: binding.page_id,
			route: binding.route,
			active: false,
			fingerprint: appBuildFingerprint("provisioned-event", event),
		});
	}
	await assertStaged();
	return evidence;
}
