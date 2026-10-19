import { type IBit, IBitTypes } from "../../packages/ui/lib/schema/bit/bit";
import type {
	ExecuteSqlResult,
	StudioForecastFamily,
	StudioForecastPayload,
	StudioForecastResult,
} from "../../packages/ui/state/backend-state/query-state";

export const machines = ["Machine A", "Machine B", "Machine C"];
const historyLength = 96;
const hour = 3_600_000;
const start = Date.UTC(2026, 9, 12);
const timestamp = (index: number) => start + index * hour;
const value = (index: number, machine: number) =>
	Math.round(
		(55 +
			machine * 5 +
			index * 0.04 +
			Math.sin(index / 3.8 + machine) * 3 +
			Math.cos(index * 1.7) * 0.6) *
			100,
	) / 100;

export const models: IBit[] = [
	["chronos2", "Chronos 2"],
	["timesfm2_5", "TimesFM 2.5"],
	["tiny_time_mixer", "IBM Granite TinyTimeMixer"],
].map(([family, name]) => ({
	id: `fixture-${family}`,
	type: IBitTypes.Forecasting,
	authors: [],
	created: "2026-10-12T00:00:00Z",
	updated: "2026-10-12T00:00:00Z",
	dependencies: [],
	dependency_tree_hash: "fixture-dependencies",
	hash: "fixture-model",
	hub: "fixture.invalid",
	parameters: { model_type: family },
	meta: {
		en: {
			name,
			description: "Browser fixture. Predictions are generated test data.",
			created_at: { secs_since_epoch: 0, nanos_since_epoch: 0 },
			updated_at: { secs_since_epoch: 0, nanos_since_epoch: 0 },
			preview_media: [],
			tags: [],
		},
	},
}));

export function preview(sql: string): ExecuteSqlResult {
	const overall = /\b(avg|sum)\s*\(/i.test(sql);
	const columns = [
		{
			name: "timestamp",
			type_name: "Timestamp(Millisecond, None)",
			position: 0,
		},
		...(!overall
			? [{ name: "machine_id", type_name: "Utf8", position: 1 }]
			: []),
		{ name: "temperature_c", type_name: "Float64", position: overall ? 1 : 2 },
		...(!overall
			? [{ name: "load_pct", type_name: "Float64", position: 3 }]
			: []),
	];
	const rows = Array.from({ length: historyLength }, (_, index) =>
		overall
			? [
					{
						timestamp: timestamp(index),
						temperature_c:
							machines.reduce(
								(sum, _, machine) => sum + value(index, machine),
								0,
							) / machines.length,
					},
				]
			: machines.map((machine_id, machine) => ({
					timestamp: timestamp(index),
					machine_id,
					temperature_c: value(index, machine),
					load_pct: Math.round(60 + machine * 7 + Math.sin(index / 8) * 15),
				})),
	).flat();
	return { columns, rows, row_count: rows.length, truncated: false };
}

export function forecast(request: StudioForecastPayload): StudioForecastResult {
	const family = request.model_bit_id.replace(
		"fixture-",
		"",
	) as StudioForecastFamily;
	const pointOnly = family === "tiny_time_mixer";
	const quantile_levels = pointOnly ? [] : [0.1, 0.5, 0.9];
	const series = request.mapping.series_id ? machines : [""];
	const combinations = series.flatMap((series_id, machine) =>
		request.mapping.targets.map((target) => ({ series_id, machine, target })),
	);
	const pointValue = (index: number, machine: number, target: string) => {
		if (target === "load_pct")
			return 60 + machine * 7 + Math.sin(index / 8) * 15;
		if (!request.mapping.series_id)
			return (
				machines.reduce((sum, _, group) => sum + value(index, group), 0) /
				machines.length
			);
		return value(index, machine);
	};
	const prediction = (index: number, machine: number, target: string) =>
		pointValue(index, machine, target) + Math.sin(index * 1.1 + machine) * 0.7;
	const points = (from: number, length: number, observed: boolean) =>
		combinations.flatMap(({ series_id, machine, target }) =>
			Array.from({ length }, (_, step) => {
				const index = from + step;
				const point = prediction(index, machine, target);
				return {
					series_id,
					target,
					timestamp_ms: timestamp(index),
					point,
					quantiles: pointOnly
						? []
						: [point - 1.5 - step * 0.04, point, point + 1.5 + step * 0.04],
					actual: observed ? pointValue(index, machine, target) : null,
				};
			}),
		);
	const offset = request.backtest_offset ?? 0;
	const holdout = request.backtest_horizon ?? 0;
	const backtestPoints = points(
		historyLength - offset - holdout,
		holdout,
		true,
	);
	return {
		run_id: `fixture-run-${offset}`,
		model: {
			bit_id: request.model_bit_id,
			family,
			repository: "Fixture data",
			revision: null,
		},
		capabilities: {
			family,
			max_context: 8192,
			max_horizon: pointOnly ? 96 : 1024,
			native_horizon: 96,
			quantile_levels,
			joint_multivariate: family === "chronos2",
			past_covariates: family === "chronos2",
			known_covariates: family === "chronos2",
		},
		input_rows: historyLength * series.length,
		horizon: request.horizon,
		quantile_levels,
		point_statistic: pointOnly ? "mean" : "median",
		history: combinations.flatMap(({ series_id, machine, target }) =>
			Array.from({ length: historyLength }, (_, index) => ({
				series_id,
				target,
				timestamp_ms: timestamp(index),
				value: pointValue(index, machine, target),
			})),
		),
		forecasts: points(historyLength, request.horizon, false),
		covariates: series.flatMap((series_id, machine) =>
			(request.mapping.past_covariates ?? []).flatMap((name) =>
				Array.from({ length: historyLength }, (_, index) => ({
					series_id,
					name,
					timestamp_ms: timestamp(index),
					kind: "past" as const,
					value: pointValue(index, machine, name),
					future: false,
				})),
			),
		),
		backtest: holdout
			? {
					horizon: holdout,
					offset,
					forecasts: backtestPoints,
					metrics: combinations.map(({ series_id, target }) => {
						const selected = backtestPoints.filter(
							(point) =>
								point.series_id === series_id && point.target === target,
						);
						const absolute = selected.reduce(
							(sum, point) => sum + Math.abs(point.point - (point.actual ?? 0)),
							0,
						);
						return {
							series_id,
							target,
							observations: selected.length,
							mae: absolute / selected.length,
							rmse: Math.sqrt(
								selected.reduce(
									(sum, point) =>
										sum + (point.point - (point.actual ?? 0)) ** 2,
									0,
								) / selected.length,
							),
							mape:
								(selected.reduce(
									(sum, point) =>
										sum +
										Math.abs(
											(point.point - (point.actual ?? 1)) / (point.actual ?? 1),
										),
									0,
								) /
									selected.length) *
								100,
							wape:
								(absolute /
									selected.reduce(
										(sum, point) => sum + Math.abs(point.actual ?? 0),
										0,
									)) *
								100,
						};
					}),
				}
			: null,
	};
}
