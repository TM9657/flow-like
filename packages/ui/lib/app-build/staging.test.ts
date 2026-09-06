import { describe, expect, test, vi } from "vitest";

import type { IBackendState } from "../../state/backend-state";
import type { IPage } from "../../state/backend-state/page-state";
import type { IWidget } from "../../state/backend-state/widget-state";
import { type IApp, IAppStatus, IAppVisibility } from "../schema/app/app";
import type { IBoard } from "../schema/flow/board";
import type { IEvent } from "../schema/flow/event";
import { type CompiledAppSpec, compileAppSpec } from "./compiler";
import { beginAppBuildStaging, promoteAppBuildResources } from "./staging";

function systemTime() {
	return { secs_since_epoch: 1, nanos_since_epoch: 0 };
}

function app(status: IAppStatus, visibility = IAppVisibility.Private): IApp {
	return {
		id: "app_fixture",
		status,
		visibility,
		boards: [],
		events: [],
		page_ids: [],
		widget_ids: [],
		authors: [],
		bits: [],
		templates: [],
		created_at: systemTime(),
		updated_at: systemTime(),
		download_count: 0,
		interactions_count: 0,
		rating_count: 0,
		rating_sum: 0,
		execution_mode: "Any",
	} as IApp;
}

function board(id: string, nonempty = false): IBoard {
	return {
		id,
		name: "Main",
		description: "",
		nodes: nonempty ? { entry: { id: "entry" } } : {},
		variables: {},
		layers: {},
		comments: {},
		refs: {},
		page_ids: [],
		version: [0, 0, 0],
		created_at: systemTime(),
		updated_at: systemTime(),
	} as unknown as IBoard;
}

function stagingBackend(options: {
	app: IApp;
	boards?: IBoard[];
	pages?: Array<{ pageId: string; boardId?: string }>;
	widgets?: Array<[string, string, undefined]>;
	events?: IEvent[];
	tables?: string[];
}) {
	let current = options.app;
	const updateApp = vi.fn(async (next: IApp) => {
		current = next;
	});
	const boards = options.boards ?? [];
	const backend = {
		appState: {
			getAppAuthoritative: vi.fn(async () => current),
			updateAppAuthoritative: updateApp,
		},
		boardState: {
			getBoardSummariesAuthoritative: vi.fn(async () =>
				boards.map((item) => ({ id: item.id })),
			),
			getBoardAuthoritative: vi.fn(async (_appId: string, boardId: string) => {
				const found = boards.find((item) => item.id === boardId);
				if (!found) throw new Error(`Missing board '${boardId}'.`);
				return found;
			}),
		},
		pageState: {
			getPagesAuthoritative: vi.fn(async () => options.pages ?? []),
		},
		widgetState: {
			getWidgetsAuthoritative: vi.fn(async () => options.widgets ?? []),
		},
		eventState: {
			getEventsAuthoritative: vi.fn(async () => options.events ?? []),
		},
		dbState: {
			listTablesAuthoritative: vi.fn(async () => options.tables ?? []),
		},
	} as unknown as IBackendState;
	return { backend, updateApp, getCurrentApp: () => current };
}

function fullPlan(): CompiledAppSpec {
	return compileAppSpec(
		{
			schema_version: 1,
			name: "Fixture",
			requirements: [{ id: "ship", description: "Ship the fixture." }],
			resources: [
				{
					key: "main",
					kind: "board",
					depends_on: [],
					requirement_ids: ["ship"],
					config: { name: "Main", instruction: "Build the workflow." },
				},
				{
					key: "home",
					kind: "page",
					depends_on: ["main"],
					requirement_ids: [],
					config: {
						name: "Home",
						route: "/",
						board: "main",
						instruction: "Build the page.",
					},
				},
				{
					key: "card",
					kind: "widget",
					depends_on: [],
					requirement_ids: [],
					config: { name: "Card", instruction: "Build the widget." },
				},
				{
					key: "records",
					kind: "table",
					depends_on: [],
					requirement_ids: [],
					config: {
						name: "records",
						columns: [{ name: "id", type: "string", nullable: false }],
					},
				},
				{
					key: "launch",
					kind: "event",
					depends_on: ["home"],
					requirement_ids: [],
					config: {
						name: "Launch",
						event_type: "page",
						page: "home",
						route: "/",
					},
				},
			],
			scenarios: [],
		},
		{ app_id: "app_fixture", build_id: "build_fixture" },
	);
}

