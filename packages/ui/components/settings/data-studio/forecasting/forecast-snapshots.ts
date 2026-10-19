import { publicWebOrigin } from "../../../../lib/public-web-origin";
import type {
	SavedStudioForecast,
	StudioForecastPayload,
	StudioForecastResult,
	StudioForecastSummary,
} from "../../../../state/backend-state/query-state";

export interface ForecastRunSnapshot {
	request: SavedStudioForecast["request"];
	result: StudioForecastResult;
	fingerprint: string;
}

export function restoreForecastSetup(input: SavedStudioForecast["request"]) {
	const request = captureForecastRequest(input);
	return {
		request,
		covariates: Object.fromEntries([
			...(request.mapping.past_covariates ?? []).map(
				(name) => [name, { enabled: true, role: "past" as const }] as const,
			),
			...(request.mapping.known_covariates ?? []).map(
				(name) => [name, { enabled: true, role: "known" as const }] as const,
			),
		]),
		extras: {
			params: request.params,
			future_params: request.future_params,
			quantile_levels: request.quantile_levels,
			budget: request.budget,
		},
	};
}

export function captureForecastRequest(
	request: StudioForecastPayload | SavedStudioForecast["request"],
): SavedStudioForecast["request"] {
	const { profile_id: _profile, ...stored } = request as StudioForecastPayload;
	return JSON.parse(JSON.stringify(stored)) as SavedStudioForecast["request"];
}

function sortedJson(value: unknown): unknown {
	if (Array.isArray(value)) return value.map(sortedJson);
	if (value !== null && typeof value === "object")
		return Object.fromEntries(
			Object.entries(value)
				.sort(([a], [b]) => a.localeCompare(b))
				.map(([key, item]) => [key, sortedJson(item)]),
		);
	return value;
}

export function forecastRequestFingerprint(
	request: StudioForecastPayload | SavedStudioForecast["request"],
): string {
	return JSON.stringify(
		sortedJson({
			model_bit_id: request.model_bit_id,
			sql: request.sql,
			params: request.params ?? {},
			mapping: {
				timestamp: request.mapping.timestamp,
				series_id: request.mapping.series_id ?? null,
				targets: request.mapping.targets,
				past_covariates: request.mapping.past_covariates ?? [],
				known_covariates: request.mapping.known_covariates ?? [],
				frequency_ms: request.mapping.frequency_ms ?? null,
			},
			horizon: request.horizon,
			quantile_levels: request.quantile_levels ?? [],
			future_sql: request.future_sql ?? null,
			future_params: request.future_params ?? {},
			backtest_horizon: request.backtest_horizon ?? 0,
			backtest_offset: request.backtest_offset ?? 0,
			budget: {
				maximum_rows: request.budget?.maximum_rows ?? 100_000,
				maximum_bytes: request.budget?.maximum_bytes ?? 256 * 1024 * 1024,
				maximum_tensor_elements:
					request.budget?.maximum_tensor_elements ?? 32 * 1024 * 1024,
			},
		}),
	);
}

export function forecastSummary(
	saved: SavedStudioForecast,
): StudioForecastSummary {
	return {
		id: saved.id,
		app_id: saved.app_id,
		name: saved.name,
		description: saved.description,
		created_by: saved.created_by,
		created_at: saved.created_at,
		model_bit_id: saved.request.model_bit_id,
		horizon: saved.request.horizon,
		run_id: saved.result.run_id,
	};
}

export function forecastShareLink(
	location: string,
	forecastId: string,
	userScoped: boolean,
	webOrigin?: string,
): string {
	let url = new URL(location);
	if (webOrigin) {
		const origin = publicWebOrigin(webOrigin);
		if (!origin) throw new Error("The Hub's public app address is invalid.");
		const destination = new URL(origin);
		destination.pathname = url.pathname;
		destination.search = url.search;
		url = destination;
	}
	if (forecastNeedsWebOrigin(url.href))
		throw new Error(
			"A public web app address is needed to share this prediction from desktop.",
		);
	url.hash = "";
	url.searchParams.delete("table");
	url.searchParams.delete("overlay");
	url.searchParams.set("view", "forecast");
	url.searchParams.set("forecast", forecastId);
	if (userScoped) url.searchParams.set("scope", "user");
	else url.searchParams.delete("scope");
	return url.toString();
}

export function forecastNeedsWebOrigin(location: string): boolean {
	const url = new URL(location);
	return (
		!/^https?:$/.test(url.protocol) ||
		url.hostname === "tauri.localhost" ||
		url.hostname.endsWith(".tauri.localhost")
	);
}

export function withoutSeriesTarget(
	targets: readonly string[],
	seriesId: string,
): string[] {
	return targets.filter((target) => target !== seriesId);
}

export function previewForecastTargets(
	targets: readonly string[],
	numericColumns: readonly string[],
	timestamp: string,
	seriesId: string,
): string[] {
	const available = numericColumns.filter(
		(name) => name !== timestamp && name !== seriesId,
	);
	const retained = targets.filter((name) => available.includes(name));
	return retained.length > 0 ? retained : available.slice(0, 1);
}
