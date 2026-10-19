import type { BoundValue, ForecastChartComponent } from "../types";

export function resolveForecastChartProps(
	component: ForecastChartComponent,
	resolve: (value: BoundValue) => unknown,
	appId?: string,
	contextAppId?: string,
) {
	const value = (bound: BoundValue | undefined) =>
		bound === undefined ? undefined : resolve(bound);
	const text = (bound: BoundValue | undefined) => {
		const resolved = value(bound);
		return typeof resolved === "string"
			? resolved.trim() || undefined
			: undefined;
	};
	const boolean = (bound: BoundValue | undefined) => {
		const resolved = value(bound);
		return typeof resolved === "boolean" ? resolved : undefined;
	};
	const height = value(component.height);
	return {
		appId: text(component.appId) || appId?.trim() || contextAppId?.trim() || "",
		forecastId: text(component.forecastId) || "",
		userScoped: boolean(component.userScoped) ?? false,
		target: text(component.target),
		seriesId: text(component.seriesId),
		view:
			value(component.view) === "evaluation"
				? ("evaluation" as const)
				: ("forecast" as const),
		showCovariates: boolean(component.showCovariates),
		showMetrics: boolean(component.showMetrics),
		showControls: boolean(component.showControls),
		showHeader: boolean(component.showHeader),
		compact: boolean(component.compact),
		height:
			typeof height === "number" && Number.isFinite(height)
				? Math.max(120, Math.min(1200, height))
				: undefined,
	};
}
