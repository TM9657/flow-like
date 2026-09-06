import { describe, expect, test } from "vitest";
import { upsertAppEvent } from "./event-tools";

function fixture(
	options: {
		routeFails?: boolean;
		cancelled?: boolean;
		responseLost?: boolean;
	} = {},
) {
	const saved: Record<string, unknown>[] = [];
	const routes: string[] = [];
	const backend = {
		eventState: {
			getEvents: async () => [],
			getEventsAuthoritative: async () => [],
			getEvent: async () => {
				throw new Error("not found");
			},
			upsertEvent: async (_app: string, event: Record<string, unknown>) => {
				saved.push(event);
				if (options.responseLost) throw new Error("response lost after write");
				return event;
			},
		},
		boardState: {},
		routeState: {
			setRoute: async (_app: string, route: string) => {
				if (options.routeFails) throw new Error("route store unavailable");
				routes.push(route);
			},
		},
	} as unknown as Parameters<typeof upsertAppEvent>[0];
	return {
		backend,
		saved,
		routes,
		context: {
			assertActive() {
				if (options.cancelled) throw new Error("cancelled");
			},
			referenceApp() {},
		},
	};
}

describe("Event provisioning receipts", () => {
	test("lost write responses preserve the attempted Event ID and unknown outcome", async () => {
		const f = fixture({ responseLost: true });
		const result = await upsertAppEvent(
			f.backend,
			{ app_id: "app", name: "Home", page_id: "page", route: "home" },
			f.context,
		);
		expect(result).toMatchObject({
			status: "unknown",
			code: "event_write_outcome_unknown",
			event_id: f.saved[0].id,
			provisioning: { event_saved: "unknown", route_applied: false },
		});
		expect(f.routes).toHaveLength(0);
	});
	test("route failure is partial and keeps the persisted Event identity", async () => {
		const f = fixture({ routeFails: true });
		const result = await upsertAppEvent(
			f.backend,
			{ app_id: "app", name: "Home", page_id: "page", route: "home" },
			f.context,
		);
		expect(result.status).toBe("partial");
		expect(result.code).toBe("event_route_incomplete");
		expect(result.event_id).toBe(f.saved[0].id);
		expect(result.provisioning).toEqual({
			event_saved: true,
			route_applied: false,
		});
	});
	test("build host reserves an inactive Event before its first write", async () => {
		const f = fixture();
		const result = await upsertAppEvent(
			f.backend,
			{ app_id: "app", name: "Home", page_id: "page", active: false },
			{ ...f.context, reservedEventId: "reserved" },
		);
		expect(result.status).toBe("ok");
		expect(f.saved[0].id).toBe("reserved");
		expect(f.saved[0].active).toBe(false);
	});
	test("ordinary updates do not reinterpret a missing Event as creation", async () => {
		const f = fixture();
		const result = await upsertAppEvent(
			f.backend,
			{ app_id: "app", name: "Home", page_id: "page", event_id: "missing" },
			f.context,
		);
		expect(result.status).toBe("error");
		expect(f.saved).toHaveLength(0);
	});
	test("expired requests cannot persist an Event", async () => {
		const f = fixture({ cancelled: true });
		const result = await upsertAppEvent(
			f.backend,
			{ app_id: "app", name: "Home", page_id: "page" },
			f.context,
		);
		expect(result.status).toBe("error");
		expect(f.saved).toHaveLength(0);
	});
});
