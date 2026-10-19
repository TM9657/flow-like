"use client";

import { TrendingUp } from "lucide-react";
import { SavedForecastView } from "../../settings/data-studio/forecasting/saved-forecast-view";
import { normalizeHomeForecastConfig } from "../home-forecast-config";
import type { HomeContentProps } from "./config";
import { HomeEmpty } from "./shared";

export function HomeSavedForecast({ widget }: HomeContentProps) {
	const config = normalizeHomeForecastConfig(widget.config);
	if (!config.appId || !config.forecastId)
		return (
			<HomeEmpty icon={<TrendingUp className="size-5" />}>
				Choose an app and a saved prediction in widget settings.
			</HomeEmpty>
		);
	return (
		<SavedForecastView
			appId={config.appId}
			forecastId={config.forecastId}
			userScoped={config.scope === "personal"}
			target={config.target}
			seriesId={config.seriesId}
			view={config.view}
			showControls
			showMetrics
			compact
			className="min-w-0"
		/>
	);
}
