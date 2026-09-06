import { beforeEach, describe, expect, test, vi } from "vitest";

const mocks = vi.hoisted(() => ({
	invoke: vi.fn(),
	fetcher: vi.fn(),
}));

vi.mock("@tauri-apps/api/core", () => ({
	invoke: mocks.invoke,
}));

vi.mock("../api", () => ({
	fetcher: mocks.fetcher,
}));

vi.mock("../apps-db", () => ({
	appsDB: {
		visibility: { get: vi.fn(), put: vi.fn() },
	},
}));

import { WidgetState } from "../../components/tauri-provider/widget-state";
import { DatabaseState } from "../../components/tauri-provider/db-state";
import { RouteState } from "../../components/tauri-provider/route-state";

function hostedBackend() {
	return {
		isLocalOnly: vi.fn().mockResolvedValue(false),
		profile: { hub: "hub.example" },
		auth: { isAuthenticated: true, user: { access_token: "token" } },
	};
}

function localBackend() {
	return {
		isLocalOnly: vi.fn().mockResolvedValue(true),
		profile: { hub: "hub.example" },
		auth: { isAuthenticated: true, user: { access_token: "token" } },
	};
}

describe("authoritative widget reads", () => {
	beforeEach(() => {
		vi.clearAllMocks();
	});

	test("propagates hosted inventory failures without consulting local storage", async () => {
		const failure = new Error("hub unavailable");
		mocks.fetcher.mockRejectedValueOnce(failure);
		const state = new WidgetState(hostedBackend() as never);

		await expect(state.getWidgetsAuthoritative("app-1", "en")).rejects.toBe(
			failure,
		);
		expect(mocks.fetcher).toHaveBeenCalledWith(
			{ hub: "hub.example" },
			"apps/app-1/widgets?language=en",
			{ method: "GET" },
			expect.objectContaining({ isAuthenticated: true }),
		);
		expect(mocks.invoke).not.toHaveBeenCalled();
	});

	test("reads an exact hosted version without caching or pushing", async () => {
		const widget = { id: "widget-1", components: [] };
		mocks.fetcher.mockResolvedValueOnce(widget);
		const state = new WidgetState(hostedBackend() as never);

		await expect(
			state.getWidgetAuthoritative("app-1", "widget-1", [3, 2, 1]),
		).resolves.toBe(widget);
		expect(mocks.fetcher.mock.calls[0][1]).toBe(
			"apps/app-1/widgets/widget-1?version=3_2_1",
		);
		expect(mocks.invoke).not.toHaveBeenCalled();
	});

	test("does not replace a missing local-only version with remote or current data", async () => {
		const failure = new Error("version missing");
		mocks.invoke.mockRejectedValueOnce(failure);
		const state = new WidgetState(localBackend() as never);

		await expect(
			state.getWidgetAuthoritative("app-1", "widget-1", [3, 2, 1]),
		).rejects.toBe(failure);
		expect(mocks.invoke).toHaveBeenCalledTimes(1);
		expect(mocks.invoke).toHaveBeenCalledWith("get_widget", {
			appId: "app-1",
			widgetId: "widget-1",
			version: [3, 2, 1],
		});
		expect(mocks.fetcher).not.toHaveBeenCalled();
	});
});

describe("authoritative database and route reads", () => {
	beforeEach(() => {
		vi.clearAllMocks();
	});

	test("propagates hosted database inventory failures without native fallback", async () => {
		const failure = new Error("hub unavailable");
		mocks.fetcher.mockRejectedValueOnce(failure);
		const state = new DatabaseState(hostedBackend() as never);

		await expect(state.listTablesAuthoritative("app-1")).rejects.toBe(failure);
		expect(mocks.fetcher.mock.calls[0][1]).toBe("apps/app-1/db");
		expect(mocks.invoke).not.toHaveBeenCalled();
	});

	test("reads one hosted schema without consulting native storage", async () => {
		const schema = { fields: [] };
		mocks.fetcher.mockResolvedValueOnce(schema);
		const state = new DatabaseState(hostedBackend() as never);

		await expect(
			state.getSchemaAuthoritative("app-1", "support tickets"),
		).resolves.toBe(schema);
		expect(mocks.fetcher.mock.calls[0][1]).toBe(
			"apps/app-1/db/support%20tickets/schema",
		);
		expect(mocks.invoke).not.toHaveBeenCalled();
	});

	test("keeps local-only schema reads native", async () => {
		const schema = { fields: [] };
		mocks.invoke.mockResolvedValueOnce(schema);
		const state = new DatabaseState(localBackend() as never);

		await expect(
			state.getSchemaAuthoritative("app-1", "tickets"),
		).resolves.toBe(schema);
		expect(mocks.invoke).toHaveBeenCalledWith("db_schema", {
			appId: "app-1",
			tableName: "tickets",
			userScoped: false,
		});
		expect(mocks.fetcher).not.toHaveBeenCalled();
	});

	test("propagates hosted route failures without local lookup or cache repair", async () => {
		const failure = new Error("hub unavailable");
		mocks.fetcher.mockRejectedValueOnce(failure);
		const state = new RouteState(hostedBackend() as never);

		await expect(
			state.getRouteByPathAuthoritative("app-1", "/support tickets"),
		).rejects.toBe(failure);
		expect(mocks.fetcher.mock.calls[0][1]).toBe(
			"apps/app-1/routes/by-path?path=%2Fsupport%20tickets",
		);
		expect(mocks.invoke).not.toHaveBeenCalled();
	});

	test("keeps local-only route reads native", async () => {
		const route = { path: "/", eventId: "event-1" };
		mocks.invoke.mockResolvedValueOnce(route);
		const state = new RouteState(localBackend() as never);

		await expect(state.getRouteByPathAuthoritative("app-1", "/")).resolves.toBe(
			route,
		);
		expect(mocks.invoke).toHaveBeenCalledWith("get_app_route_by_path", {
			appId: "app-1",
			path: "/",
		});
		expect(mocks.fetcher).not.toHaveBeenCalled();
	});
});
