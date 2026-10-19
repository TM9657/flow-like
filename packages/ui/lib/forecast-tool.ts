import type { IBackendState } from "../state/backend-state";
import type {
	CreateStudioForecastPayload,
	SavedStudioForecast,
	StudioForecastPoint,
	StudioForecastResult,
	StudioForecastSummary,
} from "../state/backend-state/query-state";
import { selectProfileForecastingModels } from "./bit/forecasting-model";

const MAX_CACHE_BYTES = 64 * 1024 * 1024;
const MAX_CACHE_RUNS = 3;
const CACHE_TTL_MS = 30 * 60 * 1000;
const MAX_REQUEST_PREVIEW = 16 * 1024;

export interface ForecastToolHost {
	backend: Pick<IBackendState, "queryState" | "bitState" | "userState">;
	/** API origin, profile, and authenticated subject, supplied by the host. */
	identity: string;
	profileId: string;
	available: boolean;
}

export interface ForecastToolContext {
	appId?: string;
	profileId?: string;
	assertActive?: () => void;
}

type CompletedRun = {
	request: SavedStudioForecast["request"];
	result: StudioForecastResult;
	bytes: number;
	expires: number;
	saves: Map<string, Promise<StudioForecastSummary>>;
};

function requiredText(value: unknown, name: string, maximum = 256): string {
	if (
		typeof value !== "string" ||
		!value.trim() ||
		value.length > maximum ||
		Array.from(value).some((character) => character.charCodeAt(0) < 32)
	)
		throw new Error(
			`${name} must be a non-empty string of at most ${maximum} characters.`,
		);
	return value;
}

function record(value: unknown): value is Record<string, unknown> {
	return !!value && typeof value === "object" && !Array.isArray(value);
}

function href(appId: string, userScoped: boolean, forecastId?: string): string {
	const params = new URLSearchParams({ id: appId, view: "forecast" });
	if (userScoped) params.set("scope", "user");
	if (forecastId) params.set("forecast", forecastId);
	return `/library/config/explore?${params}`;
}