function rollbackPlan(): CompiledAppSpec {
	return compileAppSpec(
		{
			schema_version: 1,
			name: "Rollback fixture",
			requirements: [{ id: "ship", description: "Ship the fixture." }],
			resources: [
				{
					key: "main",
					kind: "board",
					depends_on: [],
					requirement_ids: ["ship"],
					config: { name: "Main", instruction: "Build the workflow." },
				},
				...[
					["first", "First"],
					["second", "Second"],
				].map(([key, name]) => ({
					key,
					kind: "event",
					depends_on: ["main"],
					requirement_ids: [],
					config: {
						name,
						event_type: "quick_action",
						board: "main",
						entry_node: "entry",
					},
				})),
			],
			scenarios: [],
		},
		{ app_id: "app_fixture", build_id: "build_rollback" },
	);
}

function resourceId(plan: CompiledAppSpec, key: string): string {
	const resource = plan.resources.find((candidate) => candidate.key === key);
	if (!resource) throw new Error(`Missing resource '${key}'.`);
	return resource.physical_id;
}

function page(id: string, boardId: string): IPage {
	return {
		id,
		name: "Home",
		route: "/",
		content: [],
		layoutType: "stack",
		components: [{ id: "root", component: "Text" }],
		boardId,
		createdAt: "2026-01-01T00:00:00.000Z",
		updatedAt: "2026-01-01T00:00:00.000Z",
	} as unknown as IPage;
}

function widget(id: string): IWidget {
	return {
		id,
		name: "Card",
		rootComponentId: "root",
		components: [{ id: "root", component: "Text" }],
		dataModel: [],
		customizationOptions: [],
		tags: [],
		createdAt: "2026-01-01T00:00:00.000Z",
		updatedAt: "2026-01-01T00:00:00.000Z",
	} as unknown as IWidget;
}

function event(
	id: string,
	boardId: string,
	eventType: string,
	defaultPageId?: string,
): IEvent {
	return {
		id,
		name: id,
		active: false,
		board_id: boardId,
		board_version: null,
		event_version: [0, 0, 0],
		event_type: eventType,
		node_id: "entry",
		config: [],
		variables: {},
		priority: 0,
		description: "",
		default_page_id: defaultPageId,
		created_at: systemTime(),
		updated_at: systemTime(),
	};
}

