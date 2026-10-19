import type {
	CreateStudioForecastPayload,
	StudioForecastPayload,
} from "@flow-like/flow-like-ui";
import { beforeEach, describe, expect, test, vi } from "vitest";

const mocks = vi.hoisted(() => ({ invoke: vi.fn(), fetcher: vi.fn() }));
vi.mock("@tauri-apps/api/core", () => ({ invoke: mocks.invoke }));
vi.mock("../../../lib/api", () => ({ fetcher: mocks.fetcher }));

import { QueryState } from "../query-state";

const payload: StudioForecastPayload = {
	profile_id: "profile-1",
	model_bit_id: "forecast-model",
	sql: 'SELECT * FROM "demand"',
	mapping: { timestamp: "date", targets: ["sales"] },
	horizon: 7,
	backtest_horizon: 2,
	backtest_offset: 3,
};

describe("desktop forecasting transport", () => {
	beforeEach(() => vi.resetAllMocks());

	test.each([false, true])(
		"uses the authorized hosted route with user scope %s",
		async (userScoped) => {
			const backend = {
				isOffline: vi.fn().mockResolvedValue(false),
				profile: { id: "profile-1" },
				auth: { isAuthenticated: true },
			};
			const reply = {
				run_id: "native-run",
				backtest: { horizon: 2, offset: 3, metrics: [{ wape: 12.5 }] },
			};
			mocks.fetcher.mockResolvedValue(reply);
			const state = new QueryState(backend as never);
			expect(await state.forecast("app-1", payload, userScoped)).toBe(reply);
			expect(mocks.fetcher).toHaveBeenCalledWith(
				backend.profile,
				`apps/app-1/db/queries/forecast${userScoped ? "?scope=user" : ""}`,
				{ method: "POST", body: JSON.stringify(payload) },
				backend.auth,
			);
			expect(mocks.invoke).not.toHaveBeenCalled();
			mocks.fetcher.mockRejectedValueOnce(new Error("Model access denied"));
			await expect(
				state.forecast("app-1", payload, userScoped),
			).rejects.toThrow("Model access denied");
		},
	);

	test.each([undefined, true])(
		"keeps device forecasts on the native command with user scope %s",
		async (userScoped) => {
			const state = new QueryState({
				isOffline: vi.fn().mockResolvedValue(true),
			} as never);
			const reply = {
				run_id: "device-run",
				backtest: { horizon: 2, offset: 3, metrics: [{ wape: 12.5 }] },
			};
			mocks.invoke.mockResolvedValue(reply);
			expect(await state.forecast("app-1", payload, userScoped)).toBe(reply);
			expect(mocks.invoke).toHaveBeenCalledWith("query_forecast", {
				appId: "app-1",
				payload,
				userScoped: userScoped ?? false,
			});
			expect(mocks.fetcher).not.toHaveBeenCalled();
			mocks.invoke.mockRejectedValueOnce(new Error("Forecasting unavailable"));
			await expect(
				state.forecast("app-1", payload, userScoped),
			).rejects.toThrow("Forecasting unavailable");
		},
	);
});

describe("desktop saved predictions", () => {
	beforeEach(() => vi.resetAllMocks());

	test("encodes authored app IDs in every hosted snapshot route", async () => {
		const backend = {
			isOffline: vi.fn().mockResolvedValue(false),
			profile: { id: "profile-1" },
			auth: {},
		};
		const state = new QueryState(backend as never);
		const appId = "app/other?scope=project#fragment";
		const base = `apps/${encodeURIComponent(appId)}/db/forecasts`;
		const { profile_id: _profile, ...request } = payload;
		const snapshot: CreateStudioForecastPayload = {
			name: "Forecast",
			request,
			result: { run_id: "run" } as never,
		};
		mocks.fetcher
			.mockResolvedValueOnce([])
			.mockResolvedValue({ id: "saved/1" });
		await state.listForecasts(appId, true);
		await state.getForecast(appId, "saved/1", true);
		await state.createForecast(appId, snapshot, true);
		await state.deleteForecast(appId, "saved/1", true);
		expect(
			mocks.fetcher.mock.calls.map(([, url, options]) => [url, options.method]),
		).toEqual([
			[`${base}?limit=100&offset=0&scope=user`, "GET"],
			[`${base}/saved%2F1?scope=user`, "GET"],
			[`${base}?scope=user`, "POST"],
			[`${base}/saved%2F1?scope=user`, "DELETE"],
		]);
		expect(mocks.invoke).not.toHaveBeenCalled();
		expect(backend.isOffline).toHaveBeenCalledWith(appId);
	});

	test.each([false, true])(
		"routes snapshot operations with offline=%s",
		async (offline) => {
			const backend = {
				isOffline: vi.fn().mockResolvedValue(offline),
				profile: { id: "profile-1" },
				auth: {},
			};
			const state = new QueryState(backend as never);
			const { profile_id: _profile, ...request } = payload;
			const snapshot: CreateStudioForecastPayload = {
				name: "Weekly demand",
				request,
				result: { run_id: "run-1" } as never,
			};
			const saved = { id: "forecast/1", ...snapshot };
			mocks.invoke.mockResolvedValue(saved);
			mocks.fetcher.mockResolvedValue(saved);
			expect(await state.createForecast("app-1", snapshot, true)).toBe(saved);
			expect(await state.getForecast("app-1", saved.id, true)).toBe(saved);
			await state.deleteForecast("app-1", saved.id, true);
			if (offline) {
				expect(mocks.fetcher).not.toHaveBeenCalled();
				expect(mocks.invoke.mock.calls).toEqual([
					[
						"studio_forecast_create",
						{ appId: "app-1", payload: snapshot, userScoped: true },
					],
					[
						"studio_forecast_get",
						{ appId: "app-1", forecastId: saved.id, userScoped: true },
					],
					[
						"studio_forecast_delete",
						{ appId: "app-1", forecastId: saved.id, userScoped: true },
					],
				]);
			} else {
				expect(mocks.invoke).not.toHaveBeenCalled();
				expect(
					mocks.fetcher.mock.calls.map(([, url, options]) => [url, options]),
				).toEqual([
					[
						"apps/app-1/db/forecasts?scope=user",
						{ method: "POST", body: JSON.stringify(snapshot) },
					],
					[
						"apps/app-1/db/forecasts/forecast%2F1?scope=user",
						{ method: "GET" },
					],
					[
						"apps/app-1/db/forecasts/forecast%2F1?scope=user",
						{ method: "DELETE" },
					],
				]);
			}
		},
	);

	test.each([false, true])(
		"lists all snapshot pages with offline=%s",
		async (offline) => {
			const backend = {
				isOffline: vi.fn().mockResolvedValue(offline),
				profile: { id: "profile-1" },
				auth: {},
			};
			const state = new QueryState(backend as never);
			const transport = offline ? mocks.invoke : mocks.fetcher;
			transport
				.mockResolvedValueOnce(
					Array.from({ length: 100 }, (_, id) => ({ id: String(id) })),
				)
				.mockResolvedValueOnce([{ id: "100" }]);
			expect(await state.listForecasts("app-1", true)).toHaveLength(101);
			if (offline) {
				expect(mocks.invoke).toHaveBeenLastCalledWith("studio_forecast_list", {
					appId: "app-1",
					userScoped: true,
					limit: 100,
					offset: 100,
				});
			} else {
				expect(mocks.fetcher).toHaveBeenLastCalledWith(
					backend.profile,
					"apps/app-1/db/forecasts?limit=100&offset=100&scope=user",
					{ method: "GET" },
					backend.auth,
				);
			}
		},
	);
});
