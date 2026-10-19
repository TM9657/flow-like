"use client";

import { Activity, ArrowRight, Clock3, Loader2 } from "lucide-react";
import { useId, useState } from "react";
import {
	Area,
	CartesianGrid,
	ComposedChart,
	Line,
	LineChart,
	ReferenceArea,
	ReferenceLine,
	ResponsiveContainer,
	Tooltip,
	XAxis,
	YAxis,
} from "recharts";
import { cn } from "../../../../lib/utils";
import { Switch } from "../../../ui/switch";
import {
	type ForecastPlotPoint,
	formatForecastNumber,
	formatForecastTick,
	formatForecastTime,
} from "./forecast-data";

const ORANGE = "#ff8700";

export interface CovariateTimeline {
	name: string;
	role: "past" | "known";
	enabled: boolean;
	disabled?: boolean;
	points: { timestamp: number; value: number | null }[];
}

export function ForecastChart({
	points,
	cutoff,
	metric,
	seriesLabel,
	seriesColumn,
	seriesCount,
	hasEvaluationMetrics = false,
	quantileLevels,
	pointStatistic = "median",
	loading,
	stale,
	failedRefresh = false,
	backtest,
	covariates = [],
	onCovariateToggle,
	canUseCovariates = true,
	readOnly = false,
	compact = false,
	height,
}: Readonly<{
	points: ForecastPlotPoint[];
	cutoff?: number;
	metric: string;
	seriesLabel?: string;
	seriesColumn?: string;
	seriesCount?: number;
	hasEvaluationMetrics?: boolean;
	quantileLevels: number[];
	pointStatistic?: "mean" | "median";
	loading: boolean;
	stale: boolean;
	failedRefresh?: boolean;
	backtest: boolean;
	covariates?: CovariateTimeline[];
	onCovariateToggle?: (name: string, enabled: boolean) => void;
	canUseCovariates?: boolean;
	readOnly?: boolean;
	compact?: boolean;
	height?: number;
}>) {
	const gradientId = `forecast-${useId().replaceAll(":", "")}`;
	const [timelineGutter, setTimelineGutter] = useState(compact ? 64 : 130);
	const plotHeight =
		typeof height === "number" && Number.isFinite(height)
			? Math.max(120, Math.min(1200, height))
			: compact
				? 220
				: undefined;
	const levels = [...quantileLevels].sort((a, b) => a - b);
	const bandLabel =
		levels.length > 1
			? `P${Math.round(levels[0] * 100)}–P${Math.round(levels[levels.length - 1] * 100)}`
			: "Quantiles";
	const lastObserved = points.findLast((point) => point.history !== null);
	const latestForecast = points.findLast((point) => point.forecast !== null);
	const groupCount =
		typeof seriesCount === "number" && Number.isFinite(seriesCount)
			? Math.max(1, Math.floor(seriesCount))
			: 1;
	const grouped = Boolean(seriesColumn) || groupCount > 1;
	const selectedSeries = seriesLabel
		? `${seriesColumn || (grouped ? "Group" : "Series")}: ${seriesLabel}`
		: undefined;
	const domain: [number, number] | undefined = points.length
		? [points[0].timestamp, points[points.length - 1].timestamp]
		: undefined;
	return (
		<section
			className="overflow-hidden rounded-2xl border border-white/10 bg-zinc-950 text-zinc-100 shadow-sm"
			aria-label="Forecast visualization"
			aria-busy={loading}
		>
			<div
				className={cn(
					"flex flex-wrap items-start justify-between gap-4 pb-2",
					compact ? "px-3 pt-3" : "px-6 pt-6",
				)}
			>
				<div>
					<p className="flex items-center gap-2 text-[11px] font-medium uppercase tracking-[0.18em] text-zinc-400">
						<Activity className="size-3.5 text-orange-400" />
						{backtest ? "Holdout evaluation" : "Forecast outlook"}
					</p>
					<h3
						className={cn(
							"mt-2 break-words font-medium tracking-tight",
							compact ? "text-base" : "text-xl",
						)}
					>
						{metric && (
							<span className="mr-2 text-sm font-normal text-zinc-400">
								Target
							</span>
						)}
						{metric || "Your next forecast"}
					</h3>
					{points.length > 0 && (
						<div className="mt-2 space-y-1.5" aria-label="Forecast scope">
							<div className="flex flex-wrap items-center gap-x-3 gap-y-1.5 text-xs">
								<span className="rounded-full border border-zinc-700/80 px-2 py-0.5 text-zinc-300">
									{grouped ? "Per-group forecasts" : "Single series"}
								</span>
								{selectedSeries && (
									<span className="break-words text-zinc-300">
										{selectedSeries}
									</span>
								)}
								{grouped && (
									<span className="text-zinc-400">
										{groupCount > 1
											? `1 of ${groupCount.toLocaleString()} groups shown`
											: "1 group"}
									</span>
								)}
							</div>
							{grouped && (
								<p className="max-w-lg text-[11px] leading-relaxed text-zinc-400">
									Each group has its own forecast. An overall forecast needs an
									aggregated source.
								</p>
							)}
						</div>
					)}
					<div className="mt-3 flex flex-wrap items-baseline gap-x-5 gap-y-1">
						{lastObserved && (
							<span
								className={cn(
									"font-medium tabular-nums",
									compact ? "text-lg" : "text-2xl",
								)}
							>
								{formatForecastNumber(lastObserved.history)}
								<span className="ml-2 text-xs font-normal text-zinc-500">
									last observed
								</span>
							</span>
						)}
						{latestForecast && (
							<span className="flex items-center gap-1.5 text-sm tabular-nums text-orange-300">
								<ArrowRight className="size-3.5" />
								{formatForecastNumber(latestForecast.forecast)}
								<span className="text-xs text-zinc-500">
									final {pointStatistic}
								</span>
							</span>
						)}
					</div>
				</div>
				<div className="flex flex-col items-end gap-3 text-xs">
					{loading ? (
						<span className="flex items-center gap-2 text-orange-300">
							<Loader2 className="size-3.5 animate-spin" />
							Running model
						</span>
					) : stale ? (
						<span className="rounded-full border border-amber-500/25 bg-amber-500/10 px-2.5 py-1 text-amber-300">
							{failedRefresh ? "Previous result" : "Settings changed"}
						</span>
					) : cutoff !== undefined ? (
						<span className="flex items-center gap-1.5 text-zinc-500">
							<Clock3 className="size-3.5" />
							{formatForecastTime(cutoff, true)} UTC
						</span>
					) : null}
					<div className="flex flex-wrap justify-end gap-4 text-[11px] text-zinc-400">
						<span className="flex items-center gap-1.5">
							<i className="h-0.5 w-4 bg-zinc-100" />
							History
						</span>
						<span className="flex items-center gap-1.5">
							<i className="h-0.5 w-4 bg-orange-400" />
							{pointStatistic === "mean" ? "Mean" : "Median"}
						</span>
						{levels.length > 1 && (
							<span className="flex items-center gap-1.5">
								<i className="h-2 w-4 rounded-sm bg-orange-400/25" />
								{bandLabel}
							</span>
						)}
						{levels.length === 0 && latestForecast && (
							<span className="text-zinc-500">Point forecast only</span>
						)}
						{backtest && (
							<span className="flex items-center gap-1.5">
								<i className="w-4 border-t border-dashed border-zinc-400" />
								Held-out actual
							</span>
						)}
					</div>
				</div>
			</div>
			<div
				className={cn(
					"relative w-full pb-3 pt-6",
					compact ? "px-3" : "px-6",
					plotHeight === undefined && "h-[320px] sm:h-[360px]",
				)}
				style={plotHeight === undefined ? undefined : { height: plotHeight }}
			>
				{points.length > 0 ? (
					<ResponsiveContainer
						width="100%"
						height="100%"
						minWidth={0}
						initialDimension={{ width: 600, height: 300 }}
						onResize={(width) =>
							setTimelineGutter(compact || width < 480 ? 64 : 130)
						}
					>
						<ComposedChart
							data={points}
							margin={{ top: 12, right: 48, left: 0, bottom: 0 }}
							accessibilityLayer
						>
							<defs>
								<linearGradient id={gradientId} x1="0" y1="0" x2="0" y2="1">
									<stop offset="0%" stopColor={ORANGE} stopOpacity={0.3} />
									<stop offset="100%" stopColor={ORANGE} stopOpacity={0.06} />
								</linearGradient>
							</defs>
							<CartesianGrid
								vertical={false}
								stroke="#27272a"
								strokeOpacity={0.5}
							/>
							{cutoff !== undefined && domain && (
								<ReferenceArea
									x1={domain[0]}
									x2={cutoff}
									fill="#71717a"
									fillOpacity={0.08}
								/>
							)}
							<XAxis
								type="number"
								dataKey="timestamp"
								allowDataOverflow
								domain={domain}
								tickFormatter={(time: number) =>
									formatForecastTick(time, domain ? domain[1] - domain[0] : 0)
								}
								tick={{ fill: "#71717a", fontSize: 11 }}
								axisLine={false}
								tickLine={false}
								minTickGap={50}
								tickMargin={12}
							/>
							<YAxis
								tickFormatter={formatForecastNumber}
								tick={{ fill: "#71717a", fontSize: 11 }}
								axisLine={false}
								tickLine={false}
								width={timelineGutter}
								tickMargin={12}
								domain={["auto", "auto"]}
							/>
							<Tooltip
								content={<ForecastTooltip pointStatistic={pointStatistic} />}
								cursor={{ stroke: "#52525b", strokeDasharray: "3 3" }}
							/>
							{cutoff !== undefined && (
								<ReferenceLine
									x={cutoff}
									stroke="#71717a"
									label={{
										value: backtest ? "Holdout starts" : "Forecast starts",
										position: "insideTopRight",
										fill: "#a1a1aa",
										fontSize: 10,
									}}
								/>
							)}
							{levels.length > 1 && (
								<Area
									type="linear"
									dataKey="interval"
									stroke="none"
									fill={`url(#${gradientId})`}
									isAnimationActive={false}
									connectNulls={false}
								/>
							)}
							<Line
								type="linear"
								dataKey="history"
								stroke="#f4f4f5"
								strokeWidth={1.8}
								dot={false}
								isAnimationActive={false}
								connectNulls={false}
							/>
							<Line
								type="linear"
								dataKey="actual"
								stroke="#a1a1aa"
								strokeWidth={1.5}
								strokeDasharray="4 4"
								dot={false}
								isAnimationActive={false}
								connectNulls={false}
							/>
							<Line
								type="linear"
								dataKey="forecast"
								stroke={ORANGE}
								strokeWidth={2}
								dot={false}
								isAnimationActive={false}
								connectNulls={false}
							/>
						</ComposedChart>
					</ResponsiveContainer>
				) : (
					<div className="absolute inset-x-6 inset-y-8 flex flex-col items-center justify-center rounded-xl border border-dashed border-zinc-800 bg-[linear-gradient(to_bottom,transparent_31px,#27272a55_32px)] bg-[length:100%_32px] px-6 text-center">
						<Activity className="mb-3 size-7 text-zinc-600" />
						<p className="text-sm text-zinc-300">
							See where your data is heading
						</p>
						<p className="mt-1 max-w-sm text-xs leading-relaxed text-zinc-500">
							Choose a table, map a timestamp and a metric, then run your
							forecasting model.
						</p>
					</div>
				)}
			</div>
			{covariates.length > 0 && (
				<div className="border-t border-zinc-800/80">
					<div
						className={cn(
							"flex items-center justify-between gap-3 py-3 text-[10px] font-medium uppercase tracking-[0.16em] text-zinc-500",
							compact ? "px-3" : "px-6",
						)}
					>
						<span>Covariates</span>
						<span>Additional model inputs</span>
					</div>
					{covariates.map((covariate) => (
						<div
							key={covariate.name}
							className={cn(
								"grid items-center border-t border-zinc-800/50 py-2.5",
								compact ? "px-3" : "px-6",
								!covariate.enabled && "opacity-50",
							)}
							style={{
								gridTemplateColumns: `${timelineGutter}px minmax(0, 1fr) 48px`,
							}}
						>
							<div className="min-w-0">
								<p
									className="truncate text-xs leading-normal text-zinc-300"
									title={covariate.name}
								>
									{covariate.name}
								</p>
								<p className="mt-0.5 text-[10px] leading-normal text-zinc-600">
									{covariate.role === "known" ? "Known future" : "Past values"}
								</p>
							</div>
							<div className="h-8 min-w-0" aria-hidden="true">
								{covariate.points.length > 0 ? (
									<ResponsiveContainer
										width="100%"
										height="100%"
										minWidth={0}
										initialDimension={{ width: 300, height: 32 }}
									>
										<LineChart
											data={
												domain
													? covariate.points.filter(
															(point) =>
																point.timestamp >= domain[0] &&
																point.timestamp <= domain[1],
														)
													: covariate.points
											}
											margin={{ left: 0, right: 0, top: 0, bottom: 0 }}
										>
											<XAxis
												hide
												type="number"
												dataKey="timestamp"
												allowDataOverflow
												domain={domain ?? ["dataMin", "dataMax"]}
											/>
											<YAxis hide domain={["auto", "auto"]} />
											{cutoff !== undefined && domain && (
												<ReferenceArea
													x1={domain[0]}
													x2={cutoff}
													fill="#71717a"
													fillOpacity={0.08}
												/>
											)}
											{cutoff !== undefined && (
												<ReferenceLine x={cutoff} stroke="#71717a" />
											)}
											<Line
												dataKey="value"
												type="linear"
												stroke={covariate.enabled ? "#a3a3a3" : "#525252"}
												strokeWidth={1.2}
												dot={false}
												connectNulls={false}
												isAnimationActive={false}
											/>
										</LineChart>
									</ResponsiveContainer>
								) : (
									<div className="flex h-full items-center text-[10px] text-zinc-600">
										Run to view values
									</div>
								)}
							</div>
							{readOnly ? (
								<span className="justify-self-end text-[9px] text-zinc-500">
									Included
								</span>
							) : (
								<Switch
									checked={covariate.enabled}
									onCheckedChange={(enabled) =>
										onCovariateToggle?.(covariate.name, enabled)
									}
									disabled={
										!onCovariateToggle ||
										(!canUseCovariates && !covariate.enabled) ||
										covariate.disabled ||
										loading
									}
									aria-label={`Use ${covariate.name} as a covariate`}
									className="justify-self-end data-[state=checked]:bg-orange-500"
								/>
							)}
						</div>
					))}
				</div>
			)}
			{hasEvaluationMetrics && (
				<p
					className={cn(
						"border-t border-zinc-800 py-3 text-xs leading-relaxed text-zinc-400",
						compact ? "px-3" : "px-6",
					)}
					aria-label="Evaluation scope"
				>
					Holdout metrics below measure {metric} for{" "}
					{selectedSeries || "this series"}, using its held-out actual values.
				</p>
			)}
			{stale && (
				<p className="border-t border-zinc-800 px-6 py-3 text-xs text-amber-300/80">
					{failedRefresh
						? "The latest run failed. Showing the previous successful forecast."
						: "This chart belongs to the previous run. Run again to apply your changes."}
				</p>
			)}
		</section>
	);
}

