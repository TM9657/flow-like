import { describe, expect, test, vi } from "vitest";
import type { IBackendState } from "../../state/backend-state";
import {
	type AppBuildEventBinding,
	provisionAppBuildEvents,
} from "./resource-events";

const binding: AppBuildEventBinding = {
	id: "reserved",
	name: "Submit",
	event_type: "quick_action",
	board_id: "board",
	entry_node: "submitTicket",
};

function fixture() {
	const events = new Map<string, Record<string, unknown>>();
	const routes = new Map<string, string>();
	let pageBoard = "board";
	const nodes = {
		entry: {
			id: "entry",
			name: "events_simple",
			friendly_name: "submitTicket",
			pins: {
				out: {
					id: "out",
					pin_type: "Output",
					data_type: "Execution",
					connected_to: ["in"],
				},
			},
		},
		log: {
			id: "log",
			name: "log_info",
			pins: {
				in: {
					id: "in",
					pin_type: "Input",
					data_type: "Execution",
					connected_to: ["out"],
				},
			},
		},
	};
	const backend = {
		appState: { getAppAuthoritative: async () => ({ status: "Inactive" }) },
		boardState: {
			getBoardAuthoritative: async () => ({
				id: "board",
				execution_mode: "Local",
				nodes,
			}),
		},
		pageState: {
			getPageAuthoritative: async () => ({ id: "page", boardId: pageBoard }),
		},
		eventState: {
			getEventsAuthoritative: async () => [...events.values()],
			getEventAuthoritative: async (_app: string, id: string) => events.get(id),
			upsertEvent: vi.fn(
				async (_app: string, event: Record<string, unknown>) => {
					events.set(String(event.id), event);
					return event;
				},
			),
		},
		routeState: {
			setRoute: async (_app: string, path: string, id: string) => {
				routes.set(path, id);
			},
			getRouteByPathAuthoritative: async (_app: string, path: string) => ({
				eventId: routes.get(path),
			}),
		},
	} as unknown as IBackendState;
	return {
		backend,
		events,
		routes,
		nodes,
		wrongPageBoard: () => {
			pageBoard = "other";
		},
	};
}

describe("host Event binding", () => {
	test("binds a stable ID to the exact persisted runnable entry and verifies inactive readback", async () => {
		const f = fixture();
		const evidence = await provisionAppBuildEvents(
			f.backend,
			"app",
			[binding],
			{ assertActive() {} },
		);
		expect(evidence[0]).toMatchObject({
			id: "reserved",
			board_id: "board",
			node_id: "entry",
			active: false,
		});
		expect(f.events.size).toBe(1);
		await provisionAppBuildEvents(f.backend, "app", [binding], {
			assertActive() {},
		});
		expect(f.events.size).toBe(1);
	});
	test("preflights every target before writing either Event", async () => {
		const f = fixture();
		f.wrongPageBoard();
		await expect(
			provisionAppBuildEvents(
				f.backend,
				"app",
				[
					binding,
					{
						id: "page-event",
						name: "Page",
						event_type: "page",
						board_id: "board",
						page_id: "page",
					},
				],
				{ assertActive() {} },
			),
		).rejects.toThrow("does not belong");
		expect(f.backend.eventState.upsertEvent).not.toHaveBeenCalled();
	});
	test("page bindings preserve a distinct reserved ID and route without an entry node", async () => {
		const f = fixture();
		const evidence = await provisionAppBuildEvents(
			f.backend,
			"app",
			[
				{
					id: "page-event",
					name: "Page",
					event_type: "page",
					board_id: "board",
					page_id: "page",
					route: "/intake",
				},
			],
			{ assertActive() {} },
		);
		expect(evidence[0]).toMatchObject({
			id: "page-event",
			page_id: "page",
			active: false,
			route: "/intake",
		});
		expect(f.events.get("page-event")?.node_id).toBeFalsy();
		expect(f.routes.get("/intake")).toBe("page-event");
	});
	test("a disconnected entry cannot create an Event", async () => {
		const f = fixture();
		f.nodes.entry.pins.out.connected_to = [];
		await expect(
			provisionAppBuildEvents(f.backend, "app", [binding], {
				assertActive() {},
			}),
		).rejects.toThrow("0 compatible");
		expect(f.events.size).toBe(0);
	});
	test("authoritative readback must match the selected entry", async () => {
		const f = fixture();
		f.backend.eventState.getEventAuthoritative = async () =>
			({ ...f.events.get("reserved"), node_id: "other" }) as never;
		await expect(
			provisionAppBuildEvents(f.backend, "app", [binding], {
				assertActive() {},
			}),
		).rejects.toThrow("did not read back");
	});
	test("already active Events are not silently deactivated or reassigned", async () => {
		const f = fixture();
		f.events.set("reserved", { id: "reserved", active: true });
		await expect(
			provisionAppBuildEvents(f.backend, "app", [binding], {
				assertActive() {},
			}),
		).rejects.toThrow("already active");
		expect(f.backend.eventState.upsertEvent).not.toHaveBeenCalled();
	});
});
