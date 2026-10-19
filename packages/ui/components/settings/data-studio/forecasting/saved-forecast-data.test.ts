import { describe, expect, test } from "bun:test";
import type { StudioForecastResult } from "../../../../state/backend-state/query-state";
import {
	savedForecastHref,
	savedForecastPresentation,
} from "./saved-forecast-data";

function result(): StudioForecastResult {
	const predictions = (timestamp: number) =>
		["north", "south"].flatMap((series_id) =>
			["sales", "power"].map((target) => ({
				series_id,
				target,
				timestamp_ms: timestamp,
				point: 12,
				quantiles: [9, 12, 15],
				actual: timestamp < 4000 ? 10 : null,
			})),
		);
	return {
		run_id: "run",
		model: {
			bit_id: "model",
			family: "chronos2",
			repository: null,
			revision: null,
		},
		capabilities: {
			family: "chronos2",
			max_context: 100,
			max_horizon: 100,
			native_horizon: 10,
			quantile_levels: [0.1, 0.5, 0.9],
			joint_multivariate: true,
			past_covariates: true,
			known_covariates: true,
		},
		input_rows: 6,
		horizon: 1,
		quantile_levels: [0.1, 0.5, 0.9],
		point_statistic: "median",
		history: [1000, 2000, 3000].flatMap((timestamp) =>
			predictions(timestamp).map((point) => ({
				series_id: point.series_id,
				target: point.target,
				timestamp_ms: timestamp,
				value:
					point.series_id === "south" && point.target === "power" ? 100 : 10,
			})),
		),
		forecasts: predictions(4000),
		covariates: ["north", "south"].flatMap((series_id) =>
			(["past", "known"] as const).flatMap((kind) =>
				[1000, 2000, 3000, ...(kind === "known" ? [4000] : [])].map(
					(timestamp_ms) => ({
						series_id,
						timestamp_ms,
						name: kind,
						kind,
						value: 1,
						future: timestamp_ms === 4000,
					}),
				),
			),
		),
		backtest: {
			horizon: 1,
			forecasts: predictions(3000),
			metrics: [
				{
					series_id: "south",
					target: "power",
					observations: 1,
					mae: 88,
					rmse: 88,
					mape: 88,
				},
			],
		},
	};
}

describe("saved forecast display", () => {
	test("selects one series and target and preserves its saved quantile band", () => {
		const source = result();
		const before = structuredClone(source);
		const chart = savedForecastPresentation(source, {
			seriesId: "south",
			target: "power",
		});
		expect(chart.error).toBeUndefined();
		expect(chart.cutoff).toBe(3000);
		expect(chart.points.map((point) => point.history)).toEqual([
			100,
			100,
			100,
			null,
		]);
		expect(chart.points.at(-1)?.interval).toEqual([9, 15]);
		expect(chart.metrics?.mae).toBe(88);
		expect(source).toEqual(before);
	});
	test("holdout uses saved predictions and withholds past-only covariates", () => {
		const chart = savedForecastPresentation(result(), {
			seriesId: "south",
			target: "power",
			view: "evaluation",
		});
		expect(chart.cutoff).toBe(2000);
		expect(chart.evaluationWindow).toEqual({
			cutoff: 2000,
			start: 3000,
			end: 3000,
			steps: 1,
		});
		expect(chart.points.at(-1)).toMatchObject({
			timestamp: 3000,
			actual: 100,
			history: null,
			forecast: 12,
		});
		expect(
			chart.covariates
				.find((track) => track.role === "past")
				?.points.map((point) => point.timestamp),
		).toEqual([1000, 2000]);
		expect(
			chart.covariates
				.find((track) => track.role === "known")
				?.points.map((point) => point.timestamp),
		).toContain(3000);
	});
	test("historical evaluation clips later actuals and known covariates to its saved window", () => {
		const source = result();
		const firstHistory = source.history.filter(
			(point) => point.timestamp_ms === 1000,
		);
		source.history.push(
			...[4000, 5000, 6000].flatMap((timestamp_ms) =>
				firstHistory.map((point) => ({
					...point,
					timestamp_ms,
					value: timestamp_ms === 4000 ? null : point.value,
				})),
			),
		);
		source.forecasts = source.forecasts.map((point) => ({
			...point,
			timestamp_ms: 7000,
		}));
		if (!source.backtest) throw new Error("Expected a holdout fixture");
		source.backtest.horizon = 2;
		source.backtest.offset = 2;
		source.backtest.forecasts.push(
			...source.backtest.forecasts.map((point) => ({
				...point,
				timestamp_ms: 4000,
				actual: null,
			})),
		);
		source.covariates = source.covariates
			.filter((point) => point.timestamp_ms === 1000)
			.flatMap((point) =>
				[
					1000,
					2000,
					3000,
					4000,
					5000,
					6000,
					...(point.kind === "known" ? [7000] : []),
				].map((timestamp_ms) => ({
					...point,
					timestamp_ms,
					future: timestamp_ms === 7000,
				})),
			);
		const before = structuredClone(source);
		const chart = savedForecastPresentation(source, {
			seriesId: "south",
			target: "power",
			view: "evaluation",
		});
		expect(chart.evaluationWindow).toEqual({
			cutoff: 2000,
			start: 3000,
			end: 4000,
			steps: 2,
		});
		expect(chart.points.map((point) => point.timestamp)).toEqual([
			1000, 2000, 3000, 4000,
		]);
		expect(chart.points.at(-1)).toMatchObject({
			timestamp: 4000,
			forecast: 12,
			actual: null,
			history: null,
		});
		expect(
			chart.covariates
				.find((track) => track.role === "known")
				?.points.map((point) => point.timestamp),
		).toEqual([1000, 2000, 3000, 4000]);
		expect(
			chart.covariates
				.find((track) => track.role === "past")
				?.points.map((point) => point.timestamp),
		).toEqual([1000, 2000]);
		const outlook = savedForecastPresentation(source, {
			seriesId: "south",
			target: "power",
		});
		expect(outlook.cutoff).toBe(6000);
		expect(outlook.evaluationWindow).toEqual(chart.evaluationWindow);
		expect(outlook.points.at(-1)?.timestamp).toBe(7000);
		expect(source).toEqual(before);
	});
	test("invalid explicit selections and absent evaluation never silently show another result", () => {
		expect(
			savedForecastPresentation(result(), { target: "missing" }).error,
		).toContain("not part");
		expect(
			savedForecastPresentation(result(), { seriesId: "missing" }).error,
		).toContain("not part");
		expect(
			savedForecastPresentation(
				{ ...result(), backtest: null },
				{ view: "evaluation" },
			).error,
		).toContain("no saved holdout");
	});
	test("studio links preserve the saved scope and safely encode both identifiers", () => {
		const url = new URL(
			savedForecastHref("app&scope=user", "prediction#1", false),
			"https://example.com",
		);
		expect(url.pathname).toBe("/library/config/explore");
		expect(url.searchParams.get("id")).toBe("app&scope=user");
		expect(url.searchParams.get("forecast")).toBe("prediction#1");
		expect(url.searchParams.has("scope")).toBe(false);
		expect(savedForecastHref("app", "prediction", true)).toContain(
			"scope=user",
		);
	});
});
