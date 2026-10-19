import { describe, expect, test } from "bun:test";
import {
	hasRenderableNivoData,
	normalizePlotlyTitle,
	parseChartData,
	toNivoData,
	toPlotlyData,
} from "./chart-data-parser";

describe("markdown chart configuration", () => {
	test("maps Nivo labels and a positioned legend without clipping margins", () => {
		const input = parseChartData(
			`type: bar
xLabel: Quarter
yLabel: Revenue
showLegend: true
legendPosition: right
stacked: true
animate: false
---
quarter,product,services
Q1,12,18
Q2,16,21`,
			"nivo",
		);

		const result = toNivoData(input);
		expect(result.chartType).toBe("bar");
		expect(result.props.axisBottom).toEqual({
			legend: "Quarter",
			legendOffset: 44,
			legendPosition: "middle",
		});
		expect(result.props.axisLeft).toEqual({
			legend: "Revenue",
			legendOffset: -52,
			legendPosition: "middle",
		});
		expect(result.props.margin).toEqual({
			top: 30,
			right: 136,
			bottom: 68,
			left: 78,
		});
		expect(result.props.groupMode).toBe("stacked");
		expect(result.props.animate).toBe(false);
		expect(result.props.legends).toEqual([
			expect.objectContaining({
				anchor: "right",
				direction: "column",
				translateX: 112,
			}),
		]);
	});

	test("maps an explicit false stacked option to grouped Nivo bars", () => {
		const input = parseChartData(
			`type: bar
stacked: false
---
quarter,revenue,cost
Q1,120,80`,
			"nivo",
		);

		expect(toNivoData(input).props.groupMode).toBe("grouped");
	});

	test("names horizontal Nivo axes from their data and reserves legend space", () => {
		const input = parseChartData(
			`type: bar
layout: horizontal
---
Model,Downloads
Toto-Open-Base-1.0,68539
chronos-2,16564577`,
			"nivo",
		);

		const { props } = toNivoData(input);
		expect(props.layout).toBe("horizontal");
		expect(props.axisBottom).toEqual(
			expect.objectContaining({ legend: "Downloads" }),
		);
		expect(props.axisLeft).toEqual(
			expect.objectContaining({ legend: "Model" }),
		);
		expect(props.margin).toEqual({
			top: 30,
			right: 30,
			bottom: 68,
			left: 78,
		});
	});

	test("preserves explicit physical axis labels on horizontal Nivo bars", () => {
		const input = parseChartData(
			`type: bar
layout: horizontal
xLabel: Total downloads
yLabel: Forecasting model
---
Model,Downloads
chronos-2,16564577`,
			"nivo",
		);

		const { props } = toNivoData(input);
		expect(props.axisBottom).toEqual(
			expect.objectContaining({ legend: "Total downloads" }),
		);
		expect(props.axisLeft).toEqual(
			expect.objectContaining({ legend: "Forecasting model" }),
		);
	});

	test("infers only the unspecified axis and avoids assigning one measure to multiple series", () => {
		const input = parseChartData(
			`type: bar
layout: horizontal
xLabel: Amount
---
Quarter,Revenue,Cost
Q1,120,80`,
			"nivo",
		);

		const { props } = toNivoData(input);
		expect(props.axisBottom).toEqual(
			expect.objectContaining({ legend: "Amount" }),
		);
		expect(props.axisLeft).toEqual(
			expect.objectContaining({ legend: "Quarter" }),
		);
		input.config.xLabel = undefined;
		expect(toNivoData(input).props.axisBottom).toBeUndefined();
	});

	test("keeps unlabelled vertical Nivo axes unchanged", () => {
		const input = parseChartData("Model,Downloads\nchronos-2,16564577", "nivo");
		const { props } = toNivoData(input);
		expect(props.axisBottom).toBeUndefined();
		expect(props.axisLeft).toBeUndefined();
		expect(props.margin).toEqual({
			top: 30,
			right: 30,
			bottom: 50,
			left: 60,
		});
	});

	test("maps Plotly titles, stacking, horizontal bars, and legend placement", () => {
		const input = parseChartData(
			`type: bar
title: Quarterly mix
xLabel: Amount
yLabel: Quarter
showLegend: true
legendPosition: bottom
stacked: true
layout: horizontal
---
quarter,product,services
Q1,12,18
Q2,16,21`,
			"plotly",
		);

		const result = toPlotlyData(input);
		expect(result.layout.title).toEqual({ text: "Quarterly mix" });
		expect(result.layout.xaxis).toEqual(
			expect.objectContaining({ title: { text: "Amount" } }),
		);
		expect(result.layout.yaxis).toEqual({ title: { text: "Quarter" } });
		expect(result.layout.barmode).toBe("stack");
		expect(result.layout.legend).toEqual(
			expect.objectContaining({
				orientation: "h",
				xanchor: "center",
				yanchor: "top",
			}),
		);
		expect(result.layout.margin).toEqual({ t: 40, r: 20, b: 76, l: 50 });
		expect(result.data[0]).toEqual(
			expect.objectContaining({
				orientation: "h",
				x: [12, 16],
				y: ["Q1", "Q2"],
			}),
		);
	});

	test("normalizes native Plotly string titles without losing their text", () => {
		expect(normalizePlotlyTitle("Monthly sales")).toEqual({
			text: "Monthly sales",
		});
		expect(normalizePlotlyTitle({ text: "Monthly sales", x: 0.5 })).toEqual({
			text: "Monthly sales",
			x: 0.5,
		});
		expect(normalizePlotlyTitle(null)).toEqual({});
	});
});