function ForecastTooltip({
	active,
	payload,
	pointStatistic,
}: {
	active?: boolean;
	payload?: readonly { payload?: ForecastPlotPoint }[];
	pointStatistic: "mean" | "median";
}) {
	const point = payload?.[0]?.payload;
	if (!active || !point) return null;
	return (
		<div className="min-w-40 rounded-lg border border-zinc-700 bg-zinc-900/95 p-3 text-xs text-zinc-200 shadow-xl">
			<p className="mb-2 text-[10px] text-zinc-400">
				{formatForecastTime(point.timestamp, true)} UTC
			</p>
			{point.history !== null && (
				<TooltipValue label="Observed" value={point.history} />
			)}
			{point.actual !== null && (
				<TooltipValue label="Held-out actual" value={point.actual} />
			)}
			{point.forecast !== null && (
				<TooltipValue
					label={`${pointStatistic === "mean" ? "Mean" : "Median"} forecast`}
					value={point.forecast}
					orange
				/>
			)}
			{point.interval && (
				<div className="mt-1 flex justify-between gap-5 text-zinc-500">
					<span>Quantile range</span>
					<span>
						{formatForecastNumber(point.interval[0])} –{" "}
						{formatForecastNumber(point.interval[1])}
					</span>
				</div>
			)}
		</div>
	);
}

function TooltipValue({
	label,
	value,
	orange,
}: { label: string; value: number; orange?: boolean }) {
	return (
		<div
			className={cn(
				"mt-1 flex justify-between gap-5",
				orange && "text-orange-300",
			)}
		>
			<span>{label}</span>
			<span className="font-medium tabular-nums">
				{formatForecastNumber(value)}
			</span>
		</div>
	);
}
