import { afterAll, afterEach, describe, expect, mock, test } from "bun:test";
import { createElement } from "react";
import { installStageDom } from "../../interfaces/form-workbench/stage/dom-setup";
import { click } from "../../settings/devices/testing/dom-harness";
import type { ChartInput } from "./chart-data-parser";

const actualPlotlyPreview = { ...(await import("./chart-plotly-preview")) };

mock.module("./chart-plotly-preview", () => ({
	default: ({ input }: { input: ChartInput }) =>
		createElement("svg", {
			"data-chart-preview": "plotly",
			"data-input": JSON.stringify(input.jsonData),
		}),
}));

const dom = installStageDom();
const { StreamingTextEditor } = await import("../../ui/streaming-text-editor");
const { TextEditor } = await import("../../ui/text-editor");

afterEach(dom.cleanup);
afterAll(async () => {
	dom.restore();
	mock.module("./chart-plotly-preview", () => actualPlotlyPreview);
	mock.restore();
	expect((await import("./chart-plotly-preview")).default).toBe(
		actualPlotlyPreview.default,
	);
});

function required(selector: string): Element {
	const element = document.querySelector(selector);
	if (!element) throw new Error(`Missing ${selector}`);
	return element;
}

describe("chart streaming through the Markdown renderer", () => {
	test("starts with a chart placeholder, then plots committed values before JSON closes", async () => {
		const start = '```plotly\n{"data":[';
		const view = await dom.render(
			createElement(StreamingTextEditor, {
				content: start,
				presentation: "chat",
			}),
		);
		expect(required("[data-fl-chart-pending]")).toBeTruthy();
		expect(document.querySelector("pre")).toBeNull();
		const first = `${start}{"type":"scatter","x":["A","B"],"y":[10,2`;
		await view.rerender(
			createElement(StreamingTextEditor, {
				content: first,
				presentation: "chat",
			}),
		);
		const chart = required("[data-chart-preview]");
		expect(
			JSON.parse(chart.getAttribute("data-input") ?? "{}").data[0],
		).toEqual({ type: "scatter", x: ["A"], y: [10] });
		expect(document.querySelector("pre")).toBeNull();
		await view.rerender(
			createElement(StreamingTextEditor, {
				content: `${first}0],"name":"unfinished`,
				presentation: "chat",
			}),
		);
		expect(required("[data-chart-preview]")).toBe(chart);
		expect(
			JSON.parse(chart.getAttribute("data-input") ?? "{}").data[0].y,
		).toEqual([10, 20]);
		const complete = [
			`${start}{"type":"scatter","x":["A","B"],"y":[10,20]}]}`,
			"```",
		].join("\n");
		await view.rerender(
			createElement(TextEditor, {
				initialContent: complete,
				isMarkdown: true,
				presentation: "chat",
			}),
		);
		expect(
			JSON.parse(
				required("[data-chart-preview]").getAttribute("data-input") ?? "{}",
			).data[0].y,
		).toEqual([10, 20]);
	});

	test("source debugging is explicit during streaming and invalid settled data keeps its fallback", async () => {
		const content = '```plotly\n{"data": invalid';
		const view = await dom.render(
			createElement(StreamingTextEditor, { content, presentation: "chat" }),
		);
		expect(document.querySelector("pre")).toBeNull();
		const toggle = Array.from(document.querySelectorAll("button")).find(
			(button) => button.textContent === "View Source",
		);
		if (!toggle) throw new Error("Missing source toggle");
		await click(toggle);
		expect(required("[data-slot=chart-source]").textContent).toContain(
			'"data": invalid',
		);
		await view.rerender(
			createElement(TextEditor, {
				initialContent: content,
				isMarkdown: true,
				presentation: "chat",
			}),
		);
		expect(required("[data-slot=chart-source]").textContent).toContain(
			'"data": invalid',
		);
		expect(document.querySelector("[data-fl-chart-pending]")).toBeNull();
	});
});
