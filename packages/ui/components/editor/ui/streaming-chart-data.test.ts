import { describe, expect, test } from "bun:test";
import { toNivoData, toPlotlyData } from "./chart-data-parser";
import {
	advanceChartSnapshot,
	parseStreamingChartData,
	streamingChartHeight,
} from "./streaming-chart-data";

describe("progressive chart data", () => {
	test("CSV waits for a complete row, including its numeric delimiter", () => {
		const header = "type: bar\nheight: 320\n---\nmodel,power\n";
		expect(parseStreamingChartData(header, "nivo")).toBeNull();
		expect(parseStreamingChartData(`${header}A,4`, "nivo")).toBeNull();
		expect(
			parseStreamingChartData(`${header}A,440\nB,3`, "nivo")?.csvData?.rows,
		).toEqual([["A", 440]]);
		expect(streamingChartHeight(header, null)).toBe(320);
	});

	test("CSV rejects missing cells and malformed rows instead of filling zeroes", () => {
		const input = parseStreamingChartData(
			"type: bar\n---\nname,value\nA,12\nB,\nC,2,3\nD,word\n",
			"nivo",
		);
		expect(input?.csvData?.rows).toEqual([["A", 12]]);
	});

	test("settling accepts the last CSV row without a trailing newline", () => {
		const content = "type: bar\n---\nname,value\nA,12\nB,440";
		const pending = advanceChartSnapshot(null, content, "nivo", true);
		expect(pending.input?.csvData?.rows).toEqual([["A", 12]]);
		const settled = advanceChartSnapshot(pending, content, "nivo", false);
		expect(settled.input?.csvData?.rows).toEqual([
			["A", 12],
			["B", 440],
		]);
	});

	test("Nivo renders complete rows from an open data array", () => {
		const input = parseStreamingChartData(
			'{"chartType":"bar","indexBy":"name","keys":["value"],"data":[{"name":"A","value":40},{"name":"B","value":4',
			"nivo",
		);
		expect(input && toNivoData(input).data).toEqual([{ name: "A", value: 40 }]);
	});

	test("Nivo waits for required chart structure instead of guessing its type", () => {
		expect(
			parseStreamingChartData(
				'{"data":[{"name":"A","value":40}],"chartType":"ba',
				"nivo",
			),
		).toBeNull();
		expect(
			parseStreamingChartData(
				'{"chartType":"bar","data":[{"name":"A","value":40}],',
				"nivo",
			),
		).toBeNull();
	});

	test("Nivo line points can grow before their series object closes", () => {
		const input = parseStreamingChartData(
			'{"chartType":"line","data":[{"id":"Series","data":[{"x":"A","y":10},{"x":"B","y":2',
			"nivo",
		);
		expect(input && toNivoData(input).data).toEqual([
			{ id: "Series", data: [{ x: "A", y: 10 }] },
		]);
	});

	test("Plotly pairs only complete coordinates inside the first open trace", () => {
		const input = parseStreamingChartData(
			'{"data":[{"type":"scatter","name":"Series","x":["A","B"],"y":[10,2',
			"plotly",
		);
		expect(input && toPlotlyData(input).data).toEqual([
			{ type: "scatter", name: "Series", x: ["A"], y: [10] },
		]);
	});

	test("Plotly waits until the trace type and coordinates are known", () => {
		expect(
			parseStreamingChartData('{"data":[{"x":[1,2],"y":[10,20],', "plotly"),
		).toBeNull();
		expect(
			parseStreamingChartData(
				'{"data":[{"type":"scatter","x":[1,2],"y":[',
				"plotly",
			),
		).toBeNull();
	});

	test("partial line series preserve explicit gaps instead of connecting them", () => {
		const input = parseStreamingChartData(
			'{"data":[{"type":"scatter","x":["A","B","C"],"y":[10,null,20],',
			"plotly",
		);
		expect(input && toPlotlyData(input).data).toEqual([
			{ type: "scatter", x: ["A", "B", "C"], y: [10, null, 20] },
		]);
	});

	test("complete JSON retains native settings even while the answer streams", () => {
		const content =
			'{"data":[{"type":"bar","x":["A"],"y":[20],"marker":{"color":"red"}}],"layout":{"title":{"text":"Example"}}}';
		expect(parseStreamingChartData(content, "plotly")?.jsonData).toEqual(
			JSON.parse(content),
		);
	});

	test("unchanged data retains object identity while later bytes arrive", () => {
		const source = "type: bar\n---\nname,value\nA,40\nB,";
		const first = advanceChartSnapshot(null, source, "nivo", true);
		const next = advanceChartSnapshot(first, `${source}4`, "nivo", true);
		expect(next.input).toBe(first.input);
	});

	test("invalid growing payload retains its last chart and edits clear it", () => {
		const source = '{"data":[{"type":"bar","x":["A"],"y":[20]}]}';
		const first = advanceChartSnapshot(null, source, "plotly", true);
		const next = advanceChartSnapshot(
			first,
			`${source}unexpected`,
			"plotly",
			true,
		);
		expect(next.input).toBe(first.input);
		expect(advanceChartSnapshot(next, "{", "plotly", true).input).toBeNull();
		expect(
			advanceChartSnapshot(next, `${source}unexpected`, "plotly", false).input,
		).toBeNull();
	});
});
