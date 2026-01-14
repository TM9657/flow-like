"use client";

import { useEffect, useMemo, useRef, useState } from "react";
import { cn } from "../../../lib/utils";
import type { ComponentProps } from "../ComponentRegistry";
import { useData } from "../DataContext";
import { resolveInlineStyle, resolveStyle } from "../StyleResolver";
import type {
	BoundValue,
	NivoChartComponent,
	BarChartStyle,
	LineChartStyle,
	PieChartStyle,
	RadarChartStyle,
	HeatmapChartStyle,
	ScatterChartStyle,
	FunnelChartStyle,
	TreemapChartStyle,
	SankeyChartStyle,
	CalendarChartStyle,
	ChordChartStyle,
} from "../types";

function useResolved<T>(boundValue: BoundValue | undefined): T | undefined {
	const { resolve } = useData();
	if (!boundValue) return undefined;
	return resolve(boundValue) as T;
}

/**
 * NIVO CHART PACKAGES - Install as needed:
 *
 * bun add @nivo/core                    # Required
 * bun add @nivo/bar                     # Bar charts
 * bun add @nivo/line                    # Line charts
 * bun add @nivo/pie                     # Pie/Donut charts
 * bun add @nivo/radar                   # Radar/Spider charts
 * bun add @nivo/heatmap                 # Heatmaps
 * bun add @nivo/scatterplot             # Scatter plots
 * bun add @nivo/funnel                  # Funnel charts
 * bun add @nivo/treemap                 # Treemaps
 * bun add @nivo/sunburst                # Sunburst charts
 * bun add @nivo/calendar                # Calendar heatmaps
 * bun add @nivo/bump                    # Bump & Area Bump charts
 * bun add @nivo/circle-packing          # Circle packing
 * bun add @nivo/network                 # Network graphs
 * bun add @nivo/sankey                  # Sankey diagrams
 * bun add @nivo/stream                  # Stream charts
 * bun add @nivo/swarmplot               # Swarm plots
 * bun add @nivo/voronoi                 # Voronoi diagrams
 * bun add @nivo/waffle                  # Waffle charts
 * bun add @nivo/marimekko               # Marimekko charts
 * bun add @nivo/parallel-coordinates    # Parallel coordinates
 * bun add @nivo/radial-bar              # Radial bar charts
 * bun add @nivo/boxplot                 # Box plots
 * bun add @nivo/bullet                  # Bullet charts
 * bun add @nivo/chord                   # Chord diagrams
 *
 * Install all: bun add @nivo/core @nivo/bar @nivo/line @nivo/pie @nivo/radar @nivo/heatmap @nivo/scatterplot @nivo/funnel @nivo/treemap @nivo/sunburst @nivo/calendar @nivo/bump @nivo/circle-packing @nivo/network @nivo/sankey @nivo/stream @nivo/swarmplot @nivo/voronoi @nivo/waffle @nivo/marimekko @nivo/parallel-coordinates @nivo/radial-bar @nivo/boxplot @nivo/bullet @nivo/chord
 */

type ChartModule = React.ComponentType<Record<string, unknown>>;

type ChartInfo = { pkg: string; component: string; loader: () => Promise<ChartModule | null> };

// eslint-disable-next-line @typescript-eslint/no-explicit-any
const loadChart = async (component: string, importFn: () => Promise<any>): Promise<ChartModule | null> => {
	try {
		const mod = await importFn();
		return (mod[component] as ChartModule) ?? null;
	} catch {
		return null;
	}
};

