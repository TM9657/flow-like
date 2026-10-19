"use client";

import { cn } from "../../../../lib/utils";
import type { StudioBacktestMetric } from "../../../../state/backend-state/query-state";
import { formatForecastNumber } from "./forecast-data";

export function ForecastMetrics({
	metrics,
	compact = false,
	className,
}: {
	metrics: StudioBacktestMetric;
	compact?: boolean;
	className?: string;
}) {
	const items = [
		{
			key: "MAE",
			label: "Mean absolute error",
			value: metrics.mae,
			percent: false,
		},
		{
			key: "RMSE",
			label: "Root mean squared error",
			value: metrics.rmse,
			percent: false,
		},
		{
			key: "WAPE",
			label: "Weighted absolute % error",
			value: metrics.wape,
			percent: true,
		},
		{
			key: "MAPE",
			label: "Mean absolute % error",
			value: metrics.mape,
			percent: true,
		},
	];
	return (
		<section
			className={cn(
				"rounded-xl border bg-card",
				compact ? "p-3" : "p-4",
				className,
			)}
			aria-label="Measured forecast error"
		>
			<div className="mb-3 flex flex-wrap items-center justify-between gap-2">
				<h3 className="text-xs font-medium">Measured forecast error</h3>
				<span className="text-[11px] text-muted-foreground">
					{metrics.observations.toLocaleString()} observed values
				</span>
			</div>
			<dl className="grid grid-cols-[repeat(auto-fit,minmax(8rem,1fr))] gap-3">
				{items.map((item) => (
					<div key={item.key} className="min-w-0">
						<dt className="text-xs leading-normal text-muted-foreground">
							<abbr title={item.label} className="no-underline">
								{item.key}
							</abbr>
							<span className="mt-0.5 block text-[10px] leading-normal">
								{item.label}
							</span>
						</dt>
						<dd
							className={cn(
								"mt-1 font-semibold leading-tight tabular-nums",
								compact ? "text-lg" : "text-xl",
							)}
						>
							{typeof item.value === "number" && Number.isFinite(item.value) ? (
								`${formatForecastNumber(item.value)}${item.percent ? "%" : ""}`
							) : (
								<span className="text-sm font-medium text-muted-foreground">
									Unavailable
								</span>
							)}
						</dd>
					</div>
				))}
			</dl>
			<details className="mt-3 text-[11px] leading-normal text-muted-foreground">
				<summary className="cursor-pointer rounded focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring">
					How to read these metrics
				</summary>
				<div className="mt-2 space-y-1.5">
					<p className="leading-normal">
						Lower errors are better. MAE and RMSE use the metric's units; RMSE
						gives larger errors more weight. Missing actual values are excluded.
					</p>
					<p className="leading-normal">
						WAPE divides total absolute error by total absolute actual values.
						It is unavailable when that total is zero or no observations remain.
					</p>
					<p className="leading-normal">
						MAPE averages each observed value's percentage error. It is
						unavailable if any observed actual is zero or no observations
						remain.
					</p>
					{metrics.wape === undefined && (
						<p className="leading-normal">
							WAPE was not included in this saved result.
						</p>
					)}
				</div>
			</details>
		</section>
	);
}
