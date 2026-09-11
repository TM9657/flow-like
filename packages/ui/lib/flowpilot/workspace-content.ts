import type { IBackendState } from "../../state/backend-state";
import type { IPage } from "../../state/backend-state/page-state";
import { projectWorkspaceTableFields } from "./workspace-arrow";
import { loadFlowPilotWorkspaceDocs } from "./workspace-docs";
import {
	type WorkspaceCoverage,
	type WorkspaceDocument,
	type WorkspaceKind,
	type WorkspaceTarget,
	boundedWorkspaceText,
	jsonBytes,
	parseWorkspaceResourceId,
	stableJson,
	workspaceResourceId,
	workspaceRevision,
} from "./workspace-resource";
import { projectWorkspaceSchema } from "./workspace-schema";
import { extractFlowScriptWorkspaceSymbols } from "./workspace-symbols";

const MAX_APPS = 4;
const MAX_PER_KIND_PER_APP = 16;
const MAX_DOCUMENTS = 2_000;
const MAX_SOURCE_BYTES = 1_048_576;
const MAX_CORPUS_BYTES = 8_388_608;

export interface WorkspaceScope {
	getProfileAppIds: () => Promise<Set<string>>;
	getProfileIdentity?: () => Promise<string>;
	scopedAppId?: string;
}
export interface WorkspaceSelection {
	app_id?: string;
	board_id?: string;
	kinds: WorkspaceKind[];
}

export async function authorizeWorkspaceApp(
	appId: string,
	scope: WorkspaceScope,
) {
	const profile = await scope.getProfileAppIds();
	if (appId !== scope.scopedAppId && !profile.has(appId))
		throw new Error("WORKSPACE_APP_NOT_ACCESSIBLE");
}

export async function mapWorkspaceReads<T, R>(
	items: T[],
	read: (item: T) => Promise<R>,
): Promise<R[]> {
	const results: R[] = new Array(items.length);
	let index = 0;
	await Promise.all(
		Array.from({ length: Math.min(4, items.length) }, async () => {
			while (index < items.length) {
				const current = index++;
				results[current] = await read(items[current]);
			}
		}),
	);
	return results;
}

function record(value: unknown): Record<string, unknown> {
	return value && typeof value === "object" && !Array.isArray(value)
		? (value as Record<string, unknown>)
		: {};
}

function contractProjection(value: unknown, keys: string[]) {
	const source = record(value);
	return Object.fromEntries(
		keys
			.filter((key) => source[key] !== undefined)
			.map((key) => [key, source[key]]),
	);
}

/** Page action parameters and state values are intentionally outside the searchable contract. */
function pageContract(page: IPage) {
	let visited = 0;
	let truncated = false;
	const bindings: unknown[] = [];
	const routingIdentifiers = (value: unknown, keys: string[]) => {
		const source = record(value);
		const identifiers: Record<string, string> = {};
		for (const key of keys) {
			if (typeof source[key] === "string") identifiers[key] = source[key];
			else if (source[key] !== undefined) truncated = true;
		}
		return identifiers;
	};
	const visit = (value: unknown, path: string, depth: number) => {
		if (!value || typeof value !== "object") return;
		if (++visited > 4_096 || depth > 16) {
			truncated = true;
			return;
		}
		if (Array.isArray(value)) {
			if (value.length > 256) truncated = true;
			value
				.slice(0, 256)
				.forEach((item, index) => visit(item, `${path}/${index}`, depth + 1));
			return;
		}
		const object = record(value);
		if (typeof object.path === "string")
			bindings.push({ location: path, binding_path: object.path });
		if (
			typeof object.name === "string" &&
			object.context &&
			typeof object.context === "object"
		) {
			bindings.push({
				location: path,
				action: object.name,
				...routingIdentifiers(object.context, [
					"eventId",
					"event_id",
					"boardId",
					"board_id",
					"nodeId",
					"node_id",
				]),
				page_action: routingIdentifiers(object.pageAction, [
					"actionId",
					"manifestRevision",
				]),
			});
		}
		const entries = Object.entries(object);
		if (entries.length > 128) truncated = true;
		for (const [key, item] of entries.slice(0, 128)) {
			if (
				[
					"context",
					"pageAction",
					"defaultValue",
					"literalJson",
					"dataModel",
					"widgetRefs",
				].includes(key)
			)
				continue;
			visit(item, `${path}/${key}`, depth + 1);
		}
	};
	const components = page.components.slice(0, 256);
	for (const component of components)
		visit(component.component, component.id, 0);
	return {
		...contractProjection(page, [
			"id",
			"name",
			"title",
			"route",
			"boardId",
			"onLoadEventId",
			"onUnloadEventId",
			"onIntervalEventId",
			"onIntervalSeconds",
		]),
		components: components.map((component) => ({
			id: component.id,
			types:
				typeof component.component.type === "string"
					? [component.component.type]
					: Object.keys(component.component),
		})),
		bindings,
		truncated: page.components.length > 256 || truncated,
	};
}