const CHART_PACKAGES: Record<string, ChartInfo> = {
	bar: { pkg: "@nivo/bar", component: "ResponsiveBar", loader: () => loadChart("ResponsiveBar", () => import("@nivo/bar")) },
	line: { pkg: "@nivo/line", component: "ResponsiveLine", loader: () => loadChart("ResponsiveLine", () => import("@nivo/line")) },
	pie: { pkg: "@nivo/pie", component: "ResponsivePie", loader: () => loadChart("ResponsivePie", () => import("@nivo/pie")) },
	radar: { pkg: "@nivo/radar", component: "ResponsiveRadar", loader: () => loadChart("ResponsiveRadar", () => import("@nivo/radar")) },
	heatmap: { pkg: "@nivo/heatmap", component: "ResponsiveHeatMap", loader: () => loadChart("ResponsiveHeatMap", () => import("@nivo/heatmap")) },
	scatter: { pkg: "@nivo/scatterplot", component: "ResponsiveScatterPlot", loader: () => loadChart("ResponsiveScatterPlot", () => import("@nivo/scatterplot")) },
	funnel: { pkg: "@nivo/funnel", component: "ResponsiveFunnel", loader: () => loadChart("ResponsiveFunnel", () => import("@nivo/funnel")) },
	treemap: { pkg: "@nivo/treemap", component: "ResponsiveTreeMap", loader: () => loadChart("ResponsiveTreeMap", () => import("@nivo/treemap")) },
	sunburst: { pkg: "@nivo/sunburst", component: "ResponsiveSunburst", loader: () => loadChart("ResponsiveSunburst", () => import("@nivo/sunburst")) },
	calendar: { pkg: "@nivo/calendar", component: "ResponsiveCalendar", loader: () => loadChart("ResponsiveCalendar", () => import("@nivo/calendar")) },
	bump: { pkg: "@nivo/bump", component: "ResponsiveBump", loader: () => loadChart("ResponsiveBump", () => import("@nivo/bump")) },
	areaBump: { pkg: "@nivo/bump", component: "ResponsiveAreaBump", loader: () => loadChart("ResponsiveAreaBump", () => import("@nivo/bump")) },
	circlePacking: { pkg: "@nivo/circle-packing", component: "ResponsiveCirclePacking", loader: () => loadChart("ResponsiveCirclePacking", () => import("@nivo/circle-packing")) },
	network: { pkg: "@nivo/network", component: "ResponsiveNetwork", loader: () => loadChart("ResponsiveNetwork", () => import("@nivo/network")) },
	sankey: { pkg: "@nivo/sankey", component: "ResponsiveSankey", loader: () => loadChart("ResponsiveSankey", () => import("@nivo/sankey")) },
	stream: { pkg: "@nivo/stream", component: "ResponsiveStream", loader: () => loadChart("ResponsiveStream", () => import("@nivo/stream")) },
	swarmplot: { pkg: "@nivo/swarmplot", component: "ResponsiveSwarmPlot", loader: () => loadChart("ResponsiveSwarmPlot", () => import("@nivo/swarmplot")) },
	voronoi: { pkg: "@nivo/voronoi", component: "ResponsiveVoronoi", loader: () => loadChart("ResponsiveVoronoi", () => import("@nivo/voronoi")) },
	waffle: { pkg: "@nivo/waffle", component: "ResponsiveWaffle", loader: () => loadChart("ResponsiveWaffle", () => import("@nivo/waffle")) },
	marimekko: { pkg: "@nivo/marimekko", component: "ResponsiveMarimekko", loader: () => loadChart("ResponsiveMarimekko", () => import("@nivo/marimekko")) },
	parallelCoordinates: { pkg: "@nivo/parallel-coordinates", component: "ResponsiveParallelCoordinates", loader: () => loadChart("ResponsiveParallelCoordinates", () => import("@nivo/parallel-coordinates")) },
	radialBar: { pkg: "@nivo/radial-bar", component: "ResponsiveRadialBar", loader: () => loadChart("ResponsiveRadialBar", () => import("@nivo/radial-bar")) },
	boxplot: { pkg: "@nivo/boxplot", component: "ResponsiveBoxPlot", loader: () => loadChart("ResponsiveBoxPlot", () => import("@nivo/boxplot")) },
	bullet: { pkg: "@nivo/bullet", component: "ResponsiveBullet", loader: () => loadChart("ResponsiveBullet", () => import("@nivo/bullet")) },
	chord: { pkg: "@nivo/chord", component: "ResponsiveChord", loader: () => loadChart("ResponsiveChord", () => import("@nivo/chord")) },
};

