export interface HomeForecastConfig {
	appId: string;
	forecastId: string;
	scope: "project" | "personal";
	view: "forecast" | "evaluation";
	seriesId?: string;
	target?: string;
}

export function normalizeHomeForecastConfig(
	config: Record<string, unknown>,
): HomeForecastConfig {
	return {
		appId: typeof config.appId === "string" ? config.appId : "",
		forecastId: typeof config.forecastId === "string" ? config.forecastId : "",
		scope: config.scope === "personal" ? "personal" : "project",
		view: config.view === "evaluation" ? "evaluation" : "forecast",
		...(typeof config.seriesId === "string" && config.seriesId
			? { seriesId: config.seriesId }
			: {}),
		...(typeof config.target === "string" && config.target
			? { target: config.target }
			: {}),
	};
}

export function updateHomeForecastSource(
	config: HomeForecastConfig,
	update: Partial<Pick<HomeForecastConfig, "appId" | "scope" | "forecastId">>,
): HomeForecastConfig {
	const next = { ...config, ...update };
	if (next.appId !== config.appId || next.scope !== config.scope)
		return {
			appId: next.appId,
			scope: next.scope,
			forecastId: "",
			view: "forecast",
		};
	if (next.forecastId !== config.forecastId)
		return {
			appId: next.appId,
			scope: next.scope,
			forecastId: next.forecastId,
			view: "forecast",
		};
	return next;
}
