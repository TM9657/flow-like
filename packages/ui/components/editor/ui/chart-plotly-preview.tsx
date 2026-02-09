"use client";

import { useCallback, useEffect, useMemo, useRef } from "react";
import type { ChartInput } from "./chart-data-parser";
import { toPlotlyData } from "./chart-data-parser";

interface PlotlyModule {
	react: (
		root: HTMLElement,
		data: unknown[],
		layout?: Record<string, unknown>,
		config?: Record<string, unknown>,
	) => Promise<void>;
	Plots: { resize: (root: HTMLElement) => void };
	purge: (root: HTMLElement) => void;
}

interface PlotlyChartPreviewProps {
	input: ChartInput;
	height?: number;
}

function PlotlyChartPreview({ input, height = 350 }: PlotlyChartPreviewProps) {
	const containerRef = useRef<HTMLDivElement>(null);
	const plotlyRef = useRef<PlotlyModule | null>(null);

	const { data, layout, config } = useMemo(() => {
		const result = toPlotlyData(input);
		// Ensure height is set
		result.layout.height = input.config.height || height;
		return result;
	}, [input, height]);

	const handleResize = useCallback(() => {
		if (!containerRef.current || !plotlyRef.current) return;
		try {
			plotlyRef.current.Plots.resize(containerRef.current);
		} catch {
			// Ignore resize errors
		}
	}, []);

	useEffect(() => {
		let mounted = true;

		const loadAndRender = async () => {
			if (!containerRef.current) return;

			try {
				const PlotlyModule = await import("plotly.js-dist-min");
				if (!mounted) return;

				// plotly.js-dist-min exports default as the Plotly object
				const Plotly = (PlotlyModule.default ||
					PlotlyModule) as unknown as PlotlyModule;
				plotlyRef.current = Plotly;
				await Plotly.react(containerRef.current, data as any, layout, config);
			} catch (err) {
				console.error("Failed to load/render Plotly chart:", err);
			}
		};

		loadAndRender();

		return () => {
			mounted = false;
			if (containerRef.current && plotlyRef.current) {
				plotlyRef.current.purge(containerRef.current);
			}
		};
	}, [data, layout, config]);

	// Handle resize
	useEffect(() => {
		if (!containerRef.current) return;

		const resizeObserver = new ResizeObserver(handleResize);
		resizeObserver.observe(containerRef.current);
		window.addEventListener("resize", handleResize);

		return () => {
			resizeObserver.disconnect();
			window.removeEventListener("resize", handleResize);
		};
	}, [handleResize]);

	return (
		<div
			ref={containerRef}
			className="w-full min-h-0 rounded-md overflow-hidden"
			style={{ height: input.config.height || height }}
		/>
	);
}

export default PlotlyChartPreview;