const SAMPLE_DATA: Record<string, unknown> = {
	bar: [
		{ category: "A", value: 107, color: "#6366f1" },
		{ category: "B", value: 85, color: "#8b5cf6" },
		{ category: "C", value: 126, color: "#a855f7" },
		{ category: "D", value: 65, color: "#d946ef" },
	],
	line: [
		{
			id: "Series 1",
			data: [
				{ x: "Jan", y: 20 },
				{ x: "Feb", y: 45 },
				{ x: "Mar", y: 30 },
				{ x: "Apr", y: 80 },
				{ x: "May", y: 55 },
			],
		},
	],
	pie: [
		{ id: "A", label: "Category A", value: 35 },
		{ id: "B", label: "Category B", value: 25 },
		{ id: "C", label: "Category C", value: 40 },
	],
	radar: [
		{ taste: "Fruity", A: 70, B: 50 },
		{ taste: "Bitter", A: 40, B: 80 },
		{ taste: "Heavy", A: 90, B: 30 },
		{ taste: "Strong", A: 60, B: 70 },
		{ taste: "Sunny", A: 50, B: 60 },
	],
	heatmap: [
		{ id: "Row 1", data: [{ x: "A", y: 10 }, { x: "B", y: 45 }, { x: "C", y: 30 }] },
		{ id: "Row 2", data: [{ x: "A", y: 80 }, { x: "B", y: 25 }, { x: "C", y: 60 }] },
	],
	scatter: [
		{ id: "Group A", data: [{ x: 10, y: 20 }, { x: 30, y: 40 }, { x: 50, y: 30 }, { x: 70, y: 80 }] },
	],
	funnel: [
		{ id: "Step 1", value: 1000, label: "Views" },
		{ id: "Step 2", value: 700, label: "Clicks" },
		{ id: "Step 3", value: 400, label: "Signups" },
		{ id: "Step 4", value: 200, label: "Purchases" },
	],
	treemap: { name: "root", children: [{ name: "A", value: 100 }, { name: "B", value: 80 }, { name: "C", children: [{ name: "C1", value: 40 }, { name: "C2", value: 30 }] }] },
	sunburst: { name: "root", children: [{ name: "A", children: [{ name: "A1", value: 40 }, { name: "A2", value: 30 }] }, { name: "B", value: 50 }] },
	calendar: [{ day: "2024-01-15", value: 50 }, { day: "2024-02-20", value: 80 }, { day: "2024-03-10", value: 30 }, { day: "2024-04-05", value: 90 }],
	bump: [{ id: "A", data: [{ x: 2020, y: 1 }, { x: 2021, y: 2 }, { x: 2022, y: 3 }] }, { id: "B", data: [{ x: 2020, y: 2 }, { x: 2021, y: 1 }, { x: 2022, y: 1 }] }],
	areaBump: [{ id: "A", data: [{ x: 2020, y: 1 }, { x: 2021, y: 2 }, { x: 2022, y: 3 }] }, { id: "B", data: [{ x: 2020, y: 2 }, { x: 2021, y: 1 }, { x: 2022, y: 1 }] }],
	circlePacking: { name: "root", children: [{ name: "A", value: 100 }, { name: "B", children: [{ name: "B1", value: 40 }, { name: "B2", value: 30 }] }] },
	network: { nodes: [{ id: "A" }, { id: "B" }, { id: "C" }, { id: "D" }], links: [{ source: "A", target: "B" }, { source: "B", target: "C" }, { source: "C", target: "D" }] },
	sankey: { nodes: [{ id: "A" }, { id: "B" }, { id: "C" }, { id: "D" }], links: [{ source: "A", target: "C", value: 50 }, { source: "B", target: "C", value: 30 }, { source: "C", target: "D", value: 80 }] },
	stream: [{ A: 10, B: 20, C: 30 }, { A: 15, B: 25, C: 20 }, { A: 20, B: 30, C: 25 }],
	swarmplot: [{ id: "0", group: "A", value: 10 }, { id: "1", group: "A", value: 20 }, { id: "2", group: "B", value: 15 }, { id: "3", group: "B", value: 25 }],
	voronoi: [{ x: 10, y: 20 }, { x: 30, y: 40 }, { x: 50, y: 60 }, { x: 70, y: 30 }],
	waffle: [{ id: "A", label: "Category A", value: 25 }, { id: "B", label: "Category B", value: 35 }, { id: "C", label: "Category C", value: 40 }],
	marimekko: [{ id: "A", value: 100, breakdown: [{ id: "x", value: 60 }, { id: "y", value: 40 }] }, { id: "B", value: 80, breakdown: [{ id: "x", value: 30 }, { id: "y", value: 70 }] }],
	parallelCoordinates: [{ temp: 20, cost: 50, volume: 100 }, { temp: 40, cost: 80, volume: 60 }, { temp: 60, cost: 30, volume: 120 }],
	radialBar: [{ id: "A", data: [{ x: "Value", y: 75 }] }, { id: "B", data: [{ x: "Value", y: 50 }] }],
	boxplot: [{ group: "A", value: 45 }, { group: "A", value: 55 }, { group: "A", value: 60 }, { group: "B", value: 30 }, { group: "B", value: 40 }],
	bullet: [{ id: "Revenue", ranges: [0, 100, 150, 200], measures: [120], markers: [80] }],
	chord: { matrix: [[0, 10, 20], [10, 0, 15], [20, 15, 0]], keys: ["A", "B", "C"] },
};

