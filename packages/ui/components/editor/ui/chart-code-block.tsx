"use client";

import { useTranslation } from "@flow-like/locales";
import {
	type ComponentType,
	useEffect,
	useMemo,
	useRef,
	useState,
} from "react";
import { cn } from "../../../lib/utils";
import { useRendererPresentation } from "../renderer-presentation";
import type { ChartInput } from "./chart-data-parser";
import {
	type ChartSnapshot,
	advanceChartSnapshot,
	streamingChartHeight,
} from "./streaming-chart-data";

type ChartPreviewComponent = ComponentType<{
	input: ChartInput;
	height?: number;
}>;

const CHART_COMPONENTS = {
	nivo: () => import("./chart-nivo-preview"),
	plotly: () => import("./chart-plotly-preview"),
} as const;

const TOGGLE_CLASS =
	"absolute top-2 left-2 z-10 rounded-md border border-border bg-background/95 px-2 py-1 text-xs text-muted-foreground hover:text-foreground";

interface ChartCodeBlockProps {
	/** Raw content from code block */
	content: string;
	/** Language identifier (nivo or plotly) */
	language: "nivo" | "plotly";
	/** Optional CSS class name */
	className?: string;
	isStreaming?: boolean;
}

function ChartLoadingFallback({
	height,
	title,
}: { height: number; title?: string }) {
	const { t } = useTranslation("common");
	return (
		<div data-fl-chart-pending>
			{title && (
				<div className="px-1 pb-2 text-center text-sm font-medium">{title}</div>
			)}
			<div
				className="relative flex items-center justify-center rounded-md bg-muted/10"
				style={{ height }}
			>
				<div
					aria-hidden="true"
					className="absolute inset-x-8 bottom-8 top-10 border-b border-l border-border/70"
				>
					<div className="absolute inset-x-0 top-1/4 border-t border-border/35" />
					<div className="absolute inset-x-0 top-1/2 border-t border-border/35" />
					<div className="absolute inset-x-0 top-3/4 border-t border-border/35" />
				</div>
				<output className="relative rounded bg-background/90 px-3 py-1.5 text-sm text-muted-foreground motion-safe:animate-pulse">
					{t("preparingChart", "Preparing chart…")}
				</output>
			</div>
		</div>
	);
}

function ChartModuleFallback({
	error,
	onRetry,
}: {
	error: string;
	onRetry: () => void;
}) {
	const { t } = useTranslation("common");
	return (
		<div className="flex h-50 flex-col items-center justify-center gap-3 rounded-md bg-destructive/10 p-4">
			<span className="text-center text-sm text-destructive">{error}</span>
			<button
				type="button"
				onClick={onRetry}
				className="rounded bg-background/80 px-3 py-1 text-xs text-foreground"
			>
				{t("retryChartLoad", "Retry chart load")}
			</button>
		</div>
	);
}

/**
 * ChartCodeBlock renders ```nivo``` or ```plotly``` code blocks as interactive charts.
 *
 * Supports two modes:
 * 1. **CSV Mode**: Simple CSV data with optional config header
 *    ```nivo
 *    type: bar
 *    ---
 *    label,value
 *    Jan,20
 *    Feb,14
 *    Mar,25
 *    ```
 *
 * 2. **JSON Mode**: Full Plotly/Nivo JSON configuration
 *    ```plotly
 *    {
 *      "data": [{"x": [1,2,3], "y": [4,5,6], "type": "scatter"}],
 *      "layout": {"title": "My Chart"}
 *    }
 *    ```
 */