function summary(saved: SavedStudioForecast): StudioForecastSummary {
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

function pointPreview(points: StudioForecastPoint[], limit: number) {
	return {
		total: points.length,
		truncated: points.length > limit,
		points: points.slice(0, limit).map(({ quantiles, ...point }) => ({
			...point,
			// Match the quantile levels in the bounded result preview.
			quantiles: quantiles.slice(0, 21),
		})),
	};
}

function requestPreview(request: SavedStudioForecast["request"]) {
	if (JSON.stringify(request).length <= MAX_REQUEST_PREVIEW)
		return { request: structuredClone(request), request_truncated: false };
	return {
		request: {
			model_bit_id: request.model_bit_id,
			horizon: request.horizon,
			backtest_horizon: request.backtest_horizon ?? 0,
			backtest_offset: request.backtest_offset ?? 0,
		},
		request_truncated: true,
		request_note:
			"The complete setup is available in Data Studio. This preview is too large to return in chat.",
	};
}

function evaluationWindows(result: StudioForecastResult, limit: number) {
	const windows = new Map<
		string,
		{
			series_id: string;
			target: string;
			cutoff_ms: number | null;
			start_ms: number;
			end_ms: number;
			horizon: number;
			observations: number;
		}
	>();
	for (const point of result.backtest?.forecasts ?? []) {
		const key = JSON.stringify([point.series_id, point.target]);
		let window = windows.get(key);
		if (!window) {
			window = {
				series_id: point.series_id,
				target: point.target,
				cutoff_ms: null,
				start_ms: point.timestamp_ms,
				end_ms: point.timestamp_ms,
				horizon: 0,
				observations: 0,
			};
			windows.set(key, window);
		}
		window.start_ms = Math.min(window.start_ms, point.timestamp_ms);
		window.end_ms = Math.max(window.end_ms, point.timestamp_ms);
		window.horizon++;
		if (point.actual !== null) window.observations++;
	}
	for (const point of result.history) {
		const window = windows.get(JSON.stringify([point.series_id, point.target]));
		if (
			window &&
			point.timestamp_ms < window.start_ms &&
			(window.cutoff_ms === null || point.timestamp_ms > window.cutoff_ms)
		)
			window.cutoff_ms = point.timestamp_ms;
	}
	return {
		evaluation_windows: [...windows.values()].slice(0, limit),
		windows_total: windows.size,
		windows_truncated: windows.size > limit,
	};
}

function boundedResponse<T extends Record<string, unknown>>(
	value: T,
): T | Record<string, unknown> {
	if (new TextEncoder().encode(JSON.stringify(value)).byteLength <= 64 * 1024)
		return structuredClone(value);
	const compact: Record<string, unknown> = {
		status: value.status,
		app_id: value.app_id,
		user_scoped: value.user_scoped,
		run_id: value.run_id,
		href: value.href,
		response_truncated: true,
		note: "The result exceeds the chat preview limit. Open Data Studio to inspect the full prediction; a completed run can still be saved using run_id.",
	};
	if (record(value.saved))
		compact.saved = { id: value.saved.id, name: value.saved.name };
	for (const key of ["models", "forecasts"]) {
		if (Array.isArray(value[key])) {
			compact[key] = value[key].slice(0, 20).map((item) => ({
				id: String(item.id).slice(0, 256),
				name: String(item.name).slice(0, 256),
			}));
			compact.truncated = true;
		}
	}
	return compact;
}

function resultPreview(result: StudioForecastResult, limit: number) {
	return {
		run_id: result.run_id,
		model: result.model,
		capabilities: {
			...result.capabilities,
			quantile_levels: result.capabilities.quantile_levels.slice(0, 21),
		},
		input_rows: result.input_rows,
		horizon: result.horizon,
		point_statistic: result.point_statistic,
		quantile_levels: result.quantile_levels.slice(0, 21),
		quantile_levels_truncated: result.quantile_levels.length > 21,
		forecasts: pointPreview(result.forecasts, limit),
		backtest: result.backtest
			? {
					horizon: result.backtest.horizon,
					offset: result.backtest.offset ?? 0,
					...evaluationWindows(result, limit),
					metrics: result.backtest.metrics.slice(0, limit),
					metrics_total: result.backtest.metrics.length,
					metrics_truncated: result.backtest.metrics.length > limit,
					metric_units:
						"MAE and RMSE use target units. MAPE and WAPE are percentage errors, not accuracy scores; lower is better. Null means unavailable.",
					forecasts: pointPreview(result.backtest.forecasts, limit),
				}
			: null,
	};
}

/** Keeps complete native results private to this mounted executor until an approved save. */
export class ForecastToolSession {
	private host?: ForecastToolHost;
	private generation = 0;
	private readonly runs = new Map<string, CompletedRun>();
	constructor(private readonly now: () => number = Date.now) {}

	bind(host: ForecastToolHost) {
		const previous = this.host;
		if (
			!previous ||
			previous.identity !== host.identity ||
			previous.profileId !== host.profileId ||
			previous.available !== host.available ||
			previous.backend.queryState !== host.backend.queryState ||
			previous.backend.bitState !== host.backend.bitState ||
			previous.backend.userState !== host.backend.userState
		)
			this.clear();
		this.host = host;
	}

	clear() {
		this.generation++;
		this.runs.clear();
	}

	private prune() {
		let bytes = 0;
		for (const [key, run] of this.runs) {
			if (run.expires <= this.now()) this.runs.delete(key);
			else bytes += run.bytes;
		}
		for (const [key, run] of this.runs) {
			if (this.runs.size <= MAX_CACHE_RUNS && bytes <= MAX_CACHE_BYTES) break;
			this.runs.delete(key);
			bytes -= run.bytes;
		}
	}

	async execute(args: Record<string, unknown>, context: ForecastToolContext) {
		const host = this.host;
		const generation = this.generation;
		const guard = () => {
			context.assertActive?.();
			if (!host?.available || generation !== this.generation)
				throw new Error(
					"The active account, Hub, profile or forecasting session changed. Start a new request.",
				);
			if (!context.profileId || context.profileId !== host.profileId)
				throw new Error(
					"Forecasting requires the requesting session's active profile. Start a new Data Studio request.",
				);
		};
		guard();
		if (!host) throw new Error("Forecasting is unavailable.");
		const operation = requiredText(args.operation, "operation");
		const appId = requiredText(args.app_id ?? context.appId, "app_id");
		if (appId === "." || appId === "..") throw new Error("Invalid app_id.");
		if (args.user_scoped !== undefined && typeof args.user_scoped !== "boolean")
			throw new Error("user_scoped must be a boolean.");
		const userScoped = args.user_scoped === true;
		const limit = args.limit ?? 20;
		if (
			typeof limit !== "number" ||
			!Number.isInteger(limit) ||
			limit < 1 ||
			limit > 100
		)
			throw new Error("limit must be an integer between 1 and 100.");
		if ("profile_id" in args || "result" in args)
			throw new Error(
				"The host supplies the active profile and completed forecast results.",
			);
		if (!["list_models", "run", "list", "get", "save"].includes(operation))
			throw new Error(`Unsupported forecast_tool operation '${operation}'.`);
		const profile = (await host.backend.userState.getSettingsProfile())
			.hub_profile;
		guard();
		if (profile.id !== host.profileId)
			throw new Error(
				"The active profile changed. Start a new Data Studio request.",
			);
		const envelope = { status: "ok", app_id: appId, user_scoped: userScoped };
		const loadModels = async () => {
			const bits = await host.backend.bitState.getProfileBits();
			guard();
			return selectProfileForecastingModels(bits, profile.bits, profile.hub);
		};
		const query = host.backend.queryState;
		switch (operation) {
			case "list_models": {
				const models = await loadModels();
				return boundedResponse({
					...envelope,
					models: models.slice(0, limit).map((bit) => ({
						id: bit.id,
						name:
							bit.meta.en?.name ?? Object.values(bit.meta)[0]?.name ?? bit.id,
						family: bit.parameters?.model_type,
						hub: bit.hub,
						repository: bit.repository,
					})),
					total: models.length,
					truncated: models.length > limit,
					note: "These models are activated in the current profile. Checkpoint capabilities are returned by run. Add or activate a Forecasting model in the model catalog if the list is empty.",
				});
			}
			case "run": {
				if (!record(args.request) || "profile_id" in args.request)
					throw new Error(
						"run requires a forecast request without profile_id.",
					);
				const request = structuredClone(
					args.request,
				) as SavedStudioForecast["request"];
				requiredText(request.model_bit_id, "request.model_bit_id");
				const models = await loadModels();
				if (!models.some((model) => model.id === request.model_bit_id))
					throw new Error(
						"Choose a Forecasting model returned by list_models for this profile.",
					);
				guard();
				const result = await query.forecast(
					appId,
					{ ...request, profile_id: host.profileId },
					userScoped,
				);
				guard();
				if (!result.run_id || result.model.bit_id !== request.model_bit_id)
					throw new Error(
						"The forecast response does not match its model request.",
					);
				const serialized = JSON.stringify({ request, result });
				const bytes = new TextEncoder().encode(serialized).byteLength;
				if (bytes > MAX_CACHE_BYTES)
					throw new Error(
						"The completed prediction exceeds the chat snapshot limit. Run it in Data Studio with less history.",
					);
				const key = JSON.stringify([appId, userScoped, result.run_id]);
				this.runs.set(key, {
					...JSON.parse(serialized),
					bytes,
					expires: this.now() + CACHE_TTL_MS,
					saves: new Map(),
				});
				this.prune();
				return boundedResponse({
					...envelope,
					...requestPreview(request),
					...resultPreview(result, limit),
					saved: false,
					retention:
						"Save using run_id within 30 minutes. This session retains at most three completed runs and 64 MiB; changing account, Hub or profile clears them.",
				});
			}
			case "list": {
				const forecasts = await query.listForecasts(
					appId,
					userScoped,
					limit + 1,
				);
				guard();
				if (forecasts.some((saved) => saved.app_id !== appId))
					throw new Error("Saved predictions do not match the requested app.");
				return boundedResponse({
					...envelope,
					forecasts: forecasts.slice(0, limit).map((saved) => ({
						...saved,
						href: href(appId, userScoped, saved.id),
					})),
					truncated: forecasts.length > limit,
					href: href(appId, userScoped),
				});
			}
			case "get": {
				const forecastId = requiredText(args.forecast_id, "forecast_id");
				if (forecastId === "." || forecastId === "..")
					throw new Error("Invalid forecast_id.");
				const saved = await query.getForecast(appId, forecastId, userScoped);
				guard();
				if (saved.app_id !== appId || saved.id !== forecastId)
					throw new Error(
						"The saved prediction does not match its app and ID.",
					);
				return boundedResponse({
					...envelope,
					saved: summary(saved),
					href: href(appId, userScoped, saved.id),
					...requestPreview(saved.request),
					...resultPreview(saved.result, limit),
				});
			}
			case "save": {
				if ("request" in args)
					throw new Error(
						"save accepts run_id, name and description; it uses the exact completed request.",
					);
				const runId = requiredText(args.run_id, "run_id");
				const name = requiredText(args.name, "name").trim();
				if (
					args.description !== undefined &&
					(typeof args.description !== "string" ||
						args.description.length > 4000)
				)
					throw new Error(
						"description must be a string of at most 4000 characters.",
					);
				const description = args.description as string | undefined;
				this.prune();
				const run = this.runs.get(JSON.stringify([appId, userScoped, runId]));
				if (!run)
					throw new Error(
						"This completed run is unavailable in the current app, profile and data scope. Run the forecast again before saving.",
					);
				const saveKey = JSON.stringify([name, description ?? null]);
				let pending = run.saves.get(saveKey);
				if (!pending) {
					if (run.saves.size >= 32)
						throw new Error(
							"This run has reached its 32 saved-copy limit. Run another prediction to create more copies.",
						);
					const payload: CreateStudioForecastPayload = structuredClone({
						name,
						description,
						request: run.request,
						result: run.result,
					});
					guard();
					pending = query
						.createForecast(appId, payload, userScoped)
						.then((saved) => {
							if (saved.app_id !== appId || saved.result.run_id !== runId)
								throw new Error(
									"The saved prediction does not match the completed run.",
								);
							return summary(saved);
						});
					run.saves.set(saveKey, pending);
					void pending.catch(() => {
						run.saves.delete(saveKey);
					});
				}
				const saved = await pending;
				guard();
				return structuredClone({
					...envelope,
					saved,
					href: href(appId, userScoped, saved.id),
				});
			}
		}
	}
}
