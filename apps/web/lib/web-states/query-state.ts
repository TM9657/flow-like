import type {
	CreateSavedQueryPayload,
	CreateStudioForecastPayload,
	ExecuteSqlPayload,
	ExecuteSqlResult,
	IQueryState,
	SavedQuery,
	SavedStudioForecast,
	StudioForecastPayload,
	StudioForecastResult,
	StudioForecastSummary,
	UpdateSavedQueryPayload,
} from "@flow-like/flow-like-ui";
import {
	type WebBackendRef,
	apiDelete,
	apiGet,
	apiPost,
	apiPut,
} from "./api-utils";

function scopeQuery(userScoped?: boolean): string {
	return userScoped ? "?scope=user" : "";
}

export class WebQueryState implements IQueryState {
	constructor(private readonly backend: WebBackendRef) {}

	async listForecasts(
		appId: string,
		userScoped?: boolean,
		maximum = 1000,
	): Promise<StudioForecastSummary[]> {
		if (!Number.isInteger(maximum) || maximum < 1 || maximum > 1000)
			throw new Error("Prediction list size must be between 1 and 1000.");
		const forecasts = new Map<string, StudioForecastSummary>();
		for (let offset = 0; offset < maximum; offset += 100) {
			const limit = Math.min(100, maximum - offset);
			const page = await apiGet<StudioForecastSummary[]>(
				`apps/${encodeURIComponent(appId)}/db/forecasts?limit=${limit}&offset=${offset}${userScoped ? "&scope=user" : ""}`,
				this.backend.auth,
			);
			for (const forecast of page) forecasts.set(forecast.id, forecast);
			if (page.length < limit) break;
		}
		return [...forecasts.values()];
	}

	async getForecast(
		appId: string,
		forecastId: string,
		userScoped?: boolean,
	): Promise<SavedStudioForecast> {
		return apiGet<SavedStudioForecast>(
			`apps/${encodeURIComponent(appId)}/db/forecasts/${encodeURIComponent(forecastId)}${scopeQuery(userScoped)}`,
			this.backend.auth,
		);
	}

	async createForecast(
		appId: string,
		payload: CreateStudioForecastPayload,
		userScoped?: boolean,
	): Promise<SavedStudioForecast> {
		return apiPost<SavedStudioForecast>(
			`apps/${encodeURIComponent(appId)}/db/forecasts${scopeQuery(userScoped)}`,
			payload,
			this.backend.auth,
		);
	}

	async deleteForecast(
		appId: string,
		forecastId: string,
		userScoped?: boolean,
	): Promise<void> {
		await apiDelete<void>(
			`apps/${encodeURIComponent(appId)}/db/forecasts/${encodeURIComponent(forecastId)}${scopeQuery(userScoped)}`,
			this.backend.auth,
		);
	}

	async forecast(
		appId: string,
		payload: StudioForecastPayload,
		userScoped?: boolean,
	): Promise<StudioForecastResult> {
		return apiPost<StudioForecastResult>(
			`apps/${appId}/db/queries/forecast${scopeQuery(userScoped)}`,
			payload,
			this.backend.auth,
		);
	}

	async executeSql(
		appId: string,
		payload: ExecuteSqlPayload,
		userScoped?: boolean,
	): Promise<ExecuteSqlResult> {
		return apiPost<ExecuteSqlResult>(
			`apps/${appId}/db/queries/execute${scopeQuery(userScoped)}`,
			payload,
			this.backend.auth,
		);
	}

	async listSavedQueries(
		appId: string,
		userScoped?: boolean,
	): Promise<SavedQuery[]> {
		return apiGet<SavedQuery[]>(
			`apps/${appId}/db/queries${scopeQuery(userScoped)}`,
			this.backend.auth,
		);
	}

	async getSavedQuery(
		appId: string,
		queryId: string,
		userScoped?: boolean,
	): Promise<SavedQuery> {
		return apiGet<SavedQuery>(
			`apps/${appId}/db/queries/${encodeURIComponent(queryId)}${scopeQuery(userScoped)}`,
			this.backend.auth,
		);
	}

	async createSavedQuery(
		appId: string,
		payload: CreateSavedQueryPayload,
		userScoped?: boolean,
	): Promise<SavedQuery> {
		return apiPost<SavedQuery>(
			`apps/${appId}/db/queries${scopeQuery(userScoped)}`,
			payload,
			this.backend.auth,
		);
	}

	async updateSavedQuery(
		appId: string,
		queryId: string,
		payload: UpdateSavedQueryPayload,
		userScoped?: boolean,
	): Promise<SavedQuery> {
		return apiPut<SavedQuery>(
			`apps/${appId}/db/queries/${encodeURIComponent(queryId)}${scopeQuery(userScoped)}`,
			payload,
			this.backend.auth,
		);
	}

	async deleteSavedQuery(
		appId: string,
		queryId: string,
		userScoped?: boolean,
	): Promise<void> {
		await apiDelete<void>(
			`apps/${appId}/db/queries/${encodeURIComponent(queryId)}${scopeQuery(userScoped)}`,
			this.backend.auth,
		);
	}
}