export function ChartCodeBlock({
	content,
	language,
	className,
	isStreaming = false,
}: ChartCodeBlockProps) {
	const { t } = useTranslation("common");
	const presentation = useRendererPresentation();
	const isChat = presentation !== "default";
	const toggleClass = isChat
		? "mb-2 ml-auto block rounded px-2 py-1 text-xs text-muted-foreground hover:text-foreground focus-visible:outline-2 focus-visible:outline-ring"
		: TOGGLE_CLASS;
	const [showSource, setShowSource] = useState(false);
	const [ChartComponent, setChartComponent] =
		useState<ChartPreviewComponent | null>(null);
	const [moduleError, setModuleError] = useState<string | null>(null);
	const [isModuleLoading, setIsModuleLoading] = useState(false);
	const [retryKey, setRetryKey] = useState(0);

	const snapshotRef = useRef<ChartSnapshot | null>(null);
	const chartInput = useMemo(() => {
		const next = advanceChartSnapshot(
			snapshotRef.current,
			content,
			language,
			isStreaming,
		);
		snapshotRef.current = next;
		return next.input;
	}, [content, language, isStreaming]);
	const height = streamingChartHeight(content, chartInput);
	const shouldLoadModule = chartInput !== null || isStreaming;

	// biome-ignore lint/correctness/useExhaustiveDependencies: retryKey intentionally restarts a failed module load.
	useEffect(() => {
		if (!shouldLoadModule) {
			setIsModuleLoading(false);
			return;
		}
		let cancelled = false;

		setIsModuleLoading(true);
		setModuleError(null);
		setChartComponent(null);

		CHART_COMPONENTS[language]()
			.then((mod) => {
				if (cancelled) return;
				const component = (mod.default ?? null) as ChartPreviewComponent | null;
				if (!component) {
					setModuleError(
						t(
							"chartPreviewModuleLoadedWithoutADefaultExport",
							"Chart preview module loaded without a default export.",
						),
					);
					return;
				}
				setChartComponent(() => component);
			})
			.catch((error) => {
				if (cancelled) return;
				const message =
					error instanceof Error
						? error.message
						: t("failedToLoadChartPreview", "Failed to load chart preview.");
				setModuleError(message);
			})
			.finally(() => {
				if (!cancelled) {
					setIsModuleLoading(false);
				}
			});

		return () => {
			cancelled = true;
		};
	}, [language, retryKey, shouldLoadModule, t]);

	if (!chartInput && !isStreaming) {
		return (
			<div
				data-fl-chart-appearance={isChat ? "chat" : undefined}
				className={cn("relative", className)}
			>
				<p className="mb-2 text-xs text-muted-foreground">
					{t(
						"chartSourceUntilValid",
						"The chart preview appears when its data is complete and valid.",
					)}
				</p>
				<pre
					data-slot={isChat ? "chart-source" : undefined}
					className="overflow-x-auto rounded-md bg-muted/40 p-4 font-mono text-sm"
				>
					<code>{content}</code>
				</pre>
			</div>
		);
	}

	if (showSource) {
		return (
			<div
				data-fl-chart-appearance={isChat ? "chat" : undefined}
				className={cn("relative", className)}
			>
				<button
					type="button"
					onClick={() => setShowSource(false)}
					className={toggleClass}
				>
					{t("showChart", "Show Chart")}
				</button>
				<pre
					data-slot={isChat ? "chart-source" : undefined}
					className="overflow-x-auto p-4 pt-10 font-mono text-sm bg-muted/50 rounded-md"
				>
					<code>{content}</code>
				</pre>
			</div>
		);
	}

	// Top left, and only on hover: Plotly parks its toolbar in the top right, and
	// a permanent control competes with the chart title.
	return (
		<div
			data-fl-chart-appearance={isChat ? "chat" : undefined}
			className={cn("group relative", className)}
		>
			<button
				type="button"
				onClick={() => setShowSource(true)}
				className={cn(
					toggleClass,
					!isChat &&
						"opacity-0 transition-opacity group-hover:opacity-100 focus-visible:opacity-100",
				)}
			>
				{t("viewSource", "View Source")}
			</button>
			{!moduleError && (!chartInput || isModuleLoading || !ChartComponent) ? (
				<ChartLoadingFallback
					height={height}
					title={chartInput?.config.title}
				/>
			) : null}
			{!isModuleLoading && moduleError ? (
				<ChartModuleFallback
					error={moduleError}
					onRetry={() => setRetryKey((value) => value + 1)}
				/>
			) : null}
			{chartInput && !isModuleLoading && !moduleError && ChartComponent ? (
				<ChartComponent input={chartInput} height={height} />
			) : null}
		</div>
	);
}

export default ChartCodeBlock;
