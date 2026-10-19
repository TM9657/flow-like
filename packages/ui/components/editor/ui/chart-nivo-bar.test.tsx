import { describe, expect, test } from "bun:test";
import type { BarDatum, ResponsiveBarSvgProps } from "@nivo/bar";
import { load } from "cheerio";
import { renderToStaticMarkup } from "react-dom/server";
import MarkdownBarChart, { formatBarTick } from "./chart-nivo-bar";

const data = [
	{ Model: "Toto-Open-Base-1.0", Downloads: 68539 },
	{ Model: "chronos-t5-small", Downloads: 965015 },
	{ Model: "chronos-bolt-small", Downloads: 6195337 },
	{ Model: "chronos-2", Downloads: 16564577 },
];

function render(overrides: Partial<ResponsiveBarSvgProps<BarDatum>> = {}) {
	return load(
		renderToStaticMarkup(
			<MarkdownBarChart
				data={data}
				indexBy="Model"
				keys={["Downloads"]}
				layout="horizontal"
				defaultWidth={700}
				defaultHeight={350}
				animate={false}
				axisLeft={{ legend: "Model" }}
				axisBottom={{ legend: "Downloads" }}
				{...overrides}
			/>,
		),
	);
}

describe("embedded horizontal bar rendering", () => {
	test("reserves category and legend space and abbreviates numeric ticks", () => {
		const $ = render();
		const margin = Number(
			$("svg > g")
				.attr("transform")
				?.match(/translate\((\d+),/)?.[1],
		);
		expect(margin).toBeGreaterThan(150);
		const labels = $("svg text")
			.map((_, element) => $(element).text())
			.get();
		for (const item of data) expect(labels).toContain(item.Model);
		expect(labels).toContain("Model");
		expect(labels).toContain("Downloads");
		expect(labels).toContain("2M");
		expect(labels).not.toContain("2000000");
	});

	test("keeps exact values accessible while omitting labels that exceed their bars", () => {
		const $ = render();
		const bars = $("rect[data-testid]");
		expect(bars).toHaveLength(4);
		expect(bars.eq(0).parent().find("text")).toHaveLength(0);
		expect(bars.eq(1).parent().find("text")).toHaveLength(0);
		expect(bars.eq(2).parent().find("text").text()).toBe("6,195,337");
		expect(bars.eq(3).parent().find("text").text()).toBe("16,564,577");
		expect(bars.eq(0).attr("aria-label")).toBe(
			"Toto-Open-Base-1.0, Downloads: 68,539",
		);
		expect(bars.eq(0).attr("tabindex")).toBe("0");
	});

	test("uses fewer ticks at narrow widths and keeps explicit formatting and hidden axes", () => {
		const wide = render();
		const narrow = render({ defaultWidth: 350 });
		const ticks = ($: ReturnType<typeof render>) =>
			$("svg text").filter((_, element) =>
				/^\d+(?:\.\d+)?M$/.test($(element).text()),
			);
		expect(ticks(narrow).length).toBeLessThan(ticks(wide).length);
		const custom = render({
			axisLeft: null,
			axisBottom: {
				tickValues: [0, 5000000, 10000000],
				format: (value: number) => `value ${value}`,
			},
			valueFormat: (value) => `exact ${value}`,
		});
		expect(custom("svg text").text()).not.toContain("Toto-Open-Base-1.0");
		expect(custom("svg text").text()).toContain("value 5000000");
		expect(custom("rect[data-testid]").first().attr("aria-label")).toContain(
			"exact 68539",
		);
	});

	test("retains the magnitude of fractional and negative ticks", () => {
		expect(formatBarTick(0.0005)).toBe("0.0005");
		expect(formatBarTick(-2000000)).toBe("-2M");
		expect(formatBarTick(0)).toBe("0");
	});

	test("preserves explicit category formatting, margin, and legend placement", () => {
		const $ = render({
			margin: { left: 210 },
			axisLeft: {
				legend: "Model",
				legendOffset: -180,
				format: (value: string) => `Custom ${value}`,
			},
		});
		expect($("svg > g").attr("transform")).toBe("translate(210,30)");
		expect($("svg text").text()).toContain("Custom Toto-Open-Base-1.0");
		const legend = $("svg text").filter(
			(_, element) => $(element).text() === "Model",
		);
		expect(legend.attr("transform")).toContain("-180");
	});

	test("keeps very small nonzero values in accessible labels", () => {
		const $ = render({ data: [{ Model: "Small value", Downloads: 1e-21 }] });
		expect($("rect[data-testid]").attr("aria-label")).toBe(
			"Small value, Downloads: 0.000000000000000000001",
		);
	});
});
