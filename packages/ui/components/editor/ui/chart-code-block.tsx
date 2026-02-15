"use client";

import { Suspense, lazy, useMemo, useState } from "react";
import { cn } from "../../../lib/utils";
import { type ChartInput, parseChartData } from "./chart-data-parser";

// Lazy load chart components to avoid bundle bloat
const PlotlyChartPreview = lazy(() => import("./chart-plotly-preview"));
const NivoChartPreview = lazy(() => import("./chart-nivo-preview"));

interface ChartCodeBlockProps {
	/** Raw content from code block */
	content: string;
	/** Language identifier (nivo or plotly) */
	language: "nivo" | "plotly";
	/** Optional CSS class name */
	className?: string;
}

function ChartLoadingFallback() {
	return (
		<div className="flex items-center justify-center h-75 bg-muted/20 rounded-md animate-pulse">
			<span className="text-muted-foreground text-sm">Loading chart...</span>
		</div>
	);
}

function ChartErrorFallback({ error }: { error: string }) {
	return (
		<div className="flex items-center justify-center h-50 bg-destructive/10 rounded-md p-4">
			<span className="text-destructive text-sm">{error}</span>
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
}: ChartCodeBlockProps) {
	const [showSource, setShowSource] = useState(false);

	const chartInput = useMemo<ChartInput | null>(() => {
		try {
			return parseChartData(content, language);
		} catch (e) {
			return null;
		}
	}, [content, language]);

	if (!chartInput) {
		return <ChartErrorFallback error="Failed to parse chart data" />;
	}

	if (showSource) {
		return (
			<div className={cn("relative", className)}>
				<button
					type="button"
					onClick={() => setShowSource(false)}
					className="absolute top-2 right-2 z-10 text-xs text-muted-foreground hover:text-foreground bg-background/80 px-2 py-1 rounded"
				>
					Show Chart
				</button>
				<pre className="overflow-x-auto p-4 font-mono text-sm bg-muted/50 rounded-md">
					<code>{content}</code>
				</pre>
			</div>
		);
	}

	return (
		<div className={cn("relative", className)}>
			<button
				type="button"
				onClick={() => setShowSource(true)}
				className="absolute top-2 right-2 z-10 text-xs text-muted-foreground hover:text-foreground bg-background/80 px-2 py-1 rounded"
			>
				View Source
			</button>
			<Suspense fallback={<ChartLoadingFallback />}>
				{language === "plotly" ? (
					<PlotlyChartPreview input={chartInput} />
				) : (
					<NivoChartPreview input={chartInput} />
				)}
			</Suspense>
		</div>
	);
}

export default ChartCodeBlock;
