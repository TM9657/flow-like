import { describe, expect, it } from "bun:test";
import type { ComponentProps } from "react";
import { renderToStaticMarkup } from "react-dom/server";
import { ForecastChart } from "./forecast-chart";
import { buildForecastPlot } from "./forecast-data";

describe("forecast output presentation", () => {
	it("labels point-only mean output without a quantile band", () => {
		const html = renderToStaticMarkup(
			<ForecastChart
				points={buildForecastPlot(
					[{ timestamp: 1, value: 10 }],
					[{ timestamp: 2, point: 12.5, quantiles: [] }],
					1,
					[],
				)}
				cutoff={1}
				metric="Sales"
				quantileLevels={[]}
				pointStatistic="mean"
				loading={false}
				stale={false}
				backtest={false}
			/>,
		);
		expect(html).toContain("Point forecast only");
		expect(html).toContain("Mean");
		expect(html).not.toContain("Median");
		expect(html).not.toContain("P10");
		expect(html).not.toContain("P90");
	});

	it("retains the actual quantile range for probabilistic output", () => {
		const html = renderToStaticMarkup(
			<ForecastChart
				points={buildForecastPlot(
					[{ timestamp: 1, value: 10 }],
					[{ timestamp: 2, point: 12.5, quantiles: [11, 14] }],
					1,
					[0.1, 0.9],
				)}
				cutoff={1}
				metric="Sales"
				quantileLevels={[0.1, 0.9]}
				pointStatistic="median"
				loading={false}
				stale={false}
				backtest={false}
			/>,
		);
		expect(html).toContain("P10–P90");
		expect(html).toContain("Median");
		expect(html).not.toContain("Point forecast only");
	});

	it("identifies one selected group and scopes accuracy to its target", () => {
		const html = renderScope({
			seriesLabel: "machine-02",
			seriesColumn: "machine_id",
			seriesCount: 12,
			hasEvaluationMetrics: true,
			backtest: true,
		});
		expect(html).toContain("Per-group forecasts");
		expect(html).toContain("machine_id: machine-02");
		expect(html).toContain("1 of 12 groups shown");
		expect(html).toContain("Each group has its own forecast.");
		expect(html).toContain("An overall forecast needs an aggregated source.");
		expect(html).toContain(
			"Holdout metrics below measure Temperature for machine_id: machine-02, using its held-out actual values.",
		);
		expect(html).toContain("Holdout evaluation");
		expect(html).toContain("Held-out actual");
	});

	it("retains group identity when the result contains just one machine", () => {
		const html = renderScope({
			seriesLabel: "machine-02",
			seriesColumn: "machine_id",
			seriesCount: 1,
		});
		expect(html).toContain("Per-group forecasts");
		expect(html).toContain("machine_id: machine-02");
		expect(html).toContain("1 group");
		expect(html).not.toContain("Single series");
	});

	it("labels an ungrouped source without claiming it is an overall aggregate", () => {
		const html = renderScope({ seriesCount: 1, hasEvaluationMetrics: true });
		expect(html).toContain("Single series");
		expect(html).not.toContain("Per-group forecasts");
		expect(html).not.toContain("overall");
		expect(html).not.toContain("aggregated");
		expect(html).toContain(
			"Holdout metrics below measure Temperature for this series, using its held-out actual values.",
		);
	});

	it("keeps group context in compact embeds without advertising hidden metrics", () => {
		const html = renderScope({
			compact: true,
			seriesLabel: "machine-02",
			seriesCount: 4,
		});
		expect(html).toContain("Group: machine-02");
		expect(html).toContain("1 of 4 groups shown");
		expect(html).not.toContain("Evaluation scope");
		expect(html).not.toContain("Holdout metrics below");
	});
});

function renderScope(props: Partial<ComponentProps<typeof ForecastChart>>) {
	return renderToStaticMarkup(
		<ForecastChart
			points={buildForecastPlot(
				[{ timestamp: 1, value: 10 }],
				[{ timestamp: 2, point: 12.5, quantiles: [11, 14] }],
				1,
				[0.1, 0.9],
			)}
			cutoff={1}
			metric="Temperature"
			quantileLevels={[0.1, 0.9]}
			loading={false}
			stale={false}
			backtest={false}
			{...props}
		/>,
	).replace(/<[^>]*>/g, "");
}