export function A2UINivoChart({ component, style }: ComponentProps<NivoChartComponent>) {
	const containerRef = useRef<HTMLDivElement>(null);
	const [chartModule, setChartModule] = useState<ChartModule | null>(null);
	const [loading, setLoading] = useState(true);
	const [error, setError] = useState<string | null>(null);

	const chartType = useResolved<string>(component.chartType) ?? "bar";
	const title = useResolved<string>(component.title);
	const height = useResolved<string>(component.height) ?? "400px";
	const rawData = useResolved<unknown>(component.data);
	const rawConfig = useResolved<Record<string, unknown>>(component.config);
	const colors = useResolved<string | string[]>(component.colors);
	const animate = useResolved<boolean>(component.animate) ?? true;
	const showLegend = useResolved<boolean>(component.showLegend) ?? true;
	const legendPosition = useResolved<string>(component.legendPosition) ?? "bottom";
	const indexBy = useResolved<string>(component.indexBy);
	const keys = useResolved<string[]>(component.keys);
	const margin = useResolved<{ top?: number; right?: number; bottom?: number; left?: number }>(component.margin);
	const axisBottom = useResolved<Record<string, unknown>>(component.axisBottom);
	const axisLeft = useResolved<Record<string, unknown>>(component.axisLeft);
	const axisTop = useResolved<Record<string, unknown>>(component.axisTop);
	const axisRight = useResolved<Record<string, unknown>>(component.axisRight);

	// Chart-specific styles
	const barStyle = useResolved<BarChartStyle>(component.barStyle);
	const lineStyle = useResolved<LineChartStyle>(component.lineStyle);
	const pieStyle = useResolved<PieChartStyle>(component.pieStyle);
	const radarStyle = useResolved<RadarChartStyle>(component.radarStyle);
	const heatmapStyle = useResolved<HeatmapChartStyle>(component.heatmapStyle);
	const scatterStyle = useResolved<ScatterChartStyle>(component.scatterStyle);
	const funnelStyle = useResolved<FunnelChartStyle>(component.funnelStyle);
	const treemapStyle = useResolved<TreemapChartStyle>(component.treemapStyle);
	const sankeyStyle = useResolved<SankeyChartStyle>(component.sankeyStyle);
	const calendarStyle = useResolved<CalendarChartStyle>(component.calendarStyle);
	const chordStyle = useResolved<ChordChartStyle>(component.chordStyle);

	useEffect(() => {
		const loadModule = async () => {
			setLoading(true);
			setError(null);
			const chartInfo = CHART_PACKAGES[chartType];
			if (!chartInfo) {
				setError(`Unknown chart type: ${chartType}`);
				setLoading(false);
				return;
			}
			const ChartComponent = await chartInfo.loader();
			if (ChartComponent) setChartModule(() => ChartComponent);
			else setError(`Install with: bun add ${chartInfo.pkg}`);
			setLoading(false);
		};
		loadModule();
	}, [chartType]);

	const data = useMemo(() => {
		if (!rawData) return SAMPLE_DATA[chartType] ?? SAMPLE_DATA.bar;
		if (typeof rawData === "string") {
			try { return JSON.parse(rawData); } catch { return SAMPLE_DATA[chartType] ?? SAMPLE_DATA.bar; }
		}
		return rawData;
	}, [rawData, chartType]);

	const theme = useMemo(() => ({
		background: "transparent",
		text: { fill: "#888" },
		fontSize: 11,
		axis: { domain: { line: { stroke: "#444" } }, ticks: { line: { stroke: "#444" }, text: { fill: "#888" } }, legend: { text: { fill: "#888" } } },
		grid: { line: { stroke: "#333" } },
		legends: { text: { fill: "#888" } },
		labels: { text: { fill: "#fff" } },
		tooltip: { container: { background: "#1a1a1a", color: "#fff", fontSize: 12, borderRadius: 4, boxShadow: "0 2px 8px rgba(0,0,0,0.3)" } },
	}), []);

	const colorScheme = useMemo(() => {
		if (Array.isArray(colors)) return colors;
		if (typeof colors === "string") return { scheme: colors };
		return { scheme: "nivo" };
	}, [colors]);

	const legends = useMemo(() => {
		if (!showLegend) return [];
		const anchorMap: Record<string, string> = { top: "top", bottom: "bottom", left: "left", right: "right" };
		const directionMap: Record<string, string> = { top: "row", bottom: "row", left: "column", right: "column" };
		return [{ anchor: anchorMap[legendPosition] ?? "bottom", direction: directionMap[legendPosition] ?? "row", translateY: legendPosition === "bottom" ? 50 : 0, translateX: legendPosition === "right" ? 100 : 0, itemWidth: 80, itemHeight: 20, symbolSize: 12, symbolShape: "circle" }];
	}, [showLegend, legendPosition]);

	const defaultMargin = useMemo(() => ({ top: margin?.top ?? 50, right: margin?.right ?? 60, bottom: margin?.bottom ?? 60, left: margin?.left ?? 60 }), [margin]);

	const chartProps = useMemo(() => {
		const baseProps: Record<string, unknown> = { data, theme, animate, motionConfig: "gentle", margin: defaultMargin, colors: colorScheme };
		const chartsWithLegends = ["bar", "line", "pie", "radar", "heatmap", "scatter", "stream", "waffle"];
		if (chartsWithLegends.includes(chartType)) baseProps.legends = legends;

		switch (chartType) {
			case "bar": return {
				...baseProps,
				indexBy: indexBy ?? "category",
				keys: keys ?? ["value"],
				layout: barStyle?.layout ?? "vertical",
				groupMode: barStyle?.groupMode ?? "grouped",
				padding: barStyle?.padding ?? 0.3,
				innerPadding: barStyle?.innerPadding ?? 0,
				borderRadius: barStyle?.borderRadius ?? 0,
				borderWidth: barStyle?.borderWidth ?? 0,
				enableLabel: barStyle?.enableLabel ?? true,
				labelSkipWidth: barStyle?.labelSkipWidth ?? 12,
				labelSkipHeight: barStyle?.labelSkipHeight ?? 12,
				enableGridX: barStyle?.enableGridX ?? false,
				enableGridY: barStyle?.enableGridY ?? true,
				axisBottom: axisBottom ?? { tickRotation: 0 },
				axisLeft: axisLeft ?? {},
				axisTop: axisTop ?? null,
				axisRight: axisRight ?? null,
			};
			case "line": return {
				...baseProps,
				xScale: { type: "point" },
				yScale: { type: "linear", min: "auto", max: "auto" },
				curve: lineStyle?.curve ?? "monotoneX",
				lineWidth: lineStyle?.lineWidth ?? 2,
				enableArea: lineStyle?.enableArea ?? false,
				areaOpacity: lineStyle?.areaOpacity ?? 0.2,
				enablePoints: lineStyle?.enablePoints ?? true,
				pointSize: lineStyle?.pointSize ?? 10,
				pointBorderWidth: lineStyle?.pointBorderWidth ?? 2,
				enableSlices: lineStyle?.enableSlices ?? "x",
				enableCrosshair: lineStyle?.enableCrosshair ?? true,
				enableGridX: lineStyle?.enableGridX ?? false,
				enableGridY: lineStyle?.enableGridY ?? true,
				axisBottom: axisBottom ?? { tickRotation: 0 },
				axisLeft: axisLeft ?? {},
				useMesh: true,
			};
			case "pie": return {
				...baseProps,
				innerRadius: pieStyle?.innerRadius ?? 0.5,
				padAngle: pieStyle?.padAngle ?? 0.7,
				cornerRadius: pieStyle?.cornerRadius ?? 3,
				startAngle: pieStyle?.startAngle ?? 0,
				endAngle: pieStyle?.endAngle ?? 360,
				sortByValue: pieStyle?.sortByValue ?? false,
				enableArcLabels: pieStyle?.enableArcLabels ?? true,
				enableArcLinkLabels: pieStyle?.enableArcLinkLabels ?? true,
				arcLabelsSkipAngle: pieStyle?.arcLabelsSkipAngle ?? 10,
				arcLinkLabelsSkipAngle: pieStyle?.arcLinkLabelsSkipAngle ?? 10,
				activeOuterRadiusOffset: pieStyle?.activeOuterRadiusOffset ?? 8,
				borderWidth: 1,
				arcLinkLabelsTextColor: "#888",
				arcLabelsTextColor: "#fff",
			};
			case "radar": return {
				...baseProps,
				keys: keys ?? ["A", "B"],
				indexBy: indexBy ?? "taste",
				gridShape: radarStyle?.gridShape ?? "circular",
				gridLevels: radarStyle?.gridLevels ?? 5,
				gridLabelOffset: radarStyle?.gridLabelOffset ?? 20,
				dotSize: radarStyle?.dotSize ?? 10,
				dotBorderWidth: radarStyle?.dotBorderWidth ?? 2,
				enableDots: radarStyle?.enableDots ?? true,
				enableDotLabel: radarStyle?.enableDotLabel ?? false,
				fillOpacity: radarStyle?.fillOpacity ?? 0.25,
				borderWidth: radarStyle?.borderWidth ?? 2,
			};
			case "heatmap": return {
				...baseProps,
				forceSquare: heatmapStyle?.forceSquare ?? false,
				sizeVariation: heatmapStyle?.sizeVariation ?? 0,
				cellOpacity: heatmapStyle?.cellOpacity ?? 1,
				cellBorderWidth: heatmapStyle?.cellBorderWidth ?? 0,
				enableLabels: heatmapStyle?.enableLabels ?? true,
				labelTextColor: heatmapStyle?.labelTextColor,
				cellBorderColor: { from: "color", modifiers: [["darker", 0.4]] },
				axisTop: axisTop ?? { tickRotation: -45 },
				axisRight: axisRight ?? null,
				axisBottom: axisBottom ?? null,
				axisLeft: axisLeft ?? {},
			};
			case "scatter": return {
				...baseProps,
				xScale: { type: "linear", min: "auto", max: "auto" },
				yScale: { type: "linear", min: "auto", max: "auto" },
				nodeSize: scatterStyle?.nodeSize ?? 10,
				enableGridX: scatterStyle?.enableGridX ?? true,
				enableGridY: scatterStyle?.enableGridY ?? true,
				useMesh: scatterStyle?.useMesh ?? true,
				debugMesh: scatterStyle?.debugMesh ?? false,
				axisBottom: axisBottom ?? {},
				axisLeft: axisLeft ?? {},
			};
			case "funnel": return {
				...baseProps,
				direction: funnelStyle?.direction ?? "horizontal",
				interpolation: funnelStyle?.interpolation ?? "smooth",
				spacing: funnelStyle?.spacing ?? 0,
				shapeBlending: funnelStyle?.shapeBlending ?? 0.66,
				enableLabel: funnelStyle?.enableLabel ?? true,
				labelColor: funnelStyle?.labelColor ?? "#fff",
				borderWidth: 20,
				beforeSeparatorLength: 100,
				afterSeparatorLength: 100,
				currentPartSizeExtension: 10,
				motionConfig: "wobbly",
			};
			case "treemap": return {
				...baseProps,
				identity: "name",
				value: "value",
				tile: treemapStyle?.tile ?? "squarify",
				leavesOnly: treemapStyle?.leavesOnly ?? false,
				innerPadding: treemapStyle?.innerPadding ?? 3,
				outerPadding: treemapStyle?.outerPadding ?? 3,
				enableLabel: treemapStyle?.enableLabel ?? true,
				enableParentLabel: treemapStyle?.enableParentLabel ?? true,
				labelSkipSize: treemapStyle?.labelSkipSize ?? 12,
				label: "name",
				parentLabelPosition: "left",
				parentLabelTextColor: "#fff",
			};
			case "sunburst": return { ...baseProps, id: "name", value: "value", cornerRadius: 2, borderWidth: 1, enableArcLabels: true, arcLabelsSkipAngle: 10 };
			case "calendar": return {
				...baseProps,
				from: "2024-01-01",
				to: "2024-12-31",
				direction: calendarStyle?.direction ?? "horizontal",
				emptyColor: calendarStyle?.emptyColor ?? "#333",
				yearSpacing: calendarStyle?.yearSpacing ?? 40,
				yearLegendOffset: calendarStyle?.yearLegendOffset ?? 10,
				monthSpacing: calendarStyle?.monthSpacing ?? 0,
				monthBorderWidth: calendarStyle?.monthBorderWidth ?? 2,
				monthBorderColor: "#444",
				daySpacing: calendarStyle?.daySpacing ?? 0,
				dayBorderWidth: calendarStyle?.dayBorderWidth ?? 2,
				dayBorderColor: "#1a1a1a",
			};
			case "bump": case "areaBump": return { ...baseProps, xPadding: 0.5, axisTop: null, axisBottom: axisBottom ?? {}, axisLeft: axisLeft ?? {}, pointSize: 10, activePointSize: 16, pointBorderWidth: 3 };
			case "circlePacking": return { ...baseProps, id: "name", value: "value", padding: 4, leavesOnly: false, labelsSkipRadius: 16 };
			case "network": return { ...baseProps, linkDistance: 150, centeringStrength: 0.3, repulsivity: 6, nodeSize: 20, activeNodeSize: 28, linkThickness: 2 };
			case "sankey": return {
				...baseProps,
				layout: sankeyStyle?.layout ?? "horizontal",
				align: sankeyStyle?.align ?? "justify",
				nodeOpacity: sankeyStyle?.nodeOpacity ?? 1,
				nodeThickness: sankeyStyle?.nodeThickness ?? 18,
				nodeInnerPadding: sankeyStyle?.nodeInnerPadding ?? 3,
				nodeSpacing: sankeyStyle?.nodeSpacing ?? 24,
				linkOpacity: sankeyStyle?.linkOpacity ?? 0.5,
				linkBlendMode: sankeyStyle?.linkBlendMode ?? "multiply",
				enableLinkGradient: sankeyStyle?.enableLinkGradient ?? true,
				enableLabels: sankeyStyle?.enableLabels ?? true,
				labelPosition: sankeyStyle?.labelPosition ?? "outside",
			};
			case "stream": return { ...baseProps, keys: keys ?? ["A", "B", "C"], offsetType: "silhouette", order: "none", axisBottom: axisBottom ?? {}, axisLeft: null, enableGridX: true };
			case "swarmplot": return { ...baseProps, groups: ["A", "B"], identity: "id", value: "value", valueScale: { type: "linear", min: 0, max: 100 }, size: 10, spacing: 2, axisBottom: axisBottom ?? {}, axisLeft: axisLeft ?? {} };
			case "voronoi": return { ...baseProps, xDomain: [0, 100], yDomain: [0, 100], enableLinks: true, linkLineWidth: 1, enableCells: true, cellLineWidth: 2, enablePoints: true, pointSize: 6 };
			case "waffle": return { ...baseProps, total: 100, rows: 10, columns: 10, padding: 1, borderWidth: 0, motionStagger: 2 };
			case "marimekko": return { ...baseProps, id: "id", value: "value", dimensions: [{ id: "x", value: "breakdown[0].value" }, { id: "y", value: "breakdown[1].value" }], innerPadding: 3 };
			case "parallelCoordinates": return { ...baseProps, variables: [{ key: "temp", type: "linear", min: 0, max: 100 }, { key: "cost", type: "linear", min: 0, max: 100 }, { key: "volume", type: "linear", min: 0, max: 150 }], lineWidth: 2 };
			case "radialBar": return { ...baseProps, valueFormat: ">-.2f", startAngle: 0, endAngle: 360, innerRadius: 0.2, padding: 0.3, cornerRadius: 2 };
			case "boxplot": return { ...baseProps, groupBy: "group", quantiles: [0.1, 0.25, 0.5, 0.75, 0.9], whiskerWidth: 0.5, borderWidth: 2, medianWidth: 4 };
			case "bullet": return { ...baseProps, layout: "horizontal", titlePosition: "before", titleAlign: "start", measureSize: 0.5, markerSize: 0.6 };
			case "chord": return {
				...baseProps,
				padAngle: chordStyle?.padAngle ?? 0.02,
				innerRadiusRatio: chordStyle?.innerRadiusRatio ?? 0.96,
				innerRadiusOffset: chordStyle?.innerRadiusOffset ?? 0,
				arcOpacity: chordStyle?.arcOpacity ?? 1,
				arcBorderWidth: chordStyle?.arcBorderWidth ?? 1,
				ribbonOpacity: chordStyle?.ribbonOpacity ?? 0.5,
				ribbonBorderWidth: chordStyle?.ribbonBorderWidth ?? 1,
				enableLabel: chordStyle?.enableLabel ?? true,
				labelOffset: chordStyle?.labelOffset ?? 12,
				labelRotation: chordStyle?.labelRotation ?? 0,
			};
			default: return baseProps;
		}
	}, [data, chartType, theme, animate, defaultMargin, colorScheme, legends, indexBy, keys, axisBottom, axisLeft, axisTop, axisRight, barStyle, lineStyle, pieStyle, radarStyle, heatmapStyle, scatterStyle, funnelStyle, treemapStyle, sankeyStyle, calendarStyle, chordStyle]);

	const finalProps = useMemo(() => rawConfig ? { ...chartProps, ...rawConfig } : chartProps, [chartProps, rawConfig]);

	if (loading) {
		return (
			<div ref={containerRef} className={cn("w-full flex flex-col items-center justify-center text-muted-foreground", resolveStyle(style))} style={{ ...resolveInlineStyle(style), height }}>
				Loading chart...
			</div>
		);
	}

	if (error) {
		return (
			<div ref={containerRef} className={cn("w-full flex flex-col items-center justify-center text-destructive p-4", resolveStyle(style))} style={{ ...resolveInlineStyle(style), height }}>
				<div className="text-sm">{error}</div>
				<div className="text-xs text-muted-foreground mt-2">Available: {Object.keys(CHART_PACKAGES).join(", ")}</div>
			</div>
		);
	}

	const ChartComponent = chartModule;

	return (
		<div ref={containerRef} className={cn("w-full flex flex-col", resolveStyle(style))} style={{ ...resolveInlineStyle(style), height }}>
			{title && <div className="text-center text-sm font-medium text-foreground mb-2">{title}</div>}
			<div className="flex-1 min-h-0">{ChartComponent && <ChartComponent {...finalProps} />}</div>
		</div>
	);
}
