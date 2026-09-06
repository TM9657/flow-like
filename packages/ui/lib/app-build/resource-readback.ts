import type { IBackendState } from "../../state/backend-state";
import { EVENT_DEFINITIONS } from "../event-definitions";
import { parseUint8ArrayToJson } from "../uint8";
import { validateComponents } from "../../components/flowpilot/validateComponents";
import { collectRunnableWorkflowEventEntries } from "../../components/global-chat/workflow-event-entries";
import type { CompiledAppResource, CompiledAppSpec } from "./compiler";
import { appBuildFingerprint } from "./fingerprint";
import { resourceId, resolveBuildEntry } from "./resource-links";
import { schemaFields, tableSchemaIssues } from "./table-schema-readback";

export interface ResourceReadback {
	readonly exists: boolean;
	readonly fingerprint?: string;
	readonly artifact?: unknown;
	readonly issues: readonly string[];
}

function snapshot(artifact: unknown, issues: string[] = []): ResourceReadback {
	return {
		exists: true,
		artifact,
		issues,
		fingerprint: appBuildFingerprint("persisted-resource", artifact),
	};
}

/** Read authoritative resource state. Read errors stay unknown, never become "missing". */
export async function readAppBuildResource(
	backend: IBackendState,
	plan: CompiledAppSpec,
	resource: CompiledAppResource,
): Promise<ResourceReadback> {
	const appId = plan.app_id;
	const id = resource.physical_id;
	switch (resource.kind) {
		case "board": {
			const boards =
				await backend.boardState.getBoardSummariesAuthoritative(appId);
			if (!boards.some((board) => board.id === id))
				return { exists: false, issues: [] };
			const board = await backend.boardState.getBoardAuthoritative(appId, id);
			const source = await backend.boardState.getFlowScriptAuthoritative(
				appId,
				id,
			);
			const issues: string[] = [];
			if (!source.trim() || Object.keys(board.nodes ?? {}).length === 0)
				issues.push("Board has no executable content.");
			if (!backend.boardState.checkFlowScriptReconcile) {
				issues.push(
					"Authoritative FlowScript reconciliation is unavailable on this backend.",
				);
			} else {
				const check = await backend.boardState.checkFlowScriptReconcile(
					appId,
					id,
					source,
				);
				if (!check.parse_valid || !check.reconcile_valid)
					issues.push(...check.diagnostics, "FlowScript does not reconcile.");
			}
			const entries = collectRunnableWorkflowEventEntries(
				board,
				id,
				new Set(),
				(type) => EVENT_DEFINITIONS[type]?.eventTypes ?? [],
			);
			return snapshot(
				{
					id,
					source,
					entries,
					page_ids: board.page_ids ?? [],
					execution_mode: board.execution_mode,
				},
				issues,
			);
		}
		case "page": {
			const pages = await backend.pageState.getPagesAuthoritative(appId);
			if (!pages.some((page) => page.pageId === id))
				return { exists: false, issues: [] };
			const page = await backend.pageState.getPageAuthoritative(
				appId,
				id,
				resourceId(plan, resource.config.board),
			);
			const issues: string[] = [];
			if (page.boardId !== resourceId(plan, resource.config.board))
				issues.push("Page belongs to the wrong board.");
			if (page.route !== resource.config.route)
				issues.push("Page route differs from the contract.");
			for (const [selector, actual, label] of [
				[resource.config.on_load_entry, page.onLoadEventId, "load"],
				[resource.config.on_unload_entry, page.onUnloadEventId, "unload"],
				[resource.config.on_interval_entry, page.onIntervalEventId, "interval"],
			] as const) {
				if (!selector) continue;
				const expected = await resolveBuildEntry(
					backend,
					appId,
					resourceId(plan, resource.config.board),
					selector,
					"quick_action",
				);
				if (actual !== expected)
					issues.push(`Page ${label} handler differs from the contract.`);
			}
			if (
				resource.config.interval_seconds !== undefined &&
				page.onIntervalSeconds !== resource.config.interval_seconds
			) {
				issues.push("Page interval differs from the contract.");
			}
			if (!page.components.length && !page.content.length)
				issues.push("Page is empty.");
			const checked = validateComponents(page.components);
			if (checked.components.length !== page.components.length)
				issues.push("Page contains invalid or duplicate components.");
			issues.push(
				...checked.warnings
					.slice(0, 8)
					.map(
						(warning) =>
							`Page requires component repair: ${warning.slice(0, 1600)}`,
					),
			);
			const { createdAt: _created, updatedAt: _updated, ...artifact } = page;
			return snapshot(artifact, issues);
		}
		case "widget": {
			const widgets = await backend.widgetState.getWidgetsAuthoritative(appId);
			if (!widgets.some(([, widgetId]) => widgetId === id))
				return { exists: false, issues: [] };
			const widget = await backend.widgetState.getWidgetAuthoritative(
				appId,
				id,
			);
			const issues: string[] = [];
			if (!widget.components.length) issues.push("Widget is empty.");
			if (
				!widget.components.some(
					(component) => component.id === widget.rootComponentId,
				)
			)
				issues.push("Widget root does not exist.");
			const checked = validateComponents(widget.components);
			if (checked.components.length !== widget.components.length)
				issues.push("Widget contains invalid or duplicate components.");
			issues.push(
				...checked.warnings
					.slice(0, 8)
					.map(
						(warning) =>
							`Widget requires component repair: ${warning.slice(0, 1600)}`,
					),
			);
			const { createdAt: _created, updatedAt: _updated, ...artifact } = widget;
			return snapshot(artifact, issues);
		}
		case "table": {
			const tables = await backend.dbState.listTablesAuthoritative(appId);
			if (!tables.includes(resource.config.name))
				return { exists: false, issues: [] };
			const raw = await backend.dbState.getSchemaAuthoritative(
				appId,
				resource.config.name,
			);
			const fields = schemaFields(raw);
			const issues = tableSchemaIssues(fields, resource.config.columns);
			return snapshot({ table: resource.config.name, fields }, issues);
		}
		case "event": {
			const events = await backend.eventState.getEventsAuthoritative(appId);
			const event = events.find((event) => event.id === id);
			if (!event) return { exists: false, issues: [] };
			const issues: string[] = [];
			if (event.event_type !== resource.config.event_type)
				issues.push("Event type differs from the contract.");
			if (
				resource.config.page &&
				event.default_page_id !== resourceId(plan, resource.config.page)
			)
				issues.push("Event targets the wrong page.");
			if (
				resource.config.board &&
				event.board_id !== resourceId(plan, resource.config.board)
			)
				issues.push("Event targets the wrong board.");
			if (!resource.config.page && resource.config.board) {
				const expected = await resolveBuildEntry(
					backend,
					appId,
					resourceId(plan, resource.config.board),
					resource.config.entry_node,
					resource.config.event_type,
				);
				if (event.node_id !== expected)
					issues.push("Event targets a different workflow entry.");
			}
			const config = parseUint8ArrayToJson(event.config);
			for (const [key, value] of Object.entries(resource.config.config ?? {})) {
				if (
					appBuildFingerprint("event-config", config?.[key] ?? null) !==
					appBuildFingerprint("event-config", value)
				) {
					issues.push(`Event config '${key}' differs from the contract.`);
				}
			}
			if (resource.config.route) {
				const route = await backend.routeState.getRouteByPathAuthoritative(
					appId,
					resource.config.route,
				);
				if (route?.eventId !== id)
					issues.push("Event route is missing or targets another Event.");
			}
			return snapshot(
				{
					id: event.id,
					name: event.name,
					event_type: event.event_type,
					board_id: event.board_id,
					node_id: event.node_id,
					board_version: event.board_version ?? null,
					default_page_id: event.default_page_id ?? null,
					config,
					route: resource.config.route ?? null,
					execution_mode: event.execution_mode,
					variables: event.variables,
					inputs: event.inputs ?? null,
				},
				issues,
			);
		}
	}
}