function promotionBackend(
	plan: CompiledAppSpec,
	options: {
		driftVersionedBoard?: boolean;
		failEventKey?: string;
		failEventAfterCommitKey?: string;
		failEventPinAfterCommitKey?: string;
		failAppActivationAfterCommit?: boolean;
	} = {},
) {
	let currentApp = app(IAppStatus.Inactive);
	const calls: string[] = [];
	const mainId = resourceId(plan, "main");
	const draftBoard = board(mainId, true);
	const pageResource = plan.resources.find(
		(resource) => resource.kind === "page",
	);
	const widgetResource = plan.resources.find(
		(resource) => resource.kind === "widget",
	);
	const pageRecord = pageResource
		? page(pageResource.physical_id, mainId)
		: undefined;
	const widgetRecord = widgetResource
		? widget(widgetResource.physical_id)
		: undefined;
	const eventRecords = new Map<string, IEvent>();
	for (const resource of plan.resources) {
		if (resource.kind !== "event") continue;
		eventRecords.set(
			resource.physical_id,
			event(
				resource.physical_id,
				mainId,
				resource.config.event_type,
				pageResource?.physical_id,
			),
		);
	}

	const backend = {
		appState: {
			getAppAuthoritative: vi.fn(async () => currentApp),
			updateAppAuthoritative: vi.fn(async (next: IApp) => {
				calls.push(`app:${next.status}`);
				currentApp = next;
				if (
					next.status === IAppStatus.Active &&
					options.failAppActivationAfterCommit
				) {
					throw new Error("App activation response failed.");
				}
			}),
		},
		boardState: {
			getBoardAuthoritative: vi.fn(
				async (
					_appId: string,
					_boardId: string,
					version?: [number, number, number],
				) => ({ ...draftBoard, version: version ?? draftBoard.version }),
			),
			getFlowScriptAuthoritative: vi.fn(
				async (
					_appId: string,
					_boardId: string,
					version?: [number, number, number],
				) =>
					options.driftVersionedBoard && version
						? "event changed()"
						: "event entry()",
			),
			createBoardVersion: vi.fn(async () => {
				calls.push("version:board");
				return [1, 0, 0] as [number, number, number];
			}),
		},
		pageState: {
			getPageAuthoritative: vi.fn(
				async (
					_appId: string,
					_pageId: string,
					_boardId?: string,
					version?: [number, number, number],
				) => {
					if (!pageRecord) throw new Error("Fixture has no Page.");
					return { ...pageRecord, version };
				},
			),
		},
		widgetState: {
			getWidgetAuthoritative: vi.fn(
				async (
					_appId: string,
					_widgetId: string,
					version?: [number, number, number],
				) => {
					if (!widgetRecord) throw new Error("Fixture has no Widget.");
					return { ...widgetRecord, version };
				},
			),
			createWidgetVersion: vi.fn(async () => {
				calls.push("version:widget");
				return [2, 0, 0] as [number, number, number];
			}),
		},
		eventState: {
			getEventAuthoritative: vi.fn(async (_appId: string, eventId: string) => {
				const found = eventRecords.get(eventId);
				if (!found) throw new Error(`Missing Event '${eventId}'.`);
				return { ...found };
			}),
			upsertEvent: vi.fn(async (_appId: string, next: IEvent) => {
				const resource = plan.resources.find(
					(candidate) =>
						candidate.kind === "event" && candidate.physical_id === next.id,
				);
				const previous = eventRecords.get(next.id);
				const operation = next.active
					? "activate"
					: next.board_version && !previous?.board_version
						? "pin"
						: "disable";
				calls.push(`${operation}:${resource?.key}`);
				if (next.active && resource?.key === options.failEventKey) {
					throw new Error(
						`Activation failed for '${resource?.key ?? next.id}'.`,
					);
				}
				const saved = {
					...next,
					event_version: next.active ? [3, 0, 0] : next.event_version,
				};
				eventRecords.set(next.id, saved);
				if (
					operation === "pin" &&
					resource?.key === options.failEventPinAfterCommitKey
				) {
					throw new Error(
						`Pin response failed for '${resource?.key ?? next.id}'.`,
					);
				}
				if (next.active && resource?.key === options.failEventAfterCommitKey) {
					throw new Error(
						`Activation response failed for '${resource?.key ?? next.id}'.`,
					);
				}
				return { ...saved };
			}),
		},
		dbState: {
			listTablesAuthoritative: vi.fn(async () =>
				plan.resources
					.filter((resource) => resource.kind === "table")
					.map((resource) => resource.physical_id),
			),
		},
	} as unknown as IBackendState;

	return { backend, calls, eventRecords, getCurrentApp: () => currentApp };
}

