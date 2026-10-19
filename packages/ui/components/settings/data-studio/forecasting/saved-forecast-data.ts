import type { StudioForecastResult } from "../../../../state/backend-state/query-state";
import {
	buildForecastPlot,
	sampleForecastPlot,
	sampleTimeline,
	visibleCovariatePoints,
} from "./forecast-data";

export type SavedForecastViewMode = "forecast" | "evaluation";

export function savedForecastPresentation(
	result: StudioForecastResult,
	selection: {
		target?: string;
		seriesId?: string;
		view?: SavedForecastViewMode;
	},
) {
	const targets = [...new Set(result.forecasts.map((point) => point.target))];
	const seriesIds = [
		...new Set(result.forecasts.map((point) => point.series_id)),
	];
	const target = selection.target || targets[0] || "";
	const seriesId = selection.seriesId || seriesIds[0] || "";
	const evaluation = selection.view === "evaluation";
	const error =
		!targets.includes(target) || !seriesIds.includes(seriesId)
			? "The selected metric or series is not part of this saved prediction."
			: evaluation && !result.backtest
				? "This prediction has no saved holdout evaluation. Choose Outlook to view its forecast."
				: undefined;
	const history = result.history.filter(
		(point) => point.target === target && point.series_id === seriesId,
	);
	const evaluationPredictions = (result.backtest?.forecasts ?? []).filter(
		(point) => point.target === target && point.series_id === seriesId,
	);
	const evaluationTimes = evaluationPredictions.map(
		(point) => point.timestamp_ms,
	);
	const evaluationStart = evaluationTimes.reduce(
		(first, time) => Math.min(first, time),
		Number.POSITIVE_INFINITY,
	);
	const evaluationEnd = evaluationTimes.reduce(
		(last, time) => Math.max(last, time),
		Number.NEGATIVE_INFINITY,
	);
	const evaluationCutoff = history.reduce<number | undefined>(
		(last, point) =>
			point.timestamp_ms < evaluationStart &&
			(last === undefined || point.timestamp_ms > last)
				? point.timestamp_ms
				: last,
		undefined,
	);
	const evaluationWindow =
		evaluationPredictions.length > 0
			? {
					cutoff: evaluationCutoff,
					start: evaluationStart,
					end: evaluationEnd,
					steps: new Set(evaluationTimes).size,
				}
			: undefined;
	const predictions = (
		evaluation ? (result.backtest?.forecasts ?? []) : result.forecasts
	).filter((point) => point.target === target && point.series_id === seriesId);
	const firstPrediction = predictions.reduce(
		(first, point) => Math.min(first, point.timestamp_ms),
		Number.POSITIVE_INFINITY,
	);
	const cutoff = history.reduce<number | undefined>(
		(last, point) =>
			point.timestamp_ms < firstPrediction &&
			(last === undefined || point.timestamp_ms > last)
				? point.timestamp_ms
				: last,
		undefined,
	);
	const plot = buildForecastPlot(
		history
			.filter((point) => !evaluation || point.timestamp_ms <= evaluationEnd)
			.map((point) => ({
				timestamp: point.timestamp_ms,
				value: point.value,
			})),
		predictions.map((point) => ({
			timestamp: point.timestamp_ms,
			point: point.point,
			quantiles: point.quantiles,
		})),
		cutoff ?? Number.POSITIVE_INFINITY,
		result.quantile_levels,
	);
	const tracks = new Map<string, typeof result.covariates>();
	for (const point of result.covariates) {
		if (point.series_id !== seriesId) continue;
		if (evaluation && point.timestamp_ms > evaluationEnd) continue;
		const track = tracks.get(point.name) ?? [];
		track.push(point);
		tracks.set(point.name, track);
	}
	const covariates = [...tracks].map(([name, values]) => {
		const role = values[0].kind;
		return {
			name,
			role,
			enabled: true,
			points: sampleTimeline(
				visibleCovariatePoints(
					values.map((point) => ({
						timestamp: point.timestamp_ms,
						value: point.value,
					})),
					role,
					evaluation,
					cutoff,
				).sort((a, b) => a.timestamp - b.timestamp),
				(point) => point.value,
				300,
			),
		};
	});
	return {
		target,
		seriesId,
		targets,
		seriesIds,
		evaluation,
		error,
		cutoff,
		evaluationWindow,
		points: sampleForecastPlot(plot),
		fullPointCount: plot.length,
		covariates,
		metrics: result.backtest?.metrics.find(
			(metric) => metric.target === target && metric.series_id === seriesId,
		),
	};
}

export function savedForecastHref(
	appId: string,
	forecastId: string,
	userScoped: boolean,
) {
	const params = new URLSearchParams({
		id: appId,
		view: "forecast",
		forecast: forecastId,
	});
	if (userScoped) params.set("scope", "user");
	return `/library/config/explore?${params}`;
}
