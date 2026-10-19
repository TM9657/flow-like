"use client";

import { cn } from "../../../lib/utils";
import { SavedForecastView } from "../../settings/data-studio/forecasting/saved-forecast-view";
import { useActionContext } from "../ActionHandler";
import type { ComponentProps } from "../ComponentRegistry";
import { useData } from "../DataContext";
import { resolveInlineStyle, resolveStyle } from "../StyleResolver";
import type { ForecastChartComponent } from "../types";
import { resolveForecastChartProps } from "./forecast-chart-props";

export function A2UIForecastChart({
	component,
	appId,
	style,
	elementRef,
}: ComponentProps<ForecastChartComponent>) {
	const { resolve } = useData();
	const { appId: contextAppId } = useActionContext();
	const props = resolveForecastChartProps(
		component,
		resolve,
		appId,
		contextAppId,
	);
	return (
		<div
			ref={elementRef}
			className={cn("min-w-0 w-full", resolveStyle(style))}
			style={resolveInlineStyle(style)}
		>
			{!props.appId || !props.forecastId ? (
				<div className="rounded-lg border border-dashed p-6 text-center text-sm text-muted-foreground">
					{!props.appId
						? "Open this component inside a project or set its project ID."
						: "Select a saved forecast in the component properties."}
				</div>
			) : (
				<SavedForecastView {...props} />
			)}
		</div>
	);
}