describe("beginAppBuildStaging", () => {
	test("moves an empty active app to inactive and verifies the transition", async () => {
		const emptyBoard = board("empty_board");
		const initial = { ...app(IAppStatus.Active), boards: [emptyBoard.id] };
		const fixture = stagingBackend({ app: initial, boards: [emptyBoard] });

		const result = await beginAppBuildStaging(fixture.backend, initial.id);

		expect(result.status).toBe("staged");
		expect(result.app.status).toBe(IAppStatus.Inactive);
		expect(result.inventory.board_ids).toEqual([emptyBoard.id]);
		expect(result.inventory.nonempty_board_ids).toEqual([]);
		expect(fixture.updateApp).toHaveBeenCalledTimes(1);
	});

	test("refuses public or nonempty active apps without changing status", async () => {
		const publicFixture = stagingBackend({
			app: app(IAppStatus.Active, IAppVisibility.Public),
		});
		await expect(
			beginAppBuildStaging(publicFixture.backend, "app_fixture"),
		).rejects.toMatchObject({
			code: "APP_BUILD_PUBLIC_APP",
		});
		expect(publicFixture.updateApp).not.toHaveBeenCalled();

		const nonemptyFixture = stagingBackend({
			app: app(IAppStatus.Active),
			pages: [{ pageId: "existing_page" }],
		});
		await expect(
			beginAppBuildStaging(nonemptyFixture.backend, "app_fixture"),
		).rejects.toMatchObject({
			code: "APP_BUILD_NONEMPTY_APP",
		});
		expect(nonemptyFixture.updateApp).not.toHaveBeenCalled();
	});

	test("treats an empty inactive app as a resumable staging target", async () => {
		const fixture = stagingBackend({
			app: app(IAppStatus.Inactive),
		});

		const result = await beginAppBuildStaging(fixture.backend, "app_fixture");

		expect(result.status).toBe("already_staged");
		expect(result.inventory.page_ids).toEqual([]);
		expect(fixture.updateApp).not.toHaveBeenCalled();
	});

	test("refuses to adopt a nonempty inactive app as a new staging target", async () => {
		const fixture = stagingBackend({
			app: app(IAppStatus.Inactive),
			pages: [{ pageId: "existing_page" }],
		});

		await expect(
			beginAppBuildStaging(fixture.backend, "app_fixture"),
		).rejects.toMatchObject({ code: "APP_BUILD_NONEMPTY_APP" });
		expect(fixture.updateApp).not.toHaveBeenCalled();
	});

	test("checks the request fence after inventory and before deactivation", async () => {
		const fixture = stagingBackend({ app: app(IAppStatus.Active) });
		let checks = 0;

		await expect(
			beginAppBuildStaging(fixture.backend, "app_fixture", () => {
				checks += 1;
				if (checks >= 5) throw new Error("request expired");
			}),
		).rejects.toThrow("request expired");
		expect(fixture.updateApp).not.toHaveBeenCalled();
	});
});

