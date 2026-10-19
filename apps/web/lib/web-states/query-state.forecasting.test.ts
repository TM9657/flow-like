import type {
	CreateStudioForecastPayload,
	StudioForecastPayload,
} from "@flow-like/flow-like-ui";
import { beforeEach, describe, expect, test, vi } from "vitest";

const mocks = vi.hoisted(() => ({
	apiPost: vi.fn(),
	apiGet: vi.fn(),
	apiDelete: vi.fn(),
}));
vi.mock("./api-utils", () => mocks);

import { WebQueryState } from "./query-state";

const payload: StudioForecastPayload = {
	profile_id: "profile-1",
	model_bit_id: "forecast-model",
	sql: 'SELECT * FROM "demand"',
	mapping: { timestamp: "date", targets: ["sales"] },
	horizon: 7,
	future_sql: 'SELECT * FROM "planned_promotions"',
	backtest_horizon: 2,
	backtest_offset: 3,
};

describe("web forecasting transport", () => {
	beforeEach(() => vi.resetAllMocks());

	test.each([false, true])(
		"preserves the model, profile and SQL request with user scope %s",
		async (userScoped) => {
			const backend = { auth: { isAuthenticated: true } };
			const state = new WebQueryState(backend as never);
			const reply = {
				run_id: "native-run",
				backtest: { horizon: 2, offset: 3, metrics: [{ wape: 12.5 }] },
			};
			mocks.apiPost.mockResolvedValue(reply);
			expect(await state.forecast("app-1", payload, userScoped)).toBe(reply);
			expect(mocks.apiPost).toHaveBeenCalledWith(
				`apps/app-1/db/queries/forecast${userScoped ? "?scope=user" : ""}`,
				payload,
				backend.auth,
			);
			mocks.apiPost.mockRejectedValueOnce(new Error("Forecasting unavailable"));
			await expect(
				state.forecast("app-1", payload, userScoped),
			).rejects.toThrow("Forecasting unavailable");
		},
	);
});

describe("saved prediction transport", () => {
	beforeEach(() => vi.resetAllMocks());

	test("treats authored app IDs as a single path segment for every snapshot operation", async () => {
		const backend = { auth: {} };
		const state = new WebQueryState(backend as never);
		const appId = "app/other?scope=project#fragment";
		const base = `apps/${encodeURIComponent(appId)}/db/forecasts`;
		const { profile_id: _profile, ...request } = payload;
		const snapshot: CreateStudioForecastPayload = {
			name: "Forecast",
			request,
			result: { run_id: "run" } as never,
		};
		mocks.apiGet
			.mockResolvedValueOnce([])
			.mockResolvedValueOnce({ id: "saved/1" });
		await state.listForecasts(appId, true);
		await state.getForecast(appId, "saved/1", true);
		await state.createForecast(appId, snapshot, true);
		await state.deleteForecast(appId, "saved/1", true);
		expect(mocks.apiGet.mock.calls.map(([url]) => url)).toEqual([
			`${base}?limit=100&offset=0&scope=user`,
			`${base}/saved%2F1?scope=user`,
		]);
		expect(mocks.apiPost).toHaveBeenCalledWith(
			`${base}?scope=user`,
			snapshot,
			backend.auth,
		);
		expect(mocks.apiDelete).toHaveBeenCalledWith(
			`${base}/saved%2F1?scope=user`,
			backend.auth,
		);
	});

	test("checks tab availability without loading the whole library", async () => {
		const state = new WebQueryState({ auth: {} } as never);
		mocks.apiGet.mockResolvedValue([{ id: "first" }]);
		expect(await state.listForecasts("app-1", false, 1)).toEqual([
			{ id: "first" },
		]);
		expect(mocks.apiGet).toHaveBeenCalledExactlyOnceWith(
			"apps/app-1/db/forecasts?limit=1&offset=0",
			{},
		);
	});

	test.each([false, true])(
		"keeps saved predictions in scope %s",
		async (userScoped) => {
			const backend = { auth: { isAuthenticated: true } };
			const state = new WebQueryState(backend as never);
			const { profile_id: _profile, ...request } = payload;
			const snapshot: CreateStudioForecastPayload = {
				name: "Weekly demand",
				request,
				result: { run_id: "run-1" } as never,
			};
			const saved = { id: "prediction/1", ...snapshot };
			const suffix = userScoped ? "?scope=user" : "";
			mocks.apiPost.mockResolvedValue(saved);
			expect(await state.createForecast("app-1", snapshot, userScoped)).toBe(
				saved,
			);
			expect(mocks.apiPost).toHaveBeenCalledWith(
				`apps/app-1/db/forecasts${suffix}`,
				snapshot,
				backend.auth,
			);
			mocks.apiGet.mockResolvedValue(saved);
			expect(await state.getForecast("app-1", saved.id, userScoped)).toBe(
				saved,
			);
			expect(mocks.apiGet).toHaveBeenLastCalledWith(
				`apps/app-1/db/forecasts/prediction%2F1${suffix}`,
				backend.auth,
			);
			await state.deleteForecast("app-1", saved.id, userScoped);
			expect(mocks.apiDelete).toHaveBeenCalledWith(
				`apps/app-1/db/forecasts/prediction%2F1${suffix}`,
				backend.auth,
			);
			mocks.apiGet.mockRejectedValueOnce(new Error("Prediction not found"));
			await expect(
				state.getForecast("app-1", saved.id, userScoped),
			).rejects.toThrow("Prediction not found");
		},
	);

	test("lists later pages without duplicating an item changed between pages", async () => {
		const state = new WebQueryState({ auth: {} } as never);
		const first = Array.from({ length: 100 }, (_, id) => ({ id: String(id) }));
		mocks.apiGet
			.mockResolvedValueOnce(first)
			.mockResolvedValueOnce([{ id: "99" }, { id: "100" }]);
		const result = await state.listForecasts("app-1", true);
		expect(result).toHaveLength(101);
		expect(result.at(-1)?.id).toBe("100");
		expect(mocks.apiGet.mock.calls.map(([url]) => url)).toEqual([
			"apps/app-1/db/forecasts?limit=100&offset=0&scope=user",
			"apps/app-1/db/forecasts?limit=100&offset=100&scope=user",
		]);
	});
});