async function contractDocument(
	target: WorkspaceTarget,
	title: string,
	value: unknown,
	truncated = false,
): Promise<WorkspaceDocument> {
	const content = stableJson(value);
	if (jsonBytes(content) > MAX_SOURCE_BYTES)
		throw new Error("WORKSPACE_RESOURCE_TOO_LARGE");
	return {
		target,
		resource_id: workspaceResourceId(target),
		revision: await workspaceRevision(content),
		title: boundedWorkspaceText(title, 512),
		content,
		truncated,
	};
}

async function workflowDocuments(
	backend: IBackendState,
	appId: string,
	boardId: string,
): Promise<WorkspaceDocument[]> {
	const source = await backend.boardState.getFlowScriptAuthoritative(
		appId,
		boardId,
		undefined,
		true,
	);
	if (jsonBytes(source) > MAX_SOURCE_BYTES)
		throw new Error("WORKSPACE_RESOURCE_TOO_LARGE");
	const parsed = extractFlowScriptWorkspaceSymbols(source);
	if (!parsed.complete) throw new Error("WORKSPACE_SOURCE_NOT_INDEXABLE");
	const revision = await workspaceRevision(source);
	return parsed.symbols
		.filter((symbol) => symbol.kind !== "module")
		.map((symbol) => {
			const target: WorkspaceTarget = {
				kind: "workflow",
				app_id: appId,
				board_id: boardId,
				id: symbol.qualifiedName,
			};
			return {
				target,
				resource_id: workspaceResourceId(target),
				revision,
				title: symbol.qualifiedName,
				symbol: symbol.qualifiedName,
				signature: symbol.signature,
				content: source.slice(symbol.start, symbol.end),
				start_line: symbol.startLine,
				end_line: symbol.endLine,
				anchor_id: symbol.anchor?.id,
			};
		});
}

export async function readWorkspaceDocument(
	backend: IBackendState,
	target: WorkspaceTarget,
	scope: WorkspaceScope,
): Promise<WorkspaceDocument> {
	if (target.kind === "doc") {
		const docs = await loadFlowPilotWorkspaceDocs();
		const entry = docs.entries.find((entry) => entry.resource_id === target.id);
		if (!entry) throw new Error("WORKSPACE_RESOURCE_NOT_FOUND");
		return {
			target,
			resource_id: workspaceResourceId(target),
			revision: entry.revision,
			title: entry.title,
			content: entry.content,
			source_path: entry.source_path,
			start_line: entry.line_start,
			end_line: entry.line_end,
		};
	}
	const appId = target.app_id;
	if (!appId) throw new Error("WORKSPACE_APP_REQUIRED");
	await authorizeWorkspaceApp(appId, scope);
	if (target.kind === "workflow") {
		if (!target.board_id) throw new Error("WORKSPACE_BOARD_REQUIRED");
		const documents = await workflowDocuments(backend, appId, target.board_id);
		const document = documents.find(
			(document) => document.target.id === target.id,
		);
		if (!document) throw new Error("WORKSPACE_RESOURCE_NOT_FOUND");
		return document;
	}
	if (target.kind === "event") {
		const event = await backend.eventState.getEventAuthoritative(
			appId,
			target.id,
		);
		if (
			event.id !== target.id ||
			(target.board_id && event.board_id !== target.board_id)
		)
			throw new Error("WORKSPACE_RESOURCE_NOT_FOUND");
		let truncated = (event.inputs?.length ?? 0) > 128;
		const inputs = (event.inputs ?? []).slice(0, 128).map((input) => {
			const projected = contractProjection(input, [
				"id",
				"name",
				"friendly_name",
				"description",
				"data_type",
				"value_type",
				"optional",
			]);
			if (input.schema !== undefined && input.schema !== null) {
				const schema = projectWorkspaceSchema(input.schema);
				projected.schema = schema.schema;
				truncated ||= schema.truncated;
			}
			return projected;
		});
		return contractDocument(
			target,
			event.name,
			{
				...contractProjection(event, [
					"id",
					"name",
					"description",
					"event_type",
					"board_id",
					"board_version",
					"node_id",
					"route",
					"default_page_id",
					"active",
					"execution_mode",
					"exposure",
				]),
				inputs,
			},
			truncated,
		);
	}
	if (target.kind === "page") {
		const page = await backend.pageState.getPageAuthoritative(
			appId,
			target.id,
			target.board_id,
		);
		if (
			page.id !== target.id ||
			(target.board_id && page.boardId !== target.board_id)
		)
			throw new Error("WORKSPACE_RESOURCE_NOT_FOUND");
		const contract = pageContract(page);
		return contractDocument(target, page.name, contract, contract.truncated);
	}
	const schema = await backend.dbState.getSchemaAuthoritative(
		appId,
		target.id,
		false,
	);
	if (!Array.isArray(schema?.fields))
		throw new Error("WORKSPACE_SCHEMA_NOT_READABLE");
	const projected = projectWorkspaceTableFields(schema.fields);
	return contractDocument(
		target,
		target.id,
		{ table_name: target.id, fields: projected.fields },
		projected.truncated,
	);
}