describe("promoteAppBuildResources", () => {
	test("pins Events inactive before exposing the app, then activates Events", async () => {
		const plan = fullPlan();
		const fixture = promotionBackend(plan);

		const result = await promoteAppBuildResources(
			fixture.backend,
			plan,
			() => undefined,
		);

		expect(result.status).toBe("complete");
		expect(result.version_receipt.boards.main.version).toEqual([1, 0, 0]);
		expect(result.version_receipt.pages.home.board_version).toEqual([1, 0, 0]);
		expect(result.version_receipt.widgets.card.version).toEqual([2, 0, 0]);
		expect(result.version_receipt.events.launch).toMatchObject({
			event_version: [3, 0, 0],
			board_version: [1, 0, 0],
		});
		expect(fixture.calls.indexOf("version:board")).toBeLessThan(
			fixture.calls.indexOf("activate:launch"),
		);
		expect(fixture.calls.indexOf("version:widget")).toBeLessThan(
			fixture.calls.indexOf("activate:launch"),
		);
		expect(fixture.calls.indexOf("pin:launch")).toBeLessThan(
			fixture.calls.indexOf(`app:${IAppStatus.Active}`),
		);
		expect(fixture.calls.indexOf(`app:${IAppStatus.Active}`)).toBeLessThan(
			fixture.calls.indexOf("activate:launch"),
		);
		expect(result.external_effects_may_have_occurred).toBe(true);
		expect(fixture.getCurrentApp().status).toBe(IAppStatus.Active);
	});

	test("stops on version drift without exposing the app or any Event", async () => {
		const plan = fullPlan();
		const fixture = promotionBackend(plan, { driftVersionedBoard: true });

		const result = await promoteAppBuildResources(
			fixture.backend,
			plan,
			() => undefined,
		);

		expect(result.status).toBe("partial");
		expect(result.error).toContain(
			"changed while its immutable version was created",
		);
		expect(result.activated_event_ids).toEqual([]);
		expect(result.external_effects_may_have_occurred).toBe(false);
		expect(result.rollback.attempted).toBe(false);
		expect(result.rollback.disabled_event_ids).toEqual([]);
		expect(fixture.getCurrentApp().status).toBe(IAppStatus.Inactive);
		expect(
			[...fixture.eventRecords.values()].every((item) => !item.active),
		).toBe(true);
	});

	test("disables an Event activated before a later activation fails", async () => {
		const plan = rollbackPlan();
		const fixture = promotionBackend(plan, { failEventKey: "second" });

		const result = await promoteAppBuildResources(
			fixture.backend,
			plan,
			() => undefined,
		);

		const firstId = resourceId(plan, "first");
		expect(result.status).toBe("partial");
		expect(result.attempted_event_ids).toHaveLength(2);
		expect(result.activated_event_ids).toEqual([firstId]);
		expect(result.external_effects_may_have_occurred).toBe(true);
		expect(result.rollback).toMatchObject({
			attempted: true,
			disabled_event_ids: [firstId],
			failed_event_ids: [],
		});
		expect(fixture.calls).toContain("disable:first");
		expect(
			[...fixture.eventRecords.values()].every((item) => !item.active),
		).toBe(true);
		expect(fixture.getCurrentApp().status).toBe(IAppStatus.Inactive);
	});

	test("keeps a committed Event pin inactive when its response fails", async () => {
		const plan = rollbackPlan();
		const fixture = promotionBackend(plan, {
			failEventPinAfterCommitKey: "first",
		});

		const result = await promoteAppBuildResources(
			fixture.backend,
			plan,
			() => undefined,
		);

		expect(result.status).toBe("partial");
		expect(result.attempted_event_ids).toEqual([]);
		expect(result.external_effects_may_have_occurred).toBe(false);
		expect(result.rollback.attempted).toBe(false);
		expect(
			[...fixture.eventRecords.values()].every((item) => !item.active),
		).toBe(true);
		expect(fixture.getCurrentApp().status).toBe(IAppStatus.Inactive);
	});

	test("compensates when app activation commits before its response fails", async () => {
		const plan = fullPlan();
		const fixture = promotionBackend(plan, {
			failAppActivationAfterCommit: true,
		});

		const result = await promoteAppBuildResources(
			fixture.backend,
			plan,
			() => undefined,
		);

		expect(result.status).toBe("partial");
		expect(result.attempted_event_ids).toEqual([]);
		expect(result.external_effects_may_have_occurred).toBe(true);
		expect(result.rollback).toMatchObject({
			attempted: true,
			app_deactivated: true,
			disabled_event_ids: [],
			failed_event_ids: [],
		});
		expect(fixture.getCurrentApp().status).toBe(IAppStatus.Inactive);
	});

	test("detects and disables an Event when activation commits before the response fails", async () => {
		const plan = rollbackPlan();
		const fixture = promotionBackend(plan, {
			failEventAfterCommitKey: "first",
		});

		const result = await promoteAppBuildResources(
			fixture.backend,
			plan,
			() => undefined,
		);

		const firstId = resourceId(plan, "first");
		expect(result.status).toBe("partial");
		expect(result.attempted_event_ids).toEqual([firstId]);
		expect(result.activated_event_ids).toEqual([firstId]);
		expect(result.external_effects_may_have_occurred).toBe(true);
		expect(result.rollback).toMatchObject({
			attempted: true,
			disabled_event_ids: [firstId],
			failed_event_ids: [],
		});
		expect(fixture.calls).toContain("disable:first");
		expect(
			[...fixture.eventRecords.values()].every((item) => !item.active),
		).toBe(true);
		expect(fixture.getCurrentApp().status).toBe(IAppStatus.Inactive);
	});
});