describe("Plotly rendering", () => {
	test("ignores fence markers that leaked into the block content", () => {
		const input = parseChartData(
			`type: line
---
month,temp
Jan,1
Feb,2
\`\`\``,
			"plotly",
		);

		expect(input.csvData?.rows).toEqual([
			["Jan", 1],
			["Feb", 2],
		]);
		expect((toPlotlyData(input).data[0] as { x: unknown[] }).x).toEqual([
			"Jan",
			"Feb",
		]);
	});

	test("stacks areas through Plotly rather than filling to the raw series", () => {
		const input = parseChartData(
			`type: area
---
month,social,organic
Jan,10,50
Feb,20,40`,
			"plotly",
		);

		const traces = toPlotlyData(input).data as Record<string, unknown>[];
		expect(traces.map((trace) => trace.stackgroup)).toEqual(["one", "one"]);
		expect(traces.every((trace) => trace.fill === undefined)).toBe(true);
	});

	test("fills unstacked areas to zero instead of to the previous series", () => {
		const input = parseChartData(
			`type: area
stacked: false
---
month,social,organic
Jan,10,50`,
			"plotly",
		);

		const traces = toPlotlyData(input).data as Record<string, unknown>[];
		expect(traces.map((trace) => trace.fill)).toEqual(["tozeroy", "tozeroy"]);
		expect(traces.every((trace) => trace.stackgroup === undefined)).toBe(true);
	});

	test("leaves the mode bar to hover so it cannot cover the title", () => {
		const input = parseChartData("month,temp\nJan,1", "plotly");
		expect(toPlotlyData(input).config.displayModeBar).toBe("hover");
	});
});

describe("continuous colour scales", () => {
	test("wraps a heatmap scheme in the sequential config Nivo requires", () => {
		const input = parseChartData(
			`type: heatmap
colors: blues
---
region,q1,q2
North,12,18`,
			"nivo",
		);

		expect(toNivoData(input).props.colors).toEqual({
			type: "sequential",
			scheme: "blues",
		});
	});

	test("reduces a heatmap colour list to the two stops a sequential scale takes", () => {
		const input = parseChartData(
			`type: heatmap
colors: [#001, #445, #eef]
---
region,q1,q2
North,12,18`,
			"nivo",
		);

		expect(toNivoData(input).props.colors).toEqual({
			type: "sequential",
			colors: ["#001", "#eef"],
		});
	});

	test("keeps categorical palettes on charts that use ordinal colours", () => {
		const input = parseChartData(
			`type: bar
colors: [#001, #445, #eef]
---
quarter,revenue
Q1,120`,
			"nivo",
		);

		expect(toNivoData(input).props.colors).toEqual(["#001", "#445", "#eef"]);
	});

	test("drops a categorical colour array from a JSON heatmap block", () => {
		const input = parseChartData(
			JSON.stringify({
				chartType: "heatmap",
				colors: ["#001", "#445"],
				data: [{ id: "North", data: [{ x: "q1", y: 12 }] }],
			}),
			"nivo",
		);

		expect(toNivoData(input).props.colors).toEqual({
			type: "sequential",
			colors: ["#001", "#445"],
		});
	});

	test("leaves a valid continuous config on a JSON heatmap block untouched", () => {
		const input = parseChartData(
			JSON.stringify({
				chartType: "heatmap",
				colors: { type: "diverging", scheme: "red_blue" },
				data: [{ id: "North", data: [{ x: "q1", y: 12 }] }],
			}),
			"nivo",
		);

		expect(toNivoData(input).props.colors).toEqual({
			type: "diverging",
			scheme: "red_blue",
		});
	});
});

describe("renderable chart data", () => {
	test("rejects a series whose points have not streamed in yet", () => {
		const input = parseChartData(
			`type: line
---
month,revenue`,
			"nivo",
		);

		const result = toNivoData(input);
		expect(result.data).toEqual([{ id: "revenue", data: [] }]);
		expect(hasRenderableNivoData(result.data)).toBe(false);
	});

	test("rejects a series with no plottable value", () => {
		const input = parseChartData(
			`type: line
---
month,revenue
Jan,n/a`,
			"nivo",
		);

		const result = toNivoData(input);
		expect(result.data).toEqual([
			{ id: "revenue", data: [{ x: "Jan", y: null }] },
		]);
		expect(hasRenderableNivoData(result.data)).toBe(false);
	});

	test("accepts a series once one point carries a value", () => {
		const input = parseChartData(
			`type: line
---
month,revenue
Jan,n/a
Feb,18`,
			"nivo",
		);

		expect(hasRenderableNivoData(toNivoData(input).data)).toBe(true);
	});

	test("accepts flat item lists and rejects empty ones", () => {
		expect(hasRenderableNivoData([{ quarter: "Q1", revenue: 12 }])).toBe(true);
		expect(hasRenderableNivoData([])).toBe(false);
		expect(hasRenderableNivoData(null)).toBe(false);
	});

	test("rejects a node graph with no nodes", () => {
		expect(hasRenderableNivoData({ nodes: [], links: [] })).toBe(false);
		expect(hasRenderableNivoData({ nodes: [{ id: "a" }], links: [] })).toBe(
			true,
		);
	});
});