export async function collectWorkspaceDocuments(
	backend: IBackendState,
	selection: WorkspaceSelection,
	scope: WorkspaceScope,
): Promise<{ documents: WorkspaceDocument[]; coverage: WorkspaceCoverage }> {
	const coverage: WorkspaceCoverage = {
		complete: true,
		indexed_resources: 0,
		app_ids: [],
		kinds: selection.kinds,
		issues: [],
	};
	const issue = (
		code: string,
		kind?: WorkspaceKind,
		app_id?: string,
		resource?: string,
	) => {
		coverage.complete = false;
		if (coverage.issues.length < 64)
			coverage.issues.push({ code, kind, app_id, resource });
	};
	const documents: WorkspaceDocument[] = [];
	let bytes = 0;
	const add = (document: WorkspaceDocument) => {
		if (!parseWorkspaceResourceId(document.resource_id)) {
			issue("RESOURCE_ID_INVALID", document.target.kind);
			return;
		}
		const size = jsonBytes(document.content);
		if (documents.length >= MAX_DOCUMENTS || bytes + size > MAX_CORPUS_BYTES) {
			issue("CORPUS_LIMIT", document.target.kind);
			return;
		}
		if (document.truncated)
			issue(
				"CONTRACT_TRUNCATED",
				document.target.kind,
				document.target.app_id,
				document.target.id,
			);
		documents.push(document);
		bytes += size;
	};
	if (selection.kinds.includes("doc")) {
		try {
			const docs = await loadFlowPilotWorkspaceDocs();
			coverage.docs = docs.coverage;
			for (const entry of docs.entries) {
				const target: WorkspaceTarget = { kind: "doc", id: entry.resource_id };
				add({
					target,
					resource_id: workspaceResourceId(target),
					revision: entry.revision,
					title: entry.title,
					content: entry.content,
					source_path: entry.source_path,
					start_line: entry.line_start,
					end_line: entry.line_end,
				});
			}
		} catch {
			issue("DOCS_UNAVAILABLE", "doc");
		}
	}
	const appKinds = selection.kinds.filter((kind) => kind !== "doc");
	if (appKinds.length === 0)
		return {
			documents,
			coverage: { ...coverage, indexed_resources: documents.length },
		};
	const appId = selection.app_id || scope.scopedAppId;
	let appIds: string[];
	if (appId) {
		await authorizeWorkspaceApp(appId, scope);
		appIds = [appId];
	} else appIds = [...(await scope.getProfileAppIds())].sort();
	if (appIds.length > MAX_APPS) issue("APP_LIMIT");
	appIds = appIds.slice(0, MAX_APPS);
	coverage.app_ids = appIds;
	for (const app of appIds) {
		const listed = await mapWorkspaceReads(appKinds, async (kind) => {
			try {
				let targets: WorkspaceTarget[];
				if (kind === "workflow") {
					const boards =
						await backend.boardState.getBoardSummariesAuthoritative(app);
					if (
						selection.board_id &&
						!boards.some((board) => board.id === selection.board_id)
					)
						issue("BOARD_NOT_FOUND", kind, app, selection.board_id);
					targets = boards
						.filter(
							(board) => !selection.board_id || board.id === selection.board_id,
						)
						.map((board) => ({
							kind,
							app_id: app,
							board_id: board.id,
							id: board.id,
						}));
				} else if (kind === "event") {
					const events = await backend.eventState.getEventsAuthoritative(app);
					targets = events
						.filter(
							(event) =>
								!selection.board_id || event.board_id === selection.board_id,
						)
						.map((event) => ({
							kind,
							app_id: app,
							board_id: event.board_id,
							id: event.id,
						}));
				} else if (kind === "page") {
					const pages = await backend.pageState.getPagesAuthoritative(
						app,
						selection.board_id,
					);
					targets = pages.map((page) => ({
						kind,
						app_id: app,
						board_id: page.boardId,
						id: page.pageId,
					}));
				} else {
					const tables = await backend.dbState.listTablesAuthoritative(app);
					targets = tables.map((table) => ({ kind, app_id: app, id: table }));
				}
				targets.sort((a, b) => a.id.localeCompare(b.id));
				if (targets.length > MAX_PER_KIND_PER_APP)
					issue("RESOURCE_LIMIT", kind, app);
				return targets.slice(0, MAX_PER_KIND_PER_APP);
			} catch {
				issue("INVENTORY_UNREADABLE", kind, app);
				return [];
			}
		});
		const loaded = await mapWorkspaceReads(listed.flat(), async (target) => {
			try {
				if (target.kind === "workflow" && !target.board_id)
					throw new Error("WORKSPACE_BOARD_REQUIRED");
				return target.kind === "workflow"
					? await workflowDocuments(backend, app, target.board_id ?? target.id)
					: [await readWorkspaceDocument(backend, target, scope)];
			} catch {
				issue("RESOURCE_UNREADABLE", target.kind, app, target.id);
				return [];
			}
		});
		for (const group of loaded) for (const document of group) add(document);
	}
	coverage.indexed_resources = documents.length;
	return { documents, coverage };
}
